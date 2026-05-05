use crate::content::monsters::exordium::{apply_power_action, attack_actions, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    ApplyPowerStep, AttackSpec, AttackStep, DamageKind, DebuffSpec, EffectStrength,
    MonsterMoveSpec, MonsterTurnPlan, MoveStep, MoveTarget, PowerEffectKind,
};
use smallvec::smallvec;

pub struct SnakePlant;

const CHOMPY_CHOMPS: u8 = 1;
const SPORES: u8 = 2;

enum SnakePlantTurn<'a> {
    Chomp(&'a AttackSpec),
    Spores(&'a ApplyPowerStep, &'a ApplyPowerStep),
}

fn chomp_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 2 {
        8
    } else {
        7
    }
}

fn chomp_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        CHOMPY_CHOMPS,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: chomp_damage(ascension_level),
            hits: 3,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn spores_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        SPORES,
        smallvec![
            MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::Player,
                power_id: PowerId::Frail,
                amount: 2,
                effect: PowerEffectKind::Debuff,
                visible_strength: EffectStrength::Strong,
            }),
            MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::Player,
                power_id: PowerId::Weak,
                amount: 2,
                effect: PowerEffectKind::Debuff,
                visible_strength: EffectStrength::Strong,
            }),
        ],
        MonsterMoveSpec::StrongDebuff(DebuffSpec {
            power_id: PowerId::Frail,
            amount: 2,
            strength: EffectStrength::Strong,
        }),
    )
}

fn plan_for(move_id: u8, ascension_level: u8) -> MonsterTurnPlan {
    match move_id {
        CHOMPY_CHOMPS => chomp_plan(ascension_level),
        SPORES => spores_plan(),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn last_move(entity: &MonsterEntity, move_id: u8) -> bool {
    entity.move_history().back().copied() == Some(move_id)
}

fn last_move_before(entity: &MonsterEntity, move_id: u8) -> bool {
    let history = entity.move_history();
    history.len() >= 2 && history[history.len() - 2] == move_id
}

fn last_two_moves(entity: &MonsterEntity, move_id: u8) -> bool {
    let history = entity.move_history();
    history.len() >= 2
        && history[history.len() - 1] == move_id
        && history[history.len() - 2] == move_id
}

fn decode_turn<'a>(plan: &'a MonsterTurnPlan) -> SnakePlantTurn<'a> {
    match (plan.move_id, plan.steps.as_slice()) {
        (
            CHOMPY_CHOMPS,
            [MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            })],
        ) => SnakePlantTurn::Chomp(attack),
        (
            SPORES,
            [MoveStep::ApplyPower(
                frail @ ApplyPowerStep {
                    target: MoveTarget::Player,
                    power_id: PowerId::Frail,
                    effect: PowerEffectKind::Debuff,
                    ..
                },
            ), MoveStep::ApplyPower(
                weak @ ApplyPowerStep {
                    target: MoveTarget::Player,
                    power_id: PowerId::Weak,
                    effect: PowerEffectKind::Debuff,
                    ..
                },
            )],
        ) => SnakePlantTurn::Spores(frail, weak),
        (_, []) => panic!("snake plant plan missing locked truth"),
        (move_id, steps) => panic!("snake plant plan/steps mismatch: {} {:?}", move_id, steps),
    }
}

impl MonsterBehavior for SnakePlant {
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
            power_id: PowerId::Malleable,
            amount: 3,
        }]
    }

    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> MonsterTurnPlan {
        if ascension_level >= 17 {
            if num < 65 {
                if last_two_moves(entity, CHOMPY_CHOMPS) {
                    spores_plan()
                } else {
                    chomp_plan(ascension_level)
                }
            } else if last_move(entity, SPORES) || last_move_before(entity, SPORES) {
                chomp_plan(ascension_level)
            } else {
                spores_plan()
            }
        } else if num < 65 {
            if last_two_moves(entity, CHOMPY_CHOMPS) {
                spores_plan()
            } else {
                chomp_plan(ascension_level)
            }
        } else if last_move(entity, SPORES) {
            chomp_plan(ascension_level)
        } else {
            spores_plan()
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
            SnakePlantTurn::Chomp(attack) => attack_actions(entity.id, PLAYER, attack),
            SnakePlantTurn::Spores(frail, weak) => vec![
                apply_power_action(entity, frail),
                apply_power_action(entity, weak),
            ],
        };
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
