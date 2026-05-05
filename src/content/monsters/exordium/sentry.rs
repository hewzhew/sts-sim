use super::{add_card_action, attack_actions, PLAYER};
use crate::content::cards::CardId;
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    AddCardStep, AttackSpec, AttackStep, CardDestination, DamageKind, EffectStrength,
    MonsterTurnPlan, MoveStep, MoveTarget,
};

const BOLT: u8 = 3;
const BEAM: u8 = 4;

pub struct Sentry;

enum SentryTurn<'a> {
    Bolt(&'a AddCardStep),
    Beam(&'a AttackSpec),
}

fn beam_damage(asc: u8) -> i32 {
    if asc >= 3 {
        10
    } else {
        9
    }
}

fn dazed_amount(asc: u8) -> u8 {
    if asc >= 18 {
        3
    } else {
        2
    }
}

fn bolt_plan(asc: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::single(
        BOLT,
        MoveStep::AddCard(AddCardStep {
            card_id: CardId::Dazed,
            amount: dazed_amount(asc),
            upgraded: false,
            destination: CardDestination::Discard,
            visible_strength: EffectStrength::Strong,
        }),
    )
}

fn beam_plan(asc: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::single(
        BEAM,
        MoveStep::Attack(AttackStep {
            target: MoveTarget::Player,
            attack: AttackSpec {
                base_damage: beam_damage(asc),
                hits: 1,
                damage_kind: DamageKind::Normal,
            },
        }),
    )
}

fn plan_for(move_id: u8, asc: u8) -> MonsterTurnPlan {
    match move_id {
        BOLT => bolt_plan(asc),
        BEAM => beam_plan(asc),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn decode_turn<'a>(plan: &'a MonsterTurnPlan) -> SentryTurn<'a> {
    match (plan.move_id, plan.steps.as_slice()) {
        (BOLT, [MoveStep::AddCard(add_card)])
            if add_card.card_id == CardId::Dazed
                && add_card.destination == CardDestination::Discard =>
        {
            SentryTurn::Bolt(add_card)
        }
        (
            BEAM,
            [MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            })],
        ) => SentryTurn::Beam(attack),
        (_, []) => panic!("sentry plan missing locked truth"),
        (move_id, steps) => panic!("sentry plan/steps mismatch: {} {:?}", move_id, steps),
    }
}

impl MonsterBehavior for Sentry {
    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> MonsterTurnPlan {
        match entity.move_history().back().copied() {
            None if entity.slot % 2 == 0 => bolt_plan(ascension_level),
            None => beam_plan(ascension_level),
            Some(BEAM) => bolt_plan(ascension_level),
            Some(_) => beam_plan(ascension_level),
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
        let (_hp_rng, _ascension_level) =
            crate::content::monsters::legacy_pre_battle_rng(state, legacy_rng);
        vec![Action::ApplyPower {
            source: entity.id,
            target: entity.id,
            power_id: PowerId::Artifact,
            amount: 1,
        }]
    }

    fn take_turn_plan(
        _state: &mut CombatState,
        entity: &MonsterEntity,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        match decode_turn(plan) {
            SentryTurn::Bolt(add_card) => vec![
                add_card_action(add_card),
                Action::RollMonsterMove {
                    monster_id: entity.id,
                },
            ],
            SentryTurn::Beam(attack) => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                actions.push(Action::RollMonsterMove {
                    monster_id: entity.id,
                });
                actions
            }
        }
    }
}
