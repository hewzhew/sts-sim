use super::*;
use crate::ai::noncombat_strategy_v1::{StrategyDeckFormationNeedV1, StrategyDeckFormationStageV1};
use crate::ai::strategic::{BranchSignature, BranchSignatureCompact, RetentionBucket};
use crate::content::cards::CardId;
use crate::eval::branch_experiment::{
    BranchExperimentBranchReportV1, BranchExperimentBranchStatusV1, BranchExperimentChoiceV1,
    BranchExperimentFrontierV1, BranchExperimentLineageV1, BranchExperimentRunSummaryV1,
};
use crate::eval::branch_experiment_retention::{
    BranchRetentionDecisionV1, BranchRetentionRankAdjustmentV1, BranchRetentionSlotV1,
};
use crate::eval::branch_experiment_trajectory::BranchTrajectorySignatureV1;
use crate::eval::combat_lab_probe_v1::{CombatLabProbeDiagnosisV1, CombatLabProbePacketV1};
use crate::eval::run_control::{
    RunControlConfig, RunControlHpLossLimit, RunControlSearchCombatOptions, RunControlSession,
    RunControlSessionCheckpointV1,
};
use crate::state::core::EngineState;
use crate::state::events::{EventId, EventState};
use crate::state::rewards::{RewardCard, RewardItem, RewardState};
use std::collections::BTreeSet;

mod frozen_pool_tests;
mod intervention_tests;
mod report_tests;
mod retry_tests;
mod selection_tests;
mod state_store_tests;

#[test]
fn campaign_victory_quality_gate_keeps_searching_after_low_hp_win() {
    let config = BranchCampaignConfigV1::default();
    let low_victory = {
        let mut branch = test_campaign_branch("low-win", 48, 6);
        branch.status = BranchCampaignBranchStatusV1::TerminalVictory;
        branch
    };
    let active = vec![test_campaign_branch("still-live", 47, 35)];
    let frozen = vec![test_campaign_branch("backup", 47, 30)];

    assert!(!campaign_should_stop_after_victory_v1(
        &config,
        &[low_victory],
        &active,
        &frozen
    ));
}

#[test]
fn campaign_victory_quality_gate_accepts_healthy_win() {
    let config = BranchCampaignConfigV1::default();
    let healthy_victory = {
        let mut branch = test_campaign_branch("healthy-win", 48, 24);
        branch.status = BranchCampaignBranchStatusV1::TerminalVictory;
        branch
    };
    let active = vec![test_campaign_branch("still-live", 47, 35)];

    assert!(campaign_should_stop_after_victory_v1(
        &config,
        &[healthy_victory],
        &active,
        &[]
    ));
}

#[test]
fn campaign_branch_from_report_appends_new_choice_path() {
    let parent = BranchCampaignBranchV1 {
        branch_id: "root".to_string(),
        commands: vec!["rp 0".to_string()],
        choice_labels: vec!["Shockwave".to_string()],
        summary: None,
        strategic_summary: Default::default(),
        frontier_title: "Card Reward".to_string(),
        status: BranchCampaignBranchStatusV1::Active,
        stop_reason: "test".to_string(),
        lineage_decision_signal_rank_adjustment: 0,
        rank_key: 0,
        final_boss_combat_record: None,
        combat_lab_probes: Vec::new(),
    };
    let report_branch = test_report_branch(
        "root.rp 1",
        vec![("rp 1", "True Grit")],
        BranchExperimentBranchStatusV1::Active,
    );

    let child = campaign_branch_from_report_branch_v1(&parent, &report_branch);

    assert_eq!(child.branch_id, "root.rp 1");
    assert_eq!(child.commands, vec!["rp 0", "rp 1"]);
    assert_eq!(child.choice_labels, vec!["Shockwave", "True Grit"]);
    assert_eq!(child.frontier_title, "Card Reward");
}

#[test]
fn campaign_branch_from_report_ignores_deprecated_lineage_decision_signal() {
    let mut parent = test_campaign_branch("root", 4, 80);
    parent.lineage_decision_signal_rank_adjustment = -830;
    parent.rank_key = 12_000;

    let mut report_branch = test_report_branch(
        "root.event 1",
        vec![("event 1", "costly event choice")],
        BranchExperimentBranchStatusV1::Active,
    );
    report_branch.rank_key = 21_500;
    report_branch.retention.rank_adjustment = BranchRetentionRankAdjustmentV1 {
        decision_signal_adjustment: -100,
        effective_rank_key: 21_500,
        ..BranchRetentionRankAdjustmentV1::default()
    };

    let child = campaign_branch_from_report_branch_v1(&parent, &report_branch);

    assert_eq!(child.rank_key, 21_500);
    assert_eq!(child.lineage_decision_signal_rank_adjustment, 0);
}

#[test]
fn campaign_branch_from_report_prefixes_parent_branch_id() {
    let parent = BranchCampaignBranchV1 {
        branch_id: "root.rp 0.branch-skip-card-reward 0".to_string(),
        commands: vec!["rp 0".to_string(), "branch-skip-card-reward 0".to_string()],
        choice_labels: vec!["Shockwave".to_string(), "Skip card reward".to_string()],
        summary: None,
        strategic_summary: Default::default(),
        frontier_title: "Card Reward".to_string(),
        status: BranchCampaignBranchStatusV1::Active,
        stop_reason: "test".to_string(),
        lineage_decision_signal_rank_adjustment: 0,
        rank_key: 0,
        final_boss_combat_record: None,
        combat_lab_probes: Vec::new(),
    };
    let report_branch = test_report_branch(
        "root.branch-skip-card-reward 0",
        vec![("branch-skip-card-reward 0", "Skip card reward")],
        BranchExperimentBranchStatusV1::Active,
    );

    let child = campaign_branch_from_report_branch_v1(&parent, &report_branch);

    assert_eq!(
        child.branch_id,
        "root.rp 0.branch-skip-card-reward 0.branch-skip-card-reward 0"
    );
    assert_eq!(
        child.commands,
        vec![
            "rp 0",
            "branch-skip-card-reward 0",
            "branch-skip-card-reward 0"
        ]
    );
}

#[test]
fn campaign_replay_prefix_advances_before_each_recorded_choice() {
    let replay = campaign_replay_commands_for_path_v1(&["rp 0".to_string(), "event 1".to_string()]);

    assert_eq!(
        replay,
        vec![
            BRANCH_EXPERIMENT_REPLAY_ADVANCE_COMMAND,
            "rp 0",
            BRANCH_EXPERIMENT_REPLAY_ADVANCE_COMMAND,
            "event 1"
        ]
    );
}

fn test_campaign_request(kind: &str, boundary_title: &str) -> BranchCampaignStrategyRequestV1 {
    BranchCampaignStrategyRequestV1 {
        kind: kind.to_string(),
        boundary_title: boundary_title.to_string(),
        branch_count: 1,
        act: 0,
        floor: 0,
        stop_reasons: Vec::new(),
        examples: Vec::new(),
        next_card_reward_offer: None,
        boundary_details: Vec::new(),
        suggested_action: "test".to_string(),
    }
}

fn test_campaign_branch_with_boundary(
    id: &str,
    frontier_title: &str,
    stop_reason: &str,
    floor: i32,
    hp: i32,
) -> BranchCampaignBranchV1 {
    let mut branch = test_campaign_branch(id, floor, hp);
    branch.frontier_title = frontier_title.to_string();
    branch.stop_reason = stop_reason.to_string();
    branch
}

fn test_experiment_request(
    kind: &str,
    boundary_title: &str,
    stop_reason: &str,
) -> crate::eval::branch_experiment::BranchExperimentStrategyRequestV1 {
    crate::eval::branch_experiment::BranchExperimentStrategyRequestV1 {
        kind: kind.to_string(),
        boundary_title: boundary_title.to_string(),
        branch_count: 1,
        representative_branch_id: "b".to_string(),
        act: 1,
        floor: 6,
        stop_reasons: vec![stop_reason.to_string()],
        examples: vec!["example".to_string()],
        next_card_reward_offer: None,
        boundary_details: Vec::new(),
        suggested_action: "test".to_string(),
    }
}

#[test]
fn campaign_progress_events_render_concrete_stage_information() {
    let branch_line =
        render_branch_campaign_progress_event_v1(&BranchCampaignProgressEventV1::BranchFinished {
            round: 2,
            branch_index: 1,
            branch_count: 2,
            produced_branches: 8,
            explored_branch_points: 6,
            elapsed_wall_ms: 1234,
            start_elapsed_wall_ms: 4321,
            combat_budget_retry_used: true,
            wall_limit_hit: false,
            branch_limit_hit: true,
        });
    let round_line =
        render_branch_campaign_progress_event_v1(&BranchCampaignProgressEventV1::RoundStarted {
            round: 2,
            max_rounds: 4,
            active_branches: 2,
            frozen_branches: 6,
        });
    let promoted_line =
        render_branch_campaign_progress_event_v1(&BranchCampaignProgressEventV1::FrozenPromoted {
            promoted: 2,
            active_after: 2,
            frozen_remaining: 4,
            filled_active: 0,
            stronger_rebalanced: 1,
            diversity_rebalanced: 1,
            rehydrated_recovered: 0,
            checkpoint_recovered: 0,
        });

    assert_eq!(
        branch_line,
        "round 2: branch 1/2 done | produced=8 branch_points=6 elapsed_ms=1234 start_ms=4321 retry=combat_budget limits=[branch]"
    );
    assert_eq!(
        round_line,
        "round 2/4: advancing 2 active branch(es), frozen=6"
    );
    assert_eq!(
        promoted_line,
        "promoted/rebalanced 2 frozen branch(es); active_after=2 frozen=4 sources=[stronger=1 diversity=1]"
    );
}

#[test]
fn campaign_progress_summary_hides_noisy_branch_events() {
    let branch_started = render_branch_campaign_progress_event_with_detail_v1(
        &BranchCampaignProgressEventV1::BranchStarted {
            round: 2,
            branch_index: 1,
            branch_count: 2,
            choices: "Pommel Strike -> Shrug It Off".to_string(),
        },
        BranchCampaignProgressDetailV1::Summary,
    );
    let quick_branch_done = render_branch_campaign_progress_event_with_detail_v1(
        &BranchCampaignProgressEventV1::BranchFinished {
            round: 2,
            branch_index: 1,
            branch_count: 2,
            produced_branches: 3,
            explored_branch_points: 1,
            elapsed_wall_ms: 900,
            start_elapsed_wall_ms: 0,
            combat_budget_retry_used: false,
            wall_limit_hit: false,
            branch_limit_hit: false,
        },
        BranchCampaignProgressDetailV1::Summary,
    );
    let slow_branch_done = render_branch_campaign_progress_event_with_detail_v1(
        &BranchCampaignProgressEventV1::BranchFinished {
            round: 2,
            branch_index: 2,
            branch_count: 2,
            produced_branches: 4,
            explored_branch_points: 2,
            elapsed_wall_ms: 6_200,
            start_elapsed_wall_ms: 0,
            combat_budget_retry_used: false,
            wall_limit_hit: true,
            branch_limit_hit: false,
        },
        BranchCampaignProgressDetailV1::Summary,
    )
    .expect("slow branch should be visible in summary progress");

    assert_eq!(branch_started, None);
    assert_eq!(quick_branch_done, None);
    assert_eq!(
        slow_branch_done,
        "round 2: branch 2/2 done produced=4 branch_points=2 | elapsed=6.2s limits=[wall]"
    );
}

#[test]
fn campaign_round_summary_persists_timing_metrics() {
    let summary = BranchCampaignRoundSummaryV1 {
        round: 3,
        started_active: 2,
        produced_branches: 5,
        active_after: 2,
        frozen_added: 1,
        dead_added: 0,
        abandoned_added: 1,
        victories_added: 0,
        stuck_added: 0,
        discarded_added: 1,
        explored_branch_points: 4,
        wall_limit_hit: false,
        branch_limit_hit: true,
        combat_budget_retries: 1,
        elapsed_wall_ms: 7_000,
        parent_elapsed_wall_ms_sum: 6_500,
        parent_elapsed_wall_ms_max: 4_000,
        combat_retry_elapsed_wall_ms_sum: 3_000,
        combat_retry_elapsed_wall_ms_max: 3_000,
        combat_performance: BranchCampaignCombatPerformanceSummaryV1::default(),
    };

    let value = serde_json::to_value(summary).expect("round summary should serialize");

    assert_eq!(value["elapsed_wall_ms"], 7_000);
    assert_eq!(value["parent_elapsed_wall_ms_sum"], 6_500);
    assert_eq!(value["parent_elapsed_wall_ms_max"], 4_000);
    assert_eq!(value["combat_retry_elapsed_wall_ms_sum"], 3_000);
    assert_eq!(value["combat_retry_elapsed_wall_ms_max"], 3_000);
}

#[test]
fn compact_campaign_report_surfaces_timing_summary() {
    let mut report = test_campaign_report_with_active("timed", 12, 80);
    report.rounds[0].elapsed_wall_ms = 9_000;
    report.rounds[0].parent_elapsed_wall_ms_sum = 8_000;
    report.rounds[0].parent_elapsed_wall_ms_max = 5_000;
    report.rounds[0].combat_retry_elapsed_wall_ms_sum = 3_000;
    report.rounds[0].combat_retry_elapsed_wall_ms_max = 3_000;

    let rendered = render_branch_campaign_compact_with_detail_v1(
        &report,
        1,
        BranchCampaignReportDetailV1::Perf,
    );

    assert!(rendered.contains(
        "Timing: rounds=9.0s parent_sum=8.0s parent_max=5.0s combat_retry_sum=3.0s combat_retry_max=3.0s"
    ));
}

#[test]
fn campaign_round_retry_counts_parent_elapsed_as_retry_timing() {
    assert_eq!(
        campaign_retry_timing_for_parent_v1(true, 5_000, 3_000, 0, 0),
        (5_000, 3_000)
    );
    assert_eq!(
        campaign_retry_timing_for_parent_v1(false, 5_000, 3_000, 1_500, 1_000),
        (1_500, 1_000)
    );
}

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
            strategy_requests: Vec::new(),
            route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
            combat_retry_ledger: BranchCampaignCombatRetryLedgerStateV1::default(),
            rounds: Vec::new(),
            state_store: {
                let mut store = super::state_graph::BranchStateStoreV1::new();
                store.insert_session(parent.commands.clone(), session);
                store
            },
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
        strategy_requests: Vec::new(),
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerStateV1::default(),
        rounds: Vec::new(),
        state_store: {
            let mut store = super::state_graph::BranchStateStoreV1::new();
            store.insert_session(abandoned.commands.clone(), abandoned_session);
            store.insert_session(stuck.commands.clone(), stuck_session);
            store
        },
        recovered_checkpoint_failure_commands: BTreeSet::new(),
    };

    let checkpoint = campaign_checkpoint_from_state_v1(&config, &state);
    let commands = checkpoint
        .sessions
        .iter()
        .map(|entry| entry.commands.clone())
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
        strategy_requests: vec![request],
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
        rounds: Vec::new(),
    };
    let checkpoint = BranchCampaignCheckpointV1 {
        schema_name: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION,
        seed: 1,
        run_domain: BranchCampaignRunDomainV1::default(),
        rounds_completed: 0,
        nodes: Vec::new(),
        sessions: vec![BranchCampaignCheckpointSessionV1 {
            commands: restorable.commands.clone(),
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
        strategy_requests: Vec::new(),
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
        rounds: Vec::new(),
    };
    let checkpoint = BranchCampaignCheckpointV1 {
        schema_name: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION,
        seed: 1,
        run_domain: BranchCampaignRunDomainV1::default(),
        rounds_completed: 0,
        nodes: Vec::new(),
        sessions: vec![BranchCampaignCheckpointSessionV1 {
            commands: parent.commands.clone(),
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
        strategy_requests: Vec::new(),
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
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
        rounds_completed: 8,
        nodes: Vec::new(),
        sessions: vec![BranchCampaignCheckpointSessionV1 {
            commands: combat_failure.commands.clone(),
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
        strategy_requests: Vec::new(),
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
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
        rounds_completed: 48,
        nodes: Vec::new(),
        sessions: vec![BranchCampaignCheckpointSessionV1 {
            commands: stale_combat_failure.commands.clone(),
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
        strategy_requests: vec![request],
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
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
        rounds_completed: 8,
        nodes: Vec::new(),
        sessions: vec![BranchCampaignCheckpointSessionV1 {
            commands: map_overlay_stuck.commands.clone(),
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
        strategy_requests: vec![request],
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
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
        rounds_completed: 8,
        nodes: Vec::new(),
        sessions: vec![BranchCampaignCheckpointSessionV1 {
            commands: map_overlay_stuck.commands.clone(),
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
        strategy_requests: vec![request],
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
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
        rounds_completed: 8,
        nodes: Vec::new(),
        sessions: vec![BranchCampaignCheckpointSessionV1 {
            commands: map_overlay_stuck.commands.clone(),
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
            commands: branch.commands.clone(),
            session: RunControlSessionCheckpointV1::from_session(&session),
        });
        abandoned.push(branch);
    }

    let previous = BranchCampaignReportV1 {
        schema_name: BRANCH_CAMPAIGN_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_SCHEMA_VERSION,
        seed: 1,
        run_domain: BranchCampaignRunDomainV1::default(),
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
        strategy_requests: Vec::new(),
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
        rounds: Vec::new(),
    };
    let checkpoint = BranchCampaignCheckpointV1 {
        schema_name: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION,
        seed: 1,
        run_domain: BranchCampaignRunDomainV1::default(),
        rounds_completed: 8,
        nodes: Vec::new(),
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
        rounds_completed: 8,
        nodes: Vec::new(),
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
        strategy_requests: Vec::new(),
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
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

#[test]
fn campaign_status_distinguishes_pruned_from_terminal_defeat() {
    assert_eq!(
        campaign_status_from_report_status(BranchExperimentBranchStatusV1::Pruned),
        BranchCampaignBranchStatusV1::Abandoned
    );
    assert_eq!(
        campaign_status_from_report_status(BranchExperimentBranchStatusV1::TerminalDefeat),
        BranchCampaignBranchStatusV1::TerminalDefeat
    );
}

#[test]
fn campaign_branch_preserves_final_boss_combat_record_from_experiment_report() {
    let parent = test_campaign_branch("parent", 47, 70);
    let mut branch = test_report_branch(
        "winner",
        vec![("rp 0", "Limit Break")],
        BranchExperimentBranchStatusV1::TerminalVictory,
    );
    branch.final_boss_combat_record = Some(
        crate::eval::branch_experiment::BranchExperimentBossCombatRecordV1 {
            source: "final_boss_combat".to_string(),
            action_count: 1,
            actions: vec![crate::eval::run_control::CombatAutomationActionV1 {
                step_index: 0,
                action_key: "end".to_string(),
                input: crate::state::core::ClientInput::EndTurn,
                drawn_cards: Vec::new(),
                combat_after: None,
            }],
            label_role: "behavior_policy_not_teacher".to_string(),
        },
    );

    let campaign_branch = campaign_branch_from_report_branch_v1(&parent, &branch);

    let record = campaign_branch
        .final_boss_combat_record
        .expect("campaign branch should keep the experiment final boss combat record");
    assert_eq!(record.source, "final_boss_combat");
    assert_eq!(record.action_count, 1);
    assert_eq!(record.actions[0].action_key, "end");
}

fn test_campaign_report_with_active(id: &str, floor: i32, hp: i32) -> BranchCampaignReportV1 {
    BranchCampaignReportV1 {
        schema_name: BRANCH_CAMPAIGN_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_SCHEMA_VERSION,
        seed: 521,
        run_domain: BranchCampaignRunDomainV1::default(),
        rounds_completed: 3,
        stop_reason: "max_rounds".to_string(),
        active: vec![test_campaign_branch(id, floor, hp)],
        frozen: vec![test_campaign_branch("frozen-a", floor - 1, hp - 5)],
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: Vec::new(),
        stuck: Vec::new(),
        discarded_count: 4,
        discarded_examples: Vec::new(),
        strategy_requests: Vec::new(),
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
        rounds: vec![BranchCampaignRoundSummaryV1 {
            round: 1,
            started_active: 1,
            produced_branches: 2,
            active_after: 1,
            frozen_added: 1,
            dead_added: 0,
            abandoned_added: 0,
            victories_added: 0,
            stuck_added: 0,
            discarded_added: 0,
            explored_branch_points: 1,
            wall_limit_hit: false,
            branch_limit_hit: false,
            combat_budget_retries: 0,
            ..BranchCampaignRoundSummaryV1::default()
        }],
    }
}

fn test_campaign_branch(id: &str, floor: i32, hp: i32) -> BranchCampaignBranchV1 {
    BranchCampaignBranchV1 {
        branch_id: id.to_string(),
        commands: Vec::new(),
        choice_labels: Vec::new(),
        summary: Some(BranchCampaignBranchSummaryV1 {
            act: 1,
            floor,
            hp,
            max_hp: 80,
            gold: 99,
            deck_count: 10,
            deck_key: String::new(),
            formation_stage: "PlanSeeded".to_string(),
            formation_strengths: Vec::new(),
            formation_needs: Vec::new(),
            trajectory_key: "frontload=1".to_string(),
            boss: String::new(),
            boss_pressure: Vec::new(),
            run_debt: Vec::new(),
            event_boundary: None,
            reward_boundary: None,
        }),
        strategic_summary: Default::default(),
        frontier_title: "Card Reward".to_string(),
        status: BranchCampaignBranchStatusV1::Active,
        stop_reason: "test".to_string(),
        lineage_decision_signal_rank_adjustment: 0,
        rank_key: hp,
        final_boss_combat_record: None,
        combat_lab_probes: Vec::new(),
    }
}

fn test_combat_checkpoint_session(
    branch: &BranchCampaignBranchV1,
    act: u8,
    floor: i32,
    hp: i32,
) -> BranchCampaignCheckpointSessionV1 {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.engine_state = EngineState::CombatPlayerTurn;
    session.run_state.act_num = act;
    session.run_state.floor_num = floor;
    session.run_state.current_hp = hp;
    session.run_state.max_hp = 80;
    BranchCampaignCheckpointSessionV1 {
        commands: branch.commands.clone(),
        session: RunControlSessionCheckpointV1::from_session(&session),
    }
}

fn test_report_branch(
    id: &str,
    choices: Vec<(&str, &str)>,
    status: BranchExperimentBranchStatusV1,
) -> BranchExperimentBranchReportV1 {
    test_report_branch_at(id, choices, status, "Card Reward", 2, 70)
}

fn test_report_branch_at(
    id: &str,
    choices: Vec<(&str, &str)>,
    status: BranchExperimentBranchStatusV1,
    boundary_title: &str,
    floor: i32,
    hp: i32,
) -> BranchExperimentBranchReportV1 {
    BranchExperimentBranchReportV1 {
        branch_id: id.to_string(),
        status,
        rank_key: 10,
        retention: BranchRetentionDecisionV1 {
            primary_slot: BranchRetentionSlotV1::Diversity,
            selected_by_slot: Some(BranchRetentionSlotV1::Diversity),
            slots: vec![BranchRetentionSlotV1::Diversity],
            reasons: vec!["test".to_string()],
            strategic_signature: Default::default(),
            coverage_selection: Default::default(),
            rank_adjustment: Default::default(),
        },
        choices: choices
            .into_iter()
            .map(|(command, label)| BranchExperimentChoiceV1 {
                depth: 0,
                kind: "card_reward".to_string(),
                boundary_title: "Card Reward".to_string(),
                card: None,
                upgrades: None,
                selected_cards: Vec::new(),
                effect_kind: "take_card".to_string(),
                effect_key: label.to_string(),
                effect_label: label.to_string(),
                representative_count: 1,
                suppressed_count: 0,
                decision_signal: None,
                label: label.to_string(),
                command: command.to_string(),
            })
            .collect(),
        stop_reason: "test".to_string(),
        summary: BranchExperimentRunSummaryV1 {
            act: 1,
            floor,
            hp,
            max_hp: 80,
            gold: 120,
            deck_count: 11,
            relic_count: 1,
            potion_count: 0,
            formation_stage: StrategyDeckFormationStageV1::PlanSeeded,
            formation_needs: vec![StrategyDeckFormationNeedV1::Frontload],
            formation_strengths: Vec::new(),
            trajectory: BranchTrajectorySignatureV1::default(),
            boundary_title: boundary_title.to_string(),
        },
        frontier: BranchExperimentFrontierV1 {
            key: boundary_title.to_ascii_lowercase(),
            act: 1,
            floor,
            boundary_title: boundary_title.to_string(),
            card_rng_counter: 0,
            card_blizz_randomizer: 0,
            next_card_reward_offer: None,
            lineage: BranchExperimentLineageV1 {
                visibility: "test".to_string(),
                public_policy_input: true,
                direct_pick_consumes_card_rng: false,
                same_reward_offer_lineage_key: "test".to_string(),
                reward_screen_context: "test".to_string(),
                reward_count_modifiers: Vec::new(),
                card_pool_modifiers: Vec::new(),
                rarity_modifiers: Vec::new(),
                preview_modifiers: Vec::new(),
                sequence_breakers_present: Vec::new(),
            },
        },
        boundary_details: Vec::new(),
        final_boss_combat_record: None,
    }
}
