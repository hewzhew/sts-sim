use crate::engine::core::tick_engine;
use crate::runtime::combat::CombatState;
use crate::state::core::{ClientInput, PendingChoice};
use crate::state::EngineState;
use std::time::Instant;

use super::profile::SearchProfileBreakdown;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum StableBoundaryKind {
    PlayerTurnReady,
    PendingChoice,
    PostCombat,
    GameOver,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub(super) struct StepAdvanceResult {
    pub(super) alive: bool,
    pub(super) truncated: bool,
    pub(super) timed_out: bool,
    pub(super) engine_steps: u32,
}

pub(super) fn tick_until_stable_turn_bounded(
    engine_state: &mut EngineState,
    combat_state: &mut CombatState,
    input: ClientInput,
    max_engine_steps: usize,
    deadline: Option<Instant>,
    profile: &mut SearchProfileBreakdown,
) -> StepAdvanceResult {
    let started = Instant::now();
    let step_budget = max_engine_steps.max(1);
    let mut steps = 0usize;

    if deadline.is_some_and(|limit| Instant::now() >= limit) {
        profile.note_timeout_source("engine_step_deadline");
        profile.record_engine_step_advance(started.elapsed().as_millis(), 0);
        return StepAdvanceResult {
            alive: true,
            truncated: true,
            timed_out: true,
            engine_steps: 0,
        };
    }

    let alive = tick_engine(engine_state, combat_state, Some(input));
    steps += 1;
    if !alive {
        profile.record_engine_step_advance(started.elapsed().as_millis(), steps as u32);
        return StepAdvanceResult {
            alive: false,
            truncated: false,
            timed_out: false,
            engine_steps: steps as u32,
        };
    }

    normalize_player_turn_processing(engine_state, combat_state);

    loop {
        if let Some(boundary) = stable_boundary_kind(engine_state) {
            profile.record_engine_step_advance(started.elapsed().as_millis(), steps as u32);
            return StepAdvanceResult {
                alive: !matches!(boundary, StableBoundaryKind::GameOver),
                truncated: false,
                timed_out: false,
                engine_steps: steps as u32,
            };
        }

        if steps >= step_budget {
            profile.note_timeout_source("engine_step_budget");
            profile.record_engine_step_advance(started.elapsed().as_millis(), steps as u32);
            return StepAdvanceResult {
                alive: true,
                truncated: true,
                timed_out: false,
                engine_steps: steps as u32,
            };
        }

        if deadline.is_some_and(|limit| Instant::now() >= limit) {
            profile.note_timeout_source("engine_step_deadline");
            profile.record_engine_step_advance(started.elapsed().as_millis(), steps as u32);
            return StepAdvanceResult {
                alive: true,
                truncated: true,
                timed_out: true,
                engine_steps: steps as u32,
            };
        }

        let alive = tick_engine(engine_state, combat_state, None);
        steps += 1;
        if !alive {
            profile.record_engine_step_advance(started.elapsed().as_millis(), steps as u32);
            return StepAdvanceResult {
                alive: false,
                truncated: false,
                timed_out: false,
                engine_steps: steps as u32,
            };
        }
        normalize_player_turn_processing(engine_state, combat_state);
    }
}

pub(super) fn simulate_input_bounded(
    engine: &EngineState,
    combat: &CombatState,
    input: &ClientInput,
    max_engine_steps: usize,
    deadline: Option<Instant>,
    profile: &mut SearchProfileBreakdown,
) -> (EngineState, CombatState, StepAdvanceResult) {
    let mut next_engine = engine.clone();
    let mut next_combat = combat.clone();
    let outcome = tick_until_stable_turn_bounded(
        &mut next_engine,
        &mut next_combat,
        input.clone(),
        max_engine_steps,
        deadline,
        profile,
    );
    (next_engine, next_combat, outcome)
}

pub(super) fn project_turn_close_state_bounded(
    engine: &EngineState,
    combat: &CombatState,
    max_engine_steps: usize,
    deadline: Option<Instant>,
    profile: &mut SearchProfileBreakdown,
) -> (EngineState, CombatState, StepAdvanceResult) {
    let started = Instant::now();
    let mut projected_engine = engine.clone();
    let mut projected_combat = combat.clone();
    let mut outcome = StepAdvanceResult {
        alive: true,
        ..StepAdvanceResult::default()
    };

    if matches!(projected_engine, EngineState::CombatPlayerTurn) {
        outcome = tick_until_stable_turn_bounded(
            &mut projected_engine,
            &mut projected_combat,
            ClientInput::EndTurn,
            max_engine_steps,
            deadline,
            profile,
        );
    }

    profile.record_projection_call(started.elapsed().as_millis());
    (projected_engine, projected_combat, outcome)
}

fn normalize_player_turn_processing(engine_state: &mut EngineState, combat_state: &CombatState) {
    if *engine_state == EngineState::CombatPlayerTurn
        && (combat_state.has_pending_actions() || !combat_state.zones.queued_cards.is_empty())
    {
        *engine_state = EngineState::CombatProcessing;
    }
}

fn stable_boundary_kind(engine_state: &EngineState) -> Option<StableBoundaryKind> {
    match engine_state {
        EngineState::CombatPlayerTurn => Some(StableBoundaryKind::PlayerTurnReady),
        EngineState::PendingChoice(choice) if pending_choice_is_same_turn_frontier(choice) => {
            Some(StableBoundaryKind::PendingChoice)
        }
        EngineState::RewardScreen(_)
        | EngineState::Campfire
        | EngineState::Shop(_)
        | EngineState::MapNavigation
        | EngineState::EventRoom
        | EngineState::RunPendingChoice(_)
        | EngineState::EventCombat(_)
        | EngineState::BossRelicSelect(_) => Some(StableBoundaryKind::PostCombat),
        EngineState::GameOver(_) => Some(StableBoundaryKind::GameOver),
        EngineState::CombatProcessing | EngineState::PendingChoice(_) => None,
    }
}

pub(super) fn pending_choice_is_same_turn_frontier(choice: &PendingChoice) -> bool {
    match choice {
        PendingChoice::GridSelect { .. }
        | PendingChoice::HandSelect { .. }
        | PendingChoice::DiscoverySelect(_)
        | PendingChoice::ScrySelect { .. }
        | PendingChoice::StanceChoice
        | PendingChoice::CardRewardSelect { .. } => true,
    }
}
