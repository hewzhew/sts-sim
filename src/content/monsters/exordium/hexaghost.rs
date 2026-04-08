use crate::action::{Action, DamageInfo, DamageType};
use crate::combat::{CombatState, Intent, MonsterEntity, PowerId};
use crate::content::monsters::MonsterBehavior;

// Helper to calculate divider damage: (player.currentHealth / 12) + 1
fn get_divider_damage(state: &CombatState) -> i32 {
    let player = &state.player;
    (player.current_hp / 12) + 1
}

pub struct Hexaghost;

impl MonsterBehavior for Hexaghost {
    fn roll_move(
        _rng: &mut crate::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        if entity.move_history.is_empty() {
            return (5, Intent::Unknown); // ACTIVATE
        }

        // Check our orbital tracker via HexaghostOrb power amount
        // roll_move accesses localized history to determine predictable sequencing
        // So we can't read the power amount here!
        // We MUST use move_history to determine the next move!
        // The sequence after 5 is:
        // 0: SEAR (4)
        // 1: TACKLE (2)
        // 2: SEAR (4)
        // 3: INFLAME (3)
        // 4: TACKLE (2)
        // 5: SEAR (4)
        // 6: INFERNO (6)

        let history: Vec<u8> = entity.move_history.iter().copied().collect();
        let last_move = *history.last().unwrap_or(&5);

        // Disambiguate the repeating moves (2 and 4) using the second-to-last move.
        // History combinations:
        // [5] -> next 1 (DIVIDER)
        // [1] -> next 4 (SEAR 0)
        // [1, 4] -> next 2 (TACKLE 1)
        // [4, 2] -> next 4 (SEAR 2)
        // Sequence 3 (INFLAME) follows specifically if previous move 2 was 4:
        // History: 1 -> 4 -> 2 -> 4 -> 3 -> 2 -> 4 -> 6 -> 1 -> 4 -> 2...

        let prev_move = if history.len() >= 2 {
            history[history.len() - 2]
        } else {
            0
        };
        let prev_prev_move = if history.len() >= 3 {
            history[history.len() - 3]
        } else {
            0
        };

        let tackle_dmg = if ascension_level >= 4 { 6 } else { 5 };
        let inferno_dmg = if ascension_level >= 4 { 3 } else { 2 };

        match last_move {
            5 => (1, Intent::Attack { damage: 0, hits: 6 }), // Base damage is finalized effectively during take_turn resolution.
            1 | 6 => (4, Intent::AttackDebuff { damage: 6, hits: 1 }), // SEAR 0
            4 => {
                if prev_move == 1 || prev_move == 6 || prev_move == 0 {
                    (
                        2,
                        Intent::Attack {
                            damage: tackle_dmg,
                            hits: 2,
                        },
                    ) // TACKLE 1
                } else if prev_move == 2 {
                    if prev_prev_move == 4 {
                        (3, Intent::DefendBuff) // INFLAME 3
                    } else if prev_prev_move == 3 {
                        (
                            6,
                            Intent::AttackDebuff {
                                damage: inferno_dmg,
                                hits: 6,
                            },
                        ) // INFERNO 6
                    } else {
                        (3, Intent::DefendBuff) // Fallback
                    }
                } else {
                    (3, Intent::DefendBuff) // Fallback
                }
            }
            2 => {
                (4, Intent::AttackDebuff { damage: 6, hits: 1 }) // SEAR 2 or 5
            }
            3 => {
                (
                    2,
                    Intent::Attack {
                        damage: tackle_dmg,
                        hits: 2,
                    },
                ) // TACKLE 4
            }
            _ => (4, Intent::AttackDebuff { damage: 6, hits: 1 }),
        }
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let sear_dmg = 6;
        let tackle_dmg = if state.ascension_level >= 4 { 6 } else { 5 };
        let inferno_dmg = if state.ascension_level >= 4 { 3 } else { 2 };
        let str_amount = if state.ascension_level >= 19 { 3 } else { 2 };
        let sear_burn_count = if state.ascension_level >= 19 { 2 } else { 1 };
        let mut actions = Vec::new();

        match entity.next_move_byte {
            5 => {
                // ACTIVATE
                // ACTIVATE immediately overrides subsequent state move towards DIVIDER without directly inflicting damage
                // In Java, take_turn for 5 sets intent to 1 immediately!
                // Pushes Action::SetMonsterMove into queue
                let d = get_divider_damage(state);
                actions.push(Action::SetMonsterMove {
                    monster_id: entity.id,
                    next_move_byte: 1,
                    intent: Intent::Attack { damage: d, hits: 6 },
                });
                return actions;
            }
            1 => {
                // DIVIDER
                let d = get_divider_damage(state);
                for _ in 0..6 {
                    actions.push(Action::Damage(DamageInfo {
                        source: entity.id,
                        target: 0, // Player
                        base: d,
                        output: d,
                        damage_type: DamageType::Normal,
                        is_modified: false,
                    }));
                }
            }
            2 => {
                // TACKLE
                for _ in 0..2 {
                    actions.push(Action::Damage(DamageInfo {
                        source: entity.id,
                        target: 0,
                        base: tackle_dmg,
                        output: tackle_dmg,
                        damage_type: DamageType::Normal,
                        is_modified: false,
                    }));
                }
            }
            3 => {
                // INFLAME
                actions.push(Action::GainBlock {
                    target: entity.id,
                    amount: 12,
                });
                actions.push(Action::ApplyPower {
                    target: entity.id,
                    source: entity.id,
                    power_id: PowerId::Strength,
                    amount: str_amount,
                });
            }
            4 => {
                // SEAR
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0,
                    base: sear_dmg,
                    output: sear_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));

                // Add Burn to discard pile. Upgraded if INFERNO was played.
                let mut _burn_upgraded = false;
                for act in &entity.move_history {
                    if *act == 6 {
                        _burn_upgraded = true;
                        break;
                    }
                }

                // we just insert a Burn. Engine later can handle upgraded versions via different Action struct
                let card_id = crate::content::cards::CardId::Burn;

                actions.push(Action::MakeTempCardInDiscard {
                    card_id,
                    amount: sear_burn_count,
                    upgraded: _burn_upgraded,
                });
            }
            6 => {
                // INFERNO
                for _ in 0..6 {
                    actions.push(Action::Damage(DamageInfo {
                        source: entity.id,
                        target: 0,
                        base: inferno_dmg,
                        output: inferno_dmg,
                        damage_type: DamageType::Normal,
                        is_modified: false,
                    }));
                }
                actions.push(Action::UpgradeAllBurns);
            }
            _ => {}
        }

        actions.push(Action::RollMonsterMove {
            monster_id: entity.id,
        });
        actions
    }
}
