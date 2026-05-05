use crate::content::cards::CardId;
use crate::content::monsters::exordium::{
    apply_power_action, attack_actions, gain_block_action, PLAYER,
};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    AttackSpec, DamageKind, DebuffSpec, DefendSpec, EffectStrength, MonsterMoveSpec,
    MonsterTurnPlan, MoveStep, PowerEffectKind,
};
use smallvec::smallvec;

pub struct WrithingMass;

const BIG_HIT: u8 = 0;
const MULTI_HIT: u8 = 1;
const ATTACK_BLOCK: u8 = 2;
const ATTACK_DEBUFF: u8 = 3;
const MEGA_DEBUFF: u8 = 4;

fn big_hit_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 2 {
        38
    } else {
        32
    }
}

fn multi_hit_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 2 {
        9
    } else {
        7
    }
}

fn attack_block_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 2 {
        16
    } else {
        15
    }
}

fn attack_debuff_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 2 {
        12
    } else {
        10
    }
}

fn normal_debuff_amt() -> i32 {
    2
}

fn big_hit_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        BIG_HIT,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: big_hit_damage(ascension_level),
            hits: 1,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn multi_hit_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        MULTI_HIT,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: multi_hit_damage(ascension_level),
            hits: 3,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn attack_block_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        ATTACK_BLOCK,
        MonsterMoveSpec::AttackDefend(
            AttackSpec {
                base_damage: attack_block_damage(ascension_level),
                hits: 1,
                damage_kind: DamageKind::Normal,
            },
            DefendSpec {
                block: attack_block_damage(ascension_level),
            },
        ),
    )
}

fn attack_debuff_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        ATTACK_DEBUFF,
        smallvec![
            MoveStep::Attack(crate::semantics::combat::AttackStep {
                target: crate::semantics::combat::MoveTarget::Player,
                attack: AttackSpec {
                    base_damage: attack_debuff_damage(ascension_level),
                    hits: 1,
                    damage_kind: DamageKind::Normal,
                },
            }),
            MoveStep::ApplyPower(crate::semantics::combat::ApplyPowerStep {
                target: crate::semantics::combat::MoveTarget::Player,
                power_id: PowerId::Weak,
                amount: normal_debuff_amt(),
                effect: PowerEffectKind::Debuff,
                visible_strength: EffectStrength::Normal,
            }),
            MoveStep::ApplyPower(crate::semantics::combat::ApplyPowerStep {
                target: crate::semantics::combat::MoveTarget::Player,
                power_id: PowerId::Vulnerable,
                amount: normal_debuff_amt(),
                effect: PowerEffectKind::Debuff,
                visible_strength: EffectStrength::Normal,
            }),
        ],
        MonsterMoveSpec::AttackDebuff(
            AttackSpec {
                base_damage: attack_debuff_damage(ascension_level),
                hits: 1,
                damage_kind: DamageKind::Normal,
            },
            DebuffSpec {
                power_id: PowerId::Weak,
                amount: normal_debuff_amt(),
                strength: EffectStrength::Normal,
            },
        ),
    )
}

fn mega_debuff_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::unknown(MEGA_DEBUFF)
}

fn plan_for(move_id: u8, ascension_level: u8) -> MonsterTurnPlan {
    match move_id {
        BIG_HIT => big_hit_plan(ascension_level),
        MULTI_HIT => multi_hit_plan(ascension_level),
        ATTACK_BLOCK => attack_block_plan(ascension_level),
        ATTACK_DEBUFF => attack_debuff_plan(ascension_level),
        MEGA_DEBUFF => mega_debuff_plan(),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn last_move(entity: &MonsterEntity, move_id: u8) -> bool {
    entity.move_history().back().copied() == Some(move_id)
}

fn used_mega_debuff(entity: &MonsterEntity) -> bool {
    entity
        .move_history()
        .iter()
        .any(|&move_id| move_id == MEGA_DEBUFF)
}

fn roll_move_recursive(
    rng: &mut crate::runtime::rng::StsRng,
    entity: &MonsterEntity,
    ascension_level: u8,
    num: i32,
) -> MonsterTurnPlan {
    if entity.move_history().is_empty() {
        return if num < 33 {
            multi_hit_plan(ascension_level)
        } else if num < 66 {
            attack_block_plan(ascension_level)
        } else {
            attack_debuff_plan(ascension_level)
        };
    }

    if num < 10 {
        if !last_move(entity, BIG_HIT) {
            big_hit_plan(ascension_level)
        } else {
            let reroll = rng.random_range(10, 99);
            roll_move_recursive(rng, entity, ascension_level, reroll)
        }
    } else if num < 20 {
        if !used_mega_debuff(entity) && !last_move(entity, MEGA_DEBUFF) {
            mega_debuff_plan()
        } else if rng.random_boolean_chance(0.1) {
            big_hit_plan(ascension_level)
        } else {
            let reroll = rng.random_range(20, 99);
            roll_move_recursive(rng, entity, ascension_level, reroll)
        }
    } else if num < 40 {
        if !last_move(entity, ATTACK_DEBUFF) {
            attack_debuff_plan(ascension_level)
        } else if rng.random_boolean_chance(0.4) {
            let reroll = rng.random(19);
            roll_move_recursive(rng, entity, ascension_level, reroll)
        } else {
            let reroll = rng.random_range(40, 99);
            roll_move_recursive(rng, entity, ascension_level, reroll)
        }
    } else if num < 70 {
        if !last_move(entity, MULTI_HIT) {
            multi_hit_plan(ascension_level)
        } else if rng.random_boolean_chance(0.3) {
            attack_block_plan(ascension_level)
        } else {
            let reroll = rng.random(39);
            roll_move_recursive(rng, entity, ascension_level, reroll)
        }
    } else if !last_move(entity, ATTACK_BLOCK) {
        attack_block_plan(ascension_level)
    } else {
        let reroll = rng.random(69);
        roll_move_recursive(rng, entity, ascension_level, reroll)
    }
}

impl MonsterBehavior for WrithingMass {
    fn roll_move_plan(
        rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> MonsterTurnPlan {
        roll_move_recursive(rng, entity, ascension_level, num)
    }

    fn use_pre_battle_actions(
        state: &mut CombatState,
        entity: &MonsterEntity,
        legacy_rng: crate::content::monsters::PreBattleLegacyRng,
    ) -> Vec<Action> {
        let (_hp_rng, _ascension_level) =
            crate::content::monsters::legacy_pre_battle_rng(state, legacy_rng);
        vec![
            Action::ApplyPower {
                source: entity.id,
                target: entity.id,
                power_id: PowerId::Reactive,
                amount: 1,
            },
            Action::ApplyPower {
                source: entity.id,
                target: entity.id,
                power_id: PowerId::Malleable,
                amount: 3,
            },
        ]
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
            (BIG_HIT | MULTI_HIT, [MoveStep::Attack(attack)]) => {
                attack_actions(entity.id, PLAYER, &attack.attack)
            }
            (ATTACK_BLOCK, [MoveStep::Attack(attack), MoveStep::GainBlock(block)]) => {
                let mut actions = attack_actions(entity.id, PLAYER, &attack.attack);
                actions.push(gain_block_action(entity, block));
                actions
            }
            (
                ATTACK_DEBUFF,
                [MoveStep::Attack(attack), MoveStep::ApplyPower(weak), MoveStep::ApplyPower(vulnerable)],
            ) => {
                let mut actions = attack_actions(entity.id, PLAYER, &attack.attack);
                actions.push(apply_power_action(entity, weak));
                actions.push(apply_power_action(entity, vulnerable));
                actions
            }
            (MEGA_DEBUFF, []) => vec![Action::AddCardToMasterDeck {
                card_id: CardId::Parasite,
            }],
            (move_id, steps) => {
                panic!("writhing mass plan/steps mismatch: {} {:?}", move_id, steps)
            }
        };

        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
