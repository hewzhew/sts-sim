use crate::content::monsters::MonsterBehavior;
use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent, MonsterEntity, PowerId};

// Helper to calculate divider damage: (player.currentHealth / 12) + 1
fn get_divider_damage(state: &CombatState) -> i32 {
    let player = &state.entities.player;
    (player.current_hp / 12) + 1
}

pub struct Hexaghost;

impl MonsterBehavior for Hexaghost {
    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        _num: i32,
    ) -> (u8, Intent) {
        if !entity.hexaghost.activated {
            return fallback_roll_move_from_history(entity, ascension_level);
        }

        let tackle_dmg = if ascension_level >= 4 { 6 } else { 5 };
        let inferno_dmg = if ascension_level >= 4 { 3 } else { 2 };

        match entity.hexaghost.orb_active_count {
            0 => (4, Intent::AttackDebuff { damage: 6, hits: 1 }),
            1 => (
                2,
                Intent::Attack {
                    damage: tackle_dmg,
                    hits: 2,
                },
            ),
            2 => (4, Intent::AttackDebuff { damage: 6, hits: 1 }),
            3 => (3, Intent::DefendBuff),
            4 => (
                2,
                Intent::Attack {
                    damage: tackle_dmg,
                    hits: 2,
                },
            ),
            5 => (4, Intent::AttackDebuff { damage: 6, hits: 1 }),
            6 => (
                6,
                Intent::AttackDebuff {
                    damage: inferno_dmg,
                    hits: 6,
                },
            ),
            _ => fallback_roll_move_from_history(entity, ascension_level),
        }
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let sear_dmg = 6;
        let tackle_dmg = if state.meta.ascension_level >= 4 {
            6
        } else {
            5
        };
        let inferno_dmg = if state.meta.ascension_level >= 4 {
            3
        } else {
            2
        };
        let str_amount = if state.meta.ascension_level >= 19 {
            3
        } else {
            2
        };
        let sear_burn_count = if state.meta.ascension_level >= 19 {
            2
        } else {
            1
        };
        let mut actions = Vec::new();

        match entity.next_move_byte {
            5 => {
                // ACTIVATE
                // ACTIVATE immediately overrides subsequent state move towards DIVIDER without directly inflicting damage
                // In Java, take_turn for 5 sets intent to 1 immediately!
                // Pushes Action::SetMonsterMove into queue
                let d = get_divider_damage(state);
                actions.push(Action::UpdateHexaghostState {
                    monster_id: entity.id,
                    activated: Some(true),
                    orb_active_count: Some(6),
                    burn_upgraded: None,
                });
                actions.push(Action::SetMonsterMove {
                    monster_id: entity.id,
                    next_move_byte: 1,
                    intent: Intent::Attack { damage: d, hits: 6 },
                });
                return actions;
            }
            1 => {
                // DIVIDER
                // Java locks Divider damage during ACTIVATE by mutating damage[2] and
                // setting the next move immediately. Execution should use that locked
                // intent damage, not recompute from the player's current HP.
                let d = match entity.current_intent {
                    Intent::Attack { damage, .. }
                    | Intent::AttackBuff { damage, .. }
                    | Intent::AttackDebuff { damage, .. }
                    | Intent::AttackDefend { damage, .. } => damage,
                    _ => entity.intent_preview_damage.max(0),
                };
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
                actions.push(Action::UpdateHexaghostState {
                    monster_id: entity.id,
                    activated: None,
                    orb_active_count: Some(0),
                    burn_upgraded: None,
                });
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
                actions.push(Action::UpdateHexaghostState {
                    monster_id: entity.id,
                    activated: None,
                    orb_active_count: Some(
                        entity.hexaghost.orb_active_count.saturating_add(1).min(6),
                    ),
                    burn_upgraded: None,
                });
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
                actions.push(Action::UpdateHexaghostState {
                    monster_id: entity.id,
                    activated: None,
                    orb_active_count: Some(
                        entity.hexaghost.orb_active_count.saturating_add(1).min(6),
                    ),
                    burn_upgraded: None,
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
                // we just insert a Burn. Engine later can handle upgraded versions via different Action struct
                let card_id = crate::content::cards::CardId::Burn;

                actions.push(Action::MakeTempCardInDiscard {
                    card_id,
                    amount: sear_burn_count,
                    upgraded: entity.hexaghost.burn_upgraded,
                });
                actions.push(Action::UpdateHexaghostState {
                    monster_id: entity.id,
                    activated: None,
                    orb_active_count: Some(
                        entity.hexaghost.orb_active_count.saturating_add(1).min(6),
                    ),
                    burn_upgraded: None,
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
                actions.push(Action::UpdateHexaghostState {
                    monster_id: entity.id,
                    activated: None,
                    orb_active_count: Some(0),
                    burn_upgraded: Some(true),
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

fn fallback_roll_move_from_history(entity: &MonsterEntity, ascension_level: u8) -> (u8, Intent) {
    if entity.move_history.is_empty() {
        return (5, Intent::Unknown);
    }

    let history: Vec<u8> = entity.move_history.iter().copied().collect();
    let last_move = *history.last().unwrap_or(&5);
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
        5 => (1, Intent::Attack { damage: 0, hits: 6 }),
        1 | 6 => (4, Intent::AttackDebuff { damage: 6, hits: 1 }),
        4 => {
            if prev_move == 1 || prev_move == 6 || prev_move == 0 {
                (
                    2,
                    Intent::Attack {
                        damage: tackle_dmg,
                        hits: 2,
                    },
                )
            } else if prev_move == 2 {
                if prev_prev_move == 4 {
                    (3, Intent::DefendBuff)
                } else if prev_prev_move == 3 {
                    (
                        6,
                        Intent::AttackDebuff {
                            damage: inferno_dmg,
                            hits: 6,
                        },
                    )
                } else {
                    (3, Intent::DefendBuff)
                }
            } else {
                (3, Intent::DefendBuff)
            }
        }
        2 => (4, Intent::AttackDebuff { damage: 6, hits: 1 }),
        3 => (
            2,
            Intent::Attack {
                damage: tackle_dmg,
                hits: 2,
            },
        ),
        _ => (4, Intent::AttackDebuff { damage: 6, hits: 1 }),
    }
}
