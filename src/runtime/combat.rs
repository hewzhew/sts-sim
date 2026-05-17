use crate::content::cards::CardId;
use crate::content::relics::RelicState;
use crate::core::EntityId;
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::semantics::combat::{AttackSpec, DamageKind, MonsterMoveSpec, MonsterTurnPlan};
use crate::state::selection::{DomainEvent, EngineDiagnostic};
use std::collections::{HashMap, HashSet, VecDeque};
use std::ops::{Deref, DerefMut};

#[derive(Clone, Debug, PartialEq)]
pub enum MetaChange {
    AddCardToMasterDeck(CardId),
    ModifyCardMisc { card_uuid: u32, amount: i32 },
    UpgradeMasterDeckCard { card_uuid: u32 },
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
    pub master_deck_snapshot: Vec<CombatCard>,
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

impl CardZones {
    /// Internal Rust draw-pile invariant: index 0 is the next card drawn.
    /// Java CardGroup stores the draw-pile top at the end of the list, so all
    /// Java addToTop/addToBottom/addToRandomSpot semantics must pass through
    /// these helpers instead of writing `draw_pile` indices directly.
    pub fn add_to_draw_pile_top(&mut self, card: CombatCard) {
        self.draw_pile.insert(0, card);
    }

    pub fn add_to_draw_pile_bottom(&mut self, card: CombatCard) {
        self.draw_pile.push(card);
    }

    pub fn draw_top_card(&mut self) -> Option<CombatCard> {
        if self.draw_pile.is_empty() {
            None
        } else {
            Some(self.draw_pile.remove(0))
        }
    }

    pub fn add_to_draw_pile_random_spot_from_java_index(
        &mut self,
        card: CombatCard,
        java_insert_index: usize,
    ) {
        let len = self.draw_pile.len();
        if len == 0 {
            self.draw_pile.push(card);
            return;
        }
        let rust_insert_index = len.saturating_sub(java_insert_index.min(len - 1));
        self.draw_pile.insert(rust_insert_index, card);
    }

    /// Internal Rust discard-pile invariant intentionally preserves Java
    /// CardGroup order: index 0 is the bottom and the last element is the top.
    /// This differs from Rust draw_pile because Java shuffles discardPile.group
    /// in this order before moving cards back to the draw pile.
    pub fn add_to_discard_pile_top(&mut self, card: CombatCard) {
        self.discard_pile.push(card);
    }

    /// Java exhaustPile is also a CardGroup: addToTop appends to the end.
    /// Preserve that internal order here for parity with any later selection
    /// or replay that observes exhaust-pile group order.
    pub fn add_to_exhaust_pile_top(&mut self, card: CombatCard) {
        self.exhaust_pile.push(card);
    }

    /// Applies a mutation to every visible Java battle instance that shares a
    /// UUID. This is the common path for effects such as Rampage/Searing Blow
    /// growth that call AbstractDungeon.player.masterDeck.getSpecificCard()
    /// plus GetAllInBattleInstances-like combat copies.
    pub fn for_each_java_battle_instance_mut_by_uuid(
        &mut self,
        card_uuid: u32,
        mut apply: impl FnMut(&mut CombatCard),
    ) {
        for card in self
            .hand
            .iter_mut()
            .chain(self.draw_pile.iter_mut())
            .chain(self.discard_pile.iter_mut())
            .chain(self.exhaust_pile.iter_mut())
            .chain(self.limbo.iter_mut())
        {
            if card.uuid == card_uuid {
                apply(card);
            }
        }
    }

    /// Queued card plays are a Rust runtime artifact rather than a Java
    /// CardGroup, but some delayed play paths carry stat-equivalent copies
    /// there before execution. Use this only for mutations that must preserve
    /// existing queued-card behavior.
    pub fn for_each_queued_instance_mut_by_uuid(
        &mut self,
        card_uuid: u32,
        mut apply: impl FnMut(&mut CombatCard),
    ) {
        for queued in &mut self.queued_cards {
            if queued.card.uuid == card_uuid {
                apply(&mut queued.card);
            }
        }
    }
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
    Amplify,
    EchoForm,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct DrawnCardRecord {
    pub card_uuid: u32,
    pub card_id: CardId,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CombatRuntimeHints {
    pub using_card: bool,
    pub card_queue: Vec<QueuedCardHint>,
    pub colorless_combat_pool: Vec<CardId>,
    pub emitted_events: Vec<DomainEvent>,
    pub engine_diagnostics: Vec<EngineDiagnostic>,
    pub pending_rewards: Vec<crate::rewards::state::RewardItem>,
    pub power_instance_counter: u32,
    /// Narrow Rust equivalent of Java `DrawCardAction.drawnCards`.
    ///
    /// It is updated only by draw actions that explicitly opt into draw
    /// history because only Java follow-up actions should observe it.
    pub last_drawn_cards: Vec<DrawnCardRecord>,
    pub monster_protocol: HashMap<EntityId, MonsterProtocolState>,
    /// Java `AbstractRoom.mugged` equivalent at combat scope.
    ///
    /// Set when a thief escapes after actually stealing gold. This is not used
    /// for reward contents, only for post-combat presentation/state.
    pub combat_mugged: bool,
    /// Java `AbstractRoom.smoked` equivalent at combat scope.
    ///
    /// Set when Smoke Bomb ends combat. This changes reward-screen behavior:
    /// no normal combat rewards should be generated.
    pub combat_smoked: bool,
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
    pub cards_discarded_this_turn: u16,
    pub card_ids_played_this_turn: Vec<CardId>,
    pub card_ids_played_this_combat: Vec<CardId>,
    pub orbs_channeled_this_turn: Vec<OrbId>,
    pub orbs_channeled_this_combat: Vec<OrbId>,
    pub mantra_gained_this_combat: i32,
    pub times_damaged_this_combat: u8,
    pub victory_triggered: bool,
    pub discovery_cost_for_turn: Option<u8>,
    pub early_end_turn_pending: bool,
    pub skip_monster_turn_pending: bool,
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
    pub fn monster_protocol(&self, monster_id: EntityId) -> Option<&MonsterProtocolState> {
        self.runtime.monster_protocol.get(&monster_id)
    }

    pub fn monster_protocol_mut(&mut self, monster_id: EntityId) -> &mut MonsterProtocolState {
        self.runtime.monster_protocol.entry(monster_id).or_default()
    }

    pub fn clear_monster_protocol_observation(&mut self, monster_id: EntityId) {
        self.monster_protocol_mut(monster_id).observation =
            MonsterProtocolObservationState::default();
    }

    pub fn set_monster_protocol_visible_intent(&mut self, monster_id: EntityId, intent: Intent) {
        self.monster_protocol_mut(monster_id)
            .observation
            .visible_intent = intent;
    }

    pub fn set_monster_protocol_preview_damage_per_hit(
        &mut self,
        monster_id: EntityId,
        damage: i32,
    ) {
        self.monster_protocol_mut(monster_id)
            .observation
            .preview_damage_per_hit = damage;
    }

    pub fn monster_protocol_visible_intent(&self, monster_id: EntityId) -> &Intent {
        self.monster_protocol(monster_id)
            .map(|state| &state.observation.visible_intent)
            .unwrap_or(&Intent::Unknown)
    }

    pub fn monster_protocol_preview_damage_per_hit(&self, monster_id: EntityId) -> i32 {
        self.monster_protocol(monster_id)
            .map(|state| state.observation.preview_damage_per_hit)
            .unwrap_or(0)
    }

    pub fn monster_has_protocol_visible_intent(&self, monster_id: EntityId) -> bool {
        !matches!(
            self.monster_protocol_visible_intent(monster_id),
            Intent::Unknown
        )
    }

    pub fn monster_protocol_identity(
        &self,
        monster_id: EntityId,
    ) -> Option<&MonsterProtocolIdentity> {
        self.monster_protocol(monster_id)
            .map(|state| &state.identity)
    }

    pub fn monster_protocol_identity_mut(
        &mut self,
        monster_id: EntityId,
    ) -> &mut MonsterProtocolIdentity {
        &mut self.monster_protocol_mut(monster_id).identity
    }

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

    pub fn clear_post_combat_actions(&mut self) {
        self.engine
            .retain(|action| action.retained_by_java_clear_post_combat_actions());
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
        let energy = if crate::content::relics::hooks::on_calculate_energy_retained(self) {
            self.turn
                .energy
                .saturating_add(self.entities.player.energy_master)
        } else {
            self.entities.player.energy_master
        };
        self.turn.begin_next_player_turn(energy);
    }

    pub fn reset_turn_energy_from_player(&mut self) {
        self.turn.set_energy(self.entities.player.energy_master);
        self.recompute_turn_start_draw_modifier();
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

    pub fn record_card_played(&mut self, card_id: CardId) {
        self.increment_cards_played();
        self.counters.card_ids_played_this_turn.push(card_id);
        self.counters.card_ids_played_this_combat.push(card_id);
    }

    pub fn record_orb_channeled(&mut self, orb_id: OrbId) {
        self.counters.orbs_channeled_this_turn.push(orb_id);
        self.counters.orbs_channeled_this_combat.push(orb_id);
    }

    pub fn increment_attacks_played(&mut self) {
        self.counters.attacks_played_this_turn += 1;
    }

    pub fn increment_cards_discarded(&mut self) {
        self.counters.cards_discarded_this_turn =
            self.counters.cards_discarded_this_turn.saturating_add(1);
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

    pub fn mark_skip_monster_turn_pending(&mut self) {
        self.counters.skip_monster_turn_pending = true;
    }

    pub fn clear_skip_monster_turn_pending(&mut self) {
        self.counters.skip_monster_turn_pending = false;
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
        self.counters.cards_discarded_this_turn = 0;
        self.counters.card_ids_played_this_turn.clear();
        self.counters.orbs_channeled_this_turn.clear();
        self.counters.skip_monster_turn_pending = false;
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
        // `ActionInfo` order is the Java call order.  Java `addToTop`
        // inserts at index 0 immediately, so later top insertions run before
        // earlier top insertions.
        for a in actions {
            match a.insertion_mode {
                AddTo::Top => self.push_front(a.action),
                AddTo::Bottom => self.push_back(a.action),
            }
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
    pub base_passive_amount: i32,
    pub base_evoke_amount: i32,
    pub passive_amount: i32,
    pub evoke_amount: i32,
}

impl OrbEntity {
    pub fn new(id: OrbId) -> Self {
        match id {
            OrbId::Empty => OrbEntity {
                id,
                base_passive_amount: 0,
                base_evoke_amount: 0,
                passive_amount: 0,
                evoke_amount: 0,
            },
            OrbId::Lightning => OrbEntity {
                id,
                base_passive_amount: 3,
                base_evoke_amount: 8,
                passive_amount: 3,
                evoke_amount: 8,
            },
            OrbId::Dark => OrbEntity {
                id,
                base_passive_amount: 6,
                base_evoke_amount: 6,
                passive_amount: 6,
                evoke_amount: 6,
            },
            OrbId::Frost => OrbEntity {
                id,
                base_passive_amount: 2,
                base_evoke_amount: 5,
                passive_amount: 2,
                evoke_amount: 5,
            },
            OrbId::Plasma => OrbEntity {
                id,
                base_passive_amount: 1,
                base_evoke_amount: 2,
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
        if sub.at_pre_battle {
            self.relic_buses.at_pre_battle.push(index);
        }
        if sub.at_battle_start_pre_draw {
            self.relic_buses.at_battle_start_pre_draw.push(index);
        }
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

impl Intent {
    pub fn is_java_attack_intent(&self) -> bool {
        matches!(
            self,
            Intent::Attack { .. }
                | Intent::AttackBuff { .. }
                | Intent::AttackDebuff { .. }
                | Intent::AttackDefend { .. }
        )
    }

    pub fn base_damage(&self) -> Option<i32> {
        match self {
            Intent::Attack { damage, .. }
            | Intent::AttackBuff { damage, .. }
            | Intent::AttackDebuff { damage, .. }
            | Intent::AttackDefend { damage, .. } => Some(*damage),
            _ => None,
        }
    }

    pub fn hits(&self) -> i32 {
        match self {
            Intent::Attack { hits, .. }
            | Intent::AttackBuff { hits, .. }
            | Intent::AttackDebuff { hits, .. }
            | Intent::AttackDefend { hits, .. } => (*hits as i32).max(1),
            _ => 0,
        }
    }

    /// Legacy protocol/old-monster bridge only.
    ///
    /// Semantic main paths must not derive move truth from `Intent`.
    pub fn to_legacy_move_spec(&self) -> MonsterMoveSpec {
        match self {
            Intent::Attack { damage, hits } => MonsterMoveSpec::Attack(AttackSpec {
                base_damage: *damage,
                hits: *hits,
                damage_kind: DamageKind::Normal,
            }),
            Intent::AttackBuff { .. }
            | Intent::AttackDebuff { .. }
            | Intent::AttackDefend { .. }
            | Intent::Buff
            | Intent::Debuff
            | Intent::StrongDebuff
            | Intent::Defend
            | Intent::DefendDebuff
            | Intent::DefendBuff => MonsterMoveSpec::Unknown,
            Intent::Debug => MonsterMoveSpec::Debug,
            Intent::Escape => MonsterMoveSpec::Escape,
            Intent::Magic => MonsterMoveSpec::Magic,
            Intent::None => MonsterMoveSpec::None,
            Intent::Sleep => MonsterMoveSpec::Sleep,
            Intent::Stun => MonsterMoveSpec::Stun,
            Intent::Unknown => MonsterMoveSpec::Unknown,
        }
    }
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
    pub move_state: MonsterMoveState,
    pub logical_position: i32,
    pub hexaghost: HexaghostRuntimeState,
    pub louse: LouseRuntimeState,
    pub jaw_worm: JawWormRuntimeState,
    pub thief: ThiefRuntimeState,
    pub byrd: ByrdRuntimeState,
    pub chosen: ChosenRuntimeState,
    pub snecko: SneckoRuntimeState,
    pub shelled_parasite: ShelledParasiteRuntimeState,
    pub bronze_automaton: BronzeAutomatonRuntimeState,
    pub bronze_orb: BronzeOrbRuntimeState,
    pub book_of_stabbing: BookOfStabbingRuntimeState,
    pub collector: CollectorRuntimeState,
    pub champ: ChampRuntimeState,
    pub awakened_one: AwakenedOneRuntimeState,
    pub corrupt_heart: CorruptHeartRuntimeState,
    pub writhing_mass: WrithingMassRuntimeState,
    pub spiker: SpikerRuntimeState,
    pub spire_shield: SpireShieldRuntimeState,
    pub spire_spear: SpireSpearRuntimeState,
    pub slaver_red: SlaverRedRuntimeState,
    pub gremlin_leader: GremlinLeaderRuntimeState,
    pub gremlin_nob: GremlinNobRuntimeState,
    pub gremlin_wizard: GremlinWizardRuntimeState,
    pub cultist: CultistRuntimeState,
    pub sentry: SentryRuntimeState,
    pub slime_boss: SlimeBossRuntimeState,
    pub large_slime: LargeSlimeRuntimeState,
    pub spheric_guardian: SphericGuardianRuntimeState,
    pub reptomancer: ReptomancerRuntimeState,
    pub darkling: DarklingRuntimeState,
    pub nemesis: NemesisRuntimeState,
    pub giant_head: GiantHeadRuntimeState,
    pub time_eater: TimeEaterRuntimeState,
    pub donu: DonuRuntimeState,
    pub deca: DecaRuntimeState,
    pub transient: TransientRuntimeState,
    pub lagavulin: LagavulinRuntimeState,
    pub guardian: GuardianRuntimeState,
}

impl MonsterEntity {
    pub fn is_dead_or_escaped(&self) -> bool {
        self.is_dying || self.half_dead || self.is_escaped
    }

    pub fn is_alive_for_action(&self) -> bool {
        self.current_hp > 0 && !self.is_dead_or_escaped()
    }

    /// Java `MonsterGroup.getRandomMonster(..., aliveOnly=true, cardRandomRng)`
    /// filters out half-dead, dying, and escaping monsters. It does not check
    /// `currentHealth`, because subsequent actions own their own cancellation.
    pub fn is_random_target_candidate(&self) -> bool {
        !self.half_dead && !self.is_dying && !self.is_escaped
    }

    pub fn turn_plan(&self) -> MonsterTurnPlan {
        let move_id = self.planned_move_id();
        if self.is_dying || self.half_dead {
            return MonsterTurnPlan::unknown(move_id);
        }

        MonsterTurnPlan {
            move_id,
            steps: self.move_state.planned_steps.clone().unwrap_or_default(),
            visible_spec: self.move_state.planned_visible_spec.clone(),
        }
    }

    pub fn planned_move_id(&self) -> u8 {
        self.move_state.planned_move_id
    }

    pub fn set_planned_move_id(&mut self, move_id: u8) {
        self.move_state.planned_move_id = move_id;
    }

    pub fn set_planned_steps(&mut self, steps: crate::semantics::combat::MonsterTurnSteps) {
        self.move_state.planned_steps = Some(steps);
    }

    pub fn set_planned_visible_spec(
        &mut self,
        visible_spec: Option<crate::semantics::combat::MonsterMoveSpec>,
    ) {
        self.move_state.planned_visible_spec = visible_spec;
    }

    pub fn record_planned_move(&mut self, move_id: u8) {
        self.move_state.planned_move_id = move_id;
        self.move_state.history.push_back(move_id);
    }

    pub fn move_history(&self) -> &VecDeque<u8> {
        &self.move_state.history
    }

    pub fn move_history_mut(&mut self) -> &mut VecDeque<u8> {
        &mut self.move_state.history
    }

    pub fn louse_bite_damage(&self) -> Option<i32> {
        self.louse.bite_damage
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MonsterMoveState {
    pub planned_move_id: u8,
    pub history: VecDeque<u8>,
    pub planned_steps: Option<crate::semantics::combat::MonsterTurnSteps>,
    pub planned_visible_spec: Option<crate::semantics::combat::MonsterMoveSpec>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MonsterProtocolIdentity {
    pub instance_id: Option<u64>,
    pub spawn_order: Option<u64>,
    pub draw_x: Option<i32>,
    pub group_index: Option<usize>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct MonsterProtocolObservationState {
    pub visible_intent: Intent,
    /// UI / protocol preview damage after monster damage modifiers are applied.
    /// This is not an executable damage base and must not be fed back into
    /// combat resolution.
    pub preview_damage_per_hit: i32,
}

impl Default for MonsterProtocolObservationState {
    fn default() -> Self {
        Self {
            visible_intent: Intent::Unknown,
            preview_damage_per_hit: 0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct MonsterProtocolState {
    pub observation: MonsterProtocolObservationState,
    pub identity: MonsterProtocolIdentity,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct HexaghostRuntimeState {
    pub activated: bool,
    pub orb_active_count: u8,
    pub burn_upgraded: bool,
    pub divider_damage: Option<i32>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LouseRuntimeState {
    pub bite_damage: Option<i32>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct JawWormRuntimeState {
    pub protocol_seeded: bool,
    pub first_move: bool,
    pub hard_mode: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ThiefRuntimeState {
    pub protocol_seeded: bool,
    pub slash_count: u8,
    pub stolen_gold: i32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ByrdRuntimeState {
    pub protocol_seeded: bool,
    pub first_move: bool,
    pub is_flying: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ChosenRuntimeState {
    pub protocol_seeded: bool,
    pub first_turn: bool,
    pub used_hex: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SneckoRuntimeState {
    pub protocol_seeded: bool,
    pub first_turn: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ShelledParasiteRuntimeState {
    pub protocol_seeded: bool,
    pub first_move: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BronzeAutomatonRuntimeState {
    pub protocol_seeded: bool,
    pub first_turn: bool,
    pub num_turns: u8,
}

impl Default for BronzeAutomatonRuntimeState {
    fn default() -> Self {
        Self {
            protocol_seeded: false,
            first_turn: true,
            num_turns: 0,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BronzeOrbRuntimeState {
    pub protocol_seeded: bool,
    pub used_stasis: bool,
}

impl Default for BronzeOrbRuntimeState {
    fn default() -> Self {
        Self {
            protocol_seeded: false,
            used_stasis: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BookOfStabbingRuntimeState {
    pub protocol_seeded: bool,
    pub stab_count: u8,
}

impl Default for BookOfStabbingRuntimeState {
    fn default() -> Self {
        Self {
            protocol_seeded: false,
            stab_count: 1,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CollectorRuntimeState {
    pub protocol_seeded: bool,
    pub initial_spawn: bool,
    pub ult_used: bool,
    pub turns_taken: u8,
    pub enemy_slots: [Option<EntityId>; 2],
}

impl Default for CollectorRuntimeState {
    fn default() -> Self {
        Self {
            protocol_seeded: false,
            initial_spawn: true,
            ult_used: false,
            turns_taken: 0,
            enemy_slots: [None, None],
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct ChampRuntimeState {
    pub protocol_seeded: bool,
    pub first_turn: bool,
    pub num_turns: u8,
    pub forge_times: u8,
    pub threshold_reached: bool,
}

impl Default for ChampRuntimeState {
    fn default() -> Self {
        Self {
            protocol_seeded: false,
            first_turn: true,
            num_turns: 0,
            forge_times: 0,
            threshold_reached: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct AwakenedOneRuntimeState {
    pub protocol_seeded: bool,
    pub form1: bool,
    pub first_turn: bool,
}

impl Default for AwakenedOneRuntimeState {
    fn default() -> Self {
        Self {
            protocol_seeded: false,
            form1: true,
            first_turn: true,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CorruptHeartRuntimeState {
    pub protocol_seeded: bool,
    pub first_move: bool,
    pub move_count: u8,
    pub buff_count: u8,
}

impl Default for CorruptHeartRuntimeState {
    fn default() -> Self {
        Self {
            protocol_seeded: false,
            first_move: true,
            move_count: 0,
            buff_count: 0,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct WrithingMassRuntimeState {
    pub protocol_seeded: bool,
    pub used_mega_debuff: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpikerRuntimeState {
    pub protocol_seeded: bool,
    pub thorns_count: u8,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpireShieldRuntimeState {
    pub protocol_seeded: bool,
    pub move_count: u8,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SpireSpearRuntimeState {
    pub protocol_seeded: bool,
    pub move_count: u8,
}

#[derive(Clone, Debug, PartialEq)]
pub struct SlaverRedRuntimeState {
    pub protocol_seeded: bool,
    pub first_turn: bool,
    pub used_entangle: bool,
}

impl Default for SlaverRedRuntimeState {
    fn default() -> Self {
        Self {
            protocol_seeded: false,
            first_turn: true,
            used_entangle: false,
        }
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GremlinLeaderRuntimeState {
    pub protocol_seeded: bool,
    pub gremlin_slots: [Option<EntityId>; 3],
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GremlinNobRuntimeState {
    pub protocol_seeded: bool,
    pub used_bellow: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GremlinWizardRuntimeState {
    pub protocol_seeded: bool,
    pub current_charge: u8,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CultistRuntimeState {
    pub protocol_seeded: bool,
    pub first_move: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SentryRuntimeState {
    pub protocol_seeded: bool,
    pub first_move: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SlimeBossRuntimeState {
    pub protocol_seeded: bool,
    pub first_turn: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LargeSlimeRuntimeState {
    pub protocol_seeded: bool,
    pub split_triggered: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct SphericGuardianRuntimeState {
    pub protocol_seeded: bool,
    pub first_move: bool,
    pub second_move: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReptomancerRuntimeState {
    pub protocol_seeded: bool,
    pub first_move: bool,
    pub dagger_slots: [Option<EntityId>; 4],
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DarklingRuntimeState {
    pub protocol_seeded: bool,
    pub first_move: bool,
    pub nip_dmg: i32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct NemesisRuntimeState {
    pub protocol_seeded: bool,
    pub first_move: bool,
    pub scythe_cooldown: i32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct GiantHeadRuntimeState {
    pub protocol_seeded: bool,
    pub count: i32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TimeEaterRuntimeState {
    pub protocol_seeded: bool,
    pub used_haste: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DonuRuntimeState {
    pub protocol_seeded: bool,
    pub is_attacking: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct DecaRuntimeState {
    pub protocol_seeded: bool,
    pub is_attacking: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct TransientRuntimeState {
    pub protocol_seeded: bool,
    pub count: i32,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct LagavulinRuntimeState {
    pub is_out: bool,
    pub idle_count: u8,
    pub debuff_turn_count: u8,
    pub is_out_triggered: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub struct GuardianRuntimeState {
    pub damage_threshold: i32,
    pub damage_taken: i32,
    pub is_open: bool,
    pub close_up_triggered: bool,
}

impl Default for GuardianRuntimeState {
    fn default() -> Self {
        Self {
            damage_threshold: 0,
            damage_taken: 0,
            is_open: true,
            close_up_triggered: false,
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct CombatCard {
    pub id: CardId,
    pub uuid: u32,
    pub upgrades: u8,
    pub misc_value: i32,
    pub base_damage_override: Option<i32>,
    pub base_block_override: Option<i32>,
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
        let misc_value = match id {
            CardId::RitualDagger => 15,
            CardId::GeneticAlgorithm => 1,
            _ => 0,
        };
        Self {
            id,
            uuid,
            upgrades: 0,
            misc_value,
            base_damage_override: None,
            base_block_override: None,
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

    /// Java `AbstractCard.makeStatEquivalentCopy()` preserves card identity
    /// state such as upgrades, misc, base damage mutation, cost-for-turn, and
    /// free-to-play state, but it does not preserve transient calculated
    /// damage/block/magic/multi-damage or queued play metadata.
    pub fn make_stat_equivalent_copy_with_uuid(&self, uuid: u32) -> Self {
        let mut card = self.clone();
        card.uuid = uuid;
        card.base_damage_mut = 0;
        card.base_block_mut = 0;
        card.base_magic_num_mut = 0;
        card.multi_damage.clear();
        card.exhaust_override = None;
        card.retain_override = None;
        card.energy_on_use = 0;
        card
    }

    /// Java `AbstractCard.resetAttributes()` restores transient rendered values
    /// and resets `costForTurn` back to the combat cost. It does not clear
    /// persistent combat cost changes or `freeToPlayOnce`.
    pub fn reset_attributes_java(&mut self) {
        self.base_damage_mut = 0;
        self.base_block_mut = 0;
        self.base_magic_num_mut = 0;
        self.multi_damage.clear();
        self.cost_for_turn = None;
        self.exhaust_override = None;
        self.retain_override = None;
        self.energy_on_use = 0;
    }

    /// Java `AbstractCard.makeSameInstanceOf()` is a stat-equivalent copy with
    /// the original UUID restored. Replay effects such as Double Tap, Burst,
    /// Duplication Potion, and Necronomicon use this path.
    pub fn make_same_instance_of_java(&self) -> Self {
        self.make_stat_equivalent_copy_with_uuid(self.uuid)
    }

    pub fn get_cost(&self) -> i8 {
        if let Some(c) = self.cost_for_turn {
            c as i8
        } else {
            self.combat_cost_without_turn_override_java()
                .clamp(i8::MIN as i32, i8::MAX as i32) as i8
        }
    }

    pub fn base_cost_for_combat_java(&self) -> i32 {
        let def = crate::content::cards::get_card_definition(self.id);
        crate::content::cards::upgraded_base_cost_override(self).unwrap_or(def.cost) as i32
    }

    /// Java `AbstractCard.cost`: the combat copy's actual cost after
    /// cost-modifying effects, before `costForTurn` overrides.
    pub fn combat_cost_without_turn_override_java(&self) -> i32 {
        let base_cost = self.base_cost_for_combat_java();
        if base_cost < 0 {
            return base_cost;
        }
        (base_cost + self.cost_modifier as i32).max(0)
    }

    /// Java `AbstractCard.costForTurn`: the visible playable cost for this
    /// turn, falling back to the combat cost when no temporary override exists
    /// in Rust.
    pub fn cost_for_turn_java(&self) -> i32 {
        self.cost_for_turn
            .map(i32::from)
            .unwrap_or_else(|| self.combat_cost_without_turn_override_java())
    }

    pub fn set_cost_for_turn_java(&mut self, amount: i32) {
        if self.cost_for_turn_java() >= 0 {
            self.cost_for_turn = Some(clamp_turn_cost(amount));
        }
    }

    /// Mirrors Java `AbstractCard.updateCost(int)`: adjust combat cost and
    /// preserve any existing difference between `cost` and `costForTurn`.
    pub fn update_cost_java(&mut self, amount: i32) {
        let def = crate::content::cards::get_card_definition(self.id);
        if (def.card_type == crate::content::cards::CardType::Status && self.id != CardId::Slimed)
            || (def.card_type == crate::content::cards::CardType::Curse && self.id != CardId::Pride)
        {
            return;
        }

        let old_cost = self.combat_cost_without_turn_override_java();
        if old_cost < 0 {
            return;
        }
        let old_cost_for_turn = self.cost_for_turn_java();
        let cost_for_turn_diff = old_cost - old_cost_for_turn;
        let new_cost = (old_cost + amount).max(0);
        if new_cost == old_cost {
            return;
        }

        self.set_combat_cost_without_turn_override_java(new_cost);
        if self.cost_for_turn.is_some() || cost_for_turn_diff != 0 {
            self.cost_for_turn = Some(clamp_turn_cost(new_cost - cost_for_turn_diff));
        }
    }

    /// Mirrors Java `AbstractCard.modifyCostForCombat(int)`, used by effects
    /// such as Madness that mutate this combat copy's cost and visible
    /// cost-for-turn together.
    pub fn modify_cost_for_combat_java(&mut self, amount: i32) {
        let old_cost = self.combat_cost_without_turn_override_java();
        if old_cost < 0 {
            return;
        }

        let old_cost_for_turn = self.cost_for_turn_java();
        if old_cost_for_turn > 0 {
            let new_cost = (old_cost_for_turn + amount).max(0);
            self.set_combat_cost_without_turn_override_java(new_cost);
            self.cost_for_turn = Some(clamp_turn_cost(new_cost));
        } else {
            let new_cost = (old_cost + amount).max(0);
            self.set_combat_cost_without_turn_override_java(new_cost);
            self.cost_for_turn = Some(0);
        }
    }

    pub fn set_combat_cost_preserving_turn_java(&mut self, new_cost: i32) {
        if self.base_cost_for_combat_java() >= 0 {
            self.set_combat_cost_without_turn_override_java(new_cost.max(0));
        }
    }

    pub fn set_combat_and_turn_cost_java(&mut self, new_cost: i32) {
        if self.base_cost_for_combat_java() >= 0 {
            let new_cost = new_cost.max(0);
            self.set_combat_cost_without_turn_override_java(new_cost);
            self.cost_for_turn = Some(clamp_turn_cost(new_cost));
        }
    }

    fn set_combat_cost_without_turn_override_java(&mut self, new_cost: i32) {
        let base_cost = self.base_cost_for_combat_java();
        if base_cost < 0 {
            return;
        }
        self.cost_modifier = clamp_cost_modifier(new_cost - base_cost);
    }
}

fn clamp_turn_cost(cost: i32) -> u8 {
    cost.clamp(0, u8::MAX as i32) as u8
}

fn clamp_cost_modifier(modifier: i32) -> i8 {
    modifier.clamp(i8::MIN as i32, i8::MAX as i32) as i8
}

#[derive(Clone, Debug, Default, PartialEq)]
pub enum PowerPayload {
    #[default]
    None,
    Card(CombatCard),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Power {
    pub power_type: PowerId,
    pub instance_id: Option<u32>,
    pub amount: i32,
    pub extra_data: i32,
    pub payload: PowerPayload,
    pub just_applied: bool,
}

// Derived combat runtime state that is recomputed from other sources.
impl CombatState {
    pub fn recompute_turn_start_draw_modifier(&mut self) {
        let mut modifier = 0;
        if self
            .entities
            .player
            .has_relic(crate::content::relics::RelicId::RingOfTheSerpent)
        {
            modifier += 1;
        }
        if let Some(powers) = crate::content::powers::store::powers_for(self, 0) {
            for power in powers {
                match power.power_type {
                    PowerId::Draw => modifier += power.amount,
                    PowerId::DrawReduction => modifier -= power.amount,
                    _ => {}
                }
            }
        }
        self.turn.turn_start_draw_modifier = modifier;
    }
}

// Card-zone utilities used by action handlers to reconcile card movement.
impl CombatState {
    pub fn next_card_uuid(&mut self) -> u32 {
        self.zones.card_uuid_counter += 1;
        self.zones.card_uuid_counter
    }

    pub fn next_power_instance_id(&mut self) -> u32 {
        self.runtime.power_instance_counter += 1;
        self.runtime.power_instance_counter
    }

    /// Java `MonsterGroup.areMonstersBasicallyDead()` only skips monsters that
    /// are `isDying` or `isEscaping`. It does not check current HP and does not
    /// treat `halfDead` as basically dead by itself.
    pub fn are_monsters_basically_dead_java(&self) -> bool {
        self.entities
            .monsters
            .iter()
            .all(|m| m.is_dying || m.is_escaped)
    }

    /// Java `MonsterGroup.haveMonstersEscaped()` returns true only when every
    /// monster has its `escaped` flag set. Dying/dead monsters do not count.
    pub fn have_monsters_escaped_java(&self) -> bool {
        self.entities.monsters.iter().all(|m| m.is_escaped)
    }

    pub fn add_card_to_draw_pile_top(&mut self, card: CombatCard) {
        self.zones.add_to_draw_pile_top(card);
    }

    pub fn add_card_to_draw_pile_bottom(&mut self, card: CombatCard) {
        self.zones.add_to_draw_pile_bottom(card);
    }

    pub fn add_card_to_draw_pile_random_spot(&mut self, card: CombatCard) {
        let java_insert_index = if self.zones.draw_pile.is_empty() {
            0
        } else {
            self.rng
                .card_random_rng
                .random(self.zones.draw_pile.len() as i32 - 1) as usize
        };
        self.zones
            .add_to_draw_pile_random_spot_from_java_index(card, java_insert_index);
    }

    pub fn draw_top_card(&mut self) -> Option<CombatCard> {
        self.zones.draw_top_card()
    }

    pub fn add_card_to_discard_pile_top(&mut self, card: CombatCard) {
        self.zones.add_to_discard_pile_top(card);
    }

    pub fn add_card_to_exhaust_pile_top(&mut self, card: CombatCard) {
        self.zones.add_to_exhaust_pile_top(card);
    }

    pub fn shuffle_discard_pile_into_draw_pile(&mut self) {
        self.zones.draw_pile.append(&mut self.zones.discard_pile);
        crate::runtime::rng::shuffle_with_random_long(
            &mut self.zones.draw_pile,
            &mut self.rng.shuffle_rng,
        );
        // Java draw-pile top is the end of CardGroup.group. Rust draw-pile top
        // is index 0, so reverse only after preserving Java shuffle order.
        self.zones.draw_pile.reverse();
    }

    pub fn apply_java_initialize_deck_order_after_shuffle(&mut self) {
        let bottled_uuids: Vec<u32> = self
            .entities
            .player
            .relics
            .iter()
            .filter_map(|relic| match relic.id {
                crate::content::relics::RelicId::BottledFlame
                | crate::content::relics::RelicId::BottledLightning
                | crate::content::relics::RelicId::BottledTornado
                    if relic.amount > 0 =>
                {
                    Some(relic.amount as u32)
                }
                _ => None,
            })
            .collect();
        let mut java_group_order = Vec::new();
        let mut place_on_top = Vec::new();
        for card in std::mem::take(&mut self.zones.draw_pile) {
            if crate::content::cards::is_innate_card(&card) || bottled_uuids.contains(&card.uuid) {
                place_on_top.push(card);
            } else {
                java_group_order.push(card);
            }
        }
        java_group_order.extend(place_on_top);
        java_group_order.reverse();
        self.zones.draw_pile = java_group_order;
    }

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
    use std::collections::VecDeque;

    #[test]
    fn card_zones_draw_pile_top_is_index_zero() {
        let mut zones = CardZones {
            draw_pile: vec![CombatCard::new(CardId::Strike, 1)],
            hand: vec![],
            discard_pile: vec![],
            exhaust_pile: vec![],
            limbo: vec![],
            queued_cards: VecDeque::new(),
            card_uuid_counter: 1,
        };

        zones.add_to_draw_pile_top(CombatCard::new(CardId::Defend, 2));
        zones.add_to_draw_pile_bottom(CombatCard::new(CardId::Bash, 3));

        assert_eq!(
            zones.draw_top_card().map(|card| card.id),
            Some(CardId::Defend)
        );
        assert_eq!(
            zones.draw_top_card().map(|card| card.id),
            Some(CardId::Strike)
        );
        assert_eq!(
            zones.draw_top_card().map(|card| card.id),
            Some(CardId::Bash)
        );
    }

    #[test]
    fn card_zones_random_spot_maps_java_bottom_index_to_rust_top_index() {
        let mut zones = CardZones {
            draw_pile: vec![
                CombatCard::new(CardId::Strike, 1),
                CombatCard::new(CardId::Defend, 2),
                CombatCard::new(CardId::Bash, 3),
            ],
            hand: vec![],
            discard_pile: vec![],
            exhaust_pile: vec![],
            limbo: vec![],
            queued_cards: VecDeque::new(),
            card_uuid_counter: 3,
        };

        zones.add_to_draw_pile_random_spot_from_java_index(CombatCard::new(CardId::Wound, 4), 0);
        assert_eq!(zones.draw_pile[3].id, CardId::Wound);

        zones.add_to_draw_pile_random_spot_from_java_index(CombatCard::new(CardId::Burn, 5), 3);
        assert_eq!(zones.draw_pile[1].id, CardId::Burn);
        assert_eq!(zones.draw_pile[0].id, CardId::Strike);
    }

    #[test]
    fn card_zones_discard_pile_preserves_java_card_group_order() {
        let mut zones = CardZones {
            draw_pile: vec![],
            hand: vec![],
            discard_pile: vec![],
            exhaust_pile: vec![],
            limbo: vec![],
            queued_cards: VecDeque::new(),
            card_uuid_counter: 0,
        };

        zones.add_to_discard_pile_top(CombatCard::new(CardId::Strike, 1));
        zones.add_to_discard_pile_top(CombatCard::new(CardId::Defend, 2));

        assert_eq!(zones.discard_pile[0].id, CardId::Strike);
        assert_eq!(zones.discard_pile[1].id, CardId::Defend);
    }

    #[test]
    fn card_zones_exhaust_pile_preserves_java_card_group_order() {
        let mut zones = CardZones {
            draw_pile: vec![],
            hand: vec![],
            discard_pile: vec![],
            exhaust_pile: vec![],
            limbo: vec![],
            queued_cards: VecDeque::new(),
            card_uuid_counter: 0,
        };

        zones.add_to_exhaust_pile_top(CombatCard::new(CardId::Strike, 1));
        zones.add_to_exhaust_pile_top(CombatCard::new(CardId::Defend, 2));

        assert_eq!(zones.exhaust_pile[0].id, CardId::Strike);
        assert_eq!(zones.exhaust_pile[1].id, CardId::Defend);
    }

    #[test]
    fn card_zones_uuid_helper_updates_java_battle_instances_only() {
        let mut zones = CardZones {
            draw_pile: vec![CombatCard::new(CardId::Rampage, 7)],
            hand: vec![CombatCard::new(CardId::Rampage, 7)],
            discard_pile: vec![CombatCard::new(CardId::Strike, 8)],
            exhaust_pile: vec![CombatCard::new(CardId::Rampage, 7)],
            limbo: vec![CombatCard::new(CardId::Rampage, 7)],
            queued_cards: VecDeque::from([QueuedCardPlay {
                card: CombatCard::new(CardId::Rampage, 7),
                target: None,
                energy_on_use: 1,
                ignore_energy_total: false,
                autoplay: false,
                random_target: false,
                is_end_turn_autoplay: false,
                purge_on_use: false,
                source: QueuedCardSource::Normal,
            }]),
            card_uuid_counter: 8,
        };

        zones.for_each_java_battle_instance_mut_by_uuid(7, |card| {
            card.misc_value += 2;
        });

        assert_eq!(zones.hand[0].misc_value, 2);
        assert_eq!(zones.draw_pile[0].misc_value, 2);
        assert_eq!(zones.exhaust_pile[0].misc_value, 2);
        assert_eq!(zones.limbo[0].misc_value, 2);
        assert_eq!(zones.discard_pile[0].misc_value, 0);
        assert_eq!(
            zones.queued_cards[0].card.misc_value, 0,
            "queued cards are not Java CardGroup battle instances"
        );

        zones.for_each_queued_instance_mut_by_uuid(7, |card| {
            card.misc_value += 3;
        });
        assert_eq!(zones.queued_cards[0].card.misc_value, 3);
    }

    #[test]
    fn combat_card_update_cost_preserves_java_cost_for_turn_difference() {
        let mut card = CombatCard::new(CardId::BloodForBlood, 1);
        card.set_cost_for_turn_java(1);

        card.update_cost_java(-1);

        assert_eq!(card.combat_cost_without_turn_override_java(), 3);
        assert_eq!(card.cost_for_turn_java(), 0);
        assert_eq!(card.get_cost(), 0);
    }

    #[test]
    fn combat_card_modify_cost_for_combat_matches_java_zero_turn_branch() {
        let mut card = CombatCard::new(CardId::BloodForBlood, 1);
        card.set_cost_for_turn_java(0);

        card.modify_cost_for_combat_java(-1);

        assert_eq!(card.combat_cost_without_turn_override_java(), 3);
        assert_eq!(card.cost_for_turn_java(), 0);
    }

    #[test]
    fn combat_card_can_set_combat_cost_without_erasing_turn_override() {
        let mut card = CombatCard::new(CardId::BloodForBlood, 1);
        card.set_cost_for_turn_java(0);

        card.set_combat_cost_preserving_turn_java(1);

        assert_eq!(card.combat_cost_without_turn_override_java(), 1);
        assert_eq!(card.cost_for_turn_java(), 0);
    }

    #[test]
    fn java_initialize_deck_order_places_innate_on_rust_top_after_reversing() {
        let mut state = crate::test_support::blank_test_combat();
        state.zones.draw_pile = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::Writhe, 2),
            CombatCard::new(CardId::Defend, 3),
            CombatCard::new(CardId::Pride, 4),
        ];

        state.apply_java_initialize_deck_order_after_shuffle();

        assert_eq!(
            state
                .zones
                .draw_pile
                .iter()
                .map(|card| card.id)
                .collect::<Vec<_>>(),
            vec![
                CardId::Pride,
                CardId::Writhe,
                CardId::Defend,
                CardId::Strike
            ]
        );
    }

    #[test]
    fn engine_runtime_queue_actions_matches_java_add_to_top_order() {
        let mut engine = EngineRuntime {
            action_queue: VecDeque::new(),
        };

        engine.push_back(Action::DrawCards(99));
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
            Some(Action::GainBlock {
                target: 0,
                amount: 3,
            }),
            "later addToTop calls should execute before earlier top calls"
        );
        assert_eq!(
            engine.pop_front(),
            Some(Action::GainEnergy { amount: 2 }),
            "earlier addToTop calls should remain ahead of existing queued actions"
        );
        assert_eq!(
            engine.pop_front(),
            Some(Action::DrawCards(99)),
            "existing queued actions remain ahead of later addToBot actions"
        );
        assert_eq!(
            engine.pop_front(),
            Some(Action::DrawCards(1)),
            "addToBot actions should append after existing queued actions"
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
                cards_discarded_this_turn: 3,
                card_ids_played_this_turn: vec![CardId::Strike, CardId::Defend],
                card_ids_played_this_combat: vec![CardId::Zap],
                orbs_channeled_this_turn: vec![OrbId::Lightning],
                orbs_channeled_this_combat: vec![OrbId::Lightning, OrbId::Frost],
                mantra_gained_this_combat: 4,
                times_damaged_this_combat: 3,
                victory_triggered: false,
                discovery_cost_for_turn: None,
                early_end_turn_pending: true,
                skip_monster_turn_pending: true,
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
        assert_eq!(turn.counters.cards_discarded_this_turn, 0);
        assert!(turn.counters.card_ids_played_this_turn.is_empty());
        assert_eq!(
            turn.counters.card_ids_played_this_combat,
            vec![CardId::Zap],
            "combat-wide played-card history should remain untouched"
        );
        assert!(turn.counters.orbs_channeled_this_turn.is_empty());
        assert_eq!(
            turn.counters.orbs_channeled_this_combat,
            vec![OrbId::Lightning, OrbId::Frost],
            "combat-wide orb channel history should remain untouched"
        );
        assert_eq!(
            turn.counters.mantra_gained_this_combat, 4,
            "combat-wide mantra history should remain untouched"
        );
        assert_eq!(
            turn.counters.times_damaged_this_combat, 3,
            "combat-wide counters should remain untouched"
        );
        assert!(
            turn.counters.early_end_turn_pending,
            "unrelated flags should not be reset by turn-start setup"
        );
        assert!(
            !turn.counters.skip_monster_turn_pending,
            "Java clears room.skipMonsterTurn when the next player turn starts"
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

    #[test]
    fn monster_turn_plan_and_preview_follow_planned_visible_spec() {
        let spec = MonsterMoveSpec::Attack(AttackSpec {
            base_damage: 6,
            hits: 2,
            damage_kind: DamageKind::Normal,
        });
        let monster = MonsterEntity {
            id: 1,
            monster_type: 0,
            current_hp: 30,
            max_hp: 30,
            block: 0,
            slot: 0,
            is_dying: false,
            is_escaped: false,
            half_dead: false,
            move_state: MonsterMoveState {
                planned_move_id: 7,
                history: VecDeque::from([1, 7]),
                planned_steps: Some(spec.to_steps()),
                planned_visible_spec: Some(spec.clone()),
            },
            logical_position: 0,
            hexaghost: HexaghostRuntimeState::default(),
            louse: LouseRuntimeState::default(),
            jaw_worm: JawWormRuntimeState::default(),
            thief: ThiefRuntimeState::default(),
            byrd: ByrdRuntimeState::default(),
            chosen: ChosenRuntimeState::default(),
            snecko: SneckoRuntimeState::default(),
            shelled_parasite: ShelledParasiteRuntimeState::default(),
            bronze_automaton: BronzeAutomatonRuntimeState::default(),
            bronze_orb: BronzeOrbRuntimeState::default(),
            book_of_stabbing: BookOfStabbingRuntimeState::default(),
            collector: CollectorRuntimeState::default(),
            champ: ChampRuntimeState::default(),
            awakened_one: AwakenedOneRuntimeState::default(),
            corrupt_heart: CorruptHeartRuntimeState::default(),
            writhing_mass: WrithingMassRuntimeState::default(),
            spiker: SpikerRuntimeState::default(),
            spire_shield: SpireShieldRuntimeState::default(),
            spire_spear: SpireSpearRuntimeState::default(),
            slaver_red: SlaverRedRuntimeState::default(),
            gremlin_leader: GremlinLeaderRuntimeState::default(),
            gremlin_nob: GremlinNobRuntimeState::default(),
            gremlin_wizard: GremlinWizardRuntimeState::default(),
            cultist: CultistRuntimeState::default(),
            sentry: SentryRuntimeState::default(),
            slime_boss: SlimeBossRuntimeState::default(),
            large_slime: LargeSlimeRuntimeState::default(),
            spheric_guardian: SphericGuardianRuntimeState::default(),
            reptomancer: ReptomancerRuntimeState::default(),
            darkling: DarklingRuntimeState::default(),
            nemesis: NemesisRuntimeState::default(),
            giant_head: GiantHeadRuntimeState::default(),
            time_eater: TimeEaterRuntimeState::default(),
            donu: DonuRuntimeState::default(),
            deca: DecaRuntimeState::default(),
            transient: TransientRuntimeState::default(),
            lagavulin: LagavulinRuntimeState::default(),
            guardian: GuardianRuntimeState::default(),
        };

        let plan = monster.turn_plan();
        let preview = crate::projection::combat::project_monster_move_preview(&monster);

        assert_eq!(plan.move_id, 7);
        assert!(matches!(plan.summary_spec(), MonsterMoveSpec::Attack(_)));
        assert_eq!(preview.damage_per_hit, Some(6));
        assert_eq!(preview.hits, 2);
        assert_eq!(preview.total_damage, Some(12));
        assert!(crate::projection::combat::monster_has_visible_attack(
            &monster
        ));
        assert_eq!(
            crate::projection::combat::monster_preview_total_damage(&monster),
            12
        );
    }

    #[test]
    fn monster_turn_plan_uses_planned_steps_damage_without_visible_spec() {
        let monster = MonsterEntity {
            id: 1,
            monster_type: 0,
            current_hp: 12,
            max_hp: 12,
            block: 0,
            slot: 0,
            is_dying: false,
            is_escaped: false,
            half_dead: false,
            move_state: MonsterMoveState {
                planned_move_id: 3,
                history: VecDeque::from([4, 3]),
                planned_steps: Some(
                    MonsterMoveSpec::Attack(AttackSpec {
                        base_damage: 7,
                        hits: 1,
                        damage_kind: DamageKind::Normal,
                    })
                    .to_steps(),
                ),
                planned_visible_spec: None,
            },
            logical_position: 0,
            hexaghost: HexaghostRuntimeState::default(),
            louse: LouseRuntimeState::default(),
            jaw_worm: JawWormRuntimeState::default(),
            thief: ThiefRuntimeState::default(),
            byrd: ByrdRuntimeState::default(),
            chosen: ChosenRuntimeState::default(),
            snecko: SneckoRuntimeState::default(),
            shelled_parasite: ShelledParasiteRuntimeState::default(),
            bronze_automaton: BronzeAutomatonRuntimeState::default(),
            bronze_orb: BronzeOrbRuntimeState::default(),
            book_of_stabbing: BookOfStabbingRuntimeState::default(),
            collector: CollectorRuntimeState::default(),
            champ: ChampRuntimeState::default(),
            awakened_one: AwakenedOneRuntimeState::default(),
            corrupt_heart: CorruptHeartRuntimeState::default(),
            writhing_mass: WrithingMassRuntimeState::default(),
            spiker: SpikerRuntimeState::default(),
            spire_shield: SpireShieldRuntimeState::default(),
            spire_spear: SpireSpearRuntimeState::default(),
            slaver_red: SlaverRedRuntimeState::default(),
            gremlin_leader: GremlinLeaderRuntimeState::default(),
            gremlin_nob: GremlinNobRuntimeState::default(),
            gremlin_wizard: GremlinWizardRuntimeState::default(),
            cultist: CultistRuntimeState::default(),
            sentry: SentryRuntimeState::default(),
            slime_boss: SlimeBossRuntimeState::default(),
            large_slime: LargeSlimeRuntimeState::default(),
            spheric_guardian: SphericGuardianRuntimeState::default(),
            reptomancer: ReptomancerRuntimeState::default(),
            darkling: DarklingRuntimeState::default(),
            nemesis: NemesisRuntimeState::default(),
            giant_head: GiantHeadRuntimeState::default(),
            time_eater: TimeEaterRuntimeState::default(),
            donu: DonuRuntimeState::default(),
            deca: DecaRuntimeState::default(),
            transient: TransientRuntimeState::default(),
            lagavulin: LagavulinRuntimeState::default(),
            guardian: GuardianRuntimeState::default(),
        };

        let plan = monster.turn_plan();

        assert_eq!(plan.move_id, 3);
        assert_eq!(plan.attack().map(|attack| attack.base_damage), Some(7));
        assert_eq!(
            crate::projection::combat::project_monster_move_preview(&monster).damage_per_hit,
            Some(7)
        );
    }

    #[test]
    fn monster_turn_plan_hides_visible_spec_for_half_dead_or_dying_monsters() {
        let monster = MonsterEntity {
            id: 1,
            monster_type: 0,
            current_hp: 0,
            max_hp: 56,
            block: 0,
            slot: 0,
            is_dying: false,
            is_escaped: false,
            half_dead: true,
            move_state: MonsterMoveState {
                planned_move_id: 4,
                history: VecDeque::from([3, 4]),
                planned_steps: Some(
                    MonsterMoveSpec::Attack(AttackSpec {
                        base_damage: 13,
                        hits: 1,
                        damage_kind: DamageKind::Normal,
                    })
                    .to_steps(),
                ),
                planned_visible_spec: Some(MonsterMoveSpec::Attack(AttackSpec {
                    base_damage: 13,
                    hits: 1,
                    damage_kind: DamageKind::Normal,
                })),
            },
            logical_position: 0,
            hexaghost: HexaghostRuntimeState::default(),
            louse: LouseRuntimeState::default(),
            jaw_worm: JawWormRuntimeState::default(),
            thief: ThiefRuntimeState::default(),
            byrd: ByrdRuntimeState::default(),
            chosen: ChosenRuntimeState::default(),
            snecko: SneckoRuntimeState::default(),
            shelled_parasite: ShelledParasiteRuntimeState::default(),
            bronze_automaton: BronzeAutomatonRuntimeState::default(),
            bronze_orb: BronzeOrbRuntimeState::default(),
            book_of_stabbing: BookOfStabbingRuntimeState::default(),
            collector: CollectorRuntimeState::default(),
            champ: ChampRuntimeState::default(),
            awakened_one: AwakenedOneRuntimeState::default(),
            corrupt_heart: CorruptHeartRuntimeState::default(),
            writhing_mass: WrithingMassRuntimeState::default(),
            spiker: SpikerRuntimeState::default(),
            spire_shield: SpireShieldRuntimeState::default(),
            spire_spear: SpireSpearRuntimeState::default(),
            slaver_red: SlaverRedRuntimeState::default(),
            gremlin_leader: GremlinLeaderRuntimeState::default(),
            gremlin_nob: GremlinNobRuntimeState::default(),
            gremlin_wizard: GremlinWizardRuntimeState::default(),
            cultist: CultistRuntimeState::default(),
            sentry: SentryRuntimeState::default(),
            slime_boss: SlimeBossRuntimeState::default(),
            large_slime: LargeSlimeRuntimeState::default(),
            spheric_guardian: SphericGuardianRuntimeState::default(),
            reptomancer: ReptomancerRuntimeState::default(),
            darkling: DarklingRuntimeState::default(),
            nemesis: NemesisRuntimeState::default(),
            giant_head: GiantHeadRuntimeState::default(),
            time_eater: TimeEaterRuntimeState::default(),
            donu: DonuRuntimeState::default(),
            deca: DecaRuntimeState::default(),
            transient: TransientRuntimeState::default(),
            lagavulin: LagavulinRuntimeState::default(),
            guardian: GuardianRuntimeState::default(),
        };

        let plan = monster.turn_plan();
        let preview = crate::projection::combat::project_monster_move_preview(&monster);

        assert_eq!(plan.move_id, 4);
        assert!(matches!(plan.summary_spec(), MonsterMoveSpec::Unknown));
        assert_eq!(preview.damage_per_hit, None);
        assert!(!crate::projection::combat::monster_has_visible_attack(
            &monster
        ));
        assert_eq!(
            crate::projection::combat::monster_preview_total_damage(&monster),
            0
        );
    }
}
