use crate::content::cards::{CardDefinition, CardId, CardRarity, CardTarget, CardType};
use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::{CombatCard, CombatState};
use crate::state::HandSelectFilter;
use crate::state::HandSelectReason;
use smallvec::SmallVec;

pub fn definition() -> CardDefinition {
    CardDefinition {
        id: CardId::Armaments,
        name: "Armaments",
        card_type: CardType::Skill,
        rarity: CardRarity::Common,
        cost: 1,
        base_damage: 0,
        base_block: 5,
        base_magic: 0,
        target: CardTarget::SelfTarget,
        is_multi_damage: false,
        exhaust: false,
        ethereal: false,
        innate: false,
        tags: &[],
        upgrade_damage: 0,
        upgrade_block: 0,
        upgrade_magic: 0,
    }
}

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
