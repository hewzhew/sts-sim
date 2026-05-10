use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use crate::state::HandSelectFilter;
use crate::state::HandSelectReason;
use smallvec::SmallVec;

fn can_armaments_upgrade(card: &CombatCard) -> bool {
    let def = crate::content::cards::get_card_definition(card.id);
    (card.id == crate::content::cards::CardId::SearingBlow || card.upgrades == 0)
        && def.card_type != crate::content::cards::CardType::Status
        && def.card_type != crate::content::cards::CardType::Curse
}

pub fn armaments_play(state: &CombatState, card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let evaluated = crate::content::cards::evaluate_card_for_play(card, state, None);
    let mut actions = smallvec::SmallVec::new();
    actions.push(ActionInfo {
        action: Action::GainBlock {
            target: 0,
            amount: evaluated.base_block_mut,
        },
        insertion_mode: AddTo::Bottom,
    });

    if card.upgrades > 0 {
        for c in state.zones.hand.iter().filter(|c| can_armaments_upgrade(c)) {
            actions.push(ActionInfo {
                action: Action::UpgradeCard { card_uuid: c.uuid },
                insertion_mode: AddTo::Bottom,
            });
        }
    } else {
        let upgradeable: Vec<_> = state
            .zones
            .hand
            .iter()
            .filter(|c| can_armaments_upgrade(c))
            .map(|c| c.uuid)
            .collect();
        if upgradeable.is_empty() {
            return actions;
        }
        if upgradeable.len() == 1 {
            actions.push(ActionInfo {
                action: Action::UpgradeCard {
                    card_uuid: upgradeable[0],
                },
                insertion_mode: AddTo::Bottom,
            });
            return actions;
        }
        // Upgrade one via hand select
        actions.push(ActionInfo {
            action: Action::SuspendForHandSelect {
                min: 1,
                max: 1,
                can_cancel: false,
                filter: HandSelectFilter::Upgradeable,
                reason: HandSelectReason::Upgrade,
            },
            insertion_mode: AddTo::Bottom,
        });
    }

    actions
}
