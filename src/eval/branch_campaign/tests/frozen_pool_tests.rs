use super::*;

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
        bucket_mask: 0,
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
