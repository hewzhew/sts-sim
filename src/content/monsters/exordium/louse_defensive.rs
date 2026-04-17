use crate::content::monsters::MonsterBehavior;
use crate::runtime::action::{Action, DamageInfo, DamageType};
use crate::runtime::combat::{CombatState, Intent, MonsterEntity, PowerId};

// LouseDefensive
pub struct LouseDefensive;

fn bite_base_damage(entity: &MonsterEntity) -> i32 {
    match entity.current_intent {
        Intent::Attack { damage, .. }
        | Intent::AttackBuff { damage, .. }
        | Intent::AttackDebuff { damage, .. }
        | Intent::AttackDefend { damage, .. } => damage,
        _ => entity.intent_preview_damage,
    }
}

impl MonsterBehavior for LouseDefensive {
    fn roll_move(
        _rng: &mut crate::runtime::rng::StsRng,
        entity: &MonsterEntity,
        ascension_level: u8,
        num: i32,
    ) -> (u8, Intent) {
        let bite_dmg = bite_base_damage(entity);

        // 3 = BITE, 4 = WEAKEN
        let last_move = entity.move_history.back().copied();
        let last_move_before = if entity.move_history.len() >= 2 {
            entity
                .move_history
                .get(entity.move_history.len() - 2)
                .copied()
        } else {
            None
        };
        let last_two_moves_were =
            |byte: u8| -> bool { last_move == Some(byte) && last_move_before == Some(byte) };

        // Java: Asc 17+ uses lastMove(4) (single check), below Asc 17 uses lastTwoMoves(4)
        if ascension_level >= 17 {
            if num < 25 {
                if last_move == Some(4) {
                    (
                        3,
                        Intent::Attack {
                            damage: bite_dmg,
                            hits: 1,
                        },
                    )
                } else {
                    (4, Intent::Debuff)
                }
            } else if last_two_moves_were(3) {
                (4, Intent::Debuff)
            } else {
                (
                    3,
                    Intent::Attack {
                        damage: bite_dmg,
                        hits: 1,
                    },
                )
            }
        } else {
            if num < 25 {
                if last_two_moves_were(4) {
                    (
                        3,
                        Intent::Attack {
                            damage: bite_dmg,
                            hits: 1,
                        },
                    )
                } else {
                    (4, Intent::Debuff)
                }
            } else if last_two_moves_were(3) {
                (4, Intent::Debuff)
            } else {
                (
                    3,
                    Intent::Attack {
                        damage: bite_dmg,
                        hits: 1,
                    },
                )
            }
        }
    }

    fn take_turn(state: &mut CombatState, entity: &MonsterEntity) -> Vec<Action> {
        let _asc = state.meta.ascension_level;
        let bite_dmg = bite_base_damage(entity);
        let mut actions = Vec::new();

        match entity.next_move_byte {
            3 => {
                // BITE
                actions.push(Action::Damage(DamageInfo {
                    source: entity.id,
                    target: 0, // Player
                    base: bite_dmg,
                    output: bite_dmg,
                    damage_type: DamageType::Normal,
                    is_modified: false,
                }));
            }
            4 => {
                // WEAKEN
                actions.push(Action::ApplyPower {
                    target: 0, // Player
                    source: entity.id,
                    power_id: PowerId::Weak,
                    amount: 2,
                });
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
        hp_rng: &mut crate::runtime::rng::StsRng,
        ascension_level: u8,
    ) -> Vec<Action> {
        let curl_up_amount = if ascension_level >= 17 {
            hp_rng.random_range(9, 12) as i32
        } else if ascension_level >= 7 {
            hp_rng.random_range(4, 8) as i32
        } else {
            hp_rng.random_range(3, 7) as i32
        };
        vec![Action::ApplyPower {
            target: entity.id,
            source: entity.id,
            power_id: PowerId::CurlUp,
            amount: curl_up_amount,
        }]
    }
}

#[cfg(test)]
mod tests {
    use super::LouseDefensive;
    use crate::content::monsters::{EnemyId, MonsterBehavior};
    use crate::runtime::action::Action;
    use crate::runtime::combat::{
        CardZones, CombatMeta, CombatPhase, CombatRng, CombatRuntimeHints, CombatState,
        EngineRuntime, EntityState, Intent, MonsterEntity, PlayerEntity, RelicBuses, StanceId,
        TurnRuntime,
    };
    use crate::runtime::rng::RngPool;
    use std::collections::{HashMap, VecDeque};

    fn test_combat_state() -> CombatState {
        CombatState {
            meta: CombatMeta {
                ascension_level: 0,
                player_class: "Ironclad",
                is_boss_fight: false,
                is_elite_fight: false,
                meta_changes: Vec::new(),
            },
            turn: TurnRuntime {
                turn_count: 1,
                current_phase: CombatPhase::PlayerTurn,
                energy: 0,
                turn_start_draw_modifier: 0,
                counters: Default::default(),
            },
            zones: CardZones {
                draw_pile: Vec::new(),
                hand: Vec::new(),
                discard_pile: Vec::new(),
                exhaust_pile: Vec::new(),
                limbo: Vec::new(),
                queued_cards: VecDeque::new(),
                card_uuid_counter: 0,
            },
            entities: EntityState {
                player: PlayerEntity {
                    id: 0,
                    current_hp: 80,
                    max_hp: 80,
                    block: 0,
                    gold_delta_this_combat: 0,
                    gold: 99,
                    max_orbs: 0,
                    orbs: Vec::new(),
                    stance: StanceId::Neutral,
                    relics: Vec::new(),
                    relic_buses: RelicBuses::default(),
                    energy_master: 3,
                },
                monsters: Vec::new(),
                potions: vec![None, None, None],
                power_db: HashMap::new(),
            },
            engine: EngineRuntime::new(),
            rng: CombatRng::new(RngPool::new(12345)),
            runtime: CombatRuntimeHints::default(),
        }
    }

    #[test]
    fn take_turn_uses_base_intent_damage_not_adjusted_preview() {
        let mut combat = test_combat_state();
        let monster = MonsterEntity {
            id: 1,
            monster_type: EnemyId::LouseDefensive as usize,
            current_hp: 8,
            max_hp: 14,
            block: 0,
            slot: 0,
            is_dying: false,
            is_escaped: false,
            half_dead: false,
            next_move_byte: 3,
            current_intent: Intent::Attack { damage: 5, hits: 1 },
            move_history: VecDeque::from([4, 3]),
            intent_preview_damage: 8,
            logical_position: 0,
            protocol_identity: Default::default(),
            hexaghost: Default::default(),
            chosen: Default::default(),
            darkling: Default::default(),
            lagavulin: Default::default(),
        };

        let queued = LouseDefensive::take_turn(&mut combat, &monster);
        match &queued[0] {
            Action::Damage(info) => assert_eq!(info.base, 5),
            other => panic!("expected damage action, got {other:?}"),
        }
    }
}
