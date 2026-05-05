use crate::events::context::EventContext;
use crate::runtime::rng::RngPool;
use crate::state::events::EventId;

#[derive(Debug, Clone, PartialEq)]
pub enum RoomRoll {
    Monster,
    Shop,
    Treasure,
    Event,
    Elite, // Requires DeadlyEvents mod
}

#[derive(Debug, Clone, PartialEq)]
pub struct EventGenerator {
    pub event_pool: Vec<EventId>,
    pub shrine_pool: Vec<EventId>,
    pub one_time_event_pool: Vec<EventId>,

    pub monster_chance: f32,
    pub shop_chance: f32,
    pub treasure_chance: f32,
    pub shrine_chance: f32,
}

impl EventGenerator {
    pub fn new(act_num: u8) -> Self {
        let mut gen = Self {
            event_pool: Vec::new(),
            shrine_pool: Vec::new(),
            one_time_event_pool: Vec::new(),
            monster_chance: 0.10,
            shop_chance: 0.03,
            treasure_chance: 0.02,
            shrine_chance: 0.25,
        };
        gen.initialize_event_pools(act_num);
        gen
    }

    pub fn initialize_event_pools(&mut self, act_num: u8) {
        // Act-specific event pool
        self.event_pool = match act_num {
            1 => vec![
                EventId::BigFish,
                EventId::Cleric,
                EventId::DeadAdventurer,
                EventId::GoldenIdol,
                EventId::GoldenWing,
                EventId::WorldOfGoop,
                EventId::Ssssserpent,
                EventId::LivingWall,
                EventId::Mushrooms,
                EventId::ScrapOoze,
                EventId::ShiningLight,
            ],
            2 => vec![
                EventId::Addict,
                EventId::BackTotheBasics,
                EventId::Beggar,
                EventId::Colosseum,
                EventId::CursedTome,
                EventId::DrugDealer,
                EventId::ForgottenAltar,
                EventId::Ghosts,
                EventId::MaskedBandits,
                EventId::Nest,
                EventId::TheLibrary,
                EventId::Mausoleum,
                EventId::Vampires,
            ],
            _ => vec![
                EventId::Falling,
                EventId::MindBloom,
                EventId::MoaiHead,
                EventId::MysteriousSphere,
                EventId::SensoryStone,
                EventId::TombRedMask,
                EventId::WindingHalls,
            ],
        };

        // Per-act shrine pool
        self.shrine_pool = match act_num {
            1 => vec![
                EventId::MatchAndKeep,
                EventId::GoldenShrine,
                EventId::Transmorgrifier,
                EventId::Purifier,
                EventId::UpgradeShrine,
                EventId::GremlinWheelGame,
            ],
            2 => vec![
                EventId::MatchAndKeep,
                EventId::GremlinWheelGame,
                EventId::GoldenShrine,
                EventId::Transmorgrifier,
                EventId::Purifier,
                EventId::UpgradeShrine,
            ],
            _ => vec![
                EventId::MatchAndKeep,
                EventId::GremlinWheelGame,
                EventId::GoldenShrine,
                EventId::Transmorgrifier,
                EventId::Purifier,
                EventId::UpgradeShrine,
            ],
        };

        if self.one_time_event_pool.is_empty() {
            self.one_time_event_pool = vec![
                EventId::AccursedBlacksmith,
                EventId::BonfireElementals,
                EventId::Designer,
                EventId::Duplicator,
                EventId::FaceTrader,
                EventId::FountainOfCurseCleansing,
                EventId::KnowingSkull,
                EventId::Lab,
                EventId::Nloth,
                EventId::NoteForYourself,
                EventId::TheJoust,
                EventId::WeMeetAgain,
                EventId::WomanInBlue,
            ];
        }
    }

    /// Mirrors Java's EventHelper.roll(Random eventRng)
    pub fn roll_room_type(&mut self, rng: &mut RngPool, ctx: &EventContext) -> RoomRoll {
        let roll = rng.event_rng.random_f32();

        let mut force_chest = false;
        if ctx.tiny_chest_counter == 3 {
            // Java computes eventRng.random() before checking the Tiny Chest counter.
            force_chest = true;
        }

        let monster_size = (self.monster_chance * 100.0) as i32;
        let shop_size = (self.shop_chance * 100.0) as i32;
        let treasure_size = (self.treasure_chance * 100.0) as i32;

        let roll_idx = (roll * 100.0) as i32;

        let mut choice = if roll_idx < monster_size {
            RoomRoll::Monster
        } else if roll_idx < monster_size + shop_size {
            RoomRoll::Shop
        } else if roll_idx < monster_size + shop_size + treasure_size {
            RoomRoll::Treasure
        } else {
            RoomRoll::Event
        };

        if force_chest {
            choice = RoomRoll::Treasure;
        }

        if choice == RoomRoll::Monster {
            if ctx.has_juzu_bracelet {
                // Juzu Bracelet converts Monster to Event, but Monster chance still resets to 0.10.
                choice = RoomRoll::Event;
            }
            self.monster_chance = 0.10;
        } else {
            self.monster_chance += 0.10;
        }

        if choice == RoomRoll::Shop {
            self.shop_chance = 0.03;
        } else {
            self.shop_chance += 0.03;
        }

        if choice == RoomRoll::Treasure {
            self.treasure_chance = 0.02;
        } else {
            self.treasure_chance += 0.02;
        }

        choice
    }

    pub fn reset_probabilities(&mut self) {
        self.monster_chance = 0.10;
        self.shop_chance = 0.03;
        self.treasure_chance = 0.02;
    }

    /// Mirrors Java's AbstractDungeon.generateEvent(Random rng)
    pub fn generate_event(&mut self, rng: &mut RngPool, ctx: &EventContext) -> EventId {
        let shrine_roll = rng.event_rng.random_f32_range(1.0);

        if shrine_roll < self.shrine_chance {
            if !self.shrine_pool.is_empty() || !self.one_time_event_pool.is_empty() {
                return self.get_shrine_event(rng, ctx);
            }
            if !self.event_pool.is_empty() {
                return self.get_pool_event(rng, ctx);
            }
            return self.generate_event_fallback(rng);
        }

        if let Some(event) = self.try_get_pool_event(rng, ctx) {
            return event;
        }
        if !self.shrine_pool.is_empty() || !self.one_time_event_pool.is_empty() {
            return self.get_shrine_event(rng, ctx);
        }
        self.generate_event_fallback(rng)
    }

    fn get_shrine_event(&mut self, rng: &mut RngPool, ctx: &EventContext) -> EventId {
        let mut candidates: Vec<EventId> = Vec::new();
        candidates.extend_from_slice(&self.shrine_pool);

        for &event in &self.one_time_event_pool {
            let ok = match event {
                EventId::FountainOfCurseCleansing => ctx.has_curses,
                EventId::Designer => (ctx.act_num == 2 || ctx.act_num == 3) && ctx.gold >= 75,
                EventId::Duplicator => ctx.act_num == 2 || ctx.act_num == 3,
                EventId::FaceTrader => ctx.act_num == 1 || ctx.act_num == 2,
                EventId::KnowingSkull => ctx.act_num == 2 && ctx.current_hp > 12,
                EventId::Nloth => ctx.act_num == 2 && ctx.relic_count >= 2,
                EventId::TheJoust => ctx.act_num == 2 && ctx.gold >= 50,
                EventId::WomanInBlue => ctx.gold >= 50,
                EventId::NoteForYourself => ctx.ascension_level < 15,
                _ => true,
            };
            if ok {
                candidates.push(event);
            }
        }

        if candidates.is_empty() {
            return self.generate_event_fallback(rng);
        }

        let idx = rng.event_rng.random_range(0, (candidates.len() - 1) as i32) as usize;
        let chosen = candidates[idx];

        if let Some(pos) = self.shrine_pool.iter().position(|&e| e == chosen) {
            self.shrine_pool.remove(pos);
        }
        if let Some(pos) = self.one_time_event_pool.iter().position(|&e| e == chosen) {
            self.one_time_event_pool.remove(pos);
        }

        chosen
    }

    fn get_pool_event(&mut self, rng: &mut RngPool, ctx: &EventContext) -> EventId {
        self.try_get_pool_event(rng, ctx)
            .unwrap_or_else(|| self.generate_event_fallback(rng))
    }

    fn try_get_pool_event(&mut self, rng: &mut RngPool, ctx: &EventContext) -> Option<EventId> {
        let hp_pct = if ctx.max_hp > 0 {
            ctx.current_hp as f32 / ctx.max_hp as f32
        } else {
            1.0
        };
        let map_midpoint = 7;

        let mut candidates: Vec<EventId> = Vec::new();
        for &event in &self.event_pool {
            let ok = match event {
                EventId::DeadAdventurer => ctx.floor_num > 6,
                EventId::Mushrooms => ctx.floor_num > 6,
                EventId::MoaiHead => ctx.has_golden_idol || hp_pct <= 0.5,
                EventId::Cleric => ctx.gold >= 35,
                EventId::Beggar => ctx.gold >= 75,
                EventId::Colosseum => ctx.floor_num > map_midpoint,
                _ => true,
            };
            if ok {
                candidates.push(event);
            }
        }

        if candidates.is_empty() {
            return None;
        }

        let idx = rng.event_rng.random_range(0, (candidates.len() - 1) as i32) as usize;
        let chosen = candidates[idx];

        if let Some(pos) = self.event_pool.iter().position(|&e| e == chosen) {
            self.event_pool.remove(pos);
            if self.event_pool.is_empty() {
                self.repopulate_event_list(ctx.act_num);
            }
        }

        Some(chosen)
    }

    fn repopulate_event_list(&mut self, act_num: u8) {
        self.event_pool = match act_num {
            1 => vec![
                EventId::BigFish,
                EventId::Cleric,
                EventId::DeadAdventurer,
                EventId::GoldenIdol,
                EventId::GoldenWing,
                EventId::WorldOfGoop,
                EventId::Ssssserpent,
                EventId::LivingWall,
                EventId::Mushrooms,
                EventId::ScrapOoze,
                EventId::ShiningLight,
            ],
            2 => vec![
                EventId::Addict,
                EventId::BackTotheBasics,
                EventId::Beggar,
                EventId::Colosseum,
                EventId::CursedTome,
                EventId::DrugDealer,
                EventId::ForgottenAltar,
                EventId::Ghosts,
                EventId::MaskedBandits,
                EventId::Nest,
                EventId::TheLibrary,
                EventId::Mausoleum,
                EventId::Vampires,
            ],
            _ => vec![
                EventId::Falling,
                EventId::MindBloom,
                EventId::MoaiHead,
                EventId::MysteriousSphere,
                EventId::SensoryStone,
                EventId::TombRedMask,
                EventId::WindingHalls,
            ],
        };
    }

    fn generate_event_fallback(&mut self, rng: &mut RngPool) -> EventId {
        let options = [EventId::Cleric, EventId::GoldenIdol, EventId::GoldenShrine];
        let idx = rng.event_rng.random_range(0, 2) as usize;
        options[idx]
    }
}
