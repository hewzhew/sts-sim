#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum CombatSearchCandidateTier {
    SurvivalFallback,
    RelaxedCompleteWin,
    ReserveCompliantCompleteWin,
}

impl CombatSearchCandidateTier {
    fn rank(self) -> u8 {
        match self {
            Self::SurvivalFallback => 0,
            Self::RelaxedCompleteWin => 1,
            Self::ReserveCompliantCompleteWin => 2,
        }
    }

    pub(super) fn as_str(self) -> &'static str {
        match self {
            Self::SurvivalFallback => "survival_fallback",
            Self::RelaxedCompleteWin => "relaxed_complete_win",
            Self::ReserveCompliantCompleteWin => "reserve_compliant_complete_win",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CombatSearchCandidateFacts {
    pub(super) terminal_run_victory: bool,
    pub(super) tier: CombatSearchCandidateTier,
    pub(super) combat_final_hp: i32,
    pub(super) run_hp: i32,
    pub(super) potions_used: u32,
    pub(super) potions_discarded: u32,
    pub(super) turns: u32,
    pub(super) action_count: usize,
}

impl CombatSearchCandidateFacts {
    fn strictly_dominates(self, other: Self) -> bool {
        self.run_hp >= other.run_hp
            && self.potions_used <= other.potions_used
            && self.potions_discarded <= other.potions_discarded
            && (self.run_hp > other.run_hp
                || self.potions_used < other.potions_used
                || self.potions_discarded < other.potions_discarded)
    }

    fn same_resource_facts(self, other: Self) -> bool {
        self.run_hp == other.run_hp
            && self.potions_used == other.potions_used
            && self.potions_discarded == other.potions_discarded
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct CombatSearchIncumbentDecision {
    pub(super) replaced: bool,
    pub(super) reason: &'static str,
}

pub(super) struct CombatSearchIncumbent {
    selected: Option<(usize, CombatSearchCandidateFacts)>,
}

impl CombatSearchIncumbent {
    pub(super) fn new() -> Self {
        Self { selected: None }
    }

    pub(super) fn offer(
        &mut self,
        attempt_index: usize,
        candidate: CombatSearchCandidateFacts,
    ) -> CombatSearchIncumbentDecision {
        let Some((_, incumbent)) = self.selected else {
            self.selected = Some((attempt_index, candidate));
            return CombatSearchIncumbentDecision {
                replaced: true,
                reason: "initial_incumbent",
            };
        };

        let decision = compare_candidate(candidate, incumbent);
        if decision.replaced {
            self.selected = Some((attempt_index, candidate));
        }
        decision
    }

    pub(super) fn selected_index(&self) -> Option<usize> {
        self.selected.map(|(index, _)| index)
    }
}

fn compare_candidate(
    candidate: CombatSearchCandidateFacts,
    incumbent: CombatSearchCandidateFacts,
) -> CombatSearchIncumbentDecision {
    if candidate.terminal_run_victory != incumbent.terminal_run_victory {
        return if candidate.terminal_run_victory {
            replace("terminal_run_victory")
        } else {
            preserve("incumbent_terminal_run_victory")
        };
    }
    if candidate.tier.rank() != incumbent.tier.rank() {
        return if candidate.tier.rank() > incumbent.tier.rank() {
            replace("higher_candidate_tier")
        } else {
            preserve("lower_candidate_tier")
        };
    }
    if candidate.strictly_dominates(incumbent) {
        return replace("strict_resource_dominance");
    }
    if incumbent.strictly_dominates(candidate) {
        return preserve("incumbent_resource_dominance");
    }
    if candidate.same_resource_facts(incumbent) {
        if candidate.turns < incumbent.turns {
            return replace("fewer_turns");
        }
        if candidate.turns > incumbent.turns {
            return preserve("incumbent_fewer_turns");
        }
        if candidate.action_count < incumbent.action_count {
            return replace("fewer_actions");
        }
        if candidate.action_count > incumbent.action_count {
            return preserve("incumbent_fewer_actions");
        }
        return preserve("equal_candidate");
    }
    preserve("incomparable_resource_trade")
}

fn replace(reason: &'static str) -> CombatSearchIncumbentDecision {
    CombatSearchIncumbentDecision {
        replaced: true,
        reason,
    }
}

fn preserve(reason: &'static str) -> CombatSearchIncumbentDecision {
    CombatSearchIncumbentDecision {
        replaced: false,
        reason,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn candidate(
        tier: CombatSearchCandidateTier,
        run_hp: i32,
        potions_used: u32,
    ) -> CombatSearchCandidateFacts {
        CombatSearchCandidateFacts {
            terminal_run_victory: false,
            tier,
            combat_final_hp: run_hp,
            run_hp,
            potions_used,
            potions_discarded: 0,
            turns: 5,
            action_count: 20,
        }
    }

    fn reserve_win(run_hp: i32, potions_used: u32) -> CombatSearchCandidateFacts {
        candidate(
            CombatSearchCandidateTier::ReserveCompliantCompleteWin,
            run_hp,
            potions_used,
        )
    }

    fn relaxed_win(run_hp: i32, potions_used: u32) -> CombatSearchCandidateFacts {
        candidate(
            CombatSearchCandidateTier::RelaxedCompleteWin,
            run_hp,
            potions_used,
        )
    }

    #[test]
    fn same_cost_higher_hp_replaces_incumbent() {
        let mut incumbent = CombatSearchIncumbent::new();
        incumbent.offer(0, reserve_win(38, 2));
        let decision = incumbent.offer(1, reserve_win(48, 2));

        assert!(decision.replaced);
        assert_eq!(decision.reason, "strict_resource_dominance");
        assert_eq!(incumbent.selected_index(), Some(1));
    }

    #[test]
    fn incomparable_resource_trade_preserves_incumbent() {
        let mut incumbent = CombatSearchIncumbent::new();
        incumbent.offer(0, reserve_win(38, 1));
        let decision = incumbent.offer(1, reserve_win(48, 2));

        assert!(!decision.replaced);
        assert_eq!(decision.reason, "incomparable_resource_trade");
        assert_eq!(incumbent.selected_index(), Some(0));
    }

    #[test]
    fn relaxed_win_cannot_replace_reserve_compliant_win() {
        let mut incumbent = CombatSearchIncumbent::new();
        incumbent.offer(0, reserve_win(25, 2));
        let decision = incumbent.offer(1, relaxed_win(50, 0));

        assert!(!decision.replaced);
        assert_eq!(decision.reason, "lower_candidate_tier");
        assert_eq!(incumbent.selected_index(), Some(0));
    }
}
