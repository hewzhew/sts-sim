use super::combat_planning_view::oracle_lab_guide_lane_label;
use super::combat_replay_tools::replay_combat_path;
use super::combat_trace_view::{combat_position_snapshot, compact_corridor_report};
use super::*;

#[derive(Debug, Args)]
pub(super) struct CombatCaseLegacyGlobalArgs {
    #[arg(long)]
    case: PathBuf,
    /// Optional typed action residual. It changes proposal order only;
    /// the production agenda still owns search and exact replay.
    #[arg(long)]
    action_imitation_artifact: Option<PathBuf>,
    #[arg(long, default_value_t = 250_000)]
    max_nodes: usize,
    #[arg(long, default_value_t = 5_000)]
    wall_ms: u64,
    #[arg(long, default_value_t = 250)]
    max_engine_steps_per_transition: usize,
    /// Lab-only control: keep the action policy but disable every state
    /// guide, leaving the single Levin/PHS-style anchor ordering.
    #[arg(long)]
    anchor_only: bool,
    /// Diagnostic capability boundary: disable the legacy CombatSearchV2
    /// complete-suffix donor while retaining the new planner's action
    /// priors and state guides.
    #[arg(long)]
    without_v2_donor: bool,
    /// Repeat to inspect membership for several exact corridor states in
    /// one search run.
    #[arg(long)]
    watch_state_hash: Vec<String>,
    /// Replay one complete verified witness and watch every exact player-
    /// turn boundary without adding corridor guidance or changing search.
    #[arg(long)]
    watch_corridor_actions: Option<PathBuf>,
    /// Start search after this many complete player turns from the watched
    /// witness. This reuses the verified action file and avoids hand-
    /// slicing JSON prefixes.
    #[arg(
        long,
        requires = "watch_corridor_actions",
        conflicts_with = "prefix_actions"
    )]
    corridor_prefix_turns: Option<usize>,
    /// Replay one or more exact legal input-prefix files in order before
    /// starting the planner. Repeat the flag to compose verified segments.
    #[arg(long)]
    prefix_actions: Vec<PathBuf>,
    /// Stop replay as soon as this exact player-turn boundary is reached.
    /// This avoids hand-slicing a saved action prefix to inspect or export
    /// an earlier turn.
    #[arg(long, requires = "prefix_actions")]
    prefix_stop_at_player_turn: Option<u32>,
    /// Print compact, card-labelled traces instead of raw action arrays.
    #[arg(long, conflicts_with = "full")]
    readable: bool,
    /// Print the legacy full probe including raw actions and replay traces.
    /// The default is the compact one-page diagnostic report.
    #[arg(long, conflicts_with = "readable")]
    full: bool,
    /// Replay the prefix and print its exact successor without starting search.
    #[arg(long)]
    replay_only: bool,
    /// Diagnostic-only replay counterfactual. Replace the combat root's
    /// current HP before applying --prefix-actions; the output remains
    /// explicitly non-authoritative for the original run.
    #[arg(long, requires = "replay_only")]
    counterfactual_hp: Option<i32>,
    /// Save the exact prefix successor as a standalone combat case.
    #[arg(long)]
    export_prefix_case: Option<PathBuf>,
    /// Lab-only perfect-information control: replay this verified combat
    /// witness and add its exact player-turn states as a fifth shadow
    /// guide. Requires --shadow-corridor-case.
    #[arg(long, requires = "shadow_corridor_case")]
    shadow_corridor_actions: Option<PathBuf>,
    /// Combat start corresponding to --shadow-corridor-actions.
    #[arg(long, requires = "shadow_corridor_actions")]
    shadow_corridor_case: Option<PathBuf>,
    /// How the lab-only corridor guide recognizes promising states.
    /// `typed-feature` never reads an exact state hash while ranking.
    #[arg(long, value_enum, default_value_t = ShadowCorridorGuide::Exact)]
    shadow_corridor_guide: ShadowCorridorGuide,
    /// Lab-only structural control: when an exact corridor is supplied,
    /// suppress the ordinary state guides and retain only the sparse
    /// exact-corridor lane plus the policy-only anchor. Actions are still
    /// generated and executed normally; no witness action is forced.
    #[arg(long, requires = "shadow_corridor_actions")]
    shadow_corridor_only: bool,
    /// Load a distilled typed-feature prototype model. Unlike the
    /// corridor controls, inference does not load witness actions, exact
    /// hashes, or the source combat case.
    #[arg(
        long,
        conflicts_with = "shadow_corridor_actions",
        conflicts_with = "shadow_corridor_case"
    )]
    shadow_value_prototype: Option<PathBuf>,
    /// If a replay-verified win is found, save its exact ClientInput list.
    #[arg(long)]
    export_witness_actions: Option<PathBuf>,
    /// Add newly proven one-turn loss prototypes to the loaded value
    /// artifact and save a new generation. Requires evidence collection.
    #[arg(long, requires = "shadow_value_prototype")]
    export_augmented_value_prototype: Option<PathBuf>,
    /// Retain at most this many gap-free states whose fully enumerated
    /// complete-turn language consists only of terminal losses.
    #[arg(long, default_value_t = 0)]
    one_turn_loss_evidence_limit: usize,
    /// Retain at most this many states with an exact complete option that
    /// reaches the next player turn or wins immediately.
    #[arg(long, default_value_t = 0)]
    one_turn_viability_evidence_limit: usize,
}

pub(super) fn run(args: CombatCaseLegacyGlobalArgs) -> Result<(), String> {
    let CombatCaseLegacyGlobalArgs {
        case,
        action_imitation_artifact,
        max_nodes,
        wall_ms,
        max_engine_steps_per_transition,
        anchor_only,
        without_v2_donor,
        watch_state_hash,
        watch_corridor_actions,
        corridor_prefix_turns,
        prefix_actions,
        prefix_stop_at_player_turn,
        readable,
        full,
        replay_only,
        counterfactual_hp,
        export_prefix_case,
        shadow_corridor_actions,
        shadow_corridor_case,
        shadow_corridor_guide,
        shadow_corridor_only,
        shadow_value_prototype,
        export_witness_actions,
        export_augmented_value_prototype,
        one_turn_loss_evidence_limit,
        one_turn_viability_evidence_limit,
    } = args;
    let command_started = Instant::now();
    let case_path = case.clone();
    let watched_corridor = watch_corridor_actions
        .as_ref()
        .map(|actions| {
            load_exact_turn_corridor(
                &case,
                std::slice::from_ref(actions),
                max_engine_steps_per_transition,
            )
        })
        .transpose()?;
    let mut case = load_combat_case(&case)?;
    let original_hp = case.position.combat.entities.player.current_hp;
    if let Some(hp) = counterfactual_hp {
        let max_hp = case.position.combat.entities.player.max_hp;
        if !(1..=max_hp).contains(&hp) {
            return Err(format!(
                "counterfactual HP must be within 1..={max_hp}, got {hp}"
            ));
        }
        case.position.combat.entities.player.current_hp = hp;
        case.combat = sts_oracle_runtime::eval::combat_case::combat_summary(&case.position);
    }
    let stepper = EngineCombatStepper;
    let initial_position = case.position.clone();
    let mut position = initial_position.clone();
    let mut prefix = prefix_actions
        .iter()
        .map(|path| {
            serde_json::from_slice::<Vec<ClientInput>>(
                &std::fs::read(path).map_err(|error| error.to_string())?,
            )
            .map_err(|error| format!("invalid prefix action list: {error}"))
        })
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    if let Some(turns) = corridor_prefix_turns {
        let actions_path = watch_corridor_actions
            .as_ref()
            .expect("clap requires watched corridor actions");
        let corridor_actions = serde_json::from_slice::<Vec<ClientInput>>(
            &std::fs::read(actions_path).map_err(|error| error.to_string())?,
        )
        .map_err(|error| format!("invalid corridor action list: {error}"))?;
        if turns > 0 {
            let mut ended_turns = 0_usize;
            for input in corridor_actions {
                let ends_turn = matches!(input, ClientInput::EndTurn);
                prefix.push(input);
                if ends_turn {
                    ended_turns = ended_turns.saturating_add(1);
                    if ended_turns == turns {
                        break;
                    }
                }
            }
            if ended_turns != turns {
                return Err(format!(
                            "corridor contains only {ended_turns} completed player turns; requested prefix {turns}"
                        ));
            }
        }
    }
    let mut applied_prefix = Vec::with_capacity(prefix.len());
    let mut prefix_replay_actions = Vec::with_capacity(prefix.len());
    for (action_index, input) in prefix.iter().enumerate() {
        if prefix_stop_at_player_turn.is_some_and(|target_turn| {
            position.combat.turn.turn_count == target_turn
                && matches!(position.engine, EngineState::CombatPlayerTurn)
        }) {
            break;
        }
        if stepper.choice_for_legal_input(&position, input).is_none() {
            return Err(format!(
                "combat prefix action {action_index} is not legal at its exact state: {input:?}"
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
        if step.truncated {
            return Err(format!(
                "combat prefix action {action_index} exceeded the engine-step limit"
            ));
        }
        prefix_replay_actions.push(TurnOptionAction {
            input: input.clone(),
            expected_successor_hash:
                sts_oracle_runtime::ai::combat_state_key::combat_exact_state_hash_v2(
                    &step.position.engine,
                    &step.position.combat,
                )
                .into(),
            engine_steps: step.engine_steps,
        });
        applied_prefix.push(input.clone());
        position = step.position;
    }
    if let Some(target_turn) = prefix_stop_at_player_turn {
        if position.combat.turn.turn_count != target_turn
            || !matches!(position.engine, EngineState::CombatPlayerTurn)
        {
            return Err(format!(
                "prefix did not reach player turn {target_turn}; stopped at turn {} in {:?}",
                position.combat.turn.turn_count, position.engine
            ));
        }
    }
    prefix = applied_prefix;
    if let Some(path) = export_prefix_case.as_ref() {
        let mut focused_case = case.clone();
        focused_case.position = position.clone();
        focused_case.combat =
            sts_oracle_runtime::eval::combat_case::combat_summary(&focused_case.position);
        focused_case.gap.boundary = format!(
            "{} + {} exact prefix actions",
            focused_case.gap.boundary,
            prefix.len()
        );
        focused_case.gap.reason = "oracle_lab_prefix_successor".to_string();
        sts_oracle_runtime::eval::combat_case::save_combat_case(path, &focused_case)?;
    }
    if replay_only {
        let prefix_trace = replay_combat_path(
            initial_position,
            &prefix_replay_actions,
            max_engine_steps_per_transition,
        )?;
        return print_json(&serde_json::json!({
            "schema_name": "OracleCombatPrefixReplayV1",
            "schema_version": 1,
            "action_count": prefix.len(),
            "counterfactual": {
                "enabled": counterfactual_hp.is_some(),
                "original_hp": original_hp,
                "replay_hp": case.position.combat.entities.player.current_hp,
            },
            "exported_case": export_prefix_case,
            "trace": prefix_trace,
            "guide_components": {
                "progress": sts_oracle_runtime::ai::combat_search_v2::oracle_action_policy::oracle_combat_state_guide_components(&position),
                "survival": sts_oracle_runtime::ai::combat_search_v2::oracle_action_policy::oracle_combat_survival_guide_components(&position),
                "horizon": sts_oracle_runtime::ai::combat_search_v2::oracle_action_policy::oracle_combat_horizon_guide_components(&position),
                "setup": sts_oracle_runtime::ai::combat_search_v2::oracle_action_policy::oracle_combat_setup_guide_components(&position),
            },
            "successor_exact_state_hash": sts_oracle_runtime::ai::combat_state_key::combat_exact_state_hash_v2(
                &position.engine,
                &position.combat,
            ),
            "successor": combat_position_snapshot(&position),
        }));
    }
    let search_root_position = position.clone();
    let root = CombatDecisionRoot::new(position)
        .map_err(|error| format!("invalid combat case root: {error:?}"))?;
    let initial_hp = root.position().combat.entities.player.current_hp;
    let base_policy = action_imitation_artifact
        .as_deref()
        .map(|path| load_action_imitation_policy(path, existing_combat_knowledge_policy_v1()))
        .transpose()?
        .unwrap_or_else(existing_combat_knowledge_policy_v1);
    let (policy, shadow_corridor, mut shadow_value_artifact) =
        if let Some(model_path) = shadow_value_prototype.as_ref() {
            let artifact = load_value_prototype(model_path)?;
            let policy = value_prototype_shadow_policy(base_policy, &artifact);
            (policy, None, Some(artifact))
        } else {
            match (
                shadow_corridor_case.as_ref(),
                shadow_corridor_actions.as_ref(),
            ) {
                (Some(case_path), Some(actions_path)) => {
                    let corridor = load_exact_turn_corridor(
                        case_path,
                        std::slice::from_ref(actions_path),
                        max_engine_steps_per_transition,
                    )?;
                    let policy = exact_corridor_shadow_policy(
                        base_policy,
                        &corridor,
                        shadow_corridor_guide,
                        shadow_corridor_only,
                    );
                    (policy, Some(corridor), None)
                }
                (None, None) => (base_policy, None, None),
                _ => unreachable!("clap requires both shadow corridor arguments"),
            }
        };
    let policy = if anchor_only {
        anchor_only_policy(policy)
    } else {
        policy
    };
    let mut search = OracleCombatWitnessSession::with_policy(
        root,
        OracleCombatWitnessConfig {
            generator: TurnOptionGeneratorConfig {
                max_engine_steps_per_transition,
                ..TurnOptionGeneratorConfig::default()
            },
            generation_work_per_agenda_pop: 4,
            satisfaction: OracleCombatWitnessSatisfaction::BudgetOrExhaustion,
        },
        policy,
    );
    search.set_one_turn_loss_evidence_limit(one_turn_loss_evidence_limit);
    search.set_one_turn_viability_evidence_limit(one_turn_viability_evidence_limit);
    let started = Instant::now();
    let deadline = started + Duration::from_millis(wall_ms);
    let mut advisor_nodes = 0u64;
    let mut advisor_elapsed_ms = 0u64;
    let mut advisor_status = "disabled";
    if !without_v2_donor {
        let mut advisor = ExistingCombatKnowledgeAdvisorV1::new(
            &search_root_position,
            max_engine_steps_per_transition,
        );
        let remaining = deadline.saturating_duration_since(Instant::now());
        match advisor.advance(Some(remaining), Some(remaining))? {
            ExistingCombatKnowledgeAdvisorAdvanceV1::Pending => {
                advisor_status = "pending";
            }
            ExistingCombatKnowledgeAdvisorAdvanceV1::Proposal(proposal) => {
                search.offer_witness_proposal(proposal);
                advisor_status = "proposal";
            }
            ExistingCombatKnowledgeAdvisorAdvanceV1::Exhausted => {
                advisor_status = "exhausted";
            }
        }
        advisor_nodes = advisor.total_nodes();
        advisor_elapsed_ms = advisor
            .total_elapsed()
            .as_millis()
            .min(u128::from(u64::MAX)) as u64;
    }
    let report = search.advance(
        &EngineCombatStepper,
        OracleCombatWitnessQuantum {
            additional_agenda_pops: max_nodes,
            additional_generation_work: max_nodes,
            additional_engine_steps: max_nodes.saturating_mul(max_engine_steps_per_transition),
            deadline: Some(deadline),
        },
    );
    let search_elapsed = started.elapsed();
    let summary_started = Instant::now();
    let progress = search.progress_snapshot();
    if let (Some(path), Some(artifact)) = (
        export_augmented_value_prototype.as_ref(),
        shadow_value_artifact.as_mut(),
    ) {
        artifact.add_one_turn_viability_positions(
            search
                .one_turn_viability_evidence()
                .iter()
                .map(|sample| &sample.position),
        );
        artifact.add_one_turn_loss_positions(
            search
                .one_turn_loss_evidence()
                .iter()
                .map(|sample| &sample.position),
        );
        save_value_prototype(path, artifact)?;
    }
    let shadow_corridor_report = shadow_corridor
        .as_ref()
        .map(|corridor| corridor.report(&search, shadow_corridor_guide))
        .or_else(|| {
            shadow_value_artifact
                .as_ref()
                .map(CombatValuePrototypeArtifactV1::report)
        });
    let one_turn_viability_evidence = search
        .one_turn_viability_evidence()
        .iter()
        .map(|evidence| {
            json!({
                "proof": "ExactWitness",
                "horizon": "BeforeNextPlayerTurnOrWin",
                "exact_state_hash": evidence.exact_state_hash,
                "player_turn": evidence.position.combat.turn.turn_count,
                "player_hp": evidence.position.combat.entities.player.current_hp,
                "witness_boundary": format!("{:?}", evidence.witness_boundary),
                "path_action_count": evidence.actions.len(),
                "witness_turn_action_count": evidence.witness_turn_actions.len(),
                "typed_features": typed_combat_feature_components(&evidence.position),
            })
        })
        .collect::<Vec<_>>();
    let one_turn_loss_evidence = search
        .one_turn_loss_evidence()
        .iter()
        .map(|evidence| {
            json!({
                "proof": "ExhaustiveRefutation",
                "horizon": "BeforeNextPlayerTurn",
                "exact_state_hash": evidence.exact_state_hash,
                "player_turn": evidence.position.combat.turn.turn_count,
                "player_hp": evidence.position.combat.entities.player.current_hp,
                "terminal_loss_turn_options": evidence.terminal_loss_turn_options,
                "path_action_count": evidence.actions.len(),
                "typed_features": typed_combat_feature_components(&evidence.position),
            })
        })
        .collect::<Vec<_>>();
    let watched_states = watch_state_hash
        .iter()
        .map(|hash| search.state_membership_by_exact_hash(hash))
        .collect::<Vec<_>>();
    let watched_corridor_report = watched_corridor
        .as_ref()
        .map(|corridor| corridor.diagnostic_report(&search));
    let watched_state = (watched_states.len() == 1)
        .then(|| watched_states.first().cloned())
        .flatten();
    let witness = report.witness.as_ref();
    if let (Some(path), Some(witness)) = (export_witness_actions.as_ref(), witness) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        let actions = witness
            .actions
            .iter()
            .map(|action| action.input.clone())
            .collect::<Vec<_>>();
        let bytes = serde_json::to_vec_pretty(&actions).map_err(|error| error.to_string())?;
        std::fs::write(path, bytes).map_err(|error| error.to_string())?;
    }
    if !full && !readable {
        let summary_elapsed = summary_started.elapsed();
        return print_json(&serde_json::json!({
            "schema_name": "OracleCombatCaseCompactV1",
            "schema_version": 1,
            "case": case_path,
            "runtime": oracle_lab_runtime_identity(),
            "mode": {
                "v2_donor_enabled": !without_v2_donor,
                "scheduler": if anchor_only { "anchor_only" } else { "anchor_and_guides" },
                "action_imitation_artifact": action_imitation_artifact,
            },
            "status": format!("{:?}", report.status),
            "timing_ms": {
                "setup": started.duration_since(command_started).as_millis(),
                "search": search_elapsed.as_millis(),
                "summary": summary_elapsed.as_millis(),
                "total_before_print": command_started.elapsed().as_millis(),
            },
            "budget": {
                "generation_work": max_nodes,
                "wall_ms": wall_ms,
                "max_engine_steps_per_transition": max_engine_steps_per_transition,
            },
            "advisor": {
                "status": advisor_status,
                "nodes": advisor_nodes,
                "elapsed_ms": advisor_elapsed_ms,
            },
            "work": {
                "agenda_pops": report.after.agenda_pops,
                "generation_work": report.after.generation_work,
                "engine_steps": report.after.engine_steps,
                "exact_states": report.after.exact_states,
                "completed_turn_options": report.after.completed_turn_options,
                "applied_action_transitions": report.after.applied_action_transitions,
            },
            "frontier": {
                "retained_states": progress.retained_states,
                "anchor_entries": progress.queued_anchor_entries,
                "guide_queues": progress.guide_queues.iter().map(|queue| serde_json::json!({
                    "lane_id": queue.lane_id,
                    "lane": oracle_lab_guide_lane_label(queue.lane_id),
                    "entries": queue.entries,
                })).collect::<Vec<_>>(),
                "max_player_turn": progress.max_player_turn,
                "max_path_atomic_depth": progress.max_path_atomic_depth,
                "max_completed_turn_options_at_state": progress.max_completed_turn_options_at_state,
                "generation_gap_count": progress.generation_gap_count,
            },
            "root": progress.root_state,
            "deepest": {
                "survival": progress.deepest_survival_state,
                "progress": progress.deepest_progress_state,
            },
            "watched_state": watched_state,
            "watched_states": (watched_states.len() != 1).then_some(watched_states),
            "watched_corridor": compact_corridor_report(watched_corridor_report.as_ref()),
            "shadow_corridor": compact_corridor_report(shadow_corridor_report.as_ref()),
            "evidence": {
                "one_turn_viable": one_turn_viability_evidence,
                "one_turn_losses": one_turn_loss_evidence,
            },
            "exports": {
                "witness_actions": witness.is_some().then_some(export_witness_actions.as_ref()).flatten(),
                "augmented_value_prototype": export_augmented_value_prototype,
            },
            "witness": witness.map(|witness| serde_json::json!({
                "discovery_source": witness.discovery_source,
                "final_hp": witness.final_position.combat.entities.player.current_hp,
                "hp_loss": initial_hp.saturating_sub(witness.final_position.combat.entities.player.current_hp),
                "action_count": witness.actions.len(),
                "negative_log_policy": witness.negative_log_policy,
                "replay_engine_steps": witness.replay_engine_steps,
            })),
        }));
    }
    let prefix_trace = replay_combat_path(
        initial_position,
        &prefix_replay_actions,
        max_engine_steps_per_transition,
    )?;
    let deepest_progress_trace = replay_combat_path(
        search_root_position.clone(),
        &progress.deepest_progress_actions,
        max_engine_steps_per_transition,
    )?;
    let deepest_survival_trace =
        if progress.deepest_survival_actions == progress.deepest_progress_actions {
            serde_json::json!({"same_as": "deepest_progress_trace"})
        } else {
            replay_combat_path(
                search_root_position.clone(),
                &progress.deepest_survival_actions,
                max_engine_steps_per_transition,
            )?
        };
    let witness_trace = witness
        .map(|witness| {
            replay_combat_path(
                search_root_position.clone(),
                &witness.actions,
                max_engine_steps_per_transition,
            )
        })
        .transpose()?;
    if readable {
        return print_json(&serde_json::json!({
            "schema_name": "OracleCombatCaseReadableV1",
            "schema_version": 1,
            "v2_donor_enabled": !without_v2_donor,
            "action_imitation_artifact": action_imitation_artifact,
            "scheduler": if anchor_only { "anchor_only" } else { "anchor_and_guides" },
            "status": format!("{:?}", report.status),
            "elapsed_ms": started.elapsed().as_millis(),
            "budget": {
                "max_nodes": max_nodes,
                "wall_ms": wall_ms,
            },
            "advisor": {
                "status": advisor_status,
                "nodes": advisor_nodes,
                "elapsed_ms": advisor_elapsed_ms,
            },
            "shadow_corridor": shadow_corridor_report,
            "watched_corridor": watched_corridor_report,
            "one_turn_viability_evidence": one_turn_viability_evidence,
            "one_turn_loss_evidence": one_turn_loss_evidence,
            "exported_augmented_value_prototype": export_augmented_value_prototype,
            "exported_witness_actions": witness
                .is_some()
                .then_some(export_witness_actions.as_ref())
                .flatten(),
            "counters": {
                "agenda_pops": report.after.agenda_pops,
                "generation_work": report.after.generation_work,
                "exact_states": report.after.exact_states,
                "completed_turn_options": report.after.completed_turn_options,
                "exact_one_turn_viable_states": report.after.exact_one_turn_viable_states,
                "exhaustive_one_turn_losses": report.after.exhaustive_one_turn_losses,
            },
            "prefix": {
                "trace": prefix_trace,
                "successor_exact_state_hash": sts_oracle_runtime::ai::combat_state_key::combat_exact_state_hash_v2(
                    &search_root_position.engine,
                    &search_root_position.combat,
                ),
                "successor": combat_position_snapshot(&search_root_position),
            },
            "progress": {
                "max_player_turn": progress.max_player_turn,
                "deepest_survival_state": progress.deepest_survival_state,
                "deepest_survival_trace": deepest_survival_trace,
                "deepest_progress_state": progress.deepest_progress_state,
                "deepest_progress_trace": deepest_progress_trace,
                "recent_turn_survival_envelope": progress.recent_turn_survival_envelope,
                "max_completed_turn_options_at_state": progress.max_completed_turn_options_at_state,
                "generation_gap_count": progress.generation_gap_count,
                "watched_state": watched_state,
                "watched_states": watched_states,
            },
            "witness": witness.map(|witness| serde_json::json!({
                "discovery_source": witness.discovery_source,
                "final_hp": witness.final_position.combat.entities.player.current_hp,
                "hp_loss": initial_hp.saturating_sub(witness.final_position.combat.entities.player.current_hp),
                "trace": witness_trace,
            })),
        }));
    }
    print_json(&serde_json::json!({
        "schema_name": "OracleCombatCaseProbeV1",
        "schema_version": 1,
        "v2_donor_enabled": !without_v2_donor,
        "scheduler": if anchor_only { "anchor_only" } else { "anchor_and_guides" },
        "status": format!("{:?}", report.status),
        "elapsed_ms": started.elapsed().as_millis(),
        "budget": {
            "max_nodes": max_nodes,
            "wall_ms": wall_ms,
            "max_engine_steps_per_transition": max_engine_steps_per_transition,
        },
        "shadow_corridor": shadow_corridor_report,
        "watched_corridor": watched_corridor_report,
        "one_turn_viability_evidence": one_turn_viability_evidence,
        "one_turn_loss_evidence": one_turn_loss_evidence,
        "exported_augmented_value_prototype": export_augmented_value_prototype,
        "exported_witness_actions": witness
            .is_some()
            .then_some(export_witness_actions.as_ref())
            .flatten(),
        "advisor": {
            "status": advisor_status,
            "nodes": advisor_nodes,
            "elapsed_ms": advisor_elapsed_ms,
        },
        "prefix": {
            "action_count": prefix.len(),
            "actions": prefix,
            "trace": prefix_trace,
            "successor_exact_state_hash": sts_oracle_runtime::ai::combat_state_key::combat_exact_state_hash_v2(
                &search_root_position.engine,
                &search_root_position.combat,
            ),
            "successor": combat_position_snapshot(&search_root_position),
        },
        "counters": {
            "agenda_pops": report.after.agenda_pops,
            "generation_work": report.after.generation_work,
            "engine_steps": report.after.engine_steps,
            "exact_states": report.after.exact_states,
            "applied_action_transitions": report.after.applied_action_transitions,
            "unique_successor_states": report.after.unique_successor_states,
            "duplicate_exact_successors": report.after.duplicate_exact_successors,
            "completed_turn_options": report.after.completed_turn_options,
            "policy_witness_proposals": report.after.policy_witness_proposals,
            "exact_one_turn_viable_states": report.after.exact_one_turn_viable_states,
            "exhaustive_one_turn_losses": report.after.exhaustive_one_turn_losses,
        },
        "progress": {
            "retained_states": progress.retained_states,
            "queued_anchor_entries": progress.queued_anchor_entries,
            "queued_guided_entries": progress.queued_guided_entries,
            "max_player_turn": progress.max_player_turn,
            "deepest_survival_state": progress.deepest_survival_state,
            "deepest_survival_actions": progress.deepest_survival_actions,
            "deepest_survival_trace": deepest_survival_trace,
            "deepest_progress_state": progress.deepest_progress_state,
            "deepest_progress_actions": progress.deepest_progress_actions,
            "deepest_progress_trace": deepest_progress_trace,
            "recent_turn_survival_envelope": progress.recent_turn_survival_envelope,
            "max_path_atomic_depth": progress.max_path_atomic_depth,
            "max_completed_turn_options_at_state": progress.max_completed_turn_options_at_state,
            "generation_gap_count": progress.generation_gap_count,
            "root_state": progress.root_state,
            "watched_state": watched_state,
            "watched_states": watched_states,
        },
        "witness": witness.map(|witness| serde_json::json!({
            "discovery_source": witness.discovery_source,
            "final_hp": witness.final_position.combat.entities.player.current_hp,
            "hp_loss": initial_hp.saturating_sub(witness.final_position.combat.entities.player.current_hp),
            "action_count": witness.actions.len(),
            "negative_log_policy": witness.negative_log_policy,
            "actions": witness.actions,
        })),
    }))
}
