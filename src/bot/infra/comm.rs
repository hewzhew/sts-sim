use crate::state::core::{ClientInput, EngineState, PendingChoice};
use crate::state::selection::{SelectionResolution, SelectionTargetRef};

/// Translates a Rust ClientInput into the String format expected by Java's CommunicationMod over stdin/stdout.
pub fn input_to_java_command(input: &ClientInput, state: &EngineState) -> Option<String> {
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
                let monster_idx = if *t > 0 { t - 1 } else { 0 };
                cmd.push_str(&format!(" {}", monster_idx));
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
        ClientInput::SubmitHandSelect(uuids) => translate_pending_uuid_selection(uuids, state),
        ClientInput::SubmitGridSelect(uuids) => translate_pending_uuid_selection(uuids, state),
        ClientInput::SubmitSelection(SelectionResolution { selected, .. }) => {
            let uuids = selected
                .iter()
                .map(|target| match target {
                    SelectionTargetRef::CardUuid(uuid) => *uuid,
                })
                .collect::<Vec<_>>();
            translate_pending_uuid_selection(&uuids, state)
        }

        _ => {
            eprintln!("Unhandled input translation: {:?}", input);
            None
        }
    }
}

fn translate_pending_uuid_selection(uuids: &[u32], state: &EngineState) -> Option<String> {
    if uuids.is_empty() {
        return Some("PROCEED".to_string());
    }

    let candidate_uuids = match state {
        EngineState::PendingChoice(PendingChoice::HandSelect {
            candidate_uuids, ..
        })
        | EngineState::PendingChoice(PendingChoice::GridSelect {
            candidate_uuids, ..
        }) => candidate_uuids,
        _ => {
            eprintln!(
                "WARNING: selection input outside PendingChoice context: {:?}",
                uuids
            );
            return Some("CHOOSE 0".to_string());
        }
    };

    if let Some(choice_index) = uuids.iter().find_map(|uuid| {
        candidate_uuids
            .iter()
            .position(|candidate| candidate == uuid)
    }) {
        Some(format!("CHOOSE {}", choice_index))
    } else {
        eprintln!(
            "WARNING: failed to map selected UUIDs {:?} to current candidate list {:?}; defaulting to CHOOSE 0",
            uuids, candidate_uuids
        );
        Some("CHOOSE 0".to_string())
    }
}
