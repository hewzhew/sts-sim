use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};

use sts_core::content::cards::CardId;
use sts_core::content::monsters::EnemyId;
use sts_core::content::potions::{Potion, PotionId};
use sts_core::runtime::combat::CombatCard;
use sts_core::sim::combat::{
    CombatPosition, CombatStepLimits, CombatStepResult, CombatStepper, CombatTerminal,
    EngineCombatStepper,
};
use sts_core::sim::combat_action_surface::{
    combat_legal_action_surface_v2, pending_choice_input_is_legal, CombatLegalActionSurfaceV2,
};
use sts_core::state::core::{ClientInput, EngineState, HandSelectReason, PendingChoice};

use super::*;
use crate::types::exact_hash;

const PLAY: ClientInput = ClientInput::PlayCard {
    card_index: 0,
    target: None,
};

#[derive(Clone, Copy)]
struct PreferPlayPolicy;

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

#[derive(Clone)]
struct FixedWitnessProposalPolicy {
    proposal: CombatPolicyWitnessProposal,
}

#[derive(Clone)]
struct SplitGuidePolicy {
    boundary_calls: Arc<AtomicI32>,
    generation_calls: Arc<AtomicI32>,
}

impl CombatActionPolicy for SplitGuidePolicy {
    fn weights(&self, _position: &CombatPosition, choices: &[CombatPolicyChoice<'_>]) -> Vec<f64> {
        vec![1.0; choices.len()]
    }

    fn state_guide_ranks(&self, _position: &CombatPosition) -> Vec<CombatStateGuideRank> {
        self.boundary_calls.fetch_add(1, Ordering::Relaxed);
        vec![CombatStateGuideRank::new(vec![1])]
    }

    fn turn_generation_guide_ranks(&self, _position: &CombatPosition) -> Vec<CombatStateGuideRank> {
        self.generation_calls.fetch_add(1, Ordering::Relaxed);
        vec![CombatStateGuideRank::new(vec![2])]
    }
}

#[test]
fn witness_search_keeps_boundary_and_turn_generation_guides_separate() {
    let boundary_calls = Arc::new(AtomicI32::new(0));
    let generation_calls = Arc::new(AtomicI32::new(0));
    let policy = Arc::new(SplitGuidePolicy {
        boundary_calls: boundary_calls.clone(),
        generation_calls: generation_calls.clone(),
    });

    let _session = OracleCombatWitnessSession::with_policy(
        root(),
        OracleCombatWitnessConfig::default(),
        policy,
    );

    assert_eq!(boundary_calls.load(Ordering::Relaxed), 1);
    assert_eq!(generation_calls.load(Ordering::Relaxed), 1);
}

impl CombatActionPolicy for FixedWitnessProposalPolicy {
    fn weights(&self, _position: &CombatPosition, choices: &[CombatPolicyChoice<'_>]) -> Vec<f64> {
        vec![1.0; choices.len()]
    }

    fn witness_proposal(
        &self,
        _position: &CombatPosition,
        _deadline: Option<std::time::Instant>,
    ) -> Option<CombatPolicyWitnessProposal> {
        Some(self.proposal.clone())
    }
}

#[derive(Clone)]
struct TinyTurnStepper {
    opens_selection: bool,
    duplicate_play_surface: bool,
    lethal_from_turn: Option<u32>,
    terminal_loss: bool,
    play_damage: i32,
    calls: Arc<Mutex<Vec<ClientInput>>>,
    successor_salt: Arc<AtomicI32>,
}

impl TinyTurnStepper {
    fn plain() -> Self {
        Self {
            opens_selection: false,
            duplicate_play_surface: false,
            lethal_from_turn: None,
            terminal_loss: false,
            play_damage: 0,
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

    fn with_duplicate_play_surface() -> Self {
        Self {
            duplicate_play_surface: true,
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

    fn losing() -> Self {
        Self {
            terminal_loss: true,
            ..Self::plain()
        }
    }

    fn damaging(play_damage: i32) -> Self {
        Self {
            play_damage,
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
                    if let Some(monster) = next.combat.entities.monsters.first_mut() {
                        monster.current_hp = monster.current_hp.saturating_sub(self.play_damage);
                    }
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
                    next.engine = EngineState::CombatPlayerTurn;
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
                expected_successor_hash: exact_hash(&result.position),
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

#[test]
fn oracle_witness_search_crosses_turns_and_exactly_replays_first_win() {
    let stepper = TinyTurnStepper::lethal_after_current_turn();
    let mut session = OracleCombatWitnessSession::with_policy(
        root(),
        OracleCombatWitnessConfig {
            generator: config(),
            generation_work_per_agenda_pop: 1,
            satisfaction: OracleCombatWitnessSatisfaction::FirstWitness,
        },
        Arc::new(PreferPlayPolicy),
    );

    let report = session.advance(
        &stepper,
        OracleCombatWitnessQuantum::deterministic(1_000, 1_000, 4_000),
    );

    assert_eq!(report.status, OracleCombatWitnessStatus::WitnessFound);
    let witness = report.witness.expect("verified witness");
    assert_eq!(witness.actions.first().unwrap().input, ClientInput::EndTurn);
    assert_eq!(witness.actions.last().unwrap().input, PLAY);
    assert_eq!(
        stepper.terminal(&witness.final_position),
        CombatTerminal::Win
    );
    assert!(witness.replay_engine_steps > 0);
}

#[test]
fn policy_witness_proposal_only_becomes_a_witness_after_exact_root_replay() {
    let stepper = TinyTurnStepper::lethal_after_current_turn();
    let decision_root = root();
    let proposal = CombatPolicyWitnessProposal {
        actions: exact_actions(&stepper, &decision_root, [ClientInput::EndTurn, PLAY]),
        final_hp_hint: decision_root.position().combat.entities.player.current_hp,
    };
    let mut session = OracleCombatWitnessSession::with_policy(
        decision_root,
        OracleCombatWitnessConfig {
            generator: config(),
            generation_work_per_agenda_pop: 1,
            satisfaction: OracleCombatWitnessSatisfaction::FirstWitness,
        },
        Arc::new(FixedWitnessProposalPolicy { proposal }),
    );

    let report = session.advance(
        &stepper,
        OracleCombatWitnessQuantum::deterministic(8, 8, 32),
    );

    assert_eq!(report.status, OracleCombatWitnessStatus::WitnessFound);
    assert_eq!(report.after.policy_witness_proposals, 1);
    assert_eq!(report.after.generation_work, 0);
    let witness = report
        .witness
        .expect("proposal must be replayed into a witness");
    assert_eq!(witness.actions.len(), 2);
    assert_eq!(
        stepper.terminal(&witness.final_position),
        CombatTerminal::Win
    );
    assert!(witness.replay_engine_steps >= 2);
}

#[test]
fn policy_witness_proposal_with_a_false_successor_is_rejected() {
    let stepper = TinyTurnStepper::lethal_after_current_turn();
    let decision_root = root();
    let mut actions = exact_actions(&stepper, &decision_root, [ClientInput::EndTurn, PLAY]);
    actions[0].expected_successor_hash = "forged-successor".to_owned();
    let proposal = CombatPolicyWitnessProposal {
        actions,
        final_hp_hint: decision_root.position().combat.entities.player.current_hp,
    };
    let mut session = OracleCombatWitnessSession::with_policy(
        decision_root,
        OracleCombatWitnessConfig {
            generator: config(),
            generation_work_per_agenda_pop: 1,
            satisfaction: OracleCombatWitnessSatisfaction::FirstWitness,
        },
        Arc::new(FixedWitnessProposalPolicy { proposal }),
    );

    let report = session.advance(
        &stepper,
        OracleCombatWitnessQuantum::deterministic(8, 8, 32),
    );

    assert_eq!(
        report.status,
        OracleCombatWitnessStatus::ReplayMismatch(
            OracleCombatWitnessReplayError::SuccessorMismatch { action_index: 0 }
        )
    );
    assert!(report.witness.is_none());
}

#[test]
fn witness_generation_batch_reserves_engine_allowance_for_the_whole_batch() {
    let stepper = TinyTurnStepper::plain();
    let mut session = OracleCombatWitnessSession::with_policy(
        root(),
        OracleCombatWitnessConfig {
            generator: config(),
            generation_work_per_agenda_pop: 4,
            satisfaction: OracleCombatWitnessSatisfaction::BudgetOrExhaustion,
        },
        Arc::new(PreferPlayPolicy),
    );

    let report = session.advance(
        &stepper,
        OracleCombatWitnessQuantum::deterministic(1, 4, 16),
    );

    assert_eq!(report.after.agenda_pops, 1);
    assert_eq!(report.after.generation_work, 4);
    assert!(
        report.after.applied_action_transitions >= 2,
        "one agenda pop must be able to execute more than one exact transition when its generation batch requests it"
    );
}

#[test]
fn verified_witness_survives_a_serialized_search_restart() {
    let stepper = TinyTurnStepper::lethal_after_current_turn();
    let config = OracleCombatWitnessConfig {
        generator: config(),
        generation_work_per_agenda_pop: 1,
        satisfaction: OracleCombatWitnessSatisfaction::FirstWitness,
    };
    let mut original =
        OracleCombatWitnessSession::with_policy(root(), config, Arc::new(PreferPlayPolicy));
    let report = original.advance(
        &stepper,
        OracleCombatWitnessQuantum::deterministic(1_000, 1_000, 4_000),
    );
    let witness = report.witness.expect("verified witness");
    let encoded = serde_json::to_vec(&witness).expect("serialize witness");
    let decoded: OracleCombatWitness =
        serde_json::from_slice(&encoded).expect("deserialize witness");

    let mut restarted =
        OracleCombatWitnessSession::with_policy(root(), config, Arc::new(PreferPlayPolicy));
    restarted
        .restore_verified_witness(decoded)
        .expect("restore verified witness");

    let restored = restarted.witness().expect("restored incumbent");
    assert_eq!(restored.actions, witness.actions);
    assert_eq!(restored.final_position, witness.final_position);
    assert_eq!(restored.negative_log_policy, witness.negative_log_policy);
}

#[test]
fn smoke_bomb_escape_cannot_be_restored_as_a_victory_witness() {
    let stepper = TinyTurnStepper::lethal_after_current_turn();
    let config = OracleCombatWitnessConfig {
        generator: config(),
        generation_work_per_agenda_pop: 1,
        satisfaction: OracleCombatWitnessSatisfaction::FirstWitness,
    };
    let mut source =
        OracleCombatWitnessSession::with_policy(root(), config, Arc::new(PreferPlayPolicy));
    let report = source.advance(
        &stepper,
        OracleCombatWitnessQuantum::deterministic(1_000, 1_000, 4_000),
    );
    let mut escaped = report.witness.expect("verified witness");
    escaped.final_position.combat.runtime.combat_smoked = true;

    let mut restarted =
        OracleCombatWitnessSession::with_policy(root(), config, Arc::new(PreferPlayPolicy));
    let error = restarted
        .restore_verified_witness(escaped)
        .expect_err("escape is not victory");

    assert!(error.contains("Smoke Bomb escape"));
    assert!(restarted.witness().is_none());
}

#[test]
fn oracle_witness_search_retains_work_across_split_quanta() {
    let stepper = TinyTurnStepper::lethal_after_current_turn();
    let make_session = || {
        OracleCombatWitnessSession::with_policy(
            root(),
            OracleCombatWitnessConfig {
                generator: config(),
                generation_work_per_agenda_pop: 1,
                satisfaction: OracleCombatWitnessSatisfaction::FirstWitness,
            },
            Arc::new(PreferPlayPolicy),
        )
    };
    let mut one_shot = make_session();
    let one_shot_report = one_shot.advance(
        &stepper,
        OracleCombatWitnessQuantum::deterministic(1_000, 1_000, 4_000),
    );

    let mut split = make_session();
    let first = split.advance(&stepper, OracleCombatWitnessQuantum::deterministic(2, 2, 8));
    assert!(matches!(
        first.status,
        OracleCombatWitnessStatus::Partial(_)
    ));
    assert!(first.retained_state_work > 0);
    let split_report = split.advance(
        &stepper,
        OracleCombatWitnessQuantum::deterministic(998, 998, 3_992),
    );

    assert_eq!(
        one_shot_report.status,
        OracleCombatWitnessStatus::WitnessFound
    );
    assert_eq!(split_report.status, OracleCombatWitnessStatus::WitnessFound);
    assert_eq!(
        one_shot_report
            .witness
            .unwrap()
            .actions
            .into_iter()
            .map(|action| action.input)
            .collect::<Vec<_>>(),
        split_report
            .witness
            .unwrap()
            .actions
            .into_iter()
            .map(|action| action.input)
            .collect::<Vec<_>>()
    );
}

#[test]
fn witness_membership_distinguishes_generated_and_accepted_from_retained_work() {
    let stepper = TinyTurnStepper::plain();
    let decision_root = root();
    let target_hash = exact_actions(&stepper, &decision_root, [ClientInput::EndTurn])[0]
        .expected_successor_hash
        .clone();
    let mut session = OracleCombatWitnessSession::with_policy(
        decision_root,
        OracleCombatWitnessConfig {
            generator: config(),
            generation_work_per_agenda_pop: 4,
            satisfaction: OracleCombatWitnessSatisfaction::BudgetOrExhaustion,
        },
        Arc::new(PreferPlayPolicy),
    );

    session.advance(
        &stepper,
        OracleCombatWitnessQuantum::deterministic(1_024, 1_024, 4_096),
    );
    let membership = session.state_membership_by_exact_hash(&target_hash);
    let compact = session.compact_state_membership_by_exact_hash(&target_hash);
    let mut bulk = session.compact_state_memberships_by_exact_hashes([target_hash.as_str()]);
    let bulk = bulk.remove(&target_hash).expect("requested membership");

    assert!(membership.generated);
    assert!(membership.accepted);
    assert_eq!(membership.retained, membership.progress.is_some());
    assert_eq!(compact.generated, membership.generated);
    assert_eq!(compact.accepted, membership.accepted);
    assert_eq!(compact.retained, membership.retained);
    assert_eq!(bulk, compact);
    if let Some(progress) = compact.progress {
        assert_eq!(progress.anchor_states_ahead, None);
        assert_eq!(progress.guided_states_ahead, None);
    }
}

#[test]
fn budget_satisfaction_retains_a_verified_incumbent_without_stopping_on_it() {
    let stepper = TinyTurnStepper::lethal_after_current_turn();
    let mut session = OracleCombatWitnessSession::with_policy(
        root(),
        OracleCombatWitnessConfig {
            generator: config(),
            generation_work_per_agenda_pop: 1,
            satisfaction: OracleCombatWitnessSatisfaction::BudgetOrExhaustion,
        },
        Arc::new(UniformCombatActionPolicy),
    );

    let report = session.advance(
        &stepper,
        OracleCombatWitnessQuantum::deterministic(100, 100, 400),
    );

    assert!(matches!(
        report.status,
        OracleCombatWitnessStatus::Partial(_)
    ));
    assert!(
        report.witness.is_some(),
        "a verified incumbent must survive"
    );
    assert!(
        report.retained_state_work > 0,
        "quality search must continue"
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
    let prospect = ExactImmediateOptionProspect::from_option(&root, option).unwrap();
    assert_eq!(prospect.changed_potion_slots, 1);
    assert_eq!(
        prospect.occupied_potion_slots,
        ExactCountChange {
            before: 1,
            after: 0
        }
    );
    assert!(prospect.total_enemy_hp.delta() < 0);
}

#[test]
fn witness_search_records_only_gap_free_exhaustive_one_turn_loss() {
    let stepper = TinyTurnStepper::losing();
    let expected_hash = root().exact_state_hash().to_owned();
    let mut session = OracleCombatWitnessSession::with_policy(
        root(),
        OracleCombatWitnessConfig {
            generator: config(),
            generation_work_per_agenda_pop: 4,
            satisfaction: OracleCombatWitnessSatisfaction::BudgetOrExhaustion,
        },
        Arc::new(PreferPlayPolicy),
    );
    session.set_one_turn_loss_evidence_limit(1);

    let report = session.advance(
        &stepper,
        OracleCombatWitnessQuantum::deterministic(32, 128, 512),
    );

    assert_eq!(report.after.exhaustive_one_turn_losses, 1);
    let evidence = session.one_turn_loss_evidence();
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].exact_state_hash, expected_hash);
    assert!(evidence[0].actions.is_empty());
    assert!(evidence[0].terminal_loss_turn_options > 0);
}

#[test]
fn witness_search_records_exact_one_turn_viability_witness() {
    let stepper = TinyTurnStepper::plain();
    let expected_hash = root().exact_state_hash().to_owned();
    let mut session = OracleCombatWitnessSession::with_policy(
        root(),
        OracleCombatWitnessConfig {
            generator: config(),
            generation_work_per_agenda_pop: 4,
            satisfaction: OracleCombatWitnessSatisfaction::BudgetOrExhaustion,
        },
        Arc::new(PreferPlayPolicy),
    );
    session.set_one_turn_viability_evidence_limit(1);

    let report = session.advance(
        &stepper,
        OracleCombatWitnessQuantum::deterministic(16, 64, 256),
    );

    assert!(report.after.exact_one_turn_viable_states >= 1);
    let evidence = session.one_turn_viability_evidence();
    assert_eq!(evidence.len(), 1);
    assert_eq!(evidence[0].exact_state_hash, expected_hash);
    assert!(evidence[0].actions.is_empty());
    assert_eq!(
        evidence[0].witness_boundary,
        CompleteTurnOptionBoundary::NextPlayerTurn
    );
    assert!(!evidence[0].witness_turn_actions.is_empty());
}

mod agenda;
mod decision;
