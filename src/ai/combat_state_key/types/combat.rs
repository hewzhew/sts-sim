#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatExactStateKey {
    pub(crate) common: CombatRuntimeKey,
    pub(crate) player: CombatExactPlayerKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatDominanceKey {
    pub(crate) common: CombatRuntimeKey,
    pub(crate) player: CombatDominancePlayerKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatRuntimeKey {
    pub(crate) engine: String,
    pub(crate) turn: String,
    pub(crate) meta: String,
    pub(crate) zones: CombatZonesKey,
    pub(crate) monsters: Vec<String>,
    pub(crate) powers: Vec<CombatEntityPowersKey>,
    pub(crate) potions: Vec<String>,
    pub(crate) queue: Vec<String>,
    pub(crate) runtime: String,
    pub(crate) rng: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatExactPlayerKey {
    pub(crate) current_hp: i32,
    pub(crate) block: i32,
    pub(crate) future_relevant: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatDominancePlayerKey {
    pub(crate) future_relevant: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatZonesKey {
    pub(crate) card_uuid_counter: u32,
    pub(crate) hand: Vec<String>,
    pub(crate) draw: Vec<String>,
    pub(crate) discard: Vec<String>,
    pub(crate) exhaust: Vec<String>,
    pub(crate) limbo: Vec<String>,
    pub(crate) queued: Vec<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatEntityPowersKey {
    pub(crate) entity_id: usize,
    pub(crate) powers: Vec<String>,
}
