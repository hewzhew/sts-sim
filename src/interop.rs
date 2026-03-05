//! Python interop module via PyO3.
//!
//! This module exposes the Rust simulator to Python for RL training.
//! The interface follows OpenAI Gym conventions: step(), reset(), observation().
//!
//! ## Usage from Python
//!
//! ```python
//! import sts_sim
//!
//! # Create environment
//! env = sts_sim.StsEnv(seed=42, cards_path="data/cards.json")
//!
//! # Gym-style interface
//! obs = env.reset()
//! for _ in range(100):
//!     action = policy(obs)  # Your RL agent
//!     obs, reward, done, truncated, info = env.step(action)
//!     if done:
//!         break
//!
//! # Parallel batch simulation
//! config = sts_sim.SimConfig(num_simulations=10000)
//! batch_sim = sts_sim.BatchSimulator(config)
//! stats = batch_sim.run_batch_stats(base_seed=0)
//! print(f"Win rate: {stats['win_rate']:.2%}")
//! ```

use pyo3::prelude::*;
use pyo3::types::PyDict;
use rayon::prelude::*;

use crate::ai::encoding;
use crate::engine;
use crate::loader::{CardLibrary, MonsterLibrary};
use crate::items::relics::RelicLibrary;
use crate::items::potions::PotionLibrary;
use crate::schema::{CardInstance, CardColor};
use crate::state::{Enemy, GameState, GamePhase};
use crate::campfire::{CampfireOption, execute_option as execute_campfire_option, get_available_options};
use crate::shop::ShopResult;

// ============================================================================
// Action Constants for RL Agent
// ============================================================================

/// Action ID ranges for different game screens.
/// The RL agent outputs an integer action_id, which is interpreted based on
/// the current screen (GamePhase).

/// Play card from hand (index 0-9). ACTION_PLAY_CARD + card_index
pub const ACTION_PLAY_CARD_START: i32 = 0;
pub const ACTION_PLAY_CARD_END: i32 = 9;

/// End the current turn (during Combat).
pub const ACTION_END_TURN: i32 = 10;

/// Use potion from slot (index 0-3 for slots). 11=slot0, 12=slot1, 13=slot2, 14=slot3
pub const ACTION_USE_POTION_START: i32 = 11;
pub const ACTION_USE_POTION_END: i32 = 14;

/// Discard potion from slot (15-18). Used when slots are full or to make room.
pub const ACTION_DISCARD_POTION_START: i32 = 15;
pub const ACTION_DISCARD_POTION_END: i32 = 18;

/// Map node selection. ACTION_MAP_SELECT + child_index (0-9).
pub const ACTION_MAP_SELECT_START: i32 = 20;
pub const ACTION_MAP_SELECT_END: i32 = 29;

/// Reward selection. ACTION_REWARD_SELECT + reward_index (0-9).
pub const ACTION_REWARD_SELECT_START: i32 = 30;
pub const ACTION_REWARD_SELECT_END: i32 = 39;

/// Shop actions. 
/// 40-44: Buy card (index 0-4)
/// 45-47: Buy relic (index 0-2)
/// 48-50: Buy potion (index 0-2)
/// 51-60: Purge card from deck (index 0-9)
pub const ACTION_SHOP_BUY_CARD_START: i32 = 40;
pub const ACTION_SHOP_BUY_CARD_END: i32 = 44;
pub const ACTION_SHOP_BUY_RELIC_START: i32 = 45;
pub const ACTION_SHOP_BUY_RELIC_END: i32 = 47;
pub const ACTION_SHOP_BUY_POTION_START: i32 = 48;
pub const ACTION_SHOP_BUY_POTION_END: i32 = 50;
pub const ACTION_SHOP_PURGE_START: i32 = 51;
pub const ACTION_SHOP_PURGE_END: i32 = 60;

/// Campfire actions.
/// 60: Rest (heal)
/// 61: Smith (upgrade card) - needs target
/// 62: Lift (Girya)
/// 63: Toke (Peace Pipe) - needs target
/// 64: Dig (Shovel)
/// 65: Recall
pub const ACTION_CAMPFIRE_REST: i32 = 60;
pub const ACTION_CAMPFIRE_SMITH: i32 = 61;
pub const ACTION_CAMPFIRE_LIFT: i32 = 62;
pub const ACTION_CAMPFIRE_TOKE: i32 = 63;
pub const ACTION_CAMPFIRE_DIG: i32 = 64;
pub const ACTION_CAMPFIRE_RECALL: i32 = 65;

/// Target selection for Smith/Toke (card index 0-19).
pub const ACTION_CAMPFIRE_TARGET_START: i32 = 70;
pub const ACTION_CAMPFIRE_TARGET_END: i32 = 89;

/// Event choice selection (0-3).
pub const ACTION_EVENT_CHOICE_START: i32 = 90;
pub const ACTION_EVENT_CHOICE_END: i32 = 93;

/// Proceed / Skip / Leave current screen.
pub const ACTION_PROCEED: i32 = 99;

// ============================================================================
// Reward Constants
// ============================================================================

const REWARD_COMBAT_WIN: f32 = 10.0;
const REWARD_GAME_WIN: f32 = 100.0;
const REWARD_GAME_LOSS: f32 = -10.0;     // Reduced from -100 to prevent policy collapse
const REWARD_INVALID_ACTION: f32 = -0.5; // Reduced penalty
const REWARD_VALID_ACTION: f32 = 0.0;
const REWARD_DAMAGE_DEALT: f32 = 0.1;    // Increased from 0.01 - reward for dealing damage
const REWARD_DAMAGE_TAKEN: f32 = -0.05;  // Reduced from -0.1 - small penalty for damage taken
const REWARD_BLOCK_GAINED: f32 = 0.02;   // New: small reward for gaining block

// ============================================================================
// Simplified Python Interface (PyStsSim)
// ============================================================================

/// Simplified Python interface for RL training.
/// 
/// This is a clean, stub-friendly interface that wraps the full game state
/// and provides methods for the RL agent to interact with.
/// 
/// ## Usage from Python
/// ```python
/// import sts_sim
/// env = sts_sim.PyStsSim(42)
/// print("Screen:", env.get_screen_type())
/// done, reward = env.step(10)
/// ```
#[pyclass(name = "PyStsSim")]
pub struct PyStsSim {
    state: GameState,
    card_library: CardLibrary,
    monster_library: Option<MonsterLibrary>,
    relic_library: Option<RelicLibrary>,
    potion_library: Option<PotionLibrary>,
    seed: u64,
}

// Global caches for data libraries — loaded once, cloned per instance.
// This avoids re-reading JSON files for every PyStsSim::new() call.
static CARD_LIBRARY_CACHE: std::sync::OnceLock<CardLibrary> = std::sync::OnceLock::new();
static MONSTER_LIBRARY_CACHE: std::sync::OnceLock<Option<MonsterLibrary>> = std::sync::OnceLock::new();
static POTION_LIBRARY_CACHE: std::sync::OnceLock<Option<PotionLibrary>> = std::sync::OnceLock::new();

#[pymethods]
impl PyStsSim {
    /// Create a new simulator instance.
    /// 
    /// Args:
    ///     seed: Random seed for deterministic simulation (default: random)
    ///     cards_path: Path to card definitions JSON file (default: "data/cards.json")
    #[new]
    #[pyo3(signature = (seed=None, cards_path=None))]
    fn new(seed: Option<u64>, cards_path: Option<&str>) -> PyResult<Self> {
        let seed = seed.unwrap_or_else(|| {
            use std::time::{SystemTime, UNIX_EPOCH};
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .map(|d| d.as_secs())
                .unwrap_or(42)
        });
        
        // Load card library — cached after first load
        let card_path = cards_path.unwrap_or("data/cards.json");
        let card_library = CARD_LIBRARY_CACHE.get_or_init(|| {
            CardLibrary::load(card_path)
                .expect(&format!("Failed to load card library from {}", card_path))
        }).clone();
        
        // Load monster library — cached after first load
        let monster_library = MONSTER_LIBRARY_CACHE.get_or_init(|| {
            MonsterLibrary::load("data/monsters_verified.json").ok()
        }).clone();
        
        // Relic library not yet implemented as loadable
        let relic_library = None;
        
        // Load potion library — cached after first load
        let potion_library = POTION_LIBRARY_CACHE.get_or_init(|| {
            PotionLibrary::load("data/potions.json").ok()
        }).clone();
        
        // Create game state
        let state = GameState::new(seed);
        
        Ok(Self {
            state,
            card_library,
            monster_library,
            relic_library,
            potion_library,
            seed,
        })
    }
    
    /// Reset the game to a fresh run.
    /// 
    /// Args:
    ///     seed: Optional new seed. If None, increments the current seed.
    ///
    /// Returns:
    ///     The screen type after reset ("MAP")
    #[pyo3(signature = (seed=None))]
    fn reset(&mut self, seed: Option<u64>) -> String {
        // Update seed
        if let Some(s) = seed {
            self.seed = s;
        } else {
            self.seed = self.seed.wrapping_add(1);
        }
        
        // Create fresh game state
        self.state = GameState::new(self.seed);
        
        // Generate map for Act 1
        use crate::map::{generate_map, SimpleMap};
        let full_map = generate_map(self.seed as i64, 1, true);
        self.state.map = Some(SimpleMap::from_map(&full_map));
        
        // Setup starter deck
        Self::setup_starter_deck(&mut self.state, &self.card_library);
        
        self.get_screen_type()
    }
    
    /// Load game state from a CommunicationMod JSON string.
    /// Useful for injecting external states into the simulator mid-run.
    /// 
    /// Args:
    ///     json_str: The raw JSON string from CommunicationMod
    /// 
    /// Returns:
    ///     True if successfully hydrated, False otherwise.
    fn load_state_from_json(&mut self, json_str: &str) -> PyResult<bool> {
        let parsed: serde_json::Value = serde_json::from_str(json_str)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid JSON: {}", e)))?;
        
        if let Some(new_state) = crate::testing::hydrator::hydrate_combat_state(&parsed) {
            self.state = new_state;
            Ok(true)
        } else {
            Ok(false)
        }
    }
    
    /// Take an action in the game.
    /// 
    /// Args:
    ///     action_id: Integer action ID (interpretation depends on screen type)
    /// 
    /// Returns:
    ///     Tuple of (is_done, reward)
    /// 
    /// Action ID Ranges:
    /// - 0-9: Play card from hand (Combat)
    /// - 10: End turn (Combat)
    /// - 11-14: Select target enemy (Combat)
    /// - 20-29: Select map node (Map)
    /// - 30-39: Select reward (Reward)
    /// - 40-44: Buy card (Shop)
    /// - 45-47: Buy relic (Shop)
    /// - 48-50: Buy potion (Shop)
    /// - 51-60: Purge card (Shop)
    /// - 60-65: Campfire options (Rest)
    /// - 70-89: Campfire target selection
    /// - 90-93: Event choices (Event)
    /// - 99: Proceed/Skip/Leave
    fn step(&mut self, action_id: i32) -> (bool, f32) {
        // Check for game over state
        if self.state.screen == GamePhase::GameOver {
            let won = self.state.player.current_hp > 0;
            return (true, if won { REWARD_GAME_WIN } else { REWARD_GAME_LOSS });
        }
        
        // Dispatch based on current screen
        let reward = match self.state.screen {
            GamePhase::Combat => self.handle_combat_action(action_id),
            GamePhase::Map => self.handle_map_action(action_id),
            GamePhase::Reward => self.handle_reward_action(action_id),
            GamePhase::Shop => self.handle_shop_action(action_id),
            GamePhase::Rest => self.handle_campfire_action(action_id),
            GamePhase::Event => self.handle_event_action(action_id),
            GamePhase::CardSelect => self.handle_card_select_action(action_id),
            GamePhase::GameOver => {
                // Already handled above
                REWARD_VALID_ACTION
            }
        };
        
        // Check if game ended after action
        let is_done = self.state.screen == GamePhase::GameOver;
        
        (is_done, reward)
    }
    
    /// Get the current game state as a JSON string.
    /// 
    /// Returns:
    ///     JSON string representation of key state fields
    fn get_state_str(&self) -> String {
        // Create a simplified state representation
        format!(
            r#"{{"screen": "{}", "act": {}, "floor": {}, "hp": {}, "max_hp": {}, "gold": {}, "energy": {}, "hand_size": {}, "draw_pile_size": {}, "discard_pile_size": {}}}"#,
            self.get_screen_type(),
            self.state.act,
            self.state.floor,
            self.state.player.current_hp,
            self.state.player.max_hp,
            self.state.gold,
            self.state.player.energy,
            self.state.hand.len(),
            self.state.draw_pile.len(),
            self.state.discard_pile.len(),
        )
    }
    
    /// Get the current screen type.
    /// 
    /// Returns:
    ///     One of: "MAP", "COMBAT", "REWARD", "SHOP", "REST", "EVENT", "CARD_SELECT", "GAME_OVER"
    fn get_screen_type(&self) -> String {
        match self.state.screen {
            GamePhase::Map => "MAP".to_string(),
            GamePhase::Combat => "COMBAT".to_string(),
            GamePhase::Reward => "REWARD".to_string(),
            GamePhase::Shop => "SHOP".to_string(),
            GamePhase::Rest => "REST".to_string(),
            GamePhase::Event => "EVENT".to_string(),
            GamePhase::CardSelect => "CARD_SELECT".to_string(),
            GamePhase::GameOver => "GAME_OVER".to_string(),
        }
    }
    
    /// Get player's current HP.
    fn get_hp(&self) -> i32 {
        self.state.player.current_hp
    }
    
    /// Get player's max HP.
    fn get_max_hp(&self) -> i32 {
        self.state.player.max_hp
    }
    
    /// Get player's current gold.
    fn get_gold(&self) -> i32 {
        self.state.gold
    }
    
    /// Get the current act (1, 2, or 3).
    fn get_act(&self) -> u8 {
        self.state.act
    }
    
    /// Get the current floor within the act.
    fn get_floor(&self) -> u8 {
        self.state.floor
    }
    
    /// Get the current seed.
    fn get_seed(&self) -> u64 {
        self.seed
    }
    
    /// Get the observation as a flat float vector (for RL training).
    ///
    /// Returns a Vec<f32> of length 205 with normalized values.
    /// This is the primary observation method for neural network input.
    ///
    /// See `encoding.rs` for the full encoding schema.
    fn get_observation(&self) -> Vec<f32> {
        encoding::encode_state_with_library(&self.state, &self.card_library)
    }
    
    /// Get the observation dimension (for defining NN input layer).
    fn get_observation_dim(&self) -> usize {
        encoding::OBS_DIM
    }
    
    /// Get a dictionary-based observation (for debugging/visualization).
    fn get_observation_dict(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        use pyo3::types::PyDict;
        
        let obs = PyDict::new(py);
        
        // Global info
        obs.set_item("hp", self.state.player.current_hp)?;
        obs.set_item("max_hp", self.state.player.max_hp)?;
        obs.set_item("energy", self.state.player.energy)?;
        obs.set_item("max_energy", self.state.player.max_energy)?;
        obs.set_item("block", self.state.player.block)?;
        obs.set_item("gold", self.state.gold)?;
        obs.set_item("floor", self.state.floor)?;
        obs.set_item("act", self.state.act)?;
        obs.set_item("turn", self.state.turn)?;
        obs.set_item("screen", self.get_screen_type())?;
        
        // Hand
        let hand: Vec<&str> = self.state.hand.iter()
            .map(|c| c.definition_id.as_str())
            .collect();
        obs.set_item("hand", hand)?;
        
        let hand_costs: Vec<i32> = self.state.hand.iter()
            .map(|c| c.current_cost)
            .collect();
        obs.set_item("hand_costs", hand_costs)?;
        
        // Enemies
        let enemy_info: Vec<(i32, i32, i32, &str, i32)> = self.state.enemies.iter()
            .filter(|e| !e.is_dead())
            .map(|e| {
                let (intent_type, intent_dmg) = e.get_intent_info();
                (e.hp, e.max_hp, e.block, e.get_intent_type_name(), intent_dmg as i32)
            })
            .collect();
        obs.set_item("enemies", enemy_info)?;
        
        // Pile sizes
        obs.set_item("draw_pile_size", self.state.draw_pile.len())?;
        obs.set_item("discard_pile_size", self.state.discard_pile.len())?;
        obs.set_item("exhaust_pile_size", self.state.exhaust_pile.len())?;
        
        // Draw pile card names
        let draw_pile: Vec<&str> = self.state.draw_pile.iter()
            .map(|c| c.definition_id.as_str())
            .collect();
        obs.set_item("draw_pile", draw_pile)?;
        
        // Discard pile card names
        let discard_pile: Vec<&str> = self.state.discard_pile.iter()
            .map(|c| c.definition_id.as_str())
            .collect();
        obs.set_item("discard_pile", discard_pile)?;
        
        // Master deck card names
        let master_deck: Vec<&str> = self.state.master_deck.iter()
            .map(|c| c.definition_id.as_str())
            .collect();
        obs.set_item("master_deck", master_deck)?;
        
        // Current rewards info
        let mut reward_strs: Vec<String> = Vec::new();
        for r in &self.state.current_rewards {
            match r {
                crate::rewards::RewardType::Gold { amount } => {
                    reward_strs.push(format!("gold:{}", amount));
                }
                crate::rewards::RewardType::Card { cards } => {
                    let card_names: Vec<&str> = cards.iter().map(|c| c.id.as_str()).collect();
                    reward_strs.push(format!("cards:{}", card_names.join(",")));
                }
                crate::rewards::RewardType::Relic { id, .. } => {
                    reward_strs.push(format!("relic:{}", id));
                }
                crate::rewards::RewardType::Potion { id, .. } => {
                    reward_strs.push(format!("potion:{}", id));
                }
            }
        }
        obs.set_item("rewards", reward_strs)?;
        
        Ok(obs.into())
    }
    
    // ========================================================================
    // Action Masking (Phase 6.4)
    // ========================================================================
    
    /// Get a boolean mask of valid actions for the current game state.
    ///
    /// Returns a Vec<bool> of size 100 where True indicates a valid action.
    /// This is essential for masked action selection in RL training.
    ///
    /// # Action ID Layout (100 total)
    ///
    /// | Range   | Description                                    |
    /// |---------|------------------------------------------------|
    /// | 0-9     | Play card from hand (Combat)                   |
    /// | 10      | End turn (Combat)                              |
    /// | 11-14   | Use potion slot (Combat)                       |
    /// | 15-19   | Reserved                                       |
    /// | 20-29   | Select map node (Map)                          |
    /// | 30-32   | Select card reward (Reward)                    |
    /// | 33      | Take gold reward (Reward)                      |
    /// | 34      | Take relic reward (Reward)                     |
    /// | 35      | Take potion reward (Reward)                    |
    /// | 36-38   | Reserved                                       |
    /// | 39      | Skip rewards (Reward)                          |
    /// | 40-44   | Buy card (Shop)                                |
    /// | 45-47   | Buy relic (Shop)                               |
    /// | 48-50   | Buy potion (Shop)                              |
    /// | 51-59   | Purge card (Shop, uses deck index mod 9)       |
    /// | 60      | Rest (Campfire)                                |
    /// | 61      | Smith (Campfire)                               |
    /// | 62      | Lift - Girya (Campfire)                        |
    /// | 63      | Toke - Peace Pipe (Campfire)                   |
    /// | 64      | Dig - Shovel (Campfire)                        |
    /// | 65      | Recall (Campfire)                              |
    /// | 66-69   | Reserved                                       |
    /// | 70-89   | Smith/Toke target selection (card index)       |
    /// | 90-93   | Event choice (Event)                           |
    /// | 94-98   | Reserved                                       |
    /// | 99      | Proceed/Skip/Leave                             |
    fn get_valid_actions_mask(&self) -> Vec<bool> {
        let mut mask = vec![false; 100];
        
        match self.state.screen {
            GamePhase::Combat => self.compute_combat_mask(&mut mask),
            GamePhase::Map => self.compute_map_mask(&mut mask),
            GamePhase::Reward => self.compute_reward_mask(&mut mask),
            GamePhase::Shop => self.compute_shop_mask(&mut mask),
            GamePhase::Rest => self.compute_campfire_mask(&mut mask),
            GamePhase::Event => self.compute_event_mask(&mut mask),
            GamePhase::CardSelect => self.compute_card_select_mask(&mut mask),
            GamePhase::GameOver => {
                // No valid actions when game is over
            }
        }
        
        mask
    }
    
    /// Get the size of the action space.
    fn get_action_space_size(&self) -> usize {
        100
    }
    
    /// Run MCTS combat evaluation: evaluate each valid combat action via random rollouts.
    /// 
    /// Args:
    ///     n_sims: Total number of random rollouts (distributed across valid actions)
    ///     max_turns: Maximum turns per rollout (default 20 if not specified)
    /// 
    /// Returns:
    ///     A dict with:
    ///       - "best_action": int, the action ID with highest average score
    ///       - "actions": list of (action_id, avg_score, survival_rate) tuples
    ///       - "total_sims": int, total simulations performed
    /// 
    /// Only works in Combat screen. Returns empty result in other screens.
    #[pyo3(signature = (n_sims=50, max_turns=20))]
    fn mcts_evaluate(&self, py: Python<'_>, n_sims: usize, max_turns: u32) -> PyResult<Py<PyAny>> {
        use pyo3::types::{PyDict, PyList};
        
        let result_dict = PyDict::new(py);
        
        if self.state.screen != GamePhase::Combat {
            result_dict.set_item("best_action", 99)?;
            result_dict.set_item("actions", PyList::empty(py))?;
            result_dict.set_item("total_sims", 0)?;
            return Ok(result_dict.into());
        }
        
        // Run MCTS evaluation
        let mcts_result = crate::ai::mcts_combat::mcts_evaluate_combat(
            &self.state,
            &self.card_library,
            self.monster_library.as_ref().unwrap(),
            n_sims,
            max_turns,
        );
        
        result_dict.set_item("best_action", mcts_result.best_action)?;
        result_dict.set_item("total_sims", mcts_result.total_sims)?;
        
        let actions_list = PyList::empty(py);
        for eval in &mcts_result.actions {
            let entry = PyDict::new(py);
            entry.set_item("action", eval.action_id)?;
            entry.set_item("avg_score", eval.avg_score)?;
            entry.set_item("survival", eval.survival_rate)?;
            entry.set_item("turns", eval.avg_turns)?;
            actions_list.append(entry)?;
        }
        result_dict.set_item("actions", actions_list)?;
        
        Ok(result_dict.into())
    }

    // ========================================================================
    // Differential Testing / Shadow Verification
    // ========================================================================

    /// Verify a JSONL diff log from bottled_ai against the Rust engine.
    ///
    /// Takes the raw JSONL content (each line: {"step":N, "command":"...", "state":{...}}).
    /// Runs step verification on every "play" command and returns a JSON report.
    ///
    /// Returns:
    ///     A JSON string with verification results:
    ///     {
    ///       "combats": [{"name": "...", "play_steps": N, "verified": N, "skipped": N, "divergent": N, "divergences": [...]}],
    ///       "summary": {"total_play_steps": N, "total_verified": N, "total_divergent": N, "total_divergences": N}
    ///     }
    fn verify_diff_log(&self, jsonl_content: &str) -> PyResult<String> {
        use crate::testing::commod_parser::parse_diff_log;
        use crate::testing::replay::extract_combat_segments;
        use crate::testing::step_verifier::verify_combat_transitions;

        let transitions = parse_diff_log(jsonl_content);
        let segments = extract_combat_segments(&transitions);

        let mut combats_json = Vec::new();
        let mut total_play = 0u64;
        let mut total_verified = 0u64;
        let mut total_divergent = 0u64;
        let mut total_divs = 0u64;

        for segment in &segments {
            let result = verify_combat_transitions(&segment.transitions, &self.card_library);

            total_play += result.play_steps as u64;
            total_verified += result.verified_steps as u64;
            total_divergent += result.divergent_steps as u64;
            total_divs += result.total_divergences as u64;

            // Collect divergence details
            let mut div_details = Vec::new();
            for step in &result.step_results {
                if !step.divergences.is_empty() {
                    let divs: Vec<serde_json::Value> = step.divergences.iter().map(|d| {
                        serde_json::json!({
                            "field": d.field,
                            "expected": d.expected,
                            "actual": d.actual
                        })
                    }).collect();
                    div_details.push(serde_json::json!({
                        "step": step.step_num,
                        "command": step.command,
                        "divergences": divs
                    }));
                }
            }

            combats_json.push(serde_json::json!({
                "name": result.combat_name,
                "total_steps": result.total_steps,
                "play_steps": result.play_steps,
                "verified": result.verified_steps,
                "skipped": result.skipped_steps,
                "divergent": result.divergent_steps,
                "total_divergences": result.total_divergences,
                "details": div_details
            }));
        }

        let report = serde_json::json!({
            "combats": combats_json,
            "summary": {
                "total_segments": segments.len(),
                "total_play_steps": total_play,
                "total_verified": total_verified,
                "total_divergent": total_divergent,
                "total_divergences": total_divs
            }
        });

        Ok(report.to_string())
    }

    /// Verify a single step: hydrate before-state, execute command, diff against after-state.
    ///
    /// Args:
    ///     before_json: CommunicationMod JSON string for the state BEFORE the command
    ///     command: The command that was executed (e.g., "play 3 0")
    ///     after_json: CommunicationMod JSON string for the state AFTER the command
    ///
    /// Returns:
    ///     A JSON string: {"status": "ok"|"skipped"|"divergent", "divergences": [...], "skip_reason": "..."}
    fn verify_single_step(&self, before_json: &str, command: &str, after_json: &str) -> PyResult<String> {
        use crate::testing::step_verifier::verify_step;

        let before: serde_json::Value = serde_json::from_str(before_json)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid before JSON: {}", e)))?;
        let after: serde_json::Value = serde_json::from_str(after_json)
            .map_err(|e| pyo3::exceptions::PyValueError::new_err(format!("Invalid after JSON: {}", e)))?;

        let result = verify_step(&before, command, &after, &self.card_library);

        let status = if result.skipped {
            "skipped"
        } else if result.divergences.is_empty() {
            "ok"
        } else {
            "divergent"
        };

        let divs: Vec<serde_json::Value> = result.divergences.iter().map(|d| {
            serde_json::json!({
                "field": d.field,
                "expected": d.expected,
                "actual": d.actual
            })
        }).collect();

        let report = serde_json::json!({
            "status": status,
            "divergences": divs,
            "skip_reason": result.skip_reason
        });

        Ok(report.to_string())
    }
}

// ============================================================================
// PyStsSim - Action Mask Computation (non-Python exposed)
// ============================================================================

impl PyStsSim {
    /// Compute valid actions for Combat screen.
    #[inline]
    fn compute_combat_mask(&self, mask: &mut [bool]) {
        let energy = self.state.player.energy;
        let has_living_enemies = self.state.enemies.iter().any(|e| !e.is_dead());
        
        // Play card from hand (0-9)
        for (i, card) in self.state.hand.iter().enumerate().take(10) {
            // Status and Curse cards are never playable
            if card.card_type == crate::core::schema::CardType::Status ||
               card.card_type == crate::core::schema::CardType::Curse {
                mask[i] = false;
                continue;
            }
            
            // Must have enough energy
            if card.current_cost > energy {
                mask[i] = false;
                continue;
            }
            
            // Attack cards require at least one living enemy target
            if card.card_type == crate::core::schema::CardType::Attack && !has_living_enemies {
                mask[i] = false;
                continue;
            }
            
            mask[i] = true;
        }
        
        // End turn (10) - always valid in combat
        mask[10] = true;
        
        // Use potion (11-14) - check if slot has a potion
        // Note: Potions that require target selection will auto-target first enemy for now
        for (i, slot) in self.state.potions.slots().iter().enumerate().take(4) {
            if slot.is_some() {
                mask[ACTION_USE_POTION_START as usize + i] = true;
            }
        }
        
        // Discard potion (15-18) - can discard potions we have
        for (i, slot) in self.state.potions.slots().iter().enumerate().take(4) {
            if slot.is_some() {
                mask[ACTION_DISCARD_POTION_START as usize + i] = true;
            }
        }
    }
    
    /// Compute valid actions for Map screen.
    #[inline]
    fn compute_map_mask(&self, mask: &mut [bool]) {
        // Get available map moves
        let valid_moves = engine::get_valid_moves(&self.state);
        
        // Map node selection (20-29)
        for (i, _node_idx) in valid_moves.iter().enumerate().take(10) {
            mask[20 + i] = true;
        }
        
        // If no valid map moves (end of act or boss beaten), allow Proceed
        // This triggers act transition or game over check
        if valid_moves.is_empty() {
            mask[99] = true;
        }
    }
    
    /// Compute valid actions for Reward screen.
    #[inline]
    fn compute_reward_mask(&self, mask: &mut [bool]) {
        // Check each reward type and enable corresponding actions
        let mut has_card_reward = false;
        let mut has_gold_reward = false;
        let mut has_relic_reward = false;
        let mut has_potion_reward = false;
        
        for reward in &self.state.current_rewards {
            match reward {
                crate::rewards::RewardType::Card { .. } => has_card_reward = true,
                crate::rewards::RewardType::Gold { .. } => has_gold_reward = true,
                crate::rewards::RewardType::Relic { .. } => has_relic_reward = true,
                crate::rewards::RewardType::Potion { .. } => has_potion_reward = true,
            }
        }
        
        // Card rewards (30-32) - pick one of the 3 offered cards
        if has_card_reward {
            mask[30] = true;  // Pick card 0
            mask[31] = true;  // Pick card 1
            mask[32] = true;  // Pick card 2
        }
        
        // Gold reward (33) - always auto-collected, but we can enable action for clarity
        if has_gold_reward {
            mask[33] = true;  // Take gold
        }
        
        // Relic reward (34)
        if has_relic_reward {
            mask[34] = true;  // Take relic
        }
        
        // Potion reward (35)
        if has_potion_reward {
            mask[35] = true;  // Take potion
        }
        
        // Skip/Proceed (39/99) - only allow if no rewards OR after taking them
        // For now, allow skip but prioritize taking rewards
        mask[39] = true;
        mask[99] = true;
    }
    
    /// Compute valid actions for Shop screen.
    #[inline]
    fn compute_shop_mask(&self, mask: &mut [bool]) {
        let gold = self.state.gold;
        
        if let Some(ref shop) = self.state.shop_state {
            // Buy card (40-44)
            for (i, shop_card) in shop.cards.iter().enumerate().take(5) {
                mask[40 + i] = gold >= shop_card.price;
            }
            
            // Buy relic (45-47)
            for (i, shop_relic) in shop.relics.iter().enumerate().take(3) {
                mask[45 + i] = gold >= shop_relic.price;
            }
            
            // Buy potion (48-50)
            for (i, shop_potion) in shop.potions.iter().enumerate().take(3) {
                mask[48 + i] = gold >= shop_potion.price;
            }
            
            // Purge card (51-59) - requires gold and deck has cards
            // Purge cost increases with each purge: 50, 75, 100, etc.
            let purge_cost = 50 + (self.state.purge_count * 25);
            let can_purge = gold >= purge_cost && !self.state.master_deck.is_empty();
            if can_purge {
                // Enable purge for available master deck indices (simplified)
                let deck_size = self.state.master_deck.len().min(9);
                for i in 0..deck_size {
                    mask[51 + i] = true;
                }
            }
        }
        
        // Leave shop (99) - always valid
        mask[99] = true;
    }
    
    /// Compute valid actions for Campfire/Rest screen.
    #[inline]
    fn compute_campfire_mask(&self, mask: &mut [bool]) {
        let options = crate::campfire::get_available_options(&self.state);
        
        for option in options {
            match option {
                crate::campfire::CampfireOption::Rest => {
                    // Rest is valid if HP < Max HP (healing would be useful)
                    mask[60] = self.state.player.current_hp < self.state.player.max_hp;
                }
                crate::campfire::CampfireOption::Smith => {
                    // Smith is valid if there are upgradeable cards
                    let has_upgradeable = self.state.draw_pile.iter()
                        .any(|c| !c.upgraded);
                    mask[61] = has_upgradeable;
                }
                crate::campfire::CampfireOption::Lift => {
                    mask[62] = true;
                }
                crate::campfire::CampfireOption::Toke => {
                    // Toke valid if deck has removable cards
                    mask[63] = !self.state.draw_pile.is_empty();
                }
                crate::campfire::CampfireOption::Dig => {
                    mask[64] = true;
                }
                crate::campfire::CampfireOption::Recall => {
                    mask[65] = true;
                }
            }
        }
        
        // Smith target selection (70-89) - if Smith was selected
        // This would need state tracking for multi-step actions
        // For now, targets are auto-selected when Smith is chosen
        
        // Proceed/Leave (99) - can leave campfire without acting
        mask[99] = true;
    }
    
    /// Compute valid actions for Event screen.
    #[inline]
    fn compute_event_mask(&self, mask: &mut [bool]) {
        // Events have up to 4 choices (90-93)
        // In our simplified model, we enable choice 0 and proceed
        mask[90] = true;  // First choice usually available
        mask[99] = true;  // Proceed/skip
        
        // TODO: Track actual event choices when event system is enhanced
    }
    
    /// Handle combat actions: play cards, end turn, select targets.
    fn handle_combat_action(&mut self, action_id: i32) -> f32 {
        // End turn action
        if action_id == ACTION_END_TURN {
            return self.execute_end_turn();
        }
        
        // Play card from hand
        if action_id >= ACTION_PLAY_CARD_START && action_id <= ACTION_PLAY_CARD_END {
            let hand_index = (action_id - ACTION_PLAY_CARD_START) as usize;
            return self.execute_play_card(hand_index, None);
        }
        
        // Use potion (11-14)
        if action_id >= ACTION_USE_POTION_START && action_id <= ACTION_USE_POTION_END {
            let slot_idx = (action_id - ACTION_USE_POTION_START) as usize;
            return self.execute_use_potion(slot_idx);
        }
        
        // Discard potion (15-18)
        if action_id >= ACTION_DISCARD_POTION_START && action_id <= ACTION_DISCARD_POTION_END {
            let slot_idx = (action_id - ACTION_DISCARD_POTION_START) as usize;
            return self.execute_discard_potion(slot_idx);
        }
        
        // Invalid action for Combat screen
        REWARD_INVALID_ACTION
    }
    
    /// Execute playing a card from hand.
    fn execute_play_card(&mut self, hand_index: usize, target_idx: Option<usize>) -> f32 {
        // Validate hand index
        if hand_index >= self.state.hand.len() {
            return REWARD_INVALID_ACTION;
        }
        
        // Check energy cost
        let card_cost = self.state.hand[hand_index].current_cost;
        if card_cost > self.state.player.energy {
            return REWARD_INVALID_ACTION;
        }
        
        // Record HP before attack for reward calculation
        let enemy_hp_before: i32 = self.state.enemies.iter()
            .map(|e| e.hp.max(0))
            .sum();
        
        // Try to play the card
        let target = target_idx.or_else(|| {
            // Auto-target first non-dead enemy
            self.state.enemies.iter()
                .position(|e| !e.is_dead())
        });
        
        match engine::play_card_from_hand(&mut self.state, &self.card_library, hand_index, target) {
            Ok(_results) => {
                // Calculate damage dealt for reward
                let enemy_hp_after: i32 = self.state.enemies.iter()
                    .map(|e| e.hp.max(0))
                    .sum();
                let damage_dealt = (enemy_hp_before - enemy_hp_after).max(0);
                let damage_reward = damage_dealt as f32 * REWARD_DAMAGE_DEALT;
                
                // Check for combat victory
                if engine::all_enemies_dead(&self.state) {
                    self.on_combat_won();
                    return REWARD_COMBAT_WIN + damage_reward;
                }
                
                REWARD_VALID_ACTION + damage_reward
            }
            Err(_) => REWARD_INVALID_ACTION,
        }
    }
    
    /// Execute ending the turn.
    fn execute_end_turn(&mut self) -> f32 {
        // Record HP before enemy turn
        let hp_before = self.state.player.current_hp;
        
        // === P0 Fix: Fire end-of-turn power hooks BEFORE discard ===
        // Java order: callEndOfTurnActions() → applyEndOfTurnRelics, atEndOfTurnPreEndTurnCards,
        //             triggerOnEndOfTurnForPlayingCard, then DiscardAtEndOfTurnAction
        // This fires Metallicize, PlatedArmor, Regen, Combust, Constricted, etc.
        engine::on_turn_end(&mut self.state, &self.card_library, self.relic_library.as_ref());
        
        // === P0 Fix: Burn/Decay card end-of-turn triggers ===
        // Java: DiscardAtEndOfTurnAction → card.triggerOnEndOfPlayerTurn()
        // Burn deals 2 damage to player, Decay deals 2 damage to player
        {
            let mut burn_damage = 0;
            for card in self.state.hand.iter() {
                match card.definition_id.as_str() {
                    "Burn" => burn_damage += 2,
                    "Burn+" => burn_damage += 4,
                    "Decay" => burn_damage += 2,
                    _ => {}
                }
            }
            if burn_damage > 0 {
                self.state.player.current_hp -= burn_damage;
                game_log!("  🔥 Burn/Decay end-of-turn damage: {} HP", burn_damage);
            }
        }
        
        // Check for player death from end-of-turn effects
        if self.state.player.current_hp <= 0 {
            self.state.screen = GamePhase::GameOver;
            return REWARD_GAME_LOSS;
        }
        
        // End player turn (discard hand, decrement statuses)
        self.state.end_turn();
        
        // Execute enemy turn if we have monster library
        if let Some(ref monster_lib) = self.monster_library {
            engine::execute_enemy_turn(&mut self.state, monster_lib);
        } else {
            // Simplified enemy turn without library
            self.execute_simple_enemy_turn();
        }
        
        // Check for player death
        if self.state.player.current_hp <= 0 {
            self.state.screen = GamePhase::GameOver;
            return REWARD_GAME_LOSS;
        }
        
        // Combat turn limit to prevent infinite loops
        const MAX_COMBAT_TURNS: u32 = 100;
        if self.state.turn >= MAX_COMBAT_TURNS {
            game_log!("WARNING: Combat exceeded {} turns. Ending game to prevent deadlock.", MAX_COMBAT_TURNS);
            self.state.screen = GamePhase::GameOver;
            return REWARD_GAME_LOSS;
        }
        
        // Start new turn
        self.state.start_turn();
        
        // === P1.11: Fire start-of-turn power + relic hooks ===
        // Java: applyStartOfTurnRelics, applyStartOfTurnPowers, applyStartOfTurnCards
        // This fires DemonForm (+STR), Berserk (+energy), Brutality (-1HP +draw),
        // Brimstone (+1 STR all enemies), SneckoEye cost randomization, etc.
        engine::on_turn_start(&mut self.state, &self.card_library, self.relic_library.as_ref());
        
        // Calculate damage taken penalty
        let damage_taken = (hp_before - self.state.player.current_hp).max(0);
        let damage_penalty = damage_taken as f32 * REWARD_DAMAGE_TAKEN;
        
        REWARD_VALID_ACTION + damage_penalty
    }
    
    /// Execute using a potion from a slot.
    fn execute_use_potion(&mut self, slot_idx: usize) -> f32 {
        // Validate slot index and get potion ID
        let potion_id = match self.state.potions.get(slot_idx) {
            Ok(Some(id)) => id.clone(),
            _ => return REWARD_INVALID_ACTION,
        };
        
        // Get potion definition from library
        let potion_def = match &self.potion_library {
            Some(lib) => match lib.get(&potion_id) {
                Ok(def) => def.clone(),
                Err(_) => return REWARD_INVALID_ACTION,
            },
            None => return REWARD_INVALID_ACTION,
        };
        
        // Determine target for enemy-targeting potions
        let target_idx = if potion_def.requires_target() {
            // Auto-target first non-dead enemy
            self.state.enemies.iter()
                .position(|e| !e.is_dead())
        } else {
            None
        };
        
        // Check if Sacred Bark relic doubles potion effectiveness
        let has_sacred_bark = self.state.relics.iter()
            .any(|r| r.id == "SacredBark");
        
        // Record state before using potion for reward calculation
        let hp_before = self.state.player.current_hp;
        let enemy_hp_before: i32 = self.state.enemies.iter()
            .map(|e| e.hp.max(0))
            .sum();
        
        // Apply potion effects using the engine
        match engine::use_potion(&mut self.state, &potion_def, target_idx, has_sacred_bark) {
            Ok(_) => {
                // Remove potion from slot
                let _ = self.state.potions.remove(slot_idx);
                
                // Calculate reward: damage dealt + healing
                let hp_after = self.state.player.current_hp;
                let enemy_hp_after: i32 = self.state.enemies.iter()
                    .map(|e| e.hp.max(0))
                    .sum();
                
                let damage_dealt = (enemy_hp_before - enemy_hp_after).max(0);
                let healing = (hp_after - hp_before).max(0);
                
                let reward = (damage_dealt as f32 * REWARD_DAMAGE_DEALT) + 
                             (healing as f32 * 0.01); // Small reward for healing
                
                // Check for combat victory
                if engine::all_enemies_dead(&self.state) {
                    self.on_combat_won();
                    return REWARD_COMBAT_WIN + reward;
                }
                
                REWARD_VALID_ACTION + reward
            }
            Err(_) => REWARD_INVALID_ACTION,
        }
    }
    
    /// Execute discarding a potion from a slot.
    fn execute_discard_potion(&mut self, slot_idx: usize) -> f32 {
        match self.state.potions.discard(slot_idx) {
            Ok(_) => REWARD_VALID_ACTION, // Small penalty for wasting a potion?
            Err(_) => REWARD_INVALID_ACTION,
        }
    }
    
    /// Simplified enemy turn when no monster library is available.
    fn execute_simple_enemy_turn(&mut self) {
        for enemy in self.state.enemies.iter_mut() {
            if enemy.is_dead() {
                continue;
            }
            
            // Simple attack based on enemy intent
            let damage = match &enemy.current_intent {
                crate::enemy::Intent::Attack { damage, .. } => *damage,
                crate::enemy::Intent::AttackDebuff { damage, .. } => *damage,
                crate::enemy::Intent::AttackDefend { damage, .. } => *damage,
                _ => 0,
            };
            
            if damage > 0 {
                let blocked = damage.min(self.state.player.block);
                self.state.player.block -= blocked;
                let actual_damage = damage - blocked;
                self.state.player.current_hp -= actual_damage;
            }
        }
    }
    
    /// Handle combat victory.
    fn on_combat_won(&mut self) {
        engine::on_battle_end(&mut self.state, true, None); // TODO: wire relic_library
        
        // Increment combat count for encounter pool progression
        self.state.combat_count += 1;
        
        // Check if we beat the boss
        let current_room_type = self.get_current_room_type();
        let is_boss_fight = current_room_type == Some(crate::map::RoomType::Boss);
        
        // Determine room type for rewards
        let room_type = current_room_type.unwrap_or(crate::map::RoomType::Monster);
        
        // Generate rewards based on room type
        let rewards = crate::rewards::generate_rewards(
            &mut self.state, 
            room_type,
            Some(&self.card_library),
            Some(crate::schema::CardColor::Red), // Ironclad
        );
        
        // Store rewards in state for mask computation
        self.state.current_rewards = rewards.rewards;
        self.state.rewards_pending = true;
        self.state.screen = GamePhase::Reward;
        
        // Debug: show what rewards were generated
        let fight_type = if is_boss_fight { "Boss" } else { "Combat" };
        game_log!("🏁 {} End - Triggering Relics", fight_type);
        game_log!("  💰 Rewards generated: {} items", self.state.current_rewards.len());
        for (i, reward) in self.state.current_rewards.iter().enumerate() {
            match reward {
                crate::rewards::RewardType::Gold { amount } => game_log!("    [{i}] Gold: {amount}"),
                crate::rewards::RewardType::Card { cards } => game_log!("    [{i}] Cards: {} choices", cards.len()),
                crate::rewards::RewardType::Relic { id, .. } => game_log!("    [{i}] Relic: {id}"),
                crate::rewards::RewardType::Potion { id, .. } => game_log!("    [{i}] Potion: {id}"),
            }
        }
    }
    
    /// Get the room type of the current map node.
    fn get_current_room_type(&self) -> Option<crate::map::RoomType> {
        let map = self.state.map.as_ref()?;
        let node_idx = self.state.current_map_node?;
        map.get(node_idx).map(|n| n.room_type)
    }
    
    /// Handle map navigation actions.
    fn handle_map_action(&mut self, action_id: i32) -> f32 {
        // Map node selection
        if action_id >= ACTION_MAP_SELECT_START && action_id <= ACTION_MAP_SELECT_END {
            let child_index = (action_id - ACTION_MAP_SELECT_START) as usize;
            return self.execute_map_select(child_index);
        }
        
        // Proceed - handle end of act / no valid moves
        if action_id == ACTION_PROCEED {
            let valid_moves = engine::get_valid_moves(&self.state);
            if valid_moves.is_empty() {
                // No more nodes to visit - check if we should advance act or end game
                return self.handle_act_transition();
            }
            return REWARD_INVALID_ACTION;
        }
        
        REWARD_INVALID_ACTION
    }
    
    /// Handle act transition when map has no more valid moves.
    fn handle_act_transition(&mut self) -> f32 {
        // Check current floor to determine if we beat the boss
        let floor = self.state.floor;
        
        // Boss floors are typically floor 16 (Act 1), 33 (Act 2), 50 (Act 3)
        // Simplified: if floor >= 15 and no moves, consider it act complete
        if floor >= 15 {
            // Advance to next act
            if self.state.act < 3 {
                self.state.act += 1;
                self.state.floor = 0;
                self.state.current_map_node = None;
                
                // Reset combat count for new act (affects encounter pool selection)
                self.state.combat_count = 0;
                
                // Reset meta-scaling RNG for the new act
                crate::rewards::reset_act_rng(&mut self.state);
                
                // TODO: Heal player based on act transition rules
                // let heal_percent = crate::dungeon::get_act_entry_heal_percent(self.state.act, 0);
                // let heal_amount = (self.state.player.max_hp * heal_percent as i32) / 100;
                // self.state.player.current_hp = (self.state.player.current_hp + heal_amount).min(self.state.player.max_hp);
                
                // Generate new map for next act
                use crate::map::{generate_map, SimpleMap};
                let new_map = generate_map(self.seed as i64, self.state.act, true);
                self.state.map = Some(SimpleMap::from_map(&new_map));
                
                game_log!("🏆 Act {} complete! Entering Act {}. Combat count reset.", self.state.act - 1, self.state.act);
                
                // Small reward for completing an act
                return 5.0;
            } else {
                // Act 3 complete - game won!
                self.state.screen = GamePhase::GameOver;
                return REWARD_GAME_WIN;
            }
        }
        
        // Floor not high enough - this shouldn't happen, but handle gracefully
        // Maybe map generation issue - trigger game over to avoid deadlock
        game_log!("WARNING: Map has no valid moves at floor {} act {}. Ending game.", floor, self.state.act);
        self.state.screen = GamePhase::GameOver;
        REWARD_GAME_LOSS
    }
    
    /// Execute map node selection.
    fn execute_map_select(&mut self, child_index: usize) -> f32 {
        // Get valid moves
        let valid_moves = engine::get_valid_moves(&self.state);
        
        // Check if child_index is valid
        if child_index >= valid_moves.len() {
            return REWARD_INVALID_ACTION;
        }
        
        let node_index = valid_moves[child_index];
        
        // Navigate to the node
        match engine::proceed_to_node(&mut self.state, node_index, &self.card_library) {
            engine::NodeResult::Combat { encounter_id, is_elite, is_boss } => {
                // Spawn enemies for combat
                self.spawn_combat_enemies(&encounter_id, is_elite, is_boss);
                REWARD_VALID_ACTION
            }
            engine::NodeResult::Shop => {
                // Shop is already set up by proceed_to_node
                REWARD_VALID_ACTION
            }
            engine::NodeResult::Rest => {
                REWARD_VALID_ACTION
            }
            engine::NodeResult::Treasure => {
                // Treasure gives rewards
                REWARD_VALID_ACTION
            }
            engine::NodeResult::Event { .. } => {
                REWARD_VALID_ACTION
            }
            engine::NodeResult::InvalidMove { .. } | 
            engine::NodeResult::NoMap | 
            engine::NodeResult::GameOver => {
                REWARD_INVALID_ACTION
            }
        }
    }
    
    /// Spawn enemies for a combat encounter.
    /// 
    /// Uses `dungeon::spawn_encounter` to convert encounter IDs (like "2 Louse", "Gremlin Gang")
    /// into actual monster lists with proper random variants.
    fn spawn_combat_enemies(&mut self, encounter_id: &str, _is_elite: bool, _is_boss: bool) {
        use crate::enemy::MonsterState;
        use crate::dungeon::spawn_encounter;
        
        // Clear any existing enemies
        self.state.enemies.clear();
        
        // Convert encounter ID to monster list
        let monster_spawns = spawn_encounter(&mut self.state.rng, encounter_id);
        
        game_log!("⚔️ Spawning encounter: {} ({} monsters)", encounter_id, monster_spawns.len());
        
        // Spawn each monster
        for spawn in monster_spawns {
            let monster = if let Some(ref monster_lib) = self.monster_library {
                if let Ok(monster_def) = monster_lib.get(&spawn.monster_id) {
                    // Spawn from definition with proper HP range
                    let mut monster = MonsterState::new(
                        monster_def,
                        &mut self.state.rng,
                        0, // TODO: Add ascension tracking to PyStsSim
                    );
                    // Apply HP override if specified
                    if let Some(hp) = spawn.hp_override {
                        monster.hp = hp;
                        monster.max_hp = hp;
                    }
                    game_log!("   🦑 {} (HP: {}/{})", monster.name, monster.hp, monster.max_hp);
                    monster
                } else {
                    // Fallback to simple monster
                    game_log!("   ⚠️ {} not found in library, using simple monster", spawn.monster_id);
                    MonsterState::new_simple(&spawn.monster_id, spawn.hp_override.unwrap_or(50))
                }
            } else {
                // No library, create simple monster
                MonsterState::new_simple(&spawn.monster_id, spawn.hp_override.unwrap_or(50))
            };
            
            self.state.enemies.push(monster);
        }
        
        // Set up initial starter deck if master_deck is empty (first game or after reset)
        if self.state.master_deck.is_empty() {
            self.setup_starter_deck_self();
        }
        
        // Initialize combat deck: copy master_deck → draw_pile, shuffle, move Innate to top
        // This MUST be called at every combat start to properly set up card piles
        self.state.initialize_combat_deck();
        
        // Fire battle start hooks (relics, powers, etc.)
        engine::on_battle_start(&mut self.state, &self.card_library, None); // TODO: wire relic_library
        
        // Start first turn (resets energy, draws 5 cards)
        self.state.start_turn();
        
        // Fire start-of-turn power + relic hooks (DemonForm, Brutality, etc.)
        engine::on_turn_start(&mut self.state, &self.card_library, self.relic_library.as_ref());
    }
    
    /// Set up a starter deck for Ironclad (static version for reset).
    /// Ironclad starter deck: 5x Strike, 4x Defend, 1x Bash = 10 cards total.
    fn setup_starter_deck(state: &mut GameState, card_library: &CardLibrary) {
        use crate::schema::CardInstance;
        
        state.master_deck.clear();
        
        // 5 Strikes → master_deck (permanent)
        for _ in 0..5 {
            let def = card_library.get("Strike_Ironclad")
                .expect("Strike_Ironclad must exist in card library");
            state.master_deck.push(CardInstance::new_basic(&def.id, def.cost));
        }
        
        // 4 Defends → master_deck (permanent)
        for _ in 0..4 {
            let def = card_library.get("Defend_Ironclad")
                .expect("Defend_Ironclad must exist in card library");
            state.master_deck.push(CardInstance::new_basic(&def.id, def.cost));
        }
        
        // 1 Bash → master_deck (permanent)
        let def = card_library.get("Bash")
            .expect("Bash must exist in card library");
        state.master_deck.push(CardInstance::new_basic(&def.id, def.cost));
    }
    
    /// Set up a starter deck for Ironclad (instance method).
    fn setup_starter_deck_self(&mut self) {
        Self::setup_starter_deck(&mut self.state, &self.card_library);
    }
    
    /// Handle reward selection actions.
    fn handle_reward_action(&mut self, action_id: i32) -> f32 {
        use crate::rewards::RewardType;
        use crate::schema::CardInstance;
        
        // Card selection (30-32)
        if action_id >= 30 && action_id <= 32 {
            let card_index = (action_id - 30) as usize;
            
            // Find card reward in current_rewards — extract info before mutable borrow
            let card_to_add: Option<(String, i32, usize)> = {
                if let Some(pos) = self.state.current_rewards.iter().position(|r| {
                    matches!(r, RewardType::Card { .. })
                }) {
                    if let RewardType::Card { cards } = &self.state.current_rewards[pos] {
                        if card_index < cards.len() {
                            let card_id = cards[card_index].id.clone();
                            if let Ok(def) = self.card_library.get(&card_id) {
                                Some((def.id.clone(), def.cost, pos))
                            } else {
                                None
                            }
                        } else { None }
                    } else { None }
                } else { None }
            };
            
            if let Some((card_id, cost, pos)) = card_to_add {
                let card = CardInstance::new_basic(&card_id, cost);
                self.state.obtain_card(card);
                game_log!("  📦 Added card to master deck: {}", card_id);
                self.state.current_rewards.remove(pos);
                return 0.5;
            }
            return REWARD_INVALID_ACTION;
        }
        
        // Gold collection (33) - auto-collect all gold
        if action_id == 33 {
            let mut gold_taken = 0;
            self.state.current_rewards.retain(|r| {
                if let RewardType::Gold { amount } = r {
                    self.state.gold += amount;
                    gold_taken += amount;
                    game_log!("  💰 Collected {} gold (total: {})", amount, self.state.gold);
                    false // Remove this reward
                } else {
                    true // Keep other rewards
                }
            });
            if gold_taken > 0 {
                return 0.1; // Small reward for collecting gold
            }
            return REWARD_INVALID_ACTION;
        }
        
        // Relic collection (34)
        if action_id == 34 {
            if let Some(pos) = self.state.current_rewards.iter().position(|r| {
                matches!(r, RewardType::Relic { .. })
            }) {
                if let RewardType::Relic { id, .. } = self.state.current_rewards[pos].clone() {
                    game_log!("  🏺 Collected relic: {}", id);
                    // Add relic to player's collection
                    self.state.relics.push(crate::items::relics::RelicInstance::new(&id));
                    crate::items::relics::on_relic_equip(&mut self.state, &id);
                }
                self.state.current_rewards.remove(pos);
                return 1.0; // Good reward for relics
            }
            return REWARD_INVALID_ACTION;
        }
        
        // Potion collection (35)
        if action_id == 35 {
            if let Some(pos) = self.state.current_rewards.iter().position(|r| {
                matches!(r, RewardType::Potion { .. })
            }) {
                if let RewardType::Potion { id, .. } = self.state.current_rewards[pos].clone() {
                    game_log!("  🧪 Collected potion: {}", id);
                    // TODO: Add potion slots to GameState and store the potion
                    // For now, just acknowledge the collection
                }
                self.state.current_rewards.remove(pos);
                return 0.2; // Small reward for potions
            }
            return REWARD_INVALID_ACTION;
        }
        
        // Skip (39) - skip remaining rewards
        if action_id == 39 {
            // Clear remaining rewards (player chose to skip)
            if !self.state.current_rewards.is_empty() {
                game_log!("  ⏭️ Skipped {} remaining rewards", self.state.current_rewards.len());
            }
            self.state.current_rewards.clear();
            engine::finish_rewards(&mut self.state);
            return REWARD_VALID_ACTION;
        }
        
        // Proceed (99) - finish reward screen
        if action_id == ACTION_PROCEED {
            self.state.current_rewards.clear();
            engine::finish_rewards(&mut self.state);
            return REWARD_VALID_ACTION;
        }
        
        REWARD_INVALID_ACTION
    }
    
    /// Handle shop actions.
    fn handle_shop_action(&mut self, action_id: i32) -> f32 {
        // Buy card
        if action_id >= ACTION_SHOP_BUY_CARD_START && action_id <= ACTION_SHOP_BUY_CARD_END {
            let card_index = (action_id - ACTION_SHOP_BUY_CARD_START) as usize;
            return match self.state.buy_card(card_index) {
                ShopResult::Success => REWARD_VALID_ACTION,
                _ => REWARD_INVALID_ACTION,
            };
        }
        
        // Buy relic
        if action_id >= ACTION_SHOP_BUY_RELIC_START && action_id <= ACTION_SHOP_BUY_RELIC_END {
            let relic_index = (action_id - ACTION_SHOP_BUY_RELIC_START) as usize;
            return match self.state.buy_relic(relic_index, self.relic_library.as_ref()) {
                ShopResult::Success => REWARD_VALID_ACTION,
                _ => REWARD_INVALID_ACTION,
            };
        }
        
        // Buy potion
        if action_id >= ACTION_SHOP_BUY_POTION_START && action_id <= ACTION_SHOP_BUY_POTION_END {
            let potion_index = (action_id - ACTION_SHOP_BUY_POTION_START) as usize;
            return match self.state.buy_potion(potion_index) {
                ShopResult::Success => REWARD_VALID_ACTION,
                _ => REWARD_INVALID_ACTION,
            };
        }
        
        // Purge card
        if action_id >= ACTION_SHOP_PURGE_START && action_id <= ACTION_SHOP_PURGE_END {
            let deck_index = (action_id - ACTION_SHOP_PURGE_START) as usize;
            return match self.state.purge_card(deck_index) {
                ShopResult::Success => REWARD_VALID_ACTION,
                _ => REWARD_INVALID_ACTION,
            };
        }
        
        // Leave shop
        if action_id == ACTION_PROCEED {
            engine::leave_shop(&mut self.state);
            return REWARD_VALID_ACTION;
        }
        
        REWARD_INVALID_ACTION
    }
    
    /// Handle campfire (rest site) actions.
    fn handle_campfire_action(&mut self, action_id: i32) -> f32 {
        let available_options = get_available_options(&self.state);
        
        // Rest
        if action_id == ACTION_CAMPFIRE_REST {
            if available_options.contains(&CampfireOption::Rest) {
                execute_campfire_option(&mut self.state, CampfireOption::Rest, None, Some(&self.card_library));
                engine::leave_rest(&mut self.state);
                return REWARD_VALID_ACTION;
            }
            return REWARD_INVALID_ACTION;
        }
        
        // Smith (needs target - simplified: upgrade first upgradeable card)
        if action_id == ACTION_CAMPFIRE_SMITH {
            if available_options.contains(&CampfireOption::Smith) {
                // Find first upgradeable card
                let target = self.state.draw_pile.iter()
                    .chain(self.state.discard_pile.iter())
                    .position(|c| !c.upgraded);
                execute_campfire_option(&mut self.state, CampfireOption::Smith, target, Some(&self.card_library));
                engine::leave_rest(&mut self.state);
                return REWARD_VALID_ACTION;
            }
            return REWARD_INVALID_ACTION;
        }
        
        // Lift (Girya)
        if action_id == ACTION_CAMPFIRE_LIFT {
            if available_options.contains(&CampfireOption::Lift) {
                execute_campfire_option(&mut self.state, CampfireOption::Lift, None, None);
                engine::leave_rest(&mut self.state);
                return REWARD_VALID_ACTION;
            }
            return REWARD_INVALID_ACTION;
        }
        
        // Toke (Peace Pipe - remove curse)
        if action_id == ACTION_CAMPFIRE_TOKE {
            if available_options.contains(&CampfireOption::Toke) {
                // Find first curse to remove (curses typically have definition_id starting with curse-related names)
                let target = self.state.draw_pile.iter()
                    .position(|c| c.definition_id.starts_with("Curse") || c.definition_id.contains("Curse"));
                execute_campfire_option(&mut self.state, CampfireOption::Toke, target, None);
                engine::leave_rest(&mut self.state);
                return REWARD_VALID_ACTION;
            }
            return REWARD_INVALID_ACTION;
        }
        
        // Dig (Shovel)
        if action_id == ACTION_CAMPFIRE_DIG {
            if available_options.contains(&CampfireOption::Dig) {
                execute_campfire_option(&mut self.state, CampfireOption::Dig, None, None);
                engine::leave_rest(&mut self.state);
                return REWARD_VALID_ACTION;
            }
            return REWARD_INVALID_ACTION;
        }
        
        // Recall
        if action_id == ACTION_CAMPFIRE_RECALL {
            if available_options.contains(&CampfireOption::Recall) {
                execute_campfire_option(&mut self.state, CampfireOption::Recall, None, None);
                engine::leave_rest(&mut self.state);
                return REWARD_VALID_ACTION;
            }
            return REWARD_INVALID_ACTION;
        }
        
        // Target selection for Smith/Toke
        if action_id >= ACTION_CAMPFIRE_TARGET_START && action_id <= ACTION_CAMPFIRE_TARGET_END {
            // This would be handled with a two-step action system
            // For now, return invalid
            return REWARD_INVALID_ACTION;
        }
        
        // Leave without action
        if action_id == ACTION_PROCEED {
            engine::leave_rest(&mut self.state);
            return REWARD_VALID_ACTION;
        }
        
        REWARD_INVALID_ACTION
    }
    
    /// Handle event actions.
    fn handle_event_action(&mut self, action_id: i32) -> f32 {
        // Event choice selection (actions 90-93)
        if action_id >= ACTION_EVENT_CHOICE_START && action_id <= ACTION_EVENT_CHOICE_END {
            let choice_index = (action_id - ACTION_EVENT_CHOICE_START) as usize;
            
            // Execute the event option
            match engine::execute_event_option(&mut self.state, choice_index) {
                Ok(result) => {
                    match result {
                        engine::EventProcessResult::Complete => {
                            engine::finish_event(&mut self.state);
                        }
                        engine::EventProcessResult::Continue => {
                            // Event continues, stay on Event screen
                        }
                        engine::EventProcessResult::AwaitingCardSelect => {
                            // Screen already changed to CardSelect
                        }
                        engine::EventProcessResult::StartCombat { .. } => {
                            // Screen already changed to Combat
                        }
                        engine::EventProcessResult::PlayerDied => {
                            // Screen already changed to GameOver
                        }
                    }
                    return REWARD_VALID_ACTION;
                }
                Err(_) => {
                    return REWARD_INVALID_ACTION;
                }
            }
        }
        
        // Proceed / Skip - finish event
        if action_id == ACTION_PROCEED {
            engine::finish_event(&mut self.state);
            return REWARD_VALID_ACTION;
        }
        
        REWARD_INVALID_ACTION
    }
    
    /// Handle card selection actions.
    fn handle_card_select_action(&mut self, action_id: i32) -> f32 {
        // Card selection (actions 0-9 for selecting from pool)
        if action_id >= 0 && action_id < 10 {
            let card_idx = action_id as usize;
            
            // Check if index is valid in the selection pool
            if card_idx < self.state.card_select_pool.len() {
                let actual_idx = self.state.card_select_pool[card_idx];
                
                // Complete the selection with this single card
                engine::complete_card_selection(&mut self.state, &[actual_idx]);
                return REWARD_VALID_ACTION;
            }
        }
        
        // Proceed / Skip - cancel selection or complete with empty
        if action_id == ACTION_PROCEED {
            engine::complete_card_selection(&mut self.state, &[]);
            return REWARD_VALID_ACTION;
        }
        
        REWARD_INVALID_ACTION
    }
    
    /// Compute card selection mask.
    fn compute_card_select_mask(&self, mask: &mut [bool]) {
        // Enable selecting from the card pool (actions 0-9)
        for (i, _) in self.state.card_select_pool.iter().enumerate() {
            if i < 10 {
                mask[i] = true;
            }
        }
        
        // Always allow proceeding (skip/cancel)
        mask[ACTION_PROCEED as usize] = true;
    }
}

// ============================================================================
// OpenAI Gym-style Environment
// ============================================================================

/// The main Python-facing environment class.
/// Follows OpenAI Gym interface conventions for RL training.
#[pyclass(name = "StsEnv")]
pub struct StsEnv {
    state: GameState,
    library: CardLibrary,
    done: bool,
    total_reward: f64,
    seed: u64,
}

#[pymethods]
impl StsEnv {
    /// Create a new environment with optional seed.
    ///
    /// Args:
    ///     seed: Random seed for deterministic simulation (default: 42)
    ///     cards_path: Path to cards.json file (default: "data/cards.json")
    #[new]
    #[pyo3(signature = (seed=None, cards_path=None))]
    fn new(seed: Option<u64>, cards_path: Option<&str>) -> PyResult<Self> {
        let seed = seed.unwrap_or(42);
        let path = cards_path.unwrap_or("data/cards");

        let library = CardLibrary::load(path)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;

        let state = GameState::new(seed);

        Ok(Self {
            state,
            library,
            done: false,
            total_reward: 0.0,
            seed,
        })
    }

    /// Reset the environment to initial state.
    ///
    /// Args:
    ///     seed: Optional new seed for this episode
    ///
    /// Returns:
    ///     observation dict containing game state
    #[pyo3(signature = (seed=None))]
    fn reset(&mut self, seed: Option<u64>, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let seed = seed.unwrap_or(self.seed.wrapping_add(1));
        self.seed = seed;

        self.state = GameState::new(seed);

        // Setup enemy (use simple constructor for testing)
        use crate::enemy::MonsterState;
        self.state.enemies.push(MonsterState::new_simple("Jaw Worm", 44));

        // Setup starter deck (into master_deck)
        Self::setup_starter_deck(&mut self.state);
        
        // Initialize combat deck (copy master_deck → draw_pile)
        self.state.initialize_combat_deck();

        // Start first turn
        self.state.start_turn();

        self.done = false;
        self.total_reward = 0.0;

        self.get_observation(py)
    }

    /// Take a step in the environment.
    ///
    /// Args:
    ///     action: Index of card in hand to play (0-9), or 10 to end turn
    ///
    /// Returns:
    ///     Tuple of (observation, reward, done, truncated, info)
    fn step(&mut self, action: i32, py: Python<'_>) -> PyResult<Py<PyAny>> {
        if self.done {
            return Err(PyErr::new::<pyo3::exceptions::PyRuntimeError, _>(
                "Episode is done. Call reset() first.",
            ));
        }

        let mut reward: f64 = 0.0;
        let info = PyDict::new(py);

        if action == 10 {
            // End turn action
            self.state.end_turn();

            // Enemy turn (simplified: deal damage based on intent)
            let enemy_damage = 10i32;
            let blocked = enemy_damage.min(self.state.player.block);
            self.state.player.block -= blocked;
            let actual_damage = enemy_damage - blocked;
            self.state.player.current_hp -= actual_damage;

            // Reward shaping: penalize taking damage
            reward -= actual_damage as f64 * 0.1;

            // Check if player died
            if self.state.player.current_hp <= 0 {
                self.done = true;
                reward -= 10.0; // Death penalty
                info.set_item("reason", "player_died")?;
            } else {
                // Start new turn
                self.state.start_turn();
            }
        } else if action >= 0 && (action as usize) < self.state.hand.len() {
            let hand_idx = action as usize;

            // Check energy
            let card_cost = self.state.hand[hand_idx].current_cost;
            if card_cost <= self.state.player.energy {
                match engine::play_card_from_hand(&mut self.state, &self.library, hand_idx, Some(0))
                {
                    Ok(_) => {
                        // Small reward for playing cards
                        reward += 0.1;

                        // Check win condition
                        if self.state.combat_won() {
                            self.done = true;
                            reward += 10.0; // Win bonus
                            info.set_item("reason", "combat_won")?;
                        }
                    }
                    Err(e) => {
                        info.set_item("error", e)?;
                        reward -= 0.5; // Penalty for invalid action
                    }
                }
            } else {
                reward -= 0.5;
                info.set_item("error", "not_enough_energy")?;
            }
        } else {
            reward -= 1.0;
            info.set_item("error", "invalid_action")?;
        }

        self.total_reward += reward;

        let obs = self.get_observation(py)?;
        let truncated = false;

        // Return Gym-style tuple: (obs, reward, done, truncated, info)
        Ok((obs, reward, self.done, truncated, info.unbind()).into_pyobject(py)?.into_any().unbind())
    }

    /// Get current observation as a dict.
    fn observation(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        self.get_observation(py)
    }

    /// Get the action space size (10 cards + 1 end turn).
    fn action_space_size(&self) -> usize {
        11
    }

    /// Get valid actions mask as a list of booleans.
    ///
    /// Returns:
    ///     List of 11 booleans indicating which actions are valid
    fn valid_actions(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let mut mask = vec![false; 11];

        // Check which cards can be played
        for (i, card) in self.state.hand.iter().enumerate() {
            if card.current_cost <= self.state.player.energy {
                mask[i] = true;
            }
        }

        // End turn is always valid (action 10)
        mask[10] = true;

        Ok(mask.into_pyobject(py)?.into_any().unbind())
    }

    /// Get total accumulated reward for this episode.
    fn total_reward(&self) -> f64 {
        self.total_reward
    }

    /// Render current state as a human-readable string.
    fn render(&self) -> String {
        format!("{}", self.state)
    }

    /// Get number of cards currently in hand.
    fn hand_size(&self) -> usize {
        self.state.hand.len()
    }

    /// Check if episode is done.
    fn is_done(&self) -> bool {
        self.done
    }
}

impl StsEnv {
    /// Setup Ironclad starter deck.
    fn setup_starter_deck(state: &mut GameState) {
        // 5 Strikes (cost 1)
        for _ in 0..5 {
            state.obtain_card(CardInstance::new("Strike_Ironclad".to_string(), 1));
        }
        // 4 Defends (cost 1)
        for _ in 0..4 {
            state.obtain_card(CardInstance::new("Defend_Ironclad".to_string(), 1));
        }
        // 1 Bash (cost 2)
        state.obtain_card(CardInstance::new("Bash".to_string(), 2));
    }

    /// Build observation dictionary for Python.
    fn get_observation(&self, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let obs = PyDict::new(py);

        // Player stats
        obs.set_item("player_hp", self.state.player.current_hp)?;
        obs.set_item("player_max_hp", self.state.player.max_hp)?;
        obs.set_item("player_block", self.state.player.block)?;
        obs.set_item("energy", self.state.player.energy)?;
        obs.set_item("max_energy", self.state.player.max_energy)?;

        // Player buffs/debuffs
        obs.set_item("strength", self.state.player.get_strength())?;
        obs.set_item("weak", if self.state.player.is_weak() { 1 } else { 0 })?;

        // Hand info
        let hand: Vec<&str> = self.state.hand.iter().map(|c| c.definition_id.as_str()).collect();
        obs.set_item("hand", hand)?;

        let hand_costs: Vec<i32> = self.state.hand.iter().map(|c| c.current_cost).collect();
        obs.set_item("hand_costs", hand_costs)?;

        // Pile sizes
        obs.set_item("draw_pile_size", self.state.draw_pile.len())?;
        obs.set_item("discard_pile_size", self.state.discard_pile.len())?;
        obs.set_item("exhaust_pile_size", self.state.exhaust_pile.len())?;

        // Enemy info
        if let Some(enemy) = self.state.enemies.first() {
            obs.set_item("enemy_hp", enemy.hp)?;
            obs.set_item("enemy_max_hp", enemy.max_hp)?;
            obs.set_item("enemy_block", enemy.block)?;
            obs.set_item(
                "enemy_vulnerable",
                enemy.get_buff("Vulnerable"),
            )?;
            obs.set_item(
                "enemy_weak",
                enemy.get_buff("Weak"),
            )?;
        }

        // Combat info
        obs.set_item("turn", self.state.turn)?;
        obs.set_item("cards_played_this_turn", self.state.cards_played_this_turn)?;

        Ok(obs.into())
    }
}

// ============================================================================
// Parallel Batch Simulation with Rayon
// ============================================================================

/// Configuration for batch parallel simulation.
#[pyclass]
#[derive(Clone)]
pub struct SimConfig {
    /// Number of simulations to run in parallel
    #[pyo3(get, set)]
    pub num_simulations: usize,
    /// Maximum turns per combat before truncation
    #[pyo3(get, set)]
    pub max_turns: usize,
    /// Path to cards.json
    #[pyo3(get, set)]
    pub cards_path: String,
}

#[pymethods]
impl SimConfig {
    /// Create a new simulation configuration.
    ///
    /// Args:
    ///     num_simulations: Number of parallel simulations (default: 1000)
    ///     max_turns: Max turns per combat (default: 50)
    ///     cards_path: Path to cards.json (default: "data/cards.json")
    #[new]
    #[pyo3(signature = (num_simulations=1000, max_turns=50, cards_path=None))]
    fn new(num_simulations: usize, max_turns: usize, cards_path: Option<&str>) -> Self {
        Self {
            num_simulations,
            max_turns,
            cards_path: cards_path.unwrap_or("data/cards").to_string(),
        }
    }
}

/// Result of a single simulation run.
#[pyclass]
#[derive(Clone)]
pub struct SimResult {
    /// Seed used for this simulation
    #[pyo3(get)]
    pub seed: u64,
    /// Whether combat was won
    #[pyo3(get)]
    pub won: bool,
    /// Number of turns survived
    #[pyo3(get)]
    pub turns_survived: u32,
    /// Final player HP (0 if lost)
    #[pyo3(get)]
    pub final_hp: i32,
    /// Total damage dealt to enemies
    #[pyo3(get)]
    pub damage_dealt: i32,
    /// Total cards played
    #[pyo3(get)]
    pub cards_played: u32,
}

/// Run a single simulation with random policy (for benchmarking).
/// This version is optimized for speed - no logging.
fn run_single_simulation(seed: u64, library: &CardLibrary, max_turns: usize) -> SimResult {
    use crate::enemy::MonsterState;
    let mut state = GameState::new(seed);
    state.enemies.push(MonsterState::new_simple("Jaw Worm", 44));

    // Setup starter deck
    for _ in 0..5 {
        state.obtain_card(CardInstance::new("Strike_Ironclad".to_string(), 1));
    }
    for _ in 0..4 {
        state.obtain_card(CardInstance::new("Defend_Ironclad".to_string(), 1));
    }
    state.obtain_card(CardInstance::new("Bash".to_string(), 2));
    state.initialize_combat_deck();  // Copy master_deck → draw_pile

    let initial_enemy_hp = 44i32;
    let mut total_cards_played = 0u32;
    let mut total_damage_dealt = 0i32;

    // Simple greedy policy: play first playable card each step
    for _ in 0..max_turns {
        state.start_turn();

        // Play all playable cards (inline simulation without engine logs)
        loop {
            let playable_idx = state.hand.iter().position(|c| c.current_cost <= state.player.energy);

            match playable_idx {
                Some(idx) => {
                    let card = state.hand.remove(idx);
                    state.player.energy -= card.current_cost;
                    total_cards_played += 1;

                    // Simplified card effects (inline for performance)
                    match card.definition_id.as_str() {
                        "Strike_Ironclad" => {
                            // Deal 6 damage
                            if let Some(enemy) = state.enemies.first_mut() {
                                let dmg = enemy.take_damage(6);
                                total_damage_dealt += dmg;
                            }
                        }
                        "Defend_Ironclad" => {
                            // Gain 5 block
                            state.player.gain_block(5);
                        }
                        "Bash" => {
                            // Deal 8 damage, apply 2 Vulnerable
                            if let Some(enemy) = state.enemies.first_mut() {
                                let dmg = enemy.take_damage(8);
                                total_damage_dealt += dmg;
                                enemy.apply_status("Vulnerable", 2);
                            }
                        }
                        _ => {
                            // Use library for other cards (fallback)
                            if let Ok(_def) = library.get(&card.definition_id) {
                                // Just skip unknown cards in benchmark
                            }
                        }
                    }

                    state.discard_pile.push(card);

                    // Check win
                    if state.combat_won() {
                        return SimResult {
                            seed,
                            won: true,
                            turns_survived: state.turn,
                            final_hp: state.player.current_hp,
                            damage_dealt: total_damage_dealt,
                            cards_played: total_cards_played,
                        };
                    }
                }
                None => break,
            }
        }

        state.end_turn();

        // Enemy attack
        let damage = 10i32.saturating_sub(state.player.block);
        state.player.block = 0;
        state.player.current_hp -= damage.max(0);

        // Check loss
        if state.combat_lost() {
            return SimResult {
                seed,
                won: false,
                turns_survived: state.turn,
                final_hp: 0,
                damage_dealt: total_damage_dealt,
                cards_played: total_cards_played,
            };
        }
    }

    // Timeout
    SimResult {
        seed,
        won: false,
        turns_survived: state.turn,
        final_hp: state.player.current_hp,
        damage_dealt: total_damage_dealt,
        cards_played: total_cards_played,
    }
}

/// Batch simulator for running many simulations in parallel using Rayon.
#[pyclass]
pub struct BatchSimulator {
    library: CardLibrary,
    config: SimConfig,
}

#[pymethods]
impl BatchSimulator {
    /// Create a new batch simulator.
    ///
    /// Args:
    ///     config: SimConfig with simulation parameters
    #[new]
    fn new(config: SimConfig) -> PyResult<Self> {
        let library = CardLibrary::load(&config.cards_path)
            .map_err(|e| PyErr::new::<pyo3::exceptions::PyIOError, _>(e.to_string()))?;

        Ok(Self { library, config })
    }

    /// Run batch simulations in parallel.
    ///
    /// Args:
    ///     base_seed: Starting seed (simulation i uses base_seed + i)
    ///
    /// Returns:
    ///     List of SimResult objects
    fn run_batch(&self, base_seed: u64) -> PyResult<Vec<SimResult>> {
        let results: Vec<SimResult> = (0..self.config.num_simulations)
            .into_par_iter()
            .map(|i| {
                let seed = base_seed + i as u64;
                run_single_simulation(seed, &self.library, self.config.max_turns)
            })
            .collect();

        Ok(results)
    }

    /// Run batch and return summary statistics.
    ///
    /// Args:
    ///     base_seed: Starting seed for batch
    ///
    /// Returns:
    ///     Dict with win_rate, avg_turns, etc.
    fn run_batch_stats(&self, base_seed: u64, py: Python<'_>) -> PyResult<Py<PyAny>> {
        let results = self.run_batch(base_seed)?;

        let total = results.len() as f64;
        let wins = results.iter().filter(|r| r.won).count() as f64;
        let avg_turns: f64 = results.iter().map(|r| r.turns_survived as f64).sum::<f64>() / total;
        let avg_damage: f64 = results.iter().map(|r| r.damage_dealt as f64).sum::<f64>() / total;
        let avg_cards: f64 = results.iter().map(|r| r.cards_played as f64).sum::<f64>() / total;

        let stats = PyDict::new(py);
        stats.set_item("total_simulations", results.len())?;
        stats.set_item("win_rate", wins / total)?;
        stats.set_item("avg_turns_survived", avg_turns)?;
        stats.set_item("avg_damage_dealt", avg_damage)?;
        stats.set_item("avg_cards_played", avg_cards)?;
        Ok(stats.into())
    }

    /// Benchmark: measure throughput of parallel simulation.
    ///
    /// Args:
    ///     num_sims: Number of simulations to run
    ///
    /// Returns:
    ///     Dict with elapsed_seconds, simulations_per_second, etc.
    fn benchmark(&self, num_sims: usize, py: Python<'_>) -> PyResult<Py<PyAny>> {
        use std::time::Instant;

        let start = Instant::now();

        let _results: Vec<SimResult> = (0..num_sims)
            .into_par_iter()
            .map(|i| run_single_simulation(i as u64, &self.library, self.config.max_turns))
            .collect();

        let elapsed = start.elapsed();
        let sims_per_sec = num_sims as f64 / elapsed.as_secs_f64();

        let result = PyDict::new(py);
        result.set_item("num_simulations", num_sims)?;
        result.set_item("elapsed_seconds", elapsed.as_secs_f64())?;
        result.set_item("simulations_per_second", sims_per_sec)?;
        result.set_item(
            "microseconds_per_sim",
            elapsed.as_micros() as f64 / num_sims as f64,
        )?;
        Ok(result.into())
    }
}

// ============================================================================
// Python Module Definition
// ============================================================================

/// Quick benchmark function accessible from Python.
#[pyfunction]
#[pyo3(signature = (num_sims, cards_path=None))]
fn quick_benchmark(py: Python<'_>, num_sims: usize, cards_path: Option<&str>) -> PyResult<Py<PyAny>> {
    let path = cards_path.unwrap_or("data/cards");
    let config = SimConfig::new(num_sims, 50, Some(path));
    let sim = BatchSimulator::new(config)?;
    sim.benchmark(num_sims, py)
}

/// Set global verbose flag from Python.
/// Call `sts_sim.set_verbose(False)` to suppress all game logs during training.
#[pyfunction]
fn set_verbose(verbose: bool) {
    crate::set_verbose(verbose);
}

/// Python module entry point.
/// 
/// Exposes:
/// - StsEnv: Gym-style environment for RL training
/// - SimConfig: Configuration for batch simulation
/// - SimResult: Result of a single simulation
/// - BatchSimulator: Parallel batch simulation runner
/// - quick_benchmark(): Fast throughput test
/// - set_verbose(): Control game log output
#[pymodule]
fn sts_sim(m: &Bound<'_, PyModule>) -> PyResult<()> {
    // New simplified interface
    m.add_class::<PyStsSim>()?;
    
    // Legacy Gym-style environment
    m.add_class::<StsEnv>()?;
    m.add_class::<SimConfig>()?;
    m.add_class::<SimResult>()?;
    m.add_class::<BatchSimulator>()?;

    m.add("__version__", "0.1.0")?;
    
    // Add standalone functions
    m.add_function(wrap_pyfunction!(quick_benchmark, m)?)?;
    m.add_function(wrap_pyfunction!(set_verbose, m)?)?;

    Ok(())
}

// ============================================================================
// Pure Rust API (for non-Python usage)
// ============================================================================

/// Run parallel simulations from Rust code.
///
/// # Arguments
/// * `num_sims` - Number of simulations to run
/// * `base_seed` - Starting seed (sim i uses base_seed + i)
/// * `cards_path` - Path to cards.json
/// * `max_turns` - Maximum turns per combat
///
/// # Returns
/// Vector of SimResult or LoaderError
pub fn run_parallel_simulations(
    num_sims: usize,
    base_seed: u64,
    cards_path: &str,
    max_turns: usize,
) -> Result<Vec<SimResult>, crate::loader::LoaderError> {
    let library = CardLibrary::load(cards_path)?;

    let results: Vec<SimResult> = (0..num_sims)
        .into_par_iter()
        .map(|i| {
            let seed = base_seed + i as u64;
            run_single_simulation(seed, &library, max_turns)
        })
        .collect();

    Ok(results)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_single_simulation() {
        let library = CardLibrary::load("data/cards").unwrap();
        let result = run_single_simulation(42, &library, 50);

        assert!(result.turns_survived > 0);
        assert!(result.cards_played > 0);
        game_log!(
            "Simulation: won={}, turns={}, hp={}, damage={}",
            result.won, result.turns_survived, result.final_hp, result.damage_dealt
        );
    }

    #[test]
    fn test_parallel_simulations() {
        let results = run_parallel_simulations(100, 0, "data/cards", 50).unwrap();

        assert_eq!(results.len(), 100);

        let wins = results.iter().filter(|r| r.won).count();
        let avg_turns: f64 =
            results.iter().map(|r| r.turns_survived as f64).sum::<f64>() / results.len() as f64;

        game_log!(
            "Batch: {}/{} wins ({:.1}%), avg turns: {:.1}",
            wins,
            results.len(),
            wins as f64 / results.len() as f64 * 100.0,
            avg_turns
        );
    }

    #[test]
    fn test_deterministic_simulation() {
        let library = CardLibrary::load("data/cards").unwrap();

        let result1 = run_single_simulation(12345, &library, 50);
        let result2 = run_single_simulation(12345, &library, 50);

        // Same seed should produce identical results
        assert_eq!(result1.won, result2.won);
        assert_eq!(result1.turns_survived, result2.turns_survived);
        assert_eq!(result1.final_hp, result2.final_hp);
        assert_eq!(result1.damage_dealt, result2.damage_dealt);
        assert_eq!(result1.cards_played, result2.cards_played);
    }
}
