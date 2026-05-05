use crate::content::powers::PowerId;
use crate::core::EntityId;
use crate::runtime::action::Action;
use crate::runtime::combat::CombatState;
use crate::semantics::combat::{MonsterTurnPlan, MoveStep};

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

pub fn on_hp_lost(
    _state: &CombatState,
    owner: EntityId,
    amount: i32,
    source: Option<EntityId>,
    damage_type: crate::runtime::action::DamageType,
) -> smallvec::SmallVec<[Action; 2]> {
    let mut actions = smallvec::smallvec![];
    if amount > 0
        && !matches!(
            damage_type,
            crate::runtime::action::DamageType::HpLoss | crate::runtime::action::DamageType::Thorns
        )
        && source.is_some()
        && source != Some(owner)
    {
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
    if let Some(monster) = _state.entities.monsters.iter().find(|m| m.id == owner) {
        if monster.monster_type == crate::content::monsters::EnemyId::ShelledParasite as usize {
            let plan = MonsterTurnPlan::single(4, MoveStep::Stun);
            actions.push(Action::SetMonsterMove {
                monster_id: owner,
                next_move_byte: plan.move_id,
                planned_steps: plan.steps,
                planned_visible_spec: plan.visible_spec,
            });
        }
    }
    actions
}
