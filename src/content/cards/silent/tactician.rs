use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use smallvec::SmallVec;

pub fn tactician_play(_state: &CombatState, _card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    smallvec::smallvec![]
}

pub fn tactician_manual_discard(card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let def = crate::content::cards::get_card_definition(card.id);
    let upgraded = if card.upgrades > 0 { 1 } else { 0 };
    let magic = def.base_magic + upgraded * def.upgrade_magic;
    smallvec::smallvec![ActionInfo {
        action: Action::GainEnergy {
            amount: magic.max(0)
        },
        insertion_mode: AddTo::Top,
    }]
}
