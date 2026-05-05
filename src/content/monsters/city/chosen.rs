use crate::content::monsters::exordium::{apply_power_action, attack_actions, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::{Action, MonsterRuntimePatch};
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    ApplyPowerStep, AttackSpec, AttackStep, DamageKind, DebuffSpec, EffectStrength,
    MonsterMoveSpec, MonsterTurnPlan, MoveStep, MoveTarget, PowerEffectKind,
};
use smallvec::smallvec;

pub struct Chosen;

const ZAP: u8 = 1;
const DRAIN: u8 = 2;
const DEBILITATE: u8 = 3;
const HEX: u8 = 4;
const POKE: u8 = 5;

enum ChosenTurn<'a> {
    Zap(&'a AttackSpec),
    Drain(&'a ApplyPowerStep, &'a ApplyPowerStep),
    Debilitate(&'a AttackSpec, &'a ApplyPowerStep),
    Hex(&'a ApplyPowerStep),
    Poke(&'a AttackSpec),
}

fn zap_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 2 {
        21
    } else {
        18
    }
}

fn debilitate_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 2 {
        12
    } else {
        10
    }
}

fn poke_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 2 {
        6
    } else {
        5
    }
}

fn zap_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        ZAP,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: zap_damage(ascension_level),
            hits: 1,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn drain_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        DRAIN,
        smallvec![
            MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::Player,
                power_id: PowerId::Weak,
                amount: 3,
                effect: PowerEffectKind::Debuff,
                visible_strength: EffectStrength::Normal,
            }),
            MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::SelfTarget,
                power_id: PowerId::Strength,
                amount: 3,
                effect: PowerEffectKind::Buff,
                visible_strength: EffectStrength::Normal,
            })
        ],
        MonsterMoveSpec::Debuff(DebuffSpec {
            power_id: PowerId::Weak,
            amount: 3,
            strength: EffectStrength::Normal,
        }),
    )
}

fn debilitate_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        DEBILITATE,
        MonsterMoveSpec::AttackDebuff(
            AttackSpec {
                base_damage: debilitate_damage(ascension_level),
                hits: 1,
                damage_kind: DamageKind::Normal,
            },
            DebuffSpec {
                power_id: PowerId::Vulnerable,
                amount: 2,
                strength: EffectStrength::Normal,
            },
        ),
    )
}

fn hex_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        HEX,
        MonsterMoveSpec::StrongDebuff(DebuffSpec {
            power_id: PowerId::Hex,
            amount: 1,
            strength: EffectStrength::Strong,
        }),
    )
}

fn poke_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        POKE,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: poke_damage(ascension_level),
            hits: 2,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn plan_for(move_id: u8, ascension_level: u8) -> MonsterTurnPlan {
    match move_id {
        ZAP => zap_plan(ascension_level),
        DRAIN => drain_plan(),
        DEBILITATE => debilitate_plan(ascension_level),
        HEX => hex_plan(),
        POKE => poke_plan(ascension_level),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn current_runtime_flags(entity: &MonsterEntity) -> (bool, bool) {
    assert!(
        entity.chosen.protocol_seeded,
        "chosen runtime truth must be protocol-seeded or factory-seeded"
    );
    (entity.chosen.first_turn, entity.chosen.used_hex)
}

fn chosen_runtime_update(entity: &MonsterEntity, used_hex: Option<bool>) -> Action {
    Action::UpdateMonsterRuntime {
        monster_id: entity.id,
        patch: MonsterRuntimePatch::Chosen {
            first_turn: Some(false),
            used_hex,
            protocol_seeded: Some(true),
        },
    }
}

fn decode_turn<'a>(plan: &'a MonsterTurnPlan) -> ChosenTurn<'a> {
    match (plan.move_id, plan.steps.as_slice()) {
        (
            ZAP,
            [MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            })],
        ) => ChosenTurn::Zap(attack),
        (
            DRAIN,
            [MoveStep::ApplyPower(
                weak @ ApplyPowerStep {
                    target: MoveTarget::Player,
                    power_id: PowerId::Weak,
                    effect: PowerEffectKind::Debuff,
                    ..
                },
            ), MoveStep::ApplyPower(
                strength @ ApplyPowerStep {
                    target: MoveTarget::SelfTarget,
                    power_id: PowerId::Strength,
                    effect: PowerEffectKind::Buff,
                    ..
                },
            )],
        ) => ChosenTurn::Drain(weak, strength),
        (
            DEBILITATE,
            [MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            }), MoveStep::ApplyPower(
                power @ ApplyPowerStep {
                    target: MoveTarget::Player,
                    power_id: PowerId::Vulnerable,
                    effect: PowerEffectKind::Debuff,
                    ..
                },
            )],
        ) => ChosenTurn::Debilitate(attack, power),
        (
            HEX,
            [MoveStep::ApplyPower(
                power @ ApplyPowerStep {
                    target: MoveTarget::Player,
                    power_id: PowerId::Hex,
                    effect: PowerEffectKind::Debuff,
                    ..
                },
            )],
        ) => ChosenTurn::Hex(power),
        (
            POKE,
            [MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            })],
        ) => ChosenTurn::Poke(attack),
        (_, []) => panic!("chosen plan missing locked truth"),
        (move_id, steps) => panic!("chosen plan/steps mismatch: {} {:?}", move_id, steps),
    }
}

impl MonsterBehavior for Chosen {
    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> MonsterTurnPlan {
        let (first_turn, used_hex) = current_runtime_flags(entity);

        if ascension_level >= 17 {
            if !used_hex {
                return hex_plan();
            }
        } else {
            if first_turn {
                return poke_plan(ascension_level);
            }
            if !used_hex {
                return hex_plan();
            }
        }

        let last_move = entity.move_history().back().copied().unwrap_or(0);
        if last_move != DEBILITATE && last_move != DRAIN {
            if num < 50 {
                return debilitate_plan(ascension_level);
            }
            return drain_plan();
        }

        if num < 40 {
            zap_plan(ascension_level)
        } else {
            poke_plan(ascension_level)
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
        let mut actions = match decode_turn(plan) {
            ChosenTurn::Zap(attack) | ChosenTurn::Poke(attack) => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                actions.push(chosen_runtime_update(entity, None));
                actions
            }
            ChosenTurn::Drain(weak, strength) => {
                vec![
                    apply_power_action(entity, weak),
                    apply_power_action(entity, strength),
                    chosen_runtime_update(entity, None),
                ]
            }
            ChosenTurn::Debilitate(attack, power) => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                actions.push(apply_power_action(entity, power));
                actions.push(chosen_runtime_update(entity, None));
                actions
            }
            ChosenTurn::Hex(power) => {
                vec![
                    apply_power_action(entity, power),
                    chosen_runtime_update(entity, Some(true)),
                ]
            }
        };
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
