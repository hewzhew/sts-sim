use crate::ai::combat_search_v2::run_combat_search_v2;

use super::combat_line_executor::apply_selected_combat_candidate_line;
use super::combat_line_selector::{select_accepted_search_combat_line, CombatLineSelection};
use super::combat_no_win_fallback::{
    try_apply_no_win_fallback, try_apply_turn_segment_after_rejection,
};
use super::combat_search_rejection::{
    build_combat_search_rejection_outcome, CombatSearchRejectionOutcome,
};
use super::combat_search_setup::{
    effective_hp_loss_limit, prepare_search_combat, search_report_has_invalid_card_identity,
};
use super::commands::RunControlSearchCombatOptions;
use super::session::{
    RunControlCombatSearchRejection, RunControlCommandOutcome, RunControlSession,
};
use super::trace_annotation::CombatAutomationTrajectorySource;

pub(super) fn apply_search_combat(
    session: &mut RunControlSession,
    options: RunControlSearchCombatOptions,
) -> Result<RunControlCommandOutcome, String> {
    let prepared = prepare_search_combat(session, options)?;
    let options = prepared.options;
    let start = prepared.start;
    let config = prepared.config;
    let report = run_combat_search_v2(&start.engine, &start.combat, config.clone());
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
            },
        ));
    }
    let Some(trajectory) = report.best_win_trajectory.as_ref() else {
        if !options.disable_no_win_rescue {
            if let Some(outcome) = try_apply_no_win_fallback(
                session,
                &start,
                &config,
                &options,
                &report,
                effective_hp_loss_limit(session, &options),
            )? {
                return Ok(outcome);
            }
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
    apply_selected_combat_candidate_line(
        session,
        &start,
        &config,
        selected.report.as_ref().unwrap_or(&report),
        selected.line,
        CombatAutomationTrajectorySource::SearchCombat,
        summary,
        None,
    )
}

#[cfg(test)]
mod tests {
    use std::time::Duration;

    use super::super::combat_line_trace::combat_automation_step_state_v1;
    use super::super::combat_no_win_fallback::{
        segment_mode_allows_turn_segment, try_apply_smoke_bomb_survival_fallback_after_rejection,
    };
    use super::super::combat_search_setup::{
        effective_hp_loss_limit, high_stakes_search_options, search_config,
    };
    use crate::ai::combat_search_v2::{
        CombatSearchAcceptancePluginId, CombatSearchActionPriorPluginId,
        CombatSearchArtifactPluginId, CombatSearchBudgetSpec, CombatSearchPluginStack,
        CombatSearchPotionPlugin, CombatSearchProfile, CombatSearchRolloutPluginId,
        CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy, CombatSearchV2SetupBiasPolicy,
    };
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
    fn search_config_uses_profile_as_default_config_source() {
        let session = RunControlSession::new(RunControlConfig::default());
        let profile = CombatSearchProfile {
            label: "profile_default",
            budget: CombatSearchBudgetSpec {
                max_nodes: 222,
                wall_ms: 333,
            },
            plugins: CombatSearchPluginStack {
                action_prior: CombatSearchActionPriorPluginId::KeyCardOnline,
                rollout: CombatSearchRolloutPluginId::Disabled,
                ..CombatSearchPluginStack::default()
            },
            acceptance: CombatSearchAcceptancePluginId::AcceptedLineOnly,
            artifacts: CombatSearchArtifactPluginId::None,
        };

        let config = search_config(
            &session,
            RunControlSearchCombatOptions {
                profile: Some(profile),
                ..RunControlSearchCombatOptions::default()
            },
        );

        assert_eq!(config.max_nodes, 222);
        assert_eq!(config.wall_time, Some(Duration::from_millis(333)));
        assert_eq!(config.rollout_policy, CombatSearchV2RolloutPolicy::Disabled);
        assert_eq!(
            config.setup_bias_policy,
            CombatSearchV2SetupBiasPolicy::KeyCardOnline
        );

        let config = search_config(
            &session,
            RunControlSearchCombatOptions {
                profile: Some(profile),
                max_nodes: Some(444),
                ..RunControlSearchCombatOptions::default()
            },
        );
        assert_eq!(config.max_nodes, 444);
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
    fn high_stakes_search_options_does_not_override_profile_potion_plugin() {
        let session = session_with_combat_flags(true, false);
        let profile = CombatSearchProfile {
            label: "no_potion_profile",
            budget: CombatSearchBudgetSpec {
                max_nodes: 10,
                wall_ms: 20,
            },
            plugins: CombatSearchPluginStack {
                potion: CombatSearchPotionPlugin {
                    policy: CombatSearchV2PotionPolicy::Never,
                    max_potions_used: Some(0),
                },
                ..CombatSearchPluginStack::default()
            },
            acceptance: CombatSearchAcceptancePluginId::AcceptedLineOnly,
            artifacts: CombatSearchArtifactPluginId::None,
        };

        let options = high_stakes_search_options(
            &session,
            RunControlSearchCombatOptions {
                profile: Some(profile),
                ..RunControlSearchCombatOptions::default()
            },
        );

        assert_eq!(options.potion_policy, None);
        assert_eq!(options.max_potions_used, None);
        let config = search_config(&session, options);
        assert_eq!(config.potion_policy, CombatSearchV2PotionPolicy::Never);
        assert_eq!(config.max_potions_used, Some(0));
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
