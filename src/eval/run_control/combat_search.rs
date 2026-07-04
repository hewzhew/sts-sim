use crate::ai::combat_search_v2::{
    run_combat_search_v2, CombatSearchV2Config, CombatSearchV2Report,
};

use super::combat_line_executor::apply_selected_combat_candidate_line;
use super::combat_line_selector::{select_accepted_search_combat_line, CombatLineSelection};
use super::combat_no_win_fallback::{
    try_apply_no_win_fallback, try_apply_turn_segment_after_rejection,
};
use super::combat_search_rejection::{
    build_combat_search_rejection_outcome, CombatSearchRejectionOutcome,
};
use super::commands::{
    RunControlHpLossLimit, RunControlSearchCombatOptions, RunControlSearchEvidenceTarget,
};
use super::registry::BenchmarkCasePaths;
use super::search_evidence::{save_combat_search_evidence_v1, CombatSearchEvidenceContextV1};
use super::session::{
    RunControlCombatSearchRejection, RunControlCommandOutcome, RunControlSession,
};
use super::trace_annotation::CombatAutomationTrajectorySource;

pub(super) fn apply_search_combat(
    session: &mut RunControlSession,
    options: RunControlSearchCombatOptions,
) -> Result<RunControlCommandOutcome, String> {
    let options = high_stakes_search_options(session, options);
    let start = session.current_active_combat_position()?;
    let config = search_config(session, options.clone());
    let report = run_combat_search_v2(&start.engine, &start.combat, config.clone());
    let saved_evidence =
        save_search_evidence_if_requested(session, options.evidence.as_ref(), &report)?;
    if search_report_has_invalid_card_identity(&report) {
        return Ok(build_combat_search_rejection_outcome(
            session,
            &start,
            &report,
            CombatSearchRejectionOutcome {
                result: "invalid_card_identity",
                detail: None,
                rejection: RunControlCombatSearchRejection::InvalidCardIdentity,
                trace_source: "search_combat_rejected",
                saved_evidence: saved_evidence.as_deref(),
            },
        ));
    }
    let Some(trajectory) = report.best_win_trajectory.as_ref() else {
        if let Some(outcome) = try_apply_no_win_fallback(
            session,
            &start,
            &config,
            &options,
            &report,
            saved_evidence.as_deref(),
            effective_hp_loss_limit(session, &options),
        )? {
            return Ok(outcome);
        }
        return Ok(build_combat_search_rejection_outcome(
            session,
            &start,
            &report,
            CombatSearchRejectionOutcome {
                result: "no_complete_winning_candidate",
                detail: None,
                rejection: RunControlCombatSearchRejection::NoCompleteWinningCandidate,
                trace_source: "search_combat_rejected",
                saved_evidence: saved_evidence.as_deref(),
            },
        ));
    };
    let selected =
        match select_accepted_search_combat_line(session, &start, &config, &report, trajectory)? {
            CombatLineSelection::Selected(selected) => selected,
            CombatLineSelection::DirtyRejected { detail } => {
                return Ok(build_combat_search_rejection_outcome(
                    session,
                    &start,
                    &report,
                    CombatSearchRejectionOutcome {
                        result: "dirty_winning_candidate_rejected",
                        detail: Some(detail),
                        rejection: RunControlCombatSearchRejection::DirtyWinningCandidateRejected,
                        trace_source: "search_combat_rejected_dirty_win",
                        saved_evidence: saved_evidence.as_deref(),
                    },
                ));
            }
        };

    if let Some(max_hp_loss) = effective_hp_loss_limit(session, &options) {
        if selected.line.hp_loss > max_hp_loss as i32 {
            if let Some(outcome) = try_apply_turn_segment_after_rejection(
                session,
                &start,
                &config,
                &options,
                &report,
                saved_evidence.as_deref(),
                "complete_winning_candidate_exceeds_hp_loss_limit",
            )? {
                return Ok(outcome);
            }
            return Ok(build_combat_search_rejection_outcome(
                session,
                &start,
                &report,
                CombatSearchRejectionOutcome {
                    result: "complete_winning_candidate_exceeds_hp_loss_limit",
                    detail: Some(format!(
                        "candidate_hp_loss={} max_hp_loss={max_hp_loss}",
                        selected.line.hp_loss
                    )),
                    rejection: RunControlCombatSearchRejection::HpLossLimitExceeded,
                    trace_source: "search_combat_rejected",
                    saved_evidence: saved_evidence.as_deref(),
                },
            ));
        }
    }

    let mut summary = format!(
        "search-combat applied {} actions",
        selected.line.actions.len()
    );
    if let Some(repair_summary) = selected.summary.as_ref() {
        summary.push_str(&format!(" {repair_summary}"));
    }
    if let Some(path) = saved_evidence.as_ref() {
        summary.push_str(&format!(" saved_search={}", path.display()));
    }
    apply_selected_combat_candidate_line(
        session,
        &start,
        &config,
        selected.report.as_ref().unwrap_or(&report),
        saved_evidence.as_deref(),
        selected.line,
        CombatAutomationTrajectorySource::SearchCombat,
        summary,
        None,
    )
}

fn save_search_evidence_if_requested(
    session: &RunControlSession,
    target: Option<&RunControlSearchEvidenceTarget>,
    report: &CombatSearchV2Report,
) -> Result<Option<std::path::PathBuf>, String> {
    let Some(target) = target else {
        return Ok(None);
    };
    let (path, capture_case_id, capture_root, capture_path) = match target {
        RunControlSearchEvidenceTarget::Path(path) => {
            (next_available_evidence_path(path), None, None, None)
        }
        RunControlSearchEvidenceTarget::LastCaptureCase => {
            let case = session.active_capture_case().ok_or_else(|| {
                "search evidence save=case requires the current combat to have a matching cap <case_id>"
                    .to_string()
            })?;
            let paths = BenchmarkCasePaths::for_case(&case.root, &case.case_id);
            let base_path = case.root.join("search_evidence").join(format!(
                "{}.step{}.search.json",
                case.case_id, session.decision_step
            ));
            (
                next_available_evidence_path(&base_path),
                Some(case.case_id.clone()),
                Some(case.root.display().to_string()),
                Some(paths.capture_path.display().to_string()),
            )
        }
    };
    save_combat_search_evidence_v1(
        &path,
        CombatSearchEvidenceContextV1 {
            source_kind: "run_control_search_combat",
            decision_step: session.decision_step,
            capture_case_id,
            capture_root,
            capture_path,
        },
        report,
    )?;
    Ok(Some(path))
}

fn effective_hp_loss_limit(
    session: &RunControlSession,
    options: &RunControlSearchCombatOptions,
) -> Option<u32> {
    match options.max_hp_loss {
        Some(RunControlHpLossLimit::Limit(limit)) => Some(limit),
        Some(RunControlHpLossLimit::Unlimited) => None,
        None => session.search_max_hp_loss,
    }
}

pub(in crate::eval::run_control) fn high_stakes_search_options(
    session: &RunControlSession,
    mut options: RunControlSearchCombatOptions,
) -> RunControlSearchCombatOptions {
    let plan = super::combat_auto_policy::combat_auto_search_plan(session, &options);
    if options.potion_policy.is_none() && session.search_potion_policy.is_none() {
        options.potion_policy = plan.primary_potion_policy;
    }
    if options.max_potions_used.is_none() && session.search_max_potions_used.is_none() {
        options.max_potions_used = plan.primary_max_potions_used;
    }
    options
}

fn search_report_has_invalid_card_identity(report: &CombatSearchV2Report) -> bool {
    report
        .diagnostics
        .card_identity
        .states_with_uuid_card_id_conflict
        > 0
}

fn next_available_evidence_path(path: &std::path::Path) -> std::path::PathBuf {
    if !path.exists() {
        return path.to_path_buf();
    }
    let parent = path.parent().unwrap_or_else(|| std::path::Path::new(""));
    let stem = path
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("search_evidence");
    let ext = path
        .extension()
        .and_then(|ext| ext.to_str())
        .unwrap_or("json");
    for idx in 2..10_000 {
        let candidate = parent.join(format!("{stem}.{idx}.{ext}"));
        if !candidate.exists() {
            return candidate;
        }
    }
    parent.join(format!("{stem}.overflow.{ext}"))
}

fn search_config(
    session: &RunControlSession,
    options: RunControlSearchCombatOptions,
) -> CombatSearchV2Config {
    let defaults = CombatSearchV2Config::default();
    let stop_on_win_hp_loss_at_most = effective_hp_loss_limit(session, &options);
    CombatSearchV2Config {
        max_nodes: options
            .max_nodes
            .or(session.search_max_nodes)
            .unwrap_or(defaults.max_nodes),
        max_actions_per_line: options
            .max_actions_per_line
            .unwrap_or(defaults.max_actions_per_line),
        max_engine_steps_per_action: options
            .max_engine_steps_per_action
            .unwrap_or(defaults.max_engine_steps_per_action),
        wall_time: options
            .wall_ms
            .or(session.search_wall_ms)
            .map(std::time::Duration::from_millis),
        stop_on_win_hp_loss_at_most,
        min_win_candidates_before_stop: defaults.min_win_candidates_before_stop,
        input_label: Some(format!(
            "run_play_driver:search_combat:step{}",
            session.decision_step
        )),
        potion_policy: options
            .potion_policy
            .or(session.search_potion_policy)
            .unwrap_or(defaults.potion_policy),
        max_potions_used: options
            .max_potions_used
            .or(session.search_max_potions_used)
            .or(defaults.max_potions_used),
        rollout_policy: options.rollout_policy.unwrap_or(defaults.rollout_policy),
        child_rollout_policy: options
            .child_rollout_policy
            .unwrap_or(defaults.child_rollout_policy),
        rollout_max_evaluations: options
            .rollout_max_evaluations
            .unwrap_or(defaults.rollout_max_evaluations),
        rollout_max_actions: options
            .rollout_max_actions
            .unwrap_or(defaults.rollout_max_actions),
        rollout_beam_width: options
            .rollout_beam_width
            .unwrap_or(defaults.rollout_beam_width),
        turn_plan_policy: options
            .turn_plan_policy
            .unwrap_or(defaults.turn_plan_policy),
        frontier_policy: options.frontier_policy.unwrap_or(defaults.frontier_policy),
        phase_guard_policy: defaults.phase_guard_policy,
        turn_plan_probe_max_inner_nodes: defaults.turn_plan_probe_max_inner_nodes,
        turn_plan_probe_max_end_states: defaults.turn_plan_probe_max_end_states,
        turn_plan_probe_per_bucket_limit: defaults.turn_plan_probe_per_bucket_limit,
        root_action_prior: None,
        turn_plan_prior: None,
    }
}

#[cfg(test)]
mod tests {
    use std::fs;
    use std::time::{Duration, SystemTime, UNIX_EPOCH};

    use super::super::combat_no_win_fallback::{
        segment_mode_allows_turn_segment, try_apply_smoke_bomb_survival_fallback_after_rejection,
    };
    use super::{
        combat_automation_step_state_v1, effective_hp_loss_limit, high_stakes_search_options,
        next_available_evidence_path, search_config,
    };
    use crate::ai::combat_search_v2::CombatSearchV2PotionPolicy;
    use crate::content::potions::{Potion, PotionId};
    use crate::content::powers::{store, PowerId};
    use crate::eval::run_control::trace_annotation::{
        CombatAutomationActionV1, CombatAutomationTrajectoryRecordV1,
        CombatAutomationTrajectorySource, RunControlTraceAnnotationV1,
    };
    use crate::eval::run_control::{
        RunControlConfig, RunControlHpLossLimit, RunControlSearchCombatOptions, RunControlSession,
    };
    use crate::runtime::combat::CombatCard;
    use crate::state::core::{
        ActiveCombat, ClientInput, CombatContext, EngineState, RoomCombatContext,
    };
    use crate::state::map::node::RoomType;
    use crate::state::rewards::RewardScreenContext;

    fn session_with_active_combat(
        mut combat: crate::runtime::combat::CombatState,
    ) -> RunControlSession {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            {
                combat.entities.monsters = vec![crate::test_support::test_monster(
                    crate::content::monsters::EnemyId::JawWorm,
                )];
                combat
            },
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        session
    }

    #[test]
    fn combat_automation_step_state_records_time_warp_counter_and_forced_end_state() {
        let mut combat = crate::test_support::blank_test_combat();
        combat.entities.monsters = vec![crate::test_support::test_monster(
            crate::content::monsters::EnemyId::TimeEater,
        )];
        let monster_id = combat.entities.monsters[0].id;
        store::set_powers_for(
            &mut combat,
            monster_id,
            vec![
                crate::runtime::combat::Power {
                    power_type: PowerId::TimeWarp,
                    instance_id: None,
                    amount: 11,
                    extra_data: 0,
                    payload: crate::runtime::combat::PowerPayload::None,
                    just_applied: false,
                },
                crate::runtime::combat::Power {
                    power_type: PowerId::Strength,
                    instance_id: None,
                    amount: 2,
                    extra_data: 0,
                    payload: crate::runtime::combat::PowerPayload::None,
                    just_applied: false,
                },
            ],
        );
        combat.turn.counters.cards_played_this_turn = 11;
        combat.turn.mark_early_end_turn_pending();
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoomBoss,
            }),
        ));

        let snapshot = combat_automation_step_state_v1(&session)
            .expect("active combat should produce automation step state");

        assert_eq!(snapshot.cards_played_this_turn, 11);
        assert!(snapshot.early_end_turn_pending);
        assert_eq!(snapshot.monsters.len(), 1);
        assert_eq!(snapshot.monsters[0].label, "Time Eater");
        assert_eq!(snapshot.monsters[0].time_warp, 11);
        assert_eq!(snapshot.monsters[0].strength, 2);
    }

    fn session_with_combat_flags(is_boss_fight: bool, is_elite_fight: bool) -> RunControlSession {
        let mut combat = crate::test_support::blank_test_combat();
        combat.meta.is_boss_fight = is_boss_fight;
        combat.meta.is_elite_fight = is_elite_fight;
        session_with_active_combat(combat)
    }

    fn options_with_hp_loss(max_hp_loss: RunControlHpLossLimit) -> RunControlSearchCombatOptions {
        RunControlSearchCombatOptions {
            max_hp_loss: Some(max_hp_loss),
            ..RunControlSearchCombatOptions::default()
        }
    }

    fn options_with_potion_budget(
        potion_policy: CombatSearchV2PotionPolicy,
        max_potions_used: u32,
    ) -> RunControlSearchCombatOptions {
        RunControlSearchCombatOptions {
            potion_policy: Some(potion_policy),
            max_potions_used: Some(max_potions_used),
            ..RunControlSearchCombatOptions::default()
        }
    }

    fn assert_potion_budget(
        options: RunControlSearchCombatOptions,
        expected_policy: Option<CombatSearchV2PotionPolicy>,
        expected_max_used: Option<u32>,
    ) {
        assert_eq!(options.potion_policy, expected_policy);
        assert_eq!(options.max_potions_used, expected_max_used);
    }

    #[test]
    fn search_combat_uses_smoke_bomb_as_survival_fallback_when_no_win_exists() {
        let mut combat = crate::test_support::blank_test_combat();
        combat.entities.player.current_hp = 1;
        combat.entities.player.max_hp = 80;
        combat.turn.energy = 0;
        combat.meta.is_boss_fight = false;
        let mut jaw_worm =
            crate::test_support::test_monster(crate::content::monsters::EnemyId::JawWorm);
        jaw_worm.current_hp = 40;
        jaw_worm.max_hp = 40;
        let attack = crate::runtime::monster_move::MonsterMoveSpec::Attack(
            crate::runtime::monster_move::AttackSpec {
                base_damage: 10,
                hits: 1,
                damage_kind: crate::runtime::monster_move::DamageKind::Normal,
            },
        );
        jaw_worm.set_planned_steps(attack.to_steps());
        jaw_worm.set_planned_visible_spec(Some(attack));
        combat.entities.monsters = vec![jaw_worm];
        combat.zones.hand = vec![CombatCard::new(crate::content::cards::CardId::Defend, 1)];
        combat.update_hand_cards();
        combat.entities.potions = vec![Some(Potion::new(PotionId::SmokeBomb, 1))];
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::CombatPlayerTurn;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));

        let outcome = try_apply_smoke_bomb_survival_fallback_after_rejection(
            &mut session,
            None,
            "no_complete_winning_candidate",
        )
        .expect("fallback should not error")
        .expect("search combat should allow smoke bomb survival fallback");

        let EngineState::RewardScreen(rewards) = &session.engine_state else {
            panic!(
                "smoke bomb fallback should leave combat at reward screen, got {:?}",
                session.engine_state
            );
        };
        assert_eq!(rewards.screen_context, RewardScreenContext::SmokedCombat);
        assert!(
            outcome.message.contains("Smoke Bomb"),
            "fallback outcome should be explicit, got: {}",
            outcome.message
        );
    }

    #[test]
    fn search_evidence_path_does_not_overwrite_existing_file() {
        let root = std::env::temp_dir().join(format!(
            "sts_search_evidence_path_{}",
            SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock should be after unix epoch")
                .as_nanos()
        ));
        fs::create_dir_all(&root).expect("temp dir should be created");
        let base = root.join("case.step1.search.json");
        fs::write(&base, "{}").expect("base file should be written");

        let next = next_available_evidence_path(&base);

        assert_ne!(next, base);
        assert_eq!(
            next.file_name().and_then(|name| name.to_str()),
            Some("case.step1.search.2.json")
        );
        assert!(!next.exists());

        let _ = fs::remove_dir_all(root);
    }

    #[test]
    fn combat_automation_trace_annotation_preserves_action_inputs() {
        let annotation = CombatAutomationTrajectoryRecordV1::new(
            CombatAutomationTrajectorySource::SearchCombat,
            vec![CombatAutomationActionV1 {
                step_index: 7,
                action_key: "combat/end_turn".to_string(),
                input: ClientInput::EndTurn,
                drawn_cards: Vec::new(),
                combat_after: None,
            }],
        )
        .into_annotation();

        let RunControlTraceAnnotationV1::CombatAutomationTrajectory {
            source,
            action_count,
            actions,
            label_role,
        } = annotation
        else {
            panic!("expected combat automation trajectory annotation")
        };
        assert_eq!(source, CombatAutomationTrajectorySource::SearchCombat);
        assert_eq!(action_count, 1);
        assert_eq!(actions[0].step_index, 7);
        assert_eq!(actions[0].action_key, "combat/end_turn");
        assert_eq!(actions[0].input, ClientInput::EndTurn);
        assert_eq!(label_role, "simulator_generated_not_teacher_label");
    }

    #[test]
    fn hp_loss_limit_uses_session_default_and_command_override() {
        let session = RunControlSession::new(RunControlConfig {
            search_max_hp_loss: Some(12),
            ..RunControlConfig::default()
        });

        assert_eq!(
            effective_hp_loss_limit(&session, &RunControlSearchCombatOptions::default()),
            Some(12)
        );
        assert_eq!(
            search_config(&session, RunControlSearchCombatOptions::default())
                .stop_on_win_hp_loss_at_most,
            Some(12)
        );
        assert_eq!(
            effective_hp_loss_limit(
                &session,
                &options_with_hp_loss(RunControlHpLossLimit::Limit(4))
            ),
            Some(4)
        );
        assert_eq!(
            search_config(
                &session,
                options_with_hp_loss(RunControlHpLossLimit::Limit(4))
            )
            .stop_on_win_hp_loss_at_most,
            Some(4)
        );
        assert_eq!(
            effective_hp_loss_limit(
                &session,
                &options_with_hp_loss(RunControlHpLossLimit::Unlimited)
            ),
            None
        );
        assert_eq!(
            search_config(
                &session,
                options_with_hp_loss(RunControlHpLossLimit::Unlimited)
            )
            .stop_on_win_hp_loss_at_most,
            None
        );
    }

    #[test]
    fn search_config_uses_session_budget_defaults_and_command_override() {
        let session = RunControlSession::new(RunControlConfig {
            search_max_nodes: Some(1234),
            search_wall_ms: Some(5678),
            ..RunControlConfig::default()
        });

        let config = search_config(&session, RunControlSearchCombatOptions::default());
        assert_eq!(config.max_nodes, 1234);
        assert_eq!(config.wall_time, Some(Duration::from_millis(5678)));

        let config = search_config(
            &session,
            RunControlSearchCombatOptions {
                max_nodes: Some(90),
                wall_ms: Some(12),
                ..RunControlSearchCombatOptions::default()
            },
        );
        assert_eq!(config.max_nodes, 90);
        assert_eq!(config.wall_time, Some(Duration::from_millis(12)));
    }

    #[test]
    fn search_config_uses_session_potion_defaults_and_command_override() {
        let session = RunControlSession::new(RunControlConfig {
            search_potion_policy: Some(CombatSearchV2PotionPolicy::SemanticBudgeted),
            search_max_potions_used: Some(2),
            ..RunControlConfig::default()
        });

        let config = search_config(&session, RunControlSearchCombatOptions::default());
        assert_eq!(
            config.potion_policy,
            CombatSearchV2PotionPolicy::SemanticBudgeted
        );
        assert_eq!(config.max_potions_used, Some(2));

        let config = search_config(
            &session,
            RunControlSearchCombatOptions {
                potion_policy: Some(CombatSearchV2PotionPolicy::Never),
                max_potions_used: Some(0),
                ..RunControlSearchCombatOptions::default()
            },
        );
        assert_eq!(config.potion_policy, CombatSearchV2PotionPolicy::Never);
        assert_eq!(config.max_potions_used, Some(0));
    }

    #[test]
    fn high_stakes_search_options_enables_semantic_potions_for_boss_manual_search() {
        let session = session_with_combat_flags(true, false);

        let options =
            high_stakes_search_options(&session, RunControlSearchCombatOptions::default());

        assert_potion_budget(
            options,
            Some(CombatSearchV2PotionPolicy::SemanticBudgeted),
            Some(2),
        );
    }

    #[test]
    fn high_stakes_search_options_enables_single_semantic_potion_for_elite_manual_search() {
        let session = session_with_combat_flags(false, true);

        let options =
            high_stakes_search_options(&session, RunControlSearchCombatOptions::default());

        assert_potion_budget(
            options,
            Some(CombatSearchV2PotionPolicy::SemanticBudgeted),
            Some(1),
        );
    }

    #[test]
    fn non_boss_segment_mode_allows_hallway_partial_turns_but_blocks_boss_partial_turns() {
        let hallway = session_with_combat_flags(false, false);
        let hallway_start = hallway
            .current_active_combat_position()
            .expect("hallway combat position");
        assert!(segment_mode_allows_turn_segment(
            Some(crate::eval::run_control::RunControlCombatSegmentMode::NonBossTurnBoundary),
            &hallway_start
        ));

        let boss = session_with_combat_flags(true, false);
        let boss_start = boss
            .current_active_combat_position()
            .expect("boss combat position");
        assert!(!segment_mode_allows_turn_segment(
            Some(crate::eval::run_control::RunControlCombatSegmentMode::NonBossTurnBoundary),
            &boss_start
        ));
        assert!(segment_mode_allows_turn_segment(
            Some(crate::eval::run_control::RunControlCombatSegmentMode::TurnBoundary),
            &boss_start
        ));
    }

    #[test]
    fn high_stakes_search_options_keeps_ordinary_manual_search_no_potion_default() {
        let session = session_with_combat_flags(false, false);

        let options =
            high_stakes_search_options(&session, RunControlSearchCombatOptions::default());

        assert_potion_budget(options, None, None);
    }

    #[test]
    fn high_stakes_search_options_respects_user_potion_override() {
        let session = session_with_combat_flags(true, false);

        let options = high_stakes_search_options(
            &session,
            options_with_potion_budget(CombatSearchV2PotionPolicy::Never, 0),
        );

        assert_potion_budget(options, Some(CombatSearchV2PotionPolicy::Never), Some(0));
    }
}
