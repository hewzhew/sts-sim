//! Event system integration — event processing, costs, rewards.
//!
//! Corresponds to Java's vents/ package (56 event classes).

use crate::loader::CardLibrary;
use crate::schema::{CardInstance, CardType, CardLocation, CardColor};
use crate::state::{GameState, GamePhase};
use crate::dungeon::spawn_encounter;
use crate::events::{
    EventSelector, EventPoolContext, ActiveEventState, ActId, EventCommand,
    EventDefinition, DeckModOp, CardSelectAction, EventCosts, EventRewards,
    CardFilter as EventCardFilter,
};
use rand::Rng;
use rand::SeedableRng;

// ============================================================================
// Event System Integration
// ============================================================================

/// Build an EventPoolContext from the current game state
pub fn build_event_pool_context(state: &GameState) -> EventPoolContext {
    EventPoolContext {
        gold: state.gold,
        current_hp: state.player.current_hp,
        max_hp: state.player.max_hp,
        ascension: state.ascension_level,
        floor: state.floor as i32,
        act: match state.act {
            1 => ActId::Act1,
            2 => ActId::Act2,
            3 => ActId::Act3,
            _ => ActId::Act1,
        },
        relic_ids: state.relics.iter().map(|r| r.id.clone()).collect(),
        has_curse: state.draw_pile.iter().any(|c| {
            c.definition_id.to_lowercase().contains("curse") ||
            matches!(c.definition_id.as_str(), "Regret" | "Pain" | "Doubt" | "Shame" | 
                     "Decay" | "Injury" | "Clumsy" | "Writhe" | "Parasite" | 
                     "Normality" | "Pride" | "Necronomicurse" | "CurseOfTheBell" | "AscendersBane")
        }),
        elapsed_seconds: 0, // TODO: Track run time
        chest_floor: 8,     // Standard chest floor
        seen_events: state.seen_events.clone(),
    }
}

/// Result of processing event commands
#[derive(Debug, Clone)]
pub enum EventProcessResult {
    /// Event completed, return to map
    Complete,
    /// Event continues (show options again)
    Continue,
    /// Need card selection UI
    AwaitingCardSelect,
    /// Start combat
    StartCombat { encounter_id: String },
    /// Player died from event damage
    PlayerDied,
}

/// Process a list of event commands and apply them to game state
pub fn process_event_commands(
    state: &mut GameState,
    commands: &[EventCommand],
    costs: Option<&EventCosts>,
    rewards: Option<&EventRewards>,
) -> EventProcessResult {
    // First, apply costs
    if let Some(costs) = costs {
        apply_event_costs(state, costs);
        
        // Check if player died from HP cost
        if state.player.current_hp <= 0 {
            state.screen = GamePhase::GameOver;
            return EventProcessResult::PlayerDied;
        }
    }
    
    // Then, apply rewards
    if let Some(rewards) = rewards {
        apply_event_rewards(state, rewards);
    }
    
    // Process each command
    for cmd in commands {
        match process_single_event_command(state, cmd) {
            EventProcessResult::Complete => {
                // Continue processing, will finish after all commands
            }
            result @ EventProcessResult::AwaitingCardSelect => return result,
            result @ EventProcessResult::StartCombat { .. } => return result,
            result @ EventProcessResult::PlayerDied => return result,
            EventProcessResult::Continue => {
                // Continue processing
            }
        }
    }
    
    EventProcessResult::Complete
}

/// Apply event costs to the game state
fn apply_event_costs(state: &mut GameState, costs: &EventCosts) {
    let ascension = state.ascension_level;
    
    // Gold costs
    if let Some(gold) = costs.gold {
        let amount = if ascension >= 15 {
            costs.gold_ascension.unwrap_or(gold)
        } else {
            gold
        };
        state.gold = (state.gold - amount).max(0);
    }
    
    if costs.gold_all == Some(true) {
        state.gold = 0;
    }
    
    if let Some(range) = &costs.gold_random {
        use rand::Rng;
        let amount = state.rng.random_range(range.min..=range.max);
        state.gold = (state.gold - amount).max(0);
    }
    
    // HP costs
    if let Some(hp) = costs.hp {
        let amount = if ascension >= 15 {
            costs.hp_ascension.unwrap_or(hp)
        } else {
            hp
        };
        state.player.current_hp -= amount;
    }
    
    if let Some(percent) = costs.hp_percent {
        let pct = if ascension >= 15 {
            costs.hp_percent_ascension.unwrap_or(percent)
        } else {
            percent
        };
        let amount = (state.player.max_hp as f32 * pct).ceil() as i32;
        state.player.current_hp -= amount;
    }
    
    // Dynamic HP cost (e.g., Knowing Skull)
    if let Some(hp_dyn) = &costs.hp_dynamic {
        let counter_value = state.event_state.as_ref()
            .and_then(|e| e.get_state(&hp_dyn.counter))
            .and_then(|v| v.as_i64())
            .unwrap_or(0) as i32;
        
        let floor = (state.player.max_hp as f32 * hp_dyn.floor_percent).ceil() as i32;
        let cost = (hp_dyn.base + hp_dyn.per_count * counter_value).max(hp_dyn.min).max(floor);
        state.player.current_hp -= cost;
    }
    
    // Max HP costs
    if let Some(max_hp) = costs.max_hp {
        state.player.max_hp -= max_hp;
        state.player.current_hp = state.player.current_hp.min(state.player.max_hp);
    }
    
    if let Some(pct) = costs.max_hp_percent {
        let pct_actual = if ascension >= 15 {
            costs.max_hp_percent_ascension.unwrap_or(pct)
        } else {
            pct
        };
        let amount = (state.player.max_hp as f32 * pct_actual).ceil() as i32;
        state.player.max_hp -= amount;
        state.player.current_hp = state.player.current_hp.min(state.player.max_hp);
    }
    
    // Potion costs
    if let Some(_potion) = &costs.potion {
        // TODO: Remove specific potion from inventory
    }
    
    if let Some(count) = costs.potion_count {
        // TODO: Remove N potions from inventory
        let _ = count;
    }
}

/// Apply event rewards to the game state
fn apply_event_rewards(state: &mut GameState, rewards: &EventRewards) {
    let ascension = state.ascension_level;
    
    // Gold rewards
    if let Some(gold) = rewards.gold {
        let amount = if ascension >= 15 {
            rewards.gold_ascension.unwrap_or(gold)
        } else {
            gold
        };
        state.gold += amount;
    }
    
    if let Some(range) = &rewards.gold_random {
        use rand::Rng;
        let amount = state.rng.random_range(range.min..=range.max);
        state.gold += amount;
    }
    
    // Healing
    if let Some(heal) = rewards.heal {
        state.player.current_hp = (state.player.current_hp + heal).min(state.player.max_hp);
    }
    
    if let Some(percent) = rewards.heal_percent {
        let pct = if ascension >= 15 {
            rewards.heal_percent_ascension.unwrap_or(percent)
        } else {
            percent
        };
        let amount = (state.player.max_hp as f32 * pct).ceil() as i32;
        state.player.current_hp = (state.player.current_hp + amount).min(state.player.max_hp);
    }
    
    if rewards.heal_max_hp == Some(true) {
        state.player.current_hp = state.player.max_hp;
    }
    
    // Max HP rewards
    if let Some(max_hp) = rewards.max_hp {
        state.player.max_hp += max_hp;
    }
    
    if let Some(pct) = rewards.max_hp_percent {
        let amount = (state.player.max_hp as f32 * pct).ceil() as i32;
        state.player.max_hp += amount;
    }
    
    // Relic rewards
    if let Some(relic_id) = &rewards.relic {
        use crate::items::relics::RelicInstance;
        state.relics.push(RelicInstance::new(relic_id));
        crate::items::relics::on_relic_equip(state, relic_id);
    }
    
    // Card rewards (add to deck)
    if let Some(card_id) = &rewards.card {
        let card = CardInstance::new(card_id.clone(), 1); // Default cost 1
        state.draw_pile.push(card);
    }
    
    // Curse rewards (add curse to deck)
    if let Some(curse_id) = &rewards.curse {
        let curse = CardInstance::new(curse_id.clone(), -1); // Curses typically unplayable
        state.draw_pile.push(curse);
    }
    
    // Potion rewards
    if let Some(_potion_id) = &rewards.potion {
        // TODO: Add potion to inventory
    }
    
    // Key rewards (Act 3)
    if let Some(_key) = &rewards.key {
        // TODO: Grant key (Ruby/Emerald/Sapphire)
    }
}

/// Process a single event command
fn process_single_event_command(
    state: &mut GameState,
    cmd: &EventCommand,
) -> EventProcessResult {
    match cmd {
        EventCommand::Combat { enemies, enemy_selection, combat_type: _, boss_pool, special: _ } => {
            // Determine encounter
            let encounter_id: String = if !enemies.is_empty() {
                enemies[0].clone()
            } else if let Some(selection) = enemy_selection {
                selection.clone()
            } else if !boss_pool.is_empty() {
                let idx = state.rng.random_range(0..boss_pool.len());
                boss_pool[idx].clone()
            } else {
                "Cultist".to_string() // Fallback
            };
            
            // Spawn monsters
            let monsters = spawn_encounter(&mut state.rng, &encounter_id);
            state.enemies.clear();
            for spawn in monsters {
                let monster = crate::enemy::MonsterState::new_simple(&spawn.monster_id, 50); // Default HP
                state.enemies.push(monster);
            }
            
            // Clear event state and transition to combat
            state.event_state = None;
            state.screen = GamePhase::Combat;
            
            return EventProcessResult::StartCombat { encounter_id };
        }
        
        EventCommand::DeckMod { op, card_id, filter, count, count_ascension } => {
            let cnt = if state.ascension_level >= 15 {
                count_ascension.unwrap_or(count.unwrap_or(1))
            } else {
                count.unwrap_or(1)
            };
            
            match op {
                DeckModOp::Add => {
                    if let Some(cid) = card_id {
                        for _ in 0..cnt {
                            let card = CardInstance::new(cid.clone(), 1);
                            state.draw_pile.push(card);
                        }
                    }
                }
                DeckModOp::Remove => {
                    // Remove specific card or by filter
                    if let Some(cid) = card_id {
                        if let Some(idx) = state.draw_pile.iter().position(|c| &c.definition_id == cid) {
                            state.draw_pile.remove(idx);
                        }
                    }
                }
                DeckModOp::RemoveRandom => {
                    use rand::Rng;
                    for _ in 0..cnt {
                        if !state.draw_pile.is_empty() {
                            let idx = state.rng.random_range(0..state.draw_pile.len());
                            state.draw_pile.remove(idx);
                        }
                    }
                }
                DeckModOp::RemoveAll => {
                    // Remove all cards matching filter
                    if let Some(filter) = filter {
                        state.draw_pile.retain(|card| {
                            !matches_event_filter(card, filter)
                        });
                    }
                }
                DeckModOp::Upgrade => {
                    if let Some(card_id) = card_id {
                        if let Some(card) = state.draw_pile.iter_mut().find(|c| &c.definition_id == card_id) {
                            card.upgraded = true;
                        }
                    }
                }
                DeckModOp::UpgradeRandom => {
                    use rand::Rng;
                    let upgradeable: Vec<usize> = state.draw_pile.iter()
                        .enumerate()
                        .filter(|(_, c)| !c.upgraded)
                        .map(|(i, _)| i)
                        .collect();
                    
                    for _ in 0..cnt.min(upgradeable.len() as i32) {
                        if !upgradeable.is_empty() {
                            let idx = state.rng.random_range(0..upgradeable.len());
                            state.draw_pile[upgradeable[idx]].upgraded = true;
                        }
                    }
                }
                DeckModOp::UpgradeAll => {
                    for card in state.draw_pile.iter_mut() {
                        card.upgraded = true;
                    }
                }
                DeckModOp::Transform | DeckModOp::TransformRandom => {
                    // TODO: Transform requires card pool knowledge
                }
                DeckModOp::Duplicate => {
                    if let Some(card_id) = card_id {
                        if let Some(card) = state.draw_pile.iter().find(|c| &c.definition_id == card_id) {
                            let dup = card.clone();
                            state.draw_pile.push(dup);
                        }
                    }
                }
            }
        }
        
        EventCommand::CardSelect { action, pick, pool, source_amount, filter } => {
            // Set up card selection state
            state.card_select_action = Some(*action);
            state.card_select_count = *pick;
            state.card_select_filter = filter.clone();
            
            // Build card pool (indices into draw_pile)
            state.card_select_pool = state.draw_pile.iter()
                .enumerate()
                .filter(|(_, card)| {
                    if let Some(f) = filter {
                        matches_event_filter(card, f)
                    } else {
                        true
                    }
                })
                .map(|(i, _)| i)
                .collect();
            
            state.screen = GamePhase::CardSelect;
            return EventProcessResult::AwaitingCardSelect;
        }
        
        EventCommand::SetEventState { state: key, value } => {
            if let Some(event_state) = &mut state.event_state {
                event_state.set_state(key, value.clone());
            }
        }
        
        EventCommand::SetEventPhase { phase } => {
            if let Some(event_state) = &mut state.event_state {
                event_state.set_state("phase", phase.clone());
            }
        }
        
        EventCommand::Teleport { destination } => {
            // TODO: Implement teleport (e.g., to boss, to act end)
            game_log!("Teleport to: {}", destination);
        }
        
        EventCommand::LoseItem { item_type, selection } => {
            // TODO: Handle losing potions, relics, etc.
            game_log!("Lose item: {} (selection: {:?})", item_type, selection);
        }
        
        EventCommand::Minigame { game, config } => {
            // TODO: Handle minigames like Match and Keep
            game_log!("Minigame: {} (config: {:?})", game, config);
        }
    }
    
    EventProcessResult::Continue
}

/// Check if a card matches an event filter
fn matches_event_filter(card: &CardInstance, filter: &EventCardFilter) -> bool {
    if let Some(card_type) = &filter.card_type {
        let matches_type = match card_type.to_lowercase().as_str() {
            "attack" => card.definition_id.contains("Strike") || card.definition_id.contains("Attack"),
            "skill" => card.definition_id.contains("Defend") || card.definition_id.contains("Block"),
            "power" => false, // Would need card library lookup
            "curse" => card.definition_id.contains("Curse") || 
                       matches!(card.definition_id.as_str(), "Regret" | "Pain" | "Doubt" | "Shame" | 
                                "Decay" | "Injury" | "Clumsy" | "Writhe" | "Parasite"),
            "status" => matches!(card.definition_id.as_str(), "Slimed" | "Burn" | "Wound" | "Dazed" | "Void"),
            _ => true,
        };
        if !matches_type {
            return false;
        }
    }
    
    if let Some(upgradeable) = filter.upgradeable {
        if upgradeable && card.upgraded {
            return false; // Already upgraded, not upgradeable
        }
    }
    
    // Note: rarity filter would require card library lookup
    
    true
}

/// Execute a selected event option by index
/// Returns commands to process
pub fn execute_event_option(
    state: &mut GameState,
    option_idx: usize,
) -> Result<EventProcessResult, String> {
    let event_state = state.event_state.as_ref()
        .ok_or_else(|| "No active event".to_string())?;
    
    let event_def = EventDefinition::get(&event_state.event_id)
        .ok_or_else(|| format!("Event not found: {}", event_state.event_id))?;
    
    let option = event_def.options.get(option_idx)
        .ok_or_else(|| format!("Invalid option index: {}", option_idx))?;
    
    // Check if option is available
    if !event_def.is_option_available(option_idx, state.gold, state.ascension_level, event_state) {
        return Err("Option not available".to_string());
    }
    
    // Handle random outcomes if present
    if !option.random_outcomes.is_empty() {
        let iteration = event_state.iteration;
        let mut rng = rand_xoshiro::Xoshiro256StarStar::seed_from_u64(event_state.rng_seed + iteration as u64);
        
        if let Some(outcome) = option.roll_random_outcome(&mut rng, iteration, state.ascension_level) {
            // Process outcome's costs, rewards, and commands
            let result = process_event_commands(
                state, 
                &outcome.commands, 
                outcome.costs.as_ref(),
                outcome.rewards.as_ref(),
            );
            
            // If event continues (loop mechanic), increment iteration
            if matches!(result, EventProcessResult::Continue) {
                if let Some(event_state) = &mut state.event_state {
                    event_state.increment_iteration();
                }
            }
            
            return Ok(result);
        }
    }
    
    // Process option's costs, rewards, and commands
    let result = process_event_commands(
        state,
        &option.commands,
        option.costs.as_ref(),
        option.rewards.as_ref(),
    );
    
    // Handle loop events
    if let Some(loop_mech) = &event_def.loop_mechanic {
        if let Some(event_state) = &mut state.event_state {
            event_state.increment_iteration();
            
            // Check if we've hit max iterations
            if let Some(max) = loop_mech.max_iterations {
                if event_state.iteration >= max {
                    return Ok(EventProcessResult::Complete);
                }
            }
        }
    }
    
    Ok(result)
}

/// Finish an event and return to the map
pub fn finish_event(state: &mut GameState) {
    state.event_state = None;
    state.screen = GamePhase::Map;
}

/// Handle card selection completion for events
pub fn complete_card_selection(state: &mut GameState, selected_indices: &[usize]) -> EventProcessResult {
    let action = match state.card_select_action.take() {
        Some(a) => a,
        None => return EventProcessResult::Continue,
    };
    
    // Apply the action to selected cards
    match action {
        CardSelectAction::Remove => {
            // Remove selected cards (iterate in reverse to preserve indices)
            let mut indices: Vec<usize> = selected_indices.to_vec();
            indices.sort_by(|a, b| b.cmp(a));
            for idx in indices {
                if idx < state.draw_pile.len() {
                    state.draw_pile.remove(idx);
                }
            }
        }
        CardSelectAction::Upgrade => {
            for &idx in selected_indices {
                if idx < state.draw_pile.len() {
                    state.draw_pile[idx].upgraded = true;
                }
            }
        }
        CardSelectAction::Transform => {
            // TODO: Would need card pool to transform into
            for &idx in selected_indices {
                if idx < state.draw_pile.len() {
                    state.draw_pile.remove(idx);
                    // Would add transformed card here
                }
            }
        }
        CardSelectAction::Duplicate => {
            for &idx in selected_indices {
                if idx < state.draw_pile.len() {
                    let dup = state.draw_pile[idx].clone();
                    state.draw_pile.push(dup);
                }
            }
        }
        CardSelectAction::Add => {
            // Card already added to pool, just select which one
        }
        CardSelectAction::OfferSpirits => {
            // Bonfire Spirits: remove card and get reward based on rarity
            // TODO: Implement rarity-based rewards
            for &idx in selected_indices {
                if idx < state.draw_pile.len() {
                    state.draw_pile.remove(idx);
                }
            }
        }
    }
    
    // Clear selection state
    state.card_select_pool.clear();
    state.card_select_count = 0;
    state.card_select_filter = None;
    
    // Return to event or map
    if state.event_state.is_some() {
        state.screen = GamePhase::Event;
        EventProcessResult::Continue
    } else {
        state.screen = GamePhase::Map;
        EventProcessResult::Complete
    }
}

/// Get available event options for the current event
pub fn get_available_event_options(state: &GameState) -> Vec<(usize, String, bool)> {
    let event_state = match &state.event_state {
        Some(e) => e,
        None => return Vec::new(),
    };
    
    let event_def = match EventDefinition::get(&event_state.event_id) {
        Some(e) => e,
        None => return Vec::new(),
    };
    
    event_def.options.iter()
        .enumerate()
        .map(|(idx, opt)| {
            let available = event_def.is_option_available(idx, state.gold, state.ascension_level, event_state);
            (idx, opt.label.clone(), available)
        })
        .collect()
}

