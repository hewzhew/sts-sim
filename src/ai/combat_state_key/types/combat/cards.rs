use crate::content::cards::CardId;
use crate::runtime::combat::QueuedCardSource;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatZonesKey {
    pub(crate) card_uuid_counter: u32,
    pub(crate) hand: Vec<CombatCardKey>,
    pub(crate) draw: Vec<CombatCardKey>,
    pub(crate) discard: Vec<CombatCardKey>,
    pub(crate) exhaust: Vec<CombatCardKey>,
    pub(crate) limbo: Vec<CombatCardKey>,
    pub(crate) queued: Vec<CombatQueuedCardKey>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatCardKey {
    pub(crate) id: CardId,
    pub(crate) uuid: u32,
    pub(crate) upgrades: u8,
    pub(crate) misc_value: i32,
    pub(crate) base_damage_override: Option<i32>,
    pub(crate) base_block_override: Option<i32>,
    pub(crate) cost_modifier: i8,
    pub(crate) cost_for_turn: Option<u8>,
    pub(crate) base_damage_mut: i32,
    pub(crate) base_block_mut: i32,
    pub(crate) base_magic_num_mut: i32,
    pub(crate) multi_damage: Vec<i32>,
    pub(crate) exhaust_override: Option<bool>,
    pub(crate) retain_override: Option<bool>,
    pub(crate) free_to_play_once: bool,
    pub(crate) energy_on_use: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatQueuedCardKey {
    pub(crate) card: CombatCardKey,
    pub(crate) target: CombatTargetKey,
    pub(crate) energy_on_use: i32,
    pub(crate) ignore_energy_total: bool,
    pub(crate) autoplay: bool,
    pub(crate) random_target: bool,
    pub(crate) is_end_turn_autoplay: bool,
    pub(crate) purge_on_use: bool,
    pub(crate) source: QueuedCardSource,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CombatTargetKey {
    None,
    MonsterSlot(usize),
    Entity(usize),
}
