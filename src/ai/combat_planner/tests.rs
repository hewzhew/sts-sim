use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};

use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::runtime::combat::CombatCard;
use crate::sim::combat::{
    CombatPosition, CombatStepLimits, CombatStepResult, CombatStepper, CombatTerminal,
};
use crate::sim::combat_action_surface::{
    combat_legal_action_surface_v2, pending_choice_input_is_legal, CombatLegalActionSurfaceV2,
};
use crate::state::core::{ClientInput, EngineState, HandSelectReason, PendingChoice};

use super::*;

const PLAY: ClientInput = ClientInput::PlayCard {
    card_index: 0,
    target: None,
};

#[derive(Clone)]
struct TinyTurnStepper {
    opens_selection: bool,
    calls: Arc<Mutex<Vec<ClientInput>>>,
    successor_salt: Arc<AtomicI32>,
}

impl TinyTurnStepper {
    fn plain() -> Self {
        Self {
            opens_selection: false,
            calls: Arc::new(Mutex::new(Vec::new())),
            successor_salt: Arc::new(AtomicI32::new(0)),
        }
    }

    fn with_selection() -> Self {
        Self {
            opens_selection: true,
            ..Self::plain()
        }
    }

    fn call_count(&self, input: &ClientInput) -> usize {
        self.calls
            .lock()
            .unwrap()
            .iter()
            .filter(|called| *called == input)
            .count()
    }
}

impl CombatStepper for TinyTurnStepper {
    fn atomic_actions(&self, position: &CombatPosition) -> Vec<ClientInput> {
        match position.engine {
            EngineState::CombatPlayerTurn if position.combat.turn.energy > 0 => {
                vec![PLAY, ClientInput::EndTurn]
            }
            EngineState::CombatPlayerTurn => vec![ClientInput::EndTurn],
            EngineState::PendingChoice(_) => {
                combat_legal_action_surface_v2(&position.engine, &position.combat).atomic_actions
            }
            _ => Vec::new(),
        }
    }

    fn legal_action_surface(&self, position: &CombatPosition) -> CombatLegalActionSurfaceV2 {
        match position.engine {
            EngineState::PendingChoice(_) => {
                combat_legal_action_surface_v2(&position.engine, &position.combat)
            }
            _ => CombatLegalActionSurfaceV2 {
                atomic_actions: self.atomic_actions(position),
                selection_families: Vec::new(),
            },
        }
    }

    fn supports_canonical_pending_choice_actions(&self) -> bool {
        true
    }

    fn is_legal_action(&self, position: &CombatPosition, input: &ClientInput) -> bool {
        match &position.engine {
            EngineState::PendingChoice(choice) => {
                pending_choice_input_is_legal(choice, &position.combat, input)
            }
            _ => self.atomic_actions(position).contains(input),
        }
    }

    fn apply_to_stable(
        &self,
        position: &CombatPosition,
        input: ClientInput,
        _limits: CombatStepLimits,
    ) -> CombatStepResult {
        self.calls.lock().unwrap().push(input.clone());
        let mut next = position.clone();
        match input {
            ClientInput::PlayCard {
                card_index: 0,
                target: None,
            } => {
                next.combat.turn.energy = 0;
                next.combat.turn.turn_start_draw_modifier +=
                    self.successor_salt.load(Ordering::SeqCst);
                if self.opens_selection {
                    next.engine = EngineState::PendingChoice(PendingChoice::HandSelect {
                        candidate_uuids: vec![11, 22],
                        min_cards: 2,
                        max_cards: 2,
                        can_cancel: false,
                        reason: HandSelectReason::Discard,
                    });
                }
            }
            ClientInput::SubmitSelection(resolution) => {
                let selected = resolution.selected_card_uuids();
                next.combat.turn.turn_start_draw_modifier = i32::try_from(selected[0]).unwrap();
                next.engine = EngineState::CombatPlayerTurn;
            }
            ClientInput::EndTurn => {
                next.combat.turn.turn_count += 1;
                next.engine = EngineState::CombatPlayerTurn;
            }
            _ => panic!("tiny stepper received unsupported input"),
        }
        CombatStepResult {
            position: next,
            terminal: CombatTerminal::Unresolved,
            alive: true,
            truncated: false,
            timed_out: false,
            engine_steps: 1,
        }
    }

    fn terminal(&self, _position: &CombatPosition) -> CombatTerminal {
        CombatTerminal::Unresolved
    }
}

fn root() -> CombatDecisionRoot {
    let mut combat = crate::test_support::blank_test_combat();
    combat.entities.monsters = vec![crate::test_support::test_monster(EnemyId::JawWorm)];
    combat.turn.turn_count = 1;
    combat.turn.energy = 1;
    combat.zones.hand = vec![
        CombatCard::new(CardId::Strike, 11),
        CombatCard::new(CardId::Defend, 22),
    ];
    CombatDecisionRoot::new(CombatPosition::new(EngineState::CombatPlayerTurn, combat)).unwrap()
}

fn config() -> TurnOptionGeneratorConfig {
    TurnOptionGeneratorConfig {
        max_engine_steps_per_transition: 4,
    }
}

fn finish(
    session: &mut TurnOptionGeneratorSession,
    stepper: &TinyTurnStepper,
) -> TurnOptionGenerationReport {
    session.advance(stepper, CombatPlanningQuantum::deterministic(100, 100))
}

#[test]
fn only_complete_turn_options_are_public() {
    let stepper = TinyTurnStepper::plain();
    let mut session = TurnOptionGeneratorSession::new(root(), config());

    let first = session.advance(&stepper, CombatPlanningQuantum::deterministic(2, 4));
    assert!(matches!(
        first.status,
        TurnOptionGenerationStatus::Partial(GenerationInterruption::GenerationWorkBudget)
    ));
    assert!(session.completed_options().is_empty());

    let finished = finish(&mut session, &stepper);
    assert_eq!(finished.status, TurnOptionGenerationStatus::Complete);
    assert_eq!(session.completed_options().len(), 2);
    assert!(session.completed_options().iter().all(|option| {
        option.boundary() == CompleteTurnOptionBoundary::NextPlayerTurn
            && matches!(
                option.actions().last().map(|action| &action.input),
                Some(ClientInput::EndTurn)
            )
    }));
}

#[test]
fn split_quantum_matches_one_shot_without_replaying_transitions() {
    let split_stepper = TinyTurnStepper::plain();
    let mut split = TurnOptionGeneratorSession::new(root(), config());
    split.advance(&split_stepper, CombatPlanningQuantum::deterministic(2, 4));
    finish(&mut split, &split_stepper);

    let one_shot_stepper = TinyTurnStepper::plain();
    let mut one_shot = TurnOptionGeneratorSession::new(root(), config());
    finish(&mut one_shot, &one_shot_stepper);

    let split_options = split
        .completed_options()
        .iter()
        .map(|option| {
            (
                option.actions().to_vec(),
                option.exact_successor_hash().to_owned(),
            )
        })
        .collect::<Vec<_>>();
    let one_shot_options = one_shot
        .completed_options()
        .iter()
        .map(|option| {
            (
                option.actions().to_vec(),
                option.exact_successor_hash().to_owned(),
            )
        })
        .collect::<Vec<_>>();
    assert_eq!(split_options, one_shot_options);
    assert_eq!(split_stepper.call_count(&PLAY), 1);
    assert_eq!(split_stepper.call_count(&ClientInput::EndTurn), 2);
}

#[test]
fn engine_transition_waits_for_a_full_reservation() {
    let stepper = TinyTurnStepper::plain();
    let mut session = TurnOptionGeneratorSession::new(root(), config());

    let blocked = session.advance(&stepper, CombatPlanningQuantum::deterministic(10, 3));
    assert_eq!(
        blocked.status,
        TurnOptionGenerationStatus::Partial(GenerationInterruption::EngineStepBudget)
    );
    assert!(stepper.calls.lock().unwrap().is_empty());

    session.advance(&stepper, CombatPlanningQuantum::deterministic(0, 1));
    assert_eq!(stepper.call_count(&PLAY), 1);
}

#[test]
fn ordered_structured_selections_survive_complete_option_generation() {
    let stepper = TinyTurnStepper::with_selection();
    let mut session = TurnOptionGeneratorSession::new(root(), config());
    let report = finish(&mut session, &stepper);

    assert_eq!(report.status, TurnOptionGenerationStatus::Complete);
    let submitted_orders = session
        .completed_options()
        .iter()
        .flat_map(|option| option.actions())
        .filter_map(|action| match &action.input {
            ClientInput::SubmitSelection(resolution) => Some(resolution.selected_card_uuids()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert!(submitted_orders.contains(&vec![11, 22]));
    assert!(submitted_orders.contains(&vec![22, 11]));
}

#[test]
fn exact_replay_verifies_each_successor_and_final_position() {
    let stepper = TinyTurnStepper::plain();
    let root = root();
    let mut session = TurnOptionGeneratorSession::new(root.clone(), config());
    finish(&mut session, &stepper);
    let option = session
        .completed_options()
        .iter()
        .find(|option| option.actions().len() == 2)
        .unwrap();

    let replay = replay_turn_option(
        &root,
        option,
        &stepper,
        ReplayLimits::deterministic(option.engine_steps()),
    )
    .unwrap();
    assert_eq!(replay.position, *option.exact_successor());

    stepper.successor_salt.store(1, Ordering::SeqCst);
    assert_eq!(
        replay_turn_option(
            &root,
            option,
            &stepper,
            ReplayLimits::deterministic(option.engine_steps())
        )
        .unwrap_err(),
        ReplayError::SuccessorMismatch { action_index: 0 }
    );
}
