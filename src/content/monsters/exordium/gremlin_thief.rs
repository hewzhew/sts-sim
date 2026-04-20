use super::{attack_actions, set_next_move_action, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    AttackSpec, AttackStep, DamageKind, MonsterTurnPlan, MoveStep, MoveTarget,
};

const PUNCTURE: u8 = 1;
const ESCAPE: u8 = 99;

pub struct GremlinThief;

enum GremlinThiefTurn<'a> {
    Puncture(&'a AttackSpec),
    Escape,
}

fn puncture_damage(asc: u8) -> i32 {
    if asc >= 2 {
        10
    } else {
        9
    }
}

fn puncture_plan(asc: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::single(
        PUNCTURE,
        MoveStep::Attack(AttackStep {
            target: MoveTarget::Player,
            attack: AttackSpec {
                base_damage: puncture_damage(asc),
                hits: 1,
                damage_kind: DamageKind::Normal,
            },
        }),
    )
}

fn escape_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::single(ESCAPE, MoveStep::Escape)
}

fn plan_for(move_id: u8, asc: u8) -> MonsterTurnPlan {
    match move_id {
        PUNCTURE => puncture_plan(asc),
        ESCAPE => escape_plan(),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn decode_turn<'a>(plan: &'a MonsterTurnPlan) -> GremlinThiefTurn<'a> {
    match (plan.move_id, plan.steps.as_slice()) {
        (
            PUNCTURE,
            [MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            })],
        ) => GremlinThiefTurn::Puncture(attack),
        (ESCAPE, [MoveStep::Escape]) => GremlinThiefTurn::Escape,
        (_, []) => panic!("gremlin thief plan missing locked truth"),
        (move_id, steps) => panic!("gremlin thief plan/steps mismatch: {} {:?}", move_id, steps),
    }
}

impl MonsterBehavior for GremlinThief {
    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        _entity: &MonsterEntity,
        asc: u8,
        _num: i32,
    ) -> MonsterTurnPlan {
        puncture_plan(asc)
    }

    fn turn_plan(state: &CombatState, entity: &MonsterEntity) -> MonsterTurnPlan {
        plan_for(entity.planned_move_id(), state.meta.ascension_level)
    }

    fn take_turn_plan(
        state: &mut CombatState,
        entity: &MonsterEntity,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        match decode_turn(plan) {
            GremlinThiefTurn::Puncture(attack) => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                actions.push(set_next_move_action(
                    entity,
                    puncture_plan(state.meta.ascension_level),
                ));
                actions
            }
            GremlinThiefTurn::Escape => vec![Action::Escape { target: entity.id }],
        }
    }
}
