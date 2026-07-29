use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};

use sts_core::content::cards::CardId;
use sts_core::content::monsters::EnemyId;
use sts_core::content::potions::{Potion, PotionId};
use sts_core::content::powers::PowerId;
use sts_core::runtime::combat::{CombatCard, Power, PowerPayload};
use sts_core::sim::combat::{
    CombatPosition, CombatStepLimits, CombatStepResult, CombatStepper, CombatTerminal,
    EngineCombatStepper,
};
use sts_core::sim::combat_action_surface::{
    combat_legal_action_surface_v2, pending_choice_input_is_legal, CombatLegalActionSurfaceV2,
    CombatSelectionActionFamilyV2,
};
use sts_core::state::core::{ClientInput, EngineState, HandSelectReason, PendingChoice};

use super::*;
use crate::generator::TurnOptionGeneratorPreferredLane;
use crate::types::exact_hash;

mod local_turn_graph;
mod policy_discrepancy;

const PLAY: ClientInput = ClientInput::PlayCard {
    card_index: 0,
    target: None,
};

#[derive(Clone, Copy)]
struct PreferPlayPolicy;

#[derive(Clone, Copy)]
struct PreferEndTurnPolicy;

impl CombatActionPolicy for PreferPlayPolicy {
    fn weights(&self, _position: &CombatPosition, choices: &[CombatPolicyChoice<'_>]) -> Vec<f64> {
        choices
            .iter()
            .map(|choice| match choice {
                CombatPolicyChoice::Atomic(input) if **input == PLAY => 100.0,
                _ => 1.0,
            })
            .collect()
    }

    fn state_guide_rank(&self, position: &CombatPosition) -> Option<CombatStateGuideRank> {
        Some(CombatStateGuideRank::new(vec![
            i32::from(matches!(
                position.engine,
                EngineState::GameOver(sts_core::state::core::RunResult::Victory)
            )),
            position.combat.turn.turn_count as i32,
            -position
                .combat
                .entities
                .monsters
                .iter()
                .map(|monster| monster.current_hp.max(0))
                .sum::<i32>(),
        ]))
    }
}

impl CombatActionPolicy for PreferEndTurnPolicy {
    fn weights(&self, _position: &CombatPosition, choices: &[CombatPolicyChoice<'_>]) -> Vec<f64> {
        choices
            .iter()
            .map(|choice| match choice {
                CombatPolicyChoice::Atomic(ClientInput::EndTurn) => 100.0,
                _ => 1.0,
            })
            .collect()
    }

    fn state_guide_rank(&self, position: &CombatPosition) -> Option<CombatStateGuideRank> {
        Some(CombatStateGuideRank::new(vec![i32::from(matches!(
            position.engine,
            EngineState::GameOver(sts_core::state::core::RunResult::Victory)
        ))]))
    }
}

#[derive(Clone, Copy)]
struct PreferSelection22Policy;

impl CombatActionPolicy for PreferSelection22Policy {
    fn weights(&self, position: &CombatPosition, choices: &[CombatPolicyChoice<'_>]) -> Vec<f64> {
        PreferPlayPolicy.weights(position, choices)
    }

    fn structured_selection_member_weights(
        &self,
        _position: &CombatPosition,
        _family: &CombatSelectionActionFamilyV2,
        members: &[ClientInput],
    ) -> Vec<f64> {
        members
            .iter()
            .map(|input| match input {
                ClientInput::SubmitSelection(resolution)
                    if resolution.selected_card_uuids() == [22] =>
                {
                    100.0
                }
                _ => 1.0,
            })
            .collect()
    }
}

#[derive(Clone, Copy)]
struct SharedGuidePolicy;

const SHARED_TEST_GUIDE: CombatGuideLaneId = CombatGuideLaneId::new(77);

impl CombatActionPolicy for SharedGuidePolicy {
    fn weights(&self, _position: &CombatPosition, choices: &[CombatPolicyChoice<'_>]) -> Vec<f64> {
        choices
            .iter()
            .map(|choice| match choice {
                CombatPolicyChoice::Atomic(input) if **input == PLAY => 100.0,
                _ => 1.0,
            })
            .collect()
    }

    fn state_guides(&self, position: &CombatPosition) -> Vec<CombatStateGuide> {
        vec![CombatStateGuide::new(
            SHARED_TEST_GUIDE,
            vec![i32::from(position.combat.turn.energy == 0)],
        )]
    }

    fn turn_generation_guides(&self, position: &CombatPosition) -> Vec<CombatStateGuide> {
        self.state_guides(position)
    }
}

#[derive(Clone)]
struct TinyTurnStepper {
    selection_size: Option<u8>,
    winning_selection_22: bool,
    duplicate_play_surface: bool,
    opens_awakened_transition_window: bool,
    activates_finite_skill_conversion: bool,
    lethal_from_turn: Option<u32>,
    terminal_loss: bool,
    calls: Arc<Mutex<Vec<ClientInput>>>,
    successor_salt: Arc<AtomicI32>,
}

impl TinyTurnStepper {
    fn plain() -> Self {
        Self {
            selection_size: None,
            winning_selection_22: false,
            duplicate_play_surface: false,
            opens_awakened_transition_window: false,
            activates_finite_skill_conversion: false,
            lethal_from_turn: None,
            terminal_loss: false,
            calls: Arc::new(Mutex::new(Vec::new())),
            successor_salt: Arc::new(AtomicI32::new(0)),
        }
    }

    fn with_selection() -> Self {
        Self {
            selection_size: Some(2),
            ..Self::plain()
        }
    }

    fn with_single_selection() -> Self {
        Self {
            selection_size: Some(1),
            ..Self::plain()
        }
    }

    fn with_winning_single_selection() -> Self {
        Self {
            selection_size: Some(1),
            winning_selection_22: true,
            ..Self::plain()
        }
    }

    fn with_duplicate_play_surface() -> Self {
        Self {
            duplicate_play_surface: true,
            ..Self::plain()
        }
    }

    fn activating_finite_skill_conversion() -> Self {
        Self {
            activates_finite_skill_conversion: true,
            ..Self::plain()
        }
    }

    fn lethal() -> Self {
        Self {
            lethal_from_turn: Some(1),
            ..Self::plain()
        }
    }

    fn lethal_after_current_turn() -> Self {
        Self {
            lethal_from_turn: Some(2),
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
                if self.duplicate_play_surface {
                    vec![PLAY, PLAY, ClientInput::EndTurn]
                } else {
                    vec![PLAY, ClientInput::EndTurn]
                }
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
        if self.terminal_loss {
            next.engine = EngineState::GameOver(sts_core::state::core::RunResult::Defeat);
        } else {
            match input {
                ClientInput::PlayCard {
                    card_index: 0,
                    target: None,
                } => {
                    next.combat.turn.energy = 0;
                    next.combat.turn.turn_start_draw_modifier +=
                        self.successor_salt.load(Ordering::SeqCst);
                    if self.opens_awakened_transition_window {
                        let awakened = &mut next.combat.entities.monsters[0];
                        awakened.awakened_one.form1 = false;
                        awakened.half_dead = true;
                        awakened.current_hp = 0;
                    }
                    if self.activates_finite_skill_conversion {
                        let player = next.combat.entities.player.id;
                        sts_core::content::powers::store::set_powers_for(
                            &mut next.combat,
                            player,
                            vec![Power {
                                power_type: PowerId::Corruption,
                                instance_id: None,
                                amount: -1,
                                extra_data: 0,
                                payload: PowerPayload::None,
                                just_applied: false,
                            }],
                        );
                    }
                    if let Some(selection_size) = self.selection_size {
                        next.engine = EngineState::PendingChoice(PendingChoice::HandSelect {
                            candidate_uuids: vec![11, 22],
                            min_cards: selection_size,
                            max_cards: selection_size,
                            can_cancel: false,
                            reason: HandSelectReason::Discard,
                        });
                    } else if self
                        .lethal_from_turn
                        .is_some_and(|turn| next.combat.turn.turn_count >= turn)
                    {
                        next.engine =
                            EngineState::GameOver(sts_core::state::core::RunResult::Victory);
                    }
                }
                ClientInput::SubmitSelection(resolution) => {
                    let selected = resolution.selected_card_uuids();
                    next.combat.turn.turn_start_draw_modifier = i32::try_from(selected[0]).unwrap();
                    next.engine = if self.winning_selection_22 && selected == [22] {
                        EngineState::GameOver(sts_core::state::core::RunResult::Victory)
                    } else {
                        EngineState::CombatPlayerTurn
                    };
                }
                ClientInput::EndTurn => {
                    next.combat.turn.turn_count += 1;
                    next.engine = EngineState::CombatPlayerTurn;
                }
                _ => panic!("tiny stepper received unsupported input"),
            }
        }
        let terminal = self.terminal(&next);
        CombatStepResult {
            position: next,
            terminal,
            alive: true,
            truncated: false,
            timed_out: false,
            engine_steps: 1,
        }
    }

    fn terminal(&self, position: &CombatPosition) -> CombatTerminal {
        match position.engine {
            EngineState::GameOver(sts_core::state::core::RunResult::Victory) => CombatTerminal::Win,
            EngineState::GameOver(sts_core::state::core::RunResult::Defeat) => CombatTerminal::Loss,
            _ => CombatTerminal::Unresolved,
        }
    }
}

fn root() -> CombatDecisionRoot {
    let mut combat = sts_core::test_support::blank_test_combat();
    combat.entities.monsters = vec![sts_core::test_support::test_monster(EnemyId::JawWorm)];
    combat.entities.monsters[0].max_hp = 60;
    combat.entities.monsters[0].current_hp = 40;
    combat.turn.turn_count = 1;
    combat.turn.energy = 1;
    combat.zones.hand = vec![
        CombatCard::new(CardId::Strike, 11),
        CombatCard::new(CardId::Defend, 22),
    ];
    CombatDecisionRoot::new(CombatPosition::new(EngineState::CombatPlayerTurn, combat)).unwrap()
}

#[test]
fn combat_root_accepts_a_stable_pre_turn_selection_boundary() {
    let mut combat = sts_core::test_support::blank_test_combat();
    combat.entities.monsters = vec![sts_core::test_support::planned_monster(EnemyId::JawWorm, 1)];
    combat.zones.hand = vec![
        CombatCard::new(CardId::Strike, 11),
        CombatCard::new(CardId::Defend, 22),
    ];
    let engine = EngineState::PendingChoice(PendingChoice::HandSelect {
        candidate_uuids: vec![11, 22],
        min_cards: 0,
        max_cards: 2,
        can_cancel: true,
        reason: HandSelectReason::GamblingChip,
    });
    let turn_count = combat.turn.turn_count;

    let root = CombatDecisionRoot::new(CombatPosition::new(engine, combat))
        .expect("Gambling Chip is a stable combat input boundary");

    assert_eq!(root.turn_count(), turn_count);
    assert!(matches!(
        root.position().engine,
        EngineState::PendingChoice(PendingChoice::HandSelect {
            reason: HandSelectReason::GamblingChip,
            ..
        })
    ));

    let mut generator = TurnOptionGeneratorSession::new(
        root,
        TurnOptionGeneratorConfig {
            max_engine_steps_per_transition: 256,
            ..TurnOptionGeneratorConfig::default()
        },
    );
    let report = generator.advance(
        &EngineCombatStepper,
        CombatPlanningQuantum::deterministic(1_000, 8_192),
    );
    assert_eq!(
        report.status,
        TurnOptionGenerationStatus::Complete,
        "{report:#?}"
    );
    let first_inputs = generator
        .completed_options()
        .iter()
        .filter_map(|option| option.actions().first())
        .map(|action| action.input.clone())
        .collect::<Vec<_>>();
    assert!(
        generator.completed_options().iter().all(|option| {
            option.boundary() == CompleteTurnOptionBoundary::NextPlayerTurn
                && option
                    .actions()
                    .first()
                    .is_some_and(|action| {
                        matches!(
                            action.input,
                            ClientInput::Cancel | ClientInput::SubmitSelection(_)
                        )
                    })
        }),
        "every generated turn must resolve Gambling Chip before playing the combat turn: {first_inputs:?}"
    );
    assert!(
        generator.completed_options().len() >= 4,
        "all legal discard subsets must remain visible to combat planning"
    );
}

fn awakened_root() -> CombatDecisionRoot {
    let mut combat = sts_core::test_support::blank_test_combat();
    let mut awakened = sts_core::test_support::test_monster(EnemyId::AwakenedOne);
    awakened.id = 10;
    awakened.slot = 0;
    combat.entities.monsters = vec![awakened];
    combat.turn.turn_count = 1;
    combat.turn.energy = 1;
    combat.zones.hand = vec![
        CombatCard::new(CardId::Strike, 11),
        CombatCard::new(CardId::Defend, 22),
    ];
    CombatDecisionRoot::new(CombatPosition::new(EngineState::CombatPlayerTurn, combat)).unwrap()
}

fn awakened_conversion_root() -> CombatDecisionRoot {
    let mut combat = sts_core::test_support::blank_test_combat();
    let mut awakened = sts_core::test_support::test_monster(EnemyId::AwakenedOne);
    awakened.id = 10;
    awakened.slot = 0;
    combat.entities.monsters = vec![awakened];
    combat.turn.turn_count = 1;
    combat.turn.energy = 1;
    combat.zones.hand = vec![
        CombatCard::new(CardId::Corruption, 11),
        CombatCard::new(CardId::Defend, 22),
    ];
    CombatDecisionRoot::new(CombatPosition::new(EngineState::CombatPlayerTurn, combat)).unwrap()
}

fn threatened_awakened_conversion_root() -> CombatDecisionRoot {
    let mut combat = awakened_conversion_root().position().combat.clone();
    combat.entities.player.current_hp = 1;
    combat.entities.monsters[0].set_planned_move_id(1);
    CombatDecisionRoot::new(CombatPosition::new(EngineState::CombatPlayerTurn, combat)).unwrap()
}

fn transition_window_conversion_root() -> CombatDecisionRoot {
    let mut combat = awakened_conversion_root().position().combat.clone();
    combat.entities.monsters[0].awakened_one.form1 = false;
    combat.entities.monsters[0].half_dead = true;
    combat.entities.monsters[0].current_hp = 0;
    CombatDecisionRoot::new(CombatPosition::new(EngineState::CombatPlayerTurn, combat)).unwrap()
}

fn config() -> TurnOptionGeneratorConfig {
    TurnOptionGeneratorConfig {
        max_engine_steps_per_transition: 4,
        ..TurnOptionGeneratorConfig::default()
    }
}

fn exact_actions(
    stepper: &dyn CombatStepper,
    root: &CombatDecisionRoot,
    inputs: impl IntoIterator<Item = ClientInput>,
) -> Vec<TurnOptionAction> {
    let mut position = root.position().clone();
    inputs
        .into_iter()
        .map(|input| {
            let result = stepper.apply_to_stable(
                &position,
                input.clone(),
                CombatStepLimits {
                    max_engine_steps: 4,
                    deadline: None,
                },
            );
            let action = TurnOptionAction {
                input,
                expected_successor_hash: exact_hash(&result.position).into(),
                engine_steps: result.engine_steps,
            };
            position = result.position;
            action
        })
        .collect()
}

#[test]
fn policy_guided_generator_emits_preferred_and_complete_sibling_options() {
    let stepper = TinyTurnStepper::lethal();
    let mut session =
        TurnOptionGeneratorSession::with_policy(root(), config(), Arc::new(PreferPlayPolicy));

    let report = session.advance(&stepper, CombatPlanningQuantum::deterministic(4, 8));

    assert_eq!(report.newly_completed_options, 2);
    assert_eq!(session.completed_options()[0].actions()[0].input, PLAY);
    assert!(session
        .completed_options()
        .iter()
        .any(|option| option.actions()[0].input == ClientInput::EndTurn));
    assert_eq!(session.retained_work_items(), 0);
    assert!(session.is_finished());
}

#[test]
fn depth_beam_keeps_long_partial_after_a_short_turn_option_finishes() {
    let stepper = TinyTurnStepper::plain();
    let report = generate_depth_beam_turn_options(
        root(),
        DepthBeamTurnConfig {
            generator: config(),
            partial_beam_width: 1,
            retained_per_view: 1,
            max_atomic_depth: 4,
            max_structured_members_per_family: 8,
        },
        DepthBeamTurnBudget {
            max_applied_transitions: 8,
            max_engine_steps: 32,
            deadline: None,
        },
        Arc::new(PreferPlayPolicy),
        &stepper,
    );

    assert_eq!(report.status, DepthBeamTurnStatus::Complete);
    assert!(report.options.iter().any(|option| {
        option.actions().len() == 1 && option.actions()[0].input == ClientInput::EndTurn
    }));
    assert!(report.options.iter().any(|option| {
        option.actions().len() == 2
            && option.actions()[0].input == PLAY
            && option.actions()[1].input == ClientInput::EndTurn
    }));
}

#[test]
fn depth_beam_crosses_a_singleton_structured_selection() {
    let stepper = TinyTurnStepper::with_single_selection();
    let report = generate_depth_beam_turn_options(
        root(),
        DepthBeamTurnConfig {
            generator: config(),
            partial_beam_width: 1,
            retained_per_view: 1,
            max_atomic_depth: 5,
            max_structured_members_per_family: 8,
        },
        DepthBeamTurnBudget {
            max_applied_transitions: 12,
            max_engine_steps: 48,
            deadline: None,
        },
        Arc::new(PreferSelection22Policy),
        &stepper,
    );

    let selected = report
        .options
        .iter()
        .find(|option| option.actions().len() == 3)
        .expect("play, singleton choice, and end turn should form one proposal");
    assert_eq!(
        report.status,
        DepthBeamTurnStatus::Partial(DepthBeamTurnInterruption::BeamPruned)
    );
    assert_eq!(report.counters.pruned_partial_states, 1);
    assert!(matches!(
        &selected.actions()[1].input,
        ClientInput::SubmitSelection(resolution)
            if resolution.selected_card_uuids() == [22]
    ));
}

#[test]
fn depth_beam_reports_a_truncated_structured_family_as_partial() {
    let report = generate_depth_beam_turn_options(
        root(),
        DepthBeamTurnConfig {
            generator: config(),
            partial_beam_width: 8,
            retained_per_view: 1,
            max_atomic_depth: 5,
            max_structured_members_per_family: 1,
        },
        DepthBeamTurnBudget {
            max_applied_transitions: 12,
            max_engine_steps: 48,
            deadline: None,
        },
        Arc::new(PreferSelection22Policy),
        &TinyTurnStepper::with_single_selection(),
    );

    assert_eq!(
        report.status,
        DepthBeamTurnStatus::Partial(DepthBeamTurnInterruption::StructuredFamilyLimit)
    );
    assert_eq!(report.counters.truncated_structured_families, 1);
}

#[test]
fn shared_guide_publishes_and_services_the_best_partial_expansion() {
    let stepper = TinyTurnStepper::plain();
    let mut generator =
        TurnOptionGeneratorSession::with_policy(root(), config(), Arc::new(SharedGuidePolicy));

    assert_eq!(
        generator
            .best_retained_guide_promise_snapshot(SHARED_TEST_GUIDE)
            .expect("root guide promise")
            .rank,
        CombatStateGuideRank::new(vec![0])
    );

    generator.advance(&stepper, CombatPlanningQuantum::deterministic(1, 4));
    generator.prefer_lane(TurnOptionGeneratorPreferredLane::Guide(SHARED_TEST_GUIDE));
    generator.advance(&stepper, CombatPlanningQuantum::deterministic(1, 4));

    assert_eq!(
        generator
            .best_retained_guide_promise_snapshot(SHARED_TEST_GUIDE)
            .expect("partial-state guide promise")
            .rank,
        CombatStateGuideRank::new(vec![1]),
        "the resumable parent must publish its best retained partial state, not its stale root rank"
    );

    let guided_before = generator.guided_work_pops();
    generator.prefer_lane(TurnOptionGeneratorPreferredLane::Guide(SHARED_TEST_GUIDE));
    generator.advance(&stepper, CombatPlanningQuantum::deterministic(1, 4));
    assert_eq!(generator.guided_work_pops(), guided_before + 1);
}

#[test]
fn atomic_siblings_share_one_resumable_cursor_and_one_service_emits_one_edge() {
    let stepper = TinyTurnStepper::plain();
    let decision_root = root();
    let mut session = TurnOptionGeneratorSession::with_policy(
        decision_root.clone(),
        config(),
        Arc::new(PreferPlayPolicy),
    );

    let discovery = session.advance(&stepper, CombatPlanningQuantum::deterministic(1, 8));
    assert_eq!(discovery.after_diagnostics.applied_action_transitions, 0);
    assert_eq!(session.retained_work_items(), 1);
    assert_eq!(
        session.live_work_counts_at_exact_position(decision_root.position()),
        (0, 2, 0),
        "one cursor owns both concrete atomic siblings"
    );

    let first_edge = session.advance(&stepper, CombatPlanningQuantum::deterministic(1, 8));
    assert_eq!(first_edge.after_diagnostics.applied_action_transitions, 1);
    assert_eq!(
        first_edge
            .after_diagnostics
            .applied_action_transitions
            .saturating_sub(first_edge.before_diagnostics.applied_action_transitions),
        1,
        "one cursor service must not turn into eager all-successor expansion"
    );
}

#[test]
fn generator_publishes_a_reached_turn_boundary_without_rescheduling_it() {
    let stepper = TinyTurnStepper::plain();
    let mut position = root().position().clone();
    position.combat.turn.energy = 0;
    let root = CombatDecisionRoot::new(position).unwrap();
    let mut session = TurnOptionGeneratorSession::new(root, config());

    // One work item expands the root and one executes EndTurn. The resulting
    // next-player-turn state is already stable and must be published without
    // requiring a third agenda pop merely to recognize the boundary.
    let report = session.advance(&stepper, CombatPlanningQuantum::deterministic(2, 8));

    assert_eq!(report.after.generation_work, 2);
    assert_eq!(report.newly_completed_options, 1);
    assert_eq!(
        session.completed_options()[0].actions()[0].input,
        ClientInput::EndTurn
    );
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
fn generation_diagnostics_count_exact_successor_merges_without_changing_options() {
    let stepper = TinyTurnStepper::with_duplicate_play_surface();
    let mut session = TurnOptionGeneratorSession::new(root(), config());

    let report = finish(&mut session, &stepper);

    assert_eq!(report.status, TurnOptionGenerationStatus::Complete);
    assert_eq!(session.completed_options().len(), 2);
    assert_eq!(report.after_diagnostics.duplicate_exact_successors, 1);
    assert_eq!(report.after_diagnostics.applied_action_transitions, 4);
    assert_eq!(report.after_diagnostics.unique_successor_states, 3);
    assert_eq!(report.after_diagnostics.completed_turn_options, 2);
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
fn singleton_selection_member_policy_reorders_without_removing_siblings() {
    let stepper = TinyTurnStepper::with_single_selection();
    let mut uniform = TurnOptionGeneratorSession::new(root(), config());
    finish(&mut uniform, &stepper);
    let uniform_members = uniform
        .completed_options()
        .iter()
        .flat_map(|option| option.actions())
        .filter_map(|action| match &action.input {
            ClientInput::SubmitSelection(resolution) => Some(resolution.selected_card_uuids()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(uniform_members, vec![vec![11], vec![22]]);

    let mut guided = TurnOptionGeneratorSession::with_policy(
        root(),
        config(),
        Arc::new(PreferSelection22Policy),
    );
    finish(&mut guided, &stepper);
    let guided_members = guided
        .completed_options()
        .iter()
        .flat_map(|option| option.actions())
        .filter_map(|action| match &action.input {
            ClientInput::SubmitSelection(resolution) => Some(resolution.selected_card_uuids()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(guided_members, vec![vec![22], vec![11]]);
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

#[test]
fn real_engine_preserves_targeted_potion_inside_an_exact_option() {
    let mut combat = sts_core::test_support::blank_test_combat();
    let monster = sts_core::test_support::planned_monster(EnemyId::JawWorm, 1);
    let target = monster.id;
    combat.entities.monsters = vec![monster];
    combat.entities.potions = vec![Some(Potion::new(PotionId::FirePotion, 7))];
    combat.zones.hand.clear();
    let root = CombatDecisionRoot::new(CombatPosition::new(EngineState::CombatPlayerTurn, combat))
        .unwrap();
    let stepper = EngineCombatStepper;
    let mut session = TurnOptionGeneratorSession::new(
        root.clone(),
        TurnOptionGeneratorConfig {
            max_engine_steps_per_transition: 256,
            ..TurnOptionGeneratorConfig::default()
        },
    );

    let report = session.advance(&stepper, CombatPlanningQuantum::deterministic(1_000, 8_192));
    assert_eq!(report.status, TurnOptionGenerationStatus::Complete);
    let option = session
        .completed_options()
        .iter()
        .find(|option| {
            option.actions().iter().any(|action| {
                action.input
                    == ClientInput::UsePotion {
                        potion_index: 0,
                        target: Some(target),
                    }
            })
        })
        .expect("targeted Fire Potion should survive option generation");

    let replay = replay_turn_option(
        &root,
        option,
        &stepper,
        ReplayLimits::deterministic(option.engine_steps()),
    )
    .unwrap();
    assert_eq!(replay.position, *option.exact_successor());
    assert!(option.exact_successor().combat.entities.potions[0].is_none());
    assert!(
        option.exact_successor().combat.entities.monsters[0].current_hp
            < root.position().combat.entities.monsters[0].current_hp
    );
}

#[test]
fn explicit_zero_potion_generator_phase_removes_potion_expenditure_inputs() {
    let mut combat = sts_core::test_support::blank_test_combat();
    let monster = sts_core::test_support::planned_monster(EnemyId::JawWorm, 1);
    combat.entities.monsters = vec![monster];
    combat.entities.potions = vec![Some(Potion::new(PotionId::FirePotion, 7))];
    combat.zones.hand.clear();
    let root = CombatDecisionRoot::new(CombatPosition::new(EngineState::CombatPlayerTurn, combat))
        .unwrap();
    let stepper = EngineCombatStepper;
    let mut session = TurnOptionGeneratorSession::new(
        root,
        TurnOptionGeneratorConfig {
            max_engine_steps_per_transition: 256,
            allow_potion_expenditure: false,
            ..TurnOptionGeneratorConfig::default()
        },
    );

    let report = session.advance(&stepper, CombatPlanningQuantum::deterministic(1_000, 8_192));
    assert_eq!(report.status, TurnOptionGenerationStatus::Complete);
    assert!(
        session
            .completed_options()
            .iter()
            .all(|option| option.actions().iter().all(|action| !matches!(
                action.input,
                ClientInput::UsePotion { .. } | ClientInput::DiscardPotion(_)
            ))),
        "a zero-potion phase must not generate potion use or discard lines"
    );
}

#[test]
fn finite_potion_generator_limit_prunes_over_budget_prefixes() {
    let mut combat = sts_core::test_support::blank_test_combat();
    let monster = sts_core::test_support::planned_monster(EnemyId::TheGuardian, 1);
    combat.entities.monsters = vec![monster];
    combat.entities.potions = vec![
        Some(Potion::new(PotionId::EnergyPotion, 7)),
        Some(Potion::new(PotionId::EnergyPotion, 8)),
    ];
    combat.zones.hand.clear();
    let root = CombatDecisionRoot::new(CombatPosition::new(EngineState::CombatPlayerTurn, combat))
        .unwrap();
    let stepper = EngineCombatStepper;
    let mut session = TurnOptionGeneratorSession::with_policy_and_potion_limit(
        root,
        TurnOptionGeneratorConfig {
            max_engine_steps_per_transition: 256,
            ..TurnOptionGeneratorConfig::default()
        },
        Arc::new(PreferPlayPolicy),
        Some(1),
    );

    let report = session.advance(
        &stepper,
        CombatPlanningQuantum::deterministic(4_000, 32_768),
    );
    assert_eq!(report.status, TurnOptionGenerationStatus::Complete);
    let potion_expenditures = session
        .completed_options()
        .iter()
        .map(|option| {
            option
                .actions()
                .iter()
                .filter(|action| {
                    matches!(
                        action.input,
                        ClientInput::UsePotion { .. } | ClientInput::DiscardPotion(_)
                    )
                })
                .count()
        })
        .collect::<Vec<_>>();
    assert!(
        potion_expenditures.iter().any(|count| *count == 1),
        "the finite allowance must retain legal one-potion turn lines"
    );
    assert!(
        potion_expenditures.iter().all(|count| *count <= 1),
        "the generator must not spend work completing over-budget turn lines"
    );
}
