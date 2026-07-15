use crate::ai::combat_search_v2::{
    CombatSearchAcceptancePluginId, CombatSearchProfile, CombatSearchV2Config, CombatSearchV2Report,
};
use crate::sim::combat::CombatPosition;

use super::commands::{RunControlHpLossLimit, RunControlSearchCombatOptions};
use super::session::RunControlSession;

pub(super) struct PreparedCombatSearch {
    pub(super) options: RunControlSearchCombatOptions,
    pub(super) start: CombatPosition,
    pub(super) config: CombatSearchV2Config,
    pub(super) effective_profile: EffectiveCombatSearchProfile,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct EffectiveCombatSearchProfile {
    pub(super) profile_id: &'static str,
    pub(super) acceptance: CombatSearchAcceptancePluginId,
}

pub(super) fn prepare_search_combat(
    session: &RunControlSession,
    options: RunControlSearchCombatOptions,
) -> Result<PreparedCombatSearch, String> {
    let options = high_stakes_search_options(session, options);
    let start = session.current_active_combat_position()?;
    let config = search_config(session, options.clone());
    let effective_profile = effective_combat_search_profile(&options);
    Ok(PreparedCombatSearch {
        options,
        start,
        config,
        effective_profile,
    })
}

fn effective_combat_search_profile(
    options: &RunControlSearchCombatOptions,
) -> EffectiveCombatSearchProfile {
    options.profile.as_ref().map_or(
        EffectiveCombatSearchProfile {
            profile_id: "manual_default",
            acceptance: CombatSearchAcceptancePluginId::CleanAcceptedLineNoNewCurse,
        },
        |profile| EffectiveCombatSearchProfile {
            profile_id: profile.label,
            acceptance: profile.policy.acceptance,
        },
    )
}

pub(super) fn effective_hp_loss_limit(
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
    if options.profile.is_none()
        && options.potion_policy.is_none()
        && session.search_potion_policy.is_none()
    {
        options.potion_policy = plan.primary_potion_policy;
    }
    if options.profile.is_none()
        && options.max_potions_used.is_none()
        && session.search_max_potions_used.is_none()
    {
        options.max_potions_used = plan.primary_max_potions_used;
    }
    options
}

pub(super) fn search_report_has_invalid_card_identity(report: &CombatSearchV2Report) -> bool {
    report
        .diagnostics
        .card_identity
        .states_with_uuid_card_id_conflict
        > 0
}

pub(super) fn search_config(
    session: &RunControlSession,
    options: RunControlSearchCombatOptions,
) -> CombatSearchV2Config {
    let defaults = options
        .profile
        .map(CombatSearchProfile::to_config)
        .unwrap_or_default();
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
            .map(std::time::Duration::from_millis)
            .or(defaults.wall_time),
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
        phase_guard_policy: options
            .phase_guard_policy
            .unwrap_or(defaults.phase_guard_policy),
        setup_bias_policy: options
            .setup_bias_policy
            .unwrap_or(defaults.setup_bias_policy),
        turn_plan_probe_max_inner_nodes: defaults.turn_plan_probe_max_inner_nodes,
        turn_plan_probe_max_end_states: defaults.turn_plan_probe_max_end_states,
        turn_plan_probe_per_bucket_limit: defaults.turn_plan_probe_per_bucket_limit,
        root_action_prior: None,
        turn_plan_prior: None,
    }
}

#[cfg(test)]
mod adjudication_tests {
    use super::*;
    use crate::ai::combat_search_v2::{
        CombatSearchAcceptancePluginId, CombatSearchArtifactPluginId, CombatSearchAttemptPolicy,
        CombatSearchBudgetSpec, CombatSearchEngineProfile, CombatSearchPluginStack,
        CombatSearchProfile,
    };
    use crate::state::core::{ActiveCombat, CombatContext, EngineState, RoomCombatContext};
    use crate::state::map::node::RoomType;

    fn active_session() -> RunControlSession {
        let mut session = RunControlSession::new(Default::default());
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            crate::test_support::blank_test_combat(),
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));
        session
    }

    #[test]
    fn prepared_search_always_carries_named_acceptance() {
        let session = active_session();
        let manual = prepare_search_combat(&session, RunControlSearchCombatOptions::default())
            .expect("manual search should prepare");
        assert_eq!(manual.effective_profile.profile_id, "manual_default");
        assert_eq!(
            manual.effective_profile.acceptance,
            CombatSearchAcceptancePluginId::CleanAcceptedLineNoNewCurse
        );

        let profile = CombatSearchProfile {
            label: "primary",
            engine: CombatSearchEngineProfile {
                budget: CombatSearchBudgetSpec {
                    max_nodes: 10,
                    wall_ms: 20,
                },
                plugins: CombatSearchPluginStack::default(),
            },
            policy: CombatSearchAttemptPolicy {
                acceptance: CombatSearchAcceptancePluginId::AcceptedLineOnly,
                artifacts: CombatSearchArtifactPluginId::None,
            },
        };
        let prepared = prepare_search_combat(
            &session,
            RunControlSearchCombatOptions {
                profile: Some(profile),
                ..RunControlSearchCombatOptions::default()
            },
        )
        .expect("profile search should prepare");
        assert_eq!(prepared.effective_profile.profile_id, "primary");
        assert_eq!(
            prepared.effective_profile.acceptance,
            CombatSearchAcceptancePluginId::AcceptedLineOnly
        );
    }
}
