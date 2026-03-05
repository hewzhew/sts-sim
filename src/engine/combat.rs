//! Combat flow — card play, enemy turns, turn events, triggers.
//!
//! Corresponds to Java's AbstractRoom (endTurn, endBattle) and
//! GameActionManager (card queue, getNextAction).

use crate::loader::{CardLibrary, MonsterLibrary};
use crate::schema::{CardCommand, CardInstance, CardType};
use crate::state::{GameState, GamePhase, InsertPosition};
use crate::items::relics::{GameEvent, trigger_relics, apply_relic_results, RelicLibrary};
use crate::enemy::Intent;
use rand::Rng;  // for random_range on Xoshiro256StarStar
use super::{CommandResult, parse_location};
use super::commands::{apply_command, apply_hook_effects, calculate_card_damage, channel_orb};
/// Play a card from the library.
///
/// This is the main entry point for playing cards. It:
/// 1. Looks up the card definition
/// 2. Checks energy cost
/// 3. Checks for card modifiers (DoubleTap, Burst)
/// 4. Executes all commands in sequence
/// 5. Handles exhaustion if needed
pub fn play_card(
    state: &mut GameState,
    library: &CardLibrary,
    card_instance: &CardInstance,
    target_idx: Option<usize>,
) -> Result<Vec<CommandResult>, String> {
    let definition = library
        .get(&card_instance.definition_id)
        .map_err(|e| e.to_string())?;
    
    let upgraded = card_instance.upgraded;
    let card_name = if upgraded {
        format!("{}+", definition.name)
    } else {
        definition.name.clone()
    };
    
    game_log!("\n▶ Playing: {} (cost: {})", card_name, card_instance.current_cost);
    
    // Track which card is being played (for AddCard "this card" and WristBlade relic)
    state.last_played_card_id = Some(card_instance.definition_id.clone());
    state.last_played_card_cost = card_instance.current_cost;
    // Set the target enemy index so get_target_enemy() returns the correct enemy
    state.target_enemy_idx = target_idx;
    // Handle X-cost cards (cost == -1, e.g., Whirlwind, Malaise)
    // Java: WhirlwindAction reads EnergyPanel.totalCount, Chemical X adds +2
    let is_x_cost = card_instance.current_cost == -1;
    if is_x_cost {
        // X = current energy (before spending)
        let mut x_value = state.player.energy;
        // Chemical X relic: +2 to X effects
        if state.relics.iter().any(|r| r.id == "Chemical X") {
            x_value += 2;
            game_log!("  ⚗️ Chemical X: X += 2 (now {})", x_value);
        }
        state.x_cost_value = x_value;
        // Consume ALL energy (Java: this.p.energy.use(EnergyPanel.totalCount))
        state.player.energy = 0;
        game_log!("  ⚡ X-cost card: X = {} (all energy consumed)", x_value);
    } else {
        // Normal card: check energy and spend
        if state.player.energy < card_instance.current_cost {
            return Err(format!(
                "Not enough energy: have {}, need {}",
                state.player.energy, card_instance.current_cost
            ));
        }
        state.player.energy -= card_instance.current_cost;
        state.x_cost_value = 0;
    }
    state.cards_played_this_turn += 1;
    
    // OrangePellets: track card types this turn, remove debuffs when all 3 played
    // Java: onUseCard sets ATTACK/SKILL/POWER bools → RemoveDebuffsAction
    // Counter bitmask: bit 0 = Attack, bit 1 = Skill, bit 2 = Power
    if let Some(pellets) = state.relics.iter_mut().find(|r| r.id == "OrangePellets") {
        match definition.card_type {
            CardType::Attack => pellets.counter |= 1,
            CardType::Skill => pellets.counter |= 2,
            CardType::Power => pellets.counter |= 4,
            _ => {}
        }
        if pellets.counter == 7 { // All three types played
            pellets.counter = 0;
            game_log!("  🟠 OrangePellets: All 3 card types played — removing debuffs!");
            // Remove Weak, Vulnerable, Frail, and any other debuffs
            for debuff in &["Weak", "Vulnerable", "Frail", "Entangled", "NoDraw", "NoBlock"] {
                state.player.powers.remove(debuff);
            }
        }
    }
    
    // MummifiedHand: when playing a Power, reduce a random card in hand to 0 cost this turn
    // Java: onUseCard → if Power → find random card with cost > 0 → setCostForTurn(0)
    if definition.card_type == CardType::Power 
        && state.relics.iter().any(|r| r.id == "Mummified Hand" || r.id == "MummifiedHand") 
    {
        // Find cards in hand with cost > 0
        let eligible: Vec<usize> = state.hand.iter().enumerate()
            .filter(|(_, c)| c.current_cost > 0)
            .map(|(i, _)| i)
            .collect();
        if !eligible.is_empty() {
            use rand::Rng;
            let idx = state.rng.random_range(0..eligible.len());
            let card_idx = eligible[idx];
            let card_name = state.hand[card_idx].definition_id.clone();
            state.hand[card_idx].current_cost = 0;
            game_log!("  🤚 MummifiedHand: {} cost → 0 this turn", card_name);
        }
    }
    
    // Fire on_use_card power hooks (Rage, AfterImage, Corruption, etc.)
    let card_type_str = match definition.card_type {
        CardType::Attack => "Attack",
        CardType::Skill => "Skill",
        CardType::Power => "Power",
        CardType::Status => "Status",
        CardType::Curse => "Curse",
    };
    let mut hook_exhaust = false;
    {
        let power_snapshot: Vec<(String, i32)> = state.player.powers.iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        for (power_id, stacks) in &power_snapshot {
            let pi = crate::power_hooks::PowerInstance::new(
                crate::power_hooks::PowerId::from_str(power_id), *stacks
            );
            let effects = pi.on_use_card(card_type_str);
            if !effects.is_empty() {
                // Check for ExhaustPlayed (Corruption exhausts Skills)
                if effects.iter().any(|e| matches!(e, crate::power_hooks::HookEffect::ExhaustPlayed)) {
                    hook_exhaust = true;
                }
                apply_hook_effects(state, &effects, power_id, None, Some(library));
            }
        }
    }
    
    // Fire on_use_card for ENEMY powers (SharpHide, BeatOfDeath, Curiosity, etc.)
    {
        let enemy_count = state.enemies.len();
        for enemy_idx in 0..enemy_count {
            if state.enemies[enemy_idx].is_dead() { continue; }
            let power_snapshot: Vec<(String, i32)> = state.enemies[enemy_idx].powers.iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect();
            for (power_id, stacks) in &power_snapshot {
                let pi = crate::power_hooks::PowerInstance::new(
                    crate::power_hooks::PowerId::from_str(power_id), *stacks
                );
                let effects = pi.on_use_card(card_type_str);
                if !effects.is_empty() {
                    apply_hook_effects(state, &effects, power_id, Some(enemy_idx), Some(library));
                }
            }
        }
    }

    
    // Check if this card should be played twice (DoubleTap for Attacks, Burst for Skills)
    let should_double = match definition.card_type {
        CardType::Attack => {
            if state.card_modifiers.duplicate_next_attack > 0 {
                state.card_modifiers.duplicate_next_attack -= 1;
                true
            } else {
                false
            }
        }
        CardType::Skill => {
            if state.card_modifiers.duplicate_next_skill > 0 {
                state.card_modifiers.duplicate_next_skill -= 1;
                true
            } else {
                false
            }
        }
        _ => false,
    };
    
    // Necronomicon: first Attack costing ≥2 each turn plays twice
    // Java: onUseCard — card.type == ATTACK && (costForTurn >= 2 && !freeToPlayOnce) && activated
    //        atTurnStart — activated = true
    let necronomicon_replay = if definition.card_type == CardType::Attack
        && card_instance.current_cost >= 2
    {
        // Find Necronomicon relic and check if it hasn't fired this turn (counter == 0)
        let necro_idx = state.relics.iter().position(|r| r.id == "Necronomicon");
        if let Some(idx) = necro_idx {
            if state.relics[idx].counter == 0 {
                state.relics[idx].counter = 1; // Mark as fired this turn
                game_log!("  📖 Necronomicon: Attack cost ≥2 — will play again!");
                true
            } else {
                false
            }
        } else {
            false
        }
    } else {
        false
    };
    
    // Pre-process: scan for StrengthMultiplier commands and set the multiplier
    // This allows DealDamage to use the correct strength multiplier even though
    // the StrengthMultiplier command appears after DealDamage in the JSON
    for parsed_command in &definition.logic.commands {
        if let Some(CardCommand::StrengthMultiplier { base, upgrade: upg }) = parsed_command.as_known() {
            state.card_modifiers.strength_multiplier = if upgraded { *upg } else { *base };
        }
    }
    
    // Execute commands.
    let mut results = Vec::new();
    let mut should_exhaust = hook_exhaust;
    
    // Check for card overrides BEFORE running JSON commands.
    // Some cards (PerfectedStrike, SpotWeakness, Feed, Dropkick) have complex
    // logic that can't be expressed in JSON and need Rust overrides.
    if let Some(override_results) = super::card_overrides::try_override(
        state, &card_instance.definition_id, upgraded, target_idx
    ) {
        // Override handled the card — check for exhaust in override results
        for result in &override_results {
            if matches!(result, CommandResult::CardExhausted) {
                should_exhaust = true;
            }
        }
        results = override_results;
        
        // Handle exhaustion
        if should_exhaust {
            game_log!("  ✗ Card exhausted");
        }
        
        // Reset x_cost_value after card execution
        state.x_cost_value = 0;
        
        // Must also fire relic onUseCard triggers for override cards
        let relic_result = trigger_relics(state, &GameEvent::PlayerPlayCard {
            card_type: definition.card_type,
            cost: card_instance.current_cost,
            card_id: card_instance.definition_id.clone(),
        }, None);
        apply_relic_results(state, &relic_result);
        
        return Ok(results);
    }
    
    for parsed_command in &definition.logic.commands {
        match parsed_command.as_known() {
            Some(command) => {
                let result = apply_command(state, command, upgraded, target_idx, Some(library));
                
                if matches!(result, CommandResult::CardExhausted) {
                    should_exhaust = true;
                }
                
                results.push(result);
            }
            None => {
                if let Some(raw) = parsed_command.as_raw() {
                    panic!(
                        "FAIL-FAST: Unimplemented card command!\n\
                         Card: {} ({})\n\
                         Unknown command type: '{}'\n\
                         Raw params: {:?}\n\
                         \n\
                         To fix: Add parsing support for '{}' in CardCommand enum,\n\
                         or update the card's logic in cards_patched.json.",
                        definition.name,
                        card_instance.definition_id,
                        raw.command_type,
                        raw.params,
                        raw.command_type
                    );
                } else {
                    panic!(
                        "FAIL-FAST: Unimplemented card command for {} ({}) - no raw data available",
                        definition.name,
                        card_instance.definition_id
                    );
                }
            }
        }
    }
    
    // Second play if doubled (DoubleTap/Burst) or Necronomicon
    if should_double || necronomicon_replay {
        let source = if necronomicon_replay && !should_double { "Necronomicon" } else { "DoubleTap/Burst" };
        game_log!("  ⚡ Card played again ({})!", source);
        for parsed_command in &definition.logic.commands {
            if let Some(command) = parsed_command.as_known() {
                // Don't exhaust twice
                if !matches!(command, CardCommand::ExhaustSelf { .. }) {
                    let result = apply_command(state, command, upgraded, target_idx, Some(library));
                    results.push(result);
                }
            }
        }
    }
    
    // Handle exhaustion
    if should_exhaust {
        game_log!("  ✗ Card exhausted");
    }
    
    // Trigger relics for card play event — AFTER card resolution
    // Java: UseCardAction.update() calls relic.onUseCard() AFTER card.use()
    // and the queued actions resolve. So Shuriken/Kunai/PenNib/Nunchaku
    // effects apply to the NEXT card, not the current one.
    let relic_result = trigger_relics(state, &GameEvent::PlayerPlayCard {
        card_type: definition.card_type,
        cost: card_instance.current_cost,
        card_id: card_instance.definition_id.clone(),
    }, None);
    apply_relic_results(state, &relic_result);
    
    // === P1.8: onAfterUseCard power hook ===
    // Java: UseCardAction.update() → p.onAfterUseCard(card, action) for player + enemy powers
    // Key effects: TimeWarp counter check (end turn at 12 cards)
    {
        let power_snapshot: Vec<(String, i32)> = state.player.powers.iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        for (power_id, stacks) in &power_snapshot {
            match crate::power_hooks::PowerId::from_str(power_id) {
                crate::power_hooks::PowerId::TimeWarp => {
                    // TimeWarp: increment counter on each card played, at 12 → end turn + gain STR
                    let current = state.player.powers.get("TimeWarp");
                    if current >= 12 {
                        state.player.powers.remove("TimeWarp");
                        // Gain 2 Strength for the Time Eater
                        for enemy in state.enemies.iter_mut() {
                            if !enemy.is_dead() {
                                enemy.apply_status("Strength", 2);
                            }
                        }
                        // End player turn (Java: ChangeStateAction → EndTurnAction)
                        state.player.energy = 0;
                        game_log!("  ⏰ Time Warp: 12 cards played — turn ending! Enemy +2 STR");
                    }
                }
                _ => {}
            }
        }
    }
    
    // === P1.9: Panache damage trigger ===
    // Java: PanachePower.onAfterUseCard → decrement counter, at 0 deal 10 damage to all
    // on_use_card already does AddStacks(-1), check if counter reached 0
    {
        let panache_stacks = state.player.powers.get("Panache");
        if panache_stacks == 0 && state.player.powers.has("Panache") {
            // Reset to 5 and deal 10 damage to all enemies
            state.player.powers.set("Panache", 5);
            for enemy in state.enemies.iter_mut() {
                if !enemy.is_dead() {
                    let dmg = enemy.take_damage(10);
                    game_log!("  💃 Panache: {} takes {} damage (5th card!)", enemy.name, dmg);
                }
            }
        }
    }
    
    Ok(results)
}

/// Execute a full card play including moving card from hand to discard/exhaust.
pub fn play_card_from_hand(
    state: &mut GameState,
    library: &CardLibrary,
    hand_index: usize,
    target_idx: Option<usize>,
) -> Result<Vec<CommandResult>, String> {
    if hand_index >= state.hand.len() {
        return Err(format!("Invalid hand index: {}", hand_index));
    }
    
    // IMPORTANT: Remove card from hand FIRST, before executing effects.
    // Card effects like ExhaustCards can modify the hand, which would invalidate
    // our hand_index if we tried to remove after executing.
    let card_instance = state.hand.remove(hand_index);
    
    // Play the card (using the removed instance)
    let results = play_card(state, library, &card_instance, target_idx)?;
    
    // === triggerOnOtherCardPlayed ===
    // Java: After card.use(), iterate all cards in hand and call
    // triggerOnOtherCardPlayed(). Curse cards like Pain deal damage here.
    // Pain.java: triggerOnOtherCardPlayed → LoseHPAction(owner, magicNumber=1)
    {
        let mut pain_count = 0;
        for card in &state.hand {
            match card.definition_id.as_str() {
                "Pain" => pain_count += 1,
                _ => {}
            }
        }
        if pain_count > 0 {
            // Java: LoseHPAction — bypasses block, direct HP loss
            let hp_loss = pain_count;
            state.player.current_hp = (state.player.current_hp - hp_loss).max(0);
            game_log!("  💀 Pain: {} card(s) in hand → lose {} HP (HP: {})",
                pain_count, hp_loss, state.player.current_hp);
        }
    }

    let should_exhaust = results.iter().any(|r| matches!(r, CommandResult::CardExhausted));
    
    if should_exhaust {
        // Strange Spoon: 50% chance to not exhaust (card goes to discard instead)
        // Java: UseCardAction — if StrangeSpoon and cardRng.randomBoolean() → discard instead
        let spoon_saves = state.relics.iter().any(|r| r.id == "StrangeSpoon" && r.active)
            && state.rng.random_range(0..2u32) == 0;  // 50% chance
        
        if spoon_saves {
            game_log!("  🥄 Strange Spoon: Card saved from exhaust → discard!");
            state.discard_pile.push(card_instance);
        } else {
            state.exhaust_pile.push(card_instance);
            
            // Fire on_exhaust power hooks (FeelNoPain, DarkEmbrace, etc.)
            let power_snapshot: Vec<(String, i32)> = state.player.powers.iter()
                .map(|(k, v)| (k.clone(), *v))
                .collect();
            for (power_id, stacks) in &power_snapshot {
                let pi = crate::power_hooks::PowerInstance::new(
                    crate::power_hooks::PowerId::from_str(power_id), *stacks
                );
                let effects = pi.on_exhaust();
                if !effects.is_empty() {
                    apply_hook_effects(state, &effects, &power_id, None, Some(library));
                }
            }
            
            // Dead Branch: add a truly random card to hand when a card is exhausted
            // Java: onExhaust → MakeTempCardInHandAction(returnTrulyRandomCardInCombat().makeCopy())
            if state.relics.iter().any(|r| r.id == "Dead Branch") {
                if !state.enemies.iter().all(|e| e.is_dead()) {
                    if let Some(random_card) = library.get_random_card_of_color(
                        "any",
                        Some(crate::core::schema::CardColor::Red),
                        &mut state.rng,
                    ) {
                        game_log!("  🌿 Dead Branch: Exhausted card → added {} to hand", random_card.definition_id);
                        if state.hand.len() < 10 {
                            state.hand.push(random_card);
                        } else {
                            state.discard_pile.push(random_card);
                        }
                    }
                }
            }
        }
    } else {
        state.discard_pile.push(card_instance);
    }
    
    // UnceasingTop: If hand is empty after playing a card, draw 1 card.
    // Java: UnceasingTop.onRefreshHand() → if hand empty && cards available → DrawCardAction(1)
    if state.hand.is_empty() {
        if state.relics.iter().any(|r| (r.id == "UnceasingTop" || r.id == "Unceasing Top") && r.active) {
            if !state.draw_pile.is_empty() || !state.discard_pile.is_empty() {
                let drawn = state.draw_cards(1);
                if drawn > 0 {
                    game_log!("  🔄 Unceasing Top: Hand empty → drew 1 card");
                }
            }
        }
    }
    
    Ok(results)
}

/// Simulate a simple combat turn for testing.
pub fn simulate_turn(
    state: &mut GameState,
    library: &CardLibrary,
) {
    state.start_turn();
    game_log!("\n{}", state);
    
    // Play cards until out of energy or hand
    while state.player.energy > 0 && !state.hand.is_empty() {
        // Find a playable card (must have enough energy AND meet any special play conditions)
        let playable_idx = state.hand.iter().position(|c| {
            c.current_cost <= state.player.energy
                && crate::engine::card_overrides::card_can_play(&c.definition_id, state)
        });
        
        match playable_idx {
            Some(idx) => {
                if let Err(e) = play_card_from_hand(state, library, idx, Some(0)) {
                    game_log!("Error playing card: {}", e);
                    break;
                }
            }
            None => {
                game_log!("No playable cards remaining");
                break;
            }
        }
    }
    
    state.end_turn();
}

// ============================================================================
// Enemy Turn Execution
// ============================================================================



/// Calculate enemy attack damage with Weak/Vulnerable multipliers.
///
/// Java: `AbstractMonster.calculateDamage()` applies:
/// 1. Weak on attacker (enemy): damage * 0.75 (or 0.6 with PaperCrane)
/// 2. Vulnerable on defender (player): damage * 1.5 (or 1.25 with OddMushroom)
///
/// The intent damage already includes base + Strength from `resolve_intent`.
/// This function applies the missing Weak/Vuln multipliers.
fn calculate_enemy_attack_damage(
    base_damage: i32,
    enemy_powers: &crate::powers::PowerSet,
    player_powers: &crate::powers::PowerSet,
    relics: &[crate::items::relics::RelicInstance],
) -> i32 {
    let mut damage = base_damage as f64;
    
    // Step 1: Weak on enemy (reduces damage dealt)
    // Java: WeakPower.atDamageGive() → damage * 0.75 (for non-player, i.e. monsters)
    if enemy_powers.has("Weak") {
        let has_paper_crane = relics.iter().any(|r| 
            r.id == "PaperCrane" && r.active
        );
        if has_paper_crane {
            damage *= 0.6; // PaperCrane: Weak reduces by 40% instead of 25%
        } else {
            damage *= 0.75; // Standard Weak: 25% less damage
        }
    }
    
    // Step 2: Vulnerable on player (increases damage received)
    // Java: VulnerablePower.atDamageReceive() → damage * 1.5
    if player_powers.has("Vulnerable") {
        let has_odd_mushroom = relics.iter().any(|r| 
            r.id == "OddMushroom" && r.active
        );
        if has_odd_mushroom {
            damage *= 1.25; // OddMushroom: Vuln increases by 25% instead of 50%
        } else {
            damage *= 1.5; // Standard Vulnerable: 50% more damage
        }
    }
    
    // Java floors the result (MathUtils.floor in AbstractMonster.calculateDamage)
    (damage as i32).max(0)
}

/// Execute all enemy turns and plan their next moves.
/// 
/// # Arguments
/// * `state` - The game state
/// * `monster_library` - Library of monster definitions (for resolving intents)
pub fn execute_enemy_turn(state: &mut GameState, monster_library: &MonsterLibrary) {
    game_log!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    game_log!("🦑 ENEMY TURN");
    game_log!("━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
    
    // Clear enemy block at start of their turn
    for enemy in state.enemies.iter_mut() {
        enemy.clear_block();
    }
    
    // Collect intents first to avoid borrow issues
    let intents: Vec<(usize, Intent, String)> = state.enemies
        .iter()
        .enumerate()
        .filter(|(_, e)| !e.is_dead())
        .map(|(i, e)| (i, e.current_intent.clone(), e.name.clone()))
        .collect();
    
    // Execute each enemy's intent
    for (enemy_idx, intent, enemy_name) in intents {
        game_log!("\n🎭 {} executes:", enemy_name);
        
        // ExplosivePower.java: duringTurn()
        // Java: if amount == 1 → SuicideAction + DamageAction(player, 30, THORNS)
        //       else → ReducePowerAction(1)
        let explosive_stacks = state.enemies[enemy_idx].powers.get("Explosive");
        if explosive_stacks > 0 {
            if explosive_stacks == 1 {
                // EXPLODE: suicide + deal 30 THORNS damage to player
                game_log!("  💥 {} EXPLODES! Deals 30 damage to player!", enemy_name);
                state.enemies[enemy_idx].hp = 0;
                let actual = state.player_take_damage(30);
                game_log!("     Player takes {} damage from explosion (HP: {})", 
                    actual, state.player.current_hp);
                if actual > 0 {
                    let hp_loss_result = trigger_relics(state, &GameEvent::PlayerLoseHp { amount: actual }, None);
                    apply_relic_results(state, &hp_loss_result);
                    if hp_loss_result.extra_draw > 0 {
                        state.draw_cards(hp_loss_result.extra_draw);
                    }
                }
                continue; // Skip normal intent — enemy is dead
            } else {
                // Countdown: reduce by 1
                state.enemies[enemy_idx].powers.apply("Explosive", -1, None);
                game_log!("  💣 {} Explosive countdown: {} → {}", 
                    enemy_name, explosive_stacks, explosive_stacks - 1);
            }
        }
        
        // Snapshot enemy powers for damage calculation (avoids borrow conflicts)
        let enemy_powers_snapshot = state.enemies[enemy_idx].powers.clone();
        
        match intent {
            Intent::Attack { damage, times } => {
                // Apply Weak/Vulnerable multipliers (the critical fix)
                let modified_base = calculate_enemy_attack_damage(
                    damage, &enemy_powers_snapshot, &state.player.powers, &state.relics
                );
                let total_damage = modified_base * times;
                game_log!("  → Attacks for {} damage ({}x{}){}", 
                    total_damage, modified_base, times,
                    if modified_base != damage { format!(" [base {}, weak/vuln adjusted]", damage) } else { String::new() }
                );
                
                for _ in 0..times {
                    // Apply onAttackedToChangeDamage hooks (Buffer, Invincible)
                    let modified_damage = crate::power_hooks::apply_on_attacked_to_change_damage(
                        modified_base, &state.player.powers
                    );
                    let actual = state.player_take_damage(modified_damage);
                    game_log!("     Player takes {} damage (HP: {})", 
                        actual, state.player.current_hp);
                    
                    // Fire on_attacked hooks (Thorns, FlameBarrier, etc.)
                    if actual > 0 {
                        // Fire PlayerLoseHp relic event (Runic Cube, Red Skull, etc.)
                        let hp_loss_result = trigger_relics(state, &GameEvent::PlayerLoseHp { amount: actual }, None);
                        apply_relic_results(state, &hp_loss_result);
                        if hp_loss_result.extra_draw > 0 {
                            state.draw_cards(hp_loss_result.extra_draw);
                        }
                        
                        // Collect effects per power to track source
                        let power_snapshot: Vec<(String, i32)> = state.player.powers.iter()
                            .map(|(k, v)| (k.clone(), *v))
                            .collect();
                        for (power_id, stacks) in &power_snapshot {
                            let pi = crate::power_hooks::PowerInstance::new(
                                crate::power_hooks::PowerId::from_str(power_id), *stacks
                            );
                            let (_, effects) = pi.on_attacked(actual);
                            if !effects.is_empty() {
                                apply_hook_effects(state, &effects, power_id, Some(enemy_idx), None);
                            }
                        }
                    }
                }
            }
            
            Intent::AttackAll { damage } => {
                let modified_base = calculate_enemy_attack_damage(
                    damage, &enemy_powers_snapshot, &state.player.powers, &state.relics
                );
                game_log!("  → Attack ALL for {} damage{}", modified_base,
                    if modified_base != damage { format!(" [base {}, weak/vuln adjusted]", damage) } else { String::new() }
                );
                let modified_damage = crate::power_hooks::apply_on_attacked_to_change_damage(
                    modified_base, &state.player.powers
                );
                let actual = state.player_take_damage(modified_damage);
                game_log!("     Player takes {} damage (HP: {})", actual, state.player.current_hp);
                
                // Fire on_attacked hooks
                if actual > 0 {
                    // Fire PlayerLoseHp relic event
                    let hp_loss_result = trigger_relics(state, &GameEvent::PlayerLoseHp { amount: actual }, None);
                    apply_relic_results(state, &hp_loss_result);
                    if hp_loss_result.extra_draw > 0 {
                        state.draw_cards(hp_loss_result.extra_draw);
                    }
                    
                    let power_snapshot: Vec<(String, i32)> = state.player.powers.iter()
                        .map(|(k, v)| (k.clone(), *v))
                        .collect();
                    for (power_id, stacks) in &power_snapshot {
                        let pi = crate::power_hooks::PowerInstance::new(
                            crate::power_hooks::PowerId::from_str(power_id), *stacks
                        );
                        let (_, effects) = pi.on_attacked(actual);
                        if !effects.is_empty() {
                            apply_hook_effects(state, &effects, power_id, Some(enemy_idx), None);
                        }
                    }
                }
            }
            
            Intent::Defend { block } => {
                game_log!("  → Gains {} block", block);
                if let Some(enemy) = state.enemies.get_mut(enemy_idx) {
                    enemy.gain_block(block);
                    game_log!("     {} now has {} block", enemy_name, enemy.block);
                }
            }
            
            Intent::Buff { ref name, amount } => {
                game_log!("  → Gains {} {} ({} stacks)", name, amount, amount);
                if let Some(enemy) = state.enemies.get_mut(enemy_idx) {
                    // Handle special buffs
                    match name.as_str() {
                        "Strength" => {
                            // Direct strength gain
                            enemy.add_buff("Strength", amount);
                            game_log!("     {} now has {} Strength", enemy_name, enemy.strength());
                        }
                        "Ritual" => {
                            // Ritual: At the end of its turn, gains X Strength.
                            // (doesn't trigger on the turn it was gained)
                            enemy.apply_status("Ritual", amount);
                            game_log!("     {} now has {} Ritual (gains {} Strength at end of turn)", 
                                enemy_name, enemy.get_status("Ritual"), enemy.get_status("Ritual"));
                        }
                        _ => {
                            enemy.apply_status(name, amount);
                            game_log!("     {} now has {} {}", enemy_name, 
                                enemy.get_status(name), name);
                        }
                    }
                }
            }
            
            Intent::Debuff { ref name, amount } => {
                game_log!("  → Applies {} {} to player", amount, name);
                state.apply_player_debuff(name, amount);
                game_log!("     Player now has {} {}", 
                    state.player.get_status(name), name);
            }
            
            Intent::AddCard { ref card, amount, ref destination } => {
                let dest = parse_location(destination);
                // Status cards are unplayable: use cost -2 (same as JSON definition)
                let cost = match card.as_str() {
                    "Dazed" | "Burn" | "Wound" | "Slimed" | "Void" => -2,
                    _ => 0,
                };
                for _ in 0..amount {
                    state.add_card_by_id(card, cost, dest, InsertPosition::Shuffle);
                }
                game_log!("  → Shuffled {}x {} into {:?}", amount, card, dest);
            }
            
            Intent::AttackDebuff { damage, ref debuff, amount } => {
                let modified_base = calculate_enemy_attack_damage(
                    damage, &enemy_powers_snapshot, &state.player.powers, &state.relics
                );
                game_log!("  → Attacks for {} and applies {} {}", modified_base, amount, debuff);
                // Apply Buffer/Invincible
                let modified_damage = crate::power_hooks::apply_on_attacked_to_change_damage(
                    modified_base, &state.player.powers
                );
                let actual = state.player_take_damage(modified_damage);
                state.apply_player_debuff(debuff, amount);
                game_log!("     Player takes {} damage, gains {} {} (HP: {})", 
                    actual, amount, debuff, state.player.current_hp);
                if actual > 0 {
                    let hp_loss_result = trigger_relics(state, &GameEvent::PlayerLoseHp { amount: actual }, None);
                    apply_relic_results(state, &hp_loss_result);
                    if hp_loss_result.extra_draw > 0 {
                        state.draw_cards(hp_loss_result.extra_draw);
                    }
                }
            }
            
            Intent::AttackDefend { damage, block } => {
                let modified_base = calculate_enemy_attack_damage(
                    damage, &enemy_powers_snapshot, &state.player.powers, &state.relics
                );
                game_log!("  → Attacks for {} and gains {} block", modified_base, block);
                // Apply Buffer/Invincible
                let modified_damage = crate::power_hooks::apply_on_attacked_to_change_damage(
                    modified_base, &state.player.powers
                );
                let actual = state.player_take_damage(modified_damage);
                if let Some(enemy) = state.enemies.get_mut(enemy_idx) {
                    enemy.gain_block(block);
                }
                game_log!("     Player takes {} damage (HP: {})", actual, state.player.current_hp);
                if actual > 0 {
                    let hp_loss_result = trigger_relics(state, &GameEvent::PlayerLoseHp { amount: actual }, None);
                    apply_relic_results(state, &hp_loss_result);
                    if hp_loss_result.extra_draw > 0 {
                        state.draw_cards(hp_loss_result.extra_draw);
                    }
                }
            }
            
            Intent::Summon { ref monster, count } => {
                game_log!("  → Summons {} x{}", monster, count);
                // TODO: Implement monster summoning
            }
            
            Intent::Special { ref name } => {
                game_log!("  → Uses special move: {}", name);
                // TODO: Handle special moves (Split, Escape, etc.)
            }
            
            Intent::Sleep => {
                game_log!("  → Zzz... (sleeping)");
            }
            
            Intent::Stunned => {
                game_log!("  → Is stunned! (skips turn)");
            }
            
            Intent::Escape => {
                game_log!("  → Escapes from combat!");
                if let Some(enemy) = state.enemies.get_mut(enemy_idx) {
                    enemy.alive = false;
                }
            }
            
            Intent::Unknown => {
                game_log!("  → Does nothing (unknown intent)");
            }
        }
    }
    
    // End-of-turn: Apply Ritual buff (gains Strength)
    // Note: Ritual only triggers if it wasn't gained this turn
    for enemy in state.enemies.iter_mut() {
        if !enemy.is_dead() {
            let ritual = enemy.get_buff("Ritual");
            if ritual > 0 && !enemy.is_buff_new_this_turn("Ritual") {
                enemy.add_buff("Strength", ritual);
                game_log!("\n  ⚡ {}'s Ritual triggers: +{} Strength (now {})", 
                    enemy.name, ritual, enemy.strength());
            }
            // Clear the "new buff" tracking for next turn
            enemy.clear_new_buff_tracking();
        }
    }
    
    // End-of-round: Decrement player and enemy turn-based debuffs
    // Java: MonsterGroup.applyEndOfTurnPowers() -> p.atEndOfRound()
    state.player.powers.on_round_end();
    for enemy in state.enemies.iter_mut() {
        if !enemy.is_dead() {
            enemy.powers.on_round_end();
            
            // Poison: deal damage = stacks, then decrement by 1
            let poison = enemy.powers.get("Poison");
            if poison > 0 {
                enemy.hp -= poison;
                game_log!("  🧪 {}'s Poison deals {} damage (HP: {})", enemy.name, poison, enemy.hp);
                if poison <= 1 {
                    enemy.powers.remove("Poison");
                } else {
                    enemy.powers.force_set("Poison", poison - 1);
                }
            }
        }
    }
    
    // Plan next moves for all living enemies
    plan_enemy_moves(state, monster_library);
    
    game_log!("\n━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━");
}

/// Plan next moves for all living enemies.
pub fn plan_enemy_moves(state: &mut GameState, monster_library: &MonsterLibrary) {
    // Build list of ally alive states for conditions
    let allies_alive: Vec<bool> = state.enemies.iter().map(|e| !e.is_dead()).collect();
    
    for enemy in state.enemies.iter_mut() {
        if enemy.is_dead() {
            continue;
        }
        
        // Get the monster definition
        if let Ok(def) = monster_library.get(&enemy.name) {
            enemy.plan_next_move(def, &mut rand::rng(), &allies_alive);
            
            // Display intent
            let intent_str = format_intent(&enemy.current_intent);
            game_log!("  🎯 {} intends to: {} ({})", 
                enemy.name, enemy.current_move, intent_str);
        } else {
            // Fallback for simple test enemies
            enemy.current_intent = Intent::Attack { damage: 6, times: 1 };
            enemy.current_move = "Attack".to_string();
            game_log!("  🎯 {} intends to: Attack (6 damage)", enemy.name);
        }
    }
}

/// Format an intent for display.
pub(crate) fn format_intent(intent: &Intent) -> String {
    match intent {
        Intent::Attack { damage, times } => {
            if *times > 1 {
                format!("Attack {}x{}", damage, times)
            } else {
                format!("Attack {}", damage)
            }
        }
        Intent::AttackAll { damage } => format!("Attack ALL {}", damage),
        Intent::Defend { block } => format!("Block {}", block),
        Intent::Buff { name, amount } => format!("+{} {}", amount, name),
        Intent::Debuff { name, amount } => format!("Apply {} {}", amount, name),
        Intent::AddCard { card, amount, destination } => format!("Add {}x {} to {}", amount, card, destination),
        Intent::AttackDebuff { damage, debuff, amount } => {
            format!("Attack {} + {} {}", damage, amount, debuff)
        }
        Intent::AttackDefend { damage, block } => format!("Attack {} + Block {}", damage, block),
        Intent::Summon { monster, count } => format!("Summon {} x{}", monster, count),
        Intent::Special { name } => format!("Special: {}", name),
        Intent::Sleep => "Sleeping".to_string(),
        Intent::Stunned => "Stunned".to_string(),
        Intent::Escape => "Escaping".to_string(),
        Intent::Unknown => "???".to_string(),
    }
}

// ============================================================================
// Combat Event Triggers
// ============================================================================

/// Trigger relics and effects at the start of combat.
/// Call this when enemies are first spawned and combat begins.
pub fn on_battle_start(state: &mut GameState, library: &CardLibrary, relic_library: Option<&RelicLibrary>) {
    // Resolve card types for all cards in play zones
    state.resolve_all_card_types(library);
    
    game_log!("\n⚔️ Combat Start - Triggering Relics");
    let result = trigger_relics(state, &GameEvent::BattleStart, relic_library);
    apply_relic_results(state, &result);
    

    
    // MutagenicStrength: Lose 3 Str at end of first turn
    // Java: MutagenicStrength → applies LoseStrengthPower(3) at battle start
    // LoseStrengthPower: atEndOfTurn → ApplyPower(Strength, -amount) + RemoveSelf
    if state.relics.iter().any(|r| r.id == "MutagenicStrength" && r.active) {
        state.end_of_turn_effects.push(
            crate::state::EndOfTurnEffect::LoseBuff {
                buff: "Strength".to_string(),
                amount: 3,
                all: false,
            }
        );
        game_log!("  🧬 MutagenicStrength: Will lose 3 Strength at end of turn");
    }
    
    // TeardropLocket: Enter Calm stance at combat start (Watcher)
    // Java: TeardropLocket.atBattleStart() → ChangeStanceAction("Calm")
    if state.relics.iter().any(|r| r.id == "TeardropLocket" && r.active) {
        state.player.stance = crate::core::stances::Stance::Calm;
        game_log!("  💧 TeardropLocket: Entered Calm stance");
    }
    
    // Enchiridion: At battle start, add a random Power card with cost 0 to hand.
    // Java: Enchiridion.atPreBattle() → MakeTempCardInHandAction(random Power, cost=0)
    if state.relics.iter().any(|r| r.id == "Enchiridion" && r.active) {
        if let Some(mut power_card) = library.get_random_card("Power", &mut state.rng) {
            power_card.base_cost = 0;
            power_card.current_cost = 0;
            game_log!("  📖 Enchiridion: {} added to hand (cost 0)!", power_card.definition_id);
            if state.hand.len() < 10 {
                state.hand.push(power_card);
            } else {
                state.discard_pile.push(power_card);
            }
            if let Some(relic) = state.relics.iter_mut().find(|r| r.id == "Enchiridion") {
                relic.pulse();
            }
        }
    }
    // Toolbox: At battle start, choose 1 of 3 random Colorless cards → add to hand.
    // Java: Toolbox.atBattleStartPreDraw() → ChooseOneColorless (3 unique colorless, pick 1)
    if state.relics.iter().any(|r| r.id == "Toolbox" && r.active) {
        // Generate 3 unique colorless cards
        let mut choices: Vec<CardInstance> = Vec::new();
        for _ in 0..30 { // max attempts to find 3 unique
            if choices.len() >= 3 { break; }
            if let Some(card) = library.get_random_card("Colorless", &mut state.rng) {
                if !choices.iter().any(|c| c.definition_id == card.definition_id) {
                    choices.push(card);
                }
            }
        }
        if !choices.is_empty() {
            // Auto-pick: random selection (AI will learn this)
            let pick_idx = state.rng.random_range(0..choices.len());
            let picked = choices.remove(pick_idx);
            game_log!("  🧰 Toolbox: chose {} (from {} colorless options)", picked.definition_id, choices.len() + 1);
            if state.hand.len() < 10 {
                state.hand.push(picked);
            } else {
                state.discard_pile.push(picked);
            }
            if let Some(relic) = state.relics.iter_mut().find(|r| r.id == "Toolbox") {
                relic.pulse();
            }
        }
    }
    
    // Defect Orb Relics: Channel orbs at battle start
    // CrackedCore: channel 1 Lightning at battle start (Defect starter relic)
    // Java: CrackedCore.atPreBattle() → channelOrb(new Lightning())
    if state.relics.iter().any(|r| (r.id == "CrackedCore" || r.id == "Cracked Core") && r.active) {
        channel_orb(state, "Lightning", 1);
        game_log!("  ⚡ Cracked Core: Channeled 1 Lightning orb");
    }
    
    // NuclearBattery: channel 1 Plasma at battle start
    // Java: NuclearBattery.atPreBattle() → channelOrb(new Plasma())
    if state.relics.iter().any(|r| (r.id == "NuclearBattery" || r.id == "Nuclear Battery") && r.active) {
        channel_orb(state, "Plasma", 1);
        game_log!("  🔋 Nuclear Battery: Channeled 1 Plasma orb");
    }
    
    // SymbioticVirus: channel 1 Dark at battle start
    // Java: SymbioticVirus.atPreBattle() → channelOrb(new Dark())
    if state.relics.iter().any(|r| (r.id == "SymbioticVirus" || r.id == "Symbiotic Virus") && r.active) {
        channel_orb(state, "Dark", 1);
        game_log!("  🦠 Symbiotic Virus: Channeled 1 Dark orb");
    }
    
    // RunicCapacitor: +3 max orb slots at battle start
    // Java: RunicCapacitor.atTurnStart() (first turn) → IncreaseMaxOrbAction(3)
    if state.relics.iter().any(|r| (r.id == "RunicCapacitor" || r.id == "Runic Capacitor") && r.active) {
        state.max_orbs += 3;
        game_log!("  🔮 Runic Capacitor: +3 max orb slots (now {})", state.max_orbs);
    }
    
    // Draw extra cards if relics provide them
    if result.extra_draw > 0 {
        game_log!("  📜 Drawing {} extra cards from relics", result.extra_draw);
        state.draw_cards(result.extra_draw);
    }
}

/// Trigger relics and effects at the start of a turn.
/// Should be called at the beginning of `start_turn()`.
pub fn on_turn_start(state: &mut GameState, library: &CardLibrary, relic_library: Option<&RelicLibrary>) {
    // Divinity auto-exit: at start of turn, return to Neutral
    // Java: DivinityStance.atStartOfTurn() → ChangeStanceAction("Neutral")
    if state.player.stance.auto_exit_on_turn_start() {
        let old = state.player.stance;
        state.player.stance = crate::core::stances::Stance::Neutral;
        game_log!("  🧘 {} stance auto-exits → Neutral", old.name());
        // Note: Divinity has no on_exit energy; on_stance_change hooks still fire
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
            apply_hook_effects(state, &[effect.clone()], &pid_str, None, Some(library));
        }
    }
    
    let turn = state.turn;
    let result = trigger_relics(state, &GameEvent::TurnStart { turn }, relic_library);
    apply_relic_results(state, &result);
    
    // Brimstone: +1 Str to all enemies each turn
    // Java: Brimstone.atTurnStart() → all enemies gain 1 Str
    if state.relics.iter().any(|r| r.id == "Brimstone" && r.active) {
        for enemy in state.enemies.iter_mut() {
            if !enemy.is_dead() {
                enemy.apply_status("Strength", 1);
            }
        }
        game_log!("  🔥 Brimstone: All enemies +1 Strength");
    }
    
    // Philosopher's Stone: +1 Str to ALL enemies at combat start (one-time)
    // Java: Philosopher'sStone.atBattleStart() → all monsters gain 1 Str
    // +1 Energy/turn is handled by JSON TurnStart hook
    if state.turn == 1 && state.relics.iter().any(|r| (r.id == "Philosopher'sStone" || r.id == "PhilosophersStone") && r.active) {
        for enemy in state.enemies.iter_mut() {
            if !enemy.is_dead() {
                enemy.apply_status("Strength", 1);
            }
        }
        game_log!("  🔮 Philosopher's Stone: All enemies +1 Strength");
    }
    
    // SneckoEye: Confused effect — randomize all card costs in hand to 0-3
    // Java: SneckoEye.atBattleStart() → ApplyPower(Confused)
    //        ConfusedPower.onCardDraw() → card.setCostForTurn(random 0-3)
    // We apply the cost randomization after drawing cards (simpler than hooking onCardDraw)
    if state.relics.iter().any(|r| r.id == "SneckoEye" && r.active) {
        use rand::Rng;
        for card in state.hand.iter_mut() {
            if card.current_cost >= 0 { // Don't randomize unplayable (-2) or X-cost (-1)
                card.current_cost = state.rng.random_range(0..4);
            }
        }
        game_log!("  🐍 Snecko Eye: Confused — hand card costs randomized!");
    }
    
    // Fire power hooks for TurnStart (DemonForm, Berserk, Brutality, etc.)
    let turn_effects = crate::power_hooks::collect_at_start_of_turn_effects(&state.player.powers);
    for (power_id, effects) in &turn_effects {
        let id_str = state.player.powers.iter()
            .find(|(name, _)| crate::power_hooks::PowerId::from_str(name) == *power_id)
            .map(|(name, _)| name.clone())
            .unwrap_or_else(|| format!("{:?}", power_id));
        game_log!("  🔔 Power hook fires: {:?} (atStartOfTurn)", power_id);
        apply_hook_effects(state, effects, &id_str, None, Some(library));
    }
    
    // Draw extra cards if relics provide them (e.g., Snecko Eye)
    if result.extra_draw > 0 {
        state.draw_cards(result.extra_draw);
    }
    
    // Orb passives at start of turn (Plasma: gain energy)
    for i in 0..state.orb_slots.len() {
        let effect = state.orb_slots[i].on_start_of_turn();
        match effect {
            crate::core::orbs::PassiveEffect::GainEnergy(energy) => {
                state.player.energy += energy;
                game_log!("  ✨ Plasma passive: +{} energy (total: {})", energy, state.player.energy);
            }
            _ => {} // Other orbs don't fire at start of turn
        }
    }
    
    // EmotionChip: If took damage last turn (counter==1), trigger all orb passives again.
    // Java: EmotionChip.atTurnStart() → ImpulseAction (triggers all orb passives once more)
    // Counter is set by PlayerLoseHp in trigger_relics.
    if let Some(relic) = state.relics.iter_mut().find(|r| (r.id == "EmotionChip" || r.id == "Emotion Chip") && r.active) {
        if relic.counter == 1 {
            relic.counter = 0;
            game_log!("  🤖 Emotion Chip: Triggering extra orb passives (took damage last turn)");
            for i in 0..state.orb_slots.len() {
                let effect = state.orb_slots[i].on_start_of_turn();
                match effect {
                    crate::core::orbs::PassiveEffect::GainEnergy(energy) => {
                        state.player.energy += energy;
                        game_log!("    ✨ Extra Plasma passive: +{} energy", energy);
                    }
                    _ => {}
                }
            }
        }
    }
}

/// Trigger relics that fire AFTER the initial card draw.
/// Java: `atTurnStartPostDraw()` — fires after hand is drawn.
/// Call this right after `state.start_turn()`.
pub fn on_turn_start_post_draw(state: &mut GameState, library: &CardLibrary) {
    // GamblingChip: First turn only — select cards to discard, draw equal number.
    // Java: GamblingChip.atTurnStartPostDraw() → GamblingChipAction
    // GamblingChipAction: open hand select (up to 99), discard selected, DrawCardAction(selected.size())
    if let Some(relic) = state.relics.iter_mut().find(|r| r.id == "GamblingChip" && r.active) {
        if relic.counter == 0 {
            // First activation this combat (counter 0 = not activated yet)
            relic.counter = 1; // Mark as activated
            relic.pulse();
            
            // Heuristic: discard curses, status cards, and excess duplicates
            // (AI will eventually learn optimal discards)
            let mut to_discard: Vec<usize> = Vec::new();
            let mut seen_ids: Vec<String> = Vec::new();
            
            for (i, card) in state.hand.iter().enumerate() {
                let dominated = match card.card_type {
                    CardType::Curse | CardType::Status => true,
                    _ => {
                        // Discard 3rd+ copies of any card
                        let count = seen_ids.iter().filter(|id| *id == &card.definition_id).count();
                        count >= 2
                    }
                };
                seen_ids.push(card.definition_id.clone());
                if dominated {
                    to_discard.push(i);
                }
            }
            
            if !to_discard.is_empty() {
                let discard_count = to_discard.len();
                // Remove from back to front to maintain indices
                for &idx in to_discard.iter().rev() {
                    let card = state.hand.remove(idx);
                    game_log!("  🎲 Gambling Chip: discarded {}", card.definition_id);
                    state.discard_pile.push(card);
                }
                // Draw equal number
                state.draw_cards(discard_count as i32);
                game_log!("  🎲 Gambling Chip: drew {} replacement cards", discard_count);
            }
        }
    }
    
    // Pocketwatch: If ≤3 cards played last turn AND not first turn, draw 3.
    // Java: Pocketwatch.atTurnStartPostDraw() → if counter<=3 && !firstTurn: DrawCardAction(3)
    // Counter tracks cards played per turn via PlayerPlayCard event in standalone function
    let pocketwatch_draw = if let Some(relic) = state.relics.iter().find(|r| r.id == "Pocketwatch" && r.active) {
        // counter == -1 means first turn (don't trigger)
        // counter >= 0 means cards played last turn
        if relic.counter >= 0 && relic.counter <= 3 {
            true
        } else {
            false
        }
    } else {
        false
    };
    
    if pocketwatch_draw {
        state.draw_cards(3);
        game_log!("  ⏰ Pocketwatch: drew 3 cards (≤3 cards played last turn)");
        if let Some(relic) = state.relics.iter_mut().find(|r| r.id == "Pocketwatch") {
            relic.pulse();
        }
    }
    
    // Reset Pocketwatch counter for this turn
    if let Some(relic) = state.relics.iter_mut().find(|r| r.id == "Pocketwatch" && r.active) {
        relic.counter = 0; // Ready to count cards played this turn
    }
}

/// Trigger relics and effects at the end of a turn.
/// Should be called before the enemy turn.
pub fn on_turn_end(state: &mut GameState, library: &CardLibrary, relic_library: Option<&RelicLibrary>) {
    let turn = state.turn;
    let result = trigger_relics(state, &GameEvent::TurnEnd { turn }, relic_library);
    apply_relic_results(state, &result);
    
    // Fire power hooks for TurnEnd (Metallicize, Combust, Regen, etc.)
    let turn_effects = crate::power_hooks::collect_at_end_of_turn_effects(&state.player.powers);
    for (power_id, effects) in &turn_effects {
        let id_str = state.player.powers.iter()
            .find(|(name, _)| crate::power_hooks::PowerId::from_str(name) == *power_id)
            .map(|(name, _)| name.clone())
            .unwrap_or_else(|| format!("{:?}", power_id));
        
        game_log!("  🔔 Power hook fires: {:?} (atEndOfTurn)", power_id);
        apply_hook_effects(state, &effects, &id_str, None, Some(library));
    }
    
    // Execute deferred end-of-turn effects (e.g., Flex's LoseBuff)
    let eot_effects: Vec<_> = state.end_of_turn_effects.drain(..).collect();
    for effect in eot_effects {
        match effect {
            crate::state::EndOfTurnEffect::LoseBuff { ref buff, amount, all } => {
                let removed = state.player.remove_buff(buff, amount, all);
                game_log!("  → End-of-turn: lost {} stacks of '{}'", removed, buff);
            }
        }
    }
    
    // Nilry's Codex: At end of turn, choose 1 of 3 random cards → add to draw pile.
    // Java: NilrysCodex.onPlayerEndTurn() → CodexAction
    // CodexAction.generateCardChoices(): 3 unique random cards via returnTrulyRandomCardInCombat()
    // Selected card → ShowCardAndAddToDrawPileEffect (temp card in draw pile)
    if state.relics.iter().any(|r| r.id == "NilrysCodex" && r.active) {
        if !state.enemies.iter().all(|e| e.is_dead()) {
            // Generate 3 unique random cards (character-colored, like Java's returnTrulyRandomCardInCombat)
            let mut choices: Vec<CardInstance> = Vec::new();
            for _ in 0..30 {
                if choices.len() >= 3 { break; }
                if let Some(card) = library.get_random_card_of_color(
                    "any",
                    Some(crate::core::schema::CardColor::Red), // Ironclad
                    &mut state.rng,
                ) {
                    if !choices.iter().any(|c| c.definition_id == card.definition_id) {
                        choices.push(card);
                    }
                }
            }
            if !choices.is_empty() {
                // Auto-pick: random selection (AI will learn this)
                let pick_idx = state.rng.random_range(0..choices.len());
                let picked = choices.remove(pick_idx);
                game_log!("  📜 Nilry's Codex: {} added to draw pile (from {} options)", 
                    picked.definition_id, choices.len() + 1);
                // Add as temp card to draw pile (NOT master_deck — Java uses ShowCardAndAddToDrawPileEffect)
                state.add_temp_card_to_draw_pile(picked);
                if let Some(relic) = state.relics.iter_mut().find(|r| r.id == "NilrysCodex") {
                    relic.pulse();
                }
            }
        }
    }
    
    // Orb passives at end of turn (Lightning, Frost, Dark)
    for i in 0..state.orb_slots.len() {
        let effect = state.orb_slots[i].on_end_of_turn();
        match effect {
            crate::core::orbs::PassiveEffect::DamageRandom(dmg) => {
                // Lightning passive: damage enemies
                // Electrodynamics: hit ALL enemies instead of random one
                let has_electro = state.player.powers.has("Electrodynamics");
                let actual = std::cmp::max(0, dmg);
                if has_electro {
                    for enemy in state.enemies.iter_mut() {
                        if !enemy.is_dead() {
                            enemy.hp -= actual;
                            game_log!("  ⚡ Lightning passive (Electro): {} takes {} damage (HP: {})",
                                enemy.name, actual, enemy.hp);
                        }
                    }
                } else {
                    let alive_idx = state.enemies.iter()
                        .enumerate()
                        .filter(|(_, e)| !e.is_dead())
                        .map(|(idx, _)| idx)
                        .next();
                    if let Some(idx) = alive_idx {
                        state.enemies[idx].hp -= actual;
                        game_log!("  ⚡ Lightning passive: {} takes {} damage (HP: {})",
                            state.enemies[idx].name, actual, state.enemies[idx].hp);
                    }
                }
            }
            crate::core::orbs::PassiveEffect::GainBlock(block) => {
                // Frost passive: gain block
                state.player.block += std::cmp::max(0, block);
                game_log!("  ❄️ Frost passive: +{} block (total: {})", block, state.player.block);
            }
            crate::core::orbs::PassiveEffect::DarkAccumulate(added, total) => {
                // Dark passive: accumulate evoke damage
                game_log!("  🌑 Dark passive: +{} evoke damage (total: {})", added, total);
            }
            _ => {} // Plasma fires at start, None = skip
        }
    }
    
    // Gold-Plated Cables: leftmost orb triggers passive once more
    // Java: TriggerEndOfTurnOrbsAction → if hasRelic("Cables") → orbs.get(0).onEndOfTurn()
    if !state.orb_slots.is_empty() 
        && state.relics.iter().any(|r| r.id == "Cables" && r.active) 
    {
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
                    game_log!("  🔌 Cables: Extra Lightning passive → {} takes {} damage", 
                        state.enemies[idx].name, actual);
                }
            }
            crate::core::orbs::PassiveEffect::GainBlock(block) => {
                state.player.block += std::cmp::max(0, block);
                game_log!("  🔌 Cables: Extra Frost passive → +{} block", block);
            }
            crate::core::orbs::PassiveEffect::DarkAccumulate(added, _total) => {
                game_log!("  🔌 Cables: Extra Dark passive → +{} evoke damage", added);
            }
            _ => {}
        }
    }
    
    // End turn: discard hand and track manual discards
    let discarded = state.end_turn();
    
    // Trigger relics for manual discards (ToughBandages, Tingsha)
    // Java: PotionPopUp → for each manually discarded card: r.onManualDiscard()
    if discarded > 0 {
        let discard_result = trigger_relics(
            state, 
            &GameEvent::PlayerManualDiscard { count: discarded },
            relic_library,
        );
        apply_relic_results(state, &discard_result);
    }
}

/// Trigger relics when combat ends.
/// Call this when all enemies are dead or player flees.
pub fn on_battle_end(state: &mut GameState, won: bool, relic_library: Option<&RelicLibrary>) {
    game_log!("\n🏁 Combat End - Triggering Relics");
    let result = trigger_relics(state, &GameEvent::BattleEnd { won }, relic_library);
    apply_relic_results(state, &result);
}

/// Check if all enemies are dead.
pub fn all_enemies_dead(state: &GameState) -> bool {
    state.enemies.iter().all(|e| e.is_dead())
}

/// Check if player is dead.
///
/// Also triggers Fairy in a Bottle if the player would die:
/// heals to 30% max HP and consumes the potion.
pub fn player_dead(state: &mut GameState) -> bool {
    if state.player.current_hp > 0 {
        return false;
    }
    
    // Check for Fairy in a Bottle potion
    if let Some(fairy_slot) = state.potions.find("FairyinaBottle") {
        let max_hp = state.player.max_hp;
        let heal_amount = (max_hp as f64 * 0.3).round() as i32;
        state.player.current_hp = heal_amount.max(1);
        let _ = state.potions.remove(fairy_slot);
        game_log!("  🧚 Fairy in a Bottle activates! Healed to {} HP (slot {})", state.player.current_hp, fairy_slot);
        return false;
    }
    
    // Check for Lizard Tail relic (one-time death prevention)
    // Java: AbstractPlayer.java:1476 — if hasRelic("Lizard Tail") && counter == -1: onTrigger()
    // LizardTail.onTrigger() → heal 50% maxHP, counter = -2
    if let Some(relic) = state.relics.iter_mut().find(|r| r.id == "LizardTail" && r.active && r.counter == -1) {
        let max_hp = state.player.max_hp;
        let heal_amount = (max_hp / 2).max(1);
        state.player.current_hp = heal_amount;
        relic.counter = -2; // Used up for the rest of the run
        relic.pulse();
        game_log!("  🦎 Lizard Tail activates! Healed to {} HP ({}/2 max HP)", heal_amount, max_hp);
        return false;
    }
    
    true
}

