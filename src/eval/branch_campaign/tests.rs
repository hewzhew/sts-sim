use super::*;
use crate::ai::noncombat_strategy_v1::{StrategyDeckFormationNeedV1, StrategyDeckFormationStageV1};
use crate::content::cards::CardId;
use crate::eval::branch_experiment::{
    BranchExperimentBranchReportV1, BranchExperimentBranchStatusV1, BranchExperimentChoiceV1,
    BranchExperimentFrontierV1, BranchExperimentLineageV1, BranchExperimentRunSummaryV1,
};
use crate::eval::branch_experiment_retention::{BranchRetentionDecisionV1, BranchRetentionSlotV1};
use crate::eval::branch_experiment_trajectory::BranchTrajectorySignatureV1;
use crate::eval::run_control::{
    RunControlConfig, RunControlSession, RunControlSessionCheckpointV1,
};
use crate::state::core::EngineState;
use crate::state::rewards::{RewardCard, RewardState};
use std::collections::BTreeMap;

#[test]
fn campaign_selection_freezes_active_overflow() {
    let branches = vec![
        test_campaign_branch("a", 1, 80),
        test_campaign_branch("b", 2, 75),
        test_campaign_branch("c", 3, 70),
    ];

    let selected = select_campaign_branches_v1(branches, 2, 4);

    assert_eq!(selected.active.len(), 2);
    assert_eq!(selected.frozen.len(), 1);
    assert_eq!(selected.active[0].branch_id, "a");
    assert_eq!(selected.active[1].branch_id, "b");
    assert_eq!(selected.frozen[0].branch_id, "c");
}

#[test]
fn campaign_promotes_frozen_when_active_pool_is_empty() {
    let mut active = Vec::new();
    let mut frozen = vec![
        {
            let mut branch = test_campaign_branch("f1", 4, 80);
            branch.status = BranchCampaignBranchStatusV1::Frozen;
            branch
        },
        {
            let mut branch = test_campaign_branch("f2", 7, 75);
            branch.status = BranchCampaignBranchStatusV1::Frozen;
            branch
        },
    ];

    let promoted = promote_frozen_to_active_v1(&mut active, &mut frozen, 1);

    assert_eq!(promoted, 1);
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].branch_id, "f2");
    assert_eq!(active[0].status, BranchCampaignBranchStatusV1::Active);
    assert_eq!(frozen.len(), 1);
    assert_eq!(frozen[0].branch_id, "f1");
}

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
    assert_eq!(retry.search_wall_ms, Some(1_200));
    assert_eq!(
        retry.search_max_hp_loss,
        Some(RunControlHpLossLimit::Unlimited)
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
        16,
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

#[test]
fn campaign_branch_from_report_appends_new_choice_path() {
    let parent = BranchCampaignBranchV1 {
        branch_id: "root".to_string(),
        commands: vec!["rp 0".to_string()],
        choice_labels: vec!["Shockwave".to_string()],
        summary: None,
        frontier_title: "Card Reward".to_string(),
        status: BranchCampaignBranchStatusV1::Active,
        rank_key: 0,
    };
    let report_branch = test_report_branch(
        "root.rp 1",
        vec![("rp 1", "True Grit")],
        BranchExperimentBranchStatusV1::Active,
    );

    let child = campaign_branch_from_report_branch_v1(&parent, &report_branch);

    assert_eq!(child.commands, vec!["rp 0", "rp 1"]);
    assert_eq!(child.choice_labels, vec!["Shockwave", "True Grit"]);
    assert_eq!(child.frontier_title, "Card Reward");
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

#[test]
fn compact_campaign_report_renders_strategy_prompt() {
    let report = BranchCampaignReportV1 {
        schema_name: "BranchCampaignV1".to_string(),
        schema_version: 1,
        seed: 521,
        rounds_completed: 2,
        stop_reason: "needs_intervention".to_string(),
        active: vec![test_campaign_branch("a", 5, 70)],
        frozen: vec![test_campaign_branch("f", 4, 65)],
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: Vec::new(),
        stuck: Vec::new(),
        discarded_count: 3,
        strategy_requests: vec![BranchCampaignStrategyRequestV1 {
            kind: "event_strategy".to_string(),
            boundary_title: "Falling".to_string(),
            branch_count: 2,
            stop_reasons: vec!["event policy gap".to_string()],
            examples: vec!["Strike -> Defend".to_string()],
            suggested_action: "provide Falling policy".to_string(),
        }],
        rounds: Vec::new(),
    };

    let rendered = render_branch_campaign_compact_v1(&report, 2);

    assert!(rendered.contains("BranchCampaignV1 seed=521 rounds=2 stop=needs_intervention"));
    assert!(rendered.contains(
        "Active 1 | Frozen 1 | Dead 0 | Abandoned 0 | Victories 0 | Stuck 0 | Discarded 3"
    ));
    assert!(rendered.contains("Needs intervention:"));
    assert!(rendered.contains("event_strategy | Falling | branches=2"));
    assert!(rendered.contains(
        "next: write a narrow event rule or choose one branch manually, then rerun the campaign"
    ));
    assert!(rendered.contains("Top active:"));
}

#[test]
fn compact_campaign_report_renders_actionable_intervention_details() {
    let report = BranchCampaignReportV1 {
        schema_name: "BranchCampaignV1".to_string(),
        schema_version: 1,
        seed: 521,
        rounds_completed: 6,
        stop_reason: "needs_intervention".to_string(),
        active: Vec::new(),
        frozen: Vec::new(),
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: vec![test_campaign_branch("a", 16, 70)],
        stuck: Vec::new(),
        discarded_count: 0,
        strategy_requests: vec![BranchCampaignStrategyRequestV1 {
            kind: "combat_manual_or_budget".to_string(),
            boundary_title: "Combat".to_string(),
            branch_count: 2,
            stop_reasons: vec!["combat search did not find an executable complete win".to_string()],
            examples: vec!["Clash -> Disarm".to_string()],
            suggested_action: "raise combat search budget".to_string(),
        }],
        rounds: vec![BranchCampaignRoundSummaryV1 {
            round: 5,
            started_active: 2,
            produced_branches: 2,
            active_after: 0,
            frozen_added: 0,
            dead_added: 0,
            abandoned_added: 2,
            victories_added: 0,
            stuck_added: 0,
            discarded_added: 0,
            explored_branch_points: 0,
            wall_limit_hit: false,
            branch_limit_hit: false,
            combat_budget_retries: 2,
        }],
    };

    let rendered = render_branch_campaign_compact_v1(&report, 2);

    assert!(rendered.contains("Needs intervention:"));
    assert!(rendered.contains("kind: combat_unresolved_after_retry"));
    assert!(rendered.contains("stop: combat search did not find an executable complete win"));
    assert!(rendered.contains("tried: campaign search budget; combat budget retry x2"));
    assert!(rendered.contains("options: raise combat retry budget | provide a manual combat line | abandon this macro route family"));
}

#[test]
fn compact_campaign_report_renders_queued_interventions_while_continuing() {
    let report = BranchCampaignReportV1 {
        schema_name: "BranchCampaignV1".to_string(),
        schema_version: 1,
        seed: 330404105,
        rounds_completed: 2,
        stop_reason: "max_rounds".to_string(),
        active: vec![test_campaign_branch("a", 6, 80)],
        frozen: vec![test_campaign_branch("f", 5, 75)],
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: Vec::new(),
        stuck: vec![test_campaign_branch("s", 6, 70)],
        discarded_count: 0,
        strategy_requests: vec![BranchCampaignStrategyRequestV1 {
            kind: "combat_manual_or_budget".to_string(),
            boundary_title: "Combat".to_string(),
            branch_count: 1,
            stop_reasons: vec!["combat search did not find an executable complete win".to_string()],
            examples: vec!["Flex".to_string()],
            suggested_action: "raise combat search budget".to_string(),
        }],
        rounds: Vec::new(),
    };

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains("Queued interventions:"));
    assert!(!rendered.contains("Needs intervention:"));
    assert!(rendered.contains("stop: combat search did not find an executable complete win"));
}

#[test]
fn campaign_builds_intervention_when_abandoned_branches_exhaust_routes() {
    let request = abandoned_branches_intervention_request_v1(&[
        test_campaign_branch("a", 16, 70),
        test_campaign_branch("b", 16, 62),
    ])
    .expect("request");

    assert_eq!(request.kind, "combat_manual_or_budget");
    assert_eq!(request.boundary_title, "Combat");
    assert_eq!(request.branch_count, 2);
    assert_eq!(request.examples.len(), 2);
}

#[test]
fn campaign_strategy_requests_are_fatal_only_without_continuable_branches() {
    let active = vec![test_campaign_branch("a", 6, 80)];
    let frozen = vec![test_campaign_branch("f", 5, 75)];
    let request = vec![test_campaign_request("combat_manual_or_budget", "Combat")];

    assert!(!campaign_strategy_requests_are_fatal_v1(
        &active,
        &[],
        &request
    ));
    assert!(!campaign_strategy_requests_are_fatal_v1(
        &[],
        &frozen,
        &request
    ));
    assert!(campaign_strategy_requests_are_fatal_v1(&[], &[], &request));
}

#[test]
fn campaign_strategy_request_merge_keeps_stop_reasons() {
    let merged = merge_campaign_strategy_requests_v1(vec![test_experiment_request(
        "combat_manual_or_budget",
        "Combat",
        "combat search did not find an executable complete win",
    )]);

    assert_eq!(
        merged[0].stop_reasons,
        vec!["combat search did not find an executable complete win".to_string()]
    );
}

#[test]
fn campaign_strategy_request_merge_sanitizes_combat_suggestion() {
    let mut request = test_experiment_request(
        "combat_manual_or_budget",
        "Combat",
        "combat search did not find an executable complete win",
    );
    request.suggested_action =
        "raise combat search budget, relax hp-loss gate, or provide a manual combat line"
            .to_string();

    let merged = merge_campaign_strategy_requests_v1(vec![request]);

    assert!(!merged[0].suggested_action.contains("hp-loss"));
    assert!(!merged[0].suggested_action.contains("relax"));
}

#[test]
fn combat_campaign_next_step_no_longer_mentions_hp_loss_gate() {
    let next = campaign_strategy_next_step_v1("combat_manual_or_budget").expect("next step");

    assert!(!next.contains("hp-loss"));
    assert!(!next.contains("relax"));
}

#[test]
fn compact_campaign_report_renders_budget_stop_hint() {
    let report = BranchCampaignReportV1 {
        schema_name: "BranchCampaignV1".to_string(),
        schema_version: 1,
        seed: 521,
        rounds_completed: 2,
        stop_reason: "max_rounds".to_string(),
        active: vec![test_campaign_branch("a", 7, 80)],
        frozen: Vec::new(),
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: Vec::new(),
        stuck: Vec::new(),
        discarded_count: 0,
        strategy_requests: Vec::new(),
        rounds: Vec::new(),
    };

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains("Next: budget ended; use .\\tools\\campaign.ps1 -More"));
}

fn test_campaign_request(kind: &str, boundary_title: &str) -> BranchCampaignStrategyRequestV1 {
    BranchCampaignStrategyRequestV1 {
        kind: kind.to_string(),
        boundary_title: boundary_title.to_string(),
        branch_count: 1,
        stop_reasons: Vec::new(),
        examples: Vec::new(),
        suggested_action: "test".to_string(),
    }
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
        "promoted 2 frozen branch(es) after active branches ran out; active_after=2 frozen=4"
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
            strategy_requests: Vec::new(),
            rounds: Vec::new(),
            snapshot_cache: BTreeMap::from([(parent.commands.clone(), session)]),
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
    assert!(branch_labels
        .iter()
        .any(|label| label.contains("Twin Strike")));
    assert!(branch_labels.iter().any(|label| label.contains("Cleave")));
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
        rounds_completed: 0,
        stop_reason: "max_rounds".to_string(),
        active: vec![parent.clone()],
        frozen: Vec::new(),
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: Vec::new(),
        stuck: Vec::new(),
        discarded_count: 0,
        strategy_requests: Vec::new(),
        rounds: Vec::new(),
    };
    let checkpoint = BranchCampaignCheckpointV1 {
        schema_name: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION,
        seed: 1,
        rounds_completed: 0,
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
    assert!(branch_labels
        .iter()
        .any(|label| label.contains("Twin Strike")));
    assert!(branch_labels.iter().any(|label| label.contains("Cleave")));
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

fn test_campaign_report_with_active(id: &str, floor: i32, hp: i32) -> BranchCampaignReportV1 {
    BranchCampaignReportV1 {
        schema_name: BRANCH_CAMPAIGN_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_SCHEMA_VERSION,
        seed: 521,
        rounds_completed: 3,
        stop_reason: "max_rounds".to_string(),
        active: vec![test_campaign_branch(id, floor, hp)],
        frozen: vec![test_campaign_branch("frozen-a", floor - 1, hp - 5)],
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: Vec::new(),
        stuck: Vec::new(),
        discarded_count: 4,
        strategy_requests: Vec::new(),
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
            formation_stage: "PlanSeeded".to_string(),
            formation_strengths: Vec::new(),
            formation_needs: Vec::new(),
        }),
        frontier_title: "Card Reward".to_string(),
        status: BranchCampaignBranchStatusV1::Active,
        rank_key: hp,
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
        },
        choices: choices
            .into_iter()
            .map(|(command, label)| BranchExperimentChoiceV1 {
                depth: 0,
                kind: "card_reward".to_string(),
                card: None,
                upgrades: None,
                selected_cards: Vec::new(),
                effect_kind: "take_card".to_string(),
                effect_key: label.to_string(),
                effect_label: label.to_string(),
                representative_count: 1,
                suppressed_count: 0,
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
    }
}
