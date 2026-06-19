use super::*;
use crate::eval::run_control::{
    CombatAutomationActionV1, CombatAutomationTrajectoryRecordV1, RunControlSearchCombatOptions,
    RunControlSearchDefaultsCommand,
};

#[test]
fn search_defaults_command_updates_and_clears_session_defaults() {
    let mut session = RunControlSession::new(RunControlConfig::default());

    let outcome = session
        .apply_command(RunControlCommand::SearchDefaults(
            RunControlSearchDefaultsCommand::Update(RunControlSearchCombatOptions {
                max_nodes: Some(123),
                wall_ms: Some(50),
                max_hp_loss: Some(RunControlHpLossLimit::Limit(12)),
                potion_policy: Some(crate::ai::combat_search_v2::CombatSearchV2PotionPolicy::Never),
                max_potions_used: Some(0),
                ..Default::default()
            }),
        ))
        .expect("search defaults update should apply");

    assert!(outcome.message.contains("search defaults"));
    assert_eq!(session.search_max_nodes, Some(123));
    assert_eq!(session.search_wall_ms, Some(50));
    assert_eq!(session.search_max_hp_loss, Some(12));
    assert_eq!(
        session.search_potion_policy,
        Some(crate::ai::combat_search_v2::CombatSearchV2PotionPolicy::Never)
    );
    assert_eq!(session.search_max_potions_used, Some(0));

    session
        .apply_command(RunControlCommand::SearchDefaults(
            RunControlSearchDefaultsCommand::Clear,
        ))
        .expect("search defaults clear should apply");

    assert_eq!(session.search_max_nodes, None);
    assert_eq!(session.search_wall_ms, None);
    assert_eq!(session.search_max_hp_loss, None);
    assert_eq!(session.search_potion_policy, None);
    assert_eq!(session.search_max_potions_used, None);
}

#[test]
fn run_control_session_checkpoint_round_trips_exact_state() {
    let mut session = RunControlSession::new(RunControlConfig {
        seed: 590093712,
        search_max_nodes: Some(12_345),
        search_wall_ms: Some(67),
        ..RunControlConfig::default()
    });
    session
        .apply_command(RunControlCommand::DefaultCandidate)
        .expect("default Neow intro candidate should apply");

    let checkpoint = RunControlSessionCheckpointV1::from_session(&session);
    let text = serde_json::to_string(&checkpoint).expect("checkpoint should serialize");
    let loaded: RunControlSessionCheckpointV1 =
        serde_json::from_str(&text).expect("checkpoint should deserialize");
    let restored = loaded.into_session().expect("checkpoint should restore");

    assert_eq!(restored.engine_state, session.engine_state);
    assert_eq!(restored.run_state, session.run_state);
    assert_eq!(restored.active_combat, session.active_combat);
    assert_eq!(restored.decision_step, session.decision_step);
    assert_eq!(restored.search_max_nodes, session.search_max_nodes);
    assert_eq!(restored.search_wall_ms, session.search_wall_ms);
}

#[test]
fn run_control_session_checkpoint_preserves_last_combat_automation_trajectory() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.remember_combat_automation_trajectory(CombatAutomationTrajectoryRecordV1::new(
        "search_combat",
        vec![CombatAutomationActionV1 {
            step_index: 0,
            action_key: "combat/end_turn".to_string(),
            input: ClientInput::EndTurn,
            drawn_cards: Vec::new(),
            combat_after: None,
        }],
    ));

    let checkpoint = RunControlSessionCheckpointV1::from_session(&session);
    let text = serde_json::to_string(&checkpoint).expect("checkpoint should serialize");
    let loaded: RunControlSessionCheckpointV1 =
        serde_json::from_str(&text).expect("checkpoint should deserialize");
    let restored = loaded.into_session().expect("checkpoint should restore");
    let trajectory = restored
        .last_combat_automation_trajectory()
        .expect("checkpoint should preserve last automation trajectory");

    assert_eq!(trajectory.source, "search_combat");
    assert_eq!(trajectory.action_count, 1);
    assert_eq!(trajectory.actions[0].action_key, "combat/end_turn");
    assert!(
        restored
            .last_completed_combat_automation_trajectory()
            .is_none(),
        "raw automation trajectory should not masquerade as completed combat without matching sequence"
    );
}

#[test]
fn run_control_session_checkpoint_preserves_map_traversal_edges() {
    let mut session = RunControlSession::new(RunControlConfig {
        seed: 1_800_564_075,
        ..RunControlConfig::default()
    });
    let (current_x, current_y, target_x, target_y) = session
        .run_state
        .map
        .graph
        .iter()
        .enumerate()
        .flat_map(|(y, row)| {
            row.iter().filter_map(move |node| {
                node.edges
                    .iter()
                    .next()
                    .map(|edge| (node.x, y as i32, edge.dst_x, edge.dst_y))
            })
        })
        .next()
        .expect("generated map should have at least one traversable edge");
    session.run_state.map.current_x = current_x;
    session.run_state.map.current_y = current_y;
    session.run_state.event_state = None;
    session.engine_state = EngineState::MapNavigation;

    let checkpoint = RunControlSessionCheckpointV1::from_session(&session);
    let text = serde_json::to_string(&checkpoint).expect("checkpoint should serialize");
    let loaded: RunControlSessionCheckpointV1 =
        serde_json::from_str(&text).expect("checkpoint should deserialize");
    let restored = loaded.into_session().expect("checkpoint should restore");

    assert!(
        restored
            .run_state
            .map
            .can_travel_to(target_x, target_y, false),
        "checkpoint restore must preserve map edges needed for resumed route planning"
    );
}
