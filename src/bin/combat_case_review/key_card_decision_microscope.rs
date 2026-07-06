use sts_simulator::eval::combat_case::CombatCase;

use super::key_card_lifecycle::key_card_targets;
use super::options::ReviewOptions;

#[path = "key_card_decision_microscope/digest.rs"]
mod digest;
#[path = "key_card_decision_microscope/execution.rs"]
mod execution;
#[path = "key_card_decision_microscope/types.rs"]
mod types;

pub(super) use types::KeyCardDecisionMicroscopeProbe;

use execution::run_variant;

pub(super) fn run_key_card_decision_microscope_probe(
    options: &ReviewOptions,
    case: &CombatCase,
) -> Option<KeyCardDecisionMicroscopeProbe> {
    if !options.key_card_decision_microscope {
        return None;
    }
    let targets = key_card_targets(&case.position.combat);
    if targets.is_empty() {
        return Some(KeyCardDecisionMicroscopeProbe {
            schema: "key_card_decision_microscope_probe_v0",
            contract: "diagnostic_only_move_key_card_to_opening_hand_then_explain_root_decision",
            skipped_reason: Some("no_key_cards"),
            variants: Vec::new(),
        });
    }

    let variants = targets
        .into_iter()
        .map(|target| run_variant(options, case, &target.card, target.reason))
        .collect();

    Some(KeyCardDecisionMicroscopeProbe {
        schema: "key_card_decision_microscope_probe_v0",
        contract: "diagnostic_only_move_key_card_to_opening_hand_then_explain_root_decision",
        skipped_reason: None,
        variants,
    })
}
