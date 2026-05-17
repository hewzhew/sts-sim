use crate::content::relics::RelicState;
use crate::map::state::MapState;
use crate::runtime::combat::{CombatCard, PlayerEntity};
use crate::runtime::rng::{RngPool, StsRng};
use crate::state::relic_pool::{
    random_relic_by_tier_from_pools, random_relic_end_by_tier_from_pools, RelicPoolsMut,
    RelicSpawnContext,
};
use crate::state::selection::{DomainCardSnapshot, DomainEvent, DomainEventSource};
use std::cell::Cell;

thread_local! {
    static SUPPRESS_OBTAIN_LOGS_DEPTH: Cell<usize> = const { Cell::new(0) };
}

pub fn with_suppressed_obtain_logs<T>(f: impl FnOnce() -> T) -> T {
    SUPPRESS_OBTAIN_LOGS_DEPTH.with(|depth| {
        depth.set(depth.get() + 1);
        let result = f();
        depth.set(depth.get().saturating_sub(1));
        result
    })
}

#[derive(Debug, Clone, PartialEq)]
pub struct RunState {
    pub seed: u64,
    pub ascension_level: u8,
    pub is_daily_run: bool,
    pub highest_unlocked_ascension_level: u8,
    pub act_num: u8,
    pub floor_num: i32,
    pub player_class: &'static str, // "Ironclad", "Silent", "Defect", "Watcher"

    pub map: MapState,
    pub rng_pool: RngPool,
    /// Java `NeowEvent.rng`, initialized from `Settings.seed` when Neow
    /// blessings are generated and then reused by Neow reward activation.
    pub neow_rng: StsRng,

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
    pub note_for_yourself_card: crate::content::cards::CardId,
    pub note_for_yourself_upgrades: u8,

    pub event_generator: crate::events::generator::EventGenerator,
    /// Java `AbstractRoom.mugged` equivalent for the current room.
    ///
    /// Set when a thief escapes after stealing gold. Cleared when entering the
    /// next room. Used for post-combat room/reward semantics and future parity.
    pub room_mugged: bool,
    /// Java `AbstractRoom.smoked` equivalent for the current room.
    ///
    /// Set when Smoke Bomb ends the current combat. Cleared when entering the
    /// next room. Used for reward-screen semantics and future parity.
    pub room_smoked: bool,

    // Relic pools — filled at dungeon init. Normal rewards consume from the
    // front; shop/end relic paths consume from the end.
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
    /// Java `AbstractDungeon.bossKey`: the boss currently shown on the map and
    /// used by `MonsterRoomBoss` when the boss room is entered. `boss_list` is
    /// the internal queue and may contain future bosses that are not public
    /// observation.
    pub boss_key: Option<crate::content::monsters::factory::EncounterId>,
    pub boss_list: Vec<crate::content::monsters::factory::EncounterId>,

    /// Set to true after boss combat ends, consumed by post_reward_state() to trigger act advance.
    pub pending_boss_reward: bool,
    /// Set when a boss relic's on-equip effect opens a run-level selection.
    /// Java obtains the boss relic before the dungeon transition; the transition
    /// happens only after that selection resolves and the boss chest is left.
    pub pending_boss_act_transition: bool,
    pub emitted_events: Vec<DomainEvent>,
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
            is_daily_run: false,
            highest_unlocked_ascension_level: ascension_level,
            act_num: 1,
            floor_num: 0,
            player_class,
            map: MapState::new(first_map),
            rng_pool: RngPool::new(seed),
            neow_rng: StsRng::new(seed),

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
            note_for_yourself_card: crate::content::cards::CardId::IronWave,
            note_for_yourself_upgrades: 0,

            // Subsystems
            event_generator: crate::events::generator::EventGenerator::new(1),
            room_mugged: false,
            room_smoked: false,

            // Relic pools (filled by init_relic_pools)
            common_relic_pool: Vec::new(),
            uncommon_relic_pool: Vec::new(),
            rare_relic_pool: Vec::new(),
            shop_relic_pool: Vec::new(),
            boss_relic_pool: Vec::new(),

            // Encounter lists (filled by generate_encounter_lists)
            monster_list: Vec::new(),
            elite_monster_list: Vec::new(),
            boss_key: None,
            boss_list: Vec::new(),

            pending_boss_reward: false,
            pending_boss_act_transition: false,
            emitted_events: Vec::new(),
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
            rs.master_deck.push(crate::runtime::combat::CombatCard::new(
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
                "Defect" => &[
                    CardId::StrikeB,
                    CardId::StrikeB,
                    CardId::StrikeB,
                    CardId::StrikeB,
                    CardId::DefendB,
                    CardId::DefendB,
                    CardId::DefendB,
                    CardId::DefendB,
                    CardId::Zap,
                    CardId::Dualcast,
                ],
                "Watcher" => &[
                    CardId::StrikeP,
                    CardId::StrikeP,
                    CardId::StrikeP,
                    CardId::StrikeP,
                    CardId::DefendP,
                    CardId::DefendP,
                    CardId::DefendP,
                    CardId::DefendP,
                    CardId::Eruption,
                    CardId::Vigilance,
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
                    .push(crate::runtime::combat::CombatCard::new(card_id, idx as u32));
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
        let master_max_orbs = if self.player_class == "Defect" {
            3
        } else if self
            .relics
            .iter()
            .any(|relic| relic.id == crate::content::relics::RelicId::PrismaticShard)
        {
            1
        } else {
            0
        };
        let mut p = PlayerEntity {
            id,
            current_hp: self.current_hp,
            max_hp: self.max_hp,
            block: 0,
            gold_delta_this_combat: 0,
            gold: self.gold,
            max_orbs: master_max_orbs,
            orbs: (0..master_max_orbs)
                .map(|_| {
                    crate::runtime::combat::OrbEntity::new(crate::runtime::combat::OrbId::Empty)
                })
                .collect(),
            stance: crate::runtime::combat::StanceId::Neutral,
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
        self.obtain_relic_with_source(relic_id, return_state, DomainEventSource::DeckMutation)
    }

    pub fn obtain_relic_with_source(
        &mut self,
        relic_id: crate::content::relics::RelicId,
        return_state: crate::state::core::EngineState,
        source: DomainEventSource,
    ) -> Option<crate::state::core::EngineState> {
        if relic_id == crate::content::relics::RelicId::Circlet {
            if let Some(circlet) = self
                .relics
                .iter_mut()
                .find(|relic| relic.id == crate::content::relics::RelicId::Circlet)
            {
                circlet.counter += 1;
                self.emit_event(DomainEvent::RelicObtained { relic_id, source });
                return None;
            }
        }

        let previous_gold = self.gold;
        let previous_hp = self.current_hp;
        let previous_max_hp = self.max_hp;
        self.relics
            .push(crate::content::relics::RelicState::new(relic_id));
        self.emit_event(DomainEvent::RelicObtained { relic_id, source });
        let next_state = crate::engine::relic_manager::on_equip(self, relic_id, return_state);
        self.emit_run_resource_diffs(previous_gold, previous_hp, previous_max_hp, source);
        next_state
    }

    pub fn obtain_boss_relic_choice_with_source(
        &mut self,
        relic_id: crate::content::relics::RelicId,
        return_state: crate::state::core::EngineState,
        source: DomainEventSource,
    ) -> Option<crate::state::core::EngineState> {
        if is_starter_upgrade_boss_relic(relic_id) && !self.relics.is_empty() {
            let previous_gold = self.gold;
            let previous_hp = self.current_hp;
            let previous_max_hp = self.max_hp;
            let replaced_relic_id = self.relics[0].id;

            // Java BossRelicSelectScreen calls instantObtain(player, 0, true)
            // for these boss relics. That overwrites relic slot 0 and calls
            // onEquip, but does not run the old relic's onUnequip hook.
            self.relics[0] = crate::content::relics::RelicState::new(relic_id);
            self.emit_event(DomainEvent::RelicLost {
                relic_id: replaced_relic_id,
                source,
            });
            self.emit_event(DomainEvent::RelicObtained { relic_id, source });
            let next_state = crate::engine::relic_manager::on_equip(self, relic_id, return_state);
            self.emit_run_resource_diffs(previous_gold, previous_hp, previous_max_hp, source);
            return next_state;
        }

        self.obtain_relic_with_source(relic_id, return_state, source)
    }

    pub fn obtain_relic_at_with_source(
        &mut self,
        relic_id: crate::content::relics::RelicId,
        index: usize,
        return_state: crate::state::core::EngineState,
        source: DomainEventSource,
    ) -> Option<crate::state::core::EngineState> {
        if relic_id == crate::content::relics::RelicId::Circlet {
            if let Some(circlet) = self
                .relics
                .iter_mut()
                .find(|relic| relic.id == crate::content::relics::RelicId::Circlet)
            {
                circlet.counter += 1;
                self.emit_event(DomainEvent::RelicObtained { relic_id, source });
                return None;
            }
        }

        let previous_gold = self.gold;
        let previous_hp = self.current_hp;
        let previous_max_hp = self.max_hp;
        let insert_index = index.min(self.relics.len());
        self.relics.insert(
            insert_index,
            crate::content::relics::RelicState::new(relic_id),
        );
        self.emit_event(DomainEvent::RelicObtained { relic_id, source });
        let next_state = crate::engine::relic_manager::on_equip(self, relic_id, return_state);
        self.emit_run_resource_diffs(previous_gold, previous_hp, previous_max_hp, source);
        next_state
    }

    pub fn remove_relic_at_with_source(
        &mut self,
        index: usize,
        source: DomainEventSource,
    ) -> Option<crate::content::relics::RelicId> {
        if index >= self.relics.len() {
            return None;
        }
        let relic = self.relics.remove(index);
        self.emit_event(DomainEvent::RelicLost {
            relic_id: relic.id,
            source,
        });
        crate::engine::relic_manager::on_unequip(self, relic.id, source);
        Some(relic.id)
    }

    pub fn remove_first_relic_with_id_and_source(
        &mut self,
        relic_id: crate::content::relics::RelicId,
        source: DomainEventSource,
    ) -> Option<crate::content::relics::RelicId> {
        self.relics
            .iter()
            .position(|relic| relic.id == relic_id)
            .and_then(|index| self.remove_relic_at_with_source(index, source))
    }

    pub fn change_gold_with_source(&mut self, delta: i32, source: DomainEventSource) -> i32 {
        if delta > 0
            && self
                .relics
                .iter()
                .any(|relic| relic.id == crate::content::relics::RelicId::Ectoplasm)
        {
            return 0;
        }

        let old_gold = self.gold;
        self.gold = (self.gold + delta).max(0);
        let actual_delta = self.gold - old_gold;
        if actual_delta < 0 && matches!(source, DomainEventSource::Shop) {
            if let Some(relic) = self
                .relics
                .iter_mut()
                .find(|relic| relic.id == crate::content::relics::RelicId::MawBank)
            {
                relic.used_up = true;
                relic.counter = -2;
            }
        }
        if actual_delta != 0 {
            self.emit_event(DomainEvent::GoldChanged {
                delta: actual_delta,
                new_total: self.gold,
                source,
            });
        }
        if actual_delta > 0
            && self
                .relics
                .iter()
                .any(|relic| relic.id == crate::content::relics::RelicId::BloodyIdol)
        {
            self.change_hp_with_source(
                5,
                DomainEventSource::Relic(crate::content::relics::RelicId::BloodyIdol),
            );
        }
        actual_delta
    }

    pub fn set_gold_with_source(&mut self, new_total: i32, source: DomainEventSource) -> i32 {
        self.change_gold_with_source(new_total - self.gold, source)
    }

    pub fn change_hp_with_source(&mut self, delta: i32, source: DomainEventSource) -> i32 {
        self.set_current_hp_with_source(self.current_hp + delta, source)
    }

    pub fn heal_with_source(&mut self, amount: i32, source: DomainEventSource) -> i32 {
        if amount <= 0 {
            return 0;
        }
        if self
            .relics
            .iter()
            .any(|r| r.id == crate::content::relics::RelicId::MarkOfTheBloom)
        {
            return 0;
        }
        self.change_hp_with_source(amount, source)
    }

    pub fn set_current_hp_with_source(
        &mut self,
        new_current_hp: i32,
        source: DomainEventSource,
    ) -> i32 {
        let old_hp = self.current_hp;
        self.current_hp = new_current_hp.clamp(0, self.max_hp.max(0));
        let actual_delta = self.current_hp - old_hp;
        if actual_delta != 0 {
            self.emit_event(DomainEvent::HpChanged {
                delta: actual_delta,
                current_hp: self.current_hp,
                max_hp: self.max_hp,
                source,
            });
        }
        actual_delta
    }

    pub fn gain_max_hp_with_source(
        &mut self,
        amount: i32,
        heal_amount: i32,
        source: DomainEventSource,
    ) -> i32 {
        if amount <= 0 {
            return 0;
        }
        self.max_hp += amount;
        self.heal_with_source(heal_amount, source);
        self.emit_event(DomainEvent::MaxHpChanged {
            delta: amount,
            current_hp: self.current_hp,
            max_hp: self.max_hp,
            source,
        });
        amount
    }

    pub fn lose_max_hp_with_source(&mut self, amount: i32, source: DomainEventSource) -> i32 {
        if amount <= 0 {
            return 0;
        }
        let old_max_hp = self.max_hp;
        self.max_hp = (self.max_hp - amount).max(1);
        self.current_hp = self.current_hp.min(self.max_hp);
        let actual_delta = self.max_hp - old_max_hp;
        if actual_delta != 0 {
            self.emit_event(DomainEvent::MaxHpChanged {
                delta: actual_delta,
                current_hp: self.current_hp,
                max_hp: self.max_hp,
                source,
            });
        }
        actual_delta
    }

    fn emit_run_resource_diffs(
        &mut self,
        previous_gold: i32,
        previous_hp: i32,
        previous_max_hp: i32,
        source: DomainEventSource,
    ) {
        let gold_delta = self.gold - previous_gold;
        if gold_delta != 0 {
            self.emit_event(DomainEvent::GoldChanged {
                delta: gold_delta,
                new_total: self.gold,
                source,
            });
        }
        let max_hp_delta = self.max_hp - previous_max_hp;
        if max_hp_delta != 0 {
            self.emit_event(DomainEvent::MaxHpChanged {
                delta: max_hp_delta,
                current_hp: self.current_hp,
                max_hp: self.max_hp,
                source,
            });
        } else {
            let hp_delta = self.current_hp - previous_hp;
            if hp_delta != 0 {
                self.emit_event(DomainEvent::HpChanged {
                    delta: hp_delta,
                    current_hp: self.current_hp,
                    max_hp: self.max_hp,
                    source,
                });
            }
        }
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
            has_molten_egg: self
                .relics
                .iter()
                .any(|r| r.id == crate::content::relics::RelicId::MoltenEgg),
            has_toxic_egg: self
                .relics
                .iter()
                .any(|r| r.id == crate::content::relics::RelicId::ToxicEgg),
            has_frozen_egg: self
                .relics
                .iter()
                .any(|r| r.id == crate::content::relics::RelicId::FrozenEgg),
            previous_purge_count: self.shop_purge_count,
            potion_class: self.potion_class(),
            card_blizz_randomizer: self.card_blizz_randomizer,
        };

        let spawn_context = RelicSpawnContext::from_run(self);

        let crate::state::run::RunState {
            ref mut rng_pool,
            ref mut common_relic_pool,
            ref mut uncommon_relic_pool,
            ref mut rare_relic_pool,
            ref mut shop_relic_pool,
            ref mut boss_relic_pool,
            ..
        } = self;

        let mut relic_pools = RelicPoolsMut {
            common: common_relic_pool,
            uncommon: uncommon_relic_pool,
            rare: rare_relic_pool,
            shop: shop_relic_pool,
            boss: boss_relic_pool,
        };

        crate::shop::shop_screen::generate_shop(rng_pool, &config, |tier| {
            random_relic_end_by_tier_from_pools(tier, &mut relic_pools, &spawn_context)
        })
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
            is_daily_run: self.is_daily_run,
            highest_unlocked_ascension_level: self.highest_unlocked_ascension_level,
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
        self.add_card_to_deck_with_upgrades_from(card_id, 0, DomainEventSource::RewardScreen)
    }

    /// Adds a card with an explicit pre-upgrade count.
    pub fn add_card_to_deck_with_upgrades(
        &mut self,
        card_id: crate::content::cards::CardId,
        pre_upgrades: u8,
    ) -> bool {
        self.add_card_to_deck_with_upgrades_from(
            card_id,
            pre_upgrades,
            DomainEventSource::DeckMutation,
        )
    }

    pub fn add_card_to_deck_with_upgrades_from(
        &mut self,
        card_id: crate::content::cards::CardId,
        pre_upgrades: u8,
        source: DomainEventSource,
    ) -> bool {
        let ctx = self.build_deck_context();
        self.add_card_to_deck_with_context(card_id, pre_upgrades, source, ctx)
    }

    pub fn add_card_to_deck_with_omamori_snapshot_from(
        &mut self,
        card_id: crate::content::cards::CardId,
        pre_upgrades: u8,
        source: DomainEventSource,
        has_omamori: bool,
        omamori_charges: i32,
    ) -> bool {
        let mut ctx = self.build_deck_context();
        ctx.has_omamori = has_omamori;
        ctx.omamori_charges = omamori_charges;
        self.add_card_to_deck_with_context(card_id, pre_upgrades, source, ctx)
    }

    fn add_card_to_deck_with_context(
        &mut self,
        card_id: crate::content::cards::CardId,
        pre_upgrades: u8,
        source: DomainEventSource,
        ctx: crate::deck::context::DeckContext,
    ) -> bool {
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
                self.emit_event(DomainEvent::CardObtained {
                    card: Self::snapshot_card(&card),
                    source,
                });
                self.master_deck.push(card);
            }
            self.dispatch_on_master_deck_change();
        }

        self.resolve_deck_actions(result.actions, source);
        was_added
    }

    pub fn add_card_to_deck_without_interception_from(
        &mut self,
        card_id: crate::content::cards::CardId,
        pre_upgrades: u8,
        source: DomainEventSource,
    ) -> bool {
        let ctx = self.build_deck_context();
        let mut target_uuid = self.next_card_uuid();

        let result = crate::deck::manager::DeckManager::obtain_card_without_interception(
            &ctx,
            card_id,
            &mut target_uuid,
            pre_upgrades,
        );
        let mut was_added = false;

        if !result.final_cards.is_empty() {
            was_added = true;
            for card in result.final_cards {
                self.emit_event(DomainEvent::CardObtained {
                    card: Self::snapshot_card(&card),
                    source,
                });
                self.master_deck.push(card);
            }
            self.dispatch_on_master_deck_change();
        }

        self.resolve_deck_actions(result.actions, source);
        was_added
    }

    /// Adds a stat-equivalent copy of an existing master-deck card.
    ///
    /// This is the headless equivalent of Java's `makeStatEquivalentCopy()`
    /// followed by `ShowCardAndObtainEffect`: obtain hooks still run, but
    /// persistent per-card state such as `misc` is preserved on the obtained
    /// copy. Bottle ownership is represented by relic UUIDs in Rust, so the new
    /// UUID naturally clears bottle attachment.
    pub fn add_card_instance_copy_to_deck_from(
        &mut self,
        template: &crate::runtime::combat::CombatCard,
        source: DomainEventSource,
    ) -> bool {
        let ctx = self.build_deck_context();
        let mut target_uuid = self.next_card_uuid();

        let result = crate::deck::manager::DeckManager::obtain_card(
            &ctx,
            template.id,
            &mut target_uuid,
            template.upgrades,
        );
        let mut was_added = false;

        if !result.final_cards.is_empty() {
            was_added = true;
            for mut card in result.final_cards {
                card.misc_value = template.misc_value;
                card.base_damage_override = template.base_damage_override;
                card.cost_modifier = template.cost_modifier;
                card.base_damage_mut = template.base_damage_mut;
                card.base_block_mut = template.base_block_mut;
                card.base_magic_num_mut = template.base_magic_num_mut;
                self.emit_event(DomainEvent::CardObtained {
                    card: Self::snapshot_card(&card),
                    source,
                });
                self.master_deck.push(card);
            }
            self.dispatch_on_master_deck_change();
        }

        self.resolve_deck_actions(result.actions, source);
        was_added
    }

    /// Removes a specific card instance from the master deck.
    /// Handles Parasite triggers, Necronomicurse regeneration.
    pub fn remove_card_from_deck(&mut self, uuid: u32) {
        self.remove_card_from_deck_with_source(uuid, DomainEventSource::DeckMutation);
    }

    pub fn remove_card_from_deck_with_source(&mut self, uuid: u32, source: DomainEventSource) {
        let mut removed_id = None;
        if let Some(pos) = self.master_deck.iter().position(|c| c.uuid == uuid) {
            let removed = self.master_deck.remove(pos);
            self.emit_event(DomainEvent::CardRemoved {
                card: Self::snapshot_card(&removed),
                source,
            });
            removed_id = Some(removed.id);
        }

        if let Some(card_id) = removed_id {
            let result = crate::deck::manager::DeckManager::remove_card(card_id);
            self.dispatch_on_master_deck_change();
            self.resolve_deck_actions(result.actions, source);
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
        }
    }

    pub fn preview_obtain_card_upgrades(
        &self,
        card_id: crate::content::cards::CardId,
        pre_upgrades: u8,
    ) -> u8 {
        let ctx = self.build_deck_context();
        crate::deck::manager::DeckManager::preview_obtain_upgrades(&ctx, card_id, pre_upgrades)
    }

    fn resolve_deck_actions(
        &mut self,
        actions: Vec<crate::deck::manager::DeckAction>,
        source: DomainEventSource,
    ) {
        use crate::deck::manager::DeckAction;
        for action in actions {
            match action {
                DeckAction::PreventObtain => { /* Handled structurally */ }
                DeckAction::GainGold(amount) => {
                    self.change_gold_with_source(amount, source);
                }
                DeckAction::GainMaxHp(amount) => {
                    self.gain_max_hp_with_source(amount, amount, source);
                }
                DeckAction::LoseMaxHp(amount) => {
                    self.lose_max_hp_with_source(amount, source);
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
                    self.add_card_to_deck_with_upgrades_from(card_id, 0, source);
                }
            }
        }
    }

    /// Triggers AbstractRelic.onMasterDeckChange for all relics
    pub fn dispatch_on_master_deck_change(&mut self) {
        crate::content::relics::du_vu_doll::refresh_counters_from_deck(
            &self.master_deck,
            &mut self.relics,
        );
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
        crate::runtime::rng::shuffle_with_random_long(
            &mut self.common_relic_pool,
            &mut self.rng_pool.relic_rng,
        );
        crate::runtime::rng::shuffle_with_random_long(
            &mut self.uncommon_relic_pool,
            &mut self.rng_pool.relic_rng,
        );
        crate::runtime::rng::shuffle_with_random_long(
            &mut self.rare_relic_pool,
            &mut self.rng_pool.relic_rng,
        );
        crate::runtime::rng::shuffle_with_random_long(
            &mut self.shop_relic_pool,
            &mut self.rng_pool.relic_rng,
        );
        crate::runtime::rng::shuffle_with_random_long(
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

    /// Initialize the boss list for the current act and publish the visible
    /// boss key.
    ///
    /// Java: dungeon constructors call `initializeBoss()`, then
    /// `setBoss(bossList.get(0))`. The list itself is not truncated; on A20
    /// Act 3, the first boss removes index 0, then `ProceedButton` sees two
    /// remaining entries and sets `bossKey = bossList.get(0)` for the second
    /// boss.
    pub fn init_boss_list(&mut self) {
        self.boss_list = crate::content::monsters::encounter_pool::generate_boss_list(
            self.act_num,
            &mut self.rng_pool.monster_rng,
        );
        self.boss_key = self.boss_list.first().copied();
    }

    /// Pop the next boss from the pre-scheduled list.
    /// Java: `getBoss()` uses `bossKey`, then
    /// `MonsterRoomBoss.onPlayerEntry()` removes `bossList[0]`.
    pub fn next_boss(&mut self) -> Option<crate::content::monsters::factory::EncounterId> {
        let boss = self.boss_key.or_else(|| self.boss_list.first().copied());
        if !self.boss_list.is_empty() {
            self.boss_list.remove(0);
        }
        boss
    }

    pub fn reveal_next_boss_from_list(
        &mut self,
    ) -> Option<crate::content::monsters::factory::EncounterId> {
        self.boss_key = self.boss_list.first().copied();
        self.boss_key
    }

    pub fn should_start_act3_double_boss(&self) -> bool {
        self.act_num == 3 && self.ascension_level >= 20 && self.boss_list.len() == 2
    }

    pub fn enter_final_act(&mut self) {
        self.act_num = 4;
        self.pending_boss_reward = false;
        self.pending_boss_act_transition = false;
        self.apply_dungeon_transition_setup_effects();
        self.map = crate::map::state::MapState::new(crate::map::generator::generate_ending_map());
        self.init_encounter_lists();
        self.init_boss_list();
        self.event_generator.initialize_event_pools(self.act_num);
        self.event_generator.reset_probabilities();
        self.card_upgraded_chance = if self.ascension_level >= 12 {
            0.25
        } else {
            0.5
        };
    }

    /// Advance to the next act after boss defeat.
    /// Regenerates map, encounter lists, event pools, and boss list for the new act.
    /// Java flow: VictoryRoom → Proceed → next Dungeon constructor.
    pub fn advance_act(&mut self) {
        self.act_num += 1;
        self.pending_boss_reward = false;
        self.pending_boss_act_transition = false;

        self.apply_dungeon_transition_setup_effects();

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
    }

    pub fn complete_pending_boss_act_transition(&mut self) -> bool {
        if self.pending_boss_act_transition {
            self.advance_act();
            true
        } else {
            false
        }
    }

    fn apply_dungeon_transition_setup_effects(&mut self) {
        self.align_card_rng_counter_for_dungeon_transition();
        self.potion_drop_chance_mod = 0;
        self.heal_for_dungeon_transition();
    }

    fn heal_for_dungeon_transition(&mut self) {
        let missing = self.max_hp - self.current_hp;
        let heal_amount = if self.ascension_level >= 5 {
            (missing as f32 * 0.75).round() as i32
        } else {
            self.max_hp
        };
        self.current_hp = (self.current_hp + heal_amount).min(self.max_hp);
    }

    fn align_card_rng_counter_for_dungeon_transition(&mut self) {
        let target = match self.rng_pool.card_rng.counter {
            1..=249 => Some(250),
            251..=499 => Some(500),
            501..=749 => Some(750),
            _ => None,
        };
        if let Some(target) = target {
            self.rng_pool.card_rng.advance_counter_to(target);
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

    /// Pop a relic from the specified tier pool using Java's normal reward
    /// path. Common/uncommon/rare/shop pools consume from the front
    /// (`returnRandomRelicKey` / `remove(0)`), while boss relics also consume
    /// from the front. Shop screens use `random_relic_end_by_tier`.
    ///
    /// Java quirk: if a front candidate fails `canSpawn`, the fallback is
    /// `returnEndRandomRelicKey(tier)`, not another front draw.
    pub fn random_relic_by_tier(
        &mut self,
        tier: crate::content::relics::RelicTier,
    ) -> crate::content::relics::RelicId {
        let spawn_context = RelicSpawnContext::from_run(self);
        let mut relic_pools = RelicPoolsMut {
            common: &mut self.common_relic_pool,
            uncommon: &mut self.uncommon_relic_pool,
            rare: &mut self.rare_relic_pool,
            shop: &mut self.shop_relic_pool,
            boss: &mut self.boss_relic_pool,
        };
        random_relic_by_tier_from_pools(tier, &mut relic_pools, &spawn_context)
    }

    /// Pop a relic using Java's shop/end path
    /// (`returnEndRandomRelicKey`). Common/uncommon/rare/shop pools consume
    /// from the end. Empty common/uncommon/shop pools fall back to the normal
    /// front path of the next tier, matching Java's odd mixed fallback.
    pub fn random_relic_end_by_tier(
        &mut self,
        tier: crate::content::relics::RelicTier,
    ) -> crate::content::relics::RelicId {
        let spawn_context = RelicSpawnContext::from_run(self);
        let mut relic_pools = RelicPoolsMut {
            common: &mut self.common_relic_pool,
            uncommon: &mut self.uncommon_relic_pool,
            rare: &mut self.rare_relic_pool,
            shop: &mut self.shop_relic_pool,
            boss: &mut self.boss_relic_pool,
        };
        random_relic_end_by_tier_from_pools(tier, &mut relic_pools, &spawn_context)
    }

    #[cfg(test)]
    fn relic_can_spawn_now(&self, id: crate::content::relics::RelicId) -> bool {
        crate::state::relic_pool::relic_can_spawn_in_context(id, &RelicSpawnContext::from_run(self))
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

    /// Roll a relic tier, then return a Java screenless relic from that tier.
    /// Used by events that call `AbstractDungeon.returnRandomScreenlessRelic`
    /// rather than room reward generation.
    pub fn random_screenless_relic_reward(&mut self) -> crate::content::relics::RelicId {
        let tier = self.return_random_relic_tier();
        self.random_screenless_relic(tier)
    }

    /// Default random relic reward: roll tier then return a normal relic from
    /// that tier. Java combat/chest reward paths use `returnRandomRelic`, not
    /// `returnRandomScreenlessRelic`, so screen-interrupting relics such as
    /// Bottled Flame can appear here.
    pub fn random_relic(&mut self) -> crate::content::relics::RelicId {
        let tier = self.return_random_relic_tier();
        self.random_relic_by_tier(tier)
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
        self.obtain_potion_with_source(potion, DomainEventSource::DeckMutation)
    }

    pub fn obtain_potion_with_source(
        &mut self,
        potion: crate::content::potions::Potion,
        source: DomainEventSource,
    ) -> bool {
        if let Some(slot) = self.potions.iter().position(|p| p.is_none()) {
            let potion_id = potion.id;
            self.potions[slot] = Some(potion);
            self.emit_event(DomainEvent::PotionObtained {
                potion_id,
                slot,
                source,
            });
            true
        } else {
            false
        }
    }

    pub fn remove_potion_at_with_source(
        &mut self,
        slot: usize,
        source: DomainEventSource,
    ) -> Option<crate::content::potions::PotionId> {
        let potion = self.potions.get_mut(slot)?.take()?;
        let potion_id = potion.id;
        self.emit_event(DomainEvent::PotionLost {
            potion_id,
            slot,
            source,
        });
        Some(potion_id)
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
        let mut pool = COLORLESS_UNCOMMON_POOL
            .iter()
            .copied()
            .chain(COLORLESS_RARE_POOL.iter().copied())
            .collect::<Vec<_>>();
        let seed = self.rng_pool.shuffle_rng.random_long();
        let mut jur = crate::runtime::rng::JavaUtilRandom::new(seed);
        for i in (1..pool.len()).rev() {
            let j = jur.next_int((i + 1) as i32) as usize;
            pool.swap(i, j);
        }

        if let Some(card_id) = pool
            .iter()
            .copied()
            .find(|card_id| get_card_definition(*card_id).rarity == rarity)
        {
            return card_id;
        }
        if rarity == CardRarity::Rare {
            if let Some(card_id) = pool
                .iter()
                .copied()
                .find(|card_id| get_card_definition(*card_id).rarity == CardRarity::Uncommon)
            {
                return card_id;
            }
        }
        CardId::SwiftStrike
    }

    /// Returns a random card from the current class pool of the given rarity.
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
                "Defect" => CardId::StrikeB,
                "Watcher" => CardId::StrikeP,
                _ => CardId::Strike,
            };
        }
        let idx = self
            .rng_pool
            .card_rng
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
            "Defect" => defect_pool_for_type(card_type),
            "Watcher" => watcher_pool_for_type(card_type),
            _ => ironclad_pool_for_type(card_type),
        };
        if pool.is_empty() {
            return match self.player_class {
                "Silent" => CardId::StrikeG,
                "Defect" => CardId::StrikeB,
                "Watcher" => CardId::StrikeP,
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
        self.upgrade_random_cards_with_source(count, DomainEventSource::DeckMutation);
    }

    pub fn upgrade_random_cards_with_source(&mut self, count: usize, source: DomainEventSource) {
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
        crate::runtime::rng::shuffle_with_random_long(
            &mut upgradable_indices,
            &mut self.rng_pool.misc_rng,
        );

        // Upgrade up to `count` cards
        for &idx in upgradable_indices.iter().take(count) {
            let uuid = self.master_deck[idx].uuid;
            self.upgrade_card_with_source(uuid, source);
        }
    }

    /// Upgrades a specific card in the master deck by its UUID.
    pub fn upgrade_card(&mut self, uuid: u32) {
        self.upgrade_card_with_source(uuid, DomainEventSource::DeckMutation);
    }

    pub fn upgrade_card_with_source(&mut self, uuid: u32, source: DomainEventSource) {
        if let Some(card) = self.master_deck.iter_mut().find(|c| c.uuid == uuid) {
            let before = Self::snapshot_card(card);
            card.upgrades += 1;
            let after = Self::snapshot_card(card);
            self.emit_event(DomainEvent::CardUpgraded {
                before,
                after,
                source,
            });
        }
    }

    pub fn modify_card_misc_value(&mut self, uuid: u32, amount: i32) {
        if let Some(card) = self.master_deck.iter_mut().find(|c| c.uuid == uuid) {
            card.misc_value += amount;
        }
    }

    /// Transforms a card: removes it from deck and replaces with a random card of the same color.
    /// Uses DeckManager properly so Omamori/Necronomicurse triggers fire correctly.
    /// `auto_upgrade` is true when transforming via Astrolabe.
    pub fn transform_card(&mut self, deck_index: usize, auto_upgrade: bool) {
        self.transform_card_with_source(deck_index, auto_upgrade, DomainEventSource::DeckMutation);
    }

    pub fn transform_card_with_source(
        &mut self,
        deck_index: usize,
        auto_upgrade: bool,
        source: DomainEventSource,
    ) {
        if deck_index >= self.master_deck.len() {
            return;
        }

        let old_card_id = self.master_deck[deck_index].id;
        let old_card_uuid = self.master_deck[deck_index].uuid;
        let before = Self::snapshot_card(&self.master_deck[deck_index]);
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
                let idx = self.transform_random_index(filtered.len(), source);
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
                let idx = self.transform_random_index(pool.len(), source);
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
                let idx = self.transform_random_index(pool.len(), source);
                pool[idx]
            }
        };

        // 1. Remove logically without emitting a standalone remove event.
        let mut removed_id = None;
        if let Some(pos) = self
            .master_deck
            .iter()
            .position(|c| c.uuid == old_card_uuid)
        {
            let removed = self.master_deck.remove(pos);
            removed_id = Some(removed.id);
        }
        if let Some(card_id) = removed_id {
            let remove_result = crate::deck::manager::DeckManager::remove_card(card_id);
            self.resolve_deck_actions(remove_result.actions, source);
        }

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
            self.emit_event(DomainEvent::CardTransformed {
                before,
                after: Self::snapshot_card(&card),
                source,
            });
            self.master_deck.push(card);
            obtained_any = true;
        }
        let _ = def;
        let _ = obtained_any;

        // 4. Resolve obtain-triggered deck actions
        self.resolve_deck_actions(result.actions, source);
        self.dispatch_on_master_deck_change();
    }

    fn transform_random_index(&mut self, len: usize, source: DomainEventSource) -> usize {
        if len == 0 {
            return 0;
        }
        let rng = if source == DomainEventSource::Event(crate::state::events::EventId::Neow) {
            &mut self.neow_rng
        } else {
            &mut self.rng_pool.misc_rng
        };
        rng.random_range(0, len as i32 - 1) as usize
    }

    pub fn emit_event(&mut self, event: DomainEvent) {
        self.emitted_events.push(event);
    }

    pub fn take_emitted_events(&mut self) -> Vec<DomainEvent> {
        std::mem::take(&mut self.emitted_events)
    }

    fn snapshot_card(card: &CombatCard) -> DomainCardSnapshot {
        DomainCardSnapshot {
            id: card.id,
            upgrades: card.upgrades,
            uuid: card.uuid,
        }
    }
}

fn is_starter_upgrade_boss_relic(relic_id: crate::content::relics::RelicId) -> bool {
    matches!(
        relic_id,
        crate::content::relics::RelicId::BlackBlood
            | crate::content::relics::RelicId::RingOfTheSerpent
            | crate::content::relics::RelicId::FrozenCore
            | crate::content::relics::RelicId::HolyWater
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::{CardId, CardRarity};
    use crate::content::relics::RelicId;

    fn deck_ids(run: &RunState) -> Vec<CardId> {
        run.master_deck.iter().map(|card| card.id).collect()
    }

    fn card_rng_after_calls(count: u32) -> StsRng {
        let mut rng = StsRng::new(17);
        for _ in 0..count {
            let _ = rng.random(999);
        }
        rng
    }

    #[test]
    fn starting_loadouts_use_class_specific_java_starter_decks() {
        let ironclad = RunState::new(1, 0, false, "Ironclad");
        assert_eq!(
            deck_ids(&ironclad),
            vec![
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
            ]
        );
        assert_eq!(ironclad.relics[0].id, RelicId::BurningBlood);

        let silent = RunState::new(1, 0, false, "Silent");
        assert_eq!(
            deck_ids(&silent),
            vec![
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
            ]
        );
        assert_eq!(silent.relics[0].id, RelicId::SnakeRing);

        let defect = RunState::new(1, 0, false, "Defect");
        assert_eq!(
            deck_ids(&defect),
            vec![
                CardId::StrikeB,
                CardId::StrikeB,
                CardId::StrikeB,
                CardId::StrikeB,
                CardId::DefendB,
                CardId::DefendB,
                CardId::DefendB,
                CardId::DefendB,
                CardId::Zap,
                CardId::Dualcast,
            ]
        );
        assert_eq!(defect.relics[0].id, RelicId::CrackedCore);

        let watcher = RunState::new(1, 0, false, "Watcher");
        assert_eq!(
            deck_ids(&watcher),
            vec![
                CardId::StrikeP,
                CardId::StrikeP,
                CardId::StrikeP,
                CardId::StrikeP,
                CardId::DefendP,
                CardId::DefendP,
                CardId::DefendP,
                CardId::DefendP,
                CardId::Eruption,
                CardId::Vigilance,
            ]
        );
        assert_eq!(watcher.relics[0].id, RelicId::PureWater);
    }

    #[test]
    fn event_random_card_helpers_use_java_rng_streams() {
        let mut run = RunState::new(11, 0, false, "Ironclad");
        let card_before = run.rng_pool.card_rng.counter;
        let misc_before = run.rng_pool.misc_rng.counter;
        let shuffle_before = run.rng_pool.shuffle_rng.counter;

        let _ = run.random_card_by_rarity(CardRarity::Rare);

        assert_eq!(
            run.rng_pool.card_rng.counter,
            card_before + 1,
            "Java AbstractDungeon.getCard(rarity) uses cardRng via CardGroup.getRandomCard(true)"
        );
        assert_eq!(
            run.rng_pool.misc_rng.counter, misc_before,
            "rarity card selection must not consume miscRng; Match and Keep uses miscRng later for board shuffle"
        );
        assert_eq!(run.rng_pool.shuffle_rng.counter, shuffle_before);

        let card_after = run.rng_pool.card_rng.counter;
        let misc_after = run.rng_pool.misc_rng.counter;
        let shuffle_after = run.rng_pool.shuffle_rng.counter;

        let _ = run.random_colorless_card(CardRarity::Uncommon);

        assert_eq!(run.rng_pool.card_rng.counter, card_after);
        assert_eq!(run.rng_pool.misc_rng.counter, misc_after);
        assert_eq!(
            run.rng_pool.shuffle_rng.counter,
            shuffle_after + 1,
            "Java returnColorlessCard(rarity) shuffles colorlessCardPool with shuffleRng.randomLong()"
        );
    }

    #[test]
    fn boss_key_is_public_boss_while_boss_list_keeps_java_queue() {
        let mut run = RunState::new(7, 20, false, "Ironclad");
        run.act_num = 3;
        run.init_boss_list();

        assert_eq!(
            run.boss_key,
            run.boss_list.first().copied(),
            "Java setBoss(bossList[0]) publishes the current map boss"
        );
        assert_eq!(
            run.boss_list.len(),
            3,
            "Java keeps the full shuffled bossList; A20 double boss depends on the post-entry size"
        );

        let first = run.boss_key;
        assert_eq!(run.next_boss(), first);
        assert_eq!(run.boss_list.len(), 2);
        assert!(run.should_start_act3_double_boss());

        let second = run.reveal_next_boss_from_list();
        assert_eq!(second, run.boss_list.first().copied());
        assert_eq!(run.next_boss(), second);
        assert_eq!(run.boss_list.len(), 1);
        assert!(!run.should_start_act3_double_boss());
    }

    #[test]
    fn final_act_initializes_shield_spear_and_heart_context() {
        use crate::content::monsters::factory::EncounterId;

        let mut run = RunState::new(7, 20, true, "Ironclad");
        run.current_hp = 20;
        run.max_hp = 80;
        run.potion_drop_chance_mod = 30;
        run.rng_pool.card_rng = card_rng_after_calls(501);
        let mut expected_card_rng = run.rng_pool.card_rng.clone();
        expected_card_rng.advance_counter_to(750);

        run.enter_final_act();

        assert_eq!(run.act_num, 4);
        assert_eq!(
            run.current_hp, 65,
            "TheEnding constructor also runs dungeonTransitionSetup and heals once"
        );
        assert_eq!(
            run.potion_drop_chance_mod, 0,
            "Java dungeonTransitionSetup resets AbstractRoom.blizzardPotionMod on Act 4 entry too"
        );
        assert_eq!(
            run.rng_pool.card_rng, expected_card_rng,
            "TheEnding constructor also applies the cardRng counter band alignment"
        );
        assert_eq!(run.elite_monster_list, vec![EncounterId::ShieldAndSpear; 3]);
        assert_eq!(run.monster_list, vec![EncounterId::ShieldAndSpear; 3]);
        assert_eq!(run.boss_list, vec![EncounterId::TheHeart; 3]);
        assert_eq!(run.boss_key, Some(EncounterId::TheHeart));
    }

    #[test]
    fn advance_act_heals_once_like_java_dungeon_transition_setup() {
        let mut asc5 = RunState::new(7, 5, false, "Ironclad");
        asc5.current_hp = 20;
        asc5.max_hp = 80;
        asc5.advance_act();
        assert_eq!(
            asc5.current_hp, 65,
            "Java dungeonTransitionSetup heals round((max-current)*0.75) once at Ascension 5+"
        );

        let mut low_asc = RunState::new(7, 0, false, "Ironclad");
        low_asc.current_hp = 20;
        low_asc.max_hp = 80;
        low_asc.advance_act();
        assert_eq!(low_asc.current_hp, 80);
    }

    #[test]
    fn advance_act_aligns_card_rng_counter_like_java_dungeon_transition_setup() {
        for (counter_before, expected_counter_after) in [
            (0, 0),
            (1, 250),
            (249, 250),
            (250, 250),
            (251, 500),
            (499, 500),
            (500, 500),
            (501, 750),
            (749, 750),
            (750, 750),
            (800, 800),
        ] {
            let mut run = RunState::new(17, 0, false, "Ironclad");
            run.rng_pool.card_rng = card_rng_after_calls(counter_before);

            let mut expected = run.rng_pool.card_rng.clone();
            expected.advance_counter_to(expected_counter_after);

            run.advance_act();

            assert_eq!(
                run.rng_pool.card_rng, expected,
                "Java dungeonTransitionSetup aligns cardRng counter {counter_before} to {expected_counter_after} by consuming randomBoolean calls"
            );
        }
    }

    #[test]
    fn advance_act_resets_potion_drop_chance_like_java_dungeon_transition_setup() {
        let mut run = RunState::new(7, 0, false, "Ironclad");
        run.potion_drop_chance_mod = -20;

        run.advance_act();

        assert_eq!(
            run.potion_drop_chance_mod, 0,
            "Java dungeonTransitionSetup resets AbstractRoom.blizzardPotionMod between acts"
        );
    }

    #[test]
    fn boss_starter_upgrade_relics_require_matching_java_starter_relics() {
        let ironclad = RunState::new(1, 0, false, "Ironclad");
        assert!(ironclad.relic_can_spawn_now(RelicId::BlackBlood));
        assert!(!ironclad.relic_can_spawn_now(RelicId::RingOfTheSerpent));
        assert!(!ironclad.relic_can_spawn_now(RelicId::FrozenCore));
        assert!(!ironclad.relic_can_spawn_now(RelicId::HolyWater));

        let silent = RunState::new(1, 0, false, "Silent");
        assert!(silent.relic_can_spawn_now(RelicId::RingOfTheSerpent));
        assert!(!silent.relic_can_spawn_now(RelicId::BlackBlood));
        assert!(!silent.relic_can_spawn_now(RelicId::FrozenCore));
        assert!(!silent.relic_can_spawn_now(RelicId::HolyWater));

        let defect = RunState::new(1, 0, false, "Defect");
        assert!(defect.relic_can_spawn_now(RelicId::FrozenCore));
        assert!(!defect.relic_can_spawn_now(RelicId::BlackBlood));
        assert!(!defect.relic_can_spawn_now(RelicId::RingOfTheSerpent));
        assert!(!defect.relic_can_spawn_now(RelicId::HolyWater));

        let watcher = RunState::new(1, 0, false, "Watcher");
        assert!(watcher.relic_can_spawn_now(RelicId::HolyWater));
        assert!(!watcher.relic_can_spawn_now(RelicId::BlackBlood));
        assert!(!watcher.relic_can_spawn_now(RelicId::RingOfTheSerpent));
        assert!(!watcher.relic_can_spawn_now(RelicId::FrozenCore));
    }
}
