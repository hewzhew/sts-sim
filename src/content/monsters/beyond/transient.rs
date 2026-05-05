use crate::content::monsters::exordium::{attack_actions, set_next_move_action, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    AttackSpec, DamageKind, MonsterMoveSpec, MonsterTurnPlan, MoveStep,
};

pub struct Transient;

const ATTACK: u8 = 1;

fn starting_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 2 {
        40
    } else {
        30
    }
}

fn attack_damage_for_count(ascension_level: u8, count: usize) -> i32 {
    starting_damage(ascension_level) + (count as i32 * 10)
}

fn attack_plan_for_count(ascension_level: u8, count: usize) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        ATTACK,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: attack_damage_for_count(ascension_level, count),
            hits: 1,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn current_attack_count(entity: &MonsterEntity) -> usize {
    entity.move_history().len().saturating_sub(1)
}

impl MonsterBehavior for Transient {
    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> MonsterTurnPlan {
        attack_plan_for_count(ascension_level, entity.move_history().len())
    }

    fn use_pre_battle_actions(
        state: &mut CombatState,
        entity: &MonsterEntity,
        legacy_rng: crate::content::monsters::PreBattleLegacyRng,
    ) -> Vec<Action> {
        let (_rng, ascension_level) =
            crate::content::monsters::legacy_pre_battle_rng(state, legacy_rng);
        vec![
            Action::ApplyPower {
                source: entity.id,
                target: entity.id,
                power_id: PowerId::Fading,
                amount: if ascension_level >= 17 { 6 } else { 5 },
            },
            Action::ApplyPower {
                source: entity.id,
                target: entity.id,
                power_id: PowerId::Shifting,
                amount: 1,
            },
        ]
    }

    fn turn_plan(state: &CombatState, entity: &MonsterEntity) -> MonsterTurnPlan {
        match entity.planned_move_id() {
            ATTACK => {
                attack_plan_for_count(state.meta.ascension_level, current_attack_count(entity))
            }
            other => MonsterTurnPlan::unknown(other),
        }
    }

    fn take_turn_plan(
        state: &mut CombatState,
        entity: &MonsterEntity,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        match (plan.move_id, plan.steps.as_slice()) {
            (ATTACK, [MoveStep::Attack(attack)]) => {
                let mut actions = attack_actions(entity.id, PLAYER, &attack.attack);
                actions.push(set_next_move_action(
                    entity,
                    attack_plan_for_count(
                        state.meta.ascension_level,
                        current_attack_count(entity) + 1,
                    ),
                ));
                actions
            }
            (move_id, steps) => panic!("transient plan/steps mismatch: {} {:?}", move_id, steps),
        }
    }
}
