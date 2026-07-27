use crate::content::relics::RelicId;
use crate::runtime::combat::{OrbId, StanceId};
use serde::ser::SerializeSeq;

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize)]
pub(crate) struct CombatExactPlayerKey {
    pub(crate) current_hp: i32,
    pub(crate) block: i32,
    pub(crate) future_relevant: CombatPlayerFutureKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize)]
pub(crate) struct CombatDominancePlayerKey {
    pub(crate) future_relevant: CombatPlayerFutureKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize)]
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

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize)]
pub(crate) struct CombatOrbKey {
    pub(crate) id: OrbId,
    pub(crate) base_passive_amount: i32,
    pub(crate) base_evoke_amount: i32,
    pub(crate) passive_amount: i32,
    pub(crate) evoke_amount: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, serde::Serialize)]
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
/// runtime cache into 26 allocation owners. Durable identity serializes the
/// ordered semantic buses without coupling it to this packed storage layout.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
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

impl serde::Serialize for CombatRelicBusesKey {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut buses = serializer.serialize_seq(Some(RELIC_BUS_COUNT))?;
        for index in 0..RELIC_BUS_COUNT {
            buses.serialize_element(self.bus(index))?;
        }
        buses.end()
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
