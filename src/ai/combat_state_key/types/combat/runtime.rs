use crate::content::cards::CardId;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatRuntimeHintsKey {
    pub(crate) using_card: bool,
    pub(crate) card_queue: Vec<CombatQueuedCardHintKey>,
    pub(crate) colorless_combat_pool: Vec<CardId>,
    pub(crate) emitted_events: Vec<String>,
    pub(crate) engine_diagnostics: Vec<String>,
    pub(crate) pending_rewards: Vec<String>,
    pub(crate) power_instance_counter: u32,
    pub(crate) last_drawn_cards: Vec<CombatDrawnCardKey>,
    pub(crate) monster_protocol: Vec<CombatMonsterProtocolKey>,
    pub(crate) combat_mugged: bool,
    pub(crate) combat_smoked: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatQueuedCardHintKey {
    pub(crate) card_uuid: u32,
    pub(crate) card_id: CardId,
    pub(crate) target_monster_index: Option<usize>,
    pub(crate) energy_on_use: i32,
    pub(crate) ignore_energy_total: bool,
    pub(crate) autoplay: bool,
    pub(crate) random_target: bool,
    pub(crate) is_end_turn_autoplay: bool,
    pub(crate) purge_on_use: bool,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatDrawnCardKey {
    pub(crate) card_uuid: u32,
    pub(crate) card_id: CardId,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatMonsterProtocolKey {
    pub(crate) entity_id: usize,
    pub(crate) observation: CombatMonsterProtocolObservationKey,
    pub(crate) identity: CombatMonsterProtocolIdentityKey,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatMonsterProtocolObservationKey {
    pub(crate) visible_intent: CombatIntentKey,
    pub(crate) preview_damage_per_hit: i32,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatMonsterProtocolIdentityKey {
    pub(crate) instance_id: Option<u64>,
    pub(crate) spawn_order: Option<u64>,
    pub(crate) draw_x: Option<i32>,
    pub(crate) group_index: Option<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CombatIntentKey {
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
