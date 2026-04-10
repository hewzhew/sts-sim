use crate::combat::Intent;
use crate::content::cards::CardId;
use crate::content::monsters::EnemyId;
use crate::content::potions::PotionId;
use crate::content::powers::PowerId;
use crate::content::relics::RelicId;

// ============================================================================
// String → Enum Mappings (auto-generated where possible)
// ============================================================================

// Java→Rust ID mappings — AUTO-GENERATED from compiled_protocol_schema.json by build.rs
include!(concat!(env!("OUT_DIR"), "/generated_schema.rs"));

fn normalize_java_alias(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_ascii_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
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
    power_id_from_java_raw(&normalized)
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
    monster_id_from_java_raw(&normalized)
}

pub fn java_potion_id_to_rust(s: &str) -> Option<PotionId> {
    let normalized = normalize_java_alias(s);
    if normalized.is_empty() {
        return None;
    }
    java_potion_id_to_rust_raw(&normalized)
}

/// Map Java intent strings to Rust Intent enum values.
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
    use serde_json::Value;
    use std::collections::{BTreeMap, BTreeSet};

    fn compiled_schema() -> Value {
        serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/compiled_protocol_schema.json"
        )))
        .expect("compiled schema should parse")
    }

    fn observed_ids() -> Value {
        serde_json::from_str(include_str!(concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/tools/artifacts/observed_ids.json"
        )))
        .expect("observed ids should parse")
    }

    #[test]
    fn schema_regression_key_aliases_map_stably() {
        assert_eq!(
            relic_id_from_java("Fusion Hammer"),
            Some(crate::content::relics::RelicId::FusionHammer)
        );
        assert_eq!(
            relic_id_from_java("CeramicFish"),
            Some(crate::content::relics::RelicId::CeramicFish)
        );
        assert_eq!(
            relic_id_from_java("Smiling Mask"),
            Some(crate::content::relics::RelicId::SmilingMask)
        );
        assert_eq!(
            power_id_from_java("No Draw"),
            Some(crate::content::powers::PowerId::NoDraw)
        );
        assert_eq!(
            power_id_from_java("NoDraw"),
            Some(crate::content::powers::PowerId::NoDraw)
        );
        assert_eq!(
            power_id_from_java("Sharp Hide"),
            Some(crate::content::powers::PowerId::SharpHide)
        );
        assert_eq!(
            power_id_from_java("Mode Shift"),
            Some(crate::content::powers::PowerId::ModeShift)
        );
        assert_eq!(
            power_id_from_java("IntangiblePlayer"),
            Some(crate::content::powers::PowerId::IntangiblePlayer)
        );
        assert_eq!(
            power_id_from_java("Weakened"),
            Some(crate::content::powers::PowerId::Weak)
        );
        assert_eq!(
            power_id_from_java("Regeneration"),
            Some(crate::content::powers::PowerId::Regeneration)
        );
        assert_eq!(
            power_id_from_java("Regenerate"),
            Some(crate::content::powers::PowerId::Regen)
        );
        assert_eq!(
            power_id_from_java("Life Link"),
            Some(crate::content::powers::PowerId::Regrow)
        );
        assert_eq!(
            relic_id_from_java("Clockwork Souvenir"),
            Some(crate::content::relics::RelicId::ClockworkSouvenir)
        );
        assert_eq!(
            card_id_from_java("StrikeG"),
            Some(crate::content::cards::CardId::StrikeG)
        );
    }

    #[test]
    fn compiled_schema_aliases_match_runtime_mapper() {
        let compiled = compiled_schema();
        let enums = compiled["enums"]
            .as_object()
            .expect("compiled enums object");

        for (enum_key, enum_def) in enums {
            let entries = enum_def["entries"].as_object().expect("entries object");
            for entry in entries.values() {
                let status = entry["status"].as_str().unwrap_or("mapped");
                let aliases = entry["java"].as_array().cloned().unwrap_or_default();
                for alias in aliases
                    .into_iter()
                    .filter_map(|v| v.as_str().map(str::to_owned))
                {
                    let mapped = match enum_key.as_str() {
                        "card_id" => card_id_from_java(&alias).is_some(),
                        "power_id" => power_id_from_java(&alias).is_some(),
                        "relic_id" => relic_id_from_java(&alias).is_some(),
                        "monster_id" => monster_id_from_java(&alias).is_some(),
                        "potion_id" => java_potion_id_to_rust(&alias).is_some(),
                        _ => false,
                    };
                    if matches!(status, "mapped") {
                        assert!(mapped, "expected alias {:?} in {} to map", alias, enum_key);
                    } else {
                        assert!(
                            !mapped,
                            "expected alias {:?} in {} to remain unmapped for status {}",
                            alias, enum_key, status
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn silent_manual_card_ids_map_from_java() {
        assert_eq!(
            card_id_from_java("Strike_G"),
            Some(crate::content::cards::CardId::StrikeG)
        );
        assert_eq!(
            card_id_from_java("Defend_G"),
            Some(crate::content::cards::CardId::DefendG)
        );
        assert_eq!(
            card_id_from_java("Neutralize"),
            Some(crate::content::cards::CardId::Neutralize)
        );
        assert_eq!(
            card_id_from_java("Noxious Fumes"),
            Some(crate::content::cards::CardId::NoxiousFumes)
        );
        assert_eq!(
            card_id_from_java("After Image"),
            Some(crate::content::cards::CardId::AfterImage)
        );
        assert_eq!(
            card_id_from_java("Burst"),
            Some(crate::content::cards::CardId::Burst)
        );
        assert_eq!(
            crate::content::cards::java_id(crate::content::cards::CardId::StrikeG),
            "Strike_G"
        );
    }

    #[test]
    fn observed_ids_are_covered_or_explicitly_unsupported() {
        let compiled = compiled_schema();
        let observed = observed_ids();
        let enums = compiled["enums"]
            .as_object()
            .expect("compiled enums object");

        let mut alias_status: BTreeMap<String, BTreeMap<String, String>> = BTreeMap::new();
        for (enum_key, enum_def) in enums {
            let category = enum_key.trim_end_matches("_id");
            let entries = enum_def["entries"].as_object().expect("entries object");
            let category_map = alias_status.entry(category.to_string()).or_default();
            for entry in entries.values() {
                let status = entry["status"].as_str().unwrap_or("mapped").to_string();
                if let Some(java_ids) = entry["java"].as_array() {
                    for java_id in java_ids.iter().filter_map(|v| v.as_str()) {
                        category_map.insert(java_id.to_string(), status.clone());
                    }
                }
            }
        }

        let mut unresolved = BTreeSet::new();
        for (category, ids) in observed["categories"]
            .as_object()
            .expect("observed categories object")
        {
            for java_id in ids.as_object().expect("observed id object").keys() {
                let status = alias_status
                    .get(category)
                    .and_then(|entries| entries.get(java_id))
                    .map(|s| s.as_str());
                if !matches!(
                    status,
                    Some("mapped") | Some("unsupported") | Some("internal_only")
                ) {
                    unresolved.insert(format!("{category}:{java_id}"));
                }
            }
        }

        assert!(
            unresolved.is_empty(),
            "observed ids not covered by compiled schema: {:?}",
            unresolved
        );
    }
}
