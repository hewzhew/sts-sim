use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

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
        signature: &crate::interaction_coverage::InteractionSignature,
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
        signature: &crate::interaction_coverage::InteractionSignature,
    ) {
        let source_key = signature.source_combo_key();
        *self.source_signature_counts.entry(source_key).or_insert(0) += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::CoverageDb;

    #[test]
    fn legacy_coverage_json_loads_with_default_signature_fields() {
        let path =
            std::env::temp_dir().join(format!("coverage_legacy_{}.json", std::process::id()));
        let legacy = r#"{
            "tested_cards": ["Bash"],
            "tested_relics": ["BurningBlood"],
            "tested_potions": ["Gamblers Brew"],
            "failed_logic": ["ExampleFailure"]
        }"#;
        std::fs::write(&path, legacy).expect("write legacy coverage fixture");

        let db = CoverageDb::load_from_path(&path);
        let _ = std::fs::remove_file(&path);

        assert!(db.tested_cards.contains("Bash"));
        assert!(db.tested_relics.contains("BurningBlood"));
        assert!(db.tested_potions.contains("Gamblers Brew"));
        assert!(db.failed_logic.contains("ExampleFailure"));
        assert!(db.tested_signatures.is_empty());
        assert!(db.signature_counts.is_empty());
        assert!(db.source_signature_counts.is_empty());
        assert!(db.tested_archetypes.is_empty());
    }
}
