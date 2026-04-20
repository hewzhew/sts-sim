use super::{apply_power_action, attack_actions, set_next_move_action, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    ApplyPowerStep, AttackSpec, AttackStep, DamageKind, DebuffSpec, EffectStrength,
    MonsterMoveSpec, MonsterTurnPlan, MoveStep, MoveTarget, PowerEffectKind,
};
use smallvec::smallvec;

const BLUNT: u8 = 2;
const ESCAPE: u8 = 99;

pub struct GremlinFat;

enum GremlinFatTurn<'a> {
    Blunt(
        &'a AttackSpec,
        &'a ApplyPowerStep,
        Option<&'a ApplyPowerStep>,
    ),
    Escape,
}

fn blunt_damage(asc: u8) -> i32 {
    if asc >= 2 {
        5
    } else {
        4
    }
}

fn blunt_plan(asc: u8) -> MonsterTurnPlan {
    let mut steps = smallvec![
        MoveStep::Attack(AttackStep {
            target: MoveTarget::Player,
            attack: AttackSpec {
                base_damage: blunt_damage(asc),
                hits: 1,
                damage_kind: DamageKind::Normal,
            },
        }),
        MoveStep::ApplyPower(ApplyPowerStep {
            target: MoveTarget::Player,
            power_id: PowerId::Weak,
            amount: 1,
            effect: PowerEffectKind::Debuff,
            visible_strength: EffectStrength::Normal,
        }),
    ];
    if asc >= 17 {
        steps.push(MoveStep::ApplyPower(ApplyPowerStep {
            target: MoveTarget::Player,
            power_id: PowerId::Frail,
            amount: 1,
            effect: PowerEffectKind::Debuff,
            visible_strength: EffectStrength::Normal,
        }));
    }
    MonsterTurnPlan::with_visible_spec(
        BLUNT,
        steps,
        MonsterMoveSpec::AttackDebuff(
            AttackSpec {
                base_damage: blunt_damage(asc),
                hits: 1,
                damage_kind: DamageKind::Normal,
            },
            DebuffSpec {
                power_id: PowerId::Weak,
                amount: 1,
                strength: EffectStrength::Normal,
            },
        ),
    )
}

fn escape_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::single(ESCAPE, MoveStep::Escape)
}

fn plan_for(move_id: u8, asc: u8) -> MonsterTurnPlan {
    match move_id {
        BLUNT => blunt_plan(asc),
        ESCAPE => escape_plan(),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn decode_turn<'a>(plan: &'a MonsterTurnPlan) -> GremlinFatTurn<'a> {
    match (plan.move_id, plan.steps.as_slice()) {
        (
            BLUNT,
            [MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            }), MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::Player,
                power_id: PowerId::Weak,
                effect: PowerEffectKind::Debuff,
                ..
            })],
        ) => {
            let MoveStep::ApplyPower(weak) = &plan.steps[1] else {
                unreachable!()
            };
            GremlinFatTurn::Blunt(attack, weak, None)
        }
        (
            BLUNT,
            [MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            }), MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::Player,
                power_id: PowerId::Weak,
                effect: PowerEffectKind::Debuff,
                ..
            }), MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::Player,
                power_id: PowerId::Frail,
                effect: PowerEffectKind::Debuff,
                ..
            })],
        ) => {
            let MoveStep::ApplyPower(weak) = &plan.steps[1] else {
                unreachable!()
            };
            let MoveStep::ApplyPower(frail) = &plan.steps[2] else {
                unreachable!()
            };
            GremlinFatTurn::Blunt(attack, weak, Some(frail))
        }
        (ESCAPE, [MoveStep::Escape]) => GremlinFatTurn::Escape,
        (_, []) => panic!("gremlin fat plan missing locked truth"),
        (move_id, steps) => panic!("gremlin fat plan/steps mismatch: {} {:?}", move_id, steps),
    }
}

impl MonsterBehavior for GremlinFat {
    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        _entity: &MonsterEntity,
        asc: u8,
        _num: i32,
    ) -> MonsterTurnPlan {
        blunt_plan(asc)
    }

    fn turn_plan(state: &CombatState, entity: &MonsterEntity) -> MonsterTurnPlan {
        plan_for(entity.planned_move_id(), state.meta.ascension_level)
    }

    fn take_turn_plan(
        _state: &mut CombatState,
        entity: &MonsterEntity,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        match decode_turn(plan) {
            GremlinFatTurn::Blunt(attack, weak, frail) => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                actions.push(apply_power_action(entity, weak));
                if let Some(frail) = frail {
                    actions.push(apply_power_action(entity, frail));
                }
                actions.push(set_next_move_action(
                    entity,
                    blunt_plan(_state.meta.ascension_level),
                ));
                actions
            }
            GremlinFatTurn::Escape => vec![Action::Escape { target: entity.id }],
        }
    }
}
