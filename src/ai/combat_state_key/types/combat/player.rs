use crate::content::relics::RelicId;
use crate::runtime::combat::{OrbId, StanceId};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatExactPlayerKey {
    pub(crate) current_hp: i32,
    pub(crate) block: i32,
    pub(crate) future_relevant: CombatPlayerFutureKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatDominancePlayerKey {
    pub(crate) future_relevant: CombatPlayerFutureKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatPlayerFutureKey {
    pub(crate) entity_id: usize,
    pub(crate) max_hp: i32,
    pub(crate) facing_left: bool,
    pub(crate) gold_delta_this_combat: i32,
    pub(crate) gold: i32,
    pub(crate) max_orbs: u8,
    pub(crate) orbs: Vec<CombatOrbKey>,
    pub(crate) stance: StanceId,
    pub(crate) relics: Vec<CombatRelicKey>,
    pub(crate) relic_buses: CombatRelicBusesKey,
    pub(crate) energy_master: u8,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatOrbKey {
    pub(crate) id: OrbId,
    pub(crate) base_passive_amount: i32,
    pub(crate) base_evoke_amount: i32,
    pub(crate) passive_amount: i32,
    pub(crate) evoke_amount: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatRelicKey {
    pub(crate) id: RelicId,
    pub(crate) counter: i32,
    pub(crate) used_up: bool,
    pub(crate) amount: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatRelicBusesKey {
    pub(crate) at_pre_battle: Vec<usize>,
    pub(crate) at_battle_start_pre_draw: Vec<usize>,
    pub(crate) at_battle_start: Vec<usize>,
    pub(crate) at_turn_start: Vec<usize>,
    pub(crate) at_turn_start_post_draw: Vec<usize>,
    pub(crate) on_use_card: Vec<usize>,
    pub(crate) on_shuffle: Vec<usize>,
    pub(crate) on_exhaust: Vec<usize>,
    pub(crate) on_lose_hp: Vec<usize>,
    pub(crate) on_victory: Vec<usize>,
    pub(crate) on_apply_power: Vec<usize>,
    pub(crate) on_monster_death: Vec<usize>,
    pub(crate) on_spawn_monster: Vec<usize>,
    pub(crate) at_end_of_turn: Vec<usize>,
    pub(crate) on_use_potion: Vec<usize>,
    pub(crate) on_discard: Vec<usize>,
    pub(crate) on_change_stance: Vec<usize>,
    pub(crate) on_attacked_to_change_damage: Vec<usize>,
    pub(crate) on_lose_hp_last: Vec<usize>,
    pub(crate) on_calculate_heal: Vec<usize>,
    pub(crate) on_calculate_x_cost: Vec<usize>,
    pub(crate) on_calculate_block_retained: Vec<usize>,
    pub(crate) on_calculate_energy_retained: Vec<usize>,
    pub(crate) on_scry: Vec<usize>,
    pub(crate) on_receive_power_modify: Vec<usize>,
    pub(crate) on_calculate_vulnerable_multiplier: Vec<usize>,
}
