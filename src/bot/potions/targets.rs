use super::signals::CombatSignals;
use crate::content::potions::{get_potion_definition, PotionId};
use crate::content::powers::PowerId;
use crate::runtime::combat::CombatState;

pub fn best_target(
    state: &CombatState,
    signals: &CombatSignals,
    potion_id: PotionId,
    candidates: &[usize],
) -> Option<usize> {
    candidates
        .iter()
        .copied()
        .max_by_key(|target| target_score(state, signals, potion_id, *target))
}

pub fn target_score(
    state: &CombatState,
    signals: &CombatSignals,
    potion_id: PotionId,
    target: usize,
) -> i32 {
    state
        .entities
        .monsters
        .iter()
        .find(|m| m.id == target)
        .map(|monster| {
            let artifact = state.get_power(monster.id, PowerId::Artifact);
            match potion_id {
                PotionId::WeakenPotion => {
                    monster.intent_preview_total_damage() * 12
                        - artifact
                            * if signals.threat.imminent_lethal {
                                120
                            } else {
                                260
                            }
                }
                PotionId::FearPotion => {
                    signals.offense.playable_attacks * 2_000 + monster.current_hp
                        - artifact
                            * if signals.fight.elite_or_boss {
                                220
                            } else {
                                320
                            }
                }
                PotionId::FirePotion => {
                    let potency = get_potion_definition(potion_id).base_potency;
                    let lethal_bonus = if monster.current_hp <= potency {
                        8_000
                    } else {
                        0
                    };
                    lethal_bonus + monster.current_hp
                }
                PotionId::PoisonPotion => {
                    let boss_bonus = i32::from(signals.fight.elite_or_boss) * 2_000;
                    boss_bonus + monster.current_hp - artifact * 280
                }
                _ => monster.current_hp,
            }
        })
        .unwrap_or(0)
}
