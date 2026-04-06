use serde_json::Value;
use std::collections::HashMap;

#[derive(Debug, Default)]
pub struct EntityDelta {
    pub hp_diff: i64,
    pub block_diff: i64,
    pub power_changes: Vec<String>,
}

pub fn calculate_entity_delta(prev: &Value, curr: &Value) -> EntityDelta {
    let prev_hp = prev["current_hp"].as_i64().unwrap_or(0);
    let curr_hp = curr["current_hp"].as_i64().unwrap_or(0);
    let prev_block = prev["block"].as_i64().unwrap_or(0);
    let curr_block = curr["block"].as_i64().unwrap_or(0);

    let mut delta = EntityDelta {
        hp_diff: curr_hp - prev_hp,
        block_diff: curr_block - prev_block,
        power_changes: Vec::new(),
    };

    let mut prev_powers = HashMap::new();
    if let Some(arr) = prev["powers"].as_array() {
        for p in arr {
            if let (Some(id), Some(amount)) = (p["id"].as_str(), p["amount"].as_i64()) {
                prev_powers.insert(id, amount);
            }
        }
    }

    let mut curr_powers = HashMap::new();
    if let Some(arr) = curr["powers"].as_array() {
        for p in arr {
            if let (Some(id), Some(amount)) = (p["id"].as_str(), p["amount"].as_i64()) {
                curr_powers.insert(id, amount);
                if let Some(&prev_amt) = prev_powers.get(id) {
                    if amount != prev_amt {
                        delta.power_changes.push(format!("[~] {}: {} -> {}", id, prev_amt, amount));
                    }
                } else {
                    delta.power_changes.push(format!("[+] {}({})", id, amount));
                }
            }
        }
    }

    for (id, amt) in prev_powers {
        if !curr_powers.contains_key(id) {
            delta.power_changes.push(format!("[-] {}({})", id, amt));
        }
    }

    delta
}

pub fn resolve_card_name(state: &Value, hand_index: usize) -> String {
    if let Some(hand) = state["combat_state"]["player"]["hand"].as_array() {
        if let Some(card) = hand.get(hand_index) {
            let name = card["id"].as_str().unwrap_or("Unknown");
            let upgrades = card["upgrades"].as_i64().unwrap_or(0);
            if upgrades > 0 {
                return format!("{} (+{})", name, upgrades);
            }
            return name.to_string();
        }
    }
    format!("Card {}", hand_index)
}
