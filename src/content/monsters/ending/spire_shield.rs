use crate::content::monsters::exordium::{attack_actions, gain_block_action, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    ApplyPowerStep, AttackSpec, AttackStep, BlockStep, DamageKind, DebuffSpec, DefendSpec,
    EffectStrength, MonsterMoveSpec, MonsterTurnPlan, MoveStep, MoveTarget, PowerEffectKind,
};
use smallvec::smallvec;

pub struct SpireShield;

const BASH: u8 = 1;
const FORTIFY: u8 = 2;
const SMASH: u8 = 3;

fn bash_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 3 {
        14
    } else {
        12
    }
}

fn smash_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 3 {
        38
    } else {
        34
    }
}

fn smash_block(ascension_level: u8) -> i32 {
    if ascension_level >= 18 {
        99
    } else {
        smash_damage(ascension_level)
    }
}

fn bash_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        BASH,
        smallvec![
            MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack: AttackSpec {
                    base_damage: bash_damage(ascension_level),
                    hits: 1,
                    damage_kind: DamageKind::Normal,
                },
            }),
            MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::Player,
                power_id: PowerId::Strength,
                amount: -1,
                effect: PowerEffectKind::Debuff,
                visible_strength: EffectStrength::Normal,
            }),
        ],
        MonsterMoveSpec::AttackDebuff(
            AttackSpec {
                base_damage: bash_damage(ascension_level),
                hits: 1,
                damage_kind: DamageKind::Normal,
            },
            DebuffSpec {
                power_id: PowerId::Strength,
                amount: -1,
                strength: EffectStrength::Normal,
            },
        ),
    )
}

fn fortify_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        FORTIFY,
        smallvec![MoveStep::GainBlock(BlockStep {
            target: MoveTarget::AllMonsters,
            amount: 30,
        })],
        MonsterMoveSpec::Defend(DefendSpec { block: 30 }),
    )
}

fn smash_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        SMASH,
        MonsterMoveSpec::AttackDefend(
            AttackSpec {
                base_damage: smash_damage(ascension_level),
                hits: 1,
                damage_kind: DamageKind::Normal,
            },
            DefendSpec {
                block: smash_block(ascension_level),
            },
        ),
    )
}

fn plan_for(move_id: u8, ascension_level: u8) -> MonsterTurnPlan {
    match move_id {
        BASH => bash_plan(ascension_level),
        FORTIFY => fortify_plan(),
        SMASH => smash_plan(ascension_level),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

impl MonsterBehavior for SpireShield {
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
                target: PLAYER,
                power_id: PowerId::Surrounded,
                amount: 1,
            },
            Action::ApplyPower {
                source: entity.id,
                target: entity.id,
                power_id: PowerId::Artifact,
                amount: if ascension_level >= 18 { 2 } else { 1 },
            },
        ]
    }

    fn roll_move_plan(
        rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> MonsterTurnPlan {
        match entity.move_history().len() % 3 {
            0 => {
                if rng.random_boolean() {
                    fortify_plan()
                } else {
                    bash_plan(ascension_level)
                }
            }
            1 => {
                if entity.move_history().back().copied() == Some(BASH) {
                    fortify_plan()
                } else {
                    bash_plan(ascension_level)
                }
            }
            _ => smash_plan(ascension_level),
        }
    }

    fn turn_plan(state: &CombatState, entity: &MonsterEntity) -> MonsterTurnPlan {
        plan_for(entity.planned_move_id(), state.meta.ascension_level)
    }

    fn take_turn_plan(
        state: &mut CombatState,
        entity: &MonsterEntity,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        let mut actions = match (plan.move_id, plan.steps.as_slice()) {
            (
                BASH,
                [MoveStep::Attack(AttackStep {
                    target: MoveTarget::Player,
                    attack,
                }), MoveStep::ApplyPower(ApplyPowerStep {
                    target: MoveTarget::Player,
                    effect: PowerEffectKind::Debuff,
                    ..
                })],
            ) => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                let focus_roll =
                    !state.entities.player.orbs.is_empty() && state.rng.ai_rng.random_boolean();
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: PLAYER,
                    power_id: if focus_roll {
                        PowerId::Focus
                    } else {
                        PowerId::Strength
                    },
                    amount: -1,
                });
                actions
            }
            (
                FORTIFY,
                [MoveStep::GainBlock(BlockStep {
                    target: MoveTarget::AllMonsters,
                    amount,
                })],
            ) => state
                .entities
                .monsters
                .iter()
                .map(|monster| Action::GainBlock {
                    target: monster.id,
                    amount: *amount,
                })
                .collect(),
            (
                SMASH,
                [MoveStep::Attack(AttackStep {
                    target: MoveTarget::Player,
                    attack,
                }), MoveStep::GainBlock(
                    block @ BlockStep {
                        target: MoveTarget::SelfTarget,
                        ..
                    },
                )],
            ) => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                actions.push(gain_block_action(entity, block));
                actions
            }
            (_, []) => panic!("spire shield plan missing locked truth"),
            (move_id, steps) => panic!("spire shield plan/steps mismatch: {} {:?}", move_id, steps),
        };
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }

    fn on_death(state: &mut CombatState, _entity: &MonsterEntity) -> Vec<Action> {
        super::surrounded_cleanup_actions(state)
    }
}
