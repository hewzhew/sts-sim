use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent};
use crate::content::monsters::{EnemyId, MonsterBehavior};
use crate::content::powers::PowerId;

pub struct TheCollector;

impl TheCollector {
    // Java summons the nearer Torch Head first, then the farther-left one.
    // The resulting protocol draw_x values are [770, 647], and smart
    // positioning reorders the final group to [647, 770, Collector].
    const TORCH_DRAW_X: [i32; 2] = [770, 647];

    pub fn roll_move_custom(
        rng: &mut crate::runtime::rng::StsRng,
        entity: &crate::runtime::combat::MonsterEntity,
        ascension_level: u8,
        _num: i32,
        monsters: &[crate::runtime::combat::MonsterEntity],
    ) -> (u8, Intent) {
        let turn = entity.move_history.len();

        // Initial spawn happens on turn 0
        if turn == 0 {
            return (1, Intent::Unknown);
        }

        let ult_used = entity.move_history.iter().any(|&m| m == 4);
        if turn >= 3 && !ult_used {
            return (4, Intent::StrongDebuff);
        }

        let minion_dead = monsters
            .iter()
            .filter(|m| {
                crate::content::monsters::EnemyId::from_id(m.monster_type)
                    == Some(EnemyId::TorchHead)
                    && !m.is_dying
            })
            .count()
            < 2;
        let num = rng.random_range(0, 99);

        let last_move = |byte| entity.move_history.back() == Some(&byte);
        let last_two_moves = |byte| {
            entity.move_history.len() >= 2
                && entity.move_history[entity.move_history.len() - 1] == byte
                && entity.move_history[entity.move_history.len() - 2] == byte
        };

        if num <= 25 && minion_dead && !last_move(5) {
            return (5, Intent::Unknown); // Revive
        }

        let dmg = if ascension_level >= 4 { 21 } else { 18 };
        if num <= 70 && !last_two_moves(2) {
            return (
                2,
                Intent::Attack {
                    damage: dmg,
                    hits: 1,
                },
            );
        }

        if !last_move(3) {
            (3, Intent::DefendBuff)
        } else {
            (
                2,
                Intent::Attack {
                    damage: dmg,
                    hits: 1,
                },
            )
        }
    }
}

impl MonsterBehavior for TheCollector {
    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        _entity: &crate::runtime::combat::MonsterEntity,
        _ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        (2, Intent::Unknown)
    }

    fn take_turn(
        state: &mut CombatState,
        entity: &crate::runtime::combat::MonsterEntity,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        let dmg = if state.meta.ascension_level >= 4 {
            21
        } else {
            18
        };
        let block_amt = if state.meta.ascension_level >= 9 {
            18
        } else {
            15
        };
        let str_amt = if state.meta.ascension_level >= 19 {
            5
        } else if state.meta.ascension_level >= 4 {
            4
        } else {
            3
        };
        let mega_debuff_amt = if state.meta.ascension_level >= 19 {
            5
        } else {
            3
        };

        match entity.next_move_byte {
            1 => {
                // Java protocol identity for Collector minions is keyed by the final
                // draw_x ordering, not by our local relative x values.
                actions.push(Action::SpawnMonsterSmart {
                    monster_id: EnemyId::TorchHead,
                    logical_position: Self::TORCH_DRAW_X[0],
                    current_hp: 0,
                    max_hp: 0,
                    protocol_draw_x: Some(Self::TORCH_DRAW_X[0]),
                    is_minion: true,
                });
                actions.push(Action::SpawnMonsterSmart {
                    monster_id: EnemyId::TorchHead,
                    logical_position: Self::TORCH_DRAW_X[1],
                    current_hp: 0,
                    max_hp: 0,
                    protocol_draw_x: Some(Self::TORCH_DRAW_X[1]),
                    is_minion: true,
                });
            }
            2 => {
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: dmg,
                    output: dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            3 => {
                let actual_block = if state.meta.ascension_level >= 19 {
                    block_amt + 5
                } else {
                    block_amt
                };
                actions.push(Action::GainBlock {
                    target: entity.id,
                    amount: actual_block,
                });

                for m in state.entities.monsters.iter().filter(|m| !m.is_dying) {
                    actions.push(Action::ApplyPower {
                        source: entity.id,
                        target: m.id,
                        power_id: PowerId::Strength,
                        amount: str_amt,
                    });
                }
            }
            4 => {
                // Mega Debuff
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0,
                    power_id: PowerId::Weak,
                    amount: mega_debuff_amt,
                });
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0,
                    power_id: PowerId::Vulnerable,
                    amount: mega_debuff_amt,
                });
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0,
                    power_id: PowerId::Frail,
                    amount: mega_debuff_amt,
                });
            }
            5 => {
                // Revive — Java: for each slot with isDying TorchHead, spawn a new one
                // Iterate monster list finding dead TorchHeads and respawn them
                for m in &state.entities.monsters {
                    if !m.is_dying
                        && crate::content::monsters::EnemyId::from_id(m.monster_type)
                            == Some(EnemyId::TorchHead)
                    {
                        actions.push(Action::GainBlock {
                            target: m.id,
                            amount: 15,
                        });
                    }
                }
                let dead_torches: Vec<i32> = state
                    .entities
                    .monsters
                    .iter()
                    .filter(|m| {
                        m.is_dying
                            && crate::content::monsters::EnemyId::from_id(m.monster_type)
                                == Some(EnemyId::TorchHead)
                    })
                    .map(|m| m.protocol_identity.draw_x.unwrap_or(m.logical_position))
                    .collect();

                // Java revives via enemySlots entry iteration, which preserves the
                // original summon slot order [770, 647] rather than current group order.
                let mut dead_torches = dead_torches;
                dead_torches.sort_by(|a, b| b.cmp(a));

                for draw_x in dead_torches {
                    actions.push(Action::SpawnMonsterSmart {
                        monster_id: EnemyId::TorchHead,
                        logical_position: draw_x,
                        current_hp: 0, // Engine will roll HP via get_hp_range
                        max_hp: 0,
                        protocol_draw_x: Some(draw_x),
                        is_minion: true,
                    });
                }
            }
            _ => {}
        }

        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
