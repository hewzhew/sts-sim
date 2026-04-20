use crate::content::cards::CardId;
use crate::content::monsters::exordium::{add_card_action, attack_actions, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    AddCardStep, ApplyPowerStep, AttackSpec, AttackStep, CardDestination, DamageKind, DefendSpec,
    EffectStrength, MonsterMoveSpec, MonsterTurnPlan, MoveStep, MoveTarget, PowerEffectKind,
};
use smallvec::smallvec;

pub struct Deca;

const BEAM: u8 = 0;
const SQUARE_OF_PROTECTION: u8 = 2;

fn beam_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 4 {
        12
    } else {
        10
    }
}

fn beam_plan(ascension_level: u8) -> MonsterTurnPlan {
    let dazed = AddCardStep {
        card_id: CardId::Dazed,
        amount: 2,
        upgraded: false,
        destination: CardDestination::Discard,
        visible_strength: EffectStrength::Normal,
    };
    MonsterTurnPlan::with_visible_spec(
        BEAM,
        smallvec![
            MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack: AttackSpec {
                    base_damage: beam_damage(ascension_level),
                    hits: 2,
                    damage_kind: DamageKind::Normal,
                },
            }),
            MoveStep::AddCard(dazed.clone()),
        ],
        MonsterMoveSpec::AttackAddCard(
            AttackSpec {
                base_damage: beam_damage(ascension_level),
                hits: 2,
                damage_kind: DamageKind::Normal,
            },
            dazed,
        ),
    )
}

fn square_plan(ascension_level: u8) -> MonsterTurnPlan {
    let steps = if ascension_level >= 19 {
        smallvec![
            MoveStep::GainBlock(crate::semantics::combat::BlockStep {
                target: MoveTarget::AllMonsters,
                amount: 16,
            }),
            MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::AllMonsters,
                power_id: PowerId::PlatedArmor,
                amount: 3,
                effect: PowerEffectKind::Buff,
                visible_strength: EffectStrength::Normal,
            }),
        ]
    } else {
        smallvec![MoveStep::GainBlock(crate::semantics::combat::BlockStep {
            target: MoveTarget::AllMonsters,
            amount: 16,
        })]
    };

    let visible = if ascension_level >= 19 {
        MonsterMoveSpec::DefendBuff(
            DefendSpec { block: 16 },
            crate::semantics::combat::BuffSpec {
                power_id: PowerId::PlatedArmor,
                amount: 3,
            },
        )
    } else {
        MonsterMoveSpec::Defend(DefendSpec { block: 16 })
    };

    MonsterTurnPlan::with_visible_spec(SQUARE_OF_PROTECTION, steps, visible)
}

fn plan_for(move_id: u8, ascension_level: u8) -> MonsterTurnPlan {
    match move_id {
        BEAM => beam_plan(ascension_level),
        SQUARE_OF_PROTECTION => square_plan(ascension_level),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

impl MonsterBehavior for Deca {
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
            None => beam_plan(ascension_level),
            Some(BEAM) => square_plan(ascension_level),
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
                }), MoveStep::AddCard(add_card)],
            ) => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                actions.push(add_card_action(add_card));
                actions
            }
            (
                SQUARE_OF_PROTECTION,
                [MoveStep::GainBlock(crate::semantics::combat::BlockStep {
                    target: MoveTarget::AllMonsters,
                    amount,
                })],
            ) => state
                .entities
                .monsters
                .iter()
                .map(|monster| Action::GainBlock {
                    target: monster.id,
                    amount: *amount,
                })
                .collect(),
            (
                SQUARE_OF_PROTECTION,
                [MoveStep::GainBlock(crate::semantics::combat::BlockStep {
                    target: MoveTarget::AllMonsters,
                    amount,
                }), MoveStep::ApplyPower(ApplyPowerStep {
                    target: MoveTarget::AllMonsters,
                    power_id: PowerId::PlatedArmor,
                    amount: plated_amount,
                    effect: PowerEffectKind::Buff,
                    ..
                })],
            ) => {
                let mut actions = Vec::new();
                for monster in &state.entities.monsters {
                    actions.push(Action::GainBlock {
                        target: monster.id,
                        amount: *amount,
                    });
                    actions.push(Action::ApplyPower {
                        source: entity.id,
                        target: monster.id,
                        power_id: PowerId::PlatedArmor,
                        amount: *plated_amount,
                    });
                }
                actions
            }
            (_, []) => panic!("deca plan missing locked truth"),
            (move_id, steps) => panic!("deca plan/steps mismatch: {} {:?}", move_id, steps),
        };
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
