use serde::Serialize;
use sts_simulator::ai::combat_search_v2::CombatSearchV2WitnessLine;
use sts_simulator::sim::combat::CombatTerminal;

use super::super::quality_lanes::CombatLineQuality;
use super::super::search_types::SearchReview;

#[derive(Serialize)]
pub(crate) struct CounterfactualHpProbe {
    pub(super) schema: &'static str,
    pub(super) contract: &'static str,
    pub(super) original_hp: i32,
    pub(super) max_hp: i32,
    pub(super) levels: Vec<CounterfactualHpLevel>,
    pub(super) classification: CounterfactualHpClassification,
}

#[derive(Serialize)]
pub(super) struct CounterfactualHpLevel {
    pub(super) label: String,
    pub(super) hp: i32,
    pub(super) selected_lane: Option<&'static str>,
    pub(super) complete_win: bool,
    pub(super) quality: Option<CombatLineQuality>,
    pub(super) nodes_to_first_win: Option<u64>,
    pub(super) total_terminal_wins: u64,
    pub(super) replay_on_original_hp: Option<CounterfactualHpReplay>,
}

#[derive(Serialize)]
pub(super) struct CounterfactualHpReplay {
    pub(super) terminal: CombatTerminal,
    pub(super) final_hp: i32,
    pub(super) total_enemy_hp: i32,
    pub(super) living_enemy_count: usize,
    pub(super) replayed_actions: usize,
    pub(super) action_count: Option<usize>,
}

#[derive(Clone, Copy, Serialize)]
#[serde(rename_all = "snake_case")]
pub(super) enum CounterfactualHpClassification {
    OriginalHpWin,
    CounterfactualLineStillWinsOriginalHp,
    CounterfactualOnlyWin,
    NoWinFound,
}

pub(super) struct CounterfactualHpCandidate {
    pub(super) lane: &'static str,
    pub(super) review: SearchReview,
    pub(super) quality: CombatLineQuality,
    pub(super) witness: CombatSearchV2WitnessLine,
}
