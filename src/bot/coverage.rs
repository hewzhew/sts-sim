use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

pub fn archetype_tags_for_combat(combat: &crate::combat::CombatState) -> Vec<String> {
    crate::bot::evaluator::CardEvaluator::archetype_tags(
        &crate::bot::evaluator::CardEvaluator::combat_profile(combat),
    )
}

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
pub enum CoverageMode {
    Off,
    PreferNovel,
    AggressiveNovel,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
pub enum CuriosityTarget {
    Card(String),
    Relic(String),
    Potion(String),
    Archetype(String),
    PowerTag(String),
    PileTag(String),
    PendingChoice(String),
    Source(String),
}

impl CuriosityTarget {
    pub fn card(name: impl Into<String>) -> Self {
        Self::Card(name.into())
    }

    pub fn potion(name: impl Into<String>) -> Self {
        Self::Potion(name.into())
    }

    pub fn archetype(name: impl Into<String>) -> Self {
        Self::Archetype(name.into())
    }

    pub fn relic(name: impl Into<String>) -> Self {
        Self::Relic(name.into())
    }

    pub fn power_tag(name: impl Into<String>) -> Self {
        Self::PowerTag(name.into())
    }

    pub fn pile_tag(name: impl Into<String>) -> Self {
        Self::PileTag(name.into())
    }

    pub fn pending_choice(name: impl Into<String>) -> Self {
        Self::PendingChoice(name.into())
    }

    pub fn source(name: impl Into<String>) -> Self {
        Self::Source(name.into())
    }

    pub fn label(&self) -> String {
        match self {
            Self::Card(name) => format!("card:{name}"),
            Self::Relic(name) => format!("relic:{name}"),
            Self::Potion(name) => format!("potion:{name}"),
            Self::Archetype(name) => format!("archetype:{name}"),
            Self::PowerTag(name) => format!("power:{name}"),
            Self::PileTag(name) => format!("pile:{name}"),
            Self::PendingChoice(name) => format!("pending:{name}"),
            Self::Source(name) => format!("source:{name}"),
        }
    }
}

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct CoverageDb {
    pub tested_cards: HashSet<String>,
    pub tested_relics: HashSet<String>,
    pub tested_potions: HashSet<String>,
    pub failed_logic: HashSet<String>,
    #[serde(default)]
    pub tested_signatures: HashSet<String>,
    #[serde(default)]
    pub signature_counts: HashMap<String, u32>,
    #[serde(default)]
    pub source_signature_counts: HashMap<String, u32>,
    #[serde(default)]
    pub tested_archetypes: HashSet<String>,
}

impl CoverageDb {
    pub fn load_from_path(path: &std::path::Path) -> Self {
        if let Ok(data) = std::fs::read_to_string(path) {
            if let Ok(db) = serde_json::from_str(&data) {
                return db;
            }
        }
        Self::default()
    }

    pub fn load_or_default() -> Self {
        Self::load_from_path(std::path::Path::new("coverage.json"))
    }

    pub fn save(&self) {
        if let Ok(data) = serde_json::to_string_pretty(self) {
            let _ = std::fs::write("coverage.json", data);
        }
    }

    // Check if an item is completely brand new (Untested)
    pub fn is_card_untested(&self, card_name: &str) -> bool {
        !self.tested_cards.contains(card_name) && !self.failed_logic.contains(card_name)
    }

    pub fn is_relic_untested(&self, relic_name: &str) -> bool {
        !self.tested_relics.contains(relic_name) && !self.failed_logic.contains(relic_name)
    }

    pub fn is_potion_untested(&self, potion_name: &str) -> bool {
        !self.tested_potions.contains(potion_name) && !self.failed_logic.contains(potion_name)
    }

    pub fn record_signature(
        &mut self,
        signature: &crate::interaction_signatures::InteractionSignature,
    ) {
        let key = signature.canonical_key();
        let source_key = signature.source_combo_key();
        self.tested_signatures.insert(key.clone());
        *self.signature_counts.entry(key).or_insert(0) += 1;
        *self.source_signature_counts.entry(source_key).or_insert(0) += 1;
        for tag in &signature.archetype_tags {
            self.tested_archetypes.insert(tag.clone());
        }
    }

    pub fn record_source_combo(
        &mut self,
        signature: &crate::interaction_signatures::InteractionSignature,
    ) {
        let source_key = signature.source_combo_key();
        *self.source_signature_counts.entry(source_key).or_insert(0) += 1;
    }
}

pub fn novelty_bonus(
    signature_key: Option<&str>,
    source_combo_key: Option<&str>,
    db: &CoverageDb,
    mode: CoverageMode,
) -> f32 {
    match mode {
        CoverageMode::Off => 0.0,
        CoverageMode::PreferNovel => {
            let mut bonus = 0.0;
            if let Some(sig) = signature_key {
                if !db.tested_signatures.contains(sig) {
                    bonus += 120_000.0;
                }
            }
            if let Some(source_combo) = source_combo_key {
                if !db.source_signature_counts.contains_key(source_combo) {
                    bonus += 45_000.0;
                }
            }
            bonus
        }
        CoverageMode::AggressiveNovel => {
            let mut bonus = 0.0;
            if let Some(sig) = signature_key {
                if !db.tested_signatures.contains(sig) {
                    bonus += 180_000.0;
                }
            }
            if let Some(source_combo) = source_combo_key {
                if !db.source_signature_counts.contains_key(source_combo) {
                    bonus += 70_000.0;
                }
            }
            bonus
        }
    }
}

pub fn curiosity_bonus(
    signature: Option<&crate::interaction_signatures::InteractionSignature>,
    target: Option<&CuriosityTarget>,
) -> f32 {
    if let (Some(signature), Some(target)) = (signature, target) {
        if curiosity_target_matches(signature, target) {
            return 95_000.0;
        }
    }
    0.0
}

pub fn curiosity_target_matches(
    signature: &crate::interaction_signatures::InteractionSignature,
    target: &CuriosityTarget,
) -> bool {
    match target {
        CuriosityTarget::Card(name) => {
            signature.source_kind == "card" && equals_ignore_ascii_case(&signature.source_id, name)
        }
        CuriosityTarget::Relic(name) => {
            signature.source_kind == "relic" && equals_ignore_ascii_case(&signature.source_id, name)
        }
        CuriosityTarget::Potion(name) => {
            signature.source_kind == "potion"
                && equals_ignore_ascii_case(&signature.source_id, name)
        }
        CuriosityTarget::Archetype(tag) => signature
            .archetype_tags
            .iter()
            .any(|value| equals_ignore_ascii_case(value, tag)),
        CuriosityTarget::PowerTag(tag) => signature
            .power_tags
            .iter()
            .any(|value| equals_ignore_ascii_case(value, tag)),
        CuriosityTarget::PileTag(tag) => signature
            .pile_tags
            .iter()
            .any(|value| equals_ignore_ascii_case(value, tag)),
        CuriosityTarget::PendingChoice(tag) => {
            signature.pending_choice != "none"
                && signature
                    .pending_choice
                    .to_ascii_lowercase()
                    .contains(&tag.to_ascii_lowercase())
        }
        CuriosityTarget::Source(name) => equals_ignore_ascii_case(&signature.source_id, name),
    }
}

fn equals_ignore_ascii_case(left: &str, right: &str) -> bool {
    left.eq_ignore_ascii_case(right)
}
