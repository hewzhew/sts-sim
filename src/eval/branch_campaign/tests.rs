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

    assert!(store.get_session_cloned(&kept.commands).is_some());
    assert!(store.get_session_cloned(&["missing".to_string()]).is_none());
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
        strategy_requests: Vec::new(),
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerStateV1::default(),
        rounds: Vec::new(),
        state_store,
        recovered_checkpoint_failure_commands: BTreeSet::new(),
    };

    let checkpoint = campaign_checkpoint_from_state_v1(&config, &state);

    assert_eq!(checkpoint.schema_name, "BranchCampaignCheckpointV2");
    assert_eq!(checkpoint.schema_version, 2);
    assert_eq!(checkpoint.nodes.len(), 2);
    assert_eq!(checkpoint.nodes[1].parent_id, Some(0));
    assert_eq!(checkpoint.nodes[1].added_commands, vec!["go 2".to_string()]);
    assert_eq!(checkpoint.sessions.len(), 1);
    assert_eq!(checkpoint.sessions[0].commands, child_commands);
}

#[test]
fn campaign_compact_report_renders_route_evidence_summary() {
    let mut report = test_campaign_report_with_active("a", 6, 80);
    report.route_evidence = BranchCampaignRouteEvidenceSummaryV1 {
        decisions: 3,
        first_elite_forced: 1,
        first_elite_optional: 2,
        first_elite_none: 0,
        rest_bailout: 2,
        shop_bailout: 1,
        underprepared_first_elite: 1,
        avg_elite_prep_bp: 62,
        examples: vec![BranchCampaignRouteEvidenceExampleV1 {
            target: "x=5 Monster".to_string(),
            first_elite:
                "optional hallways=2-3 fires=1 shops=0 rest_bailout=true shop_bailout=false"
                    .to_string(),
            elite_prep_bp: 70,
        }],
        underprepared_examples: vec![BranchCampaignRouteEvidenceExampleV1 {
            target: "x=1 Elite".to_string(),
            first_elite:
                "forced hallways=0-0 fires=0 shops=0 rest_bailout=false shop_bailout=false"
                    .to_string(),
            elite_prep_bp: -25,
        }],
    };

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains(
        "Route evidence: decisions=3 first_elite optional=2 forced=1 none=0 avg_elite_prep=0.62 underprepared=1 bailouts=rest:2 shop:1"
    ));
    assert!(rendered.contains(
        "example: x=5 Monster | first_elite=optional hallways=2-3 fires=1 shops=0 rest_bailout=true shop_bailout=false elite_prep=0.70"
    ));
    assert!(rendered.contains(
        "Route concern: forced_first_elite_underprepared=1/3 rest_bailout=2 shop_bailout=1"
    ));
    assert!(rendered.contains(
        "concern example: x=1 Elite | first_elite=forced hallways=0-0 fires=0 shops=0 rest_bailout=false shop_bailout=false elite_prep=-0.25"
    ));
}

#[test]
fn campaign_compact_report_renders_state_store_summary() {
    let mut report = test_campaign_report_with_active("a", 6, 80);
    report.state_store = BranchCampaignStateStoreSummaryV1 {
        sessions: 4,
        nodes: 5,
        linked_nodes: 3,
        lookup_hits: 2,
        lookup_misses: 1,
        inserts: 6,
        retains: 1,
    };

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered
        .contains("State store: sessions=4 nodes=5 linked=3 lookups=2/1 inserts=6 retains=1"));
}

#[test]
fn compact_campaign_report_renders_branch_pressure_examples() {
    let mut report = test_campaign_report_with_active("a", 7, 80);
    report.discarded_count = 12;
    report.discarded_examples = vec![
        "Anger -> Clash -> Skip card reward".to_string(),
        "Body Slam -> Searing Blow".to_string(),
    ];

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains(
        "Branch pressure: discarded=12 examples=[Anger -> Clash -> Skip card reward | Body Slam -> Searing Blow]"
    ));
}

#[test]
fn compact_campaign_report_truncates_long_branch_pressure_examples() {
    let mut report = test_campaign_report_with_active("a", 7, 80);
    report.discarded_count = 1;
    report.discarded_examples = vec![
        "Wild Strike -> Headbutt -> Heavy Blade -> Heavy Blade -> [Proceed] -> [Take Card] Obtain Iron Wave. Remove a card.".to_string(),
    ];

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains(
        "Branch pressure: discarded=1 examples=[Wild Strike -> Headbutt -> ... -> [Take Card] Obtain Iron Wave. Remove a card.]"
    ));
}

#[test]
fn compact_campaign_report_renders_boss_relic_lineage_coverage() {
    let mut report = test_campaign_report_with_active("active-key", 18, 80);
    report.active[0].choice_labels = vec![
        "CursedKey adds debt curse_chest_debt".to_string(),
        "Pommel Strike".to_string(),
    ];
    let mut frozen_hammer = test_campaign_branch("frozen-hammer", 18, 78);
    frozen_hammer.status = BranchCampaignBranchStatusV1::Frozen;
    frozen_hammer.choice_labels = vec![
        "FusionHammer adds debt smith_lock".to_string(),
        "Shrug It Off".to_string(),
    ];
    let mut abandoned_key = test_campaign_branch("abandoned-key", 48, 30);
    abandoned_key.status = BranchCampaignBranchStatusV1::Abandoned;
    abandoned_key.choice_labels = vec!["CursedKey".to_string(), "Clothesline".to_string()];
    abandoned_key.summary.as_mut().unwrap().act = 3;
    report.frozen = vec![frozen_hammer];
    report.abandoned = vec![abandoned_key];

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains(
        "Boss relic coverage: active=[CursedKey=1] frozen=[FusionHammer=1] abandoned=[CursedKey=1] furthest=[CursedKey=A3F48 FusionHammer=A1F18]"
    ));
}

#[test]
fn compact_campaign_report_renders_combat_lab_probe_summary() {
    let mut report = test_campaign_report_with_active("active-key", 24, 70);
    report.active[0].combat_lab_probes = vec![CombatLabProbePacketV1 {
        kind: "current_act_boss_preview".to_string(),
        source: "campaign_key_boundary".to_string(),
        boss: Some("TheChamp".to_string()),
        boundary: "Shop".to_string(),
        result: "unresolved_no_trajectory".to_string(),
        hp_loss: None,
        final_hp: Some(70),
        max_hp: Some(94),
        actions: None,
        search_digest: vec!["result=no_complete_winning_candidate".to_string()],
        diagnosis: CombatLabProbeDiagnosisV1 {
            outcome_class: "unresolved".to_string(),
            search_reason: "wall_clock_deadline_hit".to_string(),
            confidence: "search_digest".to_string(),
            signals: vec!["budget_limited".to_string(), "no_terminal_wins".to_string()],
        },
    }];

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered
        .contains("Combat lab probes: current_act_boss_preview=1 unresolved_no_trajectory=1"));
    assert!(rendered.contains(
        "probe example: boss=TheChamp source=campaign_key_boundary boundary=Shop result=unresolved_no_trajectory"
    ));
    assert!(rendered.contains(
        "probe diagnosis: unresolved/wall_clock_deadline_hit confidence=search_digest signals=budget_limited,no_terminal_wins"
    ));
}

#[test]
fn campaign_current_act_boss_probe_gate_accepts_only_late_key_boundaries() {
    let mut late_shop = test_campaign_branch("late-shop", 24, 70);
    late_shop.frontier_title = "Shop".to_string();
    let summary = late_shop.summary.as_mut().unwrap();
    summary.act = 2;
    summary.boss = "TheChamp".to_string();

    assert!(campaign_branch_should_probe_current_act_boss_v1(&late_shop));

    let mut late_reward_overlay = late_shop.clone();
    late_reward_overlay.frontier_title = "Reward Overlay".to_string();
    late_reward_overlay.summary.as_mut().unwrap().floor = 23;
    assert!(campaign_branch_should_probe_current_act_boss_v1(
        &late_reward_overlay
    ));

    let mut early_reward = late_shop.clone();
    early_reward.frontier_title = "Card Reward".to_string();
    early_reward.summary.as_mut().unwrap().floor = 12;
    assert!(!campaign_branch_should_probe_current_act_boss_v1(
        &early_reward
    ));

    let mut combat = late_shop.clone();
    combat.frontier_title = "Combat".to_string();
    assert!(!campaign_branch_should_probe_current_act_boss_v1(&combat));
}

#[test]
fn campaign_combat_lab_boss_probe_options_are_report_only_and_capped() {
    let config = BranchCampaignConfigV1 {
        search_max_nodes: Some(200_000),
        search_wall_ms: Some(2_000),
        search_options: RunControlSearchCombatOptions {
            max_nodes: Some(150_000),
            wall_ms: Some(1_500),
            max_hp_loss: Some(RunControlHpLossLimit::Limit(3)),
            evidence: Some(
                crate::eval::run_control::RunControlSearchEvidenceTarget::LastCaptureCase,
            ),
            ..RunControlSearchCombatOptions::default()
        },
        ..BranchCampaignConfigV1::default()
    };

    let options = campaign_combat_lab_boss_probe_search_options_v1(&config);

    assert_eq!(
        options.max_nodes,
        Some(super::COMBAT_LAB_CAMPAIGN_BOSS_PROBE_MAX_NODES)
    );
    assert_eq!(
        options.wall_ms,
        Some(super::COMBAT_LAB_CAMPAIGN_BOSS_PROBE_MAX_WALL_MS)
    );
    assert_eq!(options.max_hp_loss, Some(RunControlHpLossLimit::Unlimited));
    assert_eq!(options.evidence, None);
}

#[test]
fn compact_campaign_report_renders_combat_performance_summary() {
    let mut report = test_campaign_report_with_active("a", 7, 80);
    report.rounds[0].combat_performance = BranchCampaignCombatPerformanceSummaryV1 {
        samples: 2,
        total_us: 10_000,
        rollout_us: 4_000,
        rollout_calls: 10,
        root_rollout_calls: 2,
        child_rollout_calls: 7,
        deferred_child_rollout_calls: 5,
        turn_plan_seed_rollout_calls: 1,
        deferred_child_rollout_nodes: 9,
        deferred_child_rollout_requeues: 5,
        rollout_cache_hits: 3,
        rollout_cache_queries: 13,
        rollout_cache_misses: 10,
        rollout_cache_inserts: 8,
        rollout_budget_skips: 2,
        rollout_max_evaluation_budget_skips: 1,
        rollout_deadline_budget_skips: 1,
        rollout_truncated: 1,
        rollout_terminal_wins: 4,
        rollout_cache_lookup_us: 100,
        rollout_policy_dispatch_us: 2_000,
        rollout_no_potion_iterations: 19,
        rollout_no_potion_phase_profile_us: 300,
        rollout_no_potion_legal_actions_us: 400,
        rollout_no_potion_choose_action_us: 500,
        rollout_no_potion_choose_ordering_us: 100,
        rollout_no_potion_probe_us: 250,
        rollout_no_potion_probe_score_calls: 11,
        rollout_no_potion_probe_actions_evaluated: 10,
        rollout_no_potion_probe_step_reuses: 3,
        rollout_no_potion_probe_engine_step_us: 60,
        rollout_no_potion_probe_phase_profile_us: 70,
        rollout_no_potion_probe_action_facts_us: 80,
        rollout_no_potion_engine_step_us: 600,
        rollout_no_potion_child_build_us: 200,
        terminal_child_rollout_skips: 2,
        terminal_turn_plan_seed_rollout_skips: 1,
        turn_local_dominance_rollout_skips: 3,
        expansion_us: 2_000,
        child_bookkeeping_us: 1_500,
        engine_step_us: 1_000,
        external_payoff_samples: 1,
        boss_samples: 1,
        slowest: vec![BranchCampaignCombatPerformanceExampleV1 {
            total_us: 7_000,
            act: 2,
            floor: 32,
            turn: 1,
            combat_kind: "boss".to_string(),
            enemies: "Bronze Automaton".to_string(),
            coverage_status: "TimeBudgetLimited".to_string(),
            dominant_bucket: "rollout".to_string(),
        }],
        ..BranchCampaignCombatPerformanceSummaryV1::default()
    };

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains(
        "Combat perf: samples=2 total=0.0s dominant=rollout rollout=40% expansion=20% child=15% engine=10% rollout_calls=10 root_calls=2 child_calls=7 deferred_child_calls=5 seed_calls=1 deferred_nodes=9 deferred_requeues=5 cache=hits/queries/misses/inserts:3/13/10/8 budget_skips=2(max=1 deadline=1) terminal_skips=2 seed_terminal_skips=1 dominance_skips=3 rollout_inner=iters:19 policy_total:20% phase:3% legal:4% choose:5% order:1% probe:2% probe_calls:11 probe_eval:10 probe_reuse:3 probe_engine:0% probe_phase:0% probe_facts:0% engine:6% build:2% external_payoff=1 boss=1"
    ));
    assert!(rendered.contains(
        "slowest: A2F32 turn=1 boss Bronze Automaton 0.0s bucket=rollout status=TimeBudgetLimited"
    ));
}

#[test]
fn compact_campaign_report_truncates_long_active_choice_paths() {
    let mut report = test_campaign_report_with_active("a", 7, 80);
    report.active[0].choice_labels = vec![
        "Warcry".to_string(),
        "Body Slam".to_string(),
        "Shrug It Off".to_string(),
        "Sword Boomerang".to_string(),
        "PandorasBox".to_string(),
        "Whirlwind".to_string(),
    ];

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains(
        "choices: Warcry -> Body Slam -> ... -> Sword Boomerang -> PandorasBox -> Whirlwind"
    ));
    assert!(!rendered.contains(
        "choices: Warcry -> Body Slam -> Shrug It Off -> Sword Boomerang -> PandorasBox -> Whirlwind"
    ));
    assert_eq!(report.active[0].choice_labels.len(), 6);
}

#[test]
fn compact_campaign_report_renders_active_branch_differences() {
    let mut report = test_campaign_report_with_active("baseline", 35, 80);
    report.active[0].choice_labels = vec![
        "Rampage".to_string(),
        "Sever Soul".to_string(),
        "Offering".to_string(),
        "Cleave".to_string(),
        "Buy Warcry | 23 gold".to_string(),
    ];
    let mut exhaust_branch = test_campaign_branch("exhaust", 35, 80);
    exhaust_branch.choice_labels = vec![
        "Rampage".to_string(),
        "Sever Soul".to_string(),
        "Offering".to_string(),
        "Dark Embrace".to_string(),
        "Buy Warcry | 23 gold".to_string(),
    ];
    let summary = exhaust_branch.summary.as_mut().unwrap();
    summary.formation_stage = "Mature".to_string();
    summary.formation_strengths = vec!["StrengthScaling".to_string(), "ExhaustEngine".to_string()];
    report.active.push(exhaust_branch);

    let rendered = render_branch_campaign_compact_v1(&report, 2);

    assert!(rendered
        .contains("2. A1F35 HP 80/80 gold 99 deck 10 sel=[retention_rank=80] | Card Reward"));
    assert!(rendered.contains(
        "diff: choices +Dark Embrace; stage PlanSeeded->Mature; strengths +ExhaustEngine +StrengthScaling"
    ));
}

#[test]
fn compact_campaign_report_compacts_branch_difference_labels() {
    let mut report = test_campaign_report_with_active("baseline", 35, 80);
    report.active[0].choice_labels = vec!["Skip card reward".to_string()];
    let mut shop_branch = test_campaign_branch("shop", 35, 80);
    shop_branch.choice_labels = vec![
        "Buy MembershipCard | 155 gold | total 155 gold | shop_legacy_estimate=950 | source=CandidateEvidence | auto leave shop"
            .to_string(),
    ];
    report.active.push(shop_branch);

    let rendered = render_branch_campaign_compact_v1(&report, 2);

    assert!(rendered.contains("diff: choices +Buy MembershipCard 155g"));
    assert!(!rendered.contains("source=CandidateEvidence"));
    assert!(!rendered.contains("shop_legacy_estimate"));
}

#[test]
fn compact_campaign_report_renders_abandoned_examples_while_continuing() {
    let mut report = test_campaign_report_with_active("a", 7, 80);
    let mut abandoned = test_campaign_branch("abandoned", 6, 55);
    abandoned.status = BranchCampaignBranchStatusV1::Abandoned;
    abandoned.stop_reason = "combat search did not find an executable complete win".to_string();
    abandoned.choice_labels = vec![
        "Havoc".to_string(),
        "Hemokinesis".to_string(),
        "Spot Weakness".to_string(),
        "Searing Blow".to_string(),
    ];
    report.abandoned = vec![abandoned];

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains(
        "Abandoned examples: count=1 reasons=[combat search did not find an executable complete win] examples=[Havoc -> Hemokinesis -> Spot Weakness -> Searing Blow]"
    ));
}

#[test]
fn compact_campaign_report_renders_final_boss_failure_summary() {
    let mut report = test_campaign_report_with_active("active", 47, 80);
    let mut abandoned = test_campaign_branch("boss-fail", 48, 88);
    abandoned.status = BranchCampaignBranchStatusV1::Abandoned;
    abandoned.frontier_title = "Combat".to_string();
    abandoned.stop_reason = "combat search did not find an executable complete win".to_string();
    if let Some(summary) = abandoned.summary.as_mut() {
        summary.act = 3;
        summary.floor = 48;
        summary.max_hp = 88;
        summary.deck_count = 20;
        summary.deck_key = "Strike+0x2;Defend+0x4;Bash+1x1;Bludgeon+1x1;Demon Form+0x1".to_string();
        summary.boss = "DonuAndDeca".to_string();
    }
    report.abandoned = vec![abandoned];

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered
        .contains("Final boss failures: abandoned=1 bosses=[DonuAndDeca=1] hp=88..88 deck=20..20"));
    assert!(rendered.contains("A3F48 HP 88/88 gold 99 deck 20"));
}

#[test]
fn compact_campaign_report_renders_unspent_gold_pressure_near_boss() {
    let mut report = test_campaign_report_with_active("rich", 16, 30);
    report.active[0].summary.as_mut().unwrap().gold = 485;
    report.active[0].choice_labels = vec![
        "Flame Barrier".to_string(),
        "Wild Strike".to_string(),
        "Buy Anchor | 146 gold | auto leave shop".to_string(),
        "Barricade".to_string(),
        "Smith Shockwave".to_string(),
    ];

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains(
        "Resource concern: high_unspent_gold_near_boss=1 max_gold=485 causes=[purchase_seen_gold_still_high=1]"
    ));
    assert!(
        rendered.contains("resource example: A1F16 gold 485 cause=purchase_seen_gold_still_high")
    );
    assert!(rendered.contains("Flame Barrier -> Wild Strike"));
    assert!(rendered.contains("Smith Shockwave"));
}

#[test]
fn compact_campaign_report_classifies_unspent_gold_without_shop_visit() {
    let mut report = test_campaign_report_with_active("rich", 16, 30);
    report.active[0].summary.as_mut().unwrap().gold = 485;
    report.active[0].choice_labels = vec![
        "Flame Barrier".to_string(),
        "Wild Strike".to_string(),
        "Smith Shockwave".to_string(),
    ];

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains(
        "Resource concern: high_unspent_gold_near_boss=1 max_gold=485 causes=[no_shop_action_seen=1]"
    ));
    assert!(rendered.contains(
        "resource example: A1F16 gold 485 cause=no_shop_action_seen | Flame Barrier -> Wild Strike -> Smith Shockwave"
    ));
}

#[test]
fn compact_campaign_report_classifies_shop_leave_without_purchase() {
    let mut report = test_campaign_report_with_active("rich", 16, 30);
    report.active[0].summary.as_mut().unwrap().gold = 485;
    report.active[0].choice_labels = vec![
        "Flame Barrier".to_string(),
        "Leave shop | decline selected shop purchase portfolio".to_string(),
        "Smith Shockwave".to_string(),
    ];

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains(
        "Resource concern: high_unspent_gold_near_boss=1 max_gold=485 causes=[shop_leave_without_purchase=1]"
    ));
    assert!(rendered.contains("resource example: A1F16 gold 485 cause=shop_leave_without_purchase"));
    assert!(rendered.contains("Flame Barrier -> Leave shop"));
    assert!(rendered.contains("Smith Shockwave"));
}

#[test]
fn compact_campaign_report_renders_boss_mechanic_pressure() {
    let mut report = test_campaign_report_with_active("boss", 48, 42);
    let summary = report.active[0].summary.as_mut().unwrap();
    summary.act = 3;
    summary.floor = 48;
    summary.boss = "AwakenedOne".to_string();
    summary.boss_pressure = vec![
        "pressure:dark_echo_block_check".to_string(),
        "red:enemy_strength_multi_hit_risk".to_string(),
        "missing:phase_power_plan".to_string(),
    ];

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains("Boss pressure: bosses=[AwakenedOne=1]"));
    assert!(rendered.contains("pressure:dark_echo_block_check=1"));
    assert!(rendered.contains("red:enemy_strength_multi_hit_risk=1"));
    assert!(rendered.contains(
        "boss example: A3F48 HP 42/80 deck 10 boss=AwakenedOne | pressure:dark_echo_block_check red:enemy_strength_multi_hit_risk missing:phase_power_plan"
    ));
}

#[test]
fn compact_campaign_report_renders_boss_pressure_for_abandoned_combat_branches() {
    let mut report = test_campaign_report_with_active("boss", 48, 42);
    let mut abandoned = report.active.remove(0);
    abandoned.status = BranchCampaignBranchStatusV1::Abandoned;
    abandoned.frontier_title = "Combat".to_string();
    abandoned.stop_reason = "combat search did not find an executable complete win".to_string();
    let summary = abandoned.summary.as_mut().unwrap();
    summary.act = 3;
    summary.floor = 48;
    summary.boss = "TimeEater".to_string();
    summary.boss_pressure = vec![
        "pressure:time_warp_counter_control".to_string(),
        "red:low_value_card_spam_risk".to_string(),
    ];
    report.abandoned = vec![abandoned];

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains("Boss pressure: bosses=[TimeEater=1]"));
    assert!(rendered.contains("pressure:time_warp_counter_control=1"));
    assert!(rendered.contains("red:low_value_card_spam_risk=1"));
}

#[test]
fn campaign_report_branch_preserves_stop_reason() {
    let parent = test_campaign_branch("parent", 3, 80);
    let mut child = test_report_branch(
        "child",
        vec![("rp 1", "Pommel Strike")],
        BranchExperimentBranchStatusV1::Pruned,
    );
    child.stop_reason = "combat search did not find an executable complete win".to_string();

    let campaign_branch = campaign_branch_from_report_branch_v1(&parent, &child);

    assert_eq!(
        campaign_branch.status,
        BranchCampaignBranchStatusV1::Abandoned
    );
    assert_eq!(
        campaign_branch.stop_reason,
        "combat search did not find an executable complete win"
    );
}

#[test]
fn campaign_report_branch_preserves_strategic_summary() {
    let parent = test_campaign_branch("parent", 3, 80);
    let mut child = test_report_branch(
        "child",
        vec![("rp 1", "Barricade")],
        BranchExperimentBranchStatusV1::Active,
    );
    child.retention.strategic_signature = BranchSignature {
        boss_readiness: 0.6,
        clean_score: 0.8,
        engine_score: 1.0,
        cycle_debt: 0.2,
        setup_debt: 0.4,
        economy_conversion: 0.0,
        package_coherence: 0.7,
        buckets: vec![RetentionBucket::BestCoreEngine],
    };

    let campaign_branch = campaign_branch_from_report_branch_v1(&parent, &child);
    let mut report = test_campaign_report_with_active("placeholder", 1, 80);
    report.active = vec![campaign_branch];
    report.frozen.clear();
    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains("strat=[boss:0.6 clean:0.8 eng:1.0 debt:0.2/0.4 pkg:0.7]"));
}

#[test]
fn campaign_branch_from_report_preserves_parent_trajectory_when_child_has_none() {
    let mut parent = test_campaign_branch("parent", 6, 80);
    parent.summary.as_mut().unwrap().trajectory_key =
        "setup=-|pkg=-|frontload=2|transition=1|scaling=0|defense=1|engine_gen=0|engine_payoff=0|draw_energy=1"
            .to_string();
    let mut child = test_report_branch(
        "child",
        vec![("skip", "Skip card reward")],
        BranchExperimentBranchStatusV1::Active,
    );
    child.summary.trajectory = BranchTrajectorySignatureV1::default();

    let campaign_branch = campaign_branch_from_report_branch_v1(&parent, &child);

    assert_eq!(
        campaign_branch.summary.unwrap().trajectory_key,
        "setup=-|pkg=-|frontload=2|transition=1|scaling=0|defense=1|engine_gen=0|engine_payoff=0|draw_energy=1"
    );
}

#[test]
fn campaign_branch_from_report_merges_parent_and_child_trajectory() {
    let mut parent = test_campaign_branch("parent", 6, 80);
    parent.summary.as_mut().unwrap().trajectory_key =
        "setup=exhaust_engine|pkg=-|frontload=2|transition=1|scaling=0|defense=0|engine_gen=1|engine_payoff=0|draw_energy=0"
            .to_string();
    let mut child = test_report_branch(
        "child",
        vec![("rp 2", "Heavy Blade")],
        BranchExperimentBranchStatusV1::Active,
    );
    child.summary.trajectory.frontload_picks = 1;
    child.summary.trajectory.scaling_picks = 1;
    child.summary.trajectory.engine_payoff_picks = 1;
    child
        .summary
        .trajectory
        .setup_keys
        .push("strength_scaling".to_string());
    child
        .summary
        .trajectory
        .package_keys
        .push("strength_scaling".to_string());

    let campaign_branch = campaign_branch_from_report_branch_v1(&parent, &child);

    assert_eq!(
        campaign_branch.summary.unwrap().trajectory_key,
        "setup=exhaust_engine+strength_scaling|pkg=strength_scaling|frontload=3|transition=1|scaling=1|defense=0|engine_gen=1|engine_payoff=1|draw_energy=0"
    );
}

#[test]
fn compact_campaign_report_shows_selection_basis_for_branch_examples() {
    let mut report = test_campaign_report_with_active("active", 3, 80);
    report.active[0].rank_key = 123;

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains("sel=[retention_rank=123]"));
}

#[test]
fn compact_campaign_report_formats_large_selection_rank_readably() {
    let mut report = test_campaign_report_with_active("active", 3, 80);
    report.active[0].rank_key = 11_513;

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains("sel=[retention_rank=11.5k]"));
}

#[test]
fn compact_campaign_report_shows_mechanical_deck_shape() {
    let mut report = test_campaign_report_with_active("active", 3, 80);
    report.active[0].summary.as_mut().unwrap().deck_count = 10;
    report.active[0].summary.as_mut().unwrap().deck_key =
        "Bash+1x1;Clash+0x1;Defend+0x3;Defend+1x1;Strike+0x4".to_string();

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains("deck 10 [S4 D4 starter9 add:Clash upg2]"));
}

#[test]
fn campaign_choice_label_prefixes_generic_event_leave_with_boundary() {
    let choice = BranchExperimentChoiceV1 {
        depth: 0,
        kind: "event".to_string(),
        boundary_title: "GoopPuddle".to_string(),
        card: None,
        upgrades: None,
        selected_cards: Vec::new(),
        effect_kind: "event".to_string(),
        effect_key: "leave".to_string(),
        effect_label: "[Leave] Lose 27 Gold.".to_string(),
        representative_count: 1,
        suppressed_count: 0,
        decision_signal: None,
        label: "[Leave] Lose 27 Gold.".to_string(),
        command: "event 1".to_string(),
    };

    assert_eq!(
        campaign_choice_label_v1(&choice),
        "GoopPuddle: [Leave] Lose 27 Gold."
    );
}

#[test]
fn campaign_choice_label_prefixes_bracketed_event_choices_with_boundary() {
    let choice = BranchExperimentChoiceV1 {
        depth: 0,
        kind: "event".to_string(),
        boundary_title: "GoldenWing".to_string(),
        card: None,
        upgrades: None,
        selected_cards: Vec::new(),
        effect_kind: "event".to_string(),
        effect_key: "remove_card".to_string(),
        effect_label: "[Remove a card] Take 7 damage. Remove a card from your deck.".to_string(),
        representative_count: 1,
        suppressed_count: 0,
        decision_signal: None,
        label: "[Remove a card] Take 7 damage. Remove a card from your deck.".to_string(),
        command: "event 0".to_string(),
    };

    assert_eq!(
        campaign_choice_label_v1(&choice),
        "GoldenWing: [Remove a card] Take 7 damage. Remove a card from your deck."
    );
}

#[test]
fn campaign_choice_label_omits_event_eval_from_choice_path() {
    let choice = BranchExperimentChoiceV1 {
        depth: 0,
        kind: "event".to_string(),
        boundary_title: "UpgradeShrine".to_string(),
        card: None,
        upgrades: None,
        selected_cards: Vec::new(),
        effect_kind: "event".to_string(),
        effect_key: "upgrade".to_string(),
        effect_label:
            "[Pray] Upgrade a card. | event_eval tier=Risky score=-80 reasons=mutates deck identity"
                .to_string(),
        representative_count: 1,
        suppressed_count: 0,
        decision_signal: None,
        label: "[Pray] Upgrade a card.".to_string(),
        command: "event 0 && select 7".to_string(),
    };

    assert_eq!(
        campaign_choice_label_v1(&choice),
        "UpgradeShrine: [Pray] Upgrade a card."
    );
}

#[test]
fn campaign_choice_label_compacts_shop_metadata() {
    let choice = BranchExperimentChoiceV1 {
        depth: 0,
        kind: "shop".to_string(),
        boundary_title: "Shop".to_string(),
        card: None,
        upgrades: None,
        selected_cards: Vec::new(),
        effect_kind: "shop".to_string(),
        effect_key: "buy".to_string(),
        effect_label: "Purge Strike | 75 gold then Buy Flex Potion potion | 51 gold | total 126 gold | source=PortfolioCandidate | auto leave shop".to_string(),
        representative_count: 1,
        suppressed_count: 0,
        decision_signal: None,
        label: "shop".to_string(),
        command: "shop 0".to_string(),
    };

    assert_eq!(
        campaign_choice_label_v1(&choice),
        "Purge Strike 75g then Buy Flex Potion potion 51g"
    );
}

#[test]
fn campaign_choice_label_compacts_deck_mutation_metadata() {
    let choice = BranchExperimentChoiceV1 {
        depth: 0,
        kind: "deck_mutation".to_string(),
        boundary_title: "UpgradeShrine".to_string(),
        card: None,
        upgrades: None,
        selected_cards: Vec::new(),
        effect_kind: "upgrade".to_string(),
        effect_key: "upgrade".to_string(),
        effect_label:
            "upgrade Defend | deck mutation role=SafeAlternative loss=LowValue confidence=0.66"
                .to_string(),
        representative_count: 1,
        suppressed_count: 0,
        decision_signal: None,
        label: "upgrade Defend".to_string(),
        command: "select 3".to_string(),
    };

    assert_eq!(campaign_choice_label_v1(&choice), "upgrade Defend");
}

#[test]
fn compact_campaign_report_summarizes_active_strategic_signals() {
    let parent = test_campaign_branch("parent", 3, 80);
    let mut engine = test_report_branch(
        "engine",
        vec![("rp 1", "Barricade")],
        BranchExperimentBranchStatusV1::Active,
    );
    engine.retention.strategic_signature = BranchSignature {
        boss_readiness: 0.6,
        clean_score: 0.8,
        engine_score: 1.0,
        cycle_debt: 0.2,
        setup_debt: 0.4,
        economy_conversion: 0.0,
        package_coherence: 0.7,
        buckets: vec![RetentionBucket::BestCoreEngine],
    };
    let mut clean = test_report_branch(
        "clean",
        vec![("skip", "Skip card reward")],
        BranchExperimentBranchStatusV1::Active,
    );
    clean.retention.strategic_signature = BranchSignature {
        boss_readiness: 0.2,
        clean_score: 1.0,
        engine_score: 0.0,
        cycle_debt: 0.0,
        setup_debt: 0.0,
        economy_conversion: 0.0,
        package_coherence: 0.0,
        buckets: vec![RetentionBucket::BestCleanDeck],
    };

    let mut report = test_campaign_report_with_active("placeholder", 1, 80);
    report.active = vec![
        campaign_branch_from_report_branch_v1(&parent, &engine),
        campaign_branch_from_report_branch_v1(&parent, &clean),
    ];
    report.frozen.clear();

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains(
        "Strategic signals: active n=2 avg=[boss:0.4 clean:0.9 eng:0.5 debt:0.1/0.2 pkg:0.4]"
    ));
}

#[test]
fn compact_campaign_report_flags_frozen_engine_above_active() {
    let mut report = test_campaign_report_with_active("placeholder", 1, 80);
    report.active.clear();
    report.frozen.clear();
    report.strategic_signals = BranchCampaignStrategicSignalsV1 {
        groups: vec![
            BranchCampaignStrategicSignalGroupV1 {
                label: "active".to_string(),
                count: 2,
                average: BranchSignatureCompact {
                    present: true,
                    boss_readiness_milli: 600,
                    clean_score_milli: 900,
                    engine_score_milli: 100,
                    cycle_debt_milli: 100,
                    setup_debt_milli: 0,
                    economy_conversion_milli: 0,
                    package_coherence_milli: 100,
                },
            },
            BranchCampaignStrategicSignalGroupV1 {
                label: "frozen".to_string(),
                count: 6,
                average: BranchSignatureCompact {
                    present: true,
                    boss_readiness_milli: 500,
                    clean_score_milli: 700,
                    engine_score_milli: 500,
                    cycle_debt_milli: 100,
                    setup_debt_milli: 0,
                    economy_conversion_milli: 0,
                    package_coherence_milli: 300,
                },
            },
        ],
    };

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains(
        "Strategic concern: frozen_engine_above_active=0.4 frozen_package_above_active=0.2"
    ));
}

#[test]
fn campaign_frozen_overflow_replaces_weaker_existing_branch() {
    let mut existing = test_campaign_branch("existing", 3, 80);
    existing.choice_labels = vec!["Body Slam".to_string()];
    let mut frozen = vec![existing];
    let mut discarded_count = 0usize;
    let mut discarded_examples = Vec::new();
    let mut deeper = test_campaign_branch("deeper", 4, 75);
    deeper.choice_labels = vec!["Wild Strike".to_string(), "Skip card reward".to_string()];

    let added = append_limited_frozen_v1(
        &mut frozen,
        vec![deeper],
        1,
        &mut discarded_count,
        &mut discarded_examples,
    );

    assert_eq!(added, 1);
    assert_eq!(frozen.len(), 1);
    assert_eq!(frozen[0].branch_id, "deeper");
    assert_eq!(discarded_count, 1);
    assert_eq!(discarded_examples, vec!["Body Slam"]);
}

#[test]
fn campaign_frozen_overflow_discards_weaker_incoming_branch() {
    let mut frozen = vec![test_campaign_branch("existing", 4, 80)];
    let mut discarded_count = 0usize;
    let mut discarded_examples = Vec::new();
    let mut shallow = test_campaign_branch("shallow", 3, 75);
    shallow.choice_labels = vec!["Wild Strike".to_string(), "Skip card reward".to_string()];

    let added = append_limited_frozen_v1(
        &mut frozen,
        vec![shallow],
        1,
        &mut discarded_count,
        &mut discarded_examples,
    );

    assert_eq!(added, 0);
    assert_eq!(frozen.len(), 1);
    assert_eq!(frozen[0].branch_id, "existing");
    assert_eq!(discarded_count, 1);
    assert_eq!(discarded_examples, vec!["Wild Strike -> Skip card reward"]);
}

#[test]
fn campaign_frozen_overflow_preserves_new_boss_relic_lineage() {
    let mut coffee_high = test_campaign_branch("coffee-high", 18, 80);
    coffee_high.rank_key = 20_000;
    coffee_high.choice_labels = vec![
        "CoffeeDripper adds debt rest_lock".to_string(),
        "Pommel Strike".to_string(),
    ];
    let mut coffee_low = test_campaign_branch("coffee-low", 18, 78);
    coffee_low.rank_key = 19_000;
    coffee_low.choice_labels = vec![
        "CoffeeDripper adds debt rest_lock".to_string(),
        "Clothesline".to_string(),
    ];
    let mut hammer = test_campaign_branch("hammer", 18, 72);
    hammer.rank_key = 18_000;
    hammer.choice_labels = vec![
        "FusionHammer adds debt smith_lock".to_string(),
        "Shrug It Off".to_string(),
    ];
    let mut frozen = vec![coffee_high, coffee_low];
    let mut discarded_count = 0usize;
    let mut discarded_examples = Vec::new();

    let added = append_limited_frozen_v1(
        &mut frozen,
        vec![hammer],
        2,
        &mut discarded_count,
        &mut discarded_examples,
    );

    assert_eq!(added, 1);
    assert_eq!(discarded_count, 1);
    assert!(frozen
        .iter()
        .any(|branch| branch.branch_id == "coffee-high"));
    assert!(frozen.iter().any(|branch| branch.branch_id == "hammer"));
    assert_eq!(
        frozen
            .iter()
            .filter_map(campaign_branch_boss_relic_lineage_key_v1)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["CoffeeDripper".to_string(), "FusionHammer".to_string()])
    );
}

#[test]
fn campaign_frozen_overflow_does_not_evict_unique_boss_relic_lineage_for_duplicate() {
    let mut coffee = test_campaign_branch("coffee", 18, 78);
    coffee.rank_key = 19_000;
    coffee.choice_labels = vec![
        "CoffeeDripper adds debt rest_lock".to_string(),
        "Clothesline".to_string(),
    ];
    let mut hammer = test_campaign_branch("hammer", 18, 72);
    hammer.rank_key = 18_000;
    hammer.choice_labels = vec![
        "FusionHammer adds debt smith_lock".to_string(),
        "Shrug It Off".to_string(),
    ];
    let mut better_coffee = test_campaign_branch("better-coffee", 19, 80);
    better_coffee.rank_key = 22_000;
    better_coffee.choice_labels = vec![
        "CoffeeDripper adds debt rest_lock".to_string(),
        "Pommel Strike".to_string(),
    ];
    let mut frozen = vec![coffee, hammer];
    let mut discarded_count = 0usize;
    let mut discarded_examples = Vec::new();

    let added = append_limited_frozen_v1(
        &mut frozen,
        vec![better_coffee],
        2,
        &mut discarded_count,
        &mut discarded_examples,
    );

    assert_eq!(added, 0);
    assert_eq!(discarded_count, 1);
    assert!(frozen.iter().any(|branch| branch.branch_id == "coffee"));
    assert!(frozen.iter().any(|branch| branch.branch_id == "hammer"));
}

#[test]
fn campaign_frozen_overflow_does_not_replace_branch_by_unspent_gold_pressure() {
    let mut rich = test_campaign_branch("rich", 16, 30);
    rich.summary.as_mut().unwrap().gold = 485;
    let mut frozen = vec![rich];
    let mut discarded_count = 0usize;
    let mut discarded_examples = Vec::new();
    let mut converted = test_campaign_branch("converted", 16, 30);
    converted.summary.as_mut().unwrap().gold = 120;

    let added = append_limited_frozen_v1(
        &mut frozen,
        vec![converted],
        1,
        &mut discarded_count,
        &mut discarded_examples,
    );

    assert_eq!(added, 0);
    assert_eq!(frozen[0].branch_id, "rich");
    assert_eq!(discarded_count, 1);
}

#[test]
fn campaign_frozen_overflow_does_not_replace_branch_by_strategic_summary_tie_break() {
    let mut frozen = vec![test_campaign_branch("existing-weak", 8, 70)];
    frozen[0].rank_key = 100;
    let mut incoming = test_campaign_branch("incoming-engine", 8, 70);
    incoming.rank_key = 100;
    incoming.strategic_summary = BranchSignatureCompact {
        present: true,
        boss_readiness_milli: 500,
        clean_score_milli: 700,
        engine_score_milli: 900,
        cycle_debt_milli: 100,
        setup_debt_milli: 100,
        economy_conversion_milli: 0,
        package_coherence_milli: 800,
    };
    let mut discarded_count = 0usize;
    let mut discarded_examples = Vec::new();

    let added = append_limited_frozen_v1(
        &mut frozen,
        vec![incoming],
        1,
        &mut discarded_count,
        &mut discarded_examples,
    );

    assert_eq!(added, 0);
    assert_eq!(frozen[0].branch_id, "existing-weak");
    assert_eq!(discarded_count, 1);
}

#[test]
fn campaign_frozen_merge_replaces_duplicate_with_better_branch() {
    let mut existing = test_campaign_branch("existing", 6, 70);
    existing.rank_key = 70;
    existing.choice_labels = vec!["old line".to_string()];
    let mut incoming = test_campaign_branch("incoming", 6, 70);
    incoming.rank_key = 80;
    incoming.choice_labels = vec!["better line".to_string()];
    let mut frozen = vec![existing];
    let mut discarded_count = 0usize;
    let mut discarded_examples = Vec::new();

    let added = append_limited_frozen_v1(
        &mut frozen,
        vec![incoming],
        4,
        &mut discarded_count,
        &mut discarded_examples,
    );

    assert_eq!(added, 1);
    assert_eq!(frozen.len(), 1);
    assert_eq!(frozen[0].branch_id, "incoming");
    assert_eq!(discarded_count, 1);
    assert_eq!(discarded_examples, vec!["merged duplicate: old line"]);
}

#[test]
fn campaign_selection_freezes_active_overflow_by_progress_then_rank() {
    let branches = vec![
        test_campaign_branch("a", 1, 80),
        test_campaign_branch("b", 2, 75),
        test_campaign_branch("c", 3, 70),
    ];

    let selected = select_campaign_branches_v1(branches, 2, 4);

    assert_eq!(selected.active.len(), 2);
    assert_eq!(selected.frozen.len(), 1);
    assert_eq!(
        selected
            .active
            .iter()
            .map(|branch| branch.branch_id.as_str())
            .collect::<Vec<_>>(),
        vec!["c", "b"]
    );
    assert_eq!(selected.frozen[0].branch_id, "a");
}

#[test]
fn campaign_selection_does_not_fill_active_cap_with_negative_rank_when_primary_exists() {
    let mut primary = test_campaign_branch("primary", 2, 80);
    primary.rank_key = 100;
    primary.summary.as_mut().unwrap().trajectory_key = "primary".to_string();
    let mut rejected = test_campaign_branch("rejected", 2, 80);
    rejected.rank_key = -50_000;
    rejected.summary.as_mut().unwrap().trajectory_key = "rejected".to_string();

    let selected = select_campaign_branches_v1(vec![primary, rejected], 2, 4);

    assert_eq!(selected.active.len(), 1);
    assert_eq!(selected.active[0].branch_id, "primary");
    assert_eq!(selected.frozen.len(), 1);
    assert_eq!(selected.frozen[0].branch_id, "rejected");
}

#[test]
fn campaign_selection_keeps_best_negative_rank_active_when_no_primary_exists() {
    let mut rejected_a = test_campaign_branch("rejected-a", 2, 80);
    rejected_a.rank_key = -50_000;
    rejected_a.summary.as_mut().unwrap().trajectory_key = "rejected-a".to_string();
    let mut rejected_b = test_campaign_branch("rejected-b", 2, 80);
    rejected_b.rank_key = -80_000;
    rejected_b.summary.as_mut().unwrap().trajectory_key = "rejected-b".to_string();

    let selected = select_campaign_branches_v1(vec![rejected_a, rejected_b], 2, 4);

    assert_eq!(selected.active.len(), 1);
    assert_eq!(selected.active[0].branch_id, "rejected-a");
    assert_eq!(selected.frozen.len(), 1);
    assert_eq!(selected.frozen[0].branch_id, "rejected-b");
}

#[test]
fn campaign_selection_prefers_primary_rank_over_deeper_negative_branch() {
    let mut positive = test_campaign_branch("positive-shop", 35, 82);
    positive.summary.as_mut().unwrap().act = 3;
    positive.frontier_title = "Shop".to_string();
    positive.rank_key = 34_300;

    let mut negative = test_campaign_branch("negative-combat", 37, 69);
    negative.summary.as_mut().unwrap().act = 3;
    negative.frontier_title = "Combat".to_string();
    negative.rank_key = -800_000;

    let selected = select_campaign_branches_v1(vec![negative, positive], 1, 4);

    assert_eq!(selected.active[0].branch_id, "positive-shop");
    assert_eq!(selected.frozen[0].branch_id, "negative-combat");
}

#[test]
fn campaign_selection_does_not_prefer_converted_gold_as_hidden_strategy() {
    let mut rich = test_campaign_branch("a-rich", 16, 30);
    rich.rank_key = 100;
    rich.summary.as_mut().unwrap().gold = 485;
    let mut converted = test_campaign_branch("b-converted", 16, 30);
    converted.rank_key = 100;
    converted.summary.as_mut().unwrap().gold = 120;

    let selected = select_campaign_branches_v1(vec![rich, converted], 1, 4);

    assert_eq!(selected.active.len(), 1);
    assert_eq!(
        selected.active[0].branch_id, "a-rich",
        "unspent gold pressure is a report concern, not a hidden campaign selector"
    );
}

#[test]
fn campaign_selection_keeps_raw_rank_when_unspent_gold_pressure_is_only_diagnostic() {
    let mut hoarded = test_campaign_branch("a-hoarded", 16, 41);
    hoarded.rank_key = 14_500;
    hoarded.summary.as_mut().unwrap().gold = 370;
    let mut healthy = test_campaign_branch("b-healthy", 15, 77);
    healthy.rank_key = 12_800;
    healthy.summary.as_mut().unwrap().gold = 120;

    let selected = select_campaign_branches_v1(vec![hoarded, healthy], 1, 4);

    assert_eq!(selected.active.len(), 1);
    assert_eq!(
        selected.active[0].branch_id, "a-hoarded",
        "campaign selection should not demote a branch solely because resource-concern reporting flags unspent gold"
    );
}

#[test]
fn campaign_selection_does_not_use_strategic_summary_as_ordinary_tie_break() {
    let mut weak = test_campaign_branch("a-weak", 6, 70);
    weak.rank_key = 100;
    weak.summary.as_mut().unwrap().trajectory_key = "weak".to_string();
    let mut engine = test_campaign_branch("z-engine", 6, 70);
    engine.rank_key = 100;
    engine.summary.as_mut().unwrap().trajectory_key = "engine".to_string();
    engine.strategic_summary = BranchSignatureCompact {
        present: true,
        boss_readiness_milli: 500,
        clean_score_milli: 600,
        engine_score_milli: 900,
        cycle_debt_milli: 100,
        setup_debt_milli: 100,
        economy_conversion_milli: 0,
        package_coherence_milli: 800,
    };

    let selected = select_campaign_branches_v1(vec![weak, engine], 1, 4);

    assert_eq!(selected.active.len(), 1);
    assert_eq!(
        selected.active[0].branch_id, "a-weak",
        "strategic summary should not be a hidden ordinary tie-break"
    );
}

#[test]
fn campaign_selection_does_not_use_strategy_summary_to_beat_raw_rank_gap_before_boss_approach() {
    let mut plain = test_campaign_branch("plain-short-term", 6, 78);
    plain.rank_key = 12_300;
    let mut package = test_campaign_branch("package-direction", 6, 77);
    package.rank_key = 12_100;
    package.summary.as_mut().unwrap().trajectory_key = "strength-scaling".to_string();
    package.strategic_summary = BranchSignatureCompact {
        present: true,
        boss_readiness_milli: 200,
        clean_score_milli: 1000,
        engine_score_milli: 300,
        cycle_debt_milli: 0,
        setup_debt_milli: 0,
        economy_conversion_milli: 0,
        package_coherence_milli: 500,
    };

    let selected = select_campaign_branches_v1(vec![plain, package], 1, 4);

    assert_eq!(selected.active.len(), 1);
    assert_eq!(
        selected.active[0].branch_id, "plain-short-term",
        "strategic summary should not beat raw rank before boss approach"
    );
}

#[test]
fn campaign_selection_prioritizes_progress_over_local_exact_rank() {
    let mut later_shop = test_campaign_branch("later-shop", 35, 82);
    later_shop.summary.as_mut().unwrap().act = 3;
    later_shop.frontier_title = "Shop".to_string();
    later_shop.rank_key = 34_368;

    let mut earlier_reward = test_campaign_branch("earlier-reward", 33, 81);
    earlier_reward.summary.as_mut().unwrap().act = 3;
    earlier_reward.frontier_title = "Reward Screen".to_string();
    earlier_reward.rank_key = 34_408;

    let selected = select_campaign_branches_v1(vec![earlier_reward, later_shop], 1, 4);

    assert_eq!(selected.active[0].branch_id, "later-shop");
}

#[test]
fn campaign_selection_prioritizes_boss_readiness_at_final_boss_checkpoint() {
    let mut short_term = test_campaign_branch("short-term-rank", 46, 50);
    short_term.summary.as_mut().unwrap().act = 3;
    short_term.summary.as_mut().unwrap().boss = "AwakenedOne".to_string();
    short_term.rank_key = 37_500;
    short_term.strategic_summary = BranchSignatureCompact {
        present: true,
        boss_readiness_milli: 200,
        clean_score_milli: 500,
        engine_score_milli: 0,
        cycle_debt_milli: 800,
        setup_debt_milli: 0,
        economy_conversion_milli: 0,
        package_coherence_milli: 300,
    };

    let mut boss_ready = test_campaign_branch("boss-ready", 46, 70);
    boss_ready.summary.as_mut().unwrap().act = 3;
    boss_ready.summary.as_mut().unwrap().boss = "AwakenedOne".to_string();
    boss_ready.rank_key = 28_500;
    boss_ready.strategic_summary = BranchSignatureCompact {
        present: true,
        boss_readiness_milli: 1000,
        clean_score_milli: 1000,
        engine_score_milli: 0,
        cycle_debt_milli: 600,
        setup_debt_milli: 200,
        economy_conversion_milli: 0,
        package_coherence_milli: 300,
    };

    let selected = select_campaign_branches_v1(vec![short_term, boss_ready], 1, 4);

    assert_eq!(selected.active[0].branch_id, "boss-ready");
}

#[test]
fn campaign_selection_prioritizes_act_clear_frontier_over_pre_boss_checkpoint() {
    let mut pre_boss = test_campaign_branch("pre-boss-campfire", 31, 62);
    pre_boss.summary.as_mut().unwrap().act = 2;
    pre_boss.frontier_title = "Campfire".to_string();
    pre_boss.rank_key = 23_767;
    pre_boss.strategic_summary = BranchSignatureCompact {
        present: true,
        boss_readiness_milli: 200,
        clean_score_milli: 1000,
        engine_score_milli: 0,
        cycle_debt_milli: 0,
        setup_debt_milli: 0,
        economy_conversion_milli: 0,
        package_coherence_milli: 0,
    };

    let mut act_clear = test_campaign_branch("act-clear-boss-relic", 32, 22);
    act_clear.summary.as_mut().unwrap().act = 2;
    act_clear.frontier_title = "Boss Relic".to_string();
    act_clear.rank_key = 23_563;
    act_clear.strategic_summary = BranchSignatureCompact {
        present: true,
        boss_readiness_milli: 0,
        clean_score_milli: 1000,
        engine_score_milli: 0,
        cycle_debt_milli: 0,
        setup_debt_milli: 0,
        economy_conversion_milli: 0,
        package_coherence_milli: 0,
    };

    let selected = select_campaign_branches_v1(vec![pre_boss, act_clear], 1, 4);

    assert_eq!(selected.active[0].branch_id, "act-clear-boss-relic");
}

#[test]
fn campaign_rebalance_allows_low_hp_act_clear_transition_over_pre_boss_active() {
    let mut pre_boss = test_campaign_branch("pre-boss-event", 30, 54);
    pre_boss.summary.as_mut().unwrap().act = 2;
    pre_boss.summary.as_mut().unwrap().max_hp = 85;
    pre_boss.frontier_title = "KnowingSkull".to_string();
    pre_boss.rank_key = 24_600;

    let mut act_clear = test_campaign_branch("act-clear-boss-relic", 32, 20);
    act_clear.summary.as_mut().unwrap().act = 2;
    act_clear.summary.as_mut().unwrap().max_hp = 85;
    act_clear.frontier_title = "Boss Relic".to_string();
    act_clear.rank_key = 23_500;

    let mut active = vec![pre_boss];
    let mut frozen = vec![act_clear];

    let promoted = rebalance_active_with_stronger_frozen_v1(&mut active, &mut frozen, 1);

    assert_eq!(promoted, 1);
    assert_eq!(active[0].branch_id, "act-clear-boss-relic");
    assert_eq!(frozen[0].branch_id, "pre-boss-event");
}

#[test]
fn campaign_selection_does_not_keep_act_clear_transition_over_next_act_progress() {
    let mut act_clear = test_campaign_branch("act1-boss-relic", 16, 61);
    act_clear.summary.as_mut().unwrap().act = 1;
    act_clear.frontier_title = "Boss Relic".to_string();
    act_clear.rank_key = 12_547;

    let mut next_act = test_campaign_branch("act2-reward", 18, 80);
    next_act.summary.as_mut().unwrap().act = 2;
    next_act.frontier_title = "Reward Screen".to_string();
    next_act.rank_key = 22_978;

    let selected = select_campaign_branches_v1(vec![act_clear, next_act], 1, 4);

    assert_eq!(selected.active[0].branch_id, "act2-reward");
    assert_eq!(selected.frozen[0].branch_id, "act1-boss-relic");
}

#[test]
fn campaign_selection_does_not_treat_low_hp_zero_readiness_boss_checkpoint_as_absolute_progress() {
    let mut low_hp_boss_door = test_campaign_branch("low-hp-boss-door", 46, 10);
    low_hp_boss_door.summary.as_mut().unwrap().act = 3;
    low_hp_boss_door.summary.as_mut().unwrap().max_hp = 80;
    low_hp_boss_door.summary.as_mut().unwrap().boss = "TimeEater".to_string();
    low_hp_boss_door.rank_key = 34_800;
    low_hp_boss_door.strategic_summary = BranchSignatureCompact {
        present: true,
        boss_readiness_milli: 0,
        clean_score_milli: 500,
        engine_score_milli: 0,
        cycle_debt_milli: 600,
        setup_debt_milli: 0,
        economy_conversion_milli: 0,
        package_coherence_milli: 300,
    };

    let mut healthy_pre_boss = test_campaign_branch("healthy-pre-boss", 45, 45);
    healthy_pre_boss.summary.as_mut().unwrap().act = 3;
    healthy_pre_boss.summary.as_mut().unwrap().max_hp = 80;
    healthy_pre_boss.summary.as_mut().unwrap().boss = "TimeEater".to_string();
    healthy_pre_boss.rank_key = 35_000;
    healthy_pre_boss.strategic_summary = BranchSignatureCompact {
        present: true,
        boss_readiness_milli: 0,
        clean_score_milli: 500,
        engine_score_milli: 0,
        cycle_debt_milli: 600,
        setup_debt_milli: 0,
        economy_conversion_milli: 0,
        package_coherence_milli: 300,
    };

    let selected = select_campaign_branches_v1(vec![low_hp_boss_door, healthy_pre_boss], 1, 4);

    assert_eq!(selected.active[0].branch_id, "healthy-pre-boss");
}

#[test]
fn campaign_selection_does_not_let_one_hp_at_boss_checkpoint_override_rank() {
    let mut slightly_healthier = test_campaign_branch("slightly-healthier", 27, 58);
    slightly_healthier.summary.as_mut().unwrap().act = 2;
    slightly_healthier.summary.as_mut().unwrap().max_hp = 80;
    slightly_healthier.rank_key = 21_900;
    slightly_healthier.strategic_summary = BranchSignatureCompact {
        present: true,
        boss_readiness_milli: 200,
        clean_score_milli: 0,
        engine_score_milli: 0,
        cycle_debt_milli: 400,
        setup_debt_milli: 0,
        economy_conversion_milli: 0,
        package_coherence_milli: 0,
    };

    let mut better_rank = test_campaign_branch("better-rank", 29, 57);
    better_rank.summary.as_mut().unwrap().act = 2;
    better_rank.summary.as_mut().unwrap().max_hp = 80;
    better_rank.rank_key = 22_400;
    better_rank.strategic_summary = slightly_healthier.strategic_summary;

    let selected = select_campaign_branches_v1(vec![slightly_healthier, better_rank], 1, 4);

    assert_eq!(selected.active[0].branch_id, "better-rank");
}

#[test]
fn campaign_selection_keeps_act2_boss_readiness_diagnostic_but_prioritizes_progress() {
    let mut partial_boss_answer = test_campaign_branch("partial-boss-answer", 26, 28);
    partial_boss_answer.summary.as_mut().unwrap().act = 2;
    partial_boss_answer.summary.as_mut().unwrap().max_hp = 28;
    partial_boss_answer.rank_key = 20_300;
    partial_boss_answer.strategic_summary = BranchSignatureCompact {
        present: true,
        boss_readiness_milli: 200,
        clean_score_milli: 1000,
        engine_score_milli: 0,
        cycle_debt_milli: 200,
        setup_debt_milli: 0,
        economy_conversion_milli: 0,
        package_coherence_milli: 0,
    };

    let mut stronger_general_branch = test_campaign_branch("stronger-general-branch", 24, 15);
    stronger_general_branch.summary.as_mut().unwrap().act = 2;
    stronger_general_branch.summary.as_mut().unwrap().max_hp = 85;
    stronger_general_branch.rank_key = 22_400;

    let selected =
        select_campaign_branches_v1(vec![partial_boss_answer, stronger_general_branch], 1, 4);

    assert_eq!(selected.active[0].branch_id, "partial-boss-answer");
}

#[test]
fn campaign_selection_keeps_progress_anchor_when_local_shop_variants_dominate_active() {
    let mut buy_flash = test_campaign_branch("shop-flash", 39, 77);
    buy_flash.summary.as_mut().unwrap().act = 3;
    buy_flash.summary.as_mut().unwrap().gold = 528;
    buy_flash.frontier_title = "Shop".to_string();
    buy_flash.rank_key = 35_198;
    buy_flash.choice_labels = vec!["Buy Flash of Steel | 60 gold".to_string()];

    let mut buy_heavy = test_campaign_branch("shop-heavy", 39, 77);
    buy_heavy.summary.as_mut().unwrap().act = 3;
    buy_heavy.summary.as_mut().unwrap().gold = 234;
    buy_heavy.frontier_title = "Shop".to_string();
    buy_heavy.rank_key = 35_160;
    buy_heavy.choice_labels = vec!["Buy Heavy Blade | 70 gold".to_string()];

    let mut campfire = test_campaign_branch("campfire-rest", 42, 77);
    campfire.summary.as_mut().unwrap().act = 3;
    campfire.summary.as_mut().unwrap().gold = 47;
    campfire.frontier_title = "Campfire".to_string();
    campfire.rank_key = 35_017;
    campfire.choice_labels = vec!["Rest".to_string()];

    let selected = select_campaign_branches_v1(vec![buy_flash, buy_heavy, campfire], 2, 4);

    let active_ids = selected
        .active
        .iter()
        .map(|branch| branch.branch_id.as_str())
        .collect::<Vec<_>>();
    assert!(active_ids.contains(&"shop-flash"));
    assert!(
        active_ids.contains(&"campfire-rest"),
        "local shop purchase variants should not crowd out a clearly progressed branch"
    );
    assert_eq!(selected.frozen[0].branch_id, "shop-heavy");
}

#[test]
fn campaign_rebalance_does_not_promote_critical_hp_nearby_progress_over_healthy_active() {
    let mut healthy = test_campaign_branch("healthy-nearby", 29, 41);
    healthy.summary.as_mut().unwrap().act = 2;
    healthy.summary.as_mut().unwrap().max_hp = 100;
    healthy.rank_key = 24_000;

    let mut critical_ahead = test_campaign_branch("critical-ahead", 30, 12);
    critical_ahead.summary.as_mut().unwrap().act = 2;
    critical_ahead.summary.as_mut().unwrap().max_hp = 100;
    critical_ahead.rank_key = 24_500;

    let mut active = vec![healthy];
    let mut frozen = vec![critical_ahead];

    let promoted = rebalance_active_with_stronger_frozen_v1(&mut active, &mut frozen, 1);

    assert_eq!(promoted, 0);
    assert_eq!(active[0].branch_id, "healthy-nearby");
    assert_eq!(frozen[0].branch_id, "critical-ahead");
}

#[test]
fn campaign_selection_promotes_nearby_healthy_frozen_over_critical_active() {
    let mut critical_a = test_campaign_branch("critical-a", 30, 12);
    critical_a.summary.as_mut().unwrap().act = 2;
    critical_a.summary.as_mut().unwrap().max_hp = 100;
    critical_a.rank_key = 25_000;

    let mut critical_b = test_campaign_branch("critical-b", 30, 9);
    critical_b.summary.as_mut().unwrap().act = 2;
    critical_b.summary.as_mut().unwrap().max_hp = 100;
    critical_b.rank_key = 24_900;

    let mut healthy_nearby = test_campaign_branch("healthy-nearby", 29, 41);
    healthy_nearby.summary.as_mut().unwrap().act = 2;
    healthy_nearby.summary.as_mut().unwrap().max_hp = 100;
    healthy_nearby.rank_key = 24_000;

    let selected = select_campaign_branches_v1(vec![critical_a, critical_b, healthy_nearby], 2, 4);

    let active_ids = selected
        .active
        .iter()
        .map(|branch| branch.branch_id.as_str())
        .collect::<Vec<_>>();

    assert!(active_ids.contains(&"healthy-nearby"));
    assert_eq!(selected.frozen.len(), 1);
}

#[test]
fn campaign_rebalance_promotes_healthy_salvage_checkpoint_over_critical_active() {
    let mut critical_late = test_campaign_branch("critical-late", 45, 17);
    critical_late.summary.as_mut().unwrap().act = 3;
    critical_late.summary.as_mut().unwrap().max_hp = 80;
    critical_late.rank_key = 35_900;

    let mut healthy_checkpoint = test_campaign_branch("healthy-checkpoint", 38, 80);
    healthy_checkpoint.summary.as_mut().unwrap().act = 3;
    healthy_checkpoint.summary.as_mut().unwrap().max_hp = 80;
    healthy_checkpoint.rank_key = 34_000;

    let mut active = vec![critical_late];
    let mut frozen = vec![healthy_checkpoint];

    let promoted = rebalance_active_with_stronger_frozen_v1(&mut active, &mut frozen, 1);

    assert_eq!(promoted, 1);
    assert_eq!(active[0].branch_id, "healthy-checkpoint");
    assert_eq!(frozen[0].branch_id, "critical-late");
}

#[test]
fn campaign_rebalance_promotes_stable_salvage_checkpoint_over_deep_critical_active() {
    let mut deep_critical = test_campaign_branch("deep-critical", 42, 6);
    deep_critical.summary.as_mut().unwrap().act = 3;
    deep_critical.summary.as_mut().unwrap().max_hp = 95;
    deep_critical.rank_key = 33_800;

    let mut stable_checkpoint = test_campaign_branch("stable-checkpoint", 39, 32);
    stable_checkpoint.summary.as_mut().unwrap().act = 3;
    stable_checkpoint.summary.as_mut().unwrap().max_hp = 95;
    stable_checkpoint.rank_key = 33_900;

    let mut active = vec![deep_critical];
    let mut frozen = vec![stable_checkpoint];

    let promoted = rebalance_active_with_stronger_frozen_v1(&mut active, &mut frozen, 1);

    assert_eq!(promoted, 1);
    assert_eq!(active[0].branch_id, "stable-checkpoint");
    assert_eq!(frozen[0].branch_id, "deep-critical");
}

#[test]
fn campaign_rebalance_does_not_promote_stale_rehydrated_combat_over_later_active() {
    let mut active = vec![test_campaign_branch_with_boundary(
        "act3-campfire",
        "Campfire",
        "campfire action requires human choice",
        47,
        74,
    )];
    active[0].summary.as_mut().expect("summary").act = 3;
    active[0].rank_key = 35_600;

    let mut stale = test_campaign_branch_with_boundary(
        "act2-rehydrated-combat",
        "Combat",
        "rehydrated checkpointed Abandoned combat branch: combat search did not find an executable complete win",
        32,
        87,
    );
    stale.summary.as_mut().expect("summary").act = 2;
    stale.rank_key = 800_000;
    let mut frozen = vec![stale];

    let promoted = rebalance_active_with_stronger_frozen_v1(&mut active, &mut frozen, 2);

    assert_eq!(promoted, 0);
    assert_eq!(active[0].branch_id, "act3-campfire");
    assert_eq!(frozen[0].branch_id, "act2-rehydrated-combat");
}

#[test]
fn campaign_recovered_checkpointed_combat_failure_is_frozen_not_active() {
    let mut active = Vec::new();
    let mut frozen = Vec::new();
    let recovered = test_campaign_branch_with_boundary(
        "rehydrated-combat",
        "Combat",
        "rehydrated checkpointed Abandoned combat branch: combat search did not find an executable complete win",
        48,
        97,
    );

    assert!(place_recovered_campaign_branch_v1(
        &mut active,
        &mut frozen,
        recovered,
        2,
        4
    ));

    assert!(active.is_empty());
    assert_eq!(frozen.len(), 1);
    assert_eq!(frozen[0].branch_id, "rehydrated-combat");
}

#[test]
fn campaign_promotion_prefers_real_frozen_branch_over_rehydrated_combat_failure() {
    let mut active = Vec::new();
    let mut stale = test_campaign_branch_with_boundary(
        "rehydrated-combat",
        "Combat",
        "rehydrated checkpointed Abandoned combat branch: combat search did not find an executable complete win",
        48,
        97,
    );
    stale.rank_key = 999_999;
    let mut real_branch = test_campaign_branch_with_boundary(
        "campfire-branch",
        "Campfire",
        "campfire action requires human choice",
        47,
        64,
    );
    real_branch.rank_key = 35_000;
    let mut frozen = vec![stale, real_branch];

    let promoted = promote_frozen_to_active_v1(&mut active, &mut frozen, 1);

    assert_eq!(promoted, 1);
    assert_eq!(active[0].branch_id, "campfire-branch");
    assert!(frozen
        .iter()
        .any(|branch| branch.branch_id == "rehydrated-combat"));
}

#[test]
fn campaign_selection_merges_duplicate_quality_branches() {
    let mut best = test_campaign_branch("best-frontload", 5, 80);
    best.rank_key = 120;
    best.choice_labels = vec!["Pommel Strike".to_string()];
    let mut duplicate = test_campaign_branch("weaker-frontload", 5, 80);
    duplicate.rank_key = 100;
    duplicate.choice_labels = vec!["Twin Strike".to_string()];
    let mut distinct = test_campaign_branch("distinct-defense", 5, 80);
    distinct.rank_key = 80;
    distinct.choice_labels = vec!["Shrug It Off".to_string()];
    distinct.summary.as_mut().unwrap().trajectory_key = "defense=1".to_string();

    let selected = select_campaign_branches_v1(vec![duplicate, distinct, best], 2, 4);

    assert_eq!(
        selected
            .active
            .iter()
            .map(|branch| branch.branch_id.as_str())
            .collect::<Vec<_>>(),
        vec!["best-frontload", "distinct-defense"]
    );
    assert!(selected.frozen.is_empty());
    assert_eq!(selected.discarded_count, 1);
    assert_eq!(
        selected.discarded_examples,
        vec!["merged duplicate: Twin Strike"]
    );
}

#[test]
fn campaign_selection_does_not_merge_distinct_deck_fingerprints() {
    let mut pommel = test_campaign_branch("pommel-shop", 5, 80);
    pommel.rank_key = 120;
    pommel.choice_labels = vec!["Buy Pommel Strike".to_string()];
    pommel.summary.as_mut().unwrap().deck_key = "Bash+0x1;PommelStrike+0x1;Strike+0x5".to_string();

    let mut shrug = test_campaign_branch("shrug-shop", 5, 80);
    shrug.rank_key = 100;
    shrug.choice_labels = vec!["Buy Shrug It Off".to_string()];
    shrug.summary.as_mut().unwrap().deck_key = "Bash+0x1;ShrugItOff+0x1;Strike+0x5".to_string();

    let selected = select_campaign_branches_v1(vec![shrug, pommel], 2, 4);

    assert_eq!(
        selected
            .active
            .iter()
            .map(|branch| branch.branch_id.as_str())
            .collect::<Vec<_>>(),
        vec!["pommel-shop", "shrug-shop"]
    );
    assert_eq!(selected.discarded_count, 0);
}

#[test]
fn campaign_branch_experiment_config_preserves_shop_auto_leave_guard() {
    let config = BranchCampaignConfigV1::default();

    let experiment_config = campaign_branch_experiment_config_v1(&config);

    assert!(
        experiment_config.auto_leave_after_shop_purchase_branch,
        "campaign runs should not repeatedly burn rounds on one-item shop purchase branches"
    );
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
fn campaign_promotion_does_not_fill_secondary_active_slot_with_negative_rank() {
    let mut active = vec![test_campaign_branch("primary", 4, 80)];
    active[0].rank_key = 100;
    let mut rejected = test_campaign_branch("rejected", 7, 75);
    rejected.status = BranchCampaignBranchStatusV1::Frozen;
    rejected.rank_key = -50_000;
    let mut frozen = vec![rejected];

    let promoted = promote_frozen_to_active_v1(&mut active, &mut frozen, 2);

    assert_eq!(promoted, 0);
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].branch_id, "primary");
    assert_eq!(frozen.len(), 1);
    assert_eq!(frozen[0].branch_id, "rejected");
}

#[test]
fn campaign_promotion_does_not_prefer_converted_gold_as_hidden_strategy() {
    let mut active = Vec::new();
    let mut rich = test_campaign_branch("a-rich", 16, 30);
    rich.status = BranchCampaignBranchStatusV1::Frozen;
    rich.summary.as_mut().unwrap().gold = 485;
    let mut converted = test_campaign_branch("b-converted", 16, 30);
    converted.status = BranchCampaignBranchStatusV1::Frozen;
    converted.summary.as_mut().unwrap().gold = 120;
    let mut frozen = vec![rich, converted];

    let promoted = promote_frozen_to_active_v1(&mut active, &mut frozen, 1);

    assert_eq!(promoted, 1);
    assert_eq!(active[0].branch_id, "a-rich");
    assert_eq!(frozen[0].branch_id, "b-converted");
}

#[test]
fn campaign_rebalances_stronger_frozen_branch_into_active_pool() {
    let mut active = vec![
        test_campaign_branch("active-a", 23, 68),
        test_campaign_branch("active-b", 22, 80),
    ];
    for branch in &mut active {
        branch.status = BranchCampaignBranchStatusV1::Active;
        branch.rank_key = 23_500;
    }
    let mut frozen = vec![{
        let mut branch = test_campaign_branch("frozen-strength", 23, 73);
        branch.status = BranchCampaignBranchStatusV1::Frozen;
        branch.rank_key = 25_500;
        branch
    }];

    let promoted = rebalance_active_with_stronger_frozen_v1(&mut active, &mut frozen, 2);

    assert_eq!(promoted, 1);
    assert!(active
        .iter()
        .any(|branch| branch.branch_id == "frozen-strength"));
    assert_eq!(
        active
            .iter()
            .filter(|branch| branch.status == BranchCampaignBranchStatusV1::Active)
            .count(),
        2
    );
    assert!(frozen
        .iter()
        .all(|branch| branch.status == BranchCampaignBranchStatusV1::Frozen));
}

#[test]
fn campaign_rebalance_repeats_until_active_pool_is_stable() {
    let mut active = vec![
        test_campaign_branch("active-weak-a", 20, 60),
        test_campaign_branch("active-weak-b", 20, 59),
    ];
    for branch in &mut active {
        branch.status = BranchCampaignBranchStatusV1::Active;
        branch.rank_key = 20_000;
    }
    let mut frozen = vec![
        {
            let mut branch = test_campaign_branch("frozen-strong-a", 21, 73);
            branch.status = BranchCampaignBranchStatusV1::Frozen;
            branch.rank_key = 25_500;
            branch
        },
        {
            let mut branch = test_campaign_branch("frozen-strong-b", 21, 72);
            branch.status = BranchCampaignBranchStatusV1::Frozen;
            branch.rank_key = 25_400;
            branch
        },
    ];

    let promoted = rebalance_active_with_stronger_frozen_v1(&mut active, &mut frozen, 2);

    assert_eq!(promoted, 2);
    let active_ids = active
        .iter()
        .map(|branch| branch.branch_id.as_str())
        .collect::<Vec<_>>();
    assert!(active_ids.contains(&"frozen-strong-a"));
    assert!(active_ids.contains(&"frozen-strong-b"));
}

#[test]
fn campaign_rebalance_promotes_next_act_progress_over_stale_act_clear_transition() {
    let mut active = vec![
        {
            let mut branch = test_campaign_branch("act1-boss-relic", 16, 61);
            branch.summary.as_mut().unwrap().act = 1;
            branch.frontier_title = "Boss Relic".to_string();
            branch.rank_key = 12_547;
            branch.strategic_summary = BranchSignatureCompact {
                present: true,
                boss_readiness_milli: 200,
                clean_score_milli: 1000,
                engine_score_milli: 0,
                cycle_debt_milli: 0,
                setup_debt_milli: 0,
                economy_conversion_milli: 0,
                package_coherence_milli: 0,
            };
            branch
        },
        {
            let mut branch = test_campaign_branch("act1-reward", 16, 46);
            branch.summary.as_mut().unwrap().act = 1;
            branch.frontier_title = "Reward Screen".to_string();
            branch.rank_key = 12_388;
            branch
        },
    ];
    let mut frozen = vec![
        {
            let mut branch = test_campaign_branch("act2-reward", 18, 80);
            branch.summary.as_mut().unwrap().act = 2;
            branch.frontier_title = "Reward Screen".to_string();
            branch.rank_key = 22_978;
            branch.status = BranchCampaignBranchStatusV1::Frozen;
            branch
        },
        {
            let mut branch = test_campaign_branch("act2-nest", 19, 65);
            branch.summary.as_mut().unwrap().act = 2;
            branch.frontier_title = "Nest".to_string();
            branch.rank_key = 22_937;
            branch.status = BranchCampaignBranchStatusV1::Frozen;
            branch
        },
    ];

    let promoted = rebalance_active_with_stronger_frozen_v1(&mut active, &mut frozen, 2);

    assert_eq!(promoted, 2);
    let active_ids = active
        .iter()
        .map(|branch| branch.branch_id.as_str())
        .collect::<Vec<_>>();
    assert!(active_ids.contains(&"act2-reward"));
    assert!(active_ids.contains(&"act2-nest"));
}

#[test]
fn campaign_rebalance_does_not_replace_progress_anchor_with_local_shop_variant() {
    let mut shop = test_campaign_branch("shop-representative", 39, 77);
    shop.summary.as_mut().unwrap().act = 3;
    shop.frontier_title = "Shop".to_string();
    shop.rank_key = 35_200;

    let mut campfire = test_campaign_branch("progress-anchor", 42, 77);
    campfire.summary.as_mut().unwrap().act = 3;
    campfire.frontier_title = "Campfire".to_string();
    campfire.rank_key = 35_017;

    let mut active = vec![shop, campfire];
    for branch in &mut active {
        branch.status = BranchCampaignBranchStatusV1::Active;
    }

    let mut frozen_shop = test_campaign_branch("frozen-shop-variant", 39, 77);
    frozen_shop.summary.as_mut().unwrap().act = 3;
    frozen_shop.frontier_title = "Shop".to_string();
    frozen_shop.rank_key = 35_800;
    frozen_shop.status = BranchCampaignBranchStatusV1::Frozen;
    let mut frozen = vec![frozen_shop];

    let promoted = rebalance_active_with_stronger_frozen_v1(&mut active, &mut frozen, 2);

    assert_eq!(
        promoted, 0,
        "a stronger local shop variant should not evict the only clearly progressed branch"
    );
    assert!(active
        .iter()
        .any(|branch| branch.branch_id == "progress-anchor"));
    assert_eq!(frozen[0].branch_id, "frozen-shop-variant");
}

#[test]
fn campaign_lineage_diversity_promotes_distinct_boss_relic_axis() {
    let mut active = vec![
        test_campaign_branch("key-a", 18, 80),
        test_campaign_branch("key-b", 18, 79),
    ];
    active[0].choice_labels = vec!["CursedKey".to_string(), "Pommel Strike".to_string()];
    active[1].choice_labels = vec!["CursedKey".to_string(), "Shrug It Off".to_string()];
    for branch in &mut active {
        branch.status = BranchCampaignBranchStatusV1::Active;
        branch.rank_key = 20_000;
    }
    let mut frozen = vec![{
        let mut branch = test_campaign_branch("hammer-a", 18, 78);
        branch.status = BranchCampaignBranchStatusV1::Frozen;
        branch.choice_labels = vec!["FusionHammer".to_string(), "Pommel Strike".to_string()];
        branch.rank_key = 19_950;
        branch
    }];

    let promoted = rebalance_active_lineage_diversity_v1(&mut active, &mut frozen, 2);

    assert_eq!(promoted, 1);
    assert!(active.iter().any(|branch| branch.branch_id == "hammer-a"));
    assert_eq!(
        active
            .iter()
            .filter_map(campaign_branch_boss_relic_lineage_key_v1)
            .collect::<BTreeSet<_>>(),
        BTreeSet::from(["CursedKey".to_string(), "FusionHammer".to_string()])
    );
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
    let gate = branch_report_act_boss_gate_retry_key_v1(&[abandoned_act1_boss])
        .expect("act boss combat should have a retry gate key");
    let mut ledger = BranchCampaignCombatRetryLedgerStateV1::default();

    assert!(ledger.try_consume_boss_gate_retry_v1(gate));
    assert!(ledger.try_consume_boss_gate_retry_v1(gate));
    assert!(!ledger.try_consume_boss_gate_retry_v1(gate));

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

    assert_eq!(
        branch_report_act_boss_gate_retry_key_v1(&[abandoned_hallway]),
        None
    );
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
        active: vec![test_campaign_branch("active", 2, 80)],
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

#[test]
fn compact_campaign_report_renders_strategy_prompt() {
    let report = BranchCampaignReportV1 {
        schema_name: "BranchCampaignV1".to_string(),
        schema_version: 1,
        seed: 521,
        run_domain: BranchCampaignRunDomainV1::default(),
        rounds_completed: 2,
        stop_reason: "needs_intervention".to_string(),
        active: Vec::new(),
        frozen: Vec::new(),
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: Vec::new(),
        stuck: Vec::new(),
        discarded_count: 3,
        discarded_examples: Vec::new(),
        strategy_requests: vec![BranchCampaignStrategyRequestV1 {
            kind: "event_strategy".to_string(),
            boundary_title: "Falling".to_string(),
            branch_count: 2,
            act: 0,
            floor: 0,
            stop_reasons: vec!["event policy gap".to_string()],
            examples: vec!["Strike -> Defend".to_string()],
            next_card_reward_offer: None,
            boundary_details: Vec::new(),
            suggested_action: "provide Falling policy".to_string(),
        }],
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
        rounds: Vec::new(),
    };

    let rendered = render_branch_campaign_compact_v1(&report, 2);

    assert!(rendered.contains(
        "BranchCampaignV1 seed=521 ascension=A0 domain=debug_a0 role=debug_domain class=Ironclad rounds=2 stop=needs_intervention"
    ));
    assert!(rendered.contains(
        "Active 0 | Frozen 0 | Dead 0 | Abandoned 0 | Victories 0 | Stuck 0 | Discarded 3"
    ));
    assert!(rendered.contains("Needs intervention:"));
    assert!(rendered.contains("event_strategy | Falling | branches=2"));
    assert!(rendered.contains(
        "next: write a narrow event rule or choose one branch manually, then rerun the campaign"
    ));
    assert!(!rendered.contains("Top active:"));
}

#[test]
fn compact_campaign_report_renders_actionable_intervention_details() {
    let report = BranchCampaignReportV1 {
        schema_name: "BranchCampaignV1".to_string(),
        schema_version: 1,
        seed: 521,
        run_domain: BranchCampaignRunDomainV1::default(),
        rounds_completed: 6,
        stop_reason: "needs_intervention".to_string(),
        active: Vec::new(),
        frozen: Vec::new(),
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: vec![test_campaign_branch("a", 16, 70)],
        stuck: Vec::new(),
        discarded_count: 0,
        discarded_examples: Vec::new(),
        strategy_requests: vec![BranchCampaignStrategyRequestV1 {
            kind: "combat_manual_or_budget".to_string(),
            boundary_title: "Combat".to_string(),
            branch_count: 2,
            act: 0,
            floor: 0,
            stop_reasons: vec!["combat search did not find an executable complete win".to_string()],
            examples: vec!["Clash -> Disarm".to_string()],
            next_card_reward_offer: None,
            boundary_details: Vec::new(),
            suggested_action: "raise combat search budget".to_string(),
        }],
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
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
            ..BranchCampaignRoundSummaryV1::default()
        }],
    };

    let rendered = render_branch_campaign_compact_v1(&report, 2);

    assert!(rendered.contains("Needs intervention:"));
    assert!(rendered.contains("kind: combat_unresolved_after_retry"));
    assert!(rendered.contains("stop: combat search did not find an executable complete win"));
    assert!(rendered.contains("tried: campaign search budget; combat budget retry x2"));
    assert!(rendered.contains("possible inputs: switch macro branch | provide combat tactic | add upstream route/reward rule | raise retry budget only if under-spent"));
}

#[test]
fn compact_campaign_report_suppresses_deferred_strategy_notes_while_active_continues() {
    let report = BranchCampaignReportV1 {
        schema_name: "BranchCampaignV1".to_string(),
        schema_version: 1,
        seed: 330404105,
        run_domain: BranchCampaignRunDomainV1::default(),
        rounds_completed: 2,
        stop_reason: "max_rounds".to_string(),
        active: vec![test_campaign_branch("a", 6, 80)],
        frozen: vec![test_campaign_branch("f", 5, 75)],
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: Vec::new(),
        stuck: vec![test_campaign_branch("s", 6, 70)],
        discarded_count: 0,
        discarded_examples: Vec::new(),
        strategy_requests: vec![BranchCampaignStrategyRequestV1 {
            kind: "combat_manual_or_budget".to_string(),
            boundary_title: "Combat".to_string(),
            branch_count: 1,
            act: 0,
            floor: 0,
            stop_reasons: vec!["combat search did not find an executable complete win".to_string()],
            examples: vec!["Flex".to_string()],
            next_card_reward_offer: None,
            boundary_details: Vec::new(),
            suggested_action: "raise combat search budget".to_string(),
        }],
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
        rounds: Vec::new(),
    };

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(!rendered.contains("Deferred strategy notes:"));
    assert!(!rendered.contains("Needs intervention:"));
    assert!(!rendered.contains("stop: combat search did not find an executable complete win"));
}

#[test]
fn compact_campaign_report_suppresses_stale_strategy_notes_after_victory() {
    let mut report = test_campaign_report_with_active("winner", 48, 42);
    report.stop_reason = "victory_found".to_string();
    report.active.clear();
    report.victories = vec![test_campaign_branch("winner", 48, 42)];
    report.stuck = vec![test_campaign_branch("old-stuck", 45, 35)];
    report.strategy_requests = vec![test_campaign_request("event_strategy", "MatchAndKeep")];

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(!rendered.contains("Deferred strategy notes:"));
    assert!(!rendered.contains("Needs intervention:"));
    assert!(!rendered.contains("MatchAndKeep"));
}

#[test]
fn compact_campaign_report_renders_first_and_best_victory_quality() {
    let mut report = test_campaign_report_with_active("winner", 48, 42);
    report.active.clear();
    let mut first = test_campaign_branch("first-win", 48, 6);
    first.status = BranchCampaignBranchStatusV1::TerminalVictory;
    first.choice_labels = vec!["risky line".to_string()];
    let mut best = test_campaign_branch("best-win", 48, 32);
    best.status = BranchCampaignBranchStatusV1::TerminalVictory;
    best.choice_labels = vec!["stable line".to_string()];
    report.victories = vec![first, best];

    let rendered = render_branch_campaign_compact_v1(&report, 2);

    assert!(rendered.contains("First victory: A1F48 HP 6/80"));
    assert!(rendered.contains("choices: risky line"));
    assert!(rendered.contains("Best victory: A1F48 HP 32/80"));
    assert!(rendered.contains("choices: stable line"));
}

#[test]
fn compact_campaign_report_renders_needs_intervention_even_with_frozen_branches() {
    let mut report = test_campaign_report_with_active("a", 16, 70);
    report.stop_reason = "needs_intervention".to_string();
    report.active.clear();
    report.frozen = vec![test_campaign_branch("old-frozen", 4, 80)];
    report.strategy_requests = vec![test_campaign_request("combat_manual_or_budget", "Combat")];

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains("Needs intervention:"));
    assert!(!rendered.contains("Deferred strategy notes:"));
}

#[test]
fn campaign_builds_intervention_when_abandoned_branches_exhaust_routes() {
    let mut a = test_campaign_branch("a", 16, 70);
    a.stop_reason = "combat search did not find an executable complete win".to_string();
    let mut b = test_campaign_branch("b", 16, 62);
    b.stop_reason = "wall-clock deadline hit".to_string();
    let request = abandoned_branches_intervention_request_v1(&[a, b]).expect("request");

    assert_eq!(request.kind, "combat_manual_or_budget");
    assert_eq!(request.boundary_title, "Combat");
    assert_eq!(request.branch_count, 2);
    assert_eq!(request.examples.len(), 2);
    assert_eq!(
        request.stop_reasons,
        vec![
            "combat search did not find an executable complete win".to_string(),
            "wall-clock deadline hit".to_string()
        ]
    );
}

#[test]
fn campaign_records_deferred_note_for_leading_abandoned_combat_while_frozen_exists() {
    let mut abandoned = test_campaign_branch("abandoned-combat", 48, 78);
    abandoned.status = BranchCampaignBranchStatusV1::Abandoned;
    abandoned.frontier_title = "Combat".to_string();
    abandoned.stop_reason = "combat search did not find an executable complete win".to_string();
    abandoned.summary.as_mut().expect("summary").act = 3;
    let frozen = vec![test_campaign_branch("old-frozen", 11, 80)];

    let request = leading_abandoned_combat_intervention_request_v1(&frozen, &[abandoned])
        .expect("leading abandoned combat should request intervention");

    assert_eq!(request.kind, "combat_manual_or_budget");
    assert_eq!(request.boundary_title, "Combat");
    assert_eq!(request.act, 3);
    assert_eq!(request.floor, 48);
    assert_eq!(
        request.stop_reasons,
        vec!["combat search did not find an executable complete win".to_string()]
    );
}

#[test]
fn campaign_does_not_request_leading_abandoned_intervention_when_frozen_is_not_behind() {
    let mut abandoned = test_campaign_branch("abandoned-combat", 12, 78);
    abandoned.status = BranchCampaignBranchStatusV1::Abandoned;
    abandoned.frontier_title = "Combat".to_string();
    abandoned.stop_reason = "combat search did not find an executable complete win".to_string();
    let frozen = vec![test_campaign_branch("same-progress-frozen", 12, 80)];

    assert!(leading_abandoned_combat_intervention_request_v1(&frozen, &[abandoned]).is_none());
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
fn campaign_selection_treats_combat_stuck_as_abandoned_route_branch() {
    let mut combat = test_campaign_branch_with_boundary(
        "combat-stuck",
        "Combat",
        "combat search did not find an executable complete win",
        12,
        55,
    );
    combat.status = BranchCampaignBranchStatusV1::Stuck;

    let selected = select_campaign_branches_v1(vec![combat], 2, 4);

    assert_eq!(selected.abandoned.len(), 1);
    assert_eq!(selected.abandoned[0].branch_id, "combat-stuck");
    assert!(selected.stuck.is_empty());
}

#[test]
fn campaign_selection_keeps_noncombat_stuck_for_strategy_intervention() {
    let mut event = test_campaign_branch_with_boundary(
        "event-stuck",
        "Falling",
        "event option requires human choice",
        36,
        70,
    );
    event.status = BranchCampaignBranchStatusV1::Stuck;

    let selected = select_campaign_branches_v1(vec![event], 2, 4);

    assert_eq!(selected.stuck.len(), 1);
    assert_eq!(selected.stuck[0].branch_id, "event-stuck");
    assert!(selected.abandoned.is_empty());
}

#[test]
fn campaign_strategy_request_prune_drops_requests_without_matching_blocked_branch() {
    let active = vec![test_campaign_branch("a", 10, 80)];
    let stuck = vec![test_campaign_branch_with_boundary(
        "s",
        "Map Preview",
        "route planner declined automatic map selection",
        16,
        60,
    )];
    let requests = vec![
        {
            let mut request = test_campaign_request("event_strategy", "Beggar");
            request.act = 2;
            request.floor = 21;
            request.stop_reasons = vec!["event option requires human choice".to_string()];
            request
        },
        {
            let mut request = test_campaign_request("route_policy_gap", "Map Preview");
            request.act = 1;
            request.floor = 16;
            request.stop_reasons =
                vec!["route planner declined automatic map selection".to_string()];
            request
        },
    ];

    let pruned = prune_resolved_campaign_strategy_requests_v1(requests, &active, &[], &stuck, &[]);

    assert_eq!(pruned.len(), 1);
    assert_eq!(pruned[0].kind, "route_policy_gap");
    assert_eq!(pruned[0].boundary_title, "Map Preview");
}

#[test]
fn campaign_recovers_stuck_branch_if_one_auto_step_leaves_frontier() {
    let mut active = Vec::new();
    let mut frozen = Vec::new();
    let mut stuck = vec![test_campaign_branch_with_boundary(
        "s",
        "Beggar",
        "event option requires human choice",
        21,
        53,
    )];
    let mut state_store = super::state_graph::BranchStateStoreV1::new();
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.event_state = Some(EventState {
        id: EventId::Beggar,
        current_screen: 2,
        internal_state: 0,
        completed: false,
        combat_pending: false,
        extra_data: Vec::new(),
    });
    session.engine_state = EngineState::EventRoom;
    state_store.insert_session(stuck[0].commands.clone(), session);

    let recovered = recover_auto_advanceable_stuck_branches_v1(
        &mut active,
        &mut frozen,
        &mut stuck,
        &mut state_store,
        1,
        0,
    );

    assert_eq!(recovered, 1);
    assert_eq!(stuck.len(), 0);
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].frontier_title, "Map");
    assert!(matches!(
        state_store
            .get_session(&active[0].commands)
            .expect("recovered snapshot should be retained")
            .engine_state,
        EngineState::MapNavigation
    ));
}

#[test]
fn campaign_recovers_stale_empty_campfire_portfolio_when_boundary_is_now_available() {
    let mut active = Vec::new();
    let mut frozen = Vec::new();
    let mut stuck = vec![test_campaign_branch_with_boundary(
        "campfire-stale",
        "Campfire",
        "campfire option portfolio is empty",
        38,
        80,
    )];
    let mut state_store = super::state_graph::BranchStateStoreV1::new();
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.engine_state = EngineState::Campfire;
    state_store.insert_session(stuck[0].commands.clone(), session);

    let recovered = recover_auto_advanceable_stuck_branches_v1(
        &mut active,
        &mut frozen,
        &mut stuck,
        &mut state_store,
        1,
        4,
    );

    assert_eq!(recovered, 1);
    assert!(stuck.is_empty());
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].branch_id, "campfire-stale");
    assert_eq!(active[0].frontier_title, "Campfire");
    assert!(active[0]
        .stop_reason
        .contains("recovered from current branch boundary"));
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
fn campaign_strategy_request_merge_preserves_context_for_human_research() {
    let mut request = test_experiment_request("event_strategy", "GoldenIdol", "event policy gap");
    request.act = 1;
    request.floor = 5;
    request.next_card_reward_offer = Some(vec![
        "Pommel Strike".to_string(),
        "Iron Wave".to_string(),
        "Searing Blow".to_string(),
    ]);
    request.boundary_details = vec![
        "option: [Take] Obtain Golden Idol.".to_string(),
        "option: [Leave] Leave.".to_string(),
    ];

    let merged = merge_campaign_strategy_requests_v1(vec![request]);

    assert_eq!(merged[0].act, 1);
    assert_eq!(merged[0].floor, 5);
    assert_eq!(
        merged[0].next_card_reward_offer,
        Some(vec![
            "Pommel Strike".to_string(),
            "Iron Wave".to_string(),
            "Searing Blow".to_string(),
        ])
    );
    assert_eq!(merged[0].boundary_details.len(), 2);
}

#[test]
fn campaign_strategy_request_merge_keeps_different_floors_separate() {
    let mut early = test_experiment_request(
        "route_policy_gap",
        "Map",
        "route planner declined automatic map selection",
    );
    early.act = 1;
    early.floor = 5;
    early.boundary_details = vec!["Map: current=(5, 4)".to_string()];
    let mut late = test_experiment_request(
        "route_policy_gap",
        "Map",
        "route planner declined automatic map selection",
    );
    late.act = 1;
    late.floor = 10;
    late.boundary_details = vec!["Map: current=(3, 9)".to_string()];

    let merged = merge_campaign_strategy_requests_v1(vec![early, late]);

    assert_eq!(merged.len(), 2);
    assert!(merged.iter().any(|request| {
        request.floor == 5
            && request
                .boundary_details
                .iter()
                .any(|detail| detail.contains("(5, 4)"))
            && !request
                .boundary_details
                .iter()
                .any(|detail| detail.contains("(3, 9)"))
    }));
    assert!(merged.iter().any(|request| {
        request.floor == 10
            && request
                .boundary_details
                .iter()
                .any(|detail| detail.contains("(3, 9)"))
            && !request
                .boundary_details
                .iter()
                .any(|detail| detail.contains("(5, 4)"))
    }));
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
        run_domain: BranchCampaignRunDomainV1::default(),
        rounds_completed: 2,
        stop_reason: "max_rounds".to_string(),
        active: vec![test_campaign_branch("a", 7, 80)],
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

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains("Next: budget ended; use .\\tools\\campaign.ps1 -More"));
}

#[test]
fn compact_campaign_report_labels_nonfatal_requests_as_deferred_notes() {
    let report = BranchCampaignReportV1 {
        schema_name: "BranchCampaignV1".to_string(),
        schema_version: 1,
        seed: 521,
        run_domain: BranchCampaignRunDomainV1::default(),
        rounds_completed: 2,
        stop_reason: "max_rounds".to_string(),
        active: vec![test_campaign_branch("a", 7, 80)],
        frozen: Vec::new(),
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: Vec::new(),
        stuck: Vec::new(),
        discarded_count: 0,
        discarded_examples: Vec::new(),
        strategy_requests: vec![BranchCampaignStrategyRequestV1 {
            kind: "route_policy_gap".to_string(),
            boundary_title: "Map".to_string(),
            branch_count: 2,
            act: 0,
            floor: 0,
            stop_reasons: vec!["route planner declined automatic map selection".to_string()],
            examples: vec!["Golden Idol -> Leave".to_string()],
            next_card_reward_offer: None,
            boundary_details: Vec::new(),
            suggested_action: "adjust route planner policy".to_string(),
        }],
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
        rounds: Vec::new(),
    };

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(!rendered.contains("Deferred strategy notes:"));
    assert!(!rendered.contains("Needs intervention:"));
    assert!(!rendered.contains("Queued interventions:"));
}

#[test]
fn compact_campaign_report_renders_context_only_strategy_packet() {
    let report = BranchCampaignReportV1 {
        schema_name: "BranchCampaignV1".to_string(),
        schema_version: 1,
        seed: 521,
        run_domain: BranchCampaignRunDomainV1::default(),
        rounds_completed: 4,
        stop_reason: "needs_intervention".to_string(),
        active: Vec::new(),
        frozen: Vec::new(),
        victories: Vec::new(),
        dead: Vec::new(),
        abandoned: Vec::new(),
        stuck: Vec::new(),
        discarded_count: 0,
        discarded_examples: Vec::new(),
        strategy_requests: vec![BranchCampaignStrategyRequestV1 {
            kind: "event_strategy".to_string(),
            boundary_title: "GoldenIdol".to_string(),
            branch_count: 2,
            act: 1,
            floor: 5,
            stop_reasons: vec!["event policy gap".to_string()],
            examples: vec!["Shockwave -> Clash".to_string()],
            next_card_reward_offer: Some(vec![
                "Pommel Strike".to_string(),
                "Iron Wave".to_string(),
                "Searing Blow".to_string(),
            ]),
            boundary_details: vec![
                "option: [Take] Obtain Golden Idol.".to_string(),
                "option: [Leave] Leave.".to_string(),
            ],
            suggested_action: "provide event policy".to_string(),
        }],
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
        combat_retry_ledger: BranchCampaignCombatRetryLedgerV1::default(),
        strategic_signals: Default::default(),
        state_store: BranchCampaignStateStoreSummaryV1::default(),
        rounds: Vec::new(),
    };

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains("context: A1F5"));
    assert!(rendered.contains("next reward offer: Pommel Strike | Iron Wave | Searing Blow"));
    assert!(rendered.contains("detail: option: [Take] Obtain Golden Idol."));
    assert!(!rendered.contains("pick Golden Idol"));
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

    let rendered = render_branch_campaign_compact_v1(&report, 1);

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
