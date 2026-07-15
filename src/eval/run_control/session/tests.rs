use super::*;
use crate::content::monsters::factory::EncounterId;
use crate::eval::run_control::decision_surface;
use crate::eval::run_control::{
    render_run_control_details, render_run_control_state, CombatBaselineOutcomeV1,
    RunDecisionAction,
};
use crate::state::core::ClientInput;
use crate::state::map::node::{MapEdge, MapRoomNode, RoomType};
use crate::state::map::state::MapState;
use std::fs;
use std::path::PathBuf;
use std::time::{SystemTime, UNIX_EPOCH};

#[test]
fn run_control_search_combat_applies_complete_winning_trajectory() {
    let mut session = test_session_with_first_monster_room();
    session
        .apply_decision_action(RunDecisionAction::Input(ClientInput::SelectMapNode(0)))
        .expect("map input should enter combat");

    let outcome = session
        .apply_combat_search(crate::eval::run_control::RunControlSearchCombatOptions {
            max_nodes: Some(2_000),
            wall_ms: Some(5_000),
            ..Default::default()
        })
        .expect("search-combat should resolve starter combat");

    assert!(outcome
        .message
        .contains("Search combat applied complete winning candidate"));
    assert!(outcome.message.contains("coverage_status="));
    assert!(outcome
        .message
        .contains("frontier_policy=round_robin_eval_buckets"));
    assert!(outcome.message.contains("search_diagnostics="));
    assert!(outcome.message.contains("search_performance="));
    assert!(outcome.message.contains("turn_plan_seeded="));
    assert!(outcome.message.contains("pending_high_fanout="));
    assert!(outcome.action_result.is_some());
    let accepted =
        crate::eval::run_control::accepted_combat_line_evidence_v1(&outcome.trace_annotations)
            .expect("applied search combat should expose original and selected line evidence");
    assert_eq!(accepted.original.terminal, accepted.selected.terminal);
    assert!(session.active_combat.is_none());
    assert_eq!(
        session
            .last_combat_baseline()
            .map(CombatBaselineOutcomeV1::terminal),
        Some(crate::sim::combat::CombatTerminal::Win)
    );
}

#[test]
fn run_control_auto_step_advances_routine_neow_intro_only() {
    let mut session = RunControlSession::new(RunControlConfig::default());

    let outcome = session
        .apply_progress_step(Default::default())
        .expect("auto-step should advance routine intro");

    assert!(outcome.message.contains("routine: Proceed"));
    assert!(outcome
        .message
        .contains("Reason: one atomic progress step applied"));
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
fn run_control_auto_step_neow_stop_exports_human_boundary_record() {
    let mut session = RunControlSession::new(RunControlConfig::default());

    let outcome = session
        .apply_progress_step(Default::default())
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
        .apply_progress_step(Default::default())
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
fn run_control_auto_step_labels_single_event_state_resolution_as_forced() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut event_state =
        crate::state::events::EventState::new(crate::state::events::EventId::CursedTome);
    event_state.current_screen = 1;
    session.run_state.event_state = Some(event_state);
    session.engine_state = EngineState::EventRoom;

    let outcome = session
        .apply_progress_step(crate::eval::run_control::RunControlAutoStepOptions {
            ..Default::default()
        })
        .expect("auto-step should apply the forced Cursed Tome damage screen");

    assert!(
        outcome
            .message
            .contains("forced: [Continue] Take 1 damage. (forced event resolution)"),
        "{}",
        outcome.message
    );
    assert!(!outcome.message.contains("single safe event transition"));
    assert_eq!(session.run_state.current_hp, 79);
}

#[test]
fn run_control_progress_step_stops_at_event_owner_boundary() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.run_state.gold = 0;
    session.run_state.event_state = Some(crate::state::events::EventState::new(
        crate::state::events::EventId::GoldenShrine,
    ));
    session.engine_state = EngineState::EventRoom;

    let outcome = session
        .apply_progress_step(crate::eval::run_control::RunControlAutoStepOptions {
            route: crate::eval::run_control::RunControlRouteAutomationMode::Planner,
            ..Default::default()
        })
        .expect("progress step should stop at the event owner boundary");

    assert_eq!(
        outcome.auto_stop.as_ref().map(|stop| stop.kind),
        Some(RunControlAutoStopKind::HumanBoundary)
    );
    assert!(outcome.action_result.is_none());
    assert_eq!(session.run_state.gold, 0);
    assert!(matches!(session.engine_state, EngineState::EventRoom));
    assert_eq!(
        session
            .run_state
            .event_state
            .as_ref()
            .unwrap()
            .current_screen,
        0
    );
    let record = noncombat_human_boundary_record(&outcome);
    assert_eq!(
        record.site,
        crate::ai::noncombat_decision_v1::DecisionSiteKindV1::Event
    );
    assert_eq!(
        record.data_role,
        crate::ai::noncombat_decision_v1::DataRoleV1::HumanBoundaryNotTeacher
    );
}

#[test]
fn run_control_auto_step_collapses_terminal_event_leave_to_map() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let mut event_state =
        crate::state::events::EventState::new(crate::state::events::EventId::ScrapOoze);
    event_state.current_screen = 1;
    session.run_state.event_state = Some(event_state);
    session.engine_state = EngineState::EventRoom;

    let outcome = session
        .apply_progress_step(crate::eval::run_control::RunControlAutoStepOptions {
            route: crate::eval::run_control::RunControlRouteAutomationMode::Planner,
            ..Default::default()
        })
        .expect("auto-step should collapse the terminal Scrap Ooze leave screen");

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
        .apply_progress_step(Default::default())
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
fn run_control_auto_step_campfire_stop_exports_human_boundary_record() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.engine_state = EngineState::Campfire;

    let outcome = session
        .apply_progress_step(Default::default())
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
fn run_control_progress_step_stops_at_low_hp_campfire_without_choosing() {
    let mut session = test_session_at_campfire_with_hp(20, 80);

    let outcome = session
        .apply_progress_step(crate::eval::run_control::RunControlAutoStepOptions {
            ..Default::default()
        })
        .expect("progress step should stop at the campfire owner boundary");

    assert_eq!(
        outcome.auto_stop.as_ref().map(|stop| stop.kind),
        Some(RunControlAutoStopKind::HumanBoundary)
    );
    assert!(outcome.action_result.is_none());
    assert_eq!(session.run_state.current_hp, 20);
    assert!(matches!(session.engine_state, EngineState::Campfire));
    let record = noncombat_human_boundary_record(&outcome);
    assert_eq!(
        record.site,
        crate::ai::noncombat_decision_v1::DecisionSiteKindV1::Campfire
    );
    assert_eq!(
        record.data_role,
        crate::ai::noncombat_decision_v1::DataRoleV1::HumanBoundaryNotTeacher
    );
}

#[test]
fn run_control_progress_step_stops_at_shop_owner_boundary() {
    let mut session = test_session_at_shop();
    session
        .run_state
        .add_card_to_deck_without_interception_from(
            crate::content::cards::CardId::Doubt,
            0,
            crate::state::selection::DomainEventSource::DeckMutation,
        );

    let outcome = session
        .apply_progress_step(crate::eval::run_control::RunControlAutoStepOptions {
            ..Default::default()
        })
        .expect("progress step should stop at the shop owner boundary");

    assert_eq!(
        outcome.auto_stop.as_ref().map(|stop| stop.kind),
        Some(RunControlAutoStopKind::HumanBoundary)
    );
    assert!(outcome.action_result.is_none());
    let record = noncombat_human_boundary_record(&outcome);
    assert_eq!(
        record.site,
        crate::ai::noncombat_decision_v1::DecisionSiteKindV1::Shop
    );
    assert_eq!(
        record.data_role,
        crate::ai::noncombat_decision_v1::DataRoleV1::HumanBoundaryNotTeacher
    );
    assert_eq!(session.run_state.gold, 100);
    assert!(session
        .run_state
        .master_deck
        .iter()
        .any(|card| card.id == crate::content::cards::CardId::Doubt));
    assert!(matches!(session.engine_state, EngineState::Shop(_)));
}

#[test]
fn run_control_progress_step_reopens_pending_shop_rewards() {
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
        .apply_progress_step(crate::eval::run_control::RunControlAutoStepOptions {
            ..Default::default()
        })
        .expect("progress step should reopen pending rewards");

    assert!(outcome.message.contains("routine: Open pending rewards"));
    assert!(
        session
            .run_state
            .master_deck
            .iter()
            .any(|card| card.id == crate::content::cards::CardId::Doubt),
        "shop owner action must not run before pending overlay rewards are restored"
    );
    assert!(matches!(
        session.engine_state,
        EngineState::RewardOverlay { .. }
    ));
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
        .apply_decision_action(
            crate::eval::run_control::RunDecisionAction::SkipCardReward {
                reward_item_index: 0,
            },
        )
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
fn run_control_progress_step_stops_at_run_choice_owner_boundary() {
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
            source: crate::state::selection::DomainEventSource::Selection(
                crate::state::core::RunPendingChoiceReason::PurgeNonBottled.into(),
            ),
            return_state: Box::new(EngineState::MapNavigation),
        });

    let outcome = session
        .apply_progress_step(crate::eval::run_control::RunControlAutoStepOptions {
            ..Default::default()
        })
        .expect("progress step should stop at the run-choice owner boundary");

    assert_eq!(
        outcome.auto_stop.as_ref().map(|stop| stop.kind),
        Some(RunControlAutoStopKind::HumanBoundary)
    );
    assert!(outcome.action_result.is_none());
    let record = noncombat_human_boundary_record(&outcome);
    assert_eq!(
        record.site,
        crate::ai::noncombat_decision_v1::DecisionSiteKindV1::RunChoice
    );
    assert_eq!(
        record.data_role,
        crate::ai::noncombat_decision_v1::DataRoleV1::HumanBoundaryNotTeacher
    );
    assert!(session
        .run_state
        .master_deck
        .iter()
        .any(|card| card.id == crate::content::cards::CardId::Doubt));
    assert!(matches!(
        session.engine_state,
        EngineState::RunPendingChoice(_)
    ));
}

#[test]
fn run_control_details_include_deck_mutation_compiler_groups() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session
        .run_state
        .master_deck
        .push(crate::runtime::combat::CombatCard::new(
            crate::content::cards::CardId::TrueGrit,
            99,
        ));
    session.engine_state =
        EngineState::RunPendingChoice(crate::state::core::RunPendingChoiceState {
            min_choices: 1,
            max_choices: 1,
            reason: crate::state::core::RunPendingChoiceReason::Purge,
            source: crate::state::selection::DomainEventSource::Selection(
                crate::state::core::RunPendingChoiceReason::Purge.into(),
            ),
            return_state: Box::new(EngineState::MapNavigation),
        });

    let rendered = render_run_control_details(&session);

    assert!(rendered.contains("branch_active:"));
    assert!(rendered.contains("inspect_only:"));
    assert!(rendered.contains("True Grit"));
    assert!(rendered.contains("role=InspectOnly"));
}

#[test]
fn run_control_progress_step_executes_single_forced_run_pending_choice() {
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
            source: crate::state::selection::DomainEventSource::Selection(
                crate::state::core::RunPendingChoiceReason::Upgrade.into(),
            ),
            return_state: Box::new(EngineState::MapNavigation),
        });

    session
        .apply_progress_step(crate::eval::run_control::RunControlAutoStepOptions {
            ..Default::default()
        })
        .expect("progress step should execute a single forced run pending choice");

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
        .apply_progress_step(Default::default())
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
fn run_control_progress_step_stops_on_high_agency_boss_relic_choice() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    session.engine_state =
        EngineState::BossRelicSelect(crate::state::rewards::BossRelicChoiceState::new(vec![
            crate::content::relics::RelicId::TinyHouse,
            crate::content::relics::RelicId::RunicPyramid,
            crate::content::relics::RelicId::SneckoEye,
        ]));

    let outcome = session
        .apply_progress_step(crate::eval::run_control::RunControlAutoStepOptions {
            route: crate::eval::run_control::RunControlRouteAutomationMode::Planner,
            ..Default::default()
        })
        .expect("progress step should stop for high-agency boss relic choices");

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
            source: crate::state::selection::DomainEventSource::Selection(
                crate::state::core::RunPendingChoiceReason::Upgrade.into(),
            ),
            return_state: Box::new(EngineState::MapNavigation),
        });

    let outcome = session
        .apply_progress_step(Default::default())
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
        .apply_progress_step(Default::default())
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
fn run_control_route_plan_keeps_manual_safety_gate_when_all_routes_are_forced_risk() {
    let mut session = test_session_with_forced_unsafe_elite_route();
    let before = (
        session.run_state.map.current_x,
        session.run_state.map.current_y,
        session.run_state.current_hp,
    );

    let err = session
        .apply_route_plan()
        .expect_err("manual route planning should keep the safety gate");

    assert!(err.contains("route planner selected only reject-unless-forced routes"));
    assert_eq!(
        before,
        (
            session.run_state.map.current_x,
            session.run_state.map.current_y,
            session.run_state.current_hp
        )
    );
}

#[test]
fn run_control_auto_step_applies_forced_route_when_no_safe_alternative_exists() {
    let mut session = test_session_with_forced_unsafe_elite_route();
    let before = (
        session.run_state.map.current_x,
        session.run_state.map.current_y,
        session.run_state.current_hp,
    );

    let outcome = session
        .apply_progress_step(crate::eval::run_control::RunControlAutoStepOptions {
            route: crate::eval::run_control::RunControlRouteAutomationMode::Planner,
            ..Default::default()
        })
        .expect("auto-step should apply the least-bad forced route");

    assert!(outcome
        .message
        .contains("route planner: x=0 Elite [reject_unless_forced"));
    assert!(!outcome
        .message
        .contains("route planner declined automatic map selection"));
    assert_ne!(
        before,
        (
            session.run_state.map.current_x,
            session.run_state.map.current_y,
            session.run_state.current_hp
        )
    );
    assert!(outcome.action_result.is_some());
    let record = outcome
        .trace_annotations
        .iter()
        .find_map(|annotation| match annotation {
            RunControlTraceAnnotationV1::RoutePlannerSelection {
                noncombat_record, ..
            } => noncombat_record.as_ref(),
            _ => None,
        })
        .expect("forced route planner application should attach a noncombat policy record");
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
        crate::ai::noncombat_decision_v1::PolicySelectionStatusV1::Selected
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
        .apply_progress_step(crate::eval::run_control::RunControlAutoStepOptions {
            route: crate::eval::run_control::RunControlRouteAutomationMode::Planner,
            ..Default::default()
        })
        .expect("map overlay back should be routine automation");

    assert!(outcome.message.contains("Back to reward screen"));
    assert!(!outcome
        .message
        .contains("route planner declined automatic map selection"));
    assert!(outcome
        .message
        .contains("Reason: one atomic progress step applied"));
    assert!(matches!(session.engine_state, EngineState::RewardScreen(_)));
}

#[test]
fn run_control_progress_step_can_use_route_planner() {
    let mut session = test_session_with_first_monster_room();

    let outcome = session
        .apply_progress_step(crate::eval::run_control::RunControlAutoStepOptions {
            route: crate::eval::run_control::RunControlRouteAutomationMode::Planner,
            ..Default::default()
        })
        .expect("progress step should use route planner");

    assert!(outcome
        .message
        .contains("Advanced to human boundary: Combat"));
    assert!(outcome
        .message
        .contains("Reason: one atomic progress step applied"));
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
fn run_control_progress_step_claims_safe_relic_reward_with_policy_annotation() {
    let mut session =
        test_session_at_reward_items(vec![crate::state::rewards::RewardItem::Relic {
            relic_id: crate::content::relics::RelicId::Anchor,
        }]);

    let outcome = session
        .apply_progress_step(crate::eval::run_control::RunControlAutoStepOptions {
            ..Default::default()
        })
        .expect("progress step should claim a safe relic reward");

    let changes = auto_reward_changes(&outcome).expect("reward automation should report changes");
    assert!(changes.contains(
        &crate::eval::run_control::RunActionResultChangeV1::RelicGained {
            relic: crate::content::relics::RelicId::Anchor
        }
    ));
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
fn run_control_progress_step_keeps_relic_reward_when_sapphire_key_is_available() {
    let mut session = test_session_at_reward_items(vec![
        crate::state::rewards::RewardItem::Relic {
            relic_id: crate::content::relics::RelicId::Anchor,
        },
        crate::state::rewards::RewardItem::SapphireKey,
    ]);

    let outcome = session
        .apply_progress_step(crate::eval::run_control::RunControlAutoStepOptions {
            ..Default::default()
        })
        .expect("progress step should stop for sapphire/relic choice");

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
fn run_control_progress_step_stops_on_card_reward_with_singing_bowl() {
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
        .apply_progress_step(crate::eval::run_control::RunControlAutoStepOptions {
            ..Default::default()
        })
        .expect("progress step should stop when Singing Bowl adds a strategic card reward option");

    assert!(outcome
        .message
        .contains("Reason: card reward requires human choice"));
    assert_eq!(
        outcome.auto_stop.as_ref().map(|stop| stop.kind),
        Some(RunControlAutoStopKind::HumanBoundary)
    );
    let record = noncombat_human_boundary_record(&outcome);
    assert_eq!(
        record.site,
        crate::ai::noncombat_decision_v1::DecisionSiteKindV1::CardReward
    );
    assert!(!session
        .run_state
        .master_deck
        .iter()
        .any(|card| card.id == crate::content::cards::CardId::Shockwave));
    assert!(outcome.action_result.is_none());
}

#[test]
fn run_control_manual_card_reward_pick_selects_card_with_noncombat_record() {
    let mut session = test_session_at_card_reward(vec![
        crate::content::cards::CardId::Shockwave,
        crate::content::cards::CardId::Clash,
        crate::content::cards::CardId::SeverSoul,
    ]);

    let outcome = session
        .apply_candidate_id("1")
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
fn run_control_progress_step_does_not_open_card_reward_item_with_singing_bowl() {
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
        .apply_progress_step(crate::eval::run_control::RunControlAutoStepOptions {
            ..Default::default()
        })
        .expect("progress step should stop before opening a Singing Bowl card reward item");

    assert!(outcome
        .message
        .contains("Reason: card reward requires human choice"));
    assert_eq!(
        outcome.auto_stop.as_ref().map(|stop| stop.kind),
        Some(RunControlAutoStopKind::HumanBoundary)
    );
    let record = noncombat_human_boundary_record(&outcome);
    assert_eq!(
        record.site,
        crate::ai::noncombat_decision_v1::DecisionSiteKindV1::Reward
    );
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

    let bowl_action = crate::eval::run_control::build_decision_surface(&session)
        .view
        .candidates
        .into_iter()
        .find(|candidate| candidate.id == "bowl")
        .and_then(|candidate| candidate.action.executable_action())
        .expect("bowl should be an executable typed action");
    session
        .apply_decision_action(bowl_action)
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
        .apply_progress_step(crate::eval::run_control::RunControlAutoStepOptions {
            route: crate::eval::run_control::RunControlRouteAutomationMode::Planner,
            ..Default::default()
        })
        .expect("auto-step route planner should choose a map node");

    assert!(outcome.message.contains("route planner:"));
    assert!(outcome.message.contains("x="));
    assert!(outcome.message.contains("command=go"));
    assert!(outcome
        .message
        .contains("label_role=behavior_policy_not_teacher"));
    assert!(outcome
        .message
        .contains("Reason: one atomic progress step applied"));
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
        .apply_progress_step(crate::eval::run_control::RunControlAutoStepOptions {
            route: crate::eval::run_control::RunControlRouteAutomationMode::Planner,
            ..Default::default()
        })
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
        .apply_progress_step(Default::default())
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
        .apply_progress_step(Default::default())
        .expect("auto-step should claim deterministic rewards");

    let changes = auto_reward_changes(&outcome).expect("reward automation should report changes");
    assert!(changes.contains(
        &crate::eval::run_control::RunActionResultChangeV1::GoldChanged {
            before: 99,
            after: 118
        }
    ));
    assert!(changes.contains(
        &crate::eval::run_control::RunActionResultChangeV1::PotionGained {
            potion: crate::content::potions::PotionId::EssenceOfSteel,
            slot: 0
        }
    ));
    assert!(outcome
        .message
        .contains("Reason: one atomic progress step applied"));
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
fn run_control_auto_step_resolves_one_starter_combat() {
    let mut session = test_session_with_first_monster_room();
    session
        .apply_decision_action(RunDecisionAction::Input(ClientInput::SelectMapNode(0)))
        .expect("map input should enter combat");

    let outcome = session
        .apply_progress_step(crate::eval::run_control::RunControlAutoStepOptions {
            search: crate::eval::run_control::RunControlSearchCombatOptions {
                max_nodes: Some(2_000),
                wall_ms: Some(5_000),
                ..Default::default()
            },
            ..Default::default()
        })
        .expect("auto-step should resolve starter combat");

    assert!(outcome
        .message
        .contains("combat search: search-combat applied"));
    assert!(outcome
        .message
        .contains("Reason: one atomic progress step applied"));
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
fn run_control_visible_candidate_command_advances_current_screen() {
    let mut session = RunControlSession::new(RunControlConfig::default());
    let outcome = session
        .apply_only_candidate()
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
        .apply_decision_action(RunDecisionAction::Input(ClientInput::Proceed))
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
        .apply_decision_action(RunDecisionAction::Input(ClientInput::ClaimReward(0)))
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
        .apply_only_candidate()
        .expect("Neow intro should advance");

    let err = session
        .apply_decision_action(RunDecisionAction::Input(ClientInput::SelectMapNode(0)))
        .expect_err("Neow bonus should not allow first-room travel");

    assert!(err.contains("input `go 0` is not valid"));
    assert!(err.contains("Neow Bonus"));
    assert!(matches!(session.engine_state, EngineState::EventRoom));
}

#[test]
fn run_control_shop_leave_candidate_exits_shop() {
    let mut session = test_session_at_shop();

    let outcome = session
        .apply_candidate_id("leave")
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
        .apply_candidate_id("rewards")
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
fn visible_candidate_alias_resolves_label_leave_and_skip() {
    use crate::eval::run_control::view_model::{CandidateAction, DecisionCandidate};

    let candidates = vec![
        DecisionCandidate {
            id: "0".to_string(),
            label: "Leave.".to_string(),
            action: CandidateAction::Execute(ClientInput::EventChoice(0).into()),
            key: None,
            note: None,
            resolution: None,
        },
        DecisionCandidate {
            id: "1".to_string(),
            label: "Skip card reward".to_string(),
            action: CandidateAction::Execute(ClientInput::Proceed.into()),
            key: None,
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
    outcome: &RunProgressOutcome,
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

fn auto_reward_changes(
    outcome: &RunProgressOutcome,
) -> Option<&[crate::eval::run_control::RunActionResultChangeV1]> {
    outcome
        .auto_applied_steps
        .iter()
        .find(|step| {
            step.kind == crate::eval::run_control::RunControlAutoAppliedKindV1::RewardAutomation
        })
        .and_then(|step| step.action_result.as_ref())
        .map(|result| result.changes.as_slice())
}

fn unique_temp_dir(label: &str) -> PathBuf {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system time should be after unix epoch")
        .as_nanos();
    std::env::temp_dir().join(format!("{label}_{}_{}", std::process::id(), nanos))
}
