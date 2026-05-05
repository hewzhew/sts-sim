use crate::content::cards::CardId;
use crate::content::monsters::exordium::{
    add_card_action, apply_power_action, attack_actions, PLAYER,
};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    AddCardStep, ApplyPowerStep, AttackSpec, AttackStep, CardDestination, DamageKind,
    EffectStrength, MonsterMoveSpec, MonsterTurnPlan, MoveStep, MoveTarget, PowerEffectKind,
};
use smallvec::smallvec;

pub struct Taskmaster;

const SCOURING_WHIP: u8 = 2;

fn whip_damage() -> i32 {
    7
}

fn wound_count(ascension_level: u8) -> u8 {
    if ascension_level >= 18 {
        3
    } else if ascension_level >= 3 {
        2
    } else {
        1
    }
}

fn whip_plan(ascension_level: u8) -> MonsterTurnPlan {
    let add_wound = AddCardStep {
        card_id: CardId::Wound,
        amount: wound_count(ascension_level),
        upgraded: false,
        destination: CardDestination::Discard,
        visible_strength: EffectStrength::Normal,
    };
    MonsterTurnPlan::with_visible_spec(
        SCOURING_WHIP,
        smallvec![
            MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack: AttackSpec {
                    base_damage: whip_damage(),
                    hits: 1,
                    damage_kind: DamageKind::Normal,
                },
            }),
            MoveStep::AddCard(add_wound.clone()),
        ],
        MonsterMoveSpec::AttackAddCard(
            AttackSpec {
                base_damage: whip_damage(),
                hits: 1,
                damage_kind: DamageKind::Normal,
            },
            add_wound,
        ),
    )
}

fn plan_for(move_id: u8, ascension_level: u8) -> MonsterTurnPlan {
    match move_id {
        SCOURING_WHIP => whip_plan(ascension_level),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

impl MonsterBehavior for Taskmaster {
    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        _entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> MonsterTurnPlan {
        whip_plan(ascension_level)
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
                SCOURING_WHIP,
                [MoveStep::Attack(AttackStep {
                    target: MoveTarget::Player,
                    attack,
                }), MoveStep::AddCard(add_wound)],
            ) => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                actions.push(add_card_action(add_wound));
                actions
            }
            (_, []) => panic!("taskmaster plan missing locked truth"),
            (move_id, steps) => panic!("taskmaster plan/steps mismatch: {} {:?}", move_id, steps),
        };

        if state.meta.ascension_level >= 18 {
            actions.push(apply_power_action(
                entity,
                &ApplyPowerStep {
                    target: MoveTarget::SelfTarget,
                    power_id: PowerId::Strength,
                    amount: 1,
                    effect: PowerEffectKind::Buff,
                    visible_strength: EffectStrength::Normal,
                },
            ));
        }
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
