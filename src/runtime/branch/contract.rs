use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum RunObjective {
    FirstVictory,
    FirstTerminal,
    ExhaustFrontier,
}

#[derive(Clone, Copy, Deserialize, Serialize)]
pub struct Args {
    pub seed: u64,
    pub ascension: u8,
    #[serde(default = "default_run_objective")]
    pub objective: RunObjective,
    pub generations: usize,
    pub max_branches: usize,
    pub auto_ops: usize,
    pub search_nodes: usize,
    pub search_ms: u64,
    pub rescue_search_nodes: usize,
    pub rescue_search_ms: u64,
    pub boss_search_nodes: usize,
    pub boss_search_ms: u64,
    pub wall_ms: Option<u64>,
    #[serde(skip)]
    pub checkpoint_before_combat_portfolio: bool,
    #[serde(skip)]
    pub wall_capped_search_budget: bool,
    #[serde(skip)]
    pub wall_capped_boss_budget: bool,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RunContract {
    pub game: GameRunContract,
    pub objective: RunObjective,
    pub branching: BranchingContract,
    pub automation: AutomationContract,
    pub combat_search: CombatSearchContract,
    pub slice: SliceContract,
    pub features: RuntimeFeatureContract,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct GameRunContract {
    pub seed: u64,
    pub ascension: u8,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BranchingContract {
    pub generations: usize,
    pub max_branches: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct AutomationContract {
    pub auto_ops: usize,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CombatSearchContract {
    pub primary_nodes: usize,
    pub primary_ms: u64,
    pub rescue_nodes: usize,
    pub rescue_ms: u64,
    pub boss_nodes: usize,
    pub boss_ms: u64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct SliceContract {
    pub slice_ms: Option<u64>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct RuntimeFeatureContract {
    pub checkpoint_before_combat_portfolio: bool,
}

impl RunObjective {
    pub fn parse(value: &str) -> Result<Self, String> {
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
    pub fn from_args(args: Args) -> Self {
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

pub fn default_run_objective() -> RunObjective {
    RunObjective::FirstVictory
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample_args() -> Args {
        Args {
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
