use crate::content::cards::CardId;
use crate::content::monsters::exordium::{
    add_card_action, apply_power_action, attack_actions, set_next_move_action, spawn_action, PLAYER,
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

const WOUND_TACKLE: u8 = 1;
const NORMAL_TACKLE: u8 = 2;
const SPLIT: u8 = 3;
const WEAK_LICK: u8 = 4;

pub struct AcidSlimeL;
pub struct AcidSlimeM;
pub struct AcidSlimeS;

fn large_wound_damage(asc: u8) -> i32 {
    if asc >= 2 {
        12
    } else {
        11
    }
}

fn large_normal_damage(asc: u8) -> i32 {
    if asc >= 2 {
        18
    } else {
        16
    }
}

fn medium_wound_damage(asc: u8) -> i32 {
    if asc >= 2 {
        8
    } else {
        7
    }
}

fn medium_normal_damage(asc: u8) -> i32 {
    if asc >= 2 {
        12
    } else {
        10
    }
}

fn small_tackle_damage(asc: u8) -> i32 {
    if asc >= 2 {
        4
    } else {
        3
    }
}

fn weak_plan(move_id: u8, amount: i32) -> MonsterTurnPlan {
    MonsterTurnPlan::single(
        move_id,
        MoveStep::ApplyPower(ApplyPowerStep {
            target: MoveTarget::Player,
            power_id: PowerId::Weak,
            amount,
            effect: PowerEffectKind::Debuff,
            visible_strength: EffectStrength::Normal,
        }),
    )
}

fn wound_plan(move_id: u8, damage: i32, slimed_count: u8) -> MonsterTurnPlan {
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

fn normal_attack_plan(move_id: u8, damage: i32) -> MonsterTurnPlan {
    MonsterTurnPlan::single(
        move_id,
        MoveStep::Attack(AttackStep {
            target: MoveTarget::Player,
            attack: AttackSpec {
                base_damage: damage,
                hits: 1,
                damage_kind: DamageKind::Normal,
            },
        }),
    )
}

pub(crate) fn split_plan(child_id: EnemyId) -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        SPLIT,
        smallvec::smallvec![
            MoveStep::Suicide,
            MoveStep::SpawnMonster(SpawnMonsterStep {
                monster_id: child_id,
                logical_position_offset: -1,
                protocol_draw_x_offset: Some(-134),
                hp: SpawnHpSpec {
                    current: SpawnHpValue::SourceCurrentHp,
                    max: SpawnHpValue::SourceCurrentHp,
                },
                is_minion: false,
            }),
            MoveStep::SpawnMonster(SpawnMonsterStep {
                monster_id: child_id,
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
        WOUND_TACKLE => wound_plan(WOUND_TACKLE, large_wound_damage(asc), 2),
        NORMAL_TACKLE => normal_attack_plan(NORMAL_TACKLE, large_normal_damage(asc)),
        SPLIT => split_plan(EnemyId::AcidSlimeM),
        WEAK_LICK => weak_plan(WEAK_LICK, 2),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn medium_plan_for(move_id: u8, asc: u8) -> MonsterTurnPlan {
    match move_id {
        WOUND_TACKLE => wound_plan(WOUND_TACKLE, medium_wound_damage(asc), 1),
        NORMAL_TACKLE => normal_attack_plan(NORMAL_TACKLE, medium_normal_damage(asc)),
        WEAK_LICK => weak_plan(WEAK_LICK, 1),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn small_tackle_plan(asc: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::single(
        WOUND_TACKLE,
        MoveStep::Attack(AttackStep {
            target: MoveTarget::Player,
            attack: AttackSpec {
                base_damage: small_tackle_damage(asc),
                hits: 1,
                damage_kind: DamageKind::Normal,
            },
        }),
    )
}

fn small_debuff_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::single(
        NORMAL_TACKLE,
        MoveStep::ApplyPower(ApplyPowerStep {
            target: MoveTarget::Player,
            power_id: PowerId::Weak,
            amount: 1,
            effect: PowerEffectKind::Debuff,
            visible_strength: EffectStrength::Normal,
        }),
    )
}

fn small_plan_for(move_id: u8, asc: u8) -> MonsterTurnPlan {
    match move_id {
        WOUND_TACKLE => small_tackle_plan(asc),
        NORMAL_TACKLE => small_debuff_plan(),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn large_roll_plan(
    rng: &mut crate::runtime::rng::StsRng,
    entity: &MonsterEntity,
    asc: u8,
    num: i32,
) -> MonsterTurnPlan {
    if num < 0 {
        panic!("acid slime L roll num invalid");
    }
    if asc >= 17 {
        if num < 40 {
            if last_two_moves(entity, WOUND_TACKLE) {
                if roll_chance(rng, 60) {
                    normal_attack_plan(NORMAL_TACKLE, large_normal_damage(asc))
                } else {
                    weak_plan(WEAK_LICK, 2)
                }
            } else {
                wound_plan(WOUND_TACKLE, large_wound_damage(asc), 2)
            }
        } else if num < 70 {
            if last_two_moves(entity, NORMAL_TACKLE) {
                if roll_chance(rng, 60) {
                    wound_plan(WOUND_TACKLE, large_wound_damage(asc), 2)
                } else {
                    weak_plan(WEAK_LICK, 2)
                }
            } else {
                normal_attack_plan(NORMAL_TACKLE, large_normal_damage(asc))
            }
        } else if last_move(entity, WEAK_LICK) {
            if roll_chance(rng, 40) {
                wound_plan(WOUND_TACKLE, large_wound_damage(asc), 2)
            } else {
                normal_attack_plan(NORMAL_TACKLE, large_normal_damage(asc))
            }
        } else {
            weak_plan(WEAK_LICK, 2)
        }
    } else if num < 30 {
        if last_two_moves(entity, WOUND_TACKLE) {
            if rng.random_boolean() {
                normal_attack_plan(NORMAL_TACKLE, large_normal_damage(asc))
            } else {
                weak_plan(WEAK_LICK, 2)
            }
        } else {
            wound_plan(WOUND_TACKLE, large_wound_damage(asc), 2)
        }
    } else if num < 70 {
        if last_move(entity, NORMAL_TACKLE) {
            if roll_chance(rng, 40) {
                wound_plan(WOUND_TACKLE, large_wound_damage(asc), 2)
            } else {
                weak_plan(WEAK_LICK, 2)
            }
        } else {
            normal_attack_plan(NORMAL_TACKLE, large_normal_damage(asc))
        }
    } else if last_two_moves(entity, WEAK_LICK) {
        if roll_chance(rng, 40) {
            wound_plan(WOUND_TACKLE, large_wound_damage(asc), 2)
        } else {
            normal_attack_plan(NORMAL_TACKLE, large_normal_damage(asc))
        }
    } else {
        weak_plan(WEAK_LICK, 2)
    }
}

fn medium_roll_plan(
    rng: &mut crate::runtime::rng::StsRng,
    entity: &MonsterEntity,
    asc: u8,
    num: i32,
) -> MonsterTurnPlan {
    if asc >= 17 {
        if num < 40 {
            if last_two_moves(entity, WOUND_TACKLE) {
                if rng.random_boolean() {
                    normal_attack_plan(NORMAL_TACKLE, medium_normal_damage(asc))
                } else {
                    weak_plan(WEAK_LICK, 1)
                }
            } else {
                wound_plan(WOUND_TACKLE, medium_wound_damage(asc), 1)
            }
        } else if num < 80 {
            if last_two_moves(entity, NORMAL_TACKLE) {
                if roll_chance(rng, 50) {
                    wound_plan(WOUND_TACKLE, medium_wound_damage(asc), 1)
                } else {
                    weak_plan(WEAK_LICK, 1)
                }
            } else {
                normal_attack_plan(NORMAL_TACKLE, medium_normal_damage(asc))
            }
        } else if last_move(entity, WEAK_LICK) {
            if roll_chance(rng, 40) {
                wound_plan(WOUND_TACKLE, medium_wound_damage(asc), 1)
            } else {
                normal_attack_plan(NORMAL_TACKLE, medium_normal_damage(asc))
            }
        } else {
            weak_plan(WEAK_LICK, 1)
        }
    } else if num < 30 {
        if last_two_moves(entity, WOUND_TACKLE) {
            if rng.random_boolean() {
                normal_attack_plan(NORMAL_TACKLE, medium_normal_damage(asc))
            } else {
                weak_plan(WEAK_LICK, 1)
            }
        } else {
            wound_plan(WOUND_TACKLE, medium_wound_damage(asc), 1)
        }
    } else if num < 70 {
        if last_move(entity, NORMAL_TACKLE) {
            if roll_chance(rng, 40) {
                wound_plan(WOUND_TACKLE, medium_wound_damage(asc), 1)
            } else {
                weak_plan(WEAK_LICK, 1)
            }
        } else {
            normal_attack_plan(NORMAL_TACKLE, medium_normal_damage(asc))
        }
    } else if last_two_moves(entity, WEAK_LICK) {
        if roll_chance(rng, 40) {
            wound_plan(WOUND_TACKLE, medium_wound_damage(asc), 1)
        } else {
            normal_attack_plan(NORMAL_TACKLE, medium_normal_damage(asc))
        }
    } else {
        weak_plan(WEAK_LICK, 1)
    }
}

fn small_roll_plan(
    rng: &mut crate::runtime::rng::StsRng,
    entity: &MonsterEntity,
    asc: u8,
) -> MonsterTurnPlan {
    if asc >= 17 {
        if last_two_moves(entity, WOUND_TACKLE) {
            small_tackle_plan(asc)
        } else {
            small_debuff_plan()
        }
    } else if rng.random_boolean() {
        small_tackle_plan(asc)
    } else {
        small_debuff_plan()
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

fn roll_chance(rng: &mut crate::runtime::rng::StsRng, percent: i32) -> bool {
    rng.random_range(0, 99) < percent
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
                    && apply_power.power_id == PowerId::Weak
                    && apply_power.effect == PowerEffectKind::Debuff =>
            {
                actions.push(apply_power_action(entity, apply_power))
            }
            MoveStep::AddCard(add_card) => actions.push(add_card_action(add_card)),
            MoveStep::Suicide => actions.push(Action::Suicide { target: entity.id }),
            MoveStep::SpawnMonster(step) => actions.push(spawn_action(entity, step)),
            other => panic!("acid slime step unsupported in execution: {:?}", other),
        }
    }
    actions
}

impl MonsterBehavior for AcidSlimeL {
    fn roll_move_plan(
        rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> MonsterTurnPlan {
        large_roll_plan(rng, entity, ascension_level, num)
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
        if plan.move_id != SPLIT {
            actions.push(Action::RollMonsterMove {
                monster_id: entity.id,
            });
        }
        actions
    }
}

impl MonsterBehavior for AcidSlimeM {
    fn roll_move_plan(
        rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> MonsterTurnPlan {
        medium_roll_plan(rng, entity, ascension_level, num)
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

impl MonsterBehavior for AcidSlimeS {
    fn roll_move_plan(
        rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> MonsterTurnPlan {
        small_roll_plan(rng, entity, ascension_level)
    }

    fn turn_plan(state: &CombatState, entity: &MonsterEntity) -> MonsterTurnPlan {
        small_plan_for(entity.planned_move_id(), state.meta.ascension_level)
    }

    fn take_turn_plan(
        state: &mut CombatState,
        entity: &MonsterEntity,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        match plan.steps.as_slice() {
            [MoveStep::Attack(AttackStep {
                target: MoveTarget::Player,
                attack,
            })] => {
                let mut actions = attack_actions(entity.id, PLAYER, attack);
                actions.push(set_next_move_action(entity, small_debuff_plan()));
                actions
            }
            [MoveStep::ApplyPower(apply_power)]
                if apply_power.target == MoveTarget::Player
                    && apply_power.power_id == PowerId::Weak
                    && apply_power.amount == 1
                    && apply_power.effect == PowerEffectKind::Debuff =>
            {
                vec![
                    apply_power_action(entity, apply_power),
                    set_next_move_action(entity, small_tackle_plan(state.meta.ascension_level)),
                ]
            }
            [] => panic!("acid slime S plan missing locked truth"),
            steps => panic!("acid slime S plan/steps mismatch: {:?}", steps),
        }
    }
}
