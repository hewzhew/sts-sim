use crate::content::monsters::exordium::{apply_power_action, attack_actions, PLAYER};
use crate::content::monsters::{EnemyId, MonsterBehavior, MonsterRollContext};
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
        let collector = crate::test_support::test_monster(EnemyId::TheCollector);
        let mut torch = crate::test_support::test_monster(EnemyId::TorchHead);
        torch.id = 2;
        torch.is_dying = true;
        torch.is_escaped = true;
        torch.logical_position = 647;
        let mut state = crate::test_support::combat_with_monsters(vec![collector.clone(), torch]);
        state.monster_protocol_identity_mut(2).draw_x = Some(647);

        let actions = TheCollector::take_turn_plan(&mut state, &collector, &revive_plan());

        assert!(
            actions.iter().any(|action| matches!(
                action,
                Action::SpawnMonsterSmart {
                    monster_id: EnemyId::TorchHead,
                    protocol_draw_x: Some(647),
                    ..
                }
            )),
            "Java Collector revive only checks the stored TorchHead isDying flag"
        );
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

fn minion_dead(monsters: &[MonsterEntity]) -> bool {
    monsters
        .iter()
        .any(|monster| monster.monster_type == EnemyId::TorchHead as usize && monster.is_dying)
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

fn dying_torch_draw_xs(state: &CombatState) -> Vec<i32> {
    let mut positions = state
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.monster_type == EnemyId::TorchHead as usize && monster.is_dying)
        .map(|monster| {
            state
                .monster_protocol_identity(monster.id)
                .and_then(|identity| identity.draw_x)
                .unwrap_or(monster.logical_position)
        })
        .collect::<Vec<_>>();
    positions.sort_by(|left, right| right.cmp(left));
    positions
}

fn spawn_torch_action(draw_x: i32) -> Action {
    Action::SpawnMonsterSmart {
        monster_id: EnemyId::TorchHead,
        logical_position: draw_x,
        hp: SpawnHpSpec {
            current: SpawnHpValue::Rolled,
            max: SpawnHpValue::Rolled,
        },
        protocol_draw_x: Some(draw_x),
        is_minion: true,
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
        if num <= 25 && minion_dead(monsters) && !last_move(entity, REVIVE) {
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
                .map(spawn_torch_action)
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
            REVIVE => dying_torch_draw_xs(state)
                .into_iter()
                .map(spawn_torch_action)
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
