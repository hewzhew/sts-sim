use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::action::CardDestination;
use crate::combat::{CombatCard, CombatState, Power};
use crate::content::cards::java_id as card_java_id;
use crate::content::cards::CardId;
use crate::diff::mapper::{card_id_from_java, power_id_from_java, relic_id_from_java};
use crate::diff::replay_support::tick_until_stable;
use crate::diff::state_sync::build_combat_state;
use crate::state::core::{ClientInput, EngineState, PendingChoice, PileType};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioKind {
    #[default]
    Combat,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum ScenarioOracleKind {
    Synthetic,
    #[default]
    Live,
    JavaHarness,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScenarioProvenance {
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub response_id_range: Option<(u64, u64)>,
    #[serde(default)]
    pub failure_frame: Option<u64>,
    #[serde(default)]
    pub assertion_source_frames: Vec<u64>,
    #[serde(default)]
    pub assertion_source_response_ids: Vec<u64>,
    #[serde(default)]
    pub debug_context_summary: Option<Value>,
    #[serde(default)]
    pub aspect_summary: Option<Value>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScenarioFixture {
    pub name: String,
    #[serde(default)]
    pub kind: ScenarioKind,
    #[serde(default)]
    pub oracle_kind: ScenarioOracleKind,
    pub initial_game_state: Value,
    #[serde(default)]
    pub initial_protocol_meta: Option<Value>,
    pub steps: Vec<ScenarioStep>,
    pub assertions: Vec<ScenarioAssertion>,
    #[serde(default)]
    pub provenance: Option<ScenarioProvenance>,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScenarioStep {
    pub command: String,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub response_id: Option<u64>,
    #[serde(default)]
    pub frame_id: Option<u64>,
    #[serde(default)]
    pub command_kind: Option<String>,
    #[serde(default)]
    pub structured: Option<StructuredScenarioStep>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum StructuredScenarioStep {
    Play {
        selector: ScenarioCardSelector,
        #[serde(default)]
        target: Option<usize>,
    },
    End,
    Cancel,
    Choose {
        index: usize,
    },
    PotionUse {
        slot: usize,
        #[serde(default)]
        target: Option<usize>,
    },
    HandSelect {
        selectors: Vec<ScenarioCardSelector>,
    },
    GridSelect {
        selectors: Vec<ScenarioCardSelector>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ScenarioCardSelector {
    Index {
        index: usize,
    },
    JavaId {
        id: String,
        #[serde(default = "default_occurrence")]
        occurrence: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct ScenarioAssertion {
    pub field: String,
    pub expected_kind: String,
    #[serde(default)]
    pub expected_value: Option<Value>,
    #[serde(default)]
    pub response_id: Option<u64>,
    #[serde(default)]
    pub frame_id: Option<u64>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ActualFieldValue {
    Missing,
    Number(i64),
    String(String),
    Bool(bool),
}

#[derive(Debug)]
pub struct ScenarioReplay {
    pub combat: CombatState,
    pub engine_state: EngineState,
    pub snapshots: Vec<ScenarioSnapshot>,
}

#[derive(Debug, Clone)]
pub struct ScenarioInitialState {
    pub combat: CombatState,
    pub engine_state: EngineState,
    pub response_id: Option<u64>,
    pub frame_id: Option<u64>,
}

#[derive(Debug, Clone)]
pub struct ScenarioSnapshot {
    pub response_id: Option<u64>,
    pub frame_id: Option<u64>,
    pub combat: CombatState,
}

pub fn build_live_combat_snapshot(gs: &Value) -> Value {
    let mut snapshot = gs
        .get("combat_state")
        .cloned()
        .unwrap_or_else(|| serde_json::json!({}));
    if let Some(obj) = snapshot.as_object_mut() {
        if let Some(room_type) = gs.get("room_type").cloned() {
            obj.insert("room_type".to_string(), room_type);
        }
        if let Some(potions) = gs.get("potions").cloned() {
            obj.insert("potions".to_string(), potions);
        }
    }
    snapshot
}

pub fn build_initial_engine_state(
    fixture: &ScenarioFixture,
    combat: &mut CombatState,
) -> EngineState {
    let screen_type = fixture
        .initial_game_state
        .get("screen_type")
        .and_then(|v| v.as_str())
        .unwrap_or("NONE");
    if screen_type != "CARD_REWARD" {
        return EngineState::CombatPlayerTurn;
    }

    let screen_state = fixture
        .initial_game_state
        .get("screen_state")
        .and_then(|v| v.as_object());
    let Some(screen_state) = screen_state else {
        return EngineState::CombatPlayerTurn;
    };

    let Some(cards) = screen_state.get("cards").and_then(|v| v.as_array()) else {
        return EngineState::CombatPlayerTurn;
    };
    let offered = cards
        .iter()
        .filter_map(screen_card_to_rust_id)
        .collect::<Vec<_>>();
    if offered.is_empty() {
        return EngineState::CombatPlayerTurn;
    }

    let last_command_kind = fixture
        .initial_protocol_meta
        .as_ref()
        .and_then(|m| m.get("last_command_kind"))
        .and_then(|v| v.as_str());
    let last_command = fixture
        .initial_protocol_meta
        .as_ref()
        .and_then(|m| m.get("last_command"))
        .and_then(|v| v.as_str())
        .unwrap_or("");
    let skip_available = screen_state
        .get("skip_available")
        .and_then(|v| v.as_bool())
        .unwrap_or(false);

    if matches!(last_command_kind, Some("potion")) || last_command.starts_with("POTION USE ") {
        combat.turn.counters.discovery_cost_for_turn = Some(0);
        return EngineState::PendingChoice(PendingChoice::DiscoverySelect(offered));
    }

    EngineState::PendingChoice(PendingChoice::CardRewardSelect {
        cards: offered,
        destination: CardDestination::Hand,
        can_skip: skip_available,
    })
}

pub fn initialize_fixture_state(fixture: &ScenarioFixture) -> ScenarioInitialState {
    let snapshot = build_live_combat_snapshot(&fixture.initial_game_state);
    let relics = fixture
        .initial_game_state
        .get("relics")
        .cloned()
        .unwrap_or(Value::Null);
    let mut combat = build_combat_state(&snapshot, &relics);
    let engine_state = build_initial_engine_state(fixture, &mut combat);
    let response_id = fixture
        .initial_protocol_meta
        .as_ref()
        .and_then(|meta| meta.get("response_id"))
        .and_then(json_u64);
    let frame_id = fixture
        .initial_protocol_meta
        .as_ref()
        .and_then(|meta| meta.get("state_frame_id").or_else(|| meta.get("frame_id")))
        .and_then(json_u64);

    ScenarioInitialState {
        combat,
        engine_state,
        response_id,
        frame_id,
    }
}

pub fn replay_fixture(fixture: &ScenarioFixture) -> Result<ScenarioReplay, String> {
    let initial = initialize_fixture_state(fixture);
    let mut combat = initial.combat;
    let mut engine_state = initial.engine_state;
    let mut snapshots = vec![ScenarioSnapshot {
        response_id: initial.response_id,
        frame_id: initial.frame_id,
        combat: combat.clone(),
    }];

    for step in &fixture.steps {
        let input = input_for_step(step, &engine_state, &combat).ok_or_else(|| {
            format!(
                "fixture '{}' contains unsupported step '{}'",
                fixture.name, step.command
            )
        })?;
        let alive = tick_until_stable(&mut engine_state, &mut combat, input);
        if !alive {
            return Err(format!(
                "fixture '{}' died while executing '{}'",
                fixture.name, step.command
            ));
        }
        snapshots.push(ScenarioSnapshot {
            response_id: step.response_id,
            frame_id: step.frame_id,
            combat: combat.clone(),
        });
    }

    Ok(ScenarioReplay {
        combat,
        engine_state,
        snapshots,
    })
}

pub fn assert_fixture(fixture: &ScenarioFixture) -> Result<(), String> {
    let replay = replay_fixture(fixture)?;
    for assertion in &fixture.assertions {
        let combat = combat_for_assertion(&replay, assertion).map_err(|err| {
            format!(
                "fixture '{}' could not resolve assertion scope: {err}",
                fixture.name
            )
        })?;
        let actual = extract_field_value(combat, &assertion.field);
        let expected = parse_expected(assertion)?;
        if actual != expected {
            return Err(format!(
                "fixture '{}' mismatch on {}{}: actual={actual:?} expected={expected:?}",
                fixture.name,
                assertion.field,
                format_assertion_scope(assertion),
            ));
        }
    }
    Ok(())
}

fn combat_for_assertion<'a>(
    replay: &'a ScenarioReplay,
    assertion: &ScenarioAssertion,
) -> Result<&'a CombatState, String> {
    if assertion.response_id.is_none() && assertion.frame_id.is_none() {
        return Ok(&replay.combat);
    }

    replay
        .snapshots
        .iter()
        .rev()
        .find(|snapshot| {
            assertion
                .response_id
                .map_or(true, |response_id| snapshot.response_id == Some(response_id))
                && assertion
                    .frame_id
                    .map_or(true, |frame_id| snapshot.frame_id == Some(frame_id))
        })
        .map(|snapshot| &snapshot.combat)
        .ok_or_else(|| {
            let available_response_ids = replay
                .snapshots
                .iter()
                .filter_map(|snapshot| snapshot.response_id)
                .collect::<Vec<_>>();
            let available_frame_ids = replay
                .snapshots
                .iter()
                .filter_map(|snapshot| snapshot.frame_id)
                .collect::<Vec<_>>();
            format!(
                "no snapshot for response_id={:?} frame_id={:?}; available response_ids={available_response_ids:?}, frame_ids={available_frame_ids:?}",
                assertion.response_id, assertion.frame_id
            )
        })
}

fn format_assertion_scope(assertion: &ScenarioAssertion) -> String {
    let mut parts = Vec::new();
    if let Some(response_id) = assertion.response_id {
        parts.push(format!("response_id={response_id}"));
    }
    if let Some(frame_id) = assertion.frame_id {
        parts.push(format!("frame_id={frame_id}"));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" [{}]", parts.join(", "))
    }
}

fn json_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok()))
}

pub fn input_for_step(
    step: &ScenarioStep,
    engine_state: &EngineState,
    combat: &CombatState,
) -> Option<ClientInput> {
    if let Some(structured) = &step.structured {
        return structured_input_for_state(structured, engine_state, combat);
    }
    parse_command_for_state(&step.command, engine_state)
}

pub fn parse_command_for_state(command: &str, engine_state: &EngineState) -> Option<ClientInput> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    match parts.as_slice() {
        ["END"] => Some(ClientInput::EndTurn),
        ["CANCEL"] => Some(ClientInput::Cancel),
        ["PLAY", card_idx] => Some(ClientInput::PlayCard {
            card_index: card_idx.parse::<usize>().ok()?.saturating_sub(1),
            target: None,
        }),
        ["PLAY", card_idx, target] => Some(ClientInput::PlayCard {
            card_index: card_idx.parse::<usize>().ok()?.saturating_sub(1),
            target: Some(target.parse::<usize>().ok()? + 1),
        }),
        ["POTION", "USE", slot] => Some(ClientInput::UsePotion {
            potion_index: slot.parse().ok()?,
            target: None,
        }),
        ["POTION", "USE", slot, target] => Some(ClientInput::UsePotion {
            potion_index: slot.parse().ok()?,
            target: Some(target.parse::<usize>().ok()? + 1),
        }),
        ["HUMAN_CARD_REWARD", "SKIP"] => {
            if matches!(engine_state, EngineState::PendingChoice(_)) {
                Some(ClientInput::Cancel)
            } else {
                None
            }
        }
        ["HUMAN_CARD_REWARD", choice_idx] => {
            let idx = choice_idx.parse::<usize>().ok()?;
            if matches!(engine_state, EngineState::PendingChoice(_)) {
                Some(ClientInput::SubmitDiscoverChoice(idx))
            } else {
                None
            }
        }
        _ => None,
    }
}

fn structured_input_for_state(
    step: &StructuredScenarioStep,
    engine_state: &EngineState,
    combat: &CombatState,
) -> Option<ClientInput> {
    match step {
        StructuredScenarioStep::Play { selector, target } => Some(ClientInput::PlayCard {
            card_index: resolve_hand_selector(combat, selector)?,
            target: target.map(|t| t + 1),
        }),
        StructuredScenarioStep::End => Some(ClientInput::EndTurn),
        StructuredScenarioStep::Cancel => Some(ClientInput::Cancel),
        StructuredScenarioStep::Choose { index } => {
            if matches!(engine_state, EngineState::PendingChoice(_)) {
                Some(ClientInput::SubmitDiscoverChoice(*index))
            } else {
                None
            }
        }
        StructuredScenarioStep::PotionUse { slot, target } => Some(ClientInput::UsePotion {
            potion_index: *slot,
            target: target.map(|t| t + 1),
        }),
        StructuredScenarioStep::HandSelect { selectors } => {
            let PendingChoice::HandSelect {
                candidate_uuids, ..
            } = pending_choice(engine_state)?
            else {
                return None;
            };
            let uuids = resolve_pending_selectors(selectors, candidate_uuids, &combat.zones.hand)?;
            Some(ClientInput::SubmitHandSelect(uuids))
        }
        StructuredScenarioStep::GridSelect { selectors } => {
            let PendingChoice::GridSelect {
                source_pile,
                candidate_uuids,
                ..
            } = pending_choice(engine_state)?
            else {
                return None;
            };
            let pile = pile_cards(combat, *source_pile)?;
            let uuids = resolve_pending_selectors(selectors, candidate_uuids, pile)?;
            Some(ClientInput::SubmitGridSelect(uuids))
        }
    }
}

fn pending_choice(engine_state: &EngineState) -> Option<&PendingChoice> {
    let EngineState::PendingChoice(choice) = engine_state else {
        return None;
    };
    Some(choice)
}

fn resolve_hand_selector(combat: &CombatState, selector: &ScenarioCardSelector) -> Option<usize> {
    match selector {
        ScenarioCardSelector::Index { index } => index.checked_sub(1),
        ScenarioCardSelector::JavaId { id, occurrence } => {
            nth_matching_card_index(&combat.zones.hand, id, (*occurrence).max(1))
        }
    }
}

fn resolve_pending_selectors(
    selectors: &[ScenarioCardSelector],
    candidate_uuids: &[u32],
    cards: &[CombatCard],
) -> Option<Vec<u32>> {
    let candidate_cards = candidate_uuids
        .iter()
        .filter_map(|uuid| cards.iter().find(|card| card.uuid == *uuid))
        .cloned()
        .collect::<Vec<_>>();
    let mut selected = Vec::new();
    let mut remaining = candidate_cards.clone();
    for selector in selectors {
        let uuid = match selector {
            ScenarioCardSelector::Index { index } => {
                let idx = index.checked_sub(1)?;
                remaining.get(idx)?.uuid
            }
            ScenarioCardSelector::JavaId { id, occurrence } => {
                let occurrence = (*occurrence).max(1);
                nth_matching_card(&remaining, id, occurrence)?.uuid
            }
        };
        selected.push(uuid);
        if let Some(pos) = remaining.iter().position(|card| card.uuid == uuid) {
            remaining.remove(pos);
        }
    }
    Some(selected)
}

fn pile_cards(combat: &CombatState, pile: PileType) -> Option<&[CombatCard]> {
    match pile {
        PileType::Discard => Some(&combat.zones.discard_pile),
        PileType::Draw => Some(&combat.zones.draw_pile),
        PileType::Exhaust => Some(&combat.zones.exhaust_pile),
        PileType::Hand => Some(&combat.zones.hand),
        PileType::Limbo => Some(&combat.zones.limbo),
        PileType::MasterDeck => None,
    }
}

fn nth_matching_card_index(
    cards: &[CombatCard],
    java_id: &str,
    occurrence: usize,
) -> Option<usize> {
    let mut seen = 0usize;
    for (idx, card) in cards.iter().enumerate() {
        if card_java_id(card.id) == java_id {
            seen += 1;
            if seen == occurrence {
                return Some(idx);
            }
        }
    }
    None
}

fn nth_matching_card<'a>(
    cards: &'a [CombatCard],
    java_id: &str,
    occurrence: usize,
) -> Option<&'a CombatCard> {
    let mut seen = 0usize;
    for card in cards {
        if card_java_id(card.id) == java_id {
            seen += 1;
            if seen == occurrence {
                return Some(card);
            }
        }
    }
    None
}

fn default_occurrence() -> usize {
    1
}

pub fn parse_expected(assertion: &ScenarioAssertion) -> Result<ActualFieldValue, String> {
    match assertion.expected_kind.as_str() {
        "missing" => Ok(ActualFieldValue::Missing),
        "number" => assertion
            .expected_value
            .as_ref()
            .and_then(|v| v.as_i64())
            .map(ActualFieldValue::Number)
            .ok_or_else(|| "number assertion requires integer expected_value".to_string()),
        "string" => assertion
            .expected_value
            .as_ref()
            .and_then(|v| v.as_str())
            .map(|v| ActualFieldValue::String(v.to_string()))
            .ok_or_else(|| "string assertion requires string expected_value".to_string()),
        "bool" => assertion
            .expected_value
            .as_ref()
            .and_then(|v| v.as_bool())
            .map(ActualFieldValue::Bool)
            .ok_or_else(|| "bool assertion requires boolean expected_value".to_string()),
        other => Err(format!("unsupported expected_kind '{other}'")),
    }
}

pub fn extract_field_value(combat: &CombatState, field: &str) -> ActualFieldValue {
    if field == "monster_count" {
        return ActualFieldValue::Number(combat.entities.monsters.len() as i64);
    }
    if field == "hand_size" {
        return ActualFieldValue::Number(combat.zones.hand.len() as i64);
    }
    if field == "draw_pile_size" {
        return ActualFieldValue::Number(combat.zones.draw_pile.len() as i64);
    }
    if field == "discard_pile_size" {
        return ActualFieldValue::Number(combat.zones.discard_pile.len() as i64);
    }
    if field == "exhaust_pile_size" {
        return ActualFieldValue::Number(combat.zones.exhaust_pile.len() as i64);
    }
    if field == "limbo_size" {
        return ActualFieldValue::Number(combat.zones.limbo.len() as i64);
    }
    if field == "player.energy" {
        return ActualFieldValue::Number(combat.turn.energy as i64);
    }
    if field == "player.block" {
        return ActualFieldValue::Number(combat.entities.player.block as i64);
    }
    if field == "player.current_hp" || field == "player.hp" {
        return ActualFieldValue::Number(combat.entities.player.current_hp as i64);
    }
    if let Some(rest) = field.strip_prefix("relics.") {
        return extract_relic_field(combat, rest);
    }
    for (prefix, cards) in [
        ("hand.", &combat.zones.hand),
        ("draw_pile.", &combat.zones.draw_pile),
        ("discard_pile.", &combat.zones.discard_pile),
        ("exhaust_pile.", &combat.zones.exhaust_pile),
        ("limbo.", &combat.zones.limbo),
    ] {
        if let Some(rest) = field.strip_prefix(prefix) {
            return extract_card_pile_field(cards, rest);
        }
    }
    if let Some(rest) = field.strip_prefix("player.power[") {
        return extract_power_field(crate::content::powers::store::powers_for(combat, 0), rest);
    }
    if let Some(rest) = field.strip_prefix("monster[") {
        return extract_monster_field(combat, rest);
    }
    panic!("unsupported fixture field '{field}'");
}

fn extract_card_pile_field(cards: &[CombatCard], rest: &str) -> ActualFieldValue {
    if let Some(inner) = rest.strip_prefix("count[") {
        let java_id = parse_bracket_inner(inner, "pile count");
        let count = cards
            .iter()
            .filter(|card| card_java_id(card.id) == java_id)
            .count() as i64;
        return ActualFieldValue::Number(count);
    }
    if let Some(inner) = rest.strip_prefix("contains[") {
        let java_id = parse_bracket_inner(inner, "pile contains");
        let exists = cards.iter().any(|card| card_java_id(card.id) == java_id);
        return ActualFieldValue::Bool(exists);
    }
    panic!("unsupported pile field suffix '{rest}'");
}

fn extract_relic_field(combat: &CombatState, rest: &str) -> ActualFieldValue {
    if let Some(inner) = rest.strip_prefix("count[") {
        let java_id = parse_bracket_inner(inner, "relic count");
        let relic_id =
            relic_id_from_java(java_id).unwrap_or_else(|| panic!("unknown Java relic '{java_id}'"));
        let count = combat
            .entities
            .player
            .relics
            .iter()
            .filter(|relic| relic.id == relic_id)
            .count() as i64;
        return ActualFieldValue::Number(count);
    }
    if let Some(inner) = rest.strip_prefix("contains[") {
        let java_id = parse_bracket_inner(inner, "relic contains");
        let relic_id =
            relic_id_from_java(java_id).unwrap_or_else(|| panic!("unknown Java relic '{java_id}'"));
        let exists = combat
            .entities
            .player
            .relics
            .iter()
            .any(|relic| relic.id == relic_id);
        return ActualFieldValue::Bool(exists);
    }
    panic!("unsupported relic field suffix '{rest}'");
}

fn parse_bracket_inner<'a>(rest: &'a str, context: &str) -> &'a str {
    let close = rest
        .find(']')
        .unwrap_or_else(|| panic!("{context} missing ]"));
    &rest[..close]
}

fn extract_monster_field(combat: &CombatState, rest: &str) -> ActualFieldValue {
    let close = rest.find(']').expect("monster field missing ]");
    let idx: usize = rest[..close]
        .parse()
        .expect("monster index must be integer");
    let suffix = &rest[close + 1..];
    let monster = combat.entities.monsters.get(idx);
    match suffix {
        ".hp" | ".current_hp" => monster
            .map(|m| ActualFieldValue::Number(m.current_hp as i64))
            .unwrap_or(ActualFieldValue::Missing),
        ".block" => monster
            .map(|m| ActualFieldValue::Number(m.block as i64))
            .unwrap_or(ActualFieldValue::Missing),
        _ if suffix.starts_with(".power[") => {
            let Some(monster) = monster else {
                return ActualFieldValue::Missing;
            };
            extract_power_field(
                crate::content::powers::store::powers_for(combat, monster.id),
                &suffix[".power[".len()..],
            )
        }
        _ => panic!("unsupported monster field suffix '{suffix}'"),
    }
}

fn extract_power_field(powers: Option<&[Power]>, rest: &str) -> ActualFieldValue {
    let close = rest.find(']').expect("power field missing ]");
    let power_name = &rest[..close];
    let suffix = &rest[close + 1..];
    let power_id = power_id_from_java(power_name)
        .unwrap_or_else(|| panic!("unknown Java power '{power_name}'"));
    let power = powers.and_then(|ps| ps.iter().find(|p| p.power_type == power_id));
    match suffix {
        "" => power
            .map(|p| ActualFieldValue::String(format!("amount={}", p.amount)))
            .unwrap_or(ActualFieldValue::Missing),
        ".amount" => power
            .map(|p| ActualFieldValue::Number(p.amount as i64))
            .unwrap_or(ActualFieldValue::Missing),
        _ => panic!("unsupported power field suffix '{suffix}'"),
    }
}

fn screen_card_to_rust_id(card: &Value) -> Option<CardId> {
    let java_id = card.get("id").and_then(|v| v.as_str())?;
    card_id_from_java(java_id)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::combat::{CombatPhase, Intent, MonsterEntity, PlayerEntity, RelicBuses, StanceId};
    use crate::content::powers::PowerId;
    use std::collections::{HashMap, VecDeque};

    #[test]
    fn legacy_live_fixture_defaults_to_combat_live() {
        let fixture: ScenarioFixture = serde_json::from_value(serde_json::json!({
            "name": "legacy",
            "initial_game_state": {},
            "steps": [],
            "assertions": []
        }))
        .expect("legacy fixture should parse");

        assert_eq!(fixture.kind, ScenarioKind::Combat);
        assert_eq!(fixture.oracle_kind, ScenarioOracleKind::Live);
        assert!(fixture.provenance.is_none());
        assert!(fixture.tags.is_empty());
    }

    #[test]
    fn parse_play_and_end_commands() {
        assert!(matches!(
            parse_command_for_state("PLAY 4 0", &EngineState::CombatPlayerTurn),
            Some(ClientInput::PlayCard {
                card_index: 3,
                target: Some(1)
            })
        ));
        assert!(matches!(
            parse_command_for_state("END", &EngineState::CombatPlayerTurn),
            Some(ClientInput::EndTurn)
        ));
        assert!(matches!(
            parse_command_for_state("POTION USE 0 0", &EngineState::CombatPlayerTurn),
            Some(ClientInput::UsePotion {
                potion_index: 0,
                target: Some(1)
            })
        ));
        assert!(matches!(
            parse_command_for_state(
                "HUMAN_CARD_REWARD 2",
                &EngineState::PendingChoice(PendingChoice::DiscoverySelect(vec![
                    CardId::Discovery,
                    CardId::Chrysalis,
                    CardId::ThinkingAhead
                ]))
            ),
            Some(ClientInput::SubmitDiscoverChoice(2))
        ));
    }

    #[test]
    fn structured_play_resolves_by_java_id_occurrence() {
        let mut combat = test_combat_state();
        combat.zones.hand = vec![
            CombatCard::new(CardId::Strike, 10),
            CombatCard::new(CardId::Defend, 11),
            CombatCard::new(CardId::Strike, 12),
        ];

        let input = input_for_step(
            &ScenarioStep {
                command: "PLAY_ID Strike_R #2 -> 0".to_string(),
                structured: Some(StructuredScenarioStep::Play {
                    selector: ScenarioCardSelector::JavaId {
                        id: "Strike_R".to_string(),
                        occurrence: 2,
                    },
                    target: Some(0),
                }),
                ..Default::default()
            },
            &EngineState::CombatPlayerTurn,
            &combat,
        );

        assert!(matches!(
            input,
            Some(ClientInput::PlayCard {
                card_index: 2,
                target: Some(1)
            })
        ));
    }

    #[test]
    fn structured_hand_select_resolves_by_java_id() {
        let mut combat = test_combat_state();
        combat.zones.hand = vec![
            CombatCard::new(CardId::Acrobatics, 100),
            CombatCard::new(CardId::Survivor, 101),
            CombatCard::new(CardId::Strike, 102),
        ];
        let engine_state = EngineState::PendingChoice(PendingChoice::HandSelect {
            candidate_uuids: vec![101, 102],
            min_cards: 1,
            max_cards: 1,
            can_cancel: false,
            reason: crate::state::core::HandSelectReason::Discard,
        });

        let input = input_for_step(
            &ScenarioStep {
                command: "HAND_SELECT Survivor".to_string(),
                structured: Some(StructuredScenarioStep::HandSelect {
                    selectors: vec![ScenarioCardSelector::JavaId {
                        id: "Survivor".to_string(),
                        occurrence: 1,
                    }],
                }),
                ..Default::default()
            },
            &engine_state,
            &combat,
        );

        assert!(matches!(
            input,
            Some(ClientInput::SubmitHandSelect(uuids)) if uuids == vec![101]
        ));
    }

    #[test]
    fn extract_field_value_handles_missing_power() {
        let mut combat = test_combat_state();
        assert_eq!(
            extract_field_value(&combat, "player.power[Strength]"),
            ActualFieldValue::Missing
        );
        combat.entities.power_db.insert(
            0,
            vec![Power {
                power_type: PowerId::Strength,
                amount: 2,
                extra_data: 0,
                just_applied: false,
            }],
        );
        assert_eq!(
            extract_field_value(&combat, "player.power[Strength].amount"),
            ActualFieldValue::Number(2)
        );
    }

    #[test]
    fn scoped_assertions_can_target_response_id_snapshots() {
        let mut initial = test_combat_state();
        initial.turn.energy = 4;
        let mut final_combat = initial.clone();
        final_combat.turn.energy = 2;

        let replay = ScenarioReplay {
            combat: final_combat.clone(),
            engine_state: EngineState::CombatPlayerTurn,
            snapshots: vec![
                ScenarioSnapshot {
                    response_id: Some(540),
                    frame_id: Some(540),
                    combat: initial,
                },
                ScenarioSnapshot {
                    response_id: Some(543),
                    frame_id: Some(543),
                    combat: final_combat,
                },
            ],
        };

        let scoped = ScenarioAssertion {
            field: "player.energy".to_string(),
            expected_kind: "number".to_string(),
            expected_value: Some(serde_json::json!(4)),
            response_id: Some(540),
            ..Default::default()
        };
        let final_state = ScenarioAssertion {
            field: "player.energy".to_string(),
            expected_kind: "number".to_string(),
            expected_value: Some(serde_json::json!(2)),
            ..Default::default()
        };

        let scoped_combat = combat_for_assertion(&replay, &scoped).expect("scoped snapshot");
        let final_combat = combat_for_assertion(&replay, &final_state).expect("final snapshot");

        assert_eq!(
            extract_field_value(scoped_combat, &scoped.field),
            ActualFieldValue::Number(4)
        );
        assert_eq!(
            extract_field_value(final_combat, &final_state.field),
            ActualFieldValue::Number(2)
        );
    }

    fn test_combat_state() -> CombatState {
        CombatState {
            meta: crate::combat::CombatMeta {
                ascension_level: 0,
                is_boss_fight: false,
                is_elite_fight: false,
                meta_changes: Vec::new(),
            },
            turn: crate::combat::TurnRuntime {
                turn_count: 1,
                current_phase: CombatPhase::PlayerTurn,
                energy: 3,
                turn_start_draw_modifier: 0,
                counters: Default::default(),
            },
            zones: crate::combat::CardZones {
                draw_pile: Vec::new(),
                hand: Vec::new(),
                discard_pile: Vec::new(),
                exhaust_pile: Vec::new(),
                limbo: Vec::new(),
                queued_cards: VecDeque::new(),
                card_uuid_counter: 1000,
            },
            entities: crate::combat::EntityState {
                player: PlayerEntity {
                    id: 0,
                    current_hp: 50,
                    max_hp: 80,
                    block: 0,
                    gold_delta_this_combat: 0,
                    gold: 0,
                    max_orbs: 0,
                    orbs: Vec::new(),
                    stance: StanceId::Neutral,
                    relics: Vec::new(),
                    relic_buses: RelicBuses::default(),
                    energy_master: 3,
                },
                monsters: vec![MonsterEntity {
                    id: 1,
                    monster_type: 0,
                    current_hp: 40,
                    max_hp: 40,
                    block: 0,
                    slot: 0,
                    is_dying: false,
                    half_dead: false,
                    is_escaped: false,
                    next_move_byte: 0,
                    current_intent: Intent::Unknown,
                    move_history: VecDeque::new(),
                    intent_dmg: 0,
                    logical_position: 0,
                    hexaghost: Default::default(),
                    darkling: Default::default(),
                }],
                potions: vec![None, None, None],
                power_db: HashMap::new(),
            },
            engine: crate::combat::EngineRuntime {
                action_queue: VecDeque::new(),
            },
            rng: crate::combat::CombatRng::new(crate::rng::RngPool::new(1)),
        }
    }
}