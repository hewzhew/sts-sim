use crate::content::cards::CardId;
use crate::content::relics::RelicState;
use crate::core::EntityId;
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::state::selection::{DomainEvent, EngineDiagnostic};
use std::collections::{HashMap, HashSet, VecDeque};
use std::ops::{Deref, DerefMut};

#[derive(Clone, Debug, PartialEq)]
pub enum MetaChange {
    AddCardToMasterDeck(CardId),
}

// --- ID Types ---
pub use crate::content::powers::PowerId;
pub type MonsterId = usize;

#[derive(Clone, Debug, PartialEq)]
pub struct CombatState {
    pub meta: CombatMeta,
    pub turn: TurnRuntime,
    pub zones: CardZones,
    pub entities: EntityState,
    pub engine: EngineRuntime,
    pub rng: CombatRng,
    pub runtime: CombatRuntimeHints,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CombatMeta {
    pub ascension_level: u8,
    pub player_class: &'static str,
    pub is_boss_fight: bool,
    pub is_elite_fight: bool,
    pub meta_changes: Vec<MetaChange>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct TurnRuntime {
    pub turn_count: u32,
    pub current_phase: CombatPhase,
    pub energy: u8,
    /// Narrow Rust equivalent of Java's start-of-turn draw target adjustments.
    /// This stores only modifier semantics for next turn draw count, not a
    /// full copy of Java's mutable `player.gameHandSize`.
    pub turn_start_draw_modifier: i32,
    pub counters: EphemeralCounters,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EngineRuntime {
    pub action_queue: VecDeque<Action>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CombatRng {
    pub pool: crate::runtime::rng::RngPool,
}

impl CombatRng {
    pub fn new(pool: crate::runtime::rng::RngPool) -> Self {
        Self { pool }
    }
}

impl Deref for CombatRng {
    type Target = crate::runtime::rng::RngPool;

    fn deref(&self) -> &Self::Target {
        &self.pool
    }
}

impl DerefMut for CombatRng {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.pool
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CardZones {
    pub draw_pile: Vec<CombatCard>,
    pub hand: Vec<CombatCard>,
    pub discard_pile: Vec<CombatCard>,
    pub exhaust_pile: Vec<CombatCard>,
    pub limbo: Vec<CombatCard>,
    pub queued_cards: VecDeque<QueuedCardPlay>,
    pub card_uuid_counter: u32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct EntityState {
    pub player: PlayerEntity,
    pub monsters: Vec<MonsterEntity>,
    pub potions: Vec<Option<crate::content::potions::Potion>>,
    pub power_db: HashMap<EntityId, Vec<Power>>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum QueuedCardSource {
    Normal,
    Necronomicon,
    DoubleTap,
    Duplication,
    Burst,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueuedCardPlay {
    pub card: CombatCard,
    pub target: Option<EntityId>,
    pub energy_on_use: i32,
    pub ignore_energy_total: bool,
    pub autoplay: bool,
    pub random_target: bool,
    pub is_end_turn_autoplay: bool,
    pub purge_on_use: bool,
    pub source: QueuedCardSource,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CombatRuntimeHints {
    pub using_card: bool,
    pub card_queue: Vec<QueuedCardHint>,
    pub colorless_combat_pool: Vec<CardId>,
    pub emitted_events: Vec<DomainEvent>,
    pub engine_diagnostics: Vec<EngineDiagnostic>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct QueuedCardHint {
    pub card_uuid: u32,
    pub card_id: CardId,
    pub target_monster_index: Option<usize>,
    pub energy_on_use: i32,
    pub ignore_energy_total: bool,
    pub autoplay: bool,
    pub random_target: bool,
    pub is_end_turn_autoplay: bool,
    pub purge_on_use: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum CombatPhase {
    PlayerTurn,
    MonsterTurn,
    TurnTransition,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EphemeralCounters {
    pub cards_played_this_turn: u8,
    pub attacks_played_this_turn: u8,
    pub times_damaged_this_combat: u8,
    pub victory_triggered: bool,
    pub discovery_cost_for_turn: Option<u8>,
    pub early_end_turn_pending: bool,
    pub player_escaping: bool,
    pub escape_pending_reward: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RelicBuses {
    pub at_pre_battle: smallvec::SmallVec<[usize; 4]>,
    pub at_battle_start_pre_draw: smallvec::SmallVec<[usize; 4]>,
    pub at_battle_start: smallvec::SmallVec<[usize; 4]>,
    pub at_turn_start: smallvec::SmallVec<[usize; 4]>,
    pub at_turn_start_post_draw: smallvec::SmallVec<[usize; 4]>,
    pub on_use_card: smallvec::SmallVec<[usize; 4]>,
    pub on_shuffle: smallvec::SmallVec<[usize; 4]>,
    pub on_exhaust: smallvec::SmallVec<[usize; 4]>,
    pub on_lose_hp: smallvec::SmallVec<[usize; 4]>,
    pub on_victory: smallvec::SmallVec<[usize; 4]>,
    pub on_apply_power: smallvec::SmallVec<[usize; 4]>,
    pub on_monster_death: smallvec::SmallVec<[usize; 4]>,
    pub on_spawn_monster: smallvec::SmallVec<[usize; 4]>,
    pub at_end_of_turn: smallvec::SmallVec<[usize; 4]>,
    pub on_use_potion: smallvec::SmallVec<[usize; 4]>,
    pub on_discard: smallvec::SmallVec<[usize; 4]>,
    pub on_change_stance: smallvec::SmallVec<[usize; 4]>,
    pub on_attacked_to_change_damage: smallvec::SmallVec<[usize; 4]>,
    pub on_lose_hp_last: smallvec::SmallVec<[usize; 4]>,

    // Core Engine Value Modifiers
    pub on_calculate_heal: smallvec::SmallVec<[usize; 4]>,
    pub on_calculate_x_cost: smallvec::SmallVec<[usize; 4]>,
    pub on_calculate_block_retained: smallvec::SmallVec<[usize; 4]>,
    pub on_calculate_energy_retained: smallvec::SmallVec<[usize; 4]>,
    pub on_scry: smallvec::SmallVec<[usize; 4]>,
    pub on_receive_power_modify: smallvec::SmallVec<[usize; 4]>,
    pub on_calculate_vulnerable_multiplier: smallvec::SmallVec<[usize; 4]>,
}

impl CombatState {
    pub fn emit_event(&mut self, event: DomainEvent) {
        self.runtime.emitted_events.push(event);
    }

    pub fn take_emitted_events(&mut self) -> Vec<DomainEvent> {
        std::mem::take(&mut self.runtime.emitted_events)
    }

    pub fn emit_diagnostic(&mut self, diagnostic: EngineDiagnostic) {
        self.runtime.engine_diagnostics.push(diagnostic);
    }

    pub fn take_engine_diagnostics(&mut self) -> Vec<EngineDiagnostic> {
        std::mem::take(&mut self.runtime.engine_diagnostics)
    }

    pub fn queue_action_front(&mut self, action: Action) {
        self.engine.push_front(action);
    }

    pub fn queue_action_back(&mut self, action: Action) {
        self.engine.push_back(action);
    }

    pub fn queue_actions(&mut self, actions: smallvec::SmallVec<[ActionInfo; 4]>) {
        self.engine.queue_actions(actions);
    }

    pub fn pop_next_action(&mut self) -> Option<Action> {
        self.engine.pop_front()
    }

    pub fn has_pending_actions(&self) -> bool {
        self.engine.has_actions()
    }

    pub fn action_queue_len(&self) -> usize {
        self.engine.len()
    }

    pub fn clear_pending_actions(&mut self) {
        self.engine.clear();
    }

    pub fn ensure_flush_next_queued_card(&mut self) {
        if !self.engine.has_actions() && !self.zones.queued_cards.is_empty() {
            self.engine.push_back(Action::FlushNextQueuedCard);
        }
    }

    pub fn begin_turn_transition(&mut self) {
        self.turn.begin_turn_transition();
    }

    pub fn begin_monster_turn(&mut self) {
        self.turn.begin_monster_turn();
    }

    pub fn begin_next_player_turn(&mut self) {
        self.turn
            .begin_next_player_turn(self.entities.player.energy_master);
    }

    pub fn reset_turn_energy_from_player(&mut self) {
        self.turn.set_energy(self.entities.player.energy_master);
    }
}

impl TurnRuntime {
    pub fn fresh_player_turn(energy: u8) -> Self {
        Self {
            turn_count: 0,
            current_phase: CombatPhase::PlayerTurn,
            energy,
            turn_start_draw_modifier: 0,
            counters: Default::default(),
        }
    }

    pub fn set_energy(&mut self, energy: u8) {
        self.energy = energy;
    }

    pub fn adjust_energy(&mut self, amount: i32) {
        self.energy = (self.energy as i32 + amount).max(0) as u8;
    }

    pub fn spend_energy(&mut self, amount: i32) {
        self.adjust_energy(-amount);
    }

    pub fn increment_cards_played(&mut self) {
        self.counters.cards_played_this_turn += 1;
    }

    pub fn increment_attacks_played(&mut self) {
        self.counters.attacks_played_this_turn += 1;
    }

    pub fn increment_times_damaged_this_combat(&mut self) {
        self.counters.times_damaged_this_combat += 1;
    }

    pub fn mark_early_end_turn_pending(&mut self) {
        self.counters.early_end_turn_pending = true;
    }

    pub fn clear_early_end_turn_pending(&mut self) {
        self.counters.early_end_turn_pending = false;
    }

    pub fn set_discovery_cost_for_turn(&mut self, cost: Option<u8>) {
        self.counters.discovery_cost_for_turn = cost;
    }

    pub fn take_discovery_cost_for_turn(&mut self) -> Option<u8> {
        self.counters.discovery_cost_for_turn.take()
    }

    pub fn mark_player_escaping(&mut self) {
        self.counters.player_escaping = true;
    }

    pub fn clear_escape_pending_reward(&mut self) {
        self.counters.escape_pending_reward = false;
    }

    pub fn mark_escape_pending_reward(&mut self) {
        self.counters.escape_pending_reward = true;
    }

    pub fn mark_victory_triggered(&mut self) {
        self.counters.victory_triggered = true;
    }

    pub fn begin_player_phase(&mut self) {
        self.current_phase = CombatPhase::PlayerTurn;
    }

    pub fn begin_turn_transition(&mut self) {
        self.current_phase = CombatPhase::TurnTransition;
    }

    pub fn begin_monster_turn(&mut self) {
        self.current_phase = CombatPhase::MonsterTurn;
    }

    pub fn begin_next_player_turn(&mut self, energy: u8) {
        self.turn_count += 1;
        self.begin_player_phase();
        self.energy = energy;
        self.counters.cards_played_this_turn = 0;
        self.counters.attacks_played_this_turn = 0;
    }
}

impl EngineRuntime {
    pub fn new() -> Self {
        Self {
            action_queue: VecDeque::new(),
        }
    }

    pub fn push_front(&mut self, action: Action) {
        self.action_queue.push_front(action);
    }

    pub fn push_back(&mut self, action: Action) {
        self.action_queue.push_back(action);
    }

    pub fn pop_front(&mut self) -> Option<Action> {
        self.action_queue.pop_front()
    }

    pub fn has_actions(&self) -> bool {
        !self.action_queue.is_empty()
    }

    pub fn len(&self) -> usize {
        self.action_queue.len()
    }

    pub fn clear(&mut self) {
        self.action_queue.clear();
    }

    pub fn retain(&mut self, f: impl FnMut(&Action) -> bool) {
        self.action_queue.retain(f);
    }

    pub fn queue_actions(&mut self, actions: smallvec::SmallVec<[ActionInfo; 4]>) {
        let mut to_bottom = vec![];
        let mut to_front = vec![];

        for a in actions {
            match a.insertion_mode {
                AddTo::Top => to_front.push(a.action),
                AddTo::Bottom => to_bottom.push(a.action),
            }
        }

        for action in to_front.into_iter().rev() {
            self.push_front(action);
        }
        for action in to_bottom {
            self.push_back(action);
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OrbId {
    Empty, // Placeholder for an empty orb slot
    Lightning,
    Dark,
    Frost,
    Plasma,
}

#[derive(Clone, Debug, PartialEq)]
pub struct OrbEntity {
    pub id: OrbId,
    pub passive_amount: i32,
    pub evoke_amount: i32,
}

impl OrbEntity {
    pub fn new(id: OrbId) -> Self {
        match id {
            OrbId::Empty => OrbEntity {
                id,
                passive_amount: 0,
                evoke_amount: 0,
            },
            OrbId::Lightning => OrbEntity {
                id,
                passive_amount: 3,
                evoke_amount: 8,
            },
            OrbId::Dark => OrbEntity {
                id,
                passive_amount: 6,
                evoke_amount: 6,
            },
            OrbId::Frost => OrbEntity {
                id,
                passive_amount: 2,
                evoke_amount: 5,
            },
            OrbId::Plasma => OrbEntity {
                id,
                passive_amount: 1,
                evoke_amount: 2,
            },
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StanceId {
    Neutral,
    Wrath,
    Calm,
    Divinity,
}

impl StanceId {
    pub fn as_str(&self) -> &'static str {
        match self {
            StanceId::Neutral => "Neutral",
            StanceId::Wrath => "Wrath",
            StanceId::Calm => "Calm",
            StanceId::Divinity => "Divinity",
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct PlayerEntity {
    pub id: EntityId,
    pub current_hp: i32,
    pub max_hp: i32,
    pub block: i32,
    pub gold_delta_this_combat: i32,
    pub gold: i32,
    pub max_orbs: u8,
    pub orbs: Vec<OrbEntity>,
    pub stance: StanceId,
    pub relics: Vec<RelicState>,
    pub relic_buses: RelicBuses,
    /// Java: EnergyManager.energyMaster — base energy per turn.
    /// Starts at 3, boss relics with onEquip() { ++energyMaster } increment this.
    /// SlaversCollar conditionally adds +1 at battle start (handled separately).
    pub energy_master: u8,
}

impl PlayerEntity {
    pub fn has_relic(&self, id: crate::content::relics::RelicId) -> bool {
        self.relics.iter().any(|r| r.id == id)
    }

    pub fn add_relic(&mut self, state: RelicState) {
        let index = self.relics.len();
        let sub = crate::content::relics::get_relic_subscriptions(state.id);
        self.energy_master += crate::content::relics::energy_master_delta(state.id);

        self.relics.push(state);
        self.register_relic_subscriptions(index, sub);
    }

    fn register_relic_subscriptions(
        &mut self,
        index: usize,
        sub: crate::content::relics::RelicSubscriptions,
    ) {
        if sub.at_battle_start {
            self.relic_buses.at_battle_start.push(index);
        }
        if sub.at_turn_start {
            self.relic_buses.at_turn_start.push(index);
        }
        if sub.at_turn_start_post_draw {
            self.relic_buses.at_turn_start_post_draw.push(index);
        }
        if sub.on_use_card {
            self.relic_buses.on_use_card.push(index);
        }
        if sub.on_shuffle {
            self.relic_buses.on_shuffle.push(index);
        }
        if sub.on_exhaust {
            self.relic_buses.on_exhaust.push(index);
        }
        if sub.on_lose_hp {
            self.relic_buses.on_lose_hp.push(index);
        }
        if sub.on_victory {
            self.relic_buses.on_victory.push(index);
        }
        if sub.on_apply_power {
            self.relic_buses.on_apply_power.push(index);
        }
        if sub.on_monster_death {
            self.relic_buses.on_monster_death.push(index);
        }
        if sub.on_spawn_monster {
            self.relic_buses.on_spawn_monster.push(index);
        }
        if sub.at_end_of_turn {
            self.relic_buses.at_end_of_turn.push(index);
        }
        if sub.on_use_potion {
            self.relic_buses.on_use_potion.push(index);
        }
        if sub.on_discard {
            self.relic_buses.on_discard.push(index);
        }
        if sub.on_change_stance {
            self.relic_buses.on_change_stance.push(index);
        }
        if sub.on_attacked_to_change_damage {
            self.relic_buses.on_attacked_to_change_damage.push(index);
        }
        if sub.on_lose_hp_last {
            self.relic_buses.on_lose_hp_last.push(index);
        }

        if sub.on_calculate_heal {
            self.relic_buses.on_calculate_heal.push(index);
        }
        if sub.on_calculate_x_cost {
            self.relic_buses.on_calculate_x_cost.push(index);
        }
        if sub.on_calculate_block_retained {
            self.relic_buses.on_calculate_block_retained.push(index);
        }
        if sub.on_calculate_energy_retained {
            self.relic_buses.on_calculate_energy_retained.push(index);
        }
        if sub.on_scry {
            self.relic_buses.on_scry.push(index);
        }
        if sub.on_receive_power_modify {
            self.relic_buses.on_receive_power_modify.push(index);
        }
        if sub.on_calculate_vulnerable_multiplier {
            self.relic_buses
                .on_calculate_vulnerable_multiplier
                .push(index);
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub enum Intent {
    Attack { damage: i32, hits: u8 },
    AttackBuff { damage: i32, hits: u8 },
    AttackDebuff { damage: i32, hits: u8 },
    AttackDefend { damage: i32, hits: u8 },
    Buff,
    Debuff,
    StrongDebuff,
    Debug,
    Defend,
    DefendDebuff,
    DefendBuff,
    Escape,
    Magic,
    None,
    Sleep,
    Stun,
    Unknown,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MonsterEntity {
    pub id: EntityId,
    pub monster_type: MonsterId,
    pub current_hp: i32,
    pub max_hp: i32,
    pub block: i32,
    pub slot: u8,
    pub is_dying: bool,
    pub is_escaped: bool,
    pub half_dead: bool,
    pub next_move_byte: u8,
    pub current_intent: Intent,
    pub move_history: VecDeque<u8>,
    pub intent_dmg: i32,
    pub logical_position: i32,
    pub protocol_identity: MonsterProtocolIdentity,
    pub hexaghost: HexaghostRuntimeState,
    pub chosen: ChosenRuntimeState,
    pub darkling: DarklingRuntimeState,
    pub lagavulin: LagavulinRuntimeState,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MonsterProtocolIdentity {
    pub instance_id: Option<u64>,
    pub spawn_order: Option<u64>,
    pub draw_x: Option<i32>,
    pub group_index: Option<usize>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HexaghostRuntimeState {
    pub activated: bool,
    pub orb_active_count: u8,
    pub burn_upgraded: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChosenRuntimeState {
    pub protocol_seeded: bool,
    pub first_turn: bool,
    pub used_hex: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DarklingRuntimeState {
    pub first_move: bool,
    pub nip_dmg: i32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LagavulinRuntimeState {
    pub idle_count: u8,
    pub is_out_triggered: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CombatCard {
    pub id: CardId,
    pub uuid: u32,
    pub upgrades: u8,
    pub misc_value: i32,
    pub base_damage_override: Option<i32>,
    pub cost_modifier: i8,
    pub cost_for_turn: Option<u8>,
    pub base_damage_mut: i32,
    pub base_block_mut: i32,
    pub base_magic_num_mut: i32,
    pub multi_damage: smallvec::SmallVec<[i32; 5]>,
    pub exhaust_override: Option<bool>,
    pub retain_override: Option<bool>,
    pub free_to_play_once: bool,
    pub energy_on_use: i32,
}

impl CombatCard {
    pub fn new(id: CardId, uuid: u32) -> Self {
        Self {
            id,
            uuid,
            upgrades: 0,
            misc_value: 0,
            base_damage_override: None,
            cost_modifier: 0,
            cost_for_turn: None,
            base_damage_mut: 0,
            base_block_mut: 0,
            base_magic_num_mut: 0,
            multi_damage: smallvec::smallvec![],
            exhaust_override: None,
            retain_override: None,
            free_to_play_once: false,
            energy_on_use: 0,
        }
    }

    pub fn get_cost(&self) -> i8 {
        if let Some(c) = self.cost_for_turn {
            c as i8
        } else {
            let def = crate::content::cards::get_card_definition(self.id);
            let base_cost =
                crate::content::cards::upgraded_base_cost_override(self).unwrap_or(def.cost);
            if base_cost < 0 {
                return base_cost;
            }
            let mut c = base_cost as i8 + self.cost_modifier;
            if c < 0 {
                c = 0;
            }
            c
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Power {
    pub power_type: PowerId,
    pub instance_id: Option<u32>,
    pub amount: i32,
    pub extra_data: i32,
    pub just_applied: bool,
}

// Derived combat runtime state that is recomputed from other sources.
impl CombatState {
    pub fn recompute_turn_start_draw_modifier(&mut self) {
        let mut modifier = 0;
        if let Some(powers) = crate::content::powers::store::powers_for(self, 0) {
            for power in powers {
                match power.power_type {
                    PowerId::DrawReduction => modifier -= power.amount,
                    // Future DrawPower / similar effects should add their contribution here.
                    _ => {}
                }
            }
        }
        self.turn.turn_start_draw_modifier = modifier;
    }
}

// Card-zone utilities used by action handlers to reconcile card movement.
impl CombatState {
    /// Helper to find a card by UUID in a specific slice and remove it. Returns the removed card.
    pub fn remove_card_by_uuid(pile: &mut Vec<CombatCard>, uuid: u32) -> Option<CombatCard> {
        if let Some(index) = pile.iter().position(|c| c.uuid == uuid) {
            Some(pile.remove(index))
        } else {
            None
        }
    }

    /// Looks everywhere for a card and removes it. Useful for UseCard when we don't know exactly where the card went.
    pub fn take_card_from_anywhere(&mut self, uuid: u32) -> Option<CombatCard> {
        if let Some(c) = Self::remove_card_by_uuid(&mut self.zones.hand, uuid) {
            return Some(c);
        }
        if let Some(c) = Self::remove_card_by_uuid(&mut self.zones.limbo, uuid) {
            return Some(c);
        }
        if let Some(c) = Self::remove_card_by_uuid(&mut self.zones.draw_pile, uuid) {
            return Some(c);
        }
        if let Some(c) = Self::remove_card_by_uuid(&mut self.zones.discard_pile, uuid) {
            return Some(c);
        }
        if let Some(c) = Self::remove_card_by_uuid(&mut self.zones.exhaust_pile, uuid) {
            return Some(c);
        }
        None
    }
}

// Lightweight read helpers over combat-owned runtime state.
impl CombatState {
    /// Gets the current stack amount of a specific power on an entity
    pub fn get_power(&self, target: EntityId, power_id: PowerId) -> i32 {
        crate::content::powers::store::power_amount(self, target, power_id)
    }
}

// Queue-sensitive runtime helpers for Java cardQueue approximations.
impl CombatState {
    /// Best-effort approximation of Java's cardQueue membership for effects that
    /// should avoid already-queued cards (for example Mummified Hand).
    ///
    /// We do not model AbstractDungeon.actionManager.cardQueue explicitly, but cards
    /// already in limbo or already wrapped in queued play actions should not be treated
    /// as normal in-hand candidates.
    pub fn reserved_card_uuids_for_queue_sensitive_effects(&self) -> HashSet<u32> {
        let mut reserved = HashSet::new();
        for card in &self.zones.limbo {
            reserved.insert(card.uuid);
        }
        for queued in &self.zones.queued_cards {
            reserved.insert(queued.card.uuid);
        }
        for queued in &self.runtime.card_queue {
            reserved.insert(queued.card_uuid);
        }
        for action in &self.engine.action_queue {
            match action {
                Action::EnqueueCardPlay { item, .. } => {
                    reserved.insert(item.card.uuid);
                }
                Action::PlayCardDirect { card, .. } => {
                    reserved.insert(card.uuid);
                }
                Action::UseCard { uuid, .. } => {
                    reserved.insert(*uuid);
                }
                _ => {}
            }
        }
        reserved
    }

    pub fn enqueue_card_play(&mut self, item: QueuedCardPlay, in_front: bool) {
        let was_empty = self.zones.queued_cards.is_empty();
        if in_front {
            self.zones.queued_cards.push_front(item);
        } else {
            self.zones.queued_cards.push_back(item);
        }
        if was_empty {
            self.queue_action_back(Action::FlushNextQueuedCard);
        }
    }

    pub fn colorless_combat_pool(&self) -> Vec<CardId> {
        if !self.runtime.colorless_combat_pool.is_empty() {
            self.runtime.colorless_combat_pool.clone()
        } else {
            crate::content::cards::random_colorless_in_combat_pool()
        }
    }
}

// Hand re-evaluation helpers used after state-changing effects.
impl CombatState {
    /// Reparses all cards in the hand to dynamically calculate damage, block, and magic numbers.
    /// Clones the hand to satisfy borrow-checker while allowing `PerfectedStrike` to read `&self.hand`.
    pub fn update_hand_cards(&mut self) {
        let mut new_hand = self.zones.hand.clone();
        for card in &mut new_hand {
            crate::content::cards::evaluate_card(card, self, None);
        }
        self.zones.hand = new_hand;
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::action::ActionInfo;

    #[test]
    fn engine_runtime_queue_actions_preserves_top_before_bottom_order() {
        let mut engine = EngineRuntime {
            action_queue: VecDeque::new(),
        };

        engine.queue_actions(smallvec::smallvec![
            ActionInfo {
                action: Action::DrawCards(1),
                insertion_mode: AddTo::Bottom,
            },
            ActionInfo {
                action: Action::GainEnergy { amount: 2 },
                insertion_mode: AddTo::Top,
            },
            ActionInfo {
                action: Action::GainBlock {
                    target: 0,
                    amount: 3,
                },
                insertion_mode: AddTo::Top,
            },
        ]);

        assert_eq!(
            engine.pop_front(),
            Some(Action::GainEnergy { amount: 2 }),
            "first top action should execute first"
        );
        assert_eq!(
            engine.pop_front(),
            Some(Action::GainBlock {
                target: 0,
                amount: 3,
            }),
            "subsequent top actions should preserve insertion order"
        );
        assert_eq!(
            engine.pop_front(),
            Some(Action::DrawCards(1)),
            "bottom actions should remain after top actions"
        );
    }

    #[test]
    fn turn_runtime_begin_next_player_turn_resets_turn_counters_and_energy() {
        let mut turn = TurnRuntime {
            turn_count: 2,
            current_phase: CombatPhase::MonsterTurn,
            energy: 1,
            turn_start_draw_modifier: -1,
            counters: EphemeralCounters {
                cards_played_this_turn: 4,
                attacks_played_this_turn: 2,
                times_damaged_this_combat: 3,
                victory_triggered: false,
                discovery_cost_for_turn: None,
                early_end_turn_pending: true,
                player_escaping: false,
                escape_pending_reward: false,
            },
        };

        turn.begin_next_player_turn(3);

        assert_eq!(turn.turn_count, 3);
        assert_eq!(turn.current_phase, CombatPhase::PlayerTurn);
        assert_eq!(turn.energy, 3);
        assert_eq!(turn.counters.cards_played_this_turn, 0);
        assert_eq!(turn.counters.attacks_played_this_turn, 0);
        assert_eq!(
            turn.counters.times_damaged_this_combat, 3,
            "combat-wide counters should remain untouched"
        );
        assert!(
            turn.counters.early_end_turn_pending,
            "unrelated flags should not be reset by turn-start setup"
        );
        assert_eq!(turn.turn_start_draw_modifier, -1);
    }

    #[test]
    fn turn_runtime_counter_helpers_update_expected_flags() {
        let mut turn = TurnRuntime::fresh_player_turn(3);

        turn.mark_early_end_turn_pending();
        turn.mark_player_escaping();
        turn.mark_escape_pending_reward();
        turn.mark_victory_triggered();
        turn.increment_cards_played();
        turn.increment_attacks_played();
        turn.increment_times_damaged_this_combat();
        turn.set_discovery_cost_for_turn(Some(1));

        assert!(turn.counters.early_end_turn_pending);
        assert!(turn.counters.player_escaping);
        assert!(turn.counters.escape_pending_reward);
        assert!(turn.counters.victory_triggered);
        assert_eq!(turn.counters.cards_played_this_turn, 1);
        assert_eq!(turn.counters.attacks_played_this_turn, 1);
        assert_eq!(turn.counters.times_damaged_this_combat, 1);
        assert_eq!(turn.take_discovery_cost_for_turn(), Some(1));
        assert_eq!(turn.take_discovery_cost_for_turn(), None);
    }

    #[test]
    fn turn_runtime_can_clear_transition_flags_without_touching_other_counters() {
        let mut turn = TurnRuntime::fresh_player_turn(3);
        turn.mark_early_end_turn_pending();
        turn.mark_escape_pending_reward();
        turn.increment_times_damaged_this_combat();

        turn.clear_early_end_turn_pending();
        turn.clear_escape_pending_reward();

        assert!(!turn.counters.early_end_turn_pending);
        assert!(!turn.counters.escape_pending_reward);
        assert_eq!(turn.counters.times_damaged_this_combat, 1);
    }
}
