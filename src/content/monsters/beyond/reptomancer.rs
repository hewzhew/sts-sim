use crate::content::monsters::exordium::{apply_power_action, attack_actions, PLAYER};
use crate::content::monsters::{EnemyId, MonsterBehavior, MonsterRollContext, PreBattleLegacyRng};
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::runtime::rng::StsRng;
use crate::semantics::combat::{
    ApplyPowerStep, AttackSpec, DamageKind, DebuffSpec, EffectStrength, MonsterMoveSpec,
    MonsterTurnPlan, MoveStep, MoveTarget, PowerEffectKind, SpawnHpSpec, SpawnHpValue,
};
use smallvec::smallvec;

pub struct Reptomancer;

impl Reptomancer {
    pub const DAGGER_DRAW_X: [i32; 4] = [210, -220, 180, -250];
}

const SNAKE_STRIKE: u8 = 1;
const SPAWN_DAGGER: u8 = 2;
const BIG_BITE: u8 = 3;

fn snake_strike_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 3 {
        16
    } else {
        13
    }
}

fn big_bite_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 3 {
        34
    } else {
        30
    }
}

fn daggers_per_spawn(ascension_level: u8) -> usize {
    if ascension_level >= 18 {
        2
    } else {
        1
    }
}

fn snake_strike_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        SNAKE_STRIKE,
        smallvec![
            MoveStep::Attack(crate::semantics::combat::AttackStep {
                target: MoveTarget::Player,
                attack: AttackSpec {
                    base_damage: snake_strike_damage(ascension_level),
                    hits: 2,
                    damage_kind: DamageKind::Normal,
                },
            }),
            MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::Player,
                power_id: PowerId::Weak,
                amount: 1,
                effect: PowerEffectKind::Debuff,
                visible_strength: EffectStrength::Normal,
            }),
        ],
        MonsterMoveSpec::AttackDebuff(
            AttackSpec {
                base_damage: snake_strike_damage(ascension_level),
                hits: 2,
                damage_kind: DamageKind::Normal,
            },
            DebuffSpec {
                power_id: PowerId::Weak,
                amount: 1,
                strength: EffectStrength::Normal,
            },
        ),
    )
}

fn spawn_dagger_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(SPAWN_DAGGER, smallvec![], MonsterMoveSpec::Unknown)
}

fn big_bite_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        BIG_BITE,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: big_bite_damage(ascension_level),
            hits: 1,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn plan_for(move_id: u8, ascension_level: u8) -> MonsterTurnPlan {
    match move_id {
        SNAKE_STRIKE => snake_strike_plan(ascension_level),
        SPAWN_DAGGER => spawn_dagger_plan(),
        BIG_BITE => big_bite_plan(ascension_level),
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

fn can_spawn(monsters: &[MonsterEntity], reptomancer_id: usize) -> bool {
    monsters
        .iter()
        .filter(|monster| monster.id != reptomancer_id && !monster.is_dying)
        .count()
        <= 3
}

fn occupied_dagger_slots(state: &CombatState, reptomancer_id: usize) -> [bool; 4] {
    let mut occupied = [false; 4];
    for monster in &state.entities.monsters {
        if monster.id == reptomancer_id
            || monster.is_dying
            || EnemyId::from_id(monster.monster_type) != Some(EnemyId::SnakeDagger)
        {
            continue;
        }
        for (slot, draw_x) in Reptomancer::DAGGER_DRAW_X.iter().enumerate() {
            if monster.logical_position == *draw_x {
                occupied[slot] = true;
            }
        }
    }
    occupied
}

fn roll_move_custom_plan(
    rng: &mut StsRng,
    entity: &MonsterEntity,
    ascension_level: u8,
    num: i32,
    monsters: &[MonsterEntity],
) -> MonsterTurnPlan {
    if entity.move_history().is_empty() {
        return spawn_dagger_plan();
    }

    if num < 33 {
        if !last_move(entity, SNAKE_STRIKE) {
            snake_strike_plan(ascension_level)
        } else {
            let reroll = rng.random_range(33, 99);
            roll_move_custom_plan(rng, entity, ascension_level, reroll, monsters)
        }
    } else if num < 66 {
        if !last_two_moves(entity, SPAWN_DAGGER) {
            if can_spawn(monsters, entity.id) {
                spawn_dagger_plan()
            } else {
                snake_strike_plan(ascension_level)
            }
        } else {
            snake_strike_plan(ascension_level)
        }
    } else if !last_move(entity, BIG_BITE) {
        big_bite_plan(ascension_level)
    } else {
        let reroll = rng.random(65);
        roll_move_custom_plan(rng, entity, ascension_level, reroll, monsters)
    }
}

impl MonsterBehavior for Reptomancer {
    fn use_pre_battle_actions(
        state: &mut CombatState,
        entity: &MonsterEntity,
        _legacy_rng: PreBattleLegacyRng,
    ) -> Vec<Action> {
        state
            .entities
            .monsters
            .iter()
            .filter(|monster| monster.id != entity.id)
            .map(|monster| Action::ApplyPower {
                source: entity.id,
                target: monster.id,
                power_id: PowerId::Minion,
                amount: 1,
            })
            .collect()
    }

    fn roll_move_plan_with_context(
        rng: &mut StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
        ctx: MonsterRollContext<'_>,
    ) -> MonsterTurnPlan {
        roll_move_custom_plan(rng, entity, ascension_level, num, ctx.monsters)
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
            (SNAKE_STRIKE, [MoveStep::Attack(attack), MoveStep::ApplyPower(power)]) => {
                let mut actions = attack_actions(entity.id, PLAYER, &attack.attack);
                actions.push(apply_power_action(entity, power));
                actions
            }
            (SPAWN_DAGGER, []) => {
                let occupied = occupied_dagger_slots(state, entity.id);
                let mut spawned = 0usize;
                let mut actions = Vec::new();
                for (slot, draw_x) in Reptomancer::DAGGER_DRAW_X.iter().enumerate() {
                    if spawned >= daggers_per_spawn(state.meta.ascension_level) || occupied[slot] {
                        continue;
                    }
                    spawned += 1;
                    actions.push(Action::SpawnMonsterSmart {
                        monster_id: EnemyId::SnakeDagger,
                        hp: SpawnHpSpec {
                            current: SpawnHpValue::Rolled,
                            max: SpawnHpValue::Rolled,
                        },
                        logical_position: *draw_x,
                        protocol_draw_x: Some(*draw_x),
                        is_minion: true,
                    });
                }
                actions
            }
            (BIG_BITE, [MoveStep::Attack(attack)]) => {
                attack_actions(entity.id, PLAYER, &attack.attack)
            }
            (move_id, steps) => panic!("reptomancer plan/steps mismatch: {} {:?}", move_id, steps),
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
            .filter(|monster| monster.id != entity.id && !monster.is_dying && !monster.is_escaped)
            .map(|monster| Action::Suicide { target: monster.id })
            .collect()
    }
}
