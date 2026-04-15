use crate::action::{Action, ActionInfo, AddTo};
use crate::content::cards::{CardId, CardType};
use smallvec::SmallVec;

pub struct BlueCandle;

impl BlueCandle {
    pub fn on_use_card(card_id: CardId) -> SmallVec<[ActionInfo; 4]> {
        let mut actions = SmallVec::new();
        let def = crate::content::cards::get_card_definition(card_id);

        if def.card_type == CardType::Curse {
            // Java BlueCandle.onUseCard() uses LoseHPAction(player, player, 1, FIRE),
            // so this self-loss should trigger Rupture.
            actions.push(ActionInfo {
                action: Action::LoseHp {
                    target: 0,
                    amount: 1,
                    triggers_rupture: true,
                },
                insertion_mode: AddTo::Bottom,
            });
            // The card exhausts itself natively during resolution via properties typically,
            // but we can enforce it if engine requires. The engine's UseCard handler
            // will need to know to exhaust it. Let's make sure the engine supports it!
        }
        actions
    }
}

#[cfg(test)]
mod tests {
    use super::BlueCandle;
    use crate::action::{Action, AddTo};
    use crate::content::cards::CardId;

    #[test]
    fn blue_candle_curse_hp_loss_triggers_rupture() {
        let actions = BlueCandle::on_use_card(CardId::Pain);
        assert_eq!(actions.len(), 1);
        assert_eq!(actions[0].insertion_mode, AddTo::Bottom);
        assert!(matches!(
            actions[0].action,
            Action::LoseHp {
                target: 0,
                amount: 1,
                triggers_rupture: true,
            }
        ));
    }

    #[test]
    fn blue_candle_does_not_fire_for_non_curse_cards() {
        let actions = BlueCandle::on_use_card(CardId::Strike);
        assert!(actions.is_empty());
    }
}
