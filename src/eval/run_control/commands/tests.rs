use std::path::PathBuf;

use crate::ai::combat_search_v2::{
    CombatSearchV2ChildRolloutPolicy, CombatSearchV2FrontierPolicy, CombatSearchV2PotionPolicy,
    CombatSearchV2RolloutPolicy, CombatSearchV2SetupBiasPolicy, CombatSearchV2TurnPlanPolicy,
};

use super::super::reward_auto::RewardAutomationTarget;
use super::*;

#[test]
fn run_control_parser_accepts_capture_label() {
    let parsed = parse_run_control_command("capture captures/jaw.json jaw worm start")
        .expect("capture command should parse");

    assert_eq!(
        parsed,
        RunControlCommand::Capture {
            path: PathBuf::from("captures/jaw.json"),
            label: Some("jaw worm start".to_string()),
        }
    );
}

#[test]
fn run_control_parser_accepts_case_artifact_commands() {
    assert_eq!(
        parse_run_control_command("capture-case data/bench case_a first fight")
            .expect("capture-case should parse"),
        RunControlCommand::CaptureCase {
            root: PathBuf::from("data/bench"),
            case_id: "case_a".to_string(),
            label: Some("first fight".to_string()),
        }
    );
    assert_eq!(
        parse_run_control_command("cap case_a first fight").expect("cap should parse"),
        RunControlCommand::CaptureCaseDefault {
            case_id: "case_a".to_string(),
            label: Some("first fight".to_string()),
        }
    );
    assert_eq!(
        parse_run_control_command("save-baseline-case data/bench case_a")
            .expect("save-baseline-case should parse"),
        RunControlCommand::SaveBaselineCase {
            root: PathBuf::from("data/bench"),
            case_id: "case_a".to_string(),
        }
    );
    assert_eq!(
        parse_run_control_command("baseline").expect("baseline should parse"),
        RunControlCommand::SaveBaselineForLastCaptureCase
    );
    assert_eq!(
        parse_run_control_command("b").expect("b should parse"),
        RunControlCommand::SaveBaselineForLastCaptureCase
    );
    assert_eq!(
        parse_run_control_command("bench-add data/bench case_a").expect("bench-add should parse"),
        RunControlCommand::RegisterBenchmarkCase {
            root: PathBuf::from("data/bench"),
            case_id: "case_a".to_string(),
        }
    );
}

#[test]
fn run_control_parser_accepts_recorded_card_reward_pick() {
    assert_eq!(
        parse_run_control_command("rp 1").expect("rp should parse"),
        RunControlCommand::RecordedCardRewardPick(1)
    );
    assert_eq!(
        parse_run_control_command("record-pick 2").expect("record-pick should parse"),
        RunControlCommand::RecordedCardRewardPick(2)
    );
}

#[test]
fn run_control_parser_accepts_search_combat_options() {
    assert_eq!(
            parse_run_control_command(
                "search-combat max_nodes=123 wall_ms=50 max_hp_loss=12 potion=semantic max_potions=1 rollout=turn_beam_no_potion child_rollout=immediate rollouts=7 rollout_actions=11 beam=4 turn_plan=root_frontier_seed frontier=round_robin setup_bias=key_card_online",
            )
            .expect("search-combat should parse"),
            RunControlCommand::SearchCombat(RunControlSearchCombatOptions {
                profile: None,
                max_nodes: Some(123),
                max_actions_per_line: None,
                max_engine_steps_per_action: None,
                wall_ms: Some(50),
                max_hp_loss: Some(RunControlHpLossLimit::Limit(12)),
                potion_policy: Some(CombatSearchV2PotionPolicy::SemanticBudgeted),
                max_potions_used: Some(1),
                rollout_policy: Some(CombatSearchV2RolloutPolicy::TurnBeamNoPotion),
                child_rollout_policy: Some(CombatSearchV2ChildRolloutPolicy::Immediate),
                rollout_max_evaluations: Some(7),
                rollout_max_actions: Some(11),
                rollout_beam_width: Some(4),
                turn_plan_policy: Some(CombatSearchV2TurnPlanPolicy::RootFrontierSeed),
                frontier_policy: Some(CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets),
                phase_guard_policy: None,
                setup_bias_policy: Some(CombatSearchV2SetupBiasPolicy::KeyCardOnline),
                segment_mode: None,
                disable_no_win_rescue: false,
                allow_smoke_bomb_survival_fallback: false,
            })
        );
    assert_eq!(
        parse_run_control_command("sc").expect("sc should parse"),
        RunControlCommand::SearchCombat(RunControlSearchCombatOptions::default())
    );
    let error = parse_run_control_command("sc save=case")
        .expect_err("standalone search evidence is retired");
    assert!(error.contains("unknown search-combat option 'save'"));
    assert_eq!(
        parse_run_control_command("sc segment=turn").expect("segment option should parse"),
        RunControlCommand::SearchCombat(RunControlSearchCombatOptions {
            segment_mode: Some(RunControlCombatSegmentMode::TurnBoundary),
            ..Default::default()
        })
    );
    assert_eq!(
        parse_run_control_command("sc segment=non_boss_turn")
            .expect("non-boss segment option should parse"),
        RunControlCommand::SearchCombat(RunControlSearchCombatOptions {
            segment_mode: Some(RunControlCombatSegmentMode::NonBossTurnBoundary),
            ..Default::default()
        })
    );
}

#[test]
fn run_control_parser_accepts_search_default_settings() {
    assert_eq!(
        parse_run_control_command("search-defaults").expect("search defaults status should parse"),
        RunControlCommand::SearchDefaults(RunControlSearchDefaultsCommand::Status)
    );
    assert_eq!(
        parse_run_control_command(
            "sd max_nodes=123 wall_ms=50 max_hp_loss=12 potion=semantic max_potions=1",
        )
        .expect("search defaults update should parse"),
        RunControlCommand::SearchDefaults(RunControlSearchDefaultsCommand::Update(
            RunControlSearchCombatOptions {
                max_nodes: Some(123),
                wall_ms: Some(50),
                max_hp_loss: Some(RunControlHpLossLimit::Limit(12)),
                potion_policy: Some(CombatSearchV2PotionPolicy::SemanticBudgeted),
                max_potions_used: Some(1),
                ..Default::default()
            }
        ))
    );
    assert_eq!(
        parse_run_control_command("sd clear").expect("search defaults clear should parse"),
        RunControlCommand::SearchDefaults(RunControlSearchDefaultsCommand::Clear)
    );
    assert!(
        parse_run_control_command("sd rollout=turn_beam_no_potion").is_err(),
        "sd should reject search-combat fields that are not stored as session defaults"
    );
}

#[test]
fn run_control_parser_accepts_auto_step_options() {
    assert_eq!(
        parse_run_control_command("n").expect("n should parse"),
        RunControlCommand::AutoStep(RunControlAutoStepOptions::default())
    );
    assert_eq!(
        parse_run_control_command("advance-to-human-boundary")
            .expect("long advance command should parse"),
        RunControlCommand::AutoStep(RunControlAutoStepOptions::default())
    );
    assert_eq!(
        parse_run_control_command("auto-step max_nodes=123 wall_ms=50 max_ops=9")
            .expect("auto-step should parse"),
        RunControlCommand::AutoStep(RunControlAutoStepOptions {
            search: RunControlSearchCombatOptions {
                profile: None,
                max_nodes: Some(123),
                max_actions_per_line: None,
                max_engine_steps_per_action: None,
                wall_ms: Some(50),
                max_hp_loss: None,
                potion_policy: None,
                max_potions_used: None,
                rollout_policy: None,
                child_rollout_policy: None,
                rollout_max_evaluations: None,
                rollout_max_actions: None,
                rollout_beam_width: None,
                turn_plan_policy: None,
                frontier_policy: None,
                phase_guard_policy: None,
                setup_bias_policy: None,
                segment_mode: None,
                disable_no_win_rescue: false,
                allow_smoke_bomb_survival_fallback: false,
            },
            max_operations: Some(9),
            route: RunControlRouteAutomationMode::Manual,
        })
    );
    assert_eq!(
        parse_run_control_command("auto-step route=planner max_ops=9")
            .expect("auto-step route planner should parse"),
        RunControlCommand::AutoStep(RunControlAutoStepOptions {
            search: RunControlSearchCombatOptions::default(),
            max_operations: Some(9),
            route: RunControlRouteAutomationMode::Planner,
        })
    );
    assert_eq!(
        parse_run_control_command("nr max_ops=9 max_hp_loss=8").expect("nr should parse"),
        RunControlCommand::AutoStep(RunControlAutoStepOptions {
            search: RunControlSearchCombatOptions {
                max_hp_loss: Some(RunControlHpLossLimit::Limit(8)),
                ..Default::default()
            },
            max_operations: Some(9),
            route: RunControlRouteAutomationMode::Planner,
        })
    );
    assert!(
        parse_run_control_command("nr route=manual").is_err(),
        "nr should not silently accept conflicting route mode"
    );
    assert_eq!(
        parse_run_control_command("sc max_hp_loss=off").expect("hp-loss gate off should parse"),
        RunControlCommand::SearchCombat(RunControlSearchCombatOptions {
            max_hp_loss: Some(RunControlHpLossLimit::Unlimited),
            ..Default::default()
        })
    );
    assert_eq!(
        parse_run_control_command("nr turn_plan=off").expect("disabled turn-plan should parse"),
        RunControlCommand::AutoStep(RunControlAutoStepOptions {
            route: RunControlRouteAutomationMode::Planner,
            search: RunControlSearchCombatOptions {
                turn_plan_policy: Some(CombatSearchV2TurnPlanPolicy::Disabled),
                ..Default::default()
            },
            ..Default::default()
        })
    );
    assert_eq!(
        parse_run_control_command("nr turn_plan=turn_boundary_frontier_seed frontier=round_robin")
            .expect("turn-boundary auto search policy should parse"),
        RunControlCommand::AutoStep(RunControlAutoStepOptions {
            route: RunControlRouteAutomationMode::Planner,
            search: RunControlSearchCombatOptions {
                turn_plan_policy: Some(CombatSearchV2TurnPlanPolicy::TurnBoundaryFrontierSeed),
                frontier_policy: Some(CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets),
                ..Default::default()
            },
            ..Default::default()
        })
    );
    assert_eq!(
        parse_run_control_command("nr turn_plan=tactical_seed")
            .expect("tactical-gated turn-plan policy should parse"),
        RunControlCommand::AutoStep(RunControlAutoStepOptions {
            route: RunControlRouteAutomationMode::Planner,
            search: RunControlSearchCombatOptions {
                turn_plan_policy: Some(
                    CombatSearchV2TurnPlanPolicy::TacticalEnemyTurnBoundaryFrontierSeed
                ),
                ..Default::default()
            },
            ..Default::default()
        })
    );
}

#[test]
fn run_control_parser_accepts_auto_run_options() {
    assert_eq!(
        parse_run_control_command("auto-run max_ops=33 wall_ms=100")
            .expect("auto-run should parse"),
        RunControlCommand::AutoRun(RunControlAutoStepOptions {
            search: RunControlSearchCombatOptions {
                wall_ms: Some(100),
                ..Default::default()
            },
            max_operations: Some(33),
            route: RunControlRouteAutomationMode::Planner,
        })
    );
    assert_eq!(
        parse_run_control_command("ar").expect("ar should parse"),
        RunControlCommand::AutoRun(RunControlAutoStepOptions {
            route: RunControlRouteAutomationMode::Planner,
            ..Default::default()
        })
    );
    assert!(
        parse_run_control_command("auto-run route=manual").is_err(),
        "auto-run should not silently accept conflicting route mode"
    );
}

#[test]
fn run_control_parser_accepts_auto_reward_settings() {
    assert_eq!(
        parse_run_control_command("auto-reward").expect("auto-reward should parse"),
        RunControlCommand::RewardAutomationStatus
    );
    assert_eq!(
        parse_run_control_command("auto-reward potion off")
            .expect("auto-reward setting should parse"),
        RunControlCommand::SetRewardAutomation {
            target: RewardAutomationTarget::Potion,
            enabled: false,
        }
    );
    assert_eq!(
        parse_run_control_command("auto-reward relic off")
            .expect("auto-reward relic setting should parse"),
        RunControlCommand::SetRewardAutomation {
            target: RewardAutomationTarget::Relic,
            enabled: false,
        }
    );
}

#[test]
fn run_control_parser_accepts_visible_non_numeric_ids() {
    assert_eq!(
        parse_run_control_command("card-2").expect("shop card id should parse"),
        RunControlCommand::Candidate("card-2".to_string())
    );
    assert_eq!(
        parse_run_control_command("Card-2").expect("case-insensitive shop card id should parse"),
        RunControlCommand::Candidate("Card-2".to_string())
    );
    assert_eq!(
        parse_run_control_command("relic-1").expect("shop relic id should parse"),
        RunControlCommand::Candidate("relic-1".to_string())
    );
    assert_eq!(
        parse_run_control_command("potion-0").expect("shop potion id should parse"),
        RunControlCommand::Candidate("potion-0".to_string())
    );
    assert_eq!(
        parse_run_control_command("smith-8").expect("campfire smith id should parse"),
        RunControlCommand::Candidate("smith-8".to_string())
    );
    assert_eq!(
        parse_run_control_command("leave").expect("leave id should parse"),
        RunControlCommand::Candidate("leave".to_string())
    );
    assert_eq!(
        parse_run_control_command("rewards").expect("pending reward overlay id should parse"),
        RunControlCommand::Candidate("rewards".to_string())
    );
    assert_eq!(
        parse_run_control_command("purge").expect("purge candidate should parse"),
        RunControlCommand::Candidate("purge".to_string())
    );
}

#[test]
fn run_control_parser_accepts_contextual_shop_words() {
    assert_eq!(
        parse_run_control_command("card 2").expect("card index should parse"),
        RunControlCommand::CardIndex(2)
    );
    assert_eq!(
        parse_run_control_command("relic 1").expect("relic index should parse"),
        RunControlCommand::RelicIndex(1)
    );
}

#[test]
fn run_control_parser_accepts_view_commands() {
    assert_eq!(
        parse_run_control_command("h").expect("h should parse"),
        RunControlCommand::Help
    );
    assert_eq!(
        parse_run_control_command("").expect("enter should parse"),
        RunControlCommand::DefaultCandidate
    );
    assert_eq!(
        parse_run_control_command("0").expect("candidate id should parse"),
        RunControlCommand::Candidate("0".to_string())
    );
    assert_eq!(
        parse_run_control_command("deck").expect("deck should parse"),
        RunControlCommand::Deck
    );
    assert_eq!(
        parse_run_control_command("mf").expect("mf should parse"),
        RunControlCommand::MapFull
    );
    assert_eq!(
        parse_run_control_command("ms").expect("ms should parse"),
        RunControlCommand::MapSummary
    );
    assert_eq!(
        parse_run_control_command("map-summary").expect("map-summary should parse"),
        RunControlCommand::MapSummary
    );
    assert_eq!(
        parse_run_control_command("map full").expect("map full should parse"),
        RunControlCommand::MapFull
    );
    assert_eq!(
        parse_run_control_command("map").expect("map should parse"),
        RunControlCommand::Map
    );
    assert_eq!(
        parse_run_control_command("bd").expect("bd should parse"),
        RunControlCommand::BoundaryRecord
    );
    assert_eq!(
        parse_run_control_command("boundary").expect("boundary should parse"),
        RunControlCommand::BoundaryRecord
    );
    assert_eq!(
        parse_run_control_command("boundary-record").expect("boundary-record should parse"),
        RunControlCommand::BoundaryRecord
    );
    assert_eq!(
        parse_run_control_command("rs").expect("rs should parse"),
        RunControlCommand::RouteSuggest
    );
    assert_eq!(
        parse_run_control_command("route-suggest").expect("route-suggest should parse"),
        RunControlCommand::RouteSuggest
    );
    assert_eq!(
        parse_run_control_command("rg").expect("rg should parse"),
        RunControlCommand::RouteGo
    );
    assert_eq!(
        parse_run_control_command("route-go").expect("route-go should parse"),
        RunControlCommand::RouteGo
    );
    assert_eq!(
        parse_run_control_command("d").expect("d should parse"),
        RunControlCommand::Details
    );
    assert_eq!(
        parse_run_control_command("raw").expect("raw should parse"),
        RunControlCommand::Raw
    );
    assert_eq!(
        parse_run_control_command("case").expect("case should parse"),
        RunControlCommand::SaveDecisionCase { path: None }
    );
    assert_eq!(
        parse_run_control_command("select").expect("empty select should parse"),
        RunControlCommand::SelectionIndices(Vec::new())
    );
    assert_eq!(
        parse_run_control_command("select 2 4").expect("selection indices should parse"),
        RunControlCommand::SelectionIndices(vec![2, 4])
    );
}
