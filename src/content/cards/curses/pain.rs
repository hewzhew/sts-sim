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

