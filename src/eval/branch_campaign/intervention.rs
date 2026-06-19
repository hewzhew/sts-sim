use std::collections::BTreeMap;

use crate::eval::branch_experiment::BranchExperimentStrategyRequestV1;

use super::branch_display::{render_campaign_branch_state, render_choice_path};
use super::model::{
    BranchCampaignBranchStatusV1, BranchCampaignBranchV1, BranchCampaignReportV1,
    BranchCampaignStrategyRequestV1,
};
use super::report_render::unique_limited_strings;
use super::{branch_progress_key, normalized_campaign_boundary_title};
pub(super) fn merge_campaign_strategy_requests_v1(
    requests: Vec<BranchExperimentStrategyRequestV1>,
) -> Vec<BranchCampaignStrategyRequestV1> {
    let mut merged = BTreeMap::<(String, String, u8, i32), BranchCampaignStrategyRequestV1>::new();
    for request in requests {
        let key = (
            request.kind.clone(),
            request.boundary_title.clone(),
            request.act,
            request.floor,
        );
        merged
            .entry(key)
            .and_modify(|existing| {
                existing.branch_count = existing.branch_count.saturating_add(request.branch_count);
                if (request.act, request.floor) > (existing.act, existing.floor) {
                    existing.act = request.act;
                    existing.floor = request.floor;
                }
                if existing.next_card_reward_offer.is_none() {
                    existing.next_card_reward_offer = request.next_card_reward_offer.clone();
                }
                for example in &request.examples {
                    if existing.examples.len() < 4 && !existing.examples.contains(example) {
                        existing.examples.push(example.clone());
                    }
                }
                for reason in &request.stop_reasons {
                    if existing.stop_reasons.len() < 4 && !existing.stop_reasons.contains(reason) {
                        existing.stop_reasons.push(reason.clone());
                    }
                }
                for detail in &request.boundary_details {
                    if existing.boundary_details.len() < 8
                        && !existing.boundary_details.contains(detail)
                    {
                        existing.boundary_details.push(detail.clone());
                    }
                }
            })
            .or_insert_with(|| BranchCampaignStrategyRequestV1 {
                kind: request.kind.clone(),
                boundary_title: request.boundary_title,
                branch_count: request.branch_count,
                act: request.act,
                floor: request.floor,
                stop_reasons: request.stop_reasons.into_iter().take(4).collect(),
                examples: request.examples.into_iter().take(4).collect(),
                next_card_reward_offer: request.next_card_reward_offer,
                boundary_details: request.boundary_details.into_iter().take(8).collect(),
                suggested_action: campaign_suggested_action_v1(
                    &request.kind,
                    &request.suggested_action,
                ),
            });
    }
    merged.into_values().collect()
}

fn campaign_suggested_action_v1(kind: &str, suggested_action: &str) -> String {
    match kind {
        "combat_hp_loss_policy" | "combat_manual_or_budget" => {
            "provide combat tactic or upstream route/reward strategy; raise budget only if search was clearly under-spent".to_string()
        }
        "card_reward_policy_gap" => {
            "provide reward family policy for this public offer and run context".to_string()
        }
        "event_strategy" => "provide event strategy for this event context".to_string(),
        "campfire_strategy" => {
            "provide campfire strategy for this deck and route context".to_string()
        }
        "boss_relic_strategy" => {
            "provide boss relic strategy for the current deck package".to_string()
        }
        "shop_strategy" => "provide shop strategy for this shop state".to_string(),
        "reward_claim_policy" => "provide reward claim policy for this context".to_string(),
        "route_policy_gap" => "provide route strategy for this map context".to_string(),
        _ => suggested_action.to_string(),
    }
}

pub(super) fn merge_campaign_strategy_request_queue_v1(
    existing: Vec<BranchCampaignStrategyRequestV1>,
    incoming: Vec<BranchCampaignStrategyRequestV1>,
) -> Vec<BranchCampaignStrategyRequestV1> {
    let mut merged = BTreeMap::<(String, String, u8, i32), BranchCampaignStrategyRequestV1>::new();
    for mut request in existing.into_iter().chain(incoming) {
        request.suggested_action =
            campaign_suggested_action_v1(&request.kind, &request.suggested_action);
        let key = (
            request.kind.clone(),
            request.boundary_title.clone(),
            request.act,
            request.floor,
        );
        merged
            .entry(key)
            .and_modify(|current| {
                current.branch_count = current.branch_count.saturating_add(request.branch_count);
                if (request.act, request.floor) > (current.act, current.floor) {
                    current.act = request.act;
                    current.floor = request.floor;
                }
                if current.next_card_reward_offer.is_none() {
                    current.next_card_reward_offer = request.next_card_reward_offer.clone();
                }
                for reason in &request.stop_reasons {
                    if current.stop_reasons.len() < 4 && !current.stop_reasons.contains(reason) {
                        current.stop_reasons.push(reason.clone());
                    }
                }
                for example in &request.examples {
                    if current.examples.len() < 4 && !current.examples.contains(example) {
                        current.examples.push(example.clone());
                    }
                }
                for detail in &request.boundary_details {
                    if current.boundary_details.len() < 8
                        && !current.boundary_details.contains(detail)
                    {
                        current.boundary_details.push(detail.clone());
                    }
                }
            })
            .or_insert(request);
    }
    merged.into_values().collect()
}

pub(super) fn campaign_strategy_requests_are_fatal_v1(
    active: &[BranchCampaignBranchV1],
    frozen: &[BranchCampaignBranchV1],
    strategy_requests: &[BranchCampaignStrategyRequestV1],
) -> bool {
    !strategy_requests.is_empty() && active.is_empty() && frozen.is_empty()
}

pub(super) fn abandoned_branches_intervention_request_v1(
    abandoned: &[BranchCampaignBranchV1],
) -> Option<BranchCampaignStrategyRequestV1> {
    if abandoned.is_empty() {
        return None;
    }
    let examples = abandoned
        .iter()
        .map(|branch| {
            let choices = render_choice_path(&branch.choice_labels);
            if choices == "-" {
                render_campaign_branch_state(branch)
            } else {
                choices
            }
        })
        .take(4)
        .collect::<Vec<_>>();
    let stop_reasons = unique_limited_strings(
        abandoned
            .iter()
            .map(|branch| branch.stop_reason.trim())
            .filter(|reason| !reason.is_empty())
            .map(ToOwned::to_owned),
        4,
    );
    Some(BranchCampaignStrategyRequestV1 {
        kind: "combat_manual_or_budget".to_string(),
        boundary_title: "Combat".to_string(),
        branch_count: abandoned.len(),
        act: abandoned
            .iter()
            .filter_map(|branch| branch.summary.as_ref().map(|summary| summary.act))
            .max()
            .unwrap_or_default(),
        floor: abandoned
            .iter()
            .filter_map(|branch| branch.summary.as_ref().map(|summary| summary.floor))
            .max()
            .unwrap_or_default(),
        stop_reasons: if stop_reasons.is_empty() {
            vec!["all candidate route branches were abandoned".to_string()]
        } else {
            stop_reasons
        },
        examples,
        next_card_reward_offer: None,
        boundary_details: Vec::new(),
        suggested_action:
            "provide combat tactic or upstream route/reward strategy; raise budget only if search was clearly under-spent"
                .to_string(),
    })
}

pub(super) fn leading_abandoned_combat_intervention_request_v1(
    frozen: &[BranchCampaignBranchV1],
    abandoned: &[BranchCampaignBranchV1],
) -> Option<BranchCampaignStrategyRequestV1> {
    let best_frozen_progress = frozen.iter().map(branch_progress_key).max();
    let best_abandoned_progress = abandoned
        .iter()
        .filter(|branch| is_combat_abandoned_branch_v1(branch))
        .map(branch_progress_key)
        .max()?;
    if best_frozen_progress.is_some_and(|progress| progress >= best_abandoned_progress) {
        return None;
    }

    let leading = abandoned
        .iter()
        .filter(|branch| {
            is_combat_abandoned_branch_v1(branch)
                && branch_progress_key(branch) == best_abandoned_progress
        })
        .cloned()
        .collect::<Vec<_>>();
    abandoned_branches_intervention_request_v1(&leading)
}

fn is_combat_abandoned_branch_v1(branch: &BranchCampaignBranchV1) -> bool {
    branch.status == BranchCampaignBranchStatusV1::Abandoned
        && normalized_campaign_boundary_title(&branch.frontier_title) == "combat"
}

pub(super) fn render_campaign_intervention_details_v2(
    report: &BranchCampaignReportV1,
    request: &BranchCampaignStrategyRequestV1,
) -> Vec<String> {
    vec![
        format!(
            "    kind: {}",
            campaign_intervention_kind_v2(report, request)
        ),
        format!(
            "    tried: {}",
            campaign_intervention_tried_v2(report, request)
        ),
        format!(
            "    possible inputs: {}",
            campaign_intervention_options_v2(request)
        ),
    ]
}

fn campaign_intervention_kind_v2(
    report: &BranchCampaignReportV1,
    request: &BranchCampaignStrategyRequestV1,
) -> &'static str {
    match request.kind.as_str() {
        "combat_hp_loss_policy" | "combat_manual_or_budget" => {
            if report
                .rounds
                .last()
                .map(|round| round.combat_budget_retries > 0)
                .unwrap_or(false)
            {
                "combat_unresolved_after_retry"
            } else {
                "combat_unresolved"
            }
        }
        "card_reward_policy_gap" => "card_reward_strategy_gap",
        "event_strategy" => "event_strategy_needed",
        "campfire_strategy" => "campfire_strategy_needed",
        "boss_relic_strategy" => "boss_relic_strategy_needed",
        "shop_strategy" => "shop_strategy_needed",
        "reward_claim_policy" => "reward_claim_strategy_needed",
        "route_policy_gap" => "route_strategy_gap",
        "engineering_issue" => "engineering_issue",
        _ => "strategy_needed",
    }
}

fn campaign_intervention_tried_v2(
    report: &BranchCampaignReportV1,
    request: &BranchCampaignStrategyRequestV1,
) -> String {
    match request.kind.as_str() {
        "combat_hp_loss_policy" | "combat_manual_or_budget" => {
            let retries = report
                .rounds
                .last()
                .map(|round| round.combat_budget_retries)
                .unwrap_or(0);
            if retries > 0 {
                format!("campaign search budget; combat budget retry x{retries}")
            } else {
                "campaign search budget".to_string()
            }
        }
        "card_reward_policy_gap" => {
            "branch reward candidates; current autopick gate declined".to_string()
        }
        "event_strategy" => "event boundary detected; no narrow event policy accepted".to_string(),
        "campfire_strategy" => {
            "campfire options detected; no campfire priority accepted".to_string()
        }
        "shop_strategy" => "shop options detected; purchase portfolio did not resolve".to_string(),
        _ => "current campaign policy".to_string(),
    }
}

fn campaign_intervention_options_v2(request: &BranchCampaignStrategyRequestV1) -> &'static str {
    match request.kind.as_str() {
        "combat_hp_loss_policy" | "combat_manual_or_budget" => {
            "switch macro branch | provide combat tactic | add upstream route/reward rule | raise retry budget only if under-spent"
        }
        "card_reward_policy_gap" => {
            "reward package rule | keep branching this reward family | force human judgment"
        }
        "event_strategy" => {
            "event rule | choose one event branch manually | blacklist this event branch"
        }
        "campfire_strategy" => {
            "smith/rest/recall rule | branch fewer smith targets | ask human at this campfire"
        }
        "shop_strategy" => {
            "buy/remove/leave rule | cap purchase portfolio | ask human at this shop"
        }
        "boss_relic_strategy" => {
            "boss relic package rule | preserve multiple relic branches | ask human"
        }
        "reward_claim_policy" => {
            "mark reward as safe claim | keep reward pending | ask human"
        }
        "route_policy_gap" => {
            "route rule for this context | provide one map choice | freeze this route family"
        }
        "engineering_issue" => {
            "fix simulator or command bug | rerun same seed | quarantine affected trace"
        }
        _ => "add a narrow strategy rule | keep branching | ask human",
    }
}

pub(super) fn prune_resolved_campaign_strategy_requests_v1(
    requests: Vec<BranchCampaignStrategyRequestV1>,
    _active: &[BranchCampaignBranchV1],
    _frozen: &[BranchCampaignBranchV1],
    stuck: &[BranchCampaignBranchV1],
    abandoned: &[BranchCampaignBranchV1],
) -> Vec<BranchCampaignStrategyRequestV1> {
    requests
        .into_iter()
        .filter(|request| {
            stuck
                .iter()
                .chain(abandoned.iter())
                .any(|branch| campaign_strategy_request_matches_branch_v1(request, branch))
        })
        .collect()
}

fn campaign_strategy_request_matches_branch_v1(
    request: &BranchCampaignStrategyRequestV1,
    branch: &BranchCampaignBranchV1,
) -> bool {
    normalized_campaign_boundary_title(&request.boundary_title)
        == normalized_campaign_boundary_title(&branch.frontier_title)
        && (request.act == 0
            || branch
                .summary
                .as_ref()
                .is_some_and(|summary| summary.act == request.act))
        && (request.floor == 0
            || branch
                .summary
                .as_ref()
                .is_some_and(|summary| summary.floor == request.floor))
        && (request.stop_reasons.is_empty()
            || request
                .stop_reasons
                .iter()
                .any(|reason| branch.stop_reason.contains(reason)))
}

pub(super) fn campaign_strategy_next_step_v1(kind: &str) -> Option<&'static str> {
    match kind {
        "combat_hp_loss_policy" | "combat_manual_or_budget" => Some(
            "campaign should switch remaining macro branches first; if all are exhausted, provide a combat tactic or upstream route/reward rule",
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
            Some("provide a route rule for this context, or provide a one-step map choice before continuing")
        }
        "engineering_issue" => {
            Some("fix the command/state bug before trusting this campaign branch")
        }
        _ => None,
    }
}
