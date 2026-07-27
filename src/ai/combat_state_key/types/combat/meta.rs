use crate::content::cards::CardId;
use crate::runtime::combat::{CombatCard, MasterDeckSnapshot};
use std::hash::{Hash, Hasher};

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatMetaKey {
    pub(crate) ascension_level: u8,
    pub(crate) player_class: String,
    pub(crate) is_boss_fight: bool,
    pub(crate) is_elite_fight: bool,
    pub(crate) master_deck_snapshot: CombatMasterDeckKey,
    pub(crate) meta_changes: Vec<CombatMetaChangeKey>,
}

#[derive(Clone, PartialEq, Eq)]
pub(crate) struct CombatMasterDeckKey(MasterDeckSnapshot);

impl CombatMasterDeckKey {
    pub(crate) fn new(snapshot: &MasterDeckSnapshot) -> Self {
        Self(snapshot.clone())
    }
}

impl Hash for CombatMasterDeckKey {
    fn hash<H: Hasher>(&self, state: &mut H) {
        state.write_u64(self.0.structural_hash());
    }
}

impl std::fmt::Debug for CombatMasterDeckKey {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_list()
            .entries(self.0.iter().map(CombatCardKeyDebug))
            .finish()
    }
}

struct CombatCardKeyDebug<'a>(&'a CombatCard);

impl std::fmt::Debug for CombatCardKeyDebug<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let card = self.0;
        formatter
            .debug_struct("CombatCardKey")
            .field("id", &card.id)
            .field("uuid", &card.uuid)
            .field("upgrades", &card.upgrades)
            .field("misc_value", &card.misc_value)
            .field("base_damage_override", &card.base_damage_override)
            .field("base_block_override", &card.base_block_override)
            .field("cost_modifier", &card.cost_modifier)
            .field("cost_for_turn", &card.cost_for_turn)
            .field("base_damage_mut", &card.base_damage_mut)
            .field("base_block_mut", &card.base_block_mut)
            .field("base_magic_num_mut", &card.base_magic_num_mut)
            .field("multi_damage", &card.multi_damage)
            .field("exhaust_override", &card.exhaust_override)
            .field("retain_override", &card.retain_override)
            .field("free_to_play_once", &card.free_to_play_once)
            .field("energy_on_use", &card.energy_on_use)
            .finish()
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CombatMetaChangeKey {
    AddCardToMasterDeck(CardId),
    ModifyCardMisc { card_uuid: u32, amount: i32 },
    UpgradeMasterDeckCard { card_uuid: u32 },
}
