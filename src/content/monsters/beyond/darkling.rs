use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent, MonsterEntity};

pub struct Darkling;

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
    } else if entity.intent_preview_damage > 0 {
        entity.intent_preview_damage
    } else if ascension_level >= 2 {
        11
    } else {
        9
    }
}

pub fn roll_move_custom(
    rng: &mut crate::runtime::rng::StsRng,
    entity: &crate::runtime::combat::MonsterEntity,
    ascension_level: u8,
    num: i32,
    monsters: &[MonsterEntity],
) -> (u8, Intent) {
    let chomp_dmg = if ascension_level >= 2 { 9 } else { 8 };
    let nip_dmg = current_nip_damage(entity, ascension_level);

    if entity.half_dead {
        return (5, Intent::Buff);
    }

    if entity.current_hp <= 0 {
        return (4, Intent::Unknown);
    }

    if entity.darkling.first_move {
        if num < 50 {
            return (
                2,
                if ascension_level >= 17 {
                    Intent::DefendBuff
                } else {
                    Intent::Defend
                },
            );
        }
        return (
            3,
            Intent::Attack {
                damage: nip_dmg,
                hits: 1,
            },
        );
    }

    let last_move = entity.move_history.back().copied().unwrap_or(0);
    let last_two_moves = |byte| {
        entity.move_history.len() >= 2
            && entity.move_history[entity.move_history.len() - 1] == byte
            && entity.move_history[entity.move_history.len() - 2] == byte
    };

    if num < 40 {
        if last_move != 1 && is_even_position(entity, monsters) {
            (
                1,
                Intent::Attack {
                    damage: chomp_dmg,
                    hits: 2,
                },
            )
        } else {
            let reroll = rng.random_range(40, 99);
            roll_move_custom(rng, entity, ascension_level, reroll, monsters)
        }
    } else if num < 70 {
        if last_move != 2 {
            (
                2,
                if ascension_level >= 17 {
                    Intent::DefendBuff
                } else {
                    Intent::Defend
                },
            )
        } else {
            (
                3,
                Intent::Attack {
                    damage: nip_dmg,
                    hits: 1,
                },
            )
        }
    } else if !last_two_moves(3) {
        (
            3,
            Intent::Attack {
                damage: nip_dmg,
                hits: 1,
            },
        )
    } else {
        let reroll = rng.random_range(0, 99);
        roll_move_custom(rng, entity, ascension_level, reroll, monsters)
    }
}

impl MonsterBehavior for Darkling {
    fn roll_move(
        rng: &mut crate::runtime::rng::StsRng,
        entity: &crate::runtime::combat::MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> (u8, Intent) {
        roll_move_custom(
            rng,
            entity,
            ascension_level,
            num,
            std::slice::from_ref(entity),
        )
    }

    fn use_pre_battle_action(
        entity: &crate::runtime::combat::MonsterEntity,
        _hp_rng: &mut crate::runtime::rng::StsRng,
        _ascension_level: u8,
    ) -> Vec<Action> {
        vec![Action::ApplyPower {
            source: entity.id,
            target: entity.id,
            power_id: PowerId::Regrow,
            amount: -1,
        }]
    }

    fn take_turn(
        state: &mut CombatState,
        entity: &crate::runtime::combat::MonsterEntity,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        let asc = state.meta.ascension_level;

        match entity.next_move_byte {
            1 => {
                // CHOMP
                let dmg = if asc >= 2 { 9 } else { 8 };
                for _ in 0..2 {
                    actions.push(Action::Damage(DamageInfo {
                        source: entity.id,
                        target: 0,
                        base: dmg,
                        output: dmg,
                        damage_type: DamageType::Normal,
                        is_modified: false,
                    }));
                }
            }
            2 => {
                // HARDEN
                actions.push(Action::GainBlock {
                    target: entity.id,
                    amount: 12,
                });
                if asc >= 17 {
                    actions.push(Action::ApplyPower {
                        source: entity.id,
                        target: entity.id,
                        power_id: PowerId::Strength,
                        amount: 2,
                    });
                }
            }
            3 => {
                // NIP
                let dmg = current_nip_damage(entity, asc);
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: dmg,
                    output: dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            4 => {
                // COUNT / regrow countdown line
            }
            5 => {
                if let Some(monster) = state
                    .entities
                    .monsters
                    .iter_mut()
                    .find(|m| m.id == entity.id)
                {
                    monster.half_dead = false;
                }
                actions.push(Action::Heal {
                    target: entity.id,
                    amount: entity.max_hp / 2,
                });
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: entity.id,
                    power_id: PowerId::Regrow,
                    amount: -1,
                });
                if let Some(target_idx) = state
                    .entities
                    .monsters
                    .iter()
                    .position(|m| m.id == entity.id)
                {
                    actions.extend(
                        crate::content::relics::hooks::on_spawn_monster(state, target_idx)
                            .into_iter(),
                    );
                }
            }
            _ => {}
        }

        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }

    fn on_death(
        state: &mut CombatState,
        entity: &crate::runtime::combat::MonsterEntity,
    ) -> Vec<Action> {
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
                    monster.current_intent = Intent::Unknown;
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
            monster.next_move_byte = 4;
            monster.current_intent = Intent::Unknown;
        }
        crate::content::powers::store::remove_entity_powers(state, entity.id);

        vec![Action::SetMonsterMove {
            monster_id: entity.id,
            next_move_byte: 4,
            intent: Intent::Unknown,
        }]
    }
}
