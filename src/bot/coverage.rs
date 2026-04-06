use std::collections::HashSet;
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Default, Clone, Debug)]
pub struct CoverageDb {
    pub tested_cards: HashSet<String>,
    pub tested_relics: HashSet<String>,
    pub tested_potions: HashSet<String>,
    pub failed_logic: HashSet<String>,
}

impl CoverageDb {
    pub fn load_or_default() -> Self {
        let path = "coverage.json";
        if let Ok(data) = std::fs::read_to_string(path) {
            if let Ok(db) = serde_json::from_str(&data) {
                return db;
            }
        }
        Self::default()
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
}
