use std::time::{Duration, Instant};

use super::Args;

const WALL_STOP_GUARD_MS: u64 = 1_500;

#[derive(Clone, Copy)]
pub(super) struct RunDeadline(Option<Instant>);

impl RunDeadline {
    pub(super) fn new(started: Instant, wall_ms: Option<u64>) -> Self {
        Self(wall_ms.map(|ms| started + Duration::from_millis(ms)))
    }

    pub(super) fn should_stop(self) -> bool {
        self.remaining_ms()
            .is_some_and(|remaining| remaining <= WALL_STOP_GUARD_MS)
    }

    pub(super) fn should_soft_stop(self, args: Args) -> bool {
        self.remaining_ms()
            .is_some_and(|remaining| remaining <= self.soft_stop_guard_ms(args))
    }

    pub(super) fn would_cap_core_search(self, args: Args, child_count: usize) -> bool {
        self.search_wall_ms_per_child(child_count)
            .is_some_and(|available| available < args.search_ms)
    }

    fn soft_stop_guard_ms(self, args: Args) -> u64 {
        WALL_STOP_GUARD_MS.saturating_add(args.search_ms)
    }

    pub(super) fn cap_args(self, args: Args, child_count: usize) -> Args {
        let Some(per_child) = self.search_wall_ms_per_child(child_count) else {
            return args;
        };
        cap_search_wall_budget(args, per_child)
    }

    fn search_wall_ms_per_child(self, child_count: usize) -> Option<u64> {
        self.remaining_ms().map(|remaining| {
            remaining.saturating_sub(WALL_STOP_GUARD_MS) / child_count.max(1) as u64
        })
    }

    pub(super) fn remaining_ms(self) -> Option<u64> {
        self.0
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            .map(|remaining| remaining.as_millis().min(u128::from(u64::MAX)) as u64)
    }
}

fn cap_search_wall_budget(mut args: Args, available_ms: u64) -> Args {
    // A search session consumes one ordered work plan. Preserve the initial
    // quantum first, then give refinement only the wall time that remains.
    // Capping every field independently would authorize initial+refinement
    // twice against the same outer allowance.
    let search_ms = args.search_ms.min(available_ms);
    let refinement_ms = available_ms.saturating_sub(search_ms);
    let rescue_search_ms = args.rescue_search_ms.min(refinement_ms);
    let boss_search_ms = args.boss_search_ms.min(refinement_ms);
    if search_ms != args.search_ms || rescue_search_ms != args.rescue_search_ms {
        args.wall_capped_search_budget = true;
    }
    if boss_search_ms != args.boss_search_ms {
        args.wall_capped_boss_budget = true;
    }
    args.search_ms = search_ms;
    args.rescue_search_ms = rescue_search_ms;
    args.boss_search_ms = boss_search_ms;
    args
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::branch::RunObjective;

    fn args() -> Args {
        Args {
            seed: 1,
            ascension: 0,
            objective: RunObjective::FirstVictory,
            generations: 1,
            max_branches: 1,
            auto_ops: 1,
            search_nodes: 10,
            search_ms: 1_000,
            rescue_search_nodes: 20,
            rescue_search_ms: 5_000,
            boss_search_nodes: 30,
            boss_search_ms: 20_000,
            wall_ms: None,
            checkpoint_before_combat_portfolio: false,
            wall_capped_search_budget: false,
            wall_capped_boss_budget: false,
        }
    }

    #[test]
    fn wall_allowance_is_spent_on_an_ordered_quantum_prefix() {
        let capped = cap_search_wall_budget(args(), 4_000);

        assert_eq!(capped.search_ms, 1_000);
        assert_eq!(capped.rescue_search_ms, 3_000);
        assert_eq!(capped.boss_search_ms, 3_000);
        assert_eq!(capped.search_ms + capped.rescue_search_ms, 4_000);
        assert_eq!(capped.search_ms + capped.boss_search_ms, 4_000);
        assert!(capped.wall_capped_search_budget);
        assert!(capped.wall_capped_boss_budget);
    }

    #[test]
    fn refinement_gets_no_budget_when_only_initial_fits() {
        let capped = cap_search_wall_budget(args(), 1_000);

        assert_eq!(capped.search_ms, 1_000);
        assert_eq!(capped.rescue_search_ms, 0);
        assert_eq!(capped.boss_search_ms, 0);
    }
}
