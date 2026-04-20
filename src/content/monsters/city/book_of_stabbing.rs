use crate::content::monsters::exordium::{attack_actions, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::{Action, MonsterRuntimePatch};
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    AttackSpec, AttackStep, DamageKind, MonsterMoveSpec, MonsterTurnPlan, MoveStep, MoveTarget,
};

pub struct BookOfStabbing;

const STAB: u8 = 1;
const BIG_STAB: u8 = 2;

fn stab_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 3 {
        7
    } else {
        6
    }
}

fn big_stab_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 3 {
        24
    } else {
        21
    }
}

fn current_stab_count(entity: &MonsterEntity) -> u8 {
    assert!(
        entity.book_of_stabbing.protocol_seeded,
        "book of stabbing runtime truth must be protocol-seeded or factory-seeded"
    );
    entity.book_of_stabbing.stab_count
}

fn stab_plan(ascension_level: u8, stab_count: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        STAB,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: stab_damage(ascension_level),
            hits: stab_count,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn big_stab_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        BIG_STAB,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: big_stab_damage(ascension_level),
            hits: 1,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn plan_for(move_id: u8, ascension_level: u8, stab_count: u8) -> MonsterTurnPlan {
    match move_id {
        STAB => stab_plan(ascension_level, stab_count),
        BIG_STAB => big_stab_plan(ascension_level),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn last_move(entity: &MonsterEntity, move_id: u8) -> bool {
    entity.move_history().back().copied() == Some(move_id)
}

fn last_two_moves(entity: &MonsterEntity, move_id: u8) -> bool {
    let mut history = entity.move_history().iter().rev();
    matches!(
        (history.next().copied(), history.next().copied()),
        (Some(a), Some(b)) if a == move_id && b == move_id
    )
}

fn next_stab_count(entity: &MonsterEntity, ascension_level: u8, plan: &MonsterTurnPlan) -> u8 {
    let current = current_stab_count(entity);
    match plan.move_id {
        STAB => current.saturating_add(1),
        BIG_STAB if ascension_level >= 18 => current.saturating_add(1),
        BIG_STAB => current,
        _ => current,
    }
}

fn book_runtime_update(entity: &MonsterEntity, stab_count: u8) -> Action {
    Action::UpdateMonsterRuntime {
        monster_id: entity.id,
        patch: MonsterRuntimePatch::BookOfStabbing {
            stab_count: Some(stab_count),
            protocol_seeded: Some(true),
        },
    }
}

impl MonsterBehavior for BookOfStabbing {
    fn use_pre_battle_actions(
        state: &mut CombatState,
        entity: &MonsterEntity,
        legacy_rng: crate::content::monsters::PreBattleLegacyRng,
    ) -> Vec<Action> {
        let (_rng, _ascension_level) =
            crate::content::monsters::legacy_pre_battle_rng(state, legacy_rng);
        vec![Action::ApplyPower {
            source: entity.id,
            target: entity.id,
            power_id: PowerId::PainfulStabs,
            amount: 1,
        }]
    }

    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> MonsterTurnPlan {
        let stab_count = current_stab_count(entity);
        if num < 15 {
            if last_move(entity, BIG_STAB) {
                return stab_plan(ascension_level, stab_count.saturating_add(1));
            }
            return big_stab_plan(ascension_level);
        }
        if last_two_moves(entity, STAB) {
            return big_stab_plan(ascension_level);
        }
        stab_plan(ascension_level, stab_count.saturating_add(1))
    }

    fn on_roll_move(
        ascension_level: u8,
        entity: &MonsterEntity,
        _num: i32,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        vec![book_runtime_update(
            entity,
            next_stab_count(entity, ascension_level, plan),
        )]
    }

    fn turn_plan(state: &CombatState, entity: &MonsterEntity) -> MonsterTurnPlan {
        plan_for(
            entity.planned_move_id(),
            state.meta.ascension_level,
            current_stab_count(entity),
        )
    }

    fn take_turn_plan(
        state: &mut CombatState,
        entity: &MonsterEntity,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        let mut actions = match (plan.move_id, plan.steps.as_slice()) {
            (
                STAB,
                [MoveStep::Attack(AttackStep {
                    target: MoveTarget::Player,
                    attack,
                })],
            ) => attack_actions(entity.id, PLAYER, attack),
            (
                BIG_STAB,
                [MoveStep::Attack(AttackStep {
                    target: MoveTarget::Player,
                    attack,
                })],
            ) => attack_actions(entity.id, PLAYER, attack),
            (_, []) => panic!("book of stabbing plan missing locked truth"),
            (move_id, steps) => panic!(
                "book of stabbing plan/steps mismatch: {} {:?}",
                move_id, steps
            ),
        };

        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        let _ = state;
        actions
    }
}
