//! Offline complete-turn successor evidence for search-guidance experiments.
//!
//! A verified witness supplies one known winning successor at each selected
//! turn boundary. Other generated successors are evaluated independently by
//! the exact local-turn graph search. Bounded misses remain `BudgetUnknown`.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use clap::{Args, ValueEnum};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sts_combat_planner::{
    CombatDecisionRoot, CombatPlanningQuantum, CompleteTurnOption, CompleteTurnOptionBoundary,
    LocalTurnGraphWitnessConfig, LocalTurnGraphWitnessQuantum, LocalTurnGraphWitnessReport,
    LocalTurnGraphWitnessSession, LocalTurnGraphWitnessStatus, OracleCombatWitnessSatisfaction,
    TurnOptionGeneratorConfig, TurnOptionGeneratorSession,
};
use sts_simulator::eval::combat_action_imitation::typed_combat_feature_components_v1;
use sts_simulator::sim::combat::{
    CombatPosition, CombatStepLimits, CombatStepper, EngineCombatStepper,
};
use sts_simulator::state::core::ClientInput;

use super::{
    existing_combat_knowledge_policy_v1, existing_combat_rollout_lookahead_v1,
    load_exact_turn_corridor, source_content_fingerprint,
};

const MANIFEST_SCHEMA: &str = "BoundarySuccessorCorpusManifestV1";
const CORPUS_SCHEMA: &str = "BoundarySuccessorCorpusV1";
const FEATURE_SCHEMA: &str = "combat_action_imitation/typed_combat_feature_components_v1";

#[derive(Debug, Args)]
pub struct BoundarySuccessorCorpusArgs {
    /// Compact manifest naming exact cases, verified action lists, and the
    /// selected player-turn boundary in each witness.
    #[arg(long)]
    manifest: PathBuf,
    /// Destination for the typed evidence corpus.
    #[arg(long)]
    output: PathBuf,
    /// Deterministic work used to expose complete-turn successors.
    #[arg(long, default_value_t = 40_000)]
    generation_work: usize,
    /// Maximum number of successors evaluated per boundary. The verified
    /// witness successor is always retained when it was generated.
    #[arg(long, default_value_t = 12)]
    candidate_limit: usize,
    /// Deterministic candidate coverage. Rank-stratified keeps a policy head,
    /// logarithmically spaced deeper ranks, and the verified successor.
    #[arg(long, value_enum, default_value_t = CandidateSelection::RankStratified)]
    candidate_selection: CandidateSelection,
    /// Deterministic exact-search work for each non-terminal successor.
    #[arg(long, default_value_t = 5_000)]
    solve_work_per_candidate: usize,
    /// Maximum independent successor searches evaluated concurrently.
    #[arg(long, default_value_t = 4)]
    candidate_jobs: usize,
    /// Ignore a matching content-addressed output and recompute all evidence.
    #[arg(long)]
    force_rebuild: bool,
    #[arg(long, default_value_t = 250)]
    max_engine_steps_per_transition: usize,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorpusManifest {
    schema_name: String,
    schema_version: u32,
    entries: Vec<CorpusManifestEntry>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct CorpusManifestEntry {
    id: String,
    split: CorpusSplit,
    case: PathBuf,
    actions: Vec<PathBuf>,
    boundary_rank: usize,
    /// Optional deterministic exact-search work override for this boundary.
    /// This lets a held-out evaluation boundary receive more evidence work
    /// without over-spending on every training boundary.
    solve_work_per_candidate: Option<usize>,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
enum CorpusSplit {
    Train,
    Eval,
}

#[derive(Clone, Copy, Debug, Deserialize, Serialize, ValueEnum)]
#[serde(rename_all = "snake_case")]
enum CandidateSelection {
    PolicyHead,
    RankStratified,
}

#[derive(Clone, Debug, Serialize)]
struct BoundarySuccessorCorpus {
    schema_name: String,
    schema_version: u32,
    source_identity: String,
    input_fingerprint: String,
    feature_schema: String,
    manifest: PathBuf,
    config: CorpusConfig,
    groups: Vec<BoundarySuccessorGroup>,
}

#[derive(Clone, Copy, Debug, Serialize)]
struct CorpusConfig {
    generation_work: usize,
    candidate_limit: usize,
    candidate_selection: CandidateSelection,
    solve_work_per_candidate: usize,
    candidate_jobs: usize,
    max_engine_steps_per_transition: usize,
}

#[derive(Clone, Debug, Serialize)]
struct BoundarySuccessorGroup {
    id: String,
    split: CorpusSplit,
    source_case: PathBuf,
    source_actions: Vec<PathBuf>,
    boundary_rank: usize,
    player_turn: u32,
    root_exact_state_hash: String,
    root_features: Vec<i32>,
    verified_successor_exact_state_hash: String,
    verified_successor_generated: bool,
    verified_successor_policy_rank: Option<usize>,
    verified_suffix_action_count: usize,
    solve_work_per_candidate: usize,
    candidate_jobs: usize,
    generation: GenerationEvidence,
    candidates: Vec<BoundarySuccessorCandidate>,
}

#[derive(Clone, Debug, Serialize)]
struct GenerationEvidence {
    status: String,
    generation_work: usize,
    engine_steps: usize,
    completed_options: usize,
    retained_work_items: usize,
    gap_count: usize,
}

#[derive(Clone, Debug, Serialize)]
struct BoundarySuccessorCandidate {
    policy_rank: usize,
    exact_successor_hash: String,
    boundary: String,
    action_count: usize,
    actions: Vec<ClientInput>,
    negative_log_policy: f64,
    /// Exact Oracle state for offline representation research. Runtime search
    /// never reads this corpus.
    successor_position: CombatPosition,
    successor_features: Vec<i32>,
    evidence: SuccessorEvidence,
}

#[derive(Clone, Debug, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
enum SuccessorEvidence {
    ExactWin {
        source: &'static str,
        final_hp: i32,
        suffix_action_count: usize,
        search_cost: Option<SuccessorSearchCost>,
    },
    ExactRefutation {
        source: &'static str,
        search_cost: SuccessorSearchCost,
    },
    ExactTerminalNonWin {
        boundary: String,
    },
    BudgetUnknown {
        status: String,
        search_cost: SuccessorSearchCost,
        deepest_player_turn: u32,
        gap_count: usize,
        depth_limited_successors: usize,
    },
}

#[derive(Clone, Debug, Serialize)]
struct SuccessorSearchCost {
    generation_work: usize,
    lookahead_work: usize,
    applied_action_transitions: usize,
    engine_steps: usize,
    exact_nodes: usize,
    exact_edges: usize,
}

pub fn build(args: BoundarySuccessorCorpusArgs) -> Result<Value, String> {
    if args.candidate_limit == 0 {
        return Err("--candidate-limit must be positive".to_string());
    }
    if args.generation_work == 0 || args.solve_work_per_candidate == 0 {
        return Err("generation and successor solve work must be positive".to_string());
    }
    if args.candidate_jobs == 0 {
        return Err("--candidate-jobs must be positive".to_string());
    }
    let manifest_path = args
        .manifest
        .canonicalize()
        .map_err(|error| format!("cannot resolve manifest: {error}"))?;
    let manifest = serde_json::from_slice::<CorpusManifest>(
        &std::fs::read(&manifest_path).map_err(|error| error.to_string())?,
    )
    .map_err(|error| format!("invalid boundary-successor manifest: {error}"))?;
    if manifest.schema_name != MANIFEST_SCHEMA || manifest.schema_version != 1 {
        return Err("unsupported boundary-successor manifest schema".to_string());
    }
    if manifest.entries.is_empty() {
        return Err("boundary-successor manifest has no entries".to_string());
    }
    let base = manifest_path
        .parent()
        .ok_or_else(|| "manifest has no parent directory".to_string())?;
    let config = CorpusConfig {
        generation_work: args.generation_work,
        candidate_limit: args.candidate_limit,
        candidate_selection: args.candidate_selection,
        solve_work_per_candidate: args.solve_work_per_candidate,
        candidate_jobs: args.candidate_jobs,
        max_engine_steps_per_transition: args.max_engine_steps_per_transition,
    };
    let mut input_paths = vec![manifest_path.clone()];
    for entry in &manifest.entries {
        input_paths.push(resolve_relative(base, &entry.case));
        input_paths.extend(
            entry
                .actions
                .iter()
                .map(|path| resolve_relative(base, path)),
        );
    }
    let source_identity = current_source_identity()?;
    let input_fingerprint = source_content_fingerprint(base, &input_paths)?;
    if !args.force_rebuild && args.output.is_file() {
        let cached = std::fs::read(&args.output)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Value>(&bytes).ok());
        if let Some(cached) = cached {
            let config_value = serde_json::to_value(config).map_err(|error| error.to_string())?;
            if cached.get("schema_name").and_then(Value::as_str) == Some(CORPUS_SCHEMA)
                && cached.get("schema_version").and_then(Value::as_u64) == Some(1)
                && cached.get("source_identity").and_then(Value::as_str)
                    == Some(source_identity.as_str())
                && cached.get("input_fingerprint").and_then(Value::as_str)
                    == Some(input_fingerprint.as_str())
                && cached.get("config") == Some(&config_value)
            {
                return corpus_report(&cached, &args.output, true);
            }
        }
    }
    let mut groups = Vec::with_capacity(manifest.entries.len());
    let mut seen_ids = BTreeSet::new();
    for entry in manifest.entries {
        if !seen_ids.insert(entry.id.clone()) {
            return Err(format!(
                "duplicate boundary-successor manifest entry id: {}",
                entry.id
            ));
        }
        groups.push(build_group(entry, base, config)?);
    }
    let corpus = BoundarySuccessorCorpus {
        schema_name: CORPUS_SCHEMA.to_string(),
        schema_version: 1,
        source_identity,
        input_fingerprint,
        feature_schema: FEATURE_SCHEMA.to_string(),
        manifest: manifest_path,
        config,
        groups,
    };
    if let Some(parent) = args.output.parent() {
        std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
    }
    let corpus = serde_json::to_value(corpus).map_err(|error| error.to_string())?;
    std::fs::write(
        &args.output,
        serde_json::to_vec_pretty(&corpus).map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    corpus_report(&corpus, &args.output, false)
}

fn current_source_identity() -> Result<String, String> {
    let executable =
        std::env::current_exe().map_err(|error| format!("cannot locate oracle_lab: {error}"))?;
    let fingerprint_path = executable.with_extension("source-fingerprint");
    let fingerprint = std::fs::read_to_string(&fingerprint_path).map_err(|error| {
        format!(
            "cannot read canonical source identity '{}': {error}",
            fingerprint_path.display()
        )
    })?;
    fingerprint
        .lines()
        .nth(1)
        .filter(|digest| !digest.trim().is_empty())
        .map(str::to_owned)
        .ok_or_else(|| {
            format!(
                "canonical source identity '{}' is malformed",
                fingerprint_path.display()
            )
        })
}

fn corpus_report(corpus: &Value, output: &Path, cache_hit: bool) -> Result<Value, String> {
    let groups = corpus
        .get("groups")
        .and_then(Value::as_array)
        .ok_or_else(|| "boundary-successor corpus has no groups array".to_string())?;
    let train_groups = groups
        .iter()
        .filter(|group| group.get("split").and_then(Value::as_str) == Some("train"))
        .count();
    let mut candidates = 0usize;
    let mut evidence_counts = [0usize; 4];
    let mut verified_successors_generated = 0usize;
    for group in groups {
        if group
            .get("verified_successor_generated")
            .and_then(Value::as_bool)
            == Some(true)
        {
            verified_successors_generated = verified_successors_generated.saturating_add(1);
        }
        let group_candidates = group
            .get("candidates")
            .and_then(Value::as_array)
            .ok_or_else(|| "boundary-successor group has no candidates array".to_string())?;
        candidates = candidates.saturating_add(group_candidates.len());
        for candidate in group_candidates {
            match candidate
                .get("evidence")
                .and_then(|evidence| evidence.get("kind"))
                .and_then(Value::as_str)
            {
                Some("exact_win") => evidence_counts[0] += 1,
                Some("exact_refutation") => evidence_counts[1] += 1,
                Some("exact_terminal_non_win") => evidence_counts[2] += 1,
                Some("budget_unknown") => evidence_counts[3] += 1,
                Some(kind) => return Err(format!("unknown successor evidence kind: {kind}")),
                None => return Err("candidate has no successor evidence kind".to_string()),
            }
        }
    }
    Ok(json!({
        "schema_name": "BoundarySuccessorCorpusBuildReportV1",
        "schema_version": 1,
        "cache_hit": cache_hit,
        "output": output,
        "source_identity": corpus.get("source_identity"),
        "input_fingerprint": corpus.get("input_fingerprint"),
        "groups": groups.len(),
        "train_groups": train_groups,
        "eval_groups": groups.len().saturating_sub(train_groups),
        "candidates": candidates,
        "evidence": {
            "exact_win": evidence_counts[0],
            "exact_refutation": evidence_counts[1],
            "exact_terminal_non_win": evidence_counts[2],
            "budget_unknown": evidence_counts[3],
        },
        "verified_successors_generated": verified_successors_generated,
    }))
}

fn build_group(
    entry: CorpusManifestEntry,
    base: &Path,
    config: CorpusConfig,
) -> Result<BoundarySuccessorGroup, String> {
    if entry.id.trim().is_empty() {
        return Err("boundary-successor entry id is empty".to_string());
    }
    let solve_work_per_candidate = entry
        .solve_work_per_candidate
        .unwrap_or(config.solve_work_per_candidate);
    if solve_work_per_candidate == 0 {
        return Err(format!(
            "entry {} solve_work_per_candidate must be positive",
            entry.id
        ));
    }
    let case_path = resolve_relative(base, &entry.case);
    let action_paths = entry
        .actions
        .iter()
        .map(|path| resolve_relative(base, path))
        .collect::<Vec<_>>();
    if action_paths.is_empty() {
        return Err(format!("entry {} has no verified action list", entry.id));
    }
    let corridor = load_exact_turn_corridor(
        &case_path,
        &action_paths,
        config.max_engine_steps_per_transition,
    )?;
    let root_position = corridor
        .positions_by_rank
        .get(entry.boundary_rank)
        .cloned()
        .ok_or_else(|| {
            format!(
                "entry {} boundary rank {} exceeds {} exact turn roots",
                entry.id,
                entry.boundary_rank,
                corridor.positions_by_rank.len()
            )
        })?;
    let verified_turn_actions = corridor
        .transition_actions
        .get(entry.boundary_rank)
        .ok_or_else(|| format!("entry {} has no outgoing verified turn", entry.id))?;
    let verified_successor = replay_inputs(
        root_position.clone(),
        verified_turn_actions,
        config.max_engine_steps_per_transition,
    )?;
    let verified_successor_hash = sts_simulator::ai::combat_state_key::combat_exact_state_hash_v1(
        &verified_successor.engine,
        &verified_successor.combat,
    );
    let verified_suffix_action_count = corridor.transition_actions[entry.boundary_rank..]
        .iter()
        .map(Vec::len)
        .sum();

    let root = CombatDecisionRoot::new(root_position.clone())
        .map_err(|error| format!("entry {} has invalid root: {error:?}", entry.id))?;
    let mut generator = TurnOptionGeneratorSession::with_policy(
        root,
        TurnOptionGeneratorConfig {
            max_engine_steps_per_transition: config.max_engine_steps_per_transition,
            ..TurnOptionGeneratorConfig::default()
        },
        existing_combat_knowledge_policy_v1(),
    );
    let report = generator.advance(
        &EngineCombatStepper,
        CombatPlanningQuantum::deterministic(
            config.generation_work,
            config
                .generation_work
                .saturating_mul(config.max_engine_steps_per_transition),
        ),
    );
    let mut ranked = generator.completed_options().iter().collect::<Vec<_>>();
    ranked.sort_by(|left, right| {
        left.negative_log_policy()
            .total_cmp(&right.negative_log_policy())
            .then_with(|| {
                left.exact_successor_hash()
                    .cmp(right.exact_successor_hash())
            })
    });
    let verified_policy_rank = ranked
        .iter()
        .position(|option| option.exact_successor_hash() == verified_successor_hash)
        .map(|index| index + 1);
    let selected = selected_candidates(
        &ranked,
        &verified_successor_hash,
        config.candidate_limit,
        config.candidate_selection,
    )
    .into_iter()
    .map(|(policy_rank, option)| (policy_rank, option.clone()))
    .collect::<Vec<_>>();
    let candidates = build_candidates(
        &selected,
        &verified_successor_hash,
        corridor.terminal_final_hp,
        verified_suffix_action_count,
        config,
        solve_work_per_candidate,
    )?;

    Ok(BoundarySuccessorGroup {
        id: entry.id,
        split: entry.split,
        source_case: case_path,
        source_actions: action_paths,
        boundary_rank: entry.boundary_rank,
        player_turn: root_position.combat.turn.turn_count,
        root_exact_state_hash: sts_simulator::ai::combat_state_key::combat_exact_state_hash_v1(
            &root_position.engine,
            &root_position.combat,
        ),
        root_features: typed_combat_feature_components_v1(&root_position),
        verified_successor_exact_state_hash: verified_successor_hash,
        verified_successor_generated: verified_policy_rank.is_some(),
        verified_successor_policy_rank: verified_policy_rank,
        verified_suffix_action_count,
        solve_work_per_candidate,
        candidate_jobs: config.candidate_jobs.min(selected.len().max(1)),
        generation: GenerationEvidence {
            status: format!("{:?}", report.status),
            generation_work: report.after.generation_work,
            engine_steps: report.after.engine_steps,
            completed_options: ranked.len(),
            retained_work_items: report.retained_work_items,
            gap_count: report.gaps.len(),
        },
        candidates,
    })
}

fn build_candidates(
    selected: &[(usize, CompleteTurnOption)],
    verified_successor_hash: &str,
    verified_final_hp: i32,
    verified_suffix_action_count: usize,
    config: CorpusConfig,
    solve_work_per_candidate: usize,
) -> Result<Vec<BoundarySuccessorCandidate>, String> {
    if selected.is_empty() {
        return Ok(Vec::new());
    }
    let jobs = config.candidate_jobs.min(selected.len()).max(1);
    let chunk_len = selected.len().div_ceil(jobs);
    let batches = std::thread::scope(|scope| {
        let handles = selected
            .chunks(chunk_len)
            .map(|chunk| {
                scope.spawn(move || {
                    chunk
                        .iter()
                        .map(|(policy_rank, option)| {
                            build_candidate(
                                *policy_rank,
                                option,
                                verified_successor_hash,
                                verified_final_hp,
                                verified_suffix_action_count,
                                config,
                                solve_work_per_candidate,
                            )
                        })
                        .collect::<Vec<_>>()
                })
            })
            .collect::<Vec<_>>();
        handles
            .into_iter()
            .map(|handle| {
                handle
                    .join()
                    .map_err(|_| "boundary-successor candidate worker panicked".to_string())
            })
            .collect::<Result<Vec<_>, _>>()
    })?;
    batches.into_iter().flatten().collect()
}

fn build_candidate(
    policy_rank: usize,
    option: &CompleteTurnOption,
    verified_successor_hash: &str,
    verified_final_hp: i32,
    verified_suffix_action_count: usize,
    config: CorpusConfig,
    solve_work_per_candidate: usize,
) -> Result<BoundarySuccessorCandidate, String> {
    let evidence = if option.exact_successor_hash() == verified_successor_hash {
        SuccessorEvidence::ExactWin {
            source: "verified_witness_suffix",
            final_hp: verified_final_hp,
            suffix_action_count: verified_suffix_action_count,
            search_cost: None,
        }
    } else {
        evaluate_successor(option, config, solve_work_per_candidate)?
    };
    Ok(BoundarySuccessorCandidate {
        policy_rank,
        exact_successor_hash: option.exact_successor_hash().to_string(),
        boundary: format!("{:?}", option.boundary()),
        action_count: option.actions().len(),
        actions: option
            .actions()
            .iter()
            .map(|action| action.input.clone())
            .collect(),
        negative_log_policy: option.negative_log_policy(),
        successor_position: option.exact_successor().clone(),
        successor_features: typed_combat_feature_components_v1(option.exact_successor()),
        evidence,
    })
}

fn selected_candidates<'a>(
    ranked: &[&'a CompleteTurnOption],
    verified_successor_hash: &str,
    limit: usize,
    selection: CandidateSelection,
) -> Vec<(usize, &'a CompleteTurnOption)> {
    let verified_index = ranked
        .iter()
        .enumerate()
        .find(|(_, option)| option.exact_successor_hash() == verified_successor_hash)
        .map(|(index, _)| index);
    selected_candidate_indices(ranked.len(), verified_index, limit, selection)
        .into_iter()
        .map(|index| (index + 1, ranked[index]))
        .collect()
}

fn selected_candidate_indices(
    ranked_len: usize,
    verified_index: Option<usize>,
    limit: usize,
    selection: CandidateSelection,
) -> Vec<usize> {
    if ranked_len == 0 || limit == 0 {
        return Vec::new();
    }
    let target_len = limit.min(ranked_len);
    if matches!(selection, CandidateSelection::PolicyHead) {
        let mut selected = (0..target_len).collect::<BTreeSet<_>>();
        if let Some(verified_index) = verified_index.filter(|index| *index < ranked_len) {
            if !selected.contains(&verified_index) {
                selected.pop_last();
                selected.insert(verified_index);
            }
        }
        return selected.into_iter().collect();
    }

    let reserve_verified = usize::from(verified_index.is_some());
    let coverage_slots = target_len.saturating_sub(reserve_verified);
    let head_slots = coverage_slots.div_ceil(2);
    let tail_slots = coverage_slots.saturating_sub(head_slots);
    let mut selected = (0..head_slots.min(ranked_len)).collect::<BTreeSet<_>>();
    if tail_slots > 0 && ranked_len > head_slots {
        let first_rank = head_slots.saturating_add(1).max(1) as f64;
        let last_rank = ranked_len as f64;
        for slot in 1..=tail_slots {
            let fraction = slot as f64 / tail_slots as f64;
            let rank = (first_rank.ln() + (last_rank.ln() - first_rank.ln()) * fraction)
                .exp()
                .round()
                .clamp(1.0, last_rank) as usize;
            selected.insert(rank.saturating_sub(1));
        }
    }
    if let Some(verified_index) = verified_index.filter(|index| *index < ranked_len) {
        selected.insert(verified_index);
    }
    for index in 0..ranked_len {
        if selected.len() >= target_len {
            break;
        }
        selected.insert(index);
    }
    while selected.len() > target_len {
        let removable = selected
            .iter()
            .rev()
            .copied()
            .find(|index| Some(*index) != verified_index);
        if let Some(index) = removable {
            selected.remove(&index);
        } else {
            break;
        }
    }
    selected.into_iter().collect()
}

fn evaluate_successor(
    option: &CompleteTurnOption,
    config: CorpusConfig,
    solve_work_per_candidate: usize,
) -> Result<SuccessorEvidence, String> {
    match option.boundary() {
        CompleteTurnOptionBoundary::TerminalWin => {
            return Ok(SuccessorEvidence::ExactWin {
                source: "immediate_terminal_replay",
                final_hp: option.exact_successor().combat.entities.player.current_hp,
                suffix_action_count: 0,
                search_cost: None,
            });
        }
        CompleteTurnOptionBoundary::TerminalLoss | CompleteTurnOptionBoundary::Escape => {
            return Ok(SuccessorEvidence::ExactTerminalNonWin {
                boundary: format!("{:?}", option.boundary()),
            });
        }
        CompleteTurnOptionBoundary::NextPlayerTurn => {}
    }
    let root = CombatDecisionRoot::new(option.exact_successor().clone())
        .map_err(|error| format!("invalid successor root: {error:?}"))?;
    let search_config = LocalTurnGraphWitnessConfig {
        generator: TurnOptionGeneratorConfig {
            max_engine_steps_per_transition: config.max_engine_steps_per_transition,
            ..TurnOptionGeneratorConfig::default()
        },
        generation_quantum_work: 4,
        backed_generation_quantum_work: 256,
        initial_expansion_work: 64,
        root_initial_expansion_work: 2_048,
        lookahead_max_evaluations: solve_work_per_candidate.saturating_div(24).max(1),
        lookahead_work_per_evaluation: 24,
        max_turn_depth: 32,
        satisfaction: OracleCombatWitnessSatisfaction::FirstWitness,
    };
    let mut session = LocalTurnGraphWitnessSession::with_policy_and_lookahead(
        root,
        search_config,
        existing_combat_knowledge_policy_v1(),
        existing_combat_rollout_lookahead_v1(),
    );
    let report = session.advance(
        LocalTurnGraphWitnessQuantum {
            additional_selections: solve_work_per_candidate.saturating_mul(8),
            additional_generation_work: solve_work_per_candidate,
            additional_engine_steps: solve_work_per_candidate
                .saturating_mul(config.max_engine_steps_per_transition),
            deadline: None,
        },
        &EngineCombatStepper,
    );
    if let Some(witness) = report.witness.as_ref() {
        return Ok(SuccessorEvidence::ExactWin {
            source: "bounded_exact_search",
            final_hp: witness.final_position.combat.entities.player.current_hp,
            suffix_action_count: witness.actions.len(),
            search_cost: Some(search_cost(&report)),
        });
    }
    if matches!(
        report.status,
        LocalTurnGraphWitnessStatus::FrontierExhausted
    ) && report.generation_gaps.is_empty()
        && report.counters.depth_limited_successors == 0
    {
        return Ok(SuccessorEvidence::ExactRefutation {
            source: "gap_free_frontier_exhaustion",
            search_cost: search_cost(&report),
        });
    }
    let progress = session.progress_snapshot();
    Ok(SuccessorEvidence::BudgetUnknown {
        status: format!("{:?}", report.status),
        search_cost: search_cost(&report),
        deepest_player_turn: progress.max_player_turn,
        gap_count: report.generation_gaps.len(),
        depth_limited_successors: report.counters.depth_limited_successors,
    })
}

fn search_cost(report: &LocalTurnGraphWitnessReport) -> SuccessorSearchCost {
    SuccessorSearchCost {
        generation_work: report.counters.generation_work,
        lookahead_work: report.counters.lookahead_work,
        applied_action_transitions: report.counters.applied_action_transitions,
        engine_steps: report.counters.engine_steps,
        exact_nodes: report.counters.exact_nodes,
        exact_edges: report.counters.exact_edges,
    }
}

fn replay_inputs(
    mut position: CombatPosition,
    inputs: &[ClientInput],
    max_engine_steps_per_transition: usize,
) -> Result<CombatPosition, String> {
    let stepper = EngineCombatStepper;
    for (index, input) in inputs.iter().enumerate() {
        if stepper.choice_for_legal_input(&position, input).is_none() {
            return Err(format!(
                "verified turn action {index} is not legal at its exact state"
            ));
        }
        let step = stepper.apply_to_stable(
            &position,
            input.clone(),
            CombatStepLimits {
                max_engine_steps: max_engine_steps_per_transition,
                deadline: None,
            },
        );
        if step.truncated || step.timed_out {
            return Err(format!(
                "verified turn action {index} did not reach a stable successor"
            ));
        }
        position = step.position;
    }
    Ok(position)
}

fn resolve_relative(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::{selected_candidate_indices, CandidateSelection};

    #[test]
    fn policy_head_replaces_last_slot_with_deep_verified_successor() {
        assert_eq!(
            selected_candidate_indices(100, Some(42), 4, CandidateSelection::PolicyHead),
            vec![0, 1, 2, 42]
        );
    }

    #[test]
    fn rank_stratified_covers_head_tail_and_verified_successor() {
        let selected =
            selected_candidate_indices(12_000, Some(432), 8, CandidateSelection::RankStratified);
        assert_eq!(selected.len(), 8);
        assert!(selected.starts_with(&[0, 1, 2, 3]));
        assert!(selected.contains(&432));
        assert!(selected.contains(&11_999));
    }
}
