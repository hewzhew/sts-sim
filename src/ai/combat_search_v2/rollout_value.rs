use super::*;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct CombatSearchRolloutValueV1 {
    pub(super) evaluated: i32,
    pub(super) terminal_rank: i32,
    pub(super) final_hp: i32,
    pub(super) enemy_progress: i32,
    pub(super) special_enemy_phase_progress: i32,
    pub(super) guardian_mode_shift_stability: i32,
    pub(super) lagavulin_wake_stability: i32,
    pub(super) gremlin_nob_enrage_stability: i32,
    pub(super) sentry_dazed_stability: i32,
    pub(super) hexaghost_opening_stability: i32,
    pub(super) pending_choice_fanout: i32,
    pub(super) survival_margin: i32,
    pub(super) potion_conservation: i32,
    pub(super) faster_turns: i32,
    pub(super) fewer_cards_played: i32,
}

impl Ord for CombatSearchRolloutValueV1 {
    fn cmp(&self, other: &Self) -> Ordering {
        self.evaluated
            .cmp(&other.evaluated)
            .then_with(|| self.terminal_rank.cmp(&other.terminal_rank))
            .then_with(|| self.final_hp.cmp(&other.final_hp))
            .then_with(|| self.enemy_progress.cmp(&other.enemy_progress))
            .then_with(|| {
                self.special_enemy_phase_progress
                    .cmp(&other.special_enemy_phase_progress)
            })
            .then_with(|| {
                self.guardian_mode_shift_stability
                    .cmp(&other.guardian_mode_shift_stability)
            })
            .then_with(|| {
                self.lagavulin_wake_stability
                    .cmp(&other.lagavulin_wake_stability)
            })
            .then_with(|| {
                self.gremlin_nob_enrage_stability
                    .cmp(&other.gremlin_nob_enrage_stability)
            })
            .then_with(|| {
                self.sentry_dazed_stability
                    .cmp(&other.sentry_dazed_stability)
            })
            .then_with(|| {
                self.hexaghost_opening_stability
                    .cmp(&other.hexaghost_opening_stability)
            })
            .then_with(|| self.pending_choice_fanout.cmp(&other.pending_choice_fanout))
            .then_with(|| self.survival_margin.cmp(&other.survival_margin))
            .then_with(|| self.potion_conservation.cmp(&other.potion_conservation))
            .then_with(|| self.faster_turns.cmp(&other.faster_turns))
            .then_with(|| self.fewer_cards_played.cmp(&other.fewer_cards_played))
    }
}

impl PartialOrd for CombatSearchRolloutValueV1 {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

pub(super) fn rollout_priority_value(estimate: RolloutNodeEstimate) -> CombatSearchRolloutValueV1 {
    CombatSearchRolloutValueV1 {
        evaluated: i32::from(estimate.evaluated),
        terminal_rank: estimate.priority_terminal_rank(),
        final_hp: estimate.final_hp,
        enemy_progress: estimate.enemy_progress(),
        special_enemy_phase_progress: -(estimate.special_enemy_phase_count as i32),
        guardian_mode_shift_stability: -(estimate.guardian_mode_shift_pending_count as i32),
        lagavulin_wake_stability: -(estimate.lagavulin_waking_count as i32),
        gremlin_nob_enrage_stability: -estimate.gremlin_nob_anger_amount_total,
        sentry_dazed_stability: -(estimate.sentry_dazed_pressure_count as i32),
        hexaghost_opening_stability: -(estimate.hexaghost_opening_pressure_count as i32),
        pending_choice_fanout: -(estimate.pending_choice_estimated_action_fanout as i32),
        survival_margin: estimate.survival_margin,
        potion_conservation: estimate.potion_conservation(),
        faster_turns: estimate.faster_turns(),
        fewer_cards_played: estimate.fewer_cards_played(),
    }
}

#[cfg(test)]
mod tests;
