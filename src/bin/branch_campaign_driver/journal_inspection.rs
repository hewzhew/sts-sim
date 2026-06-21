use sts_simulator::eval::branch_campaign::BranchCampaignReportV1;
use sts_simulator::eval::campaign_journal::{
    CampaignJournalCandidateDispositionV1, CampaignJournalCandidateV1,
    CampaignJournalEventPayloadV1, CampaignJournalEventV1,
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
            render_journal_candidates_v1(journal_event_candidates_v1(event), limit)
        ));
    }
    Ok(format!("{}\n", lines.join("\n")))
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
                    .is_none_or(|query| journal_event_search_text_v1(event).contains(query))
        })
        .collect()
}

fn journal_event_type_v1(event: &CampaignJournalEventV1) -> &'static str {
    match &event.payload {
        CampaignJournalEventPayloadV1::RewardCandidateSet { .. } => "reward_candidate_set",
        CampaignJournalEventPayloadV1::ShopBranchCandidateSet { .. } => "shop_branch_candidate_set",
        CampaignJournalEventPayloadV1::ShopCandidatePool { .. } => "shop_candidate_pool",
    }
}

fn journal_event_boundary_title_v1(event: &CampaignJournalEventV1) -> &str {
    match &event.payload {
        CampaignJournalEventPayloadV1::RewardCandidateSet { boundary_title, .. }
        | CampaignJournalEventPayloadV1::ShopBranchCandidateSet { boundary_title, .. }
        | CampaignJournalEventPayloadV1::ShopCandidatePool { boundary_title, .. } => boundary_title,
    }
}

fn journal_event_frontier_key_v1(event: &CampaignJournalEventV1) -> &str {
    match &event.payload {
        CampaignJournalEventPayloadV1::RewardCandidateSet { frontier_key, .. }
        | CampaignJournalEventPayloadV1::ShopBranchCandidateSet { frontier_key, .. }
        | CampaignJournalEventPayloadV1::ShopCandidatePool { frontier_key, .. } => frontier_key,
    }
}

fn journal_event_depth_v1(event: &CampaignJournalEventV1) -> usize {
    match &event.payload {
        CampaignJournalEventPayloadV1::RewardCandidateSet { depth, .. }
        | CampaignJournalEventPayloadV1::ShopBranchCandidateSet { depth, .. }
        | CampaignJournalEventPayloadV1::ShopCandidatePool { depth, .. } => *depth,
    }
}

fn journal_event_candidates_v1(event: &CampaignJournalEventV1) -> &[CampaignJournalCandidateV1] {
    match &event.payload {
        CampaignJournalEventPayloadV1::RewardCandidateSet { candidates, .. }
        | CampaignJournalEventPayloadV1::ShopBranchCandidateSet { candidates, .. }
        | CampaignJournalEventPayloadV1::ShopCandidatePool { candidates, .. } => candidates,
    }
}

fn journal_event_search_text_v1(event: &CampaignJournalEventV1) -> String {
    let mut text = Vec::new();
    text.push(event.event_id.as_str());
    text.push(event.branch_id.as_str());
    text.push(event.branch_frontier_title.as_str());
    text.push(journal_event_type_v1(event));
    text.push(journal_event_boundary_title_v1(event));
    text.push(journal_event_frontier_key_v1(event));
    for choice in &event.branch_choices {
        text.push(choice.as_str());
    }
    for command in &event.branch_commands {
        text.push(command.as_str());
    }
    for candidate in journal_event_candidates_v1(event) {
        text.push(candidate.candidate_id.as_str());
        text.push(candidate.command.as_str());
        text.push(candidate.label.as_str());
        text.push(candidate.semantic_class.as_str());
    }
    normalize_query_v1(&text.join(" "))
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

fn render_journal_candidate_v1(candidate: &CampaignJournalCandidateV1) -> String {
    format!(
        "{} {{{}}} [{}; {}]",
        candidate.label,
        candidate.command,
        candidate.semantic_class,
        render_candidate_disposition_v1(candidate.disposition)
    )
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
