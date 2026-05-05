use crate::content::monsters::EnemyId;
use crate::core::EntityId;
use crate::runtime::action::Action;
use crate::runtime::combat::CombatState;
use smallvec::{smallvec, SmallVec};

pub fn on_hp_lost(state: &CombatState, owner: EntityId, _amount: i32) -> SmallVec<[Action; 2]> {
    let mut actions = smallvec![];

    // Split triggers when HP drops to or below 50%
    if let Some(monster) = state.entities.monsters.iter().find(|m| m.id == owner) {
        if monster.current_hp <= monster.max_hp / 2 && monster.planned_move_id() != 3 {
            let plan = match EnemyId::from_id(monster.monster_type) {
                Some(EnemyId::AcidSlimeL) => {
                    crate::content::monsters::exordium::acid_slime::split_plan(EnemyId::AcidSlimeM)
                }
                Some(EnemyId::SpikeSlimeL) => {
                    crate::content::monsters::exordium::spike_slime::split_plan()
                }
                Some(EnemyId::SlimeBoss) => {
                    crate::content::monsters::exordium::slime_boss::split_plan()
                }
                _ => return actions,
            };
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
