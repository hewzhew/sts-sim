use serde_json::Value;

// ============================================================================
// JSONL Parsing
// ============================================================================

#[derive(Debug)]
pub struct ReplayData {
    pub format_version: u32,
    pub capabilities: Vec<String>,
    pub combats: Vec<CombatReplay>,
}

#[derive(Debug)]
pub struct CombatReplay {
    pub combat_idx: usize,
    pub floor: u32,
    pub monster_names: Vec<String>,
    pub start_snapshot: Value,
    pub relics_val: Value,
    pub actions: Vec<ReplayAction>,
}

#[derive(Debug)]
pub struct ReplayAction {
    pub action_type: String,  // "play" | "end_turn" | "potion"
    pub card_index: Option<usize>,
    pub target: Option<usize>,
    pub command: Option<String>,
    pub result: Value,  // The Java state snapshot AFTER this action
}

pub fn parse_replay(jsonl_path: &str) -> ReplayData {
    let content = std::fs::read_to_string(jsonl_path).expect("Failed to read JSONL");
    let events: Vec<Value> = content.lines()
        .map(|l| serde_json::from_str(l).expect("Invalid JSON line"))
        .collect();

    let mut format_version: u32 = 1; // default for old data without version
    let mut capabilities: Vec<String> = Vec::new();
    let mut combats: Vec<CombatReplay> = Vec::new();
    let mut current_combat: Option<CombatReplay> = None;

    for event in &events {
        let event_type = event["type"].as_str().unwrap_or("");
        
        match event_type {
            "init" => {
                format_version = event["format_version"].as_u64().unwrap_or(1) as u32;
                capabilities = event["capabilities"].as_array()
                    .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect())
                    .unwrap_or_default();
            }
            "combat_start" => {
                if let Some(c) = current_combat.take() {
                    combats.push(c);
                }
                let monsters: Vec<String> = event["monsters"].as_array()
                    .unwrap_or(&vec![])
                    .iter()
                    .map(|m| m["name"].as_str().unwrap_or("?").to_string())
                    .collect();
                current_combat = Some(CombatReplay {
                    combat_idx: event["combat_idx"].as_u64().unwrap_or(0) as usize,
                    floor: event["floor"].as_u64().unwrap_or(0) as u32,
                    monster_names: monsters,
                    start_snapshot: event["snapshot"].clone(),
                    relics_val: event["relics"].clone(),
                    actions: Vec::new(),
                });
            }
            "play" => {
                if let Some(ref mut c) = current_combat {
                    c.actions.push(ReplayAction {
                        action_type: "play".into(),
                        card_index: event["card_index"].as_u64().map(|v| v as usize),
                        target: event["target"].as_u64().map(|v| v as usize),
                        command: None,
                        result: event["result"].clone(),
                    });
                }
            }
            "end_turn" => {
                if let Some(ref mut c) = current_combat {
                    c.actions.push(ReplayAction {
                        action_type: "end_turn".into(),
                        card_index: None,
                        target: None,
                        command: None,
                        result: event["result"].clone(),
                    });
                }
            }
            "potion" => {
                if let Some(ref mut c) = current_combat {
                    c.actions.push(ReplayAction {
                        action_type: "potion".into(),
                        card_index: None,
                        target: None,
                        command: event["command"].as_str().map(|s| s.to_string()),
                        result: event["result"].clone(),
                    });
                }
            }
            "sync" => {
                if let Some(ref mut c) = current_combat {
                    c.actions.push(ReplayAction {
                        action_type: "sync".into(),
                        card_index: None,
                        target: None,
                        command: event["command"].as_str().map(|s| s.to_string()),
                        result: event["result"].clone(),
                    });
                }
            }
            "combat_end" => {
                if let Some(c) = current_combat.take() {
                    combats.push(c);
                }
            }
            _ => {}
        }
    }
    if let Some(c) = current_combat.take() {
        combats.push(c);
    }
    ReplayData { format_version, capabilities, combats }
}
