//! Read-only audit of bounded rollout guidance over exact next-turn states.

use std::cmp::Reverse;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;

use clap::Args;
use serde::Serialize;
use serde_json::{json, Value};
use sts_combat_planner::{
    CombatDecisionRoot, CombatPlanningQuantum, TurnOptionGeneratorConfig,
    TurnOptionGeneratorSession,
};
use sts_simulator::ai::combat_state_key::combat_exact_state_hash_v2;
use sts_simulator::eval::combat_action_imitation::typed_combat_feature_components_v1;
use sts_simulator::eval::combat_case::load_combat_case;
use sts_simulator::eval::combat_state_features::{
    semantic_combat_state_features_v1, CombatStateFeatureV1, COMBAT_STATE_FEATURE_SCHEMA_V1,
};
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
    /// Include the full legacy and semantic feature vectors for watched
    /// successors. Off by default because the semantic vector is intentionally
    /// detailed.
    #[arg(long)]
    include_watched_features: bool,
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
    action_labels: Vec<String>,
    negative_log_policy: f64,
    rollout_components: Vec<i32>,
    rollout_work: usize,
    existing_feature_equivalence_class_size: usize,
    same_existing_features_as_watched: bool,
    semantic_feature_equivalence_class_size: usize,
    same_semantic_features_as_watched: bool,
}

#[derive(Clone, Debug)]
struct EvaluatedCandidate {
    policy_rank: usize,
    exact_state_hash: String,
    watched: bool,
    boundary: String,
    action_count: usize,
    action_labels: Vec<String>,
    negative_log_policy: f64,
    rollout_components: Vec<i32>,
    rollout_work: usize,
    existing_features: Vec<i32>,
    semantic_features: Vec<CombatStateFeatureV1>,
}

pub fn audit(args: BoundarySuccessorLookaheadArgs) -> Result<Value, String> {
    if args.generation_work == 0 {
        return Err("generation_work must be positive".to_string());
    }
    if args.lookahead_work_per_successor == 0 {
        return Err("lookahead_work_per_successor must be positive".to_string());
    }

    let loaded = load_combat_case(&args.case)?;
    let root_position = loaded.position;
    let root_exact_state_hash =
        combat_exact_state_hash_v2(&root_position.engine, &root_position.combat);
    let root = CombatDecisionRoot::new(root_position.clone())
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
                action_labels: super::readable_turn_option_action_labels(
                    &root_position,
                    option.actions(),
                    args.max_engine_steps_per_transition,
                )?,
                negative_log_policy: option.negative_log_policy(),
                rollout_components: evaluation.guide.rank.components().to_vec(),
                rollout_work: evaluation.work,
                existing_features: typed_combat_feature_components_v1(option.exact_successor()),
                semantic_features: semantic_combat_state_features_v1(option.exact_successor()),
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

    let existing_feature_counts = evaluated.iter().fold(
        HashMap::<Vec<i32>, usize>::new(),
        |mut counts, candidate| {
            *counts
                .entry(candidate.existing_features.clone())
                .or_default() += 1;
            counts
        },
    );
    let distinct_existing_feature_vectors = existing_feature_counts.len();
    let largest_existing_feature_equivalence_class = existing_feature_counts
        .values()
        .copied()
        .max()
        .unwrap_or_default();
    let existing_features_by_hash = evaluated
        .iter()
        .map(|candidate| {
            (
                candidate.exact_state_hash.clone(),
                candidate.existing_features.clone(),
            )
        })
        .collect::<HashMap<_, _>>();
    let watched_existing_features = evaluated
        .iter()
        .filter(|candidate| candidate.watched)
        .map(|candidate| candidate.existing_features.clone())
        .collect::<HashSet<_>>();
    let semantic_feature_counts = evaluated.iter().fold(
        HashMap::<Vec<CombatStateFeatureV1>, usize>::new(),
        |mut counts, candidate| {
            *counts
                .entry(candidate.semantic_features.clone())
                .or_default() += 1;
            counts
        },
    );
    let distinct_semantic_feature_vectors = semantic_feature_counts.len();
    let largest_semantic_feature_equivalence_class = semantic_feature_counts
        .values()
        .copied()
        .max()
        .unwrap_or_default();
    let semantic_features_by_hash = evaluated
        .iter()
        .map(|candidate| {
            (
                candidate.exact_state_hash.clone(),
                candidate.semantic_features.clone(),
            )
        })
        .collect::<HashMap<_, _>>();
    let watched_semantic_features = evaluated
        .iter()
        .filter(|candidate| candidate.watched)
        .map(|candidate| candidate.semantic_features.clone())
        .collect::<HashSet<_>>();
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
            action_labels: candidate.action_labels,
            negative_log_policy: candidate.negative_log_policy,
            rollout_components: candidate.rollout_components,
            rollout_work: candidate.rollout_work,
            existing_feature_equivalence_class_size: existing_feature_counts
                .get(&candidate.existing_features)
                .copied()
                .unwrap_or_default(),
            same_existing_features_as_watched: watched_existing_features
                .contains(&candidate.existing_features),
            semantic_feature_equivalence_class_size: semantic_feature_counts
                .get(&candidate.semantic_features)
                .copied()
                .unwrap_or_default(),
            same_semantic_features_as_watched: watched_semantic_features
                .contains(&candidate.semantic_features),
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
                "existing_feature_components": args
                    .include_watched_features
                    .then(|| existing_features_by_hash.get(hash))
                    .flatten(),
                "existing_feature_equivalence_class_size": candidate
                    .map(|candidate| candidate.existing_feature_equivalence_class_size),
                "semantic_feature_components": args
                    .include_watched_features
                    .then(|| semantic_features_by_hash.get(hash))
                    .flatten(),
                "semantic_feature_equivalence_class_size": candidate
                    .map(|candidate| candidate.semantic_feature_equivalence_class_size),
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
        "existing_feature_audit": {
            "schema": "combat_action_imitation/typed_combat_feature_components_v1",
            "distinct_vectors": distinct_existing_feature_vectors,
            "largest_equivalence_class": largest_existing_feature_equivalence_class,
        },
        "semantic_feature_audit": {
            "schema": COMBAT_STATE_FEATURE_SCHEMA_V1,
            "distinct_vectors": distinct_semantic_feature_vectors,
            "largest_equivalence_class": largest_semantic_feature_equivalence_class,
        },
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
                action_labels: vec!["policy-first".to_string()],
                negative_log_policy: 0.0,
                rollout_components: vec![1, 0],
                rollout_work: 1,
                existing_features: vec![0],
                semantic_features: vec![],
            },
            EvaluatedCandidate {
                policy_rank: 9,
                exact_state_hash: "rollout-first".to_string(),
                watched: false,
                boundary: "NextPlayerTurn".to_string(),
                action_count: 1,
                action_labels: vec!["rollout-first".to_string()],
                negative_log_policy: 9.0,
                rollout_components: vec![2, 0],
                rollout_work: 1,
                existing_features: vec![1],
                semantic_features: vec![],
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
