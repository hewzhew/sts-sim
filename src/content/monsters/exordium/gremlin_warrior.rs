use super::{apply_power_action, attack_actions, set_next_move_action, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    ApplyPowerStep, AttackSpec, AttackStep, DamageKind, EffectStrength, MonsterTurnPlan, MoveStep,
    MoveTarget, PowerEffectKind,
};

const SCRATCH: u8 = 1;
const ESCAPE: u8 = 99;

pub struct GremlinWarrior;

enum GremlinWarriorTurn<'a> {
    Scratch(&'a AttackSpec),
    Escape,
}

fn scratch_damage(asc: u8) -> i32 {
    if asc >= 2 {
        5
    } else {
        4
    }
}

fn angry_amount(asc: u8) -> i32 {
    if asc >= 17 {
        2
    } else {
        1
    }
}

fn scratch_plan(asc: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::single(
        SCRATCH,
        MoveStep::Attack(AttackStep {
            target: MoveTarget::Player,
            attack: AttackSpec {
                base_damage: scratch_damage(asc),
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
        SCRATCH => scratch_plan(asc),
        ESCAPE => escape_plan(),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn decode_turn<'a>(plan: &'a MonsterTurnPlan) -> GremlinWarriorTurn<'a> {
    match (plan.move_id, plan.steps.as_slice()) {
        (
            SCRATCH,
            [MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            })],
        ) => GremlinWarriorTurn::Scratch(attack),
        (ESCAPE, [MoveStep::Escape]) => GremlinWarriorTurn::Escape,
        (_, []) => panic!("gremlin warrior plan missing locked truth"),
        (move_id, steps) => panic!(
            "gremlin warrior plan/steps mismatch: {} {:?}",
            move_id, steps
        ),
    }
}

impl MonsterBehavior for GremlinWarrior {
    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        _entity: &MonsterEntity,
        asc: u8,
        _num: i32,
    ) -> MonsterTurnPlan {
        scratch_plan(asc)
    }

    fn turn_plan(state: &CombatState, entity: &MonsterEntity) -> MonsterTurnPlan {
        plan_for(entity.planned_move_id(), state.meta.ascension_level)
    }

    fn use_pre_battle_actions(
        state: &mut CombatState,
        entity: &MonsterEntity,
        legacy_rng: crate::content::monsters::PreBattleLegacyRng,
    ) -> Vec<Action> {
        let (_hp_rng, asc) = crate::content::monsters::legacy_pre_battle_rng(state, legacy_rng);
        vec![apply_power_action(
            entity,
            &ApplyPowerStep {
                target: MoveTarget::SelfTarget,
                power_id: PowerId::Anger,
                amount: angry_amount(asc),
                effect: PowerEffectKind::Buff,
                visible_strength: EffectStrength::Normal,
            },
        )]
    }

    fn take_turn_plan(
        state: &mut CombatState,
        entity: &MonsterEntity,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        match decode_turn(plan) {
            GremlinWarriorTurn::Scratch(attack) => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                actions.push(set_next_move_action(
                    entity,
                    scratch_plan(state.meta.ascension_level),
                ));
                actions
            }
            GremlinWarriorTurn::Escape => vec![Action::Escape { target: entity.id }],
        }
    }
}
