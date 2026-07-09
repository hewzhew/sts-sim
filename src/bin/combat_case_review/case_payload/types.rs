use serde::Serialize;
use sts_simulator::ai::combat_search_v2::{
    CombatDeficitEvidenceReport, CombatLineLabReport, CombatSearchV2WitnessReplay,
};
use sts_simulator::ai::strategy::deck_strategic_deficit::DeckStrategicDeficit;
use sts_simulator::eval::combat_case::{
    CombatCaseCardSummary, CombatCaseCombatSummary, CombatCaseGap, CombatCasePathStep,
    CombatCaseRunSummary, CombatCaseSource,
};
use sts_simulator::eval::run_control::CombatSearchTraceSummary;

use super::super::awakened_one_evidence::{
    AwakenedOneFailureEvidenceFrame, StaticBossMatchupAuditV0,
};
use super::super::boss_pressure_lens::BossPressureLensReport;
use super::super::boss_setup_lane::BossSetupLaneReview;
use super::super::champ_phase::ChampPhaseAudit;
use super::super::classification::CombatGapReviewClassification;
use super::super::counterfactual_hp::CounterfactualHpProbe;
use super::super::focus::{CombatReviewFocus, CombatReviewFocusPriorRerun};
use super::super::forced_potion_opening::ForcedPotionOpeningReview;
use super::super::frozen_panel_lanes::FrozenPanelLaneReview;
use super::super::key_card_counterfactual::KeyCardCounterfactualProbe;
use super::super::key_card_decision_microscope::KeyCardDecisionMicroscopeProbe;
use super::super::key_card_lifecycle::KeyCardLifecycleReport;
use super::super::quality_lanes::CombatQualityLaneReview;
use super::super::root_action_role_duel::RootActionRoleDuelProbe;
use super::super::search_types::SearchReview;
use super::super::strategic_feedback::CombatStrategicFeedbackReport;

#[derive(Serialize)]
pub(crate) struct CombatCaseReview {
    pub(super) schema: &'static str,
    pub(super) case_path: String,
    pub(super) source: CombatCaseSource,
    pub(super) gap: CombatCaseGap,
    pub(super) run: CombatCaseRunSummary,
    pub(super) combat: CombatCaseCombatSummary,
    pub(super) deck: Vec<CombatCaseCardSummary>,
    pub(super) static_strategic_deficit: DeckStrategicDeficit,
    pub(super) relics: Vec<String>,
    pub(super) potions: Vec<Option<String>>,
    pub(super) path_tail: Vec<CombatCasePathStep>,
    pub(super) saved_search: Option<CombatSearchTraceSummary>,
    pub(super) ladder: Vec<SearchReview>,
    pub(super) classification: CombatGapReviewClassification,
    pub(super) review_focus: Option<CombatReviewFocus>,
    pub(super) review_focus_replay: Option<CombatSearchV2WitnessReplay>,
    pub(super) review_focus_prior_rerun: Option<CombatReviewFocusPriorRerun>,
    pub(super) line_lab: Option<CombatLineLabReport>,
    pub(super) quality_lanes: Option<CombatQualityLaneReview>,
    pub(super) counterfactual_hp_probe: Option<CounterfactualHpProbe>,
    pub(super) combat_deficit_evidence: Option<CombatDeficitEvidenceReport>,
    pub(super) combat_strategic_feedback: Option<CombatStrategicFeedbackReport>,
    pub(super) static_boss_matchup_audit_v0: Option<StaticBossMatchupAuditV0>,
    pub(super) awakened_one_failure_evidence: Option<AwakenedOneFailureEvidenceFrame>,
    pub(super) boss_pressure_lens: Option<BossPressureLensReport>,
    pub(super) boss_setup_lane: Option<BossSetupLaneReview>,
    pub(super) frozen_panel_lanes: Option<FrozenPanelLaneReview>,
    pub(super) forced_potion_opening_lanes: Option<ForcedPotionOpeningReview>,
    pub(super) key_card_counterfactual: Option<KeyCardCounterfactualProbe>,
    pub(super) key_card_decision_microscope: Option<KeyCardDecisionMicroscopeProbe>,
    pub(super) root_action_role_duel: Option<RootActionRoleDuelProbe>,
    pub(super) champ_phase_audit: Option<ChampPhaseAudit>,
    pub(super) key_card_lifecycle: Option<KeyCardLifecycleReport>,
}

pub(crate) struct CombatCaseReviewArtifacts {
    pub(crate) ladder: Vec<SearchReview>,
    pub(crate) classification: CombatGapReviewClassification,
    pub(crate) review_focus: Option<CombatReviewFocus>,
    pub(crate) review_focus_replay: Option<CombatSearchV2WitnessReplay>,
    pub(crate) review_focus_prior_rerun: Option<CombatReviewFocusPriorRerun>,
    pub(crate) line_lab: Option<CombatLineLabReport>,
    pub(crate) quality_lanes: Option<CombatQualityLaneReview>,
    pub(crate) counterfactual_hp_probe: Option<CounterfactualHpProbe>,
    pub(crate) combat_deficit_evidence: Option<CombatDeficitEvidenceReport>,
    pub(crate) static_boss_matchup_audit_v0: Option<StaticBossMatchupAuditV0>,
    pub(crate) awakened_one_failure_evidence: Option<AwakenedOneFailureEvidenceFrame>,
    pub(crate) boss_pressure_lens: Option<BossPressureLensReport>,
    pub(crate) boss_setup_lane: Option<BossSetupLaneReview>,
    pub(crate) frozen_panel_lanes: Option<FrozenPanelLaneReview>,
    pub(crate) forced_potion_opening_lanes: Option<ForcedPotionOpeningReview>,
    pub(crate) key_card_counterfactual: Option<KeyCardCounterfactualProbe>,
    pub(crate) key_card_decision_microscope: Option<KeyCardDecisionMicroscopeProbe>,
    pub(crate) root_action_role_duel: Option<RootActionRoleDuelProbe>,
    pub(crate) champ_phase_audit: Option<ChampPhaseAudit>,
}
