use serde::{Deserialize, Serialize};

use crate::eval::branch_experiment::{
    BranchExperimentBossRelicCandidateEntryV1, BranchExperimentCampfirePlanCandidateEntryV1,
    BranchExperimentEventCandidateEntryV1, BranchExperimentRewardOptionPortfolioEntryV1,
    BranchExperimentRewardOptionPortfolioV1,
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
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CampaignJournalCandidateV1 {
    pub candidate_id: String,
    pub command: String,
    pub label: String,
    pub semantic_class: String,
    pub disposition: CampaignJournalCandidateDispositionV1,
}

pub fn campaign_journal_candidate_from_campfire_entry_v1(
    candidate: &BranchExperimentCampfirePlanCandidateEntryV1,
) -> CampaignJournalCandidateV1 {
    CampaignJournalCandidateV1 {
        candidate_id: candidate.plan_id.clone(),
        command: candidate.command.clone(),
        label: candidate.label.clone(),
        semantic_class: campfire_candidate_semantic_class_v1(candidate),
        disposition: if candidate.branch_admission == "selected" {
            CampaignJournalCandidateDispositionV1::Kept
        } else {
            CampaignJournalCandidateDispositionV1::Pruned
        },
    }
}

pub fn campaign_journal_candidate_from_event_entry_v1(
    candidate: &BranchExperimentEventCandidateEntryV1,
) -> CampaignJournalCandidateV1 {
    CampaignJournalCandidateV1 {
        candidate_id: candidate.candidate_id.clone(),
        command: candidate.command.clone(),
        label: candidate.label.clone(),
        semantic_class: event_candidate_semantic_class_v1(candidate),
        disposition: if candidate.branch_admission == "selected" {
            CampaignJournalCandidateDispositionV1::Kept
        } else {
            CampaignJournalCandidateDispositionV1::Pruned
        },
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
