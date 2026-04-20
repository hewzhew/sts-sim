use super::{apply_power_action, attack_actions, gain_block_action, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    ApplyPowerStep, AttackSpec, AttackStep, BlockStep, BuffSpec, DamageKind, DefendSpec,
    EffectStrength, MonsterMoveSpec, MonsterTurnPlan, MoveStep, MoveTarget, PowerEffectKind,
};
use smallvec::smallvec;

const CHOMP: u8 = 1;
const BELLOW: u8 = 2;
const THRASH: u8 = 3;

pub struct JawWorm;

enum JawWormTurn<'a> {
    Chomp(&'a AttackSpec),
    Bellow(&'a ApplyPowerStep, &'a BlockStep),
    Thrash(&'a AttackSpec, &'a BlockStep),
}

fn chomp_damage(asc: u8) -> i32 {
    if asc >= 2 {
        12
    } else {
        11
    }
}

fn bellow_strength(asc: u8) -> i32 {
    if asc >= 17 {
        5
    } else if asc >= 2 {
        4
    } else {
        3
    }
}

fn bellow_block(asc: u8) -> i32 {
    if asc >= 17 {
        9
    } else {
        6
    }
}

fn chomp_plan(asc: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::single(
        CHOMP,
        MoveStep::Attack(AttackStep {
            target: MoveTarget::Player,
            attack: AttackSpec {
                base_damage: chomp_damage(asc),
                hits: 1,
                damage_kind: DamageKind::Normal,
            },
        }),
    )
}

fn bellow_plan(asc: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        BELLOW,
        smallvec![
            MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::SelfTarget,
                power_id: PowerId::Strength,
                amount: bellow_strength(asc),
                effect: PowerEffectKind::Buff,
                visible_strength: EffectStrength::Normal,
            }),
            MoveStep::GainBlock(BlockStep {
                target: MoveTarget::SelfTarget,
                amount: bellow_block(asc),
            }),
        ],
        MonsterMoveSpec::DefendBuff(
            DefendSpec {
                block: bellow_block(asc),
            },
            BuffSpec {
                power_id: PowerId::Strength,
                amount: bellow_strength(asc),
            },
        ),
    )
}

fn thrash_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        THRASH,
        smallvec![
            MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack: AttackSpec {
                    base_damage: 7,
                    hits: 1,
                    damage_kind: DamageKind::Normal,
                },
            }),
            MoveStep::GainBlock(BlockStep {
                target: MoveTarget::SelfTarget,
                amount: 5,
            }),
        ],
        MonsterMoveSpec::AttackDefend(
            AttackSpec {
                base_damage: 7,
                hits: 1,
                damage_kind: DamageKind::Normal,
            },
            DefendSpec { block: 5 },
        ),
    )
}

fn plan_for(move_id: u8, asc: u8) -> MonsterTurnPlan {
    match move_id {
        CHOMP => chomp_plan(asc),
        BELLOW => bellow_plan(asc),
        THRASH => thrash_plan(),
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

fn decode_turn<'a>(plan: &'a MonsterTurnPlan) -> JawWormTurn<'a> {
    match (plan.move_id, plan.steps.as_slice()) {
        (
            CHOMP,
            [MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            })],
        ) => JawWormTurn::Chomp(attack),
        (
            BELLOW,
            [MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::SelfTarget,
                power_id: PowerId::Strength,
                effect: PowerEffectKind::Buff,
                ..
            }), MoveStep::GainBlock(BlockStep {
                target: MoveTarget::SelfTarget,
                ..
            })],
        ) => {
            let MoveStep::ApplyPower(power) = &plan.steps[0] else {
                unreachable!()
            };
            let MoveStep::GainBlock(block) = &plan.steps[1] else {
                unreachable!()
            };
            JawWormTurn::Bellow(power, block)
        }
        (
            THRASH,
            [MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            }), MoveStep::GainBlock(BlockStep {
                target: MoveTarget::SelfTarget,
                ..
            })],
        ) => {
            let MoveStep::GainBlock(block) = &plan.steps[1] else {
                unreachable!()
            };
            JawWormTurn::Thrash(attack, block)
        }
        (_, []) => panic!("jaw worm plan missing locked truth"),
        (move_id, steps) => panic!("jaw worm plan/steps mismatch: {} {:?}", move_id, steps),
    }
}

impl MonsterBehavior for JawWorm {
    fn roll_move_plan(
        rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        asc: u8,
        num: i32,
    ) -> MonsterTurnPlan {
        if entity.move_history().is_empty() && !entity.jaw_worm.hard_mode {
            return chomp_plan(asc);
        }

        if num < 25 {
            if last_move(entity, CHOMP) {
                if rng.random_boolean_chance(0.5625) {
                    bellow_plan(asc)
                } else {
                    thrash_plan()
                }
            } else {
                chomp_plan(asc)
            }
        } else if num < 55 {
            if last_two_moves(entity, THRASH) {
                if rng.random_boolean_chance(0.357) {
                    chomp_plan(asc)
                } else {
                    bellow_plan(asc)
                }
            } else {
                thrash_plan()
            }
        } else if last_move(entity, BELLOW) {
            if rng.random_boolean_chance(0.416) {
                chomp_plan(asc)
            } else {
                thrash_plan()
            }
        } else {
            bellow_plan(asc)
        }
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
        if entity.jaw_worm.hard_mode {
            vec![
                apply_power_action(
                    entity,
                    &ApplyPowerStep {
                        target: MoveTarget::SelfTarget,
                        power_id: PowerId::Strength,
                        amount: bellow_strength(asc),
                        effect: PowerEffectKind::Buff,
                        visible_strength: EffectStrength::Normal,
                    },
                ),
                gain_block_action(
                    entity,
                    &BlockStep {
                        target: MoveTarget::SelfTarget,
                        amount: bellow_block(asc),
                    },
                ),
            ]
        } else {
            Vec::new()
        }
    }

    fn take_turn_plan(
        _state: &mut CombatState,
        entity: &MonsterEntity,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        let mut actions = match decode_turn(plan) {
            JawWormTurn::Chomp(attack) => attack_actions(entity.id, PLAYER, attack),
            JawWormTurn::Bellow(power, block) => {
                vec![
                    apply_power_action(entity, power),
                    gain_block_action(entity, block),
                ]
            }
            JawWormTurn::Thrash(attack, block) => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                actions.push(gain_block_action(entity, block));
                actions
            }
        };
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
