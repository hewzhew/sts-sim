mod setup;
mod target;

use super::super::action_effects::card_play_effect_facts;
use super::super::action_resource_timing::resource_timing_facts_for_play;
use super::super::enemy_phase_transition::enemy_phase_transition_hint_for_input_with_effects;
use super::super::phase_action_ordering::{
    phase_action_ordering_hint, PhaseActionAccessFacts, PhaseActionOrderingFacts,
};
use super::super::phase_profile::CombatSearchPhaseProfileV1;
use super::super::timed_enemy_threat::timed_enemy_threat_for_target;
use super::super::visible_incoming_damage;
use super::constants::*;
use super::*;
use crate::content::cards::{self, CardType};
use crate::content::powers::PowerId;
use crate::runtime::combat::CombatState;

use setup::{
    current_turn_attack_setup_score, current_turn_retaliation_protection_score,
    key_setup_card_online_candidate,
};
use target::{
    target_enemy_id, target_has_stasis_card, target_progress_hint, target_progress_kills,
};

pub(super) fn priority_for_play_card(
    combat: &CombatState,
    card_index: usize,
    target: Option<usize>,
    phase_profile: CombatSearchPhaseProfileV1,
    plugins: super::super::CombatSearchActionOrderingPlugins<'_>,
) -> ActionOrderingPriority {
    let Some(card) = combat.zones.hand.get(card_index) else {
        return ActionOrderingPriority::neutral(ActionOrderingRole::Neutral);
    };

    let evaluated = cards::evaluate_card_for_play(card, combat, target);
    let def = cards::get_card_definition(card.id);
    let target_kind = cards::effective_target(card);
    let resource_timing = resource_timing_facts_for_play(combat, card_index, target);
    let declared_damage = evaluated
        .base_damage_mut
        .max(resource_timing.conversion_damage_hint)
        .max(0);
    let effects = card_play_effect_facts(combat, card, target);
    let effect_diagnostics = effects.diagnostics();
    let block = evaluated
        .base_block_mut
        .max(resource_timing.conversion_block_hint)
        .max(0)
        .saturating_add(effects.reactive.player_block);
    let phase_transition = enemy_phase_transition_hint_for_input_with_effects(
        combat,
        card_index,
        target,
        phase_profile.enemy_mechanics,
        effects,
        plugins.phase_guard,
    );
    let damage = declared_damage.max(phase_transition.projected_damage_progress);
    let target_progress = phase_transition
        .projected_damage_progress
        .max(target_progress_hint(
            combat,
            target_kind,
            target,
            declared_damage,
        ))
        .saturating_add(effects.reactive.enemy_damage);
    let timed_threat = (target_progress > 0)
        .then(|| target.and_then(|entity_id| timed_enemy_threat_for_target(combat, entity_id)))
        .flatten();
    let mitigation = effects.net_mitigation_ordering_score().max(0);
    let reactive_risk = effects.reactive_risk_score();
    let target_lethal = phase_transition.projected_target_lethal
        || target_progress_kills(combat, target_kind, target, declared_damage);
    let future_debuff = effects.has_future_debuff();
    let current_turn_attack_setup =
        current_turn_attack_setup_score(combat, card_index, card, effects);
    let visible_damage = visible_incoming_damage(combat);
    let available_hand_slots = 10_i32
        .saturating_sub(i32::try_from(combat.zones.hand.len().saturating_sub(1)).unwrap_or(10));
    let effective_draw_supply = if combat.get_power(0, PowerId::NoDraw) != 0 {
        0
    } else {
        effects.total_draw_cards().max(0).min(available_hand_slots)
    };
    let immediate_action_supply =
        effective_draw_supply.saturating_add(effects.direct.player_energy_gain.max(0));
    let safe_immediate_action_supply = immediate_action_supply > 0
        && effects.reactive.player_hp_loss < combat.entities.player.current_hp
        && !effects.reactive.forced_turn_end;
    let current_turn_retaliation_protection = if def.card_type == CardType::Skill
        && block > 0
        && resource_timing.hand_exhaust_target_count == 0
        && !effects.reactive.forced_turn_end
    {
        current_turn_retaliation_protection_score(
            combat,
            card_index,
            block,
            card.cost_for_turn_java(),
            visible_damage,
        )
    } else {
        0
    };
    let phase_hint = phase_action_ordering_hint(
        phase_profile,
        plugins.phase_guard,
        PhaseActionOrderingFacts {
            card_type: def.card_type,
            block,
            mitigation,
            target_progress,
            target_lethal,
            future_debuff,
            access: PhaseActionAccessFacts {
                declared_draw_cards: effects.direct.declared_draw_cards,
                conditional_draw_cards: effects.direct.conditional_draw_cards,
                total_draw_cards: effects.total_draw_cards(),
                bad_draw_cards: effects.reactive.bad_draw_cards,
                forced_turn_end: effects.reactive.forced_turn_end,
            },
            target_enemy_id: target_enemy_id(combat, target),
            target_has_stasis_card: target_has_stasis_card(combat, target),
            phase_transition,
        },
    );
    let current_block = combat.entities.player.block;
    let current_hp = combat.entities.player.current_hp;
    let visible_loss_now = (visible_damage - current_block).max(0);
    let visible_loss_after_block =
        (visible_damage - current_block - block - effects.direct.visible_attack_mitigation_hint)
            .max(0)
            .saturating_add(effects.reactive.player_hp_loss);
    let guardian_mode_shift_interrupts_visible_attack =
        visible_damage > 0 && phase_transition.guardian_mode_shift_trigger_count > 0;
    let prevents_visible_lethal = visible_loss_now >= current_hp
        && (visible_loss_after_block < current_hp || guardian_mode_shift_interrupts_visible_attack);
    let prevents_hp_loss = visible_loss_after_block < visible_loss_now
        || guardian_mode_shift_interrupts_visible_attack;
    let key_setup_card = plugins.action_prior.prioritizes_key_card_online()
        && key_setup_card_online_candidate(card.id, card.upgrades);
    let (role, role_rank) = if target_lethal {
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
    } else if key_setup_card {
        (ActionOrderingRole::KeySetupCard, ROLE_KEY_SETUP_CARD)
    } else if current_turn_retaliation_protection > 0 {
        (
            ActionOrderingRole::CurrentTurnRetaliationProtection,
            ROLE_CURRENT_TURN_RETALIATION_PROTECTION,
        )
    } else if current_turn_attack_setup > 0 {
        (
            ActionOrderingRole::CurrentTurnAttackSetup,
            ROLE_CURRENT_TURN_ATTACK_SETUP,
        )
    } else if def.card_type == CardType::Power
        && effects.direct.player_vulnerable > 0
        && visible_damage > current_block
    {
        // A setup card that immediately exposes the player to amplified
        // visible damage is not automatically better than ending the turn.
        // It remains legal and searchable through the policy floor.
        (ActionOrderingRole::DeferredSetup, ROLE_END_TURN - 1)
    } else if def.card_type == CardType::Power {
        (ActionOrderingRole::DeferredSetup, ROLE_DEFERRED_SETUP)
    } else if prevents_hp_loss && reactive_risk == 0 {
        (ActionOrderingRole::PreventHpLoss, ROLE_PREVENT_HP_LOSS)
    } else if safe_immediate_action_supply {
        (
            ActionOrderingRole::ImmediateActionSupply,
            ROLE_IMMEDIATE_ACTION_SUPPLY,
        )
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
        role_rank: role_rank
            .saturating_add(phase_hint.role_rank_adjustment)
            .saturating_add(resource_timing.role_rank_adjustment),
        mitigation,
        action_supply: immediate_action_supply,
        reactive_risk: -reactive_risk,
        targets_timed_threat: i32::from(
            timed_threat.is_some_and(|fact| fact.canceled_by_owner_death),
        ),
        timed_threat_urgency: timed_threat
            .map(|fact| -(fact.owner_turns_until_trigger.min(i32::MAX as u32) as i32))
            .unwrap_or_default(),
        timed_threat_raw_damage: timed_threat
            .map(|fact| fact.raw_player_damage)
            .unwrap_or_default(),
        target_progress,
        block,
        damage,
        cheaper_cost: -card.cost_for_turn_java().max(0),
        phase_setup: phase_hint
            .phase_setup
            .saturating_add(current_turn_attack_setup)
            .saturating_add(current_turn_retaliation_protection),
        phase_survival: phase_hint.phase_survival,
        phase_transition_safety: phase_hint.phase_transition_safety,
        resource_timing: resource_timing.ordering_score,
        phase_hint,
        effects: effect_diagnostics,
        ..ActionOrderingPriority::neutral(role)
    }
}
