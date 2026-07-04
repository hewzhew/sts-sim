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
        self.cap_args(args, child_count).wall_capped_search_budget
    }

    fn soft_stop_guard_ms(self, args: Args) -> u64 {
        WALL_STOP_GUARD_MS + args.search_ms.max(args.rescue_search_ms)
    }

    pub(super) fn cap_args(self, mut args: Args, child_count: usize) -> Args {
        let Some(remaining) = self.remaining_ms() else {
            return args;
        };
        let per_child =
            (remaining.saturating_sub(WALL_STOP_GUARD_MS) / child_count.max(1) as u64).max(1);
        let search_ms = args.search_ms.min(per_child);
        let rescue_search_ms = args.rescue_search_ms.min(per_child);
        let boss_search_ms = args.boss_search_ms.min(per_child);
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

    pub(super) fn remaining_ms(self) -> Option<u64> {
        self.0
            .map(|deadline| deadline.saturating_duration_since(Instant::now()))
            .map(|remaining| remaining.as_millis().min(u128::from(u64::MAX)) as u64)
    }
}
