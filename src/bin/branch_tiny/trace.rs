use std::collections::hash_map::DefaultHasher;
use std::collections::VecDeque;
use std::fs::{self, File, OpenOptions};
use std::hash::{Hash, Hasher};
use std::io::{BufWriter, Write};
use std::path::Path;

use serde_json::{json, Map, Value};
use sts_simulator::ai::strategy::decision_pipeline::{
    candidate_lane_label, CleanupTarget, DecisionCandidateKind,
};
use sts_simulator::ai::strategy::reward_admission::RewardAdmission;
use sts_simulator::eval::run_control::{RunControlAutoAppliedKindV1, RunControlAutoAppliedStepV1};
use sts_simulator::runtime::combat::CombatCard;

use super::owner_model::{cleanup_target_label, ChoiceAnnotation, OwnerChoice};
use super::{Args, BossRetryStatus, BoundarySite, Branch, BranchPathStep, BranchStatus, Owner};

pub(super) struct TraceWriter {
    out: BufWriter<File>,
}

impl TraceWriter {
    pub(super) fn create(path: &Path) -> Result<Self, String> {
        if let Some(parent) = path
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
        {
            fs::create_dir_all(parent).map_err(|err| err.to_string())?;
        }
        let file = OpenOptions::new()
            .create(true)
            .truncate(true)
            .write(true)
            .open(path)
            .map_err(|err| format!("failed to create trace {}: {err}", path.display()))?;
        Ok(Self {
            out: BufWriter::new(file),
        })
    }

    pub(super) fn record_run(&mut self, args: Args) -> Result<(), String> {
        self.write(json!({
            "event": "run_start",
            "schema": "branch_tiny_trace_v1",
            "seed": args.seed,
            "ascension": args.ascension,
            "generations": args.generations,
            "max_branches": args.max_branches,
            "search": {"nodes": args.search_nodes, "ms": args.search_ms},
            "rescue_search": {"nodes": args.rescue_search_nodes, "ms": args.rescue_search_ms},
            "boss_search": {"nodes": args.boss_search_nodes, "ms": args.boss_search_ms},
        }))
    }

    pub(super) fn record_node(
        &mut self,
        generation: usize,
        branch: &Branch,
        choices: &[OwnerChoice],
        expanded: &[bool],
    ) -> Result<(), String> {
        self.write(json!({
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
                "deck_hash": deck_hash(&branch.session.run_state.master_deck),
            },
            "status": status_value(&branch.status),
            "arrived": branch.path.last().map(path_step_value),
            "auto": branch.auto_steps.iter()
                .filter(|step| step.kind != RunControlAutoAppliedKindV1::AutoCapture)
                .map(auto_step_value)
                .collect::<Vec<_>>(),
            "combat_search": branch.combat_search,
            "boss_retry": branch.boss_retry.as_ref().map(|retry| json!({
                "status": match &retry.status {
                    BossRetryStatus::Failed(reason) => json!({"kind": "failed", "reason": reason}),
                    BossRetryStatus::Advanced(boundary) => json!({"kind": "advanced", "boundary": boundary}),
                    BossRetryStatus::Terminal(result) => json!({"kind": "terminal", "result": result.as_str()}),
                },
                "nodes": retry.max_nodes,
                "ms": retry.wall_ms,
                "actions": retry.action_keys,
                "attempts": retry.attempts.iter().map(|attempt| json!({
                    "label": attempt.label,
                    "status": match &attempt.status {
                        BossRetryStatus::Failed(reason) => json!({"kind": "failed", "reason": reason}),
                        BossRetryStatus::Advanced(boundary) => json!({"kind": "advanced", "boundary": boundary}),
                        BossRetryStatus::Terminal(result) => json!({"kind": "terminal", "result": result.as_str()}),
                    },
                    "nodes": attempt.max_nodes,
                    "ms": attempt.wall_ms,
                    "potion_policy": attempt.potion_policy,
                    "max_potions_used": attempt.max_potions_used,
                    "actions": attempt.action_keys,
                })).collect::<Vec<_>>(),
            })),
            "choices": choices.iter().enumerate()
                .map(|(index, choice)| {
                    choice_value(index, choice, expanded.get(index).copied().unwrap_or(false))
                })
                .collect::<Vec<_>>(),
        }))
    }

    pub(super) fn record_branch_snapshot(
        &mut self,
        generation: usize,
        reason: &'static str,
        branch: &Branch,
    ) -> Result<(), String> {
        self.write(json!({
            "event": "branch_snapshot",
            "generation": generation,
            "reason": reason,
            "branch": branch_snapshot_value(branch),
        }))
    }

    pub(super) fn record_frontier_snapshot(
        &mut self,
        generation: usize,
        frontier: &VecDeque<Branch>,
    ) -> Result<(), String> {
        self.write(json!({
            "event": "frontier_snapshot",
            "generation": generation,
            "branches": frontier.iter().map(branch_snapshot_value).collect::<Vec<_>>(),
        }))
    }

    fn write(&mut self, value: Value) -> Result<(), String> {
        serde_json::to_writer(&mut self.out, &value).map_err(|err| err.to_string())?;
        self.out.write_all(b"\n").map_err(|err| err.to_string())?;
        self.out.flush().map_err(|err| err.to_string())
    }
}

fn auto_step_value(step: &RunControlAutoAppliedStepV1) -> Value {
    let Some(result) = step.action_result.as_ref() else {
        return json!({"kind": auto_step_kind_value(step.kind)});
    };
    json!({
        "kind": auto_step_kind_value(step.kind),
        "status": result.status,
        "changes": result.changes,
    })
}

fn auto_step_kind_value(kind: RunControlAutoAppliedKindV1) -> &'static str {
    match kind {
        RunControlAutoAppliedKindV1::RewardAutomation => "reward_automation",
        RunControlAutoAppliedKindV1::CombatSearch => "combat_search",
        RunControlAutoAppliedKindV1::RoutePlanner => "route_planner",
        RunControlAutoAppliedKindV1::RewardOverlay => "reward_overlay",
        RunControlAutoAppliedKindV1::NoncombatPolicy => "noncombat_policy",
        RunControlAutoAppliedKindV1::RoutineCandidate => "routine_candidate",
        RunControlAutoAppliedKindV1::AutoCapture => "auto_capture",
        RunControlAutoAppliedKindV1::OwnerRoutine => "owner_routine",
    }
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
        "key": step.key.as_ref(),
        "annotation": serde_json::to_value(&step.annotation).unwrap_or(Value::Null),
    })
}

fn annotation_value(annotation: &ChoiceAnnotation) -> Value {
    match annotation {
        ChoiceAnnotation::None => Value::Null,
        ChoiceAnnotation::Candidate(decision) => json!({
            "kind": "candidate",
            "lane": candidate_lane_label(decision.evaluation.lane),
            "score": decision.evaluation.total_score(),
            "candidate": candidate_kind_value(decision.evaluation.candidate.kind),
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
        BranchStatus::Terminal(result) => json!({"kind": "terminal", "result": result.as_str()}),
        BranchStatus::AutomationGap { boundary, site } => {
            json!({"kind": "automation_gap", "boundary": boundary, "site": site_value(*site)})
        }
        BranchStatus::CombatGap { boundary, reason } => {
            json!({"kind": "combat_gap", "boundary": boundary, "reason": reason})
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
            "deck_hash": deck_hash(&run.master_deck),
        },
        "status": status_value(&branch.status),
        "deck": run.master_deck.iter().map(card_snapshot_value).collect::<Vec<_>>(),
        "relics": run.relics.iter().map(|relic| {
            let mut value = Map::from_iter([("id".to_string(), json!(relic.id))]);
            if relic.counter != -1 {
                value.insert("counter".to_string(), json!(relic.counter));
            }
            if relic.used_up {
                value.insert("used_up".to_string(), json!(true));
            }
            if relic.amount != 0 {
                value.insert("amount".to_string(), json!(relic.amount));
            }
            Value::Object(value)
        }).collect::<Vec<_>>(),
        "potions": run.potions.iter().map(|slot| {
            slot.as_ref().map(|potion| json!({
                "id": potion.id,
                "uuid": potion.uuid,
            }))
        }).collect::<Vec<_>>(),
    })
}

fn card_snapshot_value(card: &CombatCard) -> Value {
    let mut value = Map::from_iter([
        ("id".to_string(), json!(card.id)),
        ("uuid".to_string(), json!(card.uuid)),
    ]);
    if card.upgrades != 0 {
        value.insert("upgrades".to_string(), json!(card.upgrades));
    }
    if card.misc_value != 0 {
        value.insert("misc".to_string(), json!(card.misc_value));
    }
    Value::Object(value)
}

fn deck_hash(deck: &[CombatCard]) -> String {
    let mut hasher = DefaultHasher::new();
    for card in deck {
        card.id.hash(&mut hasher);
        card.uuid.hash(&mut hasher);
        card.upgrades.hash(&mut hasher);
        card.misc_value.hash(&mut hasher);
    }
    format!("{:016x}", hasher.finish())
}
