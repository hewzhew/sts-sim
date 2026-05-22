use crate::content::cards::CardId;

use super::CombatCardKey;

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) struct CombatMetaKey {
    pub(crate) ascension_level: u8,
    pub(crate) player_class: String,
    pub(crate) is_boss_fight: bool,
    pub(crate) is_elite_fight: bool,
    pub(crate) master_deck_snapshot: Vec<CombatCardKey>,
    pub(crate) meta_changes: Vec<CombatMetaChangeKey>,
}

#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub(crate) enum CombatMetaChangeKey {
    AddCardToMasterDeck(CardId),
    ModifyCardMisc { card_uuid: u32, amount: i32 },
    UpgradeMasterDeckCard { card_uuid: u32 },
}
