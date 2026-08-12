//! Canonical combat belief environment over public history and exact particles.

use std::error::Error;
use std::fmt::{Display, Formatter};

use serde::{Deserialize, Serialize};

use crate::agent::information::action::{
    project_public_combat_actions_v1, resolve_public_combat_action_v1,
    CombatActionResolutionErrorV1, PublicCombatActionChoiceV1, PublicCombatActionSurfaceV1,
};
use crate::agent::information::run::PublicCombatRunContextV1;
use crate::agent::information::state::{public_combat_state_v1, PublicCombatStateV1};
use crate::sim::combat::{CombatPosition, CombatStepLimits, CombatStepper, CombatTerminal};

use super::combat::{
    CombatBeliefParticleV1, CombatBeliefSamplerV1, CombatBeliefSamplingErrorV1,
    CombatBeliefSamplingRequestV1,
};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPublicDecisionV1 {
    pub run_context: PublicCombatRunContextV1,
    pub observation: PublicCombatStateV1,
    pub actions: PublicCombatActionSurfaceV1,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CombatPublicBoundaryV1 {
    Decision {
        decision: CombatPublicDecisionV1,
    },
    Terminal {
        run_context: PublicCombatRunContextV1,
        observation: PublicCombatStateV1,
        outcome: CombatTerminal,
    },
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPublicHistoryEntryV1 {
    pub chosen_action: Option<PublicCombatActionChoiceV1>,
    pub boundary: CombatPublicBoundaryV1,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatPublicHistoryV1 {
    entries: Vec<CombatPublicHistoryEntryV1>,
}

impl CombatPublicHistoryV1 {
    pub fn root(boundary: CombatPublicBoundaryV1) -> Self {
        Self {
            entries: vec![CombatPublicHistoryEntryV1 {
                chosen_action: None,
                boundary,
            }],
        }
    }

    pub fn entries(&self) -> &[CombatPublicHistoryEntryV1] {
        &self.entries
    }

    pub fn current(&self) -> &CombatPublicBoundaryV1 {
        &self
            .entries
            .last()
            .expect("combat public history always contains its root")
            .boundary
    }

    fn successor(
        &self,
        chosen_action: PublicCombatActionChoiceV1,
        boundary: CombatPublicBoundaryV1,
    ) -> Self {
        let mut next = self.clone();
        next.entries.push(CombatPublicHistoryEntryV1 {
            chosen_action: Some(chosen_action),
            boundary,
        });
        next
    }
}

#[derive(Clone, Debug)]
pub struct CombatBeliefEnvironmentV1 {
    public_history: CombatPublicHistoryV1,
    particles: Vec<CombatBeliefParticleV1>,
}

impl CombatBeliefEnvironmentV1 {
    pub fn from_sampler(
        run_context: PublicCombatRunContextV1,
        exact_source: &CombatPosition,
        sampler: &impl CombatBeliefSamplerV1,
    ) -> Result<Self, CombatBeliefEnvironmentErrorV1> {
        let root = public_boundary_for_position_v1(run_context, exact_source)?;
        if !matches!(root, CombatPublicBoundaryV1::Decision { .. }) {
            return Err(CombatBeliefEnvironmentErrorV1::TerminalRoot);
        }
        let public_history = CombatPublicHistoryV1::root(root);
        Self::from_history_and_sampler(public_history, exact_source, sampler)
    }

    /// Rebuild a particle population at any represented public-history node.
    /// The sampler receives the complete prefix and declares how much of it
    /// actually conditions its distribution.
    pub fn from_history_and_sampler(
        public_history: CombatPublicHistoryV1,
        exact_source: &CombatPosition,
        sampler: &impl CombatBeliefSamplerV1,
    ) -> Result<Self, CombatBeliefEnvironmentErrorV1> {
        if !matches!(
            public_history.current(),
            CombatPublicBoundaryV1::Decision { .. }
        ) {
            return Err(CombatBeliefEnvironmentErrorV1::TerminalRoot);
        }
        let particles = sampler
            .sample(CombatBeliefSamplingRequestV1 {
                public_history: &public_history,
                exact_source,
            })
            .map_err(CombatBeliefEnvironmentErrorV1::Sampling)?;
        Self::from_particles(public_history, particles)
    }

    pub fn from_particles(
        public_history: CombatPublicHistoryV1,
        mut particles: Vec<CombatBeliefParticleV1>,
    ) -> Result<Self, CombatBeliefEnvironmentErrorV1> {
        if particles.is_empty() {
            return Err(CombatBeliefEnvironmentErrorV1::EmptyPopulation);
        }
        let total_mass = particles.iter().try_fold(0.0, |total, particle| {
            let mass = particle.probability_mass();
            if !mass.is_finite() || mass <= 0.0 {
                Err(CombatBeliefEnvironmentErrorV1::InvalidProbabilityMass)
            } else {
                Ok(total + mass)
            }
        })?;
        for particle in &mut particles {
            particle.set_probability_mass(particle.probability_mass() / total_mass);
            let actual = public_boundary_for_position_v1(
                current_run_context_v1(public_history.current()).clone(),
                particle.private_position(),
            )?;
            if &actual != public_history.current() {
                return Err(CombatBeliefEnvironmentErrorV1::ParticleBoundaryMismatch);
            }
        }
        Ok(Self {
            public_history,
            particles,
        })
    }

    pub fn public_history(&self) -> &CombatPublicHistoryV1 {
        &self.public_history
    }

    pub fn particles(&self) -> &[CombatBeliefParticleV1] {
        &self.particles
    }

    pub fn step(
        &self,
        action: PublicCombatActionChoiceV1,
        stepper: &impl CombatStepper,
        max_engine_steps: usize,
    ) -> Result<Vec<CombatBeliefChanceBranchV1>, CombatBeliefEnvironmentErrorV1> {
        let CombatPublicBoundaryV1::Decision { decision } = self.public_history.current() else {
            return Err(CombatBeliefEnvironmentErrorV1::TerminalBoundary);
        };
        let mut grouped: Vec<(CombatPublicBoundaryV1, Vec<CombatBeliefParticleV1>)> = Vec::new();
        for particle in &self.particles {
            let position = particle.private_position();
            let projection = project_public_combat_actions_v1(&position.engine, &position.combat)
                .map_err(|error| {
                CombatBeliefEnvironmentErrorV1::ActionProjection(error.to_string())
            })?;
            if projection.public != decision.actions {
                return Err(CombatBeliefEnvironmentErrorV1::ParticleBoundaryMismatch);
            }
            let input = resolve_public_combat_action_v1(&projection, &action)
                .map_err(CombatBeliefEnvironmentErrorV1::ActionResolution)?;
            if !stepper.is_legal_action(position, &input) {
                return Err(CombatBeliefEnvironmentErrorV1::ResolvedActionIllegal);
            }
            let result = stepper.apply_to_stable(
                position,
                input,
                CombatStepLimits {
                    max_engine_steps,
                    deadline: None,
                },
            );
            if result.truncated || result.timed_out {
                return Err(CombatBeliefEnvironmentErrorV1::TransitionTruncated);
            }
            let boundary =
                public_boundary_for_position_v1(decision.run_context.clone(), &result.position)?;
            let successor = CombatBeliefParticleV1::from_private_position(
                particle.origin(),
                particle.probability_mass(),
                result.position,
            );
            if let Some((_, branch_particles)) = grouped
                .iter_mut()
                .find(|(existing, _)| existing == &boundary)
            {
                branch_particles.push(successor);
            } else {
                grouped.push((boundary, vec![successor]));
            }
        }

        grouped
            .into_iter()
            .map(|(boundary, mut particles)| {
                let probability = particles
                    .iter()
                    .map(CombatBeliefParticleV1::probability_mass)
                    .sum::<f64>();
                for particle in &mut particles {
                    particle.set_probability_mass(particle.probability_mass() / probability);
                }
                let history = self.public_history.successor(action.clone(), boundary);
                Ok(CombatBeliefChanceBranchV1 {
                    probability,
                    environment: CombatBeliefEnvironmentV1::from_particles(history, particles)?,
                })
            })
            .collect()
    }
}

#[derive(Clone, Debug)]
pub struct CombatBeliefChanceBranchV1 {
    probability: f64,
    environment: CombatBeliefEnvironmentV1,
}

impl CombatBeliefChanceBranchV1 {
    pub fn probability(&self) -> f64 {
        self.probability
    }

    pub fn environment(&self) -> &CombatBeliefEnvironmentV1 {
        &self.environment
    }

    pub fn into_environment(self) -> CombatBeliefEnvironmentV1 {
        self.environment
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CombatBeliefEnvironmentErrorV1 {
    EmptyPopulation,
    InvalidProbabilityMass,
    TerminalRoot,
    TerminalBoundary,
    ParticleBoundaryMismatch,
    ActionProjection(String),
    ActionResolution(CombatActionResolutionErrorV1),
    ResolvedActionIllegal,
    TransitionTruncated,
    Sampling(CombatBeliefSamplingErrorV1),
}

impl Display for CombatBeliefEnvironmentErrorV1 {
    fn fmt(&self, formatter: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::EmptyPopulation => formatter.write_str("combat belief population is empty"),
            Self::InvalidProbabilityMass => {
                formatter.write_str("combat belief particle mass must be finite and positive")
            }
            Self::TerminalRoot => formatter.write_str("combat belief root is terminal"),
            Self::TerminalBoundary => formatter.write_str("combat belief boundary is terminal"),
            Self::ParticleBoundaryMismatch => {
                formatter.write_str("combat belief particle does not match its public history")
            }
            Self::ActionProjection(error) => {
                write!(formatter, "cannot project particle actions: {error}")
            }
            Self::ActionResolution(error) => write!(formatter, "cannot resolve action: {error}"),
            Self::ResolvedActionIllegal => {
                formatter.write_str("resolved public action is not exact-engine legal")
            }
            Self::TransitionTruncated => {
                formatter.write_str("combat belief transition did not reach a stable boundary")
            }
            Self::Sampling(error) => write!(formatter, "cannot sample belief particles: {error}"),
        }
    }
}

impl Error for CombatBeliefEnvironmentErrorV1 {}

fn public_boundary_for_position_v1(
    run_context: PublicCombatRunContextV1,
    position: &CombatPosition,
) -> Result<CombatPublicBoundaryV1, CombatBeliefEnvironmentErrorV1> {
    let observation = public_combat_state_v1(&position.combat);
    let outcome = crate::sim::combat::combat_terminal(&position.engine, &position.combat);
    if outcome != CombatTerminal::Unresolved {
        return Ok(CombatPublicBoundaryV1::Terminal {
            run_context,
            observation,
            outcome,
        });
    }
    let actions = project_public_combat_actions_v1(&position.engine, &position.combat)
        .map_err(|error| CombatBeliefEnvironmentErrorV1::ActionProjection(error.to_string()))?
        .public;
    Ok(CombatPublicBoundaryV1::Decision {
        decision: CombatPublicDecisionV1 {
            run_context,
            observation,
            actions,
        },
    })
}

fn current_run_context_v1(boundary: &CombatPublicBoundaryV1) -> &PublicCombatRunContextV1 {
    match boundary {
        CombatPublicBoundaryV1::Decision { decision } => &decision.run_context,
        CombatPublicBoundaryV1::Terminal { run_context, .. } => run_context,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::agent::belief::CombatBeliefParticleOriginV1;
    use crate::agent::belief::{
        CombatBeliefConditioningV1, CombatBeliefSamplerV1, IndependentStreamsCombatBeliefSamplerV1,
    };
    use crate::agent::information::action::PublicCombatAtomicActionV1;
    use crate::agent::information::run::PublicCombatRunContextGapV1;
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::content::potions::{Potion, PotionId};
    use crate::runtime::combat::{CombatCard, Intent};
    use crate::sim::combat::EngineCombatStepper;
    use crate::state::core::EngineState;

    #[test]
    fn one_public_action_resolves_against_each_particles_private_handles() {
        let left = potion_combat(7, 71);
        let mut right = potion_combat(700, 7100);
        right.entities.monsters[0].current_hp = left.entities.monsters[0].current_hp;
        right.entities.monsters[0].max_hp = left.entities.monsters[0].max_hp;
        let left_position = CombatPosition::new(EngineState::CombatPlayerTurn, left);
        let right_position = CombatPosition::new(EngineState::CombatPlayerTurn, right);
        let history = CombatPublicHistoryV1::root(
            public_boundary_for_position_v1(detached_context(), &left_position).unwrap(),
        );
        let environment = CombatBeliefEnvironmentV1::from_particles(
            history,
            vec![particle(1, left_position), particle(2, right_position)],
        )
        .unwrap();
        let CombatPublicBoundaryV1::Decision { decision } = environment.public_history().current()
        else {
            panic!("root must be a decision");
        };
        let action_ordinal = decision
            .actions
            .atomic_actions
            .iter()
            .position(|action| {
                matches!(
                    action,
                    PublicCombatAtomicActionV1::UsePotion {
                        potion_index: 0,
                        target_monster_index: Some(0)
                    }
                )
            })
            .expect("public fire-potion action");

        let branches = environment
            .step(
                PublicCombatActionChoiceV1::Atomic { action_ordinal },
                &EngineCombatStepper,
                100,
            )
            .expect("step every particle");

        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].probability(), 1.0);
        assert_eq!(branches[0].environment().particles().len(), 2);
        for particle in branches[0].environment().particles() {
            assert!(particle.private_position().combat.entities.potions[0].is_none());
            assert!(particle.private_position().combat.entities.monsters[0].current_hp <= 0);
        }
    }

    #[test]
    fn visible_successors_split_particles_into_conditional_chance_branches() {
        let mut left = decision_combat();
        left.zones.hand = vec![CombatCard::new(CardId::BattleTrance, 10)];
        left.zones.draw_pile = vec![
            CombatCard::new(CardId::Strike, 21),
            CombatCard::new(CardId::Defend, 22),
            CombatCard::new(CardId::Bash, 23),
        ]
        .into();
        let mut right = left.clone();
        right.zones.hand[0].uuid = 100;
        right.zones.draw_pile = vec![
            CombatCard::new(CardId::Bash, 230),
            CombatCard::new(CardId::Strike, 210),
            CombatCard::new(CardId::Defend, 220),
        ]
        .into();
        let left_position = CombatPosition::new(EngineState::CombatPlayerTurn, left);
        let right_position = CombatPosition::new(EngineState::CombatPlayerTurn, right);
        let history = CombatPublicHistoryV1::root(
            public_boundary_for_position_v1(detached_context(), &left_position).unwrap(),
        );
        let environment = CombatBeliefEnvironmentV1::from_particles(
            history,
            vec![particle(11, left_position), particle(22, right_position)],
        )
        .unwrap();
        let CombatPublicBoundaryV1::Decision { decision } = environment.public_history().current()
        else {
            panic!("root must be a decision");
        };
        let action_ordinal = decision
            .actions
            .atomic_actions
            .iter()
            .position(|action| {
                matches!(
                    action,
                    PublicCombatAtomicActionV1::PlayCard {
                        hand_index: 0,
                        target_monster_index: None
                    }
                )
            })
            .expect("public Battle Trance action");

        let branches = environment
            .step(
                PublicCombatActionChoiceV1::Atomic { action_ordinal },
                &EngineCombatStepper,
                100,
            )
            .expect("step every particle");

        assert_eq!(branches.len(), 2);
        assert!(branches.iter().all(|branch| branch.probability() == 0.5));
        assert!(branches
            .iter()
            .all(|branch| branch.environment().particles().len() == 1));
        assert!(branches
            .iter()
            .all(|branch| { branch.environment().public_history().entries().len() == 2 }));
        let hands = branches
            .iter()
            .map(
                |branch| match branch.environment().public_history().current() {
                    CombatPublicBoundaryV1::Decision { decision } => decision
                        .observation
                        .cards
                        .hand
                        .cards
                        .iter()
                        .map(|card| card.card_id)
                        .collect::<Vec<_>>(),
                    CombatPublicBoundaryV1::Terminal { .. } => Vec::new(),
                },
            )
            .collect::<Vec<_>>();
        assert_ne!(hands[0], hands[1]);
    }

    #[test]
    fn declared_sampler_builds_a_normalized_environment_at_the_public_root() {
        let mut combat = decision_combat();
        combat.zones.hand = vec![CombatCard::new(CardId::Strike, 10)];
        combat.zones.draw_pile = vec![
            CombatCard::new(CardId::Strike, 21),
            CombatCard::new(CardId::Defend, 22),
        ]
        .into();
        let source = CombatPosition::new(EngineState::CombatPlayerTurn, combat);
        let sampler = IndependentStreamsCombatBeliefSamplerV1::new(vec![101, 202]);

        let environment =
            CombatBeliefEnvironmentV1::from_sampler(detached_context(), &source, &sampler)
                .expect("build belief environment");

        assert_eq!(
            sampler.conditioning(),
            CombatBeliefConditioningV1::CurrentPublicBoundaryOnly
        );
        assert_eq!(environment.public_history().entries().len(), 1);
        assert_eq!(environment.particles().len(), 2);
        assert_eq!(
            environment
                .particles()
                .iter()
                .map(CombatBeliefParticleV1::probability_mass)
                .sum::<f64>(),
            1.0
        );
    }

    fn particle(seed: u64, position: CombatPosition) -> CombatBeliefParticleV1 {
        CombatBeliefParticleV1::from_private_position(
            CombatBeliefParticleOriginV1::IndependentStreams {
                particle_seed: seed,
            },
            0.5,
            position,
        )
    }

    fn detached_context() -> PublicCombatRunContextV1 {
        PublicCombatRunContextV1::Unavailable {
            reason: PublicCombatRunContextGapV1::DetachedExactCombatPosition,
        }
    }

    fn potion_combat(monster_id: usize, potion_uuid: u32) -> crate::runtime::combat::CombatState {
        let mut combat = decision_combat();
        combat.entities.monsters[0].id = monster_id;
        combat.set_monster_protocol_visible_intent(
            monster_id,
            Intent::Attack {
                damage: 11,
                hits: 1,
            },
        );
        combat.entities.potions = vec![Some(Potion::new(PotionId::FirePotion, potion_uuid))];
        combat
    }

    fn decision_combat() -> crate::runtime::combat::CombatState {
        let mut combat = crate::test_support::blank_test_combat();
        let mut monster = crate::test_support::test_monster(EnemyId::JawWorm);
        monster.id = 7;
        monster.slot = 0;
        combat.entities.monsters.push(monster);
        combat.set_monster_protocol_visible_intent(
            7,
            Intent::Attack {
                damage: 11,
                hits: 1,
            },
        );
        combat
    }
}
