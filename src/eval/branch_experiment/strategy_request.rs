use std::collections::{BTreeMap, BTreeSet};

use super::types::{
    BranchExperimentBranchReportV1, BranchExperimentBranchStatusV1, BranchExperimentChoiceV1,
    BranchExperimentStrategyRequestV1,
};

#[derive(Clone, Debug)]
struct StrategyRequestDraft {
    kind: String,
    boundary_title: String,
    representative_branch_id: String,
    act: u8,
    floor: i32,
    stop_reasons: BTreeSet<String>,
    examples: BTreeSet<String>,
    next_card_reward_offer: Option<Vec<String>>,
    boundary_details: Vec<String>,
    suggested_action: String,
    branch_count: usize,
}

pub(super) fn branch_strategy_requests(
    branches: &[BranchExperimentBranchReportV1],
) -> Vec<BranchExperimentStrategyRequestV1> {
    let mut groups = BTreeMap::<String, StrategyRequestDraft>::new();
    for branch in branches {
        if !request_worthy_status(branch.status) {
            continue;
        }
        let kind = request_kind(branch);
        let suggested_action = suggested_action(&kind, branch);
        let next_offer = branch.frontier.next_card_reward_offer.clone();
        let group_key = strategy_group_key(&kind, branch, next_offer.as_ref());
        groups
            .entry(group_key)
            .and_modify(|draft| {
                draft.branch_count += 1;
                draft.stop_reasons.insert(branch.stop_reason.clone());
                draft.examples.insert(choice_path(branch));
                push_unique_details(&mut draft.boundary_details, &branch.boundary_details);
                if (branch.summary.act, branch.summary.floor) > (draft.act, draft.floor) {
                    draft.act = branch.summary.act;
                    draft.floor = branch.summary.floor;
                }
            })
            .or_insert_with(|| StrategyRequestDraft {
                kind,
                boundary_title: branch.summary.boundary_title.clone(),
                representative_branch_id: branch.branch_id.clone(),
                act: branch.summary.act,
                floor: branch.summary.floor,
                stop_reasons: BTreeSet::from([branch.stop_reason.clone()]),
                examples: BTreeSet::from([choice_path(branch)]),
                next_card_reward_offer: next_offer,
                boundary_details: branch.boundary_details.clone(),
                suggested_action,
                branch_count: 1,
            });
    }

    let mut requests = groups
        .into_values()
        .map(|draft| BranchExperimentStrategyRequestV1 {
            kind: draft.kind,
            boundary_title: draft.boundary_title,
            branch_count: draft.branch_count,
            representative_branch_id: draft.representative_branch_id,
            act: draft.act,
            floor: draft.floor,
            stop_reasons: draft.stop_reasons.into_iter().take(3).collect(),
            examples: draft.examples.into_iter().take(4).collect(),
            next_card_reward_offer: draft.next_card_reward_offer,
            boundary_details: draft.boundary_details.into_iter().take(8).collect(),
            suggested_action: draft.suggested_action,
        })
        .collect::<Vec<_>>();
    requests.sort_by(|left, right| {
        request_kind_priority(&left.kind)
            .cmp(&request_kind_priority(&right.kind))
            .then_with(|| right.branch_count.cmp(&left.branch_count))
            .then_with(|| (right.act, right.floor).cmp(&(left.act, left.floor)))
            .then_with(|| left.boundary_title.cmp(&right.boundary_title))
    });
    requests
}

fn push_unique_details(target: &mut Vec<String>, source: &[String]) {
    for detail in source {
        if !target.contains(detail) {
            target.push(detail.clone());
        }
    }
}

fn request_worthy_status(status: BranchExperimentBranchStatusV1) -> bool {
    matches!(
        status,
        BranchExperimentBranchStatusV1::NeedsHumanBoundary | BranchExperimentBranchStatusV1::Failed
    )
}

fn request_kind(branch: &BranchExperimentBranchReportV1) -> String {
    let title = branch.summary.boundary_title.as_str();
    let stop = branch.stop_reason.to_ascii_lowercase();
    if matches!(branch.status, BranchExperimentBranchStatusV1::Failed) {
        return "engineering_issue".to_string();
    }
    request_kind_for_boundary(title, &stop)
}

fn request_kind_for_boundary(title: &str, stop: &str) -> String {
    if normalized_title(title) == "combat" {
        if stop.contains("hp-loss") || stop.contains("max_hp_loss") {
            return "combat_hp_loss_policy".to_string();
        }
        return "combat_manual_or_budget".to_string();
    }
    match normalized_title(title).as_str() {
        "cardreward" => "card_reward_policy_gap".to_string(),
        "campfire" => "campfire_strategy".to_string(),
        "bossrelic" => "boss_relic_strategy".to_string(),
        "shop" => "shop_strategy".to_string(),
        "rewardscreen" | "rewardoverlay" => "reward_claim_policy".to_string(),
        "map" | "mappreview" => "route_policy_gap".to_string(),
        _ => "event_strategy".to_string(),
    }
}

fn strategy_group_key(
    kind: &str,
    branch: &BranchExperimentBranchReportV1,
    next_offer: Option<&Vec<String>>,
) -> String {
    let offer_key = next_offer
        .map(|offer| offer.join("|"))
        .unwrap_or_else(|| "-".to_string());
    format!(
        "{}|{}|{}|{}",
        kind, branch.summary.boundary_title, branch.stop_reason, offer_key
    )
}

fn suggested_action(kind: &str, branch: &BranchExperimentBranchReportV1) -> String {
    match kind {
        "card_reward_policy_gap" => {
            "decide whether this reward family should branch, autopick, skip, or bowl".to_string()
        }
        "event_strategy" => format!(
            "provide an event policy for {}; keep only meaningful branches if choices are broad",
            branch.summary.boundary_title
        ),
        "combat_hp_loss_policy" => {
            "set an acceptable hp-loss gate, raise budget, or mark this combat for manual play"
                .to_string()
        }
        "combat_manual_or_budget" => {
            "raise combat search budget, relax hp-loss gate, or provide a manual combat line"
                .to_string()
        }
        "campfire_strategy" => {
            "provide rest/smith/recall priority for this deck and route".to_string()
        }
        "boss_relic_strategy" => {
            "provide boss relic priority for the current deck package".to_string()
        }
        "shop_strategy" => "provide buy/remove/leave priorities for this shop state".to_string(),
        "reward_claim_policy" => "decide which remaining reward claims are automatic".to_string(),
        "route_policy_gap" => {
            "adjust route planner policy or manually choose this map node".to_string()
        }
        "engineering_issue" => {
            "inspect the failed branch and fix the underlying command/state bug".to_string()
        }
        _ => "provide a strategy rule for this boundary".to_string(),
    }
}

fn choice_path(branch: &BranchExperimentBranchReportV1) -> String {
    if branch.choices.is_empty() {
        return "-".to_string();
    }
    branch
        .choices
        .iter()
        .map(choice_label)
        .collect::<Vec<_>>()
        .join(" -> ")
}

fn choice_label(choice: &BranchExperimentChoiceV1) -> String {
    if choice.effect_label.is_empty() {
        choice.label.clone()
    } else {
        choice.effect_label.clone()
    }
}

fn normalized_title(value: &str) -> String {
    value
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .map(|ch| ch.to_ascii_lowercase())
        .collect()
}

fn request_kind_priority(kind: &str) -> u8 {
    match kind {
        "engineering_issue" => 0,
        "event_strategy" => 1,
        "card_reward_policy_gap" => 2,
        "boss_relic_strategy" => 3,
        "campfire_strategy" => 4,
        "shop_strategy" => 5,
        "combat_hp_loss_policy" => 6,
        "combat_manual_or_budget" => 7,
        "reward_claim_policy" => 8,
        "route_policy_gap" => 9,
        _ => 10,
    }
}

#[cfg(test)]
mod tests {
    use super::request_kind_for_boundary;

    #[test]
    fn map_preview_is_a_route_gap_not_an_event_strategy_gap() {
        assert_eq!(
            request_kind_for_boundary(
                "Map Preview",
                "route planner declined automatic map selection"
            ),
            "route_policy_gap"
        );
    }
}
