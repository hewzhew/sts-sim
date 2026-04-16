use crate::content::monsters::MonsterBehavior;
use crate::content::powers::PowerId;
use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent, MonsterEntity};

pub struct Cultist;

impl MonsterBehavior for Cultist {
    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        _ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        let is_first_move = entity.move_history.is_empty();

        let attack_dmg = 6;

        if is_first_move {
            return (3, Intent::Buff); // 3 = INCANTATION
        }

        (
            1,
            Intent::Attack {
                damage: attack_dmg,
                hits: 1,
            },
        ) // 1 = DARK_STRIKE
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let asc = state.meta.ascension_level;
        let attack_dmg = 6;
        let ritual_amount = if asc >= 17 {
            5
        } else if asc >= 2 {
            4
        } else {
            3
        };

        let mut actions = Vec::new();

        match entity.next_move_byte {
            3 => {
                // INCANTATION
                // In a full implementation, we could have TalkAction too.
                actions.push(Action::ApplyPowerDetailed {
                    source: entity.id,
                    target: entity.id,
                    power_id: PowerId::Ritual,
                    amount: ritual_amount,
                    instance_id: None,
                    extra_data: Some(crate::content::powers::core::ritual::extra_data(
                        false, true,
                    )),
                });
            }
            1 => {
                // DARK_STRIKE
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0, // Player
                    base: attack_dmg,
                    output: attack_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            _ => {
                // Fallback (should not happen)
            }
        }

        // Always end a turn by rolling the NEXT move to update intent graphic!
        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });

        actions
    }
}
