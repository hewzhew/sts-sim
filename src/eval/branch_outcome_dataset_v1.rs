use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::ai::deck_startup_profile_v1::deck_startup_profile_v1;
use crate::ai::noncombat_strategy_v1::build_run_strategy_snapshot_from_run_state_v2;
use crate::ai::strategic::BranchSignatureCompact;
use crate::content::cards::{get_card_definition, CardTag, CardType};
use crate::eval::branch_campaign::{
    BranchCampaignBranchStatusV1, BranchCampaignBranchSummaryV1, BranchCampaignBranchV1,
    BranchCampaignCheckpointV1, BranchCampaignReportV1, BranchCampaignRunDomainV1,
};
use crate::eval::run_control::RunControlSession;
use crate::runtime::combat::CombatCard;

pub const BRANCH_OUTCOME_RECORD_SCHEMA_NAME: &str = "BranchOutcomeRecordV1";
pub const BRANCH_OUTCOME_RECORD_SCHEMA_VERSION: u32 = 1;
pub const BRANCH_OUTCOME_DATASET_SUMMARY_SCHEMA_NAME: &str = "BranchOutcomeDatasetSummaryV1";
pub const BRANCH_OUTCOME_DATASET_SUMMARY_SCHEMA_VERSION: u32 = 1;
pub const BRANCH_OUTCOME_DATASET_ANALYSIS_SCHEMA_NAME: &str = "BranchOutcomeDatasetAnalysisV1";
pub const BRANCH_OUTCOME_DATASET_ANALYSIS_SCHEMA_VERSION: u32 = 1;

const HIGH_LAST_COMBAT_HP_LOSS: i32 = 15;
const HIGH_LAST_COMBAT_HP_LOSS_PERCENT: i32 = 25;
const LOW_HP_PERCENT: i32 = 40;
const HIGH_UNCONVERTED_GOLD: i32 = 300;
const LARGE_DECK_COUNT: usize = 35;
const MAX_ISSUE_EXAMPLES: usize = 12;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchOutcomeClassV1 {
    OngoingActive,
    OngoingFrozen,
    TerminalVictory,
    TerminalDefeat,
    Abandoned,
    Stuck,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchOutcomeSupervisionStatusV1 {
    TerminalOutcome,
    CensoredOngoing,
    InterventionOrFailure,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchOutcomeRecordV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub label_role: String,
    pub trainable_as_action_label: bool,
    pub policy_quality_claim: bool,

    pub seed: u64,
    #[serde(default)]
    pub run_domain: BranchCampaignRunDomainV1,
    pub report_rounds_completed: usize,
    pub report_stop_reason: String,

    pub branch_group: String,
    pub branch_index: usize,
    pub branch_id: String,
    pub branch_status: BranchCampaignBranchStatusV1,
    pub outcome_class: BranchOutcomeClassV1,
    pub supervision_status: BranchOutcomeSupervisionStatusV1,

    pub rank_key: i32,
    #[serde(default)]
    pub strategic_summary: BranchSignatureCompact,
    #[serde(default, skip_serializing_if = "String::is_empty")]
    pub stop_reason: String,
    pub frontier_title: String,
    pub commands: Vec<String>,
    pub choice_labels: Vec<String>,

    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub report_summary: Option<BranchCampaignBranchSummaryV1>,
    pub checkpoint_enriched: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_features: Option<BranchOutcomeStateFeaturesV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchOutcomeStateFeaturesV1 {
    pub engine_state: String,
    pub act: u8,
    pub floor: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub ascension_level: u8,
    pub player_class: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub boss: Option<String>,
    pub boss_pressure: Vec<String>,

    pub deck: BranchOutcomeDeckFeaturesV1,
    pub relics: Vec<String>,
    pub potions: Vec<String>,
    pub formation: BranchOutcomeFormationFeaturesV1,
    pub startup: BranchOutcomeStartupFeaturesV1,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_combat: Option<BranchOutcomeLastCombatFeaturesV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchOutcomeDeckFeaturesV1 {
    pub deck_count: usize,
    pub grouped_cards: Vec<BranchOutcomeCardCountV1>,
    pub attacks: usize,
    pub skills: usize,
    pub powers: usize,
    pub curses: usize,
    pub statuses: usize,
    pub starter_strikes: usize,
    pub starter_defends: usize,
    pub upgraded: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchOutcomeCardCountV1 {
    pub id: String,
    pub name: String,
    pub upgrades: u8,
    pub card_type: String,
    pub count: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchOutcomeFormationFeaturesV1 {
    pub stage: String,
    pub needs: Vec<String>,
    pub strengths: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchOutcomeStartupFeaturesV1 {
    pub setup_debt: u8,
    pub setup_payment: u8,
    pub effective_setup_payment: u8,
    pub immediate_survival: u8,
    pub payoff_engine: u8,
    pub combat_shape_risk: u8,
    pub strong_draw_count: u8,
    pub effective_strong_draw_count: u8,
    pub exhaust_engine_count: u8,
    pub exhaust_payoff_count: u8,
    pub status_generator_count: u8,
    pub status_digest_count: u8,
    pub persistent_strength_source_count: u8,
    pub temporary_strength_burst_count: u8,
    pub strength_converter_count: u8,
    pub convertible_strength_source_count: u8,
    pub strength_payoff_count: u8,
    pub zero_cost_card_count: u8,
    pub low_cost_card_count: u8,
    pub high_cost_card_count: u8,
    pub has_snecko_eye: bool,
    pub snecko_random_cost_debt: u8,
    pub liabilities: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchOutcomeLastCombatFeaturesV1 {
    pub terminal: String,
    pub start_hp: i32,
    pub final_hp: i32,
    pub hp_loss: i32,
    pub turns: u32,
    pub potions_used: u32,
    pub cards_played: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchOutcomeDatasetSummaryV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub label_role: String,
    pub trainable_as_action_label: bool,
    pub policy_quality_claim: bool,
    pub total_records: usize,
    pub checkpoint_enriched_records: usize,
    pub outcome_class_counts: Vec<BranchOutcomeHistogramEntryV1>,
    pub supervision_status_counts: Vec<BranchOutcomeHistogramEntryV1>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchOutcomeHistogramEntryV1 {
    pub key: String,
    pub count: usize,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchOutcomeDatasetAnalysisV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub label_role: String,
    pub trainable_as_action_label: bool,
    pub policy_quality_claim: bool,
    pub total_records: usize,
    pub checkpoint_enriched_records: usize,
    pub outcome_class_counts: Vec<BranchOutcomeHistogramEntryV1>,
    pub branch_group_counts: Vec<BranchOutcomeHistogramEntryV1>,
    pub issue_counts: Vec<BranchOutcomeHistogramEntryV1>,
    pub issue_examples: Vec<BranchOutcomeIssueExampleV1>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchOutcomeIssueExampleV1 {
    pub issue_key: String,
    pub branch_group: String,
    pub outcome_class: BranchOutcomeClassV1,
    pub frontier_title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub act: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub floor: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub hp: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub max_hp: Option<i32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deck_count: Option<usize>,
    pub choice_tail: Vec<String>,
    pub details: Vec<String>,
}

pub fn extract_branch_outcome_records_v1(
    report: &BranchCampaignReportV1,
    checkpoint: Option<&BranchCampaignCheckpointV1>,
) -> Result<Vec<BranchOutcomeRecordV1>, String> {
    let sessions_by_commands = checkpoint
        .map(restored_checkpoint_sessions_by_commands)
        .transpose()?
        .unwrap_or_default();

    let mut records = Vec::new();
    append_branch_group_records(
        &mut records,
        report,
        "active",
        &report.active,
        &sessions_by_commands,
    );
    append_branch_group_records(
        &mut records,
        report,
        "frozen",
        &report.frozen,
        &sessions_by_commands,
    );
    append_branch_group_records(
        &mut records,
        report,
        "victories",
        &report.victories,
        &sessions_by_commands,
    );
    append_branch_group_records(
        &mut records,
        report,
        "dead",
        &report.dead,
        &sessions_by_commands,
    );
    append_branch_group_records(
        &mut records,
        report,
        "abandoned",
        &report.abandoned,
        &sessions_by_commands,
    );
    append_branch_group_records(
        &mut records,
        report,
        "stuck",
        &report.stuck,
        &sessions_by_commands,
    );
    Ok(records)
}

pub fn serialize_branch_outcome_records_jsonl_v1(
    records: &[BranchOutcomeRecordV1],
) -> Result<String, String> {
    let mut text = String::new();
    for record in records {
        let line = serde_json::to_string(record)
            .map_err(|err| format!("failed to serialize BranchOutcomeRecordV1: {err}"))?;
        text.push_str(&line);
        text.push('\n');
    }
    Ok(text)
}

pub fn parse_branch_outcome_records_jsonl_v1(
    text: &str,
) -> Result<Vec<BranchOutcomeRecordV1>, String> {
    let mut records = Vec::new();
    for (index, line) in text.lines().enumerate() {
        let line = line.trim();
        if line.is_empty() {
            continue;
        }
        let record = serde_json::from_str(line).map_err(|err| {
            format!(
                "failed to parse BranchOutcomeRecordV1 JSONL line {}: {err}",
                index + 1
            )
        })?;
        records.push(record);
    }
    Ok(records)
}

pub fn summarize_branch_outcome_records_v1(
    records: &[BranchOutcomeRecordV1],
) -> BranchOutcomeDatasetSummaryV1 {
    let mut outcome_class_counts = BTreeMap::<String, usize>::new();
    let mut supervision_status_counts = BTreeMap::<String, usize>::new();
    let mut checkpoint_enriched_records = 0usize;
    for record in records {
        *outcome_class_counts
            .entry(format!("{:?}", record.outcome_class))
            .or_default() += 1;
        *supervision_status_counts
            .entry(format!("{:?}", record.supervision_status))
            .or_default() += 1;
        if record.checkpoint_enriched {
            checkpoint_enriched_records += 1;
        }
    }

    BranchOutcomeDatasetSummaryV1 {
        schema_name: BRANCH_OUTCOME_DATASET_SUMMARY_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_OUTCOME_DATASET_SUMMARY_SCHEMA_VERSION,
        label_role: "campaign_observation_not_teacher".to_string(),
        trainable_as_action_label: false,
        policy_quality_claim: false,
        total_records: records.len(),
        checkpoint_enriched_records,
        outcome_class_counts: histogram_entries(outcome_class_counts),
        supervision_status_counts: histogram_entries(supervision_status_counts),
    }
}

pub fn analyze_branch_outcome_records_v1(
    records: &[BranchOutcomeRecordV1],
) -> BranchOutcomeDatasetAnalysisV1 {
    let mut outcome_class_counts = BTreeMap::<String, usize>::new();
    let mut branch_group_counts = BTreeMap::<String, usize>::new();
    let mut issue_counts = BTreeMap::<String, usize>::new();
    let mut issue_examples = Vec::new();
    let mut checkpoint_enriched_records = 0usize;

    for record in records {
        *outcome_class_counts
            .entry(format!("{:?}", record.outcome_class))
            .or_default() += 1;
        *branch_group_counts
            .entry(record.branch_group.clone())
            .or_default() += 1;
        if record.checkpoint_enriched {
            checkpoint_enriched_records += 1;
        }
        collect_record_issues(record, &mut issue_counts, &mut issue_examples);
    }

    BranchOutcomeDatasetAnalysisV1 {
        schema_name: BRANCH_OUTCOME_DATASET_ANALYSIS_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_OUTCOME_DATASET_ANALYSIS_SCHEMA_VERSION,
        label_role: "campaign_observation_not_teacher".to_string(),
        trainable_as_action_label: false,
        policy_quality_claim: false,
        total_records: records.len(),
        checkpoint_enriched_records,
        outcome_class_counts: histogram_entries(outcome_class_counts),
        branch_group_counts: histogram_entries(branch_group_counts),
        issue_counts: histogram_entries_by_count_desc(issue_counts),
        issue_examples,
    }
}

pub fn render_branch_outcome_dataset_analysis_v1(
    analysis: &BranchOutcomeDatasetAnalysisV1,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "BranchOutcomeDatasetAnalysisV1 records={} checkpoint_enriched={}",
        analysis.total_records, analysis.checkpoint_enriched_records
    ));
    if !analysis.outcome_class_counts.is_empty() {
        lines.push(format!(
            "Outcome classes: {}",
            render_histogram(&analysis.outcome_class_counts)
        ));
    }
    if !analysis.branch_group_counts.is_empty() {
        lines.push(format!(
            "Branch groups: {}",
            render_histogram(&analysis.branch_group_counts)
        ));
    }
    if analysis.issue_counts.is_empty() {
        lines.push("Issues: none".to_string());
    } else {
        lines.push("Issues:".to_string());
        for entry in &analysis.issue_counts {
            lines.push(format!("  {} | {}", entry.count, entry.key));
        }
    }
    if !analysis.issue_examples.is_empty() {
        lines.push(String::new());
        lines.push("Examples:".to_string());
        for example in &analysis.issue_examples {
            lines.push(format!(
                "  {} | {} {:?} {} A{}F{} HP {}/{} deck {} | {}",
                example.issue_key,
                example.branch_group,
                example.outcome_class,
                example.frontier_title,
                example.act.unwrap_or_default(),
                example.floor.unwrap_or_default(),
                example.hp.unwrap_or_default(),
                example.max_hp.unwrap_or_default(),
                example.deck_count.unwrap_or_default(),
                if example.choice_tail.is_empty() {
                    "-".to_string()
                } else {
                    example.choice_tail.join(" -> ")
                }
            ));
            if !example.details.is_empty() {
                lines.push(format!("    {}", example.details.join("; ")));
            }
        }
    }
    lines.join("\n")
}

fn collect_record_issues(
    record: &BranchOutcomeRecordV1,
    issue_counts: &mut BTreeMap<String, usize>,
    issue_examples: &mut Vec<BranchOutcomeIssueExampleV1>,
) {
    let Some(features) = record.state_features.as_ref() else {
        record_issue(
            issue_counts,
            issue_examples,
            record,
            "dataset:no_checkpoint_features",
            Vec::new(),
        );
        return;
    };

    let deck = &features.deck;
    if deck.starter_strikes > 0 {
        record_issue(
            issue_counts,
            issue_examples,
            record,
            "starter_debt:strikes_remaining",
            vec![format!("starter_strikes={}", deck.starter_strikes)],
        );
    }
    if deck.starter_defends > 0 {
        record_issue(
            issue_counts,
            issue_examples,
            record,
            "starter_debt:defends_remaining",
            vec![format!("starter_defends={}", deck.starter_defends)],
        );
    }
    if deck.deck_count >= LARGE_DECK_COUNT {
        record_issue(
            issue_counts,
            issue_examples,
            record,
            "deck:large",
            vec![format!("deck_count={}", deck.deck_count)],
        );
    }
    if features.act >= 3 && deck.powers == 0 {
        record_issue(
            issue_counts,
            issue_examples,
            record,
            "deck:no_powers_act3",
            vec!["act>=3 powers=0".to_string()],
        );
    }
    if features.gold >= HIGH_UNCONVERTED_GOLD {
        record_issue(
            issue_counts,
            issue_examples,
            record,
            "resource:high_gold",
            vec![format!("gold={}", features.gold)],
        );
    }
    if features.max_hp > 0 && features.hp * 100 <= features.max_hp * LOW_HP_PERCENT {
        record_issue(
            issue_counts,
            issue_examples,
            record,
            "hp:low",
            vec![format!("hp={}/{}", features.hp, features.max_hp)],
        );
    }

    for card in &deck.grouped_cards {
        if card.count > 1 && !is_basic_starter_card_name(&card.name) {
            record_issue(
                issue_counts,
                issue_examples,
                record,
                &format!("duplicate_card:{}", card.name),
                vec![format!("{}x{}", card.name, card.count)],
            );
        }
    }
    for liability in &features.startup.liabilities {
        record_issue(
            issue_counts,
            issue_examples,
            record,
            &format!("startup_liability:{liability}"),
            Vec::new(),
        );
    }
    for pressure in &features.boss_pressure {
        if pressure.starts_with("missing:") {
            record_issue(
                issue_counts,
                issue_examples,
                record,
                &format!("boss_pressure:{pressure}"),
                Vec::new(),
            );
        }
    }
    if let Some(last_combat) = features.last_combat.as_ref() {
        let high_absolute = last_combat.hp_loss >= HIGH_LAST_COMBAT_HP_LOSS;
        let high_relative = features.max_hp > 0
            && last_combat.hp_loss * 100 >= features.max_hp * HIGH_LAST_COMBAT_HP_LOSS_PERCENT;
        if high_absolute || high_relative {
            record_issue(
                issue_counts,
                issue_examples,
                record,
                "last_combat:high_hp_loss",
                vec![format!(
                    "hp_loss={} turns={} cards_played={}",
                    last_combat.hp_loss, last_combat.turns, last_combat.cards_played
                )],
            );
        }
    }
}

fn record_issue(
    issue_counts: &mut BTreeMap<String, usize>,
    issue_examples: &mut Vec<BranchOutcomeIssueExampleV1>,
    record: &BranchOutcomeRecordV1,
    issue_key: &str,
    details: Vec<String>,
) {
    *issue_counts.entry(issue_key.to_string()).or_default() += 1;
    if issue_examples.len() >= MAX_ISSUE_EXAMPLES {
        return;
    }
    if issue_examples
        .iter()
        .any(|example| example.issue_key == issue_key)
    {
        return;
    }
    let features = record.state_features.as_ref();
    issue_examples.push(BranchOutcomeIssueExampleV1 {
        issue_key: issue_key.to_string(),
        branch_group: record.branch_group.clone(),
        outcome_class: record.outcome_class.clone(),
        frontier_title: record.frontier_title.clone(),
        act: features.map(|features| features.act),
        floor: features.map(|features| features.floor),
        hp: features.map(|features| features.hp),
        max_hp: features.map(|features| features.max_hp),
        deck_count: features.map(|features| features.deck.deck_count),
        choice_tail: record
            .choice_labels
            .iter()
            .rev()
            .take(4)
            .cloned()
            .collect::<Vec<_>>()
            .into_iter()
            .rev()
            .collect(),
        details,
    });
}

fn is_basic_starter_card_name(name: &str) -> bool {
    matches!(
        name,
        "Strike"
            | "Defend"
            | "Strike_R"
            | "Defend_R"
            | "StrikeG"
            | "DefendG"
            | "StrikeB"
            | "DefendB"
            | "StrikeP"
            | "DefendP"
    )
}

fn render_histogram(entries: &[BranchOutcomeHistogramEntryV1]) -> String {
    entries
        .iter()
        .map(|entry| format!("{}:{}", entry.key, entry.count))
        .collect::<Vec<_>>()
        .join(", ")
}

fn restored_checkpoint_sessions_by_commands(
    checkpoint: &BranchCampaignCheckpointV1,
) -> Result<BTreeMap<Vec<String>, RunControlSession>, String> {
    let mut sessions_by_commands = BTreeMap::new();
    for entry in &checkpoint.sessions {
        let session = checkpoint
            .hydrated_session_checkpoint_v1(entry)?
            .into_session()
            .map_err(|err| format!("failed to restore checkpoint session: {err}"))?;
        sessions_by_commands.insert(entry.commands.clone(), session);
    }
    Ok(sessions_by_commands)
}

fn append_branch_group_records(
    records: &mut Vec<BranchOutcomeRecordV1>,
    report: &BranchCampaignReportV1,
    group: &str,
    branches: &[BranchCampaignBranchV1],
    sessions_by_commands: &BTreeMap<Vec<String>, RunControlSession>,
) {
    for (index, branch) in branches.iter().enumerate() {
        let session = sessions_by_commands.get(&branch.commands);
        records.push(branch_outcome_record(report, group, index, branch, session));
    }
}

fn branch_outcome_record(
    report: &BranchCampaignReportV1,
    group: &str,
    index: usize,
    branch: &BranchCampaignBranchV1,
    session: Option<&RunControlSession>,
) -> BranchOutcomeRecordV1 {
    BranchOutcomeRecordV1 {
        schema_name: BRANCH_OUTCOME_RECORD_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_OUTCOME_RECORD_SCHEMA_VERSION,
        label_role: "campaign_observation_not_teacher".to_string(),
        trainable_as_action_label: false,
        policy_quality_claim: false,
        seed: report.seed,
        run_domain: report.run_domain.clone(),
        report_rounds_completed: report.rounds_completed,
        report_stop_reason: report.stop_reason.clone(),
        branch_group: group.to_string(),
        branch_index: index,
        branch_id: branch.branch_id.clone(),
        branch_status: branch.status.clone(),
        outcome_class: outcome_class_for_status(&branch.status),
        supervision_status: supervision_status_for_status(&branch.status),
        rank_key: branch.rank_key,
        strategic_summary: branch.strategic_summary,
        stop_reason: branch.stop_reason.clone(),
        frontier_title: branch.frontier_title.clone(),
        commands: branch.commands.clone(),
        choice_labels: branch.choice_labels.clone(),
        report_summary: branch.summary.clone(),
        checkpoint_enriched: session.is_some(),
        state_features: session.map(branch_outcome_state_features),
    }
}

fn outcome_class_for_status(status: &BranchCampaignBranchStatusV1) -> BranchOutcomeClassV1 {
    match status {
        BranchCampaignBranchStatusV1::Active => BranchOutcomeClassV1::OngoingActive,
        BranchCampaignBranchStatusV1::Frozen => BranchOutcomeClassV1::OngoingFrozen,
        BranchCampaignBranchStatusV1::TerminalVictory => BranchOutcomeClassV1::TerminalVictory,
        BranchCampaignBranchStatusV1::TerminalDefeat => BranchOutcomeClassV1::TerminalDefeat,
        BranchCampaignBranchStatusV1::Abandoned => BranchOutcomeClassV1::Abandoned,
        BranchCampaignBranchStatusV1::Stuck => BranchOutcomeClassV1::Stuck,
    }
}

fn supervision_status_for_status(
    status: &BranchCampaignBranchStatusV1,
) -> BranchOutcomeSupervisionStatusV1 {
    match status {
        BranchCampaignBranchStatusV1::TerminalVictory
        | BranchCampaignBranchStatusV1::TerminalDefeat => {
            BranchOutcomeSupervisionStatusV1::TerminalOutcome
        }
        BranchCampaignBranchStatusV1::Active | BranchCampaignBranchStatusV1::Frozen => {
            BranchOutcomeSupervisionStatusV1::CensoredOngoing
        }
        BranchCampaignBranchStatusV1::Abandoned | BranchCampaignBranchStatusV1::Stuck => {
            BranchOutcomeSupervisionStatusV1::InterventionOrFailure
        }
    }
}

fn branch_outcome_state_features(session: &RunControlSession) -> BranchOutcomeStateFeaturesV1 {
    let strategy = build_run_strategy_snapshot_from_run_state_v2(&session.run_state);
    let formation = strategy.formation_summary();
    let startup = deck_startup_profile_v1(&session.run_state);
    let (hp, max_hp) = visible_session_hp(session);
    let boss = session
        .run_state
        .boss_key
        .or_else(|| session.run_state.boss_list.first().copied());
    let boss_pressure = boss
        .map(|boss| {
            crate::ai::boss_mechanics_v1::boss_mechanic_pressure_profile_v1(
                &session.run_state,
                boss,
            )
            .summary_labels()
        })
        .unwrap_or_default();

    BranchOutcomeStateFeaturesV1 {
        engine_state: format!("{:?}", session.engine_state),
        act: session.run_state.act_num,
        floor: session.run_state.floor_num,
        hp,
        max_hp,
        gold: session.run_state.gold,
        ascension_level: session.run_state.ascension_level,
        player_class: session.run_state.player_class.to_string(),
        boss: boss.map(|boss| format!("{boss:?}")),
        boss_pressure,
        deck: deck_features(&session.run_state.master_deck),
        relics: session
            .run_state
            .relics
            .iter()
            .map(|relic| format!("{:?}", relic.id))
            .collect(),
        potions: session
            .run_state
            .potions
            .iter()
            .filter_map(|slot| slot.as_ref().map(|potion| format!("{:?}", potion.id)))
            .collect(),
        formation: BranchOutcomeFormationFeaturesV1 {
            stage: format!("{:?}", formation.stage),
            needs: formation
                .needs
                .iter()
                .map(|need| format!("{need:?}"))
                .collect(),
            strengths: formation
                .strengths
                .iter()
                .map(|strength| format!("{strength:?}"))
                .collect(),
        },
        startup: BranchOutcomeStartupFeaturesV1 {
            setup_debt: startup.setup_debt,
            setup_payment: startup.setup_payment,
            effective_setup_payment: startup.effective_setup_payment,
            immediate_survival: startup.immediate_survival,
            payoff_engine: startup.payoff_engine,
            combat_shape_risk: startup.combat_shape_risk,
            strong_draw_count: startup.strong_draw_count,
            effective_strong_draw_count: startup.effective_strong_draw_count,
            exhaust_engine_count: startup.exhaust_engine_count,
            exhaust_payoff_count: startup.exhaust_payoff_count,
            status_generator_count: startup.status_generator_count,
            status_digest_count: startup.status_digest_count,
            persistent_strength_source_count: startup.persistent_strength_source_count,
            temporary_strength_burst_count: startup.temporary_strength_burst_count,
            strength_converter_count: startup.strength_converter_count,
            convertible_strength_source_count: startup.convertible_strength_source_count,
            strength_payoff_count: startup.strength_payoff_count,
            zero_cost_card_count: startup.zero_cost_card_count,
            low_cost_card_count: startup.low_cost_card_count,
            high_cost_card_count: startup.high_cost_card_count,
            has_snecko_eye: startup.has_snecko_eye,
            snecko_random_cost_debt: startup.snecko_random_cost_debt,
            liabilities: startup_liability_labels(&startup),
        },
        last_combat: session.last_combat_baseline().map(|outcome| {
            BranchOutcomeLastCombatFeaturesV1 {
                terminal: format!("{:?}", outcome.terminal),
                start_hp: outcome.start_hp,
                final_hp: outcome.final_hp,
                hp_loss: outcome.hp_loss,
                turns: outcome.turns,
                potions_used: outcome.potions_used,
                cards_played: outcome.cards_played,
            }
        }),
    }
}

fn visible_session_hp(session: &RunControlSession) -> (i32, i32) {
    session
        .active_combat
        .as_ref()
        .map(|active| {
            (
                active.combat_state.entities.player.current_hp,
                active.combat_state.entities.player.max_hp,
            )
        })
        .unwrap_or((session.run_state.current_hp, session.run_state.max_hp))
}

fn deck_features(cards: &[CombatCard]) -> BranchOutcomeDeckFeaturesV1 {
    let mut grouped = BTreeMap::<String, BranchOutcomeCardCountV1>::new();
    let mut attacks = 0usize;
    let mut skills = 0usize;
    let mut powers = 0usize;
    let mut curses = 0usize;
    let mut statuses = 0usize;
    let mut starter_strikes = 0usize;
    let mut starter_defends = 0usize;
    let mut upgraded = 0usize;

    for card in cards {
        let def = get_card_definition(card.id);
        match def.card_type {
            CardType::Attack => attacks += 1,
            CardType::Skill => skills += 1,
            CardType::Power => powers += 1,
            CardType::Curse => curses += 1,
            CardType::Status => statuses += 1,
        }
        if def.tags.contains(&CardTag::StarterStrike) {
            starter_strikes += 1;
        }
        if def.tags.contains(&CardTag::StarterDefend) {
            starter_defends += 1;
        }
        if card.upgrades > 0 {
            upgraded += 1;
        }

        let key = format!("{:?}:{}", card.id, card.upgrades);
        grouped
            .entry(key)
            .and_modify(|entry| entry.count += 1)
            .or_insert_with(|| BranchOutcomeCardCountV1 {
                id: format!("{:?}", card.id),
                name: def.name.to_string(),
                upgrades: card.upgrades,
                card_type: format!("{:?}", def.card_type),
                count: 1,
            });
    }

    BranchOutcomeDeckFeaturesV1 {
        deck_count: cards.len(),
        grouped_cards: grouped.into_values().collect(),
        attacks,
        skills,
        powers,
        curses,
        statuses,
        starter_strikes,
        starter_defends,
        upgraded,
    }
}

fn startup_liability_labels(
    startup: &crate::ai::deck_startup_profile_v1::DeckStartupProfileV1,
) -> Vec<String> {
    let mut labels = Vec::new();
    push_if(
        &mut labels,
        startup.has_setup_debt_high_payment_low,
        "setup_debt_high_payment_low",
    );
    push_if(
        &mut labels,
        startup.has_fnp_duplicate_without_exhaust_engine,
        "fnp_duplicate_without_exhaust",
    );
    push_if(
        &mut labels,
        startup.has_corruption_duplicate_without_payoff,
        "corruption_duplicate_without_payoff",
    );
    push_if(
        &mut labels,
        startup.has_havoc_duplicate_without_payoff,
        "havoc_duplicate_without_payoff",
    );
    push_if(
        &mut labels,
        startup.has_status_generator_saturation_without_digest,
        "status_generator_saturation_without_digest",
    );
    push_if(
        &mut labels,
        startup.has_clash_playability_debt,
        "clash_playability_debt",
    );
    push_if(
        &mut labels,
        startup.has_dual_wield_without_target,
        "dual_wield_without_target",
    );
    push_if(
        &mut labels,
        startup.has_anger_duplicate_without_digest,
        "anger_duplicate_without_digest",
    );
    push_if(
        &mut labels,
        startup.has_strength_payoff_without_strength,
        "strength_payoff_without_strength",
    );
    push_if(
        &mut labels,
        startup.has_rupture_without_self_damage,
        "rupture_without_self_damage",
    );
    push_if(
        &mut labels,
        startup.has_armaments_unupgraded_duplicate,
        "armaments_unupgraded_duplicate",
    );
    push_if(
        &mut labels,
        startup.has_pyramid_unupgraded_apparition,
        "pyramid_unupgraded_apparition",
    );
    push_if(
        &mut labels,
        startup.has_snecko_low_cost_volatility,
        "snecko_low_cost_volatility",
    );
    push_if(
        &mut labels,
        startup.has_snecko_offering_reliability_debt,
        "snecko_offering_reliability_debt",
    );
    labels
}

fn push_if(labels: &mut Vec<String>, condition: bool, label: &str) {
    if condition {
        labels.push(label.to_string());
    }
}

fn histogram_entries(counts: BTreeMap<String, usize>) -> Vec<BranchOutcomeHistogramEntryV1> {
    counts
        .into_iter()
        .map(|(key, count)| BranchOutcomeHistogramEntryV1 { key, count })
        .collect()
}

fn histogram_entries_by_count_desc(
    counts: BTreeMap<String, usize>,
) -> Vec<BranchOutcomeHistogramEntryV1> {
    let mut entries = histogram_entries(counts);
    entries.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.key.cmp(&right.key))
    });
    entries
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn outcome_dataset_analysis_surfaces_structural_issue_counts() {
        let records = vec![
            sample_record(
                "active",
                BranchOutcomeClassV1::OngoingActive,
                sample_features(vec![("Clothesline", "Clothesline", 1, "Attack", 2)], 20),
            ),
            sample_record(
                "frozen",
                BranchOutcomeClassV1::OngoingFrozen,
                sample_features(vec![("Strike", "Strike", 0, "Attack", 3)], 0),
            ),
        ];

        let analysis = analyze_branch_outcome_records_v1(&records);

        assert_eq!(analysis.total_records, 2);
        assert_eq!(analysis.checkpoint_enriched_records, 2);
        assert!(analysis
            .issue_counts
            .iter()
            .any(|entry| entry.key == "duplicate_card:Clothesline" && entry.count == 1));
        assert!(analysis
            .issue_counts
            .iter()
            .any(|entry| entry.key == "last_combat:high_hp_loss" && entry.count == 1));
        assert!(analysis
            .issue_counts
            .iter()
            .any(|entry| entry.key == "starter_debt:strikes_remaining" && entry.count == 2));
    }

    fn sample_record(
        group: &str,
        outcome_class: BranchOutcomeClassV1,
        state_features: BranchOutcomeStateFeaturesV1,
    ) -> BranchOutcomeRecordV1 {
        BranchOutcomeRecordV1 {
            schema_name: BRANCH_OUTCOME_RECORD_SCHEMA_NAME.to_string(),
            schema_version: BRANCH_OUTCOME_RECORD_SCHEMA_VERSION,
            label_role: "campaign_observation_not_teacher".to_string(),
            trainable_as_action_label: false,
            policy_quality_claim: false,
            seed: 1,
            run_domain: BranchCampaignRunDomainV1::default(),
            report_rounds_completed: 1,
            report_stop_reason: "test".to_string(),
            branch_group: group.to_string(),
            branch_index: 0,
            branch_id: group.to_string(),
            branch_status: match outcome_class {
                BranchOutcomeClassV1::OngoingActive => BranchCampaignBranchStatusV1::Active,
                BranchOutcomeClassV1::OngoingFrozen => BranchCampaignBranchStatusV1::Frozen,
                BranchOutcomeClassV1::TerminalVictory => {
                    BranchCampaignBranchStatusV1::TerminalVictory
                }
                BranchOutcomeClassV1::TerminalDefeat => {
                    BranchCampaignBranchStatusV1::TerminalDefeat
                }
                BranchOutcomeClassV1::Abandoned => BranchCampaignBranchStatusV1::Abandoned,
                BranchOutcomeClassV1::Stuck => BranchCampaignBranchStatusV1::Stuck,
            },
            outcome_class,
            supervision_status: BranchOutcomeSupervisionStatusV1::CensoredOngoing,
            rank_key: 0,
            strategic_summary: BranchSignatureCompact::default(),
            stop_reason: String::new(),
            frontier_title: "test".to_string(),
            commands: Vec::new(),
            choice_labels: vec!["choice".to_string()],
            report_summary: None,
            checkpoint_enriched: true,
            state_features: Some(state_features),
        }
    }

    fn sample_features(
        cards: Vec<(&str, &str, u8, &str, usize)>,
        last_combat_hp_loss: i32,
    ) -> BranchOutcomeStateFeaturesV1 {
        let mut grouped_cards = cards
            .into_iter()
            .map(
                |(id, name, upgrades, card_type, count)| BranchOutcomeCardCountV1 {
                    id: id.to_string(),
                    name: name.to_string(),
                    upgrades,
                    card_type: card_type.to_string(),
                    count,
                },
            )
            .collect::<Vec<_>>();
        grouped_cards.push(BranchOutcomeCardCountV1 {
            id: "Strike".to_string(),
            name: "Strike".to_string(),
            upgrades: 0,
            card_type: "Attack".to_string(),
            count: 3,
        });

        BranchOutcomeStateFeaturesV1 {
            engine_state: "Campfire".to_string(),
            act: 3,
            floor: 47,
            hp: 32,
            max_hp: 80,
            gold: 151,
            ascension_level: 0,
            player_class: "Ironclad".to_string(),
            boss: Some("DonuAndDeca".to_string()),
            boss_pressure: vec!["missing:focused_kill_order_plan".to_string()],
            deck: BranchOutcomeDeckFeaturesV1 {
                deck_count: 20,
                grouped_cards,
                attacks: 8,
                skills: 12,
                powers: 0,
                curses: 0,
                statuses: 0,
                starter_strikes: 3,
                starter_defends: 4,
                upgraded: 5,
            },
            relics: Vec::new(),
            potions: Vec::new(),
            formation: BranchOutcomeFormationFeaturesV1 {
                stage: "Transitional".to_string(),
                needs: vec!["Scaling".to_string()],
                strengths: Vec::new(),
            },
            startup: BranchOutcomeStartupFeaturesV1 {
                setup_debt: 1,
                setup_payment: 1,
                effective_setup_payment: 1,
                immediate_survival: 1,
                payoff_engine: 0,
                combat_shape_risk: 0,
                strong_draw_count: 1,
                effective_strong_draw_count: 1,
                exhaust_engine_count: 0,
                exhaust_payoff_count: 0,
                status_generator_count: 0,
                status_digest_count: 0,
                persistent_strength_source_count: 0,
                temporary_strength_burst_count: 0,
                strength_converter_count: 0,
                convertible_strength_source_count: 0,
                strength_payoff_count: 0,
                zero_cost_card_count: 0,
                low_cost_card_count: 10,
                high_cost_card_count: 2,
                has_snecko_eye: false,
                snecko_random_cost_debt: 0,
                liabilities: vec!["strength_payoff_without_strength".to_string()],
            },
            last_combat: Some(BranchOutcomeLastCombatFeaturesV1 {
                terminal: "Win".to_string(),
                start_hp: 52,
                final_hp: 52 - last_combat_hp_loss,
                hp_loss: last_combat_hp_loss,
                turns: 3,
                potions_used: 0,
                cards_played: 12,
            }),
        }
    }
}
