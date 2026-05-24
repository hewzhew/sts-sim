use super::*;

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(super) struct CombatSearchRolloutValueV1 {
    pub(super) evaluated: i32,
    pub(super) terminal_rank: i32,
    pub(super) final_hp: i32,
    pub(super) enemy_progress: i32,
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
        survival_margin: estimate.survival_margin,
        potion_conservation: estimate.potion_conservation(),
        faster_turns: estimate.faster_turns(),
        fewer_cards_played: estimate.fewer_cards_played(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollout_priority_prefers_evaluated_terminal_win() {
        let unresolved = RolloutNodeEstimate::unevaluated();
        let mut win = RolloutNodeEstimate::unevaluated();
        win.evaluated = true;
        win.terminal = SearchTerminalLabel::Win;
        win.final_hp = 3;

        assert!(rollout_priority_value(win) > rollout_priority_value(unresolved));
    }

    #[test]
    fn rollout_priority_prefers_higher_hp_after_terminal_rank() {
        let low = terminal_win_with_hp(10);
        let high = terminal_win_with_hp(20);

        assert!(rollout_priority_value(high) > rollout_priority_value(low));
    }

    #[test]
    fn rollout_priority_uses_phase_adjusted_enemy_effort_for_unresolved_states() {
        let mut lower_effort = RolloutNodeEstimate::unevaluated();
        lower_effort.evaluated = true;
        lower_effort.terminal = SearchTerminalLabel::Unresolved;
        lower_effort.final_hp = 40;
        lower_effort.phase_adjusted_enemy_effort = 30;

        let mut higher_effort = lower_effort;
        higher_effort.phase_adjusted_enemy_effort = 50;

        assert!(rollout_priority_value(lower_effort) > rollout_priority_value(higher_effort));
    }

    fn terminal_win_with_hp(final_hp: i32) -> RolloutNodeEstimate {
        let mut estimate = RolloutNodeEstimate::unevaluated();
        estimate.evaluated = true;
        estimate.terminal = SearchTerminalLabel::Win;
        estimate.final_hp = final_hp;
        estimate
    }
}
