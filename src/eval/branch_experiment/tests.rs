use super::*;
use std::collections::BTreeSet;

use crate::ai::noncombat_strategy_v1::{StrategyDeckFormationNeedV1, StrategyDeckFormationStageV1};
use crate::content::cards::CardId;
use crate::content::relics::RelicId;
use crate::content::relics::RelicState;
use crate::eval::branch_experiment_retention::BranchRetentionBudgetProfileV1;
use crate::state::core::{RunPendingChoiceReason, RunPendingChoiceState};
use crate::state::events::{EventId, EventState};
use crate::state::rewards::{BossRelicChoiceState, RewardState};
use std::fs;
use std::path::PathBuf;

#[test]
fn branch_experiment_schema_version_tracks_lineage_pruned_summary() {
    assert_eq!(BRANCH_EXPERIMENT_SCHEMA_VERSION, 17);
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
fn branch_experiment_retention_preserves_card_reward_skip_effect() {
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
            .any(|choice| choice.effect_kind == "skip_card_reward")
    }));
    assert!(report.pruned_branch_summary.choice_effect_counts["take_card"] >= 1);
}

#[test]
fn branch_experiment_retention_preserves_singing_bowl_effect() {
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
    assert!(report.branches.iter().any(|branch| {
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
        .any(|option| option.semantic_class == "pure_transition_frontload"));
}

#[test]
fn branch_experiment_expands_campfire_choices() {
    let mut session = RunControlSession::new(RunControlConfig::default());
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
    assert!(report
        .branches
        .iter()
        .any(|branch| branch.choices.iter().any(|choice| {
            choice.kind == "campfire" && choice.command == "rest" && choice.card.is_none()
        })));
    assert!(report
        .branches
        .iter()
        .any(|branch| branch.choices.iter().any(|choice| {
            choice.kind == "campfire"
                && choice.command.starts_with("smith ")
                && choice.card.is_some()
        })));
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
    assert_eq!(choices.len(), 3);
    assert!(choices.iter().all(|choice| {
        choice.kind == "event" && choice.card.is_none() && choice.upgrades.is_none()
    }));
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
    assert_eq!(report.branches.len(), 3);
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
        vec![
            ("remove Strike", 5, 4),
            ("remove Defend", 4, 3),
            ("remove Bash", 1, 0),
        ]
    );
}

#[test]
fn branch_experiment_can_chain_event_option_into_run_selection_choices() {
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
        branch.choices.len() == 2
            && branch.choices[0].kind == "event"
            && branch.choices[0].command == "event 0"
            && branch.choices[1].kind == "run_selection"
            && branch.choices[1].command.starts_with("select ")
    }));
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

fn branch_with_choice(branch_id: &str, effect_kind: &str) -> BranchWork {
    BranchWork {
        id: branch_id.to_string(),
        session: RunControlSession::new(RunControlConfig::default()),
        choices: vec![BranchExperimentChoiceV1 {
            depth: 0,
            kind: "card_reward".to_string(),
            card: Some(CardId::TwinStrike),
            upgrades: Some(0),
            selected_cards: Vec::new(),
            effect_kind: effect_kind.to_string(),
            effect_key: effect_kind.to_string(),
            effect_label: effect_kind.to_string(),
            representative_count: 1,
            suppressed_count: 0,
            label: effect_kind.to_string(),
            command: "test".to_string(),
        }],
        status: BranchExperimentBranchStatusV1::Active,
        stop_reason: "test".to_string(),
        retention: default_branch_retention_decision_v1(),
    }
}

fn retention_candidate(
    index: usize,
    trajectory: BranchTrajectorySignatureV1,
) -> BranchRetentionCandidateInputV1 {
    BranchRetentionCandidateInputV1 {
        index,
        frontier_key: "frontier".to_string(),
        rank_key: 0,
        hp: 80,
        max_hp: 80,
        gold: 99,
        deck_count: 10,
        strategy_formation: None,
        trajectory,
        choice_profiles: Vec::new(),
        choice_effect_keys: Vec::new(),
        lineage_flags: Vec::new(),
    }
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
