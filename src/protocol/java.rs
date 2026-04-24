use serde_json::{Map, Value};
use std::collections::HashMap;

use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::content::potions::PotionId;
use crate::content::powers::PowerId;
use crate::content::relics::RelicId;
use crate::runtime::combat::Intent;
use crate::state::core::ClientInput;

// Java→Rust ID mappings — AUTO-GENERATED from compiled_protocol_schema.json by build.rs
include!(concat!(env!("OUT_DIR"), "/generated_schema.rs"));

const SHARED_COMBAT_CONTEXT_KEYS: &[&str] = &["room_type", "potions", "relics"];

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtocolCombatActionKind {
    EndTurn,
    PlayCard,
    UsePotion,
    Proceed,
    Cancel,
    SubmitChoice,
}

impl ProtocolCombatActionKind {
    fn parse(value: &str) -> Option<Self> {
        match value {
            "end_turn" => Some(Self::EndTurn),
            "play_card" => Some(Self::PlayCard),
            "use_potion" => Some(Self::UsePotion),
            "proceed" => Some(Self::Proceed),
            "cancel" => Some(Self::Cancel),
            "submit_choice" => Some(Self::SubmitChoice),
            _ => None,
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::EndTurn => "end_turn",
            Self::PlayCard => "play_card",
            Self::UsePotion => "use_potion",
            Self::Proceed => "proceed",
            Self::Cancel => "cancel",
            Self::SubmitChoice => "submit_choice",
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProtocolCombatRootAction {
    pub action_id: String,
    pub kind: ProtocolCombatActionKind,
    pub command: String,
    pub target_required: bool,
    pub target_index: Option<usize>,
    pub target_options: Vec<usize>,
    pub card_uuid: Option<u32>,
    pub hand_index: Option<usize>,
    pub card_id: Option<CardId>,
    pub potion_slot: Option<usize>,
    pub choice_index: Option<usize>,
    pub mirrored_input: ClientInput,
}

#[derive(Clone, Debug)]
pub struct CombatAffordanceSnapshot {
    pub screen_type: Option<String>,
    pub actions: Vec<ProtocolCombatRootAction>,
    actions_by_id: HashMap<String, usize>,
    card_actions_by_uuid: HashMap<u32, Vec<usize>>,
    potion_actions_by_slot: HashMap<usize, Vec<usize>>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtocolNoncombatActionKind {
    Choose,
    Proceed,
    Cancel,
    PotionDiscard,
    Unknown(String),
}

impl ProtocolNoncombatActionKind {
    fn parse(value: &str) -> Self {
        match value {
            "choose" | "submit_choice" => Self::Choose,
            "proceed" => Self::Proceed,
            "cancel" => Self::Cancel,
            "potion_discard" => Self::PotionDiscard,
            other => Self::Unknown(other.to_string()),
        }
    }

    pub fn as_str(&self) -> &str {
        match self {
            Self::Choose => "choose",
            Self::Proceed => "proceed",
            Self::Cancel => "cancel",
            Self::PotionDiscard => "potion_discard",
            Self::Unknown(value) => value.as_str(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct ProtocolNoncombatAction {
    pub action_id: String,
    pub kind: ProtocolNoncombatActionKind,
    pub command: String,
    pub choice_index: Option<usize>,
    pub choice_label: Option<String>,
    pub potion_slot: Option<usize>,
}

#[derive(Clone, Debug)]
pub struct NoncombatAffordanceSnapshot {
    pub screen_type: Option<String>,
    pub actions: Vec<ProtocolNoncombatAction>,
    actions_by_id: HashMap<String, usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtocolContinuationStateStatus {
    Active,
    Resolved,
    Unknown(String),
}

impl ProtocolContinuationStateStatus {
    fn parse(value: &str) -> Self {
        match value {
            "active" => Self::Active,
            "resolved" => Self::Resolved,
            other => Self::Unknown(other.to_string()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ProtocolDeferredHookKind {
    OnUsePotion,
    Unknown(String),
}

impl ProtocolDeferredHookKind {
    fn parse(value: &str) -> Self {
        match value {
            "on_use_potion" => Self::OnUsePotion,
            other => Self::Unknown(other.to_string()),
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProtocolContinuationState {
    pub kind: Option<String>,
    pub status: Option<ProtocolContinuationStateStatus>,
    pub screen_type: Option<String>,
    pub choice_source: Option<String>,
    pub choice_destination: Option<String>,
    pub producer_kind: Option<String>,
    pub producer_id: Option<String>,
    pub deferred_hook_kinds: Vec<ProtocolDeferredHookKind>,
}

impl ProtocolContinuationState {
    pub fn requests_on_use_potion_hooks(&self) -> bool {
        self.deferred_hook_kinds
            .iter()
            .any(|kind| matches!(kind, ProtocolDeferredHookKind::OnUsePotion))
    }
}

impl CombatAffordanceSnapshot {
    pub fn protocol_root_inputs(&self) -> Vec<ClientInput> {
        self.actions
            .iter()
            .map(|action| action.mirrored_input.clone())
            .collect()
    }

    pub fn len(&self) -> usize {
        self.actions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    pub fn action_by_id(&self, action_id: &str) -> Option<&ProtocolCombatRootAction> {
        self.actions_by_id
            .get(action_id)
            .and_then(|index| self.actions.get(*index))
    }

    pub fn find_by_input(&self, input: &ClientInput) -> Option<&ProtocolCombatRootAction> {
        self.actions
            .iter()
            .find(|action| &action.mirrored_input == input)
    }

    pub fn card_actions_for_uuid(&self, uuid: u32) -> Vec<&ProtocolCombatRootAction> {
        self.card_actions_by_uuid
            .get(&uuid)
            .into_iter()
            .flat_map(|indices| indices.iter().filter_map(|index| self.actions.get(*index)))
            .collect()
    }

    pub fn potion_actions_for_slot(&self, slot: usize) -> Vec<&ProtocolCombatRootAction> {
        self.potion_actions_by_slot
            .get(&slot)
            .into_iter()
            .flat_map(|indices| indices.iter().filter_map(|index| self.actions.get(*index)))
            .collect()
    }
}

impl NoncombatAffordanceSnapshot {
    pub fn len(&self) -> usize {
        self.actions.len()
    }

    pub fn is_empty(&self) -> bool {
        self.actions.is_empty()
    }

    pub fn action_by_id(&self, action_id: &str) -> Option<&ProtocolNoncombatAction> {
        self.actions_by_id
            .get(action_id)
            .and_then(|index| self.actions.get(*index))
    }

    pub fn choice_labels(&self) -> Vec<String> {
        self.actions
            .iter()
            .filter(|action| matches!(action.kind, ProtocolNoncombatActionKind::Choose))
            .filter_map(|action| action.choice_label.clone())
            .collect()
    }

    pub fn command_for_choice_index(&self, choice_index: usize) -> Option<&str> {
        self.actions
            .iter()
            .find(|action| {
                matches!(action.kind, ProtocolNoncombatActionKind::Choose)
                    && action.choice_index == Some(choice_index)
            })
            .map(|action| action.command.as_str())
    }

    pub fn command_for_potion_discard_slot(&self, potion_slot: usize) -> Option<&str> {
        self.actions
            .iter()
            .find(|action| {
                matches!(action.kind, ProtocolNoncombatActionKind::PotionDiscard)
                    && action.potion_slot == Some(potion_slot)
            })
            .map(|action| action.command.as_str())
    }

    pub fn first_command_for_kind(&self, kind: ProtocolNoncombatActionKind) -> Option<&str> {
        self.actions
            .iter()
            .find(|action| action.kind == kind)
            .map(|action| action.command.as_str())
    }

    pub fn first_command_matching(&self, command: &str) -> Option<&str> {
        self.actions
            .iter()
            .find(|action| action.command.eq_ignore_ascii_case(command))
            .map(|action| action.command.as_str())
    }
}

fn normalize_java_alias(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn stable_u32_from_str(s: &str) -> u32 {
    let mut hash = 0x811C9DC5u32;
    for &byte in s.as_bytes() {
        hash ^= byte as u32;
        hash = hash.wrapping_mul(0x01000193);
    }
    hash
}

fn clone_object(value: Option<&Value>) -> Map<String, Value> {
    value
        .and_then(Value::as_object)
        .cloned()
        .unwrap_or_default()
}

fn overlay_shared_combat_context(snapshot: &mut Map<String, Value>, game_state: &Value) {
    for key in SHARED_COMBAT_CONTEXT_KEYS {
        if snapshot.contains_key(*key) {
            continue;
        }
        if let Some(value) = game_state.get(*key).cloned() {
            snapshot.insert((*key).to_string(), value);
        }
    }
}

fn derive_draw_pile_count(game_state: &Value) -> Option<u64> {
    game_state
        .get("combat_observation")
        .and_then(|snapshot| snapshot.get("draw_pile_count"))
        .and_then(Value::as_u64)
        .or_else(|| {
            game_state
                .get("combat_truth")
                .and_then(|snapshot| snapshot.get("draw_pile"))
                .and_then(Value::as_array)
                .map(|cards| cards.len() as u64)
        })
}

fn protocol_meta_value(value: &Value) -> Option<&Value> {
    value.get("protocol_meta").or_else(|| {
        value
            .get("protocol_context")
            .and_then(|context| context.get("protocol_meta"))
    })
}

fn protocol_continuation_value(value: &Value) -> Option<&Value> {
    value
        .get("continuation_state")
        .or_else(|| {
            value
                .get("protocol_context")
                .and_then(|context| context.get("continuation_state"))
        })
        .or_else(|| protocol_meta_value(value).and_then(|meta| meta.get("continuation_state")))
}

pub fn protocol_continuation_state_supported(value: &Value) -> bool {
    protocol_meta_value(value)
        .and_then(|meta| meta.get("capabilities"))
        .and_then(|caps| caps.get("continuation_state"))
        .and_then(Value::as_bool)
        .unwrap_or(false)
}

pub fn parse_protocol_continuation_state(value: &Value) -> Option<ProtocolContinuationState> {
    let continuation = protocol_continuation_value(value)?.as_object()?;
    Some(ProtocolContinuationState {
        kind: continuation
            .get("kind")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        status: continuation
            .get("state")
            .and_then(Value::as_str)
            .map(ProtocolContinuationStateStatus::parse),
        screen_type: continuation
            .get("screen_type")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        choice_source: continuation
            .get("choice_source")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        choice_destination: continuation
            .get("choice_destination")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        producer_kind: continuation
            .get("producer_kind")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        producer_id: continuation
            .get("producer_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        deferred_hook_kinds: continuation
            .get("deferred_hook_kinds")
            .and_then(Value::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(Value::as_str)
                    .map(ProtocolDeferredHookKind::parse)
                    .collect()
            })
            .unwrap_or_default(),
    })
}

pub fn continuation_state_requests_on_use_potion_hooks(value: &Value) -> Option<bool> {
    if let Some(continuation) = parse_protocol_continuation_state(value) {
        return Some(continuation.requests_on_use_potion_hooks());
    }
    protocol_continuation_state_supported(value).then_some(false)
}

/// Java protocol adapter: executable combat truth normalized out of the live
/// top-level `game_state`.
pub fn build_live_truth_snapshot(game_state: &Value) -> Value {
    let mut snapshot = clone_object(game_state.get("combat_truth"));
    overlay_shared_combat_context(&mut snapshot, game_state);
    Value::Object(snapshot)
}

/// Java protocol adapter: visible combat observation normalized out of the
/// live top-level `game_state`.
pub fn build_live_observation_snapshot(game_state: &Value) -> Value {
    let mut snapshot = clone_object(game_state.get("combat_observation"));
    if !snapshot.contains_key("draw_pile_count") {
        if let Some(count) = derive_draw_pile_count(game_state) {
            snapshot.insert("draw_pile_count".to_string(), Value::from(count));
        }
    }
    overlay_shared_combat_context(&mut snapshot, game_state);
    Value::Object(snapshot)
}

pub fn build_combat_affordance_snapshot(
    protocol_meta: &Value,
    combat: &crate::runtime::combat::CombatState,
) -> Result<Option<CombatAffordanceSnapshot>, String> {
    let Some(action_space) = protocol_meta.get("combat_action_space") else {
        return Ok(None);
    };
    if action_space.is_null() {
        return Ok(None);
    }
    let actions = action_space
        .get("actions")
        .and_then(Value::as_array)
        .ok_or_else(|| "protocol_meta.combat_action_space.actions missing".to_string())?;
    let screen_type = action_space
        .get("screen_type")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let mut affordance_actions = Vec::with_capacity(actions.len());
    let mut actions_by_id = HashMap::new();
    let mut card_actions_by_uuid: HashMap<u32, Vec<usize>> = HashMap::new();
    let mut potion_actions_by_slot: HashMap<usize, Vec<usize>> = HashMap::new();

    for (index, raw_action) in actions.iter().enumerate() {
        let action_id = raw_action
            .get("action_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("protocol-action-{index}"));
        let kind_raw = raw_action
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("combat_action_space action {action_id} missing kind"))?;
        let kind = ProtocolCombatActionKind::parse(kind_raw).ok_or_else(|| {
            format!("combat_action_space action {action_id} has unsupported kind '{kind_raw}'")
        })?;
        let command = raw_action
            .get("command")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("combat_action_space action {action_id} missing command"))?
            .to_string();
        let target_required = raw_action
            .get("target_required")
            .and_then(Value::as_bool)
            .unwrap_or(false);
        let target_options = raw_action
            .get("target_options")
            .and_then(Value::as_array)
            .map(|entries| {
                entries
                    .iter()
                    .filter_map(|entry| entry.as_u64().map(|value| value as usize))
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        let target_index = raw_action
            .get("target_index")
            .and_then(Value::as_u64)
            .map(|value| value as usize)
            .or_else(|| {
                if target_required && target_options.len() == 1 {
                    Some(target_options[0])
                } else {
                    None
                }
            });
        let card_uuid = raw_action
            .get("card_uuid")
            .map(|value| snapshot_uuid(value, index as u32));
        let hand_index = raw_action
            .get("hand_index")
            .and_then(Value::as_u64)
            .map(|value| value as usize);
        let card_id = raw_action
            .get("card_id")
            .and_then(Value::as_str)
            .and_then(card_id_from_java);
        let potion_slot = raw_action
            .get("potion_slot")
            .and_then(Value::as_u64)
            .map(|value| value as usize);
        let choice_index = raw_action
            .get("choice_index")
            .and_then(Value::as_u64)
            .map(|value| value as usize);
        let mirrored_input = protocol_root_action_to_input(
            &action_id,
            &kind,
            hand_index,
            potion_slot,
            choice_index,
            target_index,
            combat,
        )?;

        let affordance_action = ProtocolCombatRootAction {
            action_id: action_id.clone(),
            kind,
            command,
            target_required,
            target_index,
            target_options,
            card_uuid,
            hand_index,
            card_id,
            potion_slot,
            choice_index,
            mirrored_input,
        };
        if let Some(uuid) = affordance_action.card_uuid {
            card_actions_by_uuid.entry(uuid).or_default().push(index);
        }
        if let Some(slot) = affordance_action.potion_slot {
            potion_actions_by_slot.entry(slot).or_default().push(index);
        }
        actions_by_id.insert(action_id, index);
        affordance_actions.push(affordance_action);
    }

    Ok(Some(CombatAffordanceSnapshot {
        screen_type,
        actions: affordance_actions,
        actions_by_id,
        card_actions_by_uuid,
        potion_actions_by_slot,
    }))
}

pub fn build_noncombat_affordance_snapshot(
    protocol_meta: &Value,
) -> Result<Option<NoncombatAffordanceSnapshot>, String> {
    build_screen_action_space(protocol_meta, "noncombat_action_space")
}

pub fn build_screen_affordance_snapshot(
    protocol_meta: &Value,
) -> Result<Option<NoncombatAffordanceSnapshot>, String> {
    if let Some(snapshot) = build_screen_action_space(protocol_meta, "noncombat_action_space")? {
        return Ok(Some(snapshot));
    }
    build_screen_action_space(protocol_meta, "combat_action_space")
}

fn build_screen_action_space(
    protocol_meta: &Value,
    field_name: &str,
) -> Result<Option<NoncombatAffordanceSnapshot>, String> {
    let Some(action_space) = protocol_meta.get(field_name) else {
        return Ok(None);
    };
    if action_space.is_null() {
        return Ok(None);
    }
    let actions = action_space
        .get("actions")
        .and_then(Value::as_array)
        .ok_or_else(|| format!("protocol_meta.{field_name}.actions missing"))?;
    let screen_type = action_space
        .get("screen_type")
        .and_then(Value::as_str)
        .map(ToOwned::to_owned);
    let mut affordance_actions = Vec::with_capacity(actions.len());
    let mut actions_by_id = HashMap::new();

    for (index, raw_action) in actions.iter().enumerate() {
        let action_id = raw_action
            .get("action_id")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned)
            .unwrap_or_else(|| format!("protocol-noncombat-action-{index}"));
        let kind_raw = raw_action
            .get("kind")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{field_name} action {action_id} missing kind"))?;
        let kind = ProtocolNoncombatActionKind::parse(kind_raw);
        let command = raw_action
            .get("command")
            .and_then(Value::as_str)
            .ok_or_else(|| format!("{field_name} action {action_id} missing command"))?
            .to_string();
        let choice_index = raw_action
            .get("choice_index")
            .and_then(Value::as_u64)
            .map(|value| value as usize);
        let choice_label = raw_action
            .get("choice_label")
            .and_then(Value::as_str)
            .map(ToOwned::to_owned);
        let potion_slot = raw_action
            .get("potion_slot")
            .and_then(Value::as_u64)
            .map(|value| value as usize);

        actions_by_id.insert(action_id.clone(), index);
        affordance_actions.push(ProtocolNoncombatAction {
            action_id,
            kind,
            command,
            choice_index,
            choice_label,
            potion_slot,
        });
    }

    Ok(Some(NoncombatAffordanceSnapshot {
        screen_type,
        actions: affordance_actions,
        actions_by_id,
    }))
}

fn protocol_root_action_to_input(
    action_id: &str,
    kind: &ProtocolCombatActionKind,
    hand_index: Option<usize>,
    potion_slot: Option<usize>,
    choice_index: Option<usize>,
    target_index: Option<usize>,
    combat: &crate::runtime::combat::CombatState,
) -> Result<ClientInput, String> {
    let target = match target_index {
        Some(index) => Some(
            combat
                .entities
                .monsters
                .get(index)
                .map(|monster| monster.id)
                .ok_or_else(|| {
                    format!(
                        "combat_action_space action {action_id} has out-of-bounds target_index {index}"
                    )
                })?,
        ),
        None => None,
    };
    match kind {
        ProtocolCombatActionKind::EndTurn => Ok(ClientInput::EndTurn),
        ProtocolCombatActionKind::PlayCard => Ok(ClientInput::PlayCard {
            card_index: hand_index.ok_or_else(|| {
                format!("combat_action_space action {action_id} missing hand_index")
            })?,
            target,
        }),
        ProtocolCombatActionKind::UsePotion => Ok(ClientInput::UsePotion {
            potion_index: potion_slot.ok_or_else(|| {
                format!("combat_action_space action {action_id} missing potion_slot")
            })?,
            target,
        }),
        ProtocolCombatActionKind::Proceed => Ok(ClientInput::Proceed),
        ProtocolCombatActionKind::Cancel => Ok(ClientInput::Cancel),
        ProtocolCombatActionKind::SubmitChoice => {
            Ok(ClientInput::SubmitDiscoverChoice(choice_index.ok_or_else(
                || format!("combat_action_space action {action_id} missing choice_index"),
            )?))
        }
    }
}

pub fn snapshot_uuid(raw: &Value, fallback: u32) -> u32 {
    if let Some(uuid) = raw.as_u64() {
        uuid as u32
    } else if let Some(uuid) = raw.as_str() {
        stable_u32_from_str(uuid)
    } else {
        fallback
    }
}

pub fn card_id_from_java(s: &str) -> Option<CardId> {
    let map = crate::content::cards::build_java_id_map();
    if let Some(card) = map.get(s).copied() {
        return Some(card);
    }
    let normalized = normalize_java_alias(s);
    if normalized.is_empty() {
        return None;
    }
    map.into_iter()
        .find_map(|(java, card)| (normalize_java_alias(java) == normalized).then_some(card))
}

pub fn power_id_from_java(s: &str) -> Option<PowerId> {
    let normalized = normalize_java_alias(s);
    if normalized.is_empty() {
        return None;
    }
    if normalized.starts_with("thebomb") {
        return Some(PowerId::TheBombPower);
    }
    power_id_from_java_raw(&normalized)
}

pub fn power_instance_id_from_java(s: &str) -> Option<u32> {
    let normalized = normalize_java_alias(s);
    if normalized.is_empty() {
        return None;
    }
    if let Some(suffix) = normalized.strip_prefix("thebomb") {
        if !suffix.is_empty() {
            return suffix.parse::<u32>().ok();
        }
    }
    None
}

pub fn relic_id_from_java(s: &str) -> Option<RelicId> {
    let normalized = normalize_java_alias(s);
    if normalized.is_empty() {
        return None;
    }
    relic_id_from_java_raw(&normalized)
}

pub fn monster_id_from_java(s: &str) -> Option<EnemyId> {
    let normalized = normalize_java_alias(s);
    if normalized.is_empty() {
        return None;
    }
    if normalized == "serpent" {
        return Some(EnemyId::SpireGrowth);
    }
    monster_id_from_java_raw(&normalized)
}

pub fn java_potion_id_to_rust(s: &str) -> Option<PotionId> {
    let normalized = normalize_java_alias(s);
    if normalized.is_empty() {
        return None;
    }
    java_potion_id_to_rust_raw(&normalized)
}

/// Java protocol adapter: convert visible intent payload into the legacy
/// observation enum. Semantic execution must not use this as move truth.
pub fn intent_from_java(intent_str: &str, damage: i32, hits: i32) -> Intent {
    match intent_str {
        "ATTACK" => Intent::Attack {
            damage,
            hits: hits.max(1) as u8,
        },
        "ATTACK_BUFF" => Intent::AttackBuff {
            damage,
            hits: hits.max(1) as u8,
        },
        "ATTACK_DEBUFF" => Intent::AttackDebuff {
            damage,
            hits: hits.max(1) as u8,
        },
        "ATTACK_DEFEND" => Intent::AttackDefend {
            damage,
            hits: hits.max(1) as u8,
        },
        "BUFF" => Intent::Buff,
        "DEBUFF" => Intent::Debuff,
        "STRONG_DEBUFF" => Intent::StrongDebuff,
        "DEBUG" => Intent::Debug,
        "DEFEND" => Intent::Defend,
        "DEFEND_DEBUFF" => Intent::DefendDebuff,
        "DEFEND_BUFF" => Intent::DefendBuff,
        "ESCAPE" => Intent::Escape,
        "MAGIC" => Intent::Magic,
        "NONE" => Intent::None,
        "SLEEP" => Intent::Sleep,
        "STUN" => Intent::Stun,
        "UNKNOWN" | _ => Intent::Unknown,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::state_sync::build_combat_state_from_snapshots;
    use serde_json::json;
    use std::path::PathBuf;

    fn load_fixture_root() -> Value {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("tests")
            .join("protocol_truth_samples")
            .join("sentry_livecomm")
            .join("frame.json");
        let text = std::fs::read_to_string(path).expect("fixture");
        serde_json::from_str(&text).expect("fixture json")
    }

    fn build_fixture_combat() -> (Value, crate::runtime::combat::CombatState) {
        let root = load_fixture_root();
        let game_state = root.get("game_state").expect("game_state");
        let truth = build_live_truth_snapshot(game_state);
        let observation = build_live_observation_snapshot(game_state);
        let relics = game_state.get("relics").unwrap_or(&Value::Null);
        let combat = build_combat_state_from_snapshots(&truth, &observation, relics);
        (truth, combat)
    }

    #[test]
    fn build_combat_affordance_snapshot_maps_protocol_root_actions() {
        let (_, combat) = build_fixture_combat();
        let action_space = json!({
            "combat_action_space": {
                "screen_type": "NONE",
                "actions": [
                    {
                        "action_id": "end_turn",
                        "kind": "end_turn",
                        "command": "END",
                        "target_required": false,
                        "target_options": []
                    },
                    {
                        "action_id": "play-0",
                        "kind": "play_card",
                        "command": "PLAY 1",
                        "target_required": false,
                        "target_options": [],
                        "hand_index": 0,
                        "card_uuid": "card-uuid-1",
                        "card_id": "Strike_R"
                    },
                    {
                        "action_id": "potion-0",
                        "kind": "use_potion",
                        "command": "POTION USE 0",
                        "target_required": false,
                        "target_options": [],
                        "potion_slot": 0
                    }
                ]
            }
        });
        let snapshot = build_combat_affordance_snapshot(&action_space, &combat)
            .expect("affordance parse")
            .expect("action space");
        assert_eq!(snapshot.len(), 3);
        assert_eq!(
            snapshot.action_by_id("end_turn").unwrap().mirrored_input,
            ClientInput::EndTurn
        );
        assert_eq!(
            snapshot.action_by_id("play-0").unwrap().mirrored_input,
            ClientInput::PlayCard {
                card_index: 0,
                target: None
            }
        );
        assert_eq!(
            snapshot.action_by_id("potion-0").unwrap().mirrored_input,
            ClientInput::UsePotion {
                potion_index: 0,
                target: None
            }
        );
        let card_uuid = snapshot.action_by_id("play-0").unwrap().card_uuid.unwrap();
        assert_eq!(snapshot.card_actions_for_uuid(card_uuid).len(), 1);
        assert_eq!(snapshot.potion_actions_for_slot(0).len(), 1);
    }

    #[test]
    fn build_noncombat_affordance_snapshot_maps_protocol_actions() {
        let action_space = json!({
            "noncombat_action_space": {
                "screen_type": "COMBAT_REWARD",
                "actions": [
                    {
                        "action_id": "choice:combat_reward:7",
                        "kind": "choose",
                        "command": "CHOOSE 7",
                        "choice_index": 7,
                        "choice_label": "gold"
                    },
                    {
                        "action_id": "cancel:combat_reward",
                        "kind": "cancel",
                        "command": "SKIP"
                    },
                    {
                        "action_id": "potion_discard:1",
                        "kind": "potion_discard",
                        "command": "POTION DISCARD 1",
                        "potion_slot": 1
                    }
                ]
            }
        });

        let snapshot = build_noncombat_affordance_snapshot(&action_space)
            .expect("affordance parse")
            .expect("action space");

        assert_eq!(snapshot.len(), 3);
        assert_eq!(snapshot.screen_type.as_deref(), Some("COMBAT_REWARD"));
        assert_eq!(
            snapshot
                .action_by_id("choice:combat_reward:7")
                .unwrap()
                .choice_label
                .as_deref(),
            Some("gold")
        );
        assert_eq!(snapshot.command_for_choice_index(7), Some("CHOOSE 7"));
        assert_eq!(
            snapshot.first_command_for_kind(ProtocolNoncombatActionKind::Cancel),
            Some("SKIP")
        );
        assert_eq!(
            snapshot.command_for_potion_discard_slot(1),
            Some("POTION DISCARD 1")
        );
        assert_eq!(snapshot.choice_labels(), vec!["gold".to_string()]);
    }

    #[test]
    fn build_screen_affordance_snapshot_accepts_combat_pending_choice_actions() {
        let action_space = json!({
            "combat_action_space": {
                "screen_type": "GRID",
                "actions": [
                    {
                        "action_id": "choice:grid:0",
                        "kind": "submit_choice",
                        "command": "CHOOSE 0",
                        "choice_index": 0,
                        "choice_label": "strike"
                    },
                    {
                        "action_id": "proceed:grid",
                        "kind": "proceed",
                        "command": "CONFIRM"
                    }
                ]
            }
        });

        let snapshot = build_screen_affordance_snapshot(&action_space)
            .expect("affordance parse")
            .expect("action space");

        assert_eq!(snapshot.screen_type.as_deref(), Some("GRID"));
        assert_eq!(snapshot.command_for_choice_index(0), Some("CHOOSE 0"));
        assert_eq!(
            snapshot.first_command_for_kind(ProtocolNoncombatActionKind::Proceed),
            Some("CONFIRM")
        );
        assert_eq!(snapshot.choice_labels(), vec!["strike".to_string()]);
    }

    #[test]
    fn live_truth_import_preserves_potion_affordance_fields() {
        let root = load_fixture_root();
        let game_state = root.get("game_state").expect("game_state");
        let mut truth = build_live_truth_snapshot(game_state);
        truth["potions"] = json!([
            {
                "id": "FairyPotion",
                "name": "Fairy in a Bottle",
                "uuid": "fairy-1",
                "can_use": false,
                "can_discard": true,
                "requires_target": false
            },
            {
                "id": "Potion Slot",
                "name": "Potion Slot",
                "can_use": false,
                "can_discard": false,
                "requires_target": false
            },
            {
                "id": "Potion Slot",
                "name": "Potion Slot",
                "can_use": false,
                "can_discard": false,
                "requires_target": false
            }
        ]);
        let observation = build_live_observation_snapshot(game_state);
        let relics = game_state.get("relics").unwrap_or(&Value::Null);
        let combat = build_combat_state_from_snapshots(&truth, &observation, relics);
        let potion = combat.entities.potions[0].as_ref().expect("fairy potion");
        assert_eq!(potion.id, PotionId::FairyPotion);
        assert!(!potion.can_use);
        assert!(potion.can_discard);
        assert!(!potion.requires_target);
    }

    #[test]
    fn parse_protocol_continuation_state_reads_protocol_meta_payload() {
        let root = json!({
            "protocol_meta": {
                "capabilities": {
                    "continuation_state": true
                },
                "continuation_state": {
                    "kind": "card_reward_continuation",
                    "state": "resolved",
                    "screen_type": "CARD_REWARD",
                    "choice_source": "colorless_potion",
                    "choice_destination": "hand",
                    "producer_kind": "potion",
                    "producer_id": "ColorlessPotion",
                    "deferred_hook_kinds": ["on_use_potion"]
                }
            }
        });

        let continuation = parse_protocol_continuation_state(&root).expect("continuation");
        assert_eq!(
            continuation.kind.as_deref(),
            Some("card_reward_continuation")
        );
        assert_eq!(
            continuation.status,
            Some(ProtocolContinuationStateStatus::Resolved)
        );
        assert_eq!(
            continuation.choice_source.as_deref(),
            Some("colorless_potion")
        );
        assert_eq!(continuation.producer_id.as_deref(), Some("ColorlessPotion"));
        assert!(continuation.requests_on_use_potion_hooks());
        assert_eq!(
            continuation_state_requests_on_use_potion_hooks(&root),
            Some(true)
        );
    }

    #[test]
    fn continuation_state_capability_false_means_explicit_no_hooks() {
        let root = json!({
            "protocol_meta": {
                "capabilities": {
                    "continuation_state": true
                },
                "continuation_state": Value::Null
            }
        });

        assert!(protocol_continuation_state_supported(&root));
        assert_eq!(
            continuation_state_requests_on_use_potion_hooks(&root),
            Some(false)
        );
    }
}
