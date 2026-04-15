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

