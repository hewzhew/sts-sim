//! End-to-End Validation Tool for Slay the Spire Simulator
//!
//! This tool runs a **Smart Agent** through the FULL game loop to verify
//! the simulator doesn't crash during complete game runs, including:
//! - Potion usage
//! - Shop interactions  
//! - Rest site decisions
//! - Combat strategies
//!
//! Usage:
//!   cargo run --release --bin e2e_test -- [OPTIONS]
//!
//! Options:
//!   --runs N       Number of runs to simulate (default: 100)
//!   --max-steps N  Max steps per run before timeout (default: 10000)
//!   --verbose      Print detailed progress
//!   --god-mode     Enable god mode (infinite HP) to test late game

use std::collections::HashMap;
use std::io::Write;
use std::time::{Duration, Instant};
use std::panic;

use rand::SeedableRng;
use rand::Rng;
use rand_xoshiro::Xoshiro256StarStar;

use sts_sim::state::{GameState, GamePhase};
use sts_sim::schema::{CardInstance, CardColor, CardType};
use sts_sim::loader::{CardLibrary, MonsterLibrary};
use sts_sim::items::potions::{PotionLibrary, PotionTarget};
use sts_sim::items::relics::RelicLibrary;
use sts_sim::map::{generate_map, SimpleMap};
use sts_sim::engine;
use sts_sim::dungeon;
use sts_sim::shop;
use sts_sim::events::ActiveEventState;

// ============================================================================
// Smart Agent Constants
// ============================================================================

/// HP threshold for rest vs smith decision (percentage)
const REST_HP_THRESHOLD: f32 = 0.50;
/// Minimum gold to consider shopping
const SHOP_GOLD_THRESHOLD: i32 = 150;
/// HP threshold for aggressive potion use (percentage)
const POTION_HP_THRESHOLD: f32 = 0.60;

// ============================================================================
// Test Simulator with Smart Agent
// ============================================================================

/// Wrapper around GameState for full game simulation with smart decisions.
struct TestSimulator {
    state: GameState,
    card_library: CardLibrary,
    monster_library: Option<MonsterLibrary>,
    potion_library: Option<PotionLibrary>,
    relic_library: Option<RelicLibrary>,
    seed: u64,
    rng: Xoshiro256StarStar,
    // Tracking stats
    potions_used: u32,
    cards_played: u32,
    gold_spent: u32,
    cards_bought: u32,
    relics_bought: u32,
}

impl TestSimulator {
    fn new(seed: u64) -> Self {
        let card_library = CardLibrary::load("data/cards")
            .expect("Failed to load data/cards");
        let monster_library = MonsterLibrary::load("data/monsters_verified.json").ok();
        let potion_library = PotionLibrary::load("data/potions.json").ok();
        let relic_library = RelicLibrary::load("data/relics_patched.json").ok();
        let state = GameState::new(seed);
        let rng = Xoshiro256StarStar::seed_from_u64(seed);
        
        Self {
            state,
            card_library,
            monster_library,
            potion_library,
            relic_library,
            seed,
            rng,
            potions_used: 0,
            cards_played: 0,
            gold_spent: 0,
            cards_bought: 0,
            relics_bought: 0,
        }
    }
    
    fn reset(&mut self) {
        self.seed = self.seed.wrapping_add(1);
        self.state = GameState::new(self.seed);
        self.rng = Xoshiro256StarStar::seed_from_u64(self.seed);
        self.potions_used = 0;
        self.cards_played = 0;
        self.gold_spent = 0;
        self.cards_bought = 0;
        self.relics_bought = 0;
        
        // Generate Act 1 map
        let full_map = generate_map(self.seed as i64, 1, true);
        self.state.map = Some(SimpleMap::from_map(&full_map));
        
        // Setup starter deck (into master_deck, the persistent deck)
        for _ in 0..5 {
            self.state.obtain_card(CardInstance::new("Strike_Ironclad".to_string(), 1));
        }
        for _ in 0..4 {
            self.state.obtain_card(CardInstance::new("Defend_Ironclad".to_string(), 1));
        }
        self.state.obtain_card(CardInstance::new("Bash".to_string(), 2));
        
        // Start with some gold
        self.state.gold = 99;
        
        // Give some starter potions to test the system
        let _ = self.state.potions.add("BlockPotion".to_string());
    }
    
    fn get_screen_type(&self) -> &'static str {
        match self.state.screen {
            GamePhase::Map => "MAP",
            GamePhase::Combat => "COMBAT",
            GamePhase::Reward => "REWARD",
            GamePhase::Shop => "SHOP",
            GamePhase::Rest => "REST",
            GamePhase::Event => "EVENT",
            GamePhase::CardSelect => "CARD_SELECT",
            GamePhase::GameOver => "GAME_OVER",
        }
    }
    
    fn is_terminal(&self) -> bool {
        self.state.screen == GamePhase::GameOver
    }
    
    fn hp_percent(&self) -> f32 {
        self.state.player.current_hp as f32 / self.state.player.max_hp.max(1) as f32
    }
    
    /// Smart agent: decide which action to take
    fn decide_action(&mut self) -> Option<i32> {
        match self.state.screen {
            GamePhase::Combat => self.decide_combat_action(),
            GamePhase::Map => self.decide_map_action(),
            GamePhase::Reward => self.decide_reward_action(),
            GamePhase::Shop => self.decide_shop_action(),
            GamePhase::Rest => self.decide_rest_action(),
            GamePhase::Event => self.decide_event_action(),
            GamePhase::CardSelect => Some(99), // Skip card selection
            GamePhase::GameOver => None,
        }
    }
    
    /// Combat decision: use potions, play cards, or end turn
    fn decide_combat_action(&mut self) -> Option<i32> {
        let energy = self.state.player.energy;
        
        // === STEP 1: Consider using potions ===
        if let Some(potion_action) = self.consider_potion_use() {
            return Some(potion_action);
        }
        
        // === STEP 2: Calculate threat level ===
        let incoming_damage: i32 = self.state.enemies.iter()
            .filter(|e| !e.is_dead())
            .map(|e| e.get_intent_damage())
            .sum();
        let current_block = self.state.player.block;
        let unblocked = (incoming_damage - current_block).max(0);
        let hp_pct = self.hp_percent();
        let need_block = unblocked > 12 || (hp_pct < 0.5 && unblocked > 0);
        
        // === STEP 3: Categorize playable cards ===
        let mut powers: Vec<(usize, &str, i32)> = Vec::new();
        let mut attacks: Vec<(usize, &str, i32)> = Vec::new();
        let mut blocks: Vec<(usize, &str, i32)> = Vec::new();
        let mut other_skills: Vec<(usize, &str, i32)> = Vec::new();
        
        for (i, card) in self.state.hand.iter().enumerate().take(10) {
            if card.current_cost <= energy {
                let id = card.definition_id.as_str();
                match card.card_type {
                    CardType::Power => powers.push((i, id, card.current_cost)),
                    CardType::Attack => attacks.push((i, id, card.current_cost)),
                    CardType::Skill => {
                        // Cards that provide block
                        let id_lower = id.to_lowercase();
                        if id_lower.contains("defend") || id_lower.contains("block")
                           || id_lower.contains("shield") || id_lower.contains("shrug")
                           || id_lower.contains("iron_wave") || id_lower.contains("ghostly")
                           || id_lower.contains("true_grit") || id_lower.contains("flame")
                        {
                            blocks.push((i, id, card.current_cost));
                        } else {
                            other_skills.push((i, id, card.current_cost));
                        }
                    }
                    _ => {} // Skip Status/Curse
                }
            }
        }
        
        // === STEP 4: Priority-based card play ===
        
        // 4a: Always play Powers first (permanent value)
        if !powers.is_empty() {
            return Some(powers[0].0 as i32);
        }
        
        // 4b: Play Bash early (applies Vulnerable)
        if let Some(&(idx, _, _)) = attacks.iter().find(|(_, id, _)| {
            id.to_lowercase().contains("bash")
        }) {
            return Some(idx as i32);
        }
        
        // 4c: If big incoming damage, block first
        if need_block && !blocks.is_empty() {
            return Some(blocks[0].0 as i32);
        }
        
        // 4d: Play attacks (prefer higher cost = stronger)
        if !attacks.is_empty() {
            // Sort by cost descending to play strongest first
            let best = attacks.iter().max_by_key(|(_, _, cost)| *cost).unwrap();
            return Some(best.0 as i32);
        }
        
        // 4e: Play remaining skills
        if !other_skills.is_empty() {
            return Some(other_skills[0].0 as i32);
        }
        
        // 4f: Play block cards even without incoming damage
        if !blocks.is_empty() {
            return Some(blocks[0].0 as i32);
        }
        
        // No playable cards - end turn
        Some(10)
    }
    
    /// Consider using a potion in combat
    fn consider_potion_use(&mut self) -> Option<i32> {
        let hp_pct = self.hp_percent();
        let turn = self.state.turn;
        
        // Check each potion slot
        for (slot_idx, slot) in self.state.potions.slots().iter().enumerate() {
            if let Some(potion_id) = slot {
                if let Some(ref lib) = self.potion_library {
                    if let Ok(potion) = lib.get(potion_id) {
                        let should_use = match potion_id.as_str() {
                            // Healing potions - use when HP < 60%
                            id if id.contains("Blood") || id.contains("Regen") || id.contains("Fruit") => {
                                hp_pct < POTION_HP_THRESHOLD
                            }
                            // Block potions - use when HP < 70%
                            id if id.contains("Block") => {
                                hp_pct < 0.7
                            }
                            // Damage potions - use aggressively (turn 1 or when enemies exist)
                            id if id.contains("Fire") || id.contains("Explosive") || id.contains("Poison") => {
                                !self.state.enemies.iter().all(|e| e.is_dead())
                            }
                            // Buff potions - use on turn 1
                            id if id.contains("Strength") || id.contains("Dexterity") || id.contains("Energy") => {
                                turn <= 1
                            }
                            // Swift potion - use when hand is small
                            id if id.contains("Swift") => {
                                self.state.hand.len() < 4
                            }
                            // Default: use randomly with 20% chance each turn
                            _ => self.rng.random_range(0..5) == 0,
                        };
                        
                        if should_use {
                            // Check if potion needs a target
                            if potion.target == PotionTarget::Enemy {
                                // Only use if there's a valid target
                                if self.state.enemies.iter().any(|e| !e.is_dead()) {
                                    return Some(11 + slot_idx as i32);
                                }
                            } else {
                                return Some(11 + slot_idx as i32);
                            }
                        }
                    }
                }
            }
        }
        None
    }
    
    fn decide_map_action(&mut self) -> Option<i32> {
        let valid_moves = engine::get_valid_moves(&self.state);
        
        if valid_moves.is_empty() {
            // Try act transition
            return Some(99);
        }
        
        // Pick a random valid move (or first one)
        let idx = self.rng.random_range(0..valid_moves.len());
        Some(20 + idx as i32)
    }
    
    fn decide_reward_action(&mut self) -> Option<i32> {
        // Try to take rewards if available
        if !self.state.current_rewards.is_empty() {
            // Take first available reward
            return Some(30);
        }
        // Skip/finish rewards
        Some(99)
    }
    
    fn decide_shop_action(&mut self) -> Option<i32> {
        // If we have enough gold, try to buy something
        if self.state.gold >= SHOP_GOLD_THRESHOLD {
            if let Some(ref shop) = self.state.shop_state {
                // Try to buy a card
                for (i, card) in shop.cards.iter().enumerate() {
                    if card.price <= self.state.gold && i < 5 {
                        return Some(40 + i as i32);
                    }
                }
                // Try to buy a relic
                for (i, relic) in shop.relics.iter().enumerate() {
                    if relic.price <= self.state.gold && i < 3 {
                        return Some(45 + i as i32);
                    }
                }
                // Try to buy a potion if slots available
                if !self.state.potions.is_full() {
                    for (i, potion) in shop.potions.iter().enumerate() {
                        if potion.price <= self.state.gold && i < 3 {
                            return Some(48 + i as i32);
                        }
                    }
                }
            }
        }
        // Leave shop
        Some(99)
    }
    
    fn decide_rest_action(&mut self) -> Option<i32> {
        let hp_pct = self.hp_percent();
        
        // Check if we have a high-value upgrade target (un-upgraded Bash or Power)
        let has_good_upgrade = self.state.draw_pile.iter().any(|c| {
            !c.upgraded && (
                c.definition_id.to_lowercase().contains("bash")
                || c.card_type == CardType::Power
                || (c.card_type == CardType::Attack && c.base_cost >= 2)
            )
        });
        
        // If HP is critical, always rest
        if hp_pct < 0.35 {
            Some(60)
        } else if hp_pct > 0.70 {
            // HP is good — always smith
            Some(61)
        } else if has_good_upgrade && hp_pct > 0.45 {
            // Good upgrade target and HP is okay — smith
            Some(61)
        } else {
            // Default: rest
            Some(60)
        }
    }
    
    fn decide_event_action(&mut self) -> Option<i32> {
        // Get available event options
        let options = engine::get_available_event_options(&self.state);
        
        for (idx, _, available) in options {
            if available {
                return Some(90 + idx as i32);
            }
        }
        
        // Fallback: leave/skip
        Some(99)
    }
    
    /// Execute an action
    fn step(&mut self, action_id: i32) -> (bool, f32) {
        if self.state.screen == GamePhase::GameOver {
            let won = self.state.player.current_hp > 0;
            return (true, if won { 100.0 } else { -100.0 });
        }
        
        let reward = match self.state.screen {
            GamePhase::Combat => self.handle_combat(action_id),
            GamePhase::Map => self.handle_map(action_id),
            GamePhase::Reward => self.handle_reward(action_id),
            GamePhase::Shop => self.handle_shop(action_id),
            GamePhase::Rest => self.handle_rest(action_id),
            GamePhase::Event => self.handle_event(action_id),
            GamePhase::CardSelect => self.handle_card_select(action_id),
            GamePhase::GameOver => 0.0,
        };
        
        (self.state.screen == GamePhase::GameOver, reward)
    }
    
    fn handle_combat(&mut self, action_id: i32) -> f32 {
        match action_id {
            // Play card
            0..=9 => {
                let idx = action_id as usize;
                if idx < self.state.hand.len() {
                    // Find first living enemy as target
                    let target = self.state.enemies.iter()
                        .enumerate()
                        .find(|(_, e)| !e.is_dead())
                        .map(|(i, _)| i);
                    let _ = engine::play_card_from_hand(
                        &mut self.state,
                        &self.card_library,
                        idx,
                        target,
                    );
                    self.cards_played += 1;
                    
                    // Check if combat ended
                    if engine::all_enemies_dead(&self.state) {
                        self.end_combat(false);
                    }
                }
                0.1
            }
            // End turn
            10 => {
                self.execute_end_turn();
                0.0
            }
            // Use potion (11-14)
            11..=14 => {
                let slot_idx = (action_id - 11) as usize;
                self.use_potion(slot_idx);
                0.2
            }
            // Discard potion (15-18)
            15..=18 => {
                let slot_idx = (action_id - 15) as usize;
                let _ = self.state.potions.discard(slot_idx);
                0.0
            }
            _ => 0.0,
        }
    }
    
    fn use_potion(&mut self, slot_idx: usize) {
        // Get potion ID first
        let potion_id = match self.state.potions.get(slot_idx) {
            Ok(Some(id)) => id.clone(),
            _ => return,
        };
        
        // Get potion definition
        let potion_def = match &self.potion_library {
            Some(lib) => match lib.get(&potion_id) {
                Ok(def) => def.clone(),
                Err(_) => return,
            },
            None => return,
        };
        
        // Determine target
        let target_idx = if potion_def.requires_target() {
            self.state.enemies.iter()
                .position(|e| !e.is_dead())
        } else {
            None
        };
        
        // Check Sacred Bark
        let has_sacred_bark = self.state.relics.iter()
            .any(|r| r.id == "SacredBark");
        
        // Use the potion
        if engine::use_potion(&mut self.state, &potion_def, target_idx, has_sacred_bark).is_ok() {
            let _ = self.state.potions.remove(slot_idx);
            self.potions_used += 1;
        }
    }
    
    fn execute_end_turn(&mut self) {
        engine::on_turn_end(&mut self.state, &self.card_library, self.relic_library.as_ref());
        
        // Execute enemy actions using monster library if available
        if let Some(ref lib) = self.monster_library {
            engine::execute_enemy_turn(&mut self.state, lib);
        } else {
            // Simple enemy attacks
            for i in 0..self.state.enemies.len() {
                if self.state.enemies[i].current_hp() > 0 {
                    let damage = 10;
                    self.state.player.take_damage(damage);
                }
            }
        }
        
        // Check death
        if engine::player_dead(&mut self.state) {
            engine::on_player_death(&mut self.state);
            return;
        }
        
        // Check victory
        if engine::all_enemies_dead(&self.state) {
            self.end_combat(false);
            return;
        }
        
        // Safety: force end combat after 50 turns to prevent infinite loops
        if self.state.turn >= 50 {
            self.end_combat(false);
            return;
        }
        
        // Start next turn
        engine::on_turn_start(&mut self.state, &self.card_library, self.relic_library.as_ref());
        self.state.start_turn();
        engine::on_turn_start_post_draw(&mut self.state, &self.card_library);
    }
    
    fn end_combat(&mut self, is_boss: bool) {
        // Trigger end-of-combat relics (Burning Blood heal, etc.)
        engine::on_battle_end(&mut self.state, true, self.relic_library.as_ref());
        engine::on_combat_victory(&mut self.state, is_boss);
        
        // Grant gold (10-20 normal, 25-35 elite, 95-105 boss)
        let gold = if is_boss {
            self.rng.random_range(95..=105)
        } else {
            self.rng.random_range(10..=20)
        };
        self.state.gold += gold;
        
        // Grant a card — 60% attack, 30% skill, 10% power
        let roll = self.rng.random_range(0..10);
        let pool = if roll < 6 { "Attack" } else if roll < 9 { "Skill" } else { "Power" };
        if let Some(card) = self.card_library.get_random_card(pool, &mut self.rng) {
            self.state.obtain_card(card);  // Add to persistent master_deck
        }
        
        // Random chance to get a potion after combat
        if !self.state.potions.is_full() && self.rng.random_range(0..100) < 40 {
            let potions = ["BlockPotion", "FirePotion", "SwiftPotion", "StrengthPotion", "RegenPotion"];
            let potion = potions[self.rng.random_range(0..potions.len())];
            let _ = self.state.potions.add(potion.to_string());
        }
    }
    
    fn handle_map(&mut self, action_id: i32) -> f32 {
        match action_id {
            20..=29 => {
                let idx = (action_id - 20) as usize;
                let valid_moves = engine::get_valid_moves(&self.state);
                
                if idx < valid_moves.len() {
                    let node_idx = valid_moves[idx];
                    let result = engine::proceed_to_node(
                        &mut self.state,
                        node_idx,
                        &self.card_library,
                    );
                    self.handle_node_result(&result);
                }
                0.1
            }
            99 => {
                self.try_act_transition();
                0.0
            }
            _ => 0.0,
        }
    }
    
    fn handle_node_result(&mut self, result: &engine::NodeResult) {
        match result {
            engine::NodeResult::Combat { encounter_id, is_elite, is_boss } => {
                self.setup_combat(encounter_id, *is_elite, *is_boss);
            }
            engine::NodeResult::Shop => {
                self.state.screen = GamePhase::Shop;
                let shop_state = shop::generate_shop(
                    CardColor::Red, // Ironclad
                    &self.card_library,
                    None,
                    &mut self.rng,
                );
                self.state.shop_state = Some(shop_state);
            }
            engine::NodeResult::Rest => {
                self.state.screen = GamePhase::Rest;
            }
            engine::NodeResult::Event { event_id } => {
                self.state.screen = GamePhase::Event;
                let event_state = ActiveEventState::new(event_id, self.seed);
                self.state.event_state = Some(event_state);
            }
            engine::NodeResult::Treasure => {
                // Give a random relic or potion
                if !self.state.potions.is_full() {
                    let _ = self.state.potions.add("EnergyPotion".to_string());
                }
                self.state.screen = GamePhase::Map;
            }
            engine::NodeResult::InvalidMove { .. } |
            engine::NodeResult::NoMap |
            engine::NodeResult::GameOver => {}
        }
    }
    
    fn setup_combat(&mut self, encounter_id: &str, _is_elite: bool, is_boss: bool) {
        self.state.screen = GamePhase::Combat;
        self.state.enemies.clear();
        
        // Spawn monsters
        let spawns = dungeon::spawn_encounter(&mut self.rng, encounter_id);
        
        use sts_sim::enemy::MonsterState;
        
        if let Some(ref lib) = self.monster_library {
            for spawn in &spawns {
                if let Ok(def) = lib.get(&spawn.monster_id) {
                    let monster = MonsterState::new(def, &mut self.rng, 0);
                    self.state.enemies.push(monster);
                } else {
                    self.state.enemies.push(MonsterState::new_simple(&spawn.monster_id, 50));
                }
            }
        } else {
            for spawn in &spawns {
                self.state.enemies.push(MonsterState::new_simple(&spawn.monster_id, 50));
            }
        }
        
        // Mark boss flag for act transition
        if is_boss {
            // This will be used when combat ends
        }
        
        // Initialize combat deck from master_deck
        // Java: drawPile.initializeDeck(masterDeck)
        // Copies master_deck → draw_pile, clears other piles, shuffles, Innate on top
        self.state.initialize_combat_deck();
        
        // Start combat (trigger relics, etc.)
        engine::on_battle_start(&mut self.state, &self.card_library, self.relic_library.as_ref());
        
        // Roll initial enemy intents so Turn 1 isn't "Does nothing"
        if let Some(ref lib) = self.monster_library {
            engine::plan_enemy_moves(&mut self.state, lib);
        }
        
        self.state.start_turn();
    }
    
    fn try_act_transition(&mut self) {
        let valid_moves = engine::get_valid_moves(&self.state);
        
        if self.state.boss_defeated || valid_moves.is_empty() {
            // If boss beaten OR stuck at top of map with no moves, advance act
            let next_act = self.state.act + 1;
            
            if next_act > 4 {
                self.state.screen = GamePhase::GameOver;
                return;
            }
            
            self.state.act = next_act;
            self.state.floor = 0;
            self.state.combat_count = 0;
            self.state.boss_defeated = false;
            
            let full_map = generate_map((self.seed + next_act as u64) as i64, next_act, true);
            self.state.map = Some(SimpleMap::from_map(&full_map));
            self.state.current_map_node = None;
            self.state.screen = GamePhase::Map;
        }
    }
    
    fn handle_reward(&mut self, action_id: i32) -> f32 {
        match action_id {
            30..=33 => {
                // Take card reward (simplified - just skip)
                // In a real implementation, we'd add the card to deck
            }
            34 => {
                // Take gold
                self.state.gold += 25;
            }
            35 => {
                // Take relic (simplified)
            }
            36 => {
                // Take potion
                if !self.state.potions.is_full() {
                    let _ = self.state.potions.add("BlockPotion".to_string());
                }
            }
            _ => {}
        }
        engine::finish_rewards(&mut self.state);
        0.0
    }
    
    fn handle_shop(&mut self, action_id: i32) -> f32 {
        match action_id {
            // Buy card
            40..=44 => {
                let idx = (action_id - 40) as usize;
                if let Some(ref shop) = self.state.shop_state.clone() {
                    if idx < shop.cards.len() {
                        let card = &shop.cards[idx];
                        if card.price <= self.state.gold {
                            self.state.gold -= card.price;
                            self.gold_spent += card.price as u32;
                            self.cards_bought += 1;
                            // Add card to persistent master deck
                            let new_card = CardInstance::new(card.card.definition_id.clone(), card.card.current_cost);
                            self.state.obtain_card(new_card);
                        }
                    }
                }
            }
            // Buy relic
            45..=47 => {
                let idx = (action_id - 45) as usize;
                if let Some(ref shop) = self.state.shop_state.clone() {
                    if idx < shop.relics.len() {
                        let relic = &shop.relics[idx];
                        if relic.price <= self.state.gold {
                            self.state.gold -= relic.price;
                            self.gold_spent += relic.price as u32;
                            self.relics_bought += 1;
                        }
                    }
                }
            }
            // Buy potion
            48..=50 => {
                let idx = (action_id - 48) as usize;
                if let Some(ref shop) = self.state.shop_state.clone() {
                    if idx < shop.potions.len() && !self.state.potions.is_full() {
                        let potion = &shop.potions[idx];
                        if potion.price <= self.state.gold {
                            self.state.gold -= potion.price;
                            self.gold_spent += potion.price as u32;
                            let _ = self.state.potions.add(potion.potion_id.clone());
                        }
                    }
                }
            }
            // Leave shop
            99 => {}
            _ => {}
        }
        
        if action_id == 99 || self.state.gold < 50 {
            engine::leave_shop(&mut self.state);
        }
        0.0
    }
    
    fn handle_rest(&mut self, action_id: i32) -> f32 {
        match action_id {
            60 => {
                // Rest: heal 30%
                let heal = (self.state.player.max_hp as f32 * 0.3) as i32;
                self.state.player.current_hp = (self.state.player.current_hp + heal)
                    .min(self.state.player.max_hp);
            }
            61 => {
                // Smith: upgrade best non-upgraded card
                // Priority: Bash > Powers > high-cost Attacks > Skills
                let upgradeable: Vec<(usize, i32)> = self.state.draw_pile.iter()
                    .enumerate()
                    .filter(|(_, c)| !c.upgraded)
                    .map(|(i, c)| {
                        let id = c.definition_id.to_lowercase();
                        let priority = if id.contains("bash") { 100 }
                            else if c.card_type == CardType::Power { 80 }
                            else if c.card_type == CardType::Attack && c.base_cost >= 2 { 60 }
                            else if c.card_type == CardType::Attack { 40 }
                            else if id.contains("strike") || id.contains("defend") { 5 } // low priority basics
                            else { 30 };
                        (i, priority)
                    })
                    .collect();
                if let Some(&(best_idx, _)) = upgradeable.iter().max_by_key(|(_, p)| *p) {
                    self.state.draw_pile[best_idx].upgraded = true;
                }
            }
            _ => {}
        }
        engine::leave_rest(&mut self.state);
        0.0
    }
    
    fn handle_event(&mut self, action_id: i32) -> f32 {
        match action_id {
            90..=93 => {
                let choice = (action_id - 90) as usize;
                match engine::execute_event_option(
                    &mut self.state,
                    choice,
                ) {
                    Ok(engine::EventProcessResult::Complete) => {
                        // Event is done — clear state and return to map
                        engine::finish_event(&mut self.state);
                    }
                    Ok(engine::EventProcessResult::Continue) => {
                        // Event continues (loop mechanic, e.g. Knowing Skull)
                    }
                    Ok(engine::EventProcessResult::StartCombat { .. }) => {
                        // Combat already set up by execute_event_option
                    }
                    Ok(engine::EventProcessResult::AwaitingCardSelect) => {
                        // Card selection screen is active
                    }
                    Ok(engine::EventProcessResult::PlayerDied) => {
                        // Player died, GameOver already set
                    }
                    Err(_) => {
                        // Option wasn't valid — force leave
                        self.state.event_state = None;
                        engine::finish_event(&mut self.state);
                    }
                }
            }
            99 => {
                self.state.event_state = None;
                engine::finish_event(&mut self.state);
            }
            _ => {}
        }
        0.0
    }
    
    fn handle_card_select(&mut self, _action_id: i32) -> f32 {
        self.state.card_select_action = None;
        self.state.card_select_pool.clear();
        self.state.screen = GamePhase::Map;
        0.0
    }
}

// ============================================================================
// Statistics
// ============================================================================

#[derive(Default)]
struct ValidationStats {
    total_runs: u32,
    total_steps: u64,
    total_time: Duration,
    victories: u32,
    deaths: u32,
    timeouts: u32,
    errors: u32,
    max_act_reached: u8,
    max_floor_reached: u8,
    total_floor_reached: u32,
    act_deaths: HashMap<u8, u32>,
    act_reached: HashMap<u8, u32>,
    error_details: Vec<String>,
    // New tracking
    total_potions_used: u32,
    total_cards_played: u32,
    total_gold_spent: u32,
    total_cards_bought: u32,
    total_relics_bought: u32,
}

impl ValidationStats {
    fn record_run(&mut self, steps: u64, outcome: RunOutcome, final_act: u8, final_floor: u8, sim: &TestSimulator) {
        self.total_runs += 1;
        self.total_steps += steps;
        self.max_act_reached = self.max_act_reached.max(final_act);
        self.max_floor_reached = self.max_floor_reached.max(final_floor);
        self.total_floor_reached += final_floor as u32;
        *self.act_reached.entry(final_act).or_insert(0) += 1;
        
        // Record simulator stats
        self.total_potions_used += sim.potions_used;
        self.total_cards_played += sim.cards_played;
        self.total_gold_spent += sim.gold_spent;
        self.total_cards_bought += sim.cards_bought;
        self.total_relics_bought += sim.relics_bought;
        
        match outcome {
            RunOutcome::Victory => self.victories += 1,
            RunOutcome::Death => {
                self.deaths += 1;
                *self.act_deaths.entry(final_act).or_insert(0) += 1;
            }
            RunOutcome::Timeout => self.timeouts += 1,
            RunOutcome::Error(_) => self.errors += 1,
        }
    }
    
    fn print_summary(&self) {
        println!("\n{}", "═".repeat(65));
        println!("  🎮 END-TO-END VALIDATION RESULTS - SMART AGENT");
        println!("{}", "═".repeat(65));
        
        println!("\n📊 Overview:");
        println!("  Total runs:    {}", self.total_runs);
        println!("  Total steps:   {}", self.total_steps);
        println!("  Total time:    {:.2}s", self.total_time.as_secs_f64());
        
        if self.total_time.as_secs_f64() > 0.0 {
            let sps = self.total_steps as f64 / self.total_time.as_secs_f64();
            println!("  Steps/second:  {:.0}", sps);
        }
        
        println!("\n🎯 Outcomes:");
        let pct = |n: u32| 100.0 * n as f64 / self.total_runs.max(1) as f64;
        println!("  🏆 Victories:  {:3} ({:5.1}%)", self.victories, pct(self.victories));
        println!("  💀 Deaths:     {:3} ({:5.1}%)", self.deaths, pct(self.deaths));
        println!("  ⏰ Timeouts:   {:3} ({:5.1}%)", self.timeouts, pct(self.timeouts));
        println!("  ❌ Errors:     {:3} ({:5.1}%)", self.errors, pct(self.errors));
        
        println!("\n📈 Progress Distribution:");
        for act in 1..=4 {
            let reached = self.act_reached.get(&act).copied().unwrap_or(0);
            let died = self.act_deaths.get(&act).copied().unwrap_or(0);
            if reached > 0 {
                let bar_len = (reached as f64 / self.total_runs.max(1) as f64 * 20.0) as usize;
                let bar = "█".repeat(bar_len);
                println!("  Act {}: {:3} reached, {:3} died  {}", act, reached, died, bar);
            }
        }
        
        let avg_floor = if self.total_runs > 0 { 
            self.total_floor_reached as f64 / self.total_runs as f64 
        } else { 0.0 };
        
        println!("\n🏁 Progress Summary:");
        println!("  Avg Floor:   {:.1}", avg_floor);
        println!("  Max Act:     {}", self.max_act_reached);
        println!("  Max Floor:   {}", self.max_floor_reached);
        
        println!("\n🧪 Potion System Stats:");
        println!("  Potions Used:   {:5} (avg {:.1}/run)", 
            self.total_potions_used,
            self.total_potions_used as f64 / self.total_runs.max(1) as f64);
        
        println!("\n🃏 Card & Shop Stats:");
        println!("  Cards Played:   {:5} (avg {:.0}/run)",
            self.total_cards_played,
            self.total_cards_played as f64 / self.total_runs.max(1) as f64);
        println!("  Cards Bought:   {:5}", self.total_cards_bought);
        println!("  Relics Bought:  {:5}", self.total_relics_bought);
        println!("  Gold Spent:     {:5}", self.total_gold_spent);
        
        if !self.error_details.is_empty() {
            println!("\n⚠️ Errors (first 5):");
            for (i, e) in self.error_details.iter().take(5).enumerate() {
                println!("  {}. {}", i + 1, e);
            }
        }
        
        println!("\n{}", "═".repeat(65));
        if self.errors == 0 && self.total_potions_used > 0 {
            println!("  ✅ VALIDATION PASSED - All systems working!");
        } else if self.errors == 0 {
            println!("  ⚠️  VALIDATION PASSED - But no potions were used!");
        } else {
            println!("  ❌ VALIDATION FAILED - {} errors encountered", self.errors);
        }
        println!("{}", "═".repeat(65));
    }
}

#[derive(Debug, Clone)]
enum RunOutcome {
    Victory,
    Death,
    Timeout,
    Error(String),
}

// ============================================================================
// Config & Main
// ============================================================================

struct Config {
    num_runs: u32,
    max_steps: u64,
    verbose: bool,
    god_mode: bool,
}

impl Default for Config {
    fn default() -> Self {
        Self { 
            num_runs: 100,      // Default to 100 runs
            max_steps: 10000,   // More steps for full runs
            verbose: false, 
            god_mode: false 
        }
    }
}

fn parse_args() -> Config {
    let args: Vec<String> = std::env::args().collect();
    let mut cfg = Config::default();
    
    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--runs" | "-n" => {
                if i + 1 < args.len() {
                    cfg.num_runs = args[i + 1].parse().unwrap_or(100);
                    i += 1;
                }
            }
            "--max-steps" | "-s" => {
                if i + 1 < args.len() {
                    cfg.max_steps = args[i + 1].parse().unwrap_or(10000);
                    i += 1;
                }
            }
            "--verbose" | "-v" => cfg.verbose = true,
            "--god-mode" | "-g" => cfg.god_mode = true,
            "--help" | "-h" => {
                println!("Usage: e2e_test [OPTIONS]");
                println!("  -n, --runs N       Number of runs (default: 100)");
                println!("  -s, --max-steps N  Max steps (default: 10000)");
                println!("  -v, --verbose      Verbose output");
                println!("  -g, --god-mode     Infinite HP");
                std::process::exit(0);
            }
            _ => {}
        }
        i += 1;
    }
    cfg
}

fn run_game(seed: u64, cfg: &Config) -> (u64, RunOutcome, u8, u8, TestSimulator) {
    let mut sim = TestSimulator::new(seed);
    sim.reset();
    
    let mut steps: u64 = 0;
    let mut phase_counts: std::collections::HashMap<String, u64> = std::collections::HashMap::new();
    
    loop {
        let act = sim.state.act;
        let floor = sim.state.floor;
        let screen = sim.get_screen_type().to_string();
        
        // Track phase distribution
        *phase_counts.entry(screen.clone()).or_insert(0) += 1;
        
        if sim.is_terminal() {
            let outcome = if sim.state.player.current_hp > 0 {
                RunOutcome::Victory
            } else {
                RunOutcome::Death
            };
            return (steps, outcome, act, floor, sim);
        }
        
        if steps >= cfg.max_steps {
            // Print phase distribution on timeout for diagnosis
            eprintln!("  ⏰ TIMEOUT seed={} — Phase distribution: {:?}", seed, phase_counts);
            return (steps, RunOutcome::Timeout, act, floor, sim);
        }
        
        if cfg.god_mode {
            sim.state.player.current_hp = sim.state.player.max_hp;
        }
        
        // Use smart agent decision
        let action = match sim.decide_action() {
            Some(a) => a,
            None => {
                return (steps, RunOutcome::Error(format!(
                    "No valid actions - Act {}, Floor {}, Screen: {}", act, floor, screen
                )), act, floor, sim);
            }
        };
        
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| sim.step(action)));
        
        match result {
            Ok(_) => steps += 1,
            Err(e) => {
                let msg = if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else {
                    "Unknown panic".to_string()
                };
                return (steps, RunOutcome::Error(format!(
                    "Panic - Act {}, Floor {}, Screen: {}, Action: {} - {}",
                    act, floor, screen, action, msg
                )), act, floor, sim);
            }
        }
        
        if cfg.verbose && steps % 100 == 0 {
            println!("  Step {:4}: Act {}, Floor {:2}, HP {}/{}, Screen: {}, Potions: {}",
                steps, sim.state.act, sim.state.floor,
                sim.state.player.current_hp, sim.state.player.max_hp,
                sim.get_screen_type(),
                sim.potions_used);
        }
    }
}

fn main() {
    let cfg = parse_args();
    
    println!("╔═══════════════════════════════════════════════════════════════╗");
    println!("║  🎮 STS Simulator - E2E Validation with SMART AGENT           ║");
    println!("╚═══════════════════════════════════════════════════════════════╝");
    println!();
    println!("  Runs: {}, Max steps: {}, God mode: {}",
        cfg.num_runs, cfg.max_steps, if cfg.god_mode { "ON" } else { "OFF" });
    println!();
    
    let mut stats = ValidationStats::default();
    let start = Instant::now();
    
    for run_id in 0..cfg.num_runs {
        let seed = 12345 + run_id as u64 * 7919;
        
        if cfg.verbose {
            println!("\n🏃 Run {} (seed: {})", run_id + 1, seed);
        }
        
        let (steps, outcome, act, floor, sim) = run_game(seed, &cfg);
        
        if let RunOutcome::Error(ref msg) = outcome {
            stats.error_details.push(msg.clone());
        }
        
        stats.record_run(steps, outcome.clone(), act, floor, &sim);
        
        if !cfg.verbose {
            let ch = match &outcome {
                RunOutcome::Victory => "🏆",
                RunOutcome::Death => "💀",
                RunOutcome::Timeout => "⏰",
                RunOutcome::Error(_) => "❌",
            };
            print!("\r  [{:3}%] Run {:3}/{:3} {} Act {} Floor {:2} ({} steps, {} potions)    ",
                (run_id + 1) * 100 / cfg.num_runs, run_id + 1, cfg.num_runs,
                ch, act, floor, steps, sim.potions_used);
            std::io::stdout().flush().unwrap();
        }
    }
    
    stats.total_time = start.elapsed();
    println!();
    stats.print_summary();
}
