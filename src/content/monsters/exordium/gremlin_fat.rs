use crate::combat::{CombatState, MonsterEntity, Intent, PowerId};
use crate::action::{Action, DamageInfo, DamageType};
use crate::content::monsters::MonsterBehavior;

pub struct GremlinFat;

impl MonsterBehavior for GremlinFat {
    fn roll_move(_rng: &mut crate::rng::StsRng, _entity: &MonsterEntity, ascension_level: u8, _num: i32) -> (u8, Intent) {
        let dmg = if ascension_level >= 2 { 5 } else { 4 };
        (2, Intent::AttackDebuff { damage: dmg, hits: 1 })
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let dmg = if state.ascension_level >= 2 { 5 } else { 4 };
        let mut actions = Vec::new();

        match entity.next_move_byte {
            2 => { // BLUNT
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: dmg,
                    output: dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
                actions.push(Action::ApplyPower {
                    target: 0, 
                    source: entity.id,
                    power_id: PowerId::Weak,
                    amount: 1,
                });
                if state.ascension_level >= 17 {
                    actions.push(Action::ApplyPower {
                        target: 0,
                        source: entity.id,
                        power_id: PowerId::Frail,
                        amount: 1,
                    });
                }
            }
            99 => { // ESCAPE
                actions.push(Action::Escape { target: entity.id });
                // Need to resend escape just in case (like STS does)? No, Escape is enough
            }
            _ => { }
        }

        // It always tries to roll move again unless it's escaped
        if entity.next_move_byte != 99 {
            actions.push(Action::RollMonsterMove { monster_id: entity.id });
        }
        actions
    }
}
