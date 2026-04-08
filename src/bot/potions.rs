use crate::combat::CombatState;
use crate::content::potions::PotionId;
use crate::content::powers::PowerId;
use crate::engine::targeting;
use crate::state::core::ClientInput;

/// Checks if we should immediately consume a potion.
/// Returns the ClientInput to play the potion if appropriate, otherwise None.
pub fn should_use_potion(state: &CombatState) -> Option<ClientInput> {
    let hp_per = (state.player.current_hp as f32 / state.player.max_hp as f32) * 100.0;
    let incoming_damage: i32 = state
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped && m.current_hp > 0)
        .map(|m| m.intent_dmg.max(0))
        .sum();
    let low_hp = hp_per <= 50.0;
    let imminent_lethal = incoming_damage >= (state.player.current_hp + state.player.block);
    let nob_active = state
        .monsters
        .iter()
        .any(|m| !m.is_dying && !m.is_escaped && state.get_power(m.id, PowerId::Anger) != 0);
    let early_buff_window = state.turn_count <= 2;

    let trigger = (state.is_boss_fight && early_buff_window)
        || (state.is_elite_fight && (early_buff_window || low_hp))
        || low_hp
        || imminent_lethal
        || nob_active;

    if !trigger {
        return None;
    }

    let dont_play_potions = [
        PotionId::SmokeBomb,
        PotionId::Elixir,
        PotionId::LiquidMemories,
        PotionId::SneckoOil,
        PotionId::StancePotion,
        PotionId::Ambrosia,
        PotionId::DistilledChaosPotion, // "Chaos Potion" mapping maybe
        PotionId::FairyPotion,          // Fairy is passive
    ];

    let mut best: Option<(i32, usize, Option<usize>)> = None;

    for (i, p_opt) in state.potions.iter().enumerate() {
        let Some(potion) = p_opt else {
            continue;
        };
        if dont_play_potions.contains(&potion.id) {
            continue;
        }

        let def = crate::content::potions::get_potion_definition(potion.id);
        let target = if let Some(validation) =
            targeting::validation_for_potion_target(def.target_required)
        {
            let candidates = targeting::candidate_targets(state, validation);
            match best_potion_target(state, potion.id, &candidates) {
                Some(target) => Some(target),
                None => continue,
            }
        } else {
            None
        };

        let score = potion_score(state, potion.id);
        if score <= 0 {
            continue;
        }

        match best {
            Some((best_score, _, _)) if best_score >= score => {}
            _ => best = Some((score, i, target)),
        }
    }

    if let Some((_, potion_index, target)) = best {
        return Some(ClientInput::UsePotion {
            potion_index,
            target,
        });
    }

    None
}

fn potion_score(state: &CombatState, potion_id: PotionId) -> i32 {
    let incoming_damage: i32 = state
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped && m.current_hp > 0)
        .map(|m| m.intent_dmg.max(0))
        .sum();
    let low_hp = state.player.current_hp * 3 <= state.player.max_hp;
    let imminent_lethal = incoming_damage >= (state.player.current_hp + state.player.block);
    let elite_or_boss = state.is_elite_fight || state.is_boss_fight;
    let early_buff_window = state.turn_count <= 2;
    let nob_active = state
        .monsters
        .iter()
        .any(|m| !m.is_dying && !m.is_escaped && state.get_power(m.id, PowerId::Anger) != 0);
    let hand_has_flex = state
        .hand
        .iter()
        .any(|c| c.id == crate::content::cards::CardId::Flex);
    let player_has_artifact = state.get_power(0, PowerId::Artifact) > 0;

    match potion_id {
        PotionId::AncientPotion => {
            if hand_has_flex && !player_has_artifact {
                98
            } else if elite_or_boss && !player_has_artifact && early_buff_window {
                86
            } else if !player_has_artifact {
                82
            } else {
                15
            }
        }
        PotionId::StrengthPotion
        | PotionId::DexterityPotion
        | PotionId::SpeedPotion
        | PotionId::SteroidPotion
        | PotionId::DuplicationPotion
        | PotionId::CultistPotion
        | PotionId::HeartOfIron
        | PotionId::EssenceOfSteel
        | PotionId::LiquidBronze => {
            if elite_or_boss && early_buff_window {
                92
            } else if imminent_lethal || nob_active {
                90
            } else {
                88
            }
        }
        PotionId::EnergyPotion => {
            let playable_cards = state.hand.iter().filter(|c| c.get_cost() > 0).count() as i32;
            if playable_cards >= 2 {
                86
            } else {
                68
            }
        }
        PotionId::PowerPotion | PotionId::ColorlessPotion => 84,
        PotionId::AttackPotion | PotionId::SkillPotion | PotionId::SwiftPotion => 78,
        PotionId::FearPotion | PotionId::WeakenPotion | PotionId::FirePotion => {
            if elite_or_boss || nob_active {
                84
            } else {
                76
            }
        }
        PotionId::ExplosivePotion => 74,
        PotionId::PoisonPotion => 72,
        PotionId::BlockPotion => {
            if imminent_lethal {
                95
            } else if incoming_damage > state.player.block || low_hp {
                78
            } else {
                42
            }
        }
        PotionId::RegenPotion | PotionId::GhostInAJar => {
            if imminent_lethal {
                100
            } else if low_hp || elite_or_boss {
                88
            } else {
                62
            }
        }
        PotionId::BloodPotion | PotionId::FruitJuice => {
            if low_hp {
                58
            } else {
                20
            }
        }
        PotionId::BlessingOfTheForge => 40,
        _ => 35,
    }
}

fn best_potion_target(
    state: &CombatState,
    potion_id: PotionId,
    candidates: &[usize],
) -> Option<usize> {
    candidates.iter().copied().max_by_key(|target| {
        state
            .monsters
            .iter()
            .find(|m| m.id == *target)
            .map(|monster| {
                let attack_bias = match potion_id {
                    PotionId::WeakenPotion => monster.intent_dmg * 10,
                    _ => 0,
                };
                attack_bias + monster.current_hp
            })
            .unwrap_or(0)
    })
}
