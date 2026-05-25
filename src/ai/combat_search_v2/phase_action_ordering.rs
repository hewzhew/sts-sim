use super::enemy_phase_transition::EnemyPhaseTransitionHint;
use super::phase_profile::CombatSearchPhaseProfileV1;
use crate::content::cards::CardType;

// Kept smaller than the main role gaps in action_priority; phase facts nudge nearby
// ordering decisions without turning this module into an alternate policy.
const PHASE_ROLE_ADJUSTMENT: i32 = 12;

#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct PhaseActionOrderingFacts {
    pub(super) card_type: CardType,
    pub(super) block: i32,
    pub(super) mitigation: i32,
    pub(super) phase_transition: EnemyPhaseTransitionHint,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct PhaseActionOrderingHint {
    pub(super) role_rank_adjustment: i32,
    pub(super) phase_setup: i32,
    pub(super) phase_survival: i32,
    pub(super) phase_transition_safety: i32,
}

pub(super) fn phase_action_ordering_hint(
    profile: CombatSearchPhaseProfileV1,
    facts: PhaseActionOrderingFacts,
) -> PhaseActionOrderingHint {
    let mut hint = PhaseActionOrderingHint {
        phase_transition_safety: -facts.phase_transition.ordering_risk_score(),
        ..PhaseActionOrderingHint::default()
    };

    if profile.enemy_mechanics.lagavulin_sleeping_count > 0 {
        apply_lagavulin_sleep_hint(&mut hint, facts);
    }
    if profile.enemy_mechanics.guardian_defensive_count > 0
        || profile.enemy_mechanics.guardian_mode_shift_pending_count > 0
    {
        hint.phase_survival = facts.block.saturating_add(facts.mitigation);
    }

    hint
}

impl PhaseActionOrderingHint {
    pub(super) fn has_signal(self) -> bool {
        self.role_rank_adjustment != 0
            || self.phase_setup != 0
            || self.phase_survival != 0
            || self.phase_transition_safety != 0
    }
}

fn apply_lagavulin_sleep_hint(hint: &mut PhaseActionOrderingHint, facts: PhaseActionOrderingFacts) {
    if facts.phase_transition.lagavulin_wake_risk_count > 0 {
        hint.role_rank_adjustment = hint
            .role_rank_adjustment
            .saturating_sub(PHASE_ROLE_ADJUSTMENT);
        hint.phase_setup -= 1;
    } else if facts.card_type == CardType::Power {
        hint.role_rank_adjustment = hint
            .role_rank_adjustment
            .saturating_add(PHASE_ROLE_ADJUSTMENT);
        hint.phase_setup += 1;
    }
}

#[cfg(test)]
mod tests;
