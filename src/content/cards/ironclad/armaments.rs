use crate::action::{Action, ActionInfo, AddTo};
use crate::combat::{CombatCard, CombatState};
use crate::state::HandSelectFilter;
use crate::state::HandSelectReason;
use smallvec::SmallVec;

pub fn armaments_play(state: &CombatState, _card: &CombatCard) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = smallvec::SmallVec::new();
    // Armaments provides Block explicitly.
    actions.push(ActionInfo {
        action: Action::GainBlock {
            target: 0,
            amount: _card.base_block_mut,
        },
        insertion_mode: AddTo::Bottom,
    });

    // Armaments Upgrade logic
    if _card.upgrades > 0 {
        // Upgrade all
        for c in &state.zones.hand {
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
            .filter(|c| {
                c.upgrades == 0
                    && crate::content::cards::get_card_definition(c.id).card_type
                        != crate::content::cards::CardType::Status
            })
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
