//! Tests for Colorless cards (Status, Uncommon, Rare, Special, Curse).

use super::*;

// ============================================================================
// Status Cards (5) — most are Unplayable
// ============================================================================

// Slimed is the only playable Status card (cost 1, ExhaustSelf)
#[test]
fn test_slimed_base() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Slimed", false, None);
    assert_eq!(state.player.energy, 2, "Slimed costs 1");
    assert!(results.len() >= 1);
}

// ============================================================================
// Uncommon Attacks (4)
// ============================================================================

// --- Dramatic_Entrance ---
// Innate + DealDamageAll(8/12) + ExhaustSelf | Cost: 0

#[test]
fn test_dramatic_entrance_base() {
    let mut state = test_state_multi(42, 3, &[30, 25]);
    play_card_by_id(&mut state, "Dramatic_Entrance", false, None);
    assert_eq!(state.enemies[0].hp, 22, "Dramatic Entrance: 30-8=22");
    assert_eq!(state.enemies[1].hp, 17, "Dramatic Entrance: 25-8=17");
    assert_eq!(state.player.energy, 3, "Costs 0");
}

#[test]
fn test_dramatic_entrance_upgraded() {
    let mut state = test_state_multi(42, 3, &[30, 25]);
    play_card_by_id(&mut state, "Dramatic_Entrance", true, None);
    assert_eq!(state.enemies[0].hp, 18, "Upgraded: 30-12=18");
    assert_eq!(state.enemies[1].hp, 13, "Upgraded: 25-12=13");
}

// --- Flash_of_Steel ---
// DealDamage(3/6) + DrawCards(1) | Cost: 0

#[test]
fn test_flash_of_steel_base() {
    let mut state = test_state(42, 3, 50);
    add_draw_pile(&mut state, &["Strike_Ironclad"]);
    let hand_before = state.hand.len();
    play_card_by_id(&mut state, "Flash_of_Steel", false, Some(0));
    assert_eq!(state.enemies[0].hp, 47, "Flash of Steel: 50-3=47");
    assert_eq!(state.hand.len(), hand_before + 1, "Should draw 1");
    assert_eq!(state.player.energy, 3, "Costs 0");
}

// --- Mind_Blast ---
// Innate + DealDamage(scaling: draw pile) | Cost: 2/1

#[test]
fn test_mind_blast_runs() {
    let mut state = test_state(42, 3, 50);
    add_draw_pile(&mut state, &["Strike_Ironclad", "Defend_Ironclad"]);
    let results = play_card_by_id(&mut state, "Mind_Blast", false, Some(0));
    assert_eq!(state.player.energy, 1, "Mind Blast costs 2");
    assert!(results.len() >= 1);
}

// --- Swift_Strike ---
// DealDamage(7/10) | Cost: 0

#[test]
fn test_swift_strike_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Swift_Strike", false, Some(0));
    assert_eq!(state.enemies[0].hp, 43, "Swift Strike: 50-7=43");
    assert_eq!(state.player.energy, 3, "Costs 0");
}

#[test]
fn test_swift_strike_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Swift_Strike", true, Some(0));
    assert_eq!(state.enemies[0].hp, 40, "Upgraded: 50-10=40");
}

// ============================================================================
// Uncommon Skills (16)
// ============================================================================

// --- Bandage_Up ---
// Heal(4/6) + ExhaustSelf | Cost: 0

#[test]
fn test_bandage_up_base() {
    let mut state = test_state(42, 3, 50);
    state.player.current_hp = 30;
    state.player.max_hp = 80;
    play_card_by_id(&mut state, "Bandage_Up", false, None);
    assert_eq!(state.player.current_hp, 34, "Heal 4 HP: 30+4=34");
    assert_eq!(state.player.energy, 3, "Costs 0");
}

#[test]
fn test_bandage_up_upgraded() {
    let mut state = test_state(42, 3, 50);
    state.player.current_hp = 30;
    state.player.max_hp = 80;
    play_card_by_id(&mut state, "Bandage_Up", true, None);
    assert_eq!(state.player.current_hp, 36, "Upgraded: heal 6 HP");
}

// --- Blind ---
// ApplyStatus(Weak 2) | Cost: 0

#[test]
fn test_blind_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Blind", false, Some(0));
    assert_eq!(state.enemies[0].get_status("Weak"), 2);
    assert_eq!(state.player.energy, 3, "Costs 0");
}

// --- Dark_Shackles ---
// RemoveEnemyBuff(Strength 9/15) + ExhaustSelf | Cost: 0

#[test]
fn test_dark_shackles_base() {
    let mut state = test_state(42, 3, 50);
    state.enemies[0].apply_status("Strength", 20);
    play_card_by_id(&mut state, "Dark_Shackles", false, Some(0));
    assert_eq!(state.enemies[0].get_status("Strength"), 11,
        "Should remove 9 Strength: 20-9=11");
    assert_eq!(state.player.energy, 3, "Costs 0");
}

#[test]
fn test_dark_shackles_upgraded() {
    let mut state = test_state(42, 3, 50);
    state.enemies[0].apply_status("Strength", 20);
    play_card_by_id(&mut state, "Dark_Shackles", true, Some(0));
    assert_eq!(state.enemies[0].get_status("Strength"), 5,
        "Upgraded: remove 15 Strength: 20-15=5");
}

// --- Deep_Breath ---
// ShuffleInto(discard pile) + Draw(1/2) | Cost: 0

#[test]
fn test_deep_breath_runs() {
    let mut state = test_state(42, 3, 50);
    add_draw_pile(&mut state, &["Strike_Ironclad"]);
    let results = play_card_by_id(&mut state, "Deep_Breath", false, None);
    assert_eq!(state.player.energy, 3, "Costs 0");
    assert!(results.len() >= 1);
}

// --- Discovery ---
// ExhaustSelf(base_only) + Discover(1 of 3) | Cost: 1

#[test]
fn test_discovery_runs() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Discovery", false, None);
    assert_eq!(state.player.energy, 2, "Discovery costs 1");
    assert!(results.len() >= 1);
}

// --- Finesse ---
// GainBlock(2/4) + DrawCards(1) | Cost: 0

#[test]
fn test_finesse_base() {
    let mut state = test_state(42, 3, 50);
    add_draw_pile(&mut state, &["Strike_Ironclad"]);
    let hand_before = state.hand.len();
    play_card_by_id(&mut state, "Finesse", false, None);
    assert_eq!(state.player.block, 2, "Finesse: 2 block");
    assert_eq!(state.hand.len(), hand_before + 1, "Draw 1");
    assert_eq!(state.player.energy, 3, "Costs 0");
}

#[test]
fn test_finesse_upgraded() {
    let mut state = test_state(42, 3, 50);
    add_draw_pile(&mut state, &["Strike_Ironclad"]);
    play_card_by_id(&mut state, "Finesse", true, None);
    assert_eq!(state.player.block, 4, "Upgraded: 4 block");
}

// --- Forethought ---
// MoveCard(hand→draw bottom) | Cost: 0

#[test]
fn test_forethought_runs() {
    let mut state = test_state(42, 3, 50);
    add_hand(&mut state, &["Strike_Ironclad"]);
    let results = play_card_by_id(&mut state, "Forethought", false, None);
    assert_eq!(state.player.energy, 3, "Costs 0");
    assert!(results.len() >= 1);
}

// --- Good_Instincts ---
// GainBlock(6/9) | Cost: 0

#[test]
fn test_good_instincts_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Good_Instincts", false, None);
    assert_eq!(state.player.block, 6, "Good Instincts: 6 block");
    assert_eq!(state.player.energy, 3, "Costs 0");
}

#[test]
fn test_good_instincts_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Good_Instincts", true, None);
    assert_eq!(state.player.block, 9, "Upgraded: 9 block");
}

// --- Impatience ---
// DrawCards(2/3) | Cost: 0

#[test]
fn test_impatience_base() {
    let mut state = test_state(42, 3, 50);
    add_draw_pile(&mut state, &["Strike_Ironclad", "Defend_Ironclad"]);
    let hand_before = state.hand.len();
    play_card_by_id(&mut state, "Impatience", false, None);
    assert_eq!(state.hand.len(), hand_before + 2, "Draw 2 cards");
    assert_eq!(state.player.energy, 3, "Costs 0");
}

// --- Jack_of_All_Trades ---
// AddCard(random Colorless, hand) + ExhaustSelf | Cost: 0

#[test]
fn test_jack_of_all_trades_runs() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Jack_of_All_Trades", false, None);
    assert_eq!(state.player.energy, 3, "Costs 0");
    assert!(results.len() >= 1);
}

// --- Madness ---
// ExhaustSelf + SetCostRandom(hand, 0) | Cost: 1/0

#[test]
fn test_madness_runs() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Madness", false, None);
    assert_eq!(state.player.energy, 2, "Madness costs 1");
    assert!(results.len() >= 1);
}

// --- Panacea ---
// GainBuff(Artifact 1/2) + ExhaustSelf | Cost: 0

#[test]
fn test_panacea_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Panacea", false, None);
    assert_eq!(state.player.get_status("Artifact"), 1);
    assert_eq!(state.player.energy, 3, "Costs 0");
}

#[test]
fn test_panacea_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Panacea", true, None);
    assert_eq!(state.player.get_status("Artifact"), 2, "Upgraded: 2 Artifact");
}

// --- Panic_Button ---
// GainBlock(30/40) + ExhaustSelf + ApplyDebuff(No Block, 2) | Cost: 0

#[test]
fn test_panic_button_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Panic_Button", false, None);
    assert_eq!(state.player.block, 30, "Panic Button: 30 block");
    assert_eq!(state.player.energy, 3, "Costs 0");
}

#[test]
fn test_panic_button_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Panic_Button", true, None);
    assert_eq!(state.player.block, 40, "Upgraded: 40 block");
}

// --- Purity ---
// ExhaustSelf + ExhaustCard(hand, choose_up_to 3/5) | Cost: 0

#[test]
fn test_purity_runs() {
    let mut state = test_state(42, 3, 50);
    add_hand(&mut state, &["Strike_Ironclad", "Defend_Ironclad"]);
    let results = play_card_by_id(&mut state, "Purity", false, None);
    assert_eq!(state.player.energy, 3, "Costs 0");
    assert!(results.len() >= 1);
}

// --- Trip ---
// ApplyStatus(Vulnerable 2) | Cost: 0

#[test]
fn test_trip_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Trip", false, Some(0));
    assert_eq!(state.enemies[0].get_status("Vulnerable"), 2);
    assert_eq!(state.player.energy, 3, "Costs 0");
}

// ============================================================================
// Rare Cards (15)
// ============================================================================

// --- Hand_of_Greed ---
// DealDamage(20/25) + Conditional(GainGold if fatal) | Cost: 2

#[test]
fn test_hand_of_greed_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Hand_of_Greed", false, Some(0));
    assert_eq!(state.enemies[0].hp, 30, "Hand of Greed: 50-20=30");
    assert_eq!(state.player.energy, 1, "Costs 2");
}

#[test]
fn test_hand_of_greed_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Hand_of_Greed", true, Some(0));
    assert_eq!(state.enemies[0].hp, 25, "Upgraded: 50-25=25");
}

// --- Apotheosis ---
// UpgradeCards(ALL) + ExhaustSelf | Cost: 2/1

#[test]
fn test_apotheosis_runs() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Apotheosis", false, None);
    assert_eq!(state.player.energy, 1, "Costs 2");
    assert!(results.len() >= 1);
}

// --- Chrysalis ---
// ShuffleInto(3/5 random Skills) + ExhaustSelf | Cost: 2

#[test]
fn test_chrysalis_runs() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Chrysalis", false, None);
    assert_eq!(state.player.energy, 1, "Costs 2");
    assert!(results.len() >= 1);
}

// --- Master_of_Strategy ---
// DrawCards(3/4) + ExhaustSelf | Cost: 0

#[test]
fn test_master_of_strategy_base() {
    let mut state = test_state(42, 3, 50);
    add_draw_pile(&mut state, &["Strike_Ironclad", "Defend_Ironclad", "Bash"]);
    let hand_before = state.hand.len();
    play_card_by_id(&mut state, "Master_of_Strategy", false, None);
    assert_eq!(state.hand.len(), hand_before + 3, "Draw 3 cards");
    assert_eq!(state.player.energy, 3, "Costs 0");
}

// --- Metamorphosis ---
// ShuffleInto(3/5 random Attacks) + ExhaustSelf | Cost: 2

#[test]
fn test_metamorphosis_runs() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Metamorphosis", false, None);
    assert_eq!(state.player.energy, 1, "Costs 2");
    assert!(results.len() >= 1);
}

// --- Secret_Technique ---
// ExhaustSelf(base_only) + MoveCard(draw→hand, Skill) | Cost: 0

#[test]
fn test_secret_technique_runs() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Secret_Technique", false, None);
    assert_eq!(state.player.energy, 3, "Costs 0");
    assert!(results.len() >= 1);
}

// --- Secret_Weapon ---
// ExhaustSelf(base_only) + MoveCard(draw→hand, Attack) | Cost: 0

#[test]
fn test_secret_weapon_runs() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Secret_Weapon", false, None);
    assert_eq!(state.player.energy, 3, "Costs 0");
    assert!(results.len() >= 1);
}

// --- The_Bomb ---
// DealDamageAll(40/50) [delayed 3 turns] | Cost: 2

#[test]
fn test_the_bomb_runs() {
    let mut state = test_state_multi(42, 3, &[60, 50]);
    let results = play_card_by_id(&mut state, "The_Bomb", false, None);
    assert_eq!(state.player.energy, 1, "Costs 2");
    assert!(results.len() >= 1);
}

// --- Thinking_Ahead ---
// DrawCards(2) + PutOnTop(hand→draw) + ExhaustSelf(base_only) | Cost: 0

#[test]
fn test_thinking_ahead_runs() {
    let mut state = test_state(42, 3, 50);
    add_draw_pile(&mut state, &["Strike_Ironclad", "Defend_Ironclad"]);
    let results = play_card_by_id(&mut state, "Thinking_Ahead", false, None);
    assert_eq!(state.player.energy, 3, "Costs 0");
    assert!(results.len() >= 1);
}

// --- Transmutation ---
// AddCard(X random Colorless, hand) + ExhaustSelf | Cost: -1 (X)

#[test]
fn test_transmutation_runs() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Transmutation", false, None);
    assert!(results.len() >= 1);
}

// --- Violence ---
// ExhaustSelf + MoveCard(draw→hand, random Attack 3/4) | Cost: 0

#[test]
fn test_violence_runs() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Violence", false, None);
    assert_eq!(state.player.energy, 3, "Costs 0");
    assert!(results.len() >= 1);
}

// --- Magnetism (Power) ---
// AddCard(random Colorless, hand, trigger) | Cost: 2/1

#[test]
fn test_magnetism_runs() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Magnetism", false, None);
    assert_eq!(state.player.energy, 1, "Costs 2");
    assert!(results.len() >= 1);
}

// --- Mayhem (Power) ---
// PlayTopCard (trigger) | Cost: 2/1

#[test]
fn test_mayhem_runs() {
    let mut state = test_state(42, 3, 50);
    add_draw_pile(&mut state, &["Strike_Ironclad"]);
    let results = play_card_by_id(&mut state, "Mayhem", false, None);
    assert_eq!(state.player.energy, 1, "Costs 2");
    assert!(results.len() >= 1);
}

// --- Panache (Power) ---
// DealDamageAll(10/14) per 5 cards | Cost: 0

#[test]
fn test_panache_runs() {
    let mut state = test_state_multi(42, 3, &[30, 25]);
    let results = play_card_by_id(&mut state, "Panache", false, None);
    assert_eq!(state.player.energy, 3, "Costs 0");
    assert!(results.len() >= 1);
}

// --- Sadistic_Nature (Power) ---
// ApplyPower(Sadistic Nature 5/7) | Cost: 0

#[test]
fn test_sadistic_nature_runs() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Sadistic_Nature", false, None);
    assert_eq!(state.player.energy, 3, "Costs 0");
    assert!(results.len() >= 1);
}

// ============================================================================
// Special Cards (16)
// ============================================================================

// --- Bite ---
// DealDamage(7/8) + Heal(2/3) | Cost: 1

#[test]
fn test_bite_base() {
    let mut state = test_state(42, 3, 50);
    state.player.current_hp = 30;
    state.player.max_hp = 80;
    play_card_by_id(&mut state, "Bite", false, Some(0));
    assert_eq!(state.enemies[0].hp, 43, "Bite: 50-7=43");
    assert_eq!(state.player.current_hp, 32, "Heal 2: 30+2=32");
    assert_eq!(state.player.energy, 2, "Costs 1");
}

#[test]
fn test_bite_upgraded() {
    let mut state = test_state(42, 3, 50);
    state.player.current_hp = 30;
    state.player.max_hp = 80;
    play_card_by_id(&mut state, "Bite", true, Some(0));
    assert_eq!(state.enemies[0].hp, 42, "Upgraded: 50-8=42");
    assert_eq!(state.player.current_hp, 33, "Upgraded heal: 30+3=33");
}

// --- Expunger ---
// DealDamage(9/15) [X times] | Cost: 1

#[test]
fn test_expunger_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Expunger", false, Some(0));
    assert_eq!(state.enemies[0].hp, 41, "Expunger: 50-9=41");
    assert_eq!(state.player.energy, 2, "Costs 1");
}

// --- Ritual_Dagger ---
// DealDamage(15) + IncreaseDamage(3/5, permanent) + ExhaustSelf | Cost: 1

#[test]
fn test_ritual_dagger_base() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Ritual_Dagger", false, Some(0));
    assert_eq!(state.enemies[0].hp, 35, "Ritual Dagger: 50-15=35");
    assert_eq!(state.player.energy, 2, "Costs 1");
    assert!(results.len() >= 2);
}

// --- Shiv ---
// DealDamage(4/6) + ExhaustSelf | Cost: 0

#[test]
fn test_shiv_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Shiv", false, Some(0));
    assert_eq!(state.enemies[0].hp, 46, "Shiv: 50-4=46");
    assert_eq!(state.player.energy, 3, "Costs 0");
}

#[test]
fn test_shiv_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Shiv", true, Some(0));
    assert_eq!(state.enemies[0].hp, 44, "Upgraded: 50-6=44");
}

// --- Smite ---
// Retain + DealDamage(12/16) + ExhaustSelf | Cost: 1

#[test]
fn test_smite_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Smite", false, Some(0));
    assert_eq!(state.enemies[0].hp, 38, "Smite: 50-12=38");
    assert_eq!(state.player.energy, 2, "Costs 1");
}

#[test]
fn test_smite_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Smite", true, Some(0));
    assert_eq!(state.enemies[0].hp, 34, "Upgraded: 50-16=34");
}

// --- Through_Violence ---
// Retain + DealDamage(20/30) + ExhaustSelf | Cost: 0

#[test]
fn test_through_violence_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Through_Violence", false, Some(0));
    assert_eq!(state.enemies[0].hp, 30, "Through Violence: 50-20=30");
    assert_eq!(state.player.energy, 3, "Costs 0");
}

#[test]
fn test_through_violence_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Through_Violence", true, Some(0));
    assert_eq!(state.enemies[0].hp, 20, "Upgraded: 50-30=20");
}

// --- Apparition ---
// Ethereal + GainBuff(Intangible 1) + ExhaustSelf | Cost: 1

#[test]
fn test_apparition_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Apparition", false, None);
    assert_eq!(state.player.get_status("Intangible"), 1);
    assert_eq!(state.player.energy, 2, "Costs 1");
}

// --- Beta ---
// ShuffleInto(Omega) + ExhaustSelf | Cost: 2/1

#[test]
fn test_beta_base() {
    let mut state = test_state(42, 3, 50);
    let draw_before = state.draw_pile.len();
    play_card_by_id(&mut state, "Beta", false, None);
    assert_eq!(state.draw_pile.len(), draw_before + 1, "Should shuffle Omega in");
    assert_eq!(state.player.energy, 1, "Costs 2");
}

// --- Insight ---
// Retain + DrawCards(2/3) + ExhaustSelf | Cost: 0

#[test]
fn test_insight_base() {
    let mut state = test_state(42, 3, 50);
    add_draw_pile(&mut state, &["Strike_Ironclad", "Defend_Ironclad"]);
    let hand_before = state.hand.len();
    play_card_by_id(&mut state, "Insight", false, None);
    assert_eq!(state.hand.len(), hand_before + 2, "Draw 2 cards");
    assert_eq!(state.player.energy, 3, "Costs 0");
}

// --- J.A.X. ---
// LoseHP(3) + GainBuff(Strength 2/3) | Cost: 0

#[test]
fn test_jax_base() {
    let mut state = test_state(42, 3, 50);
    state.player.current_hp = 70;
    state.player.max_hp = 80;
    play_card_by_id(&mut state, "J.A.X.", false, None);
    assert_eq!(state.player.current_hp, 67, "Lose 3 HP");
    assert_eq!(state.player.get_status("Strength"), 2);
    assert_eq!(state.player.energy, 3, "Costs 0");
}

#[test]
fn test_jax_upgraded() {
    let mut state = test_state(42, 3, 50);
    state.player.current_hp = 70;
    state.player.max_hp = 80;
    play_card_by_id(&mut state, "J.A.X.", true, None);
    assert_eq!(state.player.current_hp, 67, "Still lose 3 HP");
    assert_eq!(state.player.get_status("Strength"), 3, "Upgraded: 3 Str");
}

// --- Miracle ---
// Retain + GainEnergy(1/2) + ExhaustSelf | Cost: 0

#[test]
fn test_miracle_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Miracle", false, None);
    assert_eq!(state.player.energy, 4, "3 + 0 + 1 = 4 energy");
}

#[test]
fn test_miracle_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Miracle", true, None);
    assert_eq!(state.player.energy, 5, "Upgraded: 3 + 0 + 2 = 5 energy");
}

// --- Safety ---
// Retain + GainBlock(12/16) + ExhaustSelf | Cost: 1

#[test]
fn test_safety_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Safety", false, None);
    assert_eq!(state.player.block, 12, "Safety: 12 block");
    assert_eq!(state.player.energy, 2, "Costs 1");
}

#[test]
fn test_safety_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Safety", true, None);
    assert_eq!(state.player.block, 16, "Upgraded: 16 block");
}

// --- Become_Almighty (Power) ---
// GainBuff(Strength 3/4) | Cost: -2 (auto)

#[test]
fn test_become_almighty_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Become_Almighty", false, None);
    assert_eq!(state.player.get_status("Strength"), 3);
}

#[test]
fn test_become_almighty_upgraded() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Become_Almighty", true, None);
    assert_eq!(state.player.get_status("Strength"), 4);
}

// --- Live_Forever (Power) ---
// GainBuff(Plated 6/8) | Cost: -2 (auto)

#[test]
fn test_live_forever_base() {
    let mut state = test_state(42, 3, 50);
    play_card_by_id(&mut state, "Live_Forever", false, None);
    assert_eq!(state.player.get_status("Plated"), 6);
}

// --- Omega (Power) ---
// DealDamageAll(50/60) trigger | Cost: 3

#[test]
fn test_omega_runs() {
    let mut state = test_state(42, 4, 50);
    let results = play_card_by_id(&mut state, "Omega", false, None);
    assert_eq!(state.player.energy, 1, "Omega costs 3");
    assert!(results.len() >= 1);
}

// ============================================================================
// Curse Cards (14) — all Unplayable except Pride (cost 1)
// ============================================================================

// Pride is the only playable Curse (cost 1, Innate + ExhaustSelf)
#[test]
fn test_pride_base() {
    let mut state = test_state(42, 3, 50);
    let results = play_card_by_id(&mut state, "Pride", false, None);
    assert_eq!(state.player.energy, 2, "Pride costs 1");
    assert!(results.len() >= 1);
}
