use super::*;

#[test]
fn campaign_resume_with_zero_rounds_preserves_previous_frontier() {
    let previous = test_campaign_report_with_active("resume-a", 20, 80);
    let resumed = run_branch_campaign_from_report_v1(
        &BranchCampaignConfigV1 {
            seed: previous.seed,
            max_rounds: 0,
            ..BranchCampaignConfigV1::default()
        },
        &previous,
    )
    .expect("resume should load previous frontier");

    assert_eq!(resumed.rounds_completed, previous.rounds_completed);
    assert_eq!(resumed.active, previous.active);
    assert_eq!(resumed.frozen, previous.frozen);
    assert_eq!(resumed.stop_reason, "max_rounds");
}

#[test]
fn campaign_report_records_branch_command_replay_root_prelude() {
    let result = run_branch_campaign_with_checkpoint_v1(&BranchCampaignConfigV1 {
        seed: 521,
        max_rounds: 0,
        prefix_commands: vec!["0".to_string(), "go 1".to_string()],
        ..BranchCampaignConfigV1::default()
    })
    .expect("campaign should run");

    assert_eq!(
        result.report.run_prelude.replay_root_id,
        "campaign_root_after_prelude"
    );
    assert_eq!(
        result.report.run_prelude.branch_command_coordinate,
        "relative_to_run_prelude"
    );
    assert_eq!(
        result.report.run_prelude.prefix_commands,
        vec!["0".to_string(), "go 1".to_string()]
    );
    assert_eq!(result.checkpoint.run_prelude, result.report.run_prelude);
}

#[test]
fn campaign_resume_prefers_report_prelude_over_cli_prefix() {
    let mut previous = test_campaign_report_with_active("resume-a", 20, 80);
    previous.run_prelude = BranchCampaignRunPreludeV1 {
        replay_root_id: "campaign_root_after_prelude".to_string(),
        branch_command_coordinate: "relative_to_run_prelude".to_string(),
        prefix_commands: vec!["source-prefix".to_string()],
    };

    let resumed = run_branch_campaign_from_report_v1(
        &BranchCampaignConfigV1 {
            seed: previous.seed,
            max_rounds: 0,
            prefix_commands: vec!["wrong-cli-prefix".to_string()],
            ..BranchCampaignConfigV1::default()
        },
        &previous,
    )
    .expect("resume should load previous frontier");

    assert_eq!(resumed.run_prelude, previous.run_prelude);
}

#[test]
fn campaign_resume_rejects_seed_mismatch() {
    let previous = test_campaign_report_with_active("resume-a", 20, 80);
    let err = run_branch_campaign_from_report_v1(
        &BranchCampaignConfigV1 {
            seed: previous.seed + 1,
            max_rounds: 0,
            ..BranchCampaignConfigV1::default()
        },
        &previous,
    )
    .expect_err("wrong seed should not resume");

    assert!(err.contains("seed mismatch"));
}

#[test]
fn campaign_state_uses_snapshot_without_replaying_parent_commands() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut reward = RewardState::new();
    reward.pending_card_choice = Some(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::Cleave, 0),
    ]);
    session.engine_state = EngineState::RewardScreen(reward);

    let mut parent = test_campaign_branch("snap-parent", 1, 80);
    parent.commands = vec!["__bad_internal_command".to_string()];
    parent.choice_labels = vec!["already replayed".to_string()];

    let report = run_branch_campaign_from_state_with_progress_v1(
        &BranchCampaignConfigV1 {
            max_rounds: 1,
            round_depth: 1,
            max_active: 4,
            max_frozen: 4,
            prefix_commands: vec!["__bad_internal_prefix".to_string()],
            ..BranchCampaignConfigV1::default()
        },
        BranchCampaignRunStateV1 {
            rounds_completed: 0,
            active: vec![parent.clone()],
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
            state_store: {
                let mut store = super::state_graph::BranchStateStoreV1::new();
                store.insert_session(parent.commands.clone(), session);
                store
            },
            decision_parent_anchor_commands: BTreeSet::new(),
            recovered_checkpoint_failure_commands: BTreeSet::new(),
        },
        |_| {},
    )
    .expect("snapshot should avoid replaying invalid prefix and parent commands");

    let branch_labels = report
        .report
        .active
        .iter()
        .chain(report.report.frozen.iter())
        .flat_map(|branch| branch.choice_labels.iter())
        .collect::<Vec<_>>();
    assert!(
        branch_labels
            .iter()
            .any(|label| label.contains("Twin Strike") || label.contains("Cleave")),
        "at least one branch should come from the restored reward-screen snapshot"
    );
}

#[test]
fn campaign_checkpoint_preserves_abandoned_and_stuck_snapshots_for_diagnostics() {
    let config = BranchCampaignConfigV1::default();
    let abandoned = {
        let mut branch = test_campaign_branch("abandoned", 32, 80);
        branch.commands = vec!["skip".to_string(), "rest".to_string()];
        branch.status = BranchCampaignBranchStatusV1::Abandoned;
        branch
    };
    let stuck = {
        let mut branch = test_campaign_branch("stuck", 30, 59);
        branch.commands = vec!["rp 0".to_string(), "skip".to_string()];
        branch.status = BranchCampaignBranchStatusV1::Stuck;
        branch
    };
    let abandoned_session = RunControlSession::new(RunControlConfig::default());
    let stuck_session = RunControlSession::new(RunControlConfig::default());
    let state = BranchCampaignRunStateV1 {
        rounds_completed: 3,
        active: Vec::new(),
        frozen: Vec::new(),
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: vec![abandoned.clone()],
        stuck: vec![stuck.clone()],
        discarded_count: 0,
        discarded_examples: Vec::new(),
        discarded_branches: Vec::new(),
        strategy_requests: Vec::new(),
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerStateV1::default(),
        rounds: Vec::new(),
        journal: Default::default(),
        state_store: {
            let mut store = super::state_graph::BranchStateStoreV1::new();
            store.insert_session(abandoned.commands.clone(), abandoned_session);
            store.insert_session(stuck.commands.clone(), stuck_session);
            store
        },
        decision_parent_anchor_commands: BTreeSet::new(),
        recovered_checkpoint_failure_commands: BTreeSet::new(),
    };

    let checkpoint = campaign_checkpoint_from_state_v1(&config, &state);
    let commands = checkpoint
        .sessions
        .iter()
        .map(|entry| checkpoint.session_commands_v1(entry).unwrap())
        .collect::<Vec<_>>();

    assert!(commands.contains(&abandoned.commands));
    assert!(commands.contains(&stuck.commands));
}

#[test]
fn campaign_resume_checkpoint_drops_unrestorable_stuck_branches_and_requests() {
    let mut restorable = test_campaign_branch_with_boundary(
        "restorable",
        "Beggar",
        "event option requires human choice",
        21,
        53,
    );
    restorable.commands = vec!["event 3".to_string(), "event 0".to_string()];
    restorable.status = BranchCampaignBranchStatusV1::Stuck;
    let mut stale = test_campaign_branch_with_boundary(
        "stale",
        "Beggar",
        "event option requires human choice",
        21,
        53,
    );
    stale.commands = vec!["event 3".to_string(), "event 1".to_string()];
    stale.status = BranchCampaignBranchStatusV1::Stuck;
    let mut request = test_campaign_request("event_strategy", "Beggar");
    request.act = 1;
    request.floor = 21;
    request.stop_reasons = vec!["event option requires human choice".to_string()];
    let previous = BranchCampaignReportV1 {
        schema_name: BRANCH_CAMPAIGN_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_SCHEMA_VERSION,
        seed: 1,
        run_domain: BranchCampaignRunDomainV1::default(),
        run_prelude: Default::default(),
        rounds_completed: 0,
        stop_reason: "needs_intervention".to_string(),
        active: Vec::new(),
        frozen: Vec::new(),
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: Vec::new(),
        stuck: vec![restorable.clone(), stale.clone()],
        discarded_count: 0,
        discarded_examples: Vec::new(),
        discarded_branches: Vec::new(),
        strategy_requests: vec![request],
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
        journal: Default::default(),
        rounds: Vec::new(),
    };
    let checkpoint = BranchCampaignCheckpointV1 {
        schema_name: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION,
        seed: 1,
        run_domain: BranchCampaignRunDomainV1::default(),
        run_prelude: Default::default(),
        rounds_completed: 0,
        nodes: Vec::new(),
        decision_parent_anchor_commands: Vec::new(),
        decision_parent_anchor_node_ids: Vec::new(),
        run_state_map_graphs: Vec::new(),
        run_state_maps: Vec::new(),
        run_state_master_decks: Vec::new(),
        run_state_relics: Vec::new(),
        run_state_potions: Vec::new(),
        run_state_schedules: Vec::new(),
        run_state_schedule_components: Default::default(),
        run_state_emitted_events: Vec::new(),
        combat_automation_trajectories: Vec::new(),
        active_combats: Vec::new(),
        sessions: vec![BranchCampaignCheckpointSessionV1 {
            node_id: None,
            commands: restorable.commands.clone(),
            run_state_map_id: None,
            run_state_master_deck_id: None,
            run_state_relics_id: None,
            run_state_potions_id: None,
            run_state_schedule_id: None,
            run_state_emitted_events_id: None,
            active_combat_id: None,
            session: RunControlSessionCheckpointV1::from_session(&RunControlSession::new(
                RunControlConfig::default(),
            )),
        }],
    };

    let state = campaign_state_from_report_and_checkpoint_v1(&previous, &checkpoint)
        .expect("resume checkpoint should load");

    assert_eq!(state.stuck.len(), 1);
    assert_eq!(state.stuck[0].commands, restorable.commands);
    assert_eq!(state.strategy_requests.len(), 1);
}

#[test]
fn campaign_resume_checkpoint_restores_snapshot_without_replaying_parent_commands() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut reward = RewardState::new();
    reward.pending_card_choice = Some(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::Cleave, 0),
    ]);
    session.engine_state = EngineState::RewardScreen(reward);

    let mut parent = test_campaign_branch("snap-parent", 1, 80);
    parent.commands = vec!["__bad_internal_command".to_string()];
    parent.choice_labels = vec!["already replayed".to_string()];

    let previous = BranchCampaignReportV1 {
        schema_name: BRANCH_CAMPAIGN_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_SCHEMA_VERSION,
        seed: 1,
        run_domain: BranchCampaignRunDomainV1::default(),
        run_prelude: Default::default(),
        rounds_completed: 0,
        stop_reason: "max_rounds".to_string(),
        active: vec![parent.clone()],
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
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
        journal: Default::default(),
        rounds: Vec::new(),
    };
    let checkpoint = BranchCampaignCheckpointV1 {
        schema_name: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION,
        seed: 1,
        run_domain: BranchCampaignRunDomainV1::default(),
        run_prelude: Default::default(),
        rounds_completed: 0,
        nodes: Vec::new(),
        decision_parent_anchor_commands: Vec::new(),
        decision_parent_anchor_node_ids: Vec::new(),
        run_state_map_graphs: Vec::new(),
        run_state_maps: Vec::new(),
        run_state_master_decks: Vec::new(),
        run_state_relics: Vec::new(),
        run_state_potions: Vec::new(),
        run_state_schedules: Vec::new(),
        run_state_schedule_components: Default::default(),
        run_state_emitted_events: Vec::new(),
        combat_automation_trajectories: Vec::new(),
        active_combats: Vec::new(),
        sessions: vec![BranchCampaignCheckpointSessionV1 {
            node_id: None,
            commands: parent.commands.clone(),
            run_state_map_id: None,
            run_state_master_deck_id: None,
            run_state_relics_id: None,
            run_state_potions_id: None,
            run_state_schedule_id: None,
            run_state_emitted_events_id: None,
            active_combat_id: None,
            session: RunControlSessionCheckpointV1::from_session(&session),
        }],
    };

    let result = run_branch_campaign_from_report_with_checkpoint_v1(
        &BranchCampaignConfigV1 {
            max_rounds: 1,
            round_depth: 1,
            max_active: 4,
            max_frozen: 4,
            prefix_commands: vec!["__bad_internal_prefix".to_string()],
            ..BranchCampaignConfigV1::default()
        },
        &previous,
        Some(&checkpoint),
    )
    .expect("checkpoint should avoid replaying invalid prefix and parent commands");

    let branch_labels = result
        .report
        .active
        .iter()
        .chain(result.report.frozen.iter())
        .flat_map(|branch| branch.choice_labels.iter())
        .collect::<Vec<_>>();
    assert!(
        branch_labels
            .iter()
            .any(|label| label.contains("Twin Strike") || label.contains("Cleave")),
        "at least one branch should come from the restored checkpoint snapshot"
    );
}

#[test]
fn campaign_resume_rehydrates_checkpointed_combat_failures() {
    let mut active = test_campaign_branch_with_boundary("active-low", "Shop", "test", 46, 7);
    active.summary.as_mut().expect("summary").act = 3;
    active.commands = vec!["active".to_string()];

    let mut combat_failure = test_campaign_branch_with_boundary(
        "combat-high",
        "Combat",
        "combat search did not find an executable complete win",
        48,
        61,
    );
    combat_failure.summary.as_mut().expect("summary").act = 3;
    combat_failure.commands = vec!["old-combat".to_string()];
    combat_failure.status = BranchCampaignBranchStatusV1::Abandoned;

    let mut noncombat_failure = test_campaign_branch_with_boundary(
        "event-old",
        "Falling",
        "event option requires human choice",
        36,
        70,
    );
    noncombat_failure.summary.as_mut().expect("summary").act = 3;
    noncombat_failure.commands = vec!["old-event".to_string()];
    noncombat_failure.status = BranchCampaignBranchStatusV1::Abandoned;

    let previous = BranchCampaignReportV1 {
        schema_name: BRANCH_CAMPAIGN_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_SCHEMA_VERSION,
        seed: 1,
        run_domain: BranchCampaignRunDomainV1::default(),
        run_prelude: Default::default(),
        rounds_completed: 8,
        stop_reason: "max_rounds".to_string(),
        active: vec![active],
        frozen: Vec::new(),
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: vec![combat_failure.clone(), noncombat_failure.clone()],
        stuck: Vec::new(),
        discarded_count: 0,
        discarded_examples: Vec::new(),
        discarded_branches: Vec::new(),
        strategy_requests: Vec::new(),
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
        journal: Default::default(),
        rounds: Vec::new(),
    };
    let mut restored_session = RunControlSession::new(RunControlConfig::default());
    restored_session.engine_state = EngineState::CombatPlayerTurn;
    restored_session.run_state.act_num = 3;
    restored_session.run_state.floor_num = 48;
    restored_session.run_state.current_hp = 61;
    restored_session.run_state.max_hp = 90;
    let checkpoint = BranchCampaignCheckpointV1 {
        schema_name: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION,
        seed: 1,
        run_domain: BranchCampaignRunDomainV1::default(),
        run_prelude: Default::default(),
        rounds_completed: 8,
        nodes: Vec::new(),
        decision_parent_anchor_commands: Vec::new(),
        decision_parent_anchor_node_ids: Vec::new(),
        run_state_map_graphs: Vec::new(),
        run_state_maps: Vec::new(),
        run_state_master_decks: Vec::new(),
        run_state_relics: Vec::new(),
        run_state_potions: Vec::new(),
        run_state_schedules: Vec::new(),
        run_state_schedule_components: Default::default(),
        run_state_emitted_events: Vec::new(),
        combat_automation_trajectories: Vec::new(),
        active_combats: Vec::new(),
        sessions: vec![BranchCampaignCheckpointSessionV1 {
            node_id: None,
            commands: combat_failure.commands.clone(),
            run_state_map_id: None,
            run_state_master_deck_id: None,
            run_state_relics_id: None,
            run_state_potions_id: None,
            run_state_schedule_id: None,
            run_state_emitted_events_id: None,
            active_combat_id: None,
            session: RunControlSessionCheckpointV1::from_session(&restored_session),
        }],
    };

    let result = run_branch_campaign_from_report_with_checkpoint_v1(
        &BranchCampaignConfigV1 {
            seed: 1,
            max_rounds: 0,
            max_active: 1,
            max_frozen: 2,
            ..BranchCampaignConfigV1::default()
        },
        &previous,
        Some(&checkpoint),
    )
    .expect("checkpointed combat failures should be resumable");

    assert!(
        result
            .report
            .frozen
            .iter()
            .any(|branch| branch.branch_id == "combat-high"),
        "old checkpointed combat failure should be reintroduced as a continuable macro branch"
    );
    assert!(
        result
            .report
            .abandoned
            .iter()
            .all(|branch| branch.branch_id != "combat-high"),
        "rehydrated combat failure should no longer remain buried in abandoned"
    );
    assert!(
        result
            .report
            .abandoned
            .iter()
            .any(|branch| branch.branch_id == "event-old"),
        "noncombat abandoned branches should not be rehydrated by the combat checkpoint path"
    );
}

#[test]
fn campaign_resume_does_not_promote_stale_combat_failure_over_later_active_branch() {
    let mut active = test_campaign_branch_with_boundary("act3-active", "Campfire", "test", 47, 74);
    active.summary.as_mut().expect("summary").act = 3;
    active.commands = vec!["act3-active".to_string()];

    let mut stale_combat_failure = test_campaign_branch_with_boundary(
        "act2-combat",
        "Combat",
        "combat search did not find an executable complete win",
        32,
        87,
    );
    stale_combat_failure.summary.as_mut().expect("summary").act = 2;
    stale_combat_failure.commands = vec!["old-act2-combat".to_string()];
    stale_combat_failure.status = BranchCampaignBranchStatusV1::Abandoned;

    let previous = BranchCampaignReportV1 {
        schema_name: BRANCH_CAMPAIGN_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_SCHEMA_VERSION,
        seed: 1,
        run_domain: BranchCampaignRunDomainV1::default(),
        run_prelude: Default::default(),
        rounds_completed: 48,
        stop_reason: "max_rounds".to_string(),
        active: vec![active],
        frozen: Vec::new(),
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: vec![stale_combat_failure.clone()],
        stuck: Vec::new(),
        discarded_count: 0,
        discarded_examples: Vec::new(),
        discarded_branches: Vec::new(),
        strategy_requests: Vec::new(),
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
        journal: Default::default(),
        rounds: Vec::new(),
    };
    let mut restored_session = RunControlSession::new(RunControlConfig::default());
    restored_session.engine_state = EngineState::CombatPlayerTurn;
    restored_session.run_state.act_num = 2;
    restored_session.run_state.floor_num = 32;
    restored_session.run_state.current_hp = 87;
    restored_session.run_state.max_hp = 97;
    let checkpoint = BranchCampaignCheckpointV1 {
        schema_name: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION,
        seed: 1,
        run_domain: BranchCampaignRunDomainV1::default(),
        run_prelude: Default::default(),
        rounds_completed: 48,
        nodes: Vec::new(),
        decision_parent_anchor_commands: Vec::new(),
        decision_parent_anchor_node_ids: Vec::new(),
        run_state_map_graphs: Vec::new(),
        run_state_maps: Vec::new(),
        run_state_master_decks: Vec::new(),
        run_state_relics: Vec::new(),
        run_state_potions: Vec::new(),
        run_state_schedules: Vec::new(),
        run_state_schedule_components: Default::default(),
        run_state_emitted_events: Vec::new(),
        combat_automation_trajectories: Vec::new(),
        active_combats: Vec::new(),
        sessions: vec![BranchCampaignCheckpointSessionV1 {
            node_id: None,
            commands: stale_combat_failure.commands.clone(),
            run_state_map_id: None,
            run_state_master_deck_id: None,
            run_state_relics_id: None,
            run_state_potions_id: None,
            run_state_schedule_id: None,
            run_state_emitted_events_id: None,
            active_combat_id: None,
            session: RunControlSessionCheckpointV1::from_session(&restored_session),
        }],
    };

    let result = run_branch_campaign_from_report_with_checkpoint_v1(
        &BranchCampaignConfigV1 {
            seed: 1,
            max_rounds: 0,
            max_active: 2,
            max_frozen: 4,
            ..BranchCampaignConfigV1::default()
        },
        &previous,
        Some(&checkpoint),
    )
    .expect("checkpointed stale combat failure should be restorable but not promoted");

    assert_eq!(result.report.active.len(), 1);
    assert!(
        result
            .report
            .active
            .iter()
            .all(|branch| branch.branch_id != "act2-combat"),
        "stale Act2 combat failure should not consume an active slot while a later Act3 branch remains active"
    );
    assert!(
        result
            .report
            .frozen
            .iter()
            .any(|branch| branch.branch_id == "act2-combat"),
        "stale combat failure should remain available as frozen diagnostic material"
    );
}

#[test]
fn campaign_resume_rehydrates_auto_advanceable_map_overlay_stuck() {
    let mut map_overlay_stuck = test_campaign_branch_with_boundary(
        "map-overlay",
        "Map Preview",
        "route planner declined automatic map selection",
        16,
        80,
    );
    map_overlay_stuck.commands = vec!["relic 0".to_string(), "skip".to_string()];
    map_overlay_stuck.status = BranchCampaignBranchStatusV1::Stuck;

    let mut request = test_campaign_request("route_policy_gap", "Map Preview");
    request.act = 1;
    request.floor = 16;
    request.stop_reasons = vec!["route planner declined automatic map selection".to_string()];

    let previous = BranchCampaignReportV1 {
        schema_name: BRANCH_CAMPAIGN_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_SCHEMA_VERSION,
        seed: 1,
        run_domain: BranchCampaignRunDomainV1::default(),
        run_prelude: Default::default(),
        rounds_completed: 8,
        stop_reason: "max_rounds".to_string(),
        active: Vec::new(),
        frozen: Vec::new(),
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: Vec::new(),
        stuck: vec![map_overlay_stuck.clone()],
        discarded_count: 0,
        discarded_examples: Vec::new(),
        discarded_branches: Vec::new(),
        strategy_requests: vec![request],
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
        journal: Default::default(),
        rounds: Vec::new(),
    };
    let mut reward = RewardState::new();
    reward.pending_card_choice = Some(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::Cleave, 0),
    ]);
    let mut restored_session = RunControlSession::new(RunControlConfig::default());
    restored_session.engine_state = EngineState::map_overlay(EngineState::RewardScreen(reward));
    restored_session.run_state.act_num = 1;
    restored_session.run_state.floor_num = 16;
    restored_session.run_state.map.current_x = 0;
    restored_session.run_state.map.current_y = 15;
    restored_session.run_state.map.boss_node_available = false;
    restored_session.run_state.pending_boss_act_transition = true;
    let checkpoint = BranchCampaignCheckpointV1 {
        schema_name: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION,
        seed: 1,
        run_domain: BranchCampaignRunDomainV1::default(),
        run_prelude: Default::default(),
        rounds_completed: 8,
        nodes: Vec::new(),
        decision_parent_anchor_commands: Vec::new(),
        decision_parent_anchor_node_ids: Vec::new(),
        run_state_map_graphs: Vec::new(),
        run_state_maps: Vec::new(),
        run_state_master_decks: Vec::new(),
        run_state_relics: Vec::new(),
        run_state_potions: Vec::new(),
        run_state_schedules: Vec::new(),
        run_state_schedule_components: Default::default(),
        run_state_emitted_events: Vec::new(),
        combat_automation_trajectories: Vec::new(),
        active_combats: Vec::new(),
        sessions: vec![BranchCampaignCheckpointSessionV1 {
            node_id: None,
            commands: map_overlay_stuck.commands.clone(),
            run_state_map_id: None,
            run_state_master_deck_id: None,
            run_state_relics_id: None,
            run_state_potions_id: None,
            run_state_schedule_id: None,
            run_state_emitted_events_id: None,
            active_combat_id: None,
            session: RunControlSessionCheckpointV1::from_session(&restored_session),
        }],
    };

    let result = run_branch_campaign_from_report_with_checkpoint_v1(
        &BranchCampaignConfigV1 {
            seed: 1,
            max_rounds: 0,
            max_active: 1,
            max_frozen: 2,
            ..BranchCampaignConfigV1::default()
        },
        &previous,
        Some(&checkpoint),
    )
    .expect("checkpointed map overlay stuck branch should be recoverable");

    assert!(
        result.report.active.iter().any(|branch| {
            branch.branch_id == "map-overlay" && branch.frontier_title == "Card Reward"
        }),
        "map overlay stuck branch should re-enter the campaign at its returned reward boundary"
    );
    assert!(
        result.report.stuck.is_empty(),
        "recovered map overlay branch should not remain in stale stuck diagnostics"
    );
    assert!(
        result.report.strategy_requests.is_empty(),
        "resolved map overlay request should be pruned after recovery"
    );
}

#[test]
fn campaign_resume_rehydrates_stale_map_preview_to_checkpoint_card_reward_frontier() {
    let mut map_overlay_stuck = test_campaign_branch_with_boundary(
        "map-overlay",
        "Map Preview",
        "route planner declined automatic map selection",
        16,
        85,
    );
    map_overlay_stuck.commands = vec!["relic 0".to_string(), "skip".to_string()];
    map_overlay_stuck.status = BranchCampaignBranchStatusV1::Stuck;

    let mut request = test_campaign_request("route_policy_gap", "Map Preview");
    request.act = 1;
    request.floor = 16;
    request.stop_reasons = vec!["route planner declined automatic map selection".to_string()];

    let previous = BranchCampaignReportV1 {
        schema_name: BRANCH_CAMPAIGN_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_SCHEMA_VERSION,
        seed: 1,
        run_domain: BranchCampaignRunDomainV1::default(),
        run_prelude: Default::default(),
        rounds_completed: 8,
        stop_reason: "max_rounds".to_string(),
        active: Vec::new(),
        frozen: Vec::new(),
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: Vec::new(),
        stuck: vec![map_overlay_stuck.clone()],
        discarded_count: 0,
        discarded_examples: Vec::new(),
        discarded_branches: Vec::new(),
        strategy_requests: vec![request],
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
        journal: Default::default(),
        rounds: Vec::new(),
    };
    let mut reward = RewardState::new();
    reward.skippable = true;
    reward.items.push(RewardItem::Card {
        cards: vec![
            RewardCard::new(CardId::WildStrike, 0),
            RewardCard::new(CardId::TrueGrit, 0),
            RewardCard::new(CardId::BattleTrance, 0),
        ],
    });
    let mut restored_session = RunControlSession::new(RunControlConfig::default());
    restored_session.engine_state = EngineState::RewardScreen(reward);
    restored_session.run_state.act_num = 1;
    restored_session.run_state.floor_num = 16;
    restored_session.run_state.current_hp = 85;
    restored_session.run_state.max_hp = 85;
    let checkpoint = BranchCampaignCheckpointV1 {
        schema_name: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION,
        seed: 1,
        run_domain: BranchCampaignRunDomainV1::default(),
        run_prelude: Default::default(),
        rounds_completed: 8,
        nodes: Vec::new(),
        decision_parent_anchor_commands: Vec::new(),
        decision_parent_anchor_node_ids: Vec::new(),
        run_state_map_graphs: Vec::new(),
        run_state_maps: Vec::new(),
        run_state_master_decks: Vec::new(),
        run_state_relics: Vec::new(),
        run_state_potions: Vec::new(),
        run_state_schedules: Vec::new(),
        run_state_schedule_components: Default::default(),
        run_state_emitted_events: Vec::new(),
        combat_automation_trajectories: Vec::new(),
        active_combats: Vec::new(),
        sessions: vec![BranchCampaignCheckpointSessionV1 {
            node_id: None,
            commands: map_overlay_stuck.commands.clone(),
            run_state_map_id: None,
            run_state_master_deck_id: None,
            run_state_relics_id: None,
            run_state_potions_id: None,
            run_state_schedule_id: None,
            run_state_emitted_events_id: None,
            active_combat_id: None,
            session: RunControlSessionCheckpointV1::from_session(&restored_session),
        }],
    };

    let result = run_branch_campaign_from_report_with_checkpoint_v1(
        &BranchCampaignConfigV1 {
            seed: 1,
            max_rounds: 0,
            max_active: 1,
            max_frozen: 2,
            ..BranchCampaignConfigV1::default()
        },
        &previous,
        Some(&checkpoint),
    )
    .expect("checkpoint frontier should be authoritative when stale report says map preview");

    assert!(
        result.report.active.iter().any(|branch| {
            branch.branch_id == "map-overlay" && branch.frontier_title == "Reward Screen"
        }),
        "stale map-preview branch should resume at the exact checkpoint reward frontier"
    );
    assert!(result.report.stuck.is_empty());
    assert!(result.report.strategy_requests.is_empty());
}

#[test]
fn campaign_resume_drops_resolved_map_overlay_stuck_when_no_branch_slot_remains() {
    let mut active = test_campaign_branch_with_boundary("active", "Campfire", "test", 24, 80);
    active.commands = vec!["active".to_string()];

    let mut map_overlay_stuck = test_campaign_branch_with_boundary(
        "map-overlay",
        "Map Preview",
        "route planner declined automatic map selection",
        16,
        80,
    );
    map_overlay_stuck.commands = vec!["relic 0".to_string(), "skip".to_string()];
    map_overlay_stuck.status = BranchCampaignBranchStatusV1::Stuck;

    let mut request = test_campaign_request("route_policy_gap", "Map Preview");
    request.act = 1;
    request.floor = 16;
    request.stop_reasons = vec!["route planner declined automatic map selection".to_string()];

    let previous = BranchCampaignReportV1 {
        schema_name: BRANCH_CAMPAIGN_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_SCHEMA_VERSION,
        seed: 1,
        run_domain: BranchCampaignRunDomainV1::default(),
        run_prelude: Default::default(),
        rounds_completed: 8,
        stop_reason: "max_rounds".to_string(),
        active: vec![active],
        frozen: Vec::new(),
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: Vec::new(),
        stuck: vec![map_overlay_stuck.clone()],
        discarded_count: 0,
        discarded_examples: Vec::new(),
        discarded_branches: Vec::new(),
        strategy_requests: vec![request],
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
        journal: Default::default(),
        rounds: Vec::new(),
    };
    let mut restored_session = RunControlSession::new(RunControlConfig::default());
    let mut empty_reward = RewardState::new();
    empty_reward.skippable = true;
    restored_session.engine_state = EngineState::RewardScreen(empty_reward);
    restored_session.run_state.act_num = 1;
    restored_session.run_state.floor_num = 16;
    restored_session.run_state.map.current_x = 0;
    restored_session.run_state.map.current_y = 15;
    let checkpoint = BranchCampaignCheckpointV1 {
        schema_name: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION,
        seed: 1,
        run_domain: BranchCampaignRunDomainV1::default(),
        run_prelude: Default::default(),
        rounds_completed: 8,
        nodes: Vec::new(),
        decision_parent_anchor_commands: Vec::new(),
        decision_parent_anchor_node_ids: Vec::new(),
        run_state_map_graphs: Vec::new(),
        run_state_maps: Vec::new(),
        run_state_master_decks: Vec::new(),
        run_state_relics: Vec::new(),
        run_state_potions: Vec::new(),
        run_state_schedules: Vec::new(),
        run_state_schedule_components: Default::default(),
        run_state_emitted_events: Vec::new(),
        combat_automation_trajectories: Vec::new(),
        active_combats: Vec::new(),
        sessions: vec![BranchCampaignCheckpointSessionV1 {
            node_id: None,
            commands: map_overlay_stuck.commands.clone(),
            run_state_map_id: None,
            run_state_master_deck_id: None,
            run_state_relics_id: None,
            run_state_potions_id: None,
            run_state_schedule_id: None,
            run_state_emitted_events_id: None,
            active_combat_id: None,
            session: RunControlSessionCheckpointV1::from_session(&restored_session),
        }],
    };

    let result = run_branch_campaign_from_report_with_checkpoint_v1(
        &BranchCampaignConfigV1 {
            seed: 1,
            max_rounds: 0,
            max_active: 1,
            max_frozen: 0,
            ..BranchCampaignConfigV1::default()
        },
        &previous,
        Some(&checkpoint),
    )
    .expect("resolved map overlay stuck should not require human strategy");

    assert!(
        result.report.stuck.is_empty(),
        "resolved map overlay branch should not remain as a stale intervention request"
    );
    assert!(
        result.report.strategy_requests.is_empty(),
        "resolved map overlay request should be pruned even when the recovered branch is discarded"
    );
}

#[test]
fn campaign_resume_rehydrates_combat_failures_as_frozen_diagnostics_only() {
    let mut abandoned = Vec::new();
    let mut checkpoint_sessions = Vec::new();
    for idx in 0..3 {
        let mut branch = test_campaign_branch_with_boundary(
            &format!("combat-{idx}"),
            "Combat",
            "combat search did not find an executable complete win",
            48,
            70 - idx,
        );
        branch.summary.as_mut().expect("summary").act = 3;
        branch.commands = vec![format!("old-combat-{idx}")];
        branch.status = BranchCampaignBranchStatusV1::Abandoned;

        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::CombatPlayerTurn;
        session.run_state.act_num = 3;
        session.run_state.floor_num = 48;
        session.run_state.current_hp = 70 - idx;
        session.run_state.max_hp = 90;
        checkpoint_sessions.push(BranchCampaignCheckpointSessionV1 {
            node_id: None,
            commands: branch.commands.clone(),
            run_state_map_id: None,
            run_state_master_deck_id: None,
            run_state_relics_id: None,
            run_state_potions_id: None,
            run_state_schedule_id: None,
            run_state_emitted_events_id: None,
            active_combat_id: None,
            session: RunControlSessionCheckpointV1::from_session(&session),
        });
        abandoned.push(branch);
    }

    let previous = BranchCampaignReportV1 {
        schema_name: BRANCH_CAMPAIGN_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_SCHEMA_VERSION,
        seed: 1,
        run_domain: BranchCampaignRunDomainV1::default(),
        run_prelude: Default::default(),
        rounds_completed: 8,
        stop_reason: "needs_intervention".to_string(),
        active: Vec::new(),
        frozen: Vec::new(),
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned,
        stuck: Vec::new(),
        discarded_count: 0,
        discarded_examples: Vec::new(),
        discarded_branches: Vec::new(),
        strategy_requests: Vec::new(),
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
        journal: Default::default(),
        rounds: Vec::new(),
    };
    let checkpoint = BranchCampaignCheckpointV1 {
        schema_name: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION,
        seed: 1,
        run_domain: BranchCampaignRunDomainV1::default(),
        run_prelude: Default::default(),
        rounds_completed: 8,
        nodes: Vec::new(),
        decision_parent_anchor_commands: Vec::new(),
        decision_parent_anchor_node_ids: Vec::new(),
        run_state_map_graphs: Vec::new(),
        run_state_maps: Vec::new(),
        run_state_master_decks: Vec::new(),
        run_state_relics: Vec::new(),
        run_state_potions: Vec::new(),
        run_state_schedules: Vec::new(),
        run_state_schedule_components: Default::default(),
        run_state_emitted_events: Vec::new(),
        combat_automation_trajectories: Vec::new(),
        active_combats: Vec::new(),
        sessions: checkpoint_sessions,
    };

    let result = run_branch_campaign_from_report_with_checkpoint_v1(
        &BranchCampaignConfigV1 {
            seed: 1,
            max_rounds: 0,
            max_active: 2,
            max_frozen: 16,
            ..BranchCampaignConfigV1::default()
        },
        &previous,
        Some(&checkpoint),
    )
    .expect("checkpointed combat failures should be resumable");

    assert_eq!(
        result.report.active.len(),
        0,
        "resume should not automatically retry abandoned combat failure representatives"
    );
    assert_eq!(
        result.report.frozen.len(),
        2,
        "resume may keep active-capacity combat failure representatives as frozen diagnostics"
    );
    assert_eq!(
        result.report.abandoned.len(),
        1,
        "extra checkpointed combat failures should remain abandoned for diagnostics"
    );
}

#[test]
fn campaign_resume_rehydrates_later_combat_failure_before_stale_early_failure() {
    let mut early_failure = test_campaign_branch_with_boundary(
        "act2-combat",
        "Combat",
        "combat search did not find an executable complete win",
        32,
        80,
    );
    early_failure.summary.as_mut().expect("summary").act = 2;
    early_failure.commands = vec!["act2".to_string()];
    early_failure.status = BranchCampaignBranchStatusV1::Abandoned;

    let mut final_boss_failure = test_campaign_branch_with_boundary(
        "act3-boss",
        "Combat",
        "combat search did not find an executable complete win",
        48,
        80,
    );
    final_boss_failure.summary.as_mut().expect("summary").act = 3;
    final_boss_failure.commands = vec!["act3".to_string()];
    final_boss_failure.status = BranchCampaignBranchStatusV1::Abandoned;

    let checkpoint = BranchCampaignCheckpointV1 {
        schema_name: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION,
        seed: 1,
        run_domain: BranchCampaignRunDomainV1::default(),
        run_prelude: Default::default(),
        rounds_completed: 8,
        nodes: Vec::new(),
        decision_parent_anchor_commands: Vec::new(),
        decision_parent_anchor_node_ids: Vec::new(),
        run_state_map_graphs: Vec::new(),
        run_state_maps: Vec::new(),
        run_state_master_decks: Vec::new(),
        run_state_relics: Vec::new(),
        run_state_potions: Vec::new(),
        run_state_schedules: Vec::new(),
        run_state_schedule_components: Default::default(),
        run_state_emitted_events: Vec::new(),
        combat_automation_trajectories: Vec::new(),
        active_combats: Vec::new(),
        sessions: vec![
            test_combat_checkpoint_session(&early_failure, 2, 32, 80),
            test_combat_checkpoint_session(&final_boss_failure, 3, 48, 80),
        ],
    };
    let previous = BranchCampaignReportV1 {
        schema_name: BRANCH_CAMPAIGN_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_SCHEMA_VERSION,
        seed: 1,
        run_domain: BranchCampaignRunDomainV1::default(),
        run_prelude: Default::default(),
        rounds_completed: 8,
        stop_reason: "max_rounds".to_string(),
        active: Vec::new(),
        frozen: Vec::new(),
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: vec![early_failure, final_boss_failure],
        stuck: Vec::new(),
        discarded_count: 0,
        discarded_examples: Vec::new(),
        discarded_branches: Vec::new(),
        strategy_requests: Vec::new(),
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
        journal: Default::default(),
        rounds: Vec::new(),
    };

    let result = run_branch_campaign_from_report_with_checkpoint_v1(
        &BranchCampaignConfigV1 {
            seed: 1,
            max_rounds: 0,
            max_active: 1,
            max_frozen: 1,
            ..BranchCampaignConfigV1::default()
        },
        &previous,
        Some(&checkpoint),
    )
    .expect("checkpointed combat failures should be resumable");

    assert_eq!(result.report.frozen.len(), 1);
    assert_eq!(result.report.frozen[0].branch_id, "act3-boss");
    assert_eq!(result.report.abandoned.len(), 1);
    assert_eq!(result.report.abandoned[0].branch_id, "act2-combat");
}
