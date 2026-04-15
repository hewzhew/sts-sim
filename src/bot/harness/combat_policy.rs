use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};

use crate::bot::agent::Agent;
use crate::bot::card_disposition::{build_context, classify_hand_card_with_context, HandCardRole};
use crate::bot::combat_heuristic::{self, HeuristicDiagnostics};
use crate::bot::combat_posture::posture_features;
use crate::bot::coverage::CoverageMode;
use crate::bot::search::{self, SearchDiagnostics};
use crate::combat::{CombatState, Intent};
use crate::content::cards::{get_card_definition, CardType};
use crate::state::core::{ClientInput, EngineState};
use crate::state::run::RunState;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum PolicyKind {
    Bot,
    Heuristic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncodedAction {
    pub kind: String,
    pub target: Option<usize>,
    pub card_index: Option<usize>,
    pub selected_indices: Option<Vec<usize>>,
    pub selected_uuids: Option<Vec<u32>>,
    pub card_name: Option<String>,
    pub card_type: Option<String>,
    pub potion_index: Option<usize>,
    pub potion_name: Option<String>,
    pub selection_len: Option<usize>,
    pub debug: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CandidateActionScore {
    pub action: EncodedAction,
    pub score: f32,
    pub source: String,
    pub visits: Option<u32>,
    pub avg_score: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PolicyDecision {
    pub policy_kind: PolicyKind,
    pub final_action: EncodedAction,
    pub final_input_debug: String,
    pub source: String,
    pub confidence: Option<f32>,
    pub fallback_used: bool,
    pub tactical_reason: Option<String>,
    pub candidate_scores: Vec<CandidateActionScore>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct EvalMetrics {
    pub average_damage_taken_per_episode: f32,
    pub average_potion_uses_per_episode: f32,
    pub bad_action_count: u32,
    pub action_kind_counts: BTreeMap<String, usize>,
    pub policy_source_counts: BTreeMap<String, usize>,
}

pub fn decide_policy_action(
    kind: PolicyKind,
    engine: &EngineState,
    combat: &CombatState,
    run_state: &RunState,
    agent: &mut Agent,
    depth: u32,
) -> PolicyDecision {
    if !matches!(engine, EngineState::CombatPlayerTurn) {
        let input = agent.decide(engine, run_state, &Some(combat.clone()), false);
        return PolicyDecision {
            policy_kind: kind,
            final_action: encode_action(combat, &input),
            final_input_debug: format!("{input:?}"),
            source: "agent_non_player_turn_fallback".to_string(),
            confidence: None,
            fallback_used: false,
            tactical_reason: None,
            candidate_scores: Vec::new(),
        };
    }

    match kind {
        PolicyKind::Bot => decide_bot_action(engine, combat, depth),
        PolicyKind::Heuristic => decide_heuristic_action(engine, combat),
    }
}

pub fn encode_action(combat: &CombatState, input: &ClientInput) -> EncodedAction {
    match input {
        ClientInput::PlayCard { card_index, target } => {
            let card = combat.zones.hand.get(*card_index);
            let def = card.map(|card| get_card_definition(card.id));
            EncodedAction {
                kind: "play_card".to_string(),
                target: *target,
                card_index: Some(*card_index),
                selected_indices: None,
                selected_uuids: None,
                card_name: def.as_ref().map(|def| def.name.to_string()),
                card_type: def
                    .as_ref()
                    .map(|def| format!("{:?}", def.card_type).to_lowercase()),
                potion_index: None,
                potion_name: None,
                selection_len: None,
                debug: format!("{input:?}"),
            }
        }
        ClientInput::UsePotion {
            potion_index,
            target,
        } => {
            let potion = combat
                .entities
                .potions
                .get(*potion_index)
                .and_then(|slot| slot.as_ref())
                .map(|p| {
                    crate::content::potions::get_potion_definition(p.id)
                        .name
                        .to_string()
                });
            EncodedAction {
                kind: "use_potion".to_string(),
                target: *target,
                card_index: None,
                selected_indices: None,
                selected_uuids: None,
                card_name: None,
                card_type: None,
                potion_index: Some(*potion_index),
                potion_name: potion,
                selection_len: None,
                debug: format!("{input:?}"),
            }
        }
        ClientInput::EndTurn => EncodedAction {
            kind: "end_turn".to_string(),
            target: None,
            card_index: None,
            selected_indices: None,
            selected_uuids: None,
            card_name: None,
            card_type: None,
            potion_index: None,
            potion_name: None,
            selection_len: None,
            debug: format!("{input:?}"),
        },
        ClientInput::SubmitHandSelect(uuids) => EncodedAction {
            kind: "submit_hand_select".to_string(),
            target: None,
            card_index: None,
            selected_indices: None,
            selected_uuids: Some(uuids.clone()),
            card_name: None,
            card_type: None,
            potion_index: None,
            potion_name: None,
            selection_len: Some(uuids.len()),
            debug: format!("{input:?}"),
        },
        ClientInput::SubmitGridSelect(uuids) => EncodedAction {
            kind: "submit_grid_select".to_string(),
            target: None,
            card_index: None,
            selected_indices: None,
            selected_uuids: Some(uuids.clone()),
            card_name: None,
            card_type: None,
            potion_index: None,
            potion_name: None,
            selection_len: Some(uuids.len()),
            debug: format!("{input:?}"),
        },
        ClientInput::SubmitDiscoverChoice(index) => EncodedAction {
            kind: "submit_discover_choice".to_string(),
            target: None,
            card_index: Some(*index),
            selected_indices: None,
            selected_uuids: None,
            card_name: None,
            card_type: None,
            potion_index: None,
            potion_name: None,
            selection_len: None,
            debug: format!("{input:?}"),
        },
        ClientInput::SubmitCardChoice(indices) => EncodedAction {
            kind: "submit_card_choice".to_string(),
            target: None,
            card_index: None,
            selected_indices: Some(indices.clone()),
            selected_uuids: None,
            card_name: None,
            card_type: None,
            potion_index: None,
            potion_name: None,
            selection_len: Some(indices.len()),
            debug: format!("{input:?}"),
        },
        ClientInput::Proceed => EncodedAction {
            kind: "proceed".to_string(),
            target: None,
            card_index: None,
            selected_indices: None,
            selected_uuids: None,
            card_name: None,
            card_type: None,
            potion_index: None,
            potion_name: None,
            selection_len: None,
            debug: format!("{input:?}"),
        },
        ClientInput::Cancel => EncodedAction {
            kind: "cancel".to_string(),
            target: None,
            card_index: None,
            selected_indices: None,
            selected_uuids: None,
            card_name: None,
            card_type: None,
            potion_index: None,
            potion_name: None,
            selection_len: None,
            debug: format!("{input:?}"),
        },
        _ => EncodedAction {
            kind: "other".to_string(),
            target: None,
            card_index: None,
            selected_indices: None,
            selected_uuids: None,
            card_name: None,
            card_type: None,
            potion_index: None,
            potion_name: None,
            selection_len: None,
            debug: format!("{input:?}"),
        },
    }
}

pub fn extract_state_features(combat: &CombatState) -> BTreeMap<String, f32> {
    let context = build_context(combat);
    let posture = posture_features(combat);
    let mut features = BTreeMap::new();
    let living_monsters = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && monster.current_hp > 0)
        .count() as f32;
    let total_monster_hp = combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && monster.current_hp > 0)
        .map(|monster| monster.current_hp)
        .sum::<i32>() as f32;
    let potion_slots_filled = combat
        .entities
        .potions
        .iter()
        .filter(|slot| slot.is_some())
        .count() as f32;

    features.insert("turn".to_string(), combat.turn.turn_count as f32);
    features.insert(
        "player_hp".to_string(),
        combat.entities.player.current_hp as f32,
    );
    features.insert(
        "player_max_hp".to_string(),
        combat.entities.player.max_hp as f32,
    );
    features.insert(
        "missing_hp".to_string(),
        (combat.entities.player.max_hp - combat.entities.player.current_hp).max(0) as f32,
    );
    features.insert(
        "player_block".to_string(),
        combat.entities.player.block as f32,
    );
    features.insert("energy".to_string(), combat.turn.energy as f32);
    features.insert("hand_size".to_string(), combat.zones.hand.len() as f32);
    features.insert("draw_size".to_string(), combat.zones.draw_pile.len() as f32);
    features.insert(
        "discard_size".to_string(),
        combat.zones.discard_pile.len() as f32,
    );
    features.insert(
        "exhaust_size".to_string(),
        combat.zones.exhaust_pile.len() as f32,
    );
    features.insert("living_monsters".to_string(), living_monsters);
    features.insert("remaining_monster_hp".to_string(), total_monster_hp);
    features.insert("incoming_damage".to_string(), context.total_incoming as f32);
    features.insert(
        "unblocked_incoming".to_string(),
        context.unblocked_incoming as f32,
    );
    features.insert(
        "attacking_target_present".to_string(),
        if context.has_attacking_target {
            1.0
        } else {
            0.0
        },
    );
    features.insert(
        "playable_attack_count".to_string(),
        context.playable_attack_count as f32,
    );
    features.insert(
        "followup_attack_count".to_string(),
        context.followup_attack_count as f32,
    );
    features.insert(
        "strength_payoff_count".to_string(),
        context.strength_payoff_count as f32,
    );
    features.insert(
        "status_or_curse_count".to_string(),
        context.status_or_curse_count as f32,
    );
    features.insert(
        "has_exhaust_engine".to_string(),
        if context.has_exhaust_engine { 1.0 } else { 0.0 },
    );
    features.insert(
        "has_exhaust_outlet".to_string(),
        if context.has_exhaust_outlet { 1.0 } else { 0.0 },
    );
    features.insert("filled_potion_slots".to_string(), potion_slots_filled);
    features.insert(
        "posture_immediate_survival_pressure".to_string(),
        posture.immediate_survival_pressure as f32,
    );
    features.insert(
        "posture_future_pollution_risk".to_string(),
        posture.future_pollution_risk as f32,
    );
    features.insert(
        "posture_expected_fight_length_bucket".to_string(),
        posture.expected_fight_length_bucket as f32,
    );
    features.insert(
        "posture_setup_payoff_density".to_string(),
        posture.setup_payoff_density as f32,
    );
    features.insert(
        "posture_resource_preservation_pressure".to_string(),
        posture.resource_preservation_pressure as f32,
    );

    let mut core = 0.0;
    let mut sequenced = 0.0;
    let mut situational = 0.0;
    let mut fuel = 0.0;
    for hand_index in 0..combat.zones.hand.len() {
        match classify_hand_card_with_context(combat, hand_index, &context) {
            HandCardRole::CoreKeeper => core += 1.0,
            HandCardRole::SequencedPiece => sequenced += 1.0,
            HandCardRole::SituationalResource => situational += 1.0,
            HandCardRole::LowValueFuel => fuel += 1.0,
        }
    }
    features.insert("hand_role_core_keeper".to_string(), core);
    features.insert("hand_role_sequenced_piece".to_string(), sequenced);
    features.insert("hand_role_situational".to_string(), situational);
    features.insert("hand_role_low_value_fuel".to_string(), fuel);

    features
}

pub fn flag_bad_action_tags(combat: &CombatState, decision: &PolicyDecision) -> Vec<String> {
    let mut tags = Vec::new();
    let incoming = incoming_damage(combat);
    let hp_ratio = if combat.entities.player.max_hp > 0 {
        combat.entities.player.current_hp as f32 / combat.entities.player.max_hp as f32
    } else {
        1.0
    };
    if decision.source == "tactical_override"
        && matches!(
            decision.tactical_reason.as_deref(),
            Some("survival override")
        )
        && decision.final_action.kind == "play_card"
    {
        if let Some(card_index) = decision.final_action.card_index {
            if let Some(card) = combat.zones.hand.get(card_index) {
                let def = get_card_definition(card.id);
                if matches!(def.card_type, CardType::Status | CardType::Curse) {
                    tags.push("survival_override_played_status_or_curse".to_string());
                }
            }
        }
    }
    if decision.final_action.kind == "use_potion"
        && matches!(
            decision.final_action.potion_name.as_deref(),
            Some("Block Potion" | "Colorless Potion")
        )
        && !combat.meta.is_elite_fight
        && !combat.meta.is_boss_fight
        && incoming <= 10
        && hp_ratio >= 0.75
    {
        tags.push("potion_used_on_low_pressure_turn".to_string());
    }
    if decision.final_action.kind == "play_card"
        && matches!(
            decision.final_action.card_name.as_deref(),
            Some("Power Through")
        )
        && incoming <= 0
    {
        tags.push("power_through_played_without_incoming".to_string());
    }
    tags
}

fn decide_bot_action(engine: &EngineState, combat: &CombatState, depth: u32) -> PolicyDecision {
    if let Some(choice) = search::tactical_override(engine, combat) {
        let final_action = encode_action(combat, &choice.input);
        return PolicyDecision {
            policy_kind: PolicyKind::Bot,
            final_input_debug: final_action.debug.clone(),
            final_action,
            source: "tactical_override".to_string(),
            confidence: None,
            fallback_used: false,
            tactical_reason: Some(choice.reason),
            candidate_scores: Vec::new(),
        };
    }

    let diagnostics = search::diagnose_root_search_with_depth(
        engine,
        combat,
        &crate::bot::coverage::CoverageDb::default(),
        CoverageMode::PreferNovel,
        None,
        depth,
        4000,
    );
    let final_action = encode_action(combat, &diagnostics.chosen_move);
    PolicyDecision {
        policy_kind: PolicyKind::Bot,
        final_input_debug: final_action.debug.clone(),
        final_action,
        source: "search".to_string(),
        confidence: search_confidence(&diagnostics),
        fallback_used: false,
        tactical_reason: None,
        candidate_scores: diagnostics
            .top_moves
            .iter()
            .map(|stat| CandidateActionScore {
                action: encode_action(combat, &stat.input),
                score: stat.avg_score,
                source: "search".to_string(),
                visits: Some(stat.visits),
                avg_score: Some(stat.avg_score),
            })
            .collect(),
    }
}

fn decide_heuristic_action(engine: &EngineState, combat: &CombatState) -> PolicyDecision {
    if let Some(choice) = search::tactical_override(engine, combat) {
        let final_action = encode_action(combat, &choice.input);
        return PolicyDecision {
            policy_kind: PolicyKind::Heuristic,
            final_input_debug: final_action.debug.clone(),
            final_action,
            source: "tactical_override".to_string(),
            confidence: None,
            fallback_used: false,
            tactical_reason: Some(choice.reason),
            candidate_scores: Vec::new(),
        };
    }

    let diagnostics = combat_heuristic::diagnose_decision(combat);
    let final_action = encode_action(combat, &diagnostics.chosen_move);
    PolicyDecision {
        policy_kind: PolicyKind::Heuristic,
        final_input_debug: final_action.debug.clone(),
        final_action,
        source: "heuristic".to_string(),
        confidence: heuristic_confidence(&diagnostics),
        fallback_used: false,
        tactical_reason: None,
        candidate_scores: diagnostics
            .top_moves
            .iter()
            .map(|stat| CandidateActionScore {
                action: encode_action(combat, &stat.input),
                score: stat.score as f32,
                source: "heuristic".to_string(),
                visits: None,
                avg_score: None,
            })
            .collect(),
    }
}

fn search_confidence(diagnostics: &SearchDiagnostics) -> Option<f32> {
    if diagnostics.top_moves.len() < 2 {
        return diagnostics
            .top_moves
            .first()
            .map(|move_stat| move_stat.avg_score);
    }
    Some(diagnostics.top_moves[0].avg_score - diagnostics.top_moves[1].avg_score)
}

fn heuristic_confidence(diagnostics: &HeuristicDiagnostics) -> Option<f32> {
    if diagnostics.top_moves.len() < 2 {
        return diagnostics
            .top_moves
            .first()
            .map(|move_stat| move_stat.score as f32);
    }
    Some((diagnostics.top_moves[0].score - diagnostics.top_moves[1].score) as f32)
}

pub fn incoming_damage(combat: &CombatState) -> i32 {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| !monster.is_dying && !monster.is_escaped && monster.current_hp > 0)
        .map(|monster| match monster.current_intent {
            Intent::Attack { hits, .. }
            | Intent::AttackBuff { hits, .. }
            | Intent::AttackDebuff { hits, .. }
            | Intent::AttackDefend { hits, .. } => monster.intent_dmg * hits as i32,
            _ => 0,
        })
        .sum()
}

