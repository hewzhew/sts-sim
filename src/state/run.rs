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
    pub reward_state: Option<crate::rewards::state::RewardState>,
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
    pub fn new(
        seed: u64,
        ascension_level: u8,
        final_act: bool,
        player_class: &'static str,
    ) -> Self {
        let base_max_hp = match player_class {
            "Silent" => 70,
            "Defect" => 75,
            "Watcher" => 72,
            _ => 80,
        };
        // Generate Act 1 map; returns the consumed mapRng for emerald key placement.
        let (mut first_map, mut map_rng) =
            crate::map::generator::generate_map_for_act(seed, 1, ascension_level == 0);
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
            current_hp: base_max_hp,
            max_hp: base_max_hp,
            gold: 99,
            shop_purge_count: 0,
            relics: Vec::new(),
            potions: if ascension_level >= 11 {
                vec![None, None]
            } else {
                vec![None, None, None]
            },
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
        rs.apply_starting_loadout();

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
                crate::content::cards::CardId::AscendersBane,
                uuid,
            ));
        }

        // Initialize Neow event for run start
        crate::content::events::neow::setup_neow_choices(&mut rs);
        rs
    }

    fn apply_starting_loadout(&mut self) {
        use crate::content::cards::CardId;
        use crate::content::relics::{RelicId, RelicState};

        if self.master_deck.is_empty() {
            let starter_cards: &'static [CardId] = match self.player_class {
                "Silent" => &[
                    CardId::StrikeG,
                    CardId::StrikeG,
                    CardId::StrikeG,
                    CardId::StrikeG,
                    CardId::StrikeG,
                    CardId::DefendG,
                    CardId::DefendG,
                    CardId::DefendG,
                    CardId::DefendG,
                    CardId::DefendG,
                    CardId::Survivor,
                    CardId::Neutralize,
                ],
                _ => &[
                    CardId::Strike,
                    CardId::Strike,
                    CardId::Strike,
                    CardId::Strike,
                    CardId::Strike,
                    CardId::Defend,
                    CardId::Defend,
                    CardId::Defend,
                    CardId::Defend,
                    CardId::Bash,
                ],
            };

            for (idx, &card_id) in starter_cards.iter().enumerate() {
                self.master_deck
                    .push(crate::combat::CombatCard::new(card_id, idx as u32));
            }
        }

        if self.relics.is_empty() {
            let starter_relic = match self.player_class {
                "Silent" => RelicId::SnakeRing,
                "Defect" => RelicId::CrackedCore,
                "Watcher" => RelicId::PureWater,
                _ => RelicId::BurningBlood,
            };
            self.relics.push(RelicState::new(starter_relic));
        }
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
    pub fn obtain_relic(
        &mut self,
        relic_id: crate::content::relics::RelicId,
        return_state: crate::state::core::EngineState,
    ) -> Option<crate::state::core::EngineState> {
        self.relics
            .push(crate::content::relics::RelicState::new(relic_id));
        crate::engine::relic_manager::on_equip(self, relic_id, return_state)
    }

    /// Triggers when the player enters a Rest Room (Campfire).
    pub fn on_enter_rest_room(&mut self) {
        for relic in &mut self.relics {
            let sub = crate::content::relics::get_relic_subscriptions(relic.id);
            if sub.on_enter_rest_room {
                match relic.id {
                    crate::content::relics::RelicId::AncientTeaSet => {
                        crate::content::relics::ancient_tea_set::AncientTeaSet::on_enter_rest_room(
                            relic,
                        );
                    }
                    _ => {}
                }
            }
        }
    }

    /// Generates ShopState with randomized prices, accounting for merchant Relics
    pub fn generate_shop(&mut self) -> crate::shop::ShopState {
        let config = crate::shop::state::ShopConfig {
            ascension_level: self.ascension_level as i32,
            player_class: self.player_class,
            has_courier: self
                .relics
                .iter()
                .any(|r| r.id == crate::content::relics::RelicId::Courier),
            has_membership_card: self
                .relics
                .iter()
                .any(|r| r.id == crate::content::relics::RelicId::MembershipCard),
            has_smiling_mask: self
                .relics
                .iter()
                .any(|r| r.id == crate::content::relics::RelicId::SmilingMask),
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
                use crate::content::relics::{RelicId, RelicTier};
                loop {
                    match tier {
                        RelicTier::Common => {
                            if let Some(id) = common_relic_pool.pop() {
                                return id;
                            }
                            tier = RelicTier::Uncommon;
                        }
                        RelicTier::Uncommon => {
                            if let Some(id) = uncommon_relic_pool.pop() {
                                return id;
                            }
                            tier = RelicTier::Rare;
                        }
                        RelicTier::Rare => {
                            return rare_relic_pool.pop().unwrap_or(RelicId::Circlet)
                        }
                        RelicTier::Shop => {
                            if let Some(id) = shop_relic_pool.pop() {
                                return id;
                            }
                            tier = RelicTier::Uncommon;
                        }
                        RelicTier::Boss => {
                            return boss_relic_pool.pop().unwrap_or(RelicId::Circlet)
                        }
                        _ => return RelicId::Circlet,
                    }
                }
            },
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
            crate::content::cards::get_card_definition(c.id).card_type
                == crate::content::cards::CardType::Curse
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
        let _room_type = self
            .event_generator
            .roll_room_type(&mut self.rng_pool, &ctx);

        // 2. Roll specific event ID
        self.event_generator
            .generate_event(&mut self.rng_pool, &ctx)
    }

    /// Adds a card to the master deck using DeckManager pipeline.
    /// Handles Omamori negation, CeramicFish gold, Elite Eggs upgrades, etc.
    /// Returns true if the card was actually added (false if Omamori blocked it).
    pub fn add_card_to_deck(&mut self, card_id: crate::content::cards::CardId) -> bool {
        self.add_card_to_deck_with_upgrades(card_id, 0)
    }

    /// Adds a card with an explicit pre-upgrade count.
    pub fn add_card_to_deck_with_upgrades(
        &mut self,
        card_id: crate::content::cards::CardId,
        pre_upgrades: u8,
    ) -> bool {
        let ctx = self.build_deck_context();
        let mut target_uuid = self.next_card_uuid();

        let result = crate::deck::manager::DeckManager::obtain_card(
            &ctx,
            card_id,
            &mut target_uuid,
            pre_upgrades,
        );
        let mut was_added = false;

        if !result.final_cards.is_empty() {
            was_added = true;
            for card in result.final_cards {
                let def = crate::content::cards::get_card_definition(card.id);
                eprintln!(
                    "  [OBTAIN] Added card to deck: {}{}",
                    def.name,
                    if card.upgrades > 0 { "+" } else { "" }
                );
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
            let removed = self.master_deck.remove(pos);
            let def = crate::content::cards::get_card_definition(removed.id);
            eprintln!(
                "  [OBTAIN] Removed card from deck: {}{}",
                def.name,
                if removed.upgrades > 0 { "+" } else { "" }
            );
            removed_id = Some(removed.id);
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
            has_darkstone_periapt: self
                .relics
                .iter()
                .any(|r| r.id == RelicId::DarkstonePeriapt),
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
        use crate::content::relics::{build_relic_pool, RelicTier};
        let player_class = self.player_class;

        self.common_relic_pool = build_relic_pool(RelicTier::Common, player_class);
        self.uncommon_relic_pool = build_relic_pool(RelicTier::Uncommon, player_class);
        self.rare_relic_pool = build_relic_pool(RelicTier::Rare, player_class);
        self.shop_relic_pool = build_relic_pool(RelicTier::Shop, player_class);
        self.boss_relic_pool = build_relic_pool(RelicTier::Boss, player_class);

        // Remove already-owned relics from all pools
        let owned: Vec<crate::content::relics::RelicId> =
            self.relics.iter().map(|r| r.id).collect();
        for &id in &owned {
            self.common_relic_pool.retain(|&r| r != id);
            self.uncommon_relic_pool.retain(|&r| r != id);
            self.rare_relic_pool.retain(|&r| r != id);
            self.shop_relic_pool.retain(|&r| r != id);
            self.boss_relic_pool.retain(|&r| r != id);
        }

        // Shuffle each pool with relicRng.randomLong() as seed (Java pattern)
        crate::rng::shuffle_with_random_long(
            &mut self.common_relic_pool,
            &mut self.rng_pool.relic_rng,
        );
        crate::rng::shuffle_with_random_long(
            &mut self.uncommon_relic_pool,
            &mut self.rng_pool.relic_rng,
        );
        crate::rng::shuffle_with_random_long(
            &mut self.rare_relic_pool,
            &mut self.rng_pool.relic_rng,
        );
        crate::rng::shuffle_with_random_long(
            &mut self.shop_relic_pool,
            &mut self.rng_pool.relic_rng,
        );
        crate::rng::shuffle_with_random_long(
            &mut self.boss_relic_pool,
            &mut self.rng_pool.relic_rng,
        );
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
            1 => 0.0, // Exordium: always 0.0
            2 => {
                if self.ascension_level >= 12 {
                    0.125
                } else {
                    0.25
                }
            } // TheCity
            3 => {
                if self.ascension_level >= 12 {
                    0.25
                } else {
                    0.5
                }
            } // TheBeyond
            _ => {
                if self.ascension_level >= 12 {
                    0.25
                } else {
                    0.5
                }
            } // TheEnding
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
    pub fn random_relic_by_tier(
        &mut self,
        tier: crate::content::relics::RelicTier,
    ) -> crate::content::relics::RelicId {
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
            RelicTier::Rare => self.rare_relic_pool.pop().unwrap_or(RelicId::Circlet),
            RelicTier::Shop => {
                if let Some(id) = self.shop_relic_pool.pop() {
                    id
                } else {
                    self.random_relic_by_tier(RelicTier::Uncommon)
                }
            }
            RelicTier::Boss => self.boss_relic_pool.pop().unwrap_or(RelicId::Circlet),
            _ => RelicId::Circlet,
        }
    }

    /// Returns a random "screenless" relic of the given tier.
    /// Skips relics that require UI interaction (BottledFlame/Lightning/Tornado/Whetstone).
    /// Java: returnRandomScreenlessRelic(tier)
    pub fn random_screenless_relic(
        &mut self,
        tier: crate::content::relics::RelicTier,
    ) -> crate::content::relics::RelicId {
        use crate::content::relics::RelicId;
        loop {
            let id = self.random_relic_by_tier(tier);
            match id {
                RelicId::BottledFlame
                | RelicId::BottledLightning
                | RelicId::BottledTornado
                | RelicId::Whetstone => {
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
        crate::content::potions::random_potion(&mut self.rng_pool.potion_rng, potion_class, false)
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

    /// Attempt to place a potion into the first empty slot, matching Java's
    /// `AbstractPlayer.obtainPotion()`. Returns true if placed, false if full.
    /// This is the ONLY correct way to add potions — never use `potions.push()`.
    pub fn obtain_potion(&mut self, potion: crate::content::potions::Potion) -> bool {
        if let Some(slot) = self.potions.iter().position(|p| p.is_none()) {
            self.potions[slot] = Some(potion);
            true
        } else {
            false
        }
    }

    /// Find first empty potion slot index, or None if full.
    pub fn find_empty_potion_slot(&self) -> Option<usize> {
        self.potions.iter().position(|p| p.is_none())
    }

    /// Returns a random colorless card of the given rarity.
    /// Mirrors Java returnColorlessCard(rarity): shuffle pool, pick first matching rarity.
    pub fn random_colorless_card(
        &mut self,
        rarity: crate::content::cards::CardRarity,
    ) -> crate::content::cards::CardId {
        use crate::content::cards::*;
        let pool = match rarity {
            CardRarity::Rare => COLORLESS_RARE_POOL,
            _ => COLORLESS_UNCOMMON_POOL,
        };
        let idx = self
            .rng_pool
            .misc_rng
            .random_range(0, pool.len() as i32 - 1) as usize;
        pool[idx]
    }

    /// Returns a random card from the Ironclad pool of the given rarity.
    /// Mirrors Java `getCard(rarity)` — picks from the rarity-specific pool.
    pub fn random_card_by_rarity(
        &mut self,
        rarity: crate::content::cards::CardRarity,
    ) -> crate::content::cards::CardId {
        use crate::content::cards::CardId;
        let pool = crate::engine::campfire_handler::nonempty_card_pool_for_class(
            self.player_class,
            rarity,
        );
        if pool.is_empty() {
            return match self.player_class {
                "Silent" => CardId::StrikeG,
                _ => CardId::Strike,
            };
        }
        let idx = self
            .rng_pool
            .misc_rng
            .random_range(0, pool.len() as i32 - 1) as usize;
        pool[idx]
    }

    /// Returns a random Ironclad card of the given CardType (Attack/Skill/Power).
    /// Mirrors Java `returnTrulyRandomCardInCombat(type)` — used by Attack/Skill/Power Potions.
    pub fn random_card_by_type(
        &mut self,
        card_type: crate::content::cards::CardType,
    ) -> crate::content::cards::CardId {
        use crate::content::cards::*;
        let pool = match self.player_class {
            "Silent" => silent_pool_for_type(card_type),
            _ => ironclad_pool_for_type(card_type),
        };
        if pool.is_empty() {
            return match self.player_class {
                "Silent" => CardId::StrikeG,
                _ => CardId::Strike,
            };
        }
        let idx = self
            .rng_pool
            .misc_rng
            .random_range(0, pool.len() as i32 - 1) as usize;
        pool[idx]
    }

    /// Shuffles upgradable cards in the master deck and upgrades up to `count`.
    /// Mirrors Java ShiningLight's upgrade logic using miscRng for shuffling.
    pub fn upgrade_random_cards(&mut self, count: usize) {
        // Collect indices of upgradable cards
        let mut upgradable_indices: Vec<usize> = self
            .master_deck
            .iter()
            .enumerate()
            .filter(|(_, c)| {
                let def = crate::content::cards::get_card_definition(c.id);
                // A card can be upgraded if it hasn't been upgraded yet (for most cards)
                // Searing Blow can be upgraded infinitely, so always qualifies
                c.id == crate::content::cards::CardId::SearingBlow
                    || c.upgrades == 0
                        && def.card_type != crate::content::cards::CardType::Status
                        && def.card_type != crate::content::cards::CardType::Curse
            })
            .map(|(i, _)| i)
            .collect();

        // Shuffle using miscRng.randomLong() seed (mirrors Java's Collections.shuffle)
        crate::rng::shuffle_with_random_long(&mut upgradable_indices, &mut self.rng_pool.misc_rng);

        // Upgrade up to `count` cards
        for &idx in upgradable_indices.iter().take(count) {
            let uuid = self.master_deck[idx].uuid;
            self.upgrade_card(uuid);
        }
    }

    /// Upgrades a specific card in the master deck by its UUID.
    pub fn upgrade_card(&mut self, uuid: u32) {
        if let Some(card) = self.master_deck.iter_mut().find(|c| c.uuid == uuid) {
            card.upgrades += 1;
        }
    }

    /// Transforms a card: removes it from deck and replaces with a random card of the same color.
    /// Uses DeckManager properly so Omamori/Necronomicurse triggers fire correctly.
    /// `auto_upgrade` is true when transforming via Astrolabe.
    pub fn transform_card(&mut self, deck_index: usize, auto_upgrade: bool) {
        if deck_index >= self.master_deck.len() {
            return;
        }

        let old_card_id = self.master_deck[deck_index].id;
        let old_card_uuid = self.master_deck[deck_index].uuid;
        let def = crate::content::cards::get_card_definition(old_card_id);

        use crate::content::cards::*;

        let new_id = if def.card_type == CardType::Curse {
            let curse_pool = get_curse_pool();
            let filtered: Vec<CardId> = curse_pool
                .iter()
                .copied()
                .filter(|&c| c != old_card_id) // Java logic: CardLibrary.getCurse(c, rng)
                .collect();
            if filtered.is_empty() {
                CardId::Clumsy
            } else {
                let idx = self
                    .rng_pool
                    .misc_rng
                    .random_range(0, filtered.len() as i32 - 1) as usize;
                filtered[idx]
            }
        } else if COLORLESS_UNCOMMON_POOL.contains(&old_card_id)
            || COLORLESS_RARE_POOL.contains(&old_card_id)
            || old_card_id == CardId::Madness
            || old_card_id == CardId::JAX
            || old_card_id == CardId::Apparition
            || old_card_id == CardId::Bite
            || old_card_id == CardId::RitualDagger
            || old_card_id == CardId::Shiv
            || old_card_id == CardId::Finesse
        {
            let pool = COLORLESS_UNCOMMON_POOL
                .iter()
                .chain(COLORLESS_RARE_POOL.iter())
                .copied()
                .filter(|&c| c != old_card_id)
                .collect::<Vec<_>>();
            if pool.is_empty() {
                old_card_id
            } else {
                let idx = self
                    .rng_pool
                    .misc_rng
                    .random_range(0, pool.len() as i32 - 1) as usize;
                pool[idx]
            }
        } else {
            // Java: returnTrulyRandomCardFromAvailable(c, rng)
            let pool: Vec<CardId> = crate::engine::campfire_handler::card_pool_for_class(
                self.player_class,
                CardRarity::Common,
            )
            .iter()
            .chain(
                crate::engine::campfire_handler::card_pool_for_class(
                    self.player_class,
                    CardRarity::Uncommon,
                )
                .iter(),
            )
            .chain(
                crate::engine::campfire_handler::card_pool_for_class(
                    self.player_class,
                    CardRarity::Rare,
                )
                .iter(),
            )
            .copied()
            .filter(|&c| c != old_card_id)
            .collect();
            if pool.is_empty() {
                old_card_id
            } else {
                let idx = self
                    .rng_pool
                    .misc_rng
                    .random_range(0, pool.len() as i32 - 1) as usize;
                pool[idx]
            }
        };

        // 1. Remove logically
        self.remove_card_from_deck(old_card_uuid);

        // 2. Add logically
        let ctx = self.build_deck_context();
        let mut target_uuid = self.next_card_uuid(); // This is just the base UUID, DeckManager will increment for actual insertions

        let result = crate::deck::manager::DeckManager::obtain_card(
            &ctx,
            new_id,
            &mut target_uuid,
            if auto_upgrade { 1 } else { 0 },
        );

        // 3. Obtain
        let mut obtained_any = false;
        for card in result.final_cards {
            let new_def = crate::content::cards::get_card_definition(card.id);
            eprintln!(
                "  [OBTAIN] Transformed {} -> {}{}",
                def.name,
                new_def.name,
                if card.upgrades > 0 { "+" } else { "" }
            );
            self.master_deck.push(card);
            obtained_any = true;
        }
        if !obtained_any {
            eprintln!("  [OBTAIN] Removed {} (Transform nullified?)", def.name);
        }

        // 4. Resolve Deck actions
        self.resolve_deck_actions(result.actions);
        self.dispatch_on_master_deck_change();
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::action::Action;
    use crate::combat::{CombatPhase, CombatState};
    use crate::content::cards::CardId;
    use crate::content::relics::{hooks, RelicId};
    use std::collections::{HashMap, VecDeque};

    #[test]
    fn silent_new_run_has_expected_starter_loadout() {
        let run = RunState::new(7, 0, false, "Silent");

        assert_eq!(run.max_hp, 70);
        assert_eq!(run.current_hp, 70);
        assert_eq!(run.relics.len(), 1);
        assert_eq!(run.relics[0].id, RelicId::SnakeRing);
        assert_eq!(run.master_deck.len(), 12);
        assert_eq!(
            run.master_deck
                .iter()
                .filter(|c| c.id == CardId::StrikeG)
                .count(),
            5
        );
        assert_eq!(
            run.master_deck
                .iter()
                .filter(|c| c.id == CardId::DefendG)
                .count(),
            5
        );
        assert!(run.master_deck.iter().any(|c| c.id == CardId::Neutralize));
        assert!(run.master_deck.iter().any(|c| c.id == CardId::Survivor));
    }

    #[test]
    fn silent_snake_ring_is_wired_into_battle_start_draw() {
        let run = RunState::new(11, 0, false, "Silent");
        let mut combat = CombatState {
            meta: crate::combat::CombatMeta {
                ascension_level: 0,
                is_boss_fight: false,
                is_elite_fight: false,
                meta_changes: Vec::new(),
            },
            turn: crate::combat::TurnRuntime {
                turn_count: 1,
                current_phase: CombatPhase::PlayerTurn,
                energy: 3,
                turn_start_draw_modifier: 0,
                counters: Default::default(),
            },
            zones: crate::combat::CardZones {
                draw_pile: Vec::new(),
                hand: Vec::new(),
                discard_pile: Vec::new(),
                exhaust_pile: Vec::new(),
                limbo: Vec::new(),
                queued_cards: VecDeque::new(),
                card_uuid_counter: 1,
            },
            entities: crate::combat::EntityState {
                player: run.build_combat_player(0),
                monsters: Vec::new(),
                potions: vec![None, None, None],
                power_db: HashMap::new(),
            },
            engine: crate::combat::EngineRuntime {
                action_queue: VecDeque::new(),
            },
            rng: crate::combat::CombatRng::new(crate::rng::RngPool::new(17)),
        };

        let actions = hooks::at_battle_start(&mut combat);
        assert!(actions
            .iter()
            .any(|info| matches!(info.action, Action::DrawCards(2))));
    }

    #[test]
    fn upgraded_after_image_is_treated_as_innate() {
        let mut card = crate::combat::CombatCard::new(CardId::AfterImage, 42);
        card.upgrades = 1;
        assert!(crate::content::cards::is_innate_card(&card));
    }
}
