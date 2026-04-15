use serde_json::Value;
use std::collections::HashSet;

use crate::runtime::combat::CombatState;
use crate::engine::{core::tick_engine, pending_choices};
use crate::state::core::{ClientInput, EngineState, HandSelectReason, PendingChoice};
use crate::state::selection::{EngineDiagnostic, EngineDiagnosticClass, EngineDiagnosticSeverity};

use crate::diff::state_sync::snapshot_uuid;

pub fn tick_until_stable(es: &mut EngineState, cs: &mut CombatState, input: ClientInput) -> bool {
    let alive = tick_engine(es, cs, Some(input));
    if !alive {
        return false;
    }

    if *es == EngineState::CombatPlayerTurn && !cs.engine.action_queue.is_empty() {
        *es = EngineState::CombatProcessing;
    }

    drain_to_stable(es, cs)
}

pub fn drain_to_stable(es: &mut EngineState, cs: &mut CombatState) -> bool {
    let mut iterations = 0;
    loop {
        match es {
            EngineState::CombatPlayerTurn => break,
            EngineState::CombatProcessing => {}
            EngineState::PendingChoice(_) => break,
            EngineState::GameOver(_) => return false,
            _ => break,
        }
        let alive = tick_engine(es, cs, None);
        if !alive {
            return false;
        }
        iterations += 1;
        if iterations > 1000 {
            cs.emit_diagnostic(EngineDiagnostic {
                severity: EngineDiagnosticSeverity::Warning,
                class: EngineDiagnosticClass::Suspicious,
                message: "tick loop exceeded 1000 iterations".to_string(),
            });
            break;
        }
    }
    true
}

pub fn continue_deferred_pending_choice(
    pending: &PendingChoice,
    cs: &mut CombatState,
    snapshot_hint: &Value,
) -> Result<bool, String> {
    match pending {
        PendingChoice::HandSelect {
            candidate_uuids,
            min_cards,
            max_cards,
            can_cancel,
            reason: HandSelectReason::GamblingChip,
        } => {
            let snapshot_hand = snapshot_hint
                .get("hand")
                .and_then(|v| v.as_array())
                .ok_or("snapshot_hint.zones.hand missing for deferred GamblingChip replay")?;

            let remaining_uuids: HashSet<u32> = snapshot_hand
                .iter()
                .enumerate()
                .map(|(i, card)| snapshot_uuid(&card["uuid"], i as u32 + 1000))
                .collect();

            let selected: Vec<u32> = candidate_uuids
                .iter()
                .copied()
                .filter(|uuid| !remaining_uuids.contains(uuid))
                .collect();

            if selected.len() < *min_cards as usize || selected.len() > *max_cards as usize {
                return Err(format!(
                    "deferred GamblingChip inference failed: selected={} min={} max={} current_hand={:?}",
                    selected.len(),
                    min_cards,
                    max_cards,
                    cs.zones.hand
                        .iter()
                        .map(|c| (c.id, c.uuid))
                        .collect::<Vec<_>>()
                ));
            }

            let mut es = EngineState::PendingChoice(pending.clone());
            let input = if selected.is_empty() && *can_cancel {
                ClientInput::Cancel
            } else {
                ClientInput::SubmitHandSelect(selected)
            };
            pending_choices::handle_hand_select(
                &mut es,
                cs,
                candidate_uuids,
                *max_cards as usize,
                *min_cards == *max_cards,
                *can_cancel,
                HandSelectReason::GamblingChip,
                input,
            )
            .map_err(|err| err.to_string())?;

            // Potion-triggered Gambling Chip style selections can leave post-potion relic
            // actions (Toy Ornithopter heal, etc.) queued behind the pending choice in Java.
            // Replay snapshot sync drops the transient action queue, so re-queue those hooks
            // here before draining the deferred continuation.
            let deferred_relic_actions = crate::content::relics::hooks::on_use_potion(cs, 0);
            crate::engine::core::queue_actions(&mut cs.engine.action_queue, deferred_relic_actions);

            Ok(drain_to_stable(&mut es, cs))
        }
        _ => Err("unsupported deferred pending choice replay continuation".into()),
    }
}
