use crate::bot::potions::choose_immediate_potion_candidate;
use crate::bot::search::{get_legal_moves, tactical_move_bonus};
use crate::combat::{CombatState, Intent};
use crate::content::cards::{get_card_definition, CardType};
use crate::content::monsters::EnemyId;
use crate::content::potions::get_potion_definition;
use crate::state::core::ClientInput;
use crate::state::EngineState;

#[derive(Clone, Debug)]
pub struct TacticalChoice {
    pub input: ClientInput,
    pub reason: String,
}

#[derive(Clone, Debug)]
pub struct TacticalMoveStat {
    pub input: ClientInput,
    pub score: f32,
    pub detail: String,
}

#[derive(Clone, Debug)]
pub struct TacticalDiagnostics {
    pub chosen: Option<TacticalChoice>,
    pub top_moves: Vec<TacticalMoveStat>,
}

pub fn tactical_override(engine: &EngineState, combat: &CombatState) -> Option<TacticalChoice> {
    diagnose_tactical_override(engine, combat).chosen
}

pub fn diagnose_tactical_override(
    engine: &EngineState,
    combat: &CombatState,
) -> TacticalDiagnostics {
    if !matches!(engine, EngineState::CombatPlayerTurn) {
        return TacticalDiagnostics {
            chosen: None,
            top_moves: Vec::new(),
        };
    }

    if let Some(candidate) = choose_immediate_potion_candidate(combat) {
        return TacticalDiagnostics {
            chosen: Some(TacticalChoice {
                input: candidate.input.clone(),
                reason: format!(
                    "potion {}: {}",
                    crate::bot::potions::category_label(candidate.category),
                    candidate.reason
                ),
            }),
            top_moves: vec![TacticalMoveStat {
                input: candidate.input,
                score: candidate.priority as f32,
                detail: format!(
                    "potion category={} reason={}",
                    crate::bot::potions::category_label(candidate.category),
                    candidate.reason
                ),
            }],
        };
    }

    let legal_moves = get_legal_moves(engine, combat);
    if legal_moves.len() <= 1 {
        return TacticalDiagnostics {
            chosen: None,
            top_moves: Vec::new(),
        };
    }

    let stable_lethal_stats = collect_stable_lethal_stats(engine, combat, &legal_moves);
    if let Some(choice) = stable_lethal_stats.first() {
        return TacticalDiagnostics {
            chosen: Some(TacticalChoice {
                input: choice.input.clone(),
                reason: "stable lethal".to_string(),
            }),
            top_moves: stable_lethal_stats,
        };
    }

    if incoming_damage(combat) >= combat.entities.player.current_hp + combat.entities.player.block {
        let end_turn_margin = legal_moves
            .iter()
            .find(|candidate| matches!(candidate, ClientInput::EndTurn))
            .map(|candidate| {
                let (sim_engine, sim_combat) =
                    simulate_to_decision_point(engine, combat, candidate.clone());
                survival_margin(&sim_engine, &sim_combat)
            })
            .unwrap_or(i32::MIN);
        let survival_stats = collect_survival_stats(engine, combat, &legal_moves, end_turn_margin);
        if let Some(choice) = survival_stats.first() {
            return TacticalDiagnostics {
                chosen: Some(TacticalChoice {
                    input: choice.input.clone(),
                    reason: "survival override".to_string(),
                }),
                top_moves: survival_stats,
            };
        }
    }

    let priority_target_stats = collect_priority_target_stats(combat, &legal_moves);
    if let Some(choice) = priority_target_stats.first() {
        return TacticalDiagnostics {
            chosen: Some(TacticalChoice {
                input: choice.input.clone(),
                reason: "priority target".to_string(),
            }),
            top_moves: priority_target_stats,
        };
    }

    TacticalDiagnostics {
        chosen: None,
        top_moves: Vec::new(),
    }
}

fn collect_stable_lethal_stats(
    engine: &EngineState,
    combat: &CombatState,
    legal_moves: &[ClientInput],
) -> Vec<TacticalMoveStat> {
    let mut scored = Vec::new();

    for candidate in legal_moves {
        if matches!(candidate, ClientInput::EndTurn) {
            continue;
        }

        let (sim_engine, sim_combat) =
            simulate_to_decision_point(engine, combat, candidate.clone());
        if !is_combat_cleared(&sim_engine, &sim_combat) {
            continue;
        }

        let score = crate::bot::evaluator::evaluate_state(&sim_engine, &sim_combat)
            + tactical_move_bonus(combat, candidate);
        scored.push(TacticalMoveStat {
            input: candidate.clone(),
            score,
            detail: "combat clears".to_string(),
        });
    }

    scored.sort_by(|a, b| b.score.total_cmp(&a.score));
    scored
}

fn collect_survival_stats(
    engine: &EngineState,
    combat: &CombatState,
    legal_moves: &[ClientInput],
    end_turn_margin: i32,
) -> Vec<TacticalMoveStat> {
    let mut scored = Vec::new();

    for candidate in legal_moves {
        if matches!(candidate, ClientInput::EndTurn) {
            continue;
        }

        let (sim_engine, sim_combat) =
            simulate_to_decision_point(engine, combat, candidate.clone());
        let margin = survival_margin(&sim_engine, &sim_combat);
        let tactical_bonus = tactical_move_bonus(combat, candidate) as i32 / 100;
        let boosted_margin = margin + tactical_bonus;
        if boosted_margin <= end_turn_margin {
            continue;
        }
        scored.push(TacticalMoveStat {
            input: candidate.clone(),
            score: boosted_margin as f32,
            detail: format!(
                "margin={} tactical_bonus={} end_margin={}",
                margin, tactical_bonus, end_turn_margin
            ),
        });
    }

    scored.sort_by(|a, b| b.score.total_cmp(&a.score));
    scored
}

fn collect_priority_target_stats(
    combat: &CombatState,
    legal_moves: &[ClientInput],
) -> Vec<TacticalMoveStat> {
    let mut scored = Vec::new();

    for candidate in legal_moves {
        let target_id = match candidate {
            ClientInput::PlayCard {
                target: Some(target),
                ..
            }
            | ClientInput::UsePotion {
                target: Some(target),
                ..
            } => *target,
            _ => continue,
        };
        let Some(monster) = combat.entities.monsters.iter().find(|m| {
            m.id == target_id && m.current_hp > 0 && !m.is_dying && !m.is_escaped && !m.half_dead
        }) else {
            continue;
        };

        let priority = monster_priority_score(combat, monster);
        if priority < 8_000.0 {
            continue;
        }

        let action_score = targeted_action_score(combat, candidate);
        let tactical_bonus = tactical_move_bonus(combat, candidate);
        let total = priority + action_score + tactical_bonus;
        scored.push(TacticalMoveStat {
            input: candidate.clone(),
            score: total,
            detail: format!(
                "monster_priority={:.1} action_score={:.1} tactical_bonus={:.1}",
                priority, action_score, tactical_bonus
            ),
        });
    }

    scored.sort_by(|a, b| b.score.total_cmp(&a.score));
    scored
}

fn simulate_to_decision_point(
    engine: &EngineState,
    combat: &CombatState,
    input: ClientInput,
) -> (EngineState, CombatState) {
    let mut sim_engine = engine.clone();
    let mut sim_combat = combat.clone();
    crate::engine::core::tick_until_stable_turn(&mut sim_engine, &mut sim_combat, input);
    (sim_engine, sim_combat)
}

fn is_combat_cleared(engine: &EngineState, combat: &CombatState) -> bool {
    matches!(
        engine,
        EngineState::GameOver(crate::state::core::RunResult::Victory)
    ) || combat
        .entities
        .monsters
        .iter()
        .all(|m| m.is_dying || m.is_escaped || m.current_hp <= 0)
}

fn survival_margin(engine: &EngineState, combat: &CombatState) -> i32 {
    if matches!(
        engine,
        EngineState::GameOver(crate::state::core::RunResult::Defeat)
    ) || combat.entities.player.current_hp <= 0
    {
        return -1_000_000;
    }

    combat.entities.player.current_hp + combat.entities.player.block - incoming_damage(combat)
}

fn incoming_damage(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|m| !m.is_dying && !m.is_escaped && !m.half_dead && m.current_hp > 0)
        .map(|m| match m.current_intent {
            Intent::Attack { hits, .. }
            | Intent::AttackBuff { hits, .. }
            | Intent::AttackDebuff { hits, .. }
            | Intent::AttackDefend { hits, .. } => (m.intent_dmg * hits as i32).max(0),
            _ => 0,
        })
        .sum()
}

fn monster_priority_score(combat: &CombatState, monster: &crate::combat::MonsterEntity) -> f32 {
    let enemy = EnemyId::from_id(monster.monster_type);
    let mut score = 0.0;

    if combat.meta.is_boss_fight {
        score += 4_000.0;
    } else if combat.meta.is_elite_fight {
        score += 1_500.0;
    }

    score += match enemy {
        Some(EnemyId::GremlinLeader)
        | Some(EnemyId::TheCollector)
        | Some(EnemyId::Reptomancer)
        | Some(EnemyId::BronzeAutomaton)
        | Some(EnemyId::TimeEater)
        | Some(EnemyId::Hexaghost)
        | Some(EnemyId::SlimeBoss) => 8_000.0,
        Some(EnemyId::Darkling) => 5_500.0,
        _ => 0.0,
    };

    score += (combat
        .get_power(monster.id, crate::combat::PowerId::Strength)
        .max(0) as f32)
        * 350.0;

    score += match monster.current_intent {
        Intent::Attack { hits, .. }
        | Intent::AttackBuff { hits, .. }
        | Intent::AttackDebuff { hits, .. }
        | Intent::AttackDefend { hits, .. } => {
            (monster.intent_dmg.max(0) * hits as i32) as f32 * 120.0
        }
        _ => 0.0,
    };

    score
}

fn targeted_action_score(combat: &CombatState, input: &ClientInput) -> f32 {
    match input {
        ClientInput::PlayCard { card_index, .. } => {
            let Some(card) = combat.zones.hand.get(*card_index) else {
                return 0.0;
            };
            let def = get_card_definition(card.id);
            let cost = card.get_cost() as i32;
            let mut score = 0.0;
            if def.card_type == CardType::Attack {
                score += 3_500.0;
                let damage = if card.base_damage_mut > 0 {
                    card.base_damage_mut
                } else {
                    def.base_damage
                };
                score += damage.max(0) as f32 * 75.0;
            }
            if cost >= 0 && cost <= 1 {
                score += 1_000.0;
            }
            score
        }
        ClientInput::UsePotion { potion_index, .. } => combat
            .entities
            .potions
            .get(*potion_index)
            .and_then(|slot| slot.as_ref())
            .map(|p| get_potion_definition(p.id).base_potency as f32 * 120.0 + 1_500.0)
            .unwrap_or(0.0),
        _ => 0.0,
    }
}

#[cfg(test)]
mod tests {
    use super::tactical_override;
    use crate::combat::{CombatState, Intent, MonsterEntity};
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::state::core::ClientInput;
    use crate::state::EngineState;
    use crate::testing::support::test_support::{combat_with_hand_and_monsters, CombatTestExt};
    use std::collections::VecDeque;

    fn test_monster(
        id: usize,
        enemy: EnemyId,
        hp: i32,
        intent: Intent,
        intent_dmg: i32,
    ) -> MonsterEntity {
        MonsterEntity {
            id,
            monster_type: enemy as usize,
            current_hp: hp,
            max_hp: hp.max(1),
            block: 0,
            slot: (id - 1) as u8,
            is_dying: false,
            is_escaped: false,
            half_dead: false,
            next_move_byte: 0,
            current_intent: intent,
            move_history: VecDeque::new(),
            intent_dmg,
            logical_position: (id - 1) as i32,
            protocol_identity: Default::default(),
            hexaghost: Default::default(),
            chosen: Default::default(),
            darkling: Default::default(),
            lagavulin: Default::default(),
        }
    }

    fn combat(hand: &[CardId], monsters: Vec<MonsterEntity>) -> CombatState {
        combat_with_hand_and_monsters(hand, monsters)
            .with_player_hp(40)
            .with_player_gold(0)
            .with_rng_seed(7)
    }

    #[test]
    fn tactical_override_takes_stable_lethal() {
        let combat = combat(
            &[CardId::Strike, CardId::Strike, CardId::Strike],
            vec![test_monster(
                1,
                EnemyId::JawWorm,
                6,
                Intent::Attack { damage: 6, hits: 1 },
                6,
            )],
        );
        let choice = tactical_override(&EngineState::CombatPlayerTurn, &combat).unwrap();
        assert!(matches!(choice.input, ClientInput::PlayCard { .. }));
        assert_eq!(choice.reason, "stable lethal");
    }

    #[test]
    fn tactical_override_prioritizes_survival_when_dead_on_board() {
        let mut combat = combat(
            &[CardId::Impervious, CardId::Strike],
            vec![test_monster(
                1,
                EnemyId::JawWorm,
                40,
                Intent::Attack {
                    damage: 27,
                    hits: 1,
                },
                27,
            )],
        );
        combat.entities.player.current_hp = 25;
        let choice = tactical_override(&EngineState::CombatPlayerTurn, &combat).unwrap();
        assert!(matches!(
            choice.input,
            ClientInput::PlayCard { card_index: 0, .. }
        ));
        assert_eq!(choice.reason, "survival override");
    }

    #[test]
    fn tactical_override_prefers_priority_target() {
        let combat = combat(
            &[CardId::Strike, CardId::Strike],
            vec![
                test_monster(1, EnemyId::GremlinFat, 20, Intent::Buff, 0),
                test_monster(2, EnemyId::GremlinLeader, 140, Intent::Buff, 0),
            ],
        );
        let choice = tactical_override(&EngineState::CombatPlayerTurn, &combat).unwrap();
        assert!(matches!(
            choice.input,
            ClientInput::PlayCard {
                target: Some(2),
                ..
            }
        ));
        assert_eq!(choice.reason, "priority target");
    }
}
