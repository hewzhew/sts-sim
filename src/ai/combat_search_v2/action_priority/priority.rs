use std::cmp::Ordering;

use super::super::action_effects::CardPlayEffectDiagnostics;
use super::super::phase_action_ordering::PhaseActionOrderingHint;
use super::constants::ROLE_END_TURN;
use super::role::ActionOrderingRole;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(in crate::ai::combat_search_v2) struct ActionOrderingPriority {
    pub(in crate::ai::combat_search_v2) role: ActionOrderingRole,
    pub(in crate::ai::combat_search_v2) role_rank: i32,
    pub(in crate::ai::combat_search_v2) potion_tactical_rank: i32,
    pub(in crate::ai::combat_search_v2) mitigation: i32,
    pub(in crate::ai::combat_search_v2) reactive_risk: i32,
    pub(in crate::ai::combat_search_v2) target_progress: i32,
    pub(in crate::ai::combat_search_v2) block: i32,
    pub(in crate::ai::combat_search_v2) damage: i32,
    pub(in crate::ai::combat_search_v2) cheaper_cost: i32,
    pub(in crate::ai::combat_search_v2) phase_setup: i32,
    pub(in crate::ai::combat_search_v2) phase_survival: i32,
    pub(in crate::ai::combat_search_v2) phase_transition_safety: i32,
    pub(in crate::ai::combat_search_v2) pending_choice_primary: i32,
    pub(in crate::ai::combat_search_v2) pending_choice_secondary: i32,
    pub(in crate::ai::combat_search_v2) pending_choice_selected_count: i32,
    pub(in crate::ai::combat_search_v2) phase_hint: PhaseActionOrderingHint,
    pub(in crate::ai::combat_search_v2) effects: CardPlayEffectDiagnostics,
}

impl ActionOrderingPriority {
    pub(super) fn neutral(role: ActionOrderingRole) -> Self {
        Self {
            role,
            role_rank: ROLE_END_TURN,
            potion_tactical_rank: 0,
            mitigation: 0,
            reactive_risk: 0,
            target_progress: 0,
            block: 0,
            damage: 0,
            cheaper_cost: 0,
            phase_setup: 0,
            phase_survival: 0,
            phase_transition_safety: 0,
            pending_choice_primary: 0,
            pending_choice_secondary: 0,
            pending_choice_selected_count: 0,
            phase_hint: PhaseActionOrderingHint::default(),
            effects: CardPlayEffectDiagnostics::default(),
        }
    }
}

impl Ord for ActionOrderingPriority {
    fn cmp(&self, other: &Self) -> Ordering {
        self.role_rank
            .cmp(&other.role_rank)
            .then_with(|| self.potion_tactical_rank.cmp(&other.potion_tactical_rank))
            .then_with(|| self.mitigation.cmp(&other.mitigation))
            .then_with(|| self.reactive_risk.cmp(&other.reactive_risk))
            .then_with(|| self.phase_setup.cmp(&other.phase_setup))
            .then_with(|| self.phase_survival.cmp(&other.phase_survival))
            .then_with(|| {
                self.phase_transition_safety
                    .cmp(&other.phase_transition_safety)
            })
            .then_with(|| self.target_progress.cmp(&other.target_progress))
            .then_with(|| self.block.cmp(&other.block))
            .then_with(|| self.damage.cmp(&other.damage))
            .then_with(|| self.cheaper_cost.cmp(&other.cheaper_cost))
            .then_with(|| {
                self.pending_choice_primary
                    .cmp(&other.pending_choice_primary)
            })
            .then_with(|| {
                self.pending_choice_secondary
                    .cmp(&other.pending_choice_secondary)
            })
            .then_with(|| {
                self.pending_choice_selected_count
                    .cmp(&other.pending_choice_selected_count)
            })
    }
}

impl PartialOrd for ActionOrderingPriority {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}
