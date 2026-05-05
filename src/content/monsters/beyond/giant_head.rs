use crate::content::monsters::exordium::{apply_power_action, attack_actions, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    AttackSpec, DamageKind, DebuffSpec, EffectStrength, MonsterMoveSpec, MonsterTurnPlan, MoveStep,
};

pub struct GiantHead;

const GLARE: u8 = 1;
const IT_IS_TIME: u8 = 2;
const COUNT: u8 = 3;

fn starting_death_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 3 {
        40
    } else {
        30
    }
}

fn starting_count(ascension_level: u8) -> i32 {
    if ascension_level >= 18 {
        4
    } else {
        5
    }
}

fn count_before_roll(entity: &MonsterEntity, ascension_level: u8) -> i32 {
    (starting_count(ascension_level) - entity.move_history().len() as i32).max(-6)
}

fn count_after_selection(entity: &MonsterEntity, ascension_level: u8) -> i32 {
    let count = count_before_roll(entity, ascension_level);
    if count > 1 {
        count - 1
    } else if count > -6 {
        count - 1
    } else {
        -6
    }
}

fn it_is_time_damage(entity: &MonsterEntity, ascension_level: u8) -> i32 {
    starting_death_damage(ascension_level) - (count_after_selection(entity, ascension_level) * 5)
}

fn glare_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        GLARE,
        MonsterMoveSpec::Debuff(DebuffSpec {
            power_id: PowerId::Weak,
            amount: 1,
            strength: EffectStrength::Normal,
        }),
    )
}

fn count_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        COUNT,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: 13,
            hits: 1,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn it_is_time_plan(entity: &MonsterEntity, ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        IT_IS_TIME,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: it_is_time_damage(entity, ascension_level),
            hits: 1,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn plan_for(entity: &MonsterEntity, ascension_level: u8, move_id: u8) -> MonsterTurnPlan {
    match move_id {
        GLARE => glare_plan(),
        COUNT => count_plan(),
        IT_IS_TIME => it_is_time_plan(entity, ascension_level),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn last_two_moves(entity: &MonsterEntity, move_id: u8) -> bool {
    let history = entity.move_history();
    history.len() >= 2
        && history[history.len() - 1] == move_id
        && history[history.len() - 2] == move_id
}

impl MonsterBehavior for GiantHead {
    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> MonsterTurnPlan {
        if count_before_roll(entity, ascension_level) <= 1 {
            return it_is_time_plan(entity, ascension_level);
        }

        if num < 50 {
            if !last_two_moves(entity, GLARE) {
                glare_plan()
            } else {
                count_plan()
            }
        } else if !last_two_moves(entity, COUNT) {
            count_plan()
        } else {
            glare_plan()
        }
    }

    fn use_pre_battle_actions(
        state: &mut CombatState,
        entity: &MonsterEntity,
        legacy_rng: crate::content::monsters::PreBattleLegacyRng,
    ) -> Vec<Action> {
        let (_hp_rng, _ascension_level) =
            crate::content::monsters::legacy_pre_battle_rng(state, legacy_rng);
        vec![Action::ApplyPower {
            source: entity.id,
            target: entity.id,
            power_id: PowerId::Slow,
            amount: 0,
        }]
    }

    fn turn_plan(state: &CombatState, entity: &MonsterEntity) -> MonsterTurnPlan {
        plan_for(entity, state.meta.ascension_level, entity.planned_move_id())
    }

    fn take_turn_plan(
        _state: &mut CombatState,
        entity: &MonsterEntity,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        let mut actions = match (plan.move_id, plan.steps.as_slice()) {
            (GLARE, [MoveStep::ApplyPower(power)]) => vec![apply_power_action(entity, power)],
            (COUNT | IT_IS_TIME, [MoveStep::Attack(attack)]) => {
                attack_actions(entity.id, PLAYER, &attack.attack)
            }
            (move_id, steps) => panic!("giant head plan/steps mismatch: {} {:?}", move_id, steps),
        };
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
