use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::engine::core::{is_smoke_escape_stable_boundary, tick_engine};
use crate::runtime::combat::CombatState;
use crate::sim::combat_action::CombatActionChoice;
use crate::state::core::{ClientInput, EngineState, RunResult};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct CombatPosition {
    pub engine: EngineState,
    pub combat: CombatState,
}

impl CombatPosition {
    pub fn new(engine: EngineState, combat: CombatState) -> Self {
        Self { engine, combat }
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatTerminal {
    Win,
    Loss,
    Unresolved,
}

#[derive(Clone, Copy, Debug)]
pub struct CombatStepLimits {
    pub max_engine_steps: usize,
    pub deadline: Option<Instant>,
}

#[derive(Clone, Debug)]
pub struct CombatStepResult {
    pub position: CombatPosition,
    pub terminal: CombatTerminal,
    pub alive: bool,
    pub truncated: bool,
    pub timed_out: bool,
    pub engine_steps: usize,
}

pub trait CombatStepper {
    fn legal_actions(&self, position: &CombatPosition) -> Vec<ClientInput>;

    fn legal_action_choices(&self, position: &CombatPosition) -> Vec<CombatActionChoice> {
        self.legal_actions(position)
            .into_iter()
            .map(|input| CombatActionChoice::from_input(&position.combat, input))
            .collect()
    }

    fn apply_to_stable(
        &self,
        position: &CombatPosition,
        input: ClientInput,
        limits: CombatStepLimits,
    ) -> CombatStepResult;

    fn terminal(&self, position: &CombatPosition) -> CombatTerminal;
}

#[derive(Clone, Copy, Debug, Default)]
pub struct EngineCombatStepper;

impl CombatStepper for EngineCombatStepper {
    fn legal_actions(&self, position: &CombatPosition) -> Vec<ClientInput> {
        crate::sim::combat_legal_actions::get_legal_moves(&position.engine, &position.combat)
    }

    fn apply_to_stable(
        &self,
        position: &CombatPosition,
        input: ClientInput,
        limits: CombatStepLimits,
    ) -> CombatStepResult {
        apply_combat_input_to_stable(position, input, limits)
    }

    fn terminal(&self, position: &CombatPosition) -> CombatTerminal {
        combat_terminal(&position.engine, &position.combat)
    }
}

pub fn combat_terminal(engine: &EngineState, combat: &CombatState) -> CombatTerminal {
    if matches!(engine, EngineState::GameOver(RunResult::Defeat))
        || combat.entities.player.current_hp <= 0
    {
        CombatTerminal::Loss
    } else if matches!(engine, EngineState::GameOver(RunResult::Victory))
        || combat_cleared(combat)
        || post_combat_engine_state(engine)
    {
        CombatTerminal::Win
    } else {
        CombatTerminal::Unresolved
    }
}

pub fn apply_combat_input_to_stable(
    position: &CombatPosition,
    input: ClientInput,
    limits: CombatStepLimits,
) -> CombatStepResult {
    if limits.deadline.is_some_and(|limit| Instant::now() >= limit) {
        return CombatStepResult {
            position: position.clone(),
            terminal: combat_terminal(&position.engine, &position.combat),
            alive: true,
            truncated: true,
            timed_out: true,
            engine_steps: 0,
        };
    }

    let mut engine = position.engine.clone();
    let mut combat = position.combat.clone();
    let mut steps = 1usize;
    let mut alive = tick_engine(&mut engine, &mut combat, Some(input));
    if !alive {
        mark_defeat_if_needed(&mut engine, &combat);
        return step_result(engine, combat, false, false, false, steps);
    }
    normalize_player_turn_processing(&mut engine, &combat);

    loop {
        if stable_boundary(&engine, &combat) {
            alive = !matches!(engine, EngineState::GameOver(_));
            return step_result(engine, combat, alive, false, false, steps);
        }
        if steps >= limits.max_engine_steps.max(1) {
            return step_result(engine, combat, true, true, false, steps);
        }
        if limits.deadline.is_some_and(|limit| Instant::now() >= limit) {
            return step_result(engine, combat, true, true, true, steps);
        }

        alive = tick_engine(&mut engine, &mut combat, None);
        steps = steps.saturating_add(1);
        if !alive {
            mark_defeat_if_needed(&mut engine, &combat);
            return step_result(engine, combat, false, false, false, steps);
        }
        normalize_player_turn_processing(&mut engine, &combat);
    }
}

pub fn stable_boundary(engine: &EngineState, combat: &CombatState) -> bool {
    match engine {
        EngineState::CombatPlayerTurn
        | EngineState::PendingChoice(_)
        | EngineState::GameOver(_) => true,
        EngineState::CombatProcessing if is_smoke_escape_stable_boundary(engine, combat) => true,
        EngineState::CombatProcessing => false,
        EngineState::CombatStart(_) => false,
        EngineState::RewardScreen(_)
        | EngineState::TreasureRoom(_)
        | EngineState::Campfire
        | EngineState::Shop(_)
        | EngineState::MapNavigation
        | EngineState::MapOverlay { .. }
        | EngineState::EventRoom
        | EngineState::RunPendingChoice(_)
        | EngineState::BossRelicSelect(_) => true,
    }
}

fn step_result(
    engine: EngineState,
    combat: CombatState,
    alive: bool,
    truncated: bool,
    timed_out: bool,
    engine_steps: usize,
) -> CombatStepResult {
    let terminal = combat_terminal(&engine, &combat);
    CombatStepResult {
        position: CombatPosition { engine, combat },
        terminal,
        alive,
        truncated,
        timed_out,
        engine_steps,
    }
}

fn normalize_player_turn_processing(engine: &mut EngineState, combat: &CombatState) {
    if *engine == EngineState::CombatPlayerTurn
        && (combat.has_pending_actions() || !combat.zones.queued_cards.is_empty())
    {
        *engine = EngineState::CombatProcessing;
    }
}

fn mark_defeat_if_needed(engine: &mut EngineState, combat: &CombatState) {
    if combat_terminal(engine, combat) == CombatTerminal::Unresolved {
        *engine = EngineState::GameOver(RunResult::Defeat);
    }
}

fn post_combat_engine_state(engine: &EngineState) -> bool {
    matches!(
        engine,
        EngineState::RewardScreen(_)
            | EngineState::TreasureRoom(_)
            | EngineState::Campfire
            | EngineState::Shop(_)
            | EngineState::MapNavigation
            | EngineState::EventRoom
            | EngineState::RunPendingChoice(_)
            | EngineState::BossRelicSelect(_)
    )
}

fn combat_cleared(combat: &CombatState) -> bool {
    combat.entities.monsters.iter().all(|monster| {
        monster.is_dying || monster.is_escaped || monster.half_dead || monster.current_hp <= 0
    })
}

#[cfg(test)]
mod tests {
    use super::{combat_terminal, stable_boundary, CombatTerminal};
    use crate::content::monsters::factory::EncounterId;
    use crate::sim::combat_start::build_natural_combat_start;
    use crate::state::core::{CombatStartRequest, EngineState, PostCombatReturn};
    use crate::state::map::node::RoomType;
    use crate::state::rewards::RewardState;
    use crate::state::run::RunState;

    #[test]
    fn combat_start_request_is_not_a_stable_search_boundary() {
        let mut run = RunState::new(1, 0, false, "Ironclad");
        let (_engine, combat) =
            build_natural_combat_start(&mut run, EncounterId::JawWorm, RoomType::MonsterRoom)
                .expect("combat should initialize");
        let event_engine = EngineState::CombatStart(CombatStartRequest::event(
            EncounterId::JawWorm,
            RewardState::new(),
            true,
            false,
            false,
            PostCombatReturn::MapNavigation,
        ));

        assert!(!stable_boundary(&event_engine, &combat));
        assert_eq!(
            combat_terminal(&event_engine, &combat),
            CombatTerminal::Unresolved
        );
    }
}
