use crate::combat::{CombatState, MonsterEntity, Intent, PowerId};
use crate::action::{Action, DamageInfo, DamageType};
use crate::content::monsters::MonsterBehavior;

pub struct SlaverBlue;

impl MonsterBehavior for SlaverBlue {
    fn roll_move(_rng: &mut crate::rng::StsRng, entity: &MonsterEntity, ascension_level: u8, num: i32) -> (u8, Intent) {
        let stab_dmg = if ascension_level >= 2 { 13 } else { 12 };
        let rake_dmg = if ascension_level >= 2 { 8 } else { 7 };

        // 1: STAB, 4: RAKE (Attack + Debuff)
        let last_move = entity.move_history.back().copied();
        let last_move_before = if entity.move_history.len() >= 2 {
            entity.move_history.get(entity.move_history.len() - 2).copied()
        } else {
            None
        };
        let last_two_moves_were = |byte: u8| -> bool {
            last_move == Some(byte) && last_move_before == Some(byte)
        };

        if num >= 40 && !last_two_moves_were(1) {
            (1, Intent::Attack { damage: stab_dmg, hits: 1 })
        } else if !last_two_moves_were(4) {
            (4, Intent::AttackDebuff { damage: rake_dmg, hits: 1 })
        } else {
            (1, Intent::Attack { damage: stab_dmg, hits: 1 })
        }
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let asc = state.ascension_level;
        let stab_dmg = if asc >= 2 { 13 } else { 12 };
        let rake_dmg = if asc >= 2 { 8 } else { 7 };
        let weak_amt = if asc >= 17 { 2 } else { 1 };
        let mut actions = Vec::new();

        match entity.next_move_byte {
            1 => { // STAB
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: stab_dmg,
                    output: stab_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            4 => { // RAKE
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: rake_dmg,
                    output: rake_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                actions.push(Action::ApplyPower {
                    target: 0, // Player
                    source: entity.id,
                    power_id: PowerId::Weak,
                    amount: weak_amt,
                });
            }
            _ => { }
        }

        actions.push(Action::RollMonsterMove { monster_id: entity.id });
        actions
    }
}
