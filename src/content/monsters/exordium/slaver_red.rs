use super::{apply_power_action, attack_actions, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    ApplyPowerStep, AttackSpec, AttackStep, DamageKind, DebuffSpec, EffectStrength,
    MonsterMoveSpec, MonsterTurnPlan, MoveStep, MoveTarget, PowerEffectKind,
};
use smallvec::smallvec;

const STAB: u8 = 1;
const ENTANGLE: u8 = 2;
const SCRAPE: u8 = 3;

pub struct SlaverRed;

enum SlaverRedTurn<'a> {
    Stab(&'a AttackSpec),
    Entangle(&'a ApplyPowerStep),
    Scrape(&'a AttackSpec, &'a ApplyPowerStep),
}

fn stab_damage(asc: u8) -> i32 {
    if asc >= 2 {
        14
    } else {
        13
    }
}

fn scrape_damage(asc: u8) -> i32 {
    if asc >= 2 {
        9
    } else {
        8
    }
}

fn vulnerable_amount(asc: u8) -> i32 {
    if asc >= 17 {
        2
    } else {
        1
    }
}

fn stab_plan(asc: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::single(
        STAB,
        MoveStep::Attack(AttackStep {
            target: MoveTarget::Player,
            attack: AttackSpec {
                base_damage: stab_damage(asc),
                hits: 1,
                damage_kind: DamageKind::Normal,
            },
        }),
    )
}

fn entangle_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::single(
        ENTANGLE,
        MoveStep::ApplyPower(ApplyPowerStep {
            target: MoveTarget::Player,
            power_id: PowerId::Entangle,
            amount: 1,
            effect: PowerEffectKind::Debuff,
            visible_strength: EffectStrength::Strong,
        }),
    )
}

fn scrape_plan(asc: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        SCRAPE,
        smallvec![
            MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack: AttackSpec {
                    base_damage: scrape_damage(asc),
                    hits: 1,
                    damage_kind: DamageKind::Normal,
                },
            }),
            MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::Player,
                power_id: PowerId::Vulnerable,
                amount: vulnerable_amount(asc),
                effect: PowerEffectKind::Debuff,
                visible_strength: EffectStrength::Normal,
            }),
        ],
        MonsterMoveSpec::AttackDebuff(
            AttackSpec {
                base_damage: scrape_damage(asc),
                hits: 1,
                damage_kind: DamageKind::Normal,
            },
            DebuffSpec {
                power_id: PowerId::Vulnerable,
                amount: vulnerable_amount(asc),
                strength: EffectStrength::Normal,
            },
        ),
    )
}

fn plan_for(move_id: u8, asc: u8) -> MonsterTurnPlan {
    match move_id {
        STAB => stab_plan(asc),
        ENTANGLE => entangle_plan(),
        SCRAPE => scrape_plan(asc),
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

fn has_used_entangle(entity: &MonsterEntity) -> bool {
    entity.move_history().contains(&ENTANGLE)
}

fn decode_turn<'a>(plan: &'a MonsterTurnPlan) -> SlaverRedTurn<'a> {
    match (plan.move_id, plan.steps.as_slice()) {
        (
            STAB,
            [MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            })],
        ) => SlaverRedTurn::Stab(attack),
        (
            ENTANGLE,
            [MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::Player,
                power_id: PowerId::Entangle,
                effect: PowerEffectKind::Debuff,
                visible_strength: EffectStrength::Strong,
                ..
            })],
        ) => {
            let MoveStep::ApplyPower(entangle) = &plan.steps[0] else {
                unreachable!()
            };
            SlaverRedTurn::Entangle(entangle)
        }
        (
            SCRAPE,
            [MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            }), MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::Player,
                power_id: PowerId::Vulnerable,
                effect: PowerEffectKind::Debuff,
                ..
            })],
        ) => {
            let MoveStep::ApplyPower(vulnerable) = &plan.steps[1] else {
                unreachable!()
            };
            SlaverRedTurn::Scrape(attack, vulnerable)
        }
        (_, []) => panic!("slaver red plan missing locked truth"),
        (move_id, steps) => panic!("slaver red plan/steps mismatch: {} {:?}", move_id, steps),
    }
}

impl MonsterBehavior for SlaverRed {
    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        asc: u8,
        num: i32,
    ) -> MonsterTurnPlan {
        if entity.move_history().is_empty() {
            return stab_plan(asc);
        }
        if num >= 75 && !has_used_entangle(entity) {
            return entangle_plan();
        }
        if num >= 55 && has_used_entangle(entity) && !last_two_moves(entity, STAB) {
            return stab_plan(asc);
        }
        if asc >= 17 {
            if !last_move(entity, SCRAPE) {
                return scrape_plan(asc);
            }
            return stab_plan(asc);
        }
        if !last_two_moves(entity, SCRAPE) {
            return scrape_plan(asc);
        }
        stab_plan(asc)
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
            SlaverRedTurn::Stab(attack) => attack_actions(entity.id, PLAYER, attack),
            SlaverRedTurn::Entangle(entangle) => vec![apply_power_action(entity, entangle)],
            SlaverRedTurn::Scrape(attack, vulnerable) => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                actions.push(apply_power_action(entity, vulnerable));
                actions
            }
        };
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
