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
    pub shop_state: Option<crate::shop::ShopState>,
    pub event_state: Option<crate::state::events::EventState>,

    pub event_generator: crate::events::generator::EventGenerator,

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

            // Subsystems
            event_generator: crate::events::generator::EventGenerator::new(1),

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
            energy_master: 3,
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

    /// Primary entry point for adding a new relic to the run.
    /// Handles appending to the relics array and immediately dispatches to the RelicManager 
    /// for onEquip hooks (e.g. increasing Max HP or interrupting the engine state with a UI).
    pub fn obtain_relic(&mut self, relic_id: crate::content::relics::RelicId, return_state: crate::state::core::EngineState) -> Option<crate::state::core::EngineState> {
        self.relics.push(crate::content::relics::RelicState::new(relic_id));
        crate::engine::relic_manager::on_equip(self, relic_id, return_state)
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
    pub fn generate_shop(&mut self) -> crate::shop::ShopState {
        let config = crate::shop::state::ShopConfig {
            ascension_level: self.ascension_level as i32,
            has_courier: self.relics.iter().any(|r| r.id == crate::content::relics::RelicId::Courier),
            has_membership_card: self.relics.iter().any(|r| r.id == crate::content::relics::RelicId::MembershipCard),
            has_smiling_mask: self.relics.iter().any(|r| r.id == crate::content::relics::RelicId::SmilingMask),
            previous_purge_count: self.shop_purge_count,
            potion_class: self.potion_class(),
            card_blizz_randomizer: self.card_blizz_randomizer,
        };

        let crate::state::run::RunState {
            ref mut rng_pool,
            ref mut common_relic_pool,
            ref mut uncommon_relic_pool,
            ref mut rare_relic_pool,
            ref mut shop_relic_pool,
            ref mut boss_relic_pool,
            ..
        } = self;

        crate::shop::shop_screen::generate_shop(
            rng_pool,
            &config,
            |mut tier| -> crate::content::relics::RelicId {
                use crate::content::relics::{RelicTier, RelicId};
                loop {
                    match tier {
                        RelicTier::Common => {
                            if let Some(id) = common_relic_pool.pop() { return id; }
                            tier = RelicTier::Uncommon;
                        }
                        RelicTier::Uncommon => {
                            if let Some(id) = uncommon_relic_pool.pop() { return id; }
                            tier = RelicTier::Rare;
                        }
                        RelicTier::Rare => return rare_relic_pool.pop().unwrap_or(RelicId::Circlet),
                        RelicTier::Shop => {
                            if let Some(id) = shop_relic_pool.pop() { return id; }
                            tier = RelicTier::Uncommon;
                        }
                        RelicTier::Boss => return boss_relic_pool.pop().unwrap_or(RelicId::Circlet),
                        _ => return RelicId::Circlet,
                    }
                }
            }
        )
    }

    /// Initialize event pools for the current act, matching Java Exordium/TheCity/TheBeyond.initializeEventList()
    /// and AbstractDungeon.initializeSpecialOneTimeEventList().
    pub fn generate_event(&mut self) -> crate::state::events::EventId {
        use crate::content::relics::RelicId;
        
        let mut tiny_chest_counter = 0;
        let mut has_juzu = false;
        let mut has_golden_idol = false;
        
        for relic in &mut self.relics {
            match relic.id {
                RelicId::TinyChest => {
                    relic.counter += 1;
                    if relic.counter == 4 {
                        relic.counter = 0;
                        tiny_chest_counter = 3; // Trigger force_chest in EventGenerator
                    } else {
                        tiny_chest_counter = relic.counter;
                    }
                }
                RelicId::JuzuBracelet => has_juzu = true,
                RelicId::GoldenIdol => has_golden_idol = true,
                _ => {}
            }
        }

        let has_curses = self.master_deck.iter().any(|c| {
            crate::content::cards::get_card_definition(c.id).card_type == crate::content::cards::CardType::Curse
        });

        let ctx = crate::events::context::EventContext {
            act_num: self.act_num,
            ascension_level: self.ascension_level,
            floor_num: self.floor_num,
            gold: self.gold,
            current_hp: self.current_hp,
            max_hp: self.max_hp,
            has_curses,
            has_golden_idol,
            tiny_chest_counter,
            has_juzu_bracelet: has_juzu,
            relic_count: self.relics.len(),
        };

        // 1. Roll room type (this consumes event_rng and updates chances, just like Java EventHelper.roll)
        // Even if we always return Event for now, we MUST roll the room type to align RNG!
        let _room_type = self.event_generator.roll_room_type(&mut self.rng_pool, &ctx);

        // 2. Roll specific event ID
        self.event_generator.generate_event(&mut self.rng_pool, &ctx)
    }

    /// Adds a card to the master deck using DeckManager pipeline.
    /// Handles Omamori negation, CeramicFish gold, Elite Eggs upgrades, etc.
    /// Returns true if the card was actually added (false if Omamori blocked it).
    pub fn add_card_to_deck(&mut self, card_id: crate::content::cards::CardId) -> bool {
        let ctx = self.build_deck_context();
        let mut target_uuid = self.next_card_uuid();
        
        let result = crate::deck::manager::DeckManager::obtain_card(&ctx, card_id, &mut target_uuid);
        let mut was_added = false;
        
        if !result.final_cards.is_empty() {
            was_added = true;
            for card in result.final_cards {
                let def = crate::content::cards::get_card_definition(card.id);
                println!("  [OBTAIN] Added card to deck: {}{}", def.name, if card.upgrades > 0 { "+" } else { "" });
                self.master_deck.push(card);
            }
            self.dispatch_on_master_deck_change();
        }
        
        self.resolve_deck_actions(result.actions);
        was_added
    }

    /// Removes a specific card instance from the master deck.
    /// Handles Parasite triggers, Necronomicurse regeneration.
    pub fn remove_card_from_deck(&mut self, uuid: u32) {
        let mut removed_id = None;
        if let Some(pos) = self.master_deck.iter().position(|c| c.uuid == uuid) {
            removed_id = Some(self.master_deck.remove(pos).id);
        }
        
        if let Some(card_id) = removed_id {
            let result = crate::deck::manager::DeckManager::remove_card(card_id);
            self.dispatch_on_master_deck_change();
            self.resolve_deck_actions(result.actions);
        }
    }

    fn build_deck_context(&self) -> crate::deck::context::DeckContext {
        use crate::content::relics::RelicId;
        let mut omamori_charges = 0;
        let mut has_omamori = false;
        
        for relic in &self.relics {
            if relic.id == RelicId::Omamori {
                has_omamori = true;
                omamori_charges = relic.counter;
            }
        }
        
        crate::deck::context::DeckContext {
            has_hoarder_mod: false,
            has_omamori,
            omamori_charges,
            has_ceramic_fish: self.relics.iter().any(|r| r.id == RelicId::CeramicFish),
            has_darkstone_periapt: self.relics.iter().any(|r| r.id == RelicId::DarkstonePeriapt),
            has_molten_egg: self.relics.iter().any(|r| r.id == RelicId::MoltenEgg),
            has_toxic_egg: self.relics.iter().any(|r| r.id == RelicId::ToxicEgg),
            has_frozen_egg: self.relics.iter().any(|r| r.id == RelicId::FrozenEgg),
            has_necronomicon: self.relics.iter().any(|r| r.id == RelicId::Necronomicon),
        }
    }

    fn resolve_deck_actions(&mut self, actions: Vec<crate::deck::manager::DeckAction>) {
        use crate::deck::manager::DeckAction;
        for action in actions {
            match action {
                DeckAction::PreventObtain => { /* Handled structurally */ }
                DeckAction::GainGold(amount) => self.gold += amount,
                DeckAction::GainMaxHp(amount) => {
                    self.max_hp += amount;
                    self.current_hp += amount;
                }
                DeckAction::LoseMaxHp(amount) => {
                    self.max_hp -= amount;
                    if self.current_hp > self.max_hp {
                        self.current_hp = self.max_hp;
                    }
                }
                DeckAction::UpdateRelicCounter(relic_id, counter) => {
                    if let Some(relic) = self.relics.iter_mut().find(|r| r.id == relic_id) {
                        relic.counter = counter;
                        if counter == 0 && relic_id == crate::content::relics::RelicId::Omamori {
                            relic.used_up = true;
                        }
                    }
                }
                DeckAction::TriggerObtainCard(card_id) => {
                    // Re-enters add_card_to_deck (e.g. for Necronomicurse)
                    self.add_card_to_deck(card_id);
                }
            }
        }
    }

    /// Triggers AbstractRelic.onMasterDeckChange for all relics
    pub fn dispatch_on_master_deck_change(&mut self) {
        // e.g., Du-Vu Doll might recalculate strength later.
        // Currently a no-op placeholder for future relic callbacks
    }

    /// Returns a simple auto-incrementing UUID for new cards.
    pub fn next_card_uuid(&self) -> u32 {
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

        // Boss defeat heal
        // Java: AbstractDungeon.dungeonTransitionSetup() -> player.heal(0.75 or maxHealth)
        let missing = self.max_hp - self.current_hp;
        let heal_amount = if self.ascension_level >= 5 {
            (missing as f32 * 0.75).round() as i32
        } else {
            self.max_hp
        };
        self.current_hp = (self.current_hp + heal_amount).min(self.max_hp);

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
        self.event_generator.initialize_event_pools(self.act_num);

        // Reset event room-type probabilities (Java: AbstractDungeon.resetProbabilities)
        self.event_generator.reset_probabilities();

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
