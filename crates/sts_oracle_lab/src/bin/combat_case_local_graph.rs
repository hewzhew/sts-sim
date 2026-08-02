use std::path::PathBuf;
use std::time::Instant;

use clap::Args;
use serde_json::Value;
use sts_combat_planner::{CombatDecisionRoot, OracleCombatWitnessSatisfaction};
use sts_oracle_runtime::eval::combat_case::load_combat_case;
use sts_oracle_runtime::eval::combat_guidance_bundle::{
    combat_value_prototype_policy_v1, CombatGuidanceBundleV1,
};
use sts_oracle_runtime::eval::run_control::existing_combat_knowledge_policy_v1;
use sts_oracle_runtime::sim::combat::EngineCombatStepper;

use super::combat_case_contract::{evaluate_local_graph_contract, LocalGraphContractRequest};
use super::combat_case_performance;
use super::combat_graph_diagnostics::{
    materialize_local_graph_diagnostics, LocalGraphDiagnosticPaths,
};
use super::combat_graph_execution::LocalGraphExecutionProfile;
use super::combat_graph_exports::{
    export_local_graph_paths, LocalGraphExportActions, LocalGraphExportPaths,
};
use super::combat_graph_observation::capture_local_graph_observation;
use super::combat_graph_report::{
    local_graph_full_report, local_graph_trace_report, LocalGraphCounterfactual,
    LocalGraphFullReportOptions, LocalGraphReportData, LocalGraphRunIdentity,
};
use super::combat_graph_search_spec::LocalGraphSearchSpec;
use super::combat_planning_view::combat_plan_transition_portfolio_v1;
use super::combat_policy_controls::load_action_imitation_policy;
use super::exact_turn_corridor::load as load_exact_turn_corridor;
use super::guidance_artifact_commands::load_value_prototype;
use super::print_json;

#[derive(Debug, Args)]
pub(super) struct CombatCaseLocalGraphArgs {
    #[arg(long)]
    case: PathBuf,
    /// Diagnostic control: preserve action-policy weights while removing
    /// every boundary and mid-turn state guide.
    #[arg(long, conflicts_with = "root_turn_anchor_only")]
    anchor_only: bool,
    /// Diagnostic control: use only action-policy anchor service during
    /// the root player turn, then restore all guides at later turns.
    #[arg(long, conflicts_with = "anchor_only")]
    root_turn_anchor_only: bool,
    /// Opt-in capability migration: lazily evaluate selected exact states
    /// with bounded rollout evidence. Rollout actions are never injected.
    #[arg(
        long,
        conflicts_with = "anchor_only",
        conflicts_with = "root_turn_anchor_only"
    )]
    rollout_lookahead: bool,
    /// Optional typed action-order policy distilled from exact witnesses.
    /// It changes guidance only; legality and terminal truth stay exact.
    #[arg(long)]
    action_imitation_artifact: Option<PathBuf>,
    /// Optional lab-only turn-boundary value prototypes distilled from an
    /// exact witness. This is a teacher upper-bound control, not production.
    #[arg(long)]
    value_prototype_artifact: Option<PathBuf>,
    /// One immutable, compatibility-checked package containing both the
    /// typed action residual and cross-turn value prototypes.
    #[arg(
        long,
        conflicts_with = "action_imitation_artifact",
        conflicts_with = "value_prototype_artifact"
    )]
    guidance_bundle: Option<PathBuf>,
    /// Replay one verified witness and observe each exact player-turn
    /// boundary without changing policy, guides, or search order.
    #[arg(long)]
    watch_corridor_actions: Vec<PathBuf>,
    /// Attach encounter-owned, typed plan facts to newly materialized
    /// exact turn-boundary edges. Diagnostic only: annotations are not
    /// read by policy, scheduling, pruning, or witness authority.
    #[arg(long)]
    plan_transition_annotations: bool,
    /// Opt-in lab control: add the encounter-owned typed combat-plan
    /// state view as one independent guide lane. Action weights,
    /// legality, exact-state identity and terminal truth remain unchanged.
    #[arg(long, conflicts_with = "anchor_only")]
    typed_plan_guide: bool,
    /// Opt-in lab control: order concrete members of structured selections
    /// using encounter-owned plan timing. This does not add a state guide.
    #[arg(long)]
    typed_plan_selection_timing: bool,
    /// Lab-only control: materialize one exact base-policy mainline at
    /// player-turn boundaries. A typed encounter plan may defer a
    /// prematurely resource-consuming action or prefer a precisely timed
    /// action; all rejected alternatives remain searchable.
    #[arg(long)]
    plan_compatible_policy_line: bool,
    /// Deterministic exact-search work granted immediately before the
    /// plan-compatible line would cross a typed combat-plan milestone.
    /// Zero disables suffix probes.
    #[arg(long, default_value_t = 0, requires = "plan_compatible_policy_line")]
    plan_compatible_suffix_work: usize,
    /// Contract assertion: return a non-zero exit status unless an exact,
    /// replay-verified combat witness is found.
    #[arg(long)]
    expect_witness: bool,
    /// Contract assertion: require the verified witness to finish with at
    /// least this much HP.
    #[arg(long, requires = "expect_witness")]
    expect_min_final_hp: Option<i32>,
    /// Contract assertion: fail if all plan-compatible suffix probes
    /// together consume more exact generation work than this allowance.
    #[arg(long, requires = "plan_compatible_policy_line")]
    expect_max_plan_suffix_work: Option<usize>,
    /// Print only the compact contract result after all requested
    /// assertions pass. This keeps repeat regression checks readable.
    #[arg(long, requires = "expect_witness")]
    contract_only: bool,
    /// Print only a hierarchical performance profile. Parent timings remain
    /// separate from nested generator and transition timings, and rates are
    /// normalized by exact work rather than inferred from wall time alone.
    #[arg(
        long,
        conflicts_with = "contract_only",
        conflicts_with = "readable",
        conflicts_with = "trace"
    )]
    performance_only: bool,
    #[arg(long, default_value_t = 250_000)]
    max_nodes: usize,
    #[arg(long, default_value_t = 1_000_000)]
    max_selections: usize,
    #[arg(long, default_value_t = 5_000)]
    wall_ms: u64,
    /// Diagnostic-only quality mode: retain the first verified witness
    /// and keep searching until the explicit work/deadline allowance.
    #[arg(long)]
    improve_incumbent: bool,
    /// Stop at the first replay-verified witness whose HP loss is at most
    /// this non-negative bound. This exposes the planner's existing
    /// satisfaction contract without collapsing every combat to either
    /// first-win or best-HP search.
    #[arg(long, conflicts_with = "improve_incumbent")]
    max_hp_loss: Option<u32>,
    /// Maximum potion resources the exact search may expend. The laboratory
    /// starts potion-free; pass a positive value to open an explicit potion
    /// lane. Every finite limit is enforced during generation, not only when
    /// a terminal witness is accepted.
    #[arg(long, default_value = "0")]
    max_potions_used: Option<u32>,
    /// All-legal diagnostic control: admit explicit potion discard actions.
    /// Semantic victory search omits them by default because discarding is not
    /// a generic way to diversify a sparse search.
    #[arg(long)]
    include_discard_actions: bool,
    #[arg(long, default_value_t = 250)]
    max_engine_steps_per_transition: usize,
    /// Uniform exploration mixed into action-policy weights, in parts per
    /// million. This is part of search identity and must be explicit when
    /// comparing this graph with another exact-search host.
    #[arg(long, default_value_t = 50_000)]
    uniform_exploration_ppm: u32,
    #[arg(long, default_value_t = 4)]
    generation_quantum_work: usize,
    #[arg(long, default_value_t = 32)]
    max_turn_depth: usize,
    /// Diagnostic counterfactual: keep the exact combat state, RNG,
    /// deck, relics and potions, but restore current HP to max HP before
    /// search. This classifies arrival debt; it is never a legal witness
    /// for the original run.
    #[arg(long)]
    full_health: bool,
    /// Include readable, exact replay traces for the deepest survival,
    /// deepest progress, and terminal witness paths.
    #[arg(long)]
    readable: bool,
    /// Print only compact per-turn traces for the deepest states and
    /// witness. Omits raw action hashes and full frontier diagnostics.
    #[arg(long, conflicts_with = "readable")]
    trace: bool,
    /// Report exact graph membership and local service for selected states.
    #[arg(long)]
    watch_exact_state_hash: Vec<String>,
    /// If a replay-verified win is found, save its exact ClientInput list.
    #[arg(long)]
    export_witness_actions: Option<PathBuf>,
    /// Save the exact deepest-survival state as a standalone diagnostic
    /// combat case. Inspect `deepest.survival_node.exhausted` before using
    /// it as a segmented-search continuation.
    #[arg(
        long,
        visible_alias = "export-deepest-case",
        conflicts_with = "export_deepest_progress_case"
    )]
    export_deepest_survival_case: Option<PathBuf>,
    /// Save the exact deepest-progress state as a new standalone combat
    /// case instead of the survival envelope.
    #[arg(long, conflicts_with = "export_deepest_survival_case")]
    export_deepest_progress_case: Option<PathBuf>,
}

pub(super) fn run(args: CombatCaseLocalGraphArgs) -> Result<(), String> {
    let CombatCaseLocalGraphArgs {
        case,
        anchor_only,
        root_turn_anchor_only,
        rollout_lookahead,
        action_imitation_artifact,
        value_prototype_artifact,
        guidance_bundle,
        watch_corridor_actions,
        plan_transition_annotations,
        typed_plan_guide,
        typed_plan_selection_timing,
        plan_compatible_policy_line,
        plan_compatible_suffix_work,
        expect_witness,
        expect_min_final_hp,
        expect_max_plan_suffix_work,
        contract_only,
        performance_only,
        max_nodes,
        max_selections,
        wall_ms,
        improve_incumbent,
        max_hp_loss,
        max_potions_used,
        include_discard_actions,
        max_engine_steps_per_transition,
        uniform_exploration_ppm,
        generation_quantum_work,
        max_turn_depth,
        full_health,
        readable,
        trace,
        watch_exact_state_hash,
        export_witness_actions,
        export_deepest_survival_case,
        export_deepest_progress_case,
    } = args;
    let command_started = Instant::now();
    let mut loaded = load_combat_case(&case)?;
    let original_hp = loaded.position.combat.entities.player.current_hp;
    if full_health {
        loaded.position.combat.entities.player.current_hp =
            loaded.position.combat.entities.player.max_hp;
        loaded.refresh_derived_summaries_and_clear_production_context();
    }
    let initial_hp = loaded.position.combat.entities.player.current_hp;
    let root_player_turn = loaded.position.combat.turn.turn_count;
    let execution_profile = LocalGraphExecutionProfile::from_controls(
        anchor_only,
        root_turn_anchor_only,
        rollout_lookahead,
        typed_plan_guide,
        typed_plan_selection_timing,
    )?;
    let search_spec = LocalGraphSearchSpec::from_controls(
        max_nodes,
        max_selections,
        wall_ms,
        max_engine_steps_per_transition,
        uniform_exploration_ppm,
        generation_quantum_work,
        max_turn_depth,
        max_potions_used,
        include_discard_actions,
    );
    let search_root_position = loaded.position.clone();
    let watched_corridor = if watch_corridor_actions.is_empty() {
        None
    } else {
        Some(load_exact_turn_corridor(
            &case,
            &watch_corridor_actions,
            max_engine_steps_per_transition,
        )?)
    };
    let root = CombatDecisionRoot::new(loaded.position.clone())
        .map_err(|error| format!("invalid combat case root: {error:?}"))?;
    let satisfaction = if improve_incumbent {
        OracleCombatWitnessSatisfaction::BudgetOrExhaustion
    } else if let Some(limit) = max_hp_loss {
        OracleCombatWitnessSatisfaction::HpLossAtMost(limit)
    } else {
        OracleCombatWitnessSatisfaction::FirstWitness
    };
    let config = search_spec.planner_config(satisfaction);
    let policy = if let Some(path) = guidance_bundle.as_deref() {
        CombatGuidanceBundleV1::load(path)?.policy(existing_combat_knowledge_policy_v1())?
    } else {
        let policy = action_imitation_artifact
            .as_deref()
            .map(|path| load_action_imitation_policy(path, existing_combat_knowledge_policy_v1()))
            .transpose()?
            .unwrap_or_else(existing_combat_knowledge_policy_v1);
        if let Some(path) = value_prototype_artifact.as_deref() {
            let artifact = load_value_prototype(path)?;
            combat_value_prototype_policy_v1(policy, &artifact)
        } else {
            policy
        }
    };
    let mut session = execution_profile.prepare_session(root, root_player_turn, config, policy);
    if plan_transition_annotations {
        session
            .enable_plan_transition_annotations()
            .map_err(|error| {
                format!(
                    "cannot enable plan transition annotations after graph construction: \
                             {error:?}"
                )
            })?;
    }
    let policy_line_report = plan_compatible_policy_line
        .then(|| {
            session.offer_plan_compatible_policy_line_with_suffix_probes(
                max_turn_depth,
                256,
                plan_compatible_suffix_work,
                &EngineCombatStepper,
            )
        })
        .transpose()?;
    let search_started = Instant::now();
    let report = session.advance(search_spec.quantum(), &EngineCombatStepper);
    let search_elapsed = search_started.elapsed();
    let progress = session.progress_snapshot();
    let witness_inputs = report.witness.as_ref().map(|witness| {
        witness
            .actions
            .iter()
            .map(|action| action.input.clone())
            .collect::<Vec<_>>()
    });
    // Exports are explicit side effects of the command, not presentation.
    // Complete them before compact-contract or performance-only reporting can
    // return early.
    let exports = export_local_graph_paths(
        &loaded,
        (!full_health).then_some(case.as_path()),
        LocalGraphExportPaths {
            witness_actions: export_witness_actions.as_deref(),
            deepest_survival_case: export_deepest_survival_case.as_deref(),
            deepest_progress_case: export_deepest_progress_case.as_deref(),
        },
        LocalGraphExportActions {
            witness: witness_inputs.as_deref(),
            witness_final_position: report
                .witness
                .as_ref()
                .map(|witness| &witness.final_position),
            deepest_survival: &progress.deepest_survival_actions,
            deepest_progress: &progress.deepest_progress_actions,
        },
        max_engine_steps_per_transition,
    )?;
    if let Some(contract_result) = evaluate_local_graph_contract(LocalGraphContractRequest {
        case: &case,
        elapsed: command_started.elapsed(),
        report: &report,
        policy_line: policy_line_report.as_ref(),
        expect_witness,
        expect_min_final_hp,
        expect_max_plan_suffix_work,
        contract_only,
    })? {
        return print_json(&contract_result);
    }
    let performance_profile =
        combat_case_performance::local_graph_performance_report(search_elapsed, &case, &report);
    if performance_only {
        return print_json(&performance_profile);
    }
    let performance_timing = combat_case_performance::local_graph_performance_timing(&report);
    let include_trace = readable || trace;
    let diagnostics = materialize_local_graph_diagnostics(
        &session,
        &search_root_position,
        LocalGraphDiagnosticPaths {
            deepest_survival: &progress.deepest_survival_actions,
            deepest_progress: &progress.deepest_progress_actions,
            witness: report
                .witness
                .as_ref()
                .map(|witness| witness.actions.as_slice()),
        },
        include_trace,
        max_engine_steps_per_transition,
    )?;
    let observation = capture_local_graph_observation(
        &session,
        &search_root_position,
        &watch_exact_state_hash,
        watched_corridor.as_ref(),
    );
    let plan_transition_portfolio = plan_transition_annotations
        .then(|| combat_plan_transition_portfolio_v1(&session))
        .unwrap_or(Value::Null);
    let report_data = LocalGraphReportData {
        run: LocalGraphRunIdentity {
            case: &case,
            elapsed: command_started.elapsed(),
            satisfaction,
            execution_profile,
            search_spec,
            counterfactual: LocalGraphCounterfactual {
                full_health,
                original_hp,
                search_hp: initial_hp,
            },
        },
        report: &report,
        progress: &progress,
        retained_state_work: session.retained_state_work(),
        storage: session.storage_snapshot(),
        policy_line: policy_line_report.as_ref(),
        plan_transition_annotations,
        plan_transition_portfolio: &plan_transition_portfolio,
        diagnostics: &diagnostics,
        observation: &observation,
        exports: &exports,
    };
    if trace {
        return print_json(&local_graph_trace_report(&report_data));
    }
    let output = local_graph_full_report(
        &report_data,
        LocalGraphFullReportOptions {
            action_imitation_artifact: action_imitation_artifact.as_deref(),
            value_prototype_artifact: value_prototype_artifact.as_deref(),
            guidance_bundle: guidance_bundle.as_deref(),
            watch_corridor_actions: &watch_corridor_actions,
            readable,
            search_elapsed,
            performance_timing: &performance_timing,
            performance_profile: &performance_profile,
        },
    );
    print_json(&output)
}
