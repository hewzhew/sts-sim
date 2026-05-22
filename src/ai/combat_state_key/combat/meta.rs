use crate::runtime::combat::{CombatState, MetaChange};

use super::super::types::{CombatMetaChangeKey, CombatMetaKey};
use super::cards::card_key;

pub(super) fn meta_key(combat: &CombatState) -> CombatMetaKey {
    let meta = &combat.meta;
    CombatMetaKey {
        ascension_level: meta.ascension_level,
        player_class: meta.player_class,
        is_boss_fight: meta.is_boss_fight,
        is_elite_fight: meta.is_elite_fight,
        master_deck_snapshot: meta.master_deck_snapshot.iter().map(card_key).collect(),
        meta_changes: meta.meta_changes.iter().map(meta_change_key).collect(),
    }
}

fn meta_change_key(change: &MetaChange) -> CombatMetaChangeKey {
    match change {
        MetaChange::AddCardToMasterDeck(card_id) => {
            CombatMetaChangeKey::AddCardToMasterDeck(*card_id)
        }
        MetaChange::ModifyCardMisc { card_uuid, amount } => CombatMetaChangeKey::ModifyCardMisc {
            card_uuid: *card_uuid,
            amount: *amount,
        },
        MetaChange::UpgradeMasterDeckCard { card_uuid } => {
            CombatMetaChangeKey::UpgradeMasterDeckCard {
                card_uuid: *card_uuid,
            }
        }
    }
}
