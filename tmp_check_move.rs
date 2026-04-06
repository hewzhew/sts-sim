use serde_json::Value;
use std::fs::File;
use std::io::{BufRead, BufReader};

fn main() {
    let file = File::open("d:/rust/sts_simulator/live_comm_raw.jsonl").unwrap();
    let reader = BufReader::new(file);

    for (i, line_res) in reader.lines().enumerate() {
        let line = line_res.unwrap();
        if let Ok(json) = serde_json::from_str::<Value>(&line) {
            if let Some(cs) = json.get("combat_state") {
                if let Some(monsters) = cs.get("monsters").and_then(|m| m.as_array()) {
                    for m in monsters {
                        if m.get("id").and_then(|id| id.as_str()) == Some("SpikeSlime_M") {
                            println!("Frame {}: move_id={:?}, intent={:?}", i, m.get("move_id"), m.get("intent"));
                        }
                    }
                }
            }
        }
    }
}
