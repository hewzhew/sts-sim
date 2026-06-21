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
        "rewardcandidateset" => Some(matches_type("rewardcandidateset")),
        "shopbranchcandidateset" => Some(matches_type("shopbranchcandidateset")),
        "shopcandidatepool" => Some(matches_type("shopcandidatepool")),
        "campfirecandidatepool" => Some(matches_type("campfirecandidatepool")),
        "eventcandidatepool" => Some(matches_type("eventcandidatepool")),
        "bossreliccandidatepool" => Some(matches_type("bossreliccandidatepool")),
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
    }
}

fn journal_event_boundary_title_v1(event: &CampaignJournalEventV1) -> &str {
    match &event.payload {
        CampaignJournalEventPayloadV1::RewardCandidateSet { boundary_title, .. }
        | CampaignJournalEventPayloadV1::ShopBranchCandidateSet { boundary_title, .. }
        | CampaignJournalEventPayloadV1::ShopCandidatePool { boundary_title, .. }
        | CampaignJournalEventPayloadV1::CampfireCandidatePool { boundary_title, .. }
        | CampaignJournalEventPayloadV1::EventCandidatePool { boundary_title, .. }
        | CampaignJournalEventPayloadV1::BossRelicCandidatePool { boundary_title, .. } => {
            boundary_title
        }
    }
}

fn journal_event_frontier_key_v1(event: &CampaignJournalEventV1) -> &str {
    match &event.payload {
        CampaignJournalEventPayloadV1::RewardCandidateSet { frontier_key, .. }
        | CampaignJournalEventPayloadV1::ShopBranchCandidateSet { frontier_key, .. }
        | CampaignJournalEventPayloadV1::ShopCandidatePool { frontier_key, .. }
        | CampaignJournalEventPayloadV1::CampfireCandidatePool { frontier_key, .. }
        | CampaignJournalEventPayloadV1::EventCandidatePool { frontier_key, .. }
        | CampaignJournalEventPayloadV1::BossRelicCandidatePool { frontier_key, .. } => {
            frontier_key
        }
    }
}

fn journal_event_depth_v1(event: &CampaignJournalEventV1) -> usize {
    match &event.payload {
        CampaignJournalEventPayloadV1::RewardCandidateSet { depth, .. }
        | CampaignJournalEventPayloadV1::ShopBranchCandidateSet { depth, .. }
        | CampaignJournalEventPayloadV1::ShopCandidatePool { depth, .. }
        | CampaignJournalEventPayloadV1::CampfireCandidatePool { depth, .. }
        | CampaignJournalEventPayloadV1::EventCandidatePool { depth, .. }
        | CampaignJournalEventPayloadV1::BossRelicCandidatePool { depth, .. } => *depth,
    }
}

fn journal_event_candidates_v1(event: &CampaignJournalEventV1) -> &[CampaignJournalCandidateV1] {
    match &event.payload {
        CampaignJournalEventPayloadV1::RewardCandidateSet { candidates, .. }
        | CampaignJournalEventPayloadV1::ShopBranchCandidateSet { candidates, .. }
        | CampaignJournalEventPayloadV1::ShopCandidatePool { candidates, .. }
        | CampaignJournalEventPayloadV1::CampfireCandidatePool { candidates, .. }
        | CampaignJournalEventPayloadV1::EventCandidatePool { candidates, .. }
        | CampaignJournalEventPayloadV1::BossRelicCandidatePool { candidates, .. } => candidates,
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

#[cfg(test)]
mod tests {
    use super::*;

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
            disposition: CampaignJournalCandidateDispositionV1::Kept,
        }
    }
}
