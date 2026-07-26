use super::*;

#[derive(Debug, Args)]
pub(super) struct TurnMembershipArgs {
    #[arg(long)]
    case: PathBuf,
    #[arg(
        long,
        required_unless_present = "corridor_actions",
        conflicts_with = "corridor_actions"
    )]
    actions: Option<PathBuf>,
    /// One or more consecutive exact action segments forming a complete
    /// verified witness. Repeat the flag instead of hand-splicing JSON.
    #[arg(long, required_unless_present = "actions", requires = "corridor_rank")]
    corridor_actions: Vec<PathBuf>,
    /// Zero-based player-turn boundary in --corridor-actions. The last
    /// boundary checks the terminal winning segment.
    #[arg(long, requires = "corridor_actions")]
    corridor_rank: Option<usize>,
    #[arg(long, default_value_t = 100_000)]
    max_work: usize,
    #[arg(long, default_value_t = 5_000)]
    wall_ms: u64,
    #[arg(long, default_value_t = 8)]
    quantum_work: usize,
    #[arg(long, default_value_t = 250)]
    max_engine_steps_per_transition: usize,
    /// Lab-only control: keep action weights but disable all state guides.
    #[arg(long)]
    anchor_only: bool,
    /// Include every target-prefix queue snapshot. By default the report
    /// stays compact and includes only the last reached and first missing
    /// prefixes.
    #[arg(long)]
    full: bool,
}

pub(super) fn run(args: TurnMembershipArgs) -> Result<(), String> {
    let TurnMembershipArgs {
        case,
        actions,
        corridor_actions,
        corridor_rank,
        max_work,
        wall_ms,
        quantum_work,
        max_engine_steps_per_transition,
        anchor_only,
        full,
    } = args;
    let (root_position, target, selected_corridor_rank) =
        match (actions.as_ref(), corridor_actions.as_slice(), corridor_rank) {
            (Some(actions), [], None) => {
                let case = load_combat_case(&case)?;
                let target = serde_json::from_slice::<Vec<ClientInput>>(
                    &std::fs::read(actions).map_err(|error| error.to_string())?,
                )
                .map_err(|error| format!("invalid target action list: {error}"))?;
                (case.position, target, None)
            }
            (None, corridor_actions, Some(rank)) if !corridor_actions.is_empty() => {
                let corridor = load_exact_turn_corridor(
                    &case,
                    corridor_actions,
                    max_engine_steps_per_transition,
                )?;
                let root_position =
                    corridor
                        .positions_by_rank
                        .get(rank)
                        .cloned()
                        .ok_or_else(|| {
                            format!(
                                "corridor rank {rank} is out of range 0..{}",
                                corridor.positions_by_rank.len()
                            )
                        })?;
                let target = corridor
                    .transition_actions
                    .get(rank)
                    .cloned()
                    .expect("verified corridor has one transition per boundary");
                (root_position, target, Some(rank))
            }
            _ => unreachable!("clap selects either actions or corridor rank"),
        };
    let (target_policy_trace, target_successor_exact_state_hash, target_prefix_positions) =
        target_atomic_policy_trace(&root_position, &target, max_engine_steps_per_transition)?;
    let root = CombatDecisionRoot::new(root_position)
        .map_err(|error| format!("invalid combat case root: {error:?}"))?;
    let policy = existing_combat_knowledge_policy_v1();
    let policy = if anchor_only {
        anchor_only_policy(policy)
    } else {
        policy
    };
    let mut generator = TurnOptionGeneratorSession::with_policy(
        root,
        TurnOptionGeneratorConfig {
            max_engine_steps_per_transition,
            ..TurnOptionGeneratorConfig::default()
        },
        policy,
    );
    let started = Instant::now();
    let deadline = started + Duration::from_millis(wall_ms);
    let mut scanned_options = 0usize;
    let mut matched = None;
    let mut prefix_insertions = vec![None; target_prefix_positions.len()];
    let mut transition_insertions = vec![None; target_prefix_positions.len()];
    let mut last_status = TurnOptionGenerationStatus::Partial(
        sts_combat_planner::GenerationInterruption::GenerationWorkBudget,
    );
    while generator.counters().generation_work < max_work
        && !generator.is_finished()
        && Instant::now() < deadline
    {
        let remaining = max_work.saturating_sub(generator.counters().generation_work);
        let work = quantum_work.max(1).min(remaining);
        let report = generator.advance(
            &EngineCombatStepper,
            CombatPlanningQuantum {
                additional_generation_work: work,
                additional_engine_steps: work.saturating_mul(max_engine_steps_per_transition),
                deadline: Some(deadline),
            },
        );
        last_status = report.status;
        for (index, position) in target_prefix_positions.iter().enumerate() {
            if prefix_insertions[index].is_none() && generator.has_seen_exact_position(position) {
                let anchor_rank = generator
                    .live_expand_queue_ranks_at_exact_position(position)
                    .map(|(anchor, _)| anchor);
                prefix_insertions[index] = Some((
                    report.after.generation_work,
                    generator.anchor_work_pops(),
                    anchor_rank,
                ));
            }
            if transition_insertions[index].is_none() {
                transition_insertions[index] = target
                    .get(index + 1)
                    .and_then(|next| generator.live_action_transition_snapshot(position, next))
                    .map(|snapshot| {
                        serde_json::json!({
                            "generation_work": report.after.generation_work,
                            "candidate_ordinal": snapshot.candidate_ordinal,
                            "remaining_candidate_count": snapshot.remaining_candidate_count,
                            "conditional_probability": snapshot.conditional_probability,
                            "candidate_negative_log_policy": snapshot.candidate_negative_log_policy,
                            "cursor_negative_log_policy": snapshot.cursor_negative_log_policy,
                            "anchor_queue_rank": snapshot.anchor_queue_rank,
                            "guide_queue_ranks": snapshot.guide_queue_ranks,
                        })
                    });
            }
        }
        for option in &generator.completed_options()[scanned_options..] {
            let exact_action_match = option.actions().len() == target.len()
                && option
                    .actions()
                    .iter()
                    .zip(&target)
                    .all(|(actual, expected)| actual.input == *expected);
            let equivalent_successor_match =
                option.exact_successor_hash() == target_successor_exact_state_hash;
            if exact_action_match || equivalent_successor_match {
                matched = Some(serde_json::json!({
                    "match_kind": if exact_action_match { "exact_actions" } else { "equivalent_exact_successor" },
                    "exact_action_match": exact_action_match,
                    "equivalent_successor_match": equivalent_successor_match,
                    "generation_work": report.after.generation_work,
                    "engine_steps": report.after.engine_steps,
                    "elapsed_ms": started.elapsed().as_millis(),
                    "boundary": format!("{:?}", option.boundary()),
                    "successor_exact_state_hash": option.exact_successor_hash(),
                    "negative_log_policy": option.negative_log_policy(),
                }));
                break;
            }
        }
        scanned_options = generator.completed_options().len();
        if matched.is_some() {
            break;
        }
    }
    let counters = generator.counters();
    let target_prefix_membership = target_prefix_positions
        .iter()
        .enumerate()
        .map(|(index, position)| {
            let insertion =
                prefix_insertions[index].map(|(generation_work, anchor_pops, anchor_rank)| {
                    serde_json::json!({
                        "generation_work": generation_work,
                        "anchor_pops": anchor_pops,
                        "anchor_rank": anchor_rank,
                        "anchor_pops_since": generator
                            .anchor_work_pops()
                            .saturating_sub(anchor_pops),
                    })
                });
            let (live_expand, live_apply_action, live_structured_selection) =
                generator.live_work_counts_at_exact_position(position);
            let queue_ranks = generator
                .live_expand_queue_ranks_at_exact_position(position)
                .map(|(anchor, guides)| {
                    serde_json::json!({
                        "anchor": anchor,
                        "guides": guides,
                    })
                });
            let next_target_transition = target
                .get(index + 1)
                .and_then(|next| generator.live_action_transition_snapshot(position, next));
            serde_json::json!({
                "through_action": index + 1,
                "exact_state_hash": sts_simulator::ai::combat_state_key::combat_exact_state_hash_v1(
                    &position.engine,
                    &position.combat,
                ),
                "seen": generator.has_seen_exact_position(position),
                "first_observed": insertion,
                "live_work": {
                    "expand": live_expand,
                    "apply_action": live_apply_action,
                    "structured_selection": live_structured_selection,
                },
                "live_expand_queue_ranks": queue_ranks,
                "next_target_transition_live": next_target_transition.is_some(),
                "next_target_transition_first_observed": transition_insertions[index],
                "next_target_transition": next_target_transition.map(|snapshot| serde_json::json!({
                    "candidate_ordinal": snapshot.candidate_ordinal,
                    "remaining_candidate_count": snapshot.remaining_candidate_count,
                    "conditional_probability": snapshot.conditional_probability,
                    "candidate_negative_log_policy": snapshot.candidate_negative_log_policy,
                    "cursor_negative_log_policy": snapshot.cursor_negative_log_policy,
                    "anchor_queue_rank": snapshot.anchor_queue_rank,
                    "guide_queue_ranks": snapshot.guide_queue_ranks,
                })),
            })
        })
        .collect::<Vec<_>>();
    let last_reached_prefix = target_prefix_membership
        .iter()
        .rev()
        .find(|prefix| prefix.get("seen").and_then(serde_json::Value::as_bool) == Some(true))
        .cloned();
    let first_missing_prefix = target_prefix_membership
        .iter()
        .find(|prefix| prefix.get("seen").and_then(serde_json::Value::as_bool) == Some(false))
        .cloned();
    let mut output = serde_json::json!({
        "schema_name": "OracleTurnMembershipProbeV1",
        "schema_version": 1,
        "scheduler": if anchor_only { "anchor_only" } else { "anchor_and_guides" },
        "matched": matched.is_some(),
        "match": matched,
        "target_action_count": target.len(),
        "corridor_rank": selected_corridor_rank,
        "target_successor_exact_state_hash": target_successor_exact_state_hash,
        "target_policy_trace": target_policy_trace,
        "last_reached_prefix": last_reached_prefix,
        "first_missing_prefix": first_missing_prefix,
        "status": format!("{:?}", last_status),
        "elapsed_ms": started.elapsed().as_millis(),
        "generation_work": counters.generation_work,
        "engine_steps": counters.engine_steps,
        "scheduler_counters": {
            "atomic_state_expansions": generator.atomic_state_expansions(),
            "anchor_work_pops": generator.anchor_work_pops(),
            "guided_work_pops": generator.guided_work_pops(),
            "applied_action_transitions": generator.diagnostics().applied_action_transitions,
        },
        "completed_turn_options": generator.completed_options().len(),
        "retained_work_items": generator.retained_work_items(),
        "finished": generator.is_finished(),
    });
    if full {
        output
            .as_object_mut()
            .expect("membership report must be an object")
            .insert(
                "target_prefix_membership".to_owned(),
                serde_json::Value::Array(target_prefix_membership),
            );
    }
    print_json(&output)
}
