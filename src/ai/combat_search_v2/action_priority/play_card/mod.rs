mod setup;
mod target;

use super::super::action_effects::card_play_effect_facts;
use super::super::action_resource_timing::resource_timing_facts_for_play;
use super::super::phase_action_ordering::{
    phase_action_ordering_hint, PhaseActionAccessFacts, PhaseActionOrderingFacts,
};
use super::super::phase_profile::CombatSearchPhaseProfileV1;
use super::super::{enemy_phase_transition_hint_for_input, visible_incoming_damage};
use super::constants::*;
use super::*;
use crate::content::cards::{self, CardType};
use crate::runtime::combat::CombatState;
use crate::state::core::ClientInput;

use setup::{current_turn_attack_setup_score, key_setup_card_online_candidate};
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
    let damage = evaluated
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
    let target_progress = target_progress_hint(combat, target_kind, target, damage)
        .saturating_add(effects.reactive.enemy_damage);
    let mitigation = effects.net_mitigation_ordering_score().max(0);
    let reactive_risk = effects.reactive_risk_score();
    let target_lethal = target_progress_kills(combat, target_kind, target, damage);
    let future_debuff = effects.has_future_debuff();
    let phase_transition = enemy_phase_transition_hint_for_input(
        combat,
        &ClientInput::PlayCard { card_index, target },
        plugins.phase_guard,
    );
    let current_turn_attack_setup =
        current_turn_attack_setup_score(combat, card_index, card, effects);
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
    let visible_damage = visible_incoming_damage(combat);
    let current_block = combat.entities.player.block;
    let current_hp = combat.entities.player.current_hp;
    let visible_loss_now = (visible_damage - current_block).max(0);
    let visible_loss_after_block =
        (visible_damage - current_block - block - effects.direct.visible_attack_mitigation_hint)
            .max(0)
            .saturating_add(effects.reactive.player_hp_loss);
    let prevents_visible_lethal =
        visible_loss_now >= current_hp && visible_loss_after_block < current_hp;
    let prevents_hp_loss = visible_loss_after_block < visible_loss_now;
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
        role_rank: role_rank
            .saturating_add(phase_hint.role_rank_adjustment)
            .saturating_add(resource_timing.role_rank_adjustment),
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
        resource_timing: resource_timing.ordering_score,
        phase_hint,
        effects: effect_diagnostics,
        ..ActionOrderingPriority::neutral(role)
    }
}
