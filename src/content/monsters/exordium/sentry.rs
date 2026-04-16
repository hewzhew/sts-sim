use crate::content::monsters::MonsterBehavior;
use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent, MonsterEntity};

pub struct Sentry;

impl MonsterBehavior for Sentry {
    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        let beam_dmg = if ascension_level >= 3 { 10 } else { 9 };

        if entity.move_history.is_empty() {
            // First move depends on its slot
            if entity.slot % 2 == 0 {
                return (3, Intent::Debuff);
            } else {
                return (
                    4,
                    Intent::Attack {
                        damage: beam_dmg,
                        hits: 1,
                    },
                );
            }
        }

        let last_move = *entity.move_history.back().unwrap();
        if last_move == 4 {
            (3, Intent::Debuff)
        } else {
            (
                4,
                Intent::Attack {
                    damage: beam_dmg,
                    hits: 1,
                },
            )
        }
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let beam_dmg = if state.meta.ascension_level >= 3 {
            10
        } else {
            9
        };
        let mut actions = Vec::new();

        match entity.next_move_byte {
            3 => {
                // BOLT
                // Adds 2 Dazed to discard pile
                actions.push(Action::MakeTempCardInDiscard {
                    card_id: crate::content::cards::CardId::Dazed,
                    amount: if state.meta.ascension_level >= 18 {
                        3
                    } else {
                        2
                    },
                    upgraded: false,
                });
            }
            4 => {
                // BEAM
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: beam_dmg,
                    output: beam_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            _ => {}
        }

        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }

    fn use_pre_battle_action(
        entity: &MonsterEntity,
        _hp_rng: &mut crate::runtime::rng::StsRng,
        _ascension_level: u8,
    ) -> Vec<Action> {
        vec![Action::ApplyPower {
            target: entity.id,
            source: entity.id,
            power_id: crate::runtime::combat::PowerId::Artifact,
            amount: 1,
        }]
    }
}
