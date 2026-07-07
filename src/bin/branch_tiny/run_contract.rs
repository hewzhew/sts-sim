use serde::{Deserialize, Serialize};

use super::{BranchStatus, TerminalOutcome};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum RunObjective {
    FirstVictory,
    FirstTerminal,
    ExhaustFrontier,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct RunContract {
    pub(super) game: GameRunContract,
    pub(super) objective: RunObjective,
    pub(super) branching: BranchingContract,
    pub(super) automation: AutomationContract,
    pub(super) combat_search: CombatSearchContract,
    pub(super) slice: SliceContract,
    pub(super) features: RuntimeFeatureContract,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct GameRunContract {
    pub(super) seed: u64,
    pub(super) ascension: u8,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct BranchingContract {
    pub(super) generations: usize,
    pub(super) max_branches: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct AutomationContract {
    pub(super) auto_ops: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct CombatSearchContract {
    pub(super) primary_nodes: usize,
    pub(super) primary_ms: u64,
    pub(super) rescue_nodes: usize,
    pub(super) rescue_ms: u64,
    pub(super) boss_nodes: usize,
    pub(super) boss_ms: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct SliceContract {
    pub(super) slice_ms: Option<u64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub(super) struct RuntimeFeatureContract {
    pub(super) checkpoint_before_combat_portfolio: bool,
}

#[derive(Clone, Copy)]
pub(super) struct CompletionReason(&'static str);

impl CompletionReason {
    pub(super) fn as_str(self) -> &'static str {
        self.0
    }
}

impl RunObjective {
    pub(super) fn parse(value: &str) -> Result<Self, String> {
        match value {
            "first-victory" | "first_victory" => Ok(Self::FirstVictory),
            "first-terminal" | "first_terminal" => Ok(Self::FirstTerminal),
            "exhaust-frontier" | "exhaust_frontier" => Ok(Self::ExhaustFrontier),
            _ => Err(format!(
                "invalid value for --objective: {value}; expected first-victory, first-terminal, or exhaust-frontier"
            )),
        }
    }
}

impl RunContract {
    pub(super) fn from_args(args: super::Args) -> Self {
        Self {
            game: GameRunContract {
                seed: args.seed,
                ascension: args.ascension,
            },
            objective: args.objective,
            branching: BranchingContract {
                generations: args.generations,
                max_branches: args.max_branches,
            },
            automation: AutomationContract {
                auto_ops: args.auto_ops,
            },
            combat_search: CombatSearchContract {
                primary_nodes: args.search_nodes,
                primary_ms: args.search_ms,
                rescue_nodes: args.rescue_search_nodes,
                rescue_ms: args.rescue_search_ms,
                boss_nodes: args.boss_search_nodes,
                boss_ms: args.boss_search_ms,
            },
            slice: SliceContract {
                slice_ms: args.wall_ms,
            },
            features: RuntimeFeatureContract {
                checkpoint_before_combat_portfolio: args.checkpoint_before_combat_portfolio,
            },
        }
    }
}

pub(super) fn default_run_objective() -> RunObjective {
    RunObjective::FirstVictory
}

pub(super) fn satisfied(
    objective: RunObjective,
    status: &BranchStatus,
) -> Option<CompletionReason> {
    match (objective, status) {
        (RunObjective::FirstVictory, BranchStatus::Terminal(TerminalOutcome::Victory)) => {
            Some(CompletionReason("victory_found"))
        }
        (RunObjective::FirstTerminal, BranchStatus::Terminal(_)) => {
            Some(CompletionReason("terminal_found"))
        }
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_args() -> super::super::Args {
        super::super::Args {
            seed: 123,
            ascension: 7,
            objective: RunObjective::ExhaustFrontier,
            generations: 11,
            max_branches: 3,
            auto_ops: 22,
            search_nodes: 101,
            search_ms: 202,
            rescue_search_nodes: 303,
            rescue_search_ms: 404,
            boss_search_nodes: 505,
            boss_search_ms: 606,
            wall_ms: Some(707),
            checkpoint_before_combat_portfolio: true,
            wall_capped_search_budget: true,
            wall_capped_boss_budget: true,
        }
    }

    #[test]
    fn run_contract_from_args_preserves_stable_runtime_fields() {
        let contract = RunContract::from_args(sample_args());

        assert_eq!(contract.game.seed, 123);
        assert_eq!(contract.game.ascension, 7);
        assert_eq!(contract.objective, RunObjective::ExhaustFrontier);
        assert_eq!(contract.branching.generations, 11);
        assert_eq!(contract.branching.max_branches, 3);
        assert_eq!(contract.automation.auto_ops, 22);
        assert_eq!(contract.combat_search.primary_nodes, 101);
        assert_eq!(contract.combat_search.primary_ms, 202);
        assert_eq!(contract.combat_search.rescue_nodes, 303);
        assert_eq!(contract.combat_search.rescue_ms, 404);
        assert_eq!(contract.combat_search.boss_nodes, 505);
        assert_eq!(contract.combat_search.boss_ms, 606);
        assert_eq!(contract.slice.slice_ms, Some(707));
        assert!(contract.features.checkpoint_before_combat_portfolio);
    }

    #[test]
    fn run_contract_does_not_encode_per_slice_wall_cap_flags() {
        let contract = RunContract::from_args(sample_args());
        let value = serde_json::to_value(contract).unwrap();

        assert!(value.get("wall_capped_search_budget").is_none());
        assert!(value.get("wall_capped_boss_budget").is_none());
    }
}
