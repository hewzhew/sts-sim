use crate::content::monsters::exordium::{apply_power_action, attack_actions, PLAYER};
use crate::content::monsters::{MonsterBehavior, MonsterRollContext};
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    AttackSpec, DamageKind, DebuffSpec, EffectStrength, MonsterMoveSpec, MonsterTurnPlan, MoveStep,
};

pub struct SpireGrowth;

const QUICK_TACKLE: u8 = 1;
const CONSTRICT: u8 = 2;
const SMASH: u8 = 3;

fn tackle_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 2 {
        18
    } else {
        16
    }
}

fn smash_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 2 {
        25
    } else {
        22
    }
}

fn constrict_amount(ascension_level: u8) -> i32 {
    if ascension_level >= 17 {
        12
    } else {
        10
    }
}

fn tackle_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        QUICK_TACKLE,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: tackle_damage(ascension_level),
            hits: 1,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn constrict_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        CONSTRICT,
        MonsterMoveSpec::StrongDebuff(DebuffSpec {
            power_id: PowerId::Constricted,
            amount: constrict_amount(ascension_level),
            strength: EffectStrength::Strong,
        }),
    )
}

fn smash_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        SMASH,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: smash_damage(ascension_level),
            hits: 1,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn last_two_moves(entity: &MonsterEntity, move_id: u8) -> bool {
    let mut history = entity.move_history().iter().rev();
    matches!(
        (history.next().copied(), history.next().copied()),
        (Some(a), Some(b)) if a == move_id && b == move_id
    )
}

impl SpireGrowth {
    fn roll_move_custom_plan(
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
        player_has_constricted: bool,
    ) -> MonsterTurnPlan {
        let last_move = entity.move_history().back().copied();

        if ascension_level >= 17 && !player_has_constricted && last_move != Some(CONSTRICT) {
            return constrict_plan(ascension_level);
        }
        if num < 50 && !last_two_moves(entity, QUICK_TACKLE) {
            return tackle_plan(ascension_level);
        }
        if !player_has_constricted && last_move != Some(CONSTRICT) {
            return constrict_plan(ascension_level);
        }
        if !last_two_moves(entity, SMASH) {
            return smash_plan(ascension_level);
        }
        tackle_plan(ascension_level)
    }
}

fn plan_for(move_id: u8, ascension_level: u8) -> MonsterTurnPlan {
    match move_id {
        QUICK_TACKLE => tackle_plan(ascension_level),
        CONSTRICT => constrict_plan(ascension_level),
        SMASH => smash_plan(ascension_level),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

impl MonsterBehavior for SpireGrowth {
    fn roll_move_plan_with_context(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
        ctx: MonsterRollContext<'_>,
    ) -> MonsterTurnPlan {
        Self::roll_move_custom_plan(
            entity,
            ascension_level,
            num,
            ctx.player_has_power(PowerId::Constricted),
        )
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
            (QUICK_TACKLE, [MoveStep::Attack(attack)]) => {
                attack_actions(entity.id, PLAYER, &attack.attack)
            }
            (CONSTRICT, [MoveStep::ApplyPower(power)]) => vec![apply_power_action(entity, power)],
            (SMASH, [MoveStep::Attack(attack)]) => {
                attack_actions(entity.id, PLAYER, &attack.attack)
            }
            (move_id, steps) => panic!("spire growth plan/steps mismatch: {} {:?}", move_id, steps),
        };
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
