use sts_simulator::diff::protocol::parser::parse_replay;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() < 3 {
        println!("Usage: cargo run --bin view_replay <jsonl_path> <combat_idx>");
        return;
    }

    let path = &args[1];
    let combat_idx: usize = args[2].parse().unwrap();

    // Parse using our existing engine diff parser
    let data = parse_replay(path);

    let combat = data.combats.iter().find(|c| c.combat_idx == combat_idx);
    match combat {
        Some(c) => {
            println!(
                "--- Combat #{} Floor {} vs {:?} ---",
                c.combat_idx, c.floor, c.monster_names
            );
            println!("Initial Start:");
            let m_init = &c.start_snapshot["combat_state"]["monsters"][0];
            println!(
                "   Monster 0 (HP: {}, Block: {}, Intent: {})",
                m_init["current_hp"], m_init["block"], m_init["intent"]
            );

            for (i, action) in c.actions.iter().enumerate() {
                let m = &action.result["combat_state"]["monsters"][0];
                let hp = m["current_hp"].as_i64().unwrap_or(-1);
                let block = m["block"].as_i64().unwrap_or(-1);
                let intent = m["intent"].as_str().unwrap_or("?");

                let detail = match action.action_type.as_str() {
                    "play" => format!(
                        "PLAY Card {} -> Target {:?}",
                        action.card_index.unwrap_or(999),
                        action.target
                    ),
                    "end_turn" => "END_TURN".to_string(),
                    "potion" => format!("POTION {}", action.command.as_deref().unwrap_or("")),
                    "sync" => "SYNC".to_string(),
                    _ => action.action_type.clone(),
                };

                println!(
                    "[{:02}] {:<30} | => Java Result | Monster HP: {:3}, Block: {:2}, Intent: {}",
                    i + 1,
                    detail,
                    hp,
                    block,
                    intent
                );

                // Show power changes optionally
                if let Some(powers) = m["powers"].as_array() {
                    let p_str: Vec<String> = powers
                        .iter()
                        .map(|p| format!("{}({})", p["id"].as_str().unwrap_or("?"), p["amount"]))
                        .collect();
                    if !p_str.is_empty() {
                        println!("      Powers: {}", p_str.join(", "));
                    }
                }
            }
        }
        None => {
            println!("Combat #{} not found in {}", combat_idx, path);
        }
    }
}
