use super::*;
use crate::ai::noncombat_strategy_v1::{StrategyDeckFormationNeedV1, StrategyDeckFormationStageV1};
use crate::ai::strategic::{BranchSignature, RetentionBucket};
use crate::content::cards::CardId;
use crate::eval::branch_experiment::{
    BranchExperimentBranchReportV1, BranchExperimentBranchStatusV1, BranchExperimentChoiceV1,
    BranchExperimentFrontierV1, BranchExperimentLineageV1, BranchExperimentRunSummaryV1,
};
use crate::eval::branch_experiment_retention::{
    BranchRetentionDecisionV1, BranchRetentionRankAdjustmentV1, BranchRetentionSlotV1,
};
use crate::eval::branch_experiment_trajectory::BranchTrajectorySignatureV1;
use crate::eval::run_control::{
    CombatAutomationActionV1, CombatAutomationTrajectoryRecordV1, RunControlConfig,
    RunControlHpLossLimit, RunControlSearchCombatOptions, RunControlSession,
    RunControlSessionCheckpointV1,
};
use crate::state::core::EngineState;
use crate::state::events::{EventId, EventState};
use crate::state::rewards::{RewardCard, RewardItem, RewardState};
use std::collections::BTreeSet;

mod intervention_tests;
mod report_tests;
mod resume_tests;
mod retry_tests;
mod state_store_tests;

#[test]
fn route_evidence_merge_preserves_candidate_pool_only_batches() {
    let mut target = BranchCampaignRouteEvidenceSummaryV1::default();
    let incoming = BranchCampaignRouteEvidenceSummaryV1 {
        candidate_pools: 1,
        candidate_pool_candidates: 4,
        candidate_pool_ok: 3,
        complete_candidate_pools: 1,
        ..Default::default()
    };

    super::route_evidence::merge_campaign_route_evidence_summary_v1(&mut target, incoming);

    assert_eq!(target.decisions, 0);
    assert_eq!(target.candidate_pools, 1);
    assert_eq!(target.candidate_pool_candidates, 4);
    assert_eq!(target.candidate_pool_ok, 3);
    assert_eq!(target.complete_candidate_pools, 1);
}

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
        status: BranchCampaignBranchStatusV1::Scheduled,
        stop_reason: "test".to_string(),
        continuation_origin: None,
        decision_candidate_axis: None,
        lineage_decision_signal_rank_adjustment: 0,
        rank_key: 0,
        rank_breakdown: None,
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
        base_rank_key: 20_500,
        formation_need_adjustment: 1_000,
        decision_signal_adjustment: -100,
        effective_rank_key: 21_500,
        reasons: vec!["formation_need_rank_adjustment:1000".to_string()],
        ..BranchRetentionRankAdjustmentV1::default()
    };

    let child = campaign_branch_from_report_branch_v1(&parent, &report_branch);

    assert_eq!(child.rank_key, 21_500);
    assert_eq!(child.lineage_decision_signal_rank_adjustment, 0);
    let breakdown = child
        .rank_breakdown
        .as_ref()
        .expect("campaign branch should carry compact rank breakdown");
    assert_eq!(breakdown.base_rank_key, 20_500);
    assert_eq!(breakdown.formation_need_adjustment, 1_000);
    assert_eq!(breakdown.effective_rank_key, 21_500);
    assert_eq!(
        breakdown.reasons,
        vec!["formation_need_rank_adjustment:1000"]
    );
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
        status: BranchCampaignBranchStatusV1::Scheduled,
        stop_reason: "test".to_string(),
        continuation_origin: None,
        decision_candidate_axis: None,
        lineage_decision_signal_rank_adjustment: 0,
        rank_key: 0,
        rank_breakdown: None,
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
fn campaign_branch_from_report_preserves_explicit_decision_candidate_axis() {
    let parent = test_campaign_branch("root", 4, 80);
    let mut report_branch = test_report_branch(
        "root.buy",
        vec![("purge 0 && buy card 2", "Purge Strike then Buy Reaper")],
        BranchExperimentBranchStatusV1::Active,
    );
    report_branch.choices[0].kind = "shop_buy_combo".to_string();
    report_branch.choices[0].boundary_title = "Shop".to_string();
    report_branch.choices[0].effect_kind = "shop_buy_combo".to_string();
    report_branch.choices[0].candidate_axis = Some("shop:shop:purge_plus_buy_card".to_string());

    let child = campaign_branch_from_report_branch_v1(&parent, &report_branch);

    assert_eq!(
        child.decision_candidate_axis.as_deref(),
        Some("shop:shop:purge_plus_buy_card")
    );
}

#[test]
fn campaign_branch_from_report_combines_contiguous_shop_candidate_axes() {
    let parent = test_campaign_branch("root", 4, 80);
    let mut report_branch = test_report_branch(
        "root.shop",
        vec![
            ("purge 5", "Purge Defend"),
            ("buy card 2", "Buy Shrug It Off"),
        ],
        BranchExperimentBranchStatusV1::Active,
    );
    report_branch.choices[0].kind = "shop_policy_purge".to_string();
    report_branch.choices[0].boundary_title = "Shop".to_string();
    report_branch.choices[0].effect_kind = "shop_purge".to_string();
    report_branch.choices[0].candidate_axis = Some("shop:shop:purge_only".to_string());
    report_branch.choices[1].kind = "shop_buy_card".to_string();
    report_branch.choices[1].boundary_title = "Shop".to_string();
    report_branch.choices[1].effect_kind = "shop_buy_card".to_string();
    report_branch.choices[1].candidate_axis = Some("shop:shop:buy_card_only".to_string());

    let child = campaign_branch_from_report_branch_v1(&parent, &report_branch);

    assert_eq!(
        child.decision_candidate_axis.as_deref(),
        Some("shop:shop:purge_plus_buy_card")
    );
}

#[test]
fn scheduler_prioritizes_mainline_before_boss_relic_axis_probes() {
    let mut strong_general = test_campaign_branch("strong-general", 18, 80);
    strong_general.rank_key = 99_000;
    let mut second_general = test_campaign_branch("second-general", 18, 79);
    second_general.rank_key = 98_000;

    let mut pyramid = test_campaign_branch("pyramid", 16, 40);
    pyramid.choice_labels = vec!["RunicPyramid".to_string()];
    pyramid.rank_key = 1_000;
    let mut astrolabe = test_campaign_branch("astrolabe", 16, 35);
    astrolabe.choice_labels = vec!["Astrolabe".to_string()];
    astrolabe.rank_key = 500;
    let mut empty_cage = test_campaign_branch("empty-cage", 16, 30);
    empty_cage.choice_labels = vec!["EmptyCage".to_string()];
    empty_cage.rank_key = 100;

    let selected = select_campaign_branches_v1(
        vec![
            strong_general,
            second_general,
            pyramid,
            astrolabe,
            empty_cage,
        ],
        3,
        8,
    );
    let scheduled_ids = selected
        .scheduled
        .iter()
        .map(|branch| branch.branch_id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(selected.scheduled.len(), 3);
    assert!(scheduled_ids.contains(&"strong-general"));
    assert!(scheduled_ids.contains(&"second-general"));
    assert!(selected
        .scheduled
        .iter()
        .any(|branch| branch.stop_reason.contains("scheduler:boss_relic_axis")));
}

#[test]
fn scheduler_small_budget_runs_mainline_not_progress_probe() {
    let mut high_rank_lower_hp = test_campaign_branch("high-rank-lower-hp", 6, 69);
    high_rank_lower_hp.rank_key = 11_511;
    let mut lower_rank_full_hp = test_campaign_branch("lower-rank-full-hp", 6, 80);
    lower_rank_full_hp.rank_key = 11_500;

    let selected = select_campaign_branches_v1(vec![lower_rank_full_hp, high_rank_lower_hp], 2, 8);

    assert_eq!(selected.scheduled.len(), 2);
    assert!(selected
        .scheduled
        .iter()
        .all(|branch| branch.stop_reason.contains("scheduler:general")));
}

#[test]
fn scheduler_mainline_rank_key_beats_one_floor_progress() {
    let mut high_rank_shop = test_campaign_branch("high-rank-shop", 7, 79);
    high_rank_shop.frontier_title = "Shop".to_string();
    high_rank_shop.rank_key = 12_615;
    let mut lower_rank_campfire = test_campaign_branch("lower-rank-campfire", 8, 80);
    lower_rank_campfire.frontier_title = "Campfire".to_string();
    lower_rank_campfire.rank_key = 12_014;

    let selected = select_campaign_branches_v1(vec![lower_rank_campfire, high_rank_shop], 1, 8);

    assert_eq!(selected.scheduled.len(), 1);
    assert_eq!(selected.scheduled[0].branch_id, "high-rank-shop");
    assert!(selected.scheduled[0]
        .stop_reason
        .contains("scheduler:general"));
}

#[test]
fn scheduler_small_budget_keeps_top_ranked_decision_candidates() {
    let mut remove_strike = test_campaign_branch("remove-strike", 6, 80);
    remove_strike.rank_key = 12_000;
    remove_strike.decision_candidate_axis = Some("event:LivingWall:remove_card".to_string());
    let mut remove_defend = test_campaign_branch("remove-defend", 6, 79);
    remove_defend.rank_key = 11_900;
    remove_defend.decision_candidate_axis = Some("event:LivingWall:remove_card".to_string());
    let mut transform_strike = test_campaign_branch("transform-strike", 6, 78);
    transform_strike.rank_key = 11_000;
    transform_strike.decision_candidate_axis = Some("event:LivingWall:transform_card".to_string());

    let selected =
        select_campaign_branches_v1(vec![remove_strike, remove_defend, transform_strike], 2, 8);
    let scheduled_ids = selected
        .scheduled
        .iter()
        .map(|branch| branch.branch_id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(selected.scheduled.len(), 2);
    assert!(scheduled_ids.contains(&"remove-strike"));
    assert!(scheduled_ids.contains(&"remove-defend"));
    assert!(selected
        .scheduled
        .iter()
        .all(|branch| branch.stop_reason.contains("scheduler:general")));
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
            replay_start_source: Some(BranchCampaignReplayStartSourceV1::Ancestor),
            replay_suffix_commands: 2,
            combat_budget_retry_used: true,
            wall_limit_hit: false,
            branch_limit_hit: true,
        });
    let round_line =
        render_branch_campaign_progress_event_v1(&BranchCampaignProgressEventV1::RoundStarted {
            round: 2,
            max_rounds: 4,
            scheduled_branches: 2,
            parked_branches: 6,
        });
    assert_eq!(
        branch_line,
        "round 2: branch 1/2 done | produced=8 branch_points=6 elapsed_ms=1234 start_ms=4321 replay=ancestor suffix=2 retry=combat_budget limits=[branch]"
    );
    assert_eq!(
        round_line,
        "round 2/4: advancing 2 scheduled branch(es), parked=6"
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
            replay_start_source: None,
            replay_suffix_commands: 0,
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
            replay_start_source: None,
            replay_suffix_commands: 0,
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
        started_scheduled: 2,
        produced_branches: 5,
        scheduled_after: 2,
        parked_added: 1,
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
        decision_observations: Vec::new(),
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
fn compact_campaign_report_surfaces_route_continuation_origin() {
    let mut report = test_campaign_report_with_active("route-gap", 6, 70);
    report.active[0].continuation_origin = Some(BranchCampaignContinuationOriginV1 {
        kind: "coverage_gap".to_string(),
        source_event_id: "route-event".to_string(),
        decision_id: "route-decision".to_string(),
        event_type: "route".to_string(),
        parent_branch_id: "root".to_string(),
        parent_frontier_title: "Map".to_string(),
        candidate_index: 1,
        candidate_id: "route_move:normal_edge:x2:y3".to_string(),
        command: "go 2".to_string(),
        label: "x=2 y=3 Elite".to_string(),
        semantic_class: "route".to_string(),
        admission: Default::default(),
        disposition: crate::eval::campaign_journal::CampaignJournalCandidateDispositionV1::Pruned,
        target_lane: None,
        target_origin_source: "route_candidate_pool".to_string(),
        route_origin: Some(BranchCampaignRouteContinuationOriginV1 {
            legal_candidate_count: 4,
            emitted_candidate_count: 4,
            complete_legal_pool: true,
            ordering: "SafetyThenScoreThenX".to_string(),
            ordering_kind: None,
            target_x: 2,
            target_y: 3,
            target_node: None,
            room_type: "Elite".to_string(),
            move_kind: "NormalEdge".to_string(),
            action_kind: "go".to_string(),
            action: None,
            projection_source: "VisibleMapDfs".to_string(),
            projection_source_kind: None,
            projection_coverage: "CompleteWithinBudget".to_string(),
            projection_coverage_kind: None,
            path_budget: 2000,
            observed_path_count: 17,
            path: Some(BranchCampaignRoutePathContinuationOriginV1 {
                path_count: 17,
                path_budget_exhausted: false,
                min_early_pressure: 2,
                max_early_pressure: 5,
                min_elites: 1,
                max_elites: 3,
                min_shops: 0,
                max_shops: 2,
                min_fires: 1,
                max_fires: 3,
                min_unknowns: 2,
                max_unknowns: 6,
                min_treasures: 1,
                max_treasures: 1,
                first_shop_floor: Some(5),
                first_fire_floor: Some(6),
                min_damage_rooms_before_recovery: 1,
                max_damage_rooms_before_recovery: 4,
                min_unknowns_before_recovery: 1,
                max_unknowns_before_recovery: 2,
                paths_with_recovery_before_damage: 3,
            }),
            first_elite: Some(BranchCampaignRouteFirstEliteContinuationOriginV1 {
                paths_with_first_elite: 12,
                forced: false,
                optional: true,
                min_hallway_fights_before: 2,
                max_hallway_fights_before: 4,
                min_unknowns_before: 1,
                max_unknowns_before: 3,
                min_fires_before: 0,
                max_fires_before: 1,
                min_shops_before: 0,
                max_shops_before: 1,
                can_bail_to_rest_before: true,
                can_bail_to_shop_before: true,
            }),
        }),
        milestone: "route_frontier".to_string(),
    });

    let rendered = render_branch_campaign_compact_with_detail_v1(
        &report,
        1,
        BranchCampaignReportDetailV1::Diagnose,
    );

    assert!(rendered.contains("origin=coverage_gap:route:x=2 y=3 Elite"));
    assert!(rendered.contains("source=route_candidate_pool"));
    assert!(rendered.contains("route=x2y3"));
    assert!(rendered.contains("coverage=CompleteWithinBudget"));
    assert!(rendered.contains("paths=17/2000"));
    assert!(rendered.contains("first_elite=optional"));
}

#[test]
fn compact_campaign_report_surfaces_coverage_gap_target_lane() {
    let mut report = test_campaign_report_with_active("shop-gap", 6, 70);
    report.active[0].continuation_origin = Some(BranchCampaignContinuationOriginV1 {
        kind: "coverage_gap".to_string(),
        source_event_id: "shop-event".to_string(),
        decision_id: "shop-decision".to_string(),
        event_type: "shop".to_string(),
        parent_branch_id: "root".to_string(),
        parent_frontier_title: "Shop".to_string(),
        candidate_index: 3,
        candidate_id: "shop:buy_relic:3".to_string(),
        command: "buy relic 3".to_string(),
        label: "Buy Orange Pellets".to_string(),
        semantic_class: "role:SingleAction".to_string(),
        admission: Default::default(),
        disposition: crate::eval::campaign_journal::CampaignJournalCandidateDispositionV1::Kept,
        target_lane: Some(BranchCampaignContinuationTargetLaneV1 {
            bucket: "shop".to_string(),
            admission_status:
                crate::eval::campaign_journal::CampaignJournalCandidateAdmissionStatusV1::Scheduled,
            disposition: crate::eval::campaign_journal::CampaignJournalCandidateDispositionV1::Kept,
            semantic_lane: "shop_action:buy_relic".to_string(),
            shop_action_kind: Some("buy_relic".to_string()),
        }),
        target_origin_source: String::new(),
        route_origin: None,
        milestone: "resource_conversion_frontier".to_string(),
    });

    let rendered = render_branch_campaign_compact_with_detail_v1(
        &report,
        1,
        BranchCampaignReportDetailV1::Diagnose,
    );

    assert!(rendered.contains("lane=shop:scheduled:kept:shop_action:buy_relic"));
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
        run_prelude: Default::default(),
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
        discarded_branches: Vec::new(),
        strategy_requests: Vec::new(),
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
        journal: Default::default(),
        rounds: vec![BranchCampaignRoundSummaryV1 {
            round: 1,
            started_scheduled: 1,
            produced_branches: 2,
            scheduled_after: 1,
            parked_added: 1,
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
        status: BranchCampaignBranchStatusV1::Scheduled,
        stop_reason: "test".to_string(),
        continuation_origin: None,
        decision_candidate_axis: None,
        lineage_decision_signal_rank_adjustment: 0,
        rank_key: hp,
        rank_breakdown: None,
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
    }
}

fn test_run_control_session_with_last_combat_trajectory(source: &str) -> RunControlSession {
    let mut checkpoint = RunControlSessionCheckpointV1::from_session(&RunControlSession::new(
        RunControlConfig::default(),
    ));
    checkpoint.restore_last_combat_automation_trajectory_record(
        CombatAutomationTrajectoryRecordV1::new(
            source,
            vec![CombatAutomationActionV1 {
                step_index: 0,
                action_key: "combat/end_turn".to_string(),
                input: crate::state::core::ClientInput::EndTurn,
                drawn_cards: Vec::new(),
                combat_after: None,
            }],
        ),
    );
    let mut session = checkpoint
        .into_session()
        .expect("checkpoint with trajectory should restore");
    session
        .run_state
        .emitted_events
        .push(crate::state::selection::DomainEvent::GoldChanged {
            delta: 1,
            new_total: 100,
            source: crate::state::selection::DomainEventSource::RewardScreen,
        });
    session
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
                candidate_axis: None,
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
