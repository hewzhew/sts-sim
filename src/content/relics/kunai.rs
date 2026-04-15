use crate::action::{Action, ActionInfo, AddTo};
use smallvec::SmallVec;

/// Kunai: Every time you play 3 Attacks in a single turn, gain 1 Dexterity.

pub fn at_turn_start() -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    actions.push(ActionInfo {
        action: Action::UpdateRelicCounter {
            relic_id: crate::content::relics::RelicId::Kunai,
            counter: 0,
        },
        insertion_mode: AddTo::Bottom,
    });
    actions
}

pub fn on_use_card(
    card_id: crate::content::cards::CardId,
    counter: i32,
) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    let def = crate::content::cards::get_card_definition(card_id);

    if def.card_type == crate::content::cards::CardType::Attack {
        let next_counter = if counter + 1 >= 3 { 0 } else { counter + 1 };

        actions.push(ActionInfo {
            action: Action::UpdateRelicCounter {
                relic_id: crate::content::relics::RelicId::Kunai,
                counter: next_counter,
            },
            insertion_mode: AddTo::Bottom,
        });

        if next_counter == 0 {
            actions.push(ActionInfo {
                action: Action::ApplyPower {
                    source: 0,
                    target: 0,
                    power_id: crate::content::powers::PowerId::Dexterity,
                    amount: 1,
                },
                insertion_mode: AddTo::Bottom,
            });
        }
    }

    actions
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::cards::CardId;
    use crate::content::powers::PowerId;
    use crate::content::relics::RelicId;

    #[test]
    fn at_turn_start_resets_kunai_counter() {
        let actions = at_turn_start();

        assert_eq!(
            actions.as_slice(),
            &[ActionInfo {
                action: Action::UpdateRelicCounter {
                    relic_id: RelicId::Kunai,
                    counter: 0,
                },
                insertion_mode: AddTo::Bottom,
            }]
        );
    }

    #[test]
    fn third_attack_grants_dexterity_and_resets_counter() {
        let actions = on_use_card(CardId::Strike, 2);

        assert_eq!(actions.len(), 2);
        assert_eq!(
            actions[0].action,
            Action::UpdateRelicCounter {
                relic_id: RelicId::Kunai,
                counter: 0,
            }
        );
        assert_eq!(
            actions[1].action,
            Action::ApplyPower {
                source: 0,
                target: 0,
                power_id: PowerId::Dexterity,
                amount: 1,
            }
        );
    }
}
