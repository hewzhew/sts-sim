//! Lightweight exact combat regression runner.
//!
//! This package deliberately excludes run exploration, shops, routes,
//! continuations, and resident-workspace state. It is the fast compilation
//! boundary for replay-verified tactical contracts.

use std::cell::Cell;
use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::Parser;
use serde::Deserialize;
use serde_json::json;
use sts_combat_knowledge::existing_combat_knowledge_policy_v1;
use sts_combat_planner::{
    combat_plan_state_guide_policy_v1, CombatDecisionRoot, LocalTurnGraphWitnessConfig,
    LocalTurnGraphWitnessQuantum, LocalTurnGraphWitnessSession, OracleCombatWitnessSatisfaction,
    TurnOptionGeneratorConfig, DETAIL_TIMING_SAMPLE_INTERVAL,
};
use sts_core::ai::combat_state_key::{
    combat_exact_state_hash_v1, combat_exact_state_key_profiled_v1, CombatExactStateKey,
};
use sts_core::sim::combat::{
    apply_combat_input_to_stable_profiled_v1, CombatPosition, CombatStepLimits, CombatStepResult,
    CombatStepper, CombatTerminal, EngineCombatStepper,
};
use sts_core::state::core::{ClientInput, EngineState};

#[derive(Debug, Parser)]
#[command(
    name = "combat_contract",
    about = "Run one replay-verified combat contract without compiling the full oracle runtime"
)]
struct Cli {
    #[arg(long)]
    case: PathBuf,
    /// Replay an exact action file and report search membership/work for each
    /// resulting player-turn boundary.
    #[arg(long)]
    watch_actions: Option<PathBuf>,
    /// Replay `--watch-actions` to this player turn and use that exact state
    /// as the search root. The remaining actions stay available as the watched
    /// suffix corridor.
    #[arg(long, requires = "watch_actions")]
    start_at_player_turn: Option<u32>,
    #[arg(long)]
    typed_plan_guide: bool,
    #[arg(long)]
    plan_compatible_policy_line: bool,
    #[arg(long, default_value_t = 0, requires = "plan_compatible_policy_line")]
    plan_compatible_suffix_work: usize,
    #[arg(long)]
    expect_witness: bool,
    #[arg(long, requires = "expect_witness")]
    expect_min_final_hp: Option<i32>,
    #[arg(long, requires = "plan_compatible_policy_line")]
    expect_max_plan_suffix_work: Option<usize>,
    #[arg(long, default_value_t = 250_000)]
    max_nodes: usize,
    #[arg(long, default_value_t = 1_000_000)]
    max_selections: usize,
    #[arg(long, default_value_t = 5_000)]
    wall_ms: u64,
    /// Continue after the first verified witness until the explicit budget is
    /// exhausted, retaining the best exact witness found.
    #[arg(long)]
    improve_incumbent: bool,
    /// Stop once a verified witness loses no more than this much HP.
    #[arg(long, conflicts_with = "improve_incumbent")]
    max_hp_loss: Option<u32>,
    /// Permit at most this many potion uses or discards in the exact witness.
    #[arg(long)]
    max_potions_used: Option<u32>,
    /// Emit only the compact performance and witness payload.
    #[arg(long)]
    performance_only: bool,
    /// Sparsely profile EngineState clone, CombatState clone, and execution.
    /// This is diagnostic-only and deliberately adds no clocks to the normal
    /// production stepper.
    #[arg(long)]
    profile_transition_clone_cost: bool,
    #[arg(long, default_value_t = 250)]
    max_engine_steps_per_transition: usize,
    #[arg(long, default_value_t = 4)]
    generation_quantum_work: usize,
    #[arg(long, default_value_t = 32)]
    max_turn_depth: usize,
}

#[derive(Deserialize)]
struct CombatCaseRoot {
    schema: String,
    position: CombatPosition,
}

const TRANSITION_CLONE_PROFILE_INTERVAL: usize = 16;

#[derive(Clone, Copy, Debug, Default)]
struct TransitionCloneProfile {
    calls: usize,
    samples: usize,
    engine_clone_elapsed_ns: u64,
    combat_clone_elapsed_ns: u64,
    combat_meta_clone_elapsed_ns: u64,
    combat_turn_clone_elapsed_ns: u64,
    combat_zones_clone_elapsed_ns: u64,
    combat_entities_clone_elapsed_ns: u64,
    combat_zone_component_elapsed_ns: [u64; 6],
    combat_entity_component_elapsed_ns: [u64; 4],
    combat_runtime_component_elapsed_ns: [u64; 6],
    exact_key_build_component_elapsed_ns: [u64; 11],
    combat_engine_clone_elapsed_ns: u64,
    combat_rng_clone_elapsed_ns: u64,
    combat_runtime_clone_elapsed_ns: u64,
    execution_elapsed_ns: u64,
    emitted_event_items: usize,
    max_emitted_event_items: usize,
    engine_diagnostic_items: usize,
    max_engine_diagnostic_items: usize,
    monster_protocol_items: usize,
    max_monster_protocol_items: usize,
}

#[derive(Debug, Default)]
struct ProfiledEngineCombatStepper {
    profile: Cell<TransitionCloneProfile>,
}

impl ProfiledEngineCombatStepper {
    fn snapshot(&self) -> TransitionCloneProfile {
        self.profile.get()
    }
}

impl CombatStepper for ProfiledEngineCombatStepper {
    fn atomic_actions(&self, position: &CombatPosition) -> Vec<ClientInput> {
        EngineCombatStepper.atomic_actions(position)
    }

    fn legal_action_surface(
        &self,
        position: &CombatPosition,
    ) -> sts_core::sim::combat_action_surface::CombatLegalActionSurfaceV2 {
        EngineCombatStepper.legal_action_surface(position)
    }

    fn supports_canonical_pending_choice_actions(&self) -> bool {
        EngineCombatStepper.supports_canonical_pending_choice_actions()
    }

    fn is_legal_action(&self, position: &CombatPosition, input: &ClientInput) -> bool {
        EngineCombatStepper.is_legal_action(position, input)
    }

    fn apply_to_stable(
        &self,
        position: &CombatPosition,
        input: ClientInput,
        limits: CombatStepLimits,
    ) -> CombatStepResult {
        let mut profile = self.profile.get();
        profile.calls = profile.calls.saturating_add(1);
        if profile.calls % TRANSITION_CLONE_PROFILE_INTERVAL != 0 {
            self.profile.set(profile);
            return EngineCombatStepper.apply_to_stable(position, input, limits);
        }

        let (step, timing) = apply_combat_input_to_stable_profiled_v1(position, input, limits);
        let (_, key_timing) =
            combat_exact_state_key_profiled_v1(&step.position.engine, &step.position.combat);
        let key_samples = [
            key_timing.engine_elapsed_ns,
            key_timing.turn_elapsed_ns,
            key_timing.meta_elapsed_ns,
            key_timing.zones_elapsed_ns,
            key_timing.monsters_elapsed_ns,
            key_timing.powers_elapsed_ns,
            key_timing.potions_elapsed_ns,
            key_timing.queue_elapsed_ns,
            key_timing.runtime_elapsed_ns,
            key_timing.rng_elapsed_ns,
            key_timing.player_elapsed_ns,
        ];
        for (total, sample) in profile
            .exact_key_build_component_elapsed_ns
            .iter_mut()
            .zip(key_samples)
        {
            *total = total.saturating_add(sample);
        }
        profile.samples = profile.samples.saturating_add(1);
        let emitted_event_items = position.combat.runtime.emitted_events.len();
        profile.emitted_event_items = profile
            .emitted_event_items
            .saturating_add(emitted_event_items);
        profile.max_emitted_event_items = profile.max_emitted_event_items.max(emitted_event_items);
        let engine_diagnostic_items = position.combat.runtime.engine_diagnostics.len();
        profile.engine_diagnostic_items = profile
            .engine_diagnostic_items
            .saturating_add(engine_diagnostic_items);
        profile.max_engine_diagnostic_items = profile
            .max_engine_diagnostic_items
            .max(engine_diagnostic_items);
        let monster_protocol_items = position.combat.runtime.monster_protocol.len();
        profile.monster_protocol_items = profile
            .monster_protocol_items
            .saturating_add(monster_protocol_items);
        profile.max_monster_protocol_items = profile
            .max_monster_protocol_items
            .max(monster_protocol_items);
        profile.engine_clone_elapsed_ns = profile
            .engine_clone_elapsed_ns
            .saturating_add(timing.engine_clone_elapsed_ns);
        profile.combat_clone_elapsed_ns = profile
            .combat_clone_elapsed_ns
            .saturating_add(timing.combat_clone_elapsed_ns);
        profile.combat_meta_clone_elapsed_ns = profile
            .combat_meta_clone_elapsed_ns
            .saturating_add(timing.combat_meta_clone_elapsed_ns);
        profile.combat_turn_clone_elapsed_ns = profile
            .combat_turn_clone_elapsed_ns
            .saturating_add(timing.combat_turn_clone_elapsed_ns);
        profile.combat_zones_clone_elapsed_ns = profile
            .combat_zones_clone_elapsed_ns
            .saturating_add(timing.combat_zones_clone_elapsed_ns);
        profile.combat_entities_clone_elapsed_ns = profile
            .combat_entities_clone_elapsed_ns
            .saturating_add(timing.combat_entities_clone_elapsed_ns);
        for (total, sample) in profile
            .combat_zone_component_elapsed_ns
            .iter_mut()
            .zip(timing.combat_zone_component_elapsed_ns)
        {
            *total = total.saturating_add(sample);
        }
        for (total, sample) in profile
            .combat_runtime_component_elapsed_ns
            .iter_mut()
            .zip(timing.combat_runtime_component_elapsed_ns)
        {
            *total = total.saturating_add(sample);
        }
        for (total, sample) in profile
            .combat_entity_component_elapsed_ns
            .iter_mut()
            .zip(timing.combat_entity_component_elapsed_ns)
        {
            *total = total.saturating_add(sample);
        }
        profile.combat_engine_clone_elapsed_ns = profile
            .combat_engine_clone_elapsed_ns
            .saturating_add(timing.combat_engine_clone_elapsed_ns);
        profile.combat_rng_clone_elapsed_ns = profile
            .combat_rng_clone_elapsed_ns
            .saturating_add(timing.combat_rng_clone_elapsed_ns);
        profile.combat_runtime_clone_elapsed_ns = profile
            .combat_runtime_clone_elapsed_ns
            .saturating_add(timing.combat_runtime_clone_elapsed_ns);
        profile.execution_elapsed_ns = profile
            .execution_elapsed_ns
            .saturating_add(timing.execution_elapsed_ns);
        self.profile.set(profile);
        step
    }

    fn terminal(&self, position: &CombatPosition) -> CombatTerminal {
        EngineCombatStepper.terminal(position)
    }
}

fn main() {
    if let Err(error) = run(Cli::parse()) {
        eprintln!("Error: {error}");
        std::process::exit(1);
    }
}

fn run(args: Cli) -> Result<(), String> {
    let started = Instant::now();
    let read_started = Instant::now();
    let bytes = std::fs::read(&args.case)
        .map_err(|error| format!("cannot read combat case '{}': {error}", args.case.display()))?;
    let read_elapsed_ns = elapsed_nanos(read_started);
    let parse_started = Instant::now();
    let loaded: CombatCaseRoot = serde_json::from_slice(&bytes).map_err(|error| {
        format!(
            "cannot parse combat case '{}': {error}",
            args.case.display()
        )
    })?;
    let parse_elapsed_ns = elapsed_nanos(parse_started);
    if loaded.schema != "combat_case" && loaded.schema != "combat_gap_case" {
        return Err(format!(
            "expected combat_case or combat_gap_case, got {}",
            loaded.schema
        ));
    }
    let watch_actions = args
        .watch_actions
        .as_ref()
        .map(|path| read_watch_actions(path))
        .transpose()?;
    let (search_position, watch_action_start) = if let Some(target_turn) = args.start_at_player_turn
    {
        replay_to_player_turn(
            &loaded.position,
            watch_actions
                .as_deref()
                .expect("clap requires watch actions for a reroot"),
            target_turn,
        )?
    } else {
        (loaded.position.clone(), 0)
    };
    let watch_corridor = watch_actions
        .as_deref()
        .map(|actions| replay_watch_corridor(&search_position, &actions[watch_action_start..]))
        .transpose()?;

    let setup_started = Instant::now();
    let search_root_player_turn = search_position.combat.turn.turn_count;
    let root = CombatDecisionRoot::new(search_position)
        .map_err(|error| format!("invalid combat case root: {error:?}"))?;
    let config = LocalTurnGraphWitnessConfig {
        generator: TurnOptionGeneratorConfig {
            max_engine_steps_per_transition: args.max_engine_steps_per_transition,
            allow_potion_expenditure: args.max_potions_used != Some(0),
            ..TurnOptionGeneratorConfig::default()
        },
        generation_quantum_work: args.generation_quantum_work,
        backed_generation_quantum_work: 256,
        initial_expansion_work: 64,
        root_initial_expansion_work: 2_048,
        lookahead_max_evaluations: args.max_nodes.saturating_div(24).max(1),
        lookahead_work_per_evaluation: 24,
        max_turn_depth: args.max_turn_depth,
        satisfaction: if args.improve_incumbent {
            OracleCombatWitnessSatisfaction::BudgetOrExhaustion
        } else if let Some(limit) = args.max_hp_loss {
            OracleCombatWitnessSatisfaction::HpLossAtMost(limit)
        } else {
            OracleCombatWitnessSatisfaction::FirstWitness
        },
        max_potions_used: args.max_potions_used,
    };
    let policy = existing_combat_knowledge_policy_v1();
    let policy = if args.typed_plan_guide {
        combat_plan_state_guide_policy_v1(policy)
    } else {
        policy
    };
    let mut session = LocalTurnGraphWitnessSession::with_policy(root, config, policy);
    let profiled_stepper = ProfiledEngineCombatStepper::default();
    let engine_stepper = EngineCombatStepper;
    let search_stepper: &dyn CombatStepper = if args.profile_transition_clone_cost {
        &profiled_stepper
    } else {
        &engine_stepper
    };
    let setup_elapsed_ns = elapsed_nanos(setup_started);
    let policy_line_started = Instant::now();
    let policy_line_report = args
        .plan_compatible_policy_line
        .then(|| {
            session.offer_plan_compatible_policy_line_with_suffix_probes(
                args.max_turn_depth,
                256,
                args.plan_compatible_suffix_work,
                &EngineCombatStepper,
            )
        })
        .transpose()?;
    let policy_line_elapsed_ns = elapsed_nanos(policy_line_started);
    let search_started = Instant::now();
    let report = session.advance(
        LocalTurnGraphWitnessQuantum {
            additional_selections: args.max_selections,
            additional_generation_work: args.max_nodes,
            additional_engine_steps: args
                .max_nodes
                .saturating_mul(args.max_engine_steps_per_transition),
            deadline: Some(Instant::now() + Duration::from_millis(args.wall_ms)),
        },
        search_stepper,
    );
    let search_elapsed_ns = elapsed_nanos(search_started);
    let root_action_families = session.root_action_families();
    let watch_corridor = watch_corridor.map(|corridor| {
        let hashes = corridor
            .iter()
            .map(|(_, exact_state_hash)| exact_state_hash.clone())
            .collect::<Vec<_>>();
        corridor
            .into_iter()
            .enumerate()
            .map(|(index, (player_turn, exact_state_hash))| {
                let snapshot = session.state_snapshot_by_exact_hash(&exact_state_hash);
                let incoming_edge = (index > 0).then(|| {
                    let parent_hash = &hashes[index - 1];
                    session.edge_snapshot_by_exact_hashes(parent_hash, &exact_state_hash)
                });
                json!({
                    "player_turn": player_turn,
                    "exact_state_hash": exact_state_hash,
                    "present": snapshot.is_some(),
                    "snapshot": snapshot,
                    "incoming_edge": incoming_edge.flatten(),
                })
            })
            .collect::<Vec<_>>()
    });

    if args.expect_witness && report.witness.is_none() {
        return Err("combat contract failed: no replay-verified witness".to_owned());
    }
    if let Some(minimum) = args.expect_min_final_hp {
        let actual = report
            .witness
            .as_ref()
            .map(|witness| witness.final_position.combat.entities.player.current_hp)
            .ok_or_else(|| "combat contract failed: final HP requires a witness".to_owned())?;
        if actual < minimum {
            return Err(format!(
                "combat contract failed: final HP {actual} is below {minimum}"
            ));
        }
    }
    if let Some(maximum) = args.expect_max_plan_suffix_work {
        let actual = policy_line_report
            .as_ref()
            .map(|line| line.suffix_probe_generation_work)
            .unwrap_or_default();
        if actual > maximum {
            return Err(format!(
                "combat contract failed: plan suffix work {actual} exceeds {maximum}"
            ));
        }
    }

    let witness = report.witness.as_ref();
    if args.performance_only {
        let transitions = report.counters.applied_action_transitions;
        let timing = report.performance_timing;
        let per_transition =
            |elapsed_ns: u64| (transitions > 0).then(|| elapsed_ns as f64 / transitions as f64);
        let clone_profile = args.profile_transition_clone_cost.then(|| {
            let profile = profiled_stepper.snapshot();
            let per_sample = |elapsed_ns: u64| {
                (profile.samples > 0).then(|| elapsed_ns as f64 / profile.samples as f64)
            };
            json!({
                "sample_interval": TRANSITION_CLONE_PROFILE_INTERVAL,
                "transition_calls": profile.calls,
                "samples": profile.samples,
                "sampled_collection_lengths": {
                    "mean_emitted_events": (profile.samples > 0).then(|| profile.emitted_event_items as f64 / profile.samples as f64),
                    "max_emitted_events": profile.max_emitted_event_items,
                    "mean_engine_diagnostics": (profile.samples > 0).then(|| profile.engine_diagnostic_items as f64 / profile.samples as f64),
                    "max_engine_diagnostics": profile.max_engine_diagnostic_items,
                    "mean_monster_protocol": (profile.samples > 0).then(|| profile.monster_protocol_items as f64 / profile.samples as f64),
                    "max_monster_protocol": profile.max_monster_protocol_items,
                },
                "type_size_bytes": {
                    "combat_exact_state_key": std::mem::size_of::<CombatExactStateKey>(),
                    "combat_state": std::mem::size_of::<sts_core::runtime::combat::CombatState>(),
                    "card_zones": std::mem::size_of::<sts_core::runtime::combat::CardZones>(),
                    "combat_card": std::mem::size_of::<sts_core::runtime::combat::CombatCard>(),
                    "entity_state": std::mem::size_of::<sts_core::runtime::combat::EntityState>(),
                    "player_entity": std::mem::size_of::<sts_core::runtime::combat::PlayerEntity>(),
                    "monster_entity": std::mem::size_of::<sts_core::runtime::combat::MonsterEntity>(),
                    "power": std::mem::size_of::<sts_core::runtime::combat::Power>(),
                    "combat_runtime_hints": std::mem::size_of::<sts_core::runtime::combat::CombatRuntimeHints>(),
                },
                "total_elapsed_ns": {
                    "engine_clone": profile.engine_clone_elapsed_ns,
                    "combat_clone": profile.combat_clone_elapsed_ns,
                    "execution": profile.execution_elapsed_ns,
                },
                "mean_ns_per_sample": {
                    "engine_clone": per_sample(profile.engine_clone_elapsed_ns),
                    "combat_clone": per_sample(profile.combat_clone_elapsed_ns),
                    "combat_clone_components": {
                        "meta": per_sample(profile.combat_meta_clone_elapsed_ns),
                        "turn": per_sample(profile.combat_turn_clone_elapsed_ns),
                        "zones": per_sample(profile.combat_zones_clone_elapsed_ns),
                        "entities": per_sample(profile.combat_entities_clone_elapsed_ns),
                        "engine": per_sample(profile.combat_engine_clone_elapsed_ns),
                        "rng": per_sample(profile.combat_rng_clone_elapsed_ns),
                        "runtime": per_sample(profile.combat_runtime_clone_elapsed_ns),
                    },
                    "zone_components": {
                        "draw_pile": per_sample(profile.combat_zone_component_elapsed_ns[0]),
                        "hand": per_sample(profile.combat_zone_component_elapsed_ns[1]),
                        "discard_pile": per_sample(profile.combat_zone_component_elapsed_ns[2]),
                        "exhaust_pile": per_sample(profile.combat_zone_component_elapsed_ns[3]),
                        "limbo": per_sample(profile.combat_zone_component_elapsed_ns[4]),
                        "queued_cards": per_sample(profile.combat_zone_component_elapsed_ns[5]),
                    },
                    "entity_components": {
                        "player": per_sample(profile.combat_entity_component_elapsed_ns[0]),
                        "monsters": per_sample(profile.combat_entity_component_elapsed_ns[1]),
                        "potions": per_sample(profile.combat_entity_component_elapsed_ns[2]),
                        "power_db": per_sample(profile.combat_entity_component_elapsed_ns[3]),
                    },
                    "runtime_components": {
                        "colorless_pool": per_sample(profile.combat_runtime_component_elapsed_ns[0]),
                        "emitted_events": per_sample(profile.combat_runtime_component_elapsed_ns[1]),
                        "engine_diagnostics": per_sample(profile.combat_runtime_component_elapsed_ns[2]),
                        "pending_rewards": per_sample(profile.combat_runtime_component_elapsed_ns[3]),
                        "last_drawn_cards": per_sample(profile.combat_runtime_component_elapsed_ns[4]),
                        "monster_protocol": per_sample(profile.combat_runtime_component_elapsed_ns[5]),
                    },
                    "key_build_components": {
                        "engine": per_sample(profile.exact_key_build_component_elapsed_ns[0]),
                        "turn": per_sample(profile.exact_key_build_component_elapsed_ns[1]),
                        "meta": per_sample(profile.exact_key_build_component_elapsed_ns[2]),
                        "zones": per_sample(profile.exact_key_build_component_elapsed_ns[3]),
                        "monsters": per_sample(profile.exact_key_build_component_elapsed_ns[4]),
                        "powers": per_sample(profile.exact_key_build_component_elapsed_ns[5]),
                        "potions": per_sample(profile.exact_key_build_component_elapsed_ns[6]),
                        "queue": per_sample(profile.exact_key_build_component_elapsed_ns[7]),
                        "runtime": per_sample(profile.exact_key_build_component_elapsed_ns[8]),
                        "rng": per_sample(profile.exact_key_build_component_elapsed_ns[9]),
                        "player": per_sample(profile.exact_key_build_component_elapsed_ns[10]),
                    },
                    "execution": per_sample(profile.execution_elapsed_ns),
                },
            })
        });
        let output = json!({
            "schema_name": "CombatCasePerformanceProfileV2",
            "schema_version": 2,
            "runner": "lightweight-combat-contract",
            "detail_timing_sample_interval": DETAIL_TIMING_SAMPLE_INTERVAL,
            "transition_clone_profile": clone_profile,
            "case": args.case,
            "search_elapsed_ns": search_elapsed_ns,
            "status": format!("{:?}", report.status),
            "witness": witness.map(|witness| json!({
                "final_hp": witness.final_position.combat.entities.player.current_hp,
                "actions": witness.actions.len(),
            })),
            "counters": {
                "selections": report.counters.selections,
                "node_visits": report.counters.node_visits,
                "generation_work": report.counters.generation_work,
                "engine_steps": report.counters.engine_steps,
                "exact_nodes": report.counters.exact_nodes,
                "exact_edges": report.counters.exact_edges,
                "completed_turn_options": report.counters.completed_turn_options,
                "applied_action_transitions": transitions,
                "unique_successor_states": report.counters.unique_successor_states,
                "duplicate_exact_successors": report.counters.duplicate_exact_successors,
                "duplicate_successor_edges": report.counters.duplicate_successor_edges,
                "terminal_win_options": report.counters.terminal_win_options,
                "witness_replay_attempts": report.counters.witness_replay_attempts,
                "witness_replay_improvements": report.counters.witness_replay_improvements,
                "witness_replay_dominated_skips": report.counters.witness_replay_dominated_skips,
            },
            "timing_ns": timing,
            "ns_per_applied_transition": {
                "simulation": per_transition(timing.transition_simulation_elapsed_ns),
                "identity": per_transition(timing.transition_identity_elapsed_ns),
                "key_build": per_transition(timing.transition_key_build_elapsed_ns),
                "key_index": per_transition(timing.transition_key_index_elapsed_ns),
                "seen_set": per_transition(timing.transition_seen_elapsed_ns),
                "publish": per_transition(timing.transition_publish_elapsed_ns),
                "publish_trace_node": per_transition(
                    timing.transition_publish_trace_node_elapsed_ns,
                ),
                "publish_boundary": per_transition(
                    timing.transition_publish_boundary_elapsed_ns,
                ),
                "publish_complete": per_transition(
                    timing.transition_publish_complete_elapsed_ns,
                ),
                "publish_push": per_transition(timing.transition_publish_push_elapsed_ns),
                "publish_guide": per_transition(
                    timing.transition_publish_guide_elapsed_ns,
                ),
                "publish_retain": per_transition(
                    timing.transition_publish_retain_elapsed_ns,
                ),
                "publish_agenda": per_transition(
                    timing.transition_publish_agenda_elapsed_ns,
                ),
            },
        });
        println!(
            "{}",
            serde_json::to_string_pretty(&output).map_err(|error| error.to_string())?
        );
        return Ok(());
    }
    let output = json!({
        "schema_name": "CombatCaseContractResultV1",
        "schema_version": 1,
        "status": if args.expect_witness { "passed" } else { "completed" },
        "runner": "lightweight-combat-contract",
        "case": args.case,
        "search_root_player_turn": search_root_player_turn,
        "elapsed_ms": started.elapsed().as_millis(),
        "final_hp": witness.map(|witness| {
            witness.final_position.combat.entities.player.current_hp
        }),
        "witness_actions": witness.map(|witness| witness.actions.len()),
        "phase_ns": {
            "read_case": read_elapsed_ns,
            "parse_case": parse_elapsed_ns,
            "setup": setup_elapsed_ns,
            "policy_line": policy_line_elapsed_ns,
            "main_search": search_elapsed_ns,
        },
        "search_counters": {
            "selections": report.counters.selections,
            "node_visits": report.counters.node_visits,
            "generation_work": report.counters.generation_work,
            "engine_steps": report.counters.engine_steps,
            "exact_nodes": report.counters.exact_nodes,
            "exact_edges": report.counters.exact_edges,
            "completed_turn_options": report.counters.completed_turn_options,
            "applied_action_transitions": report.counters.applied_action_transitions,
            "unique_successor_states": report.counters.unique_successor_states,
            "duplicate_exact_successors": report.counters.duplicate_exact_successors,
            "duplicate_successor_edges": report.counters.duplicate_successor_edges,
        },
        "root_action_families": root_action_families,
        "watch_corridor": watch_corridor,
        "performance_ns": {
            "selection": report.performance_timing.selection_elapsed_ns,
            "generation": report.performance_timing.generation_elapsed_ns,
            "admission": report.performance_timing.admission_elapsed_ns,
            "atomic_expand": report.performance_timing.atomic_expand_elapsed_ns,
            "transition_simulation": report.performance_timing.transition_simulation_elapsed_ns,
            "transition_identity": report.performance_timing.transition_identity_elapsed_ns,
            "transition_key_build": report.performance_timing.transition_key_build_elapsed_ns,
            "transition_key_index": report.performance_timing.transition_key_index_elapsed_ns,
            "transition_admission": report.performance_timing.transition_admission_elapsed_ns,
            "transition_trace": report.performance_timing.transition_trace_elapsed_ns,
            "transition_seen": report.performance_timing.transition_seen_elapsed_ns,
            "transition_publish": report.performance_timing.transition_publish_elapsed_ns,
            "transition_publish_trace_node": report.performance_timing.transition_publish_trace_node_elapsed_ns,
            "transition_publish_boundary": report.performance_timing.transition_publish_boundary_elapsed_ns,
            "transition_publish_complete": report.performance_timing.transition_publish_complete_elapsed_ns,
            "transition_publish_push": report.performance_timing.transition_publish_push_elapsed_ns,
            "transition_publish_guide": report.performance_timing.transition_publish_guide_elapsed_ns,
            "transition_publish_retain": report.performance_timing.transition_publish_retain_elapsed_ns,
            "transition_publish_agenda": report.performance_timing.transition_publish_agenda_elapsed_ns,
        },
        "plan_suffix": policy_line_report.as_ref().map(|line| json!({
            "proposed_turns": line.proposed_turns,
            "chosen_action_transitions": line.chosen_action_transitions,
            "proposed_actions": line.proposed_actions,
            "rejected_preview_transitions": line.rejected_preview_transitions,
            "deferred_actions": line.deferred_actions,
            "policy_line_engine_steps": line.engine_steps,
            "policy_line_performance_ns": {
                "legal_surface": line.legal_surface_elapsed_ns,
                "policy_ranking": line.policy_ranking_elapsed_ns,
                "transition_preview": line.transition_preview_elapsed_ns,
                "action_identity": line.action_identity_elapsed_ns,
                "plan_annotation": line.plan_annotation_elapsed_ns,
                "successor_admission": line.successor_admission_elapsed_ns,
            },
            "attempts": line.suffix_probe_attempts,
            "generation_work": line.suffix_probe_generation_work,
            "engine_steps": line.suffix_probe_engine_steps,
            "completed_turn_options": line.suffix_probe_completed_turn_options,
            "applied_action_transitions": line.suffix_probe_applied_action_transitions,
            "unique_successor_states": line.suffix_probe_unique_successor_states,
            "exact_nodes": line.suffix_probe_exact_nodes,
            "exact_edges": line.suffix_probe_exact_edges,
            "performance_ns": line.suffix_probe_performance_timing,
            "setup_elapsed_ns": line.suffix_probe_setup_elapsed_ns,
            "advance_elapsed_ns": line.suffix_probe_advance_elapsed_ns,
            "replay_elapsed_ns": line.suffix_probe_replay_elapsed_ns,
        })),
    });
    println!(
        "{}",
        serde_json::to_string_pretty(&output).map_err(|error| error.to_string())?
    );
    Ok(())
}

fn read_watch_actions(path: &PathBuf) -> Result<Vec<ClientInput>, String> {
    let bytes = std::fs::read(path)
        .map_err(|error| format!("cannot read watch actions '{}': {error}", path.display()))?;
    serde_json::from_slice::<Vec<ClientInput>>(&bytes)
        .map_err(|error| format!("cannot parse watch actions '{}': {error}", path.display()))
}

fn replay_to_player_turn(
    root: &CombatPosition,
    actions: &[ClientInput],
    target_turn: u32,
) -> Result<(CombatPosition, usize), String> {
    let stepper = EngineCombatStepper;
    let mut position = root.clone();
    if position.combat.turn.turn_count == target_turn
        && matches!(position.engine, EngineState::CombatPlayerTurn)
    {
        return Ok((position, 0));
    }
    for (index, action) in actions.iter().cloned().enumerate() {
        let step = apply_watch_action(&stepper, &position, action, index)?;
        position = step;
        if matches!(position.engine, EngineState::CombatPlayerTurn)
            && position.combat.turn.turn_count == target_turn
        {
            return Ok((position, index.saturating_add(1)));
        }
        if stepper.terminal(&position) != CombatTerminal::Unresolved {
            break;
        }
    }
    Err(format!(
        "watch replay never reached player turn {target_turn}"
    ))
}

fn replay_watch_corridor(
    root: &CombatPosition,
    actions: &[ClientInput],
) -> Result<Vec<(u32, String)>, String> {
    let stepper = EngineCombatStepper;
    let mut position = root.clone();
    let mut last_turn = position.combat.turn.turn_count;
    let mut corridor = vec![(
        last_turn,
        combat_exact_state_hash_v1(&position.engine, &position.combat),
    )];
    for (index, action) in actions.iter().cloned().enumerate() {
        position = apply_watch_action(&stepper, &position, action, index)?;
        if matches!(position.engine, EngineState::CombatPlayerTurn)
            && position.combat.turn.turn_count > last_turn
        {
            last_turn = position.combat.turn.turn_count;
            corridor.push((
                last_turn,
                combat_exact_state_hash_v1(&position.engine, &position.combat),
            ));
        }
        if stepper.terminal(&position) != CombatTerminal::Unresolved {
            break;
        }
    }
    Ok(corridor)
}

fn apply_watch_action(
    stepper: &EngineCombatStepper,
    position: &CombatPosition,
    action: ClientInput,
    index: usize,
) -> Result<CombatPosition, String> {
    let step = stepper.apply_to_stable(
        position,
        action,
        CombatStepLimits {
            max_engine_steps: 1_000,
            deadline: None,
        },
    );
    if step.truncated || step.timed_out {
        return Err(format!(
            "watch replay stopped at action {index}: truncated={} timed_out={}",
            step.truncated, step.timed_out
        ));
    }
    Ok(step.position)
}

fn elapsed_nanos(started: Instant) -> u64 {
    u64::try_from(started.elapsed().as_nanos()).unwrap_or(u64::MAX)
}
