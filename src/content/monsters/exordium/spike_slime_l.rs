use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent, MonsterEntity, PowerId};
use crate::content::monsters::MonsterBehavior;

pub struct SpikeSlimeL;

const SPIKE_SLIME_M_SPLIT_OFFSET_X: i32 = 134;

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
            amount: -1,
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
        let asc = state.meta.ascension_level;
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
                let base_draw_x = entity
                    .protocol_identity
                    .draw_x
                    .unwrap_or(entity.logical_position);
                actions.push(Action::Suicide { target: entity.id });
                actions.push(Action::SpawnMonsterSmart {
                    monster_id: crate::content::monsters::EnemyId::SpikeSlimeM,
                    logical_position: base_draw_x - SPIKE_SLIME_M_SPLIT_OFFSET_X,
                    current_hp: entity.current_hp,
                    max_hp: entity.current_hp,
                    protocol_draw_x: Some(base_draw_x - SPIKE_SLIME_M_SPLIT_OFFSET_X),
                    is_minion: false,
                });
                actions.push(Action::SpawnMonsterSmart {
                    monster_id: crate::content::monsters::EnemyId::SpikeSlimeM,
                    logical_position: base_draw_x + SPIKE_SLIME_M_SPLIT_OFFSET_X,
                    current_hp: entity.current_hp,
                    max_hp: entity.current_hp,
                    protocol_draw_x: Some(base_draw_x + SPIKE_SLIME_M_SPLIT_OFFSET_X),
                    is_minion: false,
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::content::monsters::EnemyId;
    use crate::content::test_support::{basic_combat, CombatTestExt};

    #[test]
    fn split_preserves_java_draw_x_offsets_for_medium_spike_slimes() {
        let combat = basic_combat()
            .with_monster_type(1, EnemyId::SpikeSlimeL)
            .with_monster_max_hp(1, 67)
            .with_monster_hp(1, 29);
        let mut entity = combat.entities.monsters[0].clone();
        entity.next_move_byte = 3;
        entity.logical_position = 400;
        entity.protocol_identity.draw_x = Some(-220);

        let actions = SpikeSlimeL::take_turn(&mut combat.clone(), &entity);

        assert_eq!(actions[0], Action::Suicide { target: 1 });
        assert_eq!(
            actions[1],
            Action::SpawnMonsterSmart {
                monster_id: EnemyId::SpikeSlimeM,
                logical_position: -220 - SPIKE_SLIME_M_SPLIT_OFFSET_X,
                current_hp: 29,
                max_hp: 29,
                protocol_draw_x: Some(-220 - SPIKE_SLIME_M_SPLIT_OFFSET_X),
                is_minion: false,
            }
        );
        assert_eq!(
            actions[2],
            Action::SpawnMonsterSmart {
                monster_id: EnemyId::SpikeSlimeM,
                logical_position: -220 + SPIKE_SLIME_M_SPLIT_OFFSET_X,
                current_hp: 29,
                max_hp: 29,
                protocol_draw_x: Some(-220 + SPIKE_SLIME_M_SPLIT_OFFSET_X),
                is_minion: false,
            }
        );
    }
}
