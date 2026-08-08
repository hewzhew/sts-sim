use serde::{Deserialize, Serialize};

use crate::ai::combat_learning_observation::{
    combat_learning_observation_v1, CombatLearningObservationV1,
};
use crate::ai::planner_core::{LegalCandidateSet, PlannerDecisionContext, PlannerObservation};
use crate::content::potions::PotionId;
use crate::sim::combat_action_surface::{
    combat_legal_action_surface_v2, pending_choice_input_is_legal, CombatLegalActionSurfaceV2,
};
use crate::state::core::{ClientInput, EngineState, RunResult};
use crate::state::selection::SelectionResolution;

use super::{
    capture_planner_boundary_yield_v1, CombatLearningRootContextV1, PlannerBoundaryYieldKindV1,
    RunControlConfig, RunControlSession, RunControlSessionCheckpointV1,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub enum LearningObservationCompletenessV1 {
    Complete,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LearningStrategicBoundaryV1 {
    pub observation: PlannerObservation,
    pub legal_candidates: LegalCandidateSet,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LearningCombatBoundaryV1 {
    pub observation: CombatLearningObservationV1,
    pub observation_completeness: LearningObservationCompletenessV1,
    pub legal_actions: CombatLegalActionSurfaceV2,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LearningTerminalOutcomeV1 {
    pub result: RunResult,
    pub terminal_act: u8,
    pub terminal_floor: i32,
    pub terminal_hp: i32,
    pub terminal_max_hp: i32,
    pub terminal_gold: i32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LearningBoundaryV1 {
    Strategic {
        boundary: LearningStrategicBoundaryV1,
    },
    Combat {
        boundary: LearningCombatBoundaryV1,
    },
    Terminal {
        outcome: LearningTerminalOutcomeV1,
    },
    Unsupported,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LearningBoundaryKindV1 {
    Strategic,
    Combat,
    Terminal,
    Unsupported,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[repr(u8)]
#[serde(rename_all = "snake_case")]
pub enum LearningStrategicContextKindV1 {
    Map = 1,
    CardReward = 2,
    Event = 3,
    Shop = 4,
    Reward = 5,
    Campfire = 6,
    BossRelic = 7,
    RunChoice = 8,
    Treasure = 9,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LearningPublicRunContextV1 {
    pub boundary_kind: LearningBoundaryKindV1,
    pub strategic_context_kind: Option<LearningStrategicContextKindV1>,
    pub seed: u64,
    pub act: u8,
    pub floor: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub potion_ids: Vec<Option<PotionId>>,
}

impl LearningBoundaryV1 {
    pub fn kind(&self) -> LearningBoundaryKindV1 {
        match self {
            Self::Strategic { .. } => LearningBoundaryKindV1::Strategic,
            Self::Combat { .. } => LearningBoundaryKindV1::Combat,
            Self::Terminal { .. } => LearningBoundaryKindV1::Terminal,
            Self::Unsupported => LearningBoundaryKindV1::Unsupported,
        }
    }

    pub fn terminal_reward(&self) -> i8 {
        match self {
            Self::Terminal { outcome } => match &outcome.result {
                RunResult::Victory => 1,
                RunResult::Defeat => -1,
            },
            _ => 0,
        }
    }

    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Terminal { .. })
    }

    pub fn strategic_context_kind(&self) -> Option<LearningStrategicContextKindV1> {
        let Self::Strategic { boundary } = self else {
            return None;
        };
        Some(match &boundary.observation.context {
            PlannerDecisionContext::Map { .. } => LearningStrategicContextKindV1::Map,
            PlannerDecisionContext::CardReward { .. } => LearningStrategicContextKindV1::CardReward,
            PlannerDecisionContext::Event { .. } => LearningStrategicContextKindV1::Event,
            PlannerDecisionContext::Shop { .. } => LearningStrategicContextKindV1::Shop,
            PlannerDecisionContext::Reward => LearningStrategicContextKindV1::Reward,
            PlannerDecisionContext::Campfire => LearningStrategicContextKindV1::Campfire,
            PlannerDecisionContext::BossRelic => LearningStrategicContextKindV1::BossRelic,
            PlannerDecisionContext::RunChoice => LearningStrategicContextKindV1::RunChoice,
            PlannerDecisionContext::Treasure => LearningStrategicContextKindV1::Treasure,
        })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum LearningActionV1 {
    StrategicCandidate {
        candidate_id: String,
    },
    RunSelection {
        candidate_id: String,
        resolution: SelectionResolution,
    },
    CombatInput {
        input: ClientInput,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LearningStepV1 {
    pub reward: i8,
    pub terminated: bool,
    pub boundary: LearningBoundaryV1,
}

#[derive(Clone, Debug)]
pub struct LearningEnvV1 {
    session: RunControlSession,
}

impl LearningEnvV1 {
    pub fn new(config: RunControlConfig) -> Self {
        Self {
            session: RunControlSession::new(config),
        }
    }

    pub fn from_session(session: RunControlSession) -> Self {
        Self { session }
    }

    pub fn from_checkpoint(checkpoint: RunControlSessionCheckpointV1) -> Result<Self, String> {
        Ok(Self {
            session: checkpoint.into_session()?,
        })
    }

    pub fn checkpoint(&self) -> RunControlSessionCheckpointV1 {
        RunControlSessionCheckpointV1::from_session(&self.session)
    }

    pub fn combat_root_context(&self) -> Result<CombatLearningRootContextV1, String> {
        let active = self
            .session
            .active_combat
            .as_ref()
            .ok_or_else(|| "learning environment is not at a combat root".to_string())?;
        if !matches!(
            active.engine_state,
            EngineState::CombatPlayerTurn
                | EngineState::CombatProcessing
                | EngineState::PendingChoice(_)
        ) {
            return Err("learning environment is not at an active combat input state".to_string());
        }
        super::combat_learning_env::combat_learning_root_context_v1(
            &self.session,
            &active.combat_state,
        )
    }

    pub(super) fn public_run_context(
        &self,
        boundary: &LearningBoundaryV1,
    ) -> Result<LearningPublicRunContextV1, String> {
        let (hp, max_hp, gold, potion_ids) =
            if matches!(boundary, LearningBoundaryV1::Combat { .. }) {
                let active = self.session.active_combat.as_ref().ok_or_else(|| {
                    "combat public run context requires an active combat".to_owned()
                })?;
                let combat = &active.combat_state;
                (
                    combat.entities.player.current_hp,
                    combat.entities.player.max_hp,
                    combat.entities.player.gold,
                    combat
                        .entities
                        .potions
                        .iter()
                        .map(|potion| potion.as_ref().map(|potion| potion.id))
                        .collect(),
                )
            } else {
                (
                    self.session.run_state.current_hp,
                    self.session.run_state.max_hp,
                    self.session.run_state.gold,
                    self.session
                        .run_state
                        .potions
                        .iter()
                        .map(|potion| potion.as_ref().map(|potion| potion.id))
                        .collect(),
                )
            };
        Ok(LearningPublicRunContextV1 {
            boundary_kind: boundary.kind(),
            strategic_context_kind: boundary.strategic_context_kind(),
            seed: self.session.run_state.seed,
            act: self.session.run_state.act_num,
            floor: self.session.run_state.floor_num,
            hp,
            max_hp,
            gold,
            potion_ids,
        })
    }

    pub fn restore(&mut self, checkpoint: RunControlSessionCheckpointV1) -> Result<(), String> {
        self.session = checkpoint.into_session()?;
        Ok(())
    }

    pub fn into_session(self) -> RunControlSession {
        self.session
    }

    pub fn observe(&self) -> Result<LearningBoundaryV1, String> {
        if let EngineState::GameOver(result) = &self.session.engine_state {
            return Ok(LearningBoundaryV1::Terminal {
                outcome: LearningTerminalOutcomeV1 {
                    result: result.clone(),
                    terminal_act: self.session.run_state.act_num,
                    terminal_floor: self.session.run_state.floor_num,
                    terminal_hp: self.session.run_state.current_hp,
                    terminal_max_hp: self.session.run_state.max_hp,
                    terminal_gold: self.session.run_state.gold,
                },
            });
        }
        if matches!(
            self.session.engine_state,
            EngineState::CombatPlayerTurn
                | EngineState::CombatProcessing
                | EngineState::PendingChoice(_)
        ) {
            return Ok(LearningBoundaryV1::Combat {
                boundary: learning_combat_boundary_v1(&self.session)?,
            });
        }

        let segment = capture_planner_boundary_yield_v1(
            &self.session,
            PlannerBoundaryYieldKindV1::CallbackStop,
        )?;
        let [visit] = segment.visits.as_slice() else {
            return Ok(LearningBoundaryV1::Unsupported);
        };
        Ok(LearningBoundaryV1::Strategic {
            boundary: LearningStrategicBoundaryV1 {
                observation: visit.observation.clone(),
                legal_candidates: visit.legal_candidate_set.clone(),
            },
        })
    }

    pub fn step(&mut self, action: LearningActionV1) -> Result<LearningStepV1, String> {
        let prepared = self.prepare_action(action)?;
        self.step_prepared(prepared)
    }

    pub(super) fn prepare_action(
        &self,
        action: LearningActionV1,
    ) -> Result<LearningPreparedActionV1, String> {
        match action {
            LearningActionV1::StrategicCandidate { candidate_id } => {
                self.prepare_strategic_candidate(&candidate_id)
            }
            LearningActionV1::RunSelection {
                candidate_id,
                resolution,
            } => self.prepare_run_selection(&candidate_id, resolution),
            LearningActionV1::CombatInput { input } => self.prepare_combat_input(input),
        }
    }

    pub(super) fn step_prepared(
        &mut self,
        action: LearningPreparedActionV1,
    ) -> Result<LearningStepV1, String> {
        match action {
            LearningPreparedActionV1::StrategicCandidate { run_candidate_id } => {
                self.session.apply_candidate_id(&run_candidate_id)?;
            }
            LearningPreparedActionV1::RunSelection {
                run_candidate_id,
                resolution,
            } => {
                self.session.apply_learning_candidate(
                    &run_candidate_id,
                    super::RunDecisionAction::Input(ClientInput::SubmitSelection(resolution)),
                )?;
            }
            LearningPreparedActionV1::CombatInput { input } => {
                self.session
                    .apply_decision_action(super::RunDecisionAction::Input(input))?;
            }
        }
        let boundary = self.observe()?;
        Ok(LearningStepV1 {
            reward: boundary.terminal_reward(),
            terminated: boundary.is_terminal(),
            boundary,
        })
    }

    fn prepare_strategic_candidate(
        &self,
        planner_candidate_id: &str,
    ) -> Result<LearningPreparedActionV1, String> {
        let segment = capture_planner_boundary_yield_v1(
            &self.session,
            PlannerBoundaryYieldKindV1::CallbackStop,
        )?;
        let [visit] = segment.visits.as_slice() else {
            return Err(
                "strategic learning action requires a represented planner boundary".to_string(),
            );
        };
        let link = visit
            .candidate_links
            .iter()
            .find(|link| link.planner_candidate_id == planner_candidate_id)
            .ok_or_else(|| {
                format!(
                    "planner candidate '{planner_candidate_id}' is not legal at the current boundary"
                )
            })?;
        Ok(LearningPreparedActionV1::StrategicCandidate {
            run_candidate_id: link.run_candidate_id.clone(),
        })
    }

    fn prepare_combat_input(&self, input: ClientInput) -> Result<LearningPreparedActionV1, String> {
        Ok(LearningPreparedActionV1::CombatInput {
            input: prepare_learning_combat_input_v1(&self.session, input)?,
        })
    }

    fn prepare_run_selection(
        &self,
        planner_candidate_id: &str,
        resolution: SelectionResolution,
    ) -> Result<LearningPreparedActionV1, String> {
        let segment = capture_planner_boundary_yield_v1(
            &self.session,
            PlannerBoundaryYieldKindV1::CallbackStop,
        )?;
        let [visit] = segment.visits.as_slice() else {
            return Err("run selection requires a represented planner boundary".to_string());
        };
        let link = visit
            .candidate_links
            .iter()
            .find(|link| link.planner_candidate_id == planner_candidate_id)
            .ok_or_else(|| {
                format!(
                    "run selection family '{planner_candidate_id}' is not legal at the current boundary"
                )
            })?;
        let input = ClientInput::SubmitSelection(resolution.clone());
        if super::selection_surface::current_selection_input_is_allowed(&self.session, &input)
            != Some(true)
        {
            return Err(
                "run selection resolution is not legal at the current boundary".to_string(),
            );
        }
        Ok(LearningPreparedActionV1::RunSelection {
            run_candidate_id: link.run_candidate_id.clone(),
            resolution,
        })
    }
}

pub(super) fn learning_combat_boundary_v1(
    session: &RunControlSession,
) -> Result<LearningCombatBoundaryV1, String> {
    let position = session.current_combat_position_for_actions()?;
    Ok(LearningCombatBoundaryV1 {
        observation: combat_learning_observation_v1(&position.combat),
        observation_completeness: LearningObservationCompletenessV1::Complete,
        legal_actions: combat_legal_action_surface_v2(&position.engine, &position.combat),
    })
}

pub(super) fn prepare_learning_combat_input_v1(
    session: &RunControlSession,
    input: ClientInput,
) -> Result<ClientInput, String> {
    let position = session.current_combat_position_for_actions()?;
    let surface = combat_legal_action_surface_v2(&position.engine, &position.combat);
    let legal = surface.atomic_actions.contains(&input)
        || match &position.engine {
            EngineState::PendingChoice(choice) => {
                pending_choice_input_is_legal(choice, &position.combat, &input)
            }
            _ => false,
        };
    if !legal {
        return Err("combat learning input is not legal at the current boundary".to_string());
    }
    Ok(input)
}

#[derive(Clone, Debug)]
pub(super) enum LearningPreparedActionV1 {
    StrategicCandidate {
        run_candidate_id: String,
    },
    RunSelection {
        run_candidate_id: String,
        resolution: SelectionResolution,
    },
    CombatInput {
        input: ClientInput,
    },
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::content::potions::{Potion, PotionId};
    use crate::runtime::combat::CombatCard;
    use crate::sim::combat_action_surface::{
        CombatIndexedChoiceCandidateV2, CombatIndexedChoiceReasonV2,
    };
    use crate::state::core::{ActiveCombat, CombatContext, RoomCombatContext, RunResult};
    use crate::state::map::node::RoomType;
    use crate::state::{DiscoveryChoiceState, PendingChoice};

    #[test]
    fn terminal_boundary_retains_public_run_outcome_facts() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::GameOver(RunResult::Defeat);
        session.run_state.act_num = 3;
        session.run_state.floor_num = 47;
        session.run_state.current_hp = 0;
        session.run_state.max_hp = 91;
        session.run_state.gold = 123;
        let env = LearningEnvV1::from_session(session);

        let LearningBoundaryV1::Terminal { outcome } =
            env.observe().expect("observe terminal learning boundary")
        else {
            panic!("game over should expose a terminal learning boundary");
        };
        assert_eq!(outcome.result, RunResult::Defeat);
        assert_eq!(outcome.terminal_act, 3);
        assert_eq!(outcome.terminal_floor, 47);
        assert_eq!(outcome.terminal_hp, 0);
        assert_eq!(outcome.terminal_max_hp, 91);
        assert_eq!(outcome.terminal_gold, 123);
    }

    #[test]
    fn strategic_boundary_steps_by_typed_planner_candidate_and_restores_exactly() {
        let mut env = LearningEnvV1::new(RunControlConfig::default());
        let before = env.observe().expect("observe initial learning boundary");
        let LearningBoundaryV1::Strategic { boundary } = &before else {
            panic!("new run should begin at a represented strategic boundary");
        };
        let candidate_id = boundary
            .legal_candidates
            .candidates
            .first()
            .expect("initial boundary should have a legal candidate")
            .candidate_id
            .clone();
        let checkpoint = env.checkpoint();

        let step = env
            .step(LearningActionV1::StrategicCandidate { candidate_id })
            .expect("apply typed strategic learning action");
        assert_eq!(step.reward, 0);
        assert!(!step.terminated);
        assert_ne!(step.boundary, before);

        env.restore(checkpoint)
            .expect("restore exact learning checkpoint");
        assert_eq!(
            env.observe().expect("observe restored learning boundary"),
            before
        );
    }

    #[test]
    fn repeated_in_memory_checkpoint_restore_preserves_the_learning_boundary() {
        let env = LearningEnvV1::new(RunControlConfig::default());
        let expected = env.observe().expect("observe initial learning boundary");
        let checkpoint = env.checkpoint();

        for _ in 0..1_000 {
            let restored = LearningEnvV1::from_checkpoint(checkpoint.clone())
                .expect("restore in-memory learning checkpoint");
            assert_eq!(
                restored
                    .observe()
                    .expect("observe repeated restored boundary"),
                expected
            );
        }
    }

    #[test]
    fn combat_boundary_exposes_exact_actions_with_a_complete_observation() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut combat = crate::test_support::blank_test_combat();
        combat.entities.potions = vec![Some(Potion::new(PotionId::FruitJuice, 41)), None, None];
        combat.zones.hand = vec![CombatCard::new(CardId::Bash, 51)];
        let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
        monster.id = 7;
        combat.entities.monsters.push(monster);
        session.engine_state = EngineState::CombatPlayerTurn;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        let env = LearningEnvV1::from_session(session);

        let context = env
            .combat_root_context()
            .expect("capture compact combat root context");
        assert_eq!(context.turn, 1);
        assert_eq!(context.potion_slot_count, 3);
        assert_eq!(context.filled_potion_count, 1);
        assert_eq!(context.usable_potion_count, 1);
        assert_eq!(context.hand_card_count, 1);
        assert_eq!(context.monster_count, 1);

        let LearningBoundaryV1::Combat { boundary } =
            env.observe().expect("observe combat learning boundary")
        else {
            panic!("active combat should expose a combat learning boundary");
        };
        assert_eq!(
            boundary.observation_completeness,
            LearningObservationCompletenessV1::Complete
        );
        assert!(boundary
            .legal_actions
            .atomic_actions
            .contains(&ClientInput::EndTurn));
        assert!(boundary
            .legal_actions
            .atomic_actions
            .contains(&ClientInput::UsePotion {
                potion_index: 0,
                target: None,
            }));
        assert_eq!(boundary.observation.monsters[0].entity_id, 7);
        assert!(boundary
            .legal_actions
            .atomic_actions
            .contains(&ClientInput::PlayCard {
                card_index: 0,
                target: Some(7),
            }));
    }

    #[test]
    fn combat_public_context_uses_active_resources_after_prebattle() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.run_state.current_hp = 80;
        session.run_state.gold = 99;
        session.run_state.potions = vec![None, None, None];
        let mut combat = crate::test_support::blank_test_combat();
        combat.entities.player.current_hp = 79;
        combat.entities.player.gold = 101;
        combat.entities.potions = vec![Some(Potion::new(PotionId::FearPotion, 41)), None, None];
        session.engine_state = EngineState::CombatPlayerTurn;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        let env = LearningEnvV1::from_session(session);
        let boundary = env.observe().expect("observe combat boundary");

        let context = env
            .public_run_context(&boundary)
            .expect("capture current combat resources");

        assert_eq!((context.hp, context.gold), (79, 101));
        assert_eq!(
            context.potion_ids,
            vec![Some(PotionId::FearPotion), None, None]
        );
    }

    #[test]
    fn illegal_combat_input_is_rejected_without_mutating_the_boundary() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let combat = crate::test_support::blank_test_combat();
        session.engine_state = EngineState::CombatPlayerTurn;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        let mut env = LearningEnvV1::from_session(session);
        let before = env.observe().expect("observe combat learning boundary");

        let error = env
            .step(LearningActionV1::CombatInput {
                input: ClientInput::PlayCard {
                    card_index: 99,
                    target: None,
                },
            })
            .expect_err("illegal combat input should fail");

        assert!(error.contains("not legal"));
        assert_eq!(
            env.observe().expect("observe after rejected combat input"),
            before
        );
    }

    #[test]
    fn indexed_pending_choice_exposes_typed_candidates_without_an_observation_gap() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let choice = PendingChoice::DiscoverySelect(DiscoveryChoiceState {
            cards: vec![CardId::Bash, CardId::FiendFire],
            colorless: false,
            card_type: None,
            amount: 1,
            can_skip: true,
        });
        session.engine_state = EngineState::PendingChoice(choice.clone());
        session.active_combat = Some(ActiveCombat::new(
            EngineState::PendingChoice(choice),
            crate::test_support::blank_test_combat(),
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        let env = LearningEnvV1::from_session(session);

        let LearningBoundaryV1::Combat { boundary } =
            env.observe().expect("observe indexed combat choice")
        else {
            panic!("pending combat choice should remain a combat learning boundary");
        };
        let indexed = boundary
            .legal_actions
            .indexed_choice
            .expect("indexed choice semantics");

        assert_eq!(
            indexed.reason,
            CombatIndexedChoiceReasonV2::Discovery {
                colorless: false,
                card_type: None,
                amount: 1,
            }
        );
        assert_eq!(
            indexed.candidates,
            vec![
                CombatIndexedChoiceCandidateV2::Card {
                    card_id: CardId::Bash,
                    upgrades: 0,
                },
                CombatIndexedChoiceCandidateV2::Card {
                    card_id: CardId::FiendFire,
                    upgrades: 0,
                },
            ]
        );
        assert_eq!(
            boundary.observation_completeness,
            LearningObservationCompletenessV1::Complete
        );
    }
}

#[cfg(test)]
mod smoke_tests;
