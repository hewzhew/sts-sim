use super::*;
use crate::ai::noncombat_strategy_v1::{StrategyDeckFormationNeedV1, StrategyDeckFormationStageV1};
use crate::eval::branch_experiment::{
    BranchExperimentBranchReportV1, BranchExperimentBranchStatusV1, BranchExperimentChoiceV1,
    BranchExperimentFrontierV1, BranchExperimentLineageV1, BranchExperimentRunSummaryV1,
};
use crate::eval::branch_experiment_retention::{BranchRetentionDecisionV1, BranchRetentionSlotV1};
use crate::eval::branch_experiment_trajectory::BranchTrajectorySignatureV1;

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

    assert_eq!(replay, vec!["ar", "rp 0", "ar", "event 1"]);
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
        stuck: Vec::new(),
        discarded_count: 3,
        strategy_requests: vec![BranchCampaignStrategyRequestV1 {
            kind: "event_strategy".to_string(),
            boundary_title: "Falling".to_string(),
            branch_count: 2,
            examples: vec!["Strike -> Defend".to_string()],
            suggested_action: "provide Falling policy".to_string(),
        }],
        rounds: Vec::new(),
    };

    let rendered = render_branch_campaign_compact_v1(&report, 2);

    assert!(rendered.contains("BranchCampaignV1 seed=521 rounds=2 stop=needs_intervention"));
    assert!(rendered.contains("Active 1 | Frozen 1 | Dead 0 | Victories 0 | Stuck 0 | Discarded 3"));
    assert!(rendered.contains("Needs intervention:"));
    assert!(rendered.contains("event_strategy | Falling | branches=2"));
    assert!(rendered.contains(
        "next: write a narrow event rule or choose one branch manually, then rerun the campaign"
    ));
    assert!(rendered.contains("Top active:"));
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
        stuck: Vec::new(),
        discarded_count: 0,
        strategy_requests: Vec::new(),
        rounds: Vec::new(),
    };

    let rendered = render_branch_campaign_compact_v1(&report, 1);

    assert!(rendered.contains("Next: budget ended; use .\\tools\\campaign.ps1 -More"));
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
            floor: 2,
            hp: 70,
            max_hp: 80,
            gold: 120,
            deck_count: 11,
            relic_count: 1,
            potion_count: 0,
            formation_stage: StrategyDeckFormationStageV1::PlanSeeded,
            formation_needs: vec![StrategyDeckFormationNeedV1::Frontload],
            formation_strengths: Vec::new(),
            trajectory: BranchTrajectorySignatureV1::default(),
            boundary_title: "Card Reward".to_string(),
        },
        frontier: BranchExperimentFrontierV1 {
            key: "card_reward".to_string(),
            act: 1,
            floor: 2,
            boundary_title: "Card Reward".to_string(),
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
