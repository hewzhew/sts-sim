use serde_json::Value;
use std::collections::HashSet;

use crate::engine::{
    core::{is_smoke_escape_stable_boundary, tick_engine},
    pending_choices,
};
use crate::protocol::java::continuation_state_requests_on_use_potion_hooks;
use crate::runtime::combat::CombatState;
use crate::state::core::{ClientInput, EngineState, HandSelectReason, PendingChoice};
use crate::state::selection::{EngineDiagnostic, EngineDiagnosticClass, EngineDiagnosticSeverity};

use crate::diff::state_sync::snapshot_uuid;

pub fn tick_until_stable(es: &mut EngineState, cs: &mut CombatState, input: ClientInput) -> bool {
    let alive = tick_engine(es, cs, Some(input));
    if !alive {
        return false;
    }

    if *es == EngineState::CombatPlayerTurn && cs.has_pending_actions() {
        *es = EngineState::CombatProcessing;
    }

    drain_to_stable(es, cs)
}

pub fn drain_to_stable(es: &mut EngineState, cs: &mut CombatState) -> bool {
    let mut iterations = 0;
    loop {
        match es {
            EngineState::CombatPlayerTurn => break,
            EngineState::CombatProcessing if is_smoke_escape_stable_boundary(es, cs) => break,
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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum DeferredContinuationMode {
    StrictProtocol,
    LegacyFallback,
}

fn deferred_post_potion_hook_request(
    continuation_hint: Option<&Value>,
    mode: DeferredContinuationMode,
) -> Result<bool, String> {
    if let Some(requested) =
        continuation_hint.and_then(continuation_state_requests_on_use_potion_hooks)
    {
        return Ok(requested);
    }

    match mode {
        DeferredContinuationMode::StrictProtocol => Err(
            "missing protocol_meta.continuation_state capability for deferred continuation replay"
                .to_string(),
        ),
        DeferredContinuationMode::LegacyFallback => Ok(true),
    }
}

fn emit_legacy_continuation_fallback_diagnostic(cs: &mut CombatState, context: &str) {
    cs.emit_diagnostic(EngineDiagnostic {
        severity: EngineDiagnosticSeverity::Warning,
        class: EngineDiagnosticClass::Normalization,
        message: format!(
            "legacy deferred continuation fallback used for {context}; capture missing typed continuation_state truth"
        ),
    });
}

fn queue_on_use_potion_relic_hooks(cs: &mut CombatState) {
    let deferred_relic_actions = crate::content::relics::hooks::on_use_potion(cs, 0);
    cs.queue_actions(deferred_relic_actions);
}

/// Re-queue post-potion relic hooks using typed protocol continuation truth.
///
/// Mainline replay/audit code must use this strict path. It will not guess
/// missing continuation semantics from legacy captures.
pub fn queue_deferred_post_potion_actions(
    cs: &mut CombatState,
    continuation_hint: &Value,
) -> Result<bool, String> {
    if !deferred_post_potion_hook_request(
        Some(continuation_hint),
        DeferredContinuationMode::StrictProtocol,
    )? {
        return Ok(false);
    }
    queue_on_use_potion_relic_hooks(cs);
    Ok(true)
}

/// Import-boundary compatibility shim for old captures that predate
/// `protocol_meta.continuation_state`.
pub fn queue_deferred_post_potion_actions_legacy(
    cs: &mut CombatState,
    continuation_hint: Option<&Value>,
) -> bool {
    match deferred_post_potion_hook_request(
        continuation_hint,
        DeferredContinuationMode::LegacyFallback,
    ) {
        Ok(false) => false,
        Ok(true) => {
            if continuation_hint
                .and_then(continuation_state_requests_on_use_potion_hooks)
                .is_none()
            {
                emit_legacy_continuation_fallback_diagnostic(cs, "post-potion hook replay");
            }
            queue_on_use_potion_relic_hooks(cs);
            true
        }
        Err(_) => false,
    }
}

fn extract_snapshot_hand(snapshot_hint: &Value) -> Option<&Vec<Value>> {
    snapshot_hint
        .get("hand")
        .and_then(Value::as_array)
        .or_else(|| {
            snapshot_hint
                .get("combat_truth")
                .and_then(|combat| combat.get("hand"))
                .and_then(Value::as_array)
        })
        .or_else(|| {
            snapshot_hint
                .get("game_state")
                .and_then(|state| state.get("combat_truth"))
                .and_then(|combat| combat.get("hand"))
                .and_then(Value::as_array)
        })
}

fn continue_deferred_pending_choice_inner(
    pending: &PendingChoice,
    cs: &mut CombatState,
    snapshot_hint: &Value,
    mode: DeferredContinuationMode,
) -> Result<bool, String> {
    match pending {
        PendingChoice::HandSelect {
            candidate_uuids,
            min_cards,
            max_cards,
            can_cancel,
            reason: HandSelectReason::GamblingChip,
        } => {
            let snapshot_hand = extract_snapshot_hand(snapshot_hint)
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
            match mode {
                DeferredContinuationMode::StrictProtocol => {
                    queue_deferred_post_potion_actions(cs, snapshot_hint)?;
                }
                DeferredContinuationMode::LegacyFallback => {
                    queue_deferred_post_potion_actions_legacy(cs, Some(snapshot_hint));
                }
            }

            Ok(drain_to_stable(&mut es, cs))
        }
        _ => Err("unsupported deferred pending choice replay continuation".into()),
    }
}

pub fn continue_deferred_pending_choice(
    pending: &PendingChoice,
    cs: &mut CombatState,
    snapshot_hint: &Value,
) -> Result<bool, String> {
    continue_deferred_pending_choice_inner(
        pending,
        cs,
        snapshot_hint,
        DeferredContinuationMode::StrictProtocol,
    )
}

pub fn continue_deferred_pending_choice_legacy(
    pending: &PendingChoice,
    cs: &mut CombatState,
    snapshot_hint: &Value,
) -> Result<bool, String> {
    continue_deferred_pending_choice_inner(
        pending,
        cs,
        snapshot_hint,
        DeferredContinuationMode::LegacyFallback,
    )
}

#[cfg(test)]
mod tests {
    use super::{deferred_post_potion_hook_request, DeferredContinuationMode};
    use crate::protocol::java::continuation_state_requests_on_use_potion_hooks;
    use serde_json::json;

    #[test]
    fn continuation_hint_requests_post_potion_hooks_when_protocol_declares_them() {
        let root = json!({
            "protocol_meta": {
                "capabilities": { "continuation_state": true },
                "continuation_state": {
                    "kind": "card_reward_continuation",
                    "state": "resolved",
                    "deferred_hook_kinds": ["on_use_potion"]
                }
            }
        });

        assert_eq!(
            continuation_state_requests_on_use_potion_hooks(&root),
            Some(true)
        );
    }

    #[test]
    fn continuation_hint_suppresses_post_potion_hooks_when_protocol_declares_none() {
        let root = json!({
            "protocol_meta": {
                "capabilities": { "continuation_state": true },
                "continuation_state": {
                    "kind": "card_reward_continuation",
                    "state": "resolved",
                    "deferred_hook_kinds": []
                }
            }
        });

        assert_eq!(
            continuation_state_requests_on_use_potion_hooks(&root),
            Some(false)
        );
    }

    #[test]
    fn queue_deferred_post_potion_actions_respects_explicit_protocol_absence() {
        let root = json!({
            "protocol_meta": {
                "capabilities": { "continuation_state": true },
                "continuation_state": {
                    "kind": "card_reward_continuation",
                    "state": "resolved",
                    "deferred_hook_kinds": []
                }
            }
        });

        assert!(!deferred_post_potion_hook_request(
            Some(&root),
            DeferredContinuationMode::StrictProtocol
        )
        .expect("strict protocol continuation"));
    }

    #[test]
    fn strict_continuation_requires_typed_protocol_truth() {
        let root = json!({
            "game_state": {
                "combat_truth": {}
            }
        });

        let err = deferred_post_potion_hook_request(
            Some(&root),
            DeferredContinuationMode::StrictProtocol,
        )
        .expect_err("strict mode should reject missing continuation truth");
        assert!(err.contains("continuation_state"));
    }

    #[test]
    fn legacy_continuation_allows_missing_protocol_truth() {
        let root = json!({
            "game_state": {
                "combat_truth": {}
            }
        });

        assert!(deferred_post_potion_hook_request(
            Some(&root),
            DeferredContinuationMode::LegacyFallback
        )
        .expect("legacy fallback should accept missing continuation truth"));
    }
}
