//! Read-only audit of bounded rollout guidance over exact next-turn states.

use std::cmp::Reverse;
use std::path::PathBuf;

use clap::Args;
use serde::Serialize;
use serde_json::{json, Value};
use sts_combat_planner::{
    CombatDecisionRoot, CombatPlanningQuantum, TurnOptionGeneratorConfig,
    TurnOptionGeneratorSession,
};
use sts_simulator::ai::combat_state_key::combat_exact_state_hash_v1;
use sts_simulator::eval::combat_case::load_combat_case;
use sts_simulator::eval::run_control::{
    existing_combat_knowledge_policy_v1, existing_combat_rollout_lookahead_v1,
};
use sts_simulator::sim::combat::EngineCombatStepper;

#[derive(Clone, Debug, Args)]
pub struct BoundarySuccessorLookaheadArgs {
    /// Exact combat case whose complete first-turn successors are audited.
    #[arg(long)]
    case: PathBuf,
    /// Deterministic work used to enumerate complete first-turn successors.
    #[arg(long, default_value_t = 30_000)]
    generation_work: usize,
    /// Maximum simulated player inputs used for each independent rollout.
    #[arg(long, default_value_t = 128)]
    lookahead_work_per_successor: usize,
    /// Maximum generated successors to evaluate. Zero evaluates all of them.
    /// Watched hashes are always retained when they were generated.
    #[arg(long, default_value_t = 256)]
    candidate_limit: usize,
    /// Number of rollout-ranked candidates included in the compact report.
    /// Watched hashes are always reported in the separate watched summary.
    #[arg(long, default_value_t = 16)]
    report_limit: usize,
    /// Exact successor hashes to surface explicitly in the report.
    #[arg(long)]
    watch_state_hash: Vec<String>,
    #[arg(long, default_value_t = 250)]
    max_engine_steps_per_transition: usize,
}

#[derive(Clone, Debug, Serialize)]
struct CandidateAudit {
    generated_policy_rank: usize,
    evaluated_rollout_rank: usize,
    exact_state_hash: String,
    watched: bool,
    boundary: String,
    action_count: usize,
    negative_log_policy: f64,
    rollout_components: Vec<i32>,
    rollout_work: usize,
}

#[derive(Clone, Debug)]
struct EvaluatedCandidate {
    policy_rank: usize,
    exact_state_hash: String,
    watched: bool,
    boundary: String,
    action_count: usize,
    negative_log_policy: f64,
    rollout_components: Vec<i32>,
    rollout_work: usize,
}

pub fn audit(args: BoundarySuccessorLookaheadArgs) -> Result<Value, String> {
    if args.generation_work == 0 {
        return Err("generation_work must be positive".to_string());
    }
    if args.lookahead_work_per_successor == 0 {
        return Err("lookahead_work_per_successor must be positive".to_string());
    }

    let loaded = load_combat_case(&args.case)?;
    let root_exact_state_hash =
        combat_exact_state_hash_v1(&loaded.position.engine, &loaded.position.combat);
    let root = CombatDecisionRoot::new(loaded.position)
        .map_err(|error| format!("invalid combat case root: {error:?}"))?;
    let mut generator = TurnOptionGeneratorSession::with_policy(
        root,
        TurnOptionGeneratorConfig {
            max_engine_steps_per_transition: args.max_engine_steps_per_transition,
            ..TurnOptionGeneratorConfig::default()
        },
        existing_combat_knowledge_policy_v1(),
    );
    let mut generation_status = "NotStarted".to_string();
    let mut retained_work_items = 0usize;
    let mut generation_stopped_after_candidate_head = false;
    while generator.counters().generation_work < args.generation_work && !generator.is_finished() {
        let remaining = args
            .generation_work
            .saturating_sub(generator.counters().generation_work);
        let work = remaining.min(512);
        let generation = generator.advance(
            &EngineCombatStepper,
            CombatPlanningQuantum::deterministic(
                work,
                work.saturating_mul(args.max_engine_steps_per_transition),
            ),
        );
        generation_status = format!("{:?}", generation.status);
        retained_work_items = generation.retained_work_items;
        let candidate_head_ready =
            args.candidate_limit > 0 && generator.completed_options().len() >= args.candidate_limit;
        let watched_ready = args.watch_state_hash.iter().all(|watched| {
            generator
                .completed_options()
                .iter()
                .any(|option| option.exact_successor_hash() == watched)
        });
        if candidate_head_ready && watched_ready && !generator.is_finished() {
            generation_stopped_after_candidate_head = true;
            break;
        }
    }
    let mut generated = generator.completed_options().iter().collect::<Vec<_>>();
    generated.sort_by(|left, right| {
        left.negative_log_policy()
            .total_cmp(&right.negative_log_policy())
            .then_with(|| {
                left.exact_successor_hash()
                    .cmp(right.exact_successor_hash())
            })
    });

    let mut selected = generated
        .iter()
        .enumerate()
        .filter(|(index, option)| {
            args.candidate_limit == 0
                || *index < args.candidate_limit
                || args
                    .watch_state_hash
                    .iter()
                    .any(|watched| watched == option.exact_successor_hash())
        })
        .map(|(index, option)| (index + 1, *option))
        .collect::<Vec<_>>();
    selected.sort_by_key(|(policy_rank, _)| *policy_rank);

    let evaluator = existing_combat_rollout_lookahead_v1();
    let mut evaluated = selected
        .into_iter()
        .map(|(policy_rank, option)| {
            let evaluation = evaluator
                .evaluate(
                    option.exact_successor(),
                    args.lookahead_work_per_successor,
                    None,
                )
                .ok_or_else(|| {
                    format!(
                        "rollout evaluator produced no observation for successor {}",
                        option.exact_successor_hash()
                    )
                })?;
            Ok(EvaluatedCandidate {
                policy_rank,
                exact_state_hash: option.exact_successor_hash().to_string(),
                watched: args
                    .watch_state_hash
                    .iter()
                    .any(|watched| watched == option.exact_successor_hash()),
                boundary: format!("{:?}", option.boundary()),
                action_count: option.actions().len(),
                negative_log_policy: option.negative_log_policy(),
                rollout_components: evaluation.guide.rank.components().to_vec(),
                rollout_work: evaluation.work,
            })
        })
        .collect::<Result<Vec<_>, String>>()?;
    evaluated.sort_by_key(|candidate| {
        (
            Reverse(candidate.rollout_components.clone()),
            candidate.policy_rank,
            candidate.exact_state_hash.clone(),
        )
    });

    let candidates = evaluated
        .into_iter()
        .enumerate()
        .map(|(index, candidate)| CandidateAudit {
            generated_policy_rank: candidate.policy_rank,
            evaluated_rollout_rank: index + 1,
            exact_state_hash: candidate.exact_state_hash,
            watched: candidate.watched,
            boundary: candidate.boundary,
            action_count: candidate.action_count,
            negative_log_policy: candidate.negative_log_policy,
            rollout_components: candidate.rollout_components,
            rollout_work: candidate.rollout_work,
        })
        .collect::<Vec<_>>();
    let watched = args
        .watch_state_hash
        .iter()
        .map(|hash| {
            let candidate = candidates
                .iter()
                .find(|candidate| candidate.exact_state_hash == *hash);
            json!({
                "exact_state_hash": hash,
                "generated": generated
                    .iter()
                    .any(|option| option.exact_successor_hash() == hash),
                "evaluated": candidate.is_some(),
                "generated_policy_rank": candidate
                    .map(|candidate| candidate.generated_policy_rank),
                "evaluated_rollout_rank": candidate
                    .map(|candidate| candidate.evaluated_rollout_rank),
                "rollout_components": candidate
                    .map(|candidate| candidate.rollout_components.as_slice()),
            })
        })
        .collect::<Vec<_>>();
    let reported_candidates = candidates
        .iter()
        .take(args.report_limit)
        .cloned()
        .collect::<Vec<_>>();

    Ok(json!({
        "schema_name": "BoundarySuccessorLookaheadAuditV1",
        "schema_version": 1,
        "case": args.case,
        "root_exact_state_hash": root_exact_state_hash,
        "generation": {
            "status": generation_status,
            "stopped_after_candidate_head": generation_stopped_after_candidate_head,
            "generation_work": generator.counters().generation_work,
            "engine_steps": generator.counters().engine_steps,
            "completed_options": generated.len(),
            "retained_work_items": retained_work_items,
            "gap_count": generator.gaps().len(),
        },
        "lookahead_work_per_successor": args.lookahead_work_per_successor,
        "candidate_limit": args.candidate_limit,
        "report_limit": args.report_limit,
        "evaluated_successors": candidates.len(),
        "rank_scope": {
            "policy_rank": "among successors generated before the stated work/early-stop boundary",
            "rollout_rank": "among evaluated successors only",
        },
        "watched": watched,
        "top_candidates": reported_candidates,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rollout_order_prefers_higher_components_before_policy_rank() {
        let mut candidates = [
            EvaluatedCandidate {
                policy_rank: 1,
                exact_state_hash: "policy-first".to_string(),
                watched: false,
                boundary: "NextPlayerTurn".to_string(),
                action_count: 1,
                negative_log_policy: 0.0,
                rollout_components: vec![1, 0],
                rollout_work: 1,
            },
            EvaluatedCandidate {
                policy_rank: 9,
                exact_state_hash: "rollout-first".to_string(),
                watched: false,
                boundary: "NextPlayerTurn".to_string(),
                action_count: 1,
                negative_log_policy: 9.0,
                rollout_components: vec![2, 0],
                rollout_work: 1,
            },
        ];
        candidates.sort_by_key(|candidate| {
            (
                Reverse(candidate.rollout_components.clone()),
                candidate.policy_rank,
                candidate.exact_state_hash.clone(),
            )
        });
        assert_eq!(candidates[0].exact_state_hash, "rollout-first");
    }
}
