use crate::content::cards::CardId;
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatRuntimeHintsKey {
    pub(crate) using_card: bool,
    /// V1 exact-hash compatibility for the removed, never-written legacy
    /// runtime card queue. Java cardQueue is owned by `CardZones::queued_cards`.
    pub(crate) card_queue: CombatLegacyEmptyCardQueueKey,
    pub(crate) colorless_combat_pool: Vec<CardId>,
    pub(crate) pending_rewards: Vec<String>,
    pub(crate) power_instance_counter: u32,
    pub(crate) last_drawn_cards: Vec<CombatDrawnCardKey>,
    pub(crate) monster_protocol: Vec<CombatMonsterProtocolKey>,
    pub(crate) combat_mugged: bool,
    pub(crate) combat_smoked: bool,
}

#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub(crate) struct CombatLegacyEmptyCardQueueKey;

impl std::fmt::Debug for CombatLegacyEmptyCardQueueKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.debug_list().finish()
    }
}

impl Hash for CombatLegacyEmptyCardQueueKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        // The former empty Vec field hashed only its zero length. Preserve
        // those bytes without retaining a runtime field.
        0usize.hash(state);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::hash_map::DefaultHasher;

    fn hash_of(value: &impl Hash) -> u64 {
        let mut hasher = DefaultHasher::new();
        value.hash(&mut hasher);
        hasher.finish()
    }

    #[test]
    fn legacy_empty_card_queue_key_preserves_v1_debug_and_hash_shape() {
        let compatibility_key = CombatLegacyEmptyCardQueueKey;
        let old_empty_queue = Vec::<u8>::new();

        assert_eq!(
            format!("{compatibility_key:?}"),
            format!("{old_empty_queue:?}")
        );
        assert_eq!(hash_of(&compatibility_key), hash_of(&old_empty_queue));
    }
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
