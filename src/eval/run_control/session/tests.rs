use super::*;
use crate::content::monsters::factory::EncounterId;
use crate::eval::run_control::decision_surface;
use crate::eval::run_control::registry::BenchmarkCasePaths;
use crate::eval::run_control::{
    parse_run_control_command, render_run_control_details, render_run_control_state,
    CombatBaselineOutcomeV1, RunControlCommand, RunControlHpLossLimit,
    RunControlSearchCombatOptions, RunControlSearchDefaultsCommand,
};
use crate::state::core::ClientInput;
use crate::state::map::node::{MapEdge, MapRoomNode, RoomType};
use crate::state::map::state::MapState;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn search_defaults_command_updates_and_clears_session_defaults() {
    let mut session = RunControlSession::new(RunControlConfig::default());

    let outcome = session
        .apply_command(RunControlCommand::SearchDefaults(
            RunControlSearchDefaultsCommand::Update(RunControlSearchCombatOptions {
                max_nodes: Some(123),
                wall_ms: Some(50),
                max_hp_loss: Some(RunControlHpLossLimit::Limit(12)),
                potion_policy: Some(crate::ai::combat_search_v2::CombatSearchV2PotionPolicy::Never),
                max_potions_used: Some(0),
                ..Default::default()
            }),
        ))
        .expect("search defaults update should apply");

    assert!(outcome.message.contains("search defaults"));
    assert_eq!(session.search_max_nodes, Some(123));
    assert_eq!(session.search_wall_ms, Some(50));
    assert_eq!(session.search_max_hp_loss, Some(12));
    assert_eq!(
        session.search_potion_policy,
        Some(crate::ai::combat_search_v2::CombatSearchV2PotionPolicy::Never)
    );
    assert_eq!(session.search_max_potions_used, Some(0));

    session
        .apply_command(RunControlCommand::SearchDefaults(
            RunControlSearchDefaultsCommand::Clear,
        ))
        .expect("search defaults clear should apply");

    assert_eq!(session.search_max_nodes, None);
    assert_eq!(session.search_wall_ms, None);
    assert_eq!(session.search_max_hp_loss, None);
    assert_eq!(session.search_potion_policy, None);
    assert_eq!(session.search_max_potions_used, None);
}

#[test]
fn run_control_session_checkpoint_round_trips_exact_state() {
    let mut session = RunControlSession::new(RunControlConfig {
        seed: 590093712,
        search_max_nodes: Some(12_345),
        search_wall_ms: Some(67),
        ..RunControlConfig::default()
    });
    session
        .apply_command(RunControlCommand::DefaultCandidate)
        .expect("default Neow intro candidate should apply");

    let checkpoint = RunControlSessionCheckpointV1::from_session(&session);
    let text = serde_json::to_string(&checkpoint).expect("checkpoint should serialize");
    let loaded: RunControlSessionCheckpointV1 =
        serde_json::from_str(&text).expect("checkpoint should deserialize");
    let restored = loaded.into_session().expect("checkpoint should restore");

    assert_eq!(restored.engine_state, session.engine_state);
    assert_eq!(restored.run_state, session.run_state);
    assert_eq!(restored.active_combat, session.active_combat);
    assert_eq!(restored.decision_step, session.decision_step);
    assert_eq!(restored.search_max_nodes, session.search_max_nodes);
    assert_eq!(restored.search_wall_ms, session.search_wall_ms);
}

#[test]
fn run_control_session_checkpoint_preserves_map_traversal_edges() {
    let mut session = RunControlSession::new(RunControlConfig {
        seed: 1_800_564_075,
        ..RunControlConfig::default()
    });
    let (current_x, current_y, target_x, target_y) = session
        .run_state
        .map
        .graph
        .iter()
        .enumerate()
        .flat_map(|(y, row)| {
            row.iter().filter_map(move |node| {
                node.edges
                    .iter()
                    .next()
                    .map(|edge| (node.x, y as i32, edge.dst_x, edge.dst_y))
            })
        })
        .next()
        .expect("generated map should have at least one traversable edge");
    session.run_state.map.current_x = current_x;
    session.run_state.map.current_y = current_y;
    session.run_state.event_state = None;
    session.engine_state = EngineState::MapNavigation;

    let checkpoint = RunControlSessionCheckpointV1::from_session(&session);
    let text = serde_json::to_string(&checkpoint).expect("checkpoint should serialize");
    let loaded: RunControlSessionCheckpointV1 =
        serde_json::from_str(&text).expect("checkpoint should deserialize");
    let restored = loaded.into_session().expect("checkpoint should restore");

    assert!(
        restored
            .run_state
            .map
            .can_travel_to(target_x, target_y, false),
        "checkpoint restore must preserve map edges needed for resumed route planning"
    );
}

#[test]
fn run_control_capture_command_saves_active_combat_position() {
    let mut session = test_session_with_first_monster_room();
    session
        .apply_command(RunControlCommand::Input(ClientInput::SelectMapNode(0)))
        .expect("map input should enter combat");
    assert!(matches!(
        session.engine_state,
        EngineState::CombatPlayerTurn
    ));

    let dir = unique_temp_dir("run_control_capture");
    fs::create_dir_all(&dir).expect("temp dir should be created");
    let path = dir.join("capture.json");
    let outcome = session
        .apply_command(RunControlCommand::Capture {
            path: path.clone(),
            label: Some("first fight".to_string()),
        })
        .expect("capture command should save");

    assert!(outcome.message.contains("saved CombatCaptureV1"));
    let loaded = crate::eval::combat_capture::load_combat_capture_v1(&path)
        .expect("saved capture should load");
    assert_eq!(loaded.label.as_deref(), Some("first fight"));
    assert_eq!(
        loaded.provenance.source_kind,
        crate::eval::artifact::ArtifactSourceKind::ManualRunControl
    );
    assert_eq!(
        loaded.provenance.capture_method,
        "run_control_manual_capture"
    );
    assert_eq!(loaded.source.capture_method, "run_control_manual_capture");
    assert_eq!(
        loaded
            .provenance
            .run_config
            .as_ref()
            .and_then(|config| config.seed),
        Some(session.run_state.seed)
    );
    assert!(loaded.fingerprints.is_some());
    assert!(loaded.legal_actions.is_some());
    assert!(matches!(
        loaded.position.engine,
        EngineState::CombatPlayerTurn
    ));

    let _ = fs::remove_file(path);
    let _ = fs::remove_dir(dir);
}

#[test]
fn run_control_capture_case_registers_benchmark_manifest() {
    let mut session = test_session_with_first_monster_room();
    session
        .apply_command(RunControlCommand::Input(ClientInput::SelectMapNode(0)))
        .expect("map input should enter combat");

    let root = unique_temp_dir("run_control_capture_case");
    let outcome = session
        .apply_command(RunControlCommand::CaptureCase {
            root: root.clone(),
            case_id: "first_fight".to_string(),
            label: Some("first fight".to_string()),
        })
        .expect("capture-case should save and register");

    assert!(outcome.message.contains("registered"));
    let paths = BenchmarkCasePaths::for_case(&root, "first_fight");
    assert!(paths.capture_path.exists());
    assert!(paths.benchmark_manifest.exists());
    let manifest = fs::read_to_string(&paths.benchmark_manifest).expect("manifest readable");
    assert!(manifest.contains("\"combat_snapshot\": \"captures/first_fight.capture.json\""));
    assert!(manifest.contains("\"expected_fingerprints\""));
    crate::eval::combat_search_v2::load_combat_search_v2_benchmark(&paths.benchmark_manifest)
        .expect("registered suite should validate through search benchmark loader");
    assert_eq!(
        session
            .last_capture_case()
            .map(|case| (case.root.clone(), case.case_id.clone())),
        Some((root.clone(), "first_fight".to_string()))
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn run_control_baseline_command_rejects_search_resolved_combat() {
    let mut session = test_session_with_first_monster_room();
    session
        .apply_command(RunControlCommand::Input(ClientInput::SelectMapNode(0)))
        .expect("map input should enter combat");

    let root = unique_temp_dir("run_control_baseline_last");
    session
        .apply_command(RunControlCommand::CaptureCase {
            root: root.clone(),
            case_id: "first_fight".to_string(),
            label: None,
        })
        .expect("capture-case should remember the case");
    session
        .apply_command(RunControlCommand::SearchCombat(
            crate::eval::run_control::RunControlSearchCombatOptions {
                max_nodes: Some(2_000),
                wall_ms: Some(5_000),
                ..Default::default()
            },
        ))
        .expect("search-combat should finish the captured combat");
    assert!(session.last_completed_combat_matches_capture_case());
    assert!(!session.last_completed_manual_combat_matches_capture_case());

    let err = session
        .apply_command(RunControlCommand::SaveBaselineForLastCaptureCase)
        .expect_err("search-combat outcome should not save as human baseline");

    assert!(err.contains("resolved by search-combat"));
    let paths = BenchmarkCasePaths::for_case(&root, "first_fight");
    assert!(!paths.baseline_path.exists());
    let manifest = fs::read_to_string(&paths.benchmark_manifest).expect("manifest readable");
    assert!(!manifest.contains("\"baseline\""));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn run_control_search_combat_can_save_search_evidence_for_capture_case() {
    let mut session = test_session_with_first_monster_room();
    session
        .apply_command(RunControlCommand::Input(ClientInput::SelectMapNode(0)))
        .expect("map input should enter combat");

    let root = unique_temp_dir("run_control_search_evidence");
    session
        .apply_command(RunControlCommand::CaptureCase {
            root: root.clone(),
            case_id: "first_fight".to_string(),
            label: None,
        })
        .expect("capture-case should remember the case");
    let decision_step = session.decision_step;

    let outcome = session
        .apply_command(RunControlCommand::SearchCombat(
            crate::eval::run_control::RunControlSearchCombatOptions {
                max_nodes: Some(2_000),
                wall_ms: Some(5_000),
                evidence: Some(
                    crate::eval::run_control::RunControlSearchEvidenceTarget::LastCaptureCase,
                ),
                ..Default::default()
            },
        ))
        .expect("search-combat should finish and save evidence");

    assert!(outcome.message.contains("Search evidence saved"));
    assert!(outcome
        .message
        .contains("information_access=privileged_simulator public_safe=false"));
    let evidence_path = root
        .join("search_evidence")
        .join(format!("first_fight.step{decision_step}.search.json"));
    let payload = fs::read_to_string(&evidence_path).expect("search evidence should exist");
    assert!(payload.contains("\"schema_name\": \"CombatSearchEvidenceV1\""));
    assert!(payload.contains("\"label_role\": \"search_evidence_not_human_baseline\""));
    assert!(payload.contains("\"capture_case_id\": \"first_fight\""));
    assert!(payload.contains("\"capture_path\":"));
    assert!(payload.contains("first_fight.capture.json"));
    assert!(payload.contains("\"schema_name\": \"CombatSearchV2Report\""));
    assert!(payload.contains("\"policy_evidence\":"));
    assert!(payload.contains("\"information_access\": \"privileged_simulator\""));
    assert!(payload.contains("\"public_safe\": false"));
    assert!(payload.contains("\"privileged_simulator_state\""));
    assert!(payload.contains("\"exact_rng_state\""));
    crate::eval::run_control::load_combat_search_evidence_v1(&evidence_path)
        .expect("search evidence should validate");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn run_control_capture_command_rejects_map_state() {
    let session = test_session_after_neow_at_map();

    let err = session
        .save_current_combat_capture(Path::new("unused.json"), None)
        .expect_err("map state should not capture");

    assert!(err.contains("no active combat state"));
}

#[test]
fn run_control_search_combat_applies_complete_winning_trajectory() {
    let mut session = test_session_with_first_monster_room();
    session
        .apply_command(RunControlCommand::Input(ClientInput::SelectMapNode(0)))
        .expect("map input should enter combat");

    let outcome = session
        .apply_command(RunControlCommand::SearchCombat(
            crate::eval::run_control::RunControlSearchCombatOptions {
                max_nodes: Some(2_000),
                wall_ms: Some(5_000),
                ..Default::default()
            },
        ))
        .expect("search-combat should resolve starter combat");

    assert!(outcome
        .message
        .contains("Search combat applied complete winning candidate"));
    assert!(outcome.message.contains("coverage_status="));
    assert!(outcome
        .message
        .contains("frontier_policy=round_robin_eval_buckets"));
    assert!(outcome
        .message
        .contains("turn_plan_policy=tactical_enemy_turn_boundary_frontier_seed"));
    assert!(outcome.message.contains("search_diagnostics="));
    assert!(outcome.message.contains("turn_plan_seeded="));
    assert!(outcome.message.contains("pending_high_fanout="));
    assert!(outcome.action_result.is_some());
    assert!(session.active_combat.is_none());
    assert_eq!(
        session
            .last_combat_baseline()
            .map(CombatBaselineOutcomeV1::terminal),
        Some(crate::sim::combat::CombatTerminal::Win)
    );
}

#[test]
fn run_control_combat_potion_use_updates_visible_potion_slots() {
    let mut session = test_session_with_first_monster_room();
    session.run_state.potions[1] = Some(crate::content::potions::Potion::new(
        crate::content::potions::PotionId::FruitJuice,
        42,
    ));
    session
        .apply_command(RunControlCommand::Input(ClientInput::SelectMapNode(0)))
        .expect("map input should enter combat");

    let outcome = session
        .apply_command(RunControlCommand::UsePotion {
            potion_index: 1,
            target_slot_or_id: None,
        })
        .expect("fruit juice should be usable in combat");

    assert!(outcome.message.contains("Lost potion: Fruit Juice"));
    assert!(session.active_combat.as_ref().is_some_and(|active| active
        .combat_state
        .entities
        .potions[1]
        .is_none()));
    let rendered = render_run_control_state(&session);
    assert!(!rendered.contains("Fruit Juice"));
    assert!(render_run_control_details(&session).contains("potions=0"));
}

#[test]
fn run_control_auto_step_advances_routine_neow_intro_only() {
    let mut session = RunControlSession::new(RunControlConfig::default());

    let outcome = session
        .apply_command(RunControlCommand::AutoStep(Default::default()))
        .expect("auto-step should advance routine intro");

    assert!(outcome.message.contains("routine: Proceed"));
    assert!(outcome
        .message
        .contains("Reason: Neow bonus requires human choice"));
    assert!(outcome.message.contains("Next: choose a Neow bonus id"));
    assert!(outcome.action_result.is_some());
    assert!(matches!(session.engine_state, EngineState::EventRoom));
    assert_eq!(
        session
            .run_state
            .event_state
            .as_ref()
            .map(|event| event.current_screen),
        Some(1)
    );
}

#[test]
fn run_control_auto_run_wraps_auto_step_with_run_summary() {
    let mut session = RunControlSession::new(RunControlConfig::default());

    let outcome = session
        .apply_command(RunControlCommand::AutoRun(Default::default()))
        .expect("auto-run should advance routine intro");

    assert!(outcome.message.contains("Auto-run stopped: Neow Bonus"));
    assert!(outcome
        .message
        .contains("Reason: Neow bonus requires human choice"));
    assert!(outcome.message.contains("Next: choose a Neow bonus id"));
    assert!(outcome.message.contains("route=planner"));
    assert!(outcome.message.contains("applied_operations=1"));
    assert!(outcome.message.contains("routine: Proceed"));
    assert!(outcome.action_result.is_some());
}

#[test]
fn run_control_auto_step_neow_stop_exports_human_boundary_record() {
    let mut session = RunControlSession::new(RunControlConfig::default());

    let outcome = session
        .apply_command(RunControlCommand::AutoStep(Default::default()))
        .expect("auto-step should advance intro and stop at Neow bonus");

    let record = noncombat_human_boundary_record(&outcome);
    assert_eq!(
        record.site,
        crate::ai::noncombat_decision_v1::DecisionSiteKindV1::Neow
    );
    assert_eq!(
        record.data_role,
        crate::ai::noncombat_decision_v1::DataRoleV1::HumanBoundaryNotTeacher
    );
    assert_eq!(
        record.selection.status,
        crate::ai::noncombat_decision_v1::PolicySelectionStatusV1::Stopped
    );
    assert!(!record.information_boundary.hidden_simulator_state_used);
    assert!(record.candidates.len() > 1);
    assert!(record.selection.selected_candidate_id.is_none());
}

#[test]
fn run_control_auto_step_event_stop_exports_human_boundary_record() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.event_state = Some(crate::state::events::EventState::new(
        crate::state::events::EventId::GoldenShrine,
    ));
    session.engine_state = EngineState::EventRoom;

    let outcome = session
        .apply_command(RunControlCommand::AutoStep(Default::default()))
        .expect("auto-step should stop at strategic event choice");

    let record = noncombat_human_boundary_record(&outcome);
    assert_eq!(
        record.site,
        crate::ai::noncombat_decision_v1::DecisionSiteKindV1::Event
    );
    assert_eq!(
        record.data_role,
        crate::ai::noncombat_decision_v1::DataRoleV1::HumanBoundaryNotTeacher
    );
    assert_eq!(
        record.selection.status,
        crate::ai::noncombat_decision_v1::PolicySelectionStatusV1::Stopped
    );
    assert!(record
        .selection
        .reason
        .contains("event option requires human choice"));
    assert!(record.candidates.len() > 1);
    assert!(record
        .evidence
        .items
        .iter()
        .any(|item| item.label.contains("strategy package: Resource/HpSafety")));
    assert!(matches!(session.engine_state, EngineState::EventRoom));
}

#[test]
fn run_control_auto_run_event_policy_takes_free_known_benefit() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.gold = 0;
    session.run_state.event_state = Some(crate::state::events::EventState::new(
        crate::state::events::EventId::GoldenShrine,
    ));
    session.engine_state = EngineState::EventRoom;

    let outcome = session
        .apply_command(RunControlCommand::AutoRun(
            crate::eval::run_control::RunControlAutoStepOptions {
                route: crate::eval::run_control::RunControlRouteAutomationMode::Planner,
                max_operations: Some(1),
                ..Default::default()
            },
        ))
        .expect("auto-run should take a free known event benefit");

    assert!(outcome.message.contains("event policy: [Pray]"));
    assert_eq!(session.run_state.gold, 100);
    assert!(matches!(session.engine_state, EngineState::EventRoom));
    assert_eq!(
        session
            .run_state
            .event_state
            .as_ref()
            .unwrap()
            .current_screen,
        1
    );
    let record = outcome
        .trace_annotations
        .iter()
        .find_map(|annotation| match annotation {
            crate::eval::run_control::RunControlTraceAnnotationV1::NonCombatPolicyDecision {
                record,
                ..
            } => Some(record),
            _ => None,
        })
        .expect("event policy should attach a noncombat record");
    crate::ai::noncombat_decision_v1::validate_noncombat_decision_record_v1(record)
        .expect("event policy noncombat record should validate");
    assert_eq!(
        record.site,
        crate::ai::noncombat_decision_v1::DecisionSiteKindV1::Event
    );
    assert_eq!(
        record.selection.status,
        crate::ai::noncombat_decision_v1::PolicySelectionStatusV1::Selected
    );
}

#[test]
fn run_control_auto_run_match_and_keep_flips_best_safe_pair() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut event_state =
        crate::state::events::EventState::new(crate::state::events::EventId::MatchAndKeep);
    event_state.current_screen = 1;
    event_state.extra_data = match_and_keep_board_with_entries(&[
        (crate::content::cards::CardId::Bash, 1),
        (crate::content::cards::CardId::Strike, 0),
        (crate::content::cards::CardId::Defend, 0),
        (crate::content::cards::CardId::Clumsy, 0),
        (crate::content::cards::CardId::IronWave, 0),
        (crate::content::cards::CardId::Cleave, 0),
    ]);
    session.run_state.event_state = Some(event_state);
    session.engine_state = EngineState::EventRoom;

    let outcome = session
        .apply_command(RunControlCommand::AutoRun(
            crate::eval::run_control::RunControlAutoStepOptions {
                route: crate::eval::run_control::RunControlRouteAutomationMode::Planner,
                max_operations: Some(2),
                ..Default::default()
            },
        ))
        .expect("auto-run should use the visible Match and Keep board strategy");

    assert!(
        outcome.message.contains("event policy: Match and Keep"),
        "{}",
        outcome.message
    );
    let obtained = session.run_state.master_deck.last().unwrap();
    assert_eq!(obtained.id, crate::content::cards::CardId::IronWave);
    assert_eq!(obtained.upgrades, 0);
    assert!(session
        .run_state
        .master_deck
        .iter()
        .all(|card| card.id != crate::content::cards::CardId::Clumsy));
}

#[test]
fn run_control_auto_step_collapses_terminal_event_leave_to_map() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.event_state = Some(crate::state::events::EventState::new(
        crate::state::events::EventId::ScrapOoze,
    ));
    session.engine_state = EngineState::EventRoom;

    let outcome = session
        .apply_command(RunControlCommand::AutoStep(
            crate::eval::run_control::RunControlAutoStepOptions {
                route: crate::eval::run_control::RunControlRouteAutomationMode::Planner,
                max_operations: Some(2),
                ..Default::default()
            },
        ))
        .expect("auto-step should decline Scrap Ooze and leave the event");

    assert!(
        !outcome.message.contains("repeated boundary detected"),
        "{}",
        outcome.message
    );
    assert!(matches!(session.engine_state, EngineState::MapNavigation));
    assert!(session.run_state.event_state.is_none());
}

#[test]
fn run_control_auto_step_shop_stop_exports_human_boundary_record() {
    let mut session = test_session_at_shop();

    let outcome = session
        .apply_command(RunControlCommand::AutoStep(Default::default()))
        .expect("auto-step should stop at non-empty shop");

    let record = noncombat_human_boundary_record(&outcome);
    assert_eq!(
        record.site,
        crate::ai::noncombat_decision_v1::DecisionSiteKindV1::Shop
    );
    assert_eq!(
        record.data_role,
        crate::ai::noncombat_decision_v1::DataRoleV1::HumanBoundaryNotTeacher
    );
    assert_eq!(
        record.selection.status,
        crate::ai::noncombat_decision_v1::PolicySelectionStatusV1::Stopped
    );
    assert!(record
        .selection
        .reason
        .contains("shop action requires human choice"));
    assert!(record
        .candidates
        .iter()
        .any(|candidate| candidate.candidate_id.starts_with("shop:card-")));
    assert!(record
        .candidates
        .iter()
        .any(|candidate| candidate.candidate_id.starts_with("shop:leave")));
    assert!(record.evidence.items.iter().any(|item| item
        .label
        .contains("strategy package: Route/CorePlanProtection")));
    assert!(record
        .evidence
        .items
        .iter()
        .any(|item| item.label.contains("strategy package: Resource/GoldPlan")));
    assert!(record
        .information_boundary
        .allowed_inputs
        .contains(&crate::ai::noncombat_decision_v1::InformationClassV1::Belief));
    assert!(outcome.action_result.is_none());
    assert!(matches!(session.engine_state, EngineState::Shop(_)));
}

#[test]
fn run_control_boundary_command_renders_current_noncombat_record_summary() {
    let mut session = test_session_at_shop();
    let command = parse_run_control_command("bd").expect("bd should parse as boundary view");

    let outcome = session
        .apply_command(command)
        .expect("boundary view should render current noncombat record");

    assert!(outcome.message.contains("NonCombatDecisionRecordV1"));
    assert!(outcome.message.contains("site=Shop"));
    assert!(outcome
        .message
        .contains("data_role=HumanBoundaryNotTeacher"));
    assert!(outcome.message.contains("hidden_free=true"));
    assert!(outcome.message.contains("selection=Stopped"));
    assert!(outcome.message.contains("shop:card-0"));
    assert!(outcome.message.contains("shop:leave"));
    assert!(outcome.action_result.is_none());
    assert!(matches!(session.engine_state, EngineState::Shop(_)));
}

#[test]
fn run_control_auto_step_campfire_stop_exports_human_boundary_record() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.engine_state = EngineState::Campfire;

    let outcome = session
        .apply_command(RunControlCommand::AutoStep(Default::default()))
        .expect("auto-step should stop at campfire");

    let record = noncombat_human_boundary_record(&outcome);
    assert_eq!(
        record.site,
        crate::ai::noncombat_decision_v1::DecisionSiteKindV1::Campfire
    );
    assert_eq!(
        record.data_role,
        crate::ai::noncombat_decision_v1::DataRoleV1::HumanBoundaryNotTeacher
    );
    assert!(record
        .candidates
        .iter()
        .any(|candidate| candidate.candidate_id.starts_with("campfire:rest")));
    assert!(record
        .candidates
        .iter()
        .any(|candidate| candidate.candidate_id.starts_with("campfire:smith-")));
    assert!(record.evidence.items.iter().any(|item| item
        .label
        .contains("strategy package: Route/RecoveryPressure")));
    assert!(record
        .evidence
        .items
        .iter()
        .any(|item| item.label.contains("strategy package: Resource/HpSafety")));
    assert!(record
        .information_boundary
        .allowed_inputs
        .contains(&crate::ai::noncombat_decision_v1::InformationClassV1::Belief));
    assert!(matches!(session.engine_state, EngineState::Campfire));
}

#[test]
fn run_control_auto_run_uses_recovery_route_package_to_rest_at_low_hp_campfire() {
    let mut session = test_session_at_campfire_with_hp(20, 80);

    let outcome = session
        .apply_command(RunControlCommand::AutoRun(
            crate::eval::run_control::RunControlAutoStepOptions {
                max_operations: Some(1),
                ..Default::default()
            },
        ))
        .expect("auto-run should rest when recovery pressure is strong");

    assert!(outcome.message.contains("campfire policy: rest"));
    let record = outcome
        .trace_annotations
        .iter()
        .find_map(|annotation| match annotation {
            crate::eval::run_control::RunControlTraceAnnotationV1::NonCombatPolicyDecision {
                record,
                ..
            } => Some(record),
            _ => None,
        })
        .expect("campfire policy should attach a noncombat record");
    crate::ai::noncombat_decision_v1::validate_noncombat_decision_record_v1(record)
        .expect("campfire policy noncombat record should validate");
    assert_eq!(
        record.site,
        crate::ai::noncombat_decision_v1::DecisionSiteKindV1::Campfire
    );
    assert_eq!(
        record.data_role,
        crate::ai::noncombat_decision_v1::DataRoleV1::BehaviorPolicyNotTeacher
    );
    assert!(outcome.action_result.is_some());
    assert!(
        session.run_state.current_hp > 20,
        "rest should heal before leaving the campfire"
    );
    assert!(matches!(session.engine_state, EngineState::MapNavigation));
}

#[test]
fn run_control_auto_run_smiths_clear_core_upgrade_at_healthy_campfire() {
    let mut session = test_session_at_campfire_with_hp(80, 80);

    let outcome = session
        .apply_command(RunControlCommand::AutoRun(
            crate::eval::run_control::RunControlAutoStepOptions {
                max_operations: Some(1),
                ..Default::default()
            },
        ))
        .expect("auto-run should smith a clear core upgrade at healthy campfire");

    assert!(outcome.message.contains("campfire policy: smith"));
    assert!(outcome.action_result.is_some());
    assert!(
        session
            .run_state
            .master_deck
            .iter()
            .any(|card| card.id == crate::content::cards::CardId::Bash && card.upgrades == 1),
        "healthy campfire policy should upgrade Bash as the default Ironclad core upgrade"
    );
    assert!(matches!(session.engine_state, EngineState::MapNavigation));
}

#[test]
fn run_control_auto_run_purges_curse_at_shop() {
    let mut session = test_session_at_shop();
    session
        .run_state
        .add_card_to_deck_without_interception_from(
            crate::content::cards::CardId::Doubt,
            0,
            crate::state::selection::DomainEventSource::DeckMutation,
        );

    let outcome = session
        .apply_command(RunControlCommand::AutoRun(
            crate::eval::run_control::RunControlAutoStepOptions {
                max_operations: Some(2),
                ..Default::default()
            },
        ))
        .expect("auto-run should purge a visible curse at shop");

    assert!(outcome.message.contains("shop policy: purge Doubt"));
    let record = outcome
        .trace_annotations
        .iter()
        .find_map(|annotation| match annotation {
            crate::eval::run_control::RunControlTraceAnnotationV1::NonCombatPolicyDecision {
                record,
                ..
            } => Some(record),
            _ => None,
        })
        .expect("shop policy should attach a noncombat record");
    crate::ai::noncombat_decision_v1::validate_noncombat_decision_record_v1(record)
        .expect("shop policy noncombat record should validate");
    assert_eq!(
        record.site,
        crate::ai::noncombat_decision_v1::DecisionSiteKindV1::Shop
    );
    assert_eq!(
        record.data_role,
        crate::ai::noncombat_decision_v1::DataRoleV1::BehaviorPolicyNotTeacher
    );
    assert_eq!(
        record.selection.status,
        crate::ai::noncombat_decision_v1::PolicySelectionStatusV1::Selected
    );
    assert_eq!(session.run_state.gold, 25);
    assert!(!session
        .run_state
        .master_deck
        .iter()
        .any(|card| card.id == crate::content::cards::CardId::Doubt));
    assert!(matches!(session.engine_state, EngineState::Shop(_)));
}

#[test]
fn run_control_auto_run_reopens_pending_shop_rewards_before_shop_policy() {
    let mut session = test_session_at_shop();
    session
        .reward_automation
        .claim_safe_relic_without_sapphire_key = false;
    session
        .run_state
        .add_card_to_deck_without_interception_from(
            crate::content::cards::CardId::Doubt,
            0,
            crate::state::selection::DomainEventSource::DeckMutation,
        );
    let EngineState::Shop(shop) = &mut session.engine_state else {
        panic!("test session should start in shop");
    };
    let mut pending = crate::state::rewards::RewardState::new();
    pending.items = vec![crate::state::rewards::RewardItem::Relic {
        relic_id: crate::content::relics::RelicId::Anchor,
    }];
    shop.pending_reward_overlay = Some(pending);

    let outcome = session
        .apply_command(RunControlCommand::AutoRun(
            crate::eval::run_control::RunControlAutoStepOptions {
                max_operations: Some(2),
                ..Default::default()
            },
        ))
        .expect("auto-run should reopen pending reward overlay before shop policy");

    assert!(outcome.message.contains("routine: Open pending rewards"));
    assert!(outcome
        .message
        .contains("Reason: relic reward requires human choice"));
    assert!(
        session
            .run_state
            .master_deck
            .iter()
            .any(|card| card.id == crate::content::cards::CardId::Doubt),
        "shop purge policy must not run before pending overlay rewards are restored"
    );
    assert!(matches!(
        session.engine_state,
        EngineState::RewardOverlay { .. }
    ));
}

#[test]
fn run_control_auto_run_does_not_purge_starter_shell_at_shop() {
    let mut session = test_session_at_shop();
    if let EngineState::Shop(shop) = &mut session.engine_state {
        shop.cards = vec![
            crate::state::shop::ShopCard {
                card_id: crate::content::cards::CardId::Clash,
                upgrades: 0,
                price: 45,
                can_buy: true,
                blocked_reason: None,
            },
            crate::state::shop::ShopCard {
                card_id: crate::content::cards::CardId::Flex,
                upgrades: 0,
                price: 45,
                can_buy: true,
                blocked_reason: None,
            },
        ];
    }

    let outcome = session
        .apply_command(RunControlCommand::AutoRun(
            crate::eval::run_control::RunControlAutoStepOptions {
                max_operations: Some(1),
                ..Default::default()
            },
        ))
        .expect("auto-run should stop at ordinary shop");

    assert!(outcome
        .message
        .contains("Reason: shop action requires human choice"));
    assert_eq!(session.run_state.gold, 100);
    assert_eq!(session.run_state.master_deck.len(), 10);
    assert!(matches!(session.engine_state, EngineState::Shop(_)));
}

#[test]
fn run_control_auto_run_buys_high_impact_shop_card_when_affordable() {
    let mut session = test_session_at_shop();
    session.run_state.gold = 200;
    if let EngineState::Shop(shop) = &mut session.engine_state {
        shop.cards = vec![
            crate::state::shop::ShopCard {
                card_id: crate::content::cards::CardId::Shockwave,
                upgrades: 0,
                price: 90,
                can_buy: true,
                blocked_reason: None,
            },
            crate::state::shop::ShopCard {
                card_id: crate::content::cards::CardId::Clash,
                upgrades: 0,
                price: 45,
                can_buy: true,
                blocked_reason: None,
            },
        ];
    }

    let outcome = session
        .apply_command(RunControlCommand::AutoRun(
            crate::eval::run_control::RunControlAutoStepOptions {
                max_operations: Some(2),
                ..Default::default()
            },
        ))
        .expect("auto-run should buy a clear high-impact shop card");

    assert!(
        outcome.message.contains("shop policy:"),
        "auto-run should report the shop policy action, got:\n{}",
        outcome.message
    );
    assert_eq!(session.run_state.gold, 110);
    assert!(
        session
            .run_state
            .master_deck
            .iter()
            .any(|card| card.id == crate::content::cards::CardId::Shockwave),
        "shop purchase should add the selected card to the master deck"
    );
    assert!(matches!(session.engine_state, EngineState::Shop(_)));
}

#[test]
fn branch_skip_card_reward_consumes_last_non_skippable_reward_item() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut reward = crate::state::rewards::RewardState::new();
    reward.skippable = false;
    reward.items.push(crate::state::rewards::RewardItem::Card {
        cards: vec![
            crate::state::rewards::RewardCard::new(crate::content::cards::CardId::TwinStrike, 0),
            crate::state::rewards::RewardCard::new(crate::content::cards::CardId::ShrugItOff, 0),
        ],
    });
    session.engine_state = EngineState::RewardScreen(reward);

    session
        .apply_command(RunControlCommand::BranchSkipCardReward(0))
        .expect("synthetic branch skip should consume the card reward item");

    assert!(
        matches!(session.engine_state, EngineState::MapNavigation),
        "empty non-skippable reward screens should settle through reward completion"
    );
    assert_eq!(
        session.run_state.master_deck.len(),
        10,
        "synthetic skip should not add a reward card"
    );
}

#[test]
fn run_control_auto_run_uses_core_plan_package_to_purge_starter_when_no_purchase_competes() {
    let mut session = test_session_at_shop();
    if let EngineState::Shop(shop) = &mut session.engine_state {
        shop.cards.clear();
        shop.relics.clear();
        shop.potions.clear();
    }
    session
        .run_state
        .add_card_to_deck(crate::content::cards::CardId::Inflame);
    session
        .run_state
        .add_card_to_deck(crate::content::cards::CardId::HeavyBlade);

    let outcome = session
        .apply_command(RunControlCommand::AutoRun(
            crate::eval::run_control::RunControlAutoStepOptions {
                max_operations: Some(2),
                ..Default::default()
            },
        ))
        .expect("auto-run should purge a starter strike when core-plan protection is strong");

    assert!(outcome.message.contains("shop policy: purge Strike"));
    assert_eq!(session.run_state.gold, 25);
    assert_eq!(
        session
            .run_state
            .master_deck
            .iter()
            .filter(|card| card.id == crate::content::cards::CardId::Strike)
            .count(),
        4
    );
    assert!(matches!(session.engine_state, EngineState::Shop(_)));
}

#[test]
fn run_control_auto_run_purges_curse_at_run_pending_purge_choice() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session
        .run_state
        .add_card_to_deck_without_interception_from(
            crate::content::cards::CardId::Doubt,
            0,
            crate::state::selection::DomainEventSource::DeckMutation,
        );
    session.engine_state =
        EngineState::RunPendingChoice(crate::state::core::RunPendingChoiceState {
            min_choices: 1,
            max_choices: 1,
            reason: crate::state::core::RunPendingChoiceReason::PurgeNonBottled,
            return_state: Box::new(EngineState::MapNavigation),
        });

    let outcome = session
        .apply_command(RunControlCommand::AutoRun(
            crate::eval::run_control::RunControlAutoStepOptions {
                max_operations: Some(1),
                ..Default::default()
            },
        ))
        .expect("auto-run should purge a curse at a run pending purge choice");

    assert!(outcome
        .message
        .contains("deck mutation compiler: select Doubt"));
    let record = outcome
        .trace_annotations
        .iter()
        .find_map(|annotation| match annotation {
            crate::eval::run_control::RunControlTraceAnnotationV1::NonCombatPolicyDecision {
                record,
                ..
            } => Some(record),
            _ => None,
        })
        .expect("run choice policy should attach a noncombat record");
    crate::ai::noncombat_decision_v1::validate_noncombat_decision_record_v1(record)
        .expect("run choice policy noncombat record should validate");
    assert_eq!(
        record.site,
        crate::ai::noncombat_decision_v1::DecisionSiteKindV1::RunChoice
    );
    assert_eq!(
        record.data_role,
        crate::ai::noncombat_decision_v1::DataRoleV1::BehaviorPolicyNotTeacher
    );
    assert!(!session
        .run_state
        .master_deck
        .iter()
        .any(|card| card.id == crate::content::cards::CardId::Doubt));
    assert!(matches!(session.engine_state, EngineState::MapNavigation));
}

#[test]
fn run_control_auto_run_selects_starter_at_run_pending_purge_choice() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.engine_state =
        EngineState::RunPendingChoice(crate::state::core::RunPendingChoiceState {
            min_choices: 1,
            max_choices: 1,
            reason: crate::state::core::RunPendingChoiceReason::PurgeNonBottled,
            return_state: Box::new(EngineState::MapNavigation),
        });

    let outcome = session
        .apply_command(RunControlCommand::AutoRun(
            crate::eval::run_control::RunControlAutoStepOptions {
                max_operations: Some(1),
                ..Default::default()
            },
        ))
        .expect("auto-run should select a low-value starter at a purge choice without a curse");

    assert!(outcome
        .message
        .contains("deck mutation compiler: select Strike"));
    assert_eq!(session.run_state.master_deck.len(), 9);
    assert_eq!(
        session
            .run_state
            .master_deck
            .iter()
            .filter(|card| card.id == crate::content::cards::CardId::Strike)
            .count(),
        4
    );
    assert!(matches!(session.engine_state, EngineState::MapNavigation));
}

#[test]
fn run_control_auto_run_executes_single_forced_run_pending_choice() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.master_deck = vec![crate::runtime::combat::CombatCard::new(
        crate::content::cards::CardId::Strike,
        7001,
    )];
    session.engine_state =
        EngineState::RunPendingChoice(crate::state::core::RunPendingChoiceState {
            min_choices: 1,
            max_choices: 1,
            reason: crate::state::core::RunPendingChoiceReason::Upgrade,
            return_state: Box::new(EngineState::MapNavigation),
        });

    let outcome = session
        .apply_command(RunControlCommand::AutoRun(
            crate::eval::run_control::RunControlAutoStepOptions {
                max_operations: Some(1),
                ..Default::default()
            },
        ))
        .expect("auto-run should execute a single forced run pending choice");

    assert!(outcome.message.contains("single forced run choice"));
    assert_eq!(session.run_state.master_deck[0].upgrades, 1);
    assert!(matches!(session.engine_state, EngineState::MapNavigation));
}

#[test]
fn run_control_auto_step_boss_relic_stop_exports_human_boundary_record() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.engine_state =
        EngineState::BossRelicSelect(crate::state::rewards::BossRelicChoiceState::new(vec![
            crate::content::relics::RelicId::BlackStar,
            crate::content::relics::RelicId::Astrolabe,
        ]));

    let outcome = session
        .apply_command(RunControlCommand::AutoStep(Default::default()))
        .expect("auto-step should stop at boss relic choice");

    let record = noncombat_human_boundary_record(&outcome);
    assert_eq!(
        record.site,
        crate::ai::noncombat_decision_v1::DecisionSiteKindV1::BossRelic
    );
    assert_eq!(
        record.data_role,
        crate::ai::noncombat_decision_v1::DataRoleV1::HumanBoundaryNotTeacher
    );
    assert_eq!(record.candidates.len(), 3);
    assert!(record
        .candidates
        .iter()
        .any(|candidate| candidate.candidate_id == "boss_relic:skip"));
    assert!(record
        .information_boundary
        .forbidden_inputs
        .contains(&crate::ai::noncombat_decision_v1::InformationClassV1::HiddenSimulatorState));
    assert!(record.evidence.items.iter().any(|item| item
        .label
        .contains("strategy package: Resource/RelicConstraints")));
    assert!(matches!(
        session.engine_state,
        EngineState::BossRelicSelect(_)
    ));
}

#[test]
fn run_control_auto_run_stops_on_high_agency_boss_relic_choice() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.engine_state =
        EngineState::BossRelicSelect(crate::state::rewards::BossRelicChoiceState::new(vec![
            crate::content::relics::RelicId::TinyHouse,
            crate::content::relics::RelicId::RunicPyramid,
            crate::content::relics::RelicId::SneckoEye,
        ]));

    let outcome = session
        .apply_command(RunControlCommand::AutoRun(
            crate::eval::run_control::RunControlAutoStepOptions {
                route: crate::eval::run_control::RunControlRouteAutomationMode::Planner,
                max_operations: Some(1),
                ..Default::default()
            },
        ))
        .expect("auto-run should stop for high-agency boss relic choices");

    assert!(outcome
        .message
        .contains("Reason: boss relic choice requires human choice"));
    assert!(outcome.action_result.is_none());
    assert!(matches!(
        session.engine_state,
        EngineState::BossRelicSelect(_)
    ));
    let record = noncombat_human_boundary_record(&outcome);
    assert_eq!(
        record.site,
        crate::ai::noncombat_decision_v1::DecisionSiteKindV1::BossRelic
    );
}

#[test]
fn run_control_auto_step_run_choice_stop_exports_human_boundary_record() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.engine_state =
        EngineState::RunPendingChoice(crate::state::core::RunPendingChoiceState {
            min_choices: 1,
            max_choices: 1,
            reason: crate::state::core::RunPendingChoiceReason::Upgrade,
            return_state: Box::new(EngineState::MapNavigation),
        });

    let outcome = session
        .apply_command(RunControlCommand::AutoStep(Default::default()))
        .expect("auto-step should stop at multi-candidate run choice");

    let record = noncombat_human_boundary_record(&outcome);
    assert_eq!(
        record.site,
        crate::ai::noncombat_decision_v1::DecisionSiteKindV1::RunChoice
    );
    assert_eq!(
        record.data_role,
        crate::ai::noncombat_decision_v1::DataRoleV1::HumanBoundaryNotTeacher
    );
    assert!(record
        .selection
        .reason
        .contains("card selection requires human choice"));
    assert!(record
        .candidates
        .iter()
        .any(|candidate| candidate.candidate_id.starts_with("run_choice:")));
    assert!(outcome.action_result.is_none());
    assert!(matches!(
        session.engine_state,
        EngineState::RunPendingChoice(_)
    ));
}

#[test]
fn run_control_auto_step_stops_on_map_without_mutating_state() {
    let mut session = test_session_after_neow_at_map();

    let outcome = session
        .apply_command(RunControlCommand::AutoStep(Default::default()))
        .expect("auto-step should stop at map");

    assert!(outcome.message.contains("Applied:\n  none"));
    assert!(outcome
        .message
        .contains("Reason: map route requires human choice"));
    assert!(outcome
        .message
        .contains("Next: use rs to inspect route evidence"));
    assert!(outcome.action_result.is_none());
    assert!(matches!(session.engine_state, EngineState::MapNavigation));
}

#[test]
fn run_control_auto_step_records_route_policy_stop_when_safety_gate_rejects() {
    let mut session = test_session_with_forced_unsafe_elite_route();
    let before = (
        session.run_state.map.current_x,
        session.run_state.map.current_y,
        session.run_state.current_hp,
    );

    let outcome = session
        .apply_command(RunControlCommand::AutoStep(
            crate::eval::run_control::RunControlAutoStepOptions {
                route: crate::eval::run_control::RunControlRouteAutomationMode::Planner,
                max_operations: Some(1),
                ..Default::default()
            },
        ))
        .expect("route planner safety gate should stop cleanly");

    assert!(outcome
        .message
        .contains("Reason: route planner declined automatic map selection"));
    assert!(outcome.message.contains("route planner policy stopped:"));
    assert_eq!(
        before,
        (
            session.run_state.map.current_x,
            session.run_state.map.current_y,
            session.run_state.current_hp
        )
    );
    assert!(outcome.action_result.is_none());
    let record = outcome
        .trace_annotations
        .iter()
        .find_map(|annotation| match annotation {
            RunControlTraceAnnotationV1::NonCombatPolicyDecision { record, .. } => Some(record),
            _ => None,
        })
        .expect("declined route planner should attach a noncombat policy record");
    assert_eq!(
        record.site,
        crate::ai::noncombat_decision_v1::DecisionSiteKindV1::Map
    );
    assert_eq!(
        record.data_role,
        crate::ai::noncombat_decision_v1::DataRoleV1::BehaviorPolicyNotTeacher
    );
    assert_eq!(
        record.selection.status,
        crate::ai::noncombat_decision_v1::PolicySelectionStatusV1::Stopped
    );
    assert!(!record.candidates.is_empty());
}

#[test]
fn run_control_auto_step_returns_from_map_overlay_without_paths_before_route_planner() {
    let mut session = test_session_at_card_reward(vec![
        crate::content::cards::CardId::Clash,
        crate::content::cards::CardId::PommelStrike,
        crate::content::cards::CardId::IronWave,
    ]);
    let return_state = session.engine_state.clone();
    session.engine_state = EngineState::map_overlay(return_state);
    session.run_state.map.current_x = 0;
    session.run_state.map.current_y = 15;

    let outcome = session
        .apply_command(RunControlCommand::AutoStep(
            crate::eval::run_control::RunControlAutoStepOptions {
                route: crate::eval::run_control::RunControlRouteAutomationMode::Planner,
                max_operations: Some(2),
                ..Default::default()
            },
        ))
        .expect("map overlay back should be routine automation");

    assert!(outcome.message.contains("Back to reward screen"));
    assert!(!outcome
        .message
        .contains("route planner declined automatic map selection"));
    assert!(outcome
        .message
        .contains("Reason: card reward requires human choice"));
    assert!(matches!(session.engine_state, EngineState::RewardScreen(_)));
}

#[test]
fn run_control_auto_step_can_apply_rejected_route_when_campaign_fallback_is_enabled() {
    let mut session = test_session_with_forced_unsafe_elite_route();
    let before = (
        session.run_state.map.current_x,
        session.run_state.map.current_y,
    );

    let outcome = session
        .apply_command(RunControlCommand::AutoStep(
            crate::eval::run_control::RunControlAutoStepOptions {
                route: crate::eval::run_control::RunControlRouteAutomationMode::Planner,
                max_operations: Some(1),
                allow_route_reject_unless_forced: true,
                ..Default::default()
            },
        ))
        .expect("campaign fallback should apply the forced route candidate");

    assert!(outcome.message.contains("route planner:"));
    assert!(outcome.message.contains("reject_unless_forced"));
    assert!(outcome.action_result.is_some());
    assert_ne!(
        before,
        (
            session.run_state.map.current_x,
            session.run_state.map.current_y
        )
    );
}

#[test]
fn run_control_auto_run_uses_route_planner_by_default() {
    let mut session = test_session_with_first_monster_room();

    let outcome = session
        .apply_command(RunControlCommand::AutoRun(
            crate::eval::run_control::RunControlAutoStepOptions {
                max_operations: Some(1),
                ..Default::default()
            },
        ))
        .expect("auto-run should use route planner");

    assert!(outcome.message.contains("Auto-run stopped: Combat"));
    assert!(outcome.message.contains("route=planner"));
    assert!(outcome.message.contains("route planner:"));
    assert!(outcome
        .message
        .contains("Next: play manually, cap the combat if useful"));
    assert!(matches!(
        session.engine_state,
        EngineState::CombatPlayerTurn
    ));
}

#[test]
fn run_control_card_reward_uses_runtime_outcome_calibration_estimates() {
    let mut session = test_session_at_card_reward(vec![
        crate::content::cards::CardId::TwinStrike,
        crate::content::cards::CardId::Cleave,
        crate::content::cards::CardId::SeverSoul,
    ]);
    session.card_reward_outcome_calibration = Some(test_card_reward_calibration_for_twin_strike());

    let outcome = session
        .apply_command(RunControlCommand::AutoRun(
            crate::eval::run_control::RunControlAutoStepOptions {
                max_operations: Some(1),
                ..Default::default()
            },
        ))
        .expect("auto-run should evaluate card reward with runtime calibration");

    assert!(outcome.message.contains("value_source_outcome_calibration"));
    assert!(outcome.message.contains("value_status_outcome_calibrated"));
    assert!(outcome
        .message
        .contains("gate estimates: OutcomeCalibration=1, RouteRisk=2"));
    assert!(outcome
        .message
        .contains("non-gate value candidates: Twin Strike (OutcomeCalibration), Cleave (RouteRisk), Sever Soul (RouteRisk)"));
    assert!(outcome
        .message
        .contains("Reason: card reward requires human choice"));
    assert!(!session
        .run_state
        .master_deck
        .iter()
        .any(|card| card.id == crate::content::cards::CardId::TwinStrike));

    let record = outcome
        .trace_annotations
        .iter()
        .find_map(|annotation| match annotation {
            crate::eval::run_control::RunControlTraceAnnotationV1::NonCombatPolicyDecision {
                record,
                ..
            } => Some(record),
            _ => None,
        })
        .expect("declined card reward policy should attach a noncombat record");
    assert!(record.values.iter().any(|value| {
        value
            .components
            .iter()
            .any(|component| component.name == "value_source_outcome_calibration")
    }));
}

#[test]
fn run_control_auto_run_claims_safe_relic_reward_with_policy_annotation() {
    let mut session =
        test_session_at_reward_items(vec![crate::state::rewards::RewardItem::Relic {
            relic_id: crate::content::relics::RelicId::Anchor,
        }]);

    let outcome = session
        .apply_command(RunControlCommand::AutoRun(
            crate::eval::run_control::RunControlAutoStepOptions {
                max_operations: Some(1),
                ..Default::default()
            },
        ))
        .expect("auto-run should claim a safe relic reward");

    assert!(outcome.message.contains("routine reward: Relic Anchor"));
    assert!(session
        .run_state
        .relics
        .iter()
        .any(|relic| relic.id == crate::content::relics::RelicId::Anchor));
    let record = outcome
        .trace_annotations
        .iter()
        .find_map(|annotation| match annotation {
            crate::eval::run_control::RunControlTraceAnnotationV1::NonCombatPolicyDecision {
                record,
                ..
            } => Some(record),
            _ => None,
        })
        .expect("safe relic reward auto-claim should attach a noncombat record");
    crate::ai::noncombat_decision_v1::validate_noncombat_decision_record_v1(record)
        .expect("safe relic reward noncombat record should validate");
    assert_eq!(
        record.site,
        crate::ai::noncombat_decision_v1::DecisionSiteKindV1::Reward
    );
    assert_eq!(
        record.selection.status,
        crate::ai::noncombat_decision_v1::PolicySelectionStatusV1::Selected
    );
    assert_eq!(
        record.selection.selected_candidate_id.as_deref(),
        Some("reward:relic:0:Anchor")
    );
}

#[test]
fn run_control_auto_run_keeps_relic_reward_when_sapphire_key_is_available() {
    let mut session = test_session_at_reward_items(vec![
        crate::state::rewards::RewardItem::Relic {
            relic_id: crate::content::relics::RelicId::Anchor,
        },
        crate::state::rewards::RewardItem::SapphireKey,
    ]);

    let outcome = session
        .apply_command(RunControlCommand::AutoRun(
            crate::eval::run_control::RunControlAutoStepOptions {
                max_operations: Some(1),
                ..Default::default()
            },
        ))
        .expect("auto-run should stop for sapphire/relic choice");

    assert!(outcome
        .message
        .contains("Reason: relic reward or Sapphire Key requires human choice"));
    assert!(!session
        .run_state
        .relics
        .iter()
        .any(|relic| relic.id == crate::content::relics::RelicId::Anchor));
    let EngineState::RewardScreen(reward) = &session.engine_state else {
        panic!("sapphire/relic choice should remain on reward screen");
    };
    assert!(matches!(
        reward.items.as_slice(),
        [
            crate::state::rewards::RewardItem::Relic {
                relic_id: crate::content::relics::RelicId::Anchor
            },
            crate::state::rewards::RewardItem::SapphireKey
        ]
    ));
}

#[test]
fn run_control_auto_run_stops_on_card_reward_with_singing_bowl() {
    let mut session = test_session_at_card_reward(vec![
        crate::content::cards::CardId::Shockwave,
        crate::content::cards::CardId::Clash,
        crate::content::cards::CardId::SeverSoul,
    ]);
    session
        .run_state
        .relics
        .push(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::SingingBowl,
        ));

    let outcome = session
        .apply_command(RunControlCommand::AutoRun(
            crate::eval::run_control::RunControlAutoStepOptions {
                max_operations: Some(1),
                ..Default::default()
            },
        ))
        .expect("auto-run should stop when Singing Bowl adds a strategic card reward option");

    assert!(outcome
        .message
        .contains("Reason: card reward requires human choice"));
    assert!(outcome.message.contains("card reward policy stopped:"));
    assert!(outcome.trace_annotations.iter().any(|annotation| {
        matches!(
            annotation,
            crate::eval::run_control::RunControlTraceAnnotationV1::NonCombatPolicyDecision {
                record,
                ..
            } if record.site == crate::ai::noncombat_decision_v1::DecisionSiteKindV1::CardReward
                && record.selection.status
                    == crate::ai::noncombat_decision_v1::PolicySelectionStatusV1::Stopped
        )
    }));
    assert!(!session
        .run_state
        .master_deck
        .iter()
        .any(|card| card.id == crate::content::cards::CardId::Shockwave));
    assert!(outcome.action_result.is_none());
}

#[test]
fn run_control_recorded_card_reward_pick_selects_card_with_noncombat_record() {
    let mut session = test_session_at_card_reward(vec![
        crate::content::cards::CardId::Shockwave,
        crate::content::cards::CardId::Clash,
        crate::content::cards::CardId::SeverSoul,
    ]);

    let outcome = session
        .apply_command(RunControlCommand::RecordedCardRewardPick(1))
        .expect("recorded pick should select a card reward");

    assert!(outcome.action_result.is_some());
    assert!(session
        .run_state
        .master_deck
        .iter()
        .any(|card| card.id == crate::content::cards::CardId::Clash));
    let annotations = outcome
        .trace_annotations
        .iter()
        .filter_map(|annotation| match annotation {
            crate::eval::run_control::RunControlTraceAnnotationV1::NonCombatPolicyDecision {
                record,
                card_reward_packet,
            } => Some((record, card_reward_packet)),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        annotations.len(),
        1,
        "recorded card reward pick should not duplicate selected records"
    );
    let annotation = annotations
        .first()
        .copied()
        .expect("recorded pick should attach a card reward noncombat record");
    let (record, packet) = annotation;
    crate::ai::noncombat_decision_v1::validate_noncombat_decision_record_v1(record)
        .expect("recorded card reward record should validate");
    assert_eq!(
        record.site,
        crate::ai::noncombat_decision_v1::DecisionSiteKindV1::CardReward
    );
    assert_eq!(
        record.selection.status,
        crate::ai::noncombat_decision_v1::PolicySelectionStatusV1::Selected
    );
    assert_eq!(
        record.selection.selected_candidate_id.as_deref(),
        Some("card_reward:1:Clash")
    );
    assert_eq!(record.selection.selection_mode, "human_recorded_pick");
    assert_eq!(
        record.provenance.source_policy,
        "run_control_recorded_card_reward_pick_v1"
    );
    assert!(packet.is_some());
}

#[test]
fn run_control_manual_card_reward_pick_selects_card_with_noncombat_record() {
    let mut session = test_session_at_card_reward(vec![
        crate::content::cards::CardId::Shockwave,
        crate::content::cards::CardId::Clash,
        crate::content::cards::CardId::SeverSoul,
    ]);

    let outcome = session
        .apply_command(RunControlCommand::Candidate("1".to_string()))
        .expect("manual visible card reward pick should select a card reward");

    assert!(outcome.action_result.is_some());
    assert!(session
        .run_state
        .master_deck
        .iter()
        .any(|card| card.id == crate::content::cards::CardId::Clash));
    let annotation = outcome
        .trace_annotations
        .iter()
        .find_map(|annotation| match annotation {
            crate::eval::run_control::RunControlTraceAnnotationV1::NonCombatPolicyDecision {
                record,
                card_reward_packet,
            } => Some((record, card_reward_packet)),
            _ => None,
        })
        .expect("manual pick should attach a card reward noncombat record");
    let (record, packet) = annotation;
    crate::ai::noncombat_decision_v1::validate_noncombat_decision_record_v1(record)
        .expect("manual card reward record should validate");
    assert_eq!(
        record.site,
        crate::ai::noncombat_decision_v1::DecisionSiteKindV1::CardReward
    );
    assert_eq!(
        record.selection.status,
        crate::ai::noncombat_decision_v1::PolicySelectionStatusV1::Selected
    );
    assert_eq!(
        record.selection.selected_candidate_id.as_deref(),
        Some("card_reward:1:Clash")
    );
    assert_eq!(
        record.selection.selection_mode,
        "human_visible_card_reward_pick"
    );
    assert_eq!(
        record.provenance.source_policy,
        "run_control_manual_card_reward_pick_v1"
    );
    assert!(packet.is_some());
}

#[test]
fn run_control_auto_run_does_not_open_card_reward_item_with_singing_bowl() {
    let mut session = test_session_at_reward_items(vec![crate::state::rewards::RewardItem::Card {
        cards: vec![
            crate::state::rewards::RewardCard::new(crate::content::cards::CardId::Shockwave, 0),
            crate::state::rewards::RewardCard::new(crate::content::cards::CardId::Clash, 0),
            crate::state::rewards::RewardCard::new(crate::content::cards::CardId::SeverSoul, 0),
        ],
    }]);
    session
        .run_state
        .relics
        .push(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::SingingBowl,
        ));

    let outcome = session
        .apply_command(RunControlCommand::AutoRun(
            crate::eval::run_control::RunControlAutoStepOptions {
                max_operations: Some(1),
                ..Default::default()
            },
        ))
        .expect("auto-run should stop before opening a Singing Bowl card reward item");

    assert!(outcome
        .message
        .contains("Reason: card reward requires human choice"));
    assert!(outcome.message.contains("card reward policy stopped:"));
    assert!(outcome.trace_annotations.iter().any(|annotation| {
        matches!(
            annotation,
            crate::eval::run_control::RunControlTraceAnnotationV1::NonCombatPolicyDecision {
                record,
                ..
            } if record.site == crate::ai::noncombat_decision_v1::DecisionSiteKindV1::CardReward
                && record.selection.status
                    == crate::ai::noncombat_decision_v1::PolicySelectionStatusV1::Stopped
        )
    }));
    assert!(outcome.action_result.is_none());
    let EngineState::RewardScreen(reward) = &session.engine_state else {
        panic!("Singing Bowl card reward item should remain unopened");
    };
    assert!(reward.pending_card_choice.is_none());
    assert!(session
        .run_state
        .master_deck
        .iter()
        .all(|card| card.id != crate::content::cards::CardId::Shockwave));
}

#[test]
fn visible_singing_bowl_candidate_consumes_unopened_card_reward_item() {
    let mut session = test_session_at_reward_items(vec![crate::state::rewards::RewardItem::Card {
        cards: vec![
            crate::state::rewards::RewardCard::new(crate::content::cards::CardId::Shockwave, 0),
            crate::state::rewards::RewardCard::new(crate::content::cards::CardId::Clash, 0),
            crate::state::rewards::RewardCard::new(crate::content::cards::CardId::SeverSoul, 0),
        ],
    }]);
    session
        .run_state
        .relics
        .push(crate::content::relics::RelicState::new(
            crate::content::relics::RelicId::SingingBowl,
        ));
    let before_max_hp = session.run_state.max_hp;

    session
        .apply_command(RunControlCommand::Candidate("bowl".to_string()))
        .expect("bowl should consume the visible card reward item");

    assert_eq!(session.run_state.max_hp, before_max_hp + 2);
    assert!(session
        .run_state
        .master_deck
        .iter()
        .all(|card| card.id != crate::content::cards::CardId::Shockwave));
    if let EngineState::RewardScreen(reward) = &session.engine_state {
        assert!(reward.pending_card_choice.is_none());
        assert!(reward.items.is_empty());
    }
}

#[test]
fn run_control_auto_step_route_planner_advances_map_then_stops_at_combat() {
    let mut session = test_session_with_first_monster_room();

    let outcome = session
        .apply_command(RunControlCommand::AutoStep(
            crate::eval::run_control::RunControlAutoStepOptions {
                route: crate::eval::run_control::RunControlRouteAutomationMode::Planner,
                max_operations: Some(1),
                ..Default::default()
            },
        ))
        .expect("auto-step route planner should choose a map node");

    assert!(outcome.message.contains("route planner:"));
    assert!(outcome.message.contains("x="));
    assert!(outcome.message.contains("command=go"));
    assert!(outcome
        .message
        .contains("label_role=behavior_policy_not_teacher"));
    assert!(outcome
        .message
        .contains("Reason: operation budget exhausted at 1 automatic operations"));
    assert!(outcome.action_result.is_some());
    assert!(matches!(
        session.engine_state,
        EngineState::CombatPlayerTurn
    ));
    assert_eq!(session.run_state.map.current_y, 0);
}

#[test]
fn run_control_auto_step_route_planner_reports_auto_capture() {
    let root = unique_temp_dir("run_control_auto_step_route_auto_capture");
    let mut session = test_session_with_first_monster_room();
    session.auto_capture = AutoCombatCaptureConfig {
        enabled: true,
        root: Some(root.clone()),
    };

    let outcome = session
        .apply_command(RunControlCommand::AutoStep(
            crate::eval::run_control::RunControlAutoStepOptions {
                route: crate::eval::run_control::RunControlRouteAutomationMode::Planner,
                max_operations: Some(1),
                ..Default::default()
            },
        ))
        .expect("route planner should enter combat and auto-capture");

    assert!(outcome.message.contains("route planner:"));
    assert!(outcome.message.contains("auto capture:"));
    assert!(outcome.trace_annotations.iter().any(|annotation| matches!(
        annotation,
        RunControlTraceAnnotationV1::AutoCombatCapture { .. }
    )));
    assert!(outcome.trace_annotations.iter().any(|annotation| matches!(
        annotation,
        RunControlTraceAnnotationV1::RoutePlannerSelection { .. }
    )));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn run_control_auto_step_leaves_empty_shop() {
    let mut session = test_session_at_shop();
    if let EngineState::Shop(shop) = &mut session.engine_state {
        shop.cards.clear();
        shop.relics.clear();
        shop.potions.clear();
        shop.purge_available = false;
    }

    let outcome = session
        .apply_command(RunControlCommand::AutoStep(Default::default()))
        .expect("auto-step should leave a shop with no remaining executable choices");

    assert!(outcome
        .message
        .contains("routine: Leave shop (only shop exit remains)"));
    assert!(!matches!(session.engine_state, EngineState::Shop(_)));
}

#[test]
fn run_control_auto_step_claims_low_risk_rewards_then_stops() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut rewards = crate::state::rewards::RewardState::new();
    rewards.items = vec![
        crate::state::rewards::RewardItem::Gold { amount: 19 },
        crate::state::rewards::RewardItem::Potion {
            potion_id: crate::content::potions::PotionId::EssenceOfSteel,
        },
        crate::state::rewards::RewardItem::Card {
            cards: vec![crate::state::rewards::RewardCard::new(
                crate::content::cards::CardId::ShrugItOff,
                0,
            )],
        },
    ];
    session.engine_state = EngineState::RewardScreen(rewards);

    let outcome = session
        .apply_command(RunControlCommand::AutoStep(Default::default()))
        .expect("auto-step should claim deterministic rewards");

    assert!(outcome
        .message
        .contains("routine reward: 19 gold, Essence of Steel potion"));
    assert!(outcome
        .message
        .contains("Reason: card reward requires human choice"));
    assert_eq!(session.run_state.gold, 118);
    assert_eq!(
        session.run_state.potions[0]
            .as_ref()
            .map(|potion| potion.id),
        Some(crate::content::potions::PotionId::EssenceOfSteel)
    );
    assert!(outcome.action_result.is_some());
}

#[test]
fn run_control_auto_step_solves_starter_combat_and_stops_at_reward_choice() {
    let mut session = test_session_with_first_monster_room();
    session
        .apply_command(RunControlCommand::Input(ClientInput::SelectMapNode(0)))
        .expect("map input should enter combat");

    let outcome = session
        .apply_command(RunControlCommand::AutoStep(
            crate::eval::run_control::RunControlAutoStepOptions {
                search: crate::eval::run_control::RunControlSearchCombatOptions {
                    max_nodes: Some(2_000),
                    wall_ms: Some(5_000),
                    ..Default::default()
                },
                ..Default::default()
            },
        ))
        .expect("auto-step should resolve starter combat");

    assert!(outcome
        .message
        .contains("combat search: search-combat applied"));
    assert!(
        outcome
            .message
            .contains("Reason: remaining reward requires human choice")
            || outcome
                .message
                .contains("Reason: card reward requires human choice")
    );
    assert!(outcome.action_result.is_some());
    assert!(session.active_combat.is_none());
    assert_eq!(
        session
            .last_combat_baseline()
            .map(CombatBaselineOutcomeV1::terminal),
        Some(crate::sim::combat::CombatTerminal::Win)
    );
}

#[test]
fn run_control_case_command_saves_diagnostic_decision_case() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let dir = unique_temp_dir("run_control_decision_case");
    fs::create_dir_all(&dir).expect("temp dir should be created");
    let path = dir.join("decision.json");

    let outcome = session
        .apply_command(RunControlCommand::SaveDecisionCase {
            path: Some(path.clone()),
        })
        .expect("case command should save");

    assert!(outcome.message.contains("saved RunDecisionCaseV1"));
    assert!(
        outcome.action_result.is_none(),
        "non-action commands should not fabricate action results"
    );
    let payload = fs::read_to_string(&path).expect("decision case should exist");
    assert!(payload.contains("\"schema_name\": \"sts_simulator.run_decision_case\""));
    assert!(payload.contains("\"label_role\": \"diagnostic_not_teacher_label\""));
    assert!(payload.contains("\"trainable_as_action_label\": false"));
    assert!(payload.contains("\"policy_quality_claim\": false"));
    assert!(payload.contains("\"resolution\""));

    let _ = fs::remove_file(path);
    let _ = fs::remove_dir(dir);
}

#[test]
fn run_control_visible_candidate_command_advances_current_screen() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let outcome = session
        .apply_command(RunControlCommand::DefaultCandidate)
        .expect("single visible Neow intro action should execute");

    assert!(outcome.message.contains("Neow Bonus"));
    let action_result = outcome
        .action_result
        .as_ref()
        .expect("state-changing commands should return a structured action result");
    assert!(action_result.changes.iter().any(|change| matches!(
        change,
        crate::eval::run_control::RunActionResultChangeV1::AdvancedTo { title }
            if title == "Neow Bonus"
    )));
    let json = serde_json::to_string(action_result)
        .expect("structured action result should be serializable");
    assert!(json.contains("advanced_to"));
    assert_eq!(session.decision_step, 1);
    assert_eq!(
        session
            .run_state
            .event_state
            .as_ref()
            .map(|event| event.current_screen),
        Some(1)
    );
}

#[test]
fn run_control_rejects_proceed_alias_on_neow_intro() {
    let mut session = RunControlSession::new(RunControlConfig::default());

    let err = session
        .apply_command(RunControlCommand::Input(ClientInput::Proceed))
        .expect_err("raw proceed must not be accepted on the Neow intro event screen");

    assert!(err.contains("input `proceed` is not valid"));
    assert!(err.contains("Neow Intro"));
    assert_eq!(session.decision_step, 0);
    assert!(matches!(session.engine_state, EngineState::EventRoom));
    assert_eq!(
        session
            .run_state
            .event_state
            .as_ref()
            .map(|event| event.current_screen),
        Some(0)
    );
}

#[test]
fn run_control_rejects_reward_command_on_neow_intro() {
    let mut session = RunControlSession::new(RunControlConfig::default());

    let err = session
        .apply_command(RunControlCommand::Input(ClientInput::ClaimReward(0)))
        .expect_err("reward claim must not be accepted on an event screen");

    assert!(err.contains("input `claim 0` is not valid"));
    assert!(err.contains("Neow Intro"));
    assert_eq!(session.decision_step, 0);
    assert!(matches!(session.engine_state, EngineState::EventRoom));
}

#[test]
fn run_control_rejects_map_travel_before_neow_is_complete() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session
        .apply_command(RunControlCommand::DefaultCandidate)
        .expect("Neow intro should advance");

    let err = session
        .apply_command(RunControlCommand::Input(ClientInput::SelectMapNode(0)))
        .expect_err("Neow bonus should not allow first-room travel");

    assert!(err.contains("input `go 0` is not valid"));
    assert!(err.contains("Neow Bonus"));
    assert!(matches!(session.engine_state, EngineState::EventRoom));
}

#[test]
fn run_control_shop_accepts_visible_candidate_ids_and_contextual_words() {
    let mut session = test_session_at_shop();

    let outcome = session
        .apply_command(RunControlCommand::Candidate("card-0".to_string()))
        .expect("visible shop card id should buy");
    assert!(outcome.message.contains("Added card: Armaments"));
    assert_eq!(session.run_state.gold, 51);

    let mut session = test_session_at_shop();
    let outcome = session
        .apply_command(RunControlCommand::CardIndex(1))
        .expect("card <idx> should buy in shop");
    assert!(outcome.message.contains("Added card: Shrug It Off"));
    assert_eq!(session.run_state.gold, 50);

    let mut session = test_session_at_shop();
    let outcome = session
        .apply_command(RunControlCommand::Candidate("1".to_string()))
        .expect("bare numeric shop id should fall back to card-<idx>");
    assert!(outcome.message.contains("Added card: Shrug It Off"));
    assert_eq!(session.run_state.gold, 50);
}

#[test]
fn run_control_shop_leave_candidate_exits_shop() {
    let mut session = test_session_at_shop();

    let outcome = session
        .apply_command(RunControlCommand::Candidate("leave".to_string()))
        .expect("visible leave id should leave shop");

    assert!(outcome.message.contains("Chose: Leave shop"));
    assert!(!matches!(session.engine_state, EngineState::Shop(_)));
}

#[test]
fn run_control_shop_rewards_candidate_reopens_pending_overlay() {
    let mut session = test_session_at_shop();
    let EngineState::Shop(shop) = &mut session.engine_state else {
        panic!("test session should start in shop");
    };
    let mut pending = crate::state::rewards::RewardState::new();
    pending.items = vec![crate::state::rewards::RewardItem::Card {
        cards: vec![crate::state::rewards::RewardCard::new(
            crate::content::cards::CardId::Shockwave,
            0,
        )],
    }];
    shop.pending_reward_overlay = Some(pending);

    let outcome = session
        .apply_command(parse_run_control_command("rewards").expect("rewards should parse"))
        .expect("visible rewards id should reopen overlay");

    assert!(outcome.message.contains("Chose: Open pending rewards"));
    let EngineState::RewardOverlay {
        reward_state,
        return_state,
    } = &session.engine_state
    else {
        panic!("expected reward overlay");
    };
    assert!(matches!(
        reward_state.items.as_slice(),
        [crate::state::rewards::RewardItem::Card { .. }]
    ));
    let EngineState::Shop(return_shop) = return_state.as_ref() else {
        panic!("overlay should return to shop");
    };
    assert!(return_shop.pending_reward_overlay.is_none());
}

#[test]
fn run_control_campfire_accepts_bare_smith_index_alias() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.engine_state = EngineState::Campfire;

    let outcome = session
        .apply_command(RunControlCommand::Candidate("8".to_string()))
        .expect("bare numeric campfire id should fall back to smith-<idx>");

    assert!(outcome.message.contains("Chose: Smith Defend"));
}

#[test]
fn visible_candidate_alias_resolves_label_leave_and_skip() {
    use crate::eval::run_control::view_model::{CandidateAction, DecisionCandidate};

    let candidates = vec![
        DecisionCandidate {
            id: "0".to_string(),
            label: "Leave.".to_string(),
            action: CandidateAction::Input(ClientInput::EventChoice(0)),
            note: None,
            resolution: None,
        },
        DecisionCandidate {
            id: "1".to_string(),
            label: "Skip card reward".to_string(),
            action: CandidateAction::Input(ClientInput::Proceed),
            note: None,
            resolution: None,
        },
    ];

    assert_eq!(
        decision_surface::resolve_candidate_alias(&candidates, &EngineState::EventRoom, "leave")
            .map(|candidate| candidate.id.as_str()),
        Some("0")
    );
    assert_eq!(
        decision_surface::resolve_candidate_alias(
            &candidates,
            &EngineState::RewardScreen(Default::default()),
            "skip"
        )
        .map(|candidate| candidate.id.as_str()),
        Some("1")
    );
}

#[test]
fn run_control_campfire_renders_all_upgradeable_smith_targets() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.engine_state = EngineState::Campfire;

    let rendered = render_run_control_state(&session);

    assert!(rendered.contains("smith-9 | Smith Bash"));
    assert!(
        rendered.contains("smith-8 | Smith Defend"),
        "campfire smith candidates must not truncate after the first eight deck cards"
    );
}

fn test_session_with_first_monster_room() -> RunControlSession {
    let mut session = test_session_after_neow_at_map();
    let mut first = MapRoomNode::new(0, 0);
    first.class = Some(RoomType::MonsterRoom);
    first.edges.insert(MapEdge::new(0, 0, 0, 1));
    let mut second = MapRoomNode::new(0, 1);
    second.class = Some(RoomType::MonsterRoom);
    session.run_state.map = MapState::new(vec![vec![first], vec![second]]);
    session.run_state.monster_list = vec![EncounterId::JawWorm, EncounterId::Cultist];
    session
}

fn test_session_with_forced_unsafe_elite_route() -> RunControlSession {
    let mut session = test_session_after_neow_at_map();
    session.run_state.current_hp = 1;
    let mut first = MapRoomNode::new(0, 0);
    first.class = Some(RoomType::MonsterRoomElite);
    first.edges.insert(MapEdge::new(0, 0, 0, 1));
    let mut second = MapRoomNode::new(0, 1);
    second.class = Some(RoomType::MonsterRoom);
    session.run_state.map = MapState::new(vec![vec![first], vec![second]]);
    session
}

fn test_session_at_campfire_with_hp(current_hp: i32, max_hp: i32) -> RunControlSession {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.engine_state = EngineState::Campfire;
    session.run_state.current_hp = current_hp;
    session.run_state.max_hp = max_hp;
    let mut rest = MapRoomNode::new(0, 0);
    rest.class = Some(RoomType::RestRoom);
    rest.edges.insert(MapEdge::new(0, 0, 0, 1));
    let mut next = MapRoomNode::new(0, 1);
    next.class = Some(RoomType::MonsterRoom);
    session.run_state.map = MapState::new(vec![vec![rest], vec![next]]);
    session.run_state.map.current_x = 0;
    session.run_state.map.current_y = 0;
    session
}

fn test_session_at_shop() -> RunControlSession {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.event_state = None;
    session.run_state.gold = 100;
    let mut shop = crate::state::shop::ShopState::new();
    shop.cards = vec![
        crate::state::shop::ShopCard {
            card_id: crate::content::cards::CardId::Armaments,
            upgrades: 0,
            price: 49,
            can_buy: true,
            blocked_reason: None,
        },
        crate::state::shop::ShopCard {
            card_id: crate::content::cards::CardId::ShrugItOff,
            upgrades: 0,
            price: 50,
            can_buy: true,
            blocked_reason: None,
        },
    ];
    session.engine_state = EngineState::Shop(shop);
    session
}

fn test_session_at_card_reward(card_ids: Vec<crate::content::cards::CardId>) -> RunControlSession {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.event_state = None;
    let cards = card_ids
        .into_iter()
        .map(|card_id| crate::state::rewards::RewardCard::new(card_id, 0))
        .collect::<Vec<_>>();
    let mut reward = crate::state::rewards::RewardState::new();
    reward.items = vec![crate::state::rewards::RewardItem::Card {
        cards: cards.clone(),
    }];
    reward.pending_card_choice = Some(cards);
    reward.pending_card_reward_index = Some(0);
    session.engine_state = EngineState::RewardScreen(reward);
    session
}

fn match_and_keep_board_with_entries(
    entries: &[(crate::content::cards::CardId, u8); 6],
) -> Vec<i32> {
    let mut extra_data = vec![0, 0, 1, 2, 3, 4, 5, 1, 2, 3, 4, 5, 0, 5, -1];
    for &(card_id, upgrades) in entries {
        extra_data.push(card_id as i32);
        extra_data.push(upgrades as i32);
    }
    extra_data.push(-1);
    extra_data.push(-1);
    extra_data
}

fn test_card_reward_calibration_for_twin_strike(
) -> crate::eval::card_reward_value_loop::CardRewardOutcomeCalibrationV1 {
    crate::eval::card_reward_value_loop::CardRewardOutcomeCalibrationV1 {
        schema_name:
            crate::eval::card_reward_value_loop::CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_NAME
                .to_string(),
        schema_version:
            crate::eval::card_reward_value_loop::CARD_REWARD_OUTCOME_CALIBRATION_SCHEMA_VERSION,
        label_role: "diagnostic_not_teacher_label".to_string(),
        trainable_as_action_label: false,
        policy_quality_claim: false,
        estimator_kind: "selected_outcome_card_id_prior_v1".to_string(),
        provenance: Default::default(),
        total_examples: 4,
        usable_outcome_examples: 4,
        missing_outcome_examples: 0,
        global: crate::eval::card_reward_value_loop::CardRewardOutcomeCalibrationGlobalV1 {
            selected_count: 4,
            outcome_attached_count: 4,
            mean_next_combat_hp_loss: Some(12.0),
            picked_card_drawn_observation_count: 0,
            mean_picked_card_drawn_count: None,
            picked_card_played_observation_count: 0,
            mean_picked_card_played_count: None,
        },
        card_id_buckets: vec![
            crate::eval::card_reward_value_loop::CardRewardOutcomeCalibrationBucketV1 {
                bucket_key: "card_id:TwinStrike".to_string(),
                card_id: "TwinStrike".to_string(),
                selected_count: 4,
                outcome_attached_count: 4,
                missing_outcome_count: 0,
                mean_next_combat_hp_loss: Some(6.0),
                hp_loss_bucket_counts: Vec::new(),
                upgraded_count: 0,
                removed_count: 0,
                picked_card_drawn_observation_count: 0,
                mean_picked_card_drawn_count: None,
                picked_card_played_observation_count: 0,
                mean_picked_card_played_count: None,
                confidence: 0.8,
                uncertainty: 0.15,
                usable_for_value_estimate: true,
                usable_for_autopilot_gate: false,
            },
        ],
    }
}

fn test_session_at_reward_items(
    items: Vec<crate::state::rewards::RewardItem>,
) -> RunControlSession {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.event_state = None;
    let mut reward = crate::state::rewards::RewardState::new();
    reward.items = items;
    session.engine_state = EngineState::RewardScreen(reward);
    session
}

fn test_session_after_neow_at_map() -> RunControlSession {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.event_state = None;
    session.engine_state = EngineState::MapNavigation;
    session
}

fn noncombat_human_boundary_record(
    outcome: &RunControlCommandOutcome,
) -> &crate::ai::noncombat_decision_v1::NonCombatDecisionRecordV1 {
    let record = outcome
        .trace_annotations
        .iter()
        .find_map(|annotation| match annotation {
            RunControlTraceAnnotationV1::NonCombatHumanBoundary { record } => Some(record),
            _ => None,
        })
        .expect("outcome should carry a noncombat human boundary record");
    crate::ai::noncombat_decision_v1::validate_noncombat_decision_record_v1(record)
        .expect("noncombat human boundary record should validate");
    record
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{label}_{}_{}", std::process::id(), nanos))
}
