use crate::content::monsters::exordium::{
    apply_power_action, attack_actions, gain_block_action, set_next_move_action, PLAYER,
};
use crate::content::monsters::{MonsterBehavior, MonsterRollContext};
use crate::content::powers::PowerId;
use crate::runtime::action::Action;
use crate::runtime::combat::{CombatState, MonsterEntity};
use crate::semantics::combat::{
    ApplyPowerStep, AttackSpec, BuffSpec, DamageKind, DefendSpec, EffectStrength, HealStep,
    MonsterMoveSpec, MonsterTurnPlan, MoveStep, MoveTarget, PowerEffectKind,
};
use smallvec::smallvec;

pub struct Darkling;

const CHOMP: u8 = 1;
const HARDEN: u8 = 2;
const NIP: u8 = 3;
const COUNT: u8 = 4;
const REINCARNATE: u8 = 5;

pub fn roll_nip_damage(hp_rng: &mut crate::runtime::rng::StsRng, ascension_level: u8) -> i32 {
    hp_rng.random_range(
        if ascension_level >= 2 { 9 } else { 7 },
        if ascension_level >= 2 { 13 } else { 11 },
    ) as i32
}

pub fn initialize_runtime_state(
    entity: &mut MonsterEntity,
    hp_rng: &mut crate::runtime::rng::StsRng,
    ascension_level: u8,
) {
    if crate::content::monsters::EnemyId::from_id(entity.monster_type)
        != Some(crate::content::monsters::EnemyId::Darkling)
    {
        return;
    }

    entity.darkling.first_move = true;
    entity.darkling.nip_dmg = roll_nip_damage(hp_rng, ascension_level);
}

fn chomp_damage(ascension_level: u8) -> i32 {
    if ascension_level >= 2 {
        9
    } else {
        8
    }
}

fn is_even_position(entity: &MonsterEntity, monsters: &[MonsterEntity]) -> bool {
    let position = if monsters.len() <= entity.slot as usize {
        entity.slot as usize
    } else {
        monsters
            .iter()
            .rposition(|monster| monster.id == entity.id)
            .unwrap_or(entity.slot as usize)
    };
    position % 2 == 0
}

fn current_nip_damage(entity: &MonsterEntity, ascension_level: u8) -> i32 {
    if entity.darkling.nip_dmg > 0 {
        entity.darkling.nip_dmg
    } else if entity.planned_move_id() == NIP {
        entity
            .turn_plan()
            .attack()
            .map(|attack| attack.base_damage)
            .unwrap_or(if ascension_level >= 2 { 11 } else { 9 })
    } else if ascension_level >= 2 {
        11
    } else {
        9
    }
}

fn chomp_plan(ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        CHOMP,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: chomp_damage(ascension_level),
            hits: 2,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn harden_plan(ascension_level: u8) -> MonsterTurnPlan {
    if ascension_level >= 17 {
        MonsterTurnPlan::with_visible_spec(
            HARDEN,
            smallvec![
                MoveStep::GainBlock(crate::semantics::combat::BlockStep {
                    target: MoveTarget::SelfTarget,
                    amount: 12,
                }),
                MoveStep::ApplyPower(ApplyPowerStep {
                    target: MoveTarget::SelfTarget,
                    power_id: PowerId::Strength,
                    amount: 2,
                    effect: PowerEffectKind::Buff,
                    visible_strength: EffectStrength::Normal,
                }),
            ],
            MonsterMoveSpec::DefendBuff(
                DefendSpec { block: 12 },
                BuffSpec {
                    power_id: PowerId::Strength,
                    amount: 2,
                },
            ),
        )
    } else {
        MonsterTurnPlan::from_spec(HARDEN, MonsterMoveSpec::Defend(DefendSpec { block: 12 }))
    }
}

fn nip_plan(entity: &MonsterEntity, ascension_level: u8) -> MonsterTurnPlan {
    MonsterTurnPlan::from_spec(
        NIP,
        MonsterMoveSpec::Attack(AttackSpec {
            base_damage: current_nip_damage(entity, ascension_level),
            hits: 1,
            damage_kind: DamageKind::Normal,
        }),
    )
}

fn count_plan() -> MonsterTurnPlan {
    MonsterTurnPlan::unknown(COUNT)
}

fn reincarnate_plan(entity: &MonsterEntity) -> MonsterTurnPlan {
    MonsterTurnPlan::with_visible_spec(
        REINCARNATE,
        smallvec![
            MoveStep::Heal(HealStep {
                target: MoveTarget::SelfTarget,
                amount: entity.max_hp / 2,
            }),
            MoveStep::ApplyPower(ApplyPowerStep {
                target: MoveTarget::SelfTarget,
                power_id: PowerId::Regrow,
                amount: -1,
                effect: PowerEffectKind::Buff,
                visible_strength: EffectStrength::Normal,
            }),
        ],
        MonsterMoveSpec::Buff(BuffSpec {
            power_id: PowerId::Regrow,
            amount: 1,
        }),
    )
}

fn plan_for(entity: &MonsterEntity, ascension_level: u8, move_id: u8) -> MonsterTurnPlan {
    match move_id {
        CHOMP => chomp_plan(ascension_level),
        HARDEN => harden_plan(ascension_level),
        NIP => nip_plan(entity, ascension_level),
        COUNT => count_plan(),
        REINCARNATE => reincarnate_plan(entity),
        _ => MonsterTurnPlan::unknown(move_id),
    }
}

fn roll_move_custom_plan(
    rng: &mut crate::runtime::rng::StsRng,
    entity: &MonsterEntity,
    ascension_level: u8,
    num: i32,
    monsters: &[MonsterEntity],
) -> MonsterTurnPlan {
    if entity.half_dead {
        return reincarnate_plan(entity);
    }

    if entity.current_hp <= 0 {
        return count_plan();
    }

    if entity.darkling.first_move {
        return if num < 50 {
            harden_plan(ascension_level)
        } else {
            nip_plan(entity, ascension_level)
        };
    }

    let last_move = entity.move_history().back().copied().unwrap_or(0);
    let last_two_moves = |move_id| {
        entity.move_history().len() >= 2
            && entity.move_history()[entity.move_history().len() - 1] == move_id
            && entity.move_history()[entity.move_history().len() - 2] == move_id
    };

    if num < 40 {
        if last_move != CHOMP && is_even_position(entity, monsters) {
            chomp_plan(ascension_level)
        } else {
            let reroll = rng.random_range(40, 99);
            roll_move_custom_plan(rng, entity, ascension_level, reroll, monsters)
        }
    } else if num < 70 {
        if last_move != HARDEN {
            harden_plan(ascension_level)
        } else {
            nip_plan(entity, ascension_level)
        }
    } else if !last_two_moves(NIP) {
        nip_plan(entity, ascension_level)
    } else {
        let reroll = rng.random_range(0, 99);
        roll_move_custom_plan(rng, entity, ascension_level, reroll, monsters)
    }
}

impl MonsterBehavior for Darkling {
    fn roll_move_plan_with_context(
        rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
        ctx: MonsterRollContext<'_>,
    ) -> MonsterTurnPlan {
        roll_move_custom_plan(rng, entity, ascension_level, num, ctx.monsters)
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
            power_id: PowerId::Regrow,
            amount: -1,
        }]
    }

    fn turn_plan(state: &CombatState, entity: &MonsterEntity) -> MonsterTurnPlan {
        plan_for(entity, state.meta.ascension_level, entity.planned_move_id())
    }

    fn take_turn_plan(
        state: &mut CombatState,
        entity: &MonsterEntity,
        plan: &MonsterTurnPlan,
    ) -> Vec<Action> {
        let mut actions = match (plan.move_id, plan.steps.as_slice()) {
            (CHOMP | NIP, [MoveStep::Attack(attack)]) => {
                attack_actions(entity.id, PLAYER, &attack.attack)
            }
            (HARDEN, [MoveStep::GainBlock(block)]) => vec![gain_block_action(entity, block)],
            (HARDEN, [MoveStep::GainBlock(block), MoveStep::ApplyPower(power)]) => vec![
                gain_block_action(entity, block),
                apply_power_action(entity, power),
            ],
            (COUNT, []) => Vec::new(),
            (REINCARNATE, [MoveStep::Heal(heal), MoveStep::ApplyPower(power)]) => {
                let mut actions = vec![
                    Action::ReviveMonster { target: entity.id },
                    Action::Heal {
                        target: entity.id,
                        amount: heal.amount,
                    },
                    apply_power_action(entity, power),
                ];
                if let Some(target_idx) = state
                    .entities
                    .monsters
                    .iter()
                    .position(|m| m.id == entity.id)
                {
                    actions.extend(crate::content::relics::hooks::on_spawn_monster(
                        state, target_idx,
                    ));
                }
                actions
            }
            (move_id, steps) => panic!("darkling plan/steps mismatch: {} {:?}", move_id, steps),
        };

        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }

    fn on_death(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let darkling_ids: Vec<_> = state
            .entities
            .monsters
            .iter()
            .filter(|monster| {
                crate::content::monsters::EnemyId::from_id(monster.monster_type)
                    == Some(crate::content::monsters::EnemyId::Darkling)
            })
            .map(|monster| monster.id)
            .collect();

        let all_dead = state
            .entities
            .monsters
            .iter()
            .filter(|monster| darkling_ids.contains(&monster.id))
            .all(|monster| monster.id == entity.id || monster.half_dead);

        if all_dead {
            for id in darkling_ids {
                if let Some(monster) = state.entities.monsters.iter_mut().find(|m| m.id == id) {
                    monster.half_dead = false;
                    monster.is_dying = true;
                    monster.current_hp = 0;
                }
                crate::content::powers::store::remove_entity_powers(state, id);
            }
            return Vec::new();
        }

        if let Some(monster) = state
            .entities
            .monsters
            .iter_mut()
            .find(|m| m.id == entity.id)
        {
            monster.half_dead = true;
            monster.is_dying = false;
            monster.current_hp = 0;
            monster.set_planned_move_id(COUNT);
        }
        crate::content::powers::store::remove_entity_powers(state, entity.id);

        vec![set_next_move_action(entity, count_plan())]
    }
}

#[cfg(test)]
mod tests {
    use super::{current_nip_damage, HARDEN, NIP};
    use crate::content::monsters::EnemyId;
    use crate::runtime::combat::{
        ByrdRuntimeState, ChosenRuntimeState, DarklingRuntimeState, HexaghostRuntimeState,
        LagavulinRuntimeState, MonsterEntity, MonsterMoveState, ShelledParasiteRuntimeState,
        SneckoRuntimeState,
    };
    use crate::semantics::combat::{AttackSpec, DamageKind, MonsterMoveSpec};
    use std::collections::VecDeque;

    #[test]
    fn nip_damage_uses_current_attack_plan_before_preview_projection() {
        let entity = MonsterEntity {
            id: 1,
            monster_type: EnemyId::Darkling as usize,
            current_hp: 20,
            max_hp: 56,
            block: 0,
            slot: 0,
            is_dying: false,
            is_escaped: false,
            half_dead: false,
            move_state: MonsterMoveState {
                planned_move_id: NIP,
                history: VecDeque::from([HARDEN, NIP]),
                planned_steps: Some(
                    MonsterMoveSpec::Attack(AttackSpec {
                        base_damage: 13,
                        hits: 1,
                        damage_kind: DamageKind::Normal,
                    })
                    .to_steps(),
                ),
                planned_visible_spec: None,
            },
            logical_position: 0,
            hexaghost: HexaghostRuntimeState::default(),
            louse: Default::default(),
            jaw_worm: Default::default(),
            thief: Default::default(),
            byrd: ByrdRuntimeState::default(),
            chosen: ChosenRuntimeState::default(),
            snecko: SneckoRuntimeState::default(),
            shelled_parasite: ShelledParasiteRuntimeState::default(),
            bronze_automaton: Default::default(),
            bronze_orb: Default::default(),
            book_of_stabbing: Default::default(),
            collector: Default::default(),
            champ: Default::default(),
            awakened_one: Default::default(),
            corrupt_heart: Default::default(),
            darkling: DarklingRuntimeState {
                first_move: false,
                nip_dmg: 0,
            },
            lagavulin: LagavulinRuntimeState::default(),
            guardian: Default::default(),
        };

        assert_eq!(current_nip_damage(&entity, 2), 13);
    }
}
