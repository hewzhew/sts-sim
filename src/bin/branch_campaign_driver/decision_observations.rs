use sts_simulator::eval::branch_campaign::{
    BranchCampaignDecisionObservationV1, BranchCampaignReportV1,
};
use sts_simulator::eval::campaign_journal::reward_portfolio_from_journal_event_v1;

use super::campaign_artifacts::read_campaign_report_v1;
use super::command_inputs::{InspectCommandInput, InspectFiltersInput};

pub(super) fn run_decision_observation_inspection(
    input: &InspectCommandInput,
) -> Result<(), String> {
    let path = input.report_path.as_ref().ok_or_else(|| {
        "--inspect-decision-observations requires --inspect-report PATH".to_string()
    })?;
    let report = read_campaign_report_v1(path)?;
    print!(
        "{}",
        render_decision_observation_inspection_v1(
            &report,
            &input.filters,
            input.query.as_deref(),
            input.branch_examples,
        )?
    );
    Ok(())
}

fn render_decision_observation_inspection_v1(
    report: &BranchCampaignReportV1,
    filters: &InspectFiltersInput,
    query: Option<&str>,
    limit: usize,
) -> Result<String, String> {
    let observations = matching_decision_observations_v1(report, filters, query);
    let match_count = observations.len();
    if observations.is_empty() {
        return Err(format!(
            "no decision observations matched filters act={:?} floor={:?} boundary={:?} query={:?}",
            filters.act, filters.floor, filters.boundary, query
        ));
    }
    let selected = if let Some(index) = filters.index {
        vec![observations.get(index).cloned().ok_or_else(|| {
            format!(
                "--inspect-index {} is out of range for {} matching decision observation(s)",
                index,
                observations.len()
            )
        })?]
    } else {
        let shown = limit.max(1).min(observations.len());
        observations.into_iter().take(shown).collect::<Vec<_>>()
    };

    let mut lines = Vec::new();
    lines.push(format!(
        "Decision observations: seed={} source={} matches={} shown={} query={}",
        report.seed,
        if report.journal.is_empty() {
            "round_compat"
        } else {
            "journal"
        },
        match_count,
        selected.len(),
        query.unwrap_or("-")
    ));
    for (display_index, observation) in selected.iter().enumerate() {
        let portfolio = &observation.portfolio;
        lines.push(format!(
            "{}. round={} parent={} A{}F{} {} kept={}/{} depth={} retry={}",
            display_index + 1,
            observation.round,
            observation.parent_index,
            observation.parent_act,
            observation.parent_floor,
            portfolio.boundary_title,
            portfolio.selected_count,
            portfolio.original_count,
            portfolio.depth,
            observation.combat_budget_retry_used
        ));
        lines.push(format!(
            "   parent: {}",
            render_parent_choices_v1(observation)
        ));
        lines.push(format!("   frontier: {}", portfolio.frontier_key));
        lines.push(format!(
            "   kept: {}",
            render_portfolio_entries_v1(&portfolio.selected_options)
        ));
        lines.push(format!(
            "   pruned: {}",
            render_portfolio_entries_v1(&portfolio.pruned_options)
        ));
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn matching_decision_observations_v1(
    report: &BranchCampaignReportV1,
    filters: &InspectFiltersInput,
    query: Option<&str>,
) -> Vec<BranchCampaignDecisionObservationV1> {
    let normalized_query = query
        .map(normalize_query_v1)
        .filter(|query| !query.is_empty());
    decision_observations_for_report_v1(report)
        .into_iter()
        .filter(|observation| {
            filters.act.is_none_or(|act| observation.parent_act == act)
                && filters
                    .floor
                    .is_none_or(|floor| observation.parent_floor == floor)
                && filters.boundary.as_ref().is_none_or(|boundary| {
                    normalize_query_v1(&observation.portfolio.boundary_title)
                        .contains(&normalize_query_v1(boundary))
                })
                && normalized_query
                    .as_ref()
                    .is_none_or(|query| observation_search_text_v1(observation).contains(query))
        })
        .collect()
}

fn decision_observations_for_report_v1(
    report: &BranchCampaignReportV1,
) -> Vec<BranchCampaignDecisionObservationV1> {
    if !report.journal.is_empty() {
        return report
            .journal
            .events
            .iter()
            .filter_map(|event| {
                let portfolio = reward_portfolio_from_journal_event_v1(event)?;
                Some(BranchCampaignDecisionObservationV1 {
                    round: event.round,
                    parent_index: event.branch_index,
                    parent_branch_id: event.branch_id.clone(),
                    parent_frontier_title: event.branch_frontier_title.clone(),
                    parent_act: event.act,
                    parent_floor: event.floor,
                    parent_choices: event.branch_choices.clone(),
                    parent_commands: event.branch_commands.clone(),
                    combat_budget_retry_used: event.combat_budget_retry_used,
                    portfolio,
                })
            })
            .collect();
    }
    report
        .rounds
        .iter()
        .flat_map(|round| round.decision_observations.iter().cloned())
        .collect()
}

fn observation_search_text_v1(observation: &BranchCampaignDecisionObservationV1) -> String {
    let portfolio = &observation.portfolio;
    let mut text = Vec::new();
    text.push(observation.parent_branch_id.as_str());
    text.push(observation.parent_frontier_title.as_str());
    text.push(portfolio.boundary_title.as_str());
    text.push(portfolio.frontier_key.as_str());
    for choice in &observation.parent_choices {
        text.push(choice.as_str());
    }
    for command in &observation.parent_commands {
        text.push(command.as_str());
    }
    for entry in portfolio
        .selected_options
        .iter()
        .chain(portfolio.pruned_options.iter())
    {
        text.push(entry.command.as_str());
        text.push(entry.label.as_str());
        text.push(entry.semantic_class.as_str());
    }
    normalize_query_v1(&text.join(" "))
}

fn normalize_query_v1(text: &str) -> String {
    text.to_ascii_lowercase().replace([' ', '_', '-'], "")
}

fn render_parent_choices_v1(observation: &BranchCampaignDecisionObservationV1) -> String {
    if observation.parent_choices.is_empty() {
        "-".to_string()
    } else {
        observation.parent_choices.join(" -> ")
    }
}

fn render_portfolio_entries_v1(
    entries: &[sts_simulator::eval::branch_experiment::BranchExperimentRewardOptionPortfolioEntryV1],
) -> String {
    if entries.is_empty() {
        return "-".to_string();
    }
    entries
        .iter()
        .map(|entry| format!("{} [{}]", entry.label, entry.semantic_class))
        .collect::<Vec<_>>()
        .join(" | ")
}
