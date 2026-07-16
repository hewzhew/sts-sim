use std::collections::{BTreeMap, BTreeSet};

use super::actions::exact_actions_for_particle;
use super::boundary::policy_observation_envelope;
use super::hash::stable_hash;
use super::types::{
    CombatPolicyInformationSetKeyV1, CombatPolicyObservationGroupV1, CombatPublicActionV1,
    CombatScenarioDecisionBindingV1, CombatScenarioParticleV1, CombatScenarioPolicyErrorV1,
    ExactActionMap,
};

#[derive(Clone)]
pub struct CombatScenarioGroupV1 {
    view: CombatPolicyObservationGroupV1,
    pub(super) worlds: Vec<CombatScenarioParticleV1>,
    exact_actions: BTreeMap<String, ExactActionMap>,
}

impl CombatScenarioGroupV1 {
    pub fn view(&self) -> &CombatPolicyObservationGroupV1 {
        &self.view
    }

    pub(crate) fn scenario_ids(&self) -> Vec<&str> {
        self.worlds
            .iter()
            .map(CombatScenarioParticleV1::scenario_id)
            .collect()
    }

    pub fn bind_action(
        &self,
        action: &CombatPublicActionV1,
    ) -> Result<CombatScenarioDecisionBindingV1, CombatScenarioPolicyErrorV1> {
        if !self.view.candidates.contains(action) {
            return Err(CombatScenarioPolicyErrorV1::ActionUnavailable {
                information_set: self.view.key.clone(),
                action: format!("{action:?}"),
            });
        }

        let mut exact_inputs = Vec::with_capacity(self.worlds.len());
        for world in &self.worlds {
            let input = self
                .exact_actions
                .get(world.scenario_id())
                .and_then(|actions| actions.get(action))
                .cloned()
                .ok_or_else(|| CombatScenarioPolicyErrorV1::MissingExactBinding {
                    action: format!("{action:?}"),
                })?;
            exact_inputs.push((world.scenario_id().to_string(), input));
        }

        Ok(CombatScenarioDecisionBindingV1 {
            action: action.clone(),
            exact_inputs,
        })
    }
}

struct GroupBuilder {
    view: CombatPolicyObservationGroupV1,
    worlds: Vec<CombatScenarioParticleV1>,
    exact_actions: BTreeMap<String, ExactActionMap>,
}

pub fn group_combat_scenarios_v1(
    particles: Vec<CombatScenarioParticleV1>,
) -> Result<Vec<CombatScenarioGroupV1>, CombatScenarioPolicyErrorV1> {
    if particles.is_empty() {
        return Err(CombatScenarioPolicyErrorV1::EmptyScenarioSet);
    }

    let mut seen_scenarios = BTreeSet::new();
    let mut groups = BTreeMap::<CombatPolicyInformationSetKeyV1, GroupBuilder>::new();

    for particle in particles {
        if !seen_scenarios.insert(particle.scenario_id.clone()) {
            return Err(CombatScenarioPolicyErrorV1::DuplicateScenarioId {
                scenario_id: particle.scenario_id,
            });
        }
        let envelope = policy_observation_envelope(particle.scenario_id(), &particle.position)?;
        let exact_actions = exact_actions_for_particle(&particle)?;
        let candidates = exact_actions.keys().cloned().collect::<Vec<_>>();
        let key = CombatPolicyInformationSetKeyV1 {
            public_history_id: particle.public_history_id.clone(),
            public_observation_hash: stable_hash(&envelope),
            public_candidate_set_hash: stable_hash(&candidates),
        };

        match groups.get_mut(&key) {
            Some(group) => {
                if group.view.observation != envelope || group.view.candidates != candidates {
                    return Err(CombatScenarioPolicyErrorV1::InformationSetHashCollision { key });
                }
                group.view.scenario_count = group.view.scenario_count.saturating_add(1);
                group
                    .exact_actions
                    .insert(particle.scenario_id.clone(), exact_actions);
                group.worlds.push(particle);
            }
            None => {
                let scenario_id = particle.scenario_id.clone();
                groups.insert(
                    key.clone(),
                    GroupBuilder {
                        view: CombatPolicyObservationGroupV1 {
                            key,
                            observation: envelope,
                            candidates,
                            scenario_count: 1,
                        },
                        worlds: vec![particle],
                        exact_actions: BTreeMap::from([(scenario_id, exact_actions)]),
                    },
                );
            }
        }
    }

    Ok(groups
        .into_values()
        .map(|mut group| {
            group
                .worlds
                .sort_by(|left, right| left.scenario_id.cmp(&right.scenario_id));
            CombatScenarioGroupV1 {
                view: group.view,
                worlds: group.worlds,
                exact_actions: group.exact_actions,
            }
        })
        .collect())
}
