use crate::content::monsters::exordium::{apply_power_action, attack_actions, PLAYER};
use crate::content::monsters::{MonsterBehavior, MonsterRollContext};
use crate::content::powers::PowerId;
use crate::runtime::action::{Action, MonsterRuntimePatch};
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    ApplyPowerStep, AttackSpec, DamageKind, DebuffSpec, DefendSpec, EffectStrength,
    MonsterMoveSpec, MonsterTurnPlan, MoveTarget, PowerEffectKind, SpawnHpSpec, SpawnHpValue,
};
use smallvec::smallvec;

pub struct TheCollector;

const SPAWN: u8 = 1;
const FIREBALL: u8 = 2;
const BUFF: u8 = 3;
const MEGA_DEBUFF: u8 = 4;
const REVIVE: u8 = 5;
const TORCH_DRAW_X: [i32; 2] = [770, 647];

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::monsters::EnemyId;

    #[test]
    fn buff_targets_zero_hp_non_dying_non_escaping_monsters_like_java() {
        let collector = crate::test_support::test_monster(EnemyId::TheCollector);
        let mut torch = crate::test_support::test_monster(EnemyId::TorchHead);
        torch.id = 2;
        torch.current_hp = 0;
        torch.is_dying = false;
        torch.is_escaped = false;
        let mut state = crate::test_support::combat_with_monsters(vec![collector.clone(), torch]);
        let plan = buff_plan(state.meta.ascension_level);

        let actions = TheCollector::take_turn_plan(&mut state, &collector, &plan);

        assert!(
            actions.iter().any(|action| matches!(
                action,
                Action::ApplyPower {
                    target: 2,
                    power_id: PowerId::Strength,
                    ..
                }
            )),
            "Java Collector buff skips isDead/isDying/isEscaping only; zero currentHealth alone is not a filter"
        );
    }

    #[test]
    fn death_cleanup_suicides_zero_hp_or_escaped_non_dying_minions_like_java() {
        let collector = crate::test_support::test_monster(EnemyId::TheCollector);
        let mut torch = crate::test_support::test_monster(EnemyId::TorchHead);
        torch.id = 2;
        torch.current_hp = 0;
        torch.is_dying = false;
        torch.is_escaped = true;
        let mut state = crate::test_support::combat_with_monsters(vec![collector.clone(), torch]);

        let actions = TheCollector::on_death(&mut state, &collector);

        assert!(matches!(
            actions.as_slice(),
            [Action::Suicide { target: 2 }]
        ));
    }

    #[test]
    fn revive_considers_dying_torch_even_if_escape_flag_is_set_like_java_enemy_slots() {
        let mut collector = crate::test_support::test_monster(EnemyId::TheCollector);
        let mut torch = crate::test_support::test_monster(EnemyId::TorchHead);
        torch.id = 2;
        torch.is_dying = true;
        torch.is_escaped = true;
        torch.logical_position = 647;
        collector.collector.enemy_slots = [None, Some(2)];
        let mut state = crate::test_support::combat_with_monsters(vec![collector.clone(), torch]);
        state.monster_protocol_identity_mut(2).draw_x = Some(647);

        let actions = TheCollector::take_turn_plan(&mut state, &collector, &revive_plan());

        assert!(
            actions.iter().any(|action| matches!(
                action,
                Action::SpawnCollectorTorch {
                    collector_id: 1,
                    slot: 2,
                    protocol_draw_x: Some(647),
                    ..
                }
            )),
            "Java Collector revive only checks the stored TorchHead isDying flag"
        );
    }

    #[test]
    fn revive_ignores_stale_dying_torch_not_in_java_enemy_slots() {
        let mut collector = crate::test_support::test_monster(EnemyId::TheCollector);
        collector.collector.initial_spawn = false;
        collector.collector.ult_used = true;
        collector.collector.turns_taken = 1;
        collector.collector.enemy_slots = [None, Some(3)];

        let mut stale_torch = crate::test_support::test_monster(EnemyId::TorchHead);
        stale_torch.id = 2;
        stale_torch.is_dying = true;
        let mut current_torch = crate::test_support::test_monster(EnemyId::TorchHead);
        current_torch.id = 3;
        current_torch.is_dying = false;
        let state = crate::test_support::combat_with_monsters(vec![
            collector.clone(),
            stale_torch,
            current_torch,
        ]);

        let plan = TheCollector::roll_move_custom_plan(
            &mut crate::runtime::rng::StsRng::new(0),
            &collector,
            state.meta.ascension_level,
            1,
            &state.entities.monsters,
        );

        assert_eq!(
            plan.move_id, FIREBALL,
            "Java Collector.isMinionDead only checks current enemySlots values, not stale dead TorchHead instances left in the monster group"
        );
    }

    #[test]
    fn initial_spawn_queues_two_torches_runtime_update_then_roll_like_java() {
        let collector = crate::test_support::test_monster(EnemyId::TheCollector);
        let mut state = crate::test_support::combat_with_monsters(vec![collector.clone()]);

        let actions = TheCollector::take_turn_plan(&mut state, &collector, &spawn_plan());

        assert_eq!(
            actions,
            vec![
                spawn_torch_action(collector.id, 1, TORCH_DRAW_X[0]),
                spawn_torch_action(collector.id, 2, TORCH_DRAW_X[1]),
                collector_runtime_update(&collector, Some(false), None, Some(1)),
                Action::RollMonsterMove {
                    monster_id: collector.id,
                },
            ],
            "Java Collector queues two SpawnMonsterAction calls, synchronously clears initialSpawn/increments turnsTaken, then queues RollMoveAction"
        );
    }

    #[test]
    fn roll_move_forces_initial_spawn_then_mega_debuff_until_used() {
        let mut collector = crate::test_support::test_monster(EnemyId::TheCollector);
        let monsters = vec![collector.clone()];

        let initial_plan = TheCollector::roll_move_custom_plan(
            &mut crate::runtime::rng::StsRng::new(0),
            &collector,
            19,
            99,
            &monsters,
        );
        assert_eq!(initial_plan.move_id, SPAWN);

        collector.collector.initial_spawn = false;
        collector.collector.turns_taken = 3;
        collector.collector.ult_used = false;
        let forced_ult_plan = TheCollector::roll_move_custom_plan(
            &mut crate::runtime::rng::StsRng::new(0),
            &collector,
            19,
            99,
            &monsters,
        );
        assert_eq!(forced_ult_plan.move_id, MEGA_DEBUFF);

        collector.collector.ult_used = true;
        let post_ult_plan = TheCollector::roll_move_custom_plan(
            &mut crate::runtime::rng::StsRng::new(0),
            &collector,
            19,
            99,
            &monsters,
        );
        assert_eq!(
            post_ult_plan.move_id, BUFF,
            "After ultUsed is true, Java falls through to the normal Fireball/Buff selector"
        );
    }

    #[test]
    fn roll_move_fireball_history_gate_matches_java_last_two_moves() {
        let mut collector = crate::test_support::test_monster(EnemyId::TheCollector);
        collector.collector.initial_spawn = false;
        collector.collector.ult_used = true;
        collector.collector.turns_taken = 1;
        collector.move_history_mut().push_back(FIREBALL);
        collector.move_history_mut().push_back(FIREBALL);
        let monsters = vec![collector.clone()];

        let plan = TheCollector::roll_move_custom_plan(
            &mut crate::runtime::rng::StsRng::new(0),
            &collector,
            4,
            70,
            &monsters,
        );

        assert_eq!(
            plan.move_id, BUFF,
            "Java Collector blocks Fireball only with lastTwoMoves(FIREBALL), then picks Buff if the last move is not already Buff"
        );
    }

    #[test]
    fn mega_debuff_queues_weak_vulnerable_frail_runtime_update_then_roll_like_java() {
        let mut collector = crate::test_support::test_monster(EnemyId::TheCollector);
        collector.id = 9;
        collector.collector.initial_spawn = false;
        collector.collector.turns_taken = 3;
        collector.collector.ult_used = false;
        let mut state = crate::test_support::combat_with_monsters(vec![collector.clone()]);
        state.meta.ascension_level = 19;

        let actions = TheCollector::take_turn_plan(&mut state, &collector, &mega_debuff_plan(19));

        assert!(matches!(
            actions.as_slice(),
            [
                Action::ApplyPower {
                    target: PLAYER,
                    power_id: PowerId::Weak,
                    amount: 5,
                    ..
                },
                Action::ApplyPower {
                    target: PLAYER,
                    power_id: PowerId::Vulnerable,
                    amount: 5,
                    ..
                },
                Action::ApplyPower {
                    target: PLAYER,
                    power_id: PowerId::Frail,
                    amount: 5,
                    ..
                },
                Action::UpdateMonsterRuntime {
                    monster_id: 9,
                    patch: MonsterRuntimePatch::Collector {
                        initial_spawn: Some(false),
                        ult_used: Some(true),
                        turns_taken: Some(4),
                        enemy_slots: None,
                        protocol_seeded: Some(true),
                    },
                },
                Action::RollMonsterMove { monster_id: 9 },
            ]
        ));
    }
}

fn fireball_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 4 {
        21
    } else {
        18
    }
}

fn strength_amount(ascension_level: u8) -> i32 {
    if ascension_level >= 19 {
        5
    } else if ascension_level >= 4 {
        4
    } else {
        3
    }
}

fn block_amount(ascension_level: u8) -> i32 {
    if ascension_level >= 9 {
        18
    } else {
        15
    }
}

fn actual_buff_block_amount(ascension_level: u8) -> i32 {
    let base = block_amount(ascension_level);
    if ascension_level >= 19 {
        base + 5
    } else {
        base
    }
}

fn mega_debuff_amount(ascension_level: u8) -> i32 {
    if ascension_level >= 19 {
        5
    } else {
        3
    }
}

fn spawn_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(SPAWN, smallvec![], MonsterMoveSpec::Unknown)
}

fn fireball_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        FIREBALL,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: fireball_damage(ascension_level),
            hits: 1,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn buff_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        BUFF,
        smallvec![
            crate::semantics::combat::MoveStep::GainBlock(crate::semantics::combat::BlockStep {
                target: MoveTarget::SelfTarget,
                amount: actual_buff_block_amount(ascension_level),
            }),
            crate::semantics::combat::MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::SelfTarget,
                power_id: PowerId::Strength,
                amount: strength_amount(ascension_level),
                effect: PowerEffectKind::Buff,
                visible_strength: EffectStrength::Normal,
            }),
        ],
        MonsterMoveSpec::DefendBuff(
            DefendSpec {
                block: actual_buff_block_amount(ascension_level),
            },
            crate::semantics::combat::BuffSpec {
                power_id: PowerId::Strength,
                amount: strength_amount(ascension_level),
            },
        ),
    )
}

fn mega_debuff_plan(ascension_level: u8) -> MonsterTurnPlan {
    let amount = mega_debuff_amount(ascension_level);
    MonsterTurnPlan::with_visible_spec(
        MEGA_DEBUFF,
        smallvec![
            crate::semantics::combat::MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::Player,
                power_id: PowerId::Weak,
                amount,
                effect: PowerEffectKind::Debuff,
                visible_strength: EffectStrength::Strong,
            }),
            crate::semantics::combat::MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::Player,
                power_id: PowerId::Vulnerable,
                amount,
                effect: PowerEffectKind::Debuff,
                visible_strength: EffectStrength::Strong,
            }),
            crate::semantics::combat::MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::Player,
                power_id: PowerId::Frail,
                amount,
                effect: PowerEffectKind::Debuff,
                visible_strength: EffectStrength::Strong,
            }),
        ],
        MonsterMoveSpec::StrongDebuff(DebuffSpec {
            power_id: PowerId::Weak,
            amount,
            strength: EffectStrength::Strong,
        }),
    )
}

fn revive_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(REVIVE, smallvec![], MonsterMoveSpec::Unknown)
}

fn plan_for(move_id: u8, ascension_level: u8) -> MonsterTurnPlan {
    match move_id {
        SPAWN => spawn_plan(),
        FIREBALL => fireball_plan(ascension_level),
        BUFF => buff_plan(ascension_level),
        MEGA_DEBUFF => mega_debuff_plan(ascension_level),
        REVIVE => revive_plan(),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn current_runtime(entity: &MonsterEntity) -> (bool, bool, u8) {
    assert!(
        entity.collector.protocol_seeded,
        "collector runtime truth must be protocol-seeded or factory-seeded"
    );
    (
        entity.collector.initial_spawn,
        entity.collector.ult_used,
        entity.collector.turns_taken,
    )
}

fn collector_runtime_update(
    entity: &MonsterEntity,
    initial_spawn: Option<bool>,
    ult_used: Option<bool>,
    turns_taken: Option<u8>,
) -> Action {
    Action::UpdateMonsterRuntime {
        monster_id: entity.id,
        patch: MonsterRuntimePatch::Collector {
            initial_spawn,
            ult_used,
            turns_taken,
            enemy_slots: None,
            protocol_seeded: Some(true),
        },
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

fn monster_by_id(monsters: &[MonsterEntity], id: usize) -> Option<&MonsterEntity> {
    monsters.iter().find(|monster| monster.id == id)
}

fn minion_dead(entity: &MonsterEntity, monsters: &[MonsterEntity]) -> bool {
    entity
        .collector
        .enemy_slots
        .iter()
        .flatten()
        .filter_map(|monster_id| monster_by_id(monsters, *monster_id))
        .any(|monster| monster.is_dying)
}

fn living_monster_ids(state: &CombatState) -> Vec<usize> {
    state
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped)
        .map(|monster| monster.id)
        .collect()
}

fn dying_torch_slots(entity: &MonsterEntity, state: &CombatState) -> Vec<(u8, i32)> {
    let mut slots = entity
        .collector
        .enemy_slots
        .iter()
        .enumerate()
        .filter_map(|(slot_index, monster_id)| {
            let monster = monster_by_id(&state.entities.monsters, (*monster_id)?)?;
            monster.is_dying.then(|| {
                let java_slot = slot_index as u8 + 1;
                let draw_x = state
                    .monster_protocol_identity(monster.id)
                    .and_then(|identity| identity.draw_x)
                    .unwrap_or(monster.logical_position);
                (java_slot, draw_x)
            })
        })
        .collect::<Vec<_>>();
    slots.sort_by(|left, right| right.1.cmp(&left.1));
    slots
}

fn spawn_torch_action(collector_id: usize, slot: u8, draw_x: i32) -> Action {
    Action::SpawnCollectorTorch {
        collector_id,
        slot,
        logical_position: draw_x,
        hp: SpawnHpSpec {
            current: SpawnHpValue::Rolled,
            max: SpawnHpValue::Rolled,
        },
        protocol_draw_x: Some(draw_x),
    }
}

impl TheCollector {
    fn roll_move_custom_plan(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
        monsters: &[MonsterEntity],
    ) -> MonsterTurnPlan {
        let (initial_spawn, ult_used, turns_taken) = current_runtime(entity);

        if initial_spawn {
            return spawn_plan();
        }
        if turns_taken >= 3 && !ult_used {
            return mega_debuff_plan(ascension_level);
        }
        if num <= 25 && minion_dead(entity, monsters) && !last_move(entity, REVIVE) {
            return revive_plan();
        }
        if num <= 70 && !last_two_moves(entity, FIREBALL) {
            return fireball_plan(ascension_level);
        }
        if !last_move(entity, BUFF) {
            return buff_plan(ascension_level);
        }
        fireball_plan(ascension_level)
    }
}

impl MonsterBehavior for TheCollector {
    fn use_pre_battle_actions(
        _state: &mut CombatState,
        _entity: &MonsterEntity,
        _legacy_rng: crate::content::monsters::PreBattleLegacyRng,
    ) -> Vec<Action> {
        Vec::new()
    }

    fn roll_move_plan_with_context(
        rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
        ctx: MonsterRollContext<'_>,
    ) -> MonsterTurnPlan {
        Self::roll_move_custom_plan(rng, entity, ascension_level, num, ctx.monsters)
    }

    fn turn_plan(state: &CombatState, entity: &MonsterEntity) -> MonsterTurnPlan {
        plan_for(entity.planned_move_id(), state.meta.ascension_level)
    }

    fn take_turn_plan(
        state: &mut CombatState,
        entity: &MonsterEntity,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        let next_turns_taken = entity.collector.turns_taken.saturating_add(1);
        let mut actions = match plan.move_id {
            SPAWN => TORCH_DRAW_X
                .into_iter()
                .enumerate()
                .map(|(slot_index, draw_x)| {
                    spawn_torch_action(entity.id, slot_index as u8 + 1, draw_x)
                })
                .collect::<Vec<_>>(),
            FIREBALL => attack_actions(
                entity.id,
                PLAYER,
                &AttackSpec {
                    base_damage: fireball_damage(state.meta.ascension_level),
                    hits: 1,
                    damage_kind: DamageKind::Normal,
                },
            ),
            BUFF => {
                let mut actions = vec![Action::GainBlock {
                    target: entity.id,
                    amount: actual_buff_block_amount(state.meta.ascension_level),
                }];
                for monster_id in living_monster_ids(state) {
                    actions.push(Action::ApplyPower {
                        source: entity.id,
                        target: monster_id,
                        power_id: PowerId::Strength,
                        amount: strength_amount(state.meta.ascension_level),
                    });
                }
                actions
            }
            MEGA_DEBUFF => {
                let amount = mega_debuff_amount(state.meta.ascension_level);
                vec![
                    apply_power_action(
                        entity,
                        &ApplyPowerStep {
                            target: MoveTarget::Player,
                            power_id: PowerId::Weak,
                            amount,
                            effect: PowerEffectKind::Debuff,
                            visible_strength: EffectStrength::Strong,
                        },
                    ),
                    apply_power_action(
                        entity,
                        &ApplyPowerStep {
                            target: MoveTarget::Player,
                            power_id: PowerId::Vulnerable,
                            amount,
                            effect: PowerEffectKind::Debuff,
                            visible_strength: EffectStrength::Strong,
                        },
                    ),
                    apply_power_action(
                        entity,
                        &ApplyPowerStep {
                            target: MoveTarget::Player,
                            power_id: PowerId::Frail,
                            amount,
                            effect: PowerEffectKind::Debuff,
                            visible_strength: EffectStrength::Strong,
                        },
                    ),
                ]
            }
            REVIVE => dying_torch_slots(entity, state)
                .into_iter()
                .map(|(slot, draw_x)| spawn_torch_action(entity.id, slot, draw_x))
                .collect::<Vec<_>>(),
            _ => panic!(
                "collector take_turn received unsupported move {}",
                plan.move_id
            ),
        };
        actions.push(collector_runtime_update(
            entity,
            Some(false),
            (plan.move_id == MEGA_DEBUFF).then_some(true),
            Some(next_turns_taken),
        ));
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
            .filter(|monster| monster.id != entity.id && !monster.is_dying)
            .map(|monster| Action::Suicide { target: monster.id })
            .collect()
    }
}
