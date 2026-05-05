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

pub struct Snecko;

const GLARE: u8 = 1;
const BITE: u8 = 2;
const TAIL: u8 = 3;

enum SneckoTurn<'a> {
    Glare(&'a ApplyPowerStep),
    Bite(&'a AttackSpec),
    Tail {
        attack: &'a AttackSpec,
        weak: Option<&'a ApplyPowerStep>,
        vulnerable: &'a ApplyPowerStep,
    },
}

fn bite_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 2 {
        18
    } else {
        15
    }
}

fn tail_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 2 {
        10
    } else {
        8
    }
}

fn glare_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        GLARE,
        MonsterMoveSpec::StrongDebuff(DebuffSpec {
            power_id: PowerId::Confusion,
            amount: 1,
            strength: EffectStrength::Strong,
        }),
    )
}

fn bite_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        BITE,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: bite_damage(ascension_level),
            hits: 1,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn tail_plan(ascension_level: u8) -> MonsterTurnPlan {
    let attack = AttackSpec {
        base_damage: tail_damage(ascension_level),
        hits: 1,
        damage_kind: DamageKind::Normal,
    };
    let vulnerable = ApplyPowerStep {
        target: MoveTarget::Player,
        power_id: PowerId::Vulnerable,
        amount: 2,
        effect: PowerEffectKind::Debuff,
        visible_strength: EffectStrength::Normal,
    };

    if ascension_level >= 17 {
        MonsterTurnPlan::with_visible_spec(
            TAIL,
            smallvec![
                MoveStep::Attack(AttackStep {
                    target: MoveTarget::Player,
                    attack: attack.clone(),
                }),
                MoveStep::ApplyPower(ApplyPowerStep {
                    target: MoveTarget::Player,
                    power_id: PowerId::Weak,
                    amount: 2,
                    effect: PowerEffectKind::Debuff,
                    visible_strength: EffectStrength::Normal,
                }),
                MoveStep::ApplyPower(vulnerable.clone()),
            ],
            MonsterMoveSpec::AttackDebuff(
                attack,
                DebuffSpec {
                    power_id: PowerId::Vulnerable,
                    amount: 2,
                    strength: EffectStrength::Normal,
                },
            ),
        )
    } else {
        MonsterTurnPlan::with_visible_spec(
            TAIL,
            smallvec![
                MoveStep::Attack(AttackStep {
                    target: MoveTarget::Player,
                    attack: attack.clone(),
                }),
                MoveStep::ApplyPower(vulnerable.clone()),
            ],
            MonsterMoveSpec::AttackDebuff(
                attack,
                DebuffSpec {
                    power_id: PowerId::Vulnerable,
                    amount: 2,
                    strength: EffectStrength::Normal,
                },
            ),
        )
    }
}

fn plan_for(move_id: u8, ascension_level: u8) -> MonsterTurnPlan {
    match move_id {
        GLARE => glare_plan(),
        BITE => bite_plan(ascension_level),
        TAIL => tail_plan(ascension_level),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn current_runtime_flags(entity: &MonsterEntity) -> bool {
    assert!(
        entity.snecko.protocol_seeded,
        "snecko runtime truth must be protocol-seeded or factory-seeded"
    );
    entity.snecko.first_turn
}

fn snecko_runtime_update(entity: &MonsterEntity, first_turn: Option<bool>) -> Action {
    Action::UpdateMonsterRuntime {
        monster_id: entity.id,
        patch: MonsterRuntimePatch::Snecko {
            first_turn,
            protocol_seeded: Some(true),
        },
    }
}

fn last_two_moves(entity: &MonsterEntity, move_id: u8) -> bool {
    let history = entity.move_history();
    history.len() >= 2
        && history[history.len() - 1] == move_id
        && history[history.len() - 2] == move_id
}

fn decode_turn<'a>(plan: &'a MonsterTurnPlan) -> SneckoTurn<'a> {
    match (plan.move_id, plan.steps.as_slice()) {
        (
            GLARE,
            [MoveStep::ApplyPower(
                power @ ApplyPowerStep {
                    target: MoveTarget::Player,
                    power_id: PowerId::Confusion,
                    effect: PowerEffectKind::Debuff,
                    ..
                },
            )],
        ) => SneckoTurn::Glare(power),
        (
            BITE,
            [MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            })],
        ) => SneckoTurn::Bite(attack),
        (
            TAIL,
            [MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            }), MoveStep::ApplyPower(
                vulnerable @ ApplyPowerStep {
                    target: MoveTarget::Player,
                    power_id: PowerId::Vulnerable,
                    effect: PowerEffectKind::Debuff,
                    ..
                },
            )],
        ) => SneckoTurn::Tail {
            attack,
            weak: None,
            vulnerable,
        },
        (
            TAIL,
            [MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            }), MoveStep::ApplyPower(
                weak @ ApplyPowerStep {
                    target: MoveTarget::Player,
                    power_id: PowerId::Weak,
                    effect: PowerEffectKind::Debuff,
                    ..
                },
            ), MoveStep::ApplyPower(
                vulnerable @ ApplyPowerStep {
                    target: MoveTarget::Player,
                    power_id: PowerId::Vulnerable,
                    effect: PowerEffectKind::Debuff,
                    ..
                },
            )],
        ) => SneckoTurn::Tail {
            attack,
            weak: Some(weak),
            vulnerable,
        },
        (_, []) => panic!("snecko plan missing locked truth"),
        (move_id, steps) => panic!("snecko plan/steps mismatch: {} {:?}", move_id, steps),
    }
}

impl MonsterBehavior for Snecko {
    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> MonsterTurnPlan {
        if current_runtime_flags(entity) {
            return glare_plan();
        }

        if num < 40 || last_two_moves(entity, BITE) {
            tail_plan(ascension_level)
        } else {
            bite_plan(ascension_level)
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
            SneckoTurn::Glare(power) => {
                vec![
                    apply_power_action(entity, power),
                    snecko_runtime_update(entity, Some(false)),
                ]
            }
            SneckoTurn::Bite(attack) => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                actions.push(snecko_runtime_update(entity, Some(false)));
                actions
            }
            SneckoTurn::Tail {
                attack,
                weak,
                vulnerable,
            } => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                if let Some(weak) = weak {
                    actions.push(apply_power_action(entity, weak));
                }
                actions.push(apply_power_action(entity, vulnerable));
                actions.push(snecko_runtime_update(entity, Some(false)));
                actions
            }
        };
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
