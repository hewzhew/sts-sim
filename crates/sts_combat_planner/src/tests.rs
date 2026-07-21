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
use crate::generator::TurnOptionGeneratorPreferredLane;
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
struct SplitGuidePolicy {
    boundary_calls: Arc<AtomicI32>,
    generation_calls: Arc<AtomicI32>,
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

impl CombatActionPolicy for SplitGuidePolicy {
    fn weights(&self, _position: &CombatPosition, choices: &[CombatPolicyChoice<'_>]) -> Vec<f64> {
        vec![1.0; choices.len()]
    }

    fn state_guides(&self, _position: &CombatPosition) -> Vec<CombatStateGuide> {
        self.boundary_calls.fetch_add(1, Ordering::Relaxed);
        vec![CombatStateGuide::new(CombatGuideLaneId::new(1), vec![1])]
    }

    fn turn_generation_guides(&self, _position: &CombatPosition) -> Vec<CombatStateGuide> {
        self.generation_calls.fetch_add(1, Ordering::Relaxed);
        vec![CombatStateGuide::new(CombatGuideLaneId::new(2), vec![2])]
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
        policy.clone(),
    );

    assert_eq!(boundary_calls.load(Ordering::Relaxed), 1);
    assert_eq!(generation_calls.load(Ordering::Relaxed), 1);
    let generator = TurnOptionGeneratorSession::with_policy(root(), config(), policy);
    assert!(!generator.has_guide_lane(CombatGuideLaneId::new(1)));
    assert!(generator.has_guide_lane(CombatGuideLaneId::new(2)));
}

#[derive(Clone)]
struct TinyTurnStepper {
    opens_selection: bool,
    duplicate_play_surface: bool,
    lethal_from_turn: Option<u32>,
    terminal_loss: bool,
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
fn layered_search_keeps_a_complete_turn_sibling_until_the_next_layer() {
    let stepper = TinyTurnStepper::lethal_after_current_turn();
    let report = search_layered_combat_witness(
        root(),
        LayeredCombatWitnessConfig {
            generator: config(),
            beam_width: 4,
            retained_per_view: 2,
            minimum_generation_work_per_layer: 16,
            maximum_generation_work_per_layer: 64,
            candidate_pool_multiplier: 2,
            generation_quantum_work: 4,
            max_turn_layers: 4,
        },
        LayeredCombatWitnessBudget {
            max_generation_work: 128,
            max_engine_steps: 512,
            deadline: None,
        },
        Arc::new(PreferPlayPolicy),
        &stepper,
    );

    assert_eq!(report.status, LayeredCombatWitnessStatus::WitnessFound);
    let witness = report.witness.expect("layered search should find a win");
    assert_eq!(witness.actions[0].input, ClientInput::EndTurn);
    assert_eq!(witness.actions[1].input, PLAY);
    assert_eq!(
        witness.discovery_source,
        OracleCombatWitnessDiscoverySource::PlannerSearch
    );
    assert_eq!(report.layers[0].player_turn, 1);
    assert_eq!(report.layers[0].retained_next_turn_states, 2);
}

#[test]
fn layered_search_recovers_a_winning_sibling_from_the_next_beam_window() {
    let stepper = TinyTurnStepper::lethal_after_current_turn();
    let report = search_layered_combat_witness(
        root(),
        LayeredCombatWitnessConfig {
            generator: config(),
            beam_width: 1,
            retained_per_view: 1,
            minimum_generation_work_per_layer: 16,
            maximum_generation_work_per_layer: 64,
            candidate_pool_multiplier: 2,
            generation_quantum_work: 4,
            max_turn_layers: 4,
        },
        LayeredCombatWitnessBudget {
            max_generation_work: 256,
            max_engine_steps: 1_024,
            deadline: None,
        },
        Arc::new(PreferPlayPolicy),
        &stepper,
    );

    assert_eq!(report.status, LayeredCombatWitnessStatus::WitnessFound);
    let witness = report.witness.expect("the deferred sibling should win");
    assert_eq!(witness.actions[0].input, ClientInput::EndTurn);
    assert_eq!(witness.actions[1].input, PLAY);
    assert!(report.counters.deferred_windows > 0);
    assert!(report.counters.recovered_window_expansions > 0);
}

#[test]
fn layered_session_resumes_mid_layer_without_repaying_generation_work() {
    let layered_config = LayeredCombatWitnessConfig {
        generator: config(),
        beam_width: 1,
        retained_per_view: 1,
        minimum_generation_work_per_layer: 16,
        maximum_generation_work_per_layer: 64,
        candidate_pool_multiplier: 2,
        generation_quantum_work: 4,
        max_turn_layers: 4,
    };
    let one_shot = search_layered_combat_witness(
        root(),
        layered_config,
        LayeredCombatWitnessBudget {
            max_generation_work: 256,
            max_engine_steps: 1_024,
            deadline: None,
        },
        Arc::new(PreferPlayPolicy),
        &TinyTurnStepper::lethal_after_current_turn(),
    );

    let mut session = LayeredCombatWitnessSession::with_policy(
        root(),
        layered_config,
        Arc::new(PreferPlayPolicy),
    );
    let stepper = TinyTurnStepper::lethal_after_current_turn();
    let first = session.advance(
        LayeredCombatWitnessQuantum {
            additional_generation_work: 4,
            additional_engine_steps: 16,
            deadline: None,
        },
        &stepper,
    );
    assert_eq!(
        first.status,
        LayeredCombatWitnessStatus::Partial(LayeredCombatWitnessInterruption::GenerationWorkBudget)
    );
    assert_eq!(first.counters.generation_work, 4);
    assert_eq!(first.counters.completed_layers, 0);

    let resumed = session.advance(
        LayeredCombatWitnessQuantum {
            additional_generation_work: 252,
            additional_engine_steps: 1_008,
            deadline: None,
        },
        &stepper,
    );
    assert_eq!(resumed.status, LayeredCombatWitnessStatus::WitnessFound);
    assert_eq!(resumed.counters, one_shot.counters);
    assert_eq!(
        resumed.witness.as_ref().map(|witness| &witness.actions),
        one_shot.witness.as_ref().map(|witness| &witness.actions)
    );
}

#[test]
fn candidate_continuation_race_resumes_multiple_candidates_until_one_wins() {
    let source_config = LayeredCombatWitnessConfig {
        generator: config(),
        beam_width: 1,
        retained_per_view: 1,
        minimum_generation_work_per_layer: 16,
        maximum_generation_work_per_layer: 64,
        candidate_pool_multiplier: 2,
        generation_quantum_work: 4,
        max_turn_layers: 1,
    };
    let policy: SharedCombatActionPolicy = Arc::new(PreferPlayPolicy);
    let mut source =
        LayeredCombatWitnessSession::with_policy(root(), source_config, policy.clone());
    let stepper = TinyTurnStepper::lethal_after_current_turn();
    let source_report = source.advance(
        LayeredCombatWitnessQuantum {
            additional_generation_work: 64,
            additional_engine_steps: 256,
            deadline: None,
        },
        &stepper,
    );
    assert_eq!(
        source_report.status,
        LayeredCombatWitnessStatus::Partial(LayeredCombatWitnessInterruption::TurnLayerBudget)
    );
    let windows = source.deferred_windows();
    assert_eq!(windows.len(), 2);
    let combined_window = LayeredCombatDeferredWindow {
        relative_turn_depth: 1,
        window_discrepancy: 0,
        source_window_index: 0,
        candidates: windows
            .into_iter()
            .flat_map(|window| window.candidates)
            .collect(),
    };
    let continuation = LayeredCombatWitnessConfig {
        max_turn_layers: 3,
        ..source_config
    };
    let mut race = LayeredCombatCandidateRaceSession::from_window(
        root(),
        combined_window,
        LayeredCombatCandidateRaceConfig {
            continuation,
            service_quantum_work: 4,
        },
        policy,
    );
    let report = race.advance(
        LayeredCombatWitnessQuantum {
            additional_generation_work: 256,
            additional_engine_steps: 1_024,
            deadline: None,
        },
        &stepper,
    );

    assert_eq!(
        report.status,
        LayeredCombatCandidateRaceStatus::WitnessFound
    );
    let witness = report.witness.expect("the second candidate should win");
    assert_eq!(witness.actions[0].input, ClientInput::EndTurn);
    assert_eq!(witness.actions[1].input, PLAY);
    assert!(report.candidates[0].generation_work > 0);
    assert!(report.candidates[1].generation_work > 0);
    assert!(report.candidates[1].found_witness);
}

#[test]
fn candidate_race_preserves_parent_local_deferred_windows_and_prefixes() {
    let config = LayeredCombatWitnessConfig {
        generator: config(),
        beam_width: 1,
        retained_per_view: 1,
        minimum_generation_work_per_layer: 16,
        maximum_generation_work_per_layer: 64,
        candidate_pool_multiplier: 2,
        generation_quantum_work: 4,
        max_turn_layers: 1,
    };
    let policy: SharedCombatActionPolicy = Arc::new(PreferPlayPolicy);
    let stepper = TinyTurnStepper::plain();
    let mut source = LayeredCombatWitnessSession::with_policy(root(), config, policy.clone());
    let source_report = source.advance(
        LayeredCombatWitnessQuantum {
            additional_generation_work: 64,
            additional_engine_steps: 256,
            deadline: None,
        },
        &stepper,
    );
    assert_eq!(
        source_report.status,
        LayeredCombatWitnessStatus::Partial(LayeredCombatWitnessInterruption::TurnLayerBudget)
    );
    let combined_window = LayeredCombatDeferredWindow {
        relative_turn_depth: 1,
        window_discrepancy: 0,
        source_window_index: 0,
        candidates: source
            .deferred_windows()
            .into_iter()
            .flat_map(|window| window.candidates)
            .collect(),
    };
    let parent_prefixes = combined_window
        .candidates
        .iter()
        .map(|candidate| candidate.actions.clone())
        .collect::<Vec<_>>();
    let mut race = LayeredCombatCandidateRaceSession::from_window(
        root(),
        combined_window,
        LayeredCombatCandidateRaceConfig {
            continuation: config,
            service_quantum_work: 4,
        },
        policy,
    );
    let report = race.advance(
        LayeredCombatWitnessQuantum {
            additional_generation_work: 256,
            additional_engine_steps: 1_024,
            deadline: None,
        },
        &stepper,
    );
    assert_eq!(
        report.status,
        LayeredCombatCandidateRaceStatus::CandidatesExhausted
    );

    let lineage_windows = race.deferred_lineage_windows();
    assert!(!lineage_windows.is_empty());
    for lineage in lineage_windows {
        let prefix = &parent_prefixes[lineage.parent_candidate_index];
        for candidate in lineage.window.candidates {
            assert!(candidate.actions.starts_with(prefix));
            assert!(candidate.actions.len() > prefix.len());
        }
    }
}

#[test]
fn lineage_parent_consensus_uses_ordinal_guide_evidence() {
    let mut weaker_position = root().position().clone();
    weaker_position.combat.turn.energy = 1;
    let mut stronger_position = root().position().clone();
    stronger_position.combat.turn.energy = 0;
    let windows = vec![
        LayeredCombatLineageWindow {
            parent_candidate_index: 0,
            parent_exact_state_hash: "weaker".to_owned(),
            window: LayeredCombatDeferredWindow {
                relative_turn_depth: 2,
                window_discrepancy: 0,
                source_window_index: 0,
                candidates: vec![LayeredCombatFrontierState {
                    exact_state_hash: "weaker-child".to_owned(),
                    position: weaker_position,
                    actions: Vec::new(),
                    negative_log_policy: 2.0,
                }],
            },
        },
        LayeredCombatLineageWindow {
            parent_candidate_index: 1,
            parent_exact_state_hash: "stronger".to_owned(),
            window: LayeredCombatDeferredWindow {
                relative_turn_depth: 2,
                window_discrepancy: 0,
                source_window_index: 0,
                candidates: vec![LayeredCombatFrontierState {
                    exact_state_hash: "stronger-child".to_owned(),
                    position: stronger_position,
                    actions: Vec::new(),
                    negative_log_policy: 1.0,
                }],
            },
        },
    ];

    let ranked = rank_layered_combat_lineage_parents(&windows, &SharedGuidePolicy);

    assert_eq!(ranked.len(), 2);
    assert_eq!(ranked[0].parent_candidate_index, 1);
    assert_eq!(ranked[0].consensus_rank, 1);
    assert_eq!(ranked[0].anchor_rank, 1);
    assert_eq!(ranked[0].guide_ranks, vec![(SHARED_TEST_GUIDE, 1)]);
    assert_eq!(ranked[1].parent_candidate_index, 0);
    assert_eq!(ranked[1].consensus_rank, 2);
}

#[test]
fn lineage_portfolio_recurses_at_turn_boundaries_and_replays_the_winner() {
    let source_config = LayeredCombatWitnessConfig {
        generator: config(),
        beam_width: 1,
        retained_per_view: 1,
        minimum_generation_work_per_layer: 16,
        maximum_generation_work_per_layer: 64,
        candidate_pool_multiplier: 2,
        generation_quantum_work: 4,
        max_turn_layers: 1,
    };
    let policy: SharedCombatActionPolicy = Arc::new(PreferPlayPolicy);
    let stepper = TinyTurnStepper::lethal_after_current_turn();
    let mut source =
        LayeredCombatWitnessSession::with_policy(root(), source_config, policy.clone());
    source.advance(
        LayeredCombatWitnessQuantum {
            additional_generation_work: 64,
            additional_engine_steps: 256,
            deadline: None,
        },
        &stepper,
    );
    let lineages = source
        .deferred_windows()
        .into_iter()
        .enumerate()
        .map(
            |(parent_candidate_index, window)| LayeredCombatLineageWindow {
                parent_candidate_index,
                parent_exact_state_hash: window.candidates[0].exact_state_hash.clone(),
                window,
            },
        )
        .collect::<Vec<_>>();
    let mut portfolio = LayeredCombatLineagePortfolioSession::from_lineage_windows(
        root(),
        lineages,
        LayeredCombatLineagePortfolioConfig {
            candidate_race: LayeredCombatCandidateRaceConfig {
                continuation: LayeredCombatWitnessConfig {
                    max_turn_layers: 3,
                    ..source_config
                },
                service_quantum_work: 4,
            },
            parents_per_view: 1,
            windows_per_parent: 1,
            service_quantum_work: 16,
            recursive_splits: 1,
        },
        policy,
    );

    let report = portfolio.advance(
        LayeredCombatWitnessQuantum {
            additional_generation_work: 256,
            additional_engine_steps: 1_024,
            deadline: None,
        },
        &stepper,
    );

    assert_eq!(
        report.status,
        LayeredCombatLineagePortfolioStatus::WitnessFound
    );
    assert!(report.selected_parent_count >= 1);
    assert!(report
        .entries
        .iter()
        .any(|entry| entry.recursive_splits_remaining == 0));
    let witness = report.witness.expect("selected lineage should win");
    assert_eq!(
        stepper.terminal(&witness.final_position),
        CombatTerminal::Win
    );
    assert!(witness.actions.len() >= 2);
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
fn shared_guide_publishes_and_services_the_best_partial_expansion() {
    let stepper = TinyTurnStepper::plain();
    let mut generator =
        TurnOptionGeneratorSession::with_policy(root(), config(), Arc::new(SharedGuidePolicy));

    assert_eq!(
        generator
            .best_retained_guide_promise(SHARED_TEST_GUIDE)
            .expect("root guide promise")
            .rank,
        CombatStateGuideRank::new(vec![0])
    );

    generator.advance(&stepper, CombatPlanningQuantum::deterministic(1, 4));
    generator.prefer_lane(TurnOptionGeneratorPreferredLane::Guide(SHARED_TEST_GUIDE));
    generator.advance(&stepper, CombatPlanningQuantum::deterministic(1, 4));

    assert_eq!(
        generator
            .best_retained_guide_promise(SHARED_TEST_GUIDE)
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
    assert_eq!(
        witness.discovery_source,
        OracleCombatWitnessDiscoverySource::PlannerSearch
    );
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
    let mut session = OracleCombatWitnessSession::new(
        decision_root,
        OracleCombatWitnessConfig {
            generator: config(),
            generation_work_per_agenda_pop: 1,
            satisfaction: OracleCombatWitnessSatisfaction::FirstWitness,
        },
    );
    assert!(session.offer_witness_proposal(proposal));

    let report = session.advance(
        &stepper,
        OracleCombatWitnessQuantum::deterministic(8, 8, 32),
    );

    assert_eq!(report.status, OracleCombatWitnessStatus::WitnessFound);
    assert_eq!(report.after.policy_witness_proposals, 1);
    assert_eq!(report.after.generation_work, 0);
    assert_eq!(
        report.after.exact_states, 2,
        "the verified next-turn boundary from an advisor line joins the canonical graph"
    );
    let witness = report
        .witness
        .expect("proposal must be replayed into a witness");
    assert_eq!(
        witness.discovery_source,
        OracleCombatWitnessDiscoverySource::PolicyProposal
    );
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
    let mut session = OracleCombatWitnessSession::new(
        decision_root,
        OracleCombatWitnessConfig {
            generator: config(),
            generation_work_per_agenda_pop: 1,
            satisfaction: OracleCombatWitnessSatisfaction::FirstWitness,
        },
    );
    assert!(session.offer_witness_proposal(proposal));

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
fn root_action_families_attribute_generation_and_downstream_service() {
    let stepper = TinyTurnStepper::plain();
    let mut session = OracleCombatWitnessSession::with_policy(
        root(),
        OracleCombatWitnessConfig {
            generator: config(),
            generation_work_per_agenda_pop: 4,
            satisfaction: OracleCombatWitnessSatisfaction::BudgetOrExhaustion,
        },
        Arc::new(UniformCombatActionPolicy),
    );

    session.advance(
        &stepper,
        OracleCombatWitnessQuantum::deterministic(256, 256, 1_024),
    );
    let families = session.root_action_families();
    let play = families
        .iter()
        .find(|family| family.first_action == PLAY)
        .expect("play-rooted family");
    let end_turn = families
        .iter()
        .find(|family| family.first_action == ClientInput::EndTurn)
        .expect("end-turn-rooted family");

    assert!(play.completed_root_turn_options > 0);
    assert!(play.unique_root_successors > 0);
    assert!(play.accepted_root_successors > 0);
    assert!(end_turn.completed_root_turn_options > 0);
    assert!(families.iter().any(|family| {
        family.retained_descendants > 0 && family.descendant_generation_work > 0
    }));
    assert!(families.iter().all(|family| {
        family.retained_descendants <= family.accepted_descendants
            && family.retained_root_successors <= family.accepted_root_successors
    }));
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
