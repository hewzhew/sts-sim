use crate::content::cards::CardId;
use crate::content::monsters::exordium::{add_card_action, attack_actions, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    AddCardStep, AttackSpec, CardDestination, DamageKind, EffectStrength, MonsterMoveSpec,
    MonsterTurnPlan, MoveStep,
};
use smallvec::smallvec;

pub struct OrbWalker;

const LASER: u8 = 1;
const CLAW: u8 = 2;

fn laser_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 2 {
        11
    } else {
        10
    }
}

fn claw_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 2 {
        16
    } else {
        15
    }
}

fn burn_discard_step() -> AddCardStep {
    AddCardStep {
        card_id: CardId::Burn,
        amount: 1,
        upgraded: false,
        destination: CardDestination::Discard,
        visible_strength: EffectStrength::Normal,
    }
}

fn burn_draw_step() -> AddCardStep {
    AddCardStep {
        card_id: CardId::Burn,
        amount: 1,
        upgraded: false,
        destination: CardDestination::DrawPileRandom,
        visible_strength: EffectStrength::Normal,
    }
}

fn laser_plan(ascension_level: u8) -> MonsterTurnPlan {
    let visible_add = burn_discard_step();
    MonsterTurnPlan::with_visible_spec(
        LASER,
        smallvec![
            MoveStep::Attack(crate::semantics::combat::AttackStep {
                target: crate::semantics::combat::MoveTarget::Player,
                attack: AttackSpec {
                    base_damage: laser_damage(ascension_level),
                    hits: 1,
                    damage_kind: DamageKind::Normal,
                },
            }),
            MoveStep::AddCard(burn_discard_step()),
            MoveStep::AddCard(burn_draw_step()),
        ],
        MonsterMoveSpec::AttackAddCard(
            AttackSpec {
                base_damage: laser_damage(ascension_level),
                hits: 1,
                damage_kind: DamageKind::Normal,
            },
            visible_add,
        ),
    )
}

fn claw_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        CLAW,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: claw_damage(ascension_level),
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

fn plan_for(move_id: u8, ascension_level: u8) -> MonsterTurnPlan {
    match move_id {
        LASER => laser_plan(ascension_level),
        CLAW => claw_plan(ascension_level),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

impl MonsterBehavior for OrbWalker {
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
            power_id: PowerId::GenericStrengthUp,
            amount: if ascension_level >= 17 { 5 } else { 3 },
        }]
    }

    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> MonsterTurnPlan {
        if num < 40 {
            if !last_two_moves(entity, CLAW) {
                claw_plan(ascension_level)
            } else {
                laser_plan(ascension_level)
            }
        } else if !last_two_moves(entity, LASER) {
            laser_plan(ascension_level)
        } else {
            claw_plan(ascension_level)
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
        let mut actions = match (plan.move_id, plan.steps.as_slice()) {
            (CLAW, [MoveStep::Attack(attack)]) => attack_actions(entity.id, PLAYER, &attack.attack),
            (
                LASER,
                [MoveStep::Attack(attack), MoveStep::AddCard(discard_burn), MoveStep::AddCard(draw_burn)],
            ) => {
                let mut actions = attack_actions(entity.id, PLAYER, &attack.attack);
                actions.push(add_card_action(discard_burn));
                actions.push(add_card_action(draw_burn));
                actions
            }
            (move_id, steps) => panic!("orb walker plan/steps mismatch: {} {:?}", move_id, steps),
        };
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
