use std::path::PathBuf;

use crate::ai::combat_search_v2::{CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy};
use crate::state::core::ClientInput;

use super::reward_auto::RewardAutomationTarget;

mod help;
mod options;
mod parse;

#[cfg(test)]
mod tests;

pub use help::{run_control_help, run_control_short_hint};
pub use parse::parse_run_control_command;

#[derive(Clone, Debug, PartialEq)]
pub enum RunControlCommand {
    Noop,
    DefaultCandidate,
    Candidate(String),
    Help,
    Quit,
    Main,
    Deck,
    Map,
    MapFull,
    MapSummary,
    RouteSuggest,
    RouteGo,
    Relics,
    Potions,
    Draw,
    Discard,
    Exhaust,
    Inspect(String),
    SaveDecisionCase {
        path: Option<PathBuf>,
    },
    Details,
    Raw,
    Actions,
    Capture {
        path: PathBuf,
        label: Option<String>,
    },
    CaptureCase {
        root: PathBuf,
        case_id: String,
        label: Option<String>,
    },
    CaptureCaseDefault {
        case_id: String,
        label: Option<String>,
    },
    SaveBaseline {
        path: PathBuf,
        case_id: Option<String>,
    },
    SaveBaselineCase {
        root: PathBuf,
        case_id: String,
    },
    SaveBaselineForLastCaptureCase,
    RegisterBenchmarkCase {
        root: PathBuf,
        case_id: String,
    },
    SearchCombat(RunControlSearchCombatOptions),
    AutoStep(RunControlAutoStepOptions),
    RewardAutomationStatus,
    SetRewardAutomation {
        target: RewardAutomationTarget,
        enabled: bool,
    },
    CardIndex(usize),
    RelicIndex(usize),
    SelectionIndices(Vec<usize>),
    ActionIndex(usize),
    PlayCard {
        card_index: usize,
        target_slot_or_id: Option<usize>,
    },
    UsePotion {
        potion_index: usize,
        target_slot_or_id: Option<usize>,
    },
    Input(ClientInput),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunControlSearchCombatOptions {
    pub max_nodes: Option<usize>,
    pub max_actions_per_line: Option<usize>,
    pub max_engine_steps_per_action: Option<usize>,
    pub wall_ms: Option<u64>,
    pub potion_policy: Option<CombatSearchV2PotionPolicy>,
    pub max_potions_used: Option<u32>,
    pub rollout_policy: Option<CombatSearchV2RolloutPolicy>,
    pub rollout_max_evaluations: Option<usize>,
    pub rollout_max_actions: Option<usize>,
    pub evidence: Option<RunControlSearchEvidenceTarget>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum RunControlSearchEvidenceTarget {
    LastCaptureCase,
    Path(PathBuf),
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RunControlAutoStepOptions {
    pub search: RunControlSearchCombatOptions,
    pub max_operations: Option<usize>,
    pub route: RunControlRouteAutomationMode,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum RunControlRouteAutomationMode {
    #[default]
    Manual,
    Planner,
}
