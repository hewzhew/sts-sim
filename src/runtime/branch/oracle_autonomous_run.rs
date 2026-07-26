use std::path::{Path, PathBuf};
use std::time::Instant;

use serde_json::{json, Value};

use crate::eval::run_control::{
    OracleAnalysisAdvanceRequestV1, OracleAnalysisNodeViewV1, OracleRunBoundaryV1,
};

use super::OracleAnalysisWorkspaceV1;

pub(super) struct OwnerBatchResult {
    pub(super) applied_count: usize,
    pub(super) applied: Vec<Value>,
    pub(super) stopped: String,
}

pub(super) fn apply_owner_steps(
    workspace: &mut OracleAnalysisWorkspaceV1,
    steps: u8,
) -> Result<OwnerBatchResult, String> {
    if steps == 0 {
        return Err("oracle owner steps must be positive".to_string());
    }

    let mut applied = Vec::new();
    let mut stopped = "step_limit".to_string();
    for _ in 0..steps {
        let current = workspace.view()?;
        let matching = current
            .choices
            .iter()
            .filter(|choice| choice.owner_rank == 0)
            .collect::<Vec<_>>();
        let [choice] = matching.as_slice() else {
            stopped = if current.choices.is_empty() {
                "no_choices".to_string()
            } else {
                format!("rank_zero_choice_count={}", matching.len())
            };
            break;
        };
        let parent_node_id = current.node_id;
        let candidate_id = choice.candidate_id.clone();
        let label = choice.label.clone();
        let choice_ref = choice.choice_ref.clone();
        let next = match workspace.try_choice(&choice_ref) {
            Ok(next) => next,
            Err(error) => {
                stopped = format!("choice_execution_error={error}");
                break;
            }
        };
        if next.node_id == parent_node_id {
            stopped = format!("choice_made_no_progress={label}");
            break;
        }
        applied.push(json!({
            "node": parent_node_id,
            "candidate_id": candidate_id,
            "label": label,
        }));
    }
    let applied_count = applied.len();
    Ok(OwnerBatchResult {
        applied_count,
        applied,
        stopped,
    })
}

#[derive(Clone, Debug)]
pub struct OracleAutonomousRunConfigV1 {
    pub hallway_wall_ms: u64,
    pub elite_wall_ms: u64,
    pub boss_wall_ms: u64,
    pub max_quanta: usize,
    pub quantum_nodes: usize,
    pub quantum_ms: u64,
    pub max_boundaries: usize,
    pub run_wall_ms: Option<u64>,
    pub export_continuation: Option<PathBuf>,
}

#[derive(Default)]
struct AutonomousRunTiming {
    owner_ms: u64,
    combat_advance_ms: u64,
    reported_combat_ms: u64,
}

pub fn run_oracle_analysis_to_stop_v1(
    workspace: &mut OracleAnalysisWorkspaceV1,
    config: &OracleAutonomousRunConfigV1,
) -> Result<Value, String> {
    if config.max_boundaries == 0
        || config.max_quanta == 0
        || config.quantum_nodes == 0
        || config.quantum_ms == 0
        || config.hallway_wall_ms == 0
        || config.elite_wall_ms == 0
        || config.boss_wall_ms == 0
    {
        return Err("oracle run budgets and max-boundaries must be positive".to_string());
    }

    let run_started = Instant::now();
    let start_node = workspace.view()?.node_id;
    let mut owner_decisions = 0_u64;
    let mut combats = Vec::new();
    let mut timing = AutonomousRunTiming::default();

    for _ in 0..config.max_boundaries {
        let node = workspace.view()?;
        if config
            .run_wall_ms
            .is_some_and(|limit| elapsed_millis(run_started) >= limit)
        {
            return Ok(stopped_autonomous_run_report(
                start_node,
                &node,
                owner_decisions,
                &combats,
                &timing,
                elapsed_millis(run_started),
                "run_wall_budget",
            ));
        }
        if matches!(
            node.boundary,
            OracleRunBoundaryV1::TerminalVictory | OracleRunBoundaryV1::TerminalDefeat
        ) {
            return terminal_autonomous_run_report(
                workspace,
                &node,
                start_node,
                owner_decisions,
                &combats,
                &timing,
                elapsed_millis(run_started),
                config.export_continuation.as_deref(),
            );
        }

        if !node.choices.is_empty() {
            let owner_started = Instant::now();
            let owner = apply_owner_steps(workspace, 64)?;
            timing.owner_ms = timing
                .owner_ms
                .saturating_add(elapsed_millis(owner_started));
            if owner.applied_count == 0 {
                return Ok(stopped_autonomous_run_report(
                    start_node,
                    &node,
                    owner_decisions,
                    &combats,
                    &timing,
                    elapsed_millis(run_started),
                    "owner_choice_missing_or_ambiguous",
                ));
            }
            owner_decisions = owner_decisions.saturating_add(owner.applied_count as u64);
            continue;
        }

        if node.boundary != OracleRunBoundaryV1::Combat {
            return Ok(stopped_autonomous_run_report(
                start_node,
                &node,
                owner_decisions,
                &combats,
                &timing,
                elapsed_millis(run_started),
                "noncombat_boundary_without_owner_choice",
            ));
        }

        let encounter = node
            .encounter
            .as_ref()
            .ok_or_else(|| "combat boundary omitted encounter metadata".to_string())?;
        let combat_wall_ms = if encounter.is_boss {
            config.boss_wall_ms
        } else if encounter.is_elite {
            config.elite_wall_ms
        } else {
            config.hallway_wall_ms
        };
        let wall_ms = config.run_wall_ms.map_or(combat_wall_ms, |limit| {
            combat_wall_ms.min(limit.saturating_sub(elapsed_millis(run_started)).max(1))
        });
        let combat_node = node.node_id;

        if node
            .combat
            .as_ref()
            .and_then(|combat| combat.incumbent_final_hp)
            .is_some()
        {
            let after = workspace.accept_combat_incumbent()?;
            combats.push(json!({
                "node": combat_node,
                "act": node.act,
                "floor": node.floor,
                "start_hp": node.current_hp,
                "kind": encounter_kind(encounter.is_elite, encounter.is_boss),
                "monsters": encounter.monsters.iter().map(|monster| monster.label.clone()).collect::<Vec<_>>(),
                "budget_ms": 0,
                "elapsed_ms": 0,
                "accepted_existing_incumbent": true,
                "search": compact_run_combat_progress(node.combat.as_ref()),
                "after": compact_run_node(&after),
            }));
            continue;
        }

        let advance_started = Instant::now();
        let (report, mut after) = workspace.advance(OracleAnalysisAdvanceRequestV1 {
            max_quanta: config.max_quanta,
            quantum_nodes: config.quantum_nodes,
            quantum_ms: Some(config.quantum_ms),
            wall_ms: Some(wall_ms),
            improve_incumbent: true,
        })?;
        timing.combat_advance_ms = timing
            .combat_advance_ms
            .saturating_add(elapsed_millis(advance_started));
        timing.reported_combat_ms = timing.reported_combat_ms.saturating_add(report.elapsed_ms);

        let incumbent = report
            .combat
            .as_ref()
            .and_then(|combat| combat.incumbent_final_hp);
        let materialized = after.boundary != OracleRunBoundaryV1::Combat;
        let accepted = if !materialized && incumbent.is_some() {
            after = workspace.accept_combat_incumbent()?;
            true
        } else {
            false
        };
        combats.push(json!({
            "node": combat_node,
            "act": node.act,
            "floor": node.floor,
            "start_hp": node.current_hp,
            "kind": encounter_kind(encounter.is_elite, encounter.is_boss),
            "monsters": encounter.monsters.iter().map(|monster| monster.label.clone()).collect::<Vec<_>>(),
            "budget_ms": wall_ms,
            "elapsed_ms": report.elapsed_ms,
            "accepted_incumbent": accepted,
            "search": compact_run_combat_progress(report.combat.as_ref()),
            "after": compact_run_node(&after),
        }));

        if !materialized && !accepted {
            return Ok(stopped_autonomous_run_report(
                start_node,
                &after,
                owner_decisions,
                &combats,
                &timing,
                elapsed_millis(run_started),
                "combat_budget_unknown_without_witness",
            ));
        }
    }

    let node = workspace.view()?;
    if matches!(
        node.boundary,
        OracleRunBoundaryV1::TerminalVictory | OracleRunBoundaryV1::TerminalDefeat
    ) {
        return terminal_autonomous_run_report(
            workspace,
            &node,
            start_node,
            owner_decisions,
            &combats,
            &timing,
            elapsed_millis(run_started),
            config.export_continuation.as_deref(),
        );
    }
    Ok(stopped_autonomous_run_report(
        start_node,
        &node,
        owner_decisions,
        &combats,
        &timing,
        elapsed_millis(run_started),
        "boundary_limit",
    ))
}

fn terminal_autonomous_run_report(
    workspace: &OracleAnalysisWorkspaceV1,
    node: &OracleAnalysisNodeViewV1,
    start_node: usize,
    owner_decisions: u64,
    combats: &[Value],
    timing: &AutonomousRunTiming,
    run_elapsed_ms: u64,
    export_continuation: Option<&Path>,
) -> Result<Value, String> {
    let victory = node.boundary == OracleRunBoundaryV1::TerminalVictory;
    let final_node = node.node_id;
    let mut verification_ms = 0;
    let mut export_ms = 0;
    let mut verification = None;
    let mut export = None;
    if victory {
        let continuation = workspace.continuation(final_node)?;
        if let Some(path) = export_continuation {
            let export_started = Instant::now();
            super::oracle_run::save_oracle_run_continuation_v1(path, &continuation)?;
            export_ms = elapsed_millis(export_started);
            export = Some(json!({
                "node_id": final_node,
                "path": path,
                "journal_entries": continuation.journal.entries().len(),
            }));
        }
        let verify_started = Instant::now();
        let expected_final = continuation.session.into_session()?;
        let report = crate::eval::run_control::exact_replay_run_progress_journal_v1(
            workspace.seed,
            workspace.ascension,
            &continuation.journal,
            &expected_final,
        )?;
        verification_ms = elapsed_millis(verify_started);
        verification = Some(json!({
            "schema_name": "ExactOracleRunWitnessReplayV1",
            "schema_version": 1,
            "node_id": final_node,
            "report": report,
        }));
    }
    Ok(json!({
        "schema_name": "OracleAutonomousRunReportV2",
        "schema_version": 2,
        "status": if victory { "victory_verified" } else { "terminal_defeat" },
        "start_node": start_node,
        "final": compact_run_node(node),
        "owner_decisions": owner_decisions,
        "combat_count": combats.len(),
        "total_combat_elapsed_ms": timing.reported_combat_ms,
        "timing": {
            "run_ms_before_verification_and_export": run_elapsed_ms,
            "owner_ms": timing.owner_ms,
            "combat_advance_wall_ms": timing.combat_advance_ms,
            "combat_reported_ms": timing.reported_combat_ms,
            "verification_ms": verification_ms,
            "export_ms": export_ms,
        },
        "combats": combats,
        "verification": verification,
        "continuation_export": export,
    }))
}

fn stopped_autonomous_run_report(
    start_node: usize,
    node: &OracleAnalysisNodeViewV1,
    owner_decisions: u64,
    combats: &[Value],
    timing: &AutonomousRunTiming,
    run_elapsed_ms: u64,
    reason: &str,
) -> Value {
    json!({
        "schema_name": "OracleAutonomousRunReportV2",
        "schema_version": 2,
        "status": "stopped",
        "reason": reason,
        "start_node": start_node,
        "final": compact_run_node(node),
        "owner_decisions": owner_decisions,
        "combat_count": combats.len(),
        "total_combat_elapsed_ms": timing.reported_combat_ms,
        "timing": {
            "run_ms": run_elapsed_ms,
            "owner_ms": timing.owner_ms,
            "combat_advance_wall_ms": timing.combat_advance_ms,
            "combat_reported_ms": timing.reported_combat_ms,
        },
        "combats": combats,
    })
}

fn compact_run_node(node: &OracleAnalysisNodeViewV1) -> Value {
    json!({
        "node": node.node_id,
        "parent": node.canonical_parent_node_id,
        "boundary": node.boundary,
        "act": node.act,
        "floor": node.floor,
        "hp": node.current_hp,
        "max_hp": node.max_hp,
        "gold": node.gold,
        "choice_count": node.choices.len(),
        "choices": node.choices.iter().take(8).map(|choice| json!({
            "candidate_id": choice.candidate_id,
            "label": choice.label,
            "owner_rank": choice.owner_rank,
        })).collect::<Vec<_>>(),
        "child_count": node.children.len(),
    })
}

fn compact_run_combat_progress(
    combat: Option<&crate::eval::run_control::OracleAnalysisCombatProgressV1>,
) -> Value {
    let Some(combat) = combat else {
        return Value::Null;
    };
    json!({
        "generation_work": combat.generation_work,
        "exact_states": combat.exact_states,
        "completed_turn_options": combat.completed_turn_options,
        "max_player_turn": combat.max_player_turn,
        "policy_witness_proposals": combat.policy_witness_proposals,
        "policy_witness_proposal_rejections": combat.policy_witness_proposal_rejections,
        "incumbent_final_hp": combat.incumbent_final_hp,
        "incumbent_hp_loss": combat.incumbent_hp_loss,
        "incumbent_actions": combat.incumbent_action_count,
        "last_status": combat.last_status,
        "resume_kind": combat.resume_kind,
        "restart_count": combat.restart_count,
    })
}

fn encounter_kind(is_elite: bool, is_boss: bool) -> &'static str {
    if is_boss {
        "boss"
    } else if is_elite {
        "elite"
    } else {
        "hallway_or_event"
    }
}

pub(super) fn elapsed_millis(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_millis()).unwrap_or(u64::MAX)
}
