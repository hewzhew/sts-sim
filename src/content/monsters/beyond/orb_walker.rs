use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent};
use crate::content::cards::CardId;
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct OrbWalker;

impl MonsterBehavior for OrbWalker {
    fn roll_move(
        _rng: &mut crate::rng::StsRng,
        entity: &crate::combat::MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> (u8, Intent) {
        let claw_dmg = if ascension_level >= 2 { 16 } else { 15 };
        let laser_dmg = if ascension_level >= 2 { 11 } else { 10 };

        let last_two_moves = |byte| {
            entity.move_history.len() >= 2
                && entity.move_history[entity.move_history.len() - 1] == byte
                && entity.move_history[entity.move_history.len() - 2] == byte
        };
        if num < 40 {
            if !last_two_moves(2) {
                (
                    2,
                    Intent::Attack {
                        damage: claw_dmg,
                        hits: 1,
                    },
                )
            } else {
                (
                    1,
                    Intent::AttackDebuff {
                        damage: laser_dmg,
                        hits: 1,
                    },
                )
            }
        } else if !last_two_moves(1) {
            (
                1,
                Intent::AttackDebuff {
                    damage: laser_dmg,
                    hits: 1,
                },
            )
        } else {
            (
                2,
                Intent::Attack {
                    damage: claw_dmg,
                    hits: 1,
                },
            )
        }
    }

    fn use_pre_battle_action(
        entity: &crate::combat::MonsterEntity,
        _hp_rng: &mut crate::rng::StsRng,
        ascension_level: u8,
    ) -> Vec<Action> {
        let str_amount = if ascension_level >= 17 { 5 } else { 3 };

        vec![Action::ApplyPower {
            source: entity.id,
            target: entity.id,
            power_id: PowerId::GenericStrengthUp,
            amount: str_amount,
        }]
    }

    fn take_turn(state: &mut CombatState, entity: &crate::combat::MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        let asc = state.meta.ascension_level;

        let claw_dmg = if asc >= 2 { 16 } else { 15 };
        let laser_dmg = if asc >= 2 { 11 } else { 10 };

        match entity.next_move_byte {
            2 => {
                // CLAW
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: claw_dmg,
                    output: claw_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            1 => {
                // LASER
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: laser_dmg,
                    output: laser_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                actions.push(Action::MakeTempCardInDiscard {
                    card_id: CardId::Burn,
                    amount: 1,
                    upgraded: false,
                });
                actions.push(Action::MakeTempCardInDrawPile {
                    card_id: CardId::Burn,
                    amount: 1,
                    random_spot: true,
                    upgraded: false,
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
