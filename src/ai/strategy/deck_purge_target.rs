use crate::content::cards::{
    get_card_definition, is_starter_basic, is_starter_defend, is_starter_strike, CardId, CardType,
};
use crate::runtime::combat::CombatCard;
use crate::state::run::RunState;
use crate::state::selection::SelectionTargetRef;

pub fn best_purge_uuid(run_state: &RunState, targets: &[SelectionTargetRef]) -> Option<u32> {
    targets
        .iter()
        .filter_map(|target| {
            let uuid = target.card_uuid();
            let card = run_state
                .master_deck
                .iter()
                .find(|card| card.uuid == uuid)?;
            Some((rank_purge_target(card), uuid))
        })
        .min_by_key(|(rank, _)| *rank)
        .map(|(_, uuid)| uuid)
}

pub fn rank_purge_target(card: &CombatCard) -> u8 {
    if is_non_parasite_curse(card) {
        0
    } else if is_starter_strike(card.id) {
        1
    } else if card.id == CardId::Parasite {
        2
    } else if is_starter_defend(card.id) {
        3
    } else if is_starter_basic(card.id) {
        4
    } else {
        5
    }
}

pub fn purge_rank_label(rank: u8) -> &'static str {
    match rank {
        0 => "curse",
        1 => "strike",
        2 => "parasite",
        3 => "defend",
        4 => "starter",
        _ => "no_safe_target",
    }
}

fn is_non_parasite_curse(card: &CombatCard) -> bool {
    get_card_definition(card.id).card_type == CardType::Curse && card.id != CardId::Parasite
}
