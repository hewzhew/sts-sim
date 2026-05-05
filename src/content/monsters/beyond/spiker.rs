use crate::content::monsters::exordium::{apply_power_action, attack_actions, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    AttackSpec, BuffSpec, DamageKind, MonsterMoveSpec, MonsterTurnPlan, MoveStep,
};

pub struct Spiker;

const ATTACK: u8 = 1;
const BUFF_THORNS: u8 = 2;

fn attack_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 2 {
        9
    } else {
        7
    }
}

fn starting_thorns(ascension_level: u8) -> i32 {
    let base = if ascension_level >= 2 { 4 } else { 3 };
    if ascension_level >= 17 {
        base + 3
    } else {
        base
    }
}

fn attack_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        ATTACK,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: attack_damage(ascension_level),
            hits: 1,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn buff_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        BUFF_THORNS,
        MonsterMoveSpec::Buff(BuffSpec {
            power_id: PowerId::Thorns,
            amount: 2,
        }),
    )
}

fn buff_count(entity: &MonsterEntity) -> usize {
    entity
        .move_history()
        .iter()
        .filter(|&&move_id| move_id == BUFF_THORNS)
        .count()
}

fn plan_for(move_id: u8, ascension_level: u8) -> MonsterTurnPlan {
    match move_id {
        ATTACK => attack_plan(ascension_level),
        BUFF_THORNS => buff_plan(),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

impl MonsterBehavior for Spiker {
    fn use_pre_battle_actions(
        state: &mut CombatState,
        entity: &MonsterEntity,
        legacy_rng: crate::content::monsters::PreBattleLegacyRng,
    ) -> Vec<Action> {
        let (_rng, ascension_level) =
            crate::content::monsters::legacy_pre_battle_rng(state, legacy_rng);
        vec![Action::ApplyPower {
            source: entity.id,
            target: entity.id,
            power_id: PowerId::Thorns,
            amount: starting_thorns(ascension_level),
        }]
    }

    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> MonsterTurnPlan {
        if buff_count(entity) > 5 {
            return attack_plan(ascension_level);
        }
        if num < 50 && entity.move_history().back().copied() != Some(ATTACK) {
            attack_plan(ascension_level)
        } else {
            buff_plan()
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
        let mut actions = match (plan.move_id, plan.steps.as_slice()) {
            (ATTACK, [MoveStep::Attack(attack)]) => {
                attack_actions(entity.id, PLAYER, &attack.attack)
            }
            (BUFF_THORNS, [MoveStep::ApplyPower(power)]) => vec![apply_power_action(entity, power)],
            (move_id, steps) => panic!("spiker plan/steps mismatch: {} {:?}", move_id, steps),
        };
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
