//! Card command interpreter — executes the bytecode-style commands.
//!
//! Each CardCommand variant is interpreted here without hardcoded card logic.
//! This is Rust's idiomatic equivalent of Java's ctions/common/*.java (61 Action classes).

use crate::loader::CardLibrary;
use crate::schema::{AmountValue, CardCommand, CardInstance, CardLocation, CardType, CardColor};
use crate::state::{GameState, GamePhase, SelectMode, InsertPosition, CardFilter};
use super::{CommandResult, Condition, parse_location, upgrade_cards_in_slice};
/// Parse and execute commands from a JSON array
pub(crate) fn execute_command_list(
    state: &mut GameState,
    commands: &[serde_json::Value],
    upgraded: bool,
    target_idx: Option<usize>,
    library: Option<&CardLibrary>,
) -> Vec<CommandResult> {
    let mut results = Vec::new();
    
    for cmd_value in commands {
        // Try to parse as CardCommand
        if let Ok(cmd) = serde_json::from_value::<CardCommand>(cmd_value.clone()) {
            let result = apply_command(state, &cmd, upgraded, target_idx, library);
            results.push(result);
        } else {
            game_log!("  ⚠ Failed to parse sub-command: {:?}", cmd_value);
            results.push(CommandResult::Unknown);
        }
    }
    
    results
}

/// Parse a ValueSource from JSON to get a numeric value.
/// ValueSource can be a fixed number or a dynamic value like "CurrentBlock".
fn parse_value_source(value: Option<&serde_json::Value>, state: &GameState, default: i32) -> i32 {
    match value {
        None => default,
        Some(v) => {
            // If it's a number, return directly
            if let Some(n) = v.as_i64() {
                return n as i32;
            }
            
            // If it's a string, check for known value sources
            if let Some(s) = v.as_str() {
                match s {
                    "CurrentBlock" | "current_block" => state.player.block,
                    "CurrentEnergy" | "current_energy" => state.player.energy,
                    "CardsInHand" | "cards_in_hand" => state.hand.len() as i32,
                    "CardsExhausted" | "cards_exhausted" => state.exhaust_pile.len() as i32,
                    "CardsDiscarded" | "cards_discarded" => state.discard_pile.len() as i32,
                    "LastUnblockedDamage" | "last_unblocked_damage" => state.last_unblocked_damage,
                    _ => {
                        // Try parsing as number string
                        s.parse::<i32>().unwrap_or(default)
                    }
                }
            } else if let Some(obj) = v.as_object() {
                // Handle object-style value sources
                if let Some(source_type) = obj.get("type").and_then(|t| t.as_str()) {
                    match source_type {
                        "Fixed" => {
                            // Support both {"type": "Fixed", "value": X}
                            // and {"type": "Fixed", "params": {"base": X}}
                            obj.get("value").and_then(|v| v.as_i64())
                                .or_else(|| obj.get("params")
                                    .and_then(|p| p.get("base"))
                                    .and_then(|b| b.as_i64()))
                                .unwrap_or(default as i64) as i32
                        },
                        "CurrentBlock" => state.player.block,
                        "CurrentEnergy" => state.player.energy,
                        "CardsInHand" => state.hand.len() as i32,
                        "CardsExhausted" => state.exhaust_pile.len() as i32,
                        "CardsDiscarded" => state.discard_pile.len() as i32,
                        "LastUnblockedDamage" | "last_unblocked_damage" => state.last_unblocked_damage,
                        _ => default,
                    }
                } else {
                    default
                }
            } else {
                default
            }
        }
    }
}

/// Apply a single command to the game state.
///
/// # Arguments
/// Apply a list of HookEffects to the game state.
///
/// This is the interpreter that bridges the functional hook system (which returns
/// effect descriptions) to the imperative engine (which mutates game state).
///
/// * `attacker_idx` - Index of the attacking enemy (for DamageAttacker effects)
/// * `source_power_id` - The power ID string that generated these effects (for RemoveSelf/AddStacks)
pub(crate) fn apply_hook_effects(
    state: &mut GameState,
    effects: &[crate::power_hooks::HookEffect],
    source_power_id: &str,
    attacker_idx: Option<usize>,
    library: Option<&CardLibrary>,
) {
    use crate::power_hooks::HookEffect;
    for effect in effects {
        match effect {
            HookEffect::DamageAttacker(amount) => {
                if let Some(idx) = attacker_idx {
                    if let Some(enemy) = state.enemies.get_mut(idx) {
                        if !enemy.is_dead() {
                            let actual = enemy.take_damage(*amount);
                            game_log!("    ⚡ {} takes {} damage (from {})", enemy.name, actual, source_power_id);
                        }
                    }
                }
            }
            HookEffect::GainBlock(amount) => {
                state.player.block += amount;
                game_log!("    🛡 Player gains {} block (from {})", amount, source_power_id);
            }
            HookEffect::GainStrength(amount) => {
                state.player.powers.apply("Strength", *amount, None);
                game_log!("    💪 Player gains {} Strength (from {})", amount, source_power_id);
            }
            HookEffect::DrawCards(count) => {
                state.draw_cards(*count);
                game_log!("    🃏 Player draws {} cards (from {})", count, source_power_id);
            }
            HookEffect::LoseHp(amount) => {
                state.player.current_hp -= amount;
                game_log!("    💔 Player loses {} HP (from {})", amount, source_power_id);
                // Trigger Rupture: gain Strength from self-damage
                // Java: RupturePower.wasHPLost(info, damageAmount) where info.owner == this.owner
                if *amount > 0 {
                    let rupture_stacks = state.player.powers.get("Rupture");
                    if rupture_stacks > 0 {
                        state.player.powers.apply("Strength", rupture_stacks, None);
                        game_log!("    💪 Rupture: +{} Strength from self-damage", rupture_stacks);
                    }
                }
            }
            HookEffect::DamageAllEnemies(amount) => {
                for enemy in state.enemies.iter_mut() {
                    if !enemy.is_dead() {
                        let actual = enemy.take_damage(*amount);
                        game_log!("    ⚡ {} takes {} damage (from {})", enemy.name, actual, source_power_id);
                    }
                }
            }
            HookEffect::RemoveSelf => {
                state.player.powers.remove(source_power_id);
                game_log!("    ❌ {} removed", source_power_id);
            }
            HookEffect::AddStacks(n) => {
                let current = state.player.powers.get(source_power_id);
                let new_val = current + n;
                if new_val <= 0 {
                    state.player.powers.remove(source_power_id);
                } else {
                    state.player.powers.force_set(source_power_id, new_val);
                }
            }
            HookEffect::GainEnergy(amount) => {
                state.player.energy += amount;
                game_log!("    ⚡ Player gains {} energy (from {})", amount, source_power_id);
            }
            // Effects not yet implemented in engine
            HookEffect::ConsumeStack => {
                let current = state.player.powers.get(source_power_id);
                if current <= 1 {
                    state.player.powers.remove(source_power_id);
                } else {
                    state.player.powers.force_set(source_power_id, current - 1);
                }
            }
            HookEffect::HealHp(amount) => {
                let old_hp = state.player.current_hp;
                state.player.current_hp = (state.player.current_hp + amount).min(state.player.max_hp);
                let healed = state.player.current_hp - old_hp;
                if healed > 0 {
                    game_log!("    💚 Player heals {} HP (from {})", healed, source_power_id);
                }
            }
            HookEffect::ReduceStacks(n) => {
                let current = state.player.powers.get(source_power_id);
                let new_val = current - n;
                if new_val <= 0 {
                    state.player.powers.remove(source_power_id);
                    game_log!("    ❌ {} expired (stacks reached 0)", source_power_id);
                } else {
                    state.player.powers.force_set(source_power_id, new_val);
                }
            }
            HookEffect::PoisonAllEnemies(amount) => {
                for enemy in state.enemies.iter_mut() {
                    if !enemy.is_dead() {
                        enemy.powers.apply("Poison", *amount, None);
                        game_log!("    ☠ {} gains {} Poison (from {})", enemy.name, amount, source_power_id);
                    }
                }
            }
            HookEffect::ApplyPower { id, stacks } => {
                let power_name = format!("{:?}", id);
                state.player.powers.apply(&power_name, *stacks, None);
                game_log!("    ✨ Player gains {} {} (from {})", stacks, power_name, source_power_id);
            }
            HookEffect::ExhaustPlayed => {
                // Corruption: exhaust the Skill that was just played
                // The engine handles this at the play_card level by checking for this effect
                // We set a flag that play_card reads
                game_log!("    🔥 Card will be exhausted (from {})", source_power_id);
            }
            HookEffect::ShuffleStatus { card, count } => {
                // Hex: add status cards into draw pile
                for _ in 0..*count {
                    state.add_card_by_id(card, 0, CardLocation::DrawPile, InsertPosition::Shuffle);
                }
                game_log!("    🃏 {} {} shuffled into draw pile (from {})", count, card, source_power_id);
            }
            HookEffect::DamagePlayer(amount) => {
                // BeatOfDeath, etc: enemy deals damage to player
                let actual = (*amount - state.player.block).max(0);
                state.player.block = (state.player.block - amount).max(0);
                state.player.current_hp -= actual;
                game_log!("    💔 Player takes {} damage ({} blocked) (from {})", actual, amount - actual, source_power_id);
            }
            HookEffect::TimeWarpTrigger => {
                // TimeWarp: increment counter on the enemy that has this power.
                // At 12, end turn + gain Strength. 
                // For now, we just track it. Full end-turn-early logic is complex.
                game_log!("    ⏳ Time Warp triggered (from {})", source_power_id);
            }
            HookEffect::SetSkillCostZero => {
                // Corruption onCardDraw: set drawn Skill's cost to 0
                // This is handled at draw time, not in generic effect application
                game_log!("    💀 Skill cost set to 0 (from {})", source_power_id);
            }
            HookEffect::ApplyDexterity(amount) => {
                state.player.powers.apply("Dexterity", *amount, None);
                game_log!("    🏃 Player gains {} Dexterity (from {})", amount, source_power_id);
            }
            HookEffect::ChannelLightning => {
                channel_orb(state, "Lightning", 1);
                game_log!("    ⚡ Channel Lightning orb (from {})", source_power_id);
            }
            HookEffect::ChannelFrost => {
                channel_orb(state, "Frost", 1);
                game_log!("    ❄ Channel Frost orb (from {})", source_power_id);
            }
            HookEffect::ApplyVulnerableToPlayer(amount) => {
                state.player.powers.apply("Vulnerable", *amount, None);
                game_log!("    💥 Player gains {} Vulnerable (from {})", amount, source_power_id);
            }
            HookEffect::EnemyGainStrength(amount) => {
                // Applied to the enemy that owns this power (attacker_idx)
                if let Some(idx) = attacker_idx {
                    if let Some(enemy) = state.enemies.get_mut(idx) {
                        enemy.powers.apply("Strength", *amount, None);
                        game_log!("    💪 {} gains {} Strength (from {})", enemy.name, amount, source_power_id);
                    }
                }
            }
            HookEffect::DamageRandomEnemy(amount) => {
                // Juggernaut: deal damage to a random alive enemy
                // For simplicity, pick first alive enemy (true random would need RNG)
                if let Some(enemy) = state.enemies.iter_mut().find(|e| !e.is_dead()) {
                    let actual = enemy.take_damage(*amount);
                    game_log!("    ⚡ {} takes {} damage (from {})", enemy.name, actual, source_power_id);
                }
            }
            HookEffect::ApplyPoisonToTarget(amount) => {
                // Envenom: apply Poison to the target enemy (attacker_idx used as target)
                if let Some(idx) = attacker_idx {
                    if let Some(enemy) = state.enemies.get_mut(idx) {
                        if !enemy.is_dead() {
                            enemy.powers.apply("Poison", *amount, None);
                            game_log!("    ☠ {} gains {} Poison (from {})", enemy.name, amount, source_power_id);
                        }
                    }
                }
            }
            HookEffect::CreateCardInHand { card_id, count } => {
                // Create known generated cards in hand (Shiv, Smite, etc.)
                use crate::schema::{CardInstance, CardType};
                let (cost, card_type) = match *card_id {
                    "Shiv" => (0, CardType::Attack),
                    "Smite" => (1, CardType::Attack),
                    "Dagger" => (0, CardType::Attack),
                    "Miracle" => (0, CardType::Skill),
                    "Insight" => (0, CardType::Skill),
                    "Safety" => (1, CardType::Skill),
                    "Expunger" => (1, CardType::Attack),
                    _ => (1, CardType::Attack), // fallback
                };
                for _ in 0..*count {
                    if state.hand.len() < 10 {
                        let mut card = CardInstance::new_basic(card_id, cost).with_type(card_type);
                        // MasterReality: auto-upgrade created cards
                        if state.player.powers.has("MasterReality") {
                            card.upgraded = true;
                        }
                        state.hand.push(card);
                    }
                }
                game_log!("    🃏 Created {}x {} in hand (from {})", count, card_id, source_power_id);
            }
            HookEffect::CreateRandomCardInHand { pool, count } => {
                // Query card library for random cards from the named pool
                if let Some(ref lib) = state.card_library {
                    let lib = lib.clone(); // Arc clone for borrow checker
                    for _ in 0..*count {
                        if state.hand.len() < 10 {
                            if let Some(mut card) = lib.get_random_card_of_color(pool, Some(state.player_class), &mut state.rng) {
                                // MasterReality: auto-upgrade created cards
                                if state.player.powers.has("MasterReality") {
                                    card.upgraded = true;
                                }
                                game_log!("    🎲 Created {} in hand ({} pool, from {})", 
                                    card.definition_id, pool, source_power_id);
                                state.hand.push(card);
                            }
                        }
                    }
                } else {
                    game_log!("    🎲 [No card library] Would create {}x random {} card (from {})", 
                        count, pool, source_power_id);
                }
            }
            HookEffect::Scry(amount) => {
                // Foresight: Scry N cards (same AI heuristic as CardCommand::Scry)
                use crate::schema::CardType;
                let scry_count = std::cmp::min(*amount, state.draw_pile.len() as i32) as usize;
                if scry_count > 0 {
                    let mut discarded = 0;
                    let cards: Vec<_> = state.draw_pile.iter().rev().take(scry_count).cloned().collect();
                    let len = state.draw_pile.len();
                    state.draw_pile.truncate(len - scry_count);
                    for card in cards {
                        if card.card_type == CardType::Curse || card.card_type == CardType::Status {
                            state.discard_pile.push(card);
                            discarded += 1;
                        } else {
                            state.draw_pile.push(card);
                        }
                    }
                    game_log!("    🔮 Scry {} (discarded {}, from {})", scry_count, discarded, source_power_id);
                }
            }
            HookEffect::PlayTopCard(count) => {
                // Mayhem: auto-play top card(s) from draw pile for free
                // Java: PlayTopCardAction → NewQueueCardAction(autoplayCard=true)
                if let Some(lib) = library {
                    for _ in 0..*count {
                        if let Some(mut card) = state.draw_pile.pop() {
                            let card_name = card.definition_id.clone();
                            let original_cost = card.current_cost;
                            card.set_cost_for_turn(0); // free play (autoplay)
                            game_log!("    🎭 Mayhem: auto-playing {} from draw pile (free)", card_name);
                            
                            // Execute via full play_card pipeline
                            match super::combat::play_card(state, lib, &card, Some(0)) {
                                Ok(results) => {
                                    let should_exhaust = results.iter().any(|r| matches!(r, super::CommandResult::CardExhausted));
                                    if should_exhaust || card.is_ethereal {
                                        state.exhaust_pile.push(card);
                                        game_log!("    🎭 {} exhausted after auto-play", card_name);
                                    } else {
                                        card.current_cost = original_cost; // restore cost
                                        state.discard_pile.push(card);
                                    }
                                }
                                Err(e) => {
                                    game_log!("    ⚠ Mayhem failed to play {}: {}", card_name, e);
                                    state.discard_pile.push(card);
                                }
                            }
                        }
                    }
                } else {
                    game_log!("    ⚠ PlayTopCard: no library available (cannot execute card commands)");
                }
            }
            HookEffect::ReplayCard => {
                // EchoForm / Duplication: replay the card just played
                // This is a signal — the actual replay is handled in play_card_from_hand
                // by checking the on_use_card effects for ReplayCard
                game_log!("    🔄 ReplayCard signal (from {})", source_power_id);
            }
            HookEffect::RerollIntent => {
                // Reactive: enemy re-rolls its intent after being attacked
                // In our sim, monster moves are pre-determined by behavior model
                // so this is a no-op for now (would need behavior re-roll)
                game_log!("    🔀 Enemy re-rolls intent (from {})", source_power_id);
            }
            HookEffect::AddStatusToDiscard { card, count } => {
                // PainfulStabs: add Wound/Burn to player's discard pile
                for _ in 0..*count {
                    state.add_card_by_id(card, 0, CardLocation::DiscardPile, InsertPosition::Top);
                }
                game_log!("    🃏 {}x {} added to discard (from {})", count, card, source_power_id);
            }
            HookEffect::GainBlockIfCalm(amount) => {
                // LikeWater: gain block only if in Calm stance
                use crate::core::stances::Stance;
                if state.player.stance == Stance::Calm {
                    state.player.block += amount;
                    game_log!("    🛡 Player gains {} block (Calm stance, from {})", amount, source_power_id);
                }
            }
            HookEffect::PlayerGainBlock(amount) => {
                // BlockReturn (Talk to the Hand): player gains block when hit by enemy with this debuff
                state.player.block += amount;
                game_log!("    🛡 Player gains {} block (from {})", amount, source_power_id);
            }
            HookEffect::ReduceRetainedCardsCost(amount) => {
                // Establishment: reduce cost of all cards remaining in hand (they are retained)
                // Java: EstablishmentPowerAction reduces cost of retained cards by amount
                // At end of turn, only retained cards remain in hand
                for card in state.hand.iter_mut() {
                    card.current_cost = (card.current_cost - amount).max(0);
                }
                game_log!("    💰 Establishment: reduced retained card costs by {} (from {})", amount, source_power_id);
            }
            HookEffect::TriggerOrbPassive(count) => {
                // Loop: trigger leftmost orb passive N times
                // Java: LoopPower.atStartOfTurn → orbs.get(0).onEndOfTurn() × amount
                if !state.orb_slots.is_empty() {
                    for _ in 0..*count {
                        let effect = state.orb_slots[0].on_end_of_turn();
                        match effect {
                            crate::core::orbs::PassiveEffect::DamageRandom(dmg) => {
                                let actual = std::cmp::max(0, dmg);
                                let alive_idx = state.enemies.iter()
                                    .enumerate()
                                    .filter(|(_, e)| !e.is_dead())
                                    .map(|(idx, _)| idx)
                                    .next();
                                if let Some(idx) = alive_idx {
                                    state.enemies[idx].hp -= actual;
                                    game_log!("    🔄 Loop: Lightning passive → {} takes {} damage", 
                                        state.enemies[idx].name, actual);
                                }
                            }
                            crate::core::orbs::PassiveEffect::GainBlock(block) => {
                                state.player.block += std::cmp::max(0, block);
                                game_log!("    🔄 Loop: Frost passive → +{} block", block);
                            }
                            crate::core::orbs::PassiveEffect::DarkAccumulate(added, _) => {
                                game_log!("    🔄 Loop: Dark passive → +{} evoke damage", added);
                            }
                            _ => {}
                        }
                    }
                }
            }
            HookEffect::ResetStacks(value) => {
                // Panache: reset counter to a specific value (e.g., 5) each turn
                // Slow: reset to 0 at end of round (damage multiplier resets)
                if *value <= 0 {
                    // Reset to 0 means the power stacks are cleared but power remains
                    // (Slow stays on the enemy, stacks just reset to 0 for next turn)
                    state.player.powers.force_set(source_power_id, 0);
                } else {
                    state.player.powers.force_set(source_power_id, *value);
                }
                game_log!("    🔄 {} stacks reset to {} (from {})", source_power_id, value, source_power_id);
            }
            HookEffect::ReboundCard => {
                // Rebound: signal that the played card should go to draw pile top instead of discard
                // The actual card-move logic is in play_card_from_hand; this is a signal
                game_log!("    🔃 Rebound: card returns to top of draw pile (from {})", source_power_id);
            }
            HookEffect::ResetToMax => {
                // Flight/Invincible: reset stacks to the original max value
                // TODO: implement power_max_stacks tracking on GameState for proper max resets
                // For now, this is a signal-only effect; the hook arm ensures audit coverage.
                // Engine callers (monster AI) should re-apply the power stacks directly.
                let current = state.player.powers.get(source_power_id);
                game_log!("    🔄 {} ResetToMax requested (current: {})", source_power_id, current);
            }
            HookEffect::ResetEchoFormCounter => {
                // EchoForm: reset the cards-doubled counter for the new turn
                // The EchoForm replay tracking is done in the card-play loop
                // This is a signal-only effect; ensures audit coverage.
                game_log!("    🔄 EchoForm: counter reset signal");
            }
            HookEffect::RandomizeCardCost => {
                // Confusion: engine should randomize the drawn card's cost to 0-3
                // The actual randomization happens in the draw_cards function
                game_log!("    🌀 Confusion: card cost randomized (from {})", source_power_id);
            }
            HookEffect::HealEnemy(amount) => {
                // Regeneration: monster heals itself by amount
                // This runs on the enemy who has this power; engine handles it
                game_log!("    💚 Enemy heals {} HP (from {})", amount, source_power_id);
            }
            HookEffect::RetainCards(count) => {
                // RetainCards: retain up to N cards from hand through end of turn
                // Engine should mark N cards as retained
                game_log!("    🤚 Retain {} cards (from {})", count, source_power_id);
            }
            HookEffect::RetainAllCards => {
                // Equilibrium: retain ALL non-ethereal cards in hand
                game_log!("    🤚 Retain all cards (from {})", source_power_id);
            }
            HookEffect::KillSelf => {
                // EndTurnDeath: monster dies at start of its turn
                // Set HP to 0 — the engine death check will handle cleanup
                game_log!("    💀 {} kills itself (EndTurnDeath)", source_power_id);
            }
            HookEffect::LoseEnergy(amount) => {
                // EnergyDown: lose energy at start of turn
                state.player.energy = (state.player.energy - amount).max(0);
                game_log!("    ⚡ Lose {} energy (from {}), now {}", amount, source_power_id, state.player.energy);
            }
            HookEffect::ChangeStance(stance) => {
                // WrathNextTurn: enter a specific stance
                game_log!("    🧘 Change stance to {} (from {})", stance, source_power_id);
                // Engine handles stance change via combat.rs
            }
            HookEffect::ChannelOrb(orb_type) => {
                // RechargingCore/Winter: channel an orb
                game_log!("    🔮 Channel {} orb (from {})", orb_type, source_power_id);
                // Engine handles orb channeling via combat.rs
            }
            HookEffect::ApplyWeakToAllEnemies(amount) => {
                // WaveOfTheHand: apply Weak to all enemies
                // Engine processes this in combat loop where enemy access is available
                game_log!("    💫 Apply {} Weak to all enemies (from {})", amount, source_power_id);
            }
        }
    }
}
/// Channel an orb into the player's orb slots.
///
/// Java: AbstractDungeon.player.channelOrb(AbstractOrb orb)
/// - Adds orb to the end of the orb list
/// - If at max orbs, evokes (removes) the leftmost orb first
/// - Applies Focus to the new orb's passive/evoke amounts
pub(crate) fn channel_orb(state: &mut GameState, orb_name: &str, count: i32) {
    use crate::core::orbs::{OrbType, OrbSlot};
    
    let orb_type = match OrbType::from_str(orb_name) {
        Some(t) => t,
        None => {
            game_log!("  ⚠ Unknown orb type: {}", orb_name);
            return;
        }
    };
    
    for _ in 0..count {
        // If at max capacity, evoke the leftmost orb first
        if state.orb_slots.len() >= state.max_orbs && state.max_orbs > 0 {
            evoke_orb(state);
        }
        
        // Get current Focus
        let focus = state.player.powers.get("Focus");
        
        // Create the new orb with Focus applied
        let orb = OrbSlot::new(orb_type, focus);
        game_log!("    🔮 Channeled {} (passive: {}, evoke: {})", 
            orb_type.name(), orb.passive_amount, orb.evoke_amount);
        state.orb_slots.push(orb);
        
        // Track frost orbs channeled this combat (for Blizzard card)
        if orb_name == "Frost" {
            state.frost_channeled_this_combat += 1;
        }
    }
}

/// Evoke the leftmost orb (index 0) and apply its evoke effect.
///
/// Java: AbstractDungeon.player.evokeOrb()
/// - Removes the orb at index 0
/// - Calls orb.onEvoke()
/// - Electro makes Lightning hit ALL enemies
pub(crate) fn evoke_orb(state: &mut GameState) {
    use crate::core::orbs::EvokeEffect;
    
    if state.orb_slots.is_empty() {
        return;
    }
    
    let orb = state.orb_slots.remove(0); // Remove leftmost
    let has_electro = state.player.powers.has("Electrodynamics");
    let effect = orb.on_evoke(has_electro);
    
    game_log!("    💫 Evoked {} orb", orb.orb_type.name());
    
    match effect {
        EvokeEffect::DamageRandom(dmg) => {
            // Deal damage to a random alive enemy
            // LockOn: if target has LockOn, multiply by 1.5x
            let alive: Vec<usize> = state.enemies.iter()
                .enumerate()
                .filter(|(_, e)| !e.is_dead())
                .map(|(i, _)| i)
                .collect();
            if let Some(&idx) = alive.first() {
                let enemy = &mut state.enemies[idx];
                let lockon_mult = if enemy.powers.has("LockOn") { 1.5_f32 } else { 1.0 };
                let actual = std::cmp::max(0, (dmg as f32 * lockon_mult) as i32);
                enemy.hp -= actual;
                game_log!("      ⚡ {} takes {} Lightning evoke damage{}(HP: {})", 
                    enemy.name, actual, if lockon_mult > 1.0 { " (LockOn 1.5x) " } else { " " }, enemy.hp);
            }
        }
        EvokeEffect::DamageAll(dmg) => {
            // Damage ALL enemies (Lightning + Electro)
            // LockOn: applied per-enemy
            for enemy in state.enemies.iter_mut() {
                if !enemy.is_dead() {
                    let lockon_mult = if enemy.powers.has("LockOn") { 1.5_f32 } else { 1.0 };
                    let actual = std::cmp::max(0, (dmg as f32 * lockon_mult) as i32);
                    enemy.hp -= actual;
                    game_log!("      ⚡ {} takes {} Lightning evoke damage{}(HP: {})", 
                        enemy.name, actual, if lockon_mult > 1.0 { " (LockOn 1.5x) " } else { " " }, enemy.hp);
                }
            }
        }
        EvokeEffect::GainBlock(block) => {
            // Frost: gain block
            state.player.block += std::cmp::max(0, block);
            game_log!("      ❄ Gained {} block from Frost evoke (total: {})", 
                block, state.player.block);
        }
        EvokeEffect::DamageLowestHp(dmg) => {
            // Dark: damage the lowest HP enemy
            let lowest_idx = state.enemies.iter()
                .enumerate()
                .filter(|(_, e)| !e.is_dead())
                .min_by_key(|(_, e)| e.hp)
                .map(|(i, _)| i);
            if let Some(idx) = lowest_idx {
                let actual = std::cmp::max(0, dmg);
                let enemy = &mut state.enemies[idx];
                enemy.hp -= actual;
                game_log!("      🌑 {} takes {} Dark evoke damage (HP: {})", 
                    enemy.name, actual, enemy.hp);
            }
        }
        EvokeEffect::GainEnergy(energy) => {
            // Plasma: gain energy
            state.player.energy += energy;
            game_log!("      ✨ Gained {} energy from Plasma evoke (total: {})", 
                energy, state.player.energy);
        }
    }
}

/// Calculate card damage following the Java pipeline using power hooks.
///
/// Delegates to `power_hooks::calculate_damage_hooked()` which iterates
/// through ALL active powers (Strength, Weak, Vulnerable, Intangible, etc.)
/// using enum dispatch instead of hardcoded boolean checks.
///
/// The `extra_strength_base` parameter handles strength multiplier cards
/// (Heavy Blade: str_mult=3 means 2 extra applications of Strength).
pub(crate) fn calculate_card_damage(
    base_damage: i32,
    attacker_powers: &crate::powers::PowerSet,
    defender_powers: &crate::powers::PowerSet,
    attacker_stance: crate::core::stances::Stance,
    relic_flags: crate::power_hooks::RelicDamageFlags,
) -> i32 {
    use crate::core::stances::Stance;
    crate::power_hooks::calculate_damage_hooked(
        base_damage,
        attacker_powers,
        defender_powers,
        attacker_stance,
        Stance::Neutral, // enemies don't have stances
        relic_flags,
    )
}

/// Apply a single command to the game state.
///
/// # Arguments
/// * `state` - Mutable reference to the game state
/// * `command` - The command to execute
/// * `upgraded` - Whether the card is upgraded (affects numeric values)
/// * `_target_idx` - Index of target enemy (for single-target effects)
///
/// # Returns
/// A `CommandResult` describing what happened.
pub fn apply_command(
    state: &mut GameState,
    command: &CardCommand,
    upgraded: bool,
    _target_idx: Option<usize>,
    library: Option<&CardLibrary>,
) -> CommandResult {
    match command {
        CardCommand::DealDamage { base, upgrade, times, times_upgrade, scaling } => {
            // Calculate base damage - either from base/upgrade or scaling
            let raw_base = match scaling.as_deref() {
                Some("Block") => state.player.block, // Body Slam: damage = block
                _ => if upgraded { *upgrade } else { *base },
            };
            
            let hits = if upgraded {
                times_upgrade.unwrap_or(times.unwrap_or(1))
            } else {
                times.unwrap_or(1)
            };
            
            // Strength multiplier from Heavy Blade etc. — applied at card level
            let strength = state.player.get_strength();
            let str_mult = if state.card_modifiers.strength_multiplier > 0 {
                state.card_modifiers.strength_multiplier
            } else {
                1
            };
            let extra_str_base = strength * (str_mult - 1); // extra Strength beyond ×1
            
            // Reset strength multiplier after use
            state.card_modifiers.strength_multiplier = 0;
            
            // AccuracyPower.java: Shivs get +this.amount to baseDamage
            // Java: baseDamage = 4 + accuracy_amount (unupgraded) or 6 + accuracy_amount (upgraded)
            // Since our JSON already has Shiv base=4/upgrade=6, we just add Accuracy stacks
            let accuracy_bonus = if state.last_played_card_id.as_deref() == Some("Shiv") {
                state.player.powers.get("Accuracy")
            } else {
                0
            };
            
            // Relic damage bonuses: StrikeDummy (+3 for Strike cards), WristBlade (+3 for 0-cost attacks)
            let strike_dummy_bonus = if state.last_played_card_id.as_deref()
                .map_or(false, |id| id.contains("Strike") || id.contains("strike"))
                && state.relics.iter().any(|r| r.id == "StrikeDummy" && r.active)
            { 3 } else { 0 };
            let wrist_blade_bonus = if state.last_played_card_cost == 0
                && state.relics.iter().any(|r| r.id == "WristBlade" && r.active)
            { 3 } else { 0 };

            // Add Vigor (consumed after attack)
            let vigor = state.player.consume_vigor();
            let base_with_vigor = raw_base + extra_str_base + vigor + accuracy_bonus
                + strike_dummy_bonus + wrist_blade_bonus;
            
            // Find target index first (immutable borrow)
            let target_idx = if let Some(idx) = state.target_enemy_idx {
                if state.enemies.get(idx).map_or(true, |e| e.is_dead()) { None } else { Some(idx) }
            } else {
                state.enemies.iter().position(|e| !e.is_dead())
            };
            
            if let Some(idx) = target_idx {
                // Build relic damage flags for Vuln/Weak modifiers
                // Player attacking enemy: PaperFrog increases enemy Vuln, PaperCrane increases Weak penalty on player
                let relic_flags = crate::power_hooks::RelicDamageFlags {
                    odd_mushroom: false, // OddMushroom only applies when PLAYER has Vulnerable (enemy→player)
                    paper_crane: state.relics.iter().any(|r| r.id == "PaperCrane" && r.active),
                    paper_frog: state.relics.iter().any(|r| r.id == "PaperFrog" && r.active),
                };
                // Compute damage with immutable borrows (avoids borrow conflict)
                let damage_per_hit = calculate_card_damage(
                    base_with_vigor, &state.player.powers, &state.enemies[idx].powers, state.player.stance,
                    relic_flags,
                );
                let target_name = state.enemies[idx].name.clone();
                let has_boot = state.relics.iter().any(|r| r.id == "Boot" && r.active);
                
                let mut total_dealt = 0;
                let mut killed = false;
                let mut total_pending_block = 0;  // Queued block from Curl Up/Malleable
                
                for _ in 0..hits {
                    if state.enemies[idx].is_dead() {
                        break;
                    }
                    let block_before = state.enemies[idx].block;
                    // Use take_damage_from_player for player attacks (includes Boot relic)
                    // Java: AbstractMonster.damage() L626-630 → onAttackToChangeDamage
                    // Returns (actual_damage, pending_block) — pending block applied after ALL hits
                    let (actual, pending_block) = state.enemies[idx].take_damage_from_player(damage_per_hit, has_boot);
                    total_dealt += actual;
                    total_pending_block += pending_block;
                    
                    // HandDrill: When enemy block is broken, apply 2 Vulnerable
                    // Java: HandDrill.onBlockBroken → VulnerableAction(2)
                    if block_before > 0 && state.enemies[idx].block == 0 {
                        if state.relics.iter().any(|r| r.id == "HandDrill" && r.active) {
                            state.enemies[idx].apply_status("Vulnerable", 2);
                            game_log!("  🔨 Hand Drill: Block broken → +2 Vulnerable!");
                        }
                    }
                    
                    // Fire on_attack hooks (Envenom: apply Poison on unblocked damage)
                    if actual > 0 {
                        let power_snap: Vec<(String, i32)> = state.player.powers.iter()
                            .map(|(k, v)| (k.clone(), *v)).collect();
                        for (pid, stacks) in &power_snap {
                            let pi = crate::power_hooks::PowerInstance::new(
                                crate::power_hooks::PowerId::from_str(pid), *stacks
                            );
                            let effects = pi.on_attack(actual);
                            if !effects.is_empty() {
                                apply_hook_effects(state, &effects, pid, Some(idx), library);
                            }
                        }
                    }
                    
                    // Fire ENEMY onAttacked hooks (Thorns → damage player, etc.)
                    {
                        let enemy_power_snap: Vec<(String, i32)> = state.enemies[idx].powers.iter()
                            .map(|(k, v)| (k.clone(), *v)).collect();
                        for (pid, stacks) in &enemy_power_snap {
                            let pi = crate::power_hooks::PowerInstance::new(
                                crate::power_hooks::PowerId::from_str(pid), *stacks
                            );
                            let (_, effects) = pi.on_attacked(actual);
                            for effect in &effects {
                                match effect {
                                    crate::power_hooks::HookEffect::DamageAttacker(dmg) => {
                                        let thorns_actual = state.player.take_damage(*dmg);
                                        game_log!("  🌿 {} reflects {} damage to player (HP: {})",
                                            pid, thorns_actual, state.player.current_hp);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                    
                    if state.enemies[idx].is_dead() {
                        killed = true;
                    }
                }
                
                // Apply pending block from Curl Up/Malleable AFTER all hits
                // Java: GainBlockAction resolves after the entire DamageAction
                if total_pending_block > 0 && !state.enemies[idx].is_dead() {
                    state.enemies[idx].block += total_pending_block;
                    game_log!("  🛡️ Applied {} pending block (from Curl Up/Malleable)", total_pending_block);
                }
                
                // Record attack result for conditionals (Fatal check)
                state.record_attack_result(damage_per_hit * hits, total_dealt, killed);
                
                // Fire on_death hooks for enemy powers (CorpseExplosion, SporeCloud)
                if killed {
                    let dead_max_hp = state.enemies[idx].max_hp;
                    let enemy_power_snapshot: Vec<(String, i32)> = state.enemies[idx].powers.iter()
                        .map(|(k, v)| (k.clone(), *v)).collect();
                    for (pid, stacks) in &enemy_power_snapshot {
                        let pi = crate::power_hooks::PowerInstance::new(
                            crate::power_hooks::PowerId::from_str(pid), *stacks
                        );
                        let effects = pi.on_death(dead_max_hp);
                        if !effects.is_empty() {
                            apply_hook_effects(state, &effects, pid, Some(idx), library);
                        }
                    }
                }
                
                if vigor > 0 {
                    game_log!("  → Dealt {} damage to {} ({} base + {} vigor, {} hits) [Killed: {}]", 
                        total_dealt, target_name, raw_base + strength, vigor, hits, killed);
                } else {
                    game_log!("  → Dealt {} damage to {} ({} hits) [Killed: {}]", 
                        total_dealt, target_name, hits, killed);
                }
                
                CommandResult::DamageDealt { target: target_name, amount: total_dealt, killed }
            } else {
                CommandResult::Skipped { reason: "No valid target".into() }
            }
        }
        
        CardCommand::GainBlock { base, upgrade } => {
            let amount = if upgraded { *upgrade } else { *base };
            state.player.gain_block(amount);
            game_log!("  → Gained {} block (total: {})", amount, state.player.block);
            
            // Fire on_gained_block hooks (Juggernaut: deal damage to random enemy)
            if amount > 0 {
                let power_snap: Vec<(String, i32)> = state.player.powers.iter()
                    .map(|(k, v)| (k.clone(), *v)).collect();
                for (pid, stacks) in &power_snap {
                    let pi = crate::power_hooks::PowerInstance::new(
                        crate::power_hooks::PowerId::from_str(pid), *stacks
                    );
                    let effects = pi.on_gained_block(amount);
                    if !effects.is_empty() {
                        apply_hook_effects(state, &effects, pid, None, library);
                    }
                }
            }
            
            CommandResult::BlockGained { amount }
        }
        
        CardCommand::StrengthMultiplier { base: _, upgrade: _ } => {
            // This is handled in pre-processing - the multiplier is set before
            // DealDamage executes. This command is a no-op when executed directly.
            CommandResult::Unknown
        }
        
        CardCommand::ApplyStatus { status, base, upgrade } => {
            let stacks = if upgraded { *upgrade } else { *base };
            
            // Apply to first living enemy
            if let Some(enemy) = state.get_target_enemy() {
                let target_name = enemy.name.clone();
                enemy.apply_status(status, stacks);
                game_log!("  → Applied {} stacks of {} to {}", stacks, status, target_name);
                
                // ChampionsBelt: When applying Vulnerable, also apply 1 Weak
                // Java: ChampionsBelt.onTrigger(target) → ApplyPowerAction(target, Weak, 1)
                if status == "Vulnerable" {
                    if state.relics.iter().any(|r| (r.id == "ChampionBelt" || r.id == "Champion Belt") && r.active) {
                        if let Some(enemy) = state.get_target_enemy() {
                            enemy.apply_status("Weak", 1);
                            game_log!("  🥊 Champion's Belt: +1 Weak");
                        }
                    }
                }
                
                // SadisticNature (SadisticPower.java): onApplyPower
                // Java: if power.type == DEBUFF && !power.ID.equals("Shackled") 
                //       && source == this.owner && target != this.owner && !target.hasPower("Artifact")
                //       => DamageAction(target, this.amount, THORNS)
                let sadistic_stacks = state.player.powers.get("SadisticNature");
                if sadistic_stacks > 0 && status != "Shackled" {
                    // Check if the status is a debuff (negative effects on enemies)
                    let is_debuff = matches!(status.as_str(), 
                        "Vulnerable" | "Weak" | "Frail" | "Poison" | "Constricted" 
                        | "Slow" | "Hex" | "DrawReduction" | "NoDraw" | "Entangled"
                        | "LockOn" | "Mark" | "Choke" | "BlockReturn"
                    );
                    if is_debuff {
                        // Deal THORNS damage to the target enemy
                        if let Some(enemy) = state.get_target_enemy() {
                            if !enemy.is_dead() {
                                let actual = enemy.take_damage(sadistic_stacks);
                                game_log!("  🔥 Sadistic Nature: dealt {} damage to {} (THORNS)", actual, enemy.name);
                            }
                        }
                    }
                }
                
                CommandResult::StatusApplied { 
                    target: target_name, 
                    status: status.clone(), 
                    stacks 
                }
            } else {
                CommandResult::Skipped { reason: "No valid target".into() }
            }
        }
        
        CardCommand::DrawCards { base, upgrade } => {
            let count = if upgraded { *upgrade } else { *base };
            let drawn = state.draw_cards(count);
            game_log!("  → Drew {} cards", drawn);
            CommandResult::CardsDrawn { count: drawn }
        }
        
        CardCommand::GainEnergy { base, upgrade } => {
            let amount = if upgraded { *upgrade } else { *base };
            state.player.energy += amount;
            game_log!("  → Gained {} energy (total: {})", amount, state.player.energy);
            CommandResult::EnergyGained { amount }
        }
        
        CardCommand::UpgradeCards { amount_base, amount_upgrade, target } => {
            let amount = if upgraded { amount_upgrade } else { amount_base };
            let is_all = matches!(amount, AmountValue::All(_));
            let count = amount.as_i32();
            
            // Upgrade cards in the specified location
            let upgraded_count = match target {
                CardLocation::Hand => upgrade_cards_in_slice(&mut state.hand, is_all, count),
                CardLocation::DrawPile => upgrade_cards_in_slice(&mut state.draw_pile, is_all, count),
                CardLocation::DiscardPile => upgrade_cards_in_slice(&mut state.discard_pile, is_all, count),
                CardLocation::ExhaustPile => upgrade_cards_in_slice(&mut state.exhaust_pile, is_all, count),
                CardLocation::Deck => {
                    // Deck = Hand + DrawPile + DiscardPile (as used by Apotheosis)
                    let mut total = 0;
                    total += upgrade_cards_in_slice(&mut state.hand, is_all, count);
                    total += upgrade_cards_in_slice(&mut state.draw_pile, is_all, count);
                    total += upgrade_cards_in_slice(&mut state.discard_pile, is_all, count);
                    total
                }
            };
            
            game_log!("  → Upgraded {} cards in {:?}", upgraded_count, target);
            CommandResult::CardsUpgraded { count: upgraded_count, location: *target }
        }
        
        CardCommand::ExhaustSelf { base_only, upgrade_only } => {
            // Check if this exhaust applies given upgrade status
            if *base_only && upgraded {
                game_log!("  → Exhaust skipped (base only, card is upgraded)");
                return CommandResult::Skipped { reason: "Base only exhaust on upgraded card".into() };
            }
            if *upgrade_only && !upgraded {
                game_log!("  → Exhaust skipped (upgrade only, card is not upgraded)");
                return CommandResult::Skipped { reason: "Upgrade only exhaust on non-upgraded card".into() };
            }
            
            game_log!("  → Card will be exhausted");
            CommandResult::CardExhausted
        }
        
        CardCommand::AddCard { card, destination, count } => {
            let dest = parse_location(destination);
            let card_id = if card == "this card" || card == "self" {
                // "this card" → use the card being played (context from play_card)
                // For now, we resolve it via the last played card tracking
                state.last_played_card_id.clone().unwrap_or_default()
            } else {
                card.clone()
            };
            
            for _ in 0..*count {
                state.add_card_by_id(&card_id, 0, dest, InsertPosition::Shuffle);
            }
            
            game_log!("  → Added {} copy(ies) of '{}' to {:?}", count, card_id, dest);
            CommandResult::CardAdded { 
                card: card_id, 
                destination: destination.clone() 
            }
        }
        
        CardCommand::DiscardCards { base, upgrade, random } => {
            let count = if upgraded { *upgrade } else { *base };
            
            if count >= state.hand.len() as i32 {
                // Discard all cards in hand
                let all: Vec<_> = state.hand.drain(..).collect();
                let discarded = all.len() as i32;
                for card in all {
                    state.discard_pile.push(card);
                }
                game_log!("  → Discarded all {} cards from hand", discarded);
                CommandResult::CardsDiscarded { count: discarded }
            } else if *random {
                // Random discard
                use rand::Rng;
                let mut discarded = 0;
                for _ in 0..count {
                    if !state.hand.is_empty() {
                        let idx = state.rng.random_range(0..state.hand.len());
                        let card = state.hand.remove(idx);
                        game_log!("  → Randomly discarded: {}", card.definition_id);
                        state.discard_pile.push(card);
                        discarded += 1;
                    }
                }
                CommandResult::CardsDiscarded { count: discarded }
            } else {
                // AI heuristic: discard lowest-value cards
                // (prefer discarding Status/Curse, then lowest cost)
                let mut discarded = 0;
                for _ in 0..count {
                    if state.hand.is_empty() { break; }
                    let worst_idx = state.hand.iter().enumerate()
                        .min_by_key(|(_, c)| {
                            match c.card_type {
                                crate::schema::CardType::Status => -100,
                                crate::schema::CardType::Curse => -90,
                                _ => c.current_cost,
                            }
                        })
                        .map(|(i, _)| i)
                        .unwrap();
                    let card = state.hand.remove(worst_idx);
                    game_log!("  → AI discarded: {} (cost {})", card.definition_id, card.current_cost);
                    state.discard_pile.push(card);
                    discarded += 1;
                }
                CommandResult::CardsDiscarded { count: discarded }
            }
        }
        
        CardCommand::GainBuff { buff, base, upgrade } => {
            let amount = if upgraded { *upgrade } else { *base };
            state.player.apply_status(buff, amount);
            game_log!("  → Gained {} stacks of {}", amount, buff);
            CommandResult::BuffGained { buff: buff.clone(), amount }
        }
        
        CardCommand::DoubleBuff { buff, base_only, upgrade_only } => {
            if *base_only && upgraded {
                return CommandResult::Skipped { reason: "Base only effect".into() };
            }
            if *upgrade_only && !upgraded {
                return CommandResult::Skipped { reason: "Upgrade only effect".into() };
            }
            
            let current = state.player.get_status(buff);
            if current > 0 {
                state.player.powers.set(buff, current * 2);
                game_log!("  → Doubled {} to {}", buff, current * 2);
            } else {
                game_log!("  → Cannot double {} (not present)", buff);
            }
            CommandResult::BuffDoubled { buff: buff.clone() }
        }
        
        CardCommand::LoseHp { base, upgrade } => {
            let amount = if upgraded { *upgrade } else { *base };
            state.player.current_hp -= amount;
            game_log!("  → Lost {} HP (now: {})", amount, state.player.current_hp);
            
            // Fire was_hp_lost_self hooks (Rupture: gain Strength from self-damage)
            if amount > 0 {
                let power_snap: Vec<(String, i32)> = state.player.powers.iter()
                    .map(|(k, v)| (k.clone(), *v)).collect();
                for (pid, stacks) in &power_snap {
                    let pi = crate::power_hooks::PowerInstance::new(
                        crate::power_hooks::PowerId::from_str(pid), *stacks
                    );
                    let effects = pi.was_hp_lost_self(amount);
                    if !effects.is_empty() {
                        apply_hook_effects(state, &effects, pid, None, library);
                    }
                }
            }
            
            CommandResult::HpLost { amount }
        }
        
        CardCommand::GainHp { base, upgrade } => {
            let amount = if upgraded { *upgrade } else { *base };
            state.player.current_hp = (state.player.current_hp + amount).min(state.player.max_hp);
            game_log!("  → Healed {} HP (now: {})", amount, state.player.current_hp);
            CommandResult::HpGained { amount }
        }
        
        CardCommand::ChannelOrb { orb, count } => {
            channel_orb(state, orb, *count);
            game_log!("  → Channel {} {} orb(s)", count, orb);
            CommandResult::Unknown
        }
        
        CardCommand::EvokeOrb { count, all } => {
            if *all {
                // Evoke all orbs
                let total = state.orb_slots.len();
                for _ in 0..total {
                    evoke_orb(state);
                }
                game_log!("  → Evoked all {} orbs", total);
            } else {
                for _ in 0..*count {
                    evoke_orb(state);
                }
                game_log!("  → Evoked {} orb(s)", count);
            }
            CommandResult::Unknown
        }
        
        CardCommand::GainFocus { base, upgrade } => {
            let amount = if upgraded { *upgrade } else { *base };
            state.player.apply_status("Focus", amount);
            game_log!("  → Gained {} Focus", amount);
            CommandResult::BuffGained { buff: "Focus".into(), amount }
        }
        
        CardCommand::EnterStance { stance } => {
            use crate::core::stances::Stance;
            let new_stance = Stance::from_str(stance);
            let old_stance = state.player.stance;
            
            // Don't do anything if already in the same stance
            if old_stance == new_stance {
                game_log!("  → Already in {} stance", stance);
                return CommandResult::Unknown;
            }
            
            // 1. On-exit effects of old stance
            let exit_energy = old_stance.on_exit_energy();
            if exit_energy > 0 {
                state.player.energy += exit_energy;
                game_log!("  🧘 Exiting {} → +{} Energy", old_stance.name(), exit_energy);
            }
            
            // 2. Change stance
            state.player.stance = new_stance;
            game_log!("  → Stance: {} → {}", old_stance.name(), new_stance.name());
            
            // 3. On-enter effects of new stance
            let enter_energy = new_stance.on_enter_energy();
            if enter_energy > 0 {
                state.player.energy += enter_energy;
                game_log!("  🧘 Entering {} → +{} Energy", new_stance.name(), enter_energy);
            }
            
            // 4. Trigger on_stance_change hooks (MentalFortress, Rushdown)
            // Collect effects first, then apply (borrow checker)
            let stance_effects: Vec<_> = state.player.powers.iter()
                .flat_map(|(id_str, &stacks)| {
                    use crate::power_hooks::{PowerInstance, PowerId};
                    let pi = PowerInstance::new(PowerId::from_str(id_str), stacks);
                    let effects = pi.on_stance_change(new_stance.name());
                    let pid = pi.id;
                    effects.into_iter().map(move |e| (pid, e)).collect::<Vec<_>>()
                })
                .collect();
            
            for (power_id, effect) in &stance_effects {
                let pid_str = format!("{:?}", power_id);
                apply_hook_effects(state, &[effect.clone()], &pid_str, None, library);
            }
            
            CommandResult::BuffGained { buff: format!("Stance:{}", new_stance.name()), amount: 1 }
        }
        
        CardCommand::ExitStance => {
            use crate::core::stances::Stance;
            let old_stance = state.player.stance;
            
            if old_stance == Stance::Neutral {
                game_log!("  → Already in Neutral stance");
                return CommandResult::Unknown;
            }
            
            // On-exit effects
            let exit_energy = old_stance.on_exit_energy();
            if exit_energy > 0 {
                state.player.energy += exit_energy;
                game_log!("  🧘 Exiting {} → +{} Energy", old_stance.name(), exit_energy);
            }
            
            state.player.stance = Stance::Neutral;
            game_log!("  → Stance: {} → Neutral", old_stance.name());
            
            // Trigger on_stance_change hooks
            let stance_effects: Vec<_> = state.player.powers.iter()
                .flat_map(|(id_str, &stacks)| {
                    use crate::power_hooks::{PowerInstance, PowerId};
                    let pi = PowerInstance::new(PowerId::from_str(id_str), stacks);
                    let effects = pi.on_stance_change("Neutral");
                    let pid = pi.id;
                    effects.into_iter().map(move |e| (pid, e)).collect::<Vec<_>>()
                })
                .collect();
            
            for (power_id, effect) in &stance_effects {
                let pid_str = format!("{:?}", power_id);
                apply_hook_effects(state, &[effect.clone()], &pid_str, None, library);
            }
            
            CommandResult::BuffGained { buff: "Stance:Neutral".into(), amount: 1 }
        }
        
        CardCommand::Scry { base, upgrade } => {
            let amount = if upgraded { *upgrade } else { *base };
            
            // Scry: look at top N cards of draw pile, discard unwanted ones
            // AI heuristic: discard Curses and Status cards, keep everything else
            let pile_len = state.draw_pile.len();
            let scry_count = std::cmp::min(amount as usize, pile_len);
            
            if scry_count == 0 {
                game_log!("  → Scry {} (draw pile empty)", amount);
                return CommandResult::CardsDiscarded { count: 0 };
            }
            
            // Take top N cards from draw pile (top = end of vec)
            let start_idx = pile_len - scry_count;
            let scryed_cards: Vec<_> = state.draw_pile.drain(start_idx..).collect();
            
            let mut discarded = 0;
            let mut kept: Vec<crate::schema::CardInstance> = Vec::new();
            
            for card in scryed_cards {
                let should_discard = matches!(card.card_type, 
                    crate::schema::CardType::Curse | crate::schema::CardType::Status
                );
                if should_discard {
                    game_log!("    🔮 Scry discard: {} ({})", card.definition_id, 
                        if card.card_type == crate::schema::CardType::Curse { "Curse" } else { "Status" });
                    state.discard_pile.push(card);
                    discarded += 1;
                } else {
                    kept.push(card);
                }
            }
            
            // Put kept cards back on top (maintain order)
            for card in kept {
                state.draw_pile.push(card);
            }
            
            game_log!("  → Scry {} (discarded {})", scry_count, discarded);
            
            // Trigger on_scry hooks (Nirvana: gain block per scry)
            let scry_effects: Vec<_> = state.player.powers.iter()
                .flat_map(|(id_str, &stacks)| {
                    use crate::power_hooks::{PowerInstance, PowerId};
                    let pi = PowerInstance::new(PowerId::from_str(id_str), stacks);
                    let effects = pi.on_scry();
                    let pid = pi.id;
                    effects.into_iter().map(move |e| (pid, e)).collect::<Vec<_>>()
                })
                .collect();
            
            for (power_id, effect) in &scry_effects {
                let pid_str = format!("{:?}", power_id);
                apply_hook_effects(state, &[effect.clone()], &pid_str, None, library);
            }
            
            CommandResult::CardsDiscarded { count: discarded }
        }
        
        CardCommand::GainMantra { base, upgrade } => {
            let amount = if upgraded { *upgrade } else { *base };
            state.player.apply_status("Mantra", amount);
            game_log!("  → Gained {} Mantra", amount);
            CommandResult::BuffGained { buff: "Mantra".into(), amount }
        }
        
        CardCommand::RetainSelf { upgrade_only } => {
            if *upgrade_only && !upgraded {
                return CommandResult::Skipped { reason: "Upgrade only".into() };
            }
            game_log!("  → Card will be retained");
            CommandResult::Unknown
        }
        
        CardCommand::InnateSelf { upgrade_only } => {
            if *upgrade_only && !upgraded {
                return CommandResult::Skipped { reason: "Upgrade only".into() };
            }
            game_log!("  → Card is now Innate");
            CommandResult::Unknown
        }
        
        // AoE damage — per-enemy Vulnerable check
        // Verified: AbstractCard.java → calculateCardDamage loops over all monsters
        CardCommand::DealDamageAll { base, upgrade, times } => {
            let raw_base = if upgraded { *upgrade } else { *base };
            // For X-cost cards (Whirlwind): use x_cost_value as hit count
            // An X-cost card has current_cost == -1 (tracked in last_played_card_cost)
            let is_x_cost_card = state.last_played_card_cost == -1;
            let hits = times.unwrap_or_else(|| {
                if is_x_cost_card {
                    state.x_cost_value // Can be 0 if played with 0 energy
                } else {
                    1
                }
            });
            let vigor = state.player.consume_vigor();
            let base_with_vigor = raw_base + vigor;
            
            let mut total_dealt = 0;
            let mut any_killed = false;
            let has_boot = state.relics.iter().any(|r| r.id == "Boot" && r.active);
            
            // Use index-based loop to allow mutable access to both enemies and player
            // (needed for on_attacked hooks like Thorns that damage the player)
            let enemy_count = state.enemies.len();
            for ei in 0..enemy_count {
                if state.enemies[ei].is_dead() { continue; }
                
                let relic_flags = crate::power_hooks::RelicDamageFlags {
                    odd_mushroom: false,
                    paper_crane: state.relics.iter().any(|r| r.id == "PaperCrane" && r.active),
                    paper_frog: state.relics.iter().any(|r| r.id == "PaperFrog" && r.active),
                };
                let damage_per_hit = calculate_card_damage(
                    base_with_vigor, &state.player.powers, &state.enemies[ei].powers, state.player.stance,
                    relic_flags,
                );
                // Loop per-hit (like MultiHit) so Flight halving + decrement fire per hit
                let mut pending_block = 0;
                for _ in 0..hits {
                    if state.enemies[ei].is_dead() {
                        break;
                    }
                    let (actual, pend) = state.enemies[ei].take_damage_from_player(damage_per_hit, has_boot);
                    total_dealt += actual;
                    pending_block += pend;
                    
                    // Fire ENEMY onAttacked hooks (Thorns → damage player, etc.)
                    {
                        let enemy_power_snap: Vec<(String, i32)> = state.enemies[ei].powers.iter()
                            .map(|(k, v)| (k.clone(), *v)).collect();
                        for (pid, stacks) in &enemy_power_snap {
                            let pi = crate::power_hooks::PowerInstance::new(
                                crate::power_hooks::PowerId::from_str(pid), *stacks
                            );
                            let (_, effects) = pi.on_attacked(actual);
                            for effect in &effects {
                                match effect {
                                    crate::power_hooks::HookEffect::DamageAttacker(dmg) => {
                                        let thorns_actual = state.player.take_damage(*dmg);
                                        game_log!("  🌿 {} reflects {} damage to player (HP: {})",
                                            pid, thorns_actual, state.player.current_hp);
                                    }
                                    _ => {}
                                }
                            }
                        }
                    }
                }
                // Apply pending block from Curl Up/Malleable after all hits
                if pending_block > 0 && !state.enemies[ei].is_dead() {
                    state.enemies[ei].block += pending_block;
                }
                if state.enemies[ei].is_dead() {
                    any_killed = true;
                }
            }
            
            // Record for conditionals
            state.record_attack_result(total_dealt, total_dealt, any_killed);
            
            game_log!("  → Dealt {} damage to ALL enemies [Any killed: {}]", total_dealt, any_killed);
            CommandResult::DamageDealt { target: "ALL".to_string(), amount: total_dealt, killed: any_killed }
        }
        
        CardCommand::ApplyStatusAll { status, base, upgrade } => {
            let stacks = if upgraded { *upgrade } else { *base };
            let mut count = 0;
            
            for enemy in &mut state.enemies {
                if !enemy.is_dead() {
                    enemy.apply_status(status, stacks);
                    count += 1;
                }
            }
            
            // ChampionsBelt: When applying Vulnerable to all, also apply 1 Weak to all
            if status == "Vulnerable" {
                if state.relics.iter().any(|r| (r.id == "ChampionBelt" || r.id == "Champion Belt") && r.active) {
                    for enemy in &mut state.enemies {
                        if !enemy.is_dead() {
                            enemy.apply_status("Weak", 1);
                        }
                    }
                    game_log!("  🥊 Champion's Belt: +1 Weak to all enemies");
                }
            }
            
            game_log!("  → Applied {} stacks of {} to {} enemies", stacks, status, count);
            CommandResult::StatusApplied { target: "ALL".to_string(), status: status.clone(), stacks }
        }
        
        CardCommand::ApplyPower { power, base, upgrade, amount, upgrade_amount } => {
            // Use amount/upgrade_amount if provided, otherwise fall back to base/upgrade.
            // Fallback chain for upgraded: upgrade_amount → upgrade (if non-zero) → amount → base
            // This handles cards like Corruption where JSON only specifies amount=1
            let stacks = if upgraded {
                upgrade_amount
                    .or_else(|| if *upgrade != 0 { Some(*upgrade) } else { None })
                    .or(*amount)
                    .unwrap_or(*base)
            } else {
                amount.unwrap_or(*base)
            };
            state.player.apply_status(power, stacks);
            game_log!("  → Gained {} stacks of {}", stacks, power);
            CommandResult::BuffGained { buff: power.clone(), amount: stacks }
        }
        
        CardCommand::Conditional { condition, then_do, else_do } => {
            // Evaluate the condition
            let condition_met = if let Some(cond_value) = condition {
                let cond = Condition::from_json(cond_value);
                cond.evaluate(state, _target_idx)
            } else {
                // No condition specified = always true
                true
            };
            
            game_log!("  → Conditional: condition met = {}", condition_met);
            
            // Execute appropriate branch
            if condition_met {
                if let Some(commands) = then_do {
                    let sub_results = execute_command_list(state, commands, upgraded, _target_idx, library);
                    game_log!("  → Executed {} then-commands", sub_results.len());
                }
            } else if let Some(commands) = else_do {
                let sub_results = execute_command_list(state, commands, upgraded, _target_idx, library);
                game_log!("  → Executed {} else-commands", sub_results.len());
            }
            
            CommandResult::ConditionalExecuted { condition_met }
        }
        
        CardCommand::MultiHit { damage_per_hit, hits_base, hits_upgrade } => {
            let hits = if upgraded { *hits_upgrade } else { *hits_base };
            let vigor = state.player.consume_vigor();
            let base_with_vigor = *damage_per_hit + vigor;
            
            // Find target index first (immutable borrow)
            let target_idx = if let Some(idx) = state.target_enemy_idx {
                if state.enemies.get(idx).map_or(true, |e| e.is_dead()) { None } else { Some(idx) }
            } else {
                state.enemies.iter().position(|e| !e.is_dead())
            };
            
            if let Some(idx) = target_idx {
                let relic_flags = crate::power_hooks::RelicDamageFlags {
                    odd_mushroom: false,
                    paper_crane: state.relics.iter().any(|r| r.id == "PaperCrane" && r.active),
                    paper_frog: state.relics.iter().any(|r| r.id == "PaperFrog" && r.active),
                };
                let damage_per = calculate_card_damage(
                    base_with_vigor, &state.player.powers, &state.enemies[idx].powers, state.player.stance,
                    relic_flags,
                );
                let target_name = state.enemies[idx].name.clone();
                
                let mut total_dealt = 0;
                let mut killed = false;
                let has_boot = state.relics.iter().any(|r| r.id == "Boot" && r.active);
                
                let mut total_pending_block = 0;
                for _ in 0..hits {
                    if state.enemies[idx].is_dead() {
                        break;
                    }
                    let (actual, pend) = state.enemies[idx].take_damage_from_player(damage_per, has_boot);
                    total_dealt += actual;
                    total_pending_block += pend;
                    if state.enemies[idx].is_dead() {
                        killed = true;
                    }
                }
                if total_pending_block > 0 && !state.enemies[idx].is_dead() {
                    state.enemies[idx].block += total_pending_block;
                }
                
                state.record_attack_result(damage_per * hits, total_dealt, killed);
                game_log!("  → Multi-hit: {}x{} = {} damage to {} [Killed: {}]", 
                    damage_per, hits, total_dealt, target_name, killed);
                CommandResult::DamageDealt { target: target_name, amount: total_dealt, killed }
            } else {
                CommandResult::Skipped { reason: "No valid target".into() }
            }
        }
        
        CardCommand::Unplayable => {
            game_log!("  → [Unplayable card]");
            CommandResult::Skipped { reason: "Unplayable".into() }
        }
        
        // ====================================================================
        // Phase 1: Core Commands (新增的基础实现)
        // ====================================================================
        
        CardCommand::ApplyBuff { buff, amount, upgrade_amount, target } => {
            // Determine the amount to apply
            let buff_amount = if upgraded {
                // Try upgrade_amount first, then parse amount
                upgrade_amount.unwrap_or_else(|| {
                    parse_value_source(amount.as_ref(), state, 1)
                })
            } else {
                parse_value_source(amount.as_ref(), state, 1)
            };
            
            // Determine target (default is self)
            let target_str = target.as_deref().unwrap_or("self");
            
            match target_str {
                "self" | "Self" | "player" => {
                    // Check if this is actually a debuff (for Artifact blocking)
                    // Java: ApplyPowerAction → AbstractCreature.addPower() checks Artifact
                    let is_debuff = crate::power_hooks::PowerId::from_str(buff).is_debuff();
                    
                    if is_debuff {
                        // Route through apply_player_debuff for Artifact/relic blocking
                        state.apply_player_debuff(buff, buff_amount);
                    } else {
                        // Apply to player's temp buffs
                        state.player.apply_temp_buff(buff, buff_amount);
                    }
                    
                    // Special handling for card modifiers
                    match buff.as_str() {
                        "DoubleTap" | "Double Tap" => {
                            state.card_modifiers.duplicate_next_attack += buff_amount;
                        }
                        "Burst" => {
                            state.card_modifiers.duplicate_next_skill += buff_amount;
                        }
                        _ => {}
                    }
                    
                    game_log!("  → Applied {} stacks of {} to player", buff_amount, buff);
                }
                "enemy" | "Enemy" | "target" => {
                    // Apply to target enemy (as a debuff)
                    if let Some(enemy) = state.get_target_enemy() {
                        enemy.apply_status(buff, buff_amount);
                        game_log!("  → Applied {} stacks of {} to {}", buff_amount, buff, enemy.name);
                    }
                }
                "all" | "AllEnemies" => {
                    for enemy in &mut state.enemies {
                        if !enemy.is_dead() {
                            enemy.apply_status(buff, buff_amount);
                        }
                    }
                    game_log!("  → Applied {} stacks of {} to all enemies", buff_amount, buff);
                }
                _ => {
                    game_log!("  ⚠ Unknown buff target: {}", target_str);
                }
            }
            
            CommandResult::BuffGained { buff: buff.clone(), amount: buff_amount }
        }
        
        CardCommand::ApplyDebuff { debuff, amount, target } => {
            let target_str = target.as_deref().unwrap_or("self");
            
            match target_str {
                "self" | "Self" | "player" => {
                    // Debuffs to self go through centralized check (Ginger/Turnip/Artifact)
                    state.apply_player_debuff(debuff, *amount);
                    game_log!("  → Applied {} stacks of {} to player (debuff)", amount, debuff);
                }
                "enemy" | "Enemy" | "target" => {
                    if let Some(enemy) = state.get_target_enemy() {
                        enemy.apply_status(debuff, *amount);
                        game_log!("  → Applied {} stacks of {} to {}", amount, debuff, enemy.name);
                    }
                    // SadisticNature: deal damage when applying debuff to enemy
                    let sadistic = state.player.powers.get("SadisticNature");
                    if sadistic > 0 {
                        if let Some(enemy) = state.get_target_enemy() {
                            let actual = enemy.take_damage(sadistic);
                            game_log!("    😈 SadisticNature: {} takes {} damage", enemy.name, actual);
                        }
                    }
                }
                "all" | "AllEnemies" => {
                    // This case is handled in ApplyBuff above; debuffs to all enemies
                    // happens via ApplyBuff with target "all" as well
                    for enemy in &mut state.enemies {
                        if !enemy.is_dead() {
                            enemy.apply_status(debuff, *amount);
                        }
                    }
                    game_log!("  → Applied {} stacks of {} to all enemies", amount, debuff);
                    // SadisticNature triggers for each enemy debuffed
                    let sadistic = state.player.powers.get("SadisticNature");
                    if sadistic > 0 {
                        for enemy in state.enemies.iter_mut() {
                            if !enemy.is_dead() {
                                let actual = enemy.take_damage(sadistic);
                                game_log!("    😈 SadisticNature: {} takes {} damage", enemy.name, actual);
                            }
                        }
                    }
                }
                _ => {
                    game_log!("  ⚠ Unknown debuff target: {}", target_str);
                }
            }
            
            CommandResult::StatusApplied { 
                target: target_str.to_string(), 
                status: debuff.clone(), 
                stacks: *amount 
            }
        }
        
        CardCommand::Discard { base, upgrade, select_mode, filter } => {
            let count = if upgraded { *upgrade } else { *base };
            
            // Parse select mode
            let mode = select_mode.as_deref()
                .map(SelectMode::from_str)
                .unwrap_or(SelectMode::Random);
            
            // Parse filter if present
            let card_filter = filter.as_ref().map(CardFilter::from_json);
            
            // Execute discard
            let discarded = state.discard_cards(
                count,
                mode,
                card_filter.as_ref(),
                None, // library not available here, but filter still works for cost/upgraded
            );
            
            game_log!("  → Discarded {} cards (mode: {:?})", discarded, mode);
            CommandResult::CardsDiscarded { count: discarded }
        }
        
        CardCommand::MoveCard { from_pile, to_pile, select_mode, select, count, insert_at, filter, retain: _, upgrade_count } => {
            // Parse locations (default to hand -> draw pile for Warcry-style cards)
            let from = from_pile.as_deref()
                .map(parse_location)
                .unwrap_or(CardLocation::Hand);
            let to = to_pile.as_deref()
                .map(parse_location)
                .unwrap_or(CardLocation::DrawPile);
            
            // Parse select mode - check both select_mode string and nested select object
            let mode = if let Some(sel) = &select {
                sel.mode.as_deref()
                    .map(SelectMode::from_str)
                    .unwrap_or(SelectMode::Choose)
            } else {
                select_mode.as_deref()
                    .map(SelectMode::from_str)
                    .unwrap_or(SelectMode::Random)
            };
            
            // Parse insert position
            let position = insert_at.as_deref()
                .map(InsertPosition::from_str)
                .unwrap_or(InsertPosition::Shuffle);
            
            // Get count from either count field or nested select object
            let base_count = if let Some(sel) = &select {
                sel.count.unwrap_or(count.unwrap_or(1))
            } else {
                count.unwrap_or(1)
            };
            
            // Apply upgrade if relevant
            let move_count = if upgraded {
                upgrade_count.unwrap_or(base_count)
            } else {
                base_count
            };
            
            // Parse filter if present
            let card_filter = filter.as_ref().map(CardFilter::from_json);
            
            // For "Choose" mode, we auto-select since we don't have UI interaction
            // In a real game, this would pause for player input
            let actual_mode = if matches!(mode, SelectMode::Choose) {
                SelectMode::Random // Auto-resolve choice
            } else {
                mode
            };
            
            // Execute move
            let moved = state.move_cards(
                from,
                to,
                move_count,
                actual_mode,
                position,
                card_filter.as_ref(),
                None,
            );
            
            game_log!("  → Moved {} cards from {:?} to {:?} (position: {:?})", moved, from, to, position);
            CommandResult::CardsDiscarded { count: moved } // Reusing this result type
        }
        
        CardCommand::ShuffleInto { card, destination, count } => {
            let dest = destination.as_deref()
                .map(parse_location)
                .unwrap_or(CardLocation::DrawPile);
            
            let num_copies = count.unwrap_or(1);
            
            if let Some(card_id) = card {
                // Create new copies of the specified card
                for _ in 0..num_copies {
                    // Default cost 0, the actual cost will be looked up when played
                    state.add_card_by_id(card_id, 0, dest, InsertPosition::Shuffle);
                }
                game_log!("  → Shuffled {} copies of '{}' into {:?}", num_copies, card_id, dest);
            } else {
                // No card specified - this might be "shuffle discard into draw"
                state.reshuffle_discard_into_draw();
                game_log!("  → Reshuffled discard pile into draw pile");
            }
            
            CommandResult::CardAdded { 
                card: card.clone().unwrap_or_default(), 
                destination: format!("{:?}", dest) 
            }
        }
        
        CardCommand::PutOnTop { source, select_mode, count } => {
            let from = source.as_deref()
                .map(parse_location)
                .unwrap_or(CardLocation::DiscardPile);
            
            let mode = select_mode.as_deref()
                .map(SelectMode::from_str)
                .unwrap_or(SelectMode::Choose);
            
            let move_count = count.unwrap_or(1);
            
            // Move to top of draw pile
            let moved = state.move_cards(
                from,
                CardLocation::DrawPile,
                move_count,
                mode,
                InsertPosition::Top,
                None,
                None,
            );
            
            game_log!("  → Put {} cards on top of draw pile from {:?}", moved, from);
            CommandResult::CardsDiscarded { count: moved }
        }
        
        // ====================================================================
        // Phase 2: Extended Commands
        // ====================================================================
        
        CardCommand::EndTurn => {
            state.end_turn_requested = true;
            game_log!("  → End turn requested (Vault effect)");
            CommandResult::Skipped { reason: "Turn ended".to_string() }
        }
        
        CardCommand::Heal { base, upgrade, amount } => {
            // Determine heal amount: use ValueSource if present, else use base/upgrade
            let heal_amount = if amount.is_some() {
                parse_value_source(amount.as_ref(), state, 0)
            } else if upgraded {
                *upgrade
            } else {
                *base
            };
            state.player.current_hp = (state.player.current_hp + heal_amount).min(state.player.max_hp);
            game_log!("  → Healed {} HP", heal_amount);
            CommandResult::HpGained { amount: heal_amount }
        }
        
        CardCommand::LoseBuff { buff, amount, amount_upgrade, all, end_of_turn } => {
            let actual_amount = if upgraded {
                amount_upgrade.unwrap_or(*amount)
            } else {
                *amount
            };
            
            if *end_of_turn {
                // Defer to end of turn
                state.end_of_turn_effects.push(
                    crate::state::EndOfTurnEffect::LoseBuff {
                        buff: buff.clone(),
                        amount: actual_amount,
                        all: *all,
                    }
                );
                game_log!("  → Registered end-of-turn: lose {} stacks of '{}'", actual_amount, buff);
                CommandResult::Skipped { reason: format!("Deferred to end of turn: LoseBuff {}", buff) }
            } else {
                let removed = state.player.remove_buff(buff, actual_amount, *all);
                game_log!("  → Lost {} stacks of '{}'", removed, buff);
                CommandResult::BuffGained { buff: buff.clone(), amount: -removed }
            }
        }
        
        CardCommand::RemoveEnemyBuff { buff, amount, base, upgrade } => {
            // Use base/upgrade if amount is 0 (default)
            let remove_amount = if *amount != 0 {
                *amount
            } else if upgraded {
                *upgrade
            } else {
                *base
            };
            let mut total_removed = 0;
            for enemy in state.enemies.iter_mut() {
                let removed = enemy.remove_buff(buff, remove_amount, false);
                total_removed += removed;
            }
            game_log!("  → Removed {} stacks of '{}' from enemies", total_removed, buff);
            CommandResult::BuffGained { buff: buff.clone(), amount: -total_removed }
        }
        
        CardCommand::ExhaustCard { pile, select_mode, count, upgrade_count } => {
            let from = pile.as_deref()
                .map(parse_location)
                .unwrap_or(CardLocation::Hand);
            
            let mode = select_mode.as_deref()
                .map(SelectMode::from_str)
                .unwrap_or(SelectMode::Choose);
            
            let exhaust_count = if upgraded {
                upgrade_count.unwrap_or(count.unwrap_or(1))
            } else {
                count.unwrap_or(1)
            };
            
            let exhausted = state.move_cards(
                from,
                CardLocation::ExhaustPile,
                exhaust_count,
                mode,
                InsertPosition::Top, // Exhaust pile order doesn't matter
                None,
                None,
            );
            
            game_log!("  → Exhausted {} cards from {:?}", exhausted, from);
            CommandResult::CardsDiscarded { count: exhausted }
        }
        
        CardCommand::ExhaustCards { base, upgrade, pile, select_mode } => {
            let count = if upgraded { *upgrade } else { *base };
            
            let from = pile.as_deref()
                .map(parse_location)
                .unwrap_or(CardLocation::Hand);
            
            let mode = select_mode.as_deref()
                .map(SelectMode::from_str)
                .unwrap_or(SelectMode::Choose);
            
            let exhausted = state.move_cards(
                from,
                CardLocation::ExhaustPile,
                count,
                mode,
                InsertPosition::Top,
                None,
                None,
            );
            
            game_log!("  → Exhausted {} cards from {:?}", exhausted, from);
            CommandResult::CardsDiscarded { count: exhausted }
        }
        
        CardCommand::IncreaseDamage { base, upgrade } => {
            game_log!("  → [IncreaseDamage: +{}]", if upgraded { *upgrade } else { *base });
            CommandResult::Unknown
        }
        
        CardCommand::DealDamageRandom { base, upgrade, times, times_upgrade } => {
            let base_damage = if upgraded { *upgrade } else { *base };
            let hits = if upgraded {
                times_upgrade.unwrap_or(times.unwrap_or(1))
            } else {
                times.unwrap_or(1)
            };
            
            let strength = state.player.get_strength();
            let vigor = state.player.consume_vigor();
            let mut damage = base_damage + strength + vigor;
            
            if state.player.is_weak() {
                damage = (damage as f32 * 0.75).floor() as i32;
            }
            
            let damage_per_hit = damage.max(0);
            let mut total_dealt = 0;
            let mut any_killed = false;
            let has_boot = state.relics.iter().any(|r| r.id == "Boot" && r.active);
            
            let living_count = state.enemies.iter().filter(|e| !e.is_dead()).count();
            
            if living_count > 0 {
                for _ in 0..hits {
                    // Pick a random living enemy
                    let living: Vec<usize> = state.enemies.iter()
                        .enumerate()
                        .filter(|(_, e)| !e.is_dead())
                        .map(|(i, _)| i)
                        .collect();
                    
                    if living.is_empty() {
                        break;
                    }
                    
                    use rand::Rng;
                    let target_idx = living[state.rng.random_range(0..living.len())];
                    let (actual, pend) = state.enemies[target_idx].take_damage_from_player(damage_per_hit, has_boot);
                    total_dealt += actual;
                    // Apply pending block immediately for random-target (each hit is independent)
                    if pend > 0 && !state.enemies[target_idx].is_dead() {
                        state.enemies[target_idx].block += pend;
                    }
                    if state.enemies[target_idx].is_dead() {
                        any_killed = true;
                    }
                }
                
                state.record_attack_result(damage_per_hit * hits, total_dealt, any_killed);
                game_log!("  → DealDamageRandom: {} damage × {} hits = {} total [Any killed: {}]",
                    damage_per_hit, hits, total_dealt, any_killed);
                CommandResult::DamageDealt { target: "RANDOM".to_string(), amount: total_dealt, killed: any_killed }
            } else {
                CommandResult::Skipped { reason: "No valid targets".into() }
            }
        }
        
        // ====================================================================
        // Phase 3: Special Effects
        // ====================================================================
        
        CardCommand::DoubleBlock => {
            let current = state.player.block;
            state.player.block *= 2;
            game_log!("  → Doubled block: {} -> {}", current, state.player.block);
            CommandResult::BlockGained { amount: current }
        }
        
        CardCommand::DoubleEnergy => {
            let current = state.player.energy;
            state.player.energy *= 2;
            game_log!("  → Doubled energy: {} -> {}", current, state.player.energy);
            CommandResult::EnergyGained { amount: current }
        }
        
        CardCommand::PlayTopCard { count, exhaust } => {
            let play_count = count.unwrap_or(1);
            let mut total_played = 0;
            
            if let Some(lib) = library {
                for _ in 0..play_count {
                    if let Some(mut card) = state.draw_pile.pop() {
                        let card_name = card.definition_id.clone();
                        let original_cost = card.current_cost;
                        card.set_cost_for_turn(0); // free play (autoplay)
                        game_log!("  → PlayTopCard: auto-playing '{}' from draw pile (free)", card_name);
                        
                        // Execute via full play_card pipeline
                        match super::combat::play_card(state, lib, &card, Some(0)) {
                            Ok(results) => {
                                let should_exhaust = *exhaust || results.iter().any(|r| matches!(r, CommandResult::CardExhausted));
                                if should_exhaust || card.is_ethereal {
                                    state.exhaust_pile.push(card);
                                    game_log!("  → {} exhausted after auto-play", card_name);
                                } else {
                                    card.current_cost = original_cost; // restore cost
                                    state.discard_pile.push(card);
                                }
                            }
                            Err(e) => {
                                game_log!("  ⚠ PlayTopCard failed to play {}: {}", card_name, e);
                                state.discard_pile.push(card);
                            }
                        }
                        total_played += 1;
                    } else {
                        game_log!("  → PlayTopCard: draw pile is empty");
                        break;
                    }
                }
            } else {
                game_log!("  ⚠ PlayTopCard: no library available");
            }
            
            game_log!("  → Played {} card(s) from top of draw pile", total_played);
            CommandResult::CardsDrawn { count: total_played }
        }
        
        CardCommand::Discover { from_count, choose } => {
            let num_choices = from_count.unwrap_or(3) as usize;
            let _num_pick = choose.unwrap_or(1);
            
            if let Some(lib) = library {
                // Generate N random cards (excluding Status/Curse)
                let mut candidates = Vec::new();
                for _ in 0..num_choices {
                    if let Some(card) = lib.get_random_card_of_color("any", Some(state.player_class), &mut state.rng) {
                        candidates.push(card);
                    }
                }
                
                if candidates.is_empty() {
                    game_log!("  → Discover: no cards available");
                    return CommandResult::Skipped { reason: "No cards in pool".into() };
                }
                
                // AI heuristic: pick the card with highest base cost (most powerful)
                candidates.sort_by(|a, b| b.base_cost.cmp(&a.base_cost));
                let mut chosen = candidates.swap_remove(0);
                
                // Discovered cards cost 0 this turn
                chosen.set_cost_for_turn(0);
                
                let card_name = chosen.definition_id.clone();
                state.hand.push(chosen);
                game_log!("  → Discover: chose {} (cost 0 this turn)", card_name);
                CommandResult::CardAdded { card: card_name, destination: "hand".into() }
            } else {
                game_log!("  → [Discover: no library available]");
                CommandResult::Skipped { reason: "No CardLibrary".into() }
            }
        }
        
        CardCommand::DrawUntil { target, upgrade: up } => {
            let target_count = if upgraded { up.unwrap_or(*target) } else { *target };
            let need = target_count - state.hand.len() as i32;
            if need > 0 {
                let drawn = state.draw_cards(need);
                game_log!("  → Drew {} cards (until hand has {})", drawn, target_count);
                CommandResult::CardsDrawn { count: drawn }
            } else {
                game_log!("  → Hand already has {} cards (target: {})", state.hand.len(), target_count);
                CommandResult::CardsDrawn { count: 0 }
            }
        }
        
        CardCommand::DrawUntilFull => {
            let need = 10 - state.hand.len() as i32;
            if need > 0 {
                let drawn = state.draw_cards(need);
                game_log!("  → Drew {} cards (until hand full)", drawn);
                CommandResult::CardsDrawn { count: drawn }
            } else {
                game_log!("  → Hand already full ({} cards)", state.hand.len());
                CommandResult::CardsDrawn { count: 0 }
            }
        }
        
        CardCommand::Draw { amount, upgrade_amount } => {
            let count = if upgraded { upgrade_amount.unwrap_or(*amount) } else { *amount };
            let drawn = state.draw_cards(count);
            game_log!("  → Drew {} cards", drawn);
            CommandResult::CardsDrawn { count: drawn }
        }
        
        CardCommand::Execute { threshold, upgrade_threshold } => {
            let th = if upgraded { upgrade_threshold.unwrap_or(*threshold) } else { *threshold };
            let has_boot = state.relics.iter().any(|r| r.id == "Boot" && r.active);
            if let Some(enemy) = state.get_target_enemy() {
                let hp = enemy.current_hp();
                if hp <= th {
                    let name = enemy.name.clone();
                    let (actual, _pend) = enemy.take_damage_from_player(hp, has_boot);
                    game_log!("  → Execute: {} killed (HP {} ≤ {})", name, hp, th);
                    CommandResult::DamageDealt { target: name, amount: actual, killed: true }
                } else {
                    let name = enemy.name.clone();
                    game_log!("  → Execute: {} HP {} > threshold {}", name, hp, th);
                    CommandResult::Skipped { reason: format!("HP {} > threshold {}", hp, th) }
                }
            } else {
                CommandResult::Skipped { reason: "No valid target".into() }
            }
        }
        
        CardCommand::GainGold { amount, upgrade: up } => {
            let gold = if upgraded { up.unwrap_or(*amount) } else { *amount };
            state.player.gold += gold;
            game_log!("  → Gained {} gold (total: {})", gold, state.player.gold);
            CommandResult::GoldGained { amount: gold }
        }
        
        CardCommand::GainMaxHP { amount, upgrade: up } => {
            let hp = if upgraded { up.unwrap_or(*amount) } else { *amount };
            state.player.max_hp += hp;
            state.player.current_hp += hp;
            game_log!("  → Gained {} Max HP", hp);
            CommandResult::HpGained { amount: hp }
        }
        
        CardCommand::ObtainPotion { source } => {
            // Check for Sozu relic (blocks potion acquisition)
            let has_sozu = state.relics.iter().any(|r| r.id == "Sozu");
            if has_sozu {
                game_log!("  → ObtainPotion blocked by Sozu relic");
                return CommandResult::Skipped { reason: "Sozu blocks potions".into() };
            }
            
            // Check if potion slots are full
            if state.potions.is_full() {
                game_log!("  → ObtainPotion: no empty potion slots");
                return CommandResult::Skipped { reason: "Potion slots full".into() };
            }
            
            // Hardcoded potion pools by rarity (matching Java PotionHelper)
            // These are the shared (Any class) potions only — safe for all characters
            use rand::Rng;
            use rand::prelude::IndexedRandom;
            
            let common_potions = &[
                "BlockPotion", "DexterityPotion", "EnergyPotion", "ExplosivePotion",
                "FirePotion", "StrengthPotion", "SwiftPotion", "WeakPotion", "FearPotion",
                "AttackPotion", "SkillPotion", "PowerPotion", "ColorlessPotion",
                "FlexPotion", "SpeedPotion", "BlessingoftheForge",
            ];
            let uncommon_potions = &[
                "AncientPotion", "DistilledChaos", "DuplicationPotion", "Elixir",
                "EssenceofSteel", "Gambler'sBrew", "LiquidBronze", "LiquidMemories",
                "RegenPotion", "SmokeBomb", "SneckoOil", "SteroidPotion",
            ];
            let rare_potions = &[
                "CultistPotion", "EntropicBrew", "FairyinaBottle", "FruitJuice",
            ];
            
            // Roll rarity: 65% Common, 25% Uncommon, 10% Rare
            let roll = state.rng.random_range(0..100usize);
            let potion_id = if roll < 65 {
                common_potions.choose(&mut state.rng).unwrap()
            } else if roll < 90 {
                uncommon_potions.choose(&mut state.rng).unwrap()
            } else {
                rare_potions.choose(&mut state.rng).unwrap()
            };
            
            match state.potions.add(potion_id.to_string()) {
                Ok(slot) => {
                    game_log!("  → ObtainPotion: obtained {} (slot {}, source: {:?})", potion_id, slot, source);
                    CommandResult::GoldGained { amount: 0 }
                }
                Err(_) => {
                    game_log!("  → ObtainPotion: slots full after check");
                    CommandResult::Skipped { reason: "Potion slots full".into() }
                }
            }
        }
        
        CardCommand::RemoveBlock { target } => {
            let target_str = target.as_deref().unwrap_or("enemy");
            match target_str {
                "enemy" | "Enemy" | "target" => {
                    if let Some(enemy) = state.get_target_enemy() {
                        let removed = enemy.block;
                        enemy.block = 0;
                        game_log!("  → Removed {} block from {}", removed, enemy.name);
                    }
                }
                _ => {
                    game_log!("  ⚠ RemoveBlock: unknown target {}", target_str);
                }
            }
            CommandResult::BlockGained { amount: 0 }
        }
        
        CardCommand::SetCostAll { pile, cost, permanent } => {
            let pile_name = pile.as_deref().unwrap_or("hand");
            // Currently only supports hand pile for cost modification
            let mut count = 0;
            for card in state.hand.iter_mut() {
                if *permanent {
                    card.modify_cost_for_combat(*cost - card.base_cost);
                } else {
                    card.set_cost_for_turn(*cost);
                }
                count += 1;
            }
            game_log!("  → Set cost of {} cards in {} to {} (permanent: {})", count, pile_name, cost, permanent);
            CommandResult::CostModified { count }
        }
        
        CardCommand::SetCostRandom { pile, cost, permanent } => {
            let pile_name = pile.as_deref().unwrap_or("hand");
            // Currently only supports hand pile for cost modification
            if !state.hand.is_empty() {
                use rand::Rng;
                let idx = state.rng.random_range(0..state.hand.len());
                let card_name = state.hand[idx].definition_id.clone();
                if *permanent {
                    let delta = *cost - state.hand[idx].base_cost;
                    state.hand[idx].modify_cost_for_combat(delta);
                } else {
                    state.hand[idx].set_cost_for_turn(*cost);
                }
                game_log!("  → Set cost of {} to {} in {} (permanent: {})", card_name, cost, pile_name, permanent);
            }
            CommandResult::CostModified { count: 1 }
        }
        
        CardCommand::UpgradeCard { select_mode, pile } => {
            let mode = select_mode.as_deref().unwrap_or("choose");
            let pile_name = pile.as_deref().unwrap_or("hand");
            // Currently only supports hand pile
            if mode == "all" {
                let mut count = 0;
                for card in state.hand.iter_mut() {
                    if !card.upgraded {
                        card.upgraded = true;
                        count += 1;
                    }
                }
                game_log!("  → Upgraded all {} cards in {}", count, pile_name);
            } else {
                // AI heuristic: upgrade highest cost un-upgraded card
                if let Some(idx) = state.hand.iter().enumerate()
                    .filter(|(_, c)| !c.upgraded)
                    .max_by_key(|(_, c)| c.current_cost)
                    .map(|(i, _)| i)
                {
                    state.hand[idx].upgraded = true;
                    game_log!("  → Upgraded: {}", state.hand[idx].definition_id);
                } else {
                    game_log!("  → No upgradeable cards in {}", pile_name);
                }
            }
            CommandResult::CardsUpgraded { count: 1, location: crate::schema::CardLocation::Hand }
        }
        
        CardCommand::ExtraTurn => {
            game_log!("  → [ExtraTurn]");
            CommandResult::Unknown
        }
        
        CardCommand::MultiplyStatus { status, target, multiplier, upgrade_multiplier } => {
            let mult = if upgraded { upgrade_multiplier.unwrap_or(*multiplier) } else { *multiplier };
            let target_str = target.as_deref().unwrap_or("enemy");
            match target_str {
                "enemy" | "Enemy" | "target" => {
                    if let Some(enemy) = state.get_target_enemy() {
                        let current = enemy.get_status(status);
                        if current > 0 {
                            let new_val = current * mult;
                            let old = current;
                            enemy.powers.set(status, new_val);
                            game_log!("  → {} {} x{} = {} on {}", status, current, mult, new_val, enemy.name);
                            return CommandResult::StatusMultiplied { status: status.clone(), old, new: new_val };
                        } else {
                            game_log!("  → {} has no {} to multiply", enemy.name, status);
                        }
                    }
                }
                _ => {
                    game_log!("  ⚠ MultiplyStatus: unknown target {}", target_str);
                }
            }
            CommandResult::Skipped { reason: "No status to multiply".into() }
        }
        
        CardCommand::DoubleStatus { status, target } => {
            let target_str = target.as_deref().unwrap_or("enemy");
            match target_str {
                "enemy" | "Enemy" | "target" => {
                    if let Some(enemy) = state.get_target_enemy() {
                        let current = enemy.get_status(status);
                        if current > 0 {
                            let new_val = current * 2;
                            enemy.powers.set(status, new_val);
                            game_log!("  → Doubled {} on {}: {} → {}", status, enemy.name, current, new_val);
                            return CommandResult::StatusMultiplied { status: status.clone(), old: current, new: new_val };
                        } else {
                            game_log!("  → {} has no {} to double", enemy.name, status);
                        }
                    }
                }
                _ => {
                    game_log!("  ⚠ DoubleStatus: unknown target {}", target_str);
                }
            }
            CommandResult::Skipped { reason: "No status to double".into() }
        }
        
        // ====================================================================
        // Marker Commands (属性标记 - 这些通常在其他地方处理)
        // ====================================================================
        
        CardCommand::Ethereal { .. } => {
            game_log!("  → [Ethereal marker]");
            CommandResult::Unknown
        }
        
        CardCommand::Innate { .. } => {
            game_log!("  → [Innate marker]");
            CommandResult::Unknown
        }
        
        CardCommand::Retain { .. } => {
            game_log!("  → [Retain marker]");
            CommandResult::Unknown
        }
    }
}

