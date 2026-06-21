use sts_simulator::ai::route_planner_v1::{MapDecisionPacketV1, RouteMoveCandidateV1};
use sts_simulator::eval::branch_campaign::BranchCampaignReportV1;
use sts_simulator::eval::campaign_journal::{
    CampaignJournalCandidateAdmissionReasonCategoryV1,
    CampaignJournalCandidateAdmissionReasonCodeV1, CampaignJournalCandidateAdmissionStatusV1,
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
            render_journal_event_candidates_v1(event, limit)
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
            ..
        } => format!(
            " target={} safety={} elite_prep_bp={}",
            target, safety, elite_prep_bp
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
            target,
            move_kind,
            safety,
            command,
            elite_prep_bp,
            first_elite,
            ..
        } => vec![
            route_branch_id.clone(),
            target.clone(),
            move_kind.clone(),
            safety.clone(),
            command.clone(),
            format!("elite_prep_bp:{elite_prep_bp}"),
            format!("first_elite_paths:{}", first_elite.paths_with_first_elite),
            format!("first_elite_forced:{}", first_elite.forced),
            format!("first_elite_optional:{}", first_elite.optional),
        ],
        CampaignJournalEventPayloadV1::RouteCandidatePool {
            selected_index,
            candidate_count,
            candidate_pool_provenance,
            map_decision_packet,
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
        format!("{:?}", candidate.projection.metadata.coverage),
    ]
}

fn render_journal_event_candidates_v1(event: &CampaignJournalEventV1, limit: usize) -> String {
    match &event.payload {
        CampaignJournalEventPayloadV1::RouteCandidatePool {
            candidates,
            map_decision_packet: Some(packet),
            ..
        } => render_route_journal_candidates_v1(packet, candidates, limit),
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
        "x{}y{} {} {{{}}} [route rank={} move={:?} safety={:?} score={:.2} coverage={:?} paths={}/{} elites={}-{} fires={}-{} shops={}-{}; {}; {}]",
        route.target.x,
        route.target.y,
        room_type,
        route.command,
        route.rank,
        route.target.move_kind,
        route.evaluation.safety,
        route.evaluation.total_score,
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
    use sts_simulator::eval::campaign_journal::CampaignJournalCandidateAdmissionTraceV1;

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
        let route = journal_event_with_payload(CampaignJournalEventPayloadV1::RouteDecision {
            decision_id: "route0".to_string(),
            route_branch_id: "root.go1".to_string(),
            target: "x=1 Elite".to_string(),
            move_kind: "Elite".to_string(),
            safety: "ok".to_string(),
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
}
