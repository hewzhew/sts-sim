use super::*;

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
    assert_eq!(campaign_branch.strategic_summary.boss_readiness_milli, 600);
    assert_eq!(campaign_branch.strategic_summary.clean_score_milli, 800);
    assert_eq!(campaign_branch.strategic_summary.engine_score_milli, 1000);
    assert_eq!(campaign_branch.strategic_summary.cycle_debt_milli, 200);
    assert_eq!(campaign_branch.strategic_summary.setup_debt_milli, 400);
    assert_eq!(
        campaign_branch.strategic_summary.package_coherence_milli,
        700
    );
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
        candidate_axis: None,
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
        candidate_axis: None,
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
        candidate_axis: None,
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
        candidate_axis: None,
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
        candidate_axis: None,
        representative_count: 1,
        suppressed_count: 0,
        decision_signal: None,
        label: "upgrade Defend".to_string(),
        command: "select 3".to_string(),
    };

    assert_eq!(campaign_choice_label_v1(&choice), "upgrade Defend");
}
