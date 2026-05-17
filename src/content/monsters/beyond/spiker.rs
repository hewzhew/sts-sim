use crate::content::monsters::exordium::{apply_power_action, attack_actions, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::{Action, MonsterRuntimePatch};
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
    assert!(
        entity.spiker.protocol_seeded,
        "spiker runtime truth must be protocol-seeded or factory-seeded"
    );
    entity.spiker.thorns_count as usize
}

fn increment_spiker_thorns_count(entity: &MonsterEntity) -> Action {
    Action::UpdateMonsterRuntime {
        monster_id: entity.id,
        patch: MonsterRuntimePatch::Spiker {
            thorns_count: Some(entity.spiker.thorns_count.saturating_add(1)),
            protocol_seeded: Some(true),
        },
    }
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
            (BUFF_THORNS, [MoveStep::ApplyPower(power)]) => vec![
                increment_spiker_thorns_count(entity),
                apply_power_action(entity, power),
            ],
            (move_id, steps) => panic!("spiker plan/steps mismatch: {} {:?}", move_id, steps),
        };
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::monsters::EnemyId;
    use crate::runtime::rng::StsRng;

    #[test]
    fn planned_but_unexecuted_thorns_buff_does_not_advance_spiker_count() {
        let mut spiker = crate::test_support::test_monster(EnemyId::Spiker);
        spiker.spiker.thorns_count = 5;
        spiker.move_history_mut().extend([BUFF_THORNS; 6]);

        let plan =
            <Spiker as MonsterBehavior>::roll_move_plan(&mut StsRng::new(0), &spiker, 20, 80);

        assert_eq!(
            plan.move_id, BUFF_THORNS,
            "Java thornsCount increments only when BUFF_THORNS executes; a planned move in moveHistory must not count as already executed"
        );
    }

    #[test]
    fn thorns_buff_execution_updates_runtime_before_apply_power() {
        let mut state = crate::test_support::blank_test_combat();
        let mut spiker = crate::test_support::test_monster(EnemyId::Spiker);
        spiker.id = 47;
        spiker.spiker.thorns_count = 5;

        let actions =
            <Spiker as MonsterBehavior>::take_turn_plan(&mut state, &spiker, &buff_plan());

        assert_eq!(
            actions,
            vec![
                Action::UpdateMonsterRuntime {
                    monster_id: 47,
                    patch: MonsterRuntimePatch::Spiker {
                        thorns_count: Some(6),
                        protocol_seeded: Some(true),
                    },
                },
                Action::ApplyPower {
                    source: 47,
                    target: 47,
                    power_id: PowerId::Thorns,
                    amount: 2,
                },
                Action::RollMonsterMove { monster_id: 47 },
            ],
            "Java increments thornsCount before queuing the Thorns ApplyPowerAction"
        );
    }
}
