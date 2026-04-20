use crate::content::cards::CardId;
use crate::content::monsters::exordium::{
    add_card_action, apply_power_action, attack_actions, gain_block_action, remove_power_action,
    utility_action, PLAYER,
};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
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

fn used_haste(entity: &MonsterEntity) -> bool {
    entity.move_history().contains(&HASTE)
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

fn haste_plan(entity: &MonsterEntity, ascension_level: u8) -> MonsterTurnPlan {
    let mut steps = smallvec![
        MoveStep::Utility(UtilityStep::RemoveAllDebuffs {
            target: MoveTarget::SelfTarget,
        }),
        MoveStep::RemovePower(RemovePowerStep {
            target: MoveTarget::SelfTarget,
            power_id: PowerId::Shackled,
        }),
    ];
    let heal_amount = haste_heal_amount(entity);
    if heal_amount > 0 {
        steps.push(MoveStep::Heal(crate::semantics::combat::HealStep {
            target: MoveTarget::SelfTarget,
            amount: heal_amount,
        }));
    }
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
            amount: heal_amount,
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
        if entity.current_hp < entity.max_hp / 2 && !used_haste(entity) {
            return haste_plan(entity, ascension_level);
        }
        recursive_plan(rng, entity, ascension_level, num)
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
                            amount,
                        }) => actions.push(Action::Heal {
                            target: entity.id,
                            amount: *amount,
                        }),
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
