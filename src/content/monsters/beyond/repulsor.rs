use crate::content::cards::CardId;
use crate::content::monsters::exordium::{add_card_action, attack_actions, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    AddCardStep, AttackSpec, CardDestination, DamageKind, EffectStrength, MonsterMoveSpec,
    MonsterTurnPlan, MoveStep,
};

pub struct Repulsor;

const DAZE: u8 = 1;
const ATTACK: u8 = 2;

fn attack_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 2 {
        13
    } else {
        11
    }
}

fn attack_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        ATTACK,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: attack_damage(ascension_level),
            hits: 1,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn daze_step() -> AddCardStep {
    AddCardStep {
        card_id: CardId::Dazed,
        amount: 2,
        upgraded: false,
        destination: CardDestination::DrawPileRandom,
        visible_strength: EffectStrength::Normal,
    }
}

fn daze_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(DAZE, MonsterMoveSpec::AddCard(daze_step()))
}

fn plan_for(move_id: u8, ascension_level: u8) -> MonsterTurnPlan {
    match move_id {
        ATTACK => attack_plan(ascension_level),
        DAZE => daze_plan(),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

impl MonsterBehavior for Repulsor {
    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> MonsterTurnPlan {
        if num < 20 && entity.move_history().back().copied() != Some(ATTACK) {
            attack_plan(ascension_level)
        } else {
            daze_plan()
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
            (ATTACK, [MoveStep::Attack(attack)]) => {
                attack_actions(entity.id, PLAYER, &attack.attack)
            }
            (DAZE, [MoveStep::AddCard(add_card)]) => vec![add_card_action(add_card)],
            (move_id, steps) => panic!("repulsor plan/steps mismatch: {} {:?}", move_id, steps),
        };
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
