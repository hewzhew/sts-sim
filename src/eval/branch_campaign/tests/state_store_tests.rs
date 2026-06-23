use super::*;

#[test]
fn branch_state_store_tracks_snapshot_hits_and_retention() {
    let mut store = super::state_graph::BranchStateStoreV1::new();
    let mut kept = test_campaign_branch("kept", 6, 80);
    kept.commands = vec!["rp 0".to_string(), "rp 1".to_string()];
    let mut dropped = test_campaign_branch("dropped", 6, 78);
    dropped.commands = vec!["rp 2".to_string()];
    let kept_session = RunControlSession::new(RunControlConfig::default());
    let dropped_session = RunControlSession::new(RunControlConfig::default());

    store.insert_session(kept.commands.clone(), kept_session);
    store.insert_session(dropped.commands.clone(), dropped_session);

    assert!(store.replay_start_for_commands(&kept.commands).is_some());
    assert!(store
        .replay_start_for_commands(&["missing".to_string()])
        .is_none());
    store.retain_for_branches(&[kept.clone()], &[], &[], &[]);

    let summary = store.summary();
    assert_eq!(summary.sessions, 1);
    assert_eq!(summary.nodes, 1);
    assert_eq!(summary.lookup_hits, 1);
    assert_eq!(summary.lookup_misses, 1);
    assert!(store.contains_commands(&kept.commands));
    assert!(!store.contains_commands(&dropped.commands));
}

#[test]
fn branch_state_store_records_child_parent_link_and_command_delta() {
    let mut store = super::state_graph::BranchStateStoreV1::new();
    let parent_commands = vec!["rp 0".to_string()];
    let child_commands = vec!["rp 0".to_string(), "go 2".to_string(), "rp 1".to_string()];

    store.insert_session(
        parent_commands.clone(),
        RunControlSession::new(RunControlConfig::default()),
    );
    store.insert_child_session(
        &parent_commands,
        child_commands.clone(),
        RunControlSession::new(RunControlConfig::default()),
    );

    let parent_id = store
        .node_id_for_commands(&parent_commands)
        .expect("parent node should exist");
    let child = store
        .node_for_commands(&child_commands)
        .expect("child node should exist");

    assert_eq!(child.parent_id(), Some(parent_id));
    assert_eq!(
        child.added_commands(),
        &["go 2".to_string(), "rp 1".to_string()]
    );
    assert_eq!(store.summary().linked_nodes, 1);
}

#[test]
fn branch_state_store_retain_keeps_child_ancestor_nodes_without_parent_session() {
    let mut store = super::state_graph::BranchStateStoreV1::new();
    let parent_commands = vec!["rp 0".to_string()];
    let child_commands = vec!["rp 0".to_string(), "go 2".to_string()];
    let mut child_branch = test_campaign_branch("child", 6, 80);
    child_branch.commands = child_commands.clone();

    store.insert_session(
        parent_commands.clone(),
        RunControlSession::new(RunControlConfig::default()),
    );
    store.insert_child_session(
        &parent_commands,
        child_commands.clone(),
        RunControlSession::new(RunControlConfig::default()),
    );
    store.retain_for_branches(&[child_branch], &[], &[], &[]);

    let summary = store.summary();
    assert_eq!(summary.sessions, 1);
    assert_eq!(summary.nodes, 2);
    assert_eq!(summary.linked_nodes, 1);
    assert!(!store.contains_commands(&parent_commands));
    assert!(store.node_id_for_commands(&parent_commands).is_some());
    assert!(store.node_id_for_commands(&child_commands).is_some());
}

#[test]
fn branch_state_store_retain_prunes_decision_parent_exact_session_when_not_needed() {
    let mut store = super::state_graph::BranchStateStoreV1::new();
    let parent_commands = vec!["rp 0".to_string()];
    let child_commands = vec!["rp 0".to_string(), "go 2".to_string()];
    let mut child_branch = test_campaign_branch("child", 6, 80);
    child_branch.commands = child_commands.clone();
    let mut anchors = BTreeSet::new();
    anchors.insert(parent_commands.clone());

    store.insert_session(
        parent_commands.clone(),
        RunControlSession::new(RunControlConfig::default()),
    );
    store.insert_child_session(
        &parent_commands,
        child_commands.clone(),
        RunControlSession::new(RunControlConfig::default()),
    );
    store.retain_for_branches_with_session_policy_and_anchors(
        &[child_branch],
        &[],
        &[],
        &[],
        &anchors,
        super::state_graph::BranchStateSessionRetentionPolicyV1 {
            max_frozen_exact_sessions: 0,
            max_stuck_exact_sessions: 0,
            max_abandoned_exact_sessions: 0,
            max_anchor_exact_sessions: 0,
            max_suffix_commands_without_session: usize::MAX,
        },
    );

    let summary = store.summary();
    assert_eq!(summary.sessions, 1);
    assert!(!store.contains_commands(&parent_commands));
    assert!(store.contains_commands(&child_commands));
    assert!(store.node_id_for_commands(&parent_commands).is_some());
}

#[test]
fn branch_state_store_retain_keeps_decision_parent_anchor_session_for_long_suffix() {
    let mut store = super::state_graph::BranchStateStoreV1::new();
    let parent_commands = vec!["rp 0".to_string()];
    let child_commands = vec!["rp 0".to_string(), "go 2".to_string()];
    let mut child_branch = test_campaign_branch("child", 6, 80);
    child_branch.commands = child_commands.clone();
    let mut anchors = BTreeSet::new();
    anchors.insert(parent_commands.clone());

    store.insert_session(
        parent_commands.clone(),
        RunControlSession::new(RunControlConfig::default()),
    );
    store.insert_child_session(
        &parent_commands,
        child_commands.clone(),
        RunControlSession::new(RunControlConfig::default()),
    );
    store.retain_for_branches_with_session_policy_and_anchors(
        &[child_branch],
        &[],
        &[],
        &[],
        &anchors,
        super::state_graph::BranchStateSessionRetentionPolicyV1 {
            max_frozen_exact_sessions: 0,
            max_stuck_exact_sessions: 0,
            max_abandoned_exact_sessions: 0,
            max_anchor_exact_sessions: 0,
            max_suffix_commands_without_session: 0,
        },
    );

    let summary = store.summary();
    assert_eq!(summary.sessions, 2);
    assert!(store.contains_commands(&parent_commands));
    assert!(store.contains_commands(&child_commands));
}

#[test]
fn branch_state_store_replays_from_longest_session_prefix_without_child_node() {
    let mut store = super::state_graph::BranchStateStoreV1::new();
    let parent_commands = vec!["__route_decision:0:go_1".to_string()];
    let target_commands = vec!["__route_decision:0:go_1".to_string(), "go 3".to_string()];

    store.insert_session(
        parent_commands,
        RunControlSession::new(RunControlConfig::default()),
    );

    let replay = store
        .replay_start_for_commands(&target_commands)
        .expect("synthetic route anchor should replay from exact parent session");
    assert_eq!(
        replay.source,
        super::state_graph::BranchStateReplayStartSourceV1::Ancestor
    );
    assert_eq!(replay.suffix_commands, vec!["go 3".to_string()]);
}

#[test]
fn branch_state_store_prefers_longest_session_prefix_over_shorter_node_ancestor() {
    let mut store = super::state_graph::BranchStateStoreV1::new();
    let root_commands = Vec::<String>::new();
    let parent_commands = vec!["__route_decision:0:go_1".to_string()];
    let target_commands = vec!["__route_decision:0:go_1".to_string(), "go 3".to_string()];

    store.insert_session(
        root_commands.clone(),
        RunControlSession::new(RunControlConfig::default()),
    );
    store.insert_session(
        parent_commands.clone(),
        RunControlSession::new(RunControlConfig::default()),
    );
    store
        .restore_checkpoint_nodes(&[
            super::model::BranchCampaignCheckpointNodeV1 {
                node_id: 0,
                parent_id: None,
                commands: root_commands,
                added_commands: Vec::new(),
            },
            super::model::BranchCampaignCheckpointNodeV1 {
                node_id: 1,
                parent_id: Some(0),
                commands: target_commands.clone(),
                added_commands: target_commands.clone(),
            },
        ])
        .expect("checkpoint nodes should restore");

    let replay = store
        .replay_start_for_commands(&target_commands)
        .expect("route target should prefer synthetic parent session");
    assert_eq!(
        replay.source,
        super::state_graph::BranchStateReplayStartSourceV1::Ancestor
    );
    assert_eq!(replay.suffix_commands, vec!["go 3".to_string()]);
}

#[test]
fn branch_state_store_does_not_self_parent_existing_node() {
    let mut store = super::state_graph::BranchStateStoreV1::new();
    let commands = vec!["rp 0".to_string()];

    store.insert_session(
        commands.clone(),
        RunControlSession::new(RunControlConfig::default()),
    );
    store.insert_child_session(
        &commands,
        commands.clone(),
        RunControlSession::new(RunControlConfig::default()),
    );

    let node = store
        .node_for_commands(&commands)
        .expect("node should still exist");
    assert_eq!(node.parent_id(), None);
    assert!(store
        .checkpoint_nodes()
        .iter()
        .all(|node| node.parent_id != Some(node.node_id)));
}

#[test]
fn branch_state_store_repairs_legacy_self_parent_checkpoint_nodes() {
    let mut store = super::state_graph::BranchStateStoreV1::new();

    store
        .restore_checkpoint_nodes(&[super::model::BranchCampaignCheckpointNodeV1 {
            node_id: 0,
            parent_id: Some(0),
            commands: vec!["rp 0".to_string()],
            added_commands: Vec::new(),
        }])
        .expect("legacy self-parent node should be repaired");

    let node = store
        .node_for_commands(&["rp 0".to_string()])
        .expect("node should restore");
    assert_eq!(node.parent_id(), None);
}

#[test]
fn branch_state_store_session_policy_prunes_extra_frozen_exact_sessions_only() {
    let mut store = super::state_graph::BranchStateStoreV1::new();
    let mut active = test_campaign_branch("active", 4, 80);
    active.commands = vec!["rp 0".to_string()];
    let mut frozen_kept = test_campaign_branch("frozen-kept", 4, 80);
    frozen_kept.commands = vec!["rp 1".to_string()];
    let mut frozen_pruned = test_campaign_branch("frozen-pruned", 4, 80);
    frozen_pruned.commands = vec!["rp 2".to_string()];

    for branch in [&active, &frozen_kept, &frozen_pruned] {
        store.insert_session(
            branch.commands.clone(),
            RunControlSession::new(RunControlConfig::default()),
        );
    }

    store.retain_for_branches_with_session_policy(
        &[active.clone()],
        &[frozen_kept.clone(), frozen_pruned.clone()],
        &[],
        &[],
        super::state_graph::BranchStateSessionRetentionPolicyV1 {
            max_frozen_exact_sessions: 1,
            max_stuck_exact_sessions: 0,
            max_abandoned_exact_sessions: 0,
            max_anchor_exact_sessions: 0,
            max_suffix_commands_without_session: usize::MAX,
        },
    );

    assert!(store.contains_commands(&active.commands));
    assert!(store.contains_commands(&frozen_kept.commands));
    assert!(!store.contains_commands(&frozen_pruned.commands));
    assert!(store
        .node_id_for_commands(&frozen_pruned.commands)
        .is_some());
}

#[test]
fn branch_state_store_session_policy_keeps_long_suffix_frozen_anchor() {
    let mut store = super::state_graph::BranchStateStoreV1::new();
    let mut active = test_campaign_branch("active", 4, 80);
    active.commands = vec!["rp 0".to_string()];
    let mut frozen = test_campaign_branch("frozen", 6, 80);
    frozen.commands = vec![
        "rp 0".to_string(),
        "go 2".to_string(),
        "rp 1".to_string(),
        "smith-3".to_string(),
    ];

    store.insert_session(
        active.commands.clone(),
        RunControlSession::new(RunControlConfig::default()),
    );
    store.insert_child_session(
        &active.commands,
        frozen.commands.clone(),
        RunControlSession::new(RunControlConfig::default()),
    );

    store.retain_for_branches_with_session_policy(
        &[active.clone()],
        &[frozen.clone()],
        &[],
        &[],
        super::state_graph::BranchStateSessionRetentionPolicyV1 {
            max_frozen_exact_sessions: 0,
            max_stuck_exact_sessions: 0,
            max_abandoned_exact_sessions: 0,
            max_anchor_exact_sessions: 0,
            max_suffix_commands_without_session: 2,
        },
    );

    let replay_start = store
        .replay_start_for_commands(&frozen.commands)
        .expect("long suffix frozen branch should keep exact session as an anchor");

    assert!(store.contains_commands(&frozen.commands));
    assert_eq!(replay_start.suffix_commands, Vec::<String>::new());
}

#[test]
fn campaign_session_retention_policy_keeps_all_frozen_exact_sessions() {
    let config = BranchCampaignConfigV1 {
        max_active: 3,
        max_frozen: 11,
        ..BranchCampaignConfigV1::default()
    };

    let policy = campaign_state_session_retention_policy_v1(&config);

    assert_eq!(policy.max_frozen_exact_sessions, 11);
    assert_eq!(policy.max_stuck_exact_sessions, 3);
    assert_eq!(policy.max_abandoned_exact_sessions, 0);
    assert_eq!(policy.max_anchor_exact_sessions, 14);
    assert_eq!(policy.max_suffix_commands_without_session, 6);
}

#[test]
fn branch_state_store_exports_checkpoint_node_records() {
    let mut store = super::state_graph::BranchStateStoreV1::new();
    let parent_commands = vec!["rp 0".to_string()];
    let child_commands = vec!["rp 0".to_string(), "go 2".to_string()];

    store.insert_session(
        parent_commands.clone(),
        RunControlSession::new(RunControlConfig::default()),
    );
    store.insert_child_session(
        &parent_commands,
        child_commands.clone(),
        RunControlSession::new(RunControlConfig::default()),
    );

    let nodes = store.checkpoint_nodes();

    assert_eq!(nodes.len(), 2);
    assert_eq!(nodes[0].node_id, 0);
    assert_eq!(nodes[0].parent_id, None);
    assert_eq!(nodes[0].commands, parent_commands);
    assert_eq!(nodes[1].node_id, 1);
    assert_eq!(nodes[1].parent_id, Some(0));
    assert_eq!(nodes[1].commands, child_commands);
    assert_eq!(nodes[1].added_commands, vec!["go 2".to_string()]);
}

#[test]
fn branch_state_store_restores_checkpoint_node_records_before_sessions() {
    let parent_commands = vec!["rp 0".to_string()];
    let child_commands = vec!["rp 0".to_string(), "go 2".to_string()];
    let mut store = super::state_graph::BranchStateStoreV1::new();

    store
        .restore_checkpoint_nodes(&[
            super::model::BranchCampaignCheckpointNodeV1 {
                node_id: 0,
                parent_id: None,
                commands: parent_commands.clone(),
                added_commands: parent_commands.clone(),
            },
            super::model::BranchCampaignCheckpointNodeV1 {
                node_id: 1,
                parent_id: Some(0),
                commands: child_commands.clone(),
                added_commands: vec!["go 2".to_string()],
            },
        ])
        .expect("checkpoint node graph should restore");
    store.insert_session(
        child_commands.clone(),
        RunControlSession::new(RunControlConfig::default()),
    );

    let child = store
        .node_for_commands(&child_commands)
        .expect("child node should exist");
    let parent_id = store
        .node_id_for_commands(&parent_commands)
        .expect("parent node should exist");

    assert_eq!(child.parent_id(), Some(parent_id));
    assert_eq!(store.summary().sessions, 1);
    assert_eq!(store.summary().nodes, 2);
    assert_eq!(store.summary().linked_nodes, 1);
}

#[test]
fn branch_state_store_replays_from_nearest_saved_ancestor() {
    let parent_commands = vec!["rp 0".to_string()];
    let child_commands = vec!["rp 0".to_string(), "go 2".to_string(), "rp 1".to_string()];
    let mut store = super::state_graph::BranchStateStoreV1::new();

    store
        .restore_checkpoint_nodes(&[
            super::model::BranchCampaignCheckpointNodeV1 {
                node_id: 0,
                parent_id: None,
                commands: parent_commands.clone(),
                added_commands: parent_commands.clone(),
            },
            super::model::BranchCampaignCheckpointNodeV1 {
                node_id: 1,
                parent_id: Some(0),
                commands: child_commands.clone(),
                added_commands: vec!["go 2".to_string(), "rp 1".to_string()],
            },
        ])
        .expect("checkpoint node graph should restore");
    store.insert_session(
        parent_commands,
        RunControlSession::new(RunControlConfig::default()),
    );

    let replay_start = store
        .replay_start_for_commands(&child_commands)
        .expect("child should replay from saved parent state");

    assert_eq!(
        replay_start.source,
        super::state_graph::BranchStateReplayStartSourceV1::Ancestor
    );
    assert_eq!(
        replay_start.suffix_commands,
        vec!["go 2".to_string(), "rp 1".to_string()]
    );
    assert_eq!(store.summary().lookup_hits, 1);
    assert_eq!(store.summary().lookup_misses, 0);
}

#[test]
fn branch_state_store_summary_tracks_replay_start_sources_and_suffixes() {
    let exact_commands = vec!["rp 0".to_string()];
    let ancestor_commands = vec!["rp 0".to_string(), "go 2".to_string(), "rp 1".to_string()];
    let mut store = super::state_graph::BranchStateStoreV1::new();
    store.insert_session(
        exact_commands.clone(),
        RunControlSession::new(RunControlConfig::default()),
    );
    store.insert_child_session(
        &exact_commands,
        ancestor_commands.clone(),
        RunControlSession::new(RunControlConfig::default()),
    );
    let mut active = test_campaign_branch("active", 4, 80);
    active.commands = exact_commands.clone();
    let mut frozen = test_campaign_branch("frozen", 6, 80);
    frozen.commands = ancestor_commands.clone();
    store.retain_for_branches_with_session_policy(
        &[active],
        &[frozen],
        &[],
        &[],
        super::state_graph::BranchStateSessionRetentionPolicyV1 {
            max_frozen_exact_sessions: 0,
            max_stuck_exact_sessions: 0,
            max_abandoned_exact_sessions: 0,
            max_anchor_exact_sessions: 0,
            max_suffix_commands_without_session: usize::MAX,
        },
    );

    assert!(store.replay_start_for_commands(&exact_commands).is_some());
    assert!(store
        .replay_start_for_commands(&ancestor_commands)
        .is_some());
    assert!(store
        .replay_start_for_commands(&["missing".to_string()])
        .is_none());

    let summary = store.summary();
    assert_eq!(summary.replay_exact_hits, 1);
    assert_eq!(summary.replay_ancestor_hits, 1);
    assert_eq!(summary.replay_misses, 1);
    assert_eq!(summary.replay_suffix_commands_sum, 2);
    assert_eq!(summary.replay_suffix_commands_max, 2);
    assert_eq!(summary.sessions_pruned, 1);
}

#[test]
fn branch_state_store_summary_tracks_decision_coordinate_usage() {
    let decision_commands = vec![
        "__decision_parent:card_reward:floor=3:index=0".to_string(),
        "rp 1".to_string(),
    ];
    let route_commands = vec![
        "__route_decision:map:act=1:floor=4".to_string(),
        "go 2".to_string(),
    ];
    let ordinary_commands = vec!["rp 0".to_string()];
    let mut store = super::state_graph::BranchStateStoreV1::new();

    store.insert_session(
        decision_commands.clone(),
        RunControlSession::new(RunControlConfig::default()),
    );
    store.insert_session(
        ordinary_commands,
        RunControlSession::new(RunControlConfig::default()),
    );
    store.insert_child_session(
        &decision_commands,
        route_commands,
        RunControlSession::new(RunControlConfig::default()),
    );

    let summary = store.summary();
    assert_eq!(summary.decision_coordinate_sessions, 2);
    assert_eq!(summary.route_decision_coordinate_sessions, 1);
    assert_eq!(summary.decision_coordinate_nodes, 2);
    assert_eq!(summary.route_decision_coordinate_nodes, 1);
}

#[test]
fn campaign_parent_batch_can_force_ancestor_replay_after_exact_session_pruned() {
    let mut parent_session = RunControlSession::new(RunControlConfig::default());
    let mut reward = RewardState::new();
    reward.pending_card_choice = Some(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::Cleave, 0),
    ]);
    parent_session.engine_state = EngineState::RewardScreen(reward);

    let parent_commands = Vec::<String>::new();
    let child_commands = vec!["rp 0".to_string()];
    let mut parent_branch = test_campaign_branch("parent-anchor", 1, 80);
    parent_branch.commands = parent_commands.clone();
    let mut child_branch = test_campaign_branch("child-replay", 1, 80);
    child_branch.commands = child_commands.clone();

    let mut state_store = super::state_graph::BranchStateStoreV1::new();
    state_store.insert_session(parent_commands.clone(), parent_session);
    state_store.insert_child_session(
        &parent_commands,
        child_commands.clone(),
        RunControlSession::new(RunControlConfig::default()),
    );
    state_store.retain_for_branches_with_session_policy(
        &[parent_branch],
        &[child_branch.clone()],
        &[],
        &[],
        super::state_graph::BranchStateSessionRetentionPolicyV1 {
            max_frozen_exact_sessions: 0,
            max_stuck_exact_sessions: 0,
            max_abandoned_exact_sessions: 0,
            max_anchor_exact_sessions: 0,
            max_suffix_commands_without_session: usize::MAX,
        },
    );
    assert!(!state_store.contains_commands(&child_commands));

    let mut retry_ledger = BranchCampaignCombatRetryLedgerStateV1::default();
    let mut progress_events = Vec::new();
    let config = BranchCampaignConfigV1 {
        round_depth: 0,
        max_branches_per_active: 1,
        experiment_wall_ms: Some(1_000),
        search_wall_ms: Some(10),
        search_max_nodes: Some(100),
        ..BranchCampaignConfigV1::default()
    };

    let batch = run_campaign_parent_batch_v1(
        &config,
        &[child_branch],
        &mut state_store,
        &mut retry_ledger,
        1,
        false,
        &mut |event| progress_events.push(event),
    )
    .expect("forced ancestor replay batch should run");

    let summary = state_store.summary();
    assert_eq!(summary.replay_exact_hits, 0);
    assert_eq!(summary.replay_ancestor_hits, 1);
    assert_eq!(summary.replay_suffix_commands_sum, 1);
    assert_eq!(summary.replay_suffix_commands_max, 1);
    assert_eq!(summary.replay_misses, 0);
    assert!(!batch.candidates.is_empty());
}

#[test]
fn campaign_checkpoint_writes_v2_state_graph_nodes() {
    let config = BranchCampaignConfigV1::default();
    let parent_commands = vec!["rp 0".to_string()];
    let child_commands = vec!["rp 0".to_string(), "go 2".to_string()];
    let mut child = test_campaign_branch("child", 6, 80);
    child.commands = child_commands.clone();

    let mut state_store = super::state_graph::BranchStateStoreV1::new();
    state_store.insert_session(
        parent_commands.clone(),
        RunControlSession::new(RunControlConfig::default()),
    );
    state_store.insert_child_session(
        &parent_commands,
        child_commands.clone(),
        RunControlSession::new(RunControlConfig::default()),
    );

    let state = BranchCampaignRunStateV1 {
        rounds_completed: 1,
        active: vec![child],
        frozen: Vec::new(),
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: Vec::new(),
        stuck: Vec::new(),
        discarded_count: 0,
        discarded_examples: Vec::new(),
        discarded_branches: Vec::new(),
        strategy_requests: Vec::new(),
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerStateV1::default(),
        rounds: Vec::new(),
        journal: Default::default(),
        state_store,
        decision_parent_anchor_commands: BTreeSet::from([parent_commands.clone()]),
        recovered_checkpoint_failure_commands: BTreeSet::new(),
    };

    let checkpoint = campaign_checkpoint_from_state_v1(&config, &state);

    assert_eq!(checkpoint.schema_name, "BranchCampaignCheckpointV2");
    assert_eq!(checkpoint.schema_version, 2);
    assert_eq!(checkpoint.nodes.len(), 2);
    assert_eq!(checkpoint.nodes[1].parent_id, Some(0));
    assert_eq!(checkpoint.nodes[1].added_commands, vec!["go 2".to_string()]);
    let session_commands = checkpoint
        .sessions
        .iter()
        .map(|session| session.commands.clone())
        .collect::<BTreeSet<_>>();
    assert_eq!(checkpoint.sessions.len(), 2);
    assert!(session_commands.contains(&child_commands));
    assert!(session_commands.contains(&parent_commands));
}

#[test]
fn campaign_checkpoint_deduplicates_last_combat_automation_trajectories() {
    let config = BranchCampaignConfigV1::default();
    let parent_commands = vec!["rp 0".to_string()];
    let child_commands = vec!["rp 0".to_string(), "go 2".to_string()];
    let mut parent = test_campaign_branch("parent", 6, 80);
    parent.commands = parent_commands.clone();
    let mut child = test_campaign_branch("child", 7, 80);
    child.commands = child_commands.clone();

    let mut state_store = super::state_graph::BranchStateStoreV1::new();
    state_store.insert_session(
        parent_commands.clone(),
        super::test_run_control_session_with_last_combat_trajectory("search_combat"),
    );
    state_store.insert_child_session(
        &parent_commands,
        child_commands.clone(),
        super::test_run_control_session_with_last_combat_trajectory("search_combat"),
    );

    let state = BranchCampaignRunStateV1 {
        rounds_completed: 1,
        active: vec![parent, child],
        frozen: Vec::new(),
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: Vec::new(),
        stuck: Vec::new(),
        discarded_count: 0,
        discarded_examples: Vec::new(),
        discarded_branches: Vec::new(),
        strategy_requests: Vec::new(),
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerStateV1::default(),
        rounds: Vec::new(),
        journal: Default::default(),
        state_store,
        decision_parent_anchor_commands: BTreeSet::new(),
        recovered_checkpoint_failure_commands: BTreeSet::new(),
    };

    let checkpoint = campaign_checkpoint_from_state_v1(&config, &state);

    assert_eq!(
        checkpoint.run_state_maps.len(),
        1,
        "campaign checkpoint should keep repeated run maps in a top-level pool"
    );
    assert_eq!(
        checkpoint.run_state_map_graphs.len(),
        1,
        "campaign checkpoint should keep repeated map graphs in a top-level pool"
    );
    assert!(
        checkpoint.run_state_maps[0].map.graph.is_empty(),
        "campaign checkpoint map records should reference pooled graph topology"
    );
    assert_eq!(
        checkpoint.run_state_master_decks.len(),
        1,
        "campaign checkpoint should keep repeated master decks in a top-level pool"
    );
    assert_eq!(
        checkpoint.run_state_schedules.len(),
        1,
        "campaign checkpoint should keep repeated run schedules in a top-level pool"
    );
    assert_eq!(
        checkpoint.run_state_emitted_events.len(),
        1,
        "campaign checkpoint should keep repeated emitted events in a top-level pool"
    );
    assert_eq!(checkpoint.combat_automation_trajectories.len(), 1);
    assert_eq!(
        checkpoint.combat_automation_trajectories[0].commands.len(),
        2
    );
    for entry in &checkpoint.sessions {
        let session_json =
            serde_json::to_value(&entry.session).expect("session checkpoint should serialize");
        assert!(
            session_json
                .get("run_state")
                .and_then(|run_state| run_state.get("map"))
                .is_none(),
            "campaign checkpoint sessions should reference pooled maps instead of embedding them"
        );
        assert!(
            session_json
                .get("run_state")
                .and_then(|run_state| run_state.get("master_deck"))
                .is_none(),
            "campaign checkpoint sessions should reference pooled master decks instead of embedding them"
        );
        assert!(
            session_json
                .get("run_state")
                .and_then(|run_state| run_state.get("rng_pool"))
                .is_none(),
            "campaign checkpoint sessions should reference pooled schedules instead of embedding RNG state"
        );
        assert!(
            session_json
                .get("run_state")
                .and_then(|run_state| run_state.get("event_generator"))
                .is_none(),
            "campaign checkpoint sessions should reference pooled schedules instead of embedding event scheduling state"
        );
        assert!(
            session_json
                .get("run_state")
                .and_then(|run_state| run_state.get("emitted_events"))
                .is_none(),
            "campaign checkpoint sessions should reference pooled emitted events instead of embedding domain event logs"
        );
        assert!(
            session_json
                .get("last_combat_automation_trajectory")
                .is_none(),
            "campaign checkpoint sessions should keep replay state slim"
        );
        let restored = checkpoint
            .hydrated_session_checkpoint_v1(entry)
            .expect("trajectory reference should hydrate")
            .into_session()
            .expect("hydrated checkpoint should restore");
        assert!(
            restored.last_combat_automation_trajectory().is_some(),
            "checkpoint-level trajectory records should preserve diagnostic inspect data"
        );
        assert_eq!(
            restored.run_state.master_deck.len(),
            10,
            "checkpoint-level master deck records should preserve exact run state"
        );
        assert!(
            !restored.run_state.map.graph.is_empty(),
            "checkpoint-level map graph records should hydrate exact map topology"
        );
        assert!(
            !restored.run_state.common_relic_pool.is_empty(),
            "checkpoint-level schedule records should preserve relic pools"
        );
        assert_eq!(
            restored.run_state.emitted_events.len(),
            1,
            "checkpoint-level emitted event records should preserve domain event logs"
        );
    }
}
