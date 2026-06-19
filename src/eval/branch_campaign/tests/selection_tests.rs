use super::*;

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
        bucket_mask: 0,
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
        bucket_mask: 0,
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
        bucket_mask: 0,
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
        bucket_mask: 0,
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
        bucket_mask: 0,
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
        bucket_mask: 0,
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
                bucket_mask: 0,
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
