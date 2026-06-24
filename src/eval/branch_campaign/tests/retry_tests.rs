use super::*;

#[test]
fn campaign_retry_budget_raises_combat_search_without_restoring_hp_gate() {
    let config = BranchCampaignConfigV1 {
        search_max_nodes: Some(50_000),
        search_wall_ms: Some(300),
        search_max_hp_loss: Some(RunControlHpLossLimit::Unlimited),
        ..BranchCampaignConfigV1::default()
    };

    let retry = combat_retry_campaign_config_v1(&config).expect("retry config");

    assert_eq!(retry.search_max_nodes, Some(200_000));
    assert_eq!(retry.search_wall_ms, Some(1_000));
    assert_eq!(
        retry.search_max_hp_loss,
        Some(RunControlHpLossLimit::Unlimited)
    );
}

#[test]
fn campaign_retry_budget_keeps_single_search_strong_but_limits_candidate_breadth() {
    let config = BranchCampaignConfigV1 {
        max_active: 2,
        max_branches_per_active: 12,
        search_max_nodes: Some(50_000),
        search_wall_ms: Some(300),
        ..BranchCampaignConfigV1::default()
    };

    let retry = combat_retry_campaign_config_v1(&config).expect("retry config");

    assert_eq!(retry.search_max_nodes, Some(200_000));
    assert_eq!(retry.search_wall_ms, Some(1_000));
    assert_eq!(retry.max_branches_per_active, 2);
}

#[test]
fn campaign_retry_budget_can_use_explicit_wall_cap() {
    let config = BranchCampaignConfigV1 {
        search_max_nodes: Some(10_000),
        search_wall_ms: Some(50),
        combat_retry_wall_ms: Some(1_000),
        ..BranchCampaignConfigV1::default()
    };

    let retry = combat_retry_campaign_config_v1(&config).expect("retry config");

    assert_eq!(retry.search_max_nodes, Some(200_000));
    assert_eq!(retry.search_wall_ms, Some(1_000));
}

#[test]
fn campaign_parent_batch_starts_all_parents_before_finished_events() {
    let config = BranchCampaignConfigV1 {
        round_depth: 0,
        max_branches_per_active: 1,
        experiment_wall_ms: Some(100),
        search_wall_ms: Some(10),
        search_max_nodes: Some(100),
        combat_retry_policy: BranchCampaignCombatRetryPolicyV1::Disabled,
        ..BranchCampaignConfigV1::default()
    };
    let parents = vec![root_campaign_branch_v1(), root_campaign_branch_v1()];
    let mut state_store = super::state_graph::BranchStateStoreV1::new();
    let mut retry_ledger = BranchCampaignCombatRetryLedgerStateV1::default();
    let mut progress_sequence = Vec::new();

    run_campaign_parent_batch_v1(
        &config,
        &parents,
        &mut state_store,
        &mut retry_ledger,
        1,
        false,
        &mut |event| match event {
            BranchCampaignProgressEventV1::BranchStarted { branch_index, .. } => {
                progress_sequence.push(format!("start{branch_index}"));
            }
            BranchCampaignProgressEventV1::BranchFinished { branch_index, .. } => {
                progress_sequence.push(format!("finish{branch_index}"));
            }
            _ => {}
        },
    )
    .expect("parent batch should run");

    assert_eq!(
        progress_sequence,
        vec!["start1", "start2", "finish1", "finish2"]
    );
}

#[test]
fn campaign_retries_only_when_all_results_are_abandoned_combat() {
    let abandoned_combat = test_report_branch_at(
        "a",
        Vec::new(),
        BranchExperimentBranchStatusV1::Pruned,
        "Combat",
        16,
        70,
    );
    let active_card_reward = test_report_branch_at(
        "b",
        Vec::new(),
        BranchExperimentBranchStatusV1::Active,
        "Card Reward",
        4,
        80,
    );

    assert!(branch_report_needs_combat_budget_retry_v1(&[
        abandoned_combat.clone()
    ]));
    assert!(!branch_report_needs_combat_budget_retry_v1(&[
        abandoned_combat,
        active_card_reward,
    ]));
}

#[test]
fn campaign_default_retry_policy_does_not_retry_each_parent_immediately() {
    let config = BranchCampaignConfigV1::default();
    let abandoned_combat = test_report_branch_at(
        "a",
        Vec::new(),
        BranchExperimentBranchStatusV1::Pruned,
        "Combat",
        10,
        70,
    );

    assert_eq!(
        config.combat_retry_policy,
        BranchCampaignCombatRetryPolicyV1::OnStall
    );
    assert!(!campaign_parent_should_retry_combat_budget_now_v1(
        &config,
        &[abandoned_combat]
    ));
}

#[test]
fn campaign_disabled_retry_policy_disables_act_boss_gate_parent_retry() {
    let config = BranchCampaignConfigV1 {
        combat_retry_policy: BranchCampaignCombatRetryPolicyV1::Disabled,
        ..BranchCampaignConfigV1::default()
    };
    let abandoned_act1_boss = test_report_branch_at(
        "a1",
        Vec::new(),
        BranchExperimentBranchStatusV1::Pruned,
        "Combat",
        16,
        80,
    );

    assert!(!campaign_parent_should_retry_combat_budget_now_v1(
        &config,
        &[abandoned_act1_boss]
    ));
}

#[test]
fn campaign_default_retry_policy_retries_act_boss_gate_combat_parent_immediately() {
    let config = BranchCampaignConfigV1::default();
    let abandoned_act1_boss = test_report_branch_at(
        "a1",
        Vec::new(),
        BranchExperimentBranchStatusV1::Pruned,
        "Combat",
        16,
        80,
    );
    let mut abandoned_act2_boss = test_report_branch_at(
        "a2",
        Vec::new(),
        BranchExperimentBranchStatusV1::Pruned,
        "Combat",
        32,
        80,
    );
    let mut abandoned_final_boss = test_report_branch_at(
        "a3",
        Vec::new(),
        BranchExperimentBranchStatusV1::Pruned,
        "Combat",
        48,
        80,
    );
    abandoned_act2_boss.summary.act = 2;
    abandoned_act2_boss.frontier.act = 2;
    abandoned_final_boss.summary.act = 3;
    abandoned_final_boss.frontier.act = 3;

    assert!(campaign_parent_should_retry_combat_budget_now_v1(
        &config,
        &[abandoned_act1_boss]
    ));
    assert!(campaign_parent_should_retry_combat_budget_now_v1(
        &config,
        &[abandoned_act2_boss]
    ));
    assert!(campaign_parent_should_retry_combat_budget_now_v1(
        &config,
        &[abandoned_final_boss]
    ));
}

#[test]
fn campaign_boss_gate_retry_ledger_limits_attempts_per_gate() {
    let abandoned_act1_boss = test_report_branch_at(
        "a1",
        Vec::new(),
        BranchExperimentBranchStatusV1::Pruned,
        "Combat",
        16,
        80,
    );
    let mut ledger = BranchCampaignCombatRetryLedgerStateV1::default();

    assert!(try_consume_branch_report_act_boss_gate_retry_v1(
        &mut ledger,
        &[abandoned_act1_boss.clone()]
    ));
    assert!(try_consume_branch_report_act_boss_gate_retry_v1(
        &mut ledger,
        &[abandoned_act1_boss.clone()]
    ));
    assert!(!try_consume_branch_report_act_boss_gate_retry_v1(
        &mut ledger,
        &[abandoned_act1_boss]
    ));

    let report = ledger.to_report_v1();
    assert_eq!(report.boss_gate_attempts.len(), 1);
    assert_eq!(report.boss_gate_attempts[0].act, 1);
    assert_eq!(report.boss_gate_attempts[0].floor, 16);
    assert_eq!(
        report.boss_gate_attempts[0].attempts,
        BOSS_GATE_RETRY_ATTEMPTS_PER_GATE
    );
}

#[test]
fn campaign_non_boss_gate_combat_has_no_boss_gate_retry_key() {
    let abandoned_hallway = test_report_branch_at(
        "a",
        Vec::new(),
        BranchExperimentBranchStatusV1::Pruned,
        "Combat",
        10,
        70,
    );
    let mut ledger = BranchCampaignCombatRetryLedgerStateV1::default();

    assert!(try_consume_branch_report_act_boss_gate_retry_v1(
        &mut ledger,
        &[abandoned_hallway]
    ));
    assert!(ledger.to_report_v1().boss_gate_attempts.is_empty());
}

#[test]
fn campaign_on_stall_retries_when_round_exhausts_only_abandoned_combat() {
    let config = BranchCampaignConfigV1::default();
    let mut abandoned = test_campaign_branch("abandoned-combat", 1, 80);
    abandoned.status = BranchCampaignBranchStatusV1::Abandoned;
    abandoned.frontier_title = "Combat".to_string();

    let selection = BranchCampaignSelectionV1 {
        abandoned: vec![abandoned],
        ..BranchCampaignSelectionV1::default()
    };

    assert!(campaign_round_should_retry_combat_budget_on_stall_v1(
        &config, &selection, 0
    ));
}

#[test]
fn campaign_on_stall_uses_existing_frozen_before_combat_retry() {
    let config = BranchCampaignConfigV1::default();
    let mut abandoned = test_campaign_branch("abandoned-combat", 1, 80);
    abandoned.status = BranchCampaignBranchStatusV1::Abandoned;
    abandoned.frontier_title = "Combat".to_string();

    let selection = BranchCampaignSelectionV1 {
        abandoned: vec![abandoned],
        ..BranchCampaignSelectionV1::default()
    };

    assert!(!campaign_round_should_retry_combat_budget_on_stall_v1(
        &config, &selection, 3
    ));
}

#[test]
fn campaign_on_stall_does_not_retry_when_other_branches_remain() {
    let config = BranchCampaignConfigV1::default();
    let mut abandoned = test_campaign_branch("abandoned-combat", 1, 80);
    abandoned.status = BranchCampaignBranchStatusV1::Abandoned;
    abandoned.frontier_title = "Combat".to_string();

    let selection = BranchCampaignSelectionV1 {
        scheduled: vec![test_campaign_branch("scheduled", 2, 80)],
        abandoned: vec![abandoned],
        ..BranchCampaignSelectionV1::default()
    };

    assert!(!campaign_round_should_retry_combat_budget_on_stall_v1(
        &config, &selection, 0
    ));
}

#[test]
fn campaign_on_stall_does_not_retry_non_combat_abandoned_branches() {
    let config = BranchCampaignConfigV1::default();
    let mut abandoned = test_campaign_branch("abandoned-map", 1, 80);
    abandoned.status = BranchCampaignBranchStatusV1::Abandoned;
    abandoned.frontier_title = "Map".to_string();

    let selection = BranchCampaignSelectionV1 {
        abandoned: vec![abandoned],
        ..BranchCampaignSelectionV1::default()
    };

    assert!(!campaign_round_should_retry_combat_budget_on_stall_v1(
        &config, &selection, 0
    ));
}

#[test]
fn campaign_immediate_retry_policy_keeps_old_parent_retry_behavior() {
    let config = BranchCampaignConfigV1 {
        combat_retry_policy: BranchCampaignCombatRetryPolicyV1::Immediate,
        ..BranchCampaignConfigV1::default()
    };
    let abandoned_combat = test_report_branch_at(
        "a",
        Vec::new(),
        BranchExperimentBranchStatusV1::Pruned,
        "Combat",
        16,
        70,
    );

    assert!(campaign_parent_should_retry_combat_budget_now_v1(
        &config,
        &[abandoned_combat]
    ));
}

#[test]
fn campaign_parent_replay_retry_only_handles_boundary_drift_errors() {
    assert!(campaign_parent_replay_error_is_retryable_v1(
        "error: input `event 0` is not valid on the current screen: Combat"
    ));
    assert!(campaign_parent_replay_error_is_retryable_v1(
        "rp <idx> is only valid on a card reward item or card reward screen"
    ));
    assert!(!campaign_parent_replay_error_is_retryable_v1(
        "unknown command '__bad_internal_command'"
    ));
}
