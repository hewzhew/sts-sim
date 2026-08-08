use serde::{Deserialize, Serialize};

use crate::ai::combat_state_key::combat_exact_state_hash_v2;
use crate::state::core::ClientInput;

use super::learning_env::{learning_combat_boundary_v1, prepare_learning_combat_input_v1};
use super::{
    run_control_session_fingerprint_v2, CombatBaselineOutcomeV1, LearningActionV1,
    LearningCombatBoundaryV1, RunControlSession, RunControlSessionCheckpointV1, RunDecisionAction,
};

/// Exact immutable combat root shared by every replicate in one comparison group.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatLearningRootIdentityV1 {
    /// Exact normalized run-control fingerprint, including the active combat and run context.
    pub root_id: String,
    /// Exact combat-mechanics identity retained separately for diagnostics and validation.
    pub exact_combat_state_hash: String,
}

/// Small public-state summary captured once beside an exact combat root.
///
/// This is collection metadata, not a second combat observation or a policy feature schema.
/// Counts describe the exact root boundary and are intentionally independent from display text.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatLearningRootContextV1 {
    pub act: u8,
    pub floor: i32,
    pub ascension_level: u8,
    pub turn: u32,
    pub is_boss_fight: bool,
    pub is_elite_fight: bool,
    pub monster_count: u32,
    pub living_monster_count: u32,
    pub potion_slot_count: u32,
    pub filled_potion_count: u32,
    pub usable_potion_count: u32,
    pub master_deck_card_count: u32,
    pub relic_count: u32,
    pub hand_card_count: u32,
    pub hp: i32,
    pub max_hp: i32,
}

/// One stochastic replicate from an exact combat root.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatLearningEpisodeIdentityV1 {
    pub root: CombatLearningRootIdentityV1,
    pub replicate_index: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatLearningTerminalOutcomeV1 {
    pub episode: CombatLearningEpisodeIdentityV1,
    pub combat: CombatBaselineOutcomeV1,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CombatLearningBoundaryV1 {
    Decision {
        episode: CombatLearningEpisodeIdentityV1,
        boundary: LearningCombatBoundaryV1,
    },
    Terminal {
        outcome: CombatLearningTerminalOutcomeV1,
    },
}

impl CombatLearningBoundaryV1 {
    pub fn is_terminal(&self) -> bool {
        matches!(self, Self::Terminal { .. })
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatLearningStepV1 {
    pub terminated: bool,
    pub boundary: CombatLearningBoundaryV1,
}

/// Exact resumable state for one combat episode.
///
/// The immutable root and replicate identity stay beside the current session state so a
/// checkpoint cannot silently lose the grouping needed by same-root estimators.
#[derive(Clone, Debug, PartialEq)]
pub struct CombatLearningEnvCheckpointV1 {
    episode: CombatLearningEpisodeIdentityV1,
    session: RunControlSessionCheckpointV1,
    root_previous_outcome: Option<CombatBaselineOutcomeV1>,
}

/// One immutable exact combat root from which caller-numbered replicates are created.
#[derive(Clone, Debug, PartialEq)]
pub struct CombatLearningRootV1 {
    identity: CombatLearningRootIdentityV1,
    context: CombatLearningRootContextV1,
    session: RunControlSessionCheckpointV1,
    previous_outcome: Option<CombatBaselineOutcomeV1>,
}

impl CombatLearningRootV1 {
    pub fn from_session(session: RunControlSession) -> Result<Self, String> {
        let position = session.current_combat_position_for_actions()?;
        let identity = CombatLearningRootIdentityV1 {
            root_id: run_control_session_fingerprint_v2(&session),
            exact_combat_state_hash: combat_exact_state_hash_v2(&position.engine, &position.combat),
        };
        let context = combat_learning_root_context_v1(&session, &position.combat)?;
        let previous_outcome = session.last_combat_baseline().cloned();
        Ok(Self {
            identity,
            context,
            session: RunControlSessionCheckpointV1::from_session(&session),
            previous_outcome,
        })
    }

    pub fn from_checkpoint(checkpoint: RunControlSessionCheckpointV1) -> Result<Self, String> {
        Self::from_session(checkpoint.into_session()?)
    }

    pub fn identity(&self) -> &CombatLearningRootIdentityV1 {
        &self.identity
    }

    pub fn context(&self) -> &CombatLearningRootContextV1 {
        &self.context
    }

    pub fn spawn(&self, replicate_index: u32) -> Result<CombatLearningEnvV1, String> {
        let env = CombatLearningEnvV1 {
            episode: CombatLearningEpisodeIdentityV1 {
                root: self.identity.clone(),
                replicate_index,
            },
            session: self.session.clone().into_session()?,
            root_previous_outcome: self.previous_outcome.clone(),
        };
        env.observe()?;
        Ok(env)
    }
}

fn combat_learning_root_context_v1(
    session: &RunControlSession,
    combat: &crate::runtime::combat::CombatState,
) -> Result<CombatLearningRootContextV1, String> {
    let potions = &combat.entities.potions;
    Ok(CombatLearningRootContextV1 {
        act: session.run_state.act_num,
        floor: session.run_state.floor_num,
        ascension_level: combat.meta.ascension_level,
        turn: combat.turn.turn_count,
        is_boss_fight: combat.meta.is_boss_fight,
        is_elite_fight: combat.meta.is_elite_fight,
        monster_count: combat_root_count_v1("monster", combat.entities.monsters.len())?,
        living_monster_count: combat_root_count_v1(
            "living monster",
            combat
                .entities
                .monsters
                .iter()
                .filter(|monster| monster.is_alive_for_action())
                .count(),
        )?,
        potion_slot_count: combat_root_count_v1("potion slot", potions.len())?,
        filled_potion_count: combat_root_count_v1(
            "filled potion",
            potions.iter().filter(|slot| slot.is_some()).count(),
        )?,
        usable_potion_count: combat_root_count_v1(
            "usable potion",
            potions
                .iter()
                .filter_map(Option::as_ref)
                .filter(|potion| {
                    crate::content::potions::potion_can_use_in_combat_like_java(potion, combat)
                })
                .count(),
        )?,
        master_deck_card_count: combat_root_count_v1(
            "master deck card",
            combat.meta.master_deck_snapshot.len(),
        )?,
        relic_count: combat_root_count_v1("relic", combat.entities.player.relics.len())?,
        hand_card_count: combat_root_count_v1("hand card", combat.zones.hand.len())?,
        hp: combat.entities.player.current_hp,
        max_hp: combat.entities.player.max_hp,
    })
}

fn combat_root_count_v1(kind: &str, count: usize) -> Result<u32, String> {
    u32::try_from(count).map_err(|_| format!("combat learning root {kind} count exceeds u32"))
}

#[derive(Clone, Debug)]
pub struct CombatLearningEnvV1 {
    episode: CombatLearningEpisodeIdentityV1,
    session: RunControlSession,
    root_previous_outcome: Option<CombatBaselineOutcomeV1>,
}

impl CombatLearningEnvV1 {
    pub fn from_root_session(
        session: RunControlSession,
        replicate_index: u32,
    ) -> Result<Self, String> {
        CombatLearningRootV1::from_session(session)?.spawn(replicate_index)
    }

    pub fn from_root_checkpoint(
        checkpoint: RunControlSessionCheckpointV1,
        replicate_index: u32,
    ) -> Result<Self, String> {
        Self::from_root_session(checkpoint.into_session()?, replicate_index)
    }

    pub fn from_checkpoint(checkpoint: CombatLearningEnvCheckpointV1) -> Result<Self, String> {
        validate_episode_identity_v1(&checkpoint.episode)?;
        let env = Self {
            episode: checkpoint.episode,
            session: checkpoint.session.into_session()?,
            root_previous_outcome: checkpoint.root_previous_outcome,
        };
        env.observe()?;
        Ok(env)
    }

    pub fn episode_identity(&self) -> &CombatLearningEpisodeIdentityV1 {
        &self.episode
    }

    pub fn observe(&self) -> Result<CombatLearningBoundaryV1, String> {
        if self.session.active_combat.is_some() {
            return Ok(CombatLearningBoundaryV1::Decision {
                episode: self.episode.clone(),
                boundary: learning_combat_boundary_v1(&self.session)?,
            });
        }

        let combat = self
            .session
            .last_combat_baseline()
            .filter(|outcome| Some(*outcome) != self.root_previous_outcome.as_ref())
            .cloned()
            .ok_or_else(|| {
                "combat learning episode left its root combat without a new typed outcome"
                    .to_string()
            })?;
        Ok(CombatLearningBoundaryV1::Terminal {
            outcome: CombatLearningTerminalOutcomeV1 {
                episode: self.episode.clone(),
                combat,
            },
        })
    }

    pub fn step(&mut self, action: LearningActionV1) -> Result<CombatLearningStepV1, String> {
        let input = self.prepare_action(action)?;
        self.step_prepared(input)
    }

    pub(super) fn prepare_action(&self, action: LearningActionV1) -> Result<ClientInput, String> {
        if self.session.active_combat.is_none() {
            return Err("combat learning episode is already terminal".to_string());
        }
        let LearningActionV1::CombatInput { input } = action else {
            return Err("combat learning episode accepts only combat input actions".to_string());
        };
        prepare_learning_combat_input_v1(&self.session, input)
    }

    pub(super) fn step_prepared(
        &mut self,
        input: ClientInput,
    ) -> Result<CombatLearningStepV1, String> {
        self.session
            .apply_decision_action(RunDecisionAction::Input(input))?;
        let boundary = self.observe()?;
        Ok(CombatLearningStepV1 {
            terminated: boundary.is_terminal(),
            boundary,
        })
    }

    pub fn checkpoint(&self) -> CombatLearningEnvCheckpointV1 {
        CombatLearningEnvCheckpointV1 {
            episode: self.episode.clone(),
            session: RunControlSessionCheckpointV1::from_session(&self.session),
            root_previous_outcome: self.root_previous_outcome.clone(),
        }
    }

    pub fn restore(&mut self, checkpoint: CombatLearningEnvCheckpointV1) -> Result<(), String> {
        let restored = Self::from_checkpoint(checkpoint)?;
        if restored.episode != self.episode {
            return Err("combat learning checkpoint belongs to a different episode".to_string());
        }
        *self = restored;
        Ok(())
    }

    pub fn into_session(self) -> RunControlSession {
        self.session
    }
}

fn validate_episode_identity_v1(identity: &CombatLearningEpisodeIdentityV1) -> Result<(), String> {
    if identity.root.root_id.trim().is_empty() {
        return Err("combat learning root_id cannot be empty".to_string());
    }
    if identity.root.exact_combat_state_hash.trim().is_empty() {
        return Err("combat learning exact combat state hash cannot be empty".to_string());
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::monsters::exordium::jaw_worm::JawWorm;
    use crate::content::monsters::EnemyId;
    use crate::content::monsters::MonsterBehavior;
    use crate::content::potions::{Potion, PotionId};
    use crate::content::relics::{RelicId, RelicState};
    use crate::runtime::combat::CombatCard;
    use crate::sim::combat::CombatTerminal;
    use crate::state::core::{ActiveCombat, CombatContext, EngineState, RoomCombatContext};
    use crate::state::map::node::RoomType;

    #[test]
    fn same_root_replicates_share_root_identity_without_sharing_episode_identity() {
        let root = combat_root_session(20);

        let first = CombatLearningEnvV1::from_root_session(root.clone(), 0)
            .expect("construct first combat replicate");
        let second = CombatLearningEnvV1::from_root_session(root, 1)
            .expect("construct second combat replicate");

        assert_eq!(first.episode.root, second.episode.root);
        assert_ne!(first.episode, second.episode);
    }

    #[test]
    fn root_identity_includes_run_context_not_only_combat_state() {
        let first_root = combat_root_session(20);
        let mut second_root = first_root.clone();
        second_root.run_state.gold += 1;

        let first =
            CombatLearningEnvV1::from_root_session(first_root, 0).expect("construct first root");
        let second = CombatLearningEnvV1::from_root_session(second_root, 0)
            .expect("construct context-distinct root");

        assert_eq!(
            first.episode.root.exact_combat_state_hash,
            second.episode.root.exact_combat_state_hash
        );
        assert_ne!(first.episode.root.root_id, second.episode.root.root_id);
    }

    #[test]
    fn root_context_captures_compact_public_facts_once() {
        let mut session = combat_root_session(20);
        session.run_state.act_num = 2;
        session.run_state.floor_num = 23;
        let combat = &mut session.active_combat.as_mut().unwrap().combat_state;
        combat.meta.ascension_level = 7;
        combat.meta.is_elite_fight = true;
        combat.meta.master_deck_snapshot = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Defend, 2),
        ]
        .into();
        combat.entities.player.current_hp = 61;
        combat.entities.player.max_hp = 85;
        combat.entities.player.relics = vec![RelicState::new(RelicId::BurningBlood)];
        combat.entities.potions = vec![
            Some(Potion::new(PotionId::FirePotion, 1)),
            Some(Potion::new(PotionId::FairyPotion, 2)),
            None,
        ];

        let root = CombatLearningRootV1::from_session(session).expect("construct root");

        assert_eq!(
            *root.context(),
            CombatLearningRootContextV1 {
                act: 2,
                floor: 23,
                ascension_level: 7,
                turn: 1,
                is_boss_fight: false,
                is_elite_fight: true,
                monster_count: 1,
                living_monster_count: 1,
                potion_slot_count: 3,
                filled_potion_count: 2,
                usable_potion_count: 1,
                master_deck_card_count: 2,
                relic_count: 1,
                hand_card_count: 1,
                hp: 61,
                max_hp: 85,
            }
        );
    }

    #[test]
    fn checkpoint_restores_current_decision_and_rejects_cross_episode_restore() {
        let root = combat_root_session(20);
        let first = CombatLearningEnvV1::from_root_session(root.clone(), 0)
            .expect("construct first replicate");
        let checkpoint = first.checkpoint();
        let restored = CombatLearningEnvV1::from_checkpoint(checkpoint.clone())
            .expect("restore combat episode");
        assert_eq!(restored.observe().unwrap(), first.observe().unwrap());

        let mut other =
            CombatLearningEnvV1::from_root_session(root, 1).expect("construct other replicate");
        let before = other.observe().unwrap();
        let error = other
            .restore(checkpoint)
            .expect_err("cross-replicate restore must fail");
        assert!(error.contains("different episode"));
        assert_eq!(other.observe().unwrap(), before);
    }

    #[test]
    fn leaving_combat_returns_typed_combat_terminal_instead_of_run_boundary() {
        let mut env = CombatLearningEnvV1::from_root_session(combat_root_session(1), 7)
            .expect("construct lethal combat episode");

        let step = env
            .step(LearningActionV1::CombatInput {
                input: ClientInput::PlayCard {
                    card_index: 0,
                    target: Some(7),
                },
            })
            .expect("play lethal strike");

        assert!(step.terminated);
        let CombatLearningBoundaryV1::Terminal { outcome } = step.boundary else {
            panic!("combat completion must terminate the combat episode");
        };
        assert_eq!(outcome.episode.replicate_index, 7);
        assert_eq!(outcome.combat.terminal, CombatTerminal::Win);
        assert_eq!(outcome.combat.start_hp, 80);
        assert_eq!(outcome.combat.final_hp, 80);
        assert_eq!(outcome.combat.cards_played, 1);
        assert!(env
            .step(LearningActionV1::CombatInput {
                input: ClientInput::EndTurn,
            })
            .is_err());
    }

    #[test]
    fn non_combat_root_is_rejected() {
        let error =
            CombatLearningEnvV1::from_root_session(RunControlSession::new(Default::default()), 0)
                .expect_err("strategic boundary is not a combat root");

        assert!(error.contains("no active combat"));
    }

    fn combat_root_session(monster_hp: i32) -> RunControlSession {
        let mut session = RunControlSession::new(Default::default());
        let mut combat = crate::test_support::blank_test_combat();
        combat.zones.hand = vec![CombatCard::new(CardId::Strike, 51)];
        let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
        monster.id = 7;
        monster.current_hp = monster_hp;
        monster.max_hp = monster_hp;
        monster.set_planned_move_id(1);
        let plan = JawWorm::turn_plan(&combat, &monster);
        monster.set_planned_steps(plan.steps);
        monster.set_planned_visible_spec(plan.visible_spec);
        combat.entities.monsters.push(monster);
        session.engine_state = EngineState::CombatPlayerTurn;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        session
    }
}
