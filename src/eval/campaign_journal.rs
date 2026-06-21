use serde::{Deserialize, Serialize};

use crate::eval::branch_experiment::{
    BranchExperimentBossRelicCandidateEntryV1, BranchExperimentCampfirePlanCandidateEntryV1,
    BranchExperimentEventCandidateEntryV1, BranchExperimentFirstEliteEvidenceV1,
    BranchExperimentRewardOptionPortfolioEntryV1, BranchExperimentRewardOptionPortfolioV1,
    BranchExperimentRouteCandidateEntryV1,
};

pub const CAMPAIGN_JOURNAL_SCHEMA_NAME: &str = "CampaignJournal";
pub const CAMPAIGN_JOURNAL_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
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
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
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
        candidates: Vec<CampaignJournalCandidateV1>,
    },
    RouteDecision {
        decision_id: String,
        route_branch_id: String,
        target: String,
        move_kind: String,
        safety: String,
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
        }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CampaignJournalCandidateAdmissionReasonCodeV1 {
    Unknown,
    Admit,
    Blocked,
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
    } else if candidate.safety == "reject_unless_forced" {
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
                candidate_id: "route:0:go 1".to_string(),
                rank: 0,
                selected: true,
                target: "x=1 y=1 Monster".to_string(),
                room_type: "Monster".to_string(),
                move_kind: "NormalEdge".to_string(),
                safety: "ok".to_string(),
                score: 1.25,
                command: "go 1".to_string(),
                elite_prep_bp: 42,
                first_elite: BranchExperimentFirstEliteEvidenceV1::default(),
                reasons: vec!["route planner selected".to_string()],
                cautions: Vec::new(),
            },
        );

        assert_eq!(candidate.candidate_id, "route:0:go 1");
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
}
