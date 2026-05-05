use super::{attack_actions, set_next_move_action, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    AttackSpec, AttackStep, DamageKind, MonsterMoveSpec, MonsterTurnPlan, MoveStep, MoveTarget,
};

const DOPE_MAGIC: u8 = 1;
const CHARGE: u8 = 2;
const ESCAPE: u8 = 99;

pub struct GremlinWizard;

enum GremlinWizardTurn<'a> {
    Charge,
    DopeMagic(&'a AttackSpec),
    Escape,
}

fn magic_damage(asc: u8) -> i32 {
    if asc >= 2 {
        30
    } else {
        25
    }
}

fn charge_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        CHARGE,
        smallvec::smallvec![MoveStep::Charge],
        MonsterMoveSpec::Unknown,
    )
}

fn dope_magic_plan(asc: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::single(
        DOPE_MAGIC,
        MoveStep::Attack(AttackStep {
            target: MoveTarget::Player,
            attack: AttackSpec {
                base_damage: magic_damage(asc),
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
        CHARGE => charge_plan(),
        DOPE_MAGIC => dope_magic_plan(asc),
        ESCAPE => escape_plan(),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn consecutive_charges(entity: &MonsterEntity) -> usize {
    entity
        .move_history()
        .iter()
        .rev()
        .take_while(|move_id| **move_id == CHARGE)
        .count()
}

fn decode_turn<'a>(plan: &'a MonsterTurnPlan) -> GremlinWizardTurn<'a> {
    match (plan.move_id, plan.steps.as_slice()) {
        (CHARGE, [MoveStep::Charge]) => GremlinWizardTurn::Charge,
        (
            DOPE_MAGIC,
            [MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            })],
        ) => GremlinWizardTurn::DopeMagic(attack),
        (ESCAPE, [MoveStep::Escape]) => GremlinWizardTurn::Escape,
        (_, []) => panic!("gremlin wizard plan missing locked truth"),
        (move_id, steps) => panic!(
            "gremlin wizard plan/steps mismatch: {} {:?}",
            move_id, steps
        ),
    }
}

impl MonsterBehavior for GremlinWizard {
    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        _entity: &MonsterEntity,
        _asc: u8,
        _num: i32,
    ) -> MonsterTurnPlan {
        charge_plan()
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
            GremlinWizardTurn::Charge => {
                let next_plan = if consecutive_charges(entity) >= 2 {
                    dope_magic_plan(state.meta.ascension_level)
                } else {
                    charge_plan()
                };
                vec![set_next_move_action(entity, next_plan)]
            }
            GremlinWizardTurn::DopeMagic(attack) => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                let next_plan = if state.meta.ascension_level >= 17 {
                    dope_magic_plan(state.meta.ascension_level)
                } else {
                    charge_plan()
                };
                actions.push(set_next_move_action(entity, next_plan));
                actions
            }
            GremlinWizardTurn::Escape => vec![Action::Escape { target: entity.id }],
        }
    }
}
