use crate::state::core::{EngineState, ClientInput, CampfireChoice};
use crate::state::run::RunState;
use crate::combat::CombatState;

/// An autonomous agent that can decide the next `ClientInput` based on the game state.
pub struct Agent {
    bot_depth: u32,
    /// Pre-computed optimal map path for current act (x-coords for y=0..14, plus boss)
    map_path: Vec<i32>,
    pub db: crate::bot::coverage::CoverageDb,
}

impl Agent {
    pub fn new() -> Self {
        Self {
            bot_depth: 6,
            map_path: Vec::new(),
            db: crate::bot::coverage::CoverageDb::load_or_default(),
        }
    }

    /// Sets the search depth for combat decision tree.
    pub fn set_bot_depth(&mut self, depth: u32) {
        self.bot_depth = depth;
    }

    /// Primary entry point for the bot to decide the next move.
    pub fn decide(
        &mut self,
        es: &EngineState,
        rs: &RunState,
        cs: &Option<CombatState>,
        verbose: bool,
    ) -> ClientInput {
        match es {
            EngineState::CombatPlayerTurn | EngineState::PendingChoice(_) | EngineState::EventCombat(_) => {
                if let Some(combat) = cs {
                    let chosen = crate::bot::search::find_best_move(es, combat, self.bot_depth, verbose, &self.db);
                    
                    // Live coverage tracking: mark executing moves as tested
                    match &chosen {
                        ClientInput::PlayCard { card_index, .. } => {
                            if let Some(card) = combat.hand.get(*card_index) {
                                let def = crate::content::cards::get_card_definition(card.id);
                                self.db.tested_cards.insert(def.name.to_string());
                                self.db.save();
                            }
                        },
                        ClientInput::UsePotion { potion_index, .. } => {
                            if let Some(Some(p)) = combat.potions.get(*potion_index) {
                                let def = crate::content::potions::get_potion_definition(p.id);
                                self.db.tested_potions.insert(def.name.to_string());
                                self.db.save();
                            }
                        },
                        _ => {}
                    }
                    
                    chosen
                } else {
                    ClientInput::Proceed
                }
            },
            EngineState::MapNavigation => {
                self.decide_map(rs)
            },
            EngineState::RewardScreen(reward) => {
                use crate::state::reward::RewardItem;

                // 1. Handle pending card choice
                if let Some(cards) = &reward.pending_card_choice {
                    let mut best_score = -9999;
                    let mut best_idx = None;

                    for (i, &card_id) in cards.iter().enumerate() {
                        let score = crate::bot::evaluator::CardEvaluator::evaluate_card(card_id, rs);
                        if score > best_score {
                            best_score = score;
                            best_idx = Some(i);
                        }
                    }

                    // If the best card has a negative score, just skip the reward (unless Skipping is somehow not an option)
                    if best_score < 0 {
                        ClientInput::Proceed
                    } else if let Some(idx) = best_idx {
                        ClientInput::SelectCard(idx)
                    } else {
                        ClientInput::Proceed
                    }

                } else if !reward.items.is_empty() {
                    // 2. Handle claiming items
                    let mut claimed = false;
                    let mut claim_idx = 0;

                    for (i, item) in reward.items.iter().enumerate() {
                        match item {
                            RewardItem::Potion { .. } => {
                                // Claim potion only if we have an empty slot
                                if rs.potions.iter().any(|p| p.is_none()) {
                                    claim_idx = i;
                                    claimed = true;
                                    break;
                                }
                            },
                            // Always claim gold/relics/cards/etc.
                            _ => {
                                claim_idx = i;
                                claimed = true;
                                break;
                            }
                        }
                    }

                    if claimed {
                        ClientInput::ClaimReward(claim_idx)
                    } else {
                        // Leftover items (e.g. potions when full), proceed
                        ClientInput::Proceed
                    }
                } else {
                    ClientInput::Proceed
                }
            },
            EngineState::BossRelicSelect(bs) => {
                use crate::content::relics::RelicId;
                
                // Hardcoded Boss Relic priority for AI to avoid MCTS blindness
                let priority = |id: RelicId| -> i32 {
                    match id {
                        RelicId::Sozu => 100,
                        RelicId::CursedKey => 90,
                        RelicId::Astrolabe => 80,
                        RelicId::SneckoEye => 70,
                        RelicId::BustedCrown => 60,
                        RelicId::CoffeeDripper => 50,
                        RelicId::FusionHammer => 45,
                        RelicId::Ectoplasm => 40,
                        RelicId::PhilosopherStone => 35,
                        RelicId::VelvetChoker => 30,
                        RelicId::EmptyCage => 20,
                        RelicId::CallingBell => 10,
                        _ => 0,
                    }
                };

                let mut best_idx = 0;
                let mut best_score = -1;
                for (i, relic) in bs.relics.iter().enumerate() {
                    let score = priority(*relic);
                    if score > best_score {
                        best_score = score;
                        best_idx = i;
                    }
                }
                
                ClientInput::SubmitRelicChoice(best_idx)
            },
            EngineState::Campfire => {
                self.decide_campfire(rs)
            },
            EngineState::EventRoom => {
                self.decide_event(rs)
            },
            EngineState::Shop(_) => {
                ClientInput::Proceed
            },
            EngineState::RunPendingChoice(choice_state) => {
                use crate::state::core::RunPendingChoiceReason;
                match choice_state.reason {
                    RunPendingChoiceReason::Purge => {
                        let mut best_score = 9999;
                        let mut best_idx = 0;
                        for (i, c) in rs.master_deck.iter().enumerate() {
                            let score = crate::bot::evaluator::CardEvaluator::evaluate_card(c.id, rs);
                            if score < best_score {
                                best_score = score;
                                best_idx = i;
                            }
                        }
                        if rs.master_deck.is_empty() {
                            ClientInput::Cancel
                        } else {
                            ClientInput::SubmitDeckSelect(vec![best_idx])
                        }
                    },
                    RunPendingChoiceReason::Upgrade => {
                        let mut best_score = -9999;
                        let mut best_idx = 0;
                        let mut found = false;
                        for (i, c) in rs.master_deck.iter().enumerate() {
                            if c.upgrades == 0 {
                                let score = crate::bot::evaluator::CardEvaluator::evaluate_card(c.id, rs);
                                if score > best_score {
                                    best_score = score;
                                    best_idx = i;
                                    found = true;
                                }
                            }
                        }
                        if found {
                            ClientInput::SubmitDeckSelect(vec![best_idx])
                        } else {
                            ClientInput::Cancel
                        }
                    },
                    _ => ClientInput::Cancel,
                }
            },
            EngineState::GameOver(_) => {
                ClientInput::Proceed
            },
            _ => {
                ClientInput::Proceed
            }
        }
    }

    // ─── Map path planning ──────────────────────────────────────────────────

    fn decide_map(&mut self, rs: &RunState) -> ClientInput {
        // At start of act (y == -1), compute the full path
        if rs.map.current_y < 0 {
            self.map_path = Self::compute_map_path(rs);
            println!("  [BOT] Computed map path: {:?}", self.map_path);
        }

        let path_idx = (rs.map.current_y + 1) as usize;
        if path_idx < self.map_path.len() {
            let target_x = self.map_path[path_idx];
            // Verify it's reachable, fallback to first available if not
            let next_y = rs.map.current_y + 1;
            if rs.map.can_travel_to(target_x, next_y, false) {
                ClientInput::SelectMapNode(target_x as usize)
            } else {
                for x in 0..7 {
                    if rs.map.can_travel_to(x, next_y, false) {
                        return ClientInput::SelectMapNode(x as usize);
                    }
                }
                ClientInput::SelectMapNode(0)
            }
        } else {
            // Past the path (boss node)
            let next_y = rs.map.current_y + 1;
            for x in 0..7 {
                if rs.map.can_travel_to(x, next_y, false) {
                    return ClientInput::SelectMapNode(x as usize);
                }
            }
            ClientInput::SelectMapNode(0)
        }
    }

    fn compute_map_path(rs: &RunState) -> Vec<i32> {
        let graph = &rs.map.graph;
        let act_idx = ((rs.act_num as usize).saturating_sub(1)).min(2);
        
        // Room type weights for map path DP, indexed by act (1-indexed).
        // Order: [ShopRoom, RestRoom, EventRoom, MonsterRoomElite, MonsterRoom, TreasureRoom]
        let weights: [i32; 6] = match act_idx {
            0 => [100, 1000, 100, 10,   1, 0],  // Act 1
            1 => [10,  1000, 10,  100,  1, 0],  // Act 2
            _ => [100, 1000, 100, 1,   10, 0],  // Act 3
        };

        let mut paths_a: Vec<(Vec<i32>, i32)> = vec![(vec![], 0); 7];
        let mut paths_b: Vec<(Vec<i32>, i32)> = vec![(vec![], 0); 7];

        if !graph.is_empty() {
            for x in 0..7 {
                if x < graph[0].len() {
                    let node = &graph[0][x];
                    if !node.edges.is_empty() {
                        let w = node.class.map(|rt| weights[Self::room_type_to_weight_index(rt)]).unwrap_or(0);
                        paths_a[x] = (vec![x as i32], w);
                    }
                }
            }
        }

        let max_y = graph.len().min(15);
        for y in 0..max_y.saturating_sub(1) {
            for x in 0..7 {
                paths_b[x] = (vec![], 0);
            }

            for x in 0..7 {
                if x >= graph[y].len() { continue; }
                let node = &graph[y][x];
                if node.edges.is_empty() { continue; }
                let cur_path = &paths_a[x];

                for edge in &node.edges {
                    let next_x = edge.dst_x as usize;
                    let next_y = edge.dst_y as usize;
                    if next_y >= graph.len() || next_x >= graph[next_y].len() { continue; }

                    let next_node = &graph[next_y][next_x];
                    let room_w = next_node.class.map(|rt| weights[Self::room_type_to_weight_index(rt)]).unwrap_or(0);
                    let new_weight = cur_path.1 + room_w;

                    let dest = &paths_b[next_x];
                    if dest.0.len() < cur_path.0.len() + 1 || dest.1 < new_weight {
                        let mut new_route = cur_path.0.clone();
                        new_route.push(next_x as i32);
                        paths_b[next_x] = (new_route, new_weight);
                    }
                }
            }

            std::mem::swap(&mut paths_a, &mut paths_b);
        }

        let mut best_x = 0;
        let mut best_weight = i32::MIN;
        for x in 0..7 {
            if paths_a[x].1 > best_weight && !paths_a[x].0.is_empty() {
                best_weight = paths_a[x].1;
                best_x = x;
            }
        }

        let mut route = paths_a[best_x].0.clone();
        route.push(0); // Boss node (x=0 placeholder)
        route
    }

    fn room_type_to_weight_index(rt: crate::map::node::RoomType) -> usize {
        use crate::map::node::RoomType;
        match rt {
            RoomType::ShopRoom => 0,
            RoomType::RestRoom => 1,
            RoomType::EventRoom => 2,
            RoomType::MonsterRoomElite => 3,
            RoomType::MonsterRoom => 4,
            RoomType::TreasureRoom => 5,
            _ => 4,
        }
    }

    // ─── Campfire decision ──────────────────────────────────────────────────

    fn decide_campfire(&self, rs: &RunState) -> ClientInput {
        use crate::content::relics::RelicId;

        let has_relic = |id: RelicId| rs.relics.iter().any(|r| r.id == id);
        let can_rest = !has_relic(RelicId::CoffeeDripper);
        let can_smith = !has_relic(RelicId::FusionHammer) && !rs.master_deck.is_empty();

        let hp_ratio = rs.current_hp as f32 / rs.max_hp.max(1) as f32;
        let pre_boss_floor = rs.floor_num % 17 == 15;

        if can_rest && (hp_ratio < 0.5 || (rs.act_num != 1 && pre_boss_floor && hp_ratio < 0.9)) {
            ClientInput::CampfireOption(CampfireChoice::Rest)
        } else if can_smith {
            let mut best_score = -9999;
            let mut best_idx = 0;
            
            for (i, c) in rs.master_deck.iter().enumerate() {
                if c.upgrades == 0 {
                    let score = crate::bot::evaluator::CardEvaluator::evaluate_card(c.id, rs);
                    if score > best_score {
                        best_score = score;
                        best_idx = i;
                    }
                }
            }
            
            ClientInput::CampfireOption(CampfireChoice::Smith(best_idx))
        } else if can_rest {
            ClientInput::CampfireOption(CampfireChoice::Rest)
        } else {
            ClientInput::Proceed
        }
    }

    // ─── Event decision ─────────────────────────────────────────────────────

    fn decide_event(&self, rs: &RunState) -> ClientInput {
        use crate::state::events::EventId;
        use crate::content::relics::RelicId;

        if let Some(event) = &rs.event_state {
            let hp_per = (rs.current_hp as f32 / rs.max_hp.max(1) as f32) * 100.0;
            let choice = match event.id {
                EventId::ScrapOoze => {
                    // Safety net: don't commit suicide. Flee if HP < 25 and we are still deciding.
                    if rs.current_hp < 25 && event.current_screen == 0 {
                        1 // Leave
                    } else if event.current_screen == 1 {
                        0 // Any choice leaves on screen 1, but let's be safe
                    } else {
                        0 // Reach in
                    }
                },
                EventId::BigFish => if hp_per <= 30.0 { 0 } else { 1 },
                EventId::Cleric => if hp_per <= 65.0 { 0 } else if hp_per >= 90.0 { 2 } else { 1 },
                EventId::DeadAdventurer => 1, // Escape
                EventId::GoldenIdol => {
                    if rs.relics.iter().any(|r| r.id == RelicId::Ectoplasm) {
                        1 // Leave
                    } else if hp_per >= 90.0 {
                        1 // Take damage
                    } else {
                        2 // Max HP loss
                    }
                },
                EventId::Mushrooms => if hp_per >= 40.0 { 0 } else { 1 },
                EventId::LivingWall => 2, // Upgrade
                EventId::ShiningLight => if hp_per >= 70.0 { 0 } else { 1 },
                EventId::Ssssserpent => 1, // Leave
                EventId::WorldOfGoop => if hp_per >= 80.0 { 0 } else { 1 },
                EventId::MatchAndKeep => 0, // Keep rolling
                EventId::GoldenWing => if hp_per >= 70.0 { 0 } else { 1 },
                EventId::GoldenShrine => 0,
                EventId::Purifier => 0,
                EventId::UpgradeShrine => 0,
                EventId::Transmorgrifier => 1, // Leave
                EventId::Lab => 0, // Free potions
                EventId::Duplicator => 1, // Leave
                EventId::MaskedBandits => if hp_per >= 65.0 { 1 } else { 0 }, // 1 is fight
                EventId::Vampires => 2, // Refuse
                EventId::Ghosts => 1, // Refuse
                EventId::TheLibrary => 1, // Sleep/Heal
                EventId::CursedTome => 1, // Leave
                EventId::ForgottenAltar => 1, // Leave
                EventId::KnowingSkull => 3, // Leave
                EventId::Beggar => 0, 
                EventId::Falling => 0,
                EventId::MindBloom => 0, // Fight Boss
                EventId::Colosseum => 0, // Fight
                EventId::MysteriousSphere => if hp_per >= 70.0 { 0 } else { 1 }, // 0 is fight
                EventId::TombRedMask => 1, // Leave
                EventId::WindingHalls => 2, // Loss max HP
                EventId::Nest => 0,
                EventId::FaceTrader => 2,
                EventId::Nloth => 2,
                EventId::WomanInBlue => 0,
                
                // Global fallbacks
                _ => 0,
            };
            ClientInput::EventChoice(choice)
        } else {
            ClientInput::EventChoice(0)
        }
    }
}
