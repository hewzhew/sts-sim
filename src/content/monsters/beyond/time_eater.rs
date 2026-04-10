use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct TimeEater;

impl MonsterBehavior for TimeEater {
    fn roll_move(
        _rng: &mut crate::rng::StsRng,
        entity: &crate::combat::MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> (u8, Intent) {
        // HASTE Trigger Logic (Half HP)
        let is_half_hp = entity.current_hp < entity.max_hp / 2;
        let used_haste = entity.move_history.contains(&5); // Haste is move byte 5

        if is_half_hp && !used_haste {
            return (5, Intent::Buff);
        }

        let reverb_dmg = if ascension_level >= 4 { 8 } else { 7 };
        let head_slam_dmg = if ascension_level >= 4 { 32 } else { 26 };
        let last_move = entity.move_history.back().copied().unwrap_or(0);
        let last_two_moves = |byte| {
            entity.move_history.len() >= 2
                && entity.move_history[entity.move_history.len() - 1] == byte
                && entity.move_history[entity.move_history.len() - 2] == byte
        };

        if num < 45 {
            if !last_two_moves(2) {
                return (
                    2,
                    Intent::Attack {
                        damage: reverb_dmg,
                        hits: 3,
                    },
                );
            } else {
                return (
                    4,
                    Intent::AttackDebuff {
                        damage: head_slam_dmg,
                        hits: 1,
                    },
                ); // Fallback assuming AI rng cascade
            }
        } else if num < 80 {
            if last_move != 4 {
                return (
                    4,
                    Intent::AttackDebuff {
                        damage: head_slam_dmg,
                        hits: 1,
                    },
                );
            } else if _rng.random_range(0, 99) < 66 {
                return (
                    2,
                    Intent::Attack {
                        damage: reverb_dmg,
                        hits: 3,
                    },
                );
            } else {
                return (3, Intent::DefendDebuff);
            }
        } else if last_move != 3 {
            return (3, Intent::DefendDebuff);
        } else {
            return (
                4,
                Intent::AttackDebuff {
                    damage: head_slam_dmg,
                    hits: 1,
                },
            ); // Simplified cascade
        }
    }

    fn use_pre_battle_action(
        entity: &crate::combat::MonsterEntity,
        _hp_rng: &mut crate::rng::StsRng,
        _ascension_level: u8,
    ) -> Vec<Action> {
        vec![Action::ApplyPower {
            source: entity.id,
            target: entity.id,
            power_id: PowerId::TimeWarp, // Needs registration
            amount: 0,
        }]
    }

    fn take_turn(state: &mut CombatState, entity: &crate::combat::MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        let asc = state.meta.ascension_level;

        let reverb_dmg = if asc >= 4 { 8 } else { 7 };
        let head_slam_dmg = if asc >= 4 { 32 } else { 26 };

        match entity.next_move_byte {
            2 => {
                // REVERBERATE
                for _ in 0..3 {
                    actions.push(Action::Damage(DamageInfo {
                        source: entity.id,
                        target: 0,
                        base: reverb_dmg,
                        output: reverb_dmg,
                        damage_type: DamageType::Normal,
                        is_modified: false,
                    }));
                }
            }
            3 => {
                // RIPPLE
                actions.push(Action::GainBlock {
                    target: entity.id,
                    amount: 20,
                });
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0,
                    power_id: PowerId::Vulnerable,
                    amount: 1,
                });
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0,
                    power_id: PowerId::Weak,
                    amount: 1,
                });
                if asc >= 19 {
                    actions.push(Action::ApplyPower {
                        source: entity.id,
                        target: 0,
                        power_id: PowerId::Frail,
                        amount: 1,
                    });
                }
            }
            4 => {
                // HEAD SLAM
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: head_slam_dmg,
                    output: head_slam_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0,
                    power_id: PowerId::DrawReduction,
                    amount: 1,
                });
                if asc >= 19 {
                    actions.push(Action::MakeTempCardInDiscard {
                        card_id: crate::content::cards::CardId::Slimed,
                        amount: 2,
                        upgraded: false,
                    });
                }
            }
            5 => {
                // HASTE
                actions.push(Action::RemoveAllDebuffs { target: entity.id });
                let heal_amt = (entity.max_hp / 2) - entity.current_hp;
                if heal_amt > 0 {
                    actions.push(Action::Heal {
                        target: entity.id,
                        amount: heal_amt,
                    });
                }
                if asc >= 19 {
                    actions.push(Action::GainBlock {
                        target: entity.id,
                        amount: head_slam_dmg,
                    }); // head_slam_dmg used as block in Java
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
