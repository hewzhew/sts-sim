use super::{attack_actions, gain_block_random_monster_action, set_next_move_action, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    AttackSpec, AttackStep, DamageKind, DefendSpec, MonsterMoveSpec, MonsterTurnPlan, MoveStep,
    MoveTarget, RandomBlockStep,
};

const PROTECT: u8 = 1;
const BASH: u8 = 2;
const ESCAPE: u8 = 99;

pub struct GremlinTsundere;

enum GremlinTsundereTurn<'a> {
    Protect(&'a RandomBlockStep),
    Bash(&'a AttackSpec),
    Escape,
}

fn bash_damage(asc: u8) -> i32 {
    if asc >= 2 {
        8
    } else {
        6
    }
}

fn block_amount(asc: u8) -> i32 {
    if asc >= 17 {
        11
    } else if asc >= 7 {
        8
    } else {
        7
    }
}

fn protect_plan(asc: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        PROTECT,
        smallvec::smallvec![MoveStep::GainBlockRandomMonster(RandomBlockStep {
            amount: block_amount(asc),
        })],
        MonsterMoveSpec::Defend(DefendSpec {
            block: block_amount(asc),
        }),
    )
}

fn bash_plan(asc: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::single(
        BASH,
        MoveStep::Attack(AttackStep {
            target: MoveTarget::Player,
            attack: AttackSpec {
                base_damage: bash_damage(asc),
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
        PROTECT => protect_plan(asc),
        BASH => bash_plan(asc),
        ESCAPE => escape_plan(),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn decode_turn<'a>(plan: &'a MonsterTurnPlan) -> GremlinTsundereTurn<'a> {
    match (plan.move_id, plan.steps.as_slice()) {
        (PROTECT, [MoveStep::GainBlockRandomMonster(block)]) => GremlinTsundereTurn::Protect(block),
        (
            BASH,
            [MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            })],
        ) => GremlinTsundereTurn::Bash(attack),
        (ESCAPE, [MoveStep::Escape]) => GremlinTsundereTurn::Escape,
        (_, []) => panic!("gremlin tsundere plan missing locked truth"),
        (move_id, steps) => panic!(
            "gremlin tsundere plan/steps mismatch: {} {:?}",
            move_id, steps
        ),
    }
}

fn alive_monster_count(state: &CombatState) -> usize {
    state
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && monster.current_hp > 0)
        .count()
}

impl MonsterBehavior for GremlinTsundere {
    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        _entity: &MonsterEntity,
        asc: u8,
        _num: i32,
    ) -> MonsterTurnPlan {
        protect_plan(asc)
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
            GremlinTsundereTurn::Protect(block) => {
                let mut actions = vec![gain_block_random_monster_action(entity, block)];
                let next_plan = if alive_monster_count(state) > 1 {
                    protect_plan(state.meta.ascension_level)
                } else {
                    bash_plan(state.meta.ascension_level)
                };
                actions.push(set_next_move_action(entity, next_plan));
                actions
            }
            GremlinTsundereTurn::Bash(attack) => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                actions.push(set_next_move_action(
                    entity,
                    bash_plan(state.meta.ascension_level),
                ));
                actions
            }
            GremlinTsundereTurn::Escape => vec![Action::Escape { target: entity.id }],
        }
    }
}
