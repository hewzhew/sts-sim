use std::path::{Path, PathBuf};

use crate::combat::CombatState;
use crate::diff::protocol::parser::{parse_replay, ReplayAction};
use crate::diff::replay::replay_support::{continue_deferred_pending_choice, tick_until_stable};
use crate::diff::state_sync::{build_combat_state, sync_state};
use crate::state::core::{ClientInput, EngineState, PendingChoice};

use super::signature::{command_string, signature_from_transition, ObservedInteractionRecord};

pub fn replay_records_from_path(path: &Path) -> Vec<ObservedInteractionRecord> {
    let replay = parse_replay(path.to_string_lossy().as_ref());
    let mut records = Vec::new();

    for combat in replay.combats {
        let mut combat_state = build_combat_state(&combat.start_snapshot, &combat.relics_val);
        let mut prev_snapshot = combat.start_snapshot.clone();
        let mut carried_pending: Option<PendingChoice> = None;

        for (action_idx, action) in combat.actions.iter().enumerate() {
            sync_state(&mut combat_state, &prev_snapshot);

            if action.action_type == "potion" {
                if let Some(pending) = carried_pending.take() {
                    let _ = continue_deferred_pending_choice(
                        &pending,
                        &mut combat_state,
                        &action.result,
                    );
                }
            }

            if action.action_type == "sync" {
                prev_snapshot = action.result.clone();
                continue;
            }

            if let Some(input) = replay_action_to_input(action, &combat_state) {
                let before_engine = EngineState::CombatPlayerTurn;
                let before_state = combat_state.clone();
                let mut after_engine = EngineState::CombatPlayerTurn;
                let _alive = tick_until_stable(&mut after_engine, &mut combat_state, input.clone());
                let signature = signature_from_transition(
                    &before_engine,
                    &before_state,
                    &input,
                    &after_engine,
                    &combat_state,
                );
                records.push(ObservedInteractionRecord {
                    observed_from: "replay".to_string(),
                    source_file: path.to_string_lossy().into_owned(),
                    combat_idx: Some(combat.combat_idx),
                    action_idx: Some(action_idx + 1),
                    command: command_string(&input),
                    signature_key: signature.canonical_key(),
                    source_combo_key: signature.source_combo_key(),
                    signature,
                });
                carried_pending = match &after_engine {
                    EngineState::PendingChoice(choice) => Some(choice.clone()),
                    _ => None,
                };
            }

            prev_snapshot = action.result.clone();
        }
    }

    records
}

pub fn load_live_comm_records(path: &Path) -> Vec<ObservedInteractionRecord> {
    if !path.exists() {
        return Vec::new();
    }
    let Ok(content) = std::fs::read_to_string(path) else {
        return Vec::new();
    };
    content
        .lines()
        .filter_map(|line| serde_json::from_str::<ObservedInteractionRecord>(line).ok())
        .collect()
}

pub fn default_replay_inputs(manifest_dir: &Path) -> Vec<PathBuf> {
    let mut inputs = Vec::new();
    let replay_short = manifest_dir.join("tools/replay_short.jsonl");
    if replay_short.exists() {
        inputs.push(replay_short);
    }
    let replays_dir = manifest_dir.join("tools/replays");
    if replays_dir.exists() {
        let mut replay_files: Vec<_> = std::fs::read_dir(replays_dir)
            .into_iter()
            .flatten()
            .filter_map(|entry| entry.ok())
            .map(|entry| entry.path())
            .filter(|path| path.extension().and_then(|ext| ext.to_str()) == Some("jsonl"))
            .collect();
        replay_files.sort();
        inputs.extend(replay_files);
    }
    inputs
}

fn replay_action_to_input(action: &ReplayAction, combat: &CombatState) -> Option<ClientInput> {
    match action.action_type.as_str() {
        "play" => Some(ClientInput::PlayCard {
            card_index: action.card_index?,
            target: action
                .target
                .and_then(|idx| combat.entities.monsters.get(idx).map(|m| m.id)),
        }),
        "potion" => {
            let cmd = action.command.as_deref()?;
            let parts: Vec<&str> = cmd.split_whitespace().collect();
            if parts.len() >= 3 && parts[0] == "potion" && parts[1] == "use" {
                let slot = parts[2].parse::<usize>().ok()?;
                let target = parts
                    .get(3)
                    .and_then(|s| s.parse::<usize>().ok())
                    .and_then(|idx| combat.entities.monsters.get(idx).map(|m| m.id));
                Some(ClientInput::UsePotion {
                    potion_index: slot,
                    target,
                })
            } else {
                None
            }
        }
        "end_turn" => Some(ClientInput::EndTurn),
        _ => None,
    }
}
