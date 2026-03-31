use std::collections::HashMap;
use serde_json::Value;

use sts_simulator::combat::{CombatState, Power};
use super::mapper::power_id_from_java;

// ============================================================================
// State Comparison
// ============================================================================

pub struct DiffResult {
    pub field: String,
    pub rust_val: String,
    pub java_val: String,
}

pub fn compare_powers(diffs: &mut Vec<DiffResult>, prefix: &str, entity_id: usize,
                  power_db: &HashMap<usize, Vec<Power>>, java_powers: &Value) {
    let rust_powers = power_db.get(&entity_id).cloned().unwrap_or_default();
    let java_arr = java_powers.as_array();
    
    if let Some(arr) = java_arr {
        for p in arr {
            let java_id = p["id"].as_str().unwrap_or("");
            let java_amount = p["amount"].as_i64().unwrap_or(0) as i32;
            
            if let Some(rust_pid) = power_id_from_java(java_id) {
                if let Some(rust_p) = rust_powers.iter().find(|rp| rp.power_type == rust_pid) {
                    if rust_p.amount != java_amount {
                        diffs.push(DiffResult {
                            field: format!("{}.power[{}].amount", prefix, java_id),
                            rust_val: rust_p.amount.to_string(),
                            java_val: java_amount.to_string(),
                        });
                    }
                } else {
                    diffs.push(DiffResult {
                        field: format!("{}.power[{}]", prefix, java_id),
                        rust_val: "MISSING".into(),
                        java_val: format!("amount={}", java_amount),
                    });
                }
            }
        }
    }
    
    for rp in &rust_powers {
        let has_match = java_arr.map_or(false, |arr| {
            arr.iter().any(|jp| {
                let jid = jp["id"].as_str().unwrap_or("");
                power_id_from_java(jid) == Some(rp.power_type)
            })
        });
        if !has_match {
            diffs.push(DiffResult {
                field: format!("{}.power[{:?}]", prefix, rp.power_type),
                rust_val: format!("amount={}", rp.amount),
                java_val: "MISSING".into(),
            });
        }
    }
}

pub fn compare_states(cs: &CombatState, java_snapshot: &Value, skip_piles: bool) -> Vec<DiffResult> {
    let mut diffs = Vec::new();
    let java_player = &java_snapshot["player"];
    
    let java_hp = java_player["hp"].as_i64().unwrap_or(0) as i32;
    if cs.player.current_hp != java_hp {
        diffs.push(DiffResult {
            field: "player.hp".into(),
            rust_val: cs.player.current_hp.to_string(),
            java_val: java_hp.to_string(),
        });
    }
    
    let java_block = java_player["block"].as_i64().unwrap_or(0) as i32;
    if cs.player.block != java_block {
        diffs.push(DiffResult {
            field: "player.block".into(),
            rust_val: cs.player.block.to_string(),
            java_val: java_block.to_string(),
        });
    }
    
    let java_energy = java_player["energy"].as_u64().unwrap_or(0) as u8;
    if cs.energy != java_energy {
        diffs.push(DiffResult {
            field: "player.energy".into(),
            rust_val: cs.energy.to_string(),
            java_val: java_energy.to_string(),
        });
    }
    
    let java_monsters = java_snapshot["monsters"].as_array();
    if let Some(java_ms) = java_monsters {
        for (i, jm) in java_ms.iter().enumerate() {
            if i >= cs.monsters.len() { continue; }
            let rm = &cs.monsters[i];
            let jm_hp = jm["hp"].as_i64().unwrap_or(0) as i32;
            let jm_block = jm["block"].as_i64().unwrap_or(0) as i32;
            
            if rm.current_hp != jm_hp {
                diffs.push(DiffResult {
                    field: format!("monster[{}].hp", i),
                    rust_val: rm.current_hp.to_string(),
                    java_val: jm_hp.to_string(),
                });
            }
            if rm.block != jm_block {
                diffs.push(DiffResult {
                    field: format!("monster[{}].block", i),
                    rust_val: rm.block.to_string(),
                    java_val: jm_block.to_string(),
                });
            }
        }
    }
    
    if !skip_piles {
        let java_hand_size = java_snapshot["hand_size"].as_u64().unwrap_or(0) as usize;
        if cs.hand.len() != java_hand_size {
            diffs.push(DiffResult {
                field: "hand_size".into(),
                rust_val: cs.hand.len().to_string(),
                java_val: java_hand_size.to_string(),
            });
        }
        
        let java_discard = java_snapshot["discard_pile_size"].as_u64().unwrap_or(0) as usize;
        if cs.discard_pile.len() != java_discard {
            diffs.push(DiffResult {
                field: "discard_pile_size".into(),
                rust_val: cs.discard_pile.len().to_string(),
                java_val: java_discard.to_string(),
            });
        }
        
        let java_exhaust = java_snapshot["exhaust_pile_size"].as_u64().unwrap_or(0) as usize;
        if cs.exhaust_pile.len() != java_exhaust {
            diffs.push(DiffResult {
                field: "exhaust_pile_size".into(),
                rust_val: cs.exhaust_pile.len().to_string(),
                java_val: java_exhaust.to_string(),
            });
        }
    }
    
    compare_powers(&mut diffs, "player", 0, &cs.power_db, &java_player["powers"]);
    
    if let Some(java_ms) = java_monsters {
        for (i, jm) in java_ms.iter().enumerate() {
            if i >= cs.monsters.len() { continue; }
            let entity_id = cs.monsters[i].id;
            compare_powers(&mut diffs, &format!("monster[{}]", i), entity_id, &cs.power_db, &jm["powers"]);
        }
    }
    
    diffs
}
