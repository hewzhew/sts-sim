use serde::{Deserialize, Serialize};

use crate::ai::route_planner_v1::{
    MapDecisionPacketV1, MapRouteTargetV1, NeedVectorV1, NodeFeaturesV1,
    RouteCandidatePoolProvenanceV1, RouteEvaluationCalibrationStatusV1, RouteEvaluationSourceV1,
    RouteMapActionV1, RouteMoveCandidateV1, RoutePathSummaryV1, RouteProjectionCoverageV1,
    RouteProjectionSourceV1, RouteSafetyFlagV1, RouteScoreTermsV1, RouteValueFactorsV1,
};
use crate::eval::branch_experiment::{
    BranchExperimentBossRelicCandidateEntryV1, BranchExperimentCampfirePlanCandidateEntryV1,
    BranchExperimentEventCandidateEntryV1, BranchExperimentFirstEliteEvidenceV1,
    BranchExperimentRewardOptionPortfolioEntryV1, BranchExperimentRewardOptionPortfolioV1,
    BranchExperimentRouteCandidateEntryV1,
};

pub const CAMPAIGN_JOURNAL_SCHEMA_NAME: &str = "CampaignJournal";
pub const CAMPAIGN_JOURNAL_SCHEMA_VERSION: u32 = 4;

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignJournalV1 {
    pub schema_name: String,
    pub schema_version: u32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub events: Vec<CampaignJournalEventV1>,
}

impl CampaignJournalV1 {
    pub fn new() -> Self {
        Self {
            schema_name: CAMPAIGN_JOURNAL_SCHEMA_NAME.to_string(),
            schema_version: CAMPAIGN_JOURNAL_SCHEMA_VERSION,
            events: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.events.is_empty()
    }

    pub fn extend(&mut self, events: impl IntoIterator<Item = CampaignJournalEventV1>) {
        self.events.extend(events);
        if self.schema_name.is_empty() {
            self.schema_name = CAMPAIGN_JOURNAL_SCHEMA_NAME.to_string();
        }
        if self.schema_version == 0 {
            self.schema_version = CAMPAIGN_JOURNAL_SCHEMA_VERSION;
        }
    }

    pub fn compact_for_campaign_artifact_v1(&mut self) {
        for event in &mut self.events {
            match &mut event.payload {
                CampaignJournalEventPayloadV1::RouteCandidatePool {
                    map_decision_packet,
                    route_candidates,
                    candidates,
                    ..
                } => {
                    let Some(packet) = map_decision_packet.take() else {
                        if !route_candidates.is_empty() {
                            candidates.clear();
                        }
                        continue;
                    };
                    if route_candidates.is_empty() {
                        *route_candidates = packet
                            .candidates
                            .iter()
                            .map(CampaignJournalRouteCandidateV1::from_route_move_candidate_v1)
                            .collect();
                    }
                    for candidate in route_candidates {
                        candidate.compact_for_campaign_artifact_v1();
                    }
                    candidates.clear();
                }
                CampaignJournalEventPayloadV1::RouteDecision {
                    selected_route_candidate,
                    ..
                } => {
                    *selected_route_candidate = None;
                }
                _ => {}
            }
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CampaignJournalEventV1 {
    pub event_id: String,
    pub round: usize,
    pub branch_id: String,
    pub branch_index: usize,
    #[serde(default)]
    pub branch_frontier_title: String,
    #[serde(default)]
    pub act: u8,
    #[serde(default)]
    pub floor: i32,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branch_choices: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub branch_commands: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    pub combat_budget_retry_used: bool,
    #[serde(flatten)]
    pub payload: CampaignJournalEventPayloadV1,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "event_type", rename_all = "snake_case", deny_unknown_fields)]
pub enum CampaignJournalEventPayloadV1 {
    RewardCandidateSet {
        decision_id: String,
        boundary_title: String,
        frontier_key: String,
        depth: usize,
        max_reward_options_per_branch: usize,
        original_count: usize,
        selected_count: usize,
        candidates: Vec<CampaignJournalCandidateV1>,
    },
    ShopBranchCandidateSet {
        decision_id: String,
        boundary_title: String,
        frontier_key: String,
        depth: usize,
        candidate_count: usize,
        candidates: Vec<CampaignJournalCandidateV1>,
    },
    ShopCandidatePool {
        decision_id: String,
        boundary_title: String,
        frontier_key: String,
        depth: usize,
        candidate_count: usize,
        branch_frontier_count: usize,
        rollout_head_plan_id: Option<String>,
        candidates: Vec<CampaignJournalCandidateV1>,
    },
    CampfireCandidatePool {
        decision_id: String,
        boundary_title: String,
        frontier_key: String,
        depth: usize,
        candidate_count: usize,
        branch_option_count: usize,
        selected_plan_id: Option<String>,
        candidates: Vec<CampaignJournalCandidateV1>,
    },
    EventCandidatePool {
        decision_id: String,
        boundary_title: String,
        frontier_key: String,
        depth: usize,
        game_event_id: String,
        candidate_count: usize,
        branch_option_count: usize,
        candidates: Vec<CampaignJournalCandidateV1>,
    },
    BossRelicCandidatePool {
        decision_id: String,
        boundary_title: String,
        frontier_key: String,
        depth: usize,
        candidate_count: usize,
        branch_option_count: usize,
        candidates: Vec<CampaignJournalCandidateV1>,
    },
    RouteCandidatePool {
        decision_id: String,
        boundary_title: String,
        frontier_key: String,
        depth: usize,
        candidate_count: usize,
        selected_index: Option<usize>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        candidate_pool_provenance: Option<RouteCandidatePoolProvenanceV1>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        map_decision_packet: Option<MapDecisionPacketV1>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        route_candidates: Vec<CampaignJournalRouteCandidateV1>,
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        candidates: Vec<CampaignJournalCandidateV1>,
    },
    RouteDecision {
        decision_id: String,
        route_branch_id: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selected_index: Option<usize>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selected_candidate_id: Option<String>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selected_candidate_rank: Option<usize>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selected_target_node: Option<MapRouteTargetV1>,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        selected_route_candidate: Option<CampaignJournalRouteCandidateV1>,
        target: String,
        move_kind: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        safety_flag: Option<RouteSafetyFlagV1>,
        safety: String,
        #[serde(default, skip_serializing_if = "Option::is_none")]
        candidate_pool_provenance: Option<RouteCandidatePoolProvenanceV1>,
        command: String,
        elite_prep_bp: i32,
        first_elite: BranchExperimentFirstEliteEvidenceV1,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignJournalCandidateV1 {
    pub candidate_id: String,
    pub command: String,
    pub label: String,
    pub semantic_class: String,
    #[serde(
        default,
        skip_serializing_if = "CampaignJournalCandidateAdmissionTraceV1::is_unknown"
    )]
    pub admission: CampaignJournalCandidateAdmissionTraceV1,
    pub disposition: CampaignJournalCandidateDispositionV1,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignJournalRouteCandidateV1 {
    pub candidate_id: String,
    pub rank: usize,
    pub selected: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_node: Option<MapRouteTargetV1>,
    pub target: String,
    pub room_type: String,
    pub move_kind: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<RouteMapActionV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub safety_flag: Option<RouteSafetyFlagV1>,
    pub safety: String,
    pub score: f32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub score_terms: Option<RouteScoreTermsV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub value_factors: Option<RouteValueFactorsV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluation_source: Option<RouteEvaluationSourceV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub evaluation_calibration_status: Option<RouteEvaluationCalibrationStatusV1>,
    pub command: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub node_features: Option<NodeFeaturesV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_summary: Option<RoutePathSummaryV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub needs: Option<NeedVectorV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_source: Option<RouteProjectionSourceV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub projection_coverage: Option<RouteProjectionCoverageV1>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub path_budget: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub observed_path_count: Option<usize>,
    pub elite_prep_bp: i32,
    pub first_elite: BranchExperimentFirstEliteEvidenceV1,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub reasons: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub cautions: Vec<String>,
}

impl CampaignJournalRouteCandidateV1 {
    pub fn compact_for_campaign_artifact_v1(&mut self) {
        self.score_terms = None;
        self.value_factors = None;
        self.node_features = None;
        self.needs = None;
        self.reasons.clear();
        self.cautions.clear();
    }

    pub fn from_route_entry_v1(candidate: &BranchExperimentRouteCandidateEntryV1) -> Self {
        Self {
            candidate_id: candidate.candidate_id.clone(),
            rank: candidate.rank,
            selected: candidate.selected,
            target_node: candidate.target_node.clone(),
            target: candidate.target.clone(),
            room_type: candidate.room_type.clone(),
            move_kind: candidate.move_kind.clone(),
            action: candidate.action.clone(),
            safety_flag: candidate.safety_flag,
            safety: candidate.safety.clone(),
            score: candidate.score,
            score_terms: candidate.score_terms.clone(),
            value_factors: candidate.value_factors.clone(),
            evaluation_source: candidate.evaluation_source,
            evaluation_calibration_status: candidate.evaluation_calibration_status,
            command: candidate.command.clone(),
            node_features: candidate.node_features.clone(),
            path_summary: candidate.path_summary.clone(),
            needs: candidate.needs.clone(),
            projection_source: candidate.projection_source,
            projection_coverage: candidate.projection_coverage,
            path_budget: candidate.path_budget,
            observed_path_count: candidate.observed_path_count,
            elite_prep_bp: candidate.elite_prep_bp,
            first_elite: candidate.first_elite.clone(),
            reasons: candidate.reasons.clone(),
            cautions: candidate.cautions.clone(),
        }
    }

    pub fn from_route_move_candidate_v1(candidate: &RouteMoveCandidateV1) -> Self {
        Self::from_route_move_candidate_with_selected_v1(candidate, false)
    }

    pub fn from_route_move_candidate_with_selected_v1(
        candidate: &RouteMoveCandidateV1,
        selected: bool,
    ) -> Self {
        let path = &candidate.projection.path_summary;
        Self {
            candidate_id: candidate.candidate_id.clone(),
            rank: candidate.rank,
            selected,
            target_node: Some(candidate.target.clone()),
            target: route_target_label_v1(&candidate.target),
            room_type: route_room_type_label_v1(candidate.target.room_type),
            move_kind: format!("{:?}", candidate.target.move_kind),
            action: Some(candidate.action.clone()),
            safety_flag: Some(candidate.evaluation.safety),
            safety: format!("{:?}", candidate.evaluation.safety),
            score: candidate.evaluation.total_score,
            score_terms: Some(candidate.evaluation.score_terms.clone()),
            value_factors: Some(candidate.evaluation.value_factors.clone()),
            evaluation_source: Some(candidate.evaluation.value_source),
            evaluation_calibration_status: Some(candidate.evaluation.calibration_status),
            command: candidate.command.clone(),
            node_features: Some(candidate.features.clone()),
            path_summary: Some(path.clone()),
            needs: Some(candidate.needs.clone()),
            projection_source: Some(candidate.projection.metadata.source),
            projection_coverage: Some(candidate.projection.metadata.coverage),
            path_budget: Some(candidate.projection.metadata.path_budget),
            observed_path_count: Some(candidate.projection.metadata.observed_path_count),
            elite_prep_bp: route_score_to_basis_points_v1(
                candidate.evaluation.score_terms.elite_prep,
            ),
            first_elite: BranchExperimentFirstEliteEvidenceV1 {
                paths_with_first_elite: path.first_elite.paths_with_first_elite,
                forced: path.first_elite.forced,
                optional: path.first_elite.optional,
                min_hallway_fights_before: path.first_elite.min_hallway_fights_before,
                max_hallway_fights_before: path.first_elite.max_hallway_fights_before,
                min_unknowns_before: path.first_elite.min_unknowns_before,
                max_unknowns_before: path.first_elite.max_unknowns_before,
                min_fires_before: path.first_elite.min_fires_before,
                max_fires_before: path.first_elite.max_fires_before,
                min_shops_before: path.first_elite.min_shops_before,
                max_shops_before: path.first_elite.max_shops_before,
                can_bail_to_rest_before: path.first_elite.can_bail_to_rest_before,
                can_bail_to_shop_before: path.first_elite.can_bail_to_shop_before,
            },
            reasons: candidate.evaluation.legacy_reasons.clone(),
            cautions: candidate.evaluation.legacy_cautions.clone(),
        }
    }
}

fn route_target_label_v1(target: &MapRouteTargetV1) -> String {
    format!(
        "x={} y={} {}",
        target.x,
        target.y,
        route_room_type_label_v1(target.room_type)
    )
}

fn route_room_type_label_v1(room_type: Option<crate::state::map::node::RoomType>) -> String {
    match room_type {
        Some(crate::state::map::node::RoomType::EventRoom) => "Event",
        Some(crate::state::map::node::RoomType::MonsterRoom) => "Monster",
        Some(crate::state::map::node::RoomType::MonsterRoomElite) => "Elite",
        Some(crate::state::map::node::RoomType::MonsterRoomBoss) => "Boss",
        Some(crate::state::map::node::RoomType::RestRoom) => "Rest",
        Some(crate::state::map::node::RoomType::ShopRoom) => "Shop",
        Some(crate::state::map::node::RoomType::TreasureRoom) => "Treasure",
        Some(crate::state::map::node::RoomType::TrueVictoryRoom) => "Victory",
        None => "Unknown",
    }
    .to_string()
}

fn route_score_to_basis_points_v1(score: f32) -> i32 {
    (score * 100.0).round() as i32
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignJournalCandidateAdmissionTraceV1 {
    pub status: CampaignJournalCandidateAdmissionStatusV1,
    #[serde(
        default,
        skip_serializing_if = "CampaignJournalCandidateAdmissionReasonCategoryV1::is_unknown"
    )]
    pub reason_category: CampaignJournalCandidateAdmissionReasonCategoryV1,
    #[serde(
        default,
        skip_serializing_if = "CampaignJournalCandidateAdmissionReasonCodeV1::is_unknown"
    )]
    pub reason_code: CampaignJournalCandidateAdmissionReasonCodeV1,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub source: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub reason: String,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub lane: String,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub representative_count: usize,
    #[serde(default, skip_serializing_if = "is_zero_usize")]
    pub suppressed_count: usize,
}

impl CampaignJournalCandidateAdmissionTraceV1 {
    pub fn new(
        status: CampaignJournalCandidateAdmissionStatusV1,
        source: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        let source = source.into();
        let reason = reason.into();
        Self {
            status,
            reason_category: admission_reason_category_from_source_v1(&source),
            reason_code: admission_reason_code_from_text_v1(&reason),
            source,
            reason,
            lane: String::new(),
            representative_count: 0,
            suppressed_count: 0,
        }
    }

    pub fn from_disposition(
        disposition: CampaignJournalCandidateDispositionV1,
        source: impl Into<String>,
        reason: impl Into<String>,
    ) -> Self {
        let status = match disposition {
            CampaignJournalCandidateDispositionV1::Kept => {
                CampaignJournalCandidateAdmissionStatusV1::Scheduled
            }
            CampaignJournalCandidateDispositionV1::Pruned => {
                CampaignJournalCandidateAdmissionStatusV1::Deferred
            }
        };
        Self::new(status, source, reason)
    }

    pub fn with_lane(mut self, lane: impl Into<String>) -> Self {
        self.lane = lane.into();
        self
    }

    pub fn with_counts(mut self, representative_count: usize, suppressed_count: usize) -> Self {
        self.representative_count = representative_count;
        self.suppressed_count = suppressed_count;
        self
    }

    pub fn normalized_reason_category(&self) -> CampaignJournalCandidateAdmissionReasonCategoryV1 {
        if self.reason_category != CampaignJournalCandidateAdmissionReasonCategoryV1::Unknown {
            return self.reason_category;
        }
        admission_reason_category_from_source_v1(&self.source)
    }

    pub fn normalized_reason_code(&self) -> CampaignJournalCandidateAdmissionReasonCodeV1 {
        if self.reason_code != CampaignJournalCandidateAdmissionReasonCodeV1::Unknown {
            return self.reason_code;
        }
        admission_reason_code_from_text_v1(&self.reason)
    }

    pub fn is_unknown(&self) -> bool {
        self.status == CampaignJournalCandidateAdmissionStatusV1::Unknown
            && self.reason_category == CampaignJournalCandidateAdmissionReasonCategoryV1::Unknown
            && self.reason_code == CampaignJournalCandidateAdmissionReasonCodeV1::Unknown
            && self.source.is_empty()
            && self.reason.is_empty()
            && self.lane.is_empty()
            && self.representative_count == 0
            && self.suppressed_count == 0
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignJournalCandidateAdmissionReasonCategoryV1 {
    Unknown,
    LegacyDisposition,
    RetentionBucket,
    BranchAdmission,
    EventBoundary,
}

impl Default for CampaignJournalCandidateAdmissionReasonCategoryV1 {
    fn default() -> Self {
        Self::Unknown
    }
}

impl CampaignJournalCandidateAdmissionReasonCategoryV1 {
    pub fn is_unknown(&self) -> bool {
        *self == Self::Unknown
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::LegacyDisposition => "legacy_disposition",
            Self::RetentionBucket => "retention_bucket",
            Self::BranchAdmission => "branch_admission",
            Self::EventBoundary => "event_boundary",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignJournalCandidateAdmissionReasonCodeV1 {
    Unknown,
    Admit,
    Blocked,
    CurrentEventBoundaryCandidate,
    Deferred,
    Kept,
    Pruned,
    Reject,
    Scheduled,
    Selected,
}

impl Default for CampaignJournalCandidateAdmissionReasonCodeV1 {
    fn default() -> Self {
        Self::Unknown
    }
}

impl CampaignJournalCandidateAdmissionReasonCodeV1 {
    pub fn is_unknown(&self) -> bool {
        *self == Self::Unknown
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Unknown => "unknown",
            Self::Admit => "admit",
            Self::Blocked => "blocked",
            Self::CurrentEventBoundaryCandidate => "current_event_boundary_candidate",
            Self::Deferred => "deferred",
            Self::Kept => "kept",
            Self::Pruned => "pruned",
            Self::Reject => "reject",
            Self::Scheduled => "scheduled",
            Self::Selected => "selected",
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignJournalCandidateAdmissionStatusV1 {
    Unknown,
    Scheduled,
    Deferred,
    Rejected,
}

impl Default for CampaignJournalCandidateAdmissionStatusV1 {
    fn default() -> Self {
        Self::Unknown
    }
}

pub fn campaign_journal_candidate_from_campfire_entry_v1(
    candidate: &BranchExperimentCampfirePlanCandidateEntryV1,
) -> CampaignJournalCandidateV1 {
    let disposition = if candidate.branch_admission == "selected" {
        CampaignJournalCandidateDispositionV1::Kept
    } else {
        CampaignJournalCandidateDispositionV1::Pruned
    };
    CampaignJournalCandidateV1 {
        candidate_id: candidate.plan_id.clone(),
        command: candidate.command.clone(),
        label: candidate.label.clone(),
        semantic_class: campfire_candidate_semantic_class_v1(candidate),
        admission: CampaignJournalCandidateAdmissionTraceV1::new(
            campaign_journal_status_from_branch_admission_v1(&candidate.branch_admission),
            "campfire_candidate_pool",
            candidate.branch_admission.clone(),
        )
        .with_lane(candidate.role.clone())
        .with_counts(candidate.representative_count, candidate.suppressed_count),
        disposition,
    }
}

pub fn campaign_journal_candidate_from_event_entry_v1(
    candidate: &BranchExperimentEventCandidateEntryV1,
) -> CampaignJournalCandidateV1 {
    let disposition = if candidate.branch_admission == "selected" {
        CampaignJournalCandidateDispositionV1::Kept
    } else {
        CampaignJournalCandidateDispositionV1::Pruned
    };
    CampaignJournalCandidateV1 {
        candidate_id: candidate.candidate_id.clone(),
        command: candidate.command.clone(),
        label: candidate.label.clone(),
        semantic_class: event_candidate_semantic_class_v1(candidate),
        admission: CampaignJournalCandidateAdmissionTraceV1::new(
            campaign_journal_status_from_branch_admission_v1(&candidate.branch_admission),
            "event_candidate_pool",
            candidate.branch_admission.clone(),
        )
        .with_lane(candidate.effect_kind.clone())
        .with_counts(candidate.representative_count, candidate.suppressed_count),
        disposition,
    }
}

fn event_candidate_semantic_class_v1(candidate: &BranchExperimentEventCandidateEntryV1) -> String {
    let mut parts = vec![
        format!("effect:{}", candidate.effect_kind),
        format!("branch:{}", candidate.branch_admission),
        format!("representatives:{}", candidate.representative_count),
    ];
    if let Some(class) = &candidate.event_policy_class {
        parts.push(format!("class:{class}"));
    }
    if let Some(tier) = &candidate.event_policy_tier {
        parts.push(format!("tier:{tier}"));
    }
    if let Some(score) = candidate.event_policy_score {
        parts.push(format!("score:{score}"));
    }
    if candidate.suppressed_count > 0 {
        parts.push(format!("suppressed:{}", candidate.suppressed_count));
    }
    parts.join(" ")
}

pub fn campaign_journal_candidate_from_boss_relic_entry_v1(
    candidate: &BranchExperimentBossRelicCandidateEntryV1,
) -> CampaignJournalCandidateV1 {
    CampaignJournalCandidateV1 {
        candidate_id: candidate.candidate_id.clone(),
        command: candidate.command.clone(),
        label: candidate.label.clone(),
        semantic_class: boss_relic_candidate_semantic_class_v1(candidate),
        admission: CampaignJournalCandidateAdmissionTraceV1::new(
            campaign_journal_status_from_branch_admission_v1(&candidate.branch_admission),
            "boss_relic_candidate_pool",
            candidate.branch_admission.clone(),
        )
        .with_lane(candidate.class.clone()),
        disposition: CampaignJournalCandidateDispositionV1::Kept,
    }
}

fn boss_relic_candidate_semantic_class_v1(
    candidate: &BranchExperimentBossRelicCandidateEntryV1,
) -> String {
    let mut parts = vec![
        format!("relic:{}", candidate.relic),
        format!("class:{}", candidate.class),
        format!("support:{}", candidate.support_gate),
        format!("branch:{}", candidate.branch_admission),
    ];
    if !candidate.added_debt.is_empty() {
        parts.push(format!("debt:{}", candidate.added_debt.join("+")));
    }
    if !candidate.compounding_tags.is_empty() {
        parts.push(format!(
            "compounds:{}",
            candidate.compounding_tags.join("+")
        ));
    }
    parts.join(" ")
}

pub fn campaign_journal_candidate_from_route_entry_v1(
    candidate: &BranchExperimentRouteCandidateEntryV1,
) -> CampaignJournalCandidateV1 {
    let (status, reason, disposition) = if candidate.selected {
        (
            CampaignJournalCandidateAdmissionStatusV1::Scheduled,
            "selected",
            CampaignJournalCandidateDispositionV1::Kept,
        )
    } else if candidate.resolved_safety_flag() == RouteSafetyFlagV1::RejectUnlessNoAlternative {
        (
            CampaignJournalCandidateAdmissionStatusV1::Rejected,
            "rejected",
            CampaignJournalCandidateDispositionV1::Pruned,
        )
    } else {
        (
            CampaignJournalCandidateAdmissionStatusV1::Deferred,
            "deferred",
            CampaignJournalCandidateDispositionV1::Pruned,
        )
    };
    CampaignJournalCandidateV1 {
        candidate_id: candidate.candidate_id.clone(),
        command: candidate.command.clone(),
        label: candidate.target.clone(),
        semantic_class: route_candidate_semantic_class_v1(candidate),
        admission: CampaignJournalCandidateAdmissionTraceV1::new(
            status,
            "route_candidate_pool",
            reason,
        )
        .with_lane(candidate.room_type.clone()),
        disposition,
    }
}

fn route_candidate_semantic_class_v1(candidate: &BranchExperimentRouteCandidateEntryV1) -> String {
    let mut parts = vec![
        format!("room:{}", candidate.room_type),
        format!("move:{}", candidate.move_kind),
        format!("safety:{}", candidate.safety),
        format!("rank:{}", candidate.rank),
        format!("score:{}", candidate.score),
        format!("elite_prep_bp:{}", candidate.elite_prep_bp),
    ];
    if candidate.selected {
        parts.push("selected:true".to_string());
    }
    if !candidate.reasons.is_empty() {
        parts.push(format!("reasons:{}", candidate.reasons.join("+")));
    }
    if !candidate.cautions.is_empty() {
        parts.push(format!("cautions:{}", candidate.cautions.join("+")));
    }
    parts.join(" ")
}

fn campfire_candidate_semantic_class_v1(
    candidate: &BranchExperimentCampfirePlanCandidateEntryV1,
) -> String {
    let mut parts = vec![
        format!("role:{}", candidate.role),
        format!("effect:{}", candidate.effect_kind),
        format!("score_hint:{}", candidate.score_hint),
        format!("confidence_milli:{}", candidate.confidence_milli),
        format!("execute:{}", candidate.execute_autopilot),
        format!("branch_active:{}", candidate.branch_active),
        format!("branch:{}", candidate.branch_admission),
        format!("representatives:{}", candidate.representative_count),
    ];
    if let Some(tag) = &candidate.strategy_tag {
        parts.push(format!("strategy_tag:{tag}"));
    }
    if candidate.suppressed_count > 0 {
        parts.push(format!("suppressed:{}", candidate.suppressed_count));
    }
    parts.join(" ")
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignJournalCandidateDispositionV1 {
    Kept,
    Pruned,
}

fn campaign_journal_status_from_branch_admission_v1(
    admission: &str,
) -> CampaignJournalCandidateAdmissionStatusV1 {
    match admission.to_ascii_lowercase().as_str() {
        "admit" | "selected" | "scheduled" | "kept" => {
            CampaignJournalCandidateAdmissionStatusV1::Scheduled
        }
        "reject" | "rejected" | "blocked" | "block" => {
            CampaignJournalCandidateAdmissionStatusV1::Rejected
        }
        _ => CampaignJournalCandidateAdmissionStatusV1::Deferred,
    }
}

fn admission_reason_category_from_source_v1(
    source: &str,
) -> CampaignJournalCandidateAdmissionReasonCategoryV1 {
    match source {
        "legacy_disposition" => {
            CampaignJournalCandidateAdmissionReasonCategoryV1::LegacyDisposition
        }
        "reward_portfolio" => CampaignJournalCandidateAdmissionReasonCategoryV1::RetentionBucket,
        "event_boundary_packet" => CampaignJournalCandidateAdmissionReasonCategoryV1::EventBoundary,
        source if source.ends_with("_candidate_pool") => {
            CampaignJournalCandidateAdmissionReasonCategoryV1::BranchAdmission
        }
        _ => CampaignJournalCandidateAdmissionReasonCategoryV1::Unknown,
    }
}

fn admission_reason_code_from_text_v1(
    reason: &str,
) -> CampaignJournalCandidateAdmissionReasonCodeV1 {
    match reason.to_ascii_lowercase().as_str() {
        "admit" => CampaignJournalCandidateAdmissionReasonCodeV1::Admit,
        "blocked" | "block" => CampaignJournalCandidateAdmissionReasonCodeV1::Blocked,
        "current_event_boundary_candidate" => {
            CampaignJournalCandidateAdmissionReasonCodeV1::CurrentEventBoundaryCandidate
        }
        "deferred" | "defer" => CampaignJournalCandidateAdmissionReasonCodeV1::Deferred,
        "kept" | "keep" => CampaignJournalCandidateAdmissionReasonCodeV1::Kept,
        "pruned" | "prune" => CampaignJournalCandidateAdmissionReasonCodeV1::Pruned,
        "reject" | "rejected" => CampaignJournalCandidateAdmissionReasonCodeV1::Reject,
        "scheduled" => CampaignJournalCandidateAdmissionReasonCodeV1::Scheduled,
        "selected" => CampaignJournalCandidateAdmissionReasonCodeV1::Selected,
        _ => CampaignJournalCandidateAdmissionReasonCodeV1::Unknown,
    }
}

pub fn reward_portfolio_from_journal_event_v1(
    event: &CampaignJournalEventV1,
) -> Option<BranchExperimentRewardOptionPortfolioV1> {
    let CampaignJournalEventPayloadV1::RewardCandidateSet {
        boundary_title,
        frontier_key,
        depth,
        max_reward_options_per_branch,
        original_count,
        selected_count,
        candidates,
        ..
    } = &event.payload
    else {
        return None;
    };

    let mut selected_options = Vec::new();
    let mut pruned_options = Vec::new();
    for candidate in candidates {
        let entry = BranchExperimentRewardOptionPortfolioEntryV1 {
            command: candidate.command.clone(),
            label: candidate.label.clone(),
            semantic_class: candidate.semantic_class.clone(),
        };
        match candidate.disposition {
            CampaignJournalCandidateDispositionV1::Kept => selected_options.push(entry),
            CampaignJournalCandidateDispositionV1::Pruned => pruned_options.push(entry),
        }
    }

    Some(BranchExperimentRewardOptionPortfolioV1 {
        branch_id: event.branch_id.clone(),
        branch_choices: event.branch_choices.clone(),
        branch_commands: event.branch_commands.clone(),
        depth: *depth,
        frontier_key: frontier_key.clone(),
        boundary_title: boundary_title.clone(),
        max_reward_options_per_branch: *max_reward_options_per_branch,
        original_count: *original_count,
        selected_count: *selected_count,
        selected_options,
        pruned_options,
    })
}

fn is_false(value: &bool) -> bool {
    !*value
}

fn is_zero_usize(value: &usize) -> bool {
    *value == 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn event_candidates_record_structured_admission_trace() {
        let candidate = campaign_journal_candidate_from_event_entry_v1(
            &BranchExperimentEventCandidateEntryV1 {
                candidate_id: "event:0".to_string(),
                command: "event 0".to_string(),
                label: "Take event option".to_string(),
                event_index: Some(0),
                effect_kind: "gain_relic".to_string(),
                effect_key: "golden_idol".to_string(),
                event_policy_class: Some("valuable_event".to_string()),
                event_policy_tier: Some("strong".to_string()),
                event_policy_score: Some(100),
                branch_admission: "selected".to_string(),
                representative_count: 2,
                suppressed_count: 1,
                reasons: vec!["event policy kept representative".to_string()],
            },
        );

        assert_eq!(
            candidate.admission.status,
            CampaignJournalCandidateAdmissionStatusV1::Scheduled
        );
        assert_eq!(candidate.admission.source, "event_candidate_pool");
        assert_eq!(candidate.admission.reason, "selected");
        assert_eq!(
            candidate.admission.reason_category,
            CampaignJournalCandidateAdmissionReasonCategoryV1::BranchAdmission
        );
        assert_eq!(
            candidate.admission.reason_code,
            CampaignJournalCandidateAdmissionReasonCodeV1::Selected
        );
        assert_eq!(candidate.admission.representative_count, 2);
        assert_eq!(candidate.admission.suppressed_count, 1);
    }

    #[test]
    fn old_admission_trace_normalizes_reason_from_source_text() {
        let admission: CampaignJournalCandidateAdmissionTraceV1 = serde_json::from_str(
            r#"{"status":"scheduled","source":"reward_portfolio","reason":"kept"}"#,
        )
        .expect("old admission trace should deserialize");

        assert_eq!(
            admission.reason_category,
            CampaignJournalCandidateAdmissionReasonCategoryV1::Unknown
        );
        assert_eq!(
            admission.normalized_reason_category(),
            CampaignJournalCandidateAdmissionReasonCategoryV1::RetentionBucket
        );
        assert_eq!(
            admission.normalized_reason_code(),
            CampaignJournalCandidateAdmissionReasonCodeV1::Kept
        );
    }

    #[test]
    fn route_candidates_record_structured_admission_trace() {
        let candidate = campaign_journal_candidate_from_route_entry_v1(
            &crate::eval::branch_experiment::BranchExperimentRouteCandidateEntryV1 {
                candidate_id: "route_move:normal_edge:x1:y1".to_string(),
                rank: 0,
                selected: true,
                target_node: None,
                target: "x=1 y=1 Monster".to_string(),
                room_type: "Monster".to_string(),
                move_kind: "NormalEdge".to_string(),
                action: None,
                safety_flag: None,
                safety: "ok".to_string(),
                score: 1.25,
                score_terms: None,
                value_factors: None,
                evaluation_source: None,
                evaluation_calibration_status: None,
                command: "go 1".to_string(),
                node_features: None,
                path_summary: None,
                needs: None,
                projection_source: None,
                projection_coverage: None,
                path_budget: None,
                observed_path_count: None,
                elite_prep_bp: 42,
                first_elite: BranchExperimentFirstEliteEvidenceV1::default(),
                reasons: vec!["route planner selected".to_string()],
                cautions: Vec::new(),
            },
        );

        assert_eq!(candidate.candidate_id, "route_move:normal_edge:x1:y1");
        assert_eq!(candidate.command, "go 1");
        assert_eq!(
            candidate.admission.status,
            CampaignJournalCandidateAdmissionStatusV1::Scheduled
        );
        assert_eq!(candidate.admission.source, "route_candidate_pool");
        assert_eq!(
            candidate.admission.normalized_reason_category(),
            CampaignJournalCandidateAdmissionReasonCategoryV1::BranchAdmission
        );
        assert_eq!(
            candidate.admission.normalized_reason_code(),
            CampaignJournalCandidateAdmissionReasonCodeV1::Selected
        );
    }

    #[test]
    fn journal_compaction_moves_route_map_packets_to_typed_candidates() {
        let mut run = crate::state::RunState::new(521, 0, false, "Ironclad");
        run.event_state = None;
        let trace = crate::ai::route_planner_v1::plan_route_decision_v1(
            &run,
            &crate::state::core::EngineState::MapNavigation,
            crate::ai::route_planner_v1::RoutePlannerConfigV1::default(),
        );
        let packet =
            crate::ai::route_planner_v1::MapDecisionPacketV1::from_route_decision_trace_v1(&trace);
        assert!(!packet.candidates.is_empty());
        let expected_candidate_count = packet.candidates.len();
        let selected_route_candidate =
            CampaignJournalRouteCandidateV1::from_route_move_candidate_v1(&packet.candidates[0]);
        let mut journal = CampaignJournalV1 {
            schema_name: CAMPAIGN_JOURNAL_SCHEMA_NAME.to_string(),
            schema_version: CAMPAIGN_JOURNAL_SCHEMA_VERSION,
            events: vec![
                CampaignJournalEventV1 {
                    event_id: "route-pool:candidate_set".to_string(),
                    round: 1,
                    branch_id: "root".to_string(),
                    branch_index: 0,
                    branch_frontier_title: "Map".to_string(),
                    act: 1,
                    floor: 1,
                    branch_choices: Vec::new(),
                    branch_commands: Vec::new(),
                    combat_budget_retry_used: false,
                    payload: CampaignJournalEventPayloadV1::RouteCandidatePool {
                        decision_id: "route-pool".to_string(),
                        boundary_title: "Map".to_string(),
                        frontier_key: "map".to_string(),
                        depth: 0,
                        candidate_count: expected_candidate_count,
                        selected_index: Some(0),
                        candidate_pool_provenance: None,
                        map_decision_packet: Some(packet),
                        route_candidates: Vec::new(),
                        candidates: Vec::new(),
                    },
                },
                CampaignJournalEventV1 {
                    event_id: "route-decision".to_string(),
                    round: 1,
                    branch_id: "root".to_string(),
                    branch_index: 0,
                    branch_frontier_title: "Map".to_string(),
                    act: 1,
                    floor: 1,
                    branch_choices: Vec::new(),
                    branch_commands: Vec::new(),
                    combat_budget_retry_used: false,
                    payload: CampaignJournalEventPayloadV1::RouteDecision {
                        decision_id: "route-decision".to_string(),
                        route_branch_id: "route-branch".to_string(),
                        selected_index: Some(0),
                        selected_candidate_id: Some(selected_route_candidate.candidate_id.clone()),
                        selected_candidate_rank: Some(selected_route_candidate.rank),
                        selected_target_node: selected_route_candidate.target_node.clone(),
                        selected_route_candidate: Some(selected_route_candidate),
                        target: "x=1 y=0".to_string(),
                        move_kind: "normal_edge".to_string(),
                        safety_flag: None,
                        safety: "ok".to_string(),
                        candidate_pool_provenance: None,
                        command: "go 1".to_string(),
                        elite_prep_bp: 0,
                        first_elite: BranchExperimentFirstEliteEvidenceV1::default(),
                    },
                },
            ],
        };

        journal.compact_for_campaign_artifact_v1();

        match &journal.events[0].payload {
            CampaignJournalEventPayloadV1::RouteCandidatePool {
                map_decision_packet,
                route_candidates,
                ..
            } => {
                assert!(map_decision_packet.is_none());
                assert_eq!(route_candidates.len(), expected_candidate_count);
                assert!(route_candidates[0].path_summary.is_some());
                assert!(route_candidates[0].score_terms.is_none());
                assert!(route_candidates[0].value_factors.is_none());
                assert!(route_candidates[0].node_features.is_none());
                assert!(route_candidates[0].needs.is_none());
                assert!(route_candidates[0].reasons.is_empty());
            }
            _ => panic!("expected route candidate pool"),
        }
        match &journal.events[1].payload {
            CampaignJournalEventPayloadV1::RouteDecision {
                selected_route_candidate,
                selected_candidate_id,
                selected_target_node,
                ..
            } => {
                assert!(selected_route_candidate.is_none());
                assert!(selected_candidate_id.is_some());
                assert!(selected_target_node.is_some());
            }
            _ => panic!("expected route decision"),
        }
    }
}
