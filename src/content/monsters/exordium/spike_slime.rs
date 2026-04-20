use crate::content::cards::CardId;
use crate::content::monsters::exordium::{
    add_card_action, apply_power_action, attack_actions, spawn_action, PLAYER,
};
use crate::content::monsters::{EnemyId, MonsterBehavior};
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    AddCardStep, ApplyPowerStep, AttackSpec, AttackStep, CardDestination, DamageKind,
    EffectStrength, MonsterMoveSpec, MonsterTurnPlan, MoveStep, MoveTarget, PowerEffectKind,
    SpawnHpSpec, SpawnHpValue, SpawnMonsterStep,
};

const FLAME_TACKLE: u8 = 1;
const SPLIT: u8 = 3;
const FRAIL_LICK: u8 = 4;

pub struct SpikeSlimeL;
pub struct SpikeSlimeM;
pub struct SpikeSlimeS;

fn large_damage(asc: u8) -> i32 {
    if asc >= 2 {
        18
    } else {
        16
    }
}

fn medium_damage(asc: u8) -> i32 {
    if asc >= 2 {
        10
    } else {
        8
    }
}

fn small_damage(asc: u8) -> i32 {
    if asc >= 2 {
        6
    } else {
        5
    }
}

fn frail_turns_large(asc: u8) -> i32 {
    if asc >= 17 {
        3
    } else {
        2
    }
}

fn frail_plan(move_id: u8, amount: i32) -> MonsterTurnPlan {
    MonsterTurnPlan::single(
        move_id,
        MoveStep::ApplyPower(ApplyPowerStep {
            target: MoveTarget::Player,
            power_id: PowerId::Frail,
            amount,
            effect: PowerEffectKind::Debuff,
            visible_strength: EffectStrength::Normal,
        }),
    )
}

fn flame_tackle_plan(move_id: u8, damage: i32, slimed_count: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        move_id,
        smallvec::smallvec![
            MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack: AttackSpec {
                    base_damage: damage,
                    hits: 1,
                    damage_kind: DamageKind::Normal,
                },
            }),
            MoveStep::AddCard(AddCardStep {
                card_id: CardId::Slimed,
                amount: slimed_count,
                upgraded: false,
                destination: CardDestination::Discard,
                visible_strength: EffectStrength::Normal,
            }),
        ],
        MonsterMoveSpec::AttackAddCard(
            AttackSpec {
                base_damage: damage,
                hits: 1,
                damage_kind: DamageKind::Normal,
            },
            AddCardStep {
                card_id: CardId::Slimed,
                amount: slimed_count,
                upgraded: false,
                destination: CardDestination::Discard,
                visible_strength: EffectStrength::Normal,
            },
        ),
    )
}

pub(crate) fn split_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        SPLIT,
        smallvec::smallvec![
            MoveStep::Suicide,
            MoveStep::SpawnMonster(SpawnMonsterStep {
                monster_id: EnemyId::SpikeSlimeM,
                logical_position_offset: -1,
                protocol_draw_x_offset: Some(-134),
                hp: SpawnHpSpec {
                    current: SpawnHpValue::SourceCurrentHp,
                    max: SpawnHpValue::SourceCurrentHp,
                },
                is_minion: false,
            }),
            MoveStep::SpawnMonster(SpawnMonsterStep {
                monster_id: EnemyId::SpikeSlimeM,
                logical_position_offset: 1,
                protocol_draw_x_offset: Some(134),
                hp: SpawnHpSpec {
                    current: SpawnHpValue::SourceCurrentHp,
                    max: SpawnHpValue::SourceCurrentHp,
                },
                is_minion: false,
            }),
        ],
        MonsterMoveSpec::Unknown,
    )
}

fn large_plan_for(move_id: u8, asc: u8) -> MonsterTurnPlan {
    match move_id {
        FLAME_TACKLE => flame_tackle_plan(FLAME_TACKLE, large_damage(asc), 2),
        SPLIT => split_plan(),
        FRAIL_LICK => frail_plan(FRAIL_LICK, frail_turns_large(asc)),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn medium_plan_for(move_id: u8, asc: u8) -> MonsterTurnPlan {
    match move_id {
        FLAME_TACKLE => flame_tackle_plan(FLAME_TACKLE, medium_damage(asc), 1),
        FRAIL_LICK => frail_plan(FRAIL_LICK, 1),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn small_plan_for(move_id: u8, asc: u8) -> MonsterTurnPlan {
    match move_id {
        FLAME_TACKLE => MonsterTurnPlan::single(
            FLAME_TACKLE,
            MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack: AttackSpec {
                    base_damage: small_damage(asc),
                    hits: 1,
                    damage_kind: DamageKind::Normal,
                },
            }),
        ),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn roll_large_plan(entity: &MonsterEntity, asc: u8, num: i32) -> MonsterTurnPlan {
    if asc >= 17 {
        if num < 30 {
            if last_two_moves(entity, FLAME_TACKLE) {
                frail_plan(FRAIL_LICK, frail_turns_large(asc))
            } else {
                flame_tackle_plan(FLAME_TACKLE, large_damage(asc), 2)
            }
        } else if last_move(entity, FRAIL_LICK) {
            flame_tackle_plan(FLAME_TACKLE, large_damage(asc), 2)
        } else {
            frail_plan(FRAIL_LICK, frail_turns_large(asc))
        }
    } else if num < 30 {
        if last_two_moves(entity, FLAME_TACKLE) {
            frail_plan(FRAIL_LICK, frail_turns_large(asc))
        } else {
            flame_tackle_plan(FLAME_TACKLE, large_damage(asc), 2)
        }
    } else if last_two_moves(entity, FRAIL_LICK) {
        flame_tackle_plan(FLAME_TACKLE, large_damage(asc), 2)
    } else {
        frail_plan(FRAIL_LICK, frail_turns_large(asc))
    }
}

fn roll_medium_plan(entity: &MonsterEntity, asc: u8, num: i32) -> MonsterTurnPlan {
    if asc >= 17 {
        if num < 30 {
            if last_two_moves(entity, FLAME_TACKLE) {
                frail_plan(FRAIL_LICK, 1)
            } else {
                flame_tackle_plan(FLAME_TACKLE, medium_damage(asc), 1)
            }
        } else if last_move(entity, FRAIL_LICK) {
            flame_tackle_plan(FLAME_TACKLE, medium_damage(asc), 1)
        } else {
            frail_plan(FRAIL_LICK, 1)
        }
    } else if num < 30 {
        if last_two_moves(entity, FLAME_TACKLE) {
            frail_plan(FRAIL_LICK, 1)
        } else {
            flame_tackle_plan(FLAME_TACKLE, medium_damage(asc), 1)
        }
    } else if last_two_moves(entity, FRAIL_LICK) {
        flame_tackle_plan(FLAME_TACKLE, medium_damage(asc), 1)
    } else {
        frail_plan(FRAIL_LICK, 1)
    }
}

fn last_move(entity: &MonsterEntity, move_id: u8) -> bool {
    entity.move_history().back().copied() == Some(move_id)
}

fn last_two_moves(entity: &MonsterEntity, move_id: u8) -> bool {
    entity.move_history().len() >= 2
        && entity.move_history()[entity.move_history().len() - 1] == move_id
        && entity.move_history()[entity.move_history().len() - 2] == move_id
}

fn execute_steps(entity: &MonsterEntity, plan: &MonsterTurnPlan) -> Vec<Action> {
    let mut actions = Vec::new();
    for step in &plan.steps {
        match step {
            MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            }) => actions.extend(attack_actions(entity.id, PLAYER, attack)),
            MoveStep::ApplyPower(apply_power)
                if apply_power.target == MoveTarget::Player
                    && apply_power.power_id == PowerId::Frail
                    && apply_power.effect == PowerEffectKind::Debuff =>
            {
                actions.push(apply_power_action(entity, apply_power))
            }
            MoveStep::AddCard(add_card) => actions.push(add_card_action(add_card)),
            MoveStep::Suicide => actions.push(Action::Suicide { target: entity.id }),
            MoveStep::SpawnMonster(step) => actions.push(spawn_action(entity, step)),
            other => panic!("spike slime step unsupported in execution: {:?}", other),
        }
    }
    actions
}

impl MonsterBehavior for SpikeSlimeL {
    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> MonsterTurnPlan {
        roll_large_plan(entity, ascension_level, num)
    }

    fn turn_plan(state: &CombatState, entity: &MonsterEntity) -> MonsterTurnPlan {
        large_plan_for(entity.planned_move_id(), state.meta.ascension_level)
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
            power_id: PowerId::Split,
            amount: 1,
        }]
    }

    fn take_turn_plan(
        _state: &mut CombatState,
        entity: &MonsterEntity,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        let mut actions = execute_steps(entity, plan);
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}

impl MonsterBehavior for SpikeSlimeM {
    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> MonsterTurnPlan {
        roll_medium_plan(entity, ascension_level, num)
    }

    fn turn_plan(state: &CombatState, entity: &MonsterEntity) -> MonsterTurnPlan {
        medium_plan_for(entity.planned_move_id(), state.meta.ascension_level)
    }

    fn take_turn_plan(
        _state: &mut CombatState,
        entity: &MonsterEntity,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        let mut actions = execute_steps(entity, plan);
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}

impl MonsterBehavior for SpikeSlimeS {
    fn roll_move_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        _entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> MonsterTurnPlan {
        small_plan_for(FLAME_TACKLE, ascension_level)
    }

    fn turn_plan(state: &CombatState, entity: &MonsterEntity) -> MonsterTurnPlan {
        small_plan_for(entity.planned_move_id(), state.meta.ascension_level)
    }

    fn take_turn_plan(
        _state: &mut CombatState,
        entity: &MonsterEntity,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        match plan.steps.as_slice() {
            [MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            })] => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                actions.push(Action::RollMonsterMove {
                    monster_id: entity.id,
                });
                actions
            }
            [] => panic!("spike slime S plan missing locked truth"),
            steps => panic!("spike slime S plan/steps mismatch: {:?}", steps),
        }
    }
}
