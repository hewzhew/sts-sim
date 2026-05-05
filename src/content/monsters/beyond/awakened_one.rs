use crate::content::cards::CardId;
use crate::content::monsters::exordium::{add_card_action, attack_actions, PLAYER};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::{Action, MonsterRuntimePatch};
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    AddCardStep, AttackSpec, AttackStep, CardDestination, DamageKind, EffectStrength,
    MonsterMoveSpec, MonsterTurnPlan, MoveStep, MoveTarget,
};
use smallvec::smallvec;

pub struct AwakenedOne;

const SLASH: u8 = 1;
const SOUL_STRIKE: u8 = 2;
const REBIRTH: u8 = 3;
const DARK_ECHO: u8 = 5;
const SLUDGE: u8 = 6;
const TACKLE: u8 = 8;

fn current_runtime(entity: &MonsterEntity) -> (bool, bool) {
    (entity.awakened_one.form1, entity.awakened_one.first_turn)
}

fn awakened_one_runtime_update(
    entity: &MonsterEntity,
    form1: Option<bool>,
    first_turn: Option<bool>,
) -> Action {
    Action::UpdateMonsterRuntime {
        monster_id: entity.id,
        patch: MonsterRuntimePatch::AwakenedOne {
            form1,
            first_turn,
            protocol_seeded: Some(true),
        },
    }
}

fn last_move(entity: &MonsterEntity, move_id: u8) -> bool {
    entity.move_history().back().copied() == Some(move_id)
}

fn last_two_moves(entity: &MonsterEntity, move_id: u8) -> bool {
    let mut history = entity.move_history().iter().rev();
    matches!(
        (history.next().copied(), history.next().copied()),
        (Some(a), Some(b)) if a == move_id && b == move_id
    )
}

fn slash_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        SLASH,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: 20,
            hits: 1,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn soul_strike_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        SOUL_STRIKE,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: 6,
            hits: 4,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn rebirth_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::unknown(REBIRTH)
}

fn dark_echo_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        DARK_ECHO,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: 40,
            hits: 1,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn sludge_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        SLUDGE,
        smallvec![
            MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack: AttackSpec {
                    base_damage: 18,
                    hits: 1,
                    damage_kind: DamageKind::Normal,
                },
            }),
            MoveStep::AddCard(AddCardStep {
                card_id: CardId::Void,
                amount: 1,
                upgraded: false,
                destination: CardDestination::DrawPileRandom,
                visible_strength: EffectStrength::Normal,
            }),
        ],
        MonsterMoveSpec::AttackAddCard(
            AttackSpec {
                base_damage: 18,
                hits: 1,
                damage_kind: DamageKind::Normal,
            },
            AddCardStep {
                card_id: CardId::Void,
                amount: 1,
                upgraded: false,
                destination: CardDestination::DrawPileRandom,
                visible_strength: EffectStrength::Normal,
            },
        ),
    )
}

fn tackle_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        TACKLE,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: 10,
            hits: 3,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn plan_for(move_id: u8) -> MonsterTurnPlan {
    match move_id {
        SLASH => slash_plan(),
        SOUL_STRIKE => soul_strike_plan(),
        REBIRTH => rebirth_plan(),
        DARK_ECHO => dark_echo_plan(),
        SLUDGE => sludge_plan(),
        TACKLE => tackle_plan(),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

impl MonsterBehavior for AwakenedOne {
    fn use_pre_battle_actions(
        state: &mut CombatState,
        entity: &MonsterEntity,
        legacy_rng: crate::content::monsters::PreBattleLegacyRng,
    ) -> Vec<Action> {
        let (_rng, ascension_level) =
            crate::content::monsters::legacy_pre_battle_rng(state, legacy_rng);
        let regen_amount = if ascension_level >= 19 { 15 } else { 10 };
        let curiosity_amount = if ascension_level >= 19 { 2 } else { 1 };
        let mut actions = vec![
            Action::ApplyPower {
                source: entity.id,
                target: entity.id,
                power_id: PowerId::Regen,
                amount: regen_amount,
            },
            Action::ApplyPower {
                source: entity.id,
                target: entity.id,
                power_id: PowerId::Curiosity,
                amount: curiosity_amount,
            },
            Action::ApplyPower {
                source: entity.id,
                target: entity.id,
                power_id: PowerId::Unawakened,
                amount: 1,
            },
        ];
        if ascension_level >= 4 {
            actions.push(Action::ApplyPower {
                source: entity.id,
                target: entity.id,
                power_id: PowerId::Strength,
                amount: 2,
            });
        }
        actions
    }

    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        _ascension_level: u8,
        num: i32,
    ) -> MonsterTurnPlan {
        let (form1, first_turn) = current_runtime(entity);
        if form1 {
            if first_turn {
                return slash_plan();
            }
            if num < 25 {
                if !last_move(entity, SOUL_STRIKE) {
                    soul_strike_plan()
                } else {
                    slash_plan()
                }
            } else if !last_two_moves(entity, SLASH) {
                slash_plan()
            } else {
                soul_strike_plan()
            }
        } else if first_turn {
            dark_echo_plan()
        } else if num < 50 {
            if !last_two_moves(entity, SLUDGE) {
                sludge_plan()
            } else {
                tackle_plan()
            }
        } else if !last_two_moves(entity, TACKLE) {
            tackle_plan()
        } else {
            sludge_plan()
        }
    }

    fn on_roll_move(
        _ascension_level: u8,
        entity: &MonsterEntity,
        _num: i32,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        let (form1, first_turn) = current_runtime(entity);
        if form1 && first_turn && plan.move_id == SLASH {
            vec![awakened_one_runtime_update(entity, None, Some(false))]
        } else {
            Vec::new()
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
            (
                SLASH | SOUL_STRIKE | DARK_ECHO | TACKLE,
                [MoveStep::Attack(AttackStep {
                    target: MoveTarget::Player,
                    attack,
                })],
            ) => {
                let mut actions = Vec::new();
                if plan.move_id == DARK_ECHO {
                    actions.push(awakened_one_runtime_update(entity, None, Some(false)));
                }
                actions.extend(attack_actions(entity.id, PLAYER, attack));
                actions
            }
            (
                SLUDGE,
                [MoveStep::Attack(AttackStep {
                    target: MoveTarget::Player,
                    attack,
                }), MoveStep::AddCard(add_card)],
            ) => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                actions.push(add_card_action(add_card));
                actions
            }
            (REBIRTH, []) => vec![
                Action::ReviveMonster { target: entity.id },
                Action::Heal {
                    target: entity.id,
                    amount: entity.max_hp,
                },
            ],
            (_, []) => panic!("awakened one plan missing locked truth: {}", plan.move_id),
            (move_id, steps) => panic!("awakened one plan/steps mismatch: {} {:?}", move_id, steps),
        };

        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }

    fn on_death(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        state
            .entities
            .monsters
            .iter()
            .filter(|monster| {
                monster.id != entity.id
                    && !monster.is_dying
                    && !monster.is_escaped
                    && monster.current_hp > 0
                    && crate::content::monsters::EnemyId::from_id(monster.monster_type)
                        == Some(crate::content::monsters::EnemyId::Cultist)
            })
            .map(|monster| Action::Escape { target: monster.id })
            .collect()
    }
}
