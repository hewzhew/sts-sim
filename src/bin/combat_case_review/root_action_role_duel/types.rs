use serde::Serialize;
use sts_simulator::ai::combat_search_v2::{
    CombatSearchV2DecisionMicroscopeReport, CombatSearchV2StateSummary,
};
use sts_simulator::sim::combat::CombatTerminal;
use sts_simulator::state::core::ClientInput;

use super::super::focus::CombatReviewFocus;
use super::super::key_card_lifecycle::KeyCardLifecycleReport;
use super::super::search_types::SearchReview;

#[derive(Serialize)]
pub(crate) struct RootActionRoleDuelProbe {
    pub(super) schema: &'static str,
    pub(super) contract: &'static str,
    pub(super) skipped_reason: Option<&'static str>,
    pub(super) variants: Vec<RootActionRoleDuelVariant>,
}

#[derive(Serialize)]
pub(super) struct RootActionRoleDuelVariant {
    pub(super) basis: RootActionRoleDuelBasis,
    pub(super) skipped_reason: Option<&'static str>,
    pub(super) microscope: Option<CombatSearchV2DecisionMicroscopeReport>,
    pub(super) duels: Vec<RootActionRoleDuel>,
}

#[derive(Serialize)]
pub(super) struct RootActionRoleDuelBasis {
    pub(super) label: String,
    pub(super) moved_key_card: Option<RootActionRoleDuelKeyCard>,
}

#[derive(Serialize)]
pub(super) struct RootActionRoleDuelKeyCard {
    pub(super) card: String,
    pub(super) uuid: u32,
    pub(super) reason: &'static str,
    pub(super) placement: &'static str,
}

#[derive(Serialize)]
pub(super) struct RootActionRoleDuel {
    pub(super) selection_reasons: Vec<&'static str>,
    pub(super) root_candidate: RootActionRoleDuelCandidate,
    pub(super) root_transition: RootActionRoleDuelTransition,
    pub(super) child_search: Option<SearchReview>,
    pub(super) child_best_complete_final_state: Option<CombatSearchV2StateSummary>,
    pub(super) child_focus: Option<CombatReviewFocus>,
    pub(super) key_card_lifecycle_after_root: Option<KeyCardLifecycleReport>,
}

#[derive(Serialize)]
pub(super) struct RootActionRoleDuelCandidate {
    pub(super) ordered_index: usize,
    pub(super) action_key: String,
    pub(super) action_role: &'static str,
    pub(super) selected_by_best_complete: bool,
    pub(super) input: ClientInput,
}

#[derive(Serialize)]
pub(super) struct RootActionRoleDuelTransition {
    pub(super) status: &'static str,
    pub(super) terminal: CombatTerminal,
    pub(super) engine_steps: usize,
    pub(super) player_hp: i32,
    pub(super) player_block: i32,
    pub(super) energy: u8,
    pub(super) living_enemy_count: usize,
    pub(super) total_enemy_hp: i32,
    pub(super) cultists_alive: usize,
    pub(super) visible_incoming_damage: i32,
    pub(super) survival_margin: i32,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct DuelSelection {
    pub(super) candidate_index: usize,
    pub(super) reasons: Vec<&'static str>,
}
