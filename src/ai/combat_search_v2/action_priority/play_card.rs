use super::super::action_effects::summarize_play_card_effects;
use super::super::phase_action_ordering::{phase_action_ordering_hint, PhaseActionOrderingFacts};
use super::super::phase_profile::CombatSearchPhaseProfileV1;
use super::super::{enemy_phase_transition_hint_for_input, visible_incoming_damage};
use super::constants::*;
use super::*;
use crate::content::cards::{self, CardTarget, CardType};
use crate::content::monsters::EnemyId;
use crate::content::powers::{store, PowerId};
use crate::runtime::combat::CombatState;
use crate::state::core::ClientInput;

pub(super) fn priority_for_play_card(
    combat: &CombatState,
    card_index: usize,
    target: Option<usize>,
    phase_profile: CombatSearchPhaseProfileV1,
    phase_guard_policy: super::super::CombatSearchV2PhaseGuardPolicy,
) -> ActionOrderingPriority {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return ActionOrderingPriority::neutral(ActionOrderingRole::Neutral);
    };

    let evaluated = cards::evaluate_card_for_play(card, combat, target);
    let def = cards::get_card_definition(card.id);
    let target_kind = cards::effective_target(card);
    let damage = evaluated.base_damage_mut.max(0);
    let effects = summarize_play_card_effects(combat, card, target);
    let effect_diagnostics = effects.diagnostics();
    let block = evaluated
        .base_block_mut
        .max(0)
        .saturating_add(effects.reactive_player_block);
    let target_progress = target_progress_hint(combat, target_kind, target, damage)
        .saturating_add(effects.reactive_enemy_damage);
    let mitigation = effects.net_mitigation_ordering_score().max(0);
    let reactive_risk = effects.reactive_risk_score();
    let phase_transition = enemy_phase_transition_hint_for_input(
        combat,
        &ClientInput::PlayCard { card_index, target },
        phase_guard_policy,
    );
    let current_turn_attack_setup =
        current_turn_attack_setup_score(combat, card_index, card, effects);
    let phase_hint = phase_action_ordering_hint(
        phase_profile,
        PhaseActionOrderingFacts {
            card_type: def.card_type,
            block,
            mitigation,
            target_progress,
            target_enemy_id: target_enemy_id(combat, target),
            target_has_stasis_card: target_has_stasis_card(combat, target),
            phase_transition,
        },
    );
    let visible_damage = visible_incoming_damage(combat);
    let current_block = combat.entities.player.block;
    let current_hp = combat.entities.player.current_hp;
    let visible_loss_now = (visible_damage - current_block).max(0);
    let visible_loss_after_block =
        (visible_damage - current_block - block - effects.visible_attack_mitigation_hint)
            .max(0)
            .saturating_add(effects.reactive_player_hp_loss);
    let prevents_visible_lethal =
        visible_loss_now >= current_hp && visible_loss_after_block < current_hp;
    let prevents_hp_loss = visible_loss_after_block < visible_loss_now;
    let (role, role_rank) = if target_progress_kills(combat, target_kind, target, damage) {
        (ActionOrderingRole::LethalCard, ROLE_LETHAL_CARD)
    } else if prevents_visible_lethal {
        (
            ActionOrderingRole::PreventVisibleLethal,
            ROLE_PREVENT_VISIBLE_LETHAL,
        )
    } else if mitigation > 0 {
        (
            ActionOrderingRole::SustainedMitigation,
            ROLE_SUSTAINED_MITIGATION,
        )
    } else if current_turn_attack_setup > 0 {
        (
            ActionOrderingRole::CurrentTurnAttackSetup,
            ROLE_CURRENT_TURN_ATTACK_SETUP,
        )
    } else if def.card_type == CardType::Power {
        (ActionOrderingRole::DeferredSetup, ROLE_DEFERRED_SETUP)
    } else if prevents_hp_loss && reactive_risk == 0 {
        (ActionOrderingRole::PreventHpLoss, ROLE_PREVENT_HP_LOSS)
    } else if target_progress > 0 {
        (ActionOrderingRole::DamageProgress, ROLE_DAMAGE_PROGRESS)
    } else if prevents_hp_loss {
        (
            ActionOrderingRole::ReactiveRiskPreventHpLoss,
            ROLE_REACTIVE_RISK_PREVENT_HP_LOSS,
        )
    } else if block > 0 {
        (ActionOrderingRole::Block, ROLE_BLOCK)
    } else {
        (ActionOrderingRole::UtilityPlay, ROLE_UTILITY_PLAY)
    };

    ActionOrderingPriority {
        role,
        role_rank: role_rank.saturating_add(phase_hint.role_rank_adjustment),
        mitigation,
        reactive_risk: -reactive_risk,
        target_progress,
        block,
        damage,
        cheaper_cost: -card.cost_for_turn_java().max(0),
        phase_setup: phase_hint
            .phase_setup
            .saturating_add(current_turn_attack_setup),
        phase_survival: phase_hint.phase_survival,
        phase_transition_safety: phase_hint.phase_transition_safety,
        phase_hint,
        effects: effect_diagnostics,
        ..ActionOrderingPriority::neutral(role)
    }
}

fn current_turn_attack_setup_score(
    combat: &CombatState,
    card_index: usize,
    card: &crate::runtime::combat::CombatCard,
    effects: super::super::action_effects::PlayCardEffectSummary,
) -> i32 {
    if effects.player_strength_gain <= 0 {
        return 0;
    }

    let setup_cost = card.cost_for_turn_java().max(0);
    let available_energy = i32::from(combat.turn.energy);
    if setup_cost > available_energy {
        return 0;
    }
    let remaining_energy = available_energy - setup_cost;
    let playable_attacks = combat
        .zones
        .hand
        .iter()
        .enumerate()
        .filter(|(index, candidate)| {
            *index != card_index
                && cards::get_card_definition(candidate.id).card_type == CardType::Attack
                && cards::can_play_card(candidate, combat).is_ok()
                && attack_cost_is_payable_after_setup(candidate, remaining_energy)
        })
        .count() as i32;

    effects
        .player_strength_gain
        .saturating_mul(playable_attacks)
}

fn attack_cost_is_payable_after_setup(
    card: &crate::runtime::combat::CombatCard,
    remaining_energy: i32,
) -> bool {
    let cost = card.cost_for_turn_java();
    if cost < 0 {
        return remaining_energy > 0;
    }
    cost <= remaining_energy
}

fn target_progress_hint(
    combat: &CombatState,
    target_kind: CardTarget,
    target: Option<usize>,
    damage: i32,
) -> i32 {
    if damage <= 0 {
        return 0;
    }

    match target_kind {
        CardTarget::AllEnemy => combat
            .entities
            .monsters
            .iter()
            .filter(|monster| monster.is_alive_for_action())
            .map(|monster| damage.min(monster.current_hp + monster.block).max(0))
            .sum(),
        CardTarget::Enemy | CardTarget::SelfAndEnemy => target
            .and_then(|target| monster_hp_with_block(combat, target))
            .map(|hp| damage.min(hp).max(0))
            .unwrap_or_default(),
        _ => 0,
    }
}

fn target_progress_kills(
    combat: &CombatState,
    target_kind: CardTarget,
    target: Option<usize>,
    damage: i32,
) -> bool {
    if damage <= 0 {
        return false;
    }

    match target_kind {
        CardTarget::AllEnemy => combat
            .entities
            .monsters
            .iter()
            .filter(|monster| monster.is_alive_for_action())
            .any(|monster| damage >= monster.current_hp + monster.block),
        CardTarget::Enemy | CardTarget::SelfAndEnemy => target
            .and_then(|target| monster_hp_with_block(combat, target))
            .is_some_and(|hp| damage >= hp),
        _ => false,
    }
}

fn monster_hp_with_block(combat: &CombatState, entity_id: usize) -> Option<i32> {
    combat
        .entities
        .monsters
        .iter()
        .find(|monster| monster.id == entity_id && monster.is_alive_for_action())
        .map(|monster| monster.current_hp + monster.block)
}

fn target_enemy_id(combat: &CombatState, target: Option<usize>) -> Option<EnemyId> {
    target
        .and_then(|entity_id| {
            combat
                .entities
                .monsters
                .iter()
                .find(|monster| monster.id == entity_id && monster.is_alive_for_action())
        })
        .and_then(|monster| EnemyId::from_id(monster.monster_type))
}

fn target_has_stasis_card(combat: &CombatState, target: Option<usize>) -> bool {
    target.is_some_and(|entity_id| store::has_power(combat, entity_id, PowerId::Stasis))
}
