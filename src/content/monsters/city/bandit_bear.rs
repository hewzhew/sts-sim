use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent};
use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;

pub struct BanditBear;

impl MonsterBehavior for BanditBear {
    fn roll_move(
        _rng: &mut crate::rng::StsRng,
        entity: &crate::combat::MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        if entity.move_history.is_empty() {
            return (2, Intent::StrongDebuff); // BEAR_HUG
        }

        let last_move = entity.move_history.back().copied().unwrap_or(0);
        if last_move == 2 || last_move == 1 {
            let dmg = if ascension_level >= 2 { 10 } else { 9 };
            (
                3,
                Intent::AttackDefend {
                    damage: dmg,
                    hits: 1,
                },
            ) // LUNGE
        } else {
            let dmg = if ascension_level >= 2 { 20 } else { 18 };
            (
                1,
                Intent::Attack {
                    damage: dmg,
                    hits: 1,
                },
            ) // MAUL
        }
    }

    fn take_turn(state: &mut CombatState, entity: &crate::combat::MonsterEntity) -> Vec<Action> {
        let mut actions = Vec::new();
        let asc = state.meta.ascension_level;

        match entity.next_move_byte {
            2 => {
                // BEAR_HUG
                let con_reduction = if asc >= 17 { 4 } else { 2 };
                actions.push(Action::ApplyPower {
                    source: entity.id,
                    target: 0,
                    power_id: PowerId::Dexterity,
                    amount: -con_reduction,
                });
            }
            1 => {
                // MAUL
                let dmg = if asc >= 2 { 20 } else { 18 };
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
                // LUNGE
                let dmg = if asc >= 2 { 10 } else { 9 };
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: dmg,
                    output: dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                actions.push(Action::GainBlock {
                    target: entity.id,
                    amount: 9,
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
