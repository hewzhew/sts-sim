use std::time::Instant;

use serde::{Deserialize, Serialize};

use crate::engine::core::{is_smoke_escape_stable_boundary, tick_engine};
use crate::runtime::combat::CombatState;
use crate::sim::combat_action::CombatActionChoice;
use crate::state::core::{ClientInput, EngineState, RunResult};
use crate::state::DomainCardSnapshot;

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

#[derive(Clone, Debug)]
pub struct CombatObservedStepResultV1 {
    pub step: CombatStepResult,
    pub drawn_cards: Vec<DomainCardSnapshot>,
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
        || combat.are_monsters_basically_dead_java()
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
    apply_combat_input_to_stable_inner(position, input, limits, false).step
}

pub fn apply_combat_input_to_stable_observed_v1(
    position: &CombatPosition,
    input: ClientInput,
    limits: CombatStepLimits,
) -> CombatObservedStepResultV1 {
    let result = apply_combat_input_to_stable_inner(position, input, limits, true);
    CombatObservedStepResultV1 {
        step: result.step,
        drawn_cards: result
            .drawn_cards
            .expect("observed combat steps must return draw evidence"),
    }
}

struct CombatStepResultInternal {
    step: CombatStepResult,
    drawn_cards: Option<Vec<DomainCardSnapshot>>,
}

fn apply_combat_input_to_stable_inner(
    position: &CombatPosition,
    input: ClientInput,
    limits: CombatStepLimits,
    observe_draws: bool,
) -> CombatStepResultInternal {
    let mut engine = position.engine.clone();
    let mut combat = position.combat.clone();
    combat.clear_card_draw_observation_events();

    if limits.deadline.is_some_and(|limit| Instant::now() >= limit) {
        return step_result(engine, combat, true, true, true, 0, observe_draws);
    }

    let mut steps = 1usize;
    let mut alive = tick_engine(&mut engine, &mut combat, Some(input));
    if !alive {
        mark_defeat_if_needed(&mut engine, &combat);
        return step_result(engine, combat, false, false, false, steps, observe_draws);
    }
    normalize_player_turn_processing(&mut engine, &combat);

    loop {
        if stable_boundary(&engine, &combat) {
            alive = !matches!(engine, EngineState::GameOver(_));
            return step_result(engine, combat, alive, false, false, steps, observe_draws);
        }
        if steps >= limits.max_engine_steps.max(1) {
            return step_result(engine, combat, true, true, false, steps, observe_draws);
        }
        if limits.deadline.is_some_and(|limit| Instant::now() >= limit) {
            return step_result(engine, combat, true, true, true, steps, observe_draws);
        }

        alive = tick_engine(&mut engine, &mut combat, None);
        steps = steps.saturating_add(1);
        if !alive {
            mark_defeat_if_needed(&mut engine, &combat);
            return step_result(engine, combat, false, false, false, steps, observe_draws);
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
        | EngineState::RewardOverlay { .. }
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
    mut combat: CombatState,
    alive: bool,
    truncated: bool,
    timed_out: bool,
    engine_steps: usize,
    observe_draws: bool,
) -> CombatStepResultInternal {
    let drawn_cards = if observe_draws {
        Some(combat.take_card_draw_observation_events_v1())
    } else {
        combat.clear_card_draw_observation_events();
        None
    };
    let terminal = combat_terminal(&engine, &combat);
    CombatStepResultInternal {
        step: CombatStepResult {
            position: CombatPosition { engine, combat },
            terminal,
            alive,
            truncated,
            timed_out,
            engine_steps,
        },
        drawn_cards,
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

#[cfg(test)]
mod tests {
    use super::{combat_terminal, stable_boundary, CombatTerminal};
    use crate::content::cards::CardId;
    use crate::content::monsters::factory::EncounterId;
    use crate::runtime::combat::CombatCard;
    use crate::sim::combat_start::build_natural_combat_start;
    use crate::state::core::{ClientInput, CombatStartRequest, EngineState, PostCombatReturn};
    use crate::state::map::node::RoomType;
    use crate::state::rewards::RewardState;
    use crate::state::run::RunState;
    use crate::state::{DomainCardSnapshot, DomainEvent, DomainEventSource};
    use crate::test_support::{blank_test_combat, test_monster};

    #[test]
    fn observed_step_returns_cards_drawn_during_that_action() {
        let mut combat = blank_test_combat();
        combat.entities.monsters = vec![test_monster(crate::content::monsters::EnemyId::JawWorm)];
        combat.zones.hand = vec![CombatCard::new(CardId::BattleTrance, 1)];
        combat.zones.draw_pile = vec![
            CombatCard::new(CardId::Defend, 20),
            CombatCard::new(CardId::Strike, 21),
            CombatCard::new(CardId::Bash, 22),
        ];
        combat.emit_event(DomainEvent::CardDrawn {
            card: DomainCardSnapshot {
                id: CardId::AscendersBane,
                upgrades: 0,
                uuid: 99,
            },
            source: DomainEventSource::CombatDraw,
        });
        let position = super::CombatPosition::new(EngineState::CombatPlayerTurn, combat);

        let observed = super::apply_combat_input_to_stable_observed_v1(
            &position,
            ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
            super::CombatStepLimits {
                max_engine_steps: 20,
                deadline: None,
            },
        );

        assert_eq!(
            observed.drawn_cards,
            vec![
                DomainCardSnapshot {
                    id: CardId::Defend,
                    upgrades: 0,
                    uuid: 20,
                },
                DomainCardSnapshot {
                    id: CardId::Strike,
                    upgrades: 0,
                    uuid: 21,
                },
                DomainCardSnapshot {
                    id: CardId::Bash,
                    upgrades: 0,
                    uuid: 22,
                },
            ]
        );
    }

    #[test]
    fn unobserved_step_still_clears_draw_events() {
        let mut combat = blank_test_combat();
        combat.entities.monsters = vec![test_monster(crate::content::monsters::EnemyId::JawWorm)];
        combat.zones.hand = vec![CombatCard::new(CardId::BattleTrance, 1)];
        combat.zones.draw_pile = vec![CombatCard::new(CardId::Defend, 20)];
        let position = super::CombatPosition::new(EngineState::CombatPlayerTurn, combat);

        let step = super::apply_combat_input_to_stable(
            &position,
            ClientInput::PlayCard {
                card_index: 0,
                target: None,
            },
            super::CombatStepLimits {
                max_engine_steps: 20,
                deadline: None,
            },
        );

        assert!(step
            .position
            .combat
            .runtime
            .emitted_events
            .iter()
            .all(|event| !matches!(event, DomainEvent::CardDrawn { .. })));
    }

    #[test]
    fn observed_step_with_expired_deadline_returns_no_draws() {
        let mut combat = blank_test_combat();
        combat.emit_event(DomainEvent::CardDrawn {
            card: DomainCardSnapshot {
                id: CardId::Strike,
                upgrades: 0,
                uuid: 7,
            },
            source: DomainEventSource::CombatDraw,
        });
        let position = super::CombatPosition::new(EngineState::CombatPlayerTurn, combat);

        let observed = super::apply_combat_input_to_stable_observed_v1(
            &position,
            ClientInput::EndTurn,
            super::CombatStepLimits {
                max_engine_steps: 20,
                deadline: Some(std::time::Instant::now() - std::time::Duration::from_secs(1)),
            },
        );

        assert!(observed.drawn_cards.is_empty());
        assert!(observed.step.timed_out);
        assert_eq!(observed.step.engine_steps, 0);
    }

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

    #[test]
    fn combat_terminal_requires_java_death_flags_not_only_zero_hp() {
        let mut run = RunState::new(1, 0, false, "Ironclad");
        let (_engine, mut combat) =
            build_natural_combat_start(&mut run, EncounterId::JawWorm, RoomType::MonsterRoom)
                .expect("combat should initialize");
        let monster = combat
            .entities
            .monsters
            .first_mut()
            .expect("jaw worm should exist");
        monster.current_hp = 0;
        monster.is_dying = false;
        monster.is_escaped = false;
        monster.half_dead = false;

        assert_eq!(
            combat_terminal(&EngineState::CombatPlayerTurn, &combat),
            CombatTerminal::Unresolved,
            "Java victory settlement waits for isDying/isEscaping flags; zero HP alone is not a terminal combat state"
        );
    }
}
