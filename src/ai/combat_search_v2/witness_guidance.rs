use std::collections::HashMap;

use serde::Serialize;

use crate::runtime::combat::CombatState;
use crate::sim::combat::{
    CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal, EngineCombatStepper,
};
use crate::state::core::{ClientInput, EngineState};

use super::{
    combat_exact_state_hash_v1, living_enemy_count, CombatSearchV2ActionPreview,
    CombatSearchV2RootActionPrior, SearchTerminalLabel,
};

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2WitnessLine {
    pub source: &'static str,
    pub terminal: SearchTerminalLabel,
    pub final_hp: i32,
    pub total_enemy_hp: i32,
    pub action_count: Option<usize>,
    pub actions: Vec<CombatSearchV2ActionPreview>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2WitnessReplay {
    pub action_count: Option<usize>,
    pub replayed_actions: usize,
    pub truncated_by_preview_limit: bool,
    pub terminal: CombatTerminal,
    pub final_hp: i32,
    pub total_enemy_hp: i32,
    pub living_enemy_count: usize,
    pub truncated: bool,
    pub timed_out: bool,
    pub matched_witness_terminal: bool,
    pub matched_witness_final_hp: bool,
    pub matched_witness_enemy_hp: bool,
    pub steps: Vec<CombatSearchV2WitnessReplayStep>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2WitnessReplayStep {
    pub step_index: usize,
    pub action_key: String,
    pub input: ClientInput,
    pub terminal: CombatTerminal,
    pub truncated: bool,
    pub timed_out: bool,
    pub engine_steps: usize,
    pub engine_state: String,
    pub final_hp: i32,
    pub total_enemy_hp: i32,
    pub living_enemy_count: usize,
}

#[derive(Clone, Debug)]
pub struct CombatSearchV2WitnessPrior {
    pub prior: CombatSearchV2RootActionPrior,
    pub prior_states: usize,
    pub duplicate_prior_hints: usize,
}

pub fn replay_combat_search_witness_line_v0(
    root: &CombatPosition,
    witness: &CombatSearchV2WitnessLine,
) -> CombatSearchV2WitnessReplay {
    let stepper = EngineCombatStepper;
    let mut position = root.clone();
    let mut steps = Vec::new();
    let mut truncated = false;
    let mut timed_out = false;
    for (index, action) in witness.actions.iter().cloned().enumerate() {
        if stepper.terminal(&position) != CombatTerminal::Unresolved {
            break;
        }
        let step = stepper.apply_to_stable(
            &position,
            action.input.clone(),
            CombatStepLimits {
                max_engine_steps: 250,
                deadline: None,
            },
        );
        truncated |= step.truncated;
        timed_out |= step.timed_out;
        steps.push(witness_replay_step(index + 1, action, &step));
        position = step.position;
        if truncated || timed_out || step.terminal != CombatTerminal::Unresolved {
            break;
        }
    }

    let terminal = stepper.terminal(&position);
    let final_hp = position.combat.entities.player.current_hp;
    let final_enemy_hp = total_enemy_hp(&position.combat);
    CombatSearchV2WitnessReplay {
        action_count: witness.action_count,
        replayed_actions: steps.len(),
        truncated_by_preview_limit: witness
            .action_count
            .is_some_and(|count| count > witness.actions.len()),
        terminal,
        final_hp,
        total_enemy_hp: final_enemy_hp,
        living_enemy_count: living_enemy_count(&position.combat),
        truncated,
        timed_out,
        matched_witness_terminal: terminal_matches(terminal, witness.terminal),
        matched_witness_final_hp: final_hp == witness.final_hp,
        matched_witness_enemy_hp: final_enemy_hp == witness.total_enemy_hp,
        steps,
    }
}

pub fn compile_combat_search_witness_prior_v0(
    root: &CombatPosition,
    witness: &CombatSearchV2WitnessLine,
) -> CombatSearchV2WitnessPrior {
    let stepper = EngineCombatStepper;
    let mut position = root.clone();
    let mut scores_by_state: HashMap<String, HashMap<String, f64>> = HashMap::new();
    let mut duplicate_prior_hints = 0usize;
    for action in witness.actions.iter().cloned() {
        if stepper.terminal(&position) != CombatTerminal::Unresolved {
            break;
        }
        let state_hash = combat_exact_state_hash_v1(&position.engine, &position.combat);
        let state_scores = scores_by_state.entry(state_hash).or_default();
        if state_scores.insert(action.action_key, 1.0).is_some() {
            duplicate_prior_hints = duplicate_prior_hints.saturating_add(1);
        }
        let step = stepper.apply_to_stable(
            &position,
            action.input,
            CombatStepLimits {
                max_engine_steps: 250,
                deadline: None,
            },
        );
        position = step.position;
        if step.truncated || step.timed_out || step.terminal != CombatTerminal::Unresolved {
            break;
        }
    }
    let prior_states = scores_by_state.len();
    CombatSearchV2WitnessPrior {
        prior: CombatSearchV2RootActionPrior::from_scores_with_duplicate_count(
            scores_by_state,
            duplicate_prior_hints,
        ),
        prior_states,
        duplicate_prior_hints,
    }
}

fn witness_replay_step(
    step_index: usize,
    action: CombatSearchV2ActionPreview,
    step: &crate::sim::combat::CombatStepResult,
) -> CombatSearchV2WitnessReplayStep {
    let combat = &step.position.combat;
    CombatSearchV2WitnessReplayStep {
        step_index,
        action_key: action.action_key,
        input: action.input,
        terminal: step.terminal,
        truncated: step.truncated,
        timed_out: step.timed_out,
        engine_steps: step.engine_steps,
        engine_state: engine_state_label(&step.position.engine),
        final_hp: combat.entities.player.current_hp,
        total_enemy_hp: total_enemy_hp(combat),
        living_enemy_count: living_enemy_count(combat),
    }
}

fn terminal_matches(actual: CombatTerminal, expected: SearchTerminalLabel) -> bool {
    matches!(
        (actual, expected),
        (CombatTerminal::Win, SearchTerminalLabel::Win)
            | (CombatTerminal::Loss, SearchTerminalLabel::Loss)
            | (CombatTerminal::Unresolved, SearchTerminalLabel::Unresolved)
    )
}

fn engine_state_label(engine: &EngineState) -> String {
    match engine {
        EngineState::CombatPlayerTurn => "CombatPlayerTurn".to_string(),
        EngineState::CombatStart(_) => "CombatStart".to_string(),
        EngineState::CombatProcessing => "CombatProcessing".to_string(),
        EngineState::PendingChoice(choice) => format!("PendingChoice({choice:?})"),
        other => format!("{other:?}"),
    }
}

fn total_enemy_hp(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .map(|monster| monster.current_hp.max(0) + monster.block.max(0))
        .sum()
}
