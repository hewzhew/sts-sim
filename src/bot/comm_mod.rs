use crate::state::core::{ClientInput, EngineState};

/// Translates a Rust ClientInput into the String format expected by Java's CommunicationMod over stdin/stdout.
pub fn input_to_java_command(input: &ClientInput, _state: &EngineState) -> Option<String> {
    match input {
        ClientInput::PlayCard { card_index, target } => {
            let mut cmd = format!("PLAY {}", card_index + 1); // Java expects 1-indexed cards
            if let Some(t) = target {
                // Java expects 0-indexed monster array position
                // Our entity_id is 1-indexed (player=0, first monster=1)
                // So monster_index = entity_id - 1
                let monster_idx = if *t > 0 { t - 1 } else { 0 };
                cmd.push_str(&format!(" {}", monster_idx));
            }
            Some(cmd)
        },
        ClientInput::UsePotion { potion_index, target } => {
            let mut cmd = format!("POTION USE {}", potion_index);
            if let Some(t) = target {
                cmd.push_str(&format!(" {}", t));
            }
            Some(cmd)
        },
        ClientInput::DiscardPotion(potion_index) => {
            Some(format!("POTION DISCARD {}", potion_index))
        },
        ClientInput::EndTurn => Some("END".to_string()),
        ClientInput::Proceed => Some("PROCEED".to_string()),
        ClientInput::Cancel => Some("RETURN".to_string()),
        
        // --- Choices (Event, Discovery, Map, Rewards, Shops) ---
        ClientInput::SubmitDiscoverChoice(idx) |
        ClientInput::SelectEventOption(idx) |
        ClientInput::EventChoice(idx) |
        ClientInput::SelectMapNode(idx) |
        ClientInput::ClaimReward(idx) |
        ClientInput::SelectCard(idx) |
        ClientInput::BuyCard(idx) |
        ClientInput::BuyRelic(idx) |
        ClientInput::BuyPotion(idx) |
        ClientInput::SubmitRelicChoice(idx) | // 0-indexed across the board for CHOOSE
        ClientInput::PurgeCard(idx) => {
            Some(format!("CHOOSE {}", idx))
        },

        // For complex selects where Rust atomicly expects array, picking the *first* unselected item is a naive shim 
        // to make Java advance one step and return a new intermediate frame.
        ClientInput::SubmitHandSelect(_uuids) |
        ClientInput::SubmitGridSelect(_uuids) => {
            // A more advanced map is required to find the exact index in Java's choice_list corresponding to this UUID.
            // For now, we print a placeholder or pick the first.
            // In reality, diff_driver / mapping usually bridges this, but we'll panic/error cleanly if it hits complex logic unsupported by LiveComm.
            eprintln!("WARNING: Array-based selection (Grid/Hand) is theoretically not supported in 1-pass by LiveComm. Defaulting to CHOOSE 0");
            Some(format!("CHOOSE 0"))
        },
        
        _ => {
            eprintln!("Unhandled input translation: {:?}", input);
            None
        }
    }
}
