use super::*;
use crate::ai::noncombat_strategy_v1::{
    StrategyDeckFormationNeedV1, StrategyDeckFormationStageV1, StrategyPackageIdV2,
};

#[test]
fn portfolio_retention_keeps_package_branch_over_second_frontload_branch() {
    let candidates = vec![
        retention_candidate(0, 10_900, &["Twin Strike", "Perfected Strike", "Iron Wave"]),
        BranchRetentionCandidateInputV1 {
            index: 1,
            frontier_key: "same-frontier".to_string(),
            rank_key: 10_850,
            hp: 73,
            max_hp: 80,
            gold: 120,
            deck_count: 14,
            strategy_formation: None,
            trajectory: super::super::branch_experiment_trajectory::summarize_branch_trajectory_v1(
                &[
                    semantic_profile("Barricade", &[CardRewardSemanticRoleV1::BlockPayoff]),
                    semantic_profile("Entrench", &[CardRewardSemanticRoleV1::BlockPayoff]),
                    semantic_profile("Body Slam", &[CardRewardSemanticRoleV1::BlockPayoff]),
                ],
            ),
            choice_profiles: vec![
                semantic_profile("Barricade", &[CardRewardSemanticRoleV1::BlockPayoff]),
                semantic_profile("Entrench", &[CardRewardSemanticRoleV1::BlockPayoff]),
                semantic_profile("Body Slam", &[CardRewardSemanticRoleV1::BlockPayoff]),
            ],
            choice_effect_keys: vec!["take_card".to_string()],
            lineage_flags: Vec::new(),
        },
        retention_candidate(2, 10_840, &["Wild Strike", "Cleave", "Pommel Strike"]),
    ];

    let selection = select_branch_retention_portfolio_v1(&candidates, retention_config(2, Some(2)));

    assert_eq!(selection.keep_indices.len(), 2);
    assert!(selection.keep_indices.contains(&0));
    assert!(
        selection.keep_indices.contains(&1),
        "a package candidate should survive instead of keeping a second short-term frontload branch"
    );
    assert!(!selection.keep_indices.contains(&2));
    assert_eq!(
        selection.decisions_by_index[&1].primary_slot,
        BranchRetentionSlotV1::Package
    );
}

#[test]
fn portfolio_retention_prefers_distinct_choice_prefixes_when_slots_are_redundant() {
    let candidates = vec![
        retention_candidate(0, 10_900, &["Twin Strike", "Iron Wave"]),
        retention_candidate(1, 10_850, &["Twin Strike", "Uppercut"]),
        retention_candidate(2, 10_800, &["Clash", "Pommel Strike"]),
        retention_candidate(3, 10_750, &["Sever Soul", "Clothesline"]),
    ];

    let selection = select_branch_retention_portfolio_v1(&candidates, retention_config(3, Some(3)));

    assert_eq!(selection.keep_indices.len(), 3);
    assert!(selection.keep_indices.contains(&0));
    assert!(
        selection.keep_indices.contains(&2),
        "a different first-pick family should be kept before a second Twin Strike prefix"
    );
    assert!(
        selection.keep_indices.contains(&3),
        "portfolio fill should cover another distinct first-pick family"
    );
    assert!(!selection.keep_indices.contains(&1));
}

#[test]
fn portfolio_fill_continues_preferring_new_prefixes_after_slot_pass() {
    let candidates = vec![
        retention_candidate(0, 10_900, &["Twin Strike", "Iron Wave"]),
        retention_candidate(1, 10_850, &["Twin Strike", "Uppercut"]),
        retention_candidate(2, 10_800, &["Clash", "Pommel Strike"]),
        retention_candidate(3, 10_750, &["Sever Soul", "Clothesline"]),
        retention_candidate(4, 10_700, &["Shockwave", "Body Slam"]),
    ];

    let selection = select_branch_retention_portfolio_v1(&candidates, retention_config(4, Some(4)));

    assert_eq!(selection.keep_indices.len(), 4);
    assert!(selection.keep_indices.contains(&0));
    assert!(selection.keep_indices.contains(&2));
    assert!(selection.keep_indices.contains(&3));
    assert!(
        selection.keep_indices.contains(&4),
        "fill stage should keep a lower-ranked new first-pick family before a duplicate prefix"
    );
    assert!(!selection.keep_indices.contains(&1));
}

#[test]
fn portfolio_retention_does_not_fill_budget_with_redundant_first_pick_variants() {
    let candidates = vec![
        retention_candidate(0, 10_900, &["Twin Strike", "Iron Wave"]),
        retention_candidate(1, 10_890, &["Twin Strike", "Uppercut"]),
        retention_candidate(2, 10_880, &["Twin Strike", "Clothesline"]),
        retention_candidate(3, 10_870, &["Twin Strike", "Pommel Strike"]),
        retention_candidate(4, 10_860, &["Twin Strike", "Cleave"]),
        retention_candidate(5, 10_700, &["Shockwave", "Body Slam"]),
        retention_candidate(6, 10_650, &["Armaments", "Searing Blow"]),
    ];

    let selection = select_branch_retention_portfolio_v1(&candidates, retention_config(6, Some(6)));

    let twin_strike_kept = selection
        .keep_indices
        .iter()
        .filter(|index| candidates[**index].choice_profiles[0].name == "Twin Strike")
        .count();

    assert!(
        twin_strike_kept <= 2,
        "same first-pick variants should not fill most of an exploration budget"
    );
    assert!(selection.keep_indices.contains(&5));
    assert!(selection.keep_indices.contains(&6));
    assert!(
        selection.keep_indices.len() < 6,
        "max_total is an upper bound; redundant filler branches can be left unkept"
    );
}

#[test]
fn first_pick_prefix_cap_stays_tight_for_large_branch_budgets() {
    assert_eq!(first_pick_prefix_cap(16, 3), 4);
    assert_eq!(first_pick_prefix_cap(24, 4), 4);
}

#[test]
fn portfolio_retention_reapplies_first_pick_cap_after_effect_coverage() {
    let mut candidates = vec![
        retention_candidate(0, 10_900, &["Perfected Strike", "Heavy Blade"]),
        retention_candidate(1, 10_890, &["Perfected Strike", "Thunderclap"]),
        retention_candidate(2, 10_880, &["Perfected Strike", "Fear Potion"]),
        retention_candidate(3, 10_870, &["Perfected Strike", "Leave Shop"]),
        retention_candidate(4, 10_860, &["Perfected Strike", "Spot Weakness"]),
        retention_candidate(5, 10_300, &["Anger", "Heavy Blade"]),
        retention_candidate(6, 10_200, &["Rampage", "Leave Shop"]),
    ];
    for (candidate, effect_key) in candidates.iter_mut().zip([
        "take_card",
        "shop_buy_card",
        "shop_buy_potion",
        "shop_leave",
        "take_card",
        "take_card",
        "shop_leave",
    ]) {
        candidate.choice_effect_keys = vec![effect_key.to_string()];
    }

    let selection = select_branch_retention_portfolio_v1(&candidates, retention_config(6, Some(6)));
    let perfected_strike_kept = selection
        .keep_indices
        .iter()
        .filter(|index| candidates[**index].choice_profiles[0].name == "Perfected Strike")
        .count();

    assert!(
        perfected_strike_kept <= first_pick_prefix_cap(6, 3),
        "effect coverage should not let one first-pick prefix dominate the retained portfolio"
    );
    assert!(selection
        .keep_indices
        .iter()
        .any(|index| candidates[*index].choice_profiles[0].name == "Anger"));
    assert!(selection
        .keep_indices
        .iter()
        .any(|index| candidates[*index].choice_profiles[0].name == "Rampage"));
}

#[test]
fn portfolio_retention_keeps_distinct_reward_effect_kinds_when_budget_allows() {
    let candidates = vec![
        effect_retention_candidate(0, 10_900, "take_card"),
        effect_retention_candidate(1, 10_890, "take_card"),
        effect_retention_candidate(2, 10_880, "take_card"),
        effect_retention_candidate(3, 10_100, "skip_reward"),
    ];

    let selection = select_branch_retention_portfolio_v1(&candidates, retention_config(3, Some(3)));

    assert!(
        selection.keep_indices.contains(&3),
        "a skip/bowl-style effect branch should keep a representative when branch budget allows it"
    );
    assert_eq!(selection.keep_indices.len(), 3);
}

#[test]
fn portfolio_retention_treats_bottle_card_as_distinct_effect_kind() {
    let candidates = vec![
        effect_retention_candidate(0, 10_900, "take_card"),
        effect_retention_candidate(1, 10_890, "take_card"),
        effect_retention_candidate(2, 10_100, "bottle_card"),
    ];

    let selection = select_branch_retention_portfolio_v1(&candidates, retention_config(2, Some(2)));

    assert!(selection.keep_indices.contains(&2));
    assert_eq!(selection.keep_indices.len(), 2);
}

#[test]
fn portfolio_retention_treats_special_campfire_actions_as_distinct_effect_kinds() {
    let candidates = vec![
        effect_retention_candidate(0, 10_900, "upgrade_card"),
        effect_retention_candidate(1, 10_890, "upgrade_card"),
        effect_retention_candidate(2, 10_100, "dig"),
        effect_retention_candidate(3, 10_090, "lift"),
        effect_retention_candidate(4, 10_080, "recall"),
    ];

    let selection = select_branch_retention_portfolio_v1(&candidates, retention_config(4, Some(4)));

    assert!(selection.keep_indices.contains(&2));
    assert!(selection.keep_indices.contains(&3));
    assert!(selection.keep_indices.contains(&4));
    assert_eq!(selection.keep_indices.len(), 4);
}

#[test]
fn portfolio_retention_keeps_distinct_lineage_breakers_when_budget_allows() {
    let mut lineage_breaker = effect_retention_candidate(2, 10_100, "take_card");
    lineage_breaker.lineage_flags = vec!["question_card_reward_count_plus_1".to_string()];
    let candidates = vec![
        effect_retention_candidate(0, 10_900, "take_card"),
        effect_retention_candidate(1, 10_890, "take_card"),
        lineage_breaker,
    ];

    let selection = select_branch_retention_portfolio_v1(&candidates, retention_config(2, Some(2)));

    assert!(
        selection.keep_indices.contains(&2),
        "reward-sequence breaker branches should keep a representative when budget allows"
    );
    assert_eq!(selection.keep_indices.len(), 2);
}

#[test]
fn portfolio_retention_caps_dominant_first_pick_across_distinct_families() {
    fn sever_soul_candidate(
        index: usize,
        rank_key: i32,
        setup_keys: &[&str],
        package_keys: &[&str],
        engine_generator_picks: u8,
        engine_payoff_picks: u8,
        defense_picks: u8,
    ) -> BranchRetentionCandidateInputV1 {
        named_semantic_retention_candidate(
            index,
            rank_key,
            "Sever Soul",
            trajectory_with(
                setup_keys,
                package_keys,
                0,
                engine_generator_picks,
                engine_payoff_picks,
                defense_picks,
            ),
            &[CardRewardSemanticRoleV1::ExhaustGenerator],
        )
    }

    let candidates = vec![
        sever_soul_candidate(0, 10_900, &["exhaust_engine"], &[], 1, 0, 0),
        sever_soul_candidate(1, 10_890, &["status_package"], &[], 1, 0, 0),
        sever_soul_candidate(2, 10_880, &["exhaust_engine"], &["exhaust_engine"], 1, 1, 0),
        sever_soul_candidate(3, 10_870, &[], &["block_engine"], 0, 1, 1),
        sever_soul_candidate(4, 10_860, &[], &["upgrade_sink"], 0, 1, 0),
        sever_soul_candidate(5, 10_850, &["exhaust_engine"], &["upgrade_sink"], 1, 1, 0),
        named_semantic_retention_candidate(
            6,
            10_300,
            "Shockwave",
            trajectory_with(&[], &[], 0, 0, 0, 1),
            &[
                CardRewardSemanticRoleV1::Weak,
                CardRewardSemanticRoleV1::EnemyStrengthDown,
            ],
        ),
        named_semantic_retention_candidate(
            7,
            10_200,
            "Armaments",
            trajectory_with(&[], &["upgrade_sink"], 0, 0, 1, 1),
            &[
                CardRewardSemanticRoleV1::Block,
                CardRewardSemanticRoleV1::UpgradePayoff,
            ],
        ),
    ];

    let selection = select_branch_retention_portfolio_v1(&candidates, retention_config(6, Some(6)));

    let sever_soul_kept = selection
        .keep_indices
        .iter()
        .filter(|index| candidates[**index].choice_profiles[0].name == "Sever Soul")
        .count();

    assert!(
        sever_soul_kept <= 3,
        "one first-pick prefix should not dominate the exploration budget just because its later trajectory families differ"
    );
    assert!(selection.keep_indices.contains(&6));
    assert!(selection.keep_indices.contains(&7));
}

#[test]
fn portfolio_retention_preserves_distinct_formations_under_same_first_pick() {
    let mut starter = retention_candidate(0, 10_900, &["Twin Strike", "Iron Wave"]);
    starter.strategy_formation = Some(formation(
        StrategyDeckFormationStageV1::StarterShell,
        &[StrategyDeckFormationNeedV1::Frontload],
        &[],
    ));
    let mut duplicate_starter = retention_candidate(1, 10_890, &["Twin Strike", "Pommel Strike"]);
    duplicate_starter.strategy_formation = starter.strategy_formation.clone();
    let mut block_plan = retention_candidate(2, 10_760, &["Twin Strike", "Body Slam"]);
    block_plan.strategy_formation = Some(formation(
        StrategyDeckFormationStageV1::PlanSeeded,
        &[StrategyDeckFormationNeedV1::DrawEnergy],
        &[StrategyPackageIdV2::BlockEngine],
    ));
    let mut strength_plan = retention_candidate(3, 10_740, &["Twin Strike", "Heavy Blade"]);
    strength_plan.strategy_formation = Some(formation(
        StrategyDeckFormationStageV1::PlanSeeded,
        &[StrategyDeckFormationNeedV1::Block],
        &[StrategyPackageIdV2::StrengthScaling],
    ));
    let mut other_first_pick = retention_candidate(4, 10_700, &["Shockwave", "Cleave"]);
    other_first_pick.strategy_formation = Some(formation(
        StrategyDeckFormationStageV1::StarterShell,
        &[StrategyDeckFormationNeedV1::Frontload],
        &[],
    ));

    let selection = select_branch_retention_portfolio_v1(
        &[
            starter,
            duplicate_starter,
            block_plan,
            strength_plan,
            other_first_pick,
        ],
        retention_config(4, Some(4)),
    );

    assert!(selection.keep_indices.contains(&0));
    assert!(!selection.keep_indices.contains(&1));
    assert!(selection.keep_indices.contains(&2));
    assert!(selection.keep_indices.contains(&3));
    assert!(selection.keep_indices.contains(&4));
}

#[test]
fn portfolio_retention_preserves_distinct_trajectories_under_same_formation() {
    let formation = formation(
        StrategyDeckFormationStageV1::PlanSeeded,
        &[StrategyDeckFormationNeedV1::Scaling],
        &[],
    );
    let mut transition = retention_candidate(0, 10_900, &["Twin Strike", "Cleave"]);
    transition.strategy_formation = Some(formation.clone());
    transition.trajectory =
        super::super::branch_experiment_trajectory::summarize_branch_trajectory_v1(
            &transition.choice_profiles,
        );
    let mut duplicate_transition =
        retention_candidate(1, 10_890, &["Twin Strike", "Pommel Strike"]);
    duplicate_transition.strategy_formation = Some(formation.clone());
    duplicate_transition.trajectory = transition.trajectory.clone();
    let block_engine = BranchRetentionCandidateInputV1 {
        index: 2,
        frontier_key: "same-frontier".to_string(),
        rank_key: 10_760,
        hp: 70,
        max_hp: 80,
        gold: 120,
        deck_count: 14,
        strategy_formation: Some(formation.clone()),
        trajectory: super::super::branch_experiment_trajectory::summarize_branch_trajectory_v1(&[
            semantic_profile("Barricade", &[CardRewardSemanticRoleV1::BlockRetention]),
            semantic_profile("Body Slam", &[CardRewardSemanticRoleV1::BlockPayoff]),
        ]),
        choice_profiles: vec![
            semantic_profile("Barricade", &[CardRewardSemanticRoleV1::BlockRetention]),
            semantic_profile("Body Slam", &[CardRewardSemanticRoleV1::BlockPayoff]),
        ],
        choice_effect_keys: vec!["take_card".to_string()],
        lineage_flags: Vec::new(),
    };
    let mut other_first_pick = retention_candidate(3, 10_700, &["Shockwave", "Clash"]);
    other_first_pick.strategy_formation = Some(formation);

    let selection = select_branch_retention_portfolio_v1(
        &[
            transition,
            duplicate_transition,
            block_engine,
            other_first_pick,
        ],
        retention_config(3, Some(3)),
    );

    assert!(selection.keep_indices.contains(&0));
    assert!(!selection.keep_indices.contains(&1));
    assert!(selection.keep_indices.contains(&2));
    assert!(selection.keep_indices.contains(&3));
}

#[test]
fn portfolio_budget_keeps_setup_payoff_clean_and_survival_representatives() {
    let candidates = vec![
        semantic_retention_candidate(
            0,
            10_900,
            78,
            80,
            trajectory_with(&[], &[], 3, 0, 0, 0),
            &[CardRewardSemanticRoleV1::FrontloadDamage],
        ),
        semantic_retention_candidate(
            1,
            10_890,
            77,
            80,
            trajectory_with(&["exhaust_engine"], &[], 2, 1, 0, 0),
            &[CardRewardSemanticRoleV1::ExhaustGenerator],
        ),
        semantic_retention_candidate(
            2,
            10_880,
            76,
            80,
            trajectory_with(&["exhaust_engine"], &[], 2, 1, 0, 0),
            &[CardRewardSemanticRoleV1::ExhaustGenerator],
        ),
        semantic_retention_candidate(
            3,
            10_700,
            70,
            80,
            trajectory_with(&[], &["block_engine"], 0, 0, 1, 2),
            &[CardRewardSemanticRoleV1::BlockPayoff],
        ),
        semantic_retention_candidate(
            4,
            10_650,
            72,
            80,
            trajectory_with(&[], &[], 0, 0, 0, 1),
            &[CardRewardSemanticRoleV1::Block],
        ),
        semantic_retention_candidate(
            5,
            10_600,
            50,
            80,
            trajectory_with(&["status_package"], &[], 0, 1, 0, 0),
            &[CardRewardSemanticRoleV1::StatusGenerator],
        ),
        semantic_retention_candidate(
            6,
            10_550,
            74,
            80,
            trajectory_with(&[], &["strength_scaling"], 1, 0, 1, 0),
            &[CardRewardSemanticRoleV1::StrengthPayoff],
        ),
        semantic_retention_candidate(
            7,
            10_500,
            73,
            80,
            trajectory_with(&["block_engine"], &["block_engine"], 0, 1, 1, 2),
            &[
                CardRewardSemanticRoleV1::BlockRetention,
                CardRewardSemanticRoleV1::BlockPayoff,
            ],
        ),
    ];

    let selection = select_branch_retention_portfolio_v1(&candidates, retention_config(6, Some(6)));

    assert!(
        selection.keep_indices.contains(&0),
        "survival/frontload representative"
    );
    assert!(
        selection.keep_indices.contains(&1),
        "setup-only representative"
    );
    assert!(
        !selection.keep_indices.contains(&2),
        "duplicate setup family should not displace other buckets"
    );
    assert!(selection.keep_indices.contains(&3), "payoff representative");
    assert!(
        selection.keep_indices.contains(&4),
        "clean/defense representative"
    );
    assert!(
        selection.keep_indices.contains(&6),
        "second payoff family representative"
    );
    assert!(
        selection.keep_indices.contains(&7),
        "setup+payoff engine representative"
    );
}

#[test]
fn portfolio_budget_does_not_saturate_with_pure_transition_branches() {
    let candidates = vec![
        retention_candidate(0, 10_900, &["Twin Strike", "Clash"]),
        retention_candidate(1, 10_890, &["Wild Strike", "Cleave"]),
        retention_candidate(2, 10_880, &["Pommel Strike", "Sword Boomerang"]),
        semantic_retention_candidate(
            3,
            10_500,
            50,
            80,
            trajectory_with(&["exhaust_engine"], &[], 0, 1, 0, 0),
            &[CardRewardSemanticRoleV1::ExhaustGenerator],
        ),
        semantic_retention_candidate(
            4,
            10_450,
            50,
            80,
            trajectory_with(&["status_package"], &[], 0, 1, 0, 0),
            &[CardRewardSemanticRoleV1::StatusGenerator],
        ),
    ];

    let selection = select_branch_retention_portfolio_v1(&candidates, retention_config(3, Some(3)));

    let pure_transition_kept = selection
        .keep_indices
        .iter()
        .filter(|index| **index <= 2)
        .count();

    assert_eq!(
        pure_transition_kept, 1,
        "pure transition output should have a representative, but not saturate the budget"
    );
    assert!(
        selection.keep_indices.contains(&3),
        "an exhaust setup branch should survive short-horizon transition pressure"
    );
    assert!(
        selection.keep_indices.contains(&4),
        "a status setup branch should survive short-horizon transition pressure"
    );
}

#[test]
fn package_slot_prefers_setup_and_payoff_closure_over_payoff_only() {
    let payoff_only = semantic_retention_candidate(
        0,
        10_900,
        78,
        80,
        trajectory_with(&[], &["block_engine"], 0, 0, 1, 1),
        &[CardRewardSemanticRoleV1::BlockPayoff],
    );
    let setup_and_payoff = semantic_retention_candidate(
        1,
        10_500,
        72,
        80,
        trajectory_with(&["block_engine"], &["block_engine"], 0, 1, 1, 2),
        &[
            CardRewardSemanticRoleV1::BlockRetention,
            CardRewardSemanticRoleV1::BlockPayoff,
        ],
    );

    let selection = select_branch_retention_portfolio_v1(
        &[payoff_only, setup_and_payoff],
        retention_config(1, Some(1)),
    );

    assert!(
        selection.keep_indices.contains(&1),
        "a branch with both setup and payoff should be the package representative"
    );
}

#[test]
fn portfolio_records_the_lane_that_selected_each_kept_branch() {
    let package_closure = semantic_retention_candidate(
        0,
        10_300,
        58,
        80,
        trajectory_with(&["block_engine"], &["block_engine"], 0, 1, 1, 2),
        &[
            CardRewardSemanticRoleV1::BlockRetention,
            CardRewardSemanticRoleV1::BlockPayoff,
        ],
    );
    let high_hp_payoff = semantic_retention_candidate(
        1,
        10_900,
        79,
        80,
        trajectory_with(&[], &["generic_package"], 1, 0, 1, 0),
        &[CardRewardSemanticRoleV1::PackagePayoff],
    );

    let selection = select_branch_retention_portfolio_v1(
        &[package_closure, high_hp_payoff],
        BranchRetentionConfigV1 {
            max_total: 2,
            max_per_frontier: Some(2),
            budget_profile: BranchRetentionBudgetProfileV1::Survival,
        },
    );

    assert_eq!(
        selection.decisions_by_index[&0].selected_by_slot,
        Some(BranchRetentionSlotV1::DefenseEngine)
    );
    assert_eq!(
        selection.decisions_by_index[&0].primary_slot,
        BranchRetentionSlotV1::Package,
        "a branch can be semantically package-shaped while being retained by the survival profile's defense lane"
    );
    assert_eq!(
        selection.decisions_by_index[&1].primary_slot,
        BranchRetentionSlotV1::Package,
        "candidate identity still records the highest semantic slot it qualifies for"
    );
    assert_eq!(
        selection.decisions_by_index[&1].selected_by_slot,
        Some(BranchRetentionSlotV1::Survival),
        "portfolio reporting should say this branch consumed the survival lane, not another package lane"
    );
}

#[test]
fn survival_profile_prioritizes_survival_defense_and_frontload_lanes() {
    let candidates = vec![
        semantic_retention_candidate(
            0,
            10_900,
            50,
            80,
            trajectory_with(&["exhaust_engine"], &["exhaust_engine"], 0, 1, 1, 0),
            &[
                CardRewardSemanticRoleV1::ExhaustGenerator,
                CardRewardSemanticRoleV1::ExhaustPayoff,
            ],
        ),
        semantic_retention_candidate(
            1,
            10_850,
            50,
            80,
            trajectory_with(&["status_package"], &[], 0, 1, 0, 0),
            &[CardRewardSemanticRoleV1::StatusGenerator],
        ),
        semantic_retention_candidate(
            2,
            10_800,
            50,
            80,
            trajectory_with(&[], &[], 0, 0, 0, 2),
            &[CardRewardSemanticRoleV1::Weak],
        ),
        semantic_retention_candidate(
            3,
            10_700,
            79,
            80,
            trajectory_with(&[], &[], 0, 0, 0, 0),
            &[],
        ),
        semantic_retention_candidate(
            4,
            10_600,
            50,
            80,
            trajectory_with(&[], &[], 2, 0, 0, 0),
            &[CardRewardSemanticRoleV1::FrontloadDamage],
        ),
    ];

    let selection = select_branch_retention_portfolio_v1(
        &candidates,
        BranchRetentionConfigV1 {
            max_total: 3,
            max_per_frontier: Some(3),
            budget_profile: BranchRetentionBudgetProfileV1::Survival,
        },
    );
    let lanes = selected_lanes(&selection);

    assert_eq!(
        lanes,
        vec![
            BranchRetentionSlotV1::DefenseEngine,
            BranchRetentionSlotV1::Survival,
            BranchRetentionSlotV1::Frontload,
        ],
        "survival profile should spend its small budget on immediate safety lanes before long-horizon setup"
    );
}

#[test]
fn package_profile_prioritizes_package_engine_and_scaling_lanes() {
    let candidates = vec![
        semantic_retention_candidate(
            0,
            10_900,
            79,
            80,
            trajectory_with(&[], &[], 0, 0, 0, 0),
            &[],
        ),
        semantic_retention_candidate(
            1,
            10_800,
            60,
            80,
            trajectory_with(&[], &[], 2, 0, 0, 0),
            &[CardRewardSemanticRoleV1::FrontloadDamage],
        ),
        semantic_retention_candidate(
            2,
            10_700,
            60,
            80,
            trajectory_with(&["block_engine"], &["block_engine"], 0, 1, 1, 2),
            &[
                CardRewardSemanticRoleV1::BlockRetention,
                CardRewardSemanticRoleV1::BlockPayoff,
            ],
        ),
        semantic_retention_candidate(
            3,
            10_600,
            60,
            80,
            trajectory_with(&["exhaust_engine"], &[], 0, 1, 0, 0),
            &[CardRewardSemanticRoleV1::ExhaustGenerator],
        ),
        semantic_retention_candidate(
            4,
            10_500,
            60,
            80,
            trajectory_with(&[], &["strength_scaling"], 0, 0, 1, 0),
            &[CardRewardSemanticRoleV1::StrengthPayoff],
        ),
    ];

    let selection = select_branch_retention_portfolio_v1(
        &candidates,
        BranchRetentionConfigV1 {
            max_total: 3,
            max_per_frontier: Some(3),
            budget_profile: BranchRetentionBudgetProfileV1::Package,
        },
    );
    let lanes = selected_lanes(&selection);

    assert_eq!(
        lanes,
        vec![
            BranchRetentionSlotV1::Package,
            BranchRetentionSlotV1::EngineSetup,
            BranchRetentionSlotV1::Scaling,
        ],
        "package profile should preserve long-horizon package structure before short-term safety fillers"
    );
}

#[test]
fn balanced_profile_reserves_more_budget_for_long_horizon_than_filler() {
    let lanes = retention_lane_sequence(BranchRetentionBudgetProfileV1::Balanced, 20);
    let counts = lane_counts(&lanes);

    assert_eq!(counts[&BranchRetentionSlotV1::Package], 4);
    assert_eq!(counts[&BranchRetentionSlotV1::EngineSetup], 3);
    assert_eq!(counts[&BranchRetentionSlotV1::Diversity], 2);
}

#[test]
fn setup_only_branch_gets_engine_setup_retention_slot() {
    let setup_only = semantic_retention_candidate(
        0,
        10_500,
        70,
        80,
        trajectory_with(&["exhaust_engine"], &[], 0, 1, 0, 0),
        &[CardRewardSemanticRoleV1::ExhaustGenerator],
    );

    let decision = decide_branch_retention_v1(&setup_only);

    assert_eq!(decision.primary_slot, BranchRetentionSlotV1::EngineSetup);
    assert!(
        decision.slots.contains(&BranchRetentionSlotV1::EngineSetup),
        "setup-only engine branches should be preserved by an explicit long-horizon slot"
    );
}

#[test]
fn retention_slots_come_from_semantic_profiles_not_card_names() {
    let candidate = BranchRetentionCandidateInputV1 {
        index: 0,
        frontier_key: "same-frontier".to_string(),
        rank_key: 10_000,
        hp: 70,
        max_hp: 80,
        gold: 120,
        deck_count: 12,
        strategy_formation: None,
        trajectory: super::super::branch_experiment_trajectory::summarize_branch_trajectory_v1(&[
            semantic_profile(
                "Unfamiliar Card Name",
                &[CardRewardSemanticRoleV1::BlockPayoff],
            ),
        ]),
        choice_profiles: vec![semantic_profile(
            "Unfamiliar Card Name",
            &[CardRewardSemanticRoleV1::BlockPayoff],
        )],
        choice_effect_keys: vec!["take_card".to_string()],
        lineage_flags: Vec::new(),
    };

    let decision = decide_branch_retention_v1(&candidate);

    assert!(decision.slots.contains(&BranchRetentionSlotV1::Package));
    assert!(decision
        .slots
        .contains(&BranchRetentionSlotV1::DefenseEngine));
    assert!(!decision.slots.contains(&BranchRetentionSlotV1::Frontload));
}

#[test]
fn context_packet_matches_current_formation_needs_without_card_names() {
    let mut candidate = semantic_retention_candidate(
        0,
        10_000,
        64,
        80,
        trajectory_with(&[], &[], 0, 0, 0, 1),
        &[
            CardRewardSemanticRoleV1::Block,
            CardRewardSemanticRoleV1::CardDraw,
        ],
    );
    candidate.choice_profiles[0].name = "Unfamiliar Utility".to_string();
    candidate.strategy_formation = Some(formation(
        StrategyDeckFormationStageV1::Transitional,
        &[
            StrategyDeckFormationNeedV1::Block,
            StrategyDeckFormationNeedV1::DrawEnergy,
        ],
        &[],
    ));

    let packet = branch_retention_context_packet_v2(&candidate);

    assert!(packet
        .keys
        .contains(&BranchRetentionContextKeyV2::MatchesFormationBlockNeed));
    assert!(packet
        .keys
        .contains(&BranchRetentionContextKeyV2::MatchesFormationDrawEnergyNeed));
    assert!(
        !packet
            .keys
            .contains(&BranchRetentionContextKeyV2::MatchesFormationFrontloadNeed),
        "context packet should be driven by current needs and semantic roles, not by a card name fallback"
    );
}

#[test]
fn frontload_slot_prefers_candidate_that_matches_current_frontload_need() {
    let mut generic_output = semantic_retention_candidate(
        0,
        10_900,
        70,
        80,
        trajectory_with(&[], &[], 1, 0, 0, 0),
        &[CardRewardSemanticRoleV1::FrontloadDamage],
    );
    generic_output.strategy_formation = Some(formation(
        StrategyDeckFormationStageV1::PlanSeeded,
        &[StrategyDeckFormationNeedV1::Scaling],
        &[],
    ));

    let mut contextual_output = semantic_retention_candidate(
        1,
        10_100,
        70,
        80,
        trajectory_with(&[], &[], 1, 0, 0, 0),
        &[CardRewardSemanticRoleV1::FrontloadDamage],
    );
    contextual_output.strategy_formation = Some(formation(
        StrategyDeckFormationStageV1::StarterShell,
        &[StrategyDeckFormationNeedV1::Frontload],
        &[],
    ));

    assert!(
        slot_score(&contextual_output, BranchRetentionSlotV1::Frontload)
            > slot_score(&generic_output, BranchRetentionSlotV1::Frontload),
        "frontload lane should prefer output that patches a current frontload gap over a higher-ranked generic output"
    );
}

fn retention_candidate(
    index: usize,
    rank_key: i32,
    choice_labels: &[&str],
) -> BranchRetentionCandidateInputV1 {
    let choice_profiles = choice_labels
        .iter()
        .map(|label| semantic_profile(label, &[CardRewardSemanticRoleV1::FrontloadDamage]))
        .collect::<Vec<_>>();
    let trajectory = super::super::branch_experiment_trajectory::summarize_branch_trajectory_v1(
        &choice_profiles,
    );
    BranchRetentionCandidateInputV1 {
        index,
        frontier_key: "same-frontier".to_string(),
        rank_key,
        hp: 78,
        max_hp: 80,
        gold: 120,
        deck_count: 14,
        strategy_formation: None,
        trajectory,
        choice_profiles,
        choice_effect_keys: vec!["take_card".to_string()],
        lineage_flags: Vec::new(),
    }
}

fn effect_retention_candidate(
    index: usize,
    rank_key: i32,
    effect_key: &str,
) -> BranchRetentionCandidateInputV1 {
    BranchRetentionCandidateInputV1 {
        index,
        frontier_key: "same-frontier".to_string(),
        rank_key,
        hp: 78,
        max_hp: 80,
        gold: 120,
        deck_count: 14,
        strategy_formation: None,
        trajectory: BranchTrajectorySignatureV1::default(),
        choice_profiles: Vec::new(),
        choice_effect_keys: vec![effect_key.to_string()],
        lineage_flags: Vec::new(),
    }
}

fn semantic_profile(name: &str, roles: &[CardRewardSemanticRoleV1]) -> CardRewardSemanticProfileV1 {
    CardRewardSemanticProfileV1 {
        card: crate::content::cards::CardId::Strike,
        name: name.to_string(),
        roles: roles.to_vec(),
        dependencies: Vec::new(),
        unsupported_mechanics: Vec::new(),
    }
}

fn semantic_retention_candidate(
    index: usize,
    rank_key: i32,
    hp: i32,
    max_hp: i32,
    trajectory: BranchTrajectorySignatureV1,
    roles: &[CardRewardSemanticRoleV1],
) -> BranchRetentionCandidateInputV1 {
    BranchRetentionCandidateInputV1 {
        index,
        frontier_key: "same-frontier".to_string(),
        rank_key,
        hp,
        max_hp,
        gold: 120,
        deck_count: 14,
        strategy_formation: Some(formation(
            StrategyDeckFormationStageV1::PlanSeeded,
            &[StrategyDeckFormationNeedV1::Scaling],
            &[],
        )),
        trajectory,
        choice_profiles: vec![semantic_profile("Semantic Candidate", roles)],
        choice_effect_keys: vec!["take_card".to_string()],
        lineage_flags: Vec::new(),
    }
}

fn named_semantic_retention_candidate(
    index: usize,
    rank_key: i32,
    first_pick: &str,
    trajectory: BranchTrajectorySignatureV1,
    roles: &[CardRewardSemanticRoleV1],
) -> BranchRetentionCandidateInputV1 {
    let mut candidate = semantic_retention_candidate(index, rank_key, 78, 80, trajectory, roles);
    candidate.choice_profiles[0].name = first_pick.to_string();
    candidate
}

fn trajectory_with(
    setup_keys: &[&str],
    package_keys: &[&str],
    transition_frontload_picks: u8,
    engine_generator_picks: u8,
    engine_payoff_picks: u8,
    defense_picks: u8,
) -> BranchTrajectorySignatureV1 {
    BranchTrajectorySignatureV1 {
        frontload_picks: transition_frontload_picks,
        transition_frontload_picks,
        scaling_picks: engine_payoff_picks,
        defense_picks,
        engine_generator_picks,
        engine_payoff_picks,
        draw_energy_picks: 0,
        setup_keys: setup_keys.iter().map(|key| key.to_string()).collect(),
        package_keys: package_keys.iter().map(|key| key.to_string()).collect(),
    }
}

fn formation(
    stage: StrategyDeckFormationStageV1,
    needs: &[StrategyDeckFormationNeedV1],
    strengths: &[StrategyPackageIdV2],
) -> StrategyFormationSummaryV2 {
    StrategyFormationSummaryV2 {
        stage,
        needs: needs.to_vec(),
        strengths: strengths.to_vec(),
    }
}

fn selected_lanes(selection: &BranchRetentionSelectionV1) -> Vec<BranchRetentionSlotV1> {
    selection
        .decisions_by_index
        .values()
        .filter_map(|decision| decision.selected_by_slot)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn lane_counts(lanes: &[BranchRetentionSlotV1]) -> BTreeMap<BranchRetentionSlotV1, usize> {
    let mut counts = BTreeMap::new();
    for lane in lanes {
        *counts.entry(*lane).or_default() += 1;
    }
    counts
}

fn retention_config(max_total: usize, max_per_frontier: Option<usize>) -> BranchRetentionConfigV1 {
    BranchRetentionConfigV1 {
        max_total,
        max_per_frontier,
        budget_profile: BranchRetentionBudgetProfileV1::Balanced,
    }
}
