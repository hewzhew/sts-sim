use super::*;
use std::collections::{BTreeMap, BTreeSet};

use crate::ai::card_reward_policy_v1::card_reward_semantic_profile_v1;
use crate::ai::noncombat_strategy_v1::{
    StrategyDeckFormationNeedV1, StrategyDeckFormationStageV1, StrategyFormationSummaryV2,
};
use crate::content::cards::CardId;
use crate::content::potions::PotionId;
use crate::content::relics::RelicId;
use crate::content::relics::RelicState;
use crate::eval::branch_experiment_retention::{
    branch_retention_rank_adjustment_v1, BranchRetentionBudgetProfileV1,
};
use crate::runtime::combat::CombatCard;
use crate::state::core::{
    ActiveCombat, CombatContext, RoomCombatContext, RunPendingChoiceReason, RunPendingChoiceState,
};
use crate::state::events::{EventId, EventState};
use crate::state::map::node::RoomType;
use crate::state::rewards::{BossRelicChoiceState, RewardState};
use crate::state::shop::{ShopCard, ShopPotion, ShopRelic, ShopState};
use crate::test_support::blank_test_combat;
use std::fs;
use std::path::PathBuf;

#[test]
fn branch_experiment_schema_version_tracks_lineage_pruned_summary() {
    assert_eq!(BRANCH_EXPERIMENT_SCHEMA_VERSION, 22);
}

#[test]
fn branch_experiment_expands_pending_card_reward_choices() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut reward = RewardState::new();
    reward.pending_card_choice = Some(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::Cleave, 0),
    ]);
    session.engine_state = EngineState::RewardScreen(reward);

    let report = run_branch_experiment_from_session(
        session,
        &BranchExperimentConfigV1 {
            max_depth: 1,
            max_branches: 4,
            ..BranchExperimentConfigV1::default()
        },
    );

    assert_eq!(report.explored_branch_points, 1);
    assert_eq!(report.branches.len(), 2);
    assert!(report.branches.iter().any(|branch| {
        branch.choices[0].command == "rp 0" && branch.choices[0].label == "Twin Strike"
    }));
    assert!(report.branches.iter().any(|branch| {
        branch.choices[0].command == "rp 1" && branch.choices[0].label == "Cleave"
    }));
}

#[test]
fn branch_experiment_snapshot_result_tracks_final_branch_sessions() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut reward = RewardState::new();
    reward.pending_card_choice = Some(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::Cleave, 0),
    ]);
    session.engine_state = EngineState::RewardScreen(reward);

    let result = run_branch_experiment_from_session_with_snapshots_v1(
        session,
        &BranchExperimentConfigV1 {
            max_depth: 1,
            max_branches: 4,
            ..BranchExperimentConfigV1::default()
        },
    );

    assert_eq!(result.branch_sessions.len(), result.report.branches.len());
    let twin_branch = result
        .report
        .branches
        .iter()
        .find(|branch| branch.choices[0].label == "Twin Strike")
        .expect("Twin Strike branch");
    let twin_snapshot = result
        .branch_sessions
        .get(&twin_branch.branch_id)
        .expect("snapshot for Twin Strike branch");

    assert!(
        twin_snapshot
            .run_state
            .master_deck
            .iter()
            .any(|card| card.id == CardId::TwinStrike),
        "snapshot should contain the exact post-choice run state"
    );
}

#[test]
fn branch_experiment_marks_final_settle_wall_limit_phase() {
    let session = RunControlSession::new(RunControlConfig::default());

    let report = run_branch_experiment_from_session(
        session,
        &BranchExperimentConfigV1 {
            max_depth: 0,
            experiment_wall_ms: Some(0),
            ..BranchExperimentConfigV1::default()
        },
    );

    assert_eq!(
        report.wall_limit_phase,
        Some(BranchExperimentWallLimitPhaseV1::FinalSettle)
    );
}

#[test]
fn branch_experiment_auto_leaves_after_shop_purchase_branch_by_default() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.gold = 100;
    let mut shop = ShopState::new();
    shop.cards.push(ShopCard {
        card_id: CardId::PommelStrike,
        upgrades: 0,
        price: 50,
        can_buy: true,
        blocked_reason: None,
    });
    session.engine_state = EngineState::Shop(shop);

    let report = run_branch_experiment_from_session(
        session,
        &BranchExperimentConfigV1 {
            max_depth: 1,
            max_branches: 4,
            ..BranchExperimentConfigV1::default()
        },
    );

    let buy_branch = report
        .branches
        .iter()
        .find(|branch| {
            branch
                .choices
                .iter()
                .any(|choice| choice.effect_kind == "shop_buy_card")
        })
        .expect("buy-card branch");
    let buy_choice = buy_branch
        .choices
        .iter()
        .find(|choice| choice.effect_kind == "shop_buy_card")
        .expect("buy-card choice");

    assert!(
        buy_choice.effect_label.contains("auto leave shop"),
        "the compact report should make the shop-close behavior visible"
    );
    assert_ne!(
        buy_branch.summary.boundary_title, "Shop",
        "a single purchase branch should close the shop to avoid repeated buy-combination expansion"
    );
}

#[test]
fn branch_experiment_keeps_high_gold_shop_open_after_single_purchase() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.gold = 631;
    let mut shop = ShopState::new();
    shop.cards.push(ShopCard {
        card_id: CardId::Shockwave,
        upgrades: 0,
        price: 120,
        can_buy: true,
        blocked_reason: None,
    });
    shop.cards.push(ShopCard {
        card_id: CardId::FlameBarrier,
        upgrades: 0,
        price: 90,
        can_buy: true,
        blocked_reason: None,
    });
    shop.relics.push(ShopRelic {
        relic_id: RelicId::FrozenEye,
        price: 120,
        can_buy: true,
        blocked_reason: None,
    });
    shop.potions.push(ShopPotion {
        potion_id: PotionId::FirePotion,
        price: 40,
        can_buy: true,
        blocked_reason: None,
    });
    shop.purge_available = false;
    session.engine_state = EngineState::Shop(shop);

    let report = run_branch_experiment_from_session(
        session,
        &BranchExperimentConfigV1 {
            max_depth: 1,
            max_branches: 4,
            ..BranchExperimentConfigV1::default()
        },
    );

    let buy_branch = report
        .branches
        .iter()
        .find(|branch| {
            branch
                .choices
                .iter()
                .any(|choice| choice.effect_kind == "shop_buy_card")
        })
        .expect("buy-card branch");
    let buy_choice = buy_branch
        .choices
        .iter()
        .find(|choice| choice.effect_kind == "shop_buy_card")
        .expect("buy-card choice");

    assert!(
        !buy_choice.effect_label.contains("auto leave shop"),
        "high-gold shops should keep converting gold instead of leaving after one purchase"
    );
    assert_eq!(
        buy_branch.summary.boundary_title, "Shop",
        "high-gold purchase branches should remain at the shop for additional purchases"
    );
}

#[test]
fn branch_experiment_executes_shop_combo_purchase_branch() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.floor_num = 6;
    session.run_state.gold = 631;
    let mut shop = ShopState::new();
    shop.cards.push(ShopCard {
        card_id: CardId::Shockwave,
        upgrades: 0,
        price: 120,
        can_buy: true,
        blocked_reason: None,
    });
    shop.relics.push(ShopRelic {
        relic_id: RelicId::FrozenEye,
        price: 120,
        can_buy: true,
        blocked_reason: None,
    });
    shop.potions.push(ShopPotion {
        potion_id: PotionId::FirePotion,
        price: 40,
        can_buy: true,
        blocked_reason: None,
    });
    shop.purge_available = false;
    session.engine_state = EngineState::Shop(shop);

    let report = run_branch_experiment_from_session(
        session,
        &BranchExperimentConfigV1 {
            max_depth: 1,
            max_branches: 8,
            ..BranchExperimentConfigV1::default()
        },
    );

    let combo_branch = report
        .branches
        .iter()
        .find(|branch| {
            branch
                .choices
                .iter()
                .any(|choice| choice.effect_kind == "shop_buy_combo")
        })
        .expect("shop combo branch");
    let combo_choice = combo_branch
        .choices
        .iter()
        .find(|choice| choice.effect_kind == "shop_buy_combo")
        .expect("shop combo choice");

    assert!(combo_choice.command.contains(" && "));
    assert!(combo_choice.command.contains("buy relic 0"));
    assert!(combo_choice.command.contains("buy card 0"));
    assert!(combo_choice.command.contains("buy potion 0"));
    assert!(
        combo_choice.effect_label.contains("auto leave shop"),
        "empty post-combo shop should be closed by compiler-level executable plan check"
    );
}

#[test]
fn branch_experiment_retains_shop_combo_purchase_under_branch_cap() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.act_num = 2;
    session.run_state.floor_num = 25;
    session.run_state.gold = 220;
    let mut shop = ShopState::new();
    shop.purge_available = false;
    for card_id in [
        CardId::PommelStrike,
        CardId::TwinStrike,
        CardId::ShrugItOff,
        CardId::Cleave,
        CardId::IronWave,
    ] {
        shop.cards.push(ShopCard {
            card_id,
            upgrades: 0,
            price: 50,
            can_buy: true,
            blocked_reason: None,
        });
    }
    shop.relics.push(ShopRelic {
        relic_id: RelicId::FrozenEye,
        price: 120,
        can_buy: true,
        blocked_reason: None,
    });
    shop.potions.push(ShopPotion {
        potion_id: PotionId::FirePotion,
        price: 40,
        can_buy: true,
        blocked_reason: None,
    });
    session.engine_state = EngineState::Shop(shop);

    let report = run_branch_experiment_from_session(
        session,
        &BranchExperimentConfigV1 {
            max_depth: 1,
            max_branches: 4,
            ..BranchExperimentConfigV1::default()
        },
    );

    assert!(
        report.branches.iter().any(|branch| branch
            .choices
            .iter()
            .any(|choice| choice.effect_kind == "shop_buy_combo")),
        "capped shop portfolios should retain the compact combo representative"
    );
}

#[test]
fn branch_experiment_does_not_auto_leave_shop_purchase_that_opens_reward_overlay() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.gold = 200;
    let mut shop = ShopState::new();
    shop.relics.push(ShopRelic {
        relic_id: RelicId::Orrery,
        price: 145,
        can_buy: true,
        blocked_reason: None,
    });
    session.engine_state = EngineState::Shop(shop);

    let report = run_branch_experiment_from_session(
        session,
        &BranchExperimentConfigV1 {
            max_depth: 1,
            max_branches: 4,
            ..BranchExperimentConfigV1::default()
        },
    );

    let buy_branch = report
        .branches
        .iter()
        .find(|branch| {
            branch
                .choices
                .iter()
                .any(|choice| choice.effect_kind == "shop_buy_relic")
        })
        .expect("buy-relic branch");
    let buy_choice = buy_branch
        .choices
        .iter()
        .find(|choice| choice.effect_kind == "shop_buy_relic")
        .expect("buy-relic choice");

    assert!(
        !buy_choice.effect_label.contains("auto leave shop"),
        "shop purchases that open reward overlays must leave the branch at the overlay boundary"
    );
    assert_eq!(buy_branch.summary.boundary_title, "Reward Overlay");
}

#[test]
fn branch_experiment_does_not_auto_leave_shop_purchase_that_opens_duplicate_choice() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.gold = 200;
    session.run_state.add_card_to_deck(CardId::Offering);
    let mut shop = ShopState::new();
    shop.relics.push(ShopRelic {
        relic_id: RelicId::DollysMirror,
        price: 155,
        can_buy: true,
        blocked_reason: None,
    });
    session.engine_state = EngineState::Shop(shop);

    let report = run_branch_experiment_from_session(
        session,
        &BranchExperimentConfigV1 {
            max_depth: 1,
            max_branches: 4,
            ..BranchExperimentConfigV1::default()
        },
    );

    let buy_branch = report
        .branches
        .iter()
        .find(|branch| {
            branch
                .choices
                .iter()
                .any(|choice| choice.effect_kind == "shop_buy_relic")
        })
        .expect("buy-relic branch");
    let buy_choice = buy_branch
        .choices
        .iter()
        .find(|choice| choice.effect_kind == "shop_buy_relic")
        .expect("buy-relic choice");

    assert!(
        !buy_choice.effect_label.contains("auto leave shop"),
        "shop purchases that open deck selection must leave the branch at the selection boundary"
    );
    assert_eq!(buy_branch.summary.boundary_title, "Run Choice Duplicate");
}

#[test]
fn branch_experiment_replay_trace_uses_trace_run_config() {
    let trace_path = write_trace_fixture(
        "branch_experiment_trace_config",
        &crate::eval::run_control::SessionTraceV1::new(&RunControlSession::new(RunControlConfig {
            seed: 777,
            ..RunControlConfig::default()
        })),
    );

    let report = run_branch_experiment_v1(&BranchExperimentConfigV1 {
        seed: 1,
        replay_trace_path: Some(trace_path.clone()),
        max_depth: 0,
        ..BranchExperimentConfigV1::default()
    })
    .expect("empty trace should replay");

    assert_eq!(report.seed, 777);
    assert_eq!(
        report.replay_trace_path,
        Some(trace_path.display().to_string())
    );
    assert_eq!(report.replay_trace_applied_steps, 0);
    assert_eq!(report.replay_trace_stop, Some("TraceEnd".to_string()));

    let _ = fs::remove_dir_all(trace_path.parent().unwrap());
}

#[test]
fn branch_experiment_report_counts_replayed_trace_steps() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let trace_path = unique_temp_path("branch_experiment_trace_steps").join("trace.json");
    let mut recorder =
        crate::eval::run_control::SessionTraceRecorder::new(trace_path.clone(), &session);
    let command = RunControlCommand::DefaultCandidate;
    let pending =
        crate::eval::run_control::SessionTraceRecorder::prepare_step(&session, "", &command);
    let outcome = session
        .apply_command(command)
        .expect("default candidate applies");
    recorder
        .record_action_step(
            pending,
            &session,
            outcome
                .action_result
                .as_ref()
                .expect("command should change state"),
            &outcome.trace_annotations,
        )
        .expect("trace records");
    let trace = recorder.trace().clone();
    let trace_path = write_trace_fixture("branch_experiment_trace_steps", &trace);

    let report = run_branch_experiment_v1(&BranchExperimentConfigV1 {
        replay_trace_path: Some(trace_path.clone()),
        replay_trace_max_steps: Some(1),
        max_depth: 0,
        ..BranchExperimentConfigV1::default()
    })
    .expect("one step trace should replay");

    assert_eq!(report.replay_trace_applied_steps, 1);
    assert_eq!(report.replay_trace_stop, Some("TraceEnd".to_string()));

    let _ = fs::remove_dir_all(trace_path.parent().unwrap());
}

#[test]
fn shared_start_profile_runner_reuses_replay_prefix_for_all_profiles() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let trace_path = unique_temp_path("branch_experiment_shared_start").join("trace.json");
    let mut recorder =
        crate::eval::run_control::SessionTraceRecorder::new(trace_path.clone(), &session);
    let command = RunControlCommand::DefaultCandidate;
    let pending =
        crate::eval::run_control::SessionTraceRecorder::prepare_step(&session, "", &command);
    let outcome = session
        .apply_command(command)
        .expect("default candidate applies");
    recorder
        .record_action_step(
            pending,
            &session,
            outcome
                .action_result
                .as_ref()
                .expect("command should change state"),
            &outcome.trace_annotations,
        )
        .expect("trace records");
    let trace_path = write_trace_fixture("branch_experiment_shared_start", recorder.trace());
    let configs = vec![
        BranchExperimentConfigV1 {
            replay_trace_path: Some(trace_path.clone()),
            replay_trace_max_steps: Some(1),
            retention_budget_profile: BranchRetentionBudgetProfileV1::Balanced,
            max_depth: 0,
            ..BranchExperimentConfigV1::default()
        },
        BranchExperimentConfigV1 {
            replay_trace_path: Some(trace_path.clone()),
            replay_trace_max_steps: Some(1),
            retention_budget_profile: BranchRetentionBudgetProfileV1::Package,
            max_depth: 0,
            ..BranchExperimentConfigV1::default()
        },
    ];

    let reports =
        run_branch_experiment_profiles_from_shared_start_v1(&configs).expect("shared profile run");

    assert_eq!(reports.len(), 2);
    assert_eq!(reports[0].replay_trace_applied_steps, 1);
    assert_eq!(reports[1].replay_trace_applied_steps, 1);
    assert_eq!(reports[0].replay_trace_stop, Some("TraceEnd".to_string()));
    assert_eq!(reports[1].replay_trace_stop, Some("TraceEnd".to_string()));
    assert_eq!(
        reports[0].replay_trace_path,
        Some(trace_path.display().to_string())
    );
    assert_eq!(reports[0].seed, reports[1].seed);

    let _ = fs::remove_dir_all(trace_path.parent().unwrap());
}

#[test]
fn shared_start_profile_runner_rejects_mismatched_start_inputs() {
    let configs = vec![
        BranchExperimentConfigV1 {
            seed: 1,
            retention_budget_profile: BranchRetentionBudgetProfileV1::Balanced,
            ..BranchExperimentConfigV1::default()
        },
        BranchExperimentConfigV1 {
            seed: 2,
            retention_budget_profile: BranchRetentionBudgetProfileV1::Package,
            ..BranchExperimentConfigV1::default()
        },
    ];

    let err = run_branch_experiment_profiles_from_shared_start_v1(&configs)
        .expect_err("mismatched start inputs should be rejected");

    assert!(err.contains("shared-start profile configs differ in seed"));
}

#[test]
fn branch_experiment_can_limit_reward_options_by_semantic_portfolio() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut reward = RewardState::new();
    reward.pending_card_choice = Some(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::Cleave, 0),
        RewardCard::new(CardId::ShrugItOff, 0),
    ]);
    session.engine_state = EngineState::RewardScreen(reward);

    let report = run_branch_experiment_from_session(
        session,
        &BranchExperimentConfigV1 {
            max_depth: 1,
            max_branches: 4,
            max_reward_options_per_branch: Some(2),
            auto_max_operations: 0,
            ..BranchExperimentConfigV1::default()
        },
    );

    let picked_labels = report
        .branches
        .iter()
        .map(|branch| branch.choices[0].label.as_str())
        .collect::<BTreeSet<_>>();

    assert_eq!(report.branches.len(), 2);
    assert!(
        picked_labels.contains("Shrug It Off"),
        "non-transition defense/draw candidate should not be crowded out"
    );
    assert_eq!(
        picked_labels
            .iter()
            .filter(|label| **label == "Twin Strike" || **label == "Cleave")
            .count(),
        1,
        "pure transition options should be represented, not exhaustively expanded"
    );
}

#[test]
fn branch_experiment_include_skip_expands_card_reward_skip_branch() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut reward = RewardState::new();
    reward.items.push(crate::state::rewards::RewardItem::Card {
        cards: vec![
            RewardCard::new(CardId::TwinStrike, 0),
            RewardCard::new(CardId::Cleave, 0),
            RewardCard::new(CardId::ShrugItOff, 0),
        ],
    });
    session.engine_state = EngineState::RewardScreen(reward);

    let report = run_branch_experiment_from_session(
        session,
        &BranchExperimentConfigV1 {
            max_depth: 1,
            max_branches: 8,
            auto_max_operations: 0,
            include_skip: true,
            ..BranchExperimentConfigV1::default()
        },
    );

    let labels = report
        .branches
        .iter()
        .map(|branch| branch.choices[0].label.as_str())
        .collect::<BTreeSet<_>>();

    assert_eq!(report.branches.len(), 4);
    assert!(labels.contains("Skip card reward"));
    assert!(report
        .branches
        .iter()
        .all(|branch| branch.status != BranchExperimentBranchStatusV1::Failed));
}

#[test]
fn branch_experiment_retention_uses_singing_bowl_as_decline_effect() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.relics.push(RelicState::new(
        crate::content::relics::RelicId::SingingBowl,
    ));
    let mut reward = RewardState::new();
    reward.items.push(crate::state::rewards::RewardItem::Card {
        cards: vec![
            RewardCard::new(CardId::TwinStrike, 0),
            RewardCard::new(CardId::Cleave, 0),
            RewardCard::new(CardId::ShrugItOff, 0),
        ],
    });
    session.engine_state = EngineState::RewardScreen(reward);

    let report = run_branch_experiment_from_session(
        session,
        &BranchExperimentConfigV1 {
            max_depth: 1,
            max_branches: 3,
            auto_max_operations: 0,
            include_skip: true,
            ..BranchExperimentConfigV1::default()
        },
    );

    assert_eq!(report.branches.len(), 3);
    assert!(report.branches.iter().any(|branch| {
        branch
            .choices
            .iter()
            .any(|choice| choice.effect_kind == "singing_bowl")
    }));
    assert!(!report.branches.iter().any(|branch| {
        branch
            .choices
            .iter()
            .any(|choice| choice.effect_kind == "skip_card_reward")
    }));
}

#[test]
fn branch_experiment_reports_reward_option_portfolio_pruning() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut reward = RewardState::new();
    reward.pending_card_choice = Some(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::Cleave, 0),
        RewardCard::new(CardId::ShrugItOff, 0),
    ]);
    session.engine_state = EngineState::RewardScreen(reward);

    let report = run_branch_experiment_from_session(
        session,
        &BranchExperimentConfigV1 {
            max_depth: 1,
            max_branches: 4,
            max_reward_options_per_branch: Some(2),
            auto_max_operations: 0,
            ..BranchExperimentConfigV1::default()
        },
    );

    assert_eq!(report.reward_option_portfolios.len(), 1);
    let portfolio = &report.reward_option_portfolios[0];
    assert_eq!(portfolio.depth, 0);
    assert_eq!(portfolio.original_count, 3);
    assert_eq!(portfolio.selected_count, 2);
    assert_eq!(portfolio.pruned_options.len(), 1);
    assert!(portfolio
        .selected_options
        .iter()
        .any(|option| option.label == "Shrug It Off"));
    assert!(portfolio
        .pruned_options
        .iter()
        .any(|option| option.semantic_class.contains("pure_transition_frontload")));
}

#[test]
fn branch_experiment_expands_campfire_choices() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.current_hp = 40;
    session.engine_state = EngineState::Campfire;

    let report = run_branch_experiment_from_session(
        session,
        &BranchExperimentConfigV1 {
            max_depth: 1,
            max_branches: 4,
            max_campfire_options_per_branch: Some(2),
            ..BranchExperimentConfigV1::default()
        },
    );

    assert_eq!(report.explored_branch_points, 1);
    assert!(report.branches.iter().any(|branch| branch
        .choices
        .iter()
        .any(|choice| { choice.kind == "campfire" && !choice.command.trim().is_empty() })));
}

#[test]
fn branch_experiment_expands_boss_relic_choices() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.engine_state = EngineState::BossRelicSelect(BossRelicChoiceState::new(vec![
        RelicId::BlackStar,
        RelicId::EmptyCage,
        RelicId::TinyHouse,
    ]));

    let report = run_branch_experiment_from_session(
        session,
        &BranchExperimentConfigV1 {
            max_depth: 1,
            max_branches: 4,
            auto_max_operations: 0,
            ..BranchExperimentConfigV1::default()
        },
    );

    let choices = report
        .branches
        .iter()
        .flat_map(|branch| &branch.choices)
        .collect::<Vec<_>>();
    assert_eq!(report.explored_branch_points, 1);
    assert_eq!(choices.len(), 3);
    assert!(choices.iter().all(|choice| {
        choice.kind == "boss_relic" && choice.card.is_none() && choice.upgrades.is_none()
    }));
}

#[test]
fn branch_experiment_expands_low_fanout_event_choices() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.event_state = Some(EventState::new(EventId::BigFish));
    session.engine_state = EngineState::EventRoom;

    let report = run_branch_experiment_from_session(
        session,
        &BranchExperimentConfigV1 {
            max_depth: 1,
            max_branches: 4,
            auto_max_operations: 0,
            ..BranchExperimentConfigV1::default()
        },
    );

    let choices = report
        .branches
        .iter()
        .flat_map(|branch| &branch.choices)
        .collect::<Vec<_>>();
    assert_eq!(report.explored_branch_points, 1);
    assert_eq!(choices.len(), 2);
    assert!(choices.iter().all(|choice| {
        choice.kind == "event" && choice.card.is_none() && choice.upgrades.is_none()
    }));
    let event_effects = choices
        .iter()
        .map(|choice| (choice.effect_kind.as_str(), choice.effect_label.as_str()))
        .collect::<BTreeMap<_, _>>();
    assert!(event_effects["event_heal"].contains("[Banana] Heal 26 HP."));
    assert!(event_effects["event_heal"].contains("event_eval"));
    assert!(event_effects["event_gain_max_hp"].contains("[Donut] Gain 5 Max HP."));
    assert!(event_effects["event_gain_max_hp"].contains("event_eval"));
    assert!(
        !event_effects.contains_key("event_gain_relic"),
        "Avoid-tier curse relic option should be pruned when safer event branches exist"
    );
}

#[test]
fn branch_experiment_expands_single_card_run_selection_choices() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.engine_state = EngineState::RunPendingChoice(RunPendingChoiceState {
        min_choices: 1,
        max_choices: 1,
        reason: RunPendingChoiceReason::Purge,
        return_state: Box::new(EngineState::EventRoom),
    });

    let report = run_branch_experiment_from_session(
        session,
        &BranchExperimentConfigV1 {
            max_depth: 1,
            max_branches: 4,
            auto_max_operations: 0,
            ..BranchExperimentConfigV1::default()
        },
    );

    assert_eq!(report.explored_branch_points, 1);
    assert_eq!(report.branches.len(), 2);
    assert!(report.branches.iter().all(|branch| {
        branch.summary.trajectory.frontload_picks == 0
            && branch.summary.trajectory.transition_frontload_picks == 0
            && branch.summary.trajectory.defense_picks == 0
    }));

    let choices = report
        .branches
        .iter()
        .flat_map(|branch| &branch.choices)
        .collect::<Vec<_>>();
    assert_eq!(
        choices
            .iter()
            .map(|choice| {
                (
                    choice.effect_label.as_str(),
                    choice.representative_count,
                    choice.suppressed_count,
                )
            })
            .collect::<Vec<_>>(),
        vec![("remove Strike", 5, 4), ("remove Defend", 4, 3),]
    );
    assert!(!choices
        .iter()
        .any(|choice| choice.effect_label == "remove Bash"));
}

#[test]
fn branch_experiment_applies_typed_event_deck_mutation_choices() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.master_deck.truncate(2);
    session.run_state.event_state = Some(EventState::new(EventId::UpgradeShrine));
    session.engine_state = EngineState::EventRoom;

    let report = run_branch_experiment_from_session(
        session,
        &BranchExperimentConfigV1 {
            max_depth: 2,
            max_branches: 8,
            auto_max_operations: 0,
            ..BranchExperimentConfigV1::default()
        },
    );

    assert!(report.branches.iter().any(|branch| {
        branch.choices.len() == 1
            && branch.choices[0].kind == "event"
            && branch.choices[0].command.starts_with("event-select 0 ")
            && branch.choices[0].effect_kind == "upgrade_card"
    }));
}

#[test]
fn branch_experiment_skips_disabled_event_options_but_keeps_enabled_options() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.gold = 36;
    session.run_state.event_state = Some(EventState::new(EventId::TombRedMask));
    session.engine_state = EngineState::EventRoom;

    let report = run_branch_experiment_from_session(
        session,
        &BranchExperimentConfigV1 {
            max_depth: 1,
            max_branches: 8,
            auto_max_operations: 0,
            ..BranchExperimentConfigV1::default()
        },
    );

    let commands = report
        .branches
        .iter()
        .flat_map(|branch| &branch.choices)
        .map(|choice| choice.command.as_str())
        .collect::<Vec<_>>();

    assert!(
        commands.contains(&"event 1"),
        "pay option should remain branchable when Don the Mask is locked"
    );
    assert!(
        commands.contains(&"event 2"),
        "leave option should remain branchable when Don the Mask is locked"
    );
    assert!(
        !commands.contains(&"event 0"),
        "locked Don the Mask option must not be emitted as a branch command"
    );
}

#[test]
fn pruned_first_pick_count_reports_sort_for_stable_comparison() {
    let reports = pruned_first_pick_count_reports(BTreeMap::from([
        ("Shockwave".to_string(), 2),
        ("Armaments".to_string(), 4),
        ("Clash".to_string(), 4),
    ]));

    assert_eq!(
        reports,
        vec![
            BranchExperimentPrunedFirstPickCountV1 {
                first_pick: "Armaments".to_string(),
                count: 4,
            },
            BranchExperimentPrunedFirstPickCountV1 {
                first_pick: "Clash".to_string(),
                count: 4,
            },
            BranchExperimentPrunedFirstPickCountV1 {
                first_pick: "Shockwave".to_string(),
                count: 2,
            },
        ]
    );
}

#[test]
fn pruned_branch_summary_counts_semantic_retention_loss() {
    let candidates = vec![
        retention_candidate(0, BranchTrajectorySignatureV1::default()),
        retention_candidate(1, trajectory_with_packages(&["exhaust_engine"], &[])),
        retention_candidate(
            2,
            trajectory_with_packages(&["block_engine"], &["block_engine"]),
        ),
    ];
    let decisions = BTreeMap::from([
        (
            0,
            retention_decision(
                BranchRetentionSlotV1::Frontload,
                &[BranchRetentionSlotV1::Frontload],
            ),
        ),
        (
            1,
            retention_decision(
                BranchRetentionSlotV1::EngineSetup,
                &[
                    BranchRetentionSlotV1::EngineSetup,
                    BranchRetentionSlotV1::Diversity,
                ],
            ),
        ),
        (
            2,
            retention_decision(
                BranchRetentionSlotV1::Package,
                &[
                    BranchRetentionSlotV1::Package,
                    BranchRetentionSlotV1::DefenseEngine,
                ],
            ),
        ),
    ]);
    let keep_indices = BTreeSet::from([0]);
    let mut branch_with_reward_breaker = branch_with_choice("b1", "add_card");
    branch_with_reward_breaker
        .session
        .run_state
        .relics
        .push(RelicState::new(RelicId::QuestionCard));
    let branches = vec![
        branch_with_choice("b0", "add_card"),
        branch_with_reward_breaker,
        branch_with_choice("b2", "skip_card_reward"),
    ];

    let summary =
        pruned_branch_summary_for_selection(&branches, &candidates, &decisions, &keep_indices);

    assert_eq!(
        summary.primary_slot_counts[&BranchRetentionSlotV1::EngineSetup],
        1
    );
    assert_eq!(
        summary.primary_slot_counts[&BranchRetentionSlotV1::Package],
        1
    );
    assert_eq!(
        summary.eligible_slot_counts[&BranchRetentionSlotV1::Diversity],
        1
    );
    assert_eq!(summary.package_state_counts["open:exhaust_engine"], 1);
    assert_eq!(summary.package_state_counts["closed:block_engine"], 1);
    assert_eq!(summary.choice_effect_counts["take_card"], 1);
    assert_eq!(summary.choice_effect_counts["skip_reward"], 1);
    assert_eq!(
        summary.lineage_flag_counts["question_card_reward_count_plus_1"],
        1
    );
}

#[test]
fn branch_choice_effect_key_preserves_bottle_card_effect() {
    assert_eq!(
        branch_experiment_choice_effect_key_v1("bottle_card"),
        "bottle_card"
    );
}

#[test]
fn branch_choice_effect_key_preserves_special_campfire_effects() {
    assert_eq!(branch_experiment_choice_effect_key_v1("dig"), "dig");
    assert_eq!(branch_experiment_choice_effect_key_v1("lift"), "lift");
    assert_eq!(branch_experiment_choice_effect_key_v1("recall"), "recall");
}

#[test]
fn branch_choice_effect_key_preserves_reward_skip_effects() {
    assert_eq!(
        branch_experiment_choice_effect_key_v1("reward_skip_full_potion"),
        "reward_skip_full_potion"
    );
}

#[test]
fn branch_choice_effect_key_preserves_structured_event_effects() {
    assert_eq!(
        branch_experiment_choice_effect_key_v1("event_heal"),
        "event_heal"
    );
    assert_eq!(
        branch_experiment_choice_effect_key_v1("event_gain_relic"),
        "event_gain_relic"
    );
    assert_eq!(
        branch_experiment_choice_effect_key_v1("event_upgrade_card"),
        "event_upgrade_card"
    );
}

#[test]
fn branch_choice_effect_key_preserves_shop_effects() {
    assert_eq!(
        branch_experiment_choice_effect_key_v1("shop_buy_card"),
        "shop_buy_card"
    );
    assert_eq!(
        branch_experiment_choice_effect_key_v1("shop_buy_relic"),
        "shop_buy_relic"
    );
    assert_eq!(
        branch_experiment_choice_effect_key_v1("shop_buy_potion"),
        "shop_buy_potion"
    );
    assert_eq!(
        branch_experiment_choice_effect_key_v1("shop_buy_combo"),
        "shop_buy_combo"
    );
    assert_eq!(
        branch_experiment_choice_effect_key_v1("shop_leave"),
        "shop_leave"
    );
}

#[test]
fn branch_choice_effect_key_preserves_boss_relic_axis() {
    assert_eq!(
        branch_experiment_choice_effect_key_v1("boss_relic:BustedCrown"),
        "boss_relic:BustedCrown"
    );
    assert_eq!(
        branch_experiment_choice_effect_key_v1("boss_relic:TinyHouse"),
        "boss_relic:TinyHouse"
    );
    assert_eq!(
        branch_experiment_choice_effect_key_v1("boss_relic"),
        "boss_relic"
    );
}

#[test]
fn branch_summary_tracks_shop_buy_card_choice_in_trajectory() {
    let session = RunControlSession::new(RunControlConfig::default());
    let choices = vec![BranchExperimentChoiceV1 {
        depth: 0,
        kind: "shop_buy_card".to_string(),
        boundary_title: "Shop".to_string(),
        card: Some(CardId::PommelStrike),
        upgrades: Some(0),
        selected_cards: Vec::new(),
        effect_kind: "shop_buy_card".to_string(),
        effect_key: "shop:shop_buy_card:buy card 0".to_string(),
        effect_label: "Buy Pommel Strike".to_string(),
        representative_count: 1,
        suppressed_count: 0,
        decision_signal: None,
        label: "Buy Pommel Strike".to_string(),
        command: "buy card 0".to_string(),
    }];

    let summary = run_summary(&session, &choices);

    assert_eq!(summary.trajectory.frontload_picks, 1);
    assert_eq!(summary.trajectory.draw_energy_picks, 1);
}

#[test]
fn branch_rank_does_not_hardcode_card_reward_acquisition_bias() {
    let take = branch_with_choice("take", "add_card");
    let skip = branch_with_choice("skip", "skip_card_reward");
    let full_potion_skip = branch_with_choice("potion-skip", "reward_skip_full_potion");

    assert_eq!(branch_rank_key(&take), branch_rank_key(&skip));
    assert_eq!(branch_rank_key(&take), branch_rank_key(&full_potion_skip));
}

#[test]
fn branch_rank_accounts_for_current_removable_curse_purge_debt() {
    let mut clean = branch_with_choice("clean", "event_leave");
    let mut cursed = branch_with_choice("cursed", "event_leave");
    clean.session.run_state.gold = 125;
    cursed.session.run_state.gold = 199;
    cursed
        .session
        .run_state
        .master_deck
        .push(CombatCard::new(CardId::Regret, 90_001));

    assert!(
        branch_rank_key(&clean) > branch_rank_key(&cursed),
        "gold reserved for removing a visible curse should not rank as freely converted value"
    );
}

#[test]
fn branch_rank_values_gold_beyond_current_curse_purge_debt() {
    let mut clean = branch_with_choice("clean", "event_leave");
    let mut cursed = branch_with_choice("cursed", "event_leave");
    clean.session.run_state.gold = 125;
    cursed.session.run_state.gold = 225;
    cursed
        .session
        .run_state
        .master_deck
        .push(CombatCard::new(CardId::Regret, 90_001));

    assert!(
        branch_rank_key(&cursed) > branch_rank_key(&clean),
        "curse debt should reserve purge cost, not erase all extra gold value"
    );
}

#[test]
fn branch_summary_uses_active_combat_visible_hp() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.act_num = 3;
    session.run_state.floor_num = 48;
    session.run_state.current_hp = 64;
    session.run_state.max_hp = 97;
    attach_active_combat_hp(&mut session, 13, 97);

    let summary = run_summary(&session, &[]);

    assert_eq!(summary.hp, 13);
    assert_eq!(summary.max_hp, 97);
}

#[test]
fn branch_rank_uses_active_combat_visible_hp() {
    let mut safer = branch_with_choice("safer", "test");
    let mut dying = branch_with_choice("dying", "test");
    safer.session.run_state.current_hp = 64;
    dying.session.run_state.current_hp = 64;
    attach_active_combat_hp(&mut safer.session, 40, 80);
    attach_active_combat_hp(&mut dying.session, 5, 80);

    assert!(
        branch_rank_key(&safer) > branch_rank_key(&dying),
        "combat branch ranking should use current combat HP, not stale run_state HP"
    );
}

fn branch_with_choice(branch_id: &str, effect_kind: &str) -> BranchWork {
    BranchWork {
        id: branch_id.to_string(),
        session: RunControlSession::new(RunControlConfig::default()),
        choices: vec![BranchExperimentChoiceV1 {
            depth: 0,
            kind: "card_reward".to_string(),
            boundary_title: "Card Reward".to_string(),
            card: Some(CardId::TwinStrike),
            upgrades: Some(0),
            selected_cards: Vec::new(),
            effect_kind: effect_kind.to_string(),
            effect_key: effect_kind.to_string(),
            effect_label: effect_kind.to_string(),
            representative_count: 1,
            suppressed_count: 0,
            decision_signal: None,
            label: effect_kind.to_string(),
            command: "test".to_string(),
        }],
        status: BranchExperimentBranchStatusV1::Active,
        stop_reason: "test".to_string(),
        retention: default_branch_retention_decision_v1(),
        final_boss_combat_record: None,
    }
}

fn attach_active_combat_hp(session: &mut RunControlSession, current_hp: i32, max_hp: i32) {
    let mut combat = blank_test_combat();
    combat.entities.player.current_hp = current_hp;
    combat.entities.player.max_hp = max_hp;
    session.engine_state = EngineState::CombatPlayerTurn;
    session.active_combat = Some(ActiveCombat::new(
        EngineState::CombatPlayerTurn,
        combat,
        CombatContext::Room(RoomCombatContext {
            room_type: RoomType::MonsterRoom,
        }),
    ));
}

fn retention_candidate(
    index: usize,
    trajectory: BranchTrajectorySignatureV1,
) -> BranchRetentionCandidateInputV1 {
    BranchRetentionCandidateInputV1 {
        index,
        act: 1,
        floor: 1,
        frontier_key: "frontier".to_string(),
        rank_key: 0,
        hp: 80,
        max_hp: 80,
        gold: 99,
        deck_count: 10,
        curse_count: 0,
        strategy_formation: None,
        trajectory,
        recent_choice_profiles: Vec::new(),
        choice_profiles: Vec::new(),
        choice_effect_keys: Vec::new(),
        lineage_flags: Vec::new(),
        decision_signals: Vec::new(),
        strategic_debt_tags: Vec::new(),
        startup: Default::default(),
    }
}

#[test]
fn branch_retention_reports_decision_signal_without_rank_consumption() {
    let mut candidate = retention_candidate(0, BranchTrajectorySignatureV1::default());
    candidate.rank_key = 1_000;
    candidate.decision_signals = vec![BranchExperimentChoiceDecisionSignalV1 {
        source: "test_signal".to_string(),
        verdict: "Allow".to_string(),
        tier: 0,
        score: 9_000,
        confidence_milli: 1_000,
        component_net_rank: 9_000,
        acquisition_thesis_rank_adjustment: 0,
        acquisition_thesis_summary: Vec::new(),
    }];

    let adjustment = branch_retention_rank_adjustment_v1(&candidate);

    assert_eq!(adjustment.decision_signal_adjustment, 9_000);
    assert_eq!(adjustment.effective_rank_key, 1_000);
    assert!(adjustment
        .reasons
        .contains(&"decision_signal_component_rank_hint:9000".to_string()));
}

#[test]
fn branch_retention_consumes_card_reward_reject_as_active_rank_penalty() {
    let mut candidate = retention_candidate(0, BranchTrajectorySignatureV1::default());
    candidate.rank_key = 1_000;
    candidate.decision_signals = vec![BranchExperimentChoiceDecisionSignalV1 {
        source: BRANCH_EXPERIMENT_CARD_REWARD_STRATEGIC_TRACE_SIGNAL_SOURCE_V1.to_string(),
        verdict: "Reject".to_string(),
        tier: 5,
        score: -470,
        confidence_milli: 650,
        component_net_rank: -470,
        acquisition_thesis_rank_adjustment: 0,
        acquisition_thesis_summary: Vec::new(),
    }];

    let adjustment = branch_retention_rank_adjustment_v1(&candidate);

    assert_eq!(adjustment.card_reward_plan_adjustment, -1_135);
    assert_eq!(adjustment.effective_rank_key, -135);
    assert!(adjustment
        .reasons
        .contains(&"card_reward_plan_rank_adjustment:-1135".to_string()));
}

#[test]
fn branch_retention_consumes_only_unified_shop_plan_signal() {
    let mut candidate = retention_candidate(0, BranchTrajectorySignatureV1::default());
    candidate.rank_key = 1_000;
    candidate.decision_signals = vec![
        BranchExperimentChoiceDecisionSignalV1 {
            source: "test_signal".to_string(),
            verdict: "Allow".to_string(),
            tier: 999,
            score: 99_999,
            confidence_milli: 1_000,
            component_net_rank: 99_999,
            acquisition_thesis_rank_adjustment: 0,
            acquisition_thesis_summary: Vec::new(),
        },
        BranchExperimentChoiceDecisionSignalV1 {
            source: BRANCH_EXPERIMENT_SHOP_SELECTED_PLAN_SIGNAL_SOURCE_V1.to_string(),
            verdict: "Allow".to_string(),
            tier: 330,
            score: 1_879,
            confidence_milli: 820,
            component_net_rank: 71,
            acquisition_thesis_rank_adjustment: 0,
            acquisition_thesis_summary: Vec::new(),
        },
    ];

    let adjustment = branch_retention_rank_adjustment_v1(&candidate);

    assert_eq!(adjustment.decision_signal_adjustment, 100_070);
    assert_eq!(adjustment.shop_plan_adjustment, 964);
    assert_eq!(adjustment.effective_rank_key, 1_964);
    assert!(adjustment
        .reasons
        .contains(&"shop_plan_rank_adjustment:964".to_string()));
}

#[test]
fn branch_retention_consumes_formation_need_match_as_bounded_rank_input() {
    let mut candidate = retention_candidate(0, BranchTrajectorySignatureV1::default());
    candidate.rank_key = 1_000;
    candidate.strategy_formation = Some(StrategyFormationSummaryV2 {
        stage: StrategyDeckFormationStageV1::PlanSeeded,
        needs: vec![StrategyDeckFormationNeedV1::Block],
        strengths: Vec::new(),
    });
    candidate.recent_choice_profiles = vec![card_reward_semantic_profile_v1(&RewardCard::new(
        CardId::ShrugItOff,
        0,
    ))];
    candidate.choice_profiles = candidate.recent_choice_profiles.clone();

    let adjustment = branch_retention_rank_adjustment_v1(&candidate);

    assert_eq!(adjustment.formation_need_adjustment, 350);
    assert_eq!(adjustment.effective_rank_key, 1_350);
    assert!(adjustment
        .reasons
        .contains(&"formation_context_key:matches_formation_block_need".to_string()));
}

fn trajectory_with_packages(
    setup_keys: &[&str],
    package_keys: &[&str],
) -> BranchTrajectorySignatureV1 {
    BranchTrajectorySignatureV1 {
        setup_keys: setup_keys.iter().map(|key| key.to_string()).collect(),
        package_keys: package_keys.iter().map(|key| key.to_string()).collect(),
        ..BranchTrajectorySignatureV1::default()
    }
}

fn retention_decision(
    primary_slot: BranchRetentionSlotV1,
    slots: &[BranchRetentionSlotV1],
) -> BranchRetentionDecisionV1 {
    BranchRetentionDecisionV1 {
        primary_slot,
        selected_by_slot: None,
        slots: slots.to_vec(),
        reasons: Vec::new(),
        strategic_signature: Default::default(),
        coverage_selection: Default::default(),
        rank_adjustment: Default::default(),
    }
}

#[test]
fn recorded_card_reward_pick_does_not_consume_card_reward_rng() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut reward = RewardState::new();
    reward.pending_card_choice = Some(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::Cleave, 0),
    ]);
    session.engine_state = EngineState::RewardScreen(reward);
    let card_rng_counter_before = session.run_state.rng_pool.card_rng.counter;

    session
        .apply_command(RunControlCommand::RecordedCardRewardPick(0))
        .expect("recorded pick applies");

    assert_eq!(
        session.run_state.rng_pool.card_rng.counter, card_rng_counter_before,
        "card reward choices are generated before the player picks; picking a card must not consume card reward RNG"
    );
}

#[test]
fn branch_lineage_is_privileged_and_not_public_policy_input() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut reward = RewardState::new();
    reward.pending_card_choice = Some(vec![RewardCard::new(CardId::TwinStrike, 0)]);
    session.engine_state = EngineState::RewardScreen(reward);

    let report = run_branch_experiment_from_session(
        session,
        &BranchExperimentConfigV1 {
            max_depth: 0,
            ..BranchExperimentConfigV1::default()
        },
    );

    let lineage = &report.branches[0].frontier.lineage;
    assert_eq!(lineage.visibility, "privileged_simulator_diagnostic");
    assert!(!lineage.public_policy_input);
    assert!(!lineage.direct_pick_consumes_card_rng);
    assert!(lineage.sequence_breakers_present.is_empty());
}

#[test]
fn branch_lineage_reports_reward_sequence_breakers() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.relics.clear();
    session
        .run_state
        .relics
        .push(RelicState::new(RelicId::QuestionCard));
    session
        .run_state
        .relics
        .push(RelicState::new(RelicId::PrayerWheel));
    session
        .run_state
        .relics
        .push(RelicState::new(RelicId::PrismaticShard));
    session
        .run_state
        .relics
        .push(RelicState::new(RelicId::NlothsGift));
    session.run_state.card_upgraded_chance = 0.25;
    let mut reward = RewardState::new();
    reward.pending_card_choice = Some(vec![RewardCard::new(CardId::TwinStrike, 1)]);
    session.engine_state = EngineState::RewardScreen(reward);

    let report = run_branch_experiment_from_session(
        session,
        &BranchExperimentConfigV1 {
            max_depth: 0,
            ..BranchExperimentConfigV1::default()
        },
    );

    let lineage = &report.branches[0].frontier.lineage;
    assert!(lineage
        .reward_count_modifiers
        .contains(&"question_card_reward_count_plus_1".to_string()));
    assert!(lineage
        .reward_count_modifiers
        .contains(&"prayer_wheel_extra_normal_combat_card_reward".to_string()));
    assert!(lineage
        .card_pool_modifiers
        .contains(&"prismatic_shard_any_color_pool".to_string()));
    assert!(lineage
        .rarity_modifiers
        .contains(&"nloths_gift_triple_rare_chance".to_string()));
    assert!(lineage
        .preview_modifiers
        .contains(&"card_upgrade_chance_rng_0.250".to_string()));
    assert_eq!(
        report.frontier_groups[0].lineage_flags,
        lineage.sequence_breakers_present
    );
}

#[test]
fn branch_report_exposes_strategy_formation_summary_used_for_retention() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut reward = RewardState::new();
    reward.pending_card_choice = Some(vec![RewardCard::new(CardId::TwinStrike, 0)]);
    session.engine_state = EngineState::RewardScreen(reward);

    let report = run_branch_experiment_from_session(
        session,
        &BranchExperimentConfigV1 {
            max_depth: 1,
            max_branches: 2,
            auto_max_operations: 0,
            ..BranchExperimentConfigV1::default()
        },
    );

    let summary = &report.branches[0].summary;
    assert_eq!(
        summary.formation_stage,
        StrategyDeckFormationStageV1::StarterShell
    );
    assert!(summary
        .formation_needs
        .contains(&StrategyDeckFormationNeedV1::Frontload));
    assert_eq!(summary.trajectory.frontload_picks, 1);
    assert_eq!(summary.trajectory.transition_frontload_picks, 1);
}

#[test]
fn branch_experiment_settles_after_last_depth_choice() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut reward = RewardState::new();
    reward.pending_card_choice = Some(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::Cleave, 0),
    ]);
    session.engine_state = EngineState::RewardScreen(reward);

    let report = run_branch_experiment_from_session(
        session,
        &BranchExperimentConfigV1 {
            max_depth: 1,
            max_branches: 4,
            auto_max_operations: 0,
            ..BranchExperimentConfigV1::default()
        },
    );

    assert!(
        report
            .branches
            .iter()
            .all(|branch| branch.stop_reason != "card reward branch applied"),
        "depth-exhausted branch results should be settled to a readable frontier, not left at an internal transition"
    );
}

#[test]
fn unresolved_combat_autorun_stop_is_prunable_not_human_strategy() {
    assert!(is_budget_unresolved_combat_boundary(
        "Combat",
        "combat search did not find an executable complete win"
    ));
    assert!(is_budget_unresolved_combat_boundary(
        "Combat",
        "Reason: combat search did not find an executable complete win"
    ));
    assert!(!is_budget_unresolved_combat_boundary(
        "Combat",
        "complete_winning_candidate_exceeds_hp_loss_limit"
    ));
    assert!(!is_budget_unresolved_combat_boundary(
        "Card Reward",
        "combat search did not find an executable complete win"
    ));
}

#[test]
fn combat_turn_segment_progress_is_continuable_not_prunable() {
    assert!(is_combat_turn_segment_progress_boundary(
        "Combat",
        "combat turn segment progressed; continue next campaign round"
    ));
    assert!(!is_budget_unresolved_combat_boundary(
        "Combat",
        "combat turn segment progressed; continue next campaign round"
    ));
    assert!(!is_combat_turn_segment_progress_boundary(
        "Card Reward",
        "combat turn segment progressed; continue next campaign round"
    ));
}

#[test]
fn final_boss_record_keeps_last_terminal_victory_combat_trajectory() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.engine_state = EngineState::GameOver(RunResult::Victory);
    let annotations = vec![
        RunControlTraceAnnotationV1::CombatAutomationTrajectory {
            source: "earlier_combat".to_string(),
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
        RunControlTraceAnnotationV1::CombatAutomationTrajectory {
            source: "final_boss_combat".to_string(),
            action_count: 2,
            actions: vec![
                crate::eval::run_control::CombatAutomationActionV1 {
                    step_index: 0,
                    action_key: "play Bash".to_string(),
                    input: crate::state::core::ClientInput::PlayCard {
                        card_index: 0,
                        target: Some(0),
                    },
                    drawn_cards: Vec::new(),
                    combat_after: None,
                },
                crate::eval::run_control::CombatAutomationActionV1 {
                    step_index: 1,
                    action_key: "end".to_string(),
                    input: crate::state::core::ClientInput::EndTurn,
                    drawn_cards: Vec::new(),
                    combat_after: None,
                },
            ],
            label_role: "behavior_policy_not_teacher".to_string(),
        },
    ];

    let record = final_boss_combat_record_from_annotations_v1(&session, &annotations)
        .expect("terminal victory should keep the final boss combat trajectory");

    assert_eq!(record.source, "final_boss_combat");
    assert_eq!(record.action_count, 2);
    assert_eq!(record.actions.len(), 2);
    assert_eq!(record.label_role, "behavior_policy_not_teacher");
}

#[test]
fn final_boss_record_is_absent_without_terminal_victory() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.engine_state = EngineState::CombatPlayerTurn;
    let annotations = vec![RunControlTraceAnnotationV1::CombatAutomationTrajectory {
        source: "ordinary_combat".to_string(),
        action_count: 1,
        actions: Vec::new(),
        label_role: "behavior_policy_not_teacher".to_string(),
    }];

    assert!(final_boss_combat_record_from_annotations_v1(&session, &annotations).is_none());
}

#[test]
fn branch_experiment_prunes_to_max_branches() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut reward = RewardState::new();
    reward.pending_card_choice = Some(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::Cleave, 0),
        RewardCard::new(CardId::ShrugItOff, 0),
    ]);
    session.engine_state = EngineState::RewardScreen(reward);

    let report = run_branch_experiment_from_session(
        session,
        &BranchExperimentConfigV1 {
            max_depth: 1,
            max_branches: 2,
            ..BranchExperimentConfigV1::default()
        },
    );

    assert!(report.branch_limit_hit);
    assert_eq!(report.branches.len(), 2);
}

#[test]
fn branch_experiment_caps_same_frontier_group_variants() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut reward = RewardState::new();
    reward.pending_card_choice = Some(vec![
        RewardCard::new(CardId::TwinStrike, 0),
        RewardCard::new(CardId::Cleave, 0),
    ]);
    session.engine_state = EngineState::RewardScreen(reward);

    let report = run_branch_experiment_from_session(
        session,
        &BranchExperimentConfigV1 {
            max_depth: 1,
            max_branches: 4,
            max_branches_per_frontier_group: Some(1),
            auto_max_operations: 0,
            ..BranchExperimentConfigV1::default()
        },
    );

    assert!(report.frontier_group_limit_hit);
    assert_eq!(report.pruned_branch_count, 1);
    assert_eq!(report.branches.len(), 1);
    assert_eq!(report.frontier_groups.len(), 1);
}

fn write_trace_fixture(label: &str, trace: &crate::eval::run_control::SessionTraceV1) -> PathBuf {
    let path = unique_temp_path(label).join("trace.json");
    fs::create_dir_all(path.parent().unwrap()).expect("trace parent should exist");
    fs::write(
        &path,
        serde_json::to_string_pretty(trace).expect("trace should serialize"),
    )
    .expect("trace fixture should write");
    path
}

fn unique_temp_path(label: &str) -> PathBuf {
    let mut path = std::env::temp_dir();
    path.push(format!(
        "sts_simulator_{label}_{}_{}",
        std::process::id(),
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("time should be available")
            .as_nanos()
    ));
    path
}
