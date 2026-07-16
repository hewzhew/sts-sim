use std::collections::VecDeque;

use serde_json::{json, Value};
use sts_simulator::ai::strategy::decision_pipeline::{
    candidate_lane_label, CleanupTarget, DecisionCandidateKind,
};
use sts_simulator::ai::strategy::reward_admission::RewardAdmission;
use sts_simulator::ai::strategy::shop_boss_preview::classify_shop_boss_preview_candidate;

use super::branch_path::BranchPathStep;
use super::owner_model::{cleanup_target_label, ChoiceAnnotation, OwnerChoice};
use super::{
    combat_portfolio_json, run_state_json, Args, BoundarySite, Branch, BranchStatus, Owner,
};

pub(super) fn run_start_event(args: Args) -> Value {
    json!({
        "event": "run_start",
        "schema": "branch_tiny_trace_v3",
        "seed": args.seed,
        "ascension": args.ascension,
        "generations": args.generations,
        "max_branches": args.max_branches,
        "search": {"nodes": args.search_nodes, "ms": args.search_ms},
        "rescue_search": {"nodes": args.rescue_search_nodes, "ms": args.rescue_search_ms},
        "boss_search": {"nodes": args.boss_search_nodes, "ms": args.boss_search_ms},
    })
}

pub(super) fn node_event(
    generation: usize,
    branch: &Branch,
    choices: &[OwnerChoice],
    expanded: &[bool],
) -> Value {
    json!({
        "event": "node",
        "generation": generation,
        "branch_id": branch.id,
        "parent_id": branch.parent_id,
        "path_len": branch.path.len(),
        "state": {
            "act": branch.session.run_state.act_num,
            "floor": branch.session.run_state.floor_num,
            "hp": branch.session.run_state.current_hp,
            "max_hp": branch.session.run_state.max_hp,
            "gold": branch.session.run_state.gold,
            "deck_size": branch.session.run_state.master_deck.len(),
            "deck_hash": run_state_json::deck_hash(&branch.session.run_state.master_deck),
            "strategic_deficit": run_state_json::strategic_deficit_value(&branch.session.run_state),
        },
        "status": status_value(&branch.status),
        "arrived": branch.path.last().map(path_step_value),
        "recent_progress_journal": &branch.recent_progress_journal,
        "recent_planner_capture": &branch.recent_planner_capture,
        "combat_search": branch.combat_search,
        "combat_portfolio": branch.combat_portfolio.as_ref().map(combat_portfolio_json::trace_value),
        "choices": choices.iter().enumerate()
            .map(|(index, choice)| {
                choice_value(index, choice, expanded.get(index).copied().unwrap_or(false))
            })
            .collect::<Vec<_>>(),
    })
}

pub(super) fn branch_snapshot_event(
    generation: usize,
    reason: &'static str,
    branch: &Branch,
) -> Value {
    json!({
        "event": "branch_snapshot",
        "generation": generation,
        "reason": reason,
        "branch": branch_snapshot_value(branch),
    })
}

pub(super) fn frontier_snapshot_event(generation: usize, frontier: &VecDeque<Branch>) -> Value {
    json!({
        "event": "frontier_snapshot",
        "generation": generation,
        "branches": frontier.iter().map(branch_snapshot_value).collect::<Vec<_>>(),
    })
}

fn choice_value(index: usize, choice: &OwnerChoice, expanded: bool) -> Value {
    json!({
        "rank": index + 1,
        "expanded": expanded,
        "auto_expand": choice.auto_expand_allowed(),
        "inspect_only": choice.inspect_only_reason(),
        "key": choice.key.as_ref(),
        "annotation": annotation_value(&choice.annotation),
    })
}

fn path_step_value(step: &BranchPathStep) -> Value {
    json!({
        "state_before": step.state_before.as_ref(),
        "decision_delta": step.decision_delta.as_ref(),
        "key": step.key.as_ref(),
        "annotation": serde_json::to_value(&step.annotation).unwrap_or(Value::Null),
        "candidate_pool": serde_json::to_value(&step.candidate_pool).unwrap_or(Value::Null),
        "shop_boss_preview_candidates": serde_json::to_value(&step.shop_boss_preview_candidates)
            .unwrap_or(Value::Null),
    })
}

fn annotation_value(annotation: &ChoiceAnnotation) -> Value {
    match annotation {
        ChoiceAnnotation::None => Value::Null,
        ChoiceAnnotation::Candidate(decision) => json!({
            "kind": "candidate",
            "lane": candidate_lane_label(decision.evaluation.lane),
            "score": decision.evaluation.total_score(),
            "scores": decision.evaluation.scores.iter().map(|score| json!({
                "by": score.by,
                "value": score.value,
            })).collect::<Vec<_>>(),
            "candidate": candidate_kind_value(decision.evaluation.candidate.kind),
            "shop_boss_preview": shop_boss_preview_value(decision.evaluation.candidate.kind),
            "admission": decision.admission.as_ref().map(admission_value),
        }),
        ChoiceAnnotation::BossRelic(admission) => json!({
            "kind": "boss_relic",
            "relic": admission.relic,
            "lane": format!("{:?}", admission.lane),
            "class": format!("{:?}", admission.class),
        }),
    }
}

fn admission_value(admission: &RewardAdmission) -> Value {
    json!({
        "card": admission.card,
        "class": format!("{:?}", admission.class),
    })
}

fn shop_boss_preview_value(kind: DecisionCandidateKind) -> Option<Value> {
    match kind {
        DecisionCandidateKind::ShopBuyCard { .. }
        | DecisionCandidateKind::ShopBuyRelic { .. }
        | DecisionCandidateKind::ShopBuyPotion { .. }
        | DecisionCandidateKind::ShopPurge { .. }
        | DecisionCandidateKind::ShopLeave => {
            let preview = classify_shop_boss_preview_candidate(kind);
            Some(json!({
                "class": format!("{:?}", preview.class),
                "include_in_v0": preview.include_in_v0,
                "reason": preview.reason,
            }))
        }
        _ => None,
    }
}

pub(super) fn candidate_kind_value(kind: DecisionCandidateKind) -> Value {
    match kind {
        DecisionCandidateKind::CardRewardPick { card, upgrades } => json!({
            "kind": "card_reward_pick",
            "card": card,
            "upgrades": upgrades,
        }),
        DecisionCandidateKind::CardRewardSkip => json!({"kind": "card_reward_skip"}),
        DecisionCandidateKind::BossRelicPick { relic } => json!({
            "kind": "boss_relic_pick",
            "relic": relic,
        }),
        DecisionCandidateKind::BossRelicSkip => json!({"kind": "boss_relic_skip"}),
        DecisionCandidateKind::ShopBuyCard {
            card,
            upgrades,
            price,
        } => json!({
            "kind": "shop_buy_card",
            "card": card,
            "upgrades": upgrades,
            "price": price,
        }),
        DecisionCandidateKind::ShopBuyRelic { relic, price } => json!({
            "kind": "shop_buy_relic",
            "relic": relic,
            "price": price,
        }),
        DecisionCandidateKind::ShopBuyPotion { potion, price } => json!({
            "kind": "shop_buy_potion",
            "potion": potion,
            "price": price,
        }),
        DecisionCandidateKind::ShopPurge { target } => json!({
            "kind": "shop_purge",
            "target": cleanup_target_value(target),
        }),
        DecisionCandidateKind::ShopOpenRewards => json!({"kind": "shop_open_rewards"}),
        DecisionCandidateKind::ShopLeave => json!({"kind": "shop_leave"}),
        DecisionCandidateKind::Unsupported => json!({"kind": "unsupported"}),
    }
}

fn cleanup_target_value(target: CleanupTarget) -> Value {
    json!({
        "kind": format!("{target:?}"),
        "label": cleanup_target_label(target),
    })
}

fn status_value(status: &BranchStatus) -> Value {
    match status {
        BranchStatus::Running { boundary, owner } => {
            json!({"kind": "running", "boundary": boundary, "owner": owner_value(*owner)})
        }
        BranchStatus::AwaitingAuto { boundary, reason } => {
            json!({"kind": "awaiting_auto", "boundary": boundary, "reason": reason})
        }
        BranchStatus::Terminal(result) => json!({"kind": "terminal", "result": result.as_str()}),
        BranchStatus::AutomationGap { boundary, site } => {
            json!({"kind": "automation_gap", "boundary": boundary, "site": site_value(*site)})
        }
        BranchStatus::CombatGap { boundary, reason } => {
            json!({"kind": "combat_gap", "boundary": boundary, "reason": reason})
        }
        BranchStatus::OperationBudgetExhausted { boundary, reason } => {
            json!({"kind": "operation_budget_exhausted", "boundary": boundary, "reason": reason})
        }
        BranchStatus::BudgetGap { boundary, reason } => {
            json!({"kind": "budget_gap", "boundary": boundary, "reason": reason})
        }
        BranchStatus::ApplyFailed(err) => json!({"kind": "apply_failed", "reason": err}),
        BranchStatus::AdvanceFailed(err) => json!({"kind": "advance_failed", "reason": err}),
    }
}

fn owner_value(owner: Owner) -> Value {
    match owner {
        Owner::NeowStart => json!({"kind": "neow_start"}),
        Owner::CardReward => json!({"kind": "card_reward"}),
        Owner::BossRelic => json!({"kind": "boss_relic"}),
        Owner::Event(event_id) => json!({"kind": "event", "event_id": format!("{event_id:?}")}),
        Owner::RewardTiny => json!({"kind": "reward_tiny"}),
        Owner::ShopTiny => json!({"kind": "shop_tiny"}),
        Owner::Campfire => json!({"kind": "campfire"}),
        Owner::RunChoice => json!({"kind": "run_choice"}),
    }
}

fn site_value(site: BoundarySite) -> Value {
    match site {
        BoundarySite::Event(event_id) => {
            json!({"kind": "event", "event_id": format!("{event_id:?}")})
        }
        BoundarySite::Reward => json!({"kind": "reward"}),
        BoundarySite::Shop => json!({"kind": "shop"}),
        BoundarySite::Route => json!({"kind": "route"}),
        BoundarySite::Campfire => json!({"kind": "campfire"}),
        BoundarySite::BossRelic => json!({"kind": "boss_relic"}),
        BoundarySite::RunChoice => json!({"kind": "run_choice"}),
        BoundarySite::Treasure => json!({"kind": "treasure"}),
        BoundarySite::Terminal => json!({"kind": "terminal"}),
        BoundarySite::Unknown => json!({"kind": "unknown"}),
    }
}

fn branch_snapshot_value(branch: &Branch) -> Value {
    let run = &branch.session.run_state;
    json!({
        "branch_id": branch.id,
        "parent_id": branch.parent_id,
        "path_len": branch.path.len(),
        "state": {
            "act": run.act_num,
            "floor": run.floor_num,
            "hp": run.current_hp,
            "max_hp": run.max_hp,
            "gold": run.gold,
            "deck_size": run.master_deck.len(),
            "deck_hash": run_state_json::deck_hash(&run.master_deck),
            "strategic_deficit": run_state_json::strategic_deficit_value(run),
        },
        "status": status_value(&branch.status),
        "deck": run_state_json::deck_value(run),
        "relics": run_state_json::relics_value(run),
        "potions": run_state_json::potions_value(run),
    })
}

#[cfg(test)]
mod tests {
    use sts_simulator::eval::run_control::{
        RunControlAutoStepOptions, RunControlConfig, RunControlRouteAutomationMode,
        RunControlSession, RunProgressJournalV1,
    };
    use sts_simulator::state::core::EngineState;

    #[test]
    fn route_auto_step_trace_retains_typed_map_decision_packet() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.event_state = None;
        session.engine_state = EngineState::MapNavigation;

        let outcome = session
            .apply_progress_step(RunControlAutoStepOptions {
                route: RunControlRouteAutomationMode::Planner,
                ..RunControlAutoStepOptions::default()
            })
            .expect("owner progress step should select one route");
        let journal = RunProgressJournalV1::from_committed_steps(outcome.progress_steps)
            .expect("route progress should form one journal segment");
        let value = serde_json::to_value(journal).expect("journal should serialize");
        let route_annotation = value["entries"][0]["record"]["trace_annotations"]
            .as_array()
            .and_then(|annotations| {
                annotations
                    .iter()
                    .find(|annotation| annotation["kind"] == "route_planner_selection")
            })
            .expect("decision transaction should retain route planner evidence");
        assert_eq!(
            route_annotation["map_decision_packet"]["schema_name"],
            serde_json::json!("MapDecisionPacketV1")
        );
        assert!(route_annotation["map_decision_packet"]["candidates"]
            .as_array()
            .is_some_and(|candidates| !candidates.is_empty()));
    }
}
