use crate::combat::CombatState;
use crate::content::potions::PotionId;
use crate::state::core::ClientInput;

/// Checks if we should immediately consume a potion.
/// Returns the ClientInput to play the potion if appropriate, otherwise None.
pub fn should_use_potion(state: &CombatState) -> Option<ClientInput> {
    // We only trigger automatic potions on Turn 1 of combat
    if state.turn_count != 1 {
        return None;
    }

    let hp_per = (state.player.current_hp as f32 / state.player.max_hp as f32) * 100.0;
    
    // Evaluate conditions similar to bottled_ai:
    // Boss: always trigger
    // Elite/Event: trigger if hp <= 50
    let trigger = state.is_boss_fight || (state.is_elite_fight && hp_per <= 50.0);

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
        PotionId::FairyPotion, // Fairy is passive
    ];

    for (i, p_opt) in state.potions.iter().enumerate() {
        if let Some(potion) = p_opt {
            if !dont_play_potions.contains(&potion.id) {
                let def = crate::content::potions::get_potion_definition(potion.id);
                let target = if def.target_required {
                    // Find the last non-dead monster
                    let mut found = None;
                    for m in state.monsters.iter().rev() {
                        if !m.is_dying && m.current_hp > 0 {
                            found = Some(m.logical_position as usize);
                            break;
                        }
                    }
                    if found.is_none() { 
                        continue; 
                    }
                    found
                } else {
                    None
                };

                return Some(ClientInput::UsePotion {
                    potion_index: i,
                    target,
                });
            }
        }
    }

    None
}
