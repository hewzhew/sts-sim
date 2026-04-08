use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent, MonsterEntity, PowerId};
use crate::content::monsters::MonsterBehavior;

pub struct SpikeSlimeL;

impl MonsterBehavior for SpikeSlimeL {
    fn use_pre_battle_action(
        entity: &MonsterEntity,
        _hp_rng: &mut crate::rng::StsRng,
        _ascension_level: u8,
    ) -> Vec<Action> {
        vec![Action::ApplyPower {
            target: entity.id,
            source: entity.id,
            power_id: PowerId::Split,
            amount: 1,
        }]
    }

    fn roll_move(
        _rng: &mut crate::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> (u8, Intent) {
        let tackle_dmg = if ascension_level >= 2 { 18 } else { 16 };

        // 1: FLAME_TACKLE (Attack + Debuff), 3: SPLIT, 4: FRAIL_LICK
        let last_move = entity.move_history.back().copied();
        let last_move_before = if entity.move_history.len() >= 2 {
            entity
                .move_history
                .get(entity.move_history.len() - 2)
                .copied()
        } else {
            None
        };
        let last_two_moves_were =
            |byte: u8| -> bool { last_move == Some(byte) && last_move_before == Some(byte) };

        if ascension_level >= 17 {
            if num < 30 {
                if last_two_moves_were(1) {
                    (4, Intent::Debuff)
                } else {
                    (
                        1,
                        Intent::AttackDebuff {
                            damage: tackle_dmg,
                            hits: 1,
                        },
                    )
                }
            } else if last_move == Some(4) {
                (
                    1,
                    Intent::AttackDebuff {
                        damage: tackle_dmg,
                        hits: 1,
                    },
                )
            } else {
                (4, Intent::Debuff)
            }
        } else if num < 30 {
            if last_two_moves_were(1) {
                (4, Intent::Debuff)
            } else {
                (
                    1,
                    Intent::AttackDebuff {
                        damage: tackle_dmg,
                        hits: 1,
                    },
                )
            }
        } else if last_two_moves_were(4) {
            (
                1,
                Intent::AttackDebuff {
                    damage: tackle_dmg,
                    hits: 1,
                },
            )
        } else {
            (4, Intent::Debuff)
        }
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let asc = state.ascension_level;
        let flame_tackle_dmg = if asc >= 2 { 18 } else { 16 };
        let slimed_amt = if asc >= 17 { 2 } else { 2 }; // Fixed 2 Slimed applied
        let frail_amt = if asc >= 17 { 3 } else { 2 };
        let mut actions = Vec::new();

        match entity.next_move_byte {
            1 => {
                // FLAME_TACKLE
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: flame_tackle_dmg,
                    output: flame_tackle_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                actions.push(Action::MakeTempCardInDiscard {
                    card_id: crate::content::cards::CardId::Slimed,
                    amount: slimed_amt,
                    upgraded: false,
                });
            }
            3 => {
                // SPLIT
                actions.push(Action::Suicide { target: entity.id });
                // Java uses smart positioning: first M at drawX-134 (position 0, before dead L)
                actions.push(Action::SpawnMonsterSmart {
                    monster_id: crate::content::monsters::EnemyId::SpikeSlimeM,
                    logical_position: entity.logical_position - 1,
                    current_hp: entity.current_hp,
                    max_hp: entity.current_hp,
                });
                // Second M at drawX+134 (position 2, after dead L)
                actions.push(Action::SpawnMonsterSmart {
                    monster_id: crate::content::monsters::EnemyId::SpikeSlimeM,
                    logical_position: entity.logical_position + 1,
                    current_hp: entity.current_hp,
                    max_hp: entity.current_hp,
                });
            }
            4 => {
                // FRAIL_LICK
                actions.push(Action::ApplyPower {
                    target: 0, // Player
                    source: entity.id,
                    power_id: PowerId::Frail,
                    amount: frail_amt,
                });
            }
            _ => {}
        }

        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
