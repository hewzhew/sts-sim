use crate::content::cards::CardId;
use crate::content::monsters::exordium::{
    add_card_action, apply_power_action, attack_actions, gain_block_action, remove_power_action,
    utility_action, PLAYER,
};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::{Action, MonsterRuntimePatch};
use crate::runtime::combat::{CombatState, MonsterEntity, TimeEaterRuntimeState};
use crate::semantics::combat::{
    AddCardStep, ApplyPowerStep, AttackSpec, AttackStep, BlockStep, DamageKind, DebuffSpec,
    DefendSpec, EffectStrength, HealSpec, MonsterMoveSpec, MonsterTurnPlan, MoveStep, MoveTarget,
    PowerEffectKind, RemovePowerStep, UtilityStep,
};
use smallvec::smallvec;

pub struct TimeEater;

const REVERBERATE: u8 = 2;
const RIPPLE: u8 = 3;
const HEAD_SLAM: u8 = 4;
const HASTE: u8 = 5;

pub fn initialize_runtime_state(entity: &mut MonsterEntity) {
    entity.time_eater = TimeEaterRuntimeState {
        protocol_seeded: true,
        used_haste: false,
    };
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::monsters::EnemyId;
    use crate::runtime::rng::StsRng;

    #[test]
    fn haste_heal_amount_is_read_at_execution_time_like_java() {
        let mut time_eater = crate::test_support::test_monster(EnemyId::TimeEater);
        time_eater.id = 1;
        time_eater.max_hp = 456;
        time_eater.current_hp = 220;
        let plan = haste_plan(&time_eater, 19);

        time_eater.current_hp = 100;
        let mut state = crate::test_support::combat_with_monsters(vec![time_eater.clone()]);

        let actions = TimeEater::take_turn_plan(&mut state, &time_eater, &plan);

        assert!(matches!(
            actions
                .iter()
                .find(|action| matches!(action, Action::Heal { .. })),
            Some(Action::Heal {
                target: 1,
                amount: 128
            })
        ));
    }

    #[test]
    fn haste_visible_spec_does_not_freeze_hidden_heal_amount() {
        let mut time_eater = crate::test_support::test_monster(EnemyId::TimeEater);
        time_eater.max_hp = 456;
        time_eater.current_hp = 220;

        let plan = haste_plan(&time_eater, 19);

        assert_eq!(
            plan.visible_spec,
            Some(MonsterMoveSpec::Heal(HealSpec {
                target: MoveTarget::SelfTarget,
                amount: 0,
            }))
        );
    }

    #[test]
    fn haste_selection_marks_private_used_haste_during_roll() {
        let mut time_eater = crate::test_support::test_monster(EnemyId::TimeEater);
        time_eater.max_hp = 456;
        time_eater.current_hp = 200;

        let plan = TimeEater::roll_move_plan(&mut StsRng::new(0), &time_eater, 0, 0);
        let actions = TimeEater::on_roll_move(0, &time_eater, 0, &plan);

        assert_eq!(plan.move_id, HASTE);
        assert!(matches!(
            actions.as_slice(),
            [Action::UpdateMonsterRuntime {
                patch: MonsterRuntimePatch::TimeEater {
                    used_haste: Some(true),
                    protocol_seeded: Some(true),
                },
                ..
            }]
        ));
    }

    #[test]
    fn imported_used_haste_true_with_empty_history_does_not_force_haste() {
        let mut time_eater = crate::test_support::test_monster(EnemyId::TimeEater);
        time_eater.max_hp = 456;
        time_eater.current_hp = 200;
        time_eater.time_eater.used_haste = true;
        time_eater.move_history_mut().clear();

        let plan = TimeEater::roll_move_plan(&mut StsRng::new(0), &time_eater, 0, 0);

        assert_eq!(
            plan.move_id, REVERBERATE,
            "Java gates Haste on private usedHaste, not move history"
        );
    }
}

fn reverberate_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 4 {
        8
    } else {
        7
    }
}

fn head_slam_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 4 {
        32
    } else {
        26
    }
}

fn haste_heal_amount(entity: &MonsterEntity) -> i32 {
    (entity.max_hp / 2 - entity.current_hp).max(0)
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

fn runtime(entity: &MonsterEntity) -> &TimeEaterRuntimeState {
    assert!(
        entity.time_eater.protocol_seeded,
        "time eater runtime truth must be protocol-seeded or factory-seeded"
    );
    &entity.time_eater
}

fn time_eater_runtime_update(entity: &MonsterEntity, used_haste: Option<bool>) -> Action {
    Action::UpdateMonsterRuntime {
        monster_id: entity.id,
        patch: MonsterRuntimePatch::TimeEater {
            used_haste,
            protocol_seeded: Some(true),
        },
    }
}

fn reverberate_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        REVERBERATE,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: reverberate_damage(ascension_level),
            hits: 3,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn ripple_plan(ascension_level: u8) -> MonsterTurnPlan {
    let mut steps = smallvec![
        MoveStep::GainBlock(BlockStep {
            target: MoveTarget::SelfTarget,
            amount: 20,
        }),
        MoveStep::ApplyPower(ApplyPowerStep {
            target: MoveTarget::Player,
            power_id: PowerId::Vulnerable,
            amount: 1,
            effect: PowerEffectKind::Debuff,
            visible_strength: EffectStrength::Normal,
        }),
        MoveStep::ApplyPower(ApplyPowerStep {
            target: MoveTarget::Player,
            power_id: PowerId::Weak,
            amount: 1,
            effect: PowerEffectKind::Debuff,
            visible_strength: EffectStrength::Normal,
        }),
    ];
    if ascension_level >= 19 {
        steps.push(MoveStep::ApplyPower(ApplyPowerStep {
            target: MoveTarget::Player,
            power_id: PowerId::Frail,
            amount: 1,
            effect: PowerEffectKind::Debuff,
            visible_strength: EffectStrength::Normal,
        }));
    }
    MonsterTurnPlan::with_visible_spec(
        RIPPLE,
        steps,
        MonsterMoveSpec::DefendDebuff(
            DefendSpec { block: 20 },
            DebuffSpec {
                power_id: PowerId::Vulnerable,
                amount: 1,
                strength: EffectStrength::Normal,
            },
        ),
    )
}

fn head_slam_plan(ascension_level: u8) -> MonsterTurnPlan {
    let mut steps = smallvec![
        MoveStep::Attack(AttackStep {
            target: MoveTarget::Player,
            attack: AttackSpec {
                base_damage: head_slam_damage(ascension_level),
                hits: 1,
                damage_kind: DamageKind::Normal,
            },
        }),
        MoveStep::ApplyPower(ApplyPowerStep {
            target: MoveTarget::Player,
            power_id: PowerId::DrawReduction,
            amount: 1,
            effect: PowerEffectKind::Debuff,
            visible_strength: EffectStrength::Normal,
        }),
    ];
    if ascension_level >= 19 {
        steps.push(MoveStep::AddCard(AddCardStep {
            card_id: CardId::Slimed,
            amount: 2,
            upgraded: false,
            destination: crate::semantics::combat::CardDestination::Discard,
            visible_strength: EffectStrength::Normal,
        }));
    }
    MonsterTurnPlan::with_visible_spec(
        HEAD_SLAM,
        steps,
        MonsterMoveSpec::AttackDebuff(
            AttackSpec {
                base_damage: head_slam_damage(ascension_level),
                hits: 1,
                damage_kind: DamageKind::Normal,
            },
            DebuffSpec {
                power_id: PowerId::DrawReduction,
                amount: 1,
                strength: EffectStrength::Normal,
            },
        ),
    )
}

fn haste_plan(_entity: &MonsterEntity, ascension_level: u8) -> MonsterTurnPlan {
    let mut steps = smallvec![
        MoveStep::Utility(UtilityStep::RemoveAllDebuffs {
            target: MoveTarget::SelfTarget,
        }),
        MoveStep::RemovePower(RemovePowerStep {
            target: MoveTarget::SelfTarget,
            power_id: PowerId::Shackled,
        }),
    ];
    steps.push(MoveStep::Heal(crate::semantics::combat::HealStep {
        target: MoveTarget::SelfTarget,
        amount: 0,
    }));
    if ascension_level >= 19 {
        steps.push(MoveStep::GainBlock(BlockStep {
            target: MoveTarget::SelfTarget,
            amount: head_slam_damage(ascension_level),
        }));
    }
    MonsterTurnPlan::with_visible_spec(
        HASTE,
        steps,
        MonsterMoveSpec::Heal(HealSpec {
            target: MoveTarget::SelfTarget,
            amount: 0,
        }),
    )
}

fn plan_for(entity: &MonsterEntity, ascension_level: u8, move_id: u8) -> MonsterTurnPlan {
    match move_id {
        REVERBERATE => reverberate_plan(ascension_level),
        RIPPLE => ripple_plan(ascension_level),
        HEAD_SLAM => head_slam_plan(ascension_level),
        HASTE => haste_plan(entity, ascension_level),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn recursive_plan(
    rng: &mut crate::runtime::rng::StsRng,
    entity: &MonsterEntity,
    ascension_level: u8,
    num: i32,
) -> MonsterTurnPlan {
    if num < 45 {
        if !last_two_moves(entity, REVERBERATE) {
            return reverberate_plan(ascension_level);
        }
        let reroll = rng.random_range(50, 99);
        return recursive_plan(rng, entity, ascension_level, reroll);
    }
    if num < 80 {
        if !last_move(entity, HEAD_SLAM) {
            return head_slam_plan(ascension_level);
        }
        if rng.random_boolean_chance(0.66) {
            return reverberate_plan(ascension_level);
        }
        return ripple_plan(ascension_level);
    }
    if !last_move(entity, RIPPLE) {
        return ripple_plan(ascension_level);
    }
    let reroll = rng.random(74);
    recursive_plan(rng, entity, ascension_level, reroll)
}

impl MonsterBehavior for TimeEater {
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
            power_id: PowerId::TimeWarp,
            amount: 0,
        }]
    }

    fn roll_move_plan(
        rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> MonsterTurnPlan {
        if entity.current_hp < entity.max_hp / 2 && !runtime(entity).used_haste {
            return haste_plan(entity, ascension_level);
        }
        recursive_plan(rng, entity, ascension_level, num)
    }

    fn on_roll_move(
        _ascension_level: u8,
        entity: &MonsterEntity,
        _num: i32,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        if plan.move_id == HASTE && !runtime(entity).used_haste {
            vec![time_eater_runtime_update(entity, Some(true))]
        } else {
            Vec::new()
        }
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
            (
                REVERBERATE,
                [MoveStep::Attack(AttackStep {
                    target: MoveTarget::Player,
                    attack,
                })],
            ) => attack_actions(entity.id, PLAYER, attack),
            (RIPPLE, steps) => {
                let mut actions = Vec::new();
                for step in steps {
                    match step {
                        MoveStep::GainBlock(
                            block @ BlockStep {
                                target: MoveTarget::SelfTarget,
                                ..
                            },
                        ) => actions.push(gain_block_action(entity, block)),
                        MoveStep::ApplyPower(
                            power @ ApplyPowerStep {
                                target: MoveTarget::Player,
                                effect: PowerEffectKind::Debuff,
                                ..
                            },
                        ) => actions.push(apply_power_action(entity, power)),
                        _ => panic!("time eater ripple step mismatch: {:?}", steps),
                    }
                }
                actions
            }
            (
                HEAD_SLAM,
                [MoveStep::Attack(AttackStep {
                    target: MoveTarget::Player,
                    attack,
                }), tail @ ..],
            ) => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                for step in tail {
                    match step {
                        MoveStep::ApplyPower(
                            power @ ApplyPowerStep {
                                target: MoveTarget::Player,
                                power_id: PowerId::DrawReduction,
                                effect: PowerEffectKind::Debuff,
                                ..
                            },
                        ) => actions.push(apply_power_action(entity, power)),
                        MoveStep::AddCard(add_card) => actions.push(add_card_action(add_card)),
                        _ => panic!("time eater head slam step mismatch: {:?}", plan.steps),
                    }
                }
                actions
            }
            (HASTE, steps) => {
                let mut actions = Vec::new();
                for step in steps {
                    match step {
                        MoveStep::Utility(step) => actions.push(utility_action(entity, step)),
                        MoveStep::RemovePower(
                            step @ RemovePowerStep {
                                target: MoveTarget::SelfTarget,
                                power_id: PowerId::Shackled,
                            },
                        ) => actions.push(remove_power_action(entity, step)),
                        MoveStep::Heal(crate::semantics::combat::HealStep {
                            target: MoveTarget::SelfTarget,
                            ..
                        }) => {
                            let amount = haste_heal_amount(entity);
                            if amount > 0 {
                                actions.push(Action::Heal {
                                    target: entity.id,
                                    amount,
                                });
                            }
                        }
                        MoveStep::GainBlock(
                            block @ BlockStep {
                                target: MoveTarget::SelfTarget,
                                ..
                            },
                        ) => actions.push(gain_block_action(entity, block)),
                        _ => panic!("time eater haste step mismatch: {:?}", steps),
                    }
                }
                actions
            }
            (_, []) => panic!("time eater plan missing locked truth"),
            (move_id, steps) => panic!("time eater plan/steps mismatch: {} {:?}", move_id, steps),
        };
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
