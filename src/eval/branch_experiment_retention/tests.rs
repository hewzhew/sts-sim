use super::*;
use crate::ai::noncombat_strategy_v1::{
    StrategyDeckFormationNeedV1, StrategyDeckFormationStageV1, StrategyPackageIdV2,
};
use crate::ai::strategic::RetentionBucket;
use crate::content::cards::CardId;

#[test]
fn portfolio_retention_keeps_distinct_choice_prefixes_when_slots_are_redundant() {
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
fn resource_conversion_signal_ignores_full_potion_skip() {
    let candidate = effect_retention_candidate(0, 10_000, "reward_skip_full_potion");
    let selection =
        select_branch_retention_portfolio_v1(&[candidate], retention_config(1, Some(1)));
    let decision = selection
        .decisions_by_index
        .get(&0)
        .expect("candidate should have a retention decision");

    assert_eq!(decision.strategic_signature.economy_conversion, 0.0);
    assert!(
        !decision
            .strategic_signature
            .buckets
            .contains(&RetentionBucket::BestResourceConverted),
        "skipping a potion because slots are full is not resource conversion"
    );
}

#[test]
fn resource_conversion_signal_keeps_shop_purchase() {
    let candidate = effect_retention_candidate(0, 10_000, "shop_buy_combo");
    let selection =
        select_branch_retention_portfolio_v1(&[candidate], retention_config(1, Some(1)));
    let decision = selection
        .decisions_by_index
        .get(&0)
        .expect("candidate should have a retention decision");

    assert_eq!(decision.strategic_signature.economy_conversion, 1.0);
    assert!(decision
        .strategic_signature
        .buckets
        .contains(&RetentionBucket::BestResourceConverted));
}

#[test]
fn branch_signature_counts_deck_shape_startup_debt() {
    let mut candidate = retention_candidate(0, 10_000, &["Corruption"]);
    candidate.startup.has_corruption_duplicate_without_payoff = true;
    candidate.startup.has_havoc_duplicate_without_payoff = true;
    candidate
        .startup
        .has_status_generator_saturation_without_digest = true;
    candidate.startup.has_clash_playability_debt = true;

    let decision = decide_branch_retention_v1(&candidate);

    assert!(
        decision.strategic_signature.setup_debt >= 0.75,
        "deck-shape liabilities should be visible to branch retention, not hidden in admission only"
    );
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
fn portfolio_retention_does_not_preserve_curse_debt_only_for_effect_coverage() {
    let candidates = vec![
        effect_retention_candidate(0, 10_900, "take_card"),
        effect_retention_candidate(1, 10_890, "shop_buy_relic"),
        effect_retention_candidate(2, 10_100, "event_gain_curse"),
    ];

    let selection = select_branch_retention_portfolio_v1(&candidates, retention_config(2, Some(2)));

    assert!(selection.keep_indices.contains(&0));
    assert!(selection.keep_indices.contains(&1));
    assert!(
        !selection.keep_indices.contains(&2),
        "hazardous curse-debt events should not be reintroduced solely as effect coverage"
    );
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
fn bloated_deck_transition_pick_still_claims_frontload_shape_slot() {
    let mut candidate = retention_candidate(0, 10_900, &["Pommel Strike"]);
    candidate.deck_count = 42;

    let decision = decide_branch_retention_v1(&candidate);

    assert!(
        !decision.slots.contains(&BranchRetentionSlotV1::CleanDeck),
        "taking another pure transition card in a bloated deck is not a clean-deck branch"
    );
    assert!(
        decision.slots.contains(&BranchRetentionSlotV1::Frontload),
        "deck bloat is debt evidence, not a reason to hide the candidate's frontload shape"
    );
    assert!(
        decision.strategic_signature.cycle_debt > 0.0,
        "deck bloat should remain visible as diagnostic debt"
    );
}

#[test]
fn bloated_deck_skip_can_claim_clean_deck_slot() {
    let mut candidate = effect_retention_candidate(0, 10_100, "skip_reward");
    candidate.deck_count = 42;

    let decision = decide_branch_retention_v1(&candidate);

    assert!(
        decision.slots.contains(&BranchRetentionSlotV1::CleanDeck),
        "declining a card reward is the clean-deck branch once deck size is already high"
    );
    assert!(
        branch_retention_slot_evidence_score_v1(&candidate, BranchRetentionSlotV1::CleanDeck) > 0,
        "clean-deck scoring should be able to select the skip branch under deck bloat"
    );
}

#[test]
fn late_retention_preserves_clean_branch_under_tight_budget() {
    let mut frontload = retention_candidate(0, 10_900, &["Twin Strike"]);
    frontload.act = 2;
    frontload.floor = 28;
    frontload.deck_count = 36;

    let mut defense = retention_candidate(1, 10_890, &["Flame Barrier"]);
    defense.act = 2;
    defense.floor = 28;
    defense.deck_count = 36;

    let mut skip = effect_retention_candidate(2, 10_100, "skip_reward");
    skip.act = 2;
    skip.floor = 28;
    skip.deck_count = 36;

    let candidates = vec![frontload, defense, skip];
    let selection = select_branch_retention_portfolio_v1(&candidates, retention_config(2, Some(2)));

    assert!(
        selection.keep_indices.contains(&2),
        "late branch retention should preserve a clean skip branch instead of letting immediate-power branches fill every slot"
    );
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
fn retention_decision_carries_strategic_signature_for_engine_branches() {
    let candidate = named_semantic_retention_candidate(
        0,
        10_900,
        "Barricade",
        trajectory_with(&["block_engine"], &["block_engine"], 0, 1, 1, 1),
        &[
            CardRewardSemanticRoleV1::Block,
            CardRewardSemanticRoleV1::ScalingSource,
        ],
    );

    let decision = decide_branch_retention_v1(&candidate);

    assert!(decision
        .strategic_signature
        .buckets
        .contains(&RetentionBucket::BestCoreEngine));
    assert!(decision
        .strategic_signature
        .buckets
        .contains(&RetentionBucket::BestBossPrepared));
    assert!(decision.strategic_signature.engine_score > 0.0);
    assert!(decision.strategic_signature.boss_readiness > 0.0);
}

#[test]
fn retention_decision_carries_strategic_debt_for_startup_rejected_branches() {
    let mut candidate = named_semantic_retention_candidate(
        0,
        10_900,
        "Rupture",
        BranchTrajectorySignatureV1::default(),
        &[CardRewardSemanticRoleV1::ScalingSource],
    );
    candidate.choice_profiles[0].card = CardId::Rupture;
    candidate.startup.rupture_count = 1;
    candidate.startup.self_damage_source_count = 0;
    candidate.startup.has_rupture_without_self_damage = true;

    let decision = decide_branch_retention_v1(&candidate);

    assert!(decision.strategic_signature.setup_debt > 0.0);
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
        act: 1,
        floor: 1,
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
        strategic_debt_tags: Vec::new(),
        startup: Default::default(),
        card_admission_context: None,
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
fn portfolio_retention_caps_payoff_only_package_saturation() {
    let candidates = vec![
        semantic_retention_candidate(
            0,
            10_900,
            78,
            80,
            trajectory_with(&[], &["strike_density"], 0, 0, 1, 0),
            &[CardRewardSemanticRoleV1::StrikePayoff],
        ),
        semantic_retention_candidate(
            1,
            10_890,
            78,
            80,
            trajectory_with(&[], &["strike_density"], 0, 0, 1, 0),
            &[CardRewardSemanticRoleV1::StrikePayoff],
        ),
        semantic_retention_candidate(
            2,
            10_880,
            78,
            80,
            trajectory_with(&[], &["strike_density"], 0, 0, 1, 0),
            &[CardRewardSemanticRoleV1::StrikePayoff],
        ),
        semantic_retention_candidate(
            3,
            10_870,
            78,
            80,
            trajectory_with(&[], &["strike_density"], 0, 0, 1, 0),
            &[CardRewardSemanticRoleV1::StrikePayoff],
        ),
        semantic_retention_candidate(
            4,
            10_500,
            72,
            80,
            trajectory_with(&["exhaust_engine"], &[], 0, 1, 0, 0),
            &[CardRewardSemanticRoleV1::ExhaustGenerator],
        ),
        semantic_retention_candidate(
            5,
            10_450,
            72,
            80,
            trajectory_with(&["status_package"], &[], 0, 1, 0, 0),
            &[CardRewardSemanticRoleV1::StatusGenerator],
        ),
    ];

    let selection = select_branch_retention_portfolio_v1(&candidates, retention_config(5, Some(5)));
    let payoff_only_kept = selection
        .keep_indices
        .iter()
        .filter(|index| is_payoff_only_package_branch(&candidates[**index]))
        .count();

    assert!(
        payoff_only_kept <= payoff_only_package_branch_cap(&candidates, &[0, 1, 2, 3, 4, 5], 5),
        "payoff-only package branches should keep representatives without saturating the portfolio"
    );
    assert!(selection.keep_indices.contains(&4));
    assert!(selection.keep_indices.contains(&5));
}

#[test]
fn portfolio_retention_limits_payoff_only_branches_without_committed_package() {
    let candidates = vec![
        semantic_retention_candidate(
            0,
            10_950,
            78,
            80,
            trajectory_with(&[], &["strength_scaling"], 0, 0, 1, 0),
            &[CardRewardSemanticRoleV1::StrengthPayoff],
        ),
        semantic_retention_candidate(
            1,
            10_940,
            78,
            80,
            trajectory_with(&[], &["block_engine"], 0, 0, 1, 0),
            &[CardRewardSemanticRoleV1::BlockPayoff],
        ),
        semantic_retention_candidate(
            2,
            10_930,
            78,
            80,
            trajectory_with(&[], &["upgrade_sink"], 0, 0, 1, 0),
            &[CardRewardSemanticRoleV1::UpgradePayoff],
        ),
        semantic_retention_candidate(
            3,
            10_650,
            72,
            80,
            trajectory_with(&["exhaust_engine"], &[], 0, 1, 0, 0),
            &[CardRewardSemanticRoleV1::ExhaustGenerator],
        ),
        semantic_retention_candidate(
            4,
            10_620,
            72,
            80,
            trajectory_with(&["status_package"], &[], 0, 1, 0, 0),
            &[CardRewardSemanticRoleV1::StatusGenerator],
        ),
    ];

    let selection = select_branch_retention_portfolio_v1(&candidates, retention_config(4, Some(4)));
    let payoff_only_kept = selection
        .keep_indices
        .iter()
        .filter(|index| is_payoff_only_package_branch(&candidates[**index]))
        .count();

    assert_eq!(
        payoff_only_kept, 1,
        "payoff-only branches should be sampled, not allowed to crowd out enablers before a package is committed"
    );
    assert!(selection.keep_indices.contains(&3));
    assert!(selection.keep_indices.contains(&4));
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
        act: 1,
        floor: 1,
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
        strategic_debt_tags: Vec::new(),
        startup: Default::default(),
        card_admission_context: None,
    };

    let decision = decide_branch_retention_v1(&candidate);

    assert!(decision.slots.contains(&BranchRetentionSlotV1::Package));
    assert!(decision
        .slots
        .contains(&BranchRetentionSlotV1::DefenseEngine));
    assert!(!decision.slots.contains(&BranchRetentionSlotV1::Frontload));
}

#[test]
fn startup_liability_is_rank_evidence_not_coverage_slot_gate() {
    let candidate = BranchRetentionCandidateInputV1 {
        index: 0,
        act: 2,
        floor: 25,
        frontier_key: "same-frontier".to_string(),
        rank_key: 10_000,
        hp: 70,
        max_hp: 80,
        gold: 120,
        deck_count: 20,
        strategy_formation: Some(formation(
            StrategyDeckFormationStageV1::PlanSeeded,
            &[StrategyDeckFormationNeedV1::Scaling],
            &[],
        )),
        trajectory: BranchTrajectorySignatureV1::default(),
        choice_profiles: vec![CardRewardSemanticProfileV1 {
            card: CardId::Pummel,
            name: "Pummel".to_string(),
            roles: vec![
                CardRewardSemanticRoleV1::StrengthPayoff,
                CardRewardSemanticRoleV1::PackagePayoff,
            ],
            dependencies: Vec::new(),
            unsupported_mechanics: Vec::new(),
        }],
        choice_effect_keys: vec!["take_card".to_string()],
        lineage_flags: Vec::new(),
        strategic_debt_tags: Vec::new(),
        startup: Default::default(),
        card_admission_context: None,
    };

    let decision = decide_branch_retention_v1(&candidate);

    assert!(
        decision.slots.contains(&BranchRetentionSlotV1::Package),
        "candidate shape should still be visible to package coverage"
    );
    assert!(
        decision.slots.contains(&BranchRetentionSlotV1::Scaling),
        "candidate shape should still be visible to scaling coverage"
    );
    assert!(decision
        .coverage_selection
        .slots
        .contains(&BranchRetentionSlotV1::Package));
    assert!(decision
        .coverage_selection
        .slots
        .contains(&BranchRetentionSlotV1::Scaling));
    assert!(decision
        .reasons
        .iter()
        .any(|reason| reason == "card admission rejects at least one added card"));
    assert!(decision.rank_adjustment.admission_pressure < 0);
    assert!(decision
        .rank_adjustment
        .reasons
        .iter()
        .any(|reason| reason.contains("card_admission")));
}

#[test]
fn portfolio_fill_records_startup_rejection_and_penalizes_selection() {
    let rejected_pummel = BranchRetentionCandidateInputV1 {
        index: 0,
        act: 2,
        floor: 25,
        frontier_key: "same-frontier".to_string(),
        rank_key: 50_000,
        hp: 70,
        max_hp: 80,
        gold: 120,
        deck_count: 20,
        strategy_formation: None,
        trajectory: BranchTrajectorySignatureV1::default(),
        choice_profiles: vec![CardRewardSemanticProfileV1 {
            card: CardId::Pummel,
            name: "Pummel".to_string(),
            roles: vec![
                CardRewardSemanticRoleV1::StrengthPayoff,
                CardRewardSemanticRoleV1::PackagePayoff,
            ],
            dependencies: Vec::new(),
            unsupported_mechanics: Vec::new(),
        }],
        choice_effect_keys: vec!["take_card".to_string()],
        lineage_flags: Vec::new(),
        strategic_debt_tags: Vec::new(),
        startup: Default::default(),
        card_admission_context: None,
    };
    let skip = BranchRetentionCandidateInputV1 {
        index: 1,
        act: 2,
        floor: 25,
        frontier_key: "same-frontier".to_string(),
        rank_key: 10_000,
        hp: 70,
        max_hp: 80,
        gold: 120,
        deck_count: 19,
        strategy_formation: None,
        trajectory: BranchTrajectorySignatureV1::default(),
        choice_profiles: Vec::new(),
        choice_effect_keys: vec!["skip_reward".to_string()],
        lineage_flags: Vec::new(),
        strategic_debt_tags: Vec::new(),
        startup: Default::default(),
        card_admission_context: None,
    };

    let selection = select_branch_retention_portfolio_v1(
        &[rejected_pummel.clone(), skip],
        retention_config(1, Some(1)),
    );

    assert!(!selection.keep_indices.contains(&0));
    assert!(selection.keep_indices.contains(&1));

    let decision = selection
        .decisions_by_index
        .get(&0)
        .expect("retention decision exists for rejected candidate");
    assert!(decision
        .rank_adjustment
        .reasons
        .iter()
        .any(|reason| reason.contains("card_admission")));
}

#[test]
fn adjusted_rank_keeps_card_admission_as_report_only_evidence() {
    let rejected_pummel = BranchRetentionCandidateInputV1 {
        index: 0,
        act: 2,
        floor: 25,
        frontier_key: "same-frontier".to_string(),
        rank_key: 50_000,
        hp: 70,
        max_hp: 80,
        gold: 120,
        deck_count: 20,
        strategy_formation: None,
        trajectory: BranchTrajectorySignatureV1::default(),
        choice_profiles: vec![CardRewardSemanticProfileV1 {
            card: CardId::Pummel,
            name: "Pummel".to_string(),
            roles: vec![
                CardRewardSemanticRoleV1::StrengthPayoff,
                CardRewardSemanticRoleV1::PackagePayoff,
            ],
            dependencies: Vec::new(),
            unsupported_mechanics: Vec::new(),
        }],
        choice_effect_keys: vec!["take_card".to_string()],
        lineage_flags: Vec::new(),
        strategic_debt_tags: Vec::new(),
        startup: Default::default(),
        card_admission_context: None,
    };
    let adjustment = branch_retention_rank_adjustment_v1(&rejected_pummel);

    assert_eq!(
        branch_retention_adjusted_rank_key_v1(&rejected_pummel),
        rejected_pummel.rank_key,
        "card admission evidence must not become a second rank-scoring path"
    );
    assert!(
        adjustment.admission_pressure < 0,
        "card admission pressure should remain visible for audit"
    );
}

#[test]
fn retention_selection_filters_hard_admission_before_diversity_fallback() {
    let rejected_pummel = BranchRetentionCandidateInputV1 {
        index: 0,
        act: 2,
        floor: 25,
        frontier_key: "same-frontier".to_string(),
        rank_key: 50_000,
        hp: 70,
        max_hp: 80,
        gold: 120,
        deck_count: 20,
        strategy_formation: None,
        trajectory: BranchTrajectorySignatureV1::default(),
        choice_profiles: vec![CardRewardSemanticProfileV1 {
            card: CardId::Pummel,
            name: "Pummel".to_string(),
            roles: vec![
                CardRewardSemanticRoleV1::StrengthPayoff,
                CardRewardSemanticRoleV1::PackagePayoff,
            ],
            dependencies: Vec::new(),
            unsupported_mechanics: Vec::new(),
        }],
        choice_effect_keys: vec!["take_card".to_string()],
        lineage_flags: Vec::new(),
        strategic_debt_tags: Vec::new(),
        startup: Default::default(),
        card_admission_context: None,
    };
    let skip = BranchRetentionCandidateInputV1 {
        index: 1,
        act: 2,
        floor: 25,
        frontier_key: "same-frontier".to_string(),
        rank_key: 10_000,
        hp: 70,
        max_hp: 80,
        gold: 120,
        deck_count: 19,
        strategy_formation: None,
        trajectory: BranchTrajectorySignatureV1::default(),
        choice_profiles: Vec::new(),
        choice_effect_keys: vec!["skip_reward".to_string()],
        lineage_flags: Vec::new(),
        strategic_debt_tags: Vec::new(),
        startup: Default::default(),
        card_admission_context: None,
    };

    let selection = select_branch_retention_portfolio_v1(
        &[rejected_pummel, skip],
        retention_config(1, Some(1)),
    );

    assert!(!selection.keep_indices.contains(&0));
    assert!(
        selection.keep_indices.contains(&1),
        "branch retention should prefer ordinary candidates over hard-blocked admission branches without needing rank penalties"
    );
}

#[test]
fn rank_adjustment_exposes_card_admission_pressure() {
    let rejected_pummel = BranchRetentionCandidateInputV1 {
        index: 0,
        act: 2,
        floor: 25,
        frontier_key: "same-frontier".to_string(),
        rank_key: 50_000,
        hp: 70,
        max_hp: 80,
        gold: 120,
        deck_count: 20,
        strategy_formation: None,
        trajectory: BranchTrajectorySignatureV1::default(),
        choice_profiles: vec![CardRewardSemanticProfileV1 {
            card: CardId::Pummel,
            name: "Pummel".to_string(),
            roles: vec![
                CardRewardSemanticRoleV1::StrengthPayoff,
                CardRewardSemanticRoleV1::PackagePayoff,
            ],
            dependencies: Vec::new(),
            unsupported_mechanics: Vec::new(),
        }],
        choice_effect_keys: vec!["take_card".to_string()],
        lineage_flags: Vec::new(),
        strategic_debt_tags: Vec::new(),
        startup: Default::default(),
        card_admission_context: None,
    };

    let adjustment = branch_retention_rank_adjustment_v1(&rejected_pummel);

    assert_eq!(adjustment.base_rank_key, 50_000);
    assert!(adjustment.admission_pressure < 0);
    assert_eq!(
        adjustment.effective_rank_key, rejected_pummel.rank_key,
        "card admission component evidence should not be applied to effective rank"
    );
    assert!(
        adjustment
            .reasons
            .iter()
            .any(|reason| reason.contains("card_admission")),
        "rank adjustment should name the card admission contribution"
    );
}

#[test]
fn rank_adjustment_lightly_penalizes_unresolved_deck_debt() {
    let mut unresolved_deck = BranchRetentionCandidateInputV1 {
        index: 0,
        act: 1,
        floor: 11,
        frontier_key: "same-frontier".to_string(),
        rank_key: 12_000,
        hp: 72,
        max_hp: 80,
        gold: 113,
        deck_count: 14,
        strategy_formation: None,
        trajectory: BranchTrajectorySignatureV1::default(),
        choice_profiles: Vec::new(),
        choice_effect_keys: Vec::new(),
        lineage_flags: Vec::new(),
        strategic_debt_tags: Vec::new(),
        startup: Default::default(),
        card_admission_context: None,
    };
    unresolved_deck.startup.has_strength_payoff_without_strength = true;

    let adjustment = branch_retention_rank_adjustment_v1(&unresolved_deck);
    let decision = decide_branch_retention_v1(&unresolved_deck);

    assert_eq!(adjustment.startup_adjustment, -1_000);
    assert_eq!(adjustment.effective_rank_key, 11_000);
    assert!(adjustment
        .reasons
        .iter()
        .any(|reason| reason == "current_startup_debt_rank_adjustment:-1000"));
    assert!(decision
        .reasons
        .contains(&"current deck has unresolved startup liability".to_string()));
    assert!(!decision
        .reasons
        .contains(&"card admission rejects at least one added card".to_string()));
}

#[test]
fn package_lane_selection_uses_rank_not_slot_evidence_score() {
    let high_rank_package = named_semantic_retention_candidate(
        0,
        50_000,
        "Generic Package",
        BranchTrajectorySignatureV1::default(),
        &[CardRewardSemanticRoleV1::PackagePayoff],
    );
    let mut context_boosted_package = named_semantic_retention_candidate(
        1,
        10_000,
        "Strength Package",
        BranchTrajectorySignatureV1::default(),
        &[
            CardRewardSemanticRoleV1::StrengthPayoff,
            CardRewardSemanticRoleV1::PackagePayoff,
        ],
    );
    context_boosted_package.strategy_formation = Some(formation(
        StrategyDeckFormationStageV1::PlanSeeded,
        &[],
        &[StrategyPackageIdV2::StrengthScaling],
    ));
    context_boosted_package
        .startup
        .persistent_strength_source_count = 1;

    let candidates = vec![high_rank_package, context_boosted_package];
    let selection = select_branch_retention_portfolio_v1(&candidates, retention_config(1, Some(1)));

    assert!(
        selection.keep_indices.contains(&0),
        "slot evidence/context boosts should be evidence, not the lane-local selector"
    );
    assert!(!selection.keep_indices.contains(&1));
}

#[test]
fn context_packet_keys_are_rank_evidence_not_coverage_slots() {
    let mut candidate = semantic_retention_candidate(
        0,
        10_000,
        64,
        80,
        BranchTrajectorySignatureV1::default(),
        &[CardRewardSemanticRoleV1::CardDraw],
    );
    candidate.strategy_formation = Some(formation(
        StrategyDeckFormationStageV1::Transitional,
        &[StrategyDeckFormationNeedV1::DrawEnergy],
        &[],
    ));

    let decision = decide_branch_retention_v1(&candidate);

    assert!(
        !decision.slots.contains(&BranchRetentionSlotV1::EngineSetup),
        "context packet matches should not directly add coverage slots"
    );
    assert!(
        !decision
            .coverage_selection
            .slots
            .contains(&BranchRetentionSlotV1::EngineSetup),
        "coverage selection should stay candidate-shape only"
    );
    assert!(decision
        .rank_adjustment
        .context_keys
        .iter()
        .any(|key| key == "matches_formation_draw_energy_need"));
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
        act: 1,
        floor: 1,
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
        strategic_debt_tags: Vec::new(),
        startup: Default::default(),
        card_admission_context: None,
    }
}

fn effect_retention_candidate(
    index: usize,
    rank_key: i32,
    effect_key: &str,
) -> BranchRetentionCandidateInputV1 {
    BranchRetentionCandidateInputV1 {
        index,
        act: 1,
        floor: 1,
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
        strategic_debt_tags: Vec::new(),
        startup: Default::default(),
        card_admission_context: None,
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
        act: 1,
        floor: 1,
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
        strategic_debt_tags: Vec::new(),
        startup: Default::default(),
        card_admission_context: None,
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

fn retention_config(max_total: usize, max_per_frontier: Option<usize>) -> BranchRetentionConfigV1 {
    BranchRetentionConfigV1 {
        max_total,
        max_per_frontier,
        budget_profile: BranchRetentionBudgetProfileV1::Balanced,
    }
}
