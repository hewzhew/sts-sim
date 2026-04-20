use crate::content::monsters::exordium::{attack_actions, PLAYER};
use crate::content::monsters::{EnemyId, MonsterBehavior, MonsterRollContext};
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::runtime::rng::StsRng;
use crate::semantics::combat::{
    AttackSpec, BuffSpec, DamageKind, DefendSpec, MonsterMoveSpec, MonsterTurnPlan, SpawnHpSpec,
    SpawnHpValue,
};

pub struct GremlinLeader;

const RALLY: u8 = 2;
const ENCOURAGE: u8 = 3;
const STAB: u8 = 4;
const STAB_DAMAGE: i32 = 6;
const STAB_HITS: u8 = 3;
const SUMMON_POOL: [EnemyId; 8] = [
    EnemyId::GremlinWarrior,
    EnemyId::GremlinWarrior,
    EnemyId::GremlinThief,
    EnemyId::GremlinThief,
    EnemyId::GremlinFat,
    EnemyId::GremlinFat,
    EnemyId::GremlinTsundere,
    EnemyId::GremlinWizard,
];

fn stab_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        STAB,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: STAB_DAMAGE,
            hits: STAB_HITS,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn rally_plan() -> MonsterTurnPlan {
    // The exact summoned gremlins are chosen from aiRng during execution, so the
    // visible truth is only "unknown rally" at planning time.
    MonsterTurnPlan::with_visible_spec(RALLY, smallvec::smallvec![], MonsterMoveSpec::Unknown)
}

fn encourage_strength(ascension_level: u8) -> i32 {
    if ascension_level >= 18 {
        5
    } else if ascension_level >= 3 {
        4
    } else {
        3
    }
}

fn encourage_block(ascension_level: u8) -> i32 {
    if ascension_level >= 18 {
        10
    } else {
        6
    }
}

fn encourage_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        ENCOURAGE,
        smallvec::smallvec![],
        MonsterMoveSpec::DefendBuff(
            DefendSpec {
                block: encourage_block(ascension_level),
            },
            BuffSpec {
                power_id: PowerId::Strength,
                amount: encourage_strength(ascension_level),
            },
        ),
    )
}

fn plan_for(move_id: u8, ascension_level: u8) -> MonsterTurnPlan {
    match move_id {
        RALLY => rally_plan(),
        ENCOURAGE => encourage_plan(ascension_level),
        STAB => stab_plan(),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn last_move(entity: &MonsterEntity, move_id: u8) -> bool {
    entity.move_history().back().copied() == Some(move_id)
}

fn alive_gremlin_count(monsters: &[MonsterEntity], leader_id: usize) -> usize {
    monsters
        .iter()
        .filter(|monster| {
            monster.id != leader_id
                && !monster.is_dying
                && !monster.is_escaped
                && monster.current_hp > 0
        })
        .count()
}

fn choose_move(rng: &mut StsRng, entity: &MonsterEntity, alive_gremlins: usize, num: i32) -> u8 {
    if alive_gremlins == 0 {
        if num < 75 {
            if !last_move(entity, RALLY) {
                RALLY
            } else {
                STAB
            }
        } else if !last_move(entity, STAB) {
            STAB
        } else {
            RALLY
        }
    } else if alive_gremlins < 2 {
        if num < 50 {
            if !last_move(entity, RALLY) {
                RALLY
            } else {
                let reroll = rng.random_range(50, 99);
                choose_move(rng, entity, alive_gremlins, reroll)
            }
        } else if num < 80 {
            if !last_move(entity, ENCOURAGE) {
                ENCOURAGE
            } else {
                STAB
            }
        } else if !last_move(entity, STAB) {
            STAB
        } else {
            let reroll = rng.random_range(0, 80);
            choose_move(rng, entity, alive_gremlins, reroll)
        }
    } else if num < 66 {
        if !last_move(entity, ENCOURAGE) {
            ENCOURAGE
        } else {
            STAB
        }
    } else if !last_move(entity, STAB) {
        STAB
    } else {
        ENCOURAGE
    }
}

impl GremlinLeader {
    pub const GREMLIN_SLOT_DRAW_X: [i32; 3] = [-366, -170, -532];
    pub const GREMLIN_SLOT_LOGICAL_POSITIONS: [i32; 3] = Self::GREMLIN_SLOT_DRAW_X;
    pub const LEADER_LOGICAL_POSITION: i32 = 3;

    fn protocol_draw_x_for_minion(slot: usize, monster_id: EnemyId) -> i32 {
        let slot_draw_x = Self::GREMLIN_SLOT_DRAW_X[slot];
        match monster_id {
            EnemyId::GremlinWizard => slot_draw_x - 35,
            _ => slot_draw_x,
        }
    }

    fn occupied_summon_slots(state: &CombatState, leader_id: usize) -> [bool; 3] {
        let mut occupied = [false; 3];
        for monster in &state.entities.monsters {
            if monster.id == leader_id
                || monster.is_dying
                || monster.is_escaped
                || monster.current_hp <= 0
            {
                continue;
            }
            let draw_x = state
                .monster_protocol_identity(monster.id)
                .and_then(|identity| identity.draw_x)
                .unwrap_or(monster.logical_position);
            for (slot, slot_draw_x) in Self::GREMLIN_SLOT_DRAW_X.iter().enumerate() {
                if draw_x == *slot_draw_x {
                    occupied[slot] = true;
                }
            }
        }
        occupied
    }

    fn living_ally_ids(state: &CombatState, leader_id: usize) -> Vec<usize> {
        state
            .entities
            .monsters
            .iter()
            .filter(|monster| {
                monster.id != leader_id
                    && !monster.is_dying
                    && !monster.is_escaped
                    && monster.current_hp > 0
            })
            .map(|monster| monster.id)
            .collect()
    }

    fn random_summoned_gremlin(rng: &mut StsRng) -> EnemyId {
        SUMMON_POOL[rng.random_range(0, (SUMMON_POOL.len() - 1) as i32) as usize]
    }

    fn roll_move_custom_plan(
        rng: &mut StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
        monsters: &[MonsterEntity],
    ) -> MonsterTurnPlan {
        let move_id = choose_move(rng, entity, alive_gremlin_count(monsters, entity.id), num);
        plan_for(move_id, ascension_level)
    }
}

impl MonsterBehavior for GremlinLeader {
    fn roll_move_plan_with_context(
        rng: &mut StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
        ctx: MonsterRollContext<'_>,
    ) -> MonsterTurnPlan {
        Self::roll_move_custom_plan(rng, entity, ascension_level, num, ctx.monsters)
    }

    fn use_pre_battle_actions(
        state: &mut CombatState,
        entity: &MonsterEntity,
        _legacy_rng: crate::content::monsters::PreBattleLegacyRng,
    ) -> Vec<Action> {
        Self::living_ally_ids(state, entity.id)
            .into_iter()
            .map(|ally_id| Action::ApplyPower {
                source: entity.id,
                target: ally_id,
                power_id: PowerId::Minion,
                amount: 1,
            })
            .collect()
    }

    fn turn_plan(state: &CombatState, entity: &MonsterEntity) -> MonsterTurnPlan {
        plan_for(entity.planned_move_id(), state.meta.ascension_level)
    }

    fn take_turn_plan(
        state: &mut CombatState,
        entity: &MonsterEntity,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        let mut actions = match plan.move_id {
            RALLY => {
                let mut occupied_slots = Self::occupied_summon_slots(state, entity.id);
                let mut actions = Vec::new();
                for _ in 0..2 {
                    let Some(slot) = occupied_slots.iter().position(|occupied| !occupied) else {
                        break;
                    };
                    occupied_slots[slot] = true;
                    let monster_id = Self::random_summoned_gremlin(&mut state.rng.ai_rng);
                    let draw_x = Self::protocol_draw_x_for_minion(slot, monster_id);
                    actions.push(Action::SpawnMonsterSmart {
                        monster_id,
                        logical_position: draw_x,
                        hp: SpawnHpSpec {
                            current: SpawnHpValue::Rolled,
                            max: SpawnHpValue::Rolled,
                        },
                        protocol_draw_x: Some(draw_x),
                        is_minion: true,
                    });
                }
                actions
            }
            ENCOURAGE => {
                let mut actions = vec![Action::ApplyPower {
                    source: entity.id,
                    target: entity.id,
                    power_id: PowerId::Strength,
                    amount: encourage_strength(state.meta.ascension_level),
                }];
                for ally_id in Self::living_ally_ids(state, entity.id) {
                    actions.push(Action::ApplyPower {
                        source: entity.id,
                        target: ally_id,
                        power_id: PowerId::Strength,
                        amount: encourage_strength(state.meta.ascension_level),
                    });
                    actions.push(Action::GainBlock {
                        target: ally_id,
                        amount: encourage_block(state.meta.ascension_level),
                    });
                }
                actions
            }
            STAB => attack_actions(
                entity.id,
                PLAYER,
                &AttackSpec {
                    base_damage: STAB_DAMAGE,
                    hits: STAB_HITS,
                    damage_kind: DamageKind::Normal,
                },
            ),
            _ => panic!(
                "gremlin leader take_turn received unsupported move {}",
                plan.move_id
            ),
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
            })
            .map(|monster| Action::Escape { target: monster.id })
            .collect()
    }
}
