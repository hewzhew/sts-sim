use crate::content::monsters::exordium::{attack_actions, set_next_move_action, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{AttackSpec, DamageKind, MonsterMoveSpec, MonsterTurnPlan};

pub struct TorchHead;

const TACKLE: u8 = 1;
const TACKLE_DAMAGE: i32 = 7;

fn tackle_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        TACKLE,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: TACKLE_DAMAGE,
            hits: 1,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn plan_for(move_id: u8) -> MonsterTurnPlan {
    match move_id {
        TACKLE => tackle_plan(),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

impl MonsterBehavior for TorchHead {
    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        _entity: &MonsterEntity,
        _ascension_level: u8,
        _num: i32,
    ) -> MonsterTurnPlan {
        tackle_plan()
    }

    fn turn_plan(_state: &CombatState, entity: &MonsterEntity) -> MonsterTurnPlan {
        plan_for(entity.planned_move_id())
    }

    fn take_turn_plan(
        _state: &mut CombatState,
        entity: &MonsterEntity,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        match plan.move_id {
            TACKLE => {
                let mut actions = attack_actions(
                    entity.id,
                    PLAYER,
                    &AttackSpec {
                        base_damage: TACKLE_DAMAGE,
                        hits: 1,
                        damage_kind: DamageKind::Normal,
                    },
                );
                actions.push(set_next_move_action(entity, tackle_plan()));
                actions
            }
            _ => panic!(
                "torch head take_turn received unsupported move {}",
                plan.move_id
            ),
        }
    }
}
