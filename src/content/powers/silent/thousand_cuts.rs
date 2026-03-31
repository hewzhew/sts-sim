use crate::action::Action;
use crate::core::EntityId;

pub fn on_after_card_played(owner: EntityId, amount: i32) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::SmallVec::new();
    if amount > 0 {
        // Java: ThousandCutsPower.onAfterCardPlayed
        // addToBot(new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(this.amount, true), DamageInfo.DamageType.NORMAL, AbstractGameAction.AttackEffect.NONE));
        actions.push(Action::DamageAllEnemies {
            source: owner,
            damages: smallvec::smallvec![amount, amount, amount, amount, amount],
            damage_type: crate::action::DamageType::Normal,
            is_modified: false,
        });
    }
    actions
}
