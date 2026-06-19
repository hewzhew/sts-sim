use super::*;

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

    let rendered = render_branch_campaign_compact_with_detail_v1(
        &report,
        1,
        BranchCampaignReportDetailV1::Perf,
    );

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
        replay_exact_hits: 1,
        replay_ancestor_hits: 1,
        replay_misses: 1,
        replay_suffix_commands_sum: 4,
        replay_suffix_commands_max: 3,
        sessions_pruned: 7,
        anchor_sessions_kept: 2,
        inserts: 6,
        retains: 1,
    };

    let rendered = render_branch_campaign_compact_with_detail_v1(
        &report,
        1,
        BranchCampaignReportDetailV1::Perf,
    );

    assert!(rendered.contains(
        "State store: sessions=4 nodes=5 linked=3 replay=exact:1 ancestor:1 miss:1 suffix=sum:4 max:3 cache=pruned:7 anchors:2 lookups=2/1 inserts=6 retains=1"
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

    let rendered = render_branch_campaign_compact_with_detail_v1(
        &report,
        1,
        BranchCampaignReportDetailV1::Perf,
    );

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

    let rendered = render_branch_campaign_compact_with_detail_v1(
        &report,
        1,
        BranchCampaignReportDetailV1::Diagnose,
    );

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

    let rendered = render_branch_campaign_compact_with_detail_v1(
        &report,
        1,
        BranchCampaignReportDetailV1::Perf,
    );

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

    let rendered = render_branch_campaign_compact_with_detail_v1(
        &report,
        1,
        BranchCampaignReportDetailV1::Diagnose,
    );

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

    let rendered = render_branch_campaign_compact_with_detail_v1(
        &report,
        1,
        BranchCampaignReportDetailV1::Perf,
    );

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

    let rendered = render_branch_campaign_compact_with_detail_v1(
        &report,
        1,
        BranchCampaignReportDetailV1::Diagnose,
    );

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

    let rendered = render_branch_campaign_compact_with_detail_v1(
        &report,
        1,
        BranchCampaignReportDetailV1::Diagnose,
    );

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

    let rendered = render_branch_campaign_compact_with_detail_v1(
        &report,
        1,
        BranchCampaignReportDetailV1::Diagnose,
    );

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

    let rendered = render_branch_campaign_compact_with_detail_v1(
        &report,
        1,
        BranchCampaignReportDetailV1::Diagnose,
    );

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

    let rendered = render_branch_campaign_compact_with_detail_v1(
        &report,
        1,
        BranchCampaignReportDetailV1::Diagnose,
    );

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

    let rendered = render_branch_campaign_compact_with_detail_v1(
        &report,
        1,
        BranchCampaignReportDetailV1::Diagnose,
    );

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

    let rendered = render_branch_campaign_compact_with_detail_v1(
        &report,
        1,
        BranchCampaignReportDetailV1::Diagnose,
    );

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

    let rendered = render_branch_campaign_compact_with_detail_v1(
        &report,
        1,
        BranchCampaignReportDetailV1::Diagnose,
    );

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
    let rendered = render_branch_campaign_compact_with_detail_v1(
        &report,
        1,
        BranchCampaignReportDetailV1::Perf,
    );

    assert!(rendered.contains("strat=[boss:0.6 clean:0.8 eng:1.0 debt:0.2/0.4 pkg:0.7"));
    assert!(rendered.contains("keep=[engine]"));
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

    let rendered = render_branch_campaign_compact_with_detail_v1(
        &report,
        1,
        BranchCampaignReportDetailV1::Diagnose,
    );

    assert!(rendered.contains("sel=[retention_rank=123]"));
}

#[test]
fn compact_campaign_report_formats_large_selection_rank_readably() {
    let mut report = test_campaign_report_with_active("active", 3, 80);
    report.active[0].rank_key = 11_513;

    let rendered = render_branch_campaign_compact_with_detail_v1(
        &report,
        1,
        BranchCampaignReportDetailV1::Perf,
    );

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

    let rendered = render_branch_campaign_compact_with_detail_v1(
        &report,
        1,
        BranchCampaignReportDetailV1::Diagnose,
    );

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
                    bucket_mask: 0,
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
                    bucket_mask: 0,
                },
            },
        ],
    };

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains(
        "Strategic concern: frozen_engine_above_active=0.4 frozen_package_above_active=0.2"
    ));
}
