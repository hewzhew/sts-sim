use crate::runtime::combat::CombatState;
use crate::state::core::{ClientInput, PendingChoice};
use crate::state::EngineState;
use serde_json::{json, Value};

pub(crate) fn build_decision_audit(
    engine: &EngineState,
    combat: &CombatState,
    chosen_input: &ClientInput,
) -> Value {
    let has_safe_line = super::root_policy::StatePressureFeatures::from_combat(combat).value_unblocked
        <= 0;
    json!({
        "engine_context": engine_context_label(engine),
        "chosen_input": format!("{chosen_input:?}"),
        "root_policy": super::root_policy::decision_audit_json(combat, chosen_input, has_safe_line),
        "hand_select": super::hand_select::decision_audit_json(engine, combat, chosen_input),
        "tactical_bonus": super::tactical_bonus::decision_audit_json(combat, chosen_input),
    })
}

fn engine_context_label(engine: &EngineState) -> &'static str {
    match engine {
        EngineState::CombatPlayerTurn => "combat_player_turn",
        EngineState::EventCombat(_) => "event_combat",
        EngineState::PendingChoice(PendingChoice::HandSelect { .. }) => "pending_hand_select",
        EngineState::PendingChoice(PendingChoice::GridSelect { .. }) => "pending_grid_select",
        EngineState::PendingChoice(PendingChoice::DiscoverySelect(_)) => "pending_discovery",
        EngineState::PendingChoice(PendingChoice::CardRewardSelect { .. }) => {
            "pending_card_reward_select"
        }
        EngineState::PendingChoice(PendingChoice::StanceChoice) => "pending_stance_choice",
        EngineState::PendingChoice(_) => "pending_choice",
        _ => "other_engine_state",
    }
}
