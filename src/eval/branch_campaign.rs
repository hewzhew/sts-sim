use crate::eval::branch_experiment::{
    run_branch_experiment_v1, BranchExperimentBranchReportV1, BranchExperimentBranchStatusV1,
    BranchExperimentConfigV1, BranchExperimentReportV1, BranchExperimentStrategyRequestV1,
};
use crate::eval::branch_experiment_retention::BranchRetentionBudgetProfileV1;
use crate::eval::run_control::{RunControlHpLossLimit, RunControlSearchCombatOptions};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const BRANCH_CAMPAIGN_SCHEMA_NAME: &str = "BranchCampaignV1";
pub const BRANCH_CAMPAIGN_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, PartialEq)]
pub struct BranchCampaignConfigV1 {
    pub seed: u64,
    pub ascension_level: u8,
    pub player_class: &'static str,
    pub final_act: bool,
    pub max_rounds: usize,
    pub round_depth: usize,
    pub max_active: usize,
    pub max_frozen: usize,
    pub max_branches_per_active: usize,
    pub retention_budget_profile: BranchRetentionBudgetProfileV1,
    pub max_reward_options_per_branch: Option<usize>,
    pub max_campfire_options_per_branch: usize,
    pub auto_max_operations: usize,
    pub experiment_wall_ms: Option<u64>,
    pub search_max_nodes: Option<usize>,
    pub search_wall_ms: Option<u64>,
    pub search_max_hp_loss: Option<RunControlHpLossLimit>,
    pub search_options: RunControlSearchCombatOptions,
    pub include_event_reward_skip: bool,
    pub prefix_commands: Vec<String>,
}

impl Default for BranchCampaignConfigV1 {
    fn default() -> Self {
        Self {
            seed: 1,
            ascension_level: 0,
            player_class: "Ironclad",
            final_act: false,
            max_rounds: 8,
            round_depth: 1,
            max_active: 8,
            max_frozen: 32,
            max_branches_per_active: 12,
            retention_budget_profile: BranchRetentionBudgetProfileV1::Package,
            max_reward_options_per_branch: Some(2),
            max_campfire_options_per_branch: 3,
            auto_max_operations: 128,
            experiment_wall_ms: Some(10_000),
            search_max_nodes: Some(50_000),
            search_wall_ms: Some(200),
            search_max_hp_loss: None,
            search_options: RunControlSearchCombatOptions::default(),
            include_event_reward_skip: false,
            prefix_commands: Vec::new(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum BranchCampaignBranchStatusV1 {
    Active,
    Frozen,
    TerminalVictory,
    TerminalDefeat,
    Stuck,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignBranchSummaryV1 {
    pub act: u8,
    pub floor: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub deck_count: usize,
    pub formation_stage: String,
    pub formation_strengths: Vec<String>,
    pub formation_needs: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignBranchV1 {
    pub branch_id: String,
    pub commands: Vec<String>,
    pub choice_labels: Vec<String>,
    pub summary: Option<BranchCampaignBranchSummaryV1>,
    pub frontier_title: String,
    pub status: BranchCampaignBranchStatusV1,
    pub rank_key: i32,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignSelectionV1 {
    pub active: Vec<BranchCampaignBranchV1>,
    pub frozen: Vec<BranchCampaignBranchV1>,
    pub victories: Vec<BranchCampaignBranchV1>,
    pub dead: Vec<BranchCampaignBranchV1>,
    pub stuck: Vec<BranchCampaignBranchV1>,
    pub discarded_count: usize,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignStrategyRequestV1 {
    pub kind: String,
    pub boundary_title: String,
    pub branch_count: usize,
    pub examples: Vec<String>,
    pub suggested_action: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignRoundSummaryV1 {
    pub round: usize,
    pub started_active: usize,
    pub produced_branches: usize,
    pub active_after: usize,
    pub frozen_added: usize,
    pub dead_added: usize,
    pub victories_added: usize,
    pub stuck_added: usize,
    pub discarded_added: usize,
    pub explored_branch_points: usize,
    pub wall_limit_hit: bool,
    pub branch_limit_hit: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct BranchCampaignReportV1 {
    pub schema_name: String,
    pub schema_version: u32,
    pub seed: u64,
    pub rounds_completed: usize,
    pub stop_reason: String,
    pub active: Vec<BranchCampaignBranchV1>,
    pub frozen: Vec<BranchCampaignBranchV1>,
    pub victories: Vec<BranchCampaignBranchV1>,
    pub dead: Vec<BranchCampaignBranchV1>,
    pub stuck: Vec<BranchCampaignBranchV1>,
    pub discarded_count: usize,
    pub strategy_requests: Vec<BranchCampaignStrategyRequestV1>,
    pub rounds: Vec<BranchCampaignRoundSummaryV1>,
}

pub fn run_branch_campaign_v1(
    config: &BranchCampaignConfigV1,
) -> Result<BranchCampaignReportV1, String> {
    let mut active = vec![root_campaign_branch_v1()];
    let mut frozen = Vec::new();
    let mut victories = Vec::new();
    let mut dead = Vec::new();
    let mut stuck = Vec::new();
    let mut discarded_count = 0usize;
    let mut strategy_requests = Vec::new();
    let mut rounds = Vec::new();
    let mut stop_reason = "max_rounds".to_string();

    for round in 0..config.max_rounds {
        if active.is_empty() {
            stop_reason = "no_active_branch".to_string();
            break;
        }
        let parents = std::mem::take(&mut active);
        let started_active = parents.len();
        let mut candidates = Vec::new();
        let mut round_strategy_requests = Vec::new();
        let mut explored_branch_points = 0usize;
        let mut wall_limit_hit = false;
        let mut branch_limit_hit = false;

        for parent in parents {
            let report = run_campaign_parent_round_v1(config, &parent)?;
            explored_branch_points =
                explored_branch_points.saturating_add(report.explored_branch_points);
            wall_limit_hit |= report.wall_limit_hit;
            branch_limit_hit |= report.branch_limit_hit || report.frontier_group_limit_hit;
            round_strategy_requests.extend(report.strategy_requests.iter().cloned());
            candidates.extend(
                report
                    .branches
                    .iter()
                    .map(|branch| campaign_branch_from_report_branch_v1(&parent, branch)),
            );
        }

        strategy_requests = merge_campaign_strategy_requests_v1(round_strategy_requests);
        let produced_branches = candidates.len();
        let selected =
            select_campaign_branches_v1(candidates, config.max_active, config.max_frozen);
        let frozen_added = append_limited_frozen_v1(
            &mut frozen,
            selected.frozen,
            config.max_frozen,
            &mut discarded_count,
        );
        discarded_count = discarded_count.saturating_add(selected.discarded_count);
        let round_summary = BranchCampaignRoundSummaryV1 {
            round,
            started_active,
            produced_branches,
            active_after: selected.active.len(),
            frozen_added,
            dead_added: selected.dead.len(),
            victories_added: selected.victories.len(),
            stuck_added: selected.stuck.len(),
            discarded_added: selected.discarded_count,
            explored_branch_points,
            wall_limit_hit,
            branch_limit_hit,
        };
        active = selected.active;
        victories.extend(selected.victories);
        dead.extend(selected.dead);
        stuck.extend(selected.stuck);
        rounds.push(round_summary);

        if !strategy_requests.is_empty() {
            stop_reason = "needs_intervention".to_string();
            break;
        }
        if !victories.is_empty() {
            stop_reason = "victory_found".to_string();
            break;
        }
        if !stuck.is_empty() {
            stop_reason = "stuck".to_string();
            break;
        }
        if produced_branches == 0 {
            stop_reason = "no_progress".to_string();
            break;
        }
    }

    Ok(BranchCampaignReportV1 {
        schema_name: BRANCH_CAMPAIGN_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_SCHEMA_VERSION,
        seed: config.seed,
        rounds_completed: rounds.len(),
        stop_reason,
        active,
        frozen,
        victories,
        dead,
        stuck,
        discarded_count,
        strategy_requests,
        rounds,
    })
}

pub fn render_branch_campaign_compact_v1(
    report: &BranchCampaignReportV1,
    branch_examples: usize,
) -> String {
    let mut lines = Vec::new();
    lines.push(format!(
        "{} seed={} rounds={} stop={}",
        report.schema_name, report.seed, report.rounds_completed, report.stop_reason
    ));
    lines.push(format!(
        "Active {} | Frozen {} | Dead {} | Victories {} | Stuck {} | Discarded {}",
        report.active.len(),
        report.frozen.len(),
        report.dead.len(),
        report.victories.len(),
        report.stuck.len(),
        report.discarded_count
    ));
    if let Some(round) = report.rounds.last() {
        lines.push(format!(
            "Last round: started={} produced={} branch_points={} active_after={} frozen_added={} discarded_added={} limits=[{}{}]",
            round.started_active,
            round.produced_branches,
            round.explored_branch_points,
            round.active_after,
            round.frozen_added,
            round.discarded_added,
            if round.branch_limit_hit { "branch" } else { "" },
            if round.wall_limit_hit {
                if round.branch_limit_hit { ",wall" } else { "wall" }
            } else {
                ""
            }
        ));
    }
    if report.stop_reason == "max_rounds"
        && (!report.active.is_empty() || !report.frozen.is_empty())
    {
        lines.push(
            "Next: budget ended; use .\\tools\\campaign.ps1 -More or raise -MaxRounds to keep exploring this seed"
                .to_string(),
        );
    }
    if !report.strategy_requests.is_empty() {
        lines.push(String::new());
        lines.push("Needs intervention:".to_string());
        for request in report.strategy_requests.iter().take(4) {
            lines.push(format!(
                "  {} | {} | branches={}",
                request.kind, request.boundary_title, request.branch_count
            ));
            if let Some(example) = request.examples.first() {
                lines.push(format!("    example: {example}"));
            }
            lines.push(format!("    suggested: {}", request.suggested_action));
            if let Some(next_step) = campaign_strategy_next_step_v1(&request.kind) {
                lines.push(format!("    next: {next_step}"));
            }
        }
    }
    if !report.active.is_empty() {
        lines.push(String::new());
        lines.push("Top active:".to_string());
        for (index, branch) in report.active.iter().take(branch_examples).enumerate() {
            lines.push(format!(
                "  {}. {} | {} | choices: {}",
                index + 1,
                render_campaign_branch_state(branch),
                branch.frontier_title,
                render_choice_path(&branch.choice_labels)
            ));
        }
    }
    if !report.frozen.is_empty() {
        lines.push(String::new());
        lines.push("Frozen examples:".to_string());
        for (index, branch) in report.frozen.iter().take(branch_examples).enumerate() {
            lines.push(format!(
                "  {}. {} | {} | choices: {}",
                index + 1,
                render_campaign_branch_state(branch),
                branch.frontier_title,
                render_choice_path(&branch.choice_labels)
            ));
        }
    }
    lines.join("\n")
}

fn root_campaign_branch_v1() -> BranchCampaignBranchV1 {
    BranchCampaignBranchV1 {
        branch_id: "root".to_string(),
        commands: Vec::new(),
        choice_labels: Vec::new(),
        summary: None,
        frontier_title: "start".to_string(),
        status: BranchCampaignBranchStatusV1::Active,
        rank_key: 0,
    }
}

fn run_campaign_parent_round_v1(
    config: &BranchCampaignConfigV1,
    parent: &BranchCampaignBranchV1,
) -> Result<BranchExperimentReportV1, String> {
    let mut prefix_commands = config.prefix_commands.clone();
    prefix_commands.extend(campaign_replay_commands_for_path_v1(&parent.commands));
    run_branch_experiment_v1(&BranchExperimentConfigV1 {
        seed: config.seed,
        ascension_level: config.ascension_level,
        player_class: config.player_class,
        final_act: config.final_act,
        max_branches: config.max_branches_per_active,
        retention_budget_profile: config.retention_budget_profile,
        max_reward_options_per_branch: config.max_reward_options_per_branch,
        max_campfire_options_per_branch: Some(config.max_campfire_options_per_branch),
        max_depth: config.round_depth,
        auto_max_operations: config.auto_max_operations,
        experiment_wall_ms: config.experiment_wall_ms,
        search_max_nodes: config.search_max_nodes,
        search_wall_ms: config.search_wall_ms,
        search_max_hp_loss: config.search_max_hp_loss,
        search_options: config.search_options.clone(),
        include_skip: true,
        include_event_reward_skip: config.include_event_reward_skip,
        prefix_commands,
        ..BranchExperimentConfigV1::default()
    })
}

pub fn campaign_replay_commands_for_path_v1(commands: &[String]) -> Vec<String> {
    let mut replay = Vec::with_capacity(commands.len().saturating_mul(2));
    for command in commands {
        replay.push("ar".to_string());
        replay.push(command.clone());
    }
    replay
}

fn append_limited_frozen_v1(
    frozen: &mut Vec<BranchCampaignBranchV1>,
    new_frozen: Vec<BranchCampaignBranchV1>,
    max_frozen: usize,
    discarded_count: &mut usize,
) -> usize {
    let mut added = 0usize;
    for branch in new_frozen {
        if frozen.len() < max_frozen {
            frozen.push(branch);
            added = added.saturating_add(1);
        } else {
            *discarded_count = discarded_count.saturating_add(1);
        }
    }
    added
}

fn merge_campaign_strategy_requests_v1(
    requests: Vec<BranchExperimentStrategyRequestV1>,
) -> Vec<BranchCampaignStrategyRequestV1> {
    let mut merged = BTreeMap::<(String, String), BranchCampaignStrategyRequestV1>::new();
    for request in requests {
        let key = (request.kind.clone(), request.boundary_title.clone());
        merged
            .entry(key)
            .and_modify(|existing| {
                existing.branch_count = existing.branch_count.saturating_add(request.branch_count);
                for example in &request.examples {
                    if existing.examples.len() < 4 && !existing.examples.contains(example) {
                        existing.examples.push(example.clone());
                    }
                }
            })
            .or_insert_with(|| BranchCampaignStrategyRequestV1 {
                kind: request.kind,
                boundary_title: request.boundary_title,
                branch_count: request.branch_count,
                examples: request.examples.into_iter().take(4).collect(),
                suggested_action: request.suggested_action,
            });
    }
    merged.into_values().collect()
}

pub fn select_campaign_branches_v1(
    branches: Vec<BranchCampaignBranchV1>,
    max_active: usize,
    max_frozen: usize,
) -> BranchCampaignSelectionV1 {
    let mut active_candidates = Vec::new();
    let mut selection = BranchCampaignSelectionV1::default();
    for branch in branches {
        match branch.status {
            BranchCampaignBranchStatusV1::TerminalVictory => selection.victories.push(branch),
            BranchCampaignBranchStatusV1::TerminalDefeat => selection.dead.push(branch),
            BranchCampaignBranchStatusV1::Stuck => selection.stuck.push(branch),
            BranchCampaignBranchStatusV1::Frozen | BranchCampaignBranchStatusV1::Active => {
                active_candidates.push(branch)
            }
        }
    }

    active_candidates.sort_by(|left, right| {
        right
            .rank_key
            .cmp(&left.rank_key)
            .then_with(|| branch_progress_key(right).cmp(&branch_progress_key(left)))
            .then_with(|| left.branch_id.cmp(&right.branch_id))
    });

    for (index, mut branch) in active_candidates.into_iter().enumerate() {
        if index < max_active {
            branch.status = BranchCampaignBranchStatusV1::Active;
            selection.active.push(branch);
        } else if selection.frozen.len() < max_frozen {
            branch.status = BranchCampaignBranchStatusV1::Frozen;
            selection.frozen.push(branch);
        } else {
            selection.discarded_count = selection.discarded_count.saturating_add(1);
        }
    }
    selection
}

fn render_campaign_branch_state(branch: &BranchCampaignBranchV1) -> String {
    branch
        .summary
        .as_ref()
        .map(|summary| {
            format!(
                "A{}F{} HP {}/{} gold {} deck {}",
                summary.act,
                summary.floor,
                summary.hp,
                summary.max_hp,
                summary.gold,
                summary.deck_count
            )
        })
        .unwrap_or_else(|| "start".to_string())
}

fn render_choice_path(labels: &[String]) -> String {
    if labels.is_empty() {
        "-".to_string()
    } else {
        labels.join(" -> ")
    }
}

fn campaign_strategy_next_step_v1(kind: &str) -> Option<&'static str> {
    match kind {
        "combat_hp_loss_policy" | "combat_manual_or_budget" => Some(
            "try a deeper same-seed run, e.g. .\\tools\\campaign.ps1 -More; if it still stops, inspect or hand-play that combat",
        ),
        "card_reward_policy_gap" => {
            Some("decide whether this reward family should be branched, auto-picked, skipped, or kept for human judgment")
        }
        "event_strategy" => {
            Some("write a narrow event rule or choose one branch manually, then rerun the campaign")
        }
        "campfire_strategy" => {
            Some("choose rest/smith/recall priority for this deck state, then encode only the stable part")
        }
        "boss_relic_strategy" => {
            Some("choose the boss relic package direction, then keep the other branches frozen if still plausible")
        }
        "shop_strategy" => {
            Some("choose buy/remove/leave priorities; avoid expanding every affordable purchase blindly")
        }
        "reward_claim_policy" => {
            Some("decide which remaining rewards are safe automatic claims before continuing")
        }
        "route_policy_gap" => {
            Some("adjust route policy or provide a one-step map choice before continuing")
        }
        "engineering_issue" => {
            Some("fix the command/state bug before trusting this campaign branch")
        }
        _ => None,
    }
}

pub fn campaign_branch_from_report_branch_v1(
    parent: &BranchCampaignBranchV1,
    branch: &BranchExperimentBranchReportV1,
) -> BranchCampaignBranchV1 {
    let mut commands = parent.commands.clone();
    commands.extend(branch.choices.iter().map(|choice| choice.command.clone()));
    let mut choice_labels = parent.choice_labels.clone();
    choice_labels.extend(branch.choices.iter().map(campaign_choice_label_v1));
    BranchCampaignBranchV1 {
        branch_id: branch.branch_id.clone(),
        commands,
        choice_labels,
        summary: Some(campaign_summary_from_report_branch_v1(branch)),
        frontier_title: branch.summary.boundary_title.clone(),
        status: campaign_status_from_report_status(branch.status),
        rank_key: branch.rank_key,
    }
}

fn branch_progress_key(branch: &BranchCampaignBranchV1) -> (u8, i32, i32) {
    branch
        .summary
        .as_ref()
        .map(|summary| (summary.act, summary.floor, summary.hp))
        .unwrap_or((0, 0, 0))
}

fn campaign_choice_label_v1(
    choice: &crate::eval::branch_experiment::BranchExperimentChoiceV1,
) -> String {
    if choice.effect_label.is_empty() {
        choice.label.clone()
    } else {
        choice.effect_label.clone()
    }
}

fn campaign_summary_from_report_branch_v1(
    branch: &BranchExperimentBranchReportV1,
) -> BranchCampaignBranchSummaryV1 {
    BranchCampaignBranchSummaryV1 {
        act: branch.summary.act,
        floor: branch.summary.floor,
        hp: branch.summary.hp,
        max_hp: branch.summary.max_hp,
        gold: branch.summary.gold,
        deck_count: branch.summary.deck_count,
        formation_stage: format!("{:?}", branch.summary.formation_stage),
        formation_strengths: branch
            .summary
            .formation_strengths
            .iter()
            .map(|value| format!("{value:?}"))
            .collect(),
        formation_needs: branch
            .summary
            .formation_needs
            .iter()
            .map(|value| format!("{value:?}"))
            .collect(),
    }
}

fn campaign_status_from_report_status(
    status: BranchExperimentBranchStatusV1,
) -> BranchCampaignBranchStatusV1 {
    match status {
        BranchExperimentBranchStatusV1::Active => BranchCampaignBranchStatusV1::Active,
        BranchExperimentBranchStatusV1::TerminalVictory => {
            BranchCampaignBranchStatusV1::TerminalVictory
        }
        BranchExperimentBranchStatusV1::TerminalDefeat | BranchExperimentBranchStatusV1::Pruned => {
            BranchCampaignBranchStatusV1::TerminalDefeat
        }
        BranchExperimentBranchStatusV1::NeedsHumanBoundary
        | BranchExperimentBranchStatusV1::Failed => BranchCampaignBranchStatusV1::Stuck,
    }
}

#[cfg(test)]
mod tests;
