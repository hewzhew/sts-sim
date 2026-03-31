// Pain triggers whenever you play ANOTHER card. We will hook this directly in the player input sequence where cards are played.
pub fn on_other_card_played() -> crate::action::ActionInfo {
    crate::action::ActionInfo {
        action: crate::action::Action::Damage(crate::action::DamageInfo {
            source: 0, // Player
            target: 0, // Player
            base: 1,
            output: 1,
            damage_type: crate::action::DamageType::HpLoss,
            is_modified: false,
        }),
        insertion_mode: crate::action::AddTo::Bottom,
    }
}
