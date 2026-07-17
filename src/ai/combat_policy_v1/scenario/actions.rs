use std::collections::btree_map::Entry;
use std::collections::BTreeMap;

use crate::content::cards::java_id;
use crate::content::monsters::EnemyId;
use crate::runtime::combat::CombatState;
use crate::sim::combat_legal_actions::engine_atomic_actions;
use crate::state::core::{ClientInput, EngineState};

use super::pending_choice::{exact_pending_choice_inputs, public_pending_choice_action};
use super::types::{
    CombatPublicActionV1, CombatPublicTargetV1, CombatScenarioParticleV1,
    CombatScenarioPolicyErrorV1, ExactActionMap,
};

pub(super) fn exact_actions_for_particle(
    particle: &CombatScenarioParticleV1,
) -> Result<ExactActionMap, CombatScenarioPolicyErrorV1> {
    let mut exact_actions = BTreeMap::new();
    let inputs = match &particle.position.engine {
        EngineState::PendingChoice(choice) => {
            exact_pending_choice_inputs(particle.scenario_id(), choice)?
        }
        _ => engine_atomic_actions(&particle.position.engine, &particle.position.combat),
    };
    for input in inputs {
        let public_action = public_action(
            particle.scenario_id(),
            &particle.position.engine,
            &particle.position.combat,
            &input,
        )?;
        match exact_actions.entry(public_action.clone()) {
            Entry::Vacant(entry) => {
                entry.insert(input);
            }
            Entry::Occupied(_) if matches!(input, ClientInput::SubmitSelection(_)) => {
                // Multiple exact UUID combinations may be the same public
                // card multiset. The policy may choose the multiset, not a
                // hidden UUID identity, so one exact representative is kept
                // privately for this world.
            }
            Entry::Occupied(_) => {
                return Err(CombatScenarioPolicyErrorV1::AmbiguousPublicAction {
                    scenario_id: particle.scenario_id().to_string(),
                    action: format!("{public_action:?}"),
                });
            }
        }
    }
    Ok(exact_actions)
}

fn public_action(
    scenario_id: &str,
    engine: &EngineState,
    combat: &CombatState,
    input: &ClientInput,
) -> Result<CombatPublicActionV1, CombatScenarioPolicyErrorV1> {
    if let EngineState::PendingChoice(choice) = engine {
        return public_pending_choice_action(scenario_id, combat, choice, input);
    }
    match input {
        ClientInput::PlayCard { card_index, target } => {
            let card = combat.zones.hand.get(*card_index).ok_or_else(|| {
                CombatScenarioPolicyErrorV1::InvalidLegalAction {
                    scenario_id: scenario_id.to_string(),
                    input: format!("{input:?}"),
                    reason: format!("hand index {card_index} is absent"),
                }
            })?;
            Ok(CombatPublicActionV1::PlayCard {
                hand_index: *card_index,
                card_id: java_id(card.id).to_string(),
                upgrades: card.upgrades,
                cost_for_turn: card.cost_for_turn_java(),
                target: public_target(scenario_id, combat, *target, input)?,
            })
        }
        ClientInput::UsePotion {
            potion_index,
            target,
        } => {
            let potion = combat
                .entities
                .potions
                .get(*potion_index)
                .and_then(|slot| slot.as_ref())
                .ok_or_else(|| CombatScenarioPolicyErrorV1::InvalidLegalAction {
                    scenario_id: scenario_id.to_string(),
                    input: format!("{input:?}"),
                    reason: format!("potion slot {potion_index} is empty"),
                })?;
            Ok(CombatPublicActionV1::UsePotion {
                potion_slot: *potion_index,
                potion_id: format!("{:?}", potion.id),
                target: public_target(scenario_id, combat, *target, input)?,
            })
        }
        ClientInput::DiscardPotion(potion_slot) => {
            let potion = combat
                .entities
                .potions
                .get(*potion_slot)
                .and_then(|slot| slot.as_ref())
                .ok_or_else(|| CombatScenarioPolicyErrorV1::InvalidLegalAction {
                    scenario_id: scenario_id.to_string(),
                    input: format!("{input:?}"),
                    reason: format!("potion slot {potion_slot} is empty"),
                })?;
            Ok(CombatPublicActionV1::DiscardPotion {
                potion_slot: *potion_slot,
                potion_id: format!("{:?}", potion.id),
            })
        }
        ClientInput::EndTurn => Ok(CombatPublicActionV1::EndTurn),
        ClientInput::Proceed => Ok(CombatPublicActionV1::Proceed),
        ClientInput::Cancel => Ok(CombatPublicActionV1::Cancel),
        _ => Err(CombatScenarioPolicyErrorV1::UnsupportedAction {
            scenario_id: scenario_id.to_string(),
            input: format!("{input:?}"),
        }),
    }
}

fn public_target(
    scenario_id: &str,
    combat: &CombatState,
    target: Option<usize>,
    input: &ClientInput,
) -> Result<Option<CombatPublicTargetV1>, CombatScenarioPolicyErrorV1> {
    target
        .map(|entity_id| {
            combat
                .entities
                .monsters
                .iter()
                .find(|monster| monster.id == entity_id)
                .map(|monster| CombatPublicTargetV1 {
                    monster_slot: monster.slot,
                    enemy_id: EnemyId::from_id(monster.monster_type)
                        .map(|enemy| format!("{enemy:?}"))
                        .unwrap_or_else(|| format!("monster_type:{}", monster.monster_type)),
                })
                .ok_or_else(|| CombatScenarioPolicyErrorV1::InvalidLegalAction {
                    scenario_id: scenario_id.to_string(),
                    input: format!("{input:?}"),
                    reason: format!("target entity {entity_id} is absent"),
                })
        })
        .transpose()
}
