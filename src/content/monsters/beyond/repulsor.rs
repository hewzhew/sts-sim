use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent};
use crate::content::cards::CardId;
use crate::content::monsters::MonsterBehavior;

pub struct Repulsor;

impl MonsterBehavior for Repulsor {
    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &crate::runtime::combat::MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        let attack_dmg = if ascension_level >= 2 { 13 } else { 11 };

        let last_move = entity.move_history.back().copied().unwrap_or(0);

        if entity.move_history.len() < 20 && last_move != 2 {
            (
                2,
                Intent::Attack {
                    damage: attack_dmg,
                    hits: 1,
                },
            )
        } else {
            (1, Intent::Debuff)
        }
    }

    fn take_turn(
        state: &mut CombatState,
        entity: &crate::runtime::combat::MonsterEntity,
    ) -> Vec<Action> {
        let mut actions = Vec::new();
        let asc = state.meta.ascension_level;

        match entity.next_move_byte {
            2 => {
                let dmg = if asc >= 2 { 13 } else { 11 };
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: dmg,
                    output: dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            1 => {
                actions.push(Action::MakeTempCardInDrawPile {
                    card_id: CardId::Dazed,
                    amount: 2,
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
