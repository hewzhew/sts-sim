use crate::runtime::action::{Action, ActionInfo, AddTo};
use crate::runtime::combat::CombatState;
use smallvec::SmallVec;

/// Preserved Insect: Enemies in Elite rooms have 25% less HP.
/// Applies immediately as a passive modifier to monster generation (handled elsewhere)
/// or can dispatch an immediate damage chunk if evaluated at battle start in Elite rooms.
/// Handling it as an immediate max health reduction.

pub fn at_battle_start(state: &CombatState) -> SmallVec<[ActionInfo; 4]> {
    let mut actions = SmallVec::new();

    // Check if the combat involves any elite. Slay the spire applies this to ALL
    // monsters in an elite room, so if there's an elite, everyone's HP goes down.
    let mut is_elite_combat = false;
    for m in &state.entities.monsters {
        if let Some(enemy_id) = crate::content::monsters::EnemyId::from_id(m.monster_type) {
            if matches!(
                enemy_id,
                crate::content::monsters::EnemyId::GremlinNob
                    | crate::content::monsters::EnemyId::Lagavulin
                    | crate::content::monsters::EnemyId::Sentry
                    | crate::content::monsters::EnemyId::GremlinLeader
                    | crate::content::monsters::EnemyId::BookOfStabbing
                    | crate::content::monsters::EnemyId::Taskmaster
                    | crate::content::monsters::EnemyId::GiantHead
                    | crate::content::monsters::EnemyId::Nemesis
                    | crate::content::monsters::EnemyId::Reptomancer
                    | crate::content::monsters::EnemyId::SpireShield
                    | crate::content::monsters::EnemyId::SpireSpear
            ) {
                is_elite_combat = true;
                break;
            }
        }
    }

    if is_elite_combat {
        for monster in &state.entities.monsters {
            let reduction = (monster.max_hp as f32 * 0.25).floor() as i32;
            if reduction > 0 {
                actions.push(ActionInfo {
                    action: Action::LoseMaxHp {
                        target: monster.id,
                        amount: reduction,
                    },
                    insertion_mode: AddTo::Top,
                });
            }
        }
    }

    actions
}
