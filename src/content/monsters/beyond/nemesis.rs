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

pub struct Nemesis;

const TRI_ATTACK: u8 = 2;
const SCYTHE: u8 = 3;
const TRI_BURN: u8 = 4;

fn scythe_damage() -> i32 {
    45
}

fn fire_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 3 {
        7
    } else {
        6
    }
}

fn burn_amount(ascension_level: u8) -> i32 {
    if ascension_level >= 18 {
        5
    } else {
        3
    }
}

fn tri_attack_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        TRI_ATTACK,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: fire_damage(ascension_level),
            hits: 3,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn scythe_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        SCYTHE,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: scythe_damage(),
            hits: 1,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn tri_burn_plan(ascension_level: u8) -> MonsterTurnPlan {
    let burn = AddCardStep {
        card_id: CardId::Burn,
        amount: burn_amount(ascension_level) as u8,
        upgraded: false,
        destination: CardDestination::Discard,
        visible_strength: EffectStrength::Normal,
    };
    MonsterTurnPlan::from_spec(TRI_BURN, MonsterMoveSpec::AddCard(burn))
}

fn plan_for(move_id: u8, ascension_level: u8) -> MonsterTurnPlan {
    match move_id {
        TRI_ATTACK => tri_attack_plan(ascension_level),
        SCYTHE => scythe_plan(),
        TRI_BURN => tri_burn_plan(ascension_level),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn last_move(entity: &MonsterEntity, move_id: u8) -> bool {
    entity.move_history().back().copied() == Some(move_id)
}

fn last_two_moves(entity: &MonsterEntity, move_id: u8) -> bool {
    let history = entity.move_history();
    history.len() >= 2
        && history[history.len() - 1] == move_id
        && history[history.len() - 2] == move_id
}

fn reconstruct_scythe_cooldown(history: &std::collections::VecDeque<u8>) -> i32 {
    let mut last_scythe_idx = None;
    for (i, &move_id) in history.iter().enumerate() {
        if move_id == SCYTHE {
            last_scythe_idx = Some(i);
        }
    }

    match last_scythe_idx {
        Some(idx) => 1 - ((history.len() - 1 - idx) as i32),
        None => -(history.len() as i32) - 1,
    }
}

impl MonsterBehavior for Nemesis {
    fn roll_move_plan(
        rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> MonsterTurnPlan {
        if entity.move_history().is_empty() {
            return if num < 50 {
                tri_attack_plan(ascension_level)
            } else {
                tri_burn_plan(ascension_level)
            };
        }

        let scythe_cooldown = reconstruct_scythe_cooldown(entity.move_history());

        if num < 30 {
            if !last_move(entity, SCYTHE) && scythe_cooldown <= 0 {
                scythe_plan()
            } else if rng.random_boolean() {
                if !last_two_moves(entity, TRI_ATTACK) {
                    tri_attack_plan(ascension_level)
                } else {
                    tri_burn_plan(ascension_level)
                }
            } else if !last_move(entity, TRI_BURN) {
                tri_burn_plan(ascension_level)
            } else {
                tri_attack_plan(ascension_level)
            }
        } else if num < 65 {
            if !last_two_moves(entity, TRI_ATTACK) {
                tri_attack_plan(ascension_level)
            } else if rng.random_boolean() {
                if scythe_cooldown > 0 {
                    tri_burn_plan(ascension_level)
                } else {
                    scythe_plan()
                }
            } else {
                tri_burn_plan(ascension_level)
            }
        } else if !last_move(entity, TRI_BURN) {
            tri_burn_plan(ascension_level)
        } else if rng.random_boolean() && scythe_cooldown <= 0 {
            scythe_plan()
        } else {
            tri_attack_plan(ascension_level)
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
            (SCYTHE | TRI_ATTACK, [MoveStep::Attack(attack)]) => {
                attack_actions(entity.id, PLAYER, &attack.attack)
            }
            (TRI_BURN, [MoveStep::AddCard(add_card)]) => vec![add_card_action(add_card)],
            (move_id, steps) => panic!("nemesis plan/steps mismatch: {} {:?}", move_id, steps),
        };

        if crate::content::powers::store::power_amount(state, entity.id, PowerId::Intangible) <= 0 {
            actions.push(Action::ApplyPower {
                source: entity.id,
                target: entity.id,
                power_id: PowerId::Intangible,
                amount: 1,
            });
        }
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
