use crate::content::relics::RelicId;
use crate::runtime::combat::{OrbId, StanceId};
use std::hash::{Hash, Hasher};

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

const RELIC_BUS_COUNT: usize = 26;

/// Exact identity of the derived relic dispatch cache in two allocations.
///
/// The former representation owned 26 independent Vecs for every exact
/// search key. The hook indexes are immutable projections, so one backing
/// buffer plus its slice ends preserves all ordered buses without copying the
/// runtime cache into 26 allocation owners. Custom Debug and Hash retain the
/// exact v1 field shape used by durable diagnostic identities.
#[derive(Clone, PartialEq, Eq)]
pub(crate) struct CombatRelicBusesKey {
    indices: Vec<usize>,
    ends: Box<[usize; RELIC_BUS_COUNT]>,
}

impl CombatRelicBusesKey {
    pub(crate) fn from_slices(slices: [&[usize]; RELIC_BUS_COUNT]) -> Self {
        let total_len = slices.iter().map(|slice| slice.len()).sum();
        let mut indices = Vec::with_capacity(total_len);
        let mut ends = [0; RELIC_BUS_COUNT];
        for (index, slice) in slices.into_iter().enumerate() {
            indices.extend_from_slice(slice);
            ends[index] = indices.len();
        }
        Self {
            indices,
            ends: Box::new(ends),
        }
    }

    fn bus(&self, index: usize) -> &[usize] {
        let start = index
            .checked_sub(1)
            .map_or(0, |previous| self.ends[previous]);
        &self.indices[start..self.ends[index]]
    }
}

impl std::fmt::Debug for CombatRelicBusesKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("CombatRelicBusesKey")
            .field("at_pre_battle", &self.bus(0))
            .field("at_battle_start_pre_draw", &self.bus(1))
            .field("at_battle_start", &self.bus(2))
            .field("at_turn_start", &self.bus(3))
            .field("at_turn_start_post_draw", &self.bus(4))
            .field("on_use_card", &self.bus(5))
            .field("on_shuffle", &self.bus(6))
            .field("on_exhaust", &self.bus(7))
            .field("on_lose_hp", &self.bus(8))
            .field("on_victory", &self.bus(9))
            .field("on_apply_power", &self.bus(10))
            .field("on_monster_death", &self.bus(11))
            .field("on_spawn_monster", &self.bus(12))
            .field("at_end_of_turn", &self.bus(13))
            .field("on_use_potion", &self.bus(14))
            .field("on_discard", &self.bus(15))
            .field("on_change_stance", &self.bus(16))
            .field("on_attacked_to_change_damage", &self.bus(17))
            .field("on_lose_hp_last", &self.bus(18))
            .field("on_calculate_heal", &self.bus(19))
            .field("on_calculate_x_cost", &self.bus(20))
            .field("on_calculate_block_retained", &self.bus(21))
            .field("on_calculate_energy_retained", &self.bus(22))
            .field("on_scry", &self.bus(23))
            .field("on_receive_power_modify", &self.bus(24))
            .field("on_calculate_vulnerable_multiplier", &self.bus(25))
            .finish()
    }
}

impl Hash for CombatRelicBusesKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        for index in 0..RELIC_BUS_COUNT {
            self.bus(index).hash(state);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CombatRelicBusesKey;

    #[test]
    fn relic_bus_key_does_not_restore_one_vec_owner_per_hook() {
        assert!(
            std::mem::size_of::<CombatRelicBusesKey>() <= 4 * std::mem::size_of::<usize>(),
            "packed relic bus identity should remain two allocation handles, not 26 Vec owners"
        );
    }
}
