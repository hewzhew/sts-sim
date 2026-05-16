use crate::content::cards::CardId;
use crate::content::monsters::exordium::{add_card_action, attack_actions, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    AddCardStep, AttackSpec, CardDestination, DamageKind, EffectStrength, MonsterMoveSpec,
    MonsterTurnPlan, MoveStep,
};
use smallvec::smallvec;

pub struct SnakeDagger;

const WOUND: u8 = 1;
const EXPLODE: u8 = 2;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::monsters::EnemyId;

    #[test]
    fn explode_uses_lose_hp_action_not_suicide_like_java() {
        let mut dagger = crate::test_support::test_monster(EnemyId::SnakeDagger);
        dagger.current_hp = 7;
        let mut state = crate::test_support::combat_with_monsters(vec![dagger.clone()]);

        let actions = SnakeDagger::take_turn_plan(&mut state, &dagger, &explode_plan());

        assert!(
            actions.iter().any(|action| matches!(
                action,
                Action::LoseHp {
                    target: 1,
                    amount: 7,
                    ..
                }
            )),
            "Java SnakeDagger explode queues LoseHPAction(this, this, currentHealth)"
        );
        assert!(
            !actions
                .iter()
                .any(|action| matches!(action, Action::Suicide { target: 1 })),
            "SuicideAction bypasses the Java monster damage/death pipeline"
        );
    }
}

fn wound_plan() -> MonsterTurnPlan {
    let wound = AddCardStep {
        card_id: CardId::Wound,
        amount: 1,
        upgraded: false,
        destination: CardDestination::Discard,
        visible_strength: EffectStrength::Normal,
    };
    MonsterTurnPlan::with_visible_spec(
        WOUND,
        smallvec![
            MoveStep::Attack(crate::semantics::combat::AttackStep {
                target: crate::semantics::combat::MoveTarget::Player,
                attack: AttackSpec {
                    base_damage: 9,
                    hits: 1,
                    damage_kind: DamageKind::Normal,
                },
            }),
            MoveStep::AddCard(wound.clone()),
        ],
        MonsterMoveSpec::AttackAddCard(
            AttackSpec {
                base_damage: 9,
                hits: 1,
                damage_kind: DamageKind::Normal,
            },
            wound,
        ),
    )
}

fn explode_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        EXPLODE,
        smallvec![
            MoveStep::Attack(crate::semantics::combat::AttackStep {
                target: crate::semantics::combat::MoveTarget::Player,
                attack: AttackSpec {
                    base_damage: 25,
                    hits: 1,
                    damage_kind: DamageKind::Normal,
                },
            }),
            MoveStep::Suicide,
        ],
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: 25,
            hits: 1,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn plan_for(move_id: u8) -> MonsterTurnPlan {
    match move_id {
        WOUND => wound_plan(),
        EXPLODE => explode_plan(),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

impl MonsterBehavior for SnakeDagger {
    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        _ascension_level: u8,
        _num: i32,
    ) -> MonsterTurnPlan {
        if entity.move_history().is_empty() {
            wound_plan()
        } else {
            explode_plan()
        }
    }

    fn turn_plan(_state: &CombatState, entity: &MonsterEntity) -> MonsterTurnPlan {
        plan_for(entity.planned_move_id())
    }

    fn take_turn_plan(
        _state: &mut CombatState,
        entity: &MonsterEntity,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        let mut actions = match (plan.move_id, plan.steps.as_slice()) {
            (WOUND, [MoveStep::Attack(attack), MoveStep::AddCard(wound)]) => {
                let mut actions = attack_actions(entity.id, PLAYER, &attack.attack);
                actions.push(add_card_action(wound));
                actions
            }
            (EXPLODE, [MoveStep::Attack(attack), MoveStep::Suicide]) => {
                let mut actions = attack_actions(entity.id, PLAYER, &attack.attack);
                actions.push(Action::LoseHp {
                    target: entity.id,
                    amount: entity.current_hp,
                    triggers_rupture: false,
                });
                actions
            }
            (move_id, steps) => panic!("snake dagger plan/steps mismatch: {} {:?}", move_id, steps),
        };
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
