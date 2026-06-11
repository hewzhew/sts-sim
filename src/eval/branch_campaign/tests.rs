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
use crate::state::events::{EventId, EventState};
use crate::state::rewards::{RewardCard, RewardState};
use std::collections::BTreeMap;

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

    assert!(rendered.contains("choices: Warcry -> Body Slam -> ... -> Whirlwind"));
    assert!(!rendered.contains(
        "choices: Warcry -> Body Slam -> Shrug It Off -> Sword Boomerang -> PandorasBox -> Whirlwind"
    ));
    assert_eq!(report.active[0].choice_labels.len(), 6);
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

    assert!(rendered.contains("Resource concern: high_unspent_gold_near_boss=1 max_gold=485"));
    assert!(rendered.contains(
        "resource example: A1F16 gold 485 | Flame Barrier -> Wild Strike -> ... -> Smith Shockwave"
    ));
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
fn campaign_selection_prefers_converted_gold_when_progress_is_tied() {
    let mut rich = test_campaign_branch("a-rich", 16, 30);
    rich.rank_key = 100;
    rich.summary.as_mut().unwrap().gold = 485;
    let mut converted = test_campaign_branch("b-converted", 16, 30);
    converted.rank_key = 100;
    converted.summary.as_mut().unwrap().gold = 120;

    let selected = select_campaign_branches_v1(vec![rich, converted], 1, 4);

    assert_eq!(selected.active.len(), 1);
    assert_eq!(selected.active[0].branch_id, "b-converted");
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
        &config, &selection
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
        &config, &selection
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
        &config, &selection
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
        stop_reason: "test".to_string(),
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
        rounds: Vec::new(),
    };

    let rendered = render_branch_campaign_compact_v1(&report, 2);

    assert!(rendered.contains("BranchCampaignV1 seed=521 rounds=2 stop=needs_intervention"));
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
    assert!(rendered.contains("possible inputs: switch macro branch | provide combat tactic | add upstream route/reward rule | raise retry budget only if under-spent"));
}

#[test]
fn compact_campaign_report_renders_deferred_strategy_notes_while_continuing() {
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
        rounds: Vec::new(),
    };

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains("Deferred strategy notes:"));
    assert!(!rendered.contains("Needs intervention:"));
    assert!(rendered.contains("stop: combat search did not find an executable complete win"));
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
    let mut snapshot_cache = BTreeMap::new();
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
    snapshot_cache.insert(stuck[0].commands.clone(), session);

    let recovered = recover_auto_advanceable_stuck_branches_v1(
        &mut active,
        &mut frozen,
        &mut stuck,
        &mut snapshot_cache,
        1,
        0,
    );

    assert_eq!(recovered, 1);
    assert_eq!(stuck.len(), 0);
    assert_eq!(active.len(), 1);
    assert_eq!(active[0].frontier_title, "Map");
    assert!(matches!(
        snapshot_cache
            .get(&active[0].commands)
            .expect("recovered snapshot should be retained")
            .engine_state,
        EngineState::MapNavigation
    ));
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
        rounds: Vec::new(),
    };

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains("Deferred strategy notes:"));
    assert!(!rendered.contains("Needs intervention:"));
    assert!(!rendered.contains("Queued interventions:"));
}

#[test]
fn compact_campaign_report_renders_context_only_strategy_packet() {
    let report = BranchCampaignReportV1 {
        schema_name: "BranchCampaignV1".to_string(),
        schema_version: 1,
        seed: 521,
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
            discarded_examples: Vec::new(),
            strategy_requests: Vec::new(),
            route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
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
        rounds: Vec::new(),
        snapshot_cache: BTreeMap::from([
            (abandoned.commands.clone(), abandoned_session),
            (stuck.commands.clone(), stuck_session),
        ]),
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
        rounds: Vec::new(),
    };
    let checkpoint = BranchCampaignCheckpointV1 {
        schema_name: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_NAME.to_string(),
        schema_version: BRANCH_CAMPAIGN_CHECKPOINT_SCHEMA_VERSION,
        seed: 1,
        rounds_completed: 0,
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
    assert!(
        branch_labels
            .iter()
            .any(|label| label.contains("Twin Strike") || label.contains("Cleave")),
        "at least one branch should come from the restored checkpoint snapshot"
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
        discarded_examples: Vec::new(),
        strategy_requests: Vec::new(),
        route_evidence: BranchCampaignRouteEvidenceSummaryV1::default(),
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
        stop_reason: "test".to_string(),
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
