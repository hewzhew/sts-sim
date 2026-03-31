use std::collections::HashMap;
use std::sync::{LazyLock, OnceLock};

use sts_simulator::content::cards::{CardId, build_java_id_map};
use sts_simulator::combat::Intent;
use sts_simulator::content::potions::PotionId;
use sts_simulator::content::relics::RelicId;
use sts_simulator::content::powers::PowerId;

// ============================================================================
// String → Enum Mappings (auto-generated where possible)
// ============================================================================

/// Auto-generated from all CardDefinitions. Covers every implemented card.
static JAVA_CARD_MAP: LazyLock<HashMap<&'static str, CardId>> = LazyLock::new(|| build_java_id_map());

pub fn card_id_from_java(s: &str) -> Option<CardId> {
    JAVA_CARD_MAP.get(s).copied().or_else(|| {
        eprintln!("  UNMAPPED card: {:?}", s);
        None
    })
}

// Java→Rust ID Mappings — AUTO-GENERATED from protocol_schema.json
include!("../generated/diff_driver_generated.rs");

/// Map Java intent strings to Rust Intent enum values.
pub fn intent_from_java(intent_str: &str, damage: i32, hits: i32) -> Intent {
    match intent_str {
        "ATTACK" => Intent::Attack { damage, hits: hits.max(1) as u8 },
        "ATTACK_BUFF" => Intent::AttackBuff { damage, hits: hits.max(1) as u8 },
        "ATTACK_DEBUFF" => Intent::AttackDebuff { damage, hits: hits.max(1) as u8 },
        "ATTACK_DEFEND" => Intent::AttackDefend { damage, hits: hits.max(1) as u8 },
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

// ============================================================================
// Monster ID Mapping
// ============================================================================

fn build_monster_id_map() -> HashMap<String, usize> {
    let schema_path = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("tools/protocol_schema.json");
    let schema_text = std::fs::read_to_string(&schema_path)
        .unwrap_or_else(|e| panic!("Failed to read protocol_schema.json at {:?}: {}", schema_path, e));
    let schema: serde_json::Value = serde_json::from_str(&schema_text)
        .expect("Failed to parse protocol_schema.json");
    
    let entries = schema["enums"]["monster_id"]["entries"]
        .as_object()
        .expect("monster_id.entries missing from protocol_schema.json");
    
    let mut map = std::collections::HashMap::new();
    for (_rust_name, entry) in entries {
        let index = entry["index"].as_u64().expect("monster_id entry missing 'index'") as usize;
        let java_ids = entry["java"].as_array().expect("monster_id entry missing 'java' array");
        for java_id in java_ids {
            let key = java_id.as_str().expect("java id must be string").to_string();
            map.insert(key, index);
        }
    }
    map
}

static MONSTER_ID_MAP: OnceLock<HashMap<String, usize>> = OnceLock::new();

pub fn monster_id_from_java(s: &str) -> usize {
    let map = MONSTER_ID_MAP.get_or_init(build_monster_id_map);
    match map.get(s) {
        Some(&id) => id,
        None => {
            eprintln!("  UNMAPPED monster: {:?} — add it to protocol_schema.json monster_id entries", s);
            0
        }
    }
}
