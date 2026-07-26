use super::*;

#[derive(Debug, Args)]
pub(super) struct CombatCaseFoldSolvedSuffixArgs {
    #[arg(long)]
    case: PathBuf,
    #[arg(long)]
    corridor_actions: PathBuf,
    #[arg(long)]
    solved_suffix_actions: PathBuf,
    #[arg(long)]
    solved_suffix_start_turn: usize,
    #[arg(long, default_value_t = 8_192)]
    max_generation_work_per_fold: usize,
    #[arg(long, default_value_t = 5_000)]
    wall_ms_per_fold: u64,
    #[arg(long, default_value_t = 250)]
    max_engine_steps_per_transition: usize,
    #[arg(long, default_value_t = 256)]
    beam_width: usize,
    #[arg(long, default_value_t = 32)]
    retained_per_view: usize,
    #[arg(long, default_value_t = 8)]
    generation_quantum_work: usize,
    #[arg(long)]
    export_witness_actions: Option<PathBuf>,
}

pub(super) fn run(args: CombatCaseFoldSolvedSuffixArgs) -> Result<(), String> {
    let CombatCaseFoldSolvedSuffixArgs {
        case,
        corridor_actions,
        solved_suffix_actions,
        solved_suffix_start_turn,
        max_generation_work_per_fold,
        wall_ms_per_fold,
        max_engine_steps_per_transition,
        beam_width,
        retained_per_view,
        generation_quantum_work,
        export_witness_actions,
    } = args;
    let command_started = Instant::now();
    let corridor = load_exact_turn_corridor(
        &case,
        std::slice::from_ref(&corridor_actions),
        max_engine_steps_per_transition,
    )?;
    if solved_suffix_start_turn >= corridor.positions_by_rank.len() {
        return Err(format!(
            "solved suffix starts at turn-boundary index {solved_suffix_start_turn}, but the corridor exposes only {} boundary states",
            corridor.positions_by_rank.len()
        ));
    }
    let seed_inputs = load_combat_action_segments(std::slice::from_ref(&solved_suffix_actions))?;
    let policy = existing_combat_knowledge_policy_v1();
    let report = fold_verified_suffix_through_turn_predecessors(
        &corridor.positions_by_rank[..=solved_suffix_start_turn],
        seed_inputs,
        SolvedSuffixFoldConfig {
            search: LayeredCombatWitnessConfig {
                generator: TurnOptionGeneratorConfig {
                    max_engine_steps_per_transition,
                    ..TurnOptionGeneratorConfig::default()
                },
                beam_width: beam_width.max(1),
                retained_per_view: retained_per_view.max(1),
                generation_quantum_work: generation_quantum_work.max(1),
                max_turn_layers: 1,
            },
            max_generation_work_per_fold: max_generation_work_per_fold.max(1),
            max_engine_steps_per_transition: max_engine_steps_per_transition.max(1),
            wall_time_per_fold: Some(Duration::from_millis(wall_ms_per_fold.max(1))),
        },
        policy,
        &EngineCombatStepper,
    )
    .map_err(|error| format!("solved suffix fold failed: {error:?}"))?;
    let root_witness_inputs = report.witness.as_ref().map(|witness| {
        witness
            .actions
            .iter()
            .map(|action| action.input.clone())
            .collect::<Vec<_>>()
    });
    let root_final_hp = report
        .witness
        .as_ref()
        .map(|witness| witness.final_position.combat.entities.player.current_hp);
    if let (Some(path), Some(inputs)) = (
        export_witness_actions.as_ref(),
        root_witness_inputs.as_ref(),
    ) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        std::fs::write(
            path,
            serde_json::to_vec_pretty(inputs).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
    }
    let fold_reports = report
        .steps
        .iter()
        .map(|step| {
            json!({
                "predecessor_turn_index": step.predecessor_index,
                "status": format!("{:?}", step.status),
                "elapsed_ms": step.elapsed.as_millis(),
                "generation_work": step.counters.generation_work,
                "engine_steps": step.counters.engine_steps,
                "solved_suffix_matches": step.counters.solved_suffix_matches,
                "action_count": step.action_count,
                "final_hp": step.final_hp,
            })
        })
        .collect::<Vec<_>>();
    let overall_status = match report.status {
        SolvedSuffixFoldStatus::WitnessFound => "WitnessFound",
        SolvedSuffixFoldStatus::Partial { .. } => "Partial",
    };
    print_json(&json!({
        "schema_name": "OracleCombatSolvedSuffixFoldV1",
        "schema_version": 1,
        "case": case,
        "runtime": oracle_lab_runtime_identity(),
        "status": overall_status,
        "mode": {
            "search": "exact_predecessor_proof_folding",
            "corridor_is_search_guidance": false,
            "v2_donor_enabled": false,
        },
        "budget": {
            "max_generation_work_per_fold": max_generation_work_per_fold,
            "wall_ms_per_fold": wall_ms_per_fold,
            "solved_suffix_start_turn": solved_suffix_start_turn,
        },
        "folds": fold_reports,
        "solved_suffix_count": report.solved_suffix_count,
        "elapsed_ms": command_started.elapsed().as_millis(),
        "exported_witness_actions": export_witness_actions,
        "witness": root_witness_inputs.as_ref().map(|inputs| json!({
            "action_count": inputs.len(),
            "final_hp": root_final_hp,
        })),
    }))
}
