use crate::combat::Intent;
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;
use crate::content::powers::PowerId;
use crate::content::monsters::EnemyId;

// ============================================================================
// String → Enum Mappings (auto-generated where possible)
// ============================================================================

use std::collections::HashMap;
use std::sync::LazyLock;
use crate::content::cards::{CardId, build_java_id_map};

// Java→Rust ID Mappings — AUTO-GENERATED from protocol_schema.json by build.rs
include!(concat!(env!("OUT_DIR"), "/generated_schema.rs"));

static JAVA_CARD_MAP: LazyLock<HashMap<&'static str, CardId>> = LazyLock::new(|| build_java_id_map());

pub fn card_id_from_java(s: &str) -> Option<CardId> {
    JAVA_CARD_MAP.get(s).copied().or_else(|| {
        eprintln!("  UNMAPPED card: {:?}", s);
        None
    })
}

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
