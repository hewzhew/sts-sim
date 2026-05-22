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
    pub(crate) relic_buses: String,
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
