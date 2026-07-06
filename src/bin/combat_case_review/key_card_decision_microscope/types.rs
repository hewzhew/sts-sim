use serde::Serialize;
use sts_simulator::ai::combat_search_v2::CombatSearchV2DecisionMicroscopeReport;

#[derive(Serialize)]
pub(crate) struct KeyCardDecisionMicroscopeProbe {
    pub(super) schema: &'static str,
    pub(super) contract: &'static str,
    pub(super) skipped_reason: Option<&'static str>,
    pub(super) variants: Vec<KeyCardDecisionMicroscopeVariant>,
}

#[derive(Serialize)]
pub(super) struct KeyCardDecisionMicroscopeVariant {
    pub(super) card: String,
    pub(super) uuid: u32,
    pub(super) reason: &'static str,
    pub(super) placement: &'static str,
    pub(super) skipped_reason: Option<&'static str>,
    pub(super) target_candidate: Option<KeyCardDecisionTargetCandidate>,
    pub(super) candidates_before_target: Vec<KeyCardDecisionCandidateDigest>,
    pub(super) selected_candidate: Option<KeyCardDecisionCandidateDigest>,
    pub(super) microscope: Option<CombatSearchV2DecisionMicroscopeReport>,
}

#[derive(Serialize)]
pub(super) struct KeyCardDecisionTargetCandidate {
    pub(super) ordered_index: usize,
    pub(super) action_key: String,
    pub(super) action_role: &'static str,
    pub(super) selected_by_best_complete: bool,
    pub(super) one_step_status: &'static str,
    pub(super) one_step_terminal: String,
    pub(super) visible_hp_loss_if_turn_ends: i32,
    pub(super) survival_margin: i32,
    pub(super) total_enemy_hp: i32,
}

#[derive(Serialize)]
pub(super) struct KeyCardDecisionCandidateDigest {
    pub(super) ordered_index: usize,
    pub(super) action_key: String,
    pub(super) action_role: &'static str,
    pub(super) selected_by_best_complete: bool,
    pub(super) one_step_status: &'static str,
    pub(super) visible_hp_loss_if_turn_ends: i32,
    pub(super) survival_margin: i32,
    pub(super) total_enemy_hp: i32,
}
