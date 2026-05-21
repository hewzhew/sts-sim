use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::content::potions::PotionId;
use crate::content::powers::PowerId;
use crate::content::relics::RelicId;
use crate::runtime::combat::Intent;
use serde_json::{Map, Value};

// Java→Rust ID mappings — AUTO-GENERATED from compiled_protocol_schema.json by build.rs
include!(concat!(env!("OUT_DIR"), "/generated_schema.rs"));

const SHARED_COMBAT_CONTEXT_KEYS: &[&str] = &["room_type", "potions", "relics"];

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

fn normalize_java_alias(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
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
    use crate::testing::state_sync::build_combat_state_from_snapshots;
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

    #[test]
    fn relic_id_from_java_maps_boss_pool_fallback_red_circlet() {
        assert_eq!(relic_id_from_java("Circlet"), Some(RelicId::Circlet));
        assert_eq!(relic_id_from_java("Red Circlet"), Some(RelicId::RedCirclet));
    }

    #[test]
    fn relic_id_from_java_maps_java_egg2_ids_to_rust_egg_ids() {
        assert_eq!(relic_id_from_java("Molten Egg 2"), Some(RelicId::MoltenEgg));
        assert_eq!(relic_id_from_java("Toxic Egg 2"), Some(RelicId::ToxicEgg));
        assert_eq!(relic_id_from_java("Frozen Egg 2"), Some(RelicId::FrozenEgg));
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
