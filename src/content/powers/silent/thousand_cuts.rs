use crate::core::EntityId;
use crate::runtime::action::{repeated_damage_matrix, Action};
use crate::runtime::combat::CombatState;

pub fn on_after_card_played(
    state: &CombatState,
    owner: EntityId,
    amount: i32,
) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::SmallVec::new();
    if amount > 0 {
        // Java: ThousandCutsPower.onAfterCardPlayed
        // addToBot(new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(this.amount, true), DamageInfo.DamageType.NORMAL, AbstractGameAction.AttackEffect.NONE));
        actions.push(Action::DamageAllEnemies {
            source: owner,
            damages: repeated_damage_matrix(state.entities.monsters.len(), amount),
            damage_type: crate::runtime::action::DamageType::Normal,
            is_modified: false,
        });
    }
    actions
}
