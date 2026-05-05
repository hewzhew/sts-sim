use super::{attack_actions, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    ApplyPowerStep, AttackSpec, AttackStep, DamageKind, EffectStrength, MonsterTurnPlan, MoveStep,
    MoveTarget, PowerEffectKind,
};

pub struct Cultist;

const ATTACK: u8 = 1;
const RITUAL: u8 = 3;

fn ritual_amount(asc: u8) -> i32 {
    if asc >= 17 {
        5
    } else if asc >= 2 {
        4
    } else {
        3
    }
}

fn attack_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::single(
        ATTACK,
        MoveStep::Attack(AttackStep {
            target: MoveTarget::Player,
            attack: AttackSpec {
                base_damage: 6,
                hits: 1,
                damage_kind: DamageKind::Normal,
            },
        }),
    )
}

fn ritual_plan(asc: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::single(
        RITUAL,
        MoveStep::ApplyPower(ApplyPowerStep {
            target: MoveTarget::SelfTarget,
            power_id: PowerId::Ritual,
            amount: ritual_amount(asc),
            effect: PowerEffectKind::Buff,
            visible_strength: EffectStrength::Normal,
        }),
    )
}

fn plan_for(move_id: u8, asc: u8) -> MonsterTurnPlan {
    match move_id {
        ATTACK => attack_plan(),
        RITUAL => ritual_plan(asc),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

impl MonsterBehavior for Cultist {
    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        asc: u8,
        _num: i32,
    ) -> MonsterTurnPlan {
        if entity.move_history().is_empty() {
            ritual_plan(asc)
        } else {
            attack_plan()
        }
    }

    fn turn_plan(state: &CombatState, entity: &MonsterEntity) -> MonsterTurnPlan {
        plan_for(entity.planned_move_id(), state.meta.ascension_level)
    }

    fn take_turn_plan(
        _state: &mut CombatState,
        entity: &MonsterEntity,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        match plan.steps.as_slice() {
            [MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::SelfTarget,
                power_id: PowerId::Ritual,
                amount,
                effect: PowerEffectKind::Buff,
                ..
            })] => vec![
                Action::ApplyPowerDetailed {
                    source: entity.id,
                    target: entity.id,
                    power_id: PowerId::Ritual,
                    amount: *amount,
                    instance_id: None,
                    extra_data: Some(crate::content::powers::core::ritual::extra_data(
                        false, true,
                    )),
                },
                Action::RollMonsterMove {
                    monster_id: entity.id,
                },
            ],
            [MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            })] => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                actions.push(Action::RollMonsterMove {
                    monster_id: entity.id,
                });
                actions
            }
            [] => panic!("cultist plan missing locked truth"),
            steps => panic!("cultist plan/steps mismatch: {:?}", steps),
        }
    }
}
