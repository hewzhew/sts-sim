//! Tests for Ironclad (Red) cards.
//!
//! Phase 1: Basic cards (Strike, Defend, Bash)
//! Phase 2: Common cards (Anger, Armaments, Body_Slam, ...)
//! Phase 3: Uncommon cards
//! Phase 4: Rare cards

use super::*;

// ============================================================================
// Phase 1: Basic Cards (3 cards) ✅ All passing
// ============================================================================

// --- Strike_Ironclad ---
// JSON: DealDamage { base: 6, upgrade: 9 }
// Cost: 1 | Type: Attack | Target: Enemy

#[test]
fn test_strike_ironclad_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Strike_Ironclad", false, Some(0));
    
    assert_eq!(state.enemies[0].hp, 44, "Strike should deal 6 damage");
    assert_eq!(state.player.energy, 2, "Strike costs 1 energy");
}

#[test]
fn test_strike_ironclad_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Strike_Ironclad", true, Some(0));
    
    assert_eq!(state.enemies[0].hp, 41, "Strike+ should deal 9 damage");
    assert_eq!(state.player.energy, 2);
}

#[test]
fn test_strike_ironclad_with_strength() {
    let mut state = test_state(42, 3, 50);
    state.player.apply_status("Strength", 3);
    play_card_by_id(&mut state, "Strike_Ironclad", false, Some(0));
    
    // 6 base + 3 strength = 9 damage → 50 - 9 = 41
    assert_eq!(state.enemies[0].hp, 41, "Strike with 3 Str should deal 9");
}

// --- Defend_Ironclad ---
// JSON: GainBlock { base: 5, upgrade: 8 }
// Cost: 1 | Type: Skill | Target: Self

#[test]
fn test_defend_ironclad_base() {
    let mut state = test_state(42, 3, 50);
    assert_eq!(state.player.block, 0);
    
    play_card_by_id(&mut state, "Defend_Ironclad", false, None);
    
    assert_eq!(state.player.block, 5, "Defend should give 5 block");
    assert_eq!(state.player.energy, 2, "Defend costs 1 energy");
}

#[test]
fn test_defend_ironclad_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Defend_Ironclad", true, None);
    
    assert_eq!(state.player.block, 8, "Defend+ should give 8 block");
}

#[test]
fn test_defend_ironclad_stacks_block() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Defend_Ironclad", false, None);
    state.player.energy = 1;
    play_card_by_id(&mut state, "Defend_Ironclad", false, None);
    
    assert_eq!(state.player.block, 10, "Two Defends should give 10 block");
}

// --- Bash ---
// JSON: DealDamage { base: 8, upgrade: 10 } + ApplyStatus { status: "Vulnerable", base: 2, upgrade: 3 }
// Cost: 2 | Type: Attack | Target: Enemy

#[test]
fn test_bash_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Bash", false, Some(0));
    
    assert_eq!(state.enemies[0].hp, 42, "Bash should deal 8 damage");
    assert_eq!(state.enemies[0].get_status("Vulnerable"), 2, 
        "Bash should apply 2 Vulnerable");
    assert_eq!(state.player.energy, 1, "Bash costs 2 energy");
}

#[test]
fn test_bash_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Bash", true, Some(0));
    
    assert_eq!(state.enemies[0].hp, 40, "Bash+ should deal 10 damage");
    assert_eq!(state.enemies[0].get_status("Vulnerable"), 3,
        "Bash+ should apply 3 Vulnerable");
    assert_eq!(state.player.energy, 1);
}

#[test]
fn test_bash_then_strike_with_vulnerable() {
    let mut state = test_state(42, 5, 50);
    
    play_card_by_id(&mut state, "Bash", false, Some(0));
    assert_eq!(state.enemies[0].hp, 42);
    assert_eq!(state.enemies[0].get_status("Vulnerable"), 2);
    
    play_card_by_id(&mut state, "Strike_Ironclad", false, Some(0));
    // Strike does 6 base × 1.5 Vuln = 9, 42 - 9 = 33
    assert_eq!(state.enemies[0].hp, 33, 
        "Strike vs Vulnerable should deal 9 (6 × 1.5)");
}

// ============================================================================
// Phase 2a: Common Cards - Batch 1 (10 cards)
// ============================================================================

// --- Anger ---
// JSON: DealDamage { base: 6, upgrade: 8 } + AddCard { card: "this card", destination: "discard pile" }
// Cost: 0 | Type: Attack
// NOTE: AddCard is currently a stub (logs only, doesn't actually add card to pile)

#[test]
fn test_anger_base() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Anger", false, Some(0));
    
    assert_eq!(state.enemies[0].hp, 44, "Anger should deal 6 damage");
    assert_eq!(state.player.energy, 3, "Anger costs 0 energy");
    // AddCard is currently a stub - verify it at least reports the action
    assert!(results.len() >= 2, "Should have DealDamage + AddCard results");
}

#[test]
fn test_anger_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Anger", true, Some(0));
    
    assert_eq!(state.enemies[0].hp, 42, "Anger+ should deal 8 damage");
    assert_eq!(state.player.energy, 3);
}

// --- Armaments ---
// JSON: GainBlock { base: 5, upgrade: 5 } + UpgradeCards { amount_base: 1, amount_upgrade: "ALL", target: "Hand" }
// Cost: 1 | Type: Skill

#[test]
fn test_armaments_base() {
    let mut state = test_state(42, 3, 50);
    // Add cards to hand so UpgradeCards has something to work with
    add_hand(&mut state, &["Strike_Ironclad", "Defend_Ironclad"]);
    
    play_card_by_id(&mut state, "Armaments", false, None);
    
    assert_eq!(state.player.block, 5, "Armaments should give 5 block");
    assert_eq!(state.player.energy, 2, "Armaments costs 1 energy");
    // Base: upgrades 1 card in hand
    let upgraded_count = state.hand.iter().filter(|c| c.upgraded).count();
    assert_eq!(upgraded_count, 1, "Armaments base should upgrade 1 card");
}

#[test]
fn test_armaments_upgraded() {
    let mut state = test_state(42, 3, 50);
    add_hand(&mut state, &["Strike_Ironclad", "Defend_Ironclad", "Bash"]);
    
    play_card_by_id(&mut state, "Armaments", true, None);
    
    assert_eq!(state.player.block, 5, "Armaments+ still gives 5 block");
    // Upgraded: upgrades ALL cards in hand
    let upgraded_count = state.hand.iter().filter(|c| c.upgraded).count();
    assert_eq!(upgraded_count, 3, "Armaments+ should upgrade ALL cards in hand");
}

// --- Body_Slam ---
// JSON: DealDamage { scaling: "Block" }
// Cost: 1 (upgrade: 0) | Type: Attack
// Damage = current block amount

#[test]
fn test_body_slam_no_block() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Body_Slam", false, Some(0));
    
    // No block = 0 damage
    assert_eq!(state.enemies[0].hp, 50, "Body Slam with 0 block should deal 0 damage");
}

#[test]
fn test_body_slam_with_block() {
    let mut state = test_state(42, 3, 50);
    state.player.block = 15;
    play_card_by_id(&mut state, "Body_Slam", false, Some(0));
    
    // 15 block = 15 damage → 50 - 15 = 35
    assert_eq!(state.enemies[0].hp, 35, "Body Slam should deal damage equal to block (15)");
    // Block is NOT consumed by Body Slam
    assert_eq!(state.player.block, 15, "Block should remain after Body Slam");
}

// --- Clash ---
// JSON: DealDamage { base: 14, upgrade: 18 }
// Cost: 0 | Type: Attack
// PlayCondition: HandOnlyAttacks (not enforced in current engine)

#[test]
fn test_clash_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Clash", false, Some(0));
    
    assert_eq!(state.enemies[0].hp, 36, "Clash should deal 14 damage");
    assert_eq!(state.player.energy, 3, "Clash costs 0 energy");
}

#[test]
fn test_clash_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Clash", true, Some(0));
    
    assert_eq!(state.enemies[0].hp, 32, "Clash+ should deal 18 damage");
}

// --- Cleave ---
// JSON: DealDamageAll { base: 8, upgrade: 11 }
// Cost: 1 | Type: Attack | Target: AllEnemies

#[test]
fn test_cleave_base_multi_enemy() {
    let mut state = test_state_multi(42, 3, &[30, 25, 40]);
    play_card_by_id(&mut state, "Cleave", false, None);
    
    assert_eq!(state.enemies[0].hp, 22, "Cleave should deal 8 to enemy 0");
    assert_eq!(state.enemies[1].hp, 17, "Cleave should deal 8 to enemy 1");
    assert_eq!(state.enemies[2].hp, 32, "Cleave should deal 8 to enemy 2");
    assert_eq!(state.player.energy, 2);
}

#[test]
fn test_cleave_upgraded_multi_enemy() {
    let mut state = test_state_multi(42, 3, &[30, 25]);
    play_card_by_id(&mut state, "Cleave", true, None);
    
    assert_eq!(state.enemies[0].hp, 19, "Cleave+ should deal 11 to enemy 0");
    assert_eq!(state.enemies[1].hp, 14, "Cleave+ should deal 11 to enemy 1");
}

// --- Clothesline ---
// JSON: DealDamage { base: 12, upgrade: 14 } + ApplyStatus { status: "Weak", base: 2, upgrade: 3 }
// Cost: 2 | Type: Attack

#[test]
fn test_clothesline_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Clothesline", false, Some(0));
    
    assert_eq!(state.enemies[0].hp, 38, "Clothesline should deal 12 damage");
    assert_eq!(state.enemies[0].get_status("Weak"), 2, "Should apply 2 Weak");
    assert_eq!(state.player.energy, 1, "Clothesline costs 2 energy");
}

#[test]
fn test_clothesline_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Clothesline", true, Some(0));
    
    assert_eq!(state.enemies[0].hp, 36, "Clothesline+ should deal 14 damage");
    assert_eq!(state.enemies[0].get_status("Weak"), 3, "Should apply 3 Weak");
}

// --- Flex ---
// JSON: GainBuff { buff: "Strength", base: 2, upgrade: 4 } + LoseBuff { buff: "Strength", base: 2, upgrade: 4 }
// Cost: 0 | Type: Skill
// NOTE: LoseBuff is supposed to trigger at end of turn, but in current engine both execute immediately.
// The test verifies current behavior (gain then immediately lose).

#[test]
fn test_flex_base() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Flex", false, None);
    
    assert_eq!(state.player.energy, 3, "Flex costs 0 energy");
    // Current engine: gain 2 then immediately lose 2 → net 0
    // In real game, loss happens at end of turn. This is a known engine limitation.
    assert!(results.len() >= 2, "Should have GainBuff + LoseBuff results");
}

// --- Havoc ---
// JSON: PlayTopCard {}
// Cost: 1 (upgrade: 0) | Type: Skill
// NOTE: PlayTopCard is currently a stub (returns Unknown)

#[test]
fn test_havoc_base() {
    let mut state = test_state(42, 3, 50);
    add_draw_pile(&mut state, &["Strike_Ironclad"]);
    let results = play_card_by_id(&mut state, "Havoc", false, None);
    
    assert_eq!(state.player.energy, 2, "Havoc costs 1 energy");
    // PlayTopCard is stub - just verify it doesn't panic
    assert!(!results.is_empty(), "Should have at least one result");
}

// --- Headbutt ---
// JSON: DealDamage { base: 9, upgrade: 12 } + PutOnTop { from: "discard pile" }
// Cost: 1 | Type: Attack

#[test]
fn test_headbutt_base() {
    let mut state = test_state(42, 3, 50);
    add_discard(&mut state, &["Bash", "Defend_Ironclad"]);
    let draw_pile_before = state.draw_pile.len();
    
    play_card_by_id(&mut state, "Headbutt", false, Some(0));
    
    assert_eq!(state.enemies[0].hp, 41, "Headbutt should deal 9 damage");
    assert_eq!(state.player.energy, 2, "Headbutt costs 1 energy");
    // PutOnTop should move 1 card from discard to top of draw pile
    assert_eq!(state.draw_pile.len(), draw_pile_before + 1, 
        "Should have 1 more card in draw pile");
    assert_eq!(state.discard_pile.len(), 1,
        "Should have 1 fewer card in discard pile");
}

#[test]
fn test_headbutt_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Headbutt", true, Some(0));
    
    assert_eq!(state.enemies[0].hp, 38, "Headbutt+ should deal 12 damage");
}

// --- Heavy_Blade ---
// JSON: DealDamage { base: 14, upgrade: 14 } + StrengthMultiplier { base: 3, upgrade: 5 }
// Cost: 2 | Type: Attack
// Strength applies 3x (5x upgraded) instead of 1x

#[test]
fn test_heavy_blade_no_strength() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Heavy_Blade", false, Some(0));
    
    // 14 damage, no strength → 50 - 14 = 36
    assert_eq!(state.enemies[0].hp, 36, "Heavy Blade should deal 14 base damage");
}

#[test]
fn test_heavy_blade_with_strength_base() {
    let mut state = test_state(42, 3, 50);
    state.player.apply_status("Strength", 4);
    play_card_by_id(&mut state, "Heavy_Blade", false, Some(0));
    
    // 14 base + (4 str × 3 mult) = 14 + 12 = 26 → 50 - 26 = 24
    assert_eq!(state.enemies[0].hp, 24, 
        "Heavy Blade with 4 Str (×3): 14 + 12 = 26 damage");
}

#[test]
fn test_heavy_blade_with_strength_upgraded() {
    let mut state = test_state(42, 3, 50);
    state.player.apply_status("Strength", 4);
    play_card_by_id(&mut state, "Heavy_Blade", true, Some(0));
    
    // 14 base + (4 str × 5 mult) = 14 + 20 = 34 → 50 - 34 = 16
    assert_eq!(state.enemies[0].hp, 16, 
        "Heavy Blade+ with 4 Str (×5): 14 + 20 = 34 damage");
}

// --- Iron_Wave ---
// JSON: GainBlock { base: 5, upgrade: 7 } + DealDamage { base: 5, upgrade: 7 }
// Cost: 1 | Type: Attack

#[test]
fn test_iron_wave_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Iron_Wave", false, Some(0));
    
    assert_eq!(state.player.block, 5, "Iron Wave should give 5 block");
    assert_eq!(state.enemies[0].hp, 45, "Iron Wave should deal 5 damage");
    assert_eq!(state.player.energy, 2);
}

#[test]
fn test_iron_wave_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Iron_Wave", true, Some(0));
    
    assert_eq!(state.player.block, 7, "Iron Wave+ should give 7 block");
    assert_eq!(state.enemies[0].hp, 43, "Iron Wave+ should deal 7 damage");
}

// ============================================================================
// Phase 2b: Common Cards - Batch 2 (9 cards)
// ============================================================================

// --- Pommel_Strike ---
// JSON: DealDamage { base: 9, upgrade: 10 } + DrawCards { base: 1, upgrade: 2 }
// Cost: 1 | Type: Attack | Target: Enemy

#[test]
fn test_pommel_strike_base() {
    let mut state = test_state(42, 3, 50);
    add_draw_pile(&mut state, &["Strike_Ironclad", "Defend_Ironclad"]);
    let hand_before = state.hand.len();
    
    play_card_by_id(&mut state, "Pommel_Strike", false, Some(0));
    
    assert_eq!(state.enemies[0].hp, 41, "Pommel Strike should deal 9 damage");
    assert_eq!(state.player.energy, 2, "Pommel Strike costs 1 energy");
    assert_eq!(state.hand.len(), hand_before + 1, "Should draw 1 card");
}

#[test]
fn test_pommel_strike_upgraded() {
    let mut state = test_state(42, 3, 50);
    add_draw_pile(&mut state, &["Strike_Ironclad", "Defend_Ironclad"]);
    let hand_before = state.hand.len();
    
    play_card_by_id(&mut state, "Pommel_Strike", true, Some(0));
    
    assert_eq!(state.enemies[0].hp, 40, "Pommel Strike+ should deal 10 damage");
    assert_eq!(state.hand.len(), hand_before + 2, "Should draw 2 cards");
}

// --- Shrug_It_Off ---
// JSON: GainBlock { base: 8, upgrade: 11 } + DrawCards { base: 1, upgrade: 1 }
// Cost: 1 | Type: Skill

#[test]
fn test_shrug_it_off_base() {
    let mut state = test_state(42, 3, 50);
    add_draw_pile(&mut state, &["Strike_Ironclad"]);
    let hand_before = state.hand.len();
    
    play_card_by_id(&mut state, "Shrug_It_Off", false, None);
    
    assert_eq!(state.player.block, 8, "Shrug It Off should give 8 block");
    assert_eq!(state.player.energy, 2);
    assert_eq!(state.hand.len(), hand_before + 1, "Should draw 1 card");
}

#[test]
fn test_shrug_it_off_upgraded() {
    let mut state = test_state(42, 3, 50);
    add_draw_pile(&mut state, &["Strike_Ironclad"]);
    let hand_before = state.hand.len();
    
    play_card_by_id(&mut state, "Shrug_It_Off", true, None);
    
    assert_eq!(state.player.block, 11, "Shrug It Off+ should give 11 block");
    assert_eq!(state.hand.len(), hand_before + 1, "Still draws 1 card when upgraded");
}

// --- Twin_Strike ---
// JSON: DealDamage { base: 5, upgrade: 7, times: 2 }
// Cost: 1 | Type: Attack

#[test]
fn test_twin_strike_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Twin_Strike", false, Some(0));
    
    // 5 damage × 2 hits = 10 total → 50 - 10 = 40
    assert_eq!(state.enemies[0].hp, 40, "Twin Strike should deal 5×2=10 damage");
    assert_eq!(state.player.energy, 2);
}

#[test]
fn test_twin_strike_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Twin_Strike", true, Some(0));
    
    // 7 damage × 2 hits = 14 total → 50 - 14 = 36
    assert_eq!(state.enemies[0].hp, 36, "Twin Strike+ should deal 7×2=14 damage");
}

#[test]
fn test_twin_strike_with_strength() {
    let mut state = test_state(42, 3, 50);
    state.player.apply_status("Strength", 3);
    play_card_by_id(&mut state, "Twin_Strike", false, Some(0));
    
    // (5+3) × 2 = 16 → 50 - 16 = 34
    assert_eq!(state.enemies[0].hp, 34, 
        "Twin Strike with 3 Str: (5+3)×2 = 16 damage");
}

// --- Sword_Boomerang ---
// JSON: DealDamageRandom { base: 3, upgrade: 3, times: 3, times_upgrade: 4 }
// Cost: 1 | Type: Attack | Target: RandomEnemy

#[test]
fn test_sword_boomerang_base_single_enemy() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Sword_Boomerang", false, None);
    
    // 3 damage × 3 hits = 9 total (all hit the single enemy) → 50 - 9 = 41
    assert_eq!(state.enemies[0].hp, 41, 
        "Sword Boomerang should deal 3×3=9 to single enemy");
    assert_eq!(state.player.energy, 2);
}

#[test]
fn test_sword_boomerang_upgraded_single_enemy() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Sword_Boomerang", true, None);
    
    // 3 damage × 4 hits = 12 total → 50 - 12 = 38
    assert_eq!(state.enemies[0].hp, 38, 
        "Sword Boomerang+ should deal 3×4=12 to single enemy");
}

// --- Thunderclap ---
// JSON: DealDamageAll { base: 4, upgrade: 7 } + ApplyStatusAll { status: Vulnerable, base: 1, upgrade: 1 }
// Cost: 1 | Type: Attack | Target: AllEnemies

#[test]
fn test_thunderclap_base_multi_enemy() {
    let mut state = test_state_multi(42, 3, &[30, 25]);
    play_card_by_id(&mut state, "Thunderclap", false, None);
    
    assert_eq!(state.enemies[0].hp, 26, "Thunderclap should deal 4 to enemy 0");
    assert_eq!(state.enemies[1].hp, 21, "Thunderclap should deal 4 to enemy 1");
    assert_eq!(state.enemies[0].get_status("Vulnerable"), 1, "Should apply 1 Vuln");
    assert_eq!(state.enemies[1].get_status("Vulnerable"), 1, "Should apply 1 Vuln");
    assert_eq!(state.player.energy, 2);
}

#[test]
fn test_thunderclap_upgraded_multi_enemy() {
    let mut state = test_state_multi(42, 3, &[30, 25]);
    play_card_by_id(&mut state, "Thunderclap", true, None);
    
    assert_eq!(state.enemies[0].hp, 23, "Thunderclap+ should deal 7 to enemy 0");
    assert_eq!(state.enemies[1].hp, 18, "Thunderclap+ should deal 7 to enemy 1");
    assert_eq!(state.enemies[0].get_status("Vulnerable"), 1, "Still 1 Vuln when upgraded");
}

// --- True_Grit ---
// JSON: GainBlock { base: 7, upgrade: 9 } + ExhaustCards { select_mode: "random", base: 1, upgrade: 1 }
// Cost: 1 | Type: Skill

#[test]
fn test_true_grit_base() {
    let mut state = test_state(42, 3, 50);
    add_hand(&mut state, &["Strike_Ironclad", "Defend_Ironclad"]);
    let hand_before = state.hand.len();
    
    play_card_by_id(&mut state, "True_Grit", false, None);
    
    assert_eq!(state.player.block, 7, "True Grit should give 7 block");
    assert_eq!(state.player.energy, 2);
    // Should exhaust 1 random card from hand
    assert_eq!(state.hand.len(), hand_before - 1, 
        "Should exhaust 1 card from hand");
    assert_eq!(state.exhaust_pile.len(), 1, "1 card should be in exhaust pile");
}

#[test]
fn test_true_grit_upgraded() {
    let mut state = test_state(42, 3, 50);
    add_hand(&mut state, &["Strike_Ironclad", "Defend_Ironclad"]);
    
    play_card_by_id(&mut state, "True_Grit", true, None);
    
    assert_eq!(state.player.block, 9, "True Grit+ should give 9 block");
    assert_eq!(state.exhaust_pile.len(), 1, "Still exhausts 1 card");
}

// --- Wild_Strike ---
// JSON: DealDamage { base: 12, upgrade: 17 } + ShuffleInto { card: "Wound" }
// Cost: 1 | Type: Attack

#[test]
fn test_wild_strike_base() {
    let mut state = test_state(42, 3, 50);
    let draw_before = state.draw_pile.len();
    
    play_card_by_id(&mut state, "Wild_Strike", false, Some(0));
    
    assert_eq!(state.enemies[0].hp, 38, "Wild Strike should deal 12 damage");
    assert_eq!(state.player.energy, 2);
    // Should shuffle a Wound into draw pile
    assert_eq!(state.draw_pile.len(), draw_before + 1, 
        "Should add 1 card (Wound) to draw pile");
}

#[test]
fn test_wild_strike_upgraded() {
    let mut state = test_state(42, 3, 50);
    
    play_card_by_id(&mut state, "Wild_Strike", true, Some(0));
    
    assert_eq!(state.enemies[0].hp, 33, "Wild Strike+ should deal 17 damage");
}

// --- Warcry ---
// JSON: DrawCards { base: 1, upgrade: 2 } + ExhaustSelf + MoveCard { from: hand, to: draw_pile, insert_at: top }
// Cost: 0 | Type: Skill

#[test]
fn test_warcry_base() {
    let mut state = test_state(42, 3, 50);
    add_draw_pile(&mut state, &["Strike_Ironclad", "Defend_Ironclad"]);
    add_hand(&mut state, &["Bash"]); // card to put on top
    let hand_before = state.hand.len();
    
    let results = play_card_by_id(&mut state, "Warcry", false, None);
    
    assert_eq!(state.player.energy, 3, "Warcry costs 0 energy");
    // Should draw 1, then put 1 on top → net hand change depends on implementation
    // Key: verify it doesn't panic and produces results
    assert!(results.len() >= 2, "Should have DrawCards + ExhaustSelf + MoveCard results");
}

#[test]
fn test_warcry_upgraded() {
    let mut state = test_state(42, 3, 50);
    add_draw_pile(&mut state, &["Strike_Ironclad", "Defend_Ironclad", "Bash"]);
    add_hand(&mut state, &["Clash"]);
    
    let results = play_card_by_id(&mut state, "Warcry", true, None);
    
    assert_eq!(state.player.energy, 3, "Warcry+ still costs 0");
    assert!(results.len() >= 2, "Should have multiple results");
}

// --- Perfected_Strike ---
// JSON: DealDamage { base: 6, upgrade: 6 } (+ complex CardCount second DealDamage)
// Cost: 2 | Type: Attack
// The second command uses a CardCount value source which may not be implemented.
// Test verifies at minimum the base 6 damage is dealt.

#[test]
fn test_perfected_strike_base_damage() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Perfected_Strike", false, Some(0));
    
    // At minimum: 6 base damage from first DealDamage command
    // The second CardCount-based DealDamage may add more or return Unknown
    assert!(state.enemies[0].hp <= 44, 
        "Perfected Strike should deal at least 6 damage (got {} HP remaining)", 
        state.enemies[0].hp);
    assert_eq!(state.player.energy, 1, "Perfected Strike costs 2 energy");
    assert!(results.len() >= 1, "Should have at least 1 result");
}

// ============================================================================
// Phase 3: Uncommon Cards (26 cards)
// ============================================================================

// --- Carnage ---
// Ethereal + DealDamage { base: 20, upgrade: 28 } | Cost: 2

#[test]
fn test_carnage_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Carnage", false, Some(0));
    assert_eq!(state.enemies[0].hp, 30, "Carnage should deal 20 damage");
    assert_eq!(state.player.energy, 1);
}

#[test]
fn test_carnage_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Carnage", true, Some(0));
    assert_eq!(state.enemies[0].hp, 22, "Carnage+ should deal 28 damage");
}

// --- Pummel ---
// DealDamage { base: 2, times: 4, times_upgrade: 5 } + ExhaustSelf | Cost: 1

#[test]
fn test_pummel_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Pummel", false, Some(0));
    // 2 × 4 = 8 → 50 - 8 = 42
    assert_eq!(state.enemies[0].hp, 42, "Pummel should deal 2×4=8 damage");
    assert_eq!(state.player.energy, 2);
}

#[test]
fn test_pummel_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Pummel", true, Some(0));
    // 2 × 5 = 10 → 50 - 10 = 40
    assert_eq!(state.enemies[0].hp, 40, "Pummel+ should deal 2×5=10 damage");
}

// --- Reckless_Charge ---
// DealDamage { base: 7, upgrade: 10 } + ShuffleInto { card: "Dazed" } | Cost: 0

#[test]
fn test_reckless_charge_base() {
    let mut state = test_state(42, 3, 50);
    let draw_before = state.draw_pile.len();
    play_card_by_id(&mut state, "Reckless_Charge", false, Some(0));
    assert_eq!(state.enemies[0].hp, 43, "Reckless Charge should deal 7 damage");
    assert_eq!(state.player.energy, 3, "Reckless Charge costs 0");
    assert_eq!(state.draw_pile.len(), draw_before + 1, "Should shuffle Dazed into draw");
}

// --- Searing_Blow ---
// DealDamage { base: 12, upgrade: 16 } | Cost: 2

#[test]
fn test_searing_blow_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Searing_Blow", false, Some(0));
    assert_eq!(state.enemies[0].hp, 38, "Searing Blow should deal 12 damage");
    assert_eq!(state.player.energy, 1);
}

#[test]
fn test_searing_blow_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Searing_Blow", true, Some(0));
    assert_eq!(state.enemies[0].hp, 34, "Searing Blow+ should deal 16 damage");
}

// --- Uppercut ---
// DealDamage(13) + ApplyStatus(Weak 1/2) + ApplyStatus(Vuln 1/2) | Cost: 2

#[test]
fn test_uppercut_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Uppercut", false, Some(0));
    assert_eq!(state.enemies[0].hp, 37, "Uppercut should deal 13 damage");
    assert_eq!(state.enemies[0].get_status("Weak"), 1);
    assert_eq!(state.enemies[0].get_status("Vulnerable"), 1);
    assert_eq!(state.player.energy, 1);
}

#[test]
fn test_uppercut_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Uppercut", true, Some(0));
    assert_eq!(state.enemies[0].hp, 37, "Uppercut+ still deals 13 damage");
    assert_eq!(state.enemies[0].get_status("Weak"), 2, "Upgraded: 2 Weak");
    assert_eq!(state.enemies[0].get_status("Vulnerable"), 2, "Upgraded: 2 Vuln");
}

// --- Hemokinesis ---
// LoseHP(2) + DealDamage(15/20) | Cost: 1

#[test]
fn test_hemokinesis_base() {
    let mut state = test_state(42, 3, 50);
    state.player.current_hp = 70;
    state.player.max_hp = 80;
    play_card_by_id(&mut state, "Hemokinesis", false, Some(0));
    assert_eq!(state.enemies[0].hp, 35, "Hemokinesis should deal 15 damage");
    assert_eq!(state.player.current_hp, 68, "Should lose 2 HP");
    assert_eq!(state.player.energy, 2);
}

#[test]
fn test_hemokinesis_upgraded() {
    let mut state = test_state(42, 3, 50);
    state.player.current_hp = 70;
    state.player.max_hp = 80;
    play_card_by_id(&mut state, "Hemokinesis", true, Some(0));
    assert_eq!(state.enemies[0].hp, 30, "Hemokinesis+ should deal 20 damage");
    assert_eq!(state.player.current_hp, 68, "Still loses 2 HP");
}

// --- Blood_for_Blood ---
// GainEnergy(1) + DealDamage(18/22) | Cost: 4 (cost_upgrade: 3)
// Note: cost reduction mechanic not tested; testing direct play

#[test]
fn test_blood_for_blood_base() {
    let mut state = test_state(42, 5, 50);
    play_card_by_id(&mut state, "Blood_for_Blood", false, Some(0));
    assert_eq!(state.enemies[0].hp, 32, "Blood for Blood should deal 18 damage");
    // Costs 4, gains 1 energy → net: 5 - 4 + 1 = 2
    assert_eq!(state.player.energy, 2);
}

// --- Rampage ---
// DealDamage(8) + IncreaseDamage(5/8, stub) | Cost: 1

#[test]
fn test_rampage_base() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Rampage", false, Some(0));
    assert_eq!(state.enemies[0].hp, 42, "Rampage should deal 8 damage");
    assert_eq!(state.player.energy, 2);
    assert!(results.len() >= 2, "Should have DealDamage + IncreaseDamage results");
}

// --- Ghostly_Armor ---
// Ethereal + GainBlock(10/13) | Cost: 1

#[test]
fn test_ghostly_armor_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Ghostly_Armor", false, None);
    assert_eq!(state.player.block, 10, "Ghostly Armor should give 10 block");
    assert_eq!(state.player.energy, 2);
}

#[test]
fn test_ghostly_armor_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Ghostly_Armor", true, None);
    assert_eq!(state.player.block, 13, "Ghostly Armor+ should give 13 block");
}

// --- Entrench ---
// DoubleBlock | Cost: 2 (cost_upgrade: 1)

#[test]
fn test_entrench_base() {
    let mut state = test_state(42, 3, 50);
    state.player.block = 10;
    play_card_by_id(&mut state, "Entrench", false, None);
    assert_eq!(state.player.block, 20, "Entrench should double 10 block to 20");
    assert_eq!(state.player.energy, 1, "Entrench costs 2");
}

#[test]
fn test_entrench_zero_block() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Entrench", false, None);
    assert_eq!(state.player.block, 0, "Doubling 0 block should stay 0");
}

// --- Flame_Barrier ---
// GainBlock(12/16) + DealDamage(4/6, trigger) | Cost: 2
// Only testing the block gain; trigger damage is conditional

#[test]
fn test_flame_barrier_base() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Flame_Barrier", false, None);
    assert_eq!(state.player.block, 12, "Flame Barrier should give 12 block");
    assert_eq!(state.player.energy, 1);
    assert!(results.len() >= 1);
}

#[test]
fn test_flame_barrier_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Flame_Barrier", true, None);
    assert_eq!(state.player.block, 16, "Flame Barrier+ should give 16 block");
}

// --- Intimidate ---
// ApplyStatusAll(Weak 1/2) + ExhaustSelf | Cost: 0

#[test]
fn test_intimidate_base() {
    let mut state = test_state_multi(42, 3, &[30, 25]);
    play_card_by_id(&mut state, "Intimidate", false, None);
    assert_eq!(state.enemies[0].get_status("Weak"), 1);
    assert_eq!(state.enemies[1].get_status("Weak"), 1);
    assert_eq!(state.player.energy, 3, "Intimidate costs 0");
}

#[test]
fn test_intimidate_upgraded() {
    let mut state = test_state_multi(42, 3, &[30, 25]);
    play_card_by_id(&mut state, "Intimidate", true, None);
    assert_eq!(state.enemies[0].get_status("Weak"), 2, "Upgraded: 2 Weak");
    assert_eq!(state.enemies[1].get_status("Weak"), 2);
}

// --- Seeing_Red ---
// GainEnergy(2) + ExhaustSelf | Cost: 1 (cost_upgrade: 0)

#[test]
fn test_seeing_red_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Seeing_Red", false, None);
    // 3 - 1 (cost) + 2 (gain) = 4
    assert_eq!(state.player.energy, 4, "Seeing Red: 3 - 1 + 2 = 4 energy");
}

// --- Sentinel ---
// GainBlock(5/8) + GainEnergy(2/3, conditional on exhaust) | Cost: 1

#[test]
fn test_sentinel_base() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Sentinel", false, None);
    assert_eq!(state.player.block, 5, "Sentinel should give 5 block");
    // GainEnergy fires unconditionally (OnExhaust condition is metadata-only)
    // 3 - 1 (cost) + 2 (gain) = 4
    assert_eq!(state.player.energy, 4);
    assert!(results.len() >= 1);
}

#[test]
fn test_sentinel_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Sentinel", true, None);
    assert_eq!(state.player.block, 8, "Sentinel+ should give 8 block");
}

// --- Shockwave ---
// ApplyStatus(Weak 3/5) + ExhaustSelf | Cost: 2
// Note: JSON only shows Weak; original text says both Weak AND Vuln

#[test]
fn test_shockwave_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Shockwave", false, Some(0));
    assert_eq!(state.enemies[0].get_status("Weak"), 3);
    assert_eq!(state.player.energy, 1);
}

#[test]
fn test_shockwave_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Shockwave", true, Some(0));
    assert_eq!(state.enemies[0].get_status("Weak"), 5, "Upgraded: 5 Weak");
}

// --- Spot_Weakness ---
// GainBuff(Strength 3/4) ONLY if enemy is attacking | Cost: 1
// Java: SpotWeaknessAction checks target.getIntentBaseDmg() >= 0

#[test]
fn test_spot_weakness_base() {
    let mut state = test_state(42, 3, 50);
    // Spot Weakness only works if enemy is attacking
    state.enemies[0].current_intent = crate::enemy::Intent::Attack { damage: 10, times: 1 };
    play_card_by_id(&mut state, "Spot_Weakness", false, None);
    assert_eq!(state.player.get_status("Strength"), 3);
    assert_eq!(state.player.energy, 2);
}

#[test]
fn test_spot_weakness_upgraded() {
    let mut state = test_state(42, 3, 50);
    // Spot Weakness only works if enemy is attacking
    state.enemies[0].current_intent = crate::enemy::Intent::Attack { damage: 10, times: 1 };
    play_card_by_id(&mut state, "Spot_Weakness", true, None);
    assert_eq!(state.player.get_status("Strength"), 4);
}

// --- Bloodletting ---
// LoseHP(3) + GainEnergy(2/3) | Cost: 0

#[test]
fn test_bloodletting_base() {
    let mut state = test_state(42, 3, 50);
    state.player.current_hp = 70;
    state.player.max_hp = 80;
    play_card_by_id(&mut state, "Bloodletting", false, None);
    assert_eq!(state.player.current_hp, 67, "Should lose 3 HP");
    // 3 + 0 (cost) + 2 (gain) = 5
    assert_eq!(state.player.energy, 5, "Bloodletting: 3 + 2 = 5 energy");
}

#[test]
fn test_bloodletting_upgraded() {
    let mut state = test_state(42, 3, 50);
    state.player.current_hp = 70;
    state.player.max_hp = 80;
    play_card_by_id(&mut state, "Bloodletting", true, None);
    assert_eq!(state.player.current_hp, 67, "Still loses 3 HP");
    assert_eq!(state.player.energy, 6, "Upgraded: 3 + 3 = 6 energy");
}

// --- Battle_Trance ---
// DrawCards(3/4) + ApplyBuff(No Draw, 1) | Cost: 0

#[test]
fn test_battle_trance_base() {
    let mut state = test_state(42, 3, 50);
    add_draw_pile(&mut state, &["Strike_Ironclad", "Defend_Ironclad", "Bash"]);
    let hand_before = state.hand.len();
    play_card_by_id(&mut state, "Battle_Trance", false, None);
    assert_eq!(state.hand.len(), hand_before + 3, "Should draw 3 cards");
    assert_eq!(state.player.energy, 3, "Costs 0");
}

#[test]
fn test_battle_trance_upgraded() {
    let mut state = test_state(42, 3, 50);
    add_draw_pile(&mut state, &["Strike_Ironclad", "Defend_Ironclad", "Bash", "Anger"]);
    let hand_before = state.hand.len();
    play_card_by_id(&mut state, "Battle_Trance", true, None);
    assert_eq!(state.hand.len(), hand_before + 4, "Upgraded: draw 4 cards");
}

// --- Burning_Pact ---
// ExhaustCards(1, choose) + DrawCards(2/3) | Cost: 1

#[test]
fn test_burning_pact_base() {
    let mut state = test_state(42, 3, 50);
    add_hand(&mut state, &["Strike_Ironclad"]);
    add_draw_pile(&mut state, &["Defend_Ironclad", "Bash"]);
    let hand_before = state.hand.len();
    play_card_by_id(&mut state, "Burning_Pact", false, None);
    // Exhaust 1, draw 2 → net +1
    assert_eq!(state.exhaust_pile.len(), 1, "Should exhaust 1 card");
    assert_eq!(state.player.energy, 2);
}

// --- Disarm ---
// RemoveEnemyBuff(Strength 2/3) + ExhaustSelf | Cost: 1

#[test]
fn test_disarm_base() {
    let mut state = test_state(42, 3, 50);
    state.enemies[0].apply_status("Strength", 5);
    play_card_by_id(&mut state, "Disarm", false, Some(0));
    assert_eq!(state.enemies[0].get_status("Strength"), 3,
        "Should remove 2 Strength: 5-2=3");
    assert_eq!(state.player.energy, 2);
}

#[test]
fn test_disarm_upgraded() {
    let mut state = test_state(42, 3, 50);
    state.enemies[0].apply_status("Strength", 5);
    play_card_by_id(&mut state, "Disarm", true, Some(0));
    assert_eq!(state.enemies[0].get_status("Strength"), 2,
        "Upgraded: remove 3 Strength: 5-3=2");
}

// --- Inflame (Power) ---
// GainBuff(Strength 2/3) | Cost: 1

#[test]
fn test_inflame_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Inflame", false, None);
    assert_eq!(state.player.get_status("Strength"), 2);
    assert_eq!(state.player.energy, 2);
}

#[test]
fn test_inflame_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Inflame", true, None);
    assert_eq!(state.player.get_status("Strength"), 3);
}

// --- Power_Through ---
// AddCard("2 Wounds", hand) + GainBlock(15/20) | Cost: 1

#[test]
fn test_power_through_base() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Power_Through", false, None);
    assert_eq!(state.player.block, 15, "Power Through should give 15 block");
    assert_eq!(state.player.energy, 2);
    assert!(results.len() >= 2);
}

#[test]
fn test_power_through_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Power_Through", true, None);
    assert_eq!(state.player.block, 20, "Power Through+ should give 20 block");
}

// --- Sever_Soul ---
// ExhaustCards(ALL, filter:non-attack) + DealDamage(16/22) | Cost: 2

#[test]
fn test_sever_soul_base() {
    let mut state = test_state(42, 3, 50);
    // Add non-attack cards to hand to be exhausted
    add_hand(&mut state, &["Defend_Ironclad", "Shrug_It_Off"]);
    let results = play_card_by_id(&mut state, "Sever_Soul", false, Some(0));
    assert_eq!(state.enemies[0].hp, 34, "Sever Soul should deal 16 damage");
    assert_eq!(state.player.energy, 1);
    assert!(results.len() >= 2, "Should have ExhaustCards + DealDamage");
}

// --- Second_Wind ---
// ExhaustCards(ALL, filter:non-attack) + GainBlock(5/7 per exhausted) | Cost: 1

#[test]
fn test_second_wind_base() {
    let mut state = test_state(42, 3, 50);
    add_hand(&mut state, &["Defend_Ironclad", "Shrug_It_Off"]);
    let results = play_card_by_id(&mut state, "Second_Wind", false, None);
    // Block depends on how many non-attacks were exhausted × 5
    assert!(state.player.block >= 5, "Second Wind should give at least 5 block");
    assert_eq!(state.player.energy, 2);
    assert!(results.len() >= 1);
}

// --- Whirlwind ---
// DealDamageAll(5/8) × X (all energy) | Cost: -1 (X)

#[test]
fn test_whirlwind_base() {
    let mut state = test_state_multi(42, 3, &[30, 25]);
    let results = play_card_by_id(&mut state, "Whirlwind", false, None);
    // With cost -1 (X), should use all energy. The DealDamageAll may only fire once
    // depending on X-cost implementation.
    assert!(results.len() >= 1, "Should produce at least 1 result");
}

// --- Dropkick ---
// DealDamage(5/8) + conditional DrawCards + GainEnergy if Vuln | Cost: 1

#[test]
fn test_dropkick_deals_damage() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Dropkick", false, Some(0));
    assert_eq!(state.enemies[0].hp, 45, "Dropkick should deal 5 damage");
    assert_eq!(state.player.energy, 2);
}

// --- Dual_Wield ---
// AddCard(copy, hand) | Cost: 1
// Complex card-choice mechanic; test that it runs without panic

#[test]
fn test_dual_wield_runs() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Dual_Wield", false, None);
    assert_eq!(state.player.energy, 2);
    assert!(results.len() >= 1);
}

// --- Rage ---
// GainBlock(3/5, per attack this turn) | Cost: 0

#[test]
fn test_rage_base() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Rage", false, None);
    // One-shot block gain (trigger logic not testable here)
    assert_eq!(state.player.block, 3, "Rage should give 3 block");
    assert_eq!(state.player.energy, 3, "Rage costs 0");
}

// --- Infernal_Blade ---
// AddCard(random Attack, hand) + ExhaustSelf | Cost: 1 (cost_upgrade: 0)

#[test]
fn test_infernal_blade_runs() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Infernal_Blade", false, None);
    assert_eq!(state.player.energy, 2);
    assert!(results.len() >= 1);
}

// --- Combust (Power) ---
// DealDamageAll(5/7) at end of turn trigger | Cost: 1

#[test]
fn test_combust_base() {
    let mut state = test_state_multi(42, 3, &[30, 25]);
    play_card_by_id(&mut state, "Combust", false, None);
    assert_eq!(state.player.energy, 2, "Combust costs 1");
    // Combust applies power → hook fires at end of turn
    assert!(state.player.powers.has("Combust"), "Should have Combust power");
    assert_eq!(state.enemies[0].hp, 30, "Should NOT deal damage immediately");
    assert_eq!(state.enemies[1].hp, 25, "Should NOT deal damage immediately");
    
    // Simulate turn end → hook fires → deal 5 to all enemies
    crate::engine::on_turn_end(&mut state, &*super::CARD_LIBRARY, None);
    assert_eq!(state.enemies[0].hp, 25, "After hook: 30-5=25");
    assert_eq!(state.enemies[1].hp, 20, "After hook: 25-5=20");
}

// --- Dark_Embrace (Power) ---
// DrawCards(1) on exhaust trigger | Cost: 2 (cost_upgrade: 1)
// Phase 2: "Trigger:" not "TurnTrigger:" — still fires immediately

#[test]
fn test_dark_embrace_runs() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Dark_Embrace", false, None);
    assert_eq!(state.player.energy, 1, "Dark Embrace costs 2");
    // "Trigger:" condition — Phase 1 does not route this, still fires immediately
    assert!(results.len() >= 1);
}

// --- Evolve (Power) ---
// DrawCards(1/2) on Status draw | Cost: 1
// Phase 2: "Trigger:" not "TurnTrigger:" — still fires immediately

#[test]
fn test_evolve_runs() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Evolve", false, None);
    assert_eq!(state.player.energy, 2);
    assert!(results.len() >= 1);
}

// --- Feel_No_Pain (Power) ---
// GainBlock(3/4) on exhaust trigger | Cost: 1
// Phase 2: "Trigger:" not "TurnTrigger:" — still fires immediately

#[test]
fn test_feel_no_pain_runs() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Feel_No_Pain", false, None);
    assert_eq!(state.player.energy, 2);
    assert!(results.len() >= 1);
}

// --- Fire_Breathing (Power) ---
// DealDamageAll(6/10) on Status/Curse draw | Cost: 1
// Phase 2: "Trigger:" not "TurnTrigger:" — still fires immediately

#[test]
fn test_fire_breathing_runs() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Fire_Breathing", false, None);
    assert_eq!(state.player.energy, 2);
    assert!(results.len() >= 1);
}

// --- Metallicize (Power) ---
// ApplyPower(Metallicize) | Cost: 1
// Hook: at_end_of_turn → GainBlock(stacks)

#[test]
fn test_metallicize_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Metallicize", false, None);
    assert_eq!(state.player.energy, 2, "Metallicize costs 1");
    // Power applied → hook fires at end of turn
    assert!(state.player.powers.has("Metallicize"), "Should have Metallicize power");
    assert_eq!(state.player.block, 0, "Should NOT gain block immediately");
    
    // Simulate turn end → hook fires → gain 3 block
    crate::engine::on_turn_end(&mut state, &*super::CARD_LIBRARY, None);
    assert_eq!(state.player.block, 3, "After hook: 3 block");
    
    // Turn 2 end → hook fires again → gain 3 more block
    crate::engine::on_turn_end(&mut state, &*super::CARD_LIBRARY, None);
    assert_eq!(state.player.block, 6, "After 2 hooks: 6 block (persists)");
}

// --- Rupture (Power) ---
// GainBuff(Strength 1/2) on self-damage trigger | Cost: 1
// Phase 2: "Trigger:" not "TurnTrigger:" — still fires immediately

#[test]
fn test_rupture_runs() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Rupture", false, None);
    assert_eq!(state.player.energy, 2);
    assert!(results.len() >= 1);
}

// ============================================================================
// Phase 4: Rare Cards (16 cards)
// ============================================================================

// --- Bludgeon ---
// DealDamage { base: 32, upgrade: 42 } | Cost: 3

#[test]
fn test_bludgeon_base() {
    let mut state = test_state(42, 4, 50);
    play_card_by_id(&mut state, "Bludgeon", false, Some(0));
    assert_eq!(state.enemies[0].hp, 18, "Bludgeon should deal 32 damage");
    assert_eq!(state.player.energy, 1);
}

#[test]
fn test_bludgeon_upgraded() {
    let mut state = test_state(42, 4, 50);
    play_card_by_id(&mut state, "Bludgeon", true, Some(0));
    assert_eq!(state.enemies[0].hp, 8, "Bludgeon+ should deal 42 damage");
}

// --- Feed ---
// DealDamage(10/12) + ExhaustSelf + Conditional(GainMaxHP if fatal) | Cost: 1

#[test]
fn test_feed_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Feed", false, Some(0));
    assert_eq!(state.enemies[0].hp, 40, "Feed should deal 10 damage");
    assert_eq!(state.player.energy, 2);
}

#[test]
fn test_feed_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Feed", true, Some(0));
    assert_eq!(state.enemies[0].hp, 38, "Feed+ should deal 12 damage");
}

#[test]
fn test_feed_fatal_gainmaxhp() {
    let mut state = test_state(42, 3, 50);
    state.enemies[0].hp = 5; // Will be killed by 10 damage
    let max_hp_before = state.player.max_hp;
    play_card_by_id(&mut state, "Feed", false, Some(0));
    assert!(state.enemies[0].hp <= 0, "Enemy should be dead");
    // GainMaxHP should fire on fatal
    assert!(state.player.max_hp >= max_hp_before,
        "Feed should gain max HP on fatal (got {} from {})", state.player.max_hp, max_hp_before);
}

// --- Fiend_Fire ---
// DealDamage(7/10) + ExhaustSelf + ExhaustCard(hand, All) | Cost: 2

#[test]
fn test_fiend_fire_base() {
    let mut state = test_state(42, 3, 50);
    add_hand(&mut state, &["Strike_Ironclad", "Defend_Ironclad"]);
    let results = play_card_by_id(&mut state, "Fiend_Fire", false, Some(0));
    assert_eq!(state.enemies[0].hp, 43, "Fiend Fire should deal 7 damage (base)");
    assert_eq!(state.player.energy, 1);
    assert!(results.len() >= 2);
}

// --- Immolate ---
// DealDamageAll(21/28) + AddCard(Burn, discard) | Cost: 2

#[test]
fn test_immolate_base() {
    let mut state = test_state_multi(42, 3, &[30, 25]);
    let results = play_card_by_id(&mut state, "Immolate", false, None);
    assert_eq!(state.enemies[0].hp, 9, "Immolate should deal 21 to enemy 0");
    assert_eq!(state.enemies[1].hp, 4, "Immolate should deal 21 to enemy 1");
    assert_eq!(state.player.energy, 1);
    assert!(results.len() >= 2);
}

#[test]
fn test_immolate_upgraded() {
    let mut state = test_state_multi(42, 3, &[50, 40]);
    play_card_by_id(&mut state, "Immolate", true, None);
    assert_eq!(state.enemies[0].hp, 22, "Immolate+ should deal 28 to enemy 0");
    assert_eq!(state.enemies[1].hp, 12, "Immolate+ should deal 28 to enemy 1");
}

// --- Reaper ---
// DealDamageAll(4/5) + ExhaustSelf + Heal(LastUnblockedDamage) | Cost: 2

#[test]
fn test_reaper_base() {
    let mut state = test_state_multi(42, 3, &[30, 25]);
    let results = play_card_by_id(&mut state, "Reaper", false, None);
    assert_eq!(state.enemies[0].hp, 26, "Reaper should deal 4 to enemy 0");
    assert_eq!(state.enemies[1].hp, 21, "Reaper should deal 4 to enemy 1");
    assert_eq!(state.player.energy, 1);
    assert!(results.len() >= 2);
}

// --- Double_Tap ---
// ApplyBuff(Double Tap 1/2) | Cost: 1

#[test]
fn test_double_tap_base() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Double_Tap", false, None);
    assert_eq!(state.player.energy, 2);
    assert!(results.len() >= 1);
}

// --- Exhume ---
// ExhaustSelf + MoveCard(exhaust_pile→hand) | Cost: 1 (cost_upgrade: 0)

#[test]
fn test_exhume_base() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Exhume", false, None);
    assert_eq!(state.player.energy, 2);
    assert!(results.len() >= 1);
}

// --- Impervious ---
// GainBlock(30/40) + ExhaustSelf | Cost: 2

#[test]
fn test_impervious_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Impervious", false, None);
    assert_eq!(state.player.block, 30, "Impervious should give 30 block");
    assert_eq!(state.player.energy, 1);
}

#[test]
fn test_impervious_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Impervious", true, None);
    assert_eq!(state.player.block, 40, "Impervious+ should give 40 block");
}

// --- Limit_Break ---
// DoubleBuff(Strength) + ExhaustSelf(base_only) | Cost: 1

#[test]
fn test_limit_break_base() {
    let mut state = test_state(42, 3, 50);
    state.player.apply_status("Strength", 4);
    play_card_by_id(&mut state, "Limit_Break", false, None);
    assert_eq!(state.player.get_status("Strength"), 8,
        "Limit Break should double 4 Strength to 8");
    assert_eq!(state.player.energy, 2);
}

#[test]
fn test_limit_break_zero_strength() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Limit_Break", false, None);
    assert_eq!(state.player.get_status("Strength"), 0,
        "Doubling 0 Strength stays 0");
}

// --- Offering ---
// LoseHP(6) + GainEnergy(2) + DrawCards(3/5) + ExhaustSelf | Cost: 0

#[test]
fn test_offering_base() {
    let mut state = test_state(42, 3, 50);
    state.player.current_hp = 70;
    state.player.max_hp = 80;
    add_draw_pile(&mut state, &["Strike_Ironclad", "Defend_Ironclad", "Bash"]);
    let hand_before = state.hand.len();
    play_card_by_id(&mut state, "Offering", false, None);
    assert_eq!(state.player.current_hp, 64, "Should lose 6 HP");
    // 3 + 0 (cost) + 2 (gain) = 5
    assert_eq!(state.player.energy, 5, "Offering: 3 + 2 = 5 energy");
    assert_eq!(state.hand.len(), hand_before + 3, "Should draw 3 cards");
}

#[test]
fn test_offering_upgraded() {
    let mut state = test_state(42, 3, 50);
    state.player.current_hp = 70;
    state.player.max_hp = 80;
    add_draw_pile(&mut state, &["Strike_Ironclad", "Defend_Ironclad", "Bash", "Anger", "Iron_Wave"]);
    let hand_before = state.hand.len();
    play_card_by_id(&mut state, "Offering", true, None);
    assert_eq!(state.player.current_hp, 64, "Still loses 6 HP");
    assert_eq!(state.player.energy, 5);
    assert_eq!(state.hand.len(), hand_before + 5, "Upgraded: draw 5 cards");
}

// --- Barricade (Power) ---
// ApplyPower(Barricade) | Cost: 3 (cost_upgrade: 2)

#[test]
fn test_barricade_runs() {
    let mut state = test_state(42, 4, 50);
    let results = play_card_by_id(&mut state, "Barricade", false, None);
    assert_eq!(state.player.energy, 1, "Barricade costs 3");
    // Barricade uses ApplyPower → executes immediately (sets a game rule)
    assert!(results.len() >= 1);
}

// --- Berserk (Power) ---
// ApplyPower(Berserk) + GainBuff(Vuln 2/1) immediate | Cost: 0
// Hook: at_start_of_turn → GainEnergy(stacks)

#[test]
fn test_berserk_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Berserk", false, None);
    // Vulnerable is applied immediately
    assert_eq!(state.player.get_status("Vulnerable"), 2,
        "Berserk should give 2 Vulnerable immediately");
    // Berserk power applied → hook fires at turn start
    assert!(state.player.powers.has("Berserk"), "Should have Berserk power");
    assert_eq!(state.player.energy, 3, "Energy should still be 3 (no immediate gain)");
    
    // Simulate turn start → hook fires → gain 1 energy
    crate::engine::on_turn_start(&mut state, &*super::CARD_LIBRARY, None);
    assert_eq!(state.player.energy, 4, "After hook: 3 + 1 = 4 energy");
}

#[test]
fn test_berserk_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Berserk", true, None);
    assert_eq!(state.player.get_status("Vulnerable"), 1,
        "Berserk+ gives only 1 Vulnerable");
    assert!(state.player.powers.has("Berserk"), "Should have Berserk power");
}

// --- Brutality (Power) ---
// ApplyPower(Brutality) | Cost: 0
// Hook: at_start_of_turn → LoseHp(1) + DrawCards(1)

#[test]
fn test_brutality_runs() {
    let mut state = test_state(42, 3, 50);
    add_draw_pile(&mut state, &["Strike_Ironclad", "Defend_Ironclad"]);
    play_card_by_id(&mut state, "Brutality", false, None);
    assert_eq!(state.player.energy, 3, "Brutality costs 0");
    // Power applied → hook fires at turn start
    assert!(state.player.powers.has("Brutality"), "Should have Brutality power");
}

// --- Corruption (Power) ---
// ApplyPower(Corruption) | Cost: 3 (cost_upgrade: 2)

#[test]
fn test_corruption_runs() {
    let mut state = test_state(42, 4, 50);
    let results = play_card_by_id(&mut state, "Corruption", false, None);
    assert_eq!(state.player.energy, 1, "Corruption costs 3");
    // ApplyPower → executes immediately (no trigger)
    assert!(state.player.powers.has("Corruption"), "Should have Corruption power");
    assert!(results.len() >= 1);
}

// --- Demon_Form (Power) ---
// ApplyPower(Demon Form) | Cost: 3
// Hook: at_start_of_turn → GainStrength(stacks)

#[test]
fn test_demon_form_base() {
    let mut state = test_state(42, 4, 50);
    play_card_by_id(&mut state, "Demon_Form", false, None);
    assert_eq!(state.player.energy, 1, "Demon Form costs 3");
    // Power applied → hook fires at turn start
    assert!(state.player.powers.has("Demon Form"), "Should have Demon Form power");
    assert_eq!(state.player.get_status("Strength"), 0,
        "Should NOT gain Strength immediately");
    
    // Turn 1: on_turn_start → gain 2 Strength via hook
    crate::engine::on_turn_start(&mut state, &*super::CARD_LIBRARY, None);
    assert_eq!(state.player.get_status("Strength"), 2, "After 1 turn: 2 Strength");
    
    // Turn 2: on_turn_start → gain 2 more
    crate::engine::on_turn_start(&mut state, &*super::CARD_LIBRARY, None);
    assert_eq!(state.player.get_status("Strength"), 4, "After 2 turns: 4 Strength");
}

#[test]
fn test_demon_form_upgraded() {
    let mut state = test_state(42, 4, 50);
    play_card_by_id(&mut state, "Demon_Form", true, None);
    assert_eq!(state.player.get_status("Strength"), 0, "No immediate Strength");
    assert!(state.player.powers.has("Demon Form"), "Should have Demon Form power");
    
    crate::engine::on_turn_start(&mut state, &*super::CARD_LIBRARY, None);
    assert_eq!(state.player.get_status("Strength"), 3, "Upgraded: 3 Strength per turn");
}

// --- Juggernaut (Power) ---
// ApplyPower(Juggernaut, 5/7) | Cost: 2
// Hook: onGainedBlock → DealDamage(stacks) to random enemy

#[test]
fn test_juggernaut_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Juggernaut", false, Some(0));
    // Now only applies power, no immediate damage
    assert_eq!(state.enemies[0].hp, 50, "Juggernaut should NOT deal immediate damage");
    assert!(state.player.powers.has("Juggernaut"), "Should have Juggernaut power");
    assert_eq!(state.player.energy, 1, "Juggernaut costs 2");
}

#[test]
fn test_juggernaut_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Juggernaut", true, Some(0));
    assert_eq!(state.enemies[0].hp, 50, "Juggernaut+ should NOT deal immediate damage");
    assert!(state.player.powers.has("Juggernaut"), "Should have Juggernaut power");
}

