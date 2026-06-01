use super::*;
use crate::content::cards;
#[cfg(test)]
use crate::content::powers::PowerId;
use crate::runtime::combat::CombatCard;

mod damage_projection;
mod projection;
mod transition_rules;
use projection::{PhaseProjection, ProjectedMonsterDamage};
use transition_rules::{
    observe_guardian_transition, observe_lagavulin_transition, observe_split_transition,
    GUARDIAN_MODE_SHIFT_TRIGGER_RISK, LAGAVULIN_WAKE_RISK, SPLIT_TRIGGER_RISK_PER_DEBT_HP,
};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct EnemyPhaseTransitionHint {
    pub(super) split_trigger_count: usize,
    pub(super) split_debt_hp: i32,
    pub(super) guardian_mode_shift_trigger_count: usize,
    pub(super) guardian_min_threshold_remaining_before_hit: Option<i32>,
    pub(super) lagavulin_wake_risk_count: usize,
}

pub(super) fn enemy_phase_transition_hint_for_input(
    combat: &CombatState,
    input: &ClientInput,
) -> EnemyPhaseTransitionHint {
    match input {
        ClientInput::PlayCard { card_index, target } => combat
            .zones
            .hand
            .get(*card_index)
            .map(|card| play_card_phase_transition_hint(combat, card, *target))
            .unwrap_or_default(),
        _ => EnemyPhaseTransitionHint::default(),
    }
}

impl EnemyPhaseTransitionHint {
    pub(super) fn ordering_risk_score(self) -> i32 {
        self.split_debt_hp
            .saturating_mul(SPLIT_TRIGGER_RISK_PER_DEBT_HP)
            .saturating_add(
                (self.guardian_mode_shift_trigger_count as i32)
                    .saturating_mul(GUARDIAN_MODE_SHIFT_TRIGGER_RISK),
            )
            .saturating_add(
                (self.lagavulin_wake_risk_count as i32).saturating_mul(LAGAVULIN_WAKE_RISK),
            )
    }
}

fn play_card_phase_transition_hint(
    combat: &CombatState,
    card: &CombatCard,
    target: Option<usize>,
) -> EnemyPhaseTransitionHint {
    let mut hint = EnemyPhaseTransitionHint::default();
    let evaluated = cards::evaluate_card_for_play(card, combat, target);
    let actions = cards::resolve_card_play_with_context(
        evaluated.id,
        combat,
        &evaluated,
        target,
        cards::CardUseContext {
            played_from_hand: true,
        },
    );
    let mut projection = PhaseProjection::from_combat(combat);

    damage_projection::observe_actions_damage(
        &mut projection,
        actions.into_iter().map(|action| action.action),
    );

    for projected in projection.monsters.values() {
        observe_split_transition(&mut hint, projected);
        observe_guardian_transition(&mut hint, projected);
        observe_lagavulin_transition(&mut hint, projected);
    }

    hint
}

#[cfg(test)]
mod tests;
