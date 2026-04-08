use crate::action::Action;
use crate::combat::CombatState;
use crate::content::powers::PowerId;
use crate::core::EntityId;

pub fn on_monster_turn_ended(
    _state: &CombatState,
    owner: EntityId,
    power_amount: i32,
) -> smallvec::SmallVec<[Action; 2]> {
    smallvec::smallvec![Action::GainBlock {
        target: owner,
        amount: power_amount,
    }]
}

pub fn on_attacked(
    _state: &CombatState,
    owner: EntityId,
    amount: i32,
    _power_amount: i32,
) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::smallvec![];
    if amount > 0 {
        actions.push(Action::ApplyPower {
            source: owner,
            target: owner,
            power_id: PowerId::PlatedArmor,
            amount: -1,
        });
    }
    actions
}

pub fn on_remove(_state: &CombatState, owner: EntityId) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::smallvec![];
    // Trigger `ARMOR_BREAK` for Shelled Parasite
    if let Some(monster) = _state.monsters.iter().find(|m| m.id == owner) {
        if monster.monster_type == crate::content::monsters::EnemyId::ShelledParasite as usize {
            actions.push(Action::SetMonsterMove {
                monster_id: owner,
                next_move_byte: 4,
                intent: crate::combat::Intent::Stun,
            });
        }
    }
    actions
}
