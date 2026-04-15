use crate::runtime::action::ActionInfo;
use crate::runtime::combat::CombatState;
use smallvec::SmallVec;

pub fn at_battle_start(state: &CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();
    let mut is_boss_combat = false;
    for m in &state.entities.monsters {
        if let Some(enemy_id) = crate::content::monsters::EnemyId::from_id(m.monster_type) {
            if matches!(
                enemy_id,
                crate::content::monsters::EnemyId::SlimeBoss
                    | crate::content::monsters::EnemyId::Hexaghost
                    | crate::content::monsters::EnemyId::TheGuardian
                    | crate::content::monsters::EnemyId::BronzeAutomaton
                    | crate::content::monsters::EnemyId::TheCollector
                    | crate::content::monsters::EnemyId::Champ
                    | crate::content::monsters::EnemyId::AwakenedOne
                    | crate::content::monsters::EnemyId::TimeEater
                    | crate::content::monsters::EnemyId::Donu
                    | crate::content::monsters::EnemyId::Deca
                    | crate::content::monsters::EnemyId::CorruptHeart
            ) {
                is_boss_combat = true;
                break;
            }
        }
    }
    if is_boss_combat {
        actions.push(ActionInfo {
            action: crate::runtime::action::Action::Heal {
                target: 0,
                amount: 25,
            },
            insertion_mode: crate::runtime::action::AddTo::Bottom,
        });
    }
    actions
}
