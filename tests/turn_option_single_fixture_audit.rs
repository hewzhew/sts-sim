use serde_json::Value;
use sts_simulator::bot::combat::{
    diagnose_root_search_with_depth_and_runtime, SearchExactTurnMode, SearchRuntimeBudget,
};
use sts_simulator::content::cards::CardId;
use sts_simulator::content::monsters::EnemyId;
use sts_simulator::runtime::combat::{CombatCard, CombatState};
use sts_simulator::state::EngineState;
use sts_simulator::test_support::{blank_test_combat, planned_monster};

fn card(id: CardId, uuid: u32) -> CombatCard {
    CombatCard::new(id, uuid)
}

fn gremlin_nob_single_fixture() -> CombatState {
    let mut combat = blank_test_combat();
    combat.turn.energy = 3;
    combat.turn.turn_count = 2;
    combat.entities.player.current_hp = 60;
    let mut nob = planned_monster(EnemyId::GremlinNob, 1);
    nob.current_hp = 70;
    nob.max_hp = 85;
    combat.entities.monsters.push(nob);
    combat.zones.hand.extend([
        card(CardId::Bash, 1),
        card(CardId::PommelStrike, 2),
        card(CardId::ShrugItOff, 3),
        card(CardId::Defend, 4),
        card(CardId::Strike, 5),
    ]);
    combat.zones.draw_pile.extend([
        card(CardId::Strike, 10),
        card(CardId::Strike, 11),
        card(CardId::Defend, 12),
        card(CardId::Defend, 13),
        card(CardId::Anger, 14),
    ]);
    combat
}

fn compact_line(line: &Value) -> String {
    line.as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|value| value.as_str())
                .collect::<Vec<_>>()
                .join(" -> ")
        })
        .unwrap_or_else(|| "-".to_string())
}

#[test]
fn single_fixture_reports_turn_option_information_volume() {
    let combat = gremlin_nob_single_fixture();
    let diagnostics = diagnose_root_search_with_depth_and_runtime(
        &EngineState::CombatPlayerTurn,
        &combat,
        4,
        0,
        SearchRuntimeBudget {
            exact_turn_mode: SearchExactTurnMode::Force,
            root_node_budget: 4_000,
            exact_turn_node_budget: 80_000,
            audit_budget: 3,
            ..SearchRuntimeBudget::default()
        },
    );

    let audit = &diagnostics.decision_audit;
    let evidence = audit
        .get("turn_option_evidence")
        .expect("turn option evidence audit should exist");
    let option_count = evidence
        .get("option_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let group_count = evidence
        .get("group_count")
        .and_then(Value::as_u64)
        .unwrap_or(0);
    let compression_ratio = evidence
        .get("compression_ratio")
        .and_then(Value::as_f64)
        .unwrap_or(1.0);
    let sample_groups = evidence
        .get("sample_groups")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let sample_options_debug = evidence
        .get("sample_options_debug")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default();
    let audit_json = serde_json::to_string(audit).expect("audit should serialize");
    let evidence_json = serde_json::to_string(evidence).expect("evidence should serialize");
    println!(
        "single_fixture=gremlin_nob_bullrush chosen={:?} chosen_by={} option_count={} group_count={} compression_ratio={:.3} exact_nodes={} exact_prunes={} exact_cache_hits={} audit_bytes={} turn_option_bytes={} sample_groups_emitted={} sample_options_debug_emitted={}",
        diagnostics.chosen_move,
        audit.get("chosen_by").and_then(Value::as_str).unwrap_or("?"),
        option_count,
        group_count,
        compression_ratio,
        evidence
            .get("exact_explored_nodes")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        evidence
            .get("exact_dominance_prunes")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        evidence
            .get("exact_cache_hits")
            .and_then(Value::as_u64)
            .unwrap_or(0),
        audit_json.len(),
        evidence_json.len(),
        sample_groups.len(),
        sample_options_debug.len(),
    );
    for (idx, group) in sample_groups.iter().enumerate() {
        println!(
            "sample_group_{} id={:?} size={:?} first={:?} line=[{}] effect={}",
            idx + 1,
            group.get("group_id").and_then(Value::as_u64),
            group.get("size").and_then(Value::as_u64),
            group
                .get("representative_first_input")
                .and_then(Value::as_str),
            compact_line(group.get("representative_line").unwrap_or(&Value::Null)),
            group.get("effect_summary").unwrap_or(&Value::Null),
        );
    }

    assert_eq!(
        audit.get("chosen_by").and_then(Value::as_str),
        Some("legacy_frontier_fallback")
    );
    assert_eq!(
        evidence.get("decision_role").and_then(Value::as_str),
        Some("evidence_only")
    );
    assert!(
        option_count > 0,
        "fixture should produce at least one turn option"
    );
    assert!(
        group_count > 0,
        "fixture should produce at least one turn plan group"
    );
    assert!(
        group_count < option_count,
        "effect grouping should compress raw exact-turn options: groups={group_count}, options={option_count}"
    );
    assert!(
        sample_groups
            .first()
            .and_then(|group| group.get("effect_summary"))
            .is_some(),
        "group audit should expose compact effect summaries"
    );
    assert!(
        sample_options_debug.len() <= 3,
        "raw option debug output should stay capped"
    );
    assert!(
        audit_json.len() < 50_000,
        "single fixture audit is already too large to be useful: {} bytes",
        audit_json.len()
    );
}
