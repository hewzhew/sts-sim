use super::combat_trace_view::combat_action_label;
use super::*;

#[derive(Debug, Args)]
pub(super) struct CombatCasePlanAnnotationsArgs {
    #[arg(long)]
    case: PathBuf,
    #[arg(long, default_value_t = 250)]
    max_engine_steps_per_transition: usize,
}

pub(super) fn run_annotations(args: CombatCasePlanAnnotationsArgs) -> Result<(), String> {
    let CombatCasePlanAnnotationsArgs {
        case,
        max_engine_steps_per_transition,
    } = args;
    let case_path = case.clone();
    let loaded = load_combat_case(&case)?;
    let position = loaded.position;
    let stepper = EngineCombatStepper;
    let surface = stepper.legal_action_surface(&position);
    let root_plan = awakened_one_combat_plan_v1(&position);
    let annotations = surface
        .atomic_actions
        .iter()
        .map(|input| {
            let step = stepper.apply_to_stable(
                &position,
                input.clone(),
                CombatStepLimits {
                    max_engine_steps: max_engine_steps_per_transition,
                    deadline: None,
                },
            );
            let exact_successor_hash = (!step.truncated)
                .then(|| combat_exact_state_hash_v2(&step.position.engine, &step.position.combat));
            let transition = (!step.truncated)
                .then(|| awakened_one_plan_transition_v1(&position, &step.position))
                .flatten();
            let successor_plan = (!step.truncated)
                .then(|| awakened_one_combat_plan_v1(&step.position))
                .flatten();
            json!({
                "label": combat_action_label(&position, input),
                "action_key": combat_action_key(&position.combat, input),
                "input": input,
                "engine_steps": step.engine_steps,
                "truncated": step.truncated,
                "timed_out": step.timed_out,
                "terminal": format!("{:?}", step.terminal),
                "exact_successor_hash": exact_successor_hash,
                "plan_transition": transition,
                "successor_plan": successor_plan,
            })
        })
        .collect::<Vec<_>>();
    print_json(&json!({
        "schema_name": "OracleCombatCasePlanAnnotationsV1",
        "schema_version": 1,
        "case": case_path,
        "runtime": oracle_lab_runtime_identity(),
        "contract": {
            "search": false,
            "policy_mutation": false,
            "ranking": false,
            "pruning": false,
            "terminal_truth": "exact_simulator_only",
        },
        "root_exact_state_hash": combat_exact_state_hash_v2(
            &position.engine,
            &position.combat,
        ),
        "root_plan": root_plan,
        "surface": {
            "atomic_action_count": surface.atomic_actions.len(),
            "structured_family_count": surface.selection_families.len(),
            "complete": surface.selection_families.is_empty(),
            "structured_families_unannotated": !surface.selection_families.is_empty(),
        },
        "max_engine_steps_per_transition": max_engine_steps_per_transition,
        "annotations": annotations,
    }))
}

#[derive(Debug, Args)]
pub(super) struct CombatCasePlanTraceArgs {
    #[arg(long)]
    case: PathBuf,
    /// Repeat to compose several exact action segments in order.
    #[arg(long, required = true)]
    actions: Vec<PathBuf>,
    #[arg(long, default_value_t = 250)]
    max_engine_steps_per_transition: usize,
}

pub(super) fn run_trace(args: CombatCasePlanTraceArgs) -> Result<(), String> {
    let CombatCasePlanTraceArgs {
        case,
        actions,
        max_engine_steps_per_transition,
    } = args;
    let case_path = case.clone();
    let action_paths = actions.clone();
    let loaded = load_combat_case(&case)?;
    let inputs = load_combat_action_segments(&actions)?;
    let input_count = inputs.len();
    let stepper = EngineCombatStepper;
    let mut position = loaded.position;
    let root_exact_state_hash = combat_exact_state_hash_v2(&position.engine, &position.combat);
    let root_plan = awakened_one_combat_plan_v1(&position);
    let mut trace = Vec::new();
    let mut consumed_actions = 0_usize;

    for (index, input) in inputs.into_iter().enumerate() {
        let before_hash = combat_exact_state_hash_v2(&position.engine, &position.combat);
        let label = combat_action_label(&position, &input);
        let action_key = combat_action_key(&position.combat, &input);
        let step = stepper.apply_to_stable(
            &position,
            input.clone(),
            CombatStepLimits {
                max_engine_steps: max_engine_steps_per_transition,
                deadline: None,
            },
        );
        let transition = (!step.truncated)
            .then(|| awakened_one_plan_transition_v1(&position, &step.position))
            .flatten();
        let successor_plan = (!step.truncated)
            .then(|| awakened_one_combat_plan_v1(&step.position))
            .flatten();
        let after_hash = (!step.truncated)
            .then(|| combat_exact_state_hash_v2(&step.position.engine, &step.position.combat));
        trace.push(json!({
            "action_index": index,
            "label": label,
            "action_key": action_key,
            "input": input,
            "before_exact_state_hash": before_hash,
            "after_exact_state_hash": after_hash,
            "engine_steps": step.engine_steps,
            "truncated": step.truncated,
            "timed_out": step.timed_out,
            "terminal": format!("{:?}", step.terminal),
            "plan_transition": transition,
            "successor_plan": successor_plan,
        }));
        consumed_actions = consumed_actions.saturating_add(1);
        position = step.position;
        if step.truncated || step.terminal != CombatTerminal::Unresolved {
            break;
        }
    }

    let final_terminal = combat_terminal(&position.engine, &position.combat);
    print_json(&json!({
        "schema_name": "OracleCombatCasePlanTraceV1",
        "schema_version": 1,
        "case": case_path,
        "actions": action_paths,
        "runtime": oracle_lab_runtime_identity(),
        "contract": {
            "search": false,
            "policy_mutation": false,
            "ranking": false,
            "pruning": false,
            "caller_supplied_actions": true,
            "terminal_truth": "exact_simulator_only",
        },
        "root_exact_state_hash": root_exact_state_hash,
        "root_plan": root_plan,
        "input_action_count": input_count,
        "consumed_action_count": consumed_actions,
        "unconsumed_action_count": input_count.saturating_sub(consumed_actions),
        "final_exact_state_hash": combat_exact_state_hash_v2(
            &position.engine,
            &position.combat,
        ),
        "final_terminal": format!("{final_terminal:?}"),
        "final_player_hp": position.combat.entities.player.current_hp,
        "final_plan": awakened_one_combat_plan_v1(&position),
        "max_engine_steps_per_transition": max_engine_steps_per_transition,
        "trace": trace,
    }))
}
