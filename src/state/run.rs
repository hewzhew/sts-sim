use crate::combat::{CombatCard, PlayerEntity};
use crate::content::relics::RelicState;
use crate::map::state::MapState;
use crate::rng::RngPool;

#[derive(Debug, Clone, PartialEq)]
pub struct RunState {
    pub seed: u64,
    pub ascension_level: u8,
    pub act_num: u8,
    pub floor_num: i32,
    pub player_class: &'static str, // "Ironclad", "Silent", "Defect", "Watcher"

    pub map: MapState,
    pub rng_pool: RngPool,

    // Persistent out-of-combat attributes decoupled from CombatState
    pub current_hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub shop_purge_count: i32,
    pub relics: Vec<RelicState>,
    pub potions: Vec<Option<crate::content::potions::Potion>>,
    pub keys: [bool; 3], // Red, Green, Blue
    pub is_final_act_available: bool,
    pub master_deck: Vec<CombatCard>,
    
    // Drop modifiers
    pub potion_drop_chance_mod: i32,
    pub card_blizz_randomizer: i32,
    /// Java: cardUpgradedChance — probability that a card reward is pre-upgraded.
    /// Set per act: Exordium=0.0, TheCity=0.25 (0.125 Asc12+), TheBeyond=0.5 (0.25 Asc12+)
    pub card_upgraded_chance: f32,
    
    // Transient states
    pub reward_state: Option<crate::state::reward::RewardState>,
    pub shop_state: Option<crate::state::shop::ShopState>,
    pub event_state: Option<crate::state::events::EventState>,

    // Event pools (Java: eventList, shrineList, specialOneTimeEventList)
    pub event_pool: Vec<crate::state::events::EventId>,
    pub shrine_pool: Vec<crate::state::events::EventId>,
    pub one_time_event_pool: Vec<crate::state::events::EventId>,

    // Event room-type chance trackers (Java: EventHelper static fields)
    pub event_monster_chance: f32,
    pub event_shop_chance: f32,
    pub event_treasure_chance: f32,
    pub shrine_chance: f32, // Java: AbstractDungeon.shrineChance = 0.25

    // Relic pools — filled at dungeon init, popped from end when obtaining relics.
    // Java: commonRelicPool, uncommonRelicPool, rareRelicPool, shopRelicPool, bossRelicPool
    pub common_relic_pool: Vec<crate::content::relics::RelicId>,
    pub uncommon_relic_pool: Vec<crate::content::relics::RelicId>,
    pub rare_relic_pool: Vec<crate::content::relics::RelicId>,
    pub shop_relic_pool: Vec<crate::content::relics::RelicId>,
    pub boss_relic_pool: Vec<crate::content::relics::RelicId>,

    // Encounter scheduling queues (Java: monsterList, eliteMonsterList)
    // Populated at dungeon init via weighted roll; consumed in order as player enters combat rooms.
    pub monster_list: Vec<crate::content::monsters::factory::EncounterId>,
    pub elite_monster_list: Vec<crate::content::monsters::factory::EncounterId>,
    pub boss_list: Vec<crate::content::monsters::factory::EncounterId>,

    /// Set to true after boss combat ends, consumed by post_reward_state() to trigger act advance.
    pub pending_boss_reward: bool,
}

impl RunState {
    pub fn new(seed: u64, ascension_level: u8, final_act: bool, player_class: &'static str) -> Self {
        // Generate Act 1 map; returns the consumed mapRng for emerald key placement.
        let (mut first_map, mut map_rng) = crate::map::generator::generate_map_for_act(seed, 1, ascension_level == 0);
        // Mark a random elite node for Emerald Key if Act 4 is enabled.
        // Java: setEmeraldElite() reuses the consumed mapRng, not a fresh one.
        if final_act {
            crate::map::generator::set_emerald_elite(&mut first_map, &mut map_rng);
        }
        let mut rs = RunState {
            seed,
            ascension_level,
            act_num: 1,
            floor_num: 0,
            player_class,
            map: MapState::new(first_map),
            rng_pool: RngPool::new(seed),
            
            // Typical Ironclad defaults
            current_hp: 80,
            max_hp: 80,
            gold: 99,
            shop_purge_count: 0,
            relics: Vec::new(),
            potions: if ascension_level >= 11 { vec![None, None] } else { vec![None, None, None] },
            keys: [false; 3],
            is_final_act_available: final_act,
            master_deck: Vec::new(),
            potion_drop_chance_mod: 0,
            card_blizz_randomizer: 5, // Java: cardBlizzStartOffset = 5 (added to roll, makes rare harder)
            card_upgraded_chance: 0.0, // Exordium: always 0.0 (Java: Exordium.cardUpgradedChance = 0.0f)

            // Transient states
            reward_state: None,
            shop_state: None,
            event_state: None,

            // Event pools initialized for Act 1
            event_pool: Vec::new(),
            shrine_pool: Vec::new(),
            one_time_event_pool: Vec::new(),

            // Event room-type chances (Java: resetProbabilities)
            event_monster_chance: 0.10,
            event_shop_chance: 0.03,
            event_treasure_chance: 0.02,
            shrine_chance: 0.25,

            // Relic pools (filled by init_relic_pools)
            common_relic_pool: Vec::new(),
            uncommon_relic_pool: Vec::new(),
            rare_relic_pool: Vec::new(),
            shop_relic_pool: Vec::new(),
            boss_relic_pool: Vec::new(),

            // Encounter lists (filled by generate_encounter_lists)
            monster_list: Vec::new(),
            elite_monster_list: Vec::new(),
            boss_list: Vec::new(),

            pending_boss_reward: false,
        };
        rs.initialize_event_pools();
        rs.init_relic_pools();
        rs.init_encounter_lists();
        rs.init_boss_list();

        // --- Ascension metagame effects (Java: AbstractDungeon.java L2562-2580) ---

        // Asc 14: Decrease max HP at run start (Java: player.decreaseMaxHealth(player.getAscensionMaxHPLoss()))
        // Ironclad returns 5, Silent returns 4, Defect returns 4, Watcher returns 4
        if ascension_level >= 14 {
            let hp_loss = match player_class {
                "Ironclad" => 5,
                "Silent" => 4,
                "Defect" => 4,
                "Watcher" => 4,
                _ => 5,
            };
            rs.max_hp -= hp_loss;
            rs.current_hp = rs.current_hp.min(rs.max_hp);
        }

        // Asc 6: Start at 90% of max HP (Java: player.currentHealth = round(maxHealth * 0.9f))
        if ascension_level >= 6 {
            rs.current_hp = (rs.max_hp as f32 * 0.9).round() as i32;
        }

        // Asc 10: Add Ascender's Bane (unremovable curse) to starting deck
        if ascension_level >= 10 {
            let uuid = 9999; // High UUID to avoid conflicts with starter deck
            rs.master_deck.push(crate::combat::CombatCard::new(
                crate::content::cards::CardId::AscendersBane, uuid
            ));
        }

        // Initialize Neow event for run start
        crate::content::events::neow::setup_neow_choices(&mut rs);
        rs
    }

    /// Maps player_class string to PotionClass enum for potion generation.
    pub fn potion_class(&self) -> crate::content::potions::PotionClass {
        match self.player_class {
            "Ironclad" => crate::content::potions::PotionClass::Ironclad,
            "Silent" => crate::content::potions::PotionClass::Silent,
            "Defect" => crate::content::potions::PotionClass::Defect,
            "Watcher" => crate::content::potions::PotionClass::Watcher,
            _ => crate::content::potions::PotionClass::Ironclad,
        }
    }

    /// Extends this RunState's persistent player properties into a temporary CombatState player.
    pub fn build_combat_player(&self, id: crate::core::EntityId) -> PlayerEntity {
        let mut p = PlayerEntity {
            id,
            current_hp: self.current_hp,
            max_hp: self.max_hp,
            block: 0,
            gold_delta_this_combat: 0,
            gold: self.gold,
            max_orbs: 0,
            orbs: Vec::new(),
            stance: crate::combat::StanceId::Neutral,
            relics: Vec::new(),
            relic_buses: Default::default(),
        };
        // Safely re-initialize the bus mappings through the add_relic interface
        for rs in &self.relics {
            p.add_relic(rs.clone());
        }
        p
    }

    /// Recovers persistent properties modified by the combat simulation
    pub fn absorb_combat_player(&mut self, player: PlayerEntity) {
        self.current_hp = player.current_hp;
        self.max_hp = player.max_hp;
        // Don't arbitrarily overwrite gold directly from player.gold in case we have to manage specific event drops,
        // but typically player.gold is synced directly, or using gold_delta_this_combat:
        self.gold = player.gold;

        // Persist counter states inside relics (e.g., Pen Nib)
        self.relics = player.relics;
    }

    /// Central Mutator Gateway for permanently removing a card from the deck.
    /// Handles all Global/Out-Of-Combat curse triggers securely.
    pub fn remove_card_from_master_deck(&mut self, uuid: u32) -> Result<(), &'static str> {
        let card_index = self.master_deck.iter().position(|c| c.uuid == uuid);
        
        if let Some(idx) = card_index {
            let card = self.master_deck.remove(idx);
            
            // Global Curse Hooks
            match card.id {
                crate::content::cards::CardId::Parasite => {
                    self.max_hp = (self.max_hp - 3).max(1);
                    self.current_hp = self.current_hp.min(self.max_hp);
                    // Sound effect trigger placeholder
                },
                crate::content::cards::CardId::Necronomicurse => {
                    // Block the removal by putting it right back
                    self.master_deck.push(card);
                    // Relic flash trigger placeholder
                },
                _ => {}
            }
            Ok(())
        } else {
            Err("Card UUID not found in master deck")
        }
    }

    /// Triggers when the player enters a Rest Room (Campfire).
    pub fn on_enter_rest_room(&mut self) {
        for relic in &mut self.relics {
            let sub = crate::content::relics::get_relic_subscriptions(relic.id);
            if sub.on_enter_rest_room {
                match relic.id {
                    crate::content::relics::RelicId::AncientTeaSet => {
                        crate::content::relics::ancient_tea_set::AncientTeaSet::on_enter_rest_room(relic);
                    }
                    _ => {}
                }
            }
        }
    }

    /// Generates post-combat loot transitioning into the RewardState
    pub fn generate_combat_rewards(&mut self, is_elite: bool, is_boss: bool) -> crate::state::reward::RewardState {
        let mut items = Vec::new();

        let has_golden_idol = self.relics.iter().any(|r| r.id == crate::content::relics::RelicId::GoldenIdol);
        let has_busted_crown = self.relics.iter().any(|r| r.id == crate::content::relics::RelicId::BustedCrown);
        let has_question_card = self.relics.iter().any(|r| r.id == crate::content::relics::RelicId::QuestionCard);
        let has_prayer_wheel = self.relics.iter().any(|r| r.id == crate::content::relics::RelicId::PrayerWheel);

        let has_ectoplasm = self.relics.iter().any(|r| r.id == crate::content::relics::RelicId::Ectoplasm);
        
        // 1. Generate Gold
        if !has_ectoplasm {
            if is_boss {
                let mut amount = 100 + self.rng_pool.misc_rng.random_range(-5, 5);
                if self.ascension_level >= 13 {
                    amount = (amount as f32 * 0.75).round() as i32;
                }
                if has_golden_idol {
                    amount += (amount as f32 * 0.25).round() as i32;
                }
                items.push(crate::state::RewardItem::Gold { amount });
            } else {
                let mut amount = if is_elite {
                    self.rng_pool.treasure_rng.random_range(25, 35)
                } else {
                    self.rng_pool.treasure_rng.random_range(10, 20)
                };
                if has_golden_idol {
                    amount += (amount as f32 * 0.25).round() as i32;
                }
                items.push(crate::state::RewardItem::Gold { amount });
            }
        }

        // 2. Generate Potions
        let has_sozu = self.relics.iter().any(|r| r.id == crate::content::relics::RelicId::Sozu);
        if !has_sozu {
            let mut chance = 40 + self.potion_drop_chance_mod;
            if self.relics.iter().any(|r| r.id == crate::content::relics::RelicId::WhiteBeastStatue) {
                chance = 100;
            }
            
            let roll = self.rng_pool.potion_rng.random_range(0, 99);
            if roll < chance {
                self.potion_drop_chance_mod -= 10;
                let potion_class = self.potion_class_from_player();
                let potion_id = crate::content::potions::random_potion(
                    &mut self.rng_pool.potion_rng,
                    potion_class,
                    false,
                );
                items.push(crate::state::RewardItem::Potion { potion_id });
            } else {
                self.potion_drop_chance_mod += 10;
            }
        }

        // 3. Generate Cards
        let num_cards = 3_usize;
        let mut num_cards_eff = num_cards;
        if has_busted_crown {
            num_cards_eff = num_cards_eff.saturating_sub(2).max(1);
        }
        if has_question_card {
            num_cards_eff += 1;
        }

        items.push(crate::state::RewardItem::Card { cards: self.generate_card_reward(num_cards_eff) });
        if !is_boss && has_prayer_wheel {
            items.push(crate::state::RewardItem::Card { cards: self.generate_card_reward(num_cards_eff) });
        }

        if is_elite || is_boss {
            // Java: MonsterRoomElite.dropReward() → addRelicToRewards(returnRandomRelicTier())
            let relic_id = self.random_relic();
            items.push(crate::state::RewardItem::Relic { relic_id });

            // BlackStar: second relic reward from elites
            // Java: addNoncampRelicToRewards(returnRandomRelicTier())
            if is_elite && self.relics.iter().any(|r| r.id == crate::content::relics::RelicId::BlackStar) {
                let relic_id2 = self.random_relic();
                items.push(crate::state::RewardItem::Relic { relic_id: relic_id2 });
            }
        }

        crate::state::reward::RewardState {
            items,
            skippable: !is_boss,
            pending_card_choice: None,
        }
    }

    /// Generates ShopState with randomized prices, accounting for merchant Relics
    pub fn generate_shop(&mut self) -> crate::state::shop::ShopState {
        let mut shop = crate::state::shop::ShopState::new();

        let has_membership_card = self.relics.iter().any(|r| r.id == crate::content::relics::RelicId::MembershipCard);
        let has_courier = self.relics.iter().any(|r| r.id == crate::content::relics::RelicId::Courier);
        let has_discerning_monocle = self.relics.iter().any(|r| r.id == crate::content::relics::RelicId::DiscerningMonocle);
        let has_smiling_mask = self.relics.iter().any(|r| r.id == crate::content::relics::RelicId::SmilingMask);
        let ascension_level = self.ascension_level;

        // Applies the precise Java sequential rounding chain for prices
        let apply_modifier = |mut base: f32, is_purge: bool| -> i32 {
            if ascension_level >= 16 && !is_purge {
                base = (base * 1.1).round();
            }
            if has_courier {
                base = (base * 0.8).round();
            }
            if has_discerning_monocle {
                base = (base * 0.8).round();
            }
            if has_membership_card {
                base = (base * 0.5).round();
            }
            base as i32
        };

        // 1. Cards (7 slots: 5 colored, 2 colorless)
        // Java: Merchant generates 2 Attack, 2 Skill, 1 Power with rollRarity() each
        // Base prices: Common=50, Uncommon=75, Rare=150
        let card_type_slots = [
            crate::content::cards::CardType::Attack,
            crate::content::cards::CardType::Attack,
            crate::content::cards::CardType::Skill,
            crate::content::cards::CardType::Skill,
            crate::content::cards::CardType::Power,
        ];

        let sale_index = self.rng_pool.merchant_rng.random_range(0, 4) as usize;
        let mut used_cards: Vec<crate::content::cards::CardId> = Vec::new();

        for (i, &card_type) in card_type_slots.iter().enumerate() {
            // Roll rarity for this slot
            let rarity_roll = self.rng_pool.card_rng.random_range(0, 99);
            let rarity = if rarity_roll < 9 {
                crate::content::cards::CardRarity::Rare
            } else if rarity_roll < 37 {
                crate::content::cards::CardRarity::Uncommon
            } else {
                crate::content::cards::CardRarity::Common
            };

            let base_price: f32 = match rarity {
                crate::content::cards::CardRarity::Common => 50.0,
                crate::content::cards::CardRarity::Uncommon => 75.0,
                crate::content::cards::CardRarity::Rare => 150.0,
                _ => 50.0,
            };

            // Select card from pool, filtered by type, avoiding duplicates
            let pool = crate::content::cards::ironclad_pool_for_rarity(rarity);
            let typed_pool: Vec<crate::content::cards::CardId> = pool.iter()
                .copied()
                .filter(|&id| crate::content::cards::get_card_definition(id).card_type == card_type)
                .collect();

            let card_id = if !typed_pool.is_empty() {
                let mut attempts = 0;
                loop {
                    let idx = self.rng_pool.merchant_rng.random_range(0, (typed_pool.len() - 1) as i32) as usize;
                    let candidate = typed_pool[idx];
                    if !used_cards.contains(&candidate) || attempts >= 20 {
                        used_cards.push(candidate);
                        break candidate;
                    }
                    attempts += 1;
                }
            } else {
                // Fallback: pick any card from the rarity pool
                let idx = self.rng_pool.merchant_rng.random_range(0, (pool.len() - 1) as i32) as usize;
                pool[idx]
            };

            let limit = (base_price * 0.1) as i32;
            let jitter = self.rng_pool.merchant_rng.random_range(-limit, limit);
            let mut price = base_price + jitter as f32;
            price = price.trunc();

            if i == sale_index {
                price = (price * 0.5).trunc();
            }

            shop.cards.push(crate::state::shop::ShopCard {
                card_id,
                price: apply_modifier(price, false),
            });
        }

        // Colorless cards (2 slots: 1 Uncommon, 1 Rare)
        let colorless_slots: [(f32, &[crate::content::cards::CardId]); 2] = [
            (75.0 * 1.2, crate::content::cards::COLORLESS_UNCOMMON_POOL),
            (150.0 * 1.2, crate::content::cards::COLORLESS_RARE_POOL),
        ];

        for (base, pool) in colorless_slots.iter() {
            let idx = self.rng_pool.merchant_rng.random_range(0, (pool.len() - 1) as i32) as usize;
            let card_id = pool[idx];
            let limit = (*base * 0.1) as i32;
            let jitter = self.rng_pool.merchant_rng.random_range(-limit, limit);
            let price = (*base + jitter as f32).trunc();
            shop.cards.push(crate::state::shop::ShopCard {
                card_id,
                price: apply_modifier(price, false),
            });
        }

        // 2. Relics (3 slots) — Java: ShopScreen.initRelics()
        // Slots 0-1: rollRelicTier() → returnRandomRelicEnd(tier)
        // Slot 2: fixed SHOP tier → returnRandomRelicEnd(SHOP)
        for i in 0..3 {
            let tier = if i != 2 {
                // Java: ShopScreen.rollRelicTier() — uses merchantRng
                let roll = self.rng_pool.merchant_rng.random_range(0, 99);
                if roll < 48 {
                    crate::content::relics::RelicTier::Common
                } else if roll < 82 {
                    crate::content::relics::RelicTier::Uncommon
                } else {
                    crate::content::relics::RelicTier::Rare
                }
            } else {
                crate::content::relics::RelicTier::Shop
            };

            let relic_id = self.random_relic_by_tier(tier);

            // Base price from tier (Java: AbstractRelic.getPrice())
            let base_price: f32 = match tier {
                crate::content::relics::RelicTier::Common => 150.0,
                crate::content::relics::RelicTier::Uncommon => 250.0,
                crate::content::relics::RelicTier::Rare => 300.0,
                crate::content::relics::RelicTier::Shop => 150.0,
                _ => 150.0,
            };

            // Price jitter: merchantRng.random(0.95f, 1.05f) → start + nextFloat() * (end - start)
            let jitter_mult = 0.95_f32 + self.rng_pool.merchant_rng.random_f32_range(0.1);
            let price = (base_price * jitter_mult).round();

            shop.relics.push(crate::state::shop::ShopRelic {
                relic_id,
                price: apply_modifier(price, false),
            });
        }

        // 3. Potions (3 slots) — Java: ShopScreen.initPotions()
        // Generate 3 random potions with rarity-based pricing + merchant jitter
        let pc = self.potion_class();
        for _ in 0..3 {
            let potion_id = crate::content::potions::random_potion(
                &mut self.rng_pool.potion_rng,
                pc,
                false,
            );
            let base_price = crate::content::potions::get_potion_price(potion_id) as f32;
            let limit = (base_price * 0.05) as i32;
            let jitter = self.rng_pool.merchant_rng.random_range(-limit, limit);
            let price = (base_price + jitter as f32).round();
            shop.potions.push(crate::state::shop::ShopPotion {
                potion_id,
                price: apply_modifier(price, false),
            });
        }

        // 4. Purge Logic
        if has_smiling_mask {
            shop.purge_cost = apply_modifier(50.0, true);
        } else {
            let mut base_purge = 75.0 + (self.shop_purge_count as f32 * 25.0);
            // Asc 10: Card removal costs 25 more
            if self.ascension_level >= 10 {
                base_purge += 25.0;
            }
            shop.purge_cost = apply_modifier(base_purge, true);
        }

        shop
    }

    /// Initialize event pools for the current act, matching Java Exordium/TheCity/TheBeyond.initializeEventList()
    /// and AbstractDungeon.initializeSpecialOneTimeEventList().
    pub fn initialize_event_pools(&mut self) {
        use crate::state::events::EventId;

        // Act-specific event pool (Java: eventList)
        self.event_pool = match self.act_num {
            1 => vec![
                EventId::BigFish, EventId::Cleric, EventId::DeadAdventurer,
                EventId::GoldenIdol, EventId::GoldenWing, EventId::WorldOfGoop,
                EventId::Ssssserpent, EventId::LivingWall, EventId::Mushrooms,
                EventId::ScrapOoze, EventId::ShiningLight,
            ],
            2 => vec![
                EventId::Addict, EventId::BackTotheBasics, EventId::Beggar,
                EventId::Colosseum, EventId::CursedTome, EventId::DrugDealer,
                EventId::ForgottenAltar, EventId::Ghosts, EventId::MaskedBandits,
                EventId::Nest, EventId::TheLibrary, EventId::Mausoleum,
                EventId::Vampires,
            ],
            _ => vec![
                EventId::Falling, EventId::MindBloom, EventId::MoaiHead,
                EventId::MysteriousSphere, EventId::SensoryStone,
                EventId::TombRedMask, EventId::WindingHalls,
            ],
        };

        // Per-act shrine pool (Java: shrineList)
        self.shrine_pool = match self.act_num {
            1 => vec![
                EventId::MatchAndKeep, EventId::GoldenShrine,
                EventId::Transmorgrifier, EventId::Purifier,
                EventId::UpgradeShrine, EventId::GremlinWheelGame,
            ],
            2 => vec![
                EventId::MatchAndKeep, EventId::GremlinWheelGame,
                EventId::GoldenShrine, EventId::Transmorgrifier,
                EventId::Purifier, EventId::UpgradeShrine,
            ],
            _ => vec![
                EventId::MatchAndKeep, EventId::GremlinWheelGame,
                EventId::GoldenShrine, EventId::Transmorgrifier,
                EventId::Purifier, EventId::UpgradeShrine,
            ],
        };

        // Cross-act one-time events (Java: specialOneTimeEventList, set once at dungeon init)
        // Only initialize if empty (they persist across acts)
        if self.one_time_event_pool.is_empty() {
            self.one_time_event_pool = vec![
                EventId::AccursedBlacksmith, EventId::BonfireElementals,
                EventId::Designer, EventId::Duplicator,
                EventId::FaceTrader, EventId::FountainOfCurseCleansing,
                EventId::KnowingSkull, EventId::Lab, EventId::Nloth,
                EventId::NoteForYourself,
                EventId::TheJoust, EventId::WeMeetAgain,
                EventId::WomanInBlue,
            ];
        }
    }

    /// Only repopulate eventList when it's exhausted (Java behavior).
    /// Does NOT reset shrine_pool or one_time_event_pool.
    fn repopulate_event_list(&mut self) {
        use crate::state::events::EventId;
        self.event_pool = match self.act_num {
            1 => vec![
                EventId::BigFish, EventId::Cleric, EventId::DeadAdventurer,
                EventId::GoldenIdol, EventId::GoldenWing, EventId::WorldOfGoop,
                EventId::Ssssserpent, EventId::LivingWall, EventId::Mushrooms,
                EventId::ScrapOoze, EventId::ShiningLight,
            ],
            2 => vec![
                EventId::Addict, EventId::BackTotheBasics, EventId::Beggar,
                EventId::Colosseum, EventId::CursedTome, EventId::DrugDealer,
                EventId::ForgottenAltar, EventId::Ghosts, EventId::MaskedBandits,
                EventId::Nest, EventId::TheLibrary, EventId::Mausoleum,
                EventId::Vampires,
            ],
            _ => vec![
                EventId::Falling, EventId::MindBloom, EventId::MoaiHead,
                EventId::MysteriousSphere, EventId::SensoryStone,
                EventId::TombRedMask, EventId::WindingHalls,
            ],
        };
    }

    /// Mirrors Java's AbstractDungeon.generateEvent(Random rng).
    /// Two-stage decision: 25% chance shrine/oneTime, 75% chance eventList.
    pub fn generate_event(&mut self) -> crate::state::events::EventId {

        // Step 1: Room-type roll (Java: EventHelper.roll())
        // Uses eventRng.random() → nextFloat for seed parity.
        // We compute the room type but always return Event (no room conversion support).
        let roll = self.rng_pool.event_rng.random_f32(); // Java: eventRng.random()

        // TinyChest: increment counter each ? room. Every 4th forces treasure.
        // Java: EventHelper.roll() L99-106
        let mut force_chest = false;
        if let Some(tc) = self.relics.iter_mut().find(|r| r.id == crate::content::relics::RelicId::TinyChest) {
            tc.counter += 1;
            if tc.counter == 4 {
                tc.counter = 0;
                force_chest = true;
            }
        }

        let monster_size = (self.event_monster_chance * 100.0) as i32;
        let shop_size = (self.event_shop_chance * 100.0) as i32;
        let treasure_size = (self.event_treasure_chance * 100.0) as i32;
        let roll_idx = (roll * 100.0) as i32;

        // Determine what room type WOULD have been hit
        let mut fill = 0;
        let mut is_monster = roll_idx < fill + monster_size;
        fill += monster_size;
        let is_shop = !is_monster && roll_idx < fill + shop_size;
        fill += shop_size;
        let mut is_treasure = !is_monster && !is_shop && roll_idx < fill + treasure_size;

        // TinyChest: force treasure if counter hit 4
        if force_chest {
            is_treasure = true;
            is_monster = false;
        }

        // JuzuBracelet: convert MONSTER roll to EVENT (Java: EventHelper.roll() L150-153)
        if is_monster {
            if self.relics.iter().any(|r| r.id == crate::content::relics::RelicId::JuzuBracelet) {
                is_monster = false; // Converted to EVENT
            }
        }

        // Reset/ramp chance trackers (Java: EventHelper.roll() lines 149-179)
        if is_monster {
            self.event_monster_chance = 0.10;
        } else {
            self.event_monster_chance += 0.10;
        }
        if is_shop {
            self.event_shop_chance = 0.03;
        } else {
            self.event_shop_chance += 0.03;
        }
        if is_treasure {
            self.event_treasure_chance = 0.02;
        } else {
            self.event_treasure_chance += 0.02;
        }

        // Step 2: Two-stage event selection (Java: AbstractDungeon.generateEvent)
        // Roll rng.random(1.0f) < shrineChance → getShrine, else getEvent
        let shrine_roll = self.rng_pool.event_rng.random_f32_range(1.0); // Java: rng.random(1.0f)

        if shrine_roll < self.shrine_chance {
            // Try shrine/oneTime pool first
            if !self.shrine_pool.is_empty() || !self.one_time_event_pool.is_empty() {
                return self.get_shrine_event();
            }
            // Fallback to eventList if shrine pool empty
            if !self.event_pool.is_empty() {
                return self.get_pool_event();
            }
            return self.generate_event_fallback();
        }

        // Try eventList first
        if let Some(event) = self.try_get_pool_event() {
            return event;
        }
        // Fallback to shrine if eventList exhausted
        if !self.shrine_pool.is_empty() || !self.one_time_event_pool.is_empty() {
            return self.get_shrine_event();
        }
        self.generate_event_fallback()
    }

    /// Java: AbstractDungeon.getShrine(Random rng)
    /// Builds filtered candidate list from shrine_pool + one_time_event_pool with conditions.
    fn get_shrine_event(&mut self) -> crate::state::events::EventId {
        use crate::state::events::EventId;

        let mut candidates: Vec<EventId> = Vec::new();
        candidates.extend_from_slice(&self.shrine_pool);

        // Add one-time events with Java condition checks
        let act = self.act_num;
        let gold = self.gold;
        let hp = self.current_hp;
        let relic_count = self.relics.len();
        let asc = self.ascension_level;
        let has_curse = self.master_deck.iter().any(|c| {
            let def = crate::content::cards::get_card_definition(c.id);
            def.card_type == crate::content::cards::CardType::Curse
        });

        for &event in &self.one_time_event_pool {
            let ok = match event {
                EventId::FountainOfCurseCleansing => has_curse,
                EventId::Designer => (act == 2 || act == 3) && gold >= 75,
                EventId::Duplicator => act == 2 || act == 3,
                EventId::FaceTrader => act == 1 || act == 2,
                EventId::KnowingSkull => act == 2 && hp > 12,
                EventId::Nloth => act == 2 && relic_count >= 2,
                EventId::TheJoust => act == 2 && gold >= 50,
                EventId::WomanInBlue => gold >= 50,
                EventId::NoteForYourself => asc < 15,
                _ => true, // AccursedBlacksmith, BonfireElementals, Lab, WeMeetAgain, etc.
            };
            if ok {
                candidates.push(event);
            }
        }

        if candidates.is_empty() {
            return self.generate_event_fallback();
        }

        let idx = self.rng_pool.event_rng.random_range(0, (candidates.len() - 1) as i32) as usize;
        let chosen = candidates[idx];

        // Remove from source pool
        if let Some(pos) = self.shrine_pool.iter().position(|&e| e == chosen) {
            self.shrine_pool.remove(pos);
        }
        if let Some(pos) = self.one_time_event_pool.iter().position(|&e| e == chosen) {
            self.one_time_event_pool.remove(pos);
        }

        chosen
    }

    /// Java: AbstractDungeon.getEvent(Random rng)
    /// Builds filtered candidate list from event_pool with per-event conditions.
    fn get_pool_event(&mut self) -> crate::state::events::EventId {
        self.try_get_pool_event().unwrap_or_else(|| self.generate_event_fallback())
    }

    /// Try to pick from eventList with conditions. Returns None if pool exhausted.
    fn try_get_pool_event(&mut self) -> Option<crate::state::events::EventId> {
        use crate::state::events::EventId;
        use crate::content::relics::RelicId;

        let floor = self.floor_num;
        let gold = self.gold;
        let hp_pct = if self.max_hp > 0 { self.current_hp as f32 / self.max_hp as f32 } else { 1.0 };
        let has_golden_idol = self.relics.iter().any(|r| r.id == RelicId::GoldenIdol);
        // Map midpoint: assume 15 floors per act, midpoint = 7
        let map_midpoint = 7;

        let mut candidates: Vec<EventId> = Vec::new();
        for &event in &self.event_pool {
            let ok = match event {
                EventId::DeadAdventurer => floor > 6,
                EventId::Mushrooms => floor > 6,
                EventId::MoaiHead => has_golden_idol || hp_pct <= 0.5,
                EventId::Cleric => gold >= 35,
                EventId::Beggar => gold >= 75,
                EventId::Colosseum => floor > map_midpoint,
                _ => true,
            };
            if ok {
                candidates.push(event);
            }
        }

        if candidates.is_empty() {
            return None;
        }

        let idx = self.rng_pool.event_rng.random_range(0, (candidates.len() - 1) as i32) as usize;
        let chosen = candidates[idx];

        // Remove from event_pool
        if let Some(pos) = self.event_pool.iter().position(|&e| e == chosen) {
            self.event_pool.remove(pos);
            if self.event_pool.is_empty() {
                self.repopulate_event_list();
            }
        }

        Some(chosen)
    }

    /// Fallback for generate_event when pools are unexpectedly empty
    fn generate_event_fallback(&mut self) -> crate::state::events::EventId {
        use crate::state::events::EventId;
        let options = [EventId::Cleric, EventId::GoldenIdol, EventId::GoldenShrine];
        let idx = self.rng_pool.event_rng.random_range(0, 2) as usize;
        options[idx]
    }

    /// Adds a card to the master deck with auto-generated UUID.
    /// If the card is a Curse and the player has Omamori with charges, the curse is negated instead.
    /// Returns true if the card was actually added (false if Omamori blocked it).
    pub fn add_card_to_deck(&mut self, card_id: crate::content::cards::CardId) -> bool {
        let def = crate::content::cards::get_card_definition(card_id);
        
        // Omamori: negate curses
        if def.card_type == crate::content::cards::CardType::Curse {
            if let Some(omamori) = self.relics.iter_mut().find(|r| r.id == crate::content::relics::RelicId::Omamori) {
                if omamori.counter > 0 {
                    omamori.counter -= 1;
                    if omamori.counter == 0 {
                        omamori.used_up = true;
                    }
                    return false; // Curse negated
                }
            }
        }

        // DarkstonePeriapt: +6 Max HP when obtaining a curse
        if def.card_type == crate::content::cards::CardType::Curse {
            if self.relics.iter().any(|r| r.id == crate::content::relics::RelicId::DarkstonePeriapt) {
                self.max_hp += 6;
                self.current_hp += 6;
            }
        }

        // CeramicFish: +9 gold on any card obtain
        if self.relics.iter().any(|r| r.id == crate::content::relics::RelicId::CeramicFish) {
            self.gold += 9;
        }

        // Egg auto-upgrade hooks (Java: onObtainCard in MoltenEgg/ToxicEgg/FrozenEgg)
        let should_upgrade = match def.card_type {
            crate::content::cards::CardType::Attack => {
                self.relics.iter().any(|r| r.id == crate::content::relics::RelicId::MoltenEgg)
            }
            crate::content::cards::CardType::Skill => {
                self.relics.iter().any(|r| r.id == crate::content::relics::RelicId::FrozenEgg)
            }
            crate::content::cards::CardType::Power => {
                self.relics.iter().any(|r| r.id == crate::content::relics::RelicId::ToxicEgg)
            }
            _ => false,
        };

        let uuid = self.next_card_uuid();
        let mut card = crate::combat::CombatCard::new(card_id, uuid);
        if should_upgrade {
            card.upgrades += 1;
        }
        self.master_deck.push(card);
        true
    }

    /// Returns a simple auto-incrementing UUID for new cards.
    fn next_card_uuid(&self) -> u32 {
        self.master_deck.len() as u32 + 10000
    }

    /// Initialize relic pools. Called at dungeon start.
    /// Java: initializeRelicList() + Collections.shuffle(pool, new Random(relicRng.randomLong()))
    pub fn init_relic_pools(&mut self) {
        use crate::content::relics::{RelicTier, build_relic_pool};
        let player_class = self.player_class;

        self.common_relic_pool = build_relic_pool(RelicTier::Common, player_class);
        self.uncommon_relic_pool = build_relic_pool(RelicTier::Uncommon, player_class);
        self.rare_relic_pool = build_relic_pool(RelicTier::Rare, player_class);
        self.shop_relic_pool = build_relic_pool(RelicTier::Shop, player_class);
        self.boss_relic_pool = build_relic_pool(RelicTier::Boss, player_class);

        // Remove already-owned relics from all pools
        let owned: Vec<crate::content::relics::RelicId> = self.relics.iter().map(|r| r.id).collect();
        for &id in &owned {
            self.common_relic_pool.retain(|&r| r != id);
            self.uncommon_relic_pool.retain(|&r| r != id);
            self.rare_relic_pool.retain(|&r| r != id);
            self.shop_relic_pool.retain(|&r| r != id);
            self.boss_relic_pool.retain(|&r| r != id);
        }

        // Shuffle each pool with relicRng.randomLong() as seed (Java pattern)
        crate::rng::shuffle_with_random_long(&mut self.common_relic_pool, &mut self.rng_pool.relic_rng);
        crate::rng::shuffle_with_random_long(&mut self.uncommon_relic_pool, &mut self.rng_pool.relic_rng);
        crate::rng::shuffle_with_random_long(&mut self.rare_relic_pool, &mut self.rng_pool.relic_rng);
        crate::rng::shuffle_with_random_long(&mut self.shop_relic_pool, &mut self.rng_pool.relic_rng);
        crate::rng::shuffle_with_random_long(&mut self.boss_relic_pool, &mut self.rng_pool.relic_rng);
    }

    /// Initialize encounter scheduling lists for the current act.
    /// Java: generateMonsters() in Exordium/TheCity/TheBeyond
    pub fn init_encounter_lists(&mut self) {
        let (ml, el) = crate::content::monsters::encounter_pool::generate_encounter_lists(
            self.act_num,
            &mut self.rng_pool.monster_rng,
        );
        self.monster_list = ml;
        self.elite_monster_list = el;
    }

    /// Pop the next normal encounter from the pre-scheduled list.
    /// Java: monsterList.remove(0) — consumes from front.
    pub fn next_encounter(&mut self) -> Option<crate::content::monsters::factory::EncounterId> {
        if self.monster_list.is_empty() {
            None
        } else {
            Some(self.monster_list.remove(0))
        }
    }

    /// Pop the next elite encounter from the pre-scheduled list.
    pub fn next_elite(&mut self) -> Option<crate::content::monsters::factory::EncounterId> {
        if self.elite_monster_list.is_empty() {
            None
        } else {
            Some(self.elite_monster_list.remove(0))
        }
    }

    /// Initialize the boss list for the current act.
    /// Java: initializeBoss() in Exordium/TheCity/TheBeyond — shuffle 3 bosses with monsterRng.
    pub fn init_boss_list(&mut self) {
        self.boss_list = crate::content::monsters::encounter_pool::generate_boss_list(
            self.act_num,
            &mut self.rng_pool.monster_rng,
        );
        // Asc 20: Keep first 2 bosses for double boss fights (Java: ProceedButton L95)
        // Below Asc 20: Keep only first boss (standard behavior)
        if self.ascension_level >= 20 {
            self.boss_list.truncate(2);
        } else {
            self.boss_list.truncate(1);
        }
    }

    /// Pop the next boss from the pre-scheduled list.
    /// Java: bossKey = bossList.get(0), then bossList.remove(0) in MonsterRoomBoss.
    pub fn next_boss(&mut self) -> Option<crate::content::monsters::factory::EncounterId> {
        if self.boss_list.is_empty() {
            None
        } else {
            Some(self.boss_list.remove(0))
        }
    }

    /// Advance to the next act after boss defeat.
    /// Regenerates map, encounter lists, event pools, and boss list for the new act.
    /// Java flow: VictoryRoom → Proceed → next Dungeon constructor.
    pub fn advance_act(&mut self) {
        self.act_num += 1;
        self.pending_boss_reward = false;

        // Generate new map for the next act
        // Generate map for the new act; returns consumed mapRng for emerald placement.
        let (mut new_map, mut map_rng) = crate::map::generator::generate_map_for_act(
            self.seed,
            self.act_num,
            self.ascension_level == 0,
        );

        // Mark emerald elite on new map if Act 4 is enabled and key not yet obtained.
        // Java: setEmeraldElite() reuses the consumed mapRng, not a fresh one.
        if self.is_final_act_available && !self.keys[2] {
            crate::map::generator::set_emerald_elite(&mut new_map, &mut map_rng);
        }

        self.map = crate::map::state::MapState::new(new_map);

        // Regenerate encounter lists for the new act
        self.init_encounter_lists();
        self.init_boss_list();

        // Reinitialize event pools for the new act
        self.initialize_event_pools();

        // Reset event room-type probabilities (Java: AbstractDungeon.resetProbabilities)
        self.event_monster_chance = 0.10;
        self.event_shop_chance = 0.03;
        self.event_treasure_chance = 0.02;

        // Update card_upgraded_chance per act (Java: initializeLevelSpecificChances)
        // Asc 12: halves the upgrade chance per act
        self.card_upgraded_chance = match self.act_num {
            1 => 0.0,  // Exordium: always 0.0
            2 => if self.ascension_level >= 12 { 0.125 } else { 0.25 },  // TheCity
            3 => if self.ascension_level >= 12 { 0.25 } else { 0.5 },    // TheBeyond
            _ => if self.ascension_level >= 12 { 0.25 } else { 0.5 },    // TheEnding
        };

        // Between-act healing (Java: AbstractDungeon.java L2562-2566)
        // Asc 5+: heal 75% of missing HP. Below Asc 5: heal to full.
        if self.ascension_level >= 5 {
            let missing = self.max_hp - self.current_hp;
            let heal = (missing as f32 * 0.75).round() as i32;
            self.current_hp = (self.current_hp + heal).min(self.max_hp);
        } else {
            self.current_hp = self.max_hp;
        }
    }

    /// Roll a random relic tier using relicRng.
    /// Java: returnRandomRelicTier() — roll 0..99, thresholds: Common 50, Uncommon 33, Rare 17.
    pub fn return_random_relic_tier(&mut self) -> crate::content::relics::RelicTier {
        use crate::content::relics::RelicTier;
        let roll = self.rng_pool.relic_rng.random_range(0, 99);
        if roll < 50 {
            RelicTier::Common
        } else if roll < 83 {
            RelicTier::Uncommon
        } else {
            RelicTier::Rare
        }
    }

    /// Pop a relic from the specified tier pool. Falls back on exhaustion:
    /// Common→Uncommon, Uncommon→Rare, Rare/Shop/Boss→Circlet.
    /// Java: returnEndRandomRelicKey(tier)
    pub fn random_relic_by_tier(&mut self, tier: crate::content::relics::RelicTier) -> crate::content::relics::RelicId {
        use crate::content::relics::{RelicId, RelicTier};
        match tier {
            RelicTier::Common => {
                if let Some(id) = self.common_relic_pool.pop() {
                    id
                } else {
                    self.random_relic_by_tier(RelicTier::Uncommon)
                }
            }
            RelicTier::Uncommon => {
                if let Some(id) = self.uncommon_relic_pool.pop() {
                    id
                } else {
                    self.random_relic_by_tier(RelicTier::Rare)
                }
            }
            RelicTier::Rare => {
                self.rare_relic_pool.pop().unwrap_or(RelicId::Circlet)
            }
            RelicTier::Shop => {
                if let Some(id) = self.shop_relic_pool.pop() {
                    id
                } else {
                    self.random_relic_by_tier(RelicTier::Uncommon)
                }
            }
            RelicTier::Boss => {
                self.boss_relic_pool.pop().unwrap_or(RelicId::Circlet)
            }
            _ => RelicId::Circlet,
        }
    }

    /// Returns a random "screenless" relic of the given tier.
    /// Skips relics that require UI interaction (BottledFlame/Lightning/Tornado/Whetstone).
    /// Java: returnRandomScreenlessRelic(tier)
    pub fn random_screenless_relic(&mut self, tier: crate::content::relics::RelicTier) -> crate::content::relics::RelicId {
        use crate::content::relics::RelicId;
        loop {
            let id = self.random_relic_by_tier(tier);
            match id {
                RelicId::BottledFlame | RelicId::BottledLightning
                | RelicId::BottledTornado | RelicId::Whetstone => {
                    // Skip — these need grid select. Pop next from same tier.
                    continue;
                }
                _ => return id,
            }
        }
    }

    /// Default random relic: roll tier then grab screenless relic.
    /// Java: returnRandomScreenlessRelic(returnRandomRelicTier())
    pub fn random_relic(&mut self) -> crate::content::relics::RelicId {
        let tier = self.return_random_relic_tier();
        self.random_screenless_relic(tier)
    }

    /// Returns a random potion, weighted by rarity and filtered to the current player class.
    /// Delegates to the canonical `random_potion()` with Java-accurate rarity weights.
    pub fn random_potion(&mut self) -> crate::content::potions::PotionId {
        let potion_class = self.potion_class_from_player();
        crate::content::potions::random_potion(
            &mut self.rng_pool.potion_rng,
            potion_class,
            false,
        )
    }

    /// Maps player_class string to PotionClass enum.
    fn potion_class_from_player(&self) -> crate::content::potions::PotionClass {
        match self.player_class {
            "Silent" => crate::content::potions::PotionClass::Silent,
            "Defect" => crate::content::potions::PotionClass::Defect,
            "Watcher" => crate::content::potions::PotionClass::Watcher,
            _ => crate::content::potions::PotionClass::Ironclad, // default
        }
    }

    /// Returns a random colorless card of the given rarity.
    /// Mirrors Java returnColorlessCard(rarity): shuffle pool, pick first matching rarity.
    pub fn random_colorless_card(&mut self, rarity: crate::content::cards::CardRarity) -> crate::content::cards::CardId {
        use crate::content::cards::*;
        let pool = match rarity {
            CardRarity::Rare => COLORLESS_RARE_POOL,
            _ => COLORLESS_UNCOMMON_POOL,
        };
        let idx = self.rng_pool.misc_rng.random_range(0, pool.len() as i32 - 1) as usize;
        pool[idx]
    }

    /// Returns a random card from the Ironclad pool of the given rarity.
    /// Mirrors Java `getCard(rarity)` — picks from the rarity-specific pool.
    pub fn random_card_by_rarity(&mut self, rarity: crate::content::cards::CardRarity) -> crate::content::cards::CardId {
        use crate::content::cards::*;
        let pool = ironclad_pool_for_rarity(rarity);
        let idx = self.rng_pool.misc_rng.random_range(0, pool.len() as i32 - 1) as usize;
        pool[idx]
    }

    /// Returns a random Ironclad card of the given CardType (Attack/Skill/Power).
    /// Mirrors Java `returnTrulyRandomCardInCombat(type)` — used by Attack/Skill/Power Potions.
    pub fn random_card_by_type(&mut self, card_type: crate::content::cards::CardType) -> crate::content::cards::CardId {
        use crate::content::cards::*;
        let pool = ironclad_pool_for_type(card_type);
        if pool.is_empty() {
            return CardId::Strike; // fallback
        }
        let idx = self.rng_pool.misc_rng.random_range(0, pool.len() as i32 - 1) as usize;
        pool[idx]
    }

    /// Shuffles upgradable cards in the master deck and upgrades up to `count`.
    /// Mirrors Java ShiningLight's upgrade logic using miscRng for shuffling.
    pub fn upgrade_random_cards(&mut self, count: usize) {
        // Collect indices of upgradable cards
        let mut upgradable_indices: Vec<usize> = self.master_deck.iter()
            .enumerate()
            .filter(|(_, c)| {
                let def = crate::content::cards::get_card_definition(c.id);
                // A card can be upgraded if it hasn't been upgraded yet (for most cards)
                // Searing Blow can be upgraded infinitely, so always qualifies
                c.id == crate::content::cards::CardId::SearingBlow || c.upgrades == 0 && def.card_type != crate::content::cards::CardType::Status && def.card_type != crate::content::cards::CardType::Curse
            })
            .map(|(i, _)| i)
            .collect();

        // Shuffle using miscRng.randomLong() seed (mirrors Java's Collections.shuffle)
        crate::rng::shuffle_with_random_long(&mut upgradable_indices, &mut self.rng_pool.misc_rng);

        // Upgrade up to `count` cards
        for &idx in upgradable_indices.iter().take(count) {
            self.master_deck[idx].upgrades += 1;
        }
    }

    /// Transforms a card: removes it from deck and replaces with a random card of the same color.
    /// Mirrors Java `transformCard(c, false, miscRng)`:
    ///   - Colored cards: pick from common+uncommon+rare pool excluding original card
    ///   - Curse: pick from curse pool excluding original
    ///   - Status: should not normally be transformed
    pub fn transform_card(&mut self, deck_index: usize) {
        if deck_index >= self.master_deck.len() {
            return;
        }
        let old_card = self.master_deck.remove(deck_index);
        let def = crate::content::cards::get_card_definition(old_card.id);

        use crate::content::cards::*;

        let new_id = match def.card_type {
            CardType::Curse => {
                // Java: CardLibrary.getCurse(c, rng) — pick random curse excluding original
                let curse_pool: &[CardId] = &[
                    CardId::Clumsy, CardId::Decay, CardId::Doubt, CardId::Injury,
                    CardId::Normality, CardId::Pain, CardId::Parasite, CardId::Regret,
                    CardId::Shame, CardId::Writhe,
                ];
                let filtered: Vec<CardId> = curse_pool.iter()
                    .copied()
                    .filter(|&c| c != old_card.id)
                    .collect();
                if filtered.is_empty() {
                    curse_pool[0]
                } else {
                    let idx = self.rng_pool.misc_rng.random_range(0, filtered.len() as i32 - 1) as usize;
                    filtered[idx]
                }
            },
            CardType::Status => {
                // Status cards shouldn't normally be transformed; fallback to Strike
                CardId::Strike
            },
            _ => {
                // Normal colored cards (Attack/Skill/Power):
                // Java: returnTrulyRandomCardFromAvailable — picks from entire card pool
                // (common + uncommon + rare) excluding original card, using passed rng.
                // Since simulator is Ironclad-only, use Ironclad pools.
                let pool: Vec<CardId> = IRONCLAD_COMMON_POOL.iter()
                    .chain(IRONCLAD_UNCOMMON_POOL.iter())
                    .chain(IRONCLAD_RARE_POOL.iter())
                    .copied()
                    .filter(|&c| c != old_card.id)
                    .collect();
                if pool.is_empty() {
                    old_card.id
                } else {
                    let idx = self.rng_pool.misc_rng.random_range(0, pool.len() as i32 - 1) as usize;
                    pool[idx]
                }
            },
        };

        let uuid = self.next_card_uuid();
        self.master_deck.push(crate::combat::CombatCard::new(new_id, uuid));
    }

    /// Generates a set of card rewards based on current rarity chances.
    /// Java: AbstractDungeon.getRewardCards()
    ///
    /// Rarity logic: `rollRarity()` → `roll = cardRng.random(99) + cardBlizzRandomizer`,
    /// then `getCurrRoom().getCardRarity(roll)` with baseRareCardChance=3, baseUncommonCardChance=37.
    /// After rarity: RARE → reset blizz to 5, COMMON → blizz -= 1 (min -40).
    pub fn generate_card_reward(&mut self, num_cards: usize) -> Vec<crate::content::cards::CardId> {
        use crate::content::cards::CardRarity;

        // Java constants
        const BASE_RARE_CHANCE: i32 = 3;
        const BASE_UNCOMMON_CHANCE: i32 = 37;
        const BLIZZ_START_OFFSET: i32 = 5;
        const BLIZZ_GROWTH: i32 = 1;
        const BLIZZ_MAX_OFFSET: i32 = -40;

        let mut cards = vec![];
        for _ in 0..num_cards {
            // Java: rollRarity() → cardRng.random(99) + cardBlizzRandomizer
            let base_roll = self.rng_pool.card_rng.random_range(0, 99);
            let roll = base_roll + self.card_blizz_randomizer;

            // Java: AbstractRoom.getCardRarity(roll)
            // rareCardChance starts at baseRareCardChance (3), modified by relics
            let mut rare_chance = BASE_RARE_CHANCE;
            let uncommon_chance = BASE_UNCOMMON_CHANCE;

            // NlothsGift triples rare chance (Java: changeRareCardRewardChance)
            if self.relics.iter().any(|r| r.id == crate::content::relics::RelicId::NlothsGift) {
                rare_chance *= 3;
            }

            let rarity = if roll < rare_chance {
                CardRarity::Rare
            } else if roll < rare_chance + uncommon_chance {
                CardRarity::Uncommon
            } else {
                CardRarity::Common
            };

            // Post-rarity blizz adjustment (Java: getRewardCards L1416-1426)
            match rarity {
                CardRarity::Rare => {
                    self.card_blizz_randomizer = BLIZZ_START_OFFSET;
                }
                CardRarity::Common => {
                    self.card_blizz_randomizer -= BLIZZ_GROWTH;
                    if self.card_blizz_randomizer < BLIZZ_MAX_OFFSET {
                        self.card_blizz_randomizer = BLIZZ_MAX_OFFSET;
                    }
                }
                _ => {} // Uncommon does not alter blizz
            }

            // Select from actual card pool based on rarity, avoiding duplicates
            let pool = crate::engine::campfire_handler::card_pool_for_class(self.player_class, rarity);
            if !pool.is_empty() {
                let mut contains_dupe = true;
                let mut candidate = pool[0];
                while contains_dupe {
                    contains_dupe = false;
                    let idx = self.rng_pool.card_rng.random_range(0, (pool.len() - 1) as i32) as usize;
                    candidate = pool[idx];
                    for c in &cards {
                        if *c == candidate {
                            contains_dupe = true;
                            break;
                        }
                    }
                }
                cards.push(candidate);
            }
        }

        // Post-selection upgrade pass (Java: getRewardCards L1449-1457)
        // Non-rare cards: cardRng.randomBoolean(cardUpgradedChance) → upgrade
        // Note: we only return CardIds, not mutable cards, so we track which indices to upgrade
        // and the caller handles it. But Java's upgrade happens after makeCopy, before returning.
        // Since our cards vec is just IDs, we consume the RNG calls for parity but can't upgrade here.
        // The upgrade will need to be applied by the caller when creating CombatCards.
        if self.card_upgraded_chance > 0.0 {
            for card_id in &cards {
                let def = crate::content::cards::get_card_definition(*card_id);
                if def.rarity != CardRarity::Rare {
                    // Consume the RNG call for parity, even if we can't upgrade CardId directly
                    let _should_upgrade = self.rng_pool.card_rng.random_boolean_chance(self.card_upgraded_chance);
                    // TODO: propagate upgrade flag to caller
                }
            }
        }

        cards
    }
}
