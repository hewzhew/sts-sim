// Pain triggers whenever you play ANOTHER card. We will hook this directly in the player input sequence where cards are played.
pub fn on_other_card_played() -> crate::action::ActionInfo {
    crate::action::ActionInfo {
        action: crate::action::Action::LoseHp {
            target: 0,
            amount: 1,
            triggers_rupture: true,
        },
        insertion_mode: crate::action::AddTo::Top,
    }
}

#[cfg(test)]
mod tests {
    use super::on_other_card_played;
    use crate::action::{Action, AddTo};

    #[test]
    fn pain_self_damage_triggers_rupture() {
        let action = on_other_card_played();
        assert_eq!(action.insertion_mode, AddTo::Top);
        assert!(matches!(
            action.action,
            Action::LoseHp {
                target: 0,
                amount: 1,
                triggers_rupture: true,
            }
        ));
    }
}
