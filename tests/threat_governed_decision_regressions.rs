use sts_simulator::bot::combat::{
    diagnose_root_search_with_runtime, SearchExactTurnMode, SearchRuntimeBudget,
};
use sts_simulator::content::cards::CardId;
use sts_simulator::content::monsters::EnemyId;
use sts_simulator::runtime::combat::CombatCard;
use sts_simulator::state::EngineState;
use sts_simulator::test_support::{blank_test_combat, planned_monster};

fn card(id: CardId, uuid: u32) -> CombatCard {
    CombatCard::new(id, uuid)
}

#[test]
fn threat_governed_diagnostics_emit_phase1_decision_fields_for_crisis_states() {
    let mut combat = blank_test_combat();
    combat.turn.energy = 1;
    combat.entities.player.current_hp = 5;
    combat
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 1));
    combat.zones.hand.push(card(CardId::Defend, 1));

    let diagnostics = diagnose_root_search_with_runtime(
        &EngineState::CombatPlayerTurn,
        &combat,
        300,
        SearchRuntimeBudget::default(),
    );

    let audit = &diagnostics.decision_audit;
    assert!(audit.get("root_pipeline").is_some());
    assert_eq!(
        audit.get("regime").and_then(|value| value.as_str()),
        Some("crisis")
    );
    assert!(audit.get("exact_turn_shadow").is_some());
    assert!(audit.get("frontier_outcome").is_some());
    assert!(audit.get("exact_turn_verdict").is_some());
    assert!(audit.get("takeover_policy").is_some());
    assert!(audit.get("decision_trace").is_some());

    let decision_trace = audit.get("decision_trace").expect("trace should exist");
    assert!(decision_trace
        .get("chosen_by")
        .and_then(|value| value.as_str())
        .is_some());
    assert!(decision_trace
        .get("frontier_proposal_class")
        .and_then(|value| value.as_str())
        .is_some());
    assert!(decision_trace
        .get("rejection_reasons")
        .and_then(|value| value.as_array())
        .is_some());
    assert!(decision_trace
        .get("screened_out")
        .and_then(|value| value.as_array())
        .is_some());
    assert!(decision_trace
        .get("why_not_others")
        .and_then(|value| value.as_array())
        .is_some());
    assert!(decision_trace
        .get("decision_outcomes")
        .and_then(|value| value.get("frontier"))
        .and_then(|value| value.get("survival"))
        .and_then(|value| value.as_str())
        .is_some());
    assert_eq!(
        audit
            .get("root_pipeline")
            .and_then(|value| value.get("proposal_count"))
            .and_then(|value| value.as_u64())
            .unwrap_or(0)
            >= audit
                .get("root_pipeline")
                .and_then(|value| value.get("screened_count"))
                .and_then(|value| value.as_u64())
                .unwrap_or(0),
        true
    );
    assert!(audit
        .get("root_pipeline")
        .and_then(|value| value.get("proposal_class_counts"))
        .and_then(|value| value.as_object())
        .is_some());
    assert!(audit
        .get("root_pipeline")
        .and_then(|value| value.get("screened_out"))
        .and_then(|value| value.as_array())
        .is_some());
    assert_eq!(
        decision_trace
            .get("screened_out")
            .and_then(|value| value.as_array())
            .map(|value| value.len()),
        audit
            .get("root_pipeline")
            .and_then(|value| value.get("screened_out"))
            .and_then(|value| value.as_array())
            .map(|value| value.len())
    );
}

#[test]
fn threat_governed_diagnostics_mark_exact_turn_verdict_bounded_when_budget_is_tiny() {
    let mut combat = blank_test_combat();
    combat.turn.energy = 3;
    combat.entities.player.current_hp = 20;
    combat
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 1));
    combat
        .zones
        .hand
        .extend([card(CardId::LimitBreak, 1), card(CardId::TwinStrike, 2)]);

    let diagnostics = diagnose_root_search_with_runtime(
        &EngineState::CombatPlayerTurn,
        &combat,
        900,
        SearchRuntimeBudget {
            exact_turn_mode: SearchExactTurnMode::Force,
            exact_turn_node_budget: 1,
            ..SearchRuntimeBudget::default()
        },
    );

    let verdict = diagnostics
        .decision_audit
        .get("exact_turn_verdict")
        .expect("verdict should exist");
    assert_eq!(
        verdict.get("confidence").and_then(|value| value.as_str()),
        Some("bounded")
    );
    assert_eq!(
        verdict.get("truncated").and_then(|value| value.as_bool()),
        Some(true)
    );
}

#[test]
fn threat_governed_diagnostics_preserve_legacy_shadow_fields() {
    let mut combat = blank_test_combat();
    combat.turn.energy = 1;
    combat.entities.player.current_hp = 24;
    combat
        .entities
        .monsters
        .push(planned_monster(EnemyId::Cultist, 1));
    combat.zones.hand.push(card(CardId::Defend, 1));

    let diagnostics = diagnose_root_search_with_runtime(
        &EngineState::CombatPlayerTurn,
        &combat,
        300,
        SearchRuntimeBudget::default(),
    );

    let shadow = diagnostics
        .decision_audit
        .get("exact_turn_shadow")
        .expect("legacy shadow should remain");
    assert!(shadow.get("frontier_chosen_move").is_some());
    assert!(shadow.get("agrees_with_frontier").is_some() || shadow.get("skipped").is_some());
}
