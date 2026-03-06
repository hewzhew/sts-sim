//! Card override system for complex cards that JSON can't fully express.
//!
//! Some Ironclad cards have logic that requires runtime state inspection
//! (enemy intent, cards in piles, kill checks, etc.) which can't be encoded
//! in the data-driven JSON command system. This module provides Rust overrides
//! that replace the JSON command execution for these specific cards.
//!
//! Mirrors Java's approach where each card has its own `use()` method with
//! custom Action subclasses (SpotWeaknessAction, FeedAction, DropkickAction, etc.)

use crate::state::GameState;
use crate::engine::CommandResult;
use crate::engine::commands::calculate_card_damage;
use crate::power_hooks::RelicDamageFlags;
use crate::core::schema::CardType;

/// Check if a card has special play conditions that prevent it from being played.
/// Returns `true` if the card CAN be played, `false` if conditions are not met.
/// Cards without special conditions always return `true`.
///
/// Java: AbstractCard.canUse() override per card class.
pub fn card_can_play(card_id: &str, state: &GameState) -> bool {
    match card_id {
        // Grand Finale: can only play if draw pile is empty
        // Java: GrandFinale.canUse() → player.drawPile.isEmpty()
        "Grand_Finale" => state.draw_pile.is_empty(),
        
        // Signature Move: can only play if it's the only Attack in hand
        // Java: SignatureMove.canUse() → only attack card in hand
        "Signature_Move" => {
            let attack_count = state.hand.iter()
                .filter(|c| c.card_type == CardType::Attack)
                .count();
            // Must be exactly 1 attack (itself)
            attack_count <= 1
        }
        
        // Clash: can only play if all cards in hand are Attacks
        // Java: Clash.canUse() → every card in hand is CardType.ATTACK
        "Clash" => {
            state.hand.iter().all(|c| c.card_type == CardType::Attack)
        }
        
        // All other cards: no special condition
        _ => true,
    }
}

/// Build relic damage flags from the player's active relics.
fn build_relic_flags(state: &GameState) -> RelicDamageFlags {
    RelicDamageFlags {
        odd_mushroom: false, // Only applies to enemy→player damage
        paper_crane: state.relics.iter().any(|r| r.id == "PaperCrane" && r.active),
        paper_frog: state.relics.iter().any(|r| r.id == "PaperFrog" && r.active),
    }
}

/// Fire enemy's onAttacked power hooks after dealing damage.
/// This handles reflect effects like Thorns, Flame Barrier, and Caltrops.
/// Java: AbstractCreature.damage() → p.onAttacked() for each power
fn fire_on_attacked_hooks(state: &mut GameState, enemy_idx: usize, actual_damage: i32) {
    let enemy_power_snap: Vec<(String, i32)> = state.enemies[enemy_idx].powers.iter()
        .map(|(k, v)| (k.clone(), *v)).collect();
    for (pid, stacks) in &enemy_power_snap {
        let pi = crate::power_hooks::PowerInstance::new(
            crate::power_hooks::PowerId::from_str(pid), *stacks
        );
        let (_, effects) = pi.on_attacked(actual_damage);
        for effect in &effects {
            match effect {
                crate::power_hooks::HookEffect::DamageAttacker(dmg) => {
                    let thorns_actual = state.player.take_damage(*dmg);
                    game_log!("  \u{1f33f} {} reflects {} damage to player (HP: {})",
                        pid, thorns_actual, state.player.current_hp);
                }
                _ => {}
            }
        }
    }
}

/// Try to execute a card override. Returns `Some(results)` if the card has
/// an override, `None` to fall through to normal JSON command execution.
pub fn try_override(
    state: &mut GameState,
    card_id: &str,
    upgraded: bool,
    target_idx: Option<usize>,
) -> Option<Vec<CommandResult>> {
    match card_id {
        "Perfected_Strike" => Some(perfected_strike(state, upgraded, target_idx)),
        "Spot_Weakness" => Some(spot_weakness(state, upgraded, target_idx)),
        "Feed" => Some(feed(state, upgraded, target_idx)),
        "Dropkick" => Some(dropkick(state, upgraded, target_idx)),
        "Blizzard" => Some(blizzard(state, upgraded)),
        "Enlightenment" => Some(enlightenment(state, upgraded)),
        "Foreign_Influence" => Some(foreign_influence(state, upgraded)),
        "Phantasmal_Killer" => Some(phantasmal_killer(state)),
        "Second_Wind" => Some(second_wind(state, upgraded)),
        "Wallop" => Some(wallop(state, upgraded, target_idx)),
        "Spirit_Shield" | "SpiritShield" => Some(spirit_shield(state, upgraded)),
        "Indignation" => Some(indignation(state, upgraded)),
        _ => None,
    }
}

// ============================================================================
// Perfected Strike
// ============================================================================
// Java: PerfectedStrike.countCards() counts ALL cards with STRIKE tag
//       across hand + draw + discard piles.
//       baseDamage += magicNumber * countCards()
//       magicNumber = 2 (base), 3 (upgraded)

fn perfected_strike(
    state: &mut GameState,
    upgraded: bool,
    target_idx: Option<usize>,
) -> Vec<CommandResult> {
    let magic_number = if upgraded { 3 } else { 2 };
    let base_damage = 6; // Java: this.baseDamage = 6
    
    // Count Strike-tagged cards in hand + draw + discard
    // Java: PerfectedStrike.isStrike(c) → c.hasTag(CardTags.STRIKE)
    // In our system: card IDs containing "Strike" match the STRIKE tag
    //
    // IMPORTANT: play_card_from_hand() removes the card from hand BEFORE calling
    // this override. Since Perfected Strike IS a Strike card, we must add 1 to
    // account for itself. Java's calculateCardDamage() counts the card while it's
    // still in hand.
    let strike_count = count_strikes_in_piles(state) + 1; // +1 for self (already removed from hand)
    
    let total_base = base_damage + magic_number * strike_count;
    
    // Calculate final damage with powers (Strength, Weak, Vulnerable, etc.)
    let target_enemy = target_idx.unwrap_or(0);
    let final_damage = if let Some(enemy) = state.enemies.get(target_enemy) {
        calculate_card_damage(
            total_base, &state.player.powers, &enemy.powers, state.player.stance,
            build_relic_flags(state),
        )
    } else {
        total_base
    };
    
    // Apply damage
    let mut killed = false;
    let has_boot = state.relics.iter().any(|r| r.id == "Boot" && r.active);
    if target_enemy < state.enemies.len() && !state.enemies[target_enemy].is_dead() {
        let (actual, pend) = state.enemies[target_enemy].take_damage_from_player(final_damage, has_boot);
        if pend > 0 && !state.enemies[target_enemy].is_dead() { state.enemies[target_enemy].block += pend; }
        killed = state.enemies[target_enemy].is_dead();
        state.record_attack_result(actual, actual, killed);
        fire_on_attacked_hooks(state, target_enemy, actual);
        game_log!("  → Perfected Strike: {} base + {}×{} strikes = {} → {} damage{}",
            base_damage, magic_number, strike_count, total_base, actual,
            if killed { " (KILL)" } else { "" });
    }
    
    vec![CommandResult::DamageDealt {
        target: format!("Enemy {}", target_enemy),
        amount: final_damage,
        killed,
    }]
}

/// Count cards with the STRIKE tag across all piles.
/// Java: PerfectedStrike.countCards() checks hand + drawPile + discardPile.
fn count_strikes_in_piles(state: &GameState) -> i32 {
    let mut count = 0;
    for card in &state.hand {
        if is_strike(&card.definition_id) { count += 1; }
    }
    for card in &state.draw_pile {
        if is_strike(&card.definition_id) { count += 1; }
    }
    for card in &state.discard_pile {
        if is_strike(&card.definition_id) { count += 1; }
    }
    count
}

/// Check if a card has the STRIKE tag.
/// Java: AbstractCard.hasTag(CardTags.STRIKE)
/// Cards with STRIKE tag: Strike, Twin Strike, Perfected Strike, Pommel Strike,
/// Wild Strike, Meteor Strike, Thunder Strike, etc.
fn is_strike(card_id: &str) -> bool {
    card_id.contains("Strike")
}

// ============================================================================
// Spot Weakness
// ============================================================================
// Java: SpotWeaknessAction.update():
//   if target.getIntentBaseDmg() >= 0 → apply Strength to player
//   Meaning: only gains Strength if enemy intends to Attack
//   magicNumber = 3 (base), 4 (upgraded)

fn spot_weakness(
    state: &mut GameState,
    upgraded: bool,
    target_idx: Option<usize>,
) -> Vec<CommandResult> {
    let strength_gain = if upgraded { 4 } else { 3 };
    let target_enemy = target_idx.unwrap_or(0);
    
    // Java: this.targetMonster.getIntentBaseDmg() >= 0
    // getIntentBaseDmg() returns -1 for non-attack intents
    let is_attacking = if let Some(enemy) = state.enemies.get(target_enemy) {
        matches!(enemy.current_intent,
            crate::enemy::Intent::Attack { .. } |
            crate::enemy::Intent::AttackAll { .. } |
            crate::enemy::Intent::AttackDebuff { .. } |
            crate::enemy::Intent::AttackDefend { .. }
        )
    } else {
        false
    };
    
    if is_attacking {
        state.player.apply_status("Strength", strength_gain);
        game_log!("  → Spot Weakness: Enemy is attacking! Gained {} Strength", strength_gain);
        vec![CommandResult::BuffGained { buff: "Strength".to_string(), amount: strength_gain }]
    } else {
        game_log!("  → Spot Weakness: Enemy is NOT attacking. No effect.");
        vec![CommandResult::Skipped { reason: "Enemy not attacking".to_string() }]
    }
}

// ============================================================================
// Feed
// ============================================================================
// Java: FeedAction.update():
//   Deal damage to target
//   If target is dying (isDying || hp <= 0) AND not halfDead AND not Minion:
//     → player.increaseMaxHp(magicNumber)
//   baseDamage = 10, upgradeDamage(2) → 12
//   magicNumber = 3, upgradeMagicNumber(1) → 4
//   Exhaust = true

fn feed(
    state: &mut GameState,
    upgraded: bool,
    target_idx: Option<usize>,
) -> Vec<CommandResult> {
    let base_damage = if upgraded { 12 } else { 10 };
    let hp_gain = if upgraded { 4 } else { 3 };
    let target_enemy = target_idx.unwrap_or(0);
    
    // Calculate damage
    let final_damage = if let Some(enemy) = state.enemies.get(target_enemy) {
        calculate_card_damage(
            base_damage, &state.player.powers, &enemy.powers, state.player.stance,
            build_relic_flags(state),
        )
    } else {
        base_damage
    };
    
    // Apply damage
    let mut killed = false;
    let mut actual = 0;
    let has_boot = state.relics.iter().any(|r| r.id == "Boot" && r.active);
    if target_enemy < state.enemies.len() && !state.enemies[target_enemy].is_dead() {
        let (act, pend) = state.enemies[target_enemy].take_damage_from_player(final_damage, has_boot);
        actual = act;
        if pend > 0 && !state.enemies[target_enemy].is_dead() { state.enemies[target_enemy].block += pend; }
        killed = state.enemies[target_enemy].is_dead();
        fire_on_attacked_hooks(state, target_enemy, actual);
    }
    
    state.record_attack_result(actual, actual, killed);
    
    let mut results = vec![CommandResult::DamageDealt {
        target: format!("Enemy {}", target_enemy),
        amount: actual,
        killed,
    }];
    
    // If killed, gain max HP
    // Java: isDying && currentHealth <= 0 && !halfDead && !hasPower("Minion")
    if killed {
        state.player.max_hp += hp_gain;
        state.player.current_hp += hp_gain;
        game_log!("  → Feed: Killed enemy! +{} Max HP (now {}/{})",
            hp_gain, state.player.current_hp, state.player.max_hp);
        results.push(CommandResult::HpGained { amount: hp_gain });
    } else {
        game_log!("  → Feed: Dealt {} damage (enemy survived)", actual);
    }
    
    // Exhaust
    results.push(CommandResult::CardExhausted);
    
    results
}

// ============================================================================
// Dropkick
// ============================================================================
// Java: DropkickAction.update():
//   If target has Vulnerable → Draw 1, Gain 1 Energy
//   Then deal damage (always, regardless of Vulnerable)
//   baseDamage = 5, upgradeDamage(3) → 8

fn dropkick(
    state: &mut GameState,
    upgraded: bool,
    target_idx: Option<usize>,
) -> Vec<CommandResult> {
    let base_damage = if upgraded { 8 } else { 5 };
    let target_enemy = target_idx.unwrap_or(0);
    
    let mut results = Vec::new();
    
    // Check if enemy has Vulnerable BEFORE dealing damage
    // Java: this.target.hasPower("Vulnerable")
    let has_vulnerable = state.enemies.get(target_enemy)
        .map_or(false, |e| e.powers.has("Vulnerable"));
    
    if has_vulnerable {
        // Gain 1 energy
        state.player.energy += 1;
        game_log!("  → Dropkick: Enemy Vulnerable! +1 Energy (now {})", state.player.energy);
        results.push(CommandResult::EnergyGained { amount: 1 });
        
        // Draw 1 card
        state.draw_cards(1);
        game_log!("  → Dropkick: Drew 1 card");
        results.push(CommandResult::CardsDrawn { count: 1 });
    }
    
    // Calculate and deal damage
    let final_damage = if let Some(enemy) = state.enemies.get(target_enemy) {
        calculate_card_damage(
            base_damage, &state.player.powers, &enemy.powers, state.player.stance,
            build_relic_flags(state),
        )
    } else {
        base_damage
    };
    
    let mut killed = false;
    let mut actual = 0;
    let has_boot = state.relics.iter().any(|r| r.id == "Boot" && r.active);
    if target_enemy < state.enemies.len() && !state.enemies[target_enemy].is_dead() {
        let (act, pend) = state.enemies[target_enemy].take_damage_from_player(final_damage, has_boot);
        actual = act;
        if pend > 0 && !state.enemies[target_enemy].is_dead() { state.enemies[target_enemy].block += pend; }
        killed = state.enemies[target_enemy].is_dead();
        fire_on_attacked_hooks(state, target_enemy, actual);
    }
    
    state.record_attack_result(actual, actual, killed);
    
    game_log!("  → Dropkick: Dealt {} damage{}{}", 
        actual, 
        if has_vulnerable { " (Vulnerable bonus)" } else { "" },
        if killed { " (KILL)" } else { "" });
    
    results.push(CommandResult::DamageDealt {
        target: format!("Enemy {}", target_enemy),
        amount: actual,
        killed,
    });
    
    results
}

// ============================================================================
// Blizzard (Defect - Uncommon Attack)
// ============================================================================
// Java: Blizzard.use():
//   frostCount = orbsChanneledThisCombat.stream().filter(Frost).count()
//   baseDamage = frostCount * magicNumber (2 base, 3 upgraded)
//   DamageAllEnemiesAction with multiDamage

fn blizzard(
    state: &mut GameState,
    upgraded: bool,
) -> Vec<CommandResult> {
    let magic_number = if upgraded { 3 } else { 2 };
    let frost_count = state.frost_channeled_this_combat;
    let base_damage = frost_count * magic_number;
    
    let mut results = Vec::new();
    
    for i in 0..state.enemies.len() {
        let final_damage = {
            let enemy = &state.enemies[i];
            if enemy.is_dead() { continue; }
            calculate_card_damage(
                base_damage, &state.player.powers, &enemy.powers, state.player.stance,
                build_relic_flags(state),
            )
        };
        
        let has_boot = state.relics.iter().any(|r| r.id == "Boot" && r.active);
        let (actual, killed, enemy_name) = {
            let enemy = &mut state.enemies[i];
            let (actual, pend) = enemy.take_damage_from_player(final_damage, has_boot);
            if pend > 0 && !enemy.is_dead() { enemy.block += pend; }
            let killed = enemy.is_dead();
            let name = enemy.name.clone();
            (actual, killed, name)
        };
        fire_on_attacked_hooks(state, i, actual);
        state.record_attack_result(actual, actual, killed);
        
        game_log!("  → Blizzard: {} frost × {} = {} base → {} to {}{}",
            frost_count, magic_number, base_damage, actual, enemy_name,
            if killed { " (KILL)" } else { "" });
        
        results.push(CommandResult::DamageDealt {
            target: format!("Enemy {}", i),
            amount: actual,
            killed,
        });
    }
    
    results
}

// ============================================================================
// Enlightenment (Colorless - Uncommon Skill)
// ============================================================================
// Java: EnlightenmentAction.update():
//   Base: for each card in hand with costForTurn > 1 → costForTurn = 1
//   Upgraded: also permanently sets card.cost = 1

fn enlightenment(
    state: &mut GameState,
    upgraded: bool,
) -> Vec<CommandResult> {
    let mut count = 0;
    for card in &mut state.hand {
        if card.current_cost > 1 {
            card.current_cost = 1;
            if upgraded {
                card.base_cost = 1;
            }
            count += 1;
        }
    }
    
    game_log!("  → Enlightenment{}: Set {} card costs to 1",
        if upgraded { "+" } else { "" }, count);
    
    vec![CommandResult::Skipped { reason: format!("Reduced {} card costs to 1", count) }]
}

// ============================================================================
// Foreign Influence (Watcher - Uncommon Skill)
// ============================================================================
// Java: ForeignInfluenceAction.update():
//   Get 3 random Attack cards from other classes
//   Player chooses 1 to add to hand (0-cost for turn)
//   Upgraded: choose 2
//   Card has Exhaust
// Sim simplification: add 1 random Attack to hand (or 2 if upgraded)

fn foreign_influence(
    state: &mut GameState,
    upgraded: bool,
) -> Vec<CommandResult> {
    let count = if upgraded { 2 } else { 1 };
    
    // Add random Attack cards to hand
    // In sim, we generate random attack cards with 0 cost for this turn
    if let Some(ref card_lib) = state.card_library.clone() {
        for _ in 0..count {
            if let Some(mut card) = card_lib.get_random_card("Attack", &mut state.rng) {
                card.current_cost = 0; // Free for this turn
                state.hand.push(card);
            }
        }
    }
    
    game_log!("  → Foreign Influence: Added {} random Attack(s) to hand (0-cost)", count);
    
    vec![CommandResult::CardsDrawn { count }]
}

// ============================================================================
// Phantasmal Killer (Silent - Rare Skill)
// ============================================================================
// Java: PhantasmalKiller.use():
//   ApplyPowerAction(player, player, PhantasmalPower(1))
//   PhantasmalPower.atStartOfTurn(): apply DoubleDamagePower
//   Cost: 1 (upgraded: 0)
//   Ethereal (base only)

fn phantasmal_killer(
    state: &mut GameState,
) -> Vec<CommandResult> {
    // Apply DoubleDamage power directly (simplified from Phantasmal → DoubleDamage)
    // Java: PhantasmalPower gives DoubleDamage at start of next turn
    // For sim accuracy: we apply DoubleDamage immediately (it will x2 next attacks)
    state.player.apply_status("DoubleDamage", 1);
    
    game_log!("  → Phantasmal Killer: Applied DoubleDamage (next turn deal double damage)");
    
    vec![CommandResult::BuffGained { buff: "DoubleDamage".to_string(), amount: 1 }]
}

// ============================================================================
// Second Wind (Ironclad - Uncommon Skill)
// ============================================================================
// Java: SecondWindAction.update():
//   for (card in hand) {
//       if (card.type != ATTACK) {
//           hand.moveToExhaustPile(card);
//           addToBot(GainBlockAction(player, blockPerCard));
//       }
//   }
//   blockPerCard = 5 (base), 7 (upgraded)
//
// The JSON version does ExhaustCards(ALL, non-attack) + GainBlock(flat 5/7),
// which only gives a single flat block instead of per-card block.

fn second_wind(
    state: &mut GameState,
    upgraded: bool,
) -> Vec<CommandResult> {
    use crate::power_hooks::{calculate_block_hooked, PowerInstance, PowerId};
    
    let block_per_card = if upgraded { 7 } else { 5 };
    
    // Collect indices of non-Attack cards in hand (excluding the card being played,
    // which is already removed from hand by play_card_from_hand before override fires)
    let exhaust_indices: Vec<usize> = state.hand.iter().enumerate()
        .filter(|(_, c)| c.card_type != CardType::Attack)
        .map(|(i, _)| i)
        .collect();
    
    let exhaust_count = exhaust_indices.len() as i32;
    
    // Remove cards from hand in reverse order (to keep indices stable)
    let mut exhausted_cards = Vec::new();
    for &idx in exhaust_indices.iter().rev() {
        let card = state.hand.remove(idx);
        exhausted_cards.push(card);
    }
    
    // For each exhausted card: gain block + fire on_exhaust hooks
    let mut total_block = 0;
    for card in &exhausted_cards {
        // Calculate block through the pipeline (Dex, Frail, etc.)
        let actual_block = calculate_block_hooked(block_per_card, &state.player.powers);
        state.player.block += actual_block;
        total_block += actual_block;
        game_log!("  → Exhausted {} → gained {} block", card.definition_id, actual_block);
    }
    
    // Move exhausted cards to exhaust pile
    for card in exhausted_cards {
        state.exhaust_pile.push(card);
    }
    
    // Fire on_exhaust power hooks (Feel No Pain, Dark Embrace, etc.)
    // Each exhausted card triggers these hooks individually
    for _ in 0..exhaust_count {
        let power_snap: Vec<(String, i32)> = state.player.powers.iter()
            .map(|(k, v)| (k.clone(), *v))
            .collect();
        for (pid, stacks) in &power_snap {
            let pi = PowerInstance::new(PowerId::from_str(pid), *stacks);
            let effects = pi.on_exhaust();
            if !effects.is_empty() {
                super::commands::apply_hook_effects(state, &effects, pid, None, None);
            }
        }
    }
    
    game_log!("  → Second Wind: Exhausted {} non-Attack cards, gained {} total block", 
        exhaust_count, total_block);
    
    vec![
        CommandResult::BlockGained { amount: total_block },
        CommandResult::CardExhausted,
    ]
}

// ============================================================================
// Wallop (Watcher - Uncommon Attack)
// ============================================================================
// Java: WallopAction.update():
//   target.damage(info);
//   if (target.lastDamageTaken > 0) → GainBlockAction(source, target.lastDamageTaken)
//   baseDamage = 9, upgradeDamage(3) → 12
//   Cost: 2

fn wallop(
    state: &mut GameState,
    upgraded: bool,
    target_idx: Option<usize>,
) -> Vec<CommandResult> {
    use crate::power_hooks::calculate_block_hooked;
    
    let base_damage = if upgraded { 12 } else { 9 };
    let target_enemy = target_idx.unwrap_or(0);
    
    // Calculate damage
    let final_damage = if let Some(enemy) = state.enemies.get(target_enemy) {
        calculate_card_damage(
            base_damage, &state.player.powers, &enemy.powers, state.player.stance,
            build_relic_flags(state),
        )
    } else {
        base_damage
    };
    
    // Deal damage
    let mut killed = false;
    let mut actual = 0;
    let has_boot = state.relics.iter().any(|r| r.id == "Boot" && r.active);
    if target_enemy < state.enemies.len() && !state.enemies[target_enemy].is_dead() {
        let (act, pend) = state.enemies[target_enemy].take_damage_from_player(final_damage, has_boot);
        actual = act;
        if pend > 0 && !state.enemies[target_enemy].is_dead() { state.enemies[target_enemy].block += pend; }
        killed = state.enemies[target_enemy].is_dead();
        fire_on_attacked_hooks(state, target_enemy, actual);
    }
    
    state.record_attack_result(actual, actual, killed);
    
    let mut results = vec![CommandResult::DamageDealt {
        target: format!("Enemy {}", target_enemy),
        amount: actual,
        killed,
    }];
    
    // Java: if (target.lastDamageTaken > 0) → gain block = lastDamageTaken
    // lastDamageTaken = actual unblocked damage dealt (after block reduction)
    if actual > 0 {
        state.player.block += actual;
        game_log!("  → Wallop: Dealt {} damage → gained {} block (player block now {})",
            actual, actual, state.player.block);
        results.push(CommandResult::BlockGained { amount: actual });
    } else {
        game_log!("  → Wallop: Dealt 0 unblocked damage, no block gained");
    }
    
    results
}

// ============================================================================
// Spirit Shield (Watcher - Rare Skill)
// ============================================================================
// Java: SpiritShield.applyPowers():
//   count = hand.size() - 1 (excluding self, which is still in hand during applyPowers)
//   baseBlock = count * magicNumber
//   super.applyPowers() → applies Dex, Frail, etc.
//   Then: GainBlockAction(player, this.block)
//   magicNumber = 3 (base), 4 (upgraded)
//   Cost: 2
//
// NOTE: play_card_from_hand removes the card BEFORE calling override,
// so hand.len() already excludes Spirit Shield itself.

fn spirit_shield(
    state: &mut GameState,
    upgraded: bool,
) -> Vec<CommandResult> {
    use crate::power_hooks::calculate_block_hooked;
    
    let magic_number = if upgraded { 4 } else { 3 };
    
    // Count cards in hand (Spirit Shield already removed by play_card_from_hand)
    let card_count = state.hand.len() as i32;
    
    let base_block = card_count * magic_number;
    
    // Apply block through pipeline (Dex, Frail, etc.)
    let actual_block = calculate_block_hooked(base_block, &state.player.powers);
    state.player.block += actual_block;
    
    game_log!("  → Spirit Shield: {} cards × {} = {} base → {} actual block (player block now {})",
        card_count, magic_number, base_block, actual_block, state.player.block);
    
    vec![CommandResult::BlockGained { amount: actual_block }]
}

// ============================================================================
// Indignation (Watcher - Uncommon Skill)
// ============================================================================
// Java: IndignationAction.update():
//   if (player.stance.ID.equals("Wrath")) →
//     Apply Vulnerable(magicNumber) to ALL enemies
//   else →
//     ChangeStanceAction("Wrath") (no Vulnerable)
//   magicNumber = 3 (base), 5 (upgraded)
//   Cost: 1

fn indignation(
    state: &mut GameState,
    upgraded: bool,
) -> Vec<CommandResult> {
    use crate::core::stances::Stance;
    
    let vuln_amount = if upgraded { 5 } else { 3 };
    
    if state.player.stance == Stance::Wrath {
        // Already in Wrath → apply Vulnerable to ALL enemies
        let mut results = Vec::new();
        for i in 0..state.enemies.len() {
            if state.enemies[i].is_dead() { continue; }
            state.enemies[i].powers.apply("Vulnerable", vuln_amount, None);
            game_log!("  → Indignation (Wrath): Applied Vulnerable({}) to {}",
                vuln_amount, state.enemies[i].name);
            results.push(CommandResult::StatusApplied {
                target: state.enemies[i].name.clone(),
                status: "Vulnerable".to_string(),
                stacks: vuln_amount,
            });
        }
        results
    } else {
        // Not in Wrath → enter Wrath (triggers stance change hooks)
        let old_stance = state.player.stance;
        
        // 1. On-exit effects of old stance
        let exit_energy = old_stance.on_exit_energy();
        if exit_energy > 0 {
            state.player.energy += exit_energy;
            game_log!("  🧘 Indignation: Exiting {} → +{} Energy", old_stance.name(), exit_energy);
        }
        
        // 2. Change stance
        state.player.stance = Stance::Wrath;
        game_log!("  → Indignation: {} → Wrath", old_stance.name());
        
        // 3. On-enter effects
        let enter_energy = Stance::Wrath.on_enter_energy();
        if enter_energy > 0 {
            state.player.energy += enter_energy;
        }
        
        // 4. Trigger on_stance_change hooks (MentalFortress, Rushdown)
        let stance_effects: Vec<_> = state.player.powers.iter()
            .flat_map(|(id_str, &stacks)| {
                use crate::power_hooks::{PowerInstance, PowerId};
                let pi = PowerInstance::new(PowerId::from_str(id_str), stacks);
                let effects = pi.on_stance_change("Wrath");
                let pid = pi.id;
                effects.into_iter().map(move |e| (pid, e)).collect::<Vec<_>>()
            })
            .collect();
        
        for (power_id, effect) in &stance_effects {
            let pid_str = format!("{:?}", power_id);
            crate::engine::commands::apply_hook_effects(state, &[effect.clone()], &pid_str, None, None);
        }
        
        vec![CommandResult::Skipped { reason: "Entered Wrath".to_string() }]
    }
}
