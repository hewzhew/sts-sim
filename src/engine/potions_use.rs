//! Potion usage — applying potion effects to game state.
//!
//! Corresponds to Java's potions/ package (44 potion classes).

use crate::schema::{CardCommand, CardInstance, CardLocation, CardType};
use crate::state::{GameState, GamePhase, InsertPosition};
use crate::items::potions::{PotionDefinition, PotionCommand};
use super::commands::apply_command;
use super::CommandResult;
use rand::Rng;

// ============================================================================
// Potion Usage
// ============================================================================

/// Apply the effects of using a potion.
///
/// # Arguments
/// * `state` - The game state to modify
/// * `potion` - The potion definition
/// * `target_idx` - Optional enemy target index (for enemy-targeting potions)
/// * `has_sacred_bark` - Whether Sacred Bark doubles potion effects
///
/// # Returns
/// * `Ok(())` - Potion used successfully
/// * `Err(String)` - Error message if potion use failed
pub fn use_potion(
    state: &mut GameState,
    potion: &PotionDefinition,
    target_idx: Option<usize>,
    has_sacred_bark: bool,
) -> Result<(), String> {
    let potency = potion.get_potency(has_sacred_bark);
    let potency_percent = potion.get_potency_percent(has_sacred_bark);
    
    match &potion.command_hint {
        // === Damage Effects ===
        PotionCommand::DealDamage => {
            // Fire Potion, Explosive Potion
            let target = target_idx.ok_or("Target required for damage potion")?;
            if let Some(enemy) = state.enemies.get_mut(target) {
                let actual = enemy.take_damage(potency);
                if enemy.is_dead() {
                    state.last_attack_killed = true;
                }
                game_log!("  → [Potion] Dealt {} damage to {}", actual, enemy.name);
            }
        }
        
        PotionCommand::DealDamageAll => {
            // Explosive Potion (all enemies)
            for enemy in state.enemies.iter_mut() {
                if !enemy.is_dead() {
                    let actual = enemy.take_damage(potency);
                    game_log!("  → [Potion] Dealt {} damage to {}", actual, enemy.name);
                }
            }
        }
        
        // === Block Effects ===
        PotionCommand::GainBlock => {
            // Block Potion
            state.player.block += potency;
            game_log!("  → [Potion] Gained {} Block", potency);
        }
        
        // === Healing Effects ===
        PotionCommand::Heal => {
            // Regen Potion - flat heal
            let heal_amount = potency.min(state.player.max_hp - state.player.current_hp);
            state.player.current_hp += heal_amount;
            game_log!("  → [Potion] Healed {} HP", heal_amount);
        }
        
        PotionCommand::HealPercent => {
            // Blood Potion (heal % of max HP)
            let heal_target = (state.player.max_hp * potency_percent) / 100;
            let heal_amount = heal_target.min(state.player.max_hp - state.player.current_hp);
            state.player.current_hp += heal_amount;
            game_log!("  → [Potion] Healed {} HP ({}% of max)", heal_amount, potency_percent);
        }
        
        PotionCommand::GainMaxHP => {
            // Fruit Juice
            state.player.max_hp += potency;
            state.player.current_hp += potency;
            game_log!("  → [Potion] Gained {} Max HP", potency);
        }
        
        // === Debuff Effects (apply to enemies) ===
        PotionCommand::ApplyWeak => {
            // Weak Potion
            let target = target_idx.ok_or("Target required for weak potion")?;
            if let Some(enemy) = state.enemies.get_mut(target) {
                enemy.add_buff("Weak", potency);
                game_log!("  → [Potion] Applied {} Weak to {}", potency, enemy.name);
            }
        }
        
        PotionCommand::ApplyVulnerable => {
            // Fear Potion
            let target = target_idx.ok_or("Target required for vulnerable potion")?;
            if let Some(enemy) = state.enemies.get_mut(target) {
                enemy.add_buff("Vulnerable", potency);
                game_log!("  → [Potion] Applied {} Vulnerable to {}", potency, enemy.name);
            }
        }
        
        PotionCommand::ApplyPoison => {
            // Poison Potion
            let target = target_idx.ok_or("Target required for poison potion")?;
            if let Some(enemy) = state.enemies.get_mut(target) {
                enemy.add_buff("Poison", potency);
                game_log!("  → [Potion] Applied {} Poison to {}", potency, enemy.name);
            }
        }
        
        // === Buff Effects (apply to player) ===
        PotionCommand::GainStrength => {
            // Strength Potion
            state.player.apply_status("Strength", potency);
            game_log!("  → [Potion] Gained {} Strength", potency);
        }
        
        PotionCommand::GainDexterity => {
            // Dexterity Potion
            state.player.apply_status("Dexterity", potency);
            game_log!("  → [Potion] Gained {} Dexterity", potency);
        }
        
        PotionCommand::GainArtifact => {
            // Ancient Potion
            state.player.apply_status("Artifact", potency);
            game_log!("  → [Potion] Gained {} Artifact", potency);
        }
        
        PotionCommand::GainPlatedArmor => {
            // Liquid Bronze
            state.player.apply_status("PlatedArmor", potency);
            game_log!("  → [Potion] Gained {} Plated Armor", potency);
        }
        
        PotionCommand::GainThorns => {
            // Thorns potion
            state.player.apply_status("Thorns", potency);
            game_log!("  → [Potion] Gained {} Thorns", potency);
        }
        
        PotionCommand::GainMetallicize => {
            // Metallicize potion
            state.player.apply_status("Metallicize", potency);
            game_log!("  → [Potion] Gained {} Metallicize", potency);
        }
        
        PotionCommand::GainRitual => {
            // Ritual potion
            state.player.apply_status("Ritual", potency);
            game_log!("  → [Potion] Gained {} Ritual", potency);
        }
        
        PotionCommand::GainRegeneration => {
            // Regen Potion (buff variant)
            state.player.apply_status("Regeneration", potency);
            game_log!("  → [Potion] Gained {} Regeneration", potency);
        }
        
        PotionCommand::GainIntangible => {
            // Ghost in a Jar
            state.player.apply_temp_buff("Intangible", potency);
            game_log!("  → [Potion] Gained {} Intangible", potency);
        }
        
        // === Resource Effects ===
        PotionCommand::GainEnergy => {
            // Energy Potion
            state.player.energy += potency;
            game_log!("  → [Potion] Gained {} Energy", potency);
        }
        
        PotionCommand::DrawCards => {
            // Swift Potion (draw cards)
            state.draw_cards(potency);
            game_log!("  → [Potion] Drew {} cards", potency);
        }
        
        // === Discovery / Card Generation ===
        PotionCommand::DiscoverAttack => {
            // Attack Potion - discover an attack card
            // MVP: Add a random attack to hand (simplified)
            game_log!("  → [Potion] Discover Attack (not fully implemented)");
            // TODO: Implement card discovery UI
        }
        
        PotionCommand::DiscoverSkill => {
            // Skill Potion - discover a skill card
            game_log!("  → [Potion] Discover Skill (not fully implemented)");
        }
        
        PotionCommand::DiscoverPower => {
            // Power Potion - discover a power card
            game_log!("  → [Potion] Discover Power (not fully implemented)");
        }
        
        PotionCommand::DiscoverColorless => {
            // Colorless Potion - discover a colorless card
            game_log!("  → [Potion] Discover Colorless (not fully implemented)");
        }
        
        PotionCommand::DiscoverRare => {
            // Elixir - discover a rare card
            game_log!("  → [Potion] Discover Rare (not fully implemented)");
        }
        
        PotionCommand::AddMiracles => {
            // Blessing of Holy Water / Bottled Miracle
            for _ in 0..potency {
                let miracle = CardInstance::new_basic("Miracle", 0);
                state.hand.push(miracle);
            }
            game_log!("  → [Potion] Added {} Miracle(s) to hand", potency);
        }
        
        PotionCommand::AddShivs => {
            // Bottled Tornado / Shiv Potion effect
            for _ in 0..potency {
                let shiv = CardInstance::new_basic("Shiv", 0);
                state.hand.push(shiv);
            }
            game_log!("  → [Potion] Added {} Shiv(s) to hand", potency);
        }
        
        // === Defect-Specific ===
        PotionCommand::GainFocus => {
            // Focus Potion
            state.player.apply_status("Focus", potency);
            game_log!("  → [Potion] Gained {} Focus", potency);
        }
        
        PotionCommand::GainOrbSlots => {
            // Potion of Capacity — increase max orb slots
            state.max_orbs += potency as usize;
            game_log!("  → [Potion] Gained {} Orb Slots (max: {})", potency, state.max_orbs);
        }
        
        PotionCommand::ChannelDark => {
            // Essence of Darkness — channel Dark orbs
            super::commands::channel_orb(state, "Dark", potency);
            game_log!("  → [Potion] Channeled {} Dark orb(s)", potency);
        }
        
        // === Watcher-Specific ===
        PotionCommand::EnterStance => {
            // Stance Potion — AI heuristic: enter Wrath (more offensive value)
            use crate::core::stances::Stance;
            let old = state.player.stance;
            let new_stance = if old != Stance::Wrath { Stance::Wrath } else { Stance::Calm };
            
            // Exit energy (Calm → +2)
            let exit_e = old.on_exit_energy();
            if exit_e > 0 { state.player.energy += exit_e; }
            
            state.player.stance = new_stance;
            
            // Enter energy (Divinity → +3)
            let enter_e = new_stance.on_enter_energy();
            if enter_e > 0 { state.player.energy += enter_e; }
            
            game_log!("  → [Potion] Entered {} stance", new_stance.name());
        }
        
        // === Special Effects ===
        PotionCommand::RecallFromDiscard => {
            // Liquid Memories - put a card from discard into hand
            // MVP: Return the most recent discard
            if let Some(card) = state.discard_pile.pop() {
                game_log!("  → [Potion] Returned {} to hand", card.definition_id);
                state.hand.push(card);
            }
        }
        
        PotionCommand::ExhaustCards => {
            // Smoke Bomb (exhaust effect variant)
            // Typically used for escape, but can exhaust random cards
            game_log!("  → [Potion] Exhaust effect (not fully implemented)");
        }
        
        PotionCommand::UpgradeCards => {
            // Blessing (upgrade cards in hand)
            let mut upgraded_count = 0;
            for card in state.hand.iter_mut() {
                if !card.upgraded {
                    card.upgraded = true;
                    upgraded_count += 1;
                    if upgraded_count >= potency {
                        break;
                    }
                }
            }
            game_log!("  → [Potion] Upgraded {} cards in hand", upgraded_count);
        }
        
        PotionCommand::DoubleTap => {
            // Duplication Potion - next attack is played twice
            state.card_modifiers.duplicate_next_attack += 1;
            game_log!("  → [Potion] Next attack will be played twice");
        }
        
        PotionCommand::FillPotions => {
            // Alchemize - fill empty potion slots with random potions
            // TODO: Implement random potion generation
            game_log!("  → [Potion] Fill potion slots (not fully implemented)");
        }
        
        PotionCommand::Escape => {
            // Smoke Bomb - escape from non-boss combat
            // Check if this is a boss fight
            if state.current_map_node.is_some() {
                // MVP: Set a flag or change screen
                game_log!("  → [Potion] Escaped from combat!");
                // Note: Actual escape logic would need to skip rewards
            }
        }
        
        PotionCommand::FairyRevive => {
            // Fairy in a Bottle - revives on death (passive, shouldn't be "used")
            // This is a passive effect that triggers on death
            game_log!("  → [Potion] Fairy in a Bottle is passive (activates on death)");
        }
        
        PotionCommand::PlayFromDraw => {
            // Distilled Chaos - play top cards from draw pile
            for _ in 0..potency {
                if let Some(card) = state.draw_pile.pop() {
                    game_log!("  → [Potion] Playing {} from draw pile", card.definition_id);
                    // Would need to actually play the card here
                    state.hand.push(card); // MVP: Just add to hand
                }
            }
        }
        
        PotionCommand::GamblerDraw => {
            // Gambler's Brew - discard any number, draw that many
            // MVP: Can't implement interactive discard, so just draw potency cards
            state.draw_cards(potency);
            game_log!("  → [Potion] Gambler's Brew drew {} cards", potency);
        }
        
        PotionCommand::SneckoEffect => {
            // Snecko Oil - draw cards and randomize costs
            state.draw_cards(potency);
            // Randomize costs of cards in hand
            for card in state.hand.iter_mut() {
                let new_cost = state.rng.random_range(0..=3);
                card.current_cost = new_cost;
            }
            game_log!("  → [Potion] Snecko Oil: drew {} cards and randomized costs", potency);
        }
    }
    
    // Trigger relics on potion use (e.g., ToyOrnithopter, Test1)
    // Java: PotionPopUp → for (r : player.relics) r.onUsePotion()
    {
        use crate::items::relics::{GameEvent, trigger_relics, apply_relic_results};
        let event = GameEvent::PlayerUsePotion;
        let result = trigger_relics(state, &event, None);
        apply_relic_results(state, &result);
    }
    
    Ok(())
}

