use crate::content::cards::CardId;
use crate::content::monsters::exordium::{add_card_action, attack_actions, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::{Action, MonsterRuntimePatch};
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    AddCardStep, ApplyPowerStep, AttackSpec, AttackStep, BuffSpec, DamageKind, EffectStrength,
    MonsterMoveSpec, MonsterTurnPlan, MoveStep, MoveTarget, PowerEffectKind,
};
use smallvec::smallvec;

pub struct SpireSpear;

const BURN_STRIKE: u8 = 1;
const PIERCER: u8 = 2;
const SKEWER: u8 = 3;

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::monsters::EnemyId;

    #[test]
    fn asc18_burn_strike_adds_burns_to_draw_pile_top_like_java() {
        let mut spear = crate::test_support::test_monster(EnemyId::SpireSpear);
        spear.id = 1;
        let plan = burn_strike_plan(18);
        let mut state = crate::test_support::combat_with_monsters(vec![spear.clone()]);

        let actions = SpireSpear::take_turn_plan(&mut state, &spear, &plan);

        assert!(actions.iter().any(|action| matches!(
            action,
            Action::MakeTempCardInDrawPile {
                card_id: CardId::Burn,
                amount: 2,
                random_spot: false,
                to_bottom: false,
                upgraded: false,
            }
        )));
    }

    #[test]
    fn roll_uses_private_move_count_not_truncated_move_history() {
        let mut spear = crate::test_support::test_monster(EnemyId::SpireSpear);
        spear.spire_spear.move_count = 1;
        spear.move_history_mut().clear();

        let plan =
            SpireSpear::roll_move_plan(&mut crate::runtime::rng::StsRng::new(0), &spear, 20, 0);

        assert_eq!(
            plan.move_id, SKEWER,
            "Java SpireSpear.getMove branches on private moveCount, not recoverable moveHistory length"
        );
    }

    #[test]
    fn roll_updates_private_move_count_like_java_get_move() {
        let mut spear = crate::test_support::test_monster(EnemyId::SpireSpear);
        spear.id = 64;
        spear.spire_spear.move_count = 2;

        let actions = SpireSpear::on_roll_move(20, &spear, 0, &piercer_plan());

        assert_eq!(
            actions,
            vec![Action::UpdateMonsterRuntime {
                monster_id: 64,
                patch: MonsterRuntimePatch::SpireSpear {
                    move_count: Some(3),
                    protocol_seeded: Some(true),
                },
            }]
        );
    }
}

fn burn_strike_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 3 {
        6
    } else {
        5
    }
}

fn skewer_hits(ascension_level: u8) -> u8 {
    if ascension_level >= 3 {
        4
    } else {
        3
    }
}

fn burn_destination(ascension_level: u8) -> crate::semantics::combat::CardDestination {
    if ascension_level >= 18 {
        crate::semantics::combat::CardDestination::DrawPileTop
    } else {
        crate::semantics::combat::CardDestination::Discard
    }
}

fn burn_strike_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        BURN_STRIKE,
        smallvec![
            MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack: AttackSpec {
                    base_damage: burn_strike_damage(ascension_level),
                    hits: 2,
                    damage_kind: DamageKind::Normal,
                },
            }),
            MoveStep::AddCard(AddCardStep {
                card_id: CardId::Burn,
                amount: 2,
                upgraded: false,
                destination: burn_destination(ascension_level),
                visible_strength: EffectStrength::Normal,
            }),
        ],
        MonsterMoveSpec::AttackAddCard(
            AttackSpec {
                base_damage: burn_strike_damage(ascension_level),
                hits: 2,
                damage_kind: DamageKind::Normal,
            },
            AddCardStep {
                card_id: CardId::Burn,
                amount: 2,
                upgraded: false,
                destination: burn_destination(ascension_level),
                visible_strength: EffectStrength::Normal,
            },
        ),
    )
}

fn piercer_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        PIERCER,
        smallvec![MoveStep::ApplyPower(ApplyPowerStep {
            target: MoveTarget::AllMonsters,
            power_id: PowerId::Strength,
            amount: 2,
            effect: PowerEffectKind::Buff,
            visible_strength: EffectStrength::Normal,
        })],
        MonsterMoveSpec::Buff(BuffSpec {
            power_id: PowerId::Strength,
            amount: 2,
        }),
    )
}

fn skewer_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        SKEWER,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: 10,
            hits: skewer_hits(ascension_level),
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn plan_for(move_id: u8, ascension_level: u8) -> MonsterTurnPlan {
    match move_id {
        BURN_STRIKE => burn_strike_plan(ascension_level),
        PIERCER => piercer_plan(),
        SKEWER => skewer_plan(ascension_level),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn current_move_count(entity: &MonsterEntity) -> u8 {
    assert!(
        entity.spire_spear.protocol_seeded,
        "spire spear runtime truth must be protocol-seeded or factory-seeded"
    );
    entity.spire_spear.move_count
}

fn increment_move_count(entity: &MonsterEntity) -> Action {
    Action::UpdateMonsterRuntime {
        monster_id: entity.id,
        patch: MonsterRuntimePatch::SpireSpear {
            move_count: Some(entity.spire_spear.move_count.saturating_add(1)),
            protocol_seeded: Some(true),
        },
    }
}

impl MonsterBehavior for SpireSpear {
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
            amount: if ascension_level >= 18 { 2 } else { 1 },
        }]
    }

    fn roll_move_plan(
        rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> MonsterTurnPlan {
        match current_move_count(entity) % 3 {
            0 => {
                if entity.move_history().back().copied() == Some(BURN_STRIKE) {
                    piercer_plan()
                } else {
                    burn_strike_plan(ascension_level)
                }
            }
            1 => skewer_plan(ascension_level),
            _ => {
                if rng.random_boolean() {
                    piercer_plan()
                } else {
                    burn_strike_plan(ascension_level)
                }
            }
        }
    }

    fn on_roll_move(
        _ascension_level: u8,
        entity: &MonsterEntity,
        _num: i32,
        _plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        vec![increment_move_count(entity)]
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
                BURN_STRIKE,
                [MoveStep::Attack(AttackStep {
                    target: MoveTarget::Player,
                    attack,
                }), MoveStep::AddCard(add_card)],
            ) => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                actions.push(add_card_action(add_card));
                actions
            }
            (
                PIERCER,
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
            (
                SKEWER,
                [MoveStep::Attack(AttackStep {
                    target: MoveTarget::Player,
                    attack,
                })],
            ) => attack_actions(entity.id, PLAYER, attack),
            (_, []) => panic!("spire spear plan missing locked truth"),
            (move_id, steps) => panic!("spire spear plan/steps mismatch: {} {:?}", move_id, steps),
        };
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }

    fn on_death(state: &mut CombatState, _entity: &MonsterEntity) -> Vec<Action> {
        super::surrounded_cleanup_actions(state)
    }
}
