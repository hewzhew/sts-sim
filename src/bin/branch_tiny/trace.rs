use std::fs::{self, File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::Path;

use serde_json::{json, Value};
use sts_simulator::eval::run_control::{render_auto_applied_step_compact_v1, RunControlCommand};

use super::owners::{
    render_shop_tiny_annotation_compact, reward_plan_lane_label, ChoiceAnnotation, OwnerChoice,
};
use super::render;
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
            "boss_search": {"nodes": args.boss_search_nodes, "ms": args.boss_search_ms},
        }))
    }

    pub(super) fn record_node(
        &mut self,
        generation: usize,
        branch: &Branch,
        choices: &[OwnerChoice],
        expanded: usize,
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
            },
            "status": status_value(&branch.status),
            "arrived": branch.path.last().map(path_step_value),
            "auto": branch.auto_steps.iter()
                .map(render_auto_applied_step_compact_v1)
                .collect::<Vec<_>>(),
            "boss_retry": branch.boss_retry.as_ref().map(|retry| json!({
                "status": match &retry.status {
                    BossRetryStatus::Failed(reason) => json!({"kind": "failed", "reason": reason}),
                    BossRetryStatus::Won => json!({"kind": "won"}),
                    BossRetryStatus::Advanced(boundary) => json!({"kind": "advanced", "boundary": boundary}),
                },
                "nodes": retry.max_nodes,
                "ms": retry.wall_ms,
                "actions": retry.action_keys,
            })),
            "choices": choices.iter().enumerate()
                .map(|(index, choice)| choice_value(index, choice, index < expanded))
                .collect::<Vec<_>>(),
        }))
    }

    fn write(&mut self, value: Value) -> Result<(), String> {
        serde_json::to_writer(&mut self.out, &value).map_err(|err| err.to_string())?;
        self.out.write_all(b"\n").map_err(|err| err.to_string())
    }
}

fn choice_value(index: usize, choice: &OwnerChoice, expanded: bool) -> Value {
    json!({
        "rank": index + 1,
        "expanded": expanded,
        "auto_expand": choice.auto_expand_allowed(),
        "inspect_only": choice.inspect_only_reason(),
        "key": choice.key.as_ref(),
        "label": choice.label,
        "action": command_value(&choice.action),
        "annotation": annotation_value(&choice.annotation),
        "rendered": render::render_timeline_choice(choice),
    })
}

fn path_step_value(step: &BranchPathStep) -> Value {
    json!({
        "key": step.key.as_ref(),
        "label": step.label,
        "action": command_value(&step.action),
        "annotation": annotation_value(&step.annotation),
    })
}

fn annotation_value(annotation: &ChoiceAnnotation) -> Value {
    match annotation {
        ChoiceAnnotation::None => Value::Null,
        ChoiceAnnotation::Reward { admission, lane } => json!({
            "kind": "reward",
            "lane": reward_plan_lane_label(*lane),
            "admission": sts_simulator::ai::strategy::reward_admission::render_reward_admission_compact(admission),
        }),
        ChoiceAnnotation::BossRelic(admission) => json!({
            "kind": "boss_relic",
            "admission": sts_simulator::ai::strategy::boss_relic_admission::render_boss_relic_admission_compact(admission),
        }),
        ChoiceAnnotation::ShopTiny(annotation) => json!({
            "kind": "shop_tiny",
            "summary": render_shop_tiny_annotation_compact(annotation),
        }),
    }
}

fn status_value(status: &BranchStatus) -> Value {
    match status {
        BranchStatus::Running { boundary, owner } => {
            json!({"kind": "running", "boundary": boundary, "owner": owner_value(*owner)})
        }
        BranchStatus::Terminal(result) => json!({"kind": "terminal", "result": result}),
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

fn command_value(command: &RunControlCommand) -> Value {
    json!({"debug": format!("{command:?}")})
}
