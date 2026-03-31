

use std::collections::{HashMap, VecDeque};
use crate::core::EntityId;
use crate::content::cards::CardId;
use crate::action::Action;
use crate::content::relics::RelicState;

#[derive(Clone, Debug, PartialEq)]
pub enum MetaChange {
    AddCardToMasterDeck(CardId),
}

// --- ID Types ---
pub use crate::content::powers::PowerId;
pub type MonsterId = usize;

#[derive(Clone, Debug, PartialEq)]
pub struct CombatState {
    pub ascension_level: u8,
    pub turn_count: u32,
    pub current_phase: CombatPhase,
    pub energy: u8,
    pub draw_pile: Vec<CombatCard>,
    pub hand: Vec<CombatCard>,
    pub discard_pile: Vec<CombatCard>,
    pub exhaust_pile: Vec<CombatCard>,
    pub limbo: Vec<CombatCard>,
    pub player: PlayerEntity,
    pub monsters: Vec<MonsterEntity>,
    pub potions: Vec<Option<crate::content::potions::Potion>>,
    pub power_db: HashMap<EntityId, Vec<Power>>,
    pub action_queue: VecDeque<Action>,
    pub counters: EphemeralCounters,
    pub card_uuid_counter: u32,
    pub rng: crate::rng::RngPool,
    pub is_boss_fight: bool,
    pub is_elite_fight: bool,
    pub meta_changes: Vec<MetaChange>,
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
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct EpsteinCounters {
//  placeholder left open;
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct RelicBuses {
    pub at_battle_start: smallvec::SmallVec<[usize; 4]>,
    pub at_turn_start: smallvec::SmallVec<[usize; 4]>,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum OrbId {
    Empty,      // Placeholder for an empty orb slot
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
            OrbId::Empty => OrbEntity { id, passive_amount: 0, evoke_amount: 0 },
            OrbId::Lightning => OrbEntity { id, passive_amount: 3, evoke_amount: 8 },
            OrbId::Dark => OrbEntity { id, passive_amount: 6, evoke_amount: 6 },
            OrbId::Frost => OrbEntity { id, passive_amount: 2, evoke_amount: 5 },
            OrbId::Plasma => OrbEntity { id, passive_amount: 1, evoke_amount: 2 },
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
}

impl PlayerEntity {
    pub fn has_relic(&self, id: crate::content::relics::RelicId) -> bool {
        self.relics.iter().any(|r| r.id == id)
    }

    pub fn add_relic(&mut self, state: RelicState) {
        let index = self.relics.len();
        let sub = crate::content::relics::get_relic_subscriptions(state.id);
        
        self.relics.push(state);
        
        if sub.at_battle_start { self.relic_buses.at_battle_start.push(index); }
        if sub.at_turn_start { self.relic_buses.at_turn_start.push(index); }
        if sub.on_use_card { self.relic_buses.on_use_card.push(index); }
        if sub.on_shuffle { self.relic_buses.on_shuffle.push(index); }
        if sub.on_exhaust { self.relic_buses.on_exhaust.push(index); }
        if sub.on_lose_hp { self.relic_buses.on_lose_hp.push(index); }
        if sub.on_victory { self.relic_buses.on_victory.push(index); }
        if sub.on_apply_power { self.relic_buses.on_apply_power.push(index); }
        if sub.on_monster_death { self.relic_buses.on_monster_death.push(index); }
        if sub.on_spawn_monster { self.relic_buses.on_spawn_monster.push(index); }
        if sub.at_end_of_turn { self.relic_buses.at_end_of_turn.push(index); }
        if sub.on_use_potion { self.relic_buses.on_use_potion.push(index); }
        if sub.on_discard { self.relic_buses.on_discard.push(index); }
        if sub.on_change_stance { self.relic_buses.on_change_stance.push(index); }
        if sub.on_attacked_to_change_damage { self.relic_buses.on_attacked_to_change_damage.push(index); }
        if sub.on_lose_hp_last { self.relic_buses.on_lose_hp_last.push(index); }

        if sub.on_calculate_heal { self.relic_buses.on_calculate_heal.push(index); }
        if sub.on_calculate_x_cost { self.relic_buses.on_calculate_x_cost.push(index); }
        if sub.on_calculate_block_retained { self.relic_buses.on_calculate_block_retained.push(index); }
        if sub.on_calculate_energy_retained { self.relic_buses.on_calculate_energy_retained.push(index); }
        if sub.on_scry { self.relic_buses.on_scry.push(index); }
        if sub.on_receive_power_modify { self.relic_buses.on_receive_power_modify.push(index); }
        if sub.on_calculate_vulnerable_multiplier { self.relic_buses.on_calculate_vulnerable_multiplier.push(index); }
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
    pub next_move_byte: u8,
    pub current_intent: Intent,
    pub move_history: VecDeque<u8>,
    pub intent_dmg: i32,
    pub logical_position: i32,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CombatCard {
    pub id: CardId,
    pub uuid: u32,
    pub upgrades: u8,
    pub misc_value: i32,
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
            if def.cost < 0 {
                return def.cost;
            }
            let mut c = def.cost as i8 + self.cost_modifier;
            if c < 0 { c = 0; }
            c
        }
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Power {
    pub power_type: PowerId,
    pub amount: i32,
    pub extra_data: i32,
    pub just_applied: bool,
}

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
        if let Some(c) = Self::remove_card_by_uuid(&mut self.hand, uuid) { return Some(c); }
        if let Some(c) = Self::remove_card_by_uuid(&mut self.limbo, uuid) { return Some(c); }
        if let Some(c) = Self::remove_card_by_uuid(&mut self.draw_pile, uuid) { return Some(c); }
        if let Some(c) = Self::remove_card_by_uuid(&mut self.discard_pile, uuid) { return Some(c); }
        if let Some(c) = Self::remove_card_by_uuid(&mut self.exhaust_pile, uuid) { return Some(c); }
        None
    }

    /// Gets the current stack amount of a specific power on an entity
    pub fn get_power(&self, target: EntityId, power_id: PowerId) -> i32 {
        if let Some(powers) = self.power_db.get(&target) {
            if let Some(power) = powers.iter().find(|p| p.power_type == power_id) {
                return power.amount;
            }
        }
        0
    }

    /// Reparses all cards in the hand to dynamically calculate damage, block, and magic numbers.
    /// Clones the hand to satisfy borrow-checker while allowing `PerfectedStrike` to read `&self.hand`.
    pub fn update_hand_cards(&mut self) {
        let mut new_hand = self.hand.clone();
        for card in &mut new_hand {
            crate::content::cards::evaluate_card(card, self, None);
        }
        self.hand = new_hand;
    }
}
