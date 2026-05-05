use crate::content::monsters::exordium::{attack_actions, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    ApplyPowerStep, AttackSpec, AttackStep, BuffSpec, DamageKind, EffectStrength, MonsterMoveSpec,
    MonsterTurnPlan, MoveStep, MoveTarget, PowerEffectKind,
};
use smallvec::smallvec;

pub struct Donu;

const BEAM: u8 = 0;
const CIRCLE_OF_PROTECTION: u8 = 2;

fn beam_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 4 {
        12
    } else {
        10
    }
}

fn beam_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        BEAM,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: beam_damage(ascension_level),
            hits: 2,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn circle_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        CIRCLE_OF_PROTECTION,
        smallvec![MoveStep::ApplyPower(ApplyPowerStep {
            target: MoveTarget::AllMonsters,
            power_id: PowerId::Strength,
            amount: 3,
            effect: PowerEffectKind::Buff,
            visible_strength: EffectStrength::Normal,
        })],
        MonsterMoveSpec::Buff(BuffSpec {
            power_id: PowerId::Strength,
            amount: 3,
        }),
    )
}

fn plan_for(move_id: u8, ascension_level: u8) -> MonsterTurnPlan {
    match move_id {
        BEAM => beam_plan(ascension_level),
        CIRCLE_OF_PROTECTION => circle_plan(),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

impl MonsterBehavior for Donu {
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
            power_id: PowerId::Artifact,
            amount: if ascension_level >= 19 { 3 } else { 2 },
        }]
    }

    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> MonsterTurnPlan {
        match entity.move_history().back().copied() {
            None => circle_plan(),
            Some(BEAM) => circle_plan(),
            _ => beam_plan(ascension_level),
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
                BEAM,
                [MoveStep::Attack(AttackStep {
                    target: MoveTarget::Player,
                    attack,
                })],
            ) => attack_actions(entity.id, PLAYER, attack),
            (
                CIRCLE_OF_PROTECTION,
                [MoveStep::ApplyPower(ApplyPowerStep {
                    target: MoveTarget::AllMonsters,
                    power_id: PowerId::Strength,
                    amount,
                    effect: PowerEffectKind::Buff,
                    ..
                })],
            ) => state
                .entities
                .monsters
                .iter()
                .map(|monster| Action::ApplyPower {
                    source: entity.id,
                    target: monster.id,
                    power_id: PowerId::Strength,
                    amount: *amount,
                })
                .collect(),
            (_, []) => panic!("donu plan missing locked truth"),
            (move_id, steps) => panic!("donu plan/steps mismatch: {} {:?}", move_id, steps),
        };
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
