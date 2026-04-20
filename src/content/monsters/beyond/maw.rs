use crate::content::monsters::exordium::{apply_power_action, attack_actions, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    ApplyPowerStep, AttackSpec, BuffSpec, DamageKind, DebuffSpec, EffectStrength, MonsterMoveSpec,
    MonsterTurnPlan, MoveStep, MoveTarget, PowerEffectKind,
};
use smallvec::smallvec;

pub struct Maw;

const ROAR: u8 = 2;
const SLAM: u8 = 3;
const DROOL: u8 = 4;
const NOMNOMNOM: u8 = 5;

fn slam_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 2 {
        30
    } else {
        25
    }
}

fn strength_up(ascension_level: u8) -> i32 {
    if ascension_level >= 17 {
        5
    } else {
        3
    }
}

fn terrify_duration(ascension_level: u8) -> i32 {
    if ascension_level >= 17 {
        5
    } else {
        3
    }
}

fn nom_hits_for_roll(entity: &MonsterEntity) -> u8 {
    (((entity.move_history().len() as i32) + 2) / 2).max(1) as u8
}

fn nom_hits_for_turn(entity: &MonsterEntity) -> u8 {
    (((entity.move_history().len() as i32) + 1) / 2).max(1) as u8
}

fn roar_plan(ascension_level: u8) -> MonsterTurnPlan {
    let terrify = terrify_duration(ascension_level);
    MonsterTurnPlan::with_visible_spec(
        ROAR,
        smallvec![
            MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::Player,
                power_id: PowerId::Weak,
                amount: terrify,
                effect: PowerEffectKind::Debuff,
                visible_strength: EffectStrength::Strong,
            }),
            MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::Player,
                power_id: PowerId::Frail,
                amount: terrify,
                effect: PowerEffectKind::Debuff,
                visible_strength: EffectStrength::Strong,
            }),
        ],
        MonsterMoveSpec::StrongDebuff(DebuffSpec {
            power_id: PowerId::Weak,
            amount: terrify,
            strength: EffectStrength::Strong,
        }),
    )
}

fn slam_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        SLAM,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: slam_damage(ascension_level),
            hits: 1,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn drool_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        DROOL,
        MonsterMoveSpec::Buff(BuffSpec {
            power_id: PowerId::Strength,
            amount: strength_up(ascension_level),
        }),
    )
}

fn nom_plan(hits: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        NOMNOMNOM,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: 5,
            hits,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn has_roared(entity: &MonsterEntity) -> bool {
    entity.move_history().contains(&ROAR)
}

fn plan_for(move_id: u8, ascension_level: u8, entity: &MonsterEntity) -> MonsterTurnPlan {
    match move_id {
        ROAR => roar_plan(ascension_level),
        SLAM => slam_plan(ascension_level),
        DROOL => drool_plan(ascension_level),
        NOMNOMNOM => nom_plan(nom_hits_for_turn(entity)),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

impl MonsterBehavior for Maw {
    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> MonsterTurnPlan {
        if !has_roared(entity) {
            return roar_plan(ascension_level);
        }

        let last_move = entity.move_history().back().copied();
        if num < 50 && last_move != Some(NOMNOMNOM) {
            return nom_plan(nom_hits_for_roll(entity));
        }
        if matches!(last_move, Some(SLAM | NOMNOMNOM)) {
            return drool_plan(ascension_level);
        }
        slam_plan(ascension_level)
    }

    fn turn_plan(state: &CombatState, entity: &MonsterEntity) -> MonsterTurnPlan {
        plan_for(entity.planned_move_id(), state.meta.ascension_level, entity)
    }

    fn take_turn_plan(
        _state: &mut CombatState,
        entity: &MonsterEntity,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        let mut actions = match (plan.move_id, plan.steps.as_slice()) {
            (ROAR, [MoveStep::ApplyPower(weak), MoveStep::ApplyPower(frail)]) => {
                vec![
                    apply_power_action(entity, weak),
                    apply_power_action(entity, frail),
                ]
            }
            (SLAM, [MoveStep::Attack(attack)]) => attack_actions(entity.id, PLAYER, &attack.attack),
            (DROOL, [MoveStep::ApplyPower(power)]) => vec![apply_power_action(entity, power)],
            (NOMNOMNOM, [MoveStep::Attack(attack)]) => {
                attack_actions(entity.id, PLAYER, &attack.attack)
            }
            (move_id, steps) => panic!("maw plan/steps mismatch: {} {:?}", move_id, steps),
        };
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
