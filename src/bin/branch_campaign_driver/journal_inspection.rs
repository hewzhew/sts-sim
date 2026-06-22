use sts_simulator::ai::route_planner_v1::{MapDecisionPacketV1, RouteMoveCandidateV1};
use sts_simulator::eval::branch_campaign::{BranchCampaignBranchV1, BranchCampaignReportV1};
use sts_simulator::eval::campaign_journal::{
    CampaignJournalCandidateAdmissionReasonCategoryV1,
    CampaignJournalCandidateAdmissionReasonCodeV1, CampaignJournalCandidateAdmissionStatusV1,
    CampaignJournalCandidateDispositionV1, CampaignJournalCandidateV1,
    CampaignJournalEventPayloadV1, CampaignJournalEventV1, CampaignJournalRouteCandidateV1,
};
use sts_simulator::eval::decision_path::{
    decision_path_command_is_coordinate_v1, DecisionPathEnvelopeV1,
};

use super::campaign_artifacts::read_campaign_report_v1;
use super::command_inputs::{InspectCommandInput, InspectFiltersInput};

pub(super) fn run_campaign_journal_inspection(input: &InspectCommandInput) -> Result<(), String> {
    let path = input
        .report_path
        .as_ref()
        .ok_or_else(|| "--inspect-journal requires --inspect-report PATH".to_string())?;
    let report = read_campaign_report_v1(path)?;
    print!(
        "{}",
        render_campaign_journal_inspection_v1(
            &report,
            &input.filters,
            input.query.as_deref(),
            input.branch_examples,
        )?
    );
    Ok(())
}

pub(super) fn run_campaign_lineage_decision_inspection(
    input: &InspectCommandInput,
) -> Result<(), String> {
    let path = input
        .report_path
        .as_ref()
        .ok_or_else(|| "--inspect-lineage-decisions requires --inspect-report PATH".to_string())?;
    let report = read_campaign_report_v1(path)?;
    print!(
        "{}",
        render_campaign_lineage_decision_audit_v1(
            &report,
            &input.filters,
            input.query.as_deref(),
            input.branch_examples,
        )?
    );
    Ok(())
}

fn render_campaign_journal_inspection_v1(
    report: &BranchCampaignReportV1,
    filters: &InspectFiltersInput,
    query: Option<&str>,
    limit: usize,
) -> Result<String, String> {
    if report.journal.is_empty() {
        return Err("campaign report has no CampaignJournal events".to_string());
    }
    let events = matching_journal_events_v1(report, filters, query);
    let match_count = events.len();
    if events.is_empty() {
        return Err(format!(
            "no journal events matched filters act={:?} floor={:?} boundary={:?} query={:?}",
            filters.act, filters.floor, filters.boundary, query
        ));
    }

    let selected = if let Some(index) = filters.index {
        vec![events.get(index).copied().ok_or_else(|| {
            format!(
                "--inspect-index {} is out of range for {} matching journal event(s)",
                index,
                events.len()
            )
        })?]
    } else {
        let shown = limit.max(1).min(events.len());
        events.into_iter().take(shown).collect::<Vec<_>>()
    };

    let mut lines = Vec::new();
    lines.push(format!(
        "Campaign journal: seed={} events={} matches={} shown={} query={}",
        report.seed,
        report.journal.events.len(),
        match_count,
        selected.len(),
        query.unwrap_or("-")
    ));
    for (display_index, event) in selected.iter().enumerate() {
        lines.push(format!(
            "{}. {} id={} round={} parent={} A{}F{} {} depth={} candidates={}{} retry={}",
            display_index + 1,
            journal_event_type_v1(event),
            event.event_id,
            event.round,
            event.branch_index,
            event.act,
            event.floor,
            journal_event_boundary_title_v1(event),
            journal_event_depth_v1(event),
            journal_event_candidates_v1(event).len(),
            journal_event_extra_summary_v1(event),
            event.combat_budget_retry_used
        ));
        lines.push(format!(
            "   parent: {}",
            if event.branch_choices.is_empty() {
                "-".to_string()
            } else {
                event.branch_choices.join(" -> ")
            }
        ));
        lines.push(format!(
            "   frontier: {}",
            journal_event_frontier_key_v1(event)
        ));
        lines.push(format!(
            "   candidates: {}",
            render_journal_event_candidates_v1(event, limit)
        ));
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn render_campaign_lineage_decision_audit_v1(
    report: &BranchCampaignReportV1,
    filters: &InspectFiltersInput,
    query: Option<&str>,
    limit: usize,
) -> Result<String, String> {
    if report.journal.is_empty() {
        return Err("campaign report has no CampaignJournal events".to_string());
    }
    if let Some(query) = query.filter(|query| !normalize_query_v1(query).is_empty()) {
        if let Some(rendered) =
            render_campaign_lineage_candidate_query_v1(report, filters, query, limit)
        {
            return Ok(rendered);
        }
    }
    let branches = matching_lineage_audit_branches_v1(report, filters, query);
    if branches.is_empty() {
        return Err(format!(
            "no report branches matched filters act={:?} floor={:?} boundary={:?} hp={:?} query={:?}",
            filters.act, filters.floor, filters.boundary, filters.hp, query
        ));
    }
    let inspect_index = filters.index.unwrap_or(0);
    let Some(target) = branches.get(inspect_index) else {
        return Err(format!(
            "--inspect-index {} is out of range for {} matching branch(es)",
            inspect_index,
            branches.len()
        ));
    };

    let lineage_events = lineage_candidate_pool_events_v1(report, target.branch);
    let missing_commands = lineage_missing_decision_commands_v1(report, target.branch);
    let mut lines = Vec::new();
    lines.push(format!(
        "Branch lineage decision audit: seed={} branch_matches={} selected={} events={} missing_steps={}",
        report.seed,
        branches.len(),
        target.label,
        lineage_events.len(),
        missing_commands.len()
    ));
    lines.push(format!(
        "branch: {} {} A{}F{} HP {}/{} | {}",
        target.label,
        target.branch.branch_id,
        target
            .branch
            .summary
            .as_ref()
            .map(|summary| summary.act)
            .unwrap_or(0),
        target
            .branch
            .summary
            .as_ref()
            .map(|summary| summary.floor)
            .unwrap_or(0),
        target
            .branch
            .summary
            .as_ref()
            .map(|summary| summary.hp)
            .unwrap_or(0),
        target
            .branch
            .summary
            .as_ref()
            .map(|summary| summary.max_hp)
            .unwrap_or(0),
        target.branch.frontier_title
    ));
    lines.push(format!(
        "choices: {}",
        if target.branch.choice_labels.is_empty() {
            "-".to_string()
        } else {
            target.branch.choice_labels.join(" -> ")
        }
    ));
    if !missing_commands.is_empty() {
        lines.push(format!(
            "missing_journal_context: {}",
            missing_commands.join(" -> ")
        ));
    }
    for (index, event) in lineage_events.iter().enumerate() {
        let parent_depth =
            lineage_event_parent_command_count_v1(event, &target.branch.commands).unwrap_or(0);
        let chosen_command = lineage_event_chosen_command_v1(event, &target.branch.commands);
        let chosen = render_lineage_chosen_candidate_v1(event, chosen_command);
        lines.push(format!(
            "{}. {} round={} A{}F{} {} parent_depth={} chosen={}",
            index + 1,
            journal_event_type_v1(event),
            event.round,
            event.act,
            event.floor,
            journal_event_boundary_title_v1(event),
            parent_depth,
            chosen
        ));
        lines.push(format!(
            "   parent: {}",
            if event.branch_choices.is_empty() {
                "-".to_string()
            } else {
                event.branch_choices.join(" -> ")
            }
        ));
        lines.push(format!(
            "   candidates: {}",
            render_lineage_journal_candidates_v1(event, chosen_command, limit)
        ));
    }
    Ok(format!("{}\n", lines.join("\n")))
}

fn render_campaign_lineage_candidate_query_v1(
    report: &BranchCampaignReportV1,
    filters: &InspectFiltersInput,
    query: &str,
    limit: usize,
) -> Option<String> {
    let normalized_query = normalize_query_v1(query);
    let branches = matching_lineage_audit_branches_v1(report, filters, None);
    let mut matches = Vec::new();
    'branch_loop: for branch_ref in branches {
        for event in lineage_candidate_pool_events_v1(report, branch_ref.branch) {
            let matching_candidates =
                lineage_query_matching_candidate_renders_v1(event, &normalized_query);
            if matching_candidates.is_empty() {
                continue;
            }
            let parent_depth =
                lineage_event_parent_command_count_v1(event, &branch_ref.branch.commands)
                    .unwrap_or(0);
            let chosen_command =
                lineage_event_chosen_command_v1(event, &branch_ref.branch.commands);
            matches.push(LineageCandidateQueryMatchV1 {
                branch_label: branch_ref.label.clone(),
                branch_summary: render_lineage_query_branch_summary_v1(branch_ref.branch),
                branch_choices: render_lineage_query_branch_choices_v1(branch_ref.branch),
                event,
                parent_depth,
                chosen: render_lineage_chosen_candidate_v1(event, chosen_command),
                matching_candidates,
            });
            if matches.len() >= limit.max(1) {
                break 'branch_loop;
            }
        }
    }
    if matches.is_empty() {
        return None;
    }
    let mut lines = Vec::new();
    lines.push(format!(
        "Branch lineage candidate query: seed={} query={} shown={} limit={}",
        report.seed,
        query,
        matches.len(),
        limit.max(1)
    ));
    for (index, matched) in matches.iter().enumerate() {
        lines.push(format!(
            "{}. branch={} {} | {} round={} A{}F{} parent_depth={} chosen={}",
            index + 1,
            matched.branch_label,
            matched.branch_summary,
            journal_event_type_v1(matched.event),
            matched.event.round,
            matched.event.act,
            matched.event.floor,
            matched.parent_depth,
            matched.chosen,
        ));
        if !matched.branch_choices.is_empty() {
            lines.push(format!("   choices: {}", matched.branch_choices));
        }
        lines.push(format!(
            "   parent: {}",
            if matched.event.branch_choices.is_empty() {
                "-".to_string()
            } else {
                matched.event.branch_choices.join(" -> ")
            }
        ));
        lines.push(format!(
            "   match={}",
            matched.matching_candidates.join(" | match=")
        ));
    }
    Some(format!("{}\n", lines.join("\n")))
}

struct LineageCandidateQueryMatchV1<'a> {
    branch_label: String,
    branch_summary: String,
    branch_choices: String,
    event: &'a CampaignJournalEventV1,
    parent_depth: usize,
    chosen: String,
    matching_candidates: Vec<String>,
}

fn lineage_candidate_matches_query_v1(
    event: &CampaignJournalEventV1,
    candidate_index: usize,
    candidate: &CampaignJournalCandidateV1,
    normalized_query: &str,
) -> bool {
    normalize_query_v1(&candidate.label).contains(normalized_query)
        || normalize_query_v1(&candidate.command).contains(normalized_query)
        || normalize_query_v1(&candidate.semantic_class).contains(normalized_query)
        || normalize_query_v1(&render_journal_candidate_v1(candidate)).contains(normalized_query)
        || render_journal_event_candidate_at_v1(event, candidate_index)
            .is_some_and(|rendered| normalize_query_v1(&rendered).contains(normalized_query))
        || route_candidate_search_text_at_v1(event, candidate_index)
            .is_some_and(|text| normalize_query_v1(&text).contains(normalized_query))
}

fn lineage_query_matching_candidate_renders_v1(
    event: &CampaignJournalEventV1,
    normalized_query: &str,
) -> Vec<String> {
    journal_event_candidates_v1(event)
        .iter()
        .enumerate()
        .filter(|(index, candidate)| {
            lineage_candidate_matches_query_v1(event, *index, candidate, normalized_query)
        })
        .filter_map(|(index, candidate)| {
            render_journal_event_candidate_at_v1(event, index)
                .or_else(|| Some(format!("{} {{{}}}", candidate.label, candidate.command)))
        })
        .collect()
}

fn route_candidate_search_text_at_v1(
    event: &CampaignJournalEventV1,
    candidate_index: usize,
) -> Option<String> {
    match &event.payload {
        CampaignJournalEventPayloadV1::RouteCandidatePool {
            map_decision_packet: Some(packet),
            ..
        } => packet
            .candidates
            .get(candidate_index)
            .map(route_candidate_search_terms_v1),
        CampaignJournalEventPayloadV1::RouteCandidatePool {
            route_candidates, ..
        } => route_candidates
            .get(candidate_index)
            .map(journal_route_candidate_search_terms_v1),
        _ => None,
    }
    .map(|terms| terms.join(" "))
}

fn render_journal_event_candidate_at_v1(
    event: &CampaignJournalEventV1,
    candidate_index: usize,
) -> Option<String> {
    let candidate = journal_event_candidates_v1(event).get(candidate_index)?;
    match &event.payload {
        CampaignJournalEventPayloadV1::RouteCandidatePool {
            map_decision_packet: Some(packet),
            ..
        } => packet
            .candidates
            .get(candidate_index)
            .map(|route| render_route_journal_candidate_v1(candidate, route))
            .or_else(|| Some(render_journal_candidate_v1(candidate))),
        CampaignJournalEventPayloadV1::RouteCandidatePool {
            route_candidates, ..
        } if !route_candidates.is_empty() => route_candidates
            .get(candidate_index)
            .map(|route| render_typed_route_journal_candidate_v1(candidate, route))
            .or_else(|| Some(render_journal_candidate_v1(candidate))),
        _ => Some(render_journal_candidate_v1(candidate)),
    }
}

fn render_lineage_query_branch_summary_v1(branch: &BranchCampaignBranchV1) -> String {
    if let Some(summary) = branch.summary.as_ref() {
        format!(
            "A{}F{} HP {}/{} | {}",
            summary.act, summary.floor, summary.hp, summary.max_hp, branch.frontier_title
        )
    } else {
        branch.frontier_title.clone()
    }
}

fn render_lineage_query_branch_choices_v1(branch: &BranchCampaignBranchV1) -> String {
    if branch.choice_labels.is_empty() {
        return String::new();
    }
    truncate_lineage_text_v1(&branch.choice_labels.join(" -> "), 180)
}

fn truncate_lineage_text_v1(text: &str, max_chars: usize) -> String {
    if text.chars().count() <= max_chars {
        return text.to_string();
    }
    let mut truncated = text
        .chars()
        .take(max_chars.saturating_sub(4))
        .collect::<String>();
    truncated.push_str(" ...");
    truncated
}

struct LineageAuditBranchRef<'a> {
    label: String,
    branch: &'a BranchCampaignBranchV1,
}

fn matching_lineage_audit_branches_v1<'a>(
    report: &'a BranchCampaignReportV1,
    filters: &InspectFiltersInput,
    query: Option<&str>,
) -> Vec<LineageAuditBranchRef<'a>> {
    let normalized_query = query
        .map(normalize_query_v1)
        .filter(|query| !query.is_empty());
    let mut matches = Vec::new();
    append_lineage_branch_pool_matches_v1(
        &mut matches,
        "active",
        &report.active,
        filters,
        normalized_query.as_deref(),
    );
    append_lineage_branch_pool_matches_v1(
        &mut matches,
        "frozen",
        &report.frozen,
        filters,
        normalized_query.as_deref(),
    );
    append_lineage_branch_pool_matches_v1(
        &mut matches,
        "abandoned",
        &report.abandoned,
        filters,
        normalized_query.as_deref(),
    );
    append_lineage_branch_pool_matches_v1(
        &mut matches,
        "stuck",
        &report.stuck,
        filters,
        normalized_query.as_deref(),
    );
    append_lineage_branch_pool_matches_v1(
        &mut matches,
        "victory",
        &report.victories,
        filters,
        normalized_query.as_deref(),
    );
    append_lineage_branch_pool_matches_v1(
        &mut matches,
        "dead",
        &report.dead,
        filters,
        normalized_query.as_deref(),
    );
    matches
}

fn append_lineage_branch_pool_matches_v1<'a>(
    target: &mut Vec<LineageAuditBranchRef<'a>>,
    pool_label: &str,
    branches: &'a [BranchCampaignBranchV1],
    filters: &InspectFiltersInput,
    normalized_query: Option<&str>,
) {
    for (index, branch) in branches.iter().enumerate() {
        if !lineage_audit_branch_matches_filters_v1(branch, filters) {
            continue;
        }
        if normalized_query
            .is_some_and(|query| !lineage_audit_branch_search_text_v1(branch).contains(query))
        {
            continue;
        }
        target.push(LineageAuditBranchRef {
            label: format!("{pool_label}[{index}]"),
            branch,
        });
    }
}

fn lineage_audit_branch_matches_filters_v1(
    branch: &BranchCampaignBranchV1,
    filters: &InspectFiltersInput,
) -> bool {
    if filters.act.is_some_and(|act| {
        branch
            .summary
            .as_ref()
            .is_none_or(|summary| summary.act != act)
    }) {
        return false;
    }
    if filters.floor.is_some_and(|floor| {
        branch
            .summary
            .as_ref()
            .is_none_or(|summary| summary.floor != floor)
    }) {
        return false;
    }
    if filters.hp.is_some_and(|hp| {
        branch
            .summary
            .as_ref()
            .is_none_or(|summary| summary.hp != hp)
    }) {
        return false;
    }
    if let Some(boundary) = filters.boundary.as_ref() {
        if normalize_query_v1(&branch.frontier_title) != normalize_query_v1(boundary) {
            return false;
        }
    }
    true
}

fn lineage_audit_branch_search_text_v1(branch: &BranchCampaignBranchV1) -> String {
    let mut parts = vec![
        branch.branch_id.clone(),
        branch.frontier_title.clone(),
        format!("{:?}", branch.status),
    ];
    parts.extend(branch.commands.iter().cloned());
    parts.extend(branch.choice_labels.iter().cloned());
    normalize_query_v1(&parts.join(" "))
}

fn lineage_candidate_pool_events_v1<'a>(
    report: &'a BranchCampaignReportV1,
    branch: &BranchCampaignBranchV1,
) -> Vec<&'a CampaignJournalEventV1> {
    report
        .journal
        .events
        .iter()
        .filter(|event| lineage_event_parent_command_count_v1(event, &branch.commands).is_some())
        .filter(|event| !journal_event_candidates_v1(event).is_empty())
        .collect()
}

fn lineage_missing_decision_commands_v1(
    report: &BranchCampaignReportV1,
    branch: &BranchCampaignBranchV1,
) -> Vec<String> {
    branch
        .commands
        .iter()
        .enumerate()
        .filter_map(|(index, command)| {
            if decision_path_command_is_coordinate_v1(command) {
                return None;
            }
            let matched = report.journal.events.iter().any(|event| {
                lineage_event_parent_command_count_v1(event, &branch.commands) == Some(index)
                    && journal_event_matches_command_v1(event, command.as_str())
            });
            (!matched).then(|| command.clone())
        })
        .collect()
}

fn lineage_event_parent_command_count_v1(
    event: &CampaignJournalEventV1,
    commands: &[String],
) -> Option<usize> {
    let event_path = DecisionPathEnvelopeV1::from_commands(&event.branch_commands);
    let branch_path = DecisionPathEnvelopeV1::from_commands(commands);
    event_path.journal_parent_depth_against(&branch_path)
}

fn journal_event_matches_command_v1(event: &CampaignJournalEventV1, command: &str) -> bool {
    if journal_event_candidates_v1(event)
        .iter()
        .any(|candidate| candidate.command == command)
    {
        return true;
    }
    matches!(
        &event.payload,
        CampaignJournalEventPayloadV1::RouteDecision {
            command: route_command,
            ..
        } if route_command == command
    )
}

fn lineage_event_chosen_command_v1<'a>(
    event: &CampaignJournalEventV1,
    commands: &'a [String],
) -> Option<&'a str> {
    let parent_count = lineage_event_parent_command_count_v1(event, commands)?;
    commands.get(parent_count).map(|command| command.as_str())
}

fn render_lineage_chosen_candidate_v1(
    event: &CampaignJournalEventV1,
    chosen_command: Option<&str>,
) -> String {
    let Some(chosen_command) = chosen_command else {
        return "pending_current_boundary".to_string();
    };
    journal_event_candidates_v1(event)
        .iter()
        .find(|candidate| candidate.command == chosen_command)
        .map(|candidate| format!("{} {{{}}}", candidate.label, candidate.command))
        .unwrap_or_else(|| format!("unmatched_command {{{chosen_command}}}"))
}

fn render_lineage_journal_candidates_v1(
    event: &CampaignJournalEventV1,
    chosen_command: Option<&str>,
    limit: usize,
) -> String {
    let candidates = journal_event_candidates_v1(event);
    if candidates.is_empty() {
        return "-".to_string();
    }
    let shown = limit.max(1).min(candidates.len());
    let mut parts = candidates
        .iter()
        .enumerate()
        .take(shown)
        .map(|(index, candidate)| {
            let marker = if chosen_command == Some(candidate.command.as_str()) {
                "* "
            } else {
                "  "
            };
            let rendered = render_journal_event_candidate_at_v1(event, index)
                .unwrap_or_else(|| render_journal_candidate_v1(candidate));
            format!("{marker}{rendered}")
        })
        .collect::<Vec<_>>();
    if candidates.len() > shown {
        parts.push(format!("... {} more", candidates.len() - shown));
    }
    parts.join(" | ")
}

fn journal_event_extra_summary_v1(event: &CampaignJournalEventV1) -> String {
    match &event.payload {
        CampaignJournalEventPayloadV1::ShopCandidatePool {
            branch_frontier_count,
            rollout_head_plan_id,
            ..
        } => format!(
            " branch_frontier={} rollout_head={}",
            branch_frontier_count,
            rollout_head_plan_id.as_deref().unwrap_or("-")
        ),
        CampaignJournalEventPayloadV1::CampfireCandidatePool {
            branch_option_count,
            selected_plan_id,
            ..
        } => format!(
            " branch_options={} selected_plan={}",
            branch_option_count,
            selected_plan_id.as_deref().unwrap_or("-")
        ),
        CampaignJournalEventPayloadV1::EventCandidatePool {
            game_event_id,
            branch_option_count,
            ..
        } => format!(
            " event={} branch_options={}",
            game_event_id, branch_option_count
        ),
        CampaignJournalEventPayloadV1::BossRelicCandidatePool {
            branch_option_count,
            ..
        } => format!(" branch_options={}", branch_option_count),
        CampaignJournalEventPayloadV1::RouteCandidatePool {
            candidate_count,
            selected_index,
            ..
        } => format!(
            " candidates={} selected_index={}",
            candidate_count,
            selected_index
                .map(|index| index.to_string())
                .unwrap_or_else(|| "-".to_string())
        ),
        CampaignJournalEventPayloadV1::RouteDecision {
            target,
            safety,
            elite_prep_bp,
            selected_index,
            selected_candidate_id,
            ..
        } => format!(
            " target={} safety={} selected_index={} selected_candidate={} elite_prep_bp={}",
            target,
            safety,
            selected_index
                .map(|index| index.to_string())
                .unwrap_or_else(|| "-".to_string()),
            selected_candidate_id.as_deref().unwrap_or("-"),
            elite_prep_bp
        ),
        _ => String::new(),
    }
}

fn matching_journal_events_v1<'a>(
    report: &'a BranchCampaignReportV1,
    filters: &InspectFiltersInput,
    query: Option<&str>,
) -> Vec<&'a CampaignJournalEventV1> {
    let normalized_query = query
        .map(normalize_query_v1)
        .filter(|query| !query.is_empty());
    report
        .journal
        .events
        .iter()
        .filter(|event| {
            filters.act.is_none_or(|act| event.act == act)
                && filters.floor.is_none_or(|floor| event.floor == floor)
                && filters.boundary.as_ref().is_none_or(|boundary| {
                    normalize_query_v1(journal_event_boundary_title_v1(event))
                        .contains(&normalize_query_v1(boundary))
                })
                && normalized_query
                    .as_ref()
                    .is_none_or(|query| journal_event_matches_query_v1(event, query))
        })
        .collect()
}

fn journal_event_matches_query_v1(event: &CampaignJournalEventV1, normalized_query: &str) -> bool {
    match structured_journal_query_match_v1(event, normalized_query) {
        Some(matches) => matches,
        None => journal_event_search_text_v1(event).contains(normalized_query),
    }
}

fn structured_journal_query_match_v1(
    event: &CampaignJournalEventV1,
    normalized_query: &str,
) -> Option<bool> {
    let event_type = normalize_query_v1(journal_event_type_v1(event));
    let matches_type = |expected: &str| event_type == expected;
    match normalized_query {
        "reward" | "cardreward" | "cardrewards" => Some(matches!(
            &event.payload,
            CampaignJournalEventPayloadV1::RewardCandidateSet { .. }
        )),
        "shop" | "shops" => Some(matches!(
            &event.payload,
            CampaignJournalEventPayloadV1::ShopBranchCandidateSet { .. }
                | CampaignJournalEventPayloadV1::ShopCandidatePool { .. }
        )),
        "campfire" | "campfires" => Some(matches!(
            &event.payload,
            CampaignJournalEventPayloadV1::CampfireCandidatePool { .. }
        )),
        "event" | "events" => Some(matches!(
            &event.payload,
            CampaignJournalEventPayloadV1::EventCandidatePool { .. }
        )),
        "bossrelic" | "bossrelics" => Some(matches!(
            &event.payload,
            CampaignJournalEventPayloadV1::BossRelicCandidatePool { .. }
        )),
        "route" | "routes" | "map" => Some(matches!(
            &event.payload,
            CampaignJournalEventPayloadV1::RouteCandidatePool { .. }
                | CampaignJournalEventPayloadV1::RouteDecision { .. }
        )),
        "rewardcandidateset" => Some(matches_type("rewardcandidateset")),
        "shopbranchcandidateset" => Some(matches_type("shopbranchcandidateset")),
        "shopcandidatepool" => Some(matches_type("shopcandidatepool")),
        "campfirecandidatepool" => Some(matches_type("campfirecandidatepool")),
        "eventcandidatepool" => Some(matches_type("eventcandidatepool")),
        "bossreliccandidatepool" => Some(matches_type("bossreliccandidatepool")),
        "routecandidatepool" => Some(matches_type("routecandidatepool")),
        "routedecision" => Some(matches_type("routedecision")),
        _ => None,
    }
}

fn journal_event_type_v1(event: &CampaignJournalEventV1) -> &'static str {
    match &event.payload {
        CampaignJournalEventPayloadV1::RewardCandidateSet { .. } => "reward_candidate_set",
        CampaignJournalEventPayloadV1::ShopBranchCandidateSet { .. } => "shop_branch_candidate_set",
        CampaignJournalEventPayloadV1::ShopCandidatePool { .. } => "shop_candidate_pool",
        CampaignJournalEventPayloadV1::CampfireCandidatePool { .. } => "campfire_candidate_pool",
        CampaignJournalEventPayloadV1::EventCandidatePool { .. } => "event_candidate_pool",
        CampaignJournalEventPayloadV1::BossRelicCandidatePool { .. } => "boss_relic_candidate_pool",
        CampaignJournalEventPayloadV1::RouteCandidatePool { .. } => "route_candidate_pool",
        CampaignJournalEventPayloadV1::RouteDecision { .. } => "route_decision",
    }
}

fn journal_event_boundary_title_v1(event: &CampaignJournalEventV1) -> &str {
    match &event.payload {
        CampaignJournalEventPayloadV1::RewardCandidateSet { boundary_title, .. }
        | CampaignJournalEventPayloadV1::ShopBranchCandidateSet { boundary_title, .. }
        | CampaignJournalEventPayloadV1::ShopCandidatePool { boundary_title, .. }
        | CampaignJournalEventPayloadV1::CampfireCandidatePool { boundary_title, .. }
        | CampaignJournalEventPayloadV1::EventCandidatePool { boundary_title, .. }
        | CampaignJournalEventPayloadV1::BossRelicCandidatePool { boundary_title, .. }
        | CampaignJournalEventPayloadV1::RouteCandidatePool { boundary_title, .. } => {
            boundary_title
        }
        CampaignJournalEventPayloadV1::RouteDecision { .. } => "Map",
    }
}

fn journal_event_frontier_key_v1(event: &CampaignJournalEventV1) -> &str {
    match &event.payload {
        CampaignJournalEventPayloadV1::RewardCandidateSet { frontier_key, .. }
        | CampaignJournalEventPayloadV1::ShopBranchCandidateSet { frontier_key, .. }
        | CampaignJournalEventPayloadV1::ShopCandidatePool { frontier_key, .. }
        | CampaignJournalEventPayloadV1::CampfireCandidatePool { frontier_key, .. }
        | CampaignJournalEventPayloadV1::EventCandidatePool { frontier_key, .. }
        | CampaignJournalEventPayloadV1::BossRelicCandidatePool { frontier_key, .. }
        | CampaignJournalEventPayloadV1::RouteCandidatePool { frontier_key, .. } => frontier_key,
        CampaignJournalEventPayloadV1::RouteDecision { target, .. } => target,
    }
}

fn journal_event_depth_v1(event: &CampaignJournalEventV1) -> usize {
    match &event.payload {
        CampaignJournalEventPayloadV1::RewardCandidateSet { depth, .. }
        | CampaignJournalEventPayloadV1::ShopBranchCandidateSet { depth, .. }
        | CampaignJournalEventPayloadV1::ShopCandidatePool { depth, .. }
        | CampaignJournalEventPayloadV1::CampfireCandidatePool { depth, .. }
        | CampaignJournalEventPayloadV1::EventCandidatePool { depth, .. }
        | CampaignJournalEventPayloadV1::BossRelicCandidatePool { depth, .. }
        | CampaignJournalEventPayloadV1::RouteCandidatePool { depth, .. } => *depth,
        CampaignJournalEventPayloadV1::RouteDecision { .. } => 0,
    }
}

fn journal_event_candidates_v1(event: &CampaignJournalEventV1) -> &[CampaignJournalCandidateV1] {
    match &event.payload {
        CampaignJournalEventPayloadV1::RewardCandidateSet { candidates, .. }
        | CampaignJournalEventPayloadV1::ShopBranchCandidateSet { candidates, .. }
        | CampaignJournalEventPayloadV1::ShopCandidatePool { candidates, .. }
        | CampaignJournalEventPayloadV1::CampfireCandidatePool { candidates, .. }
        | CampaignJournalEventPayloadV1::EventCandidatePool { candidates, .. }
        | CampaignJournalEventPayloadV1::BossRelicCandidatePool { candidates, .. }
        | CampaignJournalEventPayloadV1::RouteCandidatePool { candidates, .. } => candidates,
        CampaignJournalEventPayloadV1::RouteDecision { .. } => &[],
    }
}

fn journal_event_search_text_v1(event: &CampaignJournalEventV1) -> String {
    let mut text = Vec::new();
    text.push(event.event_id.clone());
    text.push(event.branch_id.clone());
    text.push(event.branch_frontier_title.clone());
    text.push(journal_event_type_v1(event).to_string());
    text.push(journal_event_boundary_title_v1(event).to_string());
    text.push(journal_event_frontier_key_v1(event).to_string());
    for choice in &event.branch_choices {
        text.push(choice.clone());
    }
    for command in &event.branch_commands {
        text.push(command.clone());
    }
    for candidate in journal_event_candidates_v1(event) {
        text.push(candidate.candidate_id.clone());
        text.push(candidate.command.clone());
        text.push(candidate.label.clone());
        text.push(candidate.semantic_class.clone());
        text.push(render_candidate_admission_status_v1(candidate.admission.status).to_string());
        text.push(format!(
            "admission={}",
            render_candidate_admission_status_v1(candidate.admission.status)
        ));
        let reason_category = render_candidate_admission_reason_category_v1(
            candidate.admission.normalized_reason_category(),
        );
        text.push(reason_category.to_string());
        text.push(format!("reason_category={reason_category}"));
        let reason_code =
            render_candidate_admission_reason_code_v1(candidate.admission.normalized_reason_code());
        text.push(reason_code.to_string());
        text.push(format!("reason_code={reason_code}"));
        text.push(candidate.admission.source.clone());
        if !candidate.admission.source.is_empty() {
            text.push(format!("source={}", candidate.admission.source));
        }
        text.push(candidate.admission.reason.clone());
        if !candidate.admission.reason.is_empty() {
            text.push(format!("reason={}", candidate.admission.reason));
        }
        text.push(candidate.admission.lane.clone());
        if candidate.admission.representative_count > 0 {
            text.push(format!(
                "representatives:{}",
                candidate.admission.representative_count
            ));
        }
        if candidate.admission.suppressed_count > 0 {
            text.push(format!(
                "suppressed:{}",
                candidate.admission.suppressed_count
            ));
        }
    }
    let payload_terms = journal_event_payload_search_terms_v1(event);
    text.extend(payload_terms);
    normalize_query_v1(&text.join(" "))
}

fn journal_event_payload_search_terms_v1(event: &CampaignJournalEventV1) -> Vec<String> {
    match &event.payload {
        CampaignJournalEventPayloadV1::RouteDecision {
            route_branch_id,
            selected_index,
            selected_candidate_id,
            selected_candidate_rank,
            selected_target_node,
            target,
            move_kind,
            safety_flag,
            safety,
            candidate_pool_provenance,
            command,
            elite_prep_bp,
            first_elite,
            selected_route_candidate,
            ..
        } => {
            let mut parts = vec![
                route_branch_id.clone(),
                target.clone(),
                move_kind.clone(),
                safety.clone(),
                command.clone(),
                format!(
                    "selected_index:{}",
                    selected_index
                        .map(|index| index.to_string())
                        .unwrap_or_else(|| "-".to_string())
                ),
                selected_candidate_id.clone().unwrap_or_default(),
                format!(
                    "selected_candidate_rank:{}",
                    selected_candidate_rank
                        .map(|rank| rank.to_string())
                        .unwrap_or_else(|| "-".to_string())
                ),
                format!("elite_prep_bp:{elite_prep_bp}"),
                format!("first_elite_paths:{}", first_elite.paths_with_first_elite),
                format!("first_elite_forced:{}", first_elite.forced),
                format!("first_elite_optional:{}", first_elite.optional),
            ];
            if let Some(target) = selected_target_node {
                parts.push(format!("typed_x:{}", target.x));
                parts.push(format!("typed_y:{}", target.y));
                parts.push(format!("typed_room:{:?}", target.room_type));
                parts.push(format!("typed_move:{:?}", target.move_kind));
            }
            if let Some(safety) = safety_flag {
                parts.push(format!("typed_safety:{:?}", safety));
            }
            if let Some(provenance) = candidate_pool_provenance {
                parts.push(format!(
                    "complete_legal_pool:{}",
                    provenance.complete_legal_pool
                ));
                parts.push(format!(
                    "legal_candidates:{}",
                    provenance.legal_candidate_count
                ));
                parts.push(format!(
                    "emitted_candidates:{}",
                    provenance.emitted_candidate_count
                ));
            }
            if let Some(candidate) = selected_route_candidate {
                parts.extend(journal_route_candidate_search_terms_v1(candidate));
            }
            parts
        }
        CampaignJournalEventPayloadV1::RouteCandidatePool {
            selected_index,
            candidate_count,
            candidate_pool_provenance,
            map_decision_packet,
            route_candidates,
            ..
        } => {
            let mut parts = vec![
                format!("candidate_count:{candidate_count}"),
                format!(
                    "selected_index:{}",
                    selected_index
                        .map(|index| index.to_string())
                        .unwrap_or_else(|| "-".to_string())
                ),
            ];
            if let Some(provenance) = candidate_pool_provenance {
                parts.push(format!(
                    "legal_candidates:{}",
                    provenance.legal_candidate_count
                ));
                parts.push(format!(
                    "complete_legal_pool:{}",
                    provenance.complete_legal_pool
                ));
                parts.push(format!("ordering:{:?}", provenance.ordering));
            }
            if let Some(packet) = map_decision_packet {
                for candidate in &packet.candidates {
                    parts.extend(route_candidate_search_terms_v1(candidate));
                }
            } else {
                for candidate in route_candidates {
                    parts.extend(journal_route_candidate_search_terms_v1(candidate));
                }
            }
            parts
        }
        _ => Vec::new(),
    }
}

fn route_candidate_search_terms_v1(candidate: &RouteMoveCandidateV1) -> Vec<String> {
    let room_type = candidate
        .target
        .room_type
        .map(|room| format!("{:?}", room))
        .unwrap_or_else(|| "Unknown".to_string());
    vec![
        candidate.candidate_id.clone(),
        candidate.command.clone(),
        format!("x{}", candidate.target.x),
        format!("y{}", candidate.target.y),
        room_type,
        format!("{:?}", candidate.target.move_kind),
        format!("{:?}", candidate.evaluation.safety),
        format!("{:?}", candidate.evaluation.value_source),
        format!("{:?}", candidate.evaluation.calibration_status),
        format!("{:?}", candidate.projection.metadata.coverage),
    ]
}

fn journal_route_candidate_search_terms_v1(
    candidate: &CampaignJournalRouteCandidateV1,
) -> Vec<String> {
    let mut parts = vec![
        candidate.candidate_id.clone(),
        candidate.command.clone(),
        candidate.target.clone(),
        candidate.room_type.clone(),
        candidate.move_kind.clone(),
        candidate.safety.clone(),
    ];
    if let Some(target) = &candidate.target_node {
        parts.push(format!("x{}", target.x));
        parts.push(format!("y{}", target.y));
        parts.push(format!("{:?}", target.move_kind));
        if let Some(room) = target.room_type {
            parts.push(format!("{:?}", room));
        }
    }
    if let Some(action) = &candidate.action {
        match action {
            sts_simulator::ai::route_planner_v1::RouteMapActionV1::Go { x } => {
                parts.push("action:go".to_string());
                parts.push(format!("action_x:{x}"));
            }
            sts_simulator::ai::route_planner_v1::RouteMapActionV1::Fly { x, y } => {
                parts.push("action:fly".to_string());
                parts.push(format!("action_x:{x}"));
                parts.push(format!("action_y:{y}"));
            }
        }
    }
    if let Some(path) = &candidate.path_summary {
        parts.push(format!("path_count:{}", path.path_count));
        parts.push(format!("min_elites:{}", path.min_elites));
        parts.push(format!("max_elites:{}", path.max_elites));
        parts.push(format!("min_fires:{}", path.min_fires));
        parts.push(format!("max_fires:{}", path.max_fires));
    }
    if let Some(coverage) = candidate.projection_coverage {
        parts.push(format!("{:?}", coverage));
    }
    if let Some(source) = candidate.projection_source {
        parts.push(format!("{:?}", source));
    }
    if let Some(source) = candidate.evaluation_source {
        parts.push(format!("{:?}", source));
    }
    if let Some(status) = candidate.evaluation_calibration_status {
        parts.push(format!("{:?}", status));
    }
    if let Some(path_budget) = candidate.path_budget {
        parts.push(format!("path_budget:{path_budget}"));
    }
    if let Some(observed_path_count) = candidate.observed_path_count {
        parts.push(format!("observed_paths:{observed_path_count}"));
    }
    parts.push(format!("route_rank:{}", candidate.rank));
    parts.push(format!("route_selected:{}", candidate.selected));
    parts.push(format!("route_score:{:.2}", candidate.score));
    parts.push(format!("elite_prep_bp:{}", candidate.elite_prep_bp));
    parts.push(format!(
        "first_elite_paths:{}",
        candidate.first_elite.paths_with_first_elite
    ));
    parts.push(format!(
        "first_elite_forced:{}",
        candidate.first_elite.forced
    ));
    parts.push(format!(
        "first_elite_optional:{}",
        candidate.first_elite.optional
    ));
    parts.extend(candidate.reasons.iter().cloned());
    parts.extend(candidate.cautions.iter().cloned());
    parts
}

fn render_journal_event_candidates_v1(event: &CampaignJournalEventV1, limit: usize) -> String {
    match &event.payload {
        CampaignJournalEventPayloadV1::RouteCandidatePool {
            candidates,
            map_decision_packet: Some(packet),
            ..
        } => render_route_journal_candidates_v1(packet, candidates, limit),
        CampaignJournalEventPayloadV1::RouteCandidatePool {
            candidates,
            route_candidates,
            ..
        } if !route_candidates.is_empty() => {
            render_typed_route_journal_candidates_v1(route_candidates, candidates, limit)
        }
        _ => render_journal_candidates_v1(journal_event_candidates_v1(event), limit),
    }
}

fn render_journal_candidates_v1(candidates: &[CampaignJournalCandidateV1], limit: usize) -> String {
    if candidates.is_empty() {
        return "-".to_string();
    }
    let shown = limit.max(1).min(candidates.len());
    let mut parts = candidates
        .iter()
        .take(shown)
        .map(render_journal_candidate_v1)
        .collect::<Vec<_>>();
    if candidates.len() > shown {
        parts.push(format!("... {} more", candidates.len() - shown));
    }
    parts.join(" | ")
}

fn render_route_journal_candidates_v1(
    packet: &MapDecisionPacketV1,
    candidates: &[CampaignJournalCandidateV1],
    limit: usize,
) -> String {
    if candidates.is_empty() {
        return "-".to_string();
    }
    let shown = limit.max(1).min(candidates.len());
    let mut parts = candidates
        .iter()
        .take(shown)
        .enumerate()
        .map(|(index, candidate)| {
            packet.candidates.get(index).map_or_else(
                || render_journal_candidate_v1(candidate),
                |route| render_route_journal_candidate_v1(candidate, route),
            )
        })
        .collect::<Vec<_>>();
    if candidates.len() > shown {
        parts.push(format!("... {} more", candidates.len() - shown));
    }
    parts.join(" | ")
}

fn render_typed_route_journal_candidates_v1(
    route_candidates: &[CampaignJournalRouteCandidateV1],
    candidates: &[CampaignJournalCandidateV1],
    limit: usize,
) -> String {
    if candidates.is_empty() {
        return "-".to_string();
    }
    let shown = limit.max(1).min(candidates.len());
    let mut parts = candidates
        .iter()
        .take(shown)
        .enumerate()
        .map(|(index, candidate)| {
            route_candidates.get(index).map_or_else(
                || render_journal_candidate_v1(candidate),
                |route| render_typed_route_journal_candidate_v1(candidate, route),
            )
        })
        .collect::<Vec<_>>();
    if candidates.len() > shown {
        parts.push(format!("... {} more", candidates.len() - shown));
    }
    parts.join(" | ")
}

fn render_route_journal_candidate_v1(
    candidate: &CampaignJournalCandidateV1,
    route: &RouteMoveCandidateV1,
) -> String {
    let room_type = route
        .target
        .room_type
        .map(|room| format!("{:?}", room))
        .unwrap_or_else(|| "Unknown".to_string());
    let path = &route.projection.path_summary;
    let projection = &route.projection.metadata;
    format!(
        "x{}y{} {} {{{}}} [route rank={} move={:?} safety={:?} score={:.2} vf={} coverage={:?} paths={}/{} elites={}-{} fires={}-{} shops={}-{}; {}; {}]",
        route.target.x,
        route.target.y,
        room_type,
        route.command,
        route.rank,
        route.target.move_kind,
        route.evaluation.safety,
        route.evaluation.total_score,
        render_route_value_factors_v1(route),
        projection.coverage,
        projection.observed_path_count,
        projection.path_budget,
        path.min_elites,
        path.max_elites,
        path.min_fires,
        path.max_fires,
        path.min_shops,
        path.max_shops,
        render_candidate_admission_v1(candidate),
        render_candidate_disposition_v1(candidate.disposition)
    )
}

fn render_typed_route_journal_candidate_v1(
    candidate: &CampaignJournalCandidateV1,
    route: &CampaignJournalRouteCandidateV1,
) -> String {
    let path = route.path_summary.as_ref();
    let target = route.target_node.as_ref();
    let x = target
        .map(|target| target.x.to_string())
        .unwrap_or_else(|| "?".to_string());
    let y = target
        .map(|target| target.y.to_string())
        .unwrap_or_else(|| "?".to_string());
    let coverage = route
        .projection_coverage
        .map(|coverage| format!("{:?}", coverage))
        .unwrap_or_else(|| "Unknown".to_string());
    format!(
        "x{}y{} {} {{{}}} [route rank={} move={} safety={} score={:.2} vf={} coverage={} paths={}/{} elites={}-{} fires={}-{} shops={}-{}; {}; {}]",
        x,
        y,
        route.room_type,
        route.command,
        route.rank,
        route.move_kind,
        route.safety,
        route.score,
        render_typed_route_value_factors_v1(route),
        coverage,
        route.observed_path_count.unwrap_or_else(|| path.map(|path| path.path_count).unwrap_or(0)),
        route.path_budget.unwrap_or(0),
        path.map(|path| path.min_elites).unwrap_or(0),
        path.map(|path| path.max_elites).unwrap_or(0),
        path.map(|path| path.min_fires).unwrap_or(0),
        path.map(|path| path.max_fires).unwrap_or(0),
        path.map(|path| path.min_shops).unwrap_or(0),
        path.map(|path| path.max_shops).unwrap_or(0),
        render_candidate_admission_v1(candidate),
        render_candidate_disposition_v1(candidate.disposition)
    )
}

fn render_route_value_factors_v1(route: &RouteMoveCandidateV1) -> String {
    let factors = &route.evaluation.value_factors;
    format!(
        "card:{:.1}/relic:{:.1}/shop:{:.1}/heal:{:.1}/hp90:{:.1}/risk:{:.2}/flex:{:.1}/elite:{:.1}",
        factors.card_reward_access,
        factors.relic_access,
        factors.shop_access,
        factors.heal_access,
        factors.hp_loss_p90,
        factors.death_risk,
        factors.flexibility,
        factors.first_elite_prep_signal
    )
}

fn render_typed_route_value_factors_v1(route: &CampaignJournalRouteCandidateV1) -> String {
    let Some(factors) = route.value_factors.as_ref() else {
        return "-".to_string();
    };
    format!(
        "card:{:.1}/relic:{:.1}/shop:{:.1}/heal:{:.1}/hp90:{:.1}/risk:{:.2}/flex:{:.1}/elite:{:.1}",
        factors.card_reward_access,
        factors.relic_access,
        factors.shop_access,
        factors.heal_access,
        factors.hp_loss_p90,
        factors.death_risk,
        factors.flexibility,
        factors.first_elite_prep_signal
    )
}

fn render_journal_candidate_v1(candidate: &CampaignJournalCandidateV1) -> String {
    format!(
        "{} {{{}}} [{}; {}; {}]",
        candidate.label,
        candidate.command,
        candidate.semantic_class,
        render_candidate_admission_v1(candidate),
        render_candidate_disposition_v1(candidate.disposition)
    )
}

fn render_candidate_admission_v1(candidate: &CampaignJournalCandidateV1) -> String {
    let mut parts = vec![format!(
        "admission={}",
        render_candidate_admission_status_v1(candidate.admission.status)
    )];
    if !candidate.admission.source.is_empty() {
        parts.push(format!("source={}", candidate.admission.source));
    }
    if candidate.admission.normalized_reason_category()
        != CampaignJournalCandidateAdmissionReasonCategoryV1::Unknown
    {
        parts.push(format!(
            "reason_category={}",
            render_candidate_admission_reason_category_v1(
                candidate.admission.normalized_reason_category()
            )
        ));
    }
    if candidate.admission.normalized_reason_code()
        != CampaignJournalCandidateAdmissionReasonCodeV1::Unknown
    {
        parts.push(format!(
            "reason_code={}",
            render_candidate_admission_reason_code_v1(candidate.admission.normalized_reason_code())
        ));
    }
    if !candidate.admission.reason.is_empty() {
        parts.push(format!("reason={}", candidate.admission.reason));
    }
    if !candidate.admission.lane.is_empty() {
        parts.push(format!("lane={}", candidate.admission.lane));
    }
    if candidate.admission.representative_count > 0 {
        parts.push(format!(
            "representatives={}",
            candidate.admission.representative_count
        ));
    }
    if candidate.admission.suppressed_count > 0 {
        parts.push(format!(
            "suppressed={}",
            candidate.admission.suppressed_count
        ));
    }
    parts.join(" ")
}

fn render_candidate_admission_status_v1(
    status: CampaignJournalCandidateAdmissionStatusV1,
) -> &'static str {
    match status {
        CampaignJournalCandidateAdmissionStatusV1::Unknown => "unknown",
        CampaignJournalCandidateAdmissionStatusV1::Scheduled => "scheduled",
        CampaignJournalCandidateAdmissionStatusV1::Deferred => "deferred",
        CampaignJournalCandidateAdmissionStatusV1::Rejected => "rejected",
    }
}

fn render_candidate_admission_reason_category_v1(
    category: CampaignJournalCandidateAdmissionReasonCategoryV1,
) -> &'static str {
    category.as_str()
}

fn render_candidate_admission_reason_code_v1(
    code: CampaignJournalCandidateAdmissionReasonCodeV1,
) -> &'static str {
    code.as_str()
}

fn render_candidate_disposition_v1(
    disposition: CampaignJournalCandidateDispositionV1,
) -> &'static str {
    match disposition {
        CampaignJournalCandidateDispositionV1::Kept => "kept",
        CampaignJournalCandidateDispositionV1::Pruned => "pruned",
    }
}

fn normalize_query_v1(text: &str) -> String {
    text.to_ascii_lowercase().replace([' ', '_', '-'], "")
}

#[cfg(test)]
mod tests {
    use super::*;
    use sts_simulator::eval::branch_campaign::{
        BranchCampaignBranchStatusV1, BranchCampaignBranchSummaryV1, BranchCampaignBranchV1,
    };
    use sts_simulator::eval::campaign_journal::{
        CampaignJournalCandidateAdmissionTraceV1, CampaignJournalV1,
    };

    #[test]
    fn structured_event_query_does_not_match_parent_event_text_on_reward() {
        let reward =
            journal_event_with_payload(CampaignJournalEventPayloadV1::RewardCandidateSet {
                decision_id: "reward0".to_string(),
                boundary_title: "Reward Screen".to_string(),
                frontier_key: "frontier".to_string(),
                depth: 0,
                max_reward_options_per_branch: 3,
                original_count: 1,
                selected_count: 1,
                candidates: vec![candidate("Carnage", "rp 0", "frontload")],
            });
        let event = journal_event_with_payload(CampaignJournalEventPayloadV1::EventCandidatePool {
            decision_id: "event0".to_string(),
            boundary_title: "GoldenIdol".to_string(),
            frontier_key: "frontier".to_string(),
            depth: 0,
            game_event_id: "GoldenIdol".to_string(),
            candidate_count: 2,
            branch_option_count: 2,
            candidates: vec![candidate("Leave", "event 1", "effect:event_leave")],
        });

        assert!(!journal_event_matches_query_v1(
            &reward,
            &normalize_query_v1("event")
        ));
        assert!(journal_event_matches_query_v1(
            &event,
            &normalize_query_v1("event")
        ));
    }

    #[test]
    fn unstructured_journal_query_still_searches_candidate_text() {
        let reward =
            journal_event_with_payload(CampaignJournalEventPayloadV1::RewardCandidateSet {
                decision_id: "reward0".to_string(),
                boundary_title: "Reward Screen".to_string(),
                frontier_key: "frontier".to_string(),
                depth: 0,
                max_reward_options_per_branch: 3,
                original_count: 1,
                selected_count: 1,
                candidates: vec![candidate(
                    "Golden Idol",
                    "event 0",
                    "effect:event_gain_relic",
                )],
            });

        assert!(journal_event_matches_query_v1(
            &reward,
            &normalize_query_v1("Golden Idol")
        ));
    }

    #[test]
    fn structured_route_query_matches_route_decision_only() {
        let selected_route_candidate = CampaignJournalRouteCandidateV1 {
            candidate_id: "route_move:normal_edge:x1:y0".to_string(),
            rank: 0,
            selected: true,
            target_node: None,
            target: "x=1 Elite".to_string(),
            room_type: "Elite".to_string(),
            move_kind: "NormalEdge".to_string(),
            action: None,
            safety_flag: None,
            safety: "ok".to_string(),
            score: 1.0,
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
            first_elite:
                sts_simulator::eval::branch_experiment::BranchExperimentFirstEliteEvidenceV1::default(),
            reasons: vec!["route planner selected".to_string()],
            cautions: Vec::new(),
        };
        let route = journal_event_with_payload(CampaignJournalEventPayloadV1::RouteDecision {
            decision_id: "route0".to_string(),
            route_branch_id: "root.go1".to_string(),
            selected_index: Some(0),
            selected_candidate_id: Some("route_move:normal_edge:x1:y0".to_string()),
            selected_candidate_rank: Some(0),
            selected_target_node: None,
            selected_route_candidate: Some(selected_route_candidate),
            target: "x=1 Elite".to_string(),
            move_kind: "Elite".to_string(),
            safety_flag: None,
            safety: "ok".to_string(),
            candidate_pool_provenance: None,
            command: "go 1".to_string(),
            elite_prep_bp: 42,
            first_elite:
                sts_simulator::eval::branch_experiment::BranchExperimentFirstEliteEvidenceV1::default(),
        });
        let reward =
            journal_event_with_payload(CampaignJournalEventPayloadV1::RewardCandidateSet {
                decision_id: "reward0".to_string(),
                boundary_title: "Reward Screen".to_string(),
                frontier_key: "frontier".to_string(),
                depth: 0,
                max_reward_options_per_branch: 3,
                original_count: 1,
                selected_count: 1,
                candidates: vec![candidate("Route Sense", "rp 0", "route_like_text")],
            });

        assert!(journal_event_matches_query_v1(
            &route,
            &normalize_query_v1("route")
        ));
        assert!(!journal_event_matches_query_v1(
            &reward,
            &normalize_query_v1("route")
        ));
        assert!(journal_event_matches_query_v1(
            &route,
            &normalize_query_v1("x=1 Elite")
        ));
        assert!(journal_event_matches_query_v1(
            &route,
            &normalize_query_v1("route_move:normal_edge:x1:y0")
        ));
        assert!(journal_event_matches_query_v1(
            &route,
            &normalize_query_v1("route planner selected")
        ));
    }

    #[test]
    fn structured_route_query_matches_selected_route_candidate_projection() {
        let mut run = sts_simulator::state::RunState::new(521, 0, false, "Ironclad");
        run.event_state = None;
        let trace = sts_simulator::ai::route_planner_v1::plan_route_decision_v1(
            &run,
            &sts_simulator::state::core::EngineState::MapNavigation,
            sts_simulator::ai::route_planner_v1::RoutePlannerConfigV1::default(),
        );
        let packet = MapDecisionPacketV1::from_route_decision_trace_v1(&trace);
        let route_candidate = packet
            .candidates
            .first()
            .expect("route packet should have a candidate");
        let route = journal_event_with_payload(CampaignJournalEventPayloadV1::RouteDecision {
            decision_id: "route0".to_string(),
            route_branch_id: "root.go".to_string(),
            selected_index: Some(route_candidate.rank),
            selected_candidate_id: Some(route_candidate.candidate_id.clone()),
            selected_candidate_rank: Some(route_candidate.rank),
            selected_target_node: Some(route_candidate.target.clone()),
            selected_route_candidate: Some(
                CampaignJournalRouteCandidateV1::from_route_move_candidate_with_selected_v1(
                    route_candidate,
                    true,
                ),
            ),
            target: format!("x={} y={}", route_candidate.target.x, route_candidate.target.y),
            move_kind: format!("{:?}", route_candidate.target.move_kind),
            safety_flag: Some(route_candidate.evaluation.safety),
            safety: format!("{:?}", route_candidate.evaluation.safety),
            candidate_pool_provenance: Some(packet.candidate_pool.clone()),
            command: route_candidate.command.clone(),
            elite_prep_bp: 0,
            first_elite:
                sts_simulator::eval::branch_experiment::BranchExperimentFirstEliteEvidenceV1::default(),
        });

        assert!(journal_event_matches_query_v1(
            &route,
            &normalize_query_v1("action:go")
        ));
        assert!(journal_event_matches_query_v1(
            &route,
            &normalize_query_v1(&format!(
                "path_count:{}",
                route_candidate.projection.path_summary.path_count
            ))
        ));
        assert!(journal_event_matches_query_v1(
            &route,
            &normalize_query_v1(&format!(
                "{:?}",
                route_candidate.projection.metadata.coverage
            ))
        ));
        assert!(journal_event_matches_query_v1(
            &route,
            &normalize_query_v1("heuristic_route_planner_v1")
        ));
        assert!(journal_event_matches_query_v1(
            &route,
            &normalize_query_v1("uncalibrated_behavior_estimate")
        ));
    }

    #[test]
    fn journal_candidate_render_shows_admission_trace_status() {
        let mut candidate = candidate("Armaments", "rp 1", "block");
        candidate.admission = CampaignJournalCandidateAdmissionTraceV1::new(
            CampaignJournalCandidateAdmissionStatusV1::Scheduled,
            "reward_portfolio",
            "selected",
        );

        let rendered = render_journal_candidate_v1(&candidate);

        assert!(rendered.contains("admission=scheduled"));
        assert!(rendered.contains("source=reward_portfolio"));
        assert!(rendered.contains("reason_category=retention_bucket"));
        assert!(rendered.contains("reason_code=selected"));
    }

    #[test]
    fn journal_query_searches_candidate_admission_trace() {
        let mut candidate = candidate("Armaments", "rp 1", "block");
        candidate.admission = CampaignJournalCandidateAdmissionTraceV1::new(
            CampaignJournalCandidateAdmissionStatusV1::Scheduled,
            "reward_portfolio",
            "selected",
        );
        let reward =
            journal_event_with_payload(CampaignJournalEventPayloadV1::RewardCandidateSet {
                decision_id: "reward0".to_string(),
                boundary_title: "Reward Screen".to_string(),
                frontier_key: "frontier".to_string(),
                depth: 0,
                max_reward_options_per_branch: 3,
                original_count: 1,
                selected_count: 1,
                candidates: vec![candidate],
            });

        assert!(journal_event_matches_query_v1(
            &reward,
            &normalize_query_v1("scheduled")
        ));
        assert!(journal_event_matches_query_v1(
            &reward,
            &normalize_query_v1("reward_portfolio")
        ));
        assert!(journal_event_matches_query_v1(
            &reward,
            &normalize_query_v1("retention_bucket")
        ));
        assert!(journal_event_matches_query_v1(
            &reward,
            &normalize_query_v1("reason_code=selected")
        ));
    }

    #[test]
    fn route_candidate_pool_render_prefers_typed_map_packet() {
        let mut run = sts_simulator::state::RunState::new(521, 0, false, "Ironclad");
        run.event_state = None;
        let trace = sts_simulator::ai::route_planner_v1::plan_route_decision_v1(
            &run,
            &sts_simulator::state::core::EngineState::MapNavigation,
            sts_simulator::ai::route_planner_v1::RoutePlannerConfigV1::default(),
        );
        let packet = MapDecisionPacketV1::from_route_decision_trace_v1(&trace);
        assert!(!packet.candidates.is_empty());
        let first_route = &packet.candidates[0];
        let event = journal_event_with_payload(CampaignJournalEventPayloadV1::RouteCandidatePool {
            decision_id: "route-pool0".to_string(),
            boundary_title: "Map".to_string(),
            frontier_key: "map-frontier".to_string(),
            depth: 0,
            candidate_count: packet.candidates.len(),
            selected_index: packet.selected_index,
            candidate_pool_provenance: Some(packet.candidate_pool.clone()),
            map_decision_packet: Some(packet.clone()),
            route_candidates: packet
                .candidates
                .iter()
                .map(CampaignJournalRouteCandidateV1::from_route_move_candidate_v1)
                .collect(),
            candidates: packet
                .candidates
                .iter()
                .map(|route| candidate("legacy route label", &route.command, "legacy route"))
                .collect(),
        });

        let rendered = render_journal_event_candidates_v1(&event, 3);

        assert!(rendered.contains(&format!(
            "x{}y{}",
            first_route.target.x, first_route.target.y
        )));
        assert!(rendered.contains("coverage="));
        assert!(rendered.contains("paths="));
        assert!(!rendered.contains("legacy route label"));
        assert!(journal_event_matches_query_v1(
            &event,
            &normalize_query_v1(&format!("x{}", first_route.target.x))
        ));
        assert!(journal_event_matches_query_v1(
            &event,
            &normalize_query_v1(&format!("{:?}", first_route.projection.metadata.coverage))
        ));
    }

    #[test]
    fn route_candidate_pool_render_uses_typed_route_candidates_without_map_packet() {
        let mut run = sts_simulator::state::RunState::new(521, 0, false, "Ironclad");
        run.event_state = None;
        let trace = sts_simulator::ai::route_planner_v1::plan_route_decision_v1(
            &run,
            &sts_simulator::state::core::EngineState::MapNavigation,
            sts_simulator::ai::route_planner_v1::RoutePlannerConfigV1::default(),
        );
        let packet = MapDecisionPacketV1::from_route_decision_trace_v1(&trace);
        assert!(!packet.candidates.is_empty());
        let first_route = &packet.candidates[0];
        let event = journal_event_with_payload(CampaignJournalEventPayloadV1::RouteCandidatePool {
            decision_id: "route-pool0".to_string(),
            boundary_title: "Map".to_string(),
            frontier_key: "map-frontier".to_string(),
            depth: 0,
            candidate_count: packet.candidates.len(),
            selected_index: packet.selected_index,
            candidate_pool_provenance: Some(packet.candidate_pool.clone()),
            map_decision_packet: None,
            route_candidates: packet
                .candidates
                .iter()
                .map(CampaignJournalRouteCandidateV1::from_route_move_candidate_v1)
                .collect(),
            candidates: packet
                .candidates
                .iter()
                .map(|route| candidate("legacy route label", &route.command, "legacy route"))
                .collect(),
        });

        let rendered = render_journal_event_candidates_v1(&event, 3);

        assert!(rendered.contains(&format!(
            "x{}y{}",
            first_route.target.x, first_route.target.y
        )));
        assert!(rendered.contains("coverage="));
        assert!(rendered.contains("paths="));
        assert!(!rendered.contains("legacy route label"));
        assert!(journal_event_matches_query_v1(
            &event,
            &normalize_query_v1(&format!("x{}", first_route.target.x))
        ));
        assert!(journal_event_matches_query_v1(
            &event,
            &normalize_query_v1(&format!("{:?}", first_route.projection.metadata.coverage))
        ));
    }

    #[test]
    fn lineage_decision_audit_filters_events_to_target_branch_prefix() {
        let target = branch(
            "target",
            vec!["rp 1", "event 0"],
            vec!["Pommel Strike", "Fight"],
        );
        let report = report_with_branch_and_journal(
            target,
            vec![
                journal_event_at(
                    "reward-root",
                    vec!["__decision_parent:0:reward:test"],
                    CampaignJournalEventPayloadV1::RewardCandidateSet {
                        decision_id: "reward-root".to_string(),
                        boundary_title: "Reward Screen".to_string(),
                        frontier_key: "reward-frontier".to_string(),
                        depth: 0,
                        max_reward_options_per_branch: 3,
                        original_count: 2,
                        selected_count: 2,
                        candidates: vec![
                            candidate("Strike Dummy", "rp 0", "frontload"),
                            candidate("Pommel Strike", "rp 1", "draw"),
                        ],
                    },
                ),
                journal_event_at(
                    "event-after-pommel",
                    vec!["rp 1", "__decision_parent:1:event:test"],
                    CampaignJournalEventPayloadV1::EventCandidatePool {
                        decision_id: "event-after-pommel".to_string(),
                        boundary_title: "Mushrooms".to_string(),
                        frontier_key: "event-frontier".to_string(),
                        depth: 0,
                        game_event_id: "Mushrooms".to_string(),
                        candidate_count: 2,
                        branch_option_count: 2,
                        candidates: vec![
                            candidate("Fight", "event 0", "effect:fight"),
                            candidate("Leave", "event 1", "effect:leave"),
                        ],
                    },
                ),
                journal_event_at(
                    "unrelated-after-rp0",
                    vec!["rp 0"],
                    CampaignJournalEventPayloadV1::RewardCandidateSet {
                        decision_id: "unrelated".to_string(),
                        boundary_title: "Reward Screen".to_string(),
                        frontier_key: "unrelated-frontier".to_string(),
                        depth: 0,
                        max_reward_options_per_branch: 3,
                        original_count: 1,
                        selected_count: 1,
                        candidates: vec![candidate("Feel No Pain", "rp 2", "power")],
                    },
                ),
            ],
        );

        let rendered = render_campaign_lineage_decision_audit_v1(
            &report,
            &InspectFiltersInput {
                act: None,
                floor: None,
                boundary: None,
                hp: None,
                index: Some(0),
            },
            None,
            8,
        )
        .expect("lineage audit should render");

        assert!(rendered.contains("Branch lineage decision audit"));
        assert!(rendered.contains("branch: active[0] target"));
        assert!(rendered.contains("chosen=Pommel Strike {rp 1}"));
        assert!(rendered.contains("chosen=Fight {event 0}"));
        assert!(rendered.contains("* Pommel Strike"));
        assert!(rendered.contains("* Fight"));
        assert!(rendered.contains("missing_steps=0"));
        assert!(!rendered.contains("Feel No Pain"));
    }

    #[test]
    fn lineage_query_summarizes_matching_candidates_across_branch_lineages() {
        let report = report_with_branches_and_journal(
            vec![
                branch("picked-iron-wave", vec!["rp 2"], vec!["Iron Wave"]),
                branch(
                    "skipped-iron-wave",
                    vec!["branch-skip-card-reward 0"],
                    vec!["Skip card reward"],
                ),
            ],
            Vec::new(),
            vec![journal_event_at(
                "reward-root",
                vec!["__decision_parent:0:reward:test"],
                CampaignJournalEventPayloadV1::RewardCandidateSet {
                    decision_id: "reward-root".to_string(),
                    boundary_title: "Reward Screen".to_string(),
                    frontier_key: "reward-frontier".to_string(),
                    depth: 0,
                    max_reward_options_per_branch: 3,
                    original_count: 3,
                    selected_count: 3,
                    candidates: vec![
                        candidate("Burning Pact", "rp 1", "setup:exhaust_engine"),
                        candidate("Iron Wave+", "rp 2", "stabilizer:Block"),
                        candidate("Skip card reward", "branch-skip-card-reward 0", "decline"),
                    ],
                },
            )],
        );

        let rendered = render_campaign_lineage_decision_audit_v1(
            &report,
            &InspectFiltersInput {
                act: None,
                floor: None,
                boundary: None,
                hp: None,
                index: None,
            },
            Some("Iron Wave"),
            8,
        )
        .expect("lineage query should render");

        assert!(rendered.contains("Branch lineage candidate query"));
        assert!(rendered.contains("query=Iron Wave"));
        assert!(rendered.contains("branch=active[0] A1F3 HP 70/80 | Reward Screen"));
        assert!(rendered.contains("chosen=Iron Wave+ {rp 2}"));
        assert!(rendered.contains("match=Iron Wave+ {rp 2}"));
        assert!(rendered.contains("branch=active[1] A1F3 HP 70/80 | Reward Screen"));
        assert!(rendered.contains("chosen=Skip card reward {branch-skip-card-reward 0}"));
        assert!(!rendered.contains("match=Burning Pact"));
    }

    #[test]
    fn lineage_query_matches_and_renders_typed_route_candidates() {
        let mut run = sts_simulator::state::RunState::new(521, 0, false, "Ironclad");
        run.event_state = None;
        let trace = sts_simulator::ai::route_planner_v1::plan_route_decision_v1(
            &run,
            &sts_simulator::state::core::EngineState::MapNavigation,
            sts_simulator::ai::route_planner_v1::RoutePlannerConfigV1::default(),
        );
        let packet = MapDecisionPacketV1::from_route_decision_trace_v1(&trace);
        assert!(!packet.candidates.is_empty());
        let first_route = &packet.candidates[0];
        let route_command = first_route.command.clone();
        let route_x_query = format!("x{}", first_route.target.x);
        let report = report_with_branches_and_journal(
            vec![branch(
                "route-branch",
                vec![route_command.as_str()],
                vec!["Take route"],
            )],
            Vec::new(),
            vec![journal_event_at(
                "route-root",
                Vec::new(),
                CampaignJournalEventPayloadV1::RouteCandidatePool {
                    decision_id: "route-root".to_string(),
                    boundary_title: "Map".to_string(),
                    frontier_key: "map-frontier".to_string(),
                    depth: 0,
                    candidate_count: packet.candidates.len(),
                    selected_index: packet.selected_index,
                    candidate_pool_provenance: Some(packet.candidate_pool.clone()),
                    map_decision_packet: Some(packet.clone()),
                    route_candidates: packet
                        .candidates
                        .iter()
                        .map(CampaignJournalRouteCandidateV1::from_route_move_candidate_v1)
                        .collect(),
                    candidates: packet
                        .candidates
                        .iter()
                        .map(|route| {
                            candidate("legacy route label", &route.command, "legacy route")
                        })
                        .collect(),
                },
            )],
        );

        let rendered = render_campaign_lineage_decision_audit_v1(
            &report,
            &InspectFiltersInput {
                act: None,
                floor: None,
                boundary: None,
                hp: None,
                index: None,
            },
            Some(&route_x_query),
            8,
        )
        .expect("typed route candidate query should render");

        assert!(rendered.contains("Branch lineage candidate query"));
        assert!(rendered.contains(&format!("query={route_x_query}")));
        assert!(rendered.contains("route_candidate_pool"));
        assert!(rendered.contains("match=x"));
        assert!(rendered.contains("coverage="));
        assert!(rendered.contains("paths="));
        assert!(rendered.contains("elites="));
        assert!(!rendered.contains("match=legacy route label"));
    }

    fn journal_event_with_payload(
        payload: CampaignJournalEventPayloadV1,
    ) -> CampaignJournalEventV1 {
        CampaignJournalEventV1 {
            event_id: "journal-event".to_string(),
            round: 1,
            branch_id: "root".to_string(),
            branch_index: 0,
            branch_frontier_title: "Reward Screen".to_string(),
            act: 1,
            floor: 1,
            branch_choices: vec!["GoldenIdol: [Leave]".to_string()],
            branch_commands: vec!["event 1".to_string()],
            combat_budget_retry_used: false,
            payload,
        }
    }

    fn candidate(label: &str, command: &str, semantic_class: &str) -> CampaignJournalCandidateV1 {
        CampaignJournalCandidateV1 {
            candidate_id: command.to_string(),
            command: command.to_string(),
            label: label.to_string(),
            semantic_class: semantic_class.to_string(),
            admission: Default::default(),
            disposition: CampaignJournalCandidateDispositionV1::Kept,
        }
    }

    fn journal_event_at(
        event_id: &str,
        branch_commands: Vec<&str>,
        payload: CampaignJournalEventPayloadV1,
    ) -> CampaignJournalEventV1 {
        CampaignJournalEventV1 {
            event_id: event_id.to_string(),
            round: 1,
            branch_id: "root".to_string(),
            branch_index: 0,
            branch_frontier_title: "Reward Screen".to_string(),
            act: 1,
            floor: 1,
            branch_choices: Vec::new(),
            branch_commands: branch_commands
                .into_iter()
                .map(ToString::to_string)
                .collect(),
            combat_budget_retry_used: false,
            payload,
        }
    }

    fn branch(id: &str, commands: Vec<&str>, choices: Vec<&str>) -> BranchCampaignBranchV1 {
        BranchCampaignBranchV1 {
            branch_id: id.to_string(),
            commands: commands.into_iter().map(ToString::to_string).collect(),
            choice_labels: choices.into_iter().map(ToString::to_string).collect(),
            summary: Some(BranchCampaignBranchSummaryV1 {
                act: 1,
                floor: 3,
                hp: 70,
                max_hp: 80,
                gold: 99,
                deck_count: 12,
                deck_key: String::new(),
                formation_stage: "test".to_string(),
                formation_strengths: Vec::new(),
                formation_needs: Vec::new(),
                trajectory_key: String::new(),
                boss: String::new(),
                boss_pressure: Vec::new(),
                run_debt: Vec::new(),
                event_boundary: None,
                reward_boundary: None,
            }),
            strategic_summary: Default::default(),
            frontier_title: "Reward Screen".to_string(),
            status: BranchCampaignBranchStatusV1::Active,
            stop_reason: "test".to_string(),
            continuation_origin: None,
            lineage_decision_signal_rank_adjustment: 0,
            rank_key: 0,
            final_boss_combat_record: None,
            combat_lab_probes: Vec::new(),
        }
    }

    fn report_with_branch_and_journal(
        branch: BranchCampaignBranchV1,
        events: Vec<CampaignJournalEventV1>,
    ) -> BranchCampaignReportV1 {
        report_with_branches_and_journal(vec![branch], Vec::new(), events)
    }

    fn report_with_branches_and_journal(
        active: Vec<BranchCampaignBranchV1>,
        frozen: Vec<BranchCampaignBranchV1>,
        events: Vec<CampaignJournalEventV1>,
    ) -> BranchCampaignReportV1 {
        BranchCampaignReportV1 {
            schema_name: "BranchCampaignV1".to_string(),
            schema_version: 1,
            seed: 1,
            run_domain: Default::default(),
            run_prelude: Default::default(),
            rounds_completed: 1,
            stop_reason: "test".to_string(),
            active,
            frozen,
            victories: Vec::new(),
            dead: Vec::new(),
            abandoned: Vec::new(),
            stuck: Vec::new(),
            discarded_count: 0,
            discarded_examples: Vec::new(),
            discarded_branches: Vec::new(),
            strategy_requests: Vec::new(),
            route_evidence: Default::default(),
            combat_retry_ledger: Default::default(),
            strategic_signals: Default::default(),
            state_store: Default::default(),
            journal: CampaignJournalV1 {
                schema_name: "CampaignJournal".to_string(),
                schema_version: 4,
                events,
            },
            rounds: Vec::new(),
        }
    }
}
