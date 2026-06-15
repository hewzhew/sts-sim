use std::collections::BTreeSet;

use crate::runtime::combat::CombatCard;
use crate::state::core::{ActiveCombat, ClientInput, EngineState, RunResult};

use super::commands::{
    RunControlAutoStepOptions, RunControlRouteAutomationMode, RunControlSearchCombatOptions,
};
use super::session::{RunControlCommandOutcome, RunControlSession};
use super::trace_annotation::RunControlTraceAnnotationV1;
use super::transition_report::{
    action_result_from_transition, render_action_result, RunApplyStatus, RunVisibleSnapshot,
    TransitionAction,
};
use super::view_model::{build_run_control_view_model, DecisionCandidate, RunControlViewModel};

const DEFAULT_MAX_OPERATIONS: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(in crate::eval::run_control) enum NonCombatAutoMode {
    FullPlanner,
    BranchExperimentBoundary,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AutoAdvanceClass {
    Routine,
    Forced,
    Strategic,
    Unsafe,
}

struct AutoAdvanceCandidate<'a> {
    candidate: &'a DecisionCandidate,
    class: AutoAdvanceClass,
    reason: &'static str,
}

pub(super) fn apply_guarded_auto_step(
    session: &mut RunControlSession,
    options: RunControlAutoStepOptions,
) -> Result<RunControlCommandOutcome, String> {
    apply_guarded_auto_step_with_mode(session, options, NonCombatAutoMode::FullPlanner)
}

pub(in crate::eval::run_control) fn apply_guarded_auto_step_with_mode(
    session: &mut RunControlSession,
    options: RunControlAutoStepOptions,
    noncombat_mode: NonCombatAutoMode,
) -> Result<RunControlCommandOutcome, String> {
    let before = RunVisibleSnapshot::capture(session);
    let mut applied = Vec::new();
    let mut trace_annotations = Vec::new();
    let mut seen_boundaries = BTreeSet::new();
    let max_operations = options.max_operations.unwrap_or(DEFAULT_MAX_OPERATIONS);

    for _ in 0..max_operations {
        let boundary_key = auto_boundary_key(session);
        if !seen_boundaries.insert(boundary_key.clone()) {
            return finish_auto_step(
                session,
                &before,
                applied,
                trace_annotations,
                "repeated boundary detected while advancing automatically",
                Some(format!(
                    "boundary={boundary_key}\nThis usually means an event or transition kept presenting the same screen after an automatic action."
                )),
            );
        }

        let reward_report = super::reward_auto::apply_reward_automation(session)?;
        if !reward_report.is_empty() {
            applied.push(format!(
                "routine reward: {}",
                reward_report
                    .claims
                    .iter()
                    .map(|claim| claim.label.as_str())
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            trace_annotations.extend(reward_report.trace_annotations);
            continue;
        }

        if session.current_active_combat_position().is_ok() {
            if high_stakes_auto_search_requires_hp_loss_gate(session, &options.search) {
                return finish_auto_step(
                    session,
                    &before,
                    applied,
                    trace_annotations,
                    "high-stakes combat auto-search requires an hp-loss gate",
                    Some(
                        "Use `n max_hp_loss=N` or `nr max_hp_loss=N` for this combat, or `sd max_hp_loss=N` to set a session default. Use `n max_hp_loss=off` only when you deliberately want to accept any winning search line."
                            .to_string(),
                    ),
                );
            }

            let mut no_potion_rejection = None;
            if let Some(no_potion_options) = auto_no_potion_first_options(session, &options.search)
            {
                let outcome =
                    super::combat_search::apply_search_combat(session, no_potion_options)?;
                if let Some(result) = outcome.action_result.as_ref() {
                    applied.push(format!("combat search(no potion): {}", result.chosen_label));
                    let auto_capture_summaries = auto_capture_summaries(&outcome.trace_annotations);
                    trace_annotations.extend(outcome.trace_annotations);
                    applied.extend(auto_capture_summaries);
                    continue;
                }
                no_potion_rejection = Some(trim_search_rejection(&outcome.message));
            }

            let outcome = super::combat_search::apply_search_combat(
                session,
                auto_search_options(session, options.search.clone()),
            )?;
            if let Some(result) = outcome.action_result.as_ref() {
                let label = if no_potion_rejection.is_some() {
                    format!("combat search(semantic fallback): {}", result.chosen_label)
                } else {
                    format!("combat search: {}", result.chosen_label)
                };
                applied.push(label);
                let auto_capture_summaries = auto_capture_summaries(&outcome.trace_annotations);
                trace_annotations.extend(outcome.trace_annotations);
                applied.extend(auto_capture_summaries);
                continue;
            }
            let fallback_rejection = trim_search_rejection(&outcome.message);
            if let Some(rescue_options) = auto_potion_rescue_options(session, &options.search) {
                let rescue = super::combat_search::apply_search_combat(session, rescue_options)?;
                if let Some(result) = rescue.action_result.as_ref() {
                    applied.push(format!(
                        "combat search(potion rescue): {}",
                        result.chosen_label
                    ));
                    let auto_capture_summaries = auto_capture_summaries(&rescue.trace_annotations);
                    trace_annotations.extend(rescue.trace_annotations);
                    applied.extend(auto_capture_summaries);
                    continue;
                }
                return finish_auto_step(
                    session,
                    &before,
                    applied,
                    trace_annotations,
                    "combat search did not find an executable complete win",
                    Some(combine_three_search_rejections(
                        no_potion_rejection,
                        fallback_rejection,
                        trim_search_rejection(&rescue.message),
                    )),
                );
            }
            return finish_auto_step(
                session,
                &before,
                applied,
                trace_annotations,
                "combat search did not find an executable complete win",
                Some(combine_search_rejections(
                    no_potion_rejection,
                    fallback_rejection,
                )),
            );
        }

        if let Some((outcome, summary)) = apply_map_overlay_back_without_route_candidates(session)?
        {
            let auto_capture_summaries = auto_capture_summaries(&outcome.trace_annotations);
            trace_annotations.extend(outcome.trace_annotations);
            applied.push(summary);
            applied.extend(auto_capture_summaries);
            continue;
        }

        if session.engine_state.is_map_surface()
            && options.route == RunControlRouteAutomationMode::Planner
        {
            let route_result =
                super::route_policy::apply_route_go_with_summary_allowing_forced_risk(session);
            match route_result {
                Ok(applied_route) => {
                    if applied_route.outcome.action_result.is_some() {
                        let auto_capture_summaries =
                            auto_capture_summaries(&applied_route.outcome.trace_annotations);
                        trace_annotations.extend(applied_route.outcome.trace_annotations);
                        applied.push(applied_route.auto_step_summary);
                        applied.extend(auto_capture_summaries);
                        continue;
                    }
                    trace_annotations.extend(applied_route.outcome.trace_annotations);
                    return finish_auto_step(
                        session,
                        &before,
                        applied,
                        trace_annotations,
                        "route planner did not modify state",
                        Some(applied_route.outcome.message),
                    );
                }
                Err(err) => {
                    let detail =
                        match super::route_policy::route_policy_stop_for_session(session, &err)? {
                            Some((annotation, summary)) => {
                                trace_annotations.push(annotation);
                                Some(format!("{summary}\n{err}"))
                            }
                            None => Some(err),
                        };
                    return finish_auto_step(
                        session,
                        &before,
                        applied,
                        trace_annotations,
                        "route planner declined automatic map selection",
                        detail,
                    );
                }
            }
        }

        if let Some((outcome, summary)) = apply_pending_shop_reward_overlay(session)? {
            let auto_capture_summaries = auto_capture_summaries(&outcome.trace_annotations);
            trace_annotations.extend(outcome.trace_annotations);
            applied.push(summary);
            applied.extend(auto_capture_summaries);
            continue;
        }

        if options.route == super::commands::RunControlRouteAutomationMode::Planner {
            if let Some(application) = apply_noncombat_policy(session, noncombat_mode)? {
                let auto_capture_summaries =
                    auto_capture_summaries(&application.outcome.trace_annotations);
                trace_annotations.extend(application.outcome.trace_annotations);
                applied.push(application.summary);
                applied.extend(auto_capture_summaries);
                if let Some(reason) = application.stop_after_reason {
                    return finish_auto_step(
                        session,
                        &before,
                        applied,
                        trace_annotations,
                        reason,
                        None,
                    );
                }
                continue;
            }
        }
        let card_reward_policy_stop =
            super::noncombat_auto::planner_noncombat_policy_stop_annotation(session)?;

        if noncombat_mode == NonCombatAutoMode::BranchExperimentBoundary
            && branch_experiment_should_stop_before_visible_candidate(session)
        {
            return finish_auto_step(
                session,
                &before,
                applied,
                trace_annotations,
                human_stop_reason(session),
                None,
            );
        }

        let view = build_run_control_view_model(session);
        if let Some(auto_candidate) = auto_advance_candidate(session, &view) {
            let Some(input) = auto_candidate.candidate.action.executable_input() else {
                return finish_auto_step(
                    session,
                    &before,
                    applied,
                    trace_annotations,
                    "auto-selected candidate is not executable",
                    None,
                );
            };
            let outcome = session.apply_input(input)?;
            let label = outcome
                .action_result
                .as_ref()
                .map(|result| result.chosen_label.clone())
                .unwrap_or_else(|| auto_candidate.candidate.label.clone());
            let auto_capture_summaries = auto_capture_summaries(&outcome.trace_annotations);
            trace_annotations.extend(outcome.trace_annotations);
            applied.push(format!(
                "{}: {label} ({})",
                auto_class_label(auto_candidate.class),
                auto_candidate.reason
            ));
            applied.extend(auto_capture_summaries);
            continue;
        }

        let detail = card_reward_policy_stop.map(|(annotation, detail)| {
            trace_annotations.push(annotation);
            detail
        });
        return finish_auto_step(
            session,
            &before,
            applied,
            trace_annotations,
            human_stop_reason(session),
            detail,
        );
    }

    finish_auto_step(
        session,
        &before,
        applied,
        trace_annotations,
        format!("operation budget exhausted at {max_operations} automatic operations"),
        None,
    )
}

fn branch_experiment_should_stop_before_visible_candidate(session: &RunControlSession) -> bool {
    match &session.engine_state {
        EngineState::RewardScreen(reward) => {
            reward.pending_card_choice.is_some() || !reward.items.is_empty()
        }
        EngineState::RewardOverlay { reward_state, .. } => {
            reward_state.pending_card_choice.is_some() || !reward_state.items.is_empty()
        }
        EngineState::EventRoom => !event_room_has_safe_auto_advance(session),
        EngineState::Campfire
        | EngineState::Shop(_)
        | EngineState::RunPendingChoice(_)
        | EngineState::BossRelicSelect(_) => true,
        _ => false,
    }
}

fn event_room_has_safe_auto_advance(session: &RunControlSession) -> bool {
    let view = build_run_control_view_model(session);
    auto_advance_candidate(session, &view).is_some()
}

fn apply_noncombat_policy(
    session: &mut RunControlSession,
    mode: NonCombatAutoMode,
) -> Result<Option<super::noncombat_auto::NonCombatAutoApplication>, String> {
    match mode {
        NonCombatAutoMode::FullPlanner => {
            super::noncombat_auto::apply_planner_noncombat_policy(session)
        }
        NonCombatAutoMode::BranchExperimentBoundary => {
            super::noncombat_auto::apply_branch_experiment_noncombat_policy(session)
        }
    }
}

fn apply_pending_shop_reward_overlay(
    session: &mut RunControlSession,
) -> Result<Option<(RunControlCommandOutcome, String)>, String> {
    let EngineState::Shop(shop) = &session.engine_state else {
        return Ok(None);
    };
    if shop.pending_reward_overlay.is_none() {
        return Ok(None);
    }

    let view = build_run_control_view_model(session);
    let Some(candidate) = view.candidates.iter().find(|candidate| {
        candidate.action.executable_input() == Some(ClientInput::OpenRewardOverlay)
    }) else {
        return Ok(None);
    };
    let label = candidate.label.clone();
    let outcome = session.apply_input(ClientInput::OpenRewardOverlay)?;
    Ok(Some((
        outcome,
        format!("routine: {label} (pending shop reward overlay)"),
    )))
}

fn auto_capture_summaries(annotations: &[RunControlTraceAnnotationV1]) -> Vec<String> {
    annotations
        .iter()
        .filter_map(|annotation| match annotation {
            RunControlTraceAnnotationV1::AutoCombatCapture {
                case_id,
                capture_path,
                ..
            } => Some(format!("auto capture: {case_id} -> {capture_path}")),
            RunControlTraceAnnotationV1::RoutePlannerSelection { .. }
            | RunControlTraceAnnotationV1::NonCombatPolicyDecision { .. }
            | RunControlTraceAnnotationV1::NonCombatHumanBoundary { .. }
            | RunControlTraceAnnotationV1::CombatAutomationTrajectory { .. } => None,
        })
        .collect()
}

fn auto_search_options(
    session: &RunControlSession,
    mut options: RunControlSearchCombatOptions,
) -> RunControlSearchCombatOptions {
    let plan = super::combat_auto_policy::combat_auto_search_plan(session, &options);
    if options.wall_ms.is_none() {
        options.wall_ms = plan.default_wall_ms;
    }
    if options.potion_policy.is_none() && session.search_potion_policy.is_none() {
        options.potion_policy = plan.primary_potion_policy;
    }
    if options.max_potions_used.is_none() && session.search_max_potions_used.is_none() {
        options.max_potions_used = plan.primary_max_potions_used;
    }
    options
}

fn auto_no_potion_first_options(
    session: &RunControlSession,
    options: &RunControlSearchCombatOptions,
) -> Option<RunControlSearchCombatOptions> {
    let plan = super::combat_auto_policy::combat_auto_search_plan(session, options);
    if !plan.no_potion_first {
        return None;
    }

    let mut no_potion = options.clone();
    if no_potion.wall_ms.is_none() {
        no_potion.wall_ms = plan.default_wall_ms;
    }
    no_potion.potion_policy = Some(crate::ai::combat_search_v2::CombatSearchV2PotionPolicy::Never);
    no_potion.max_potions_used = Some(0);
    no_potion.evidence = None;
    Some(no_potion)
}

fn auto_potion_rescue_options(
    session: &RunControlSession,
    options: &RunControlSearchCombatOptions,
) -> Option<RunControlSearchCombatOptions> {
    let plan = super::combat_auto_policy::combat_auto_search_plan(session, options);
    let Some(potion_policy) = plan.potion_rescue_policy else {
        return None;
    };

    let mut rescue = options.clone();
    if rescue.wall_ms.is_none() {
        rescue.wall_ms = plan.default_wall_ms;
    }
    rescue.potion_policy = Some(potion_policy);
    rescue.max_potions_used = plan.potion_rescue_max_potions_used;
    Some(rescue)
}

fn high_stakes_auto_search_requires_hp_loss_gate(
    session: &RunControlSession,
    options: &RunControlSearchCombatOptions,
) -> bool {
    super::combat_auto_policy::combat_auto_search_plan(session, options)
        .requires_explicit_hp_loss_gate
}

fn apply_map_overlay_back_without_route_candidates(
    session: &mut RunControlSession,
) -> Result<Option<(RunControlCommandOutcome, String)>, String> {
    if !matches!(session.engine_state, EngineState::MapOverlay { .. }) {
        return Ok(None);
    }

    let view = build_run_control_view_model(session);
    let has_route_candidate = view.candidates.iter().any(|candidate| {
        matches!(
            candidate.action.executable_input(),
            Some(ClientInput::SelectMapNode(_))
        )
    });
    if has_route_candidate {
        return Ok(None);
    }

    let Some(label) = view
        .candidates
        .iter()
        .find(|candidate| candidate.action.executable_input() == Some(ClientInput::Cancel))
        .map(|candidate| candidate.label.clone())
    else {
        return Ok(None);
    };

    let outcome = session.apply_input(ClientInput::Cancel)?;
    let label = outcome
        .action_result
        .as_ref()
        .map(|result| result.chosen_label.clone())
        .unwrap_or(label);
    Ok(Some((
        outcome,
        format!("routine: {label} (map preview has no route action)"),
    )))
}

fn auto_advance_candidate<'a>(
    session: &RunControlSession,
    view: &'a RunControlViewModel,
) -> Option<AutoAdvanceCandidate<'a>> {
    if let EngineState::RewardScreen(reward) = &session.engine_state {
        if reward.pending_card_choice.is_none() && reward.items.is_empty() && reward.skippable {
            return view
                .candidates
                .iter()
                .find(|candidate| candidate.action.executable_input() == Some(ClientInput::Proceed))
                .map(|candidate| AutoAdvanceCandidate {
                    candidate,
                    class: AutoAdvanceClass::Routine,
                    reason: "empty reward screen",
                });
        }
    }
    if let EngineState::RewardOverlay { reward_state, .. } = &session.engine_state {
        if reward_state.pending_card_choice.is_none()
            && reward_state.items.is_empty()
            && reward_state.skippable
        {
            return view
                .candidates
                .iter()
                .find(|candidate| candidate.action.executable_input() == Some(ClientInput::Cancel))
                .map(|candidate| AutoAdvanceCandidate {
                    candidate,
                    class: AutoAdvanceClass::Routine,
                    reason: "empty overlay reward screen",
                });
        }
    }
    if view.candidates.len() == 1
        && view.candidates[0].note.as_deref() == Some("routine")
        && view.candidates[0].action.executable_input().is_some()
    {
        return Some(AutoAdvanceCandidate {
            candidate: &view.candidates[0],
            class: AutoAdvanceClass::Routine,
            reason: "single routine action",
        });
    }

    let executable = view
        .candidates
        .iter()
        .filter(|candidate| candidate.action.executable_input().is_some())
        .collect::<Vec<_>>();
    if executable.len() == 1 {
        let candidate = executable[0];
        let class = classify_single_executable_candidate(session, candidate);
        if matches!(class, AutoAdvanceClass::Routine | AutoAdvanceClass::Forced) {
            return Some(AutoAdvanceCandidate {
                candidate,
                class,
                reason: single_candidate_reason(session, candidate, class),
            });
        }
    }

    None
}

fn classify_single_executable_candidate(
    session: &RunControlSession,
    candidate: &DecisionCandidate,
) -> AutoAdvanceClass {
    if candidate.action.executable_input().is_none() {
        return AutoAdvanceClass::Unsafe;
    }
    match &session.engine_state {
        EngineState::TreasureRoom(_)
            if candidate.action.executable_input() == Some(ClientInput::OpenChest) =>
        {
            AutoAdvanceClass::Routine
        }
        EngineState::Shop(_) if candidate.id == "leave" => AutoAdvanceClass::Routine,
        EngineState::RewardScreen(reward)
            if reward.pending_card_choice.is_none()
                && reward.items.is_empty()
                && candidate.action.executable_input() == Some(ClientInput::Proceed) =>
        {
            AutoAdvanceClass::Routine
        }
        EngineState::RewardOverlay { reward_state, .. }
            if reward_state.pending_card_choice.is_none()
                && reward_state.items.is_empty()
                && candidate.action.executable_input() == Some(ClientInput::Cancel) =>
        {
            AutoAdvanceClass::Routine
        }
        EngineState::EventRoom if event_single_candidate_is_safe(session, candidate) => {
            AutoAdvanceClass::Forced
        }
        EngineState::RunPendingChoice(choice)
            if choice.min_choices == 1 && choice.max_choices == 1 =>
        {
            AutoAdvanceClass::Forced
        }
        EngineState::GameOver(_) => AutoAdvanceClass::Unsafe,
        _ => AutoAdvanceClass::Strategic,
    }
}

fn event_single_candidate_is_safe(
    session: &RunControlSession,
    candidate: &DecisionCandidate,
) -> bool {
    if session.run_state.event_state.as_ref().is_some_and(|event| {
        event.id == crate::state::events::EventId::Neow && event.current_screen > 0
    }) {
        return false;
    }
    let Some(resolution) = candidate.resolution.as_ref() else {
        return candidate.note.as_deref() == Some("routine");
    };
    resolution.known_effects.is_empty()
        && resolution.unresolved_effects.is_empty()
        && matches!(
            resolution.followup,
            Some(
                super::view_model::FollowupBoundary::EventScreenAdvance
                    | super::view_model::FollowupBoundary::EventComplete
            )
        )
}

fn single_candidate_reason(
    session: &RunControlSession,
    candidate: &DecisionCandidate,
    class: AutoAdvanceClass,
) -> &'static str {
    match (&session.engine_state, class, candidate.id.as_str()) {
        (EngineState::TreasureRoom(_), AutoAdvanceClass::Routine, _) => "single chest action",
        (EngineState::Shop(_), AutoAdvanceClass::Routine, "leave") => "only shop exit remains",
        (EngineState::EventRoom, AutoAdvanceClass::Forced, _) => "single safe event transition",
        (EngineState::RunPendingChoice(_), AutoAdvanceClass::Forced, _) => {
            "single forced run choice"
        }
        _ => "single safe action",
    }
}

fn auto_class_label(class: AutoAdvanceClass) -> &'static str {
    match class {
        AutoAdvanceClass::Routine => "routine",
        AutoAdvanceClass::Forced => "forced",
        AutoAdvanceClass::Strategic => "strategic",
        AutoAdvanceClass::Unsafe => "unsafe",
    }
}

fn human_stop_reason(session: &RunControlSession) -> String {
    match &session.engine_state {
        EngineState::EventRoom => {
            let is_neow_bonus = session.run_state.event_state.as_ref().is_some_and(|event| {
                event.id == crate::state::events::EventId::Neow && event.current_screen > 0
            });
            if is_neow_bonus {
                "Neow bonus requires human choice".to_string()
            } else {
                "event option requires human choice".to_string()
            }
        }
        EngineState::MapNavigation => "map route requires human choice".to_string(),
        EngineState::MapOverlay { .. } => "map preview requires route choice or cancel".to_string(),
        EngineState::RewardScreen(reward) if reward.pending_card_choice.is_some() => {
            "card reward requires human choice".to_string()
        }
        EngineState::RewardOverlay { reward_state, .. }
            if reward_state.pending_card_choice.is_some() =>
        {
            "card reward requires human choice".to_string()
        }
        EngineState::RewardScreen(reward) if reward_has_card_item(reward) => {
            "card reward requires human choice".to_string()
        }
        EngineState::RewardOverlay { reward_state, .. } if reward_has_card_item(reward_state) => {
            "card reward requires human choice".to_string()
        }
        EngineState::RewardScreen(reward)
            if reward_has_relic_item(reward) && reward_has_sapphire_key_item(reward) =>
        {
            "relic reward or Sapphire Key requires human choice".to_string()
        }
        EngineState::RewardOverlay { reward_state, .. }
            if reward_has_relic_item(reward_state)
                && reward_has_sapphire_key_item(reward_state) =>
        {
            "relic reward or Sapphire Key requires human choice".to_string()
        }
        EngineState::RewardScreen(reward) if reward_has_relic_item(reward) => {
            "relic reward requires human choice".to_string()
        }
        EngineState::RewardOverlay { reward_state, .. } if reward_has_relic_item(reward_state) => {
            "relic reward requires human choice".to_string()
        }
        EngineState::RewardScreen(reward) if !reward.items.is_empty() => {
            "remaining reward requires human choice".to_string()
        }
        EngineState::RewardOverlay { reward_state, .. } if !reward_state.items.is_empty() => {
            "remaining overlay reward requires human choice".to_string()
        }
        EngineState::RewardScreen(_) => {
            "reward screen cannot be advanced automatically".to_string()
        }
        EngineState::RewardOverlay { .. } => {
            "overlay reward screen cannot be advanced automatically".to_string()
        }
        EngineState::TreasureRoom(_) => {
            "treasure room is not at an executable routine boundary".to_string()
        }
        EngineState::Campfire => "campfire action requires human choice".to_string(),
        EngineState::Shop(_) => "shop action requires human choice".to_string(),
        EngineState::RunPendingChoice(_) => "card selection requires human choice".to_string(),
        EngineState::BossRelicSelect(_) => "boss relic choice requires human choice".to_string(),
        EngineState::CombatStart(_) => {
            "combat start is not yet a stable player boundary".to_string()
        }
        EngineState::CombatProcessing => "combat is still processing".to_string(),
        EngineState::CombatPlayerTurn | EngineState::PendingChoice(_) => {
            "combat boundary requires search or human action".to_string()
        }
        EngineState::GameOver(_) => "run is over".to_string(),
    }
}

fn reward_has_card_item(reward: &crate::state::rewards::RewardState) -> bool {
    reward
        .items
        .iter()
        .any(|item| matches!(item, crate::state::rewards::RewardItem::Card { .. }))
}

fn reward_has_relic_item(reward: &crate::state::rewards::RewardState) -> bool {
    reward
        .items
        .iter()
        .any(|item| matches!(item, crate::state::rewards::RewardItem::Relic { .. }))
}

fn reward_has_sapphire_key_item(reward: &crate::state::rewards::RewardState) -> bool {
    reward
        .items
        .iter()
        .any(|item| matches!(item, crate::state::rewards::RewardItem::SapphireKey))
}

fn finish_auto_step(
    session: &RunControlSession,
    before: &RunVisibleSnapshot,
    applied: Vec<String>,
    mut trace_annotations: Vec<RunControlTraceAnnotationV1>,
    reason: impl Into<String>,
    detail: Option<String>,
) -> Result<RunControlCommandOutcome, String> {
    let reason = reason.into();
    let view = build_run_control_view_model(session);
    let mut lines = vec![
        format!("Advanced to human boundary: {}", view.header.title),
        "Applied:".to_string(),
    ];
    if applied.is_empty() {
        lines.push("  none".to_string());
    } else {
        for item in &applied {
            lines.push(format!("  - {item}"));
        }
    }
    lines.push(format!("Reason: {reason}"));
    lines.push(super::next_hint::run_control_next_hint(session).to_string());
    if let Some(detail) = detail.filter(|detail| !detail.trim().is_empty()) {
        lines.push("Detail:".to_string());
        lines.extend(detail.lines().map(|line| format!("  {line}")));
    }
    if let Some(annotation) =
        super::noncombat_boundary::noncombat_human_boundary_annotation(session, &reason)?
    {
        trace_annotations.push(annotation);
    }

    if applied.is_empty() {
        lines.push(super::render::render_run_control_state(session));
        return Ok(RunControlCommandOutcome::message(lines.join("\n"))
            .with_trace_annotations(trace_annotations));
    }

    let after = RunVisibleSnapshot::capture(session);
    let action_result = action_result_from_transition(
        TransitionAction {
            label: format!(
                "advance-to-human-boundary applied {} operation(s)",
                applied.len()
            ),
        },
        before,
        &after,
        current_run_apply_status(session),
    );
    lines.push(render_action_result(&action_result));
    lines.push(super::render::render_run_control_state(session));
    Ok(
        RunControlCommandOutcome::action(lines.join("\n"), action_result)
            .with_trace_annotations(trace_annotations),
    )
}

fn auto_boundary_key(session: &RunControlSession) -> String {
    let view = build_run_control_view_model(session);
    let active_combat = session
        .active_combat
        .as_ref()
        .map(active_combat_boundary_key)
        .unwrap_or_else(|| "no-combat".to_string());
    let event = session
        .run_state
        .event_state
        .as_ref()
        .map(|event| format!("{:?}:screen{}", event.id, event.current_screen))
        .unwrap_or_else(|| "no-event".to_string());
    let candidates = view
        .candidates
        .iter()
        .map(|candidate| format!("{}={}", candidate.id, candidate.action.command_hint()))
        .collect::<Vec<_>>()
        .join(",");
    let (player_hp, _) = session.visible_player_hp();
    format!(
        "{:?}|{}|{}|act{}|floor{}|hp{}|gold{}|{}|{}",
        session.engine_state,
        view.header.title,
        event,
        session.run_state.act_num,
        session.run_state.floor_num,
        player_hp,
        session.run_state.gold,
        active_combat,
        candidates
    )
}

fn active_combat_boundary_key(active: &ActiveCombat) -> String {
    let combat = &active.combat_state;
    let player = &combat.entities.player;
    let zones = &combat.zones;
    let monsters = combat
        .entities
        .monsters
        .iter()
        .map(|monster| {
            format!(
                "slot{}:id{}:hp{}/{}:block{}:dying{}:escaped{}:half{}:move{}:move_state{:?}",
                monster.slot,
                monster.monster_type,
                monster.current_hp,
                monster.max_hp,
                monster.block,
                monster.is_dying,
                monster.is_escaped,
                monster.half_dead,
                monster.planned_move_id(),
                monster.move_state,
            )
        })
        .collect::<Vec<_>>()
        .join(",");
    let mut powers = combat
        .entities
        .power_db
        .iter()
        .map(|(entity, powers)| format!("{entity}:{powers:?}"))
        .collect::<Vec<_>>();
    powers.sort();

    format!(
        "{:?}:turn{}:{:?}:energy{}:draw_mod{}:player_hp{}/{}:block{}:stance{:?}:gold{}:hand[{}]:draw[{}]:discard[{}]:exhaust[{}]:limbo[{}]:queued{:?}:uuid{}:monsters[{}]:powers[{}]:queue{:?}",
        active.engine_state,
        combat.turn.turn_count,
        combat.turn.current_phase,
        combat.turn.energy,
        combat.turn.turn_start_draw_modifier,
        player.current_hp,
        player.max_hp,
        player.block,
        player.stance,
        player.gold,
        combat_card_sequence_key(&zones.hand),
        combat_card_sequence_key(&zones.draw_pile),
        combat_card_sequence_key(&zones.discard_pile),
        combat_card_sequence_key(&zones.exhaust_pile),
        combat_card_sequence_key(&zones.limbo),
        zones.queued_cards,
        zones.card_uuid_counter,
        monsters,
        powers.join(","),
        combat.engine.action_queue,
    )
}

fn combat_card_sequence_key(cards: &[CombatCard]) -> String {
    cards
        .iter()
        .map(combat_card_boundary_key)
        .collect::<Vec<_>>()
        .join(",")
}

fn combat_card_boundary_key(card: &CombatCard) -> String {
    format!(
        "{:?}+{}#{}:misc{}:cost{}:turn{:?}:free{}:ex{:?}:ret{:?}:d{:?}:b{:?}:dm{}:bm{}:mm{}",
        card.id,
        card.upgrades,
        card.uuid,
        card.misc_value,
        card.cost_modifier,
        card.cost_for_turn,
        card.free_to_play_once,
        card.exhaust_override,
        card.retain_override,
        card.base_damage_override,
        card.base_block_override,
        card.base_damage_mut,
        card.base_block_mut,
        card.base_magic_num_mut,
    )
}

fn current_run_apply_status(session: &RunControlSession) -> RunApplyStatus {
    match session.engine_state {
        EngineState::GameOver(RunResult::Victory) => RunApplyStatus::Victory,
        EngineState::GameOver(RunResult::Defeat) => RunApplyStatus::Defeat,
        _ => RunApplyStatus::Running,
    }
}

fn trim_search_rejection(message: &str) -> String {
    message
        .lines()
        .take_while(|line| !line.starts_with("===="))
        .take(12)
        .collect::<Vec<_>>()
        .join("\n")
}

fn combine_search_rejections(no_potion: Option<String>, fallback: String) -> String {
    match no_potion {
        Some(no_potion) => format!(
            "No-potion attempt:\n{}\n\nFallback attempt:\n{}",
            indent_block(&no_potion),
            indent_block(&fallback)
        ),
        None => fallback,
    }
}

fn combine_three_search_rejections(
    no_potion: Option<String>,
    fallback: String,
    potion_rescue: String,
) -> String {
    let mut sections = Vec::new();
    if let Some(no_potion) = no_potion {
        sections.push(format!("No-potion attempt:\n{}", indent_block(&no_potion)));
    }
    sections.push(format!("Fallback attempt:\n{}", indent_block(&fallback)));
    sections.push(format!(
        "Potion rescue attempt:\n{}",
        indent_block(&potion_rescue)
    ));
    sections.join("\n\n")
}

fn indent_block(text: &str) -> String {
    text.lines()
        .map(|line| format!("  {line}"))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::{
        apply_guarded_auto_step, auto_boundary_key, auto_no_potion_first_options,
        auto_potion_rescue_options, auto_search_options,
        high_stakes_auto_search_requires_hp_loss_gate,
    };
    use crate::ai::combat_search_v2::{
        CombatSearchV2FrontierPolicy, CombatSearchV2PotionPolicy, CombatSearchV2TurnPlanPolicy,
    };
    use crate::content::potions::{Potion, PotionId};
    use crate::eval::run_control::{
        RunControlConfig, RunControlHpLossLimit, RunControlSearchCombatOptions,
        RunControlSearchEvidenceTarget, RunControlSession,
    };
    use crate::state::core::{ActiveCombat, CombatContext, EngineState, RoomCombatContext};
    use crate::state::map::node::RoomType;
    use std::path::PathBuf;

    #[test]
    fn auto_boundary_key_distinguishes_combat_enemy_hp_changes() {
        let mut first =
            crate::test_support::combat_with_monsters(vec![crate::test_support::test_monster(
                crate::content::monsters::EnemyId::Cultist,
            )]);
        first.zones.hand = vec![crate::runtime::combat::CombatCard::new(
            crate::content::cards::CardId::Strike,
            10,
        )];
        first.entities.monsters[0]
            .set_planned_visible_spec(Some(crate::runtime::monster_move::MonsterMoveSpec::Unknown));
        let mut second = first.clone();
        second.entities.monsters[0].current_hp -= 1;

        let make_session = |combat| {
            let mut session = RunControlSession::new(RunControlConfig::default());
            session.engine_state = EngineState::CombatPlayerTurn;
            session.active_combat = Some(ActiveCombat::new(
                EngineState::CombatPlayerTurn,
                combat,
                CombatContext::Room(RoomCombatContext {
                    room_type: RoomType::MonsterRoom,
                }),
            ));
            session
        };

        assert_eq!(first.zones.hand.len(), second.zones.hand.len());
        assert_ne!(
            auto_boundary_key(&make_session(first)),
            auto_boundary_key(&make_session(second))
        );
    }

    #[test]
    fn auto_search_options_only_add_interactive_time_budget_without_strategy_overrides() {
        let session = RunControlSession::new(RunControlConfig {
            search_wall_ms: Some(30_000),
            ..RunControlConfig::default()
        });

        let options = auto_search_options(&session, RunControlSearchCombatOptions::default());
        assert_eq!(options.wall_ms, None);
        assert_eq!(options.turn_plan_policy, None);
        assert_eq!(options.frontier_policy, None);

        let options = auto_search_options(
            &session,
            RunControlSearchCombatOptions {
                wall_ms: Some(500),
                turn_plan_policy: Some(CombatSearchV2TurnPlanPolicy::DiagnosticOnly),
                frontier_policy: Some(CombatSearchV2FrontierPolicy::SingleQueue),
                ..RunControlSearchCombatOptions::default()
            },
        );
        assert_eq!(options.wall_ms, Some(500));
        assert_eq!(
            options.turn_plan_policy,
            Some(CombatSearchV2TurnPlanPolicy::DiagnosticOnly)
        );
        assert_eq!(
            options.frontier_policy,
            Some(CombatSearchV2FrontierPolicy::SingleQueue)
        );

        let session = RunControlSession::new(RunControlConfig::default());
        let options = auto_search_options(&session, RunControlSearchCombatOptions::default());
        assert_eq!(options.wall_ms, Some(5_000));
    }

    #[test]
    fn auto_search_options_enables_semantic_potions_for_boss_auto_combat() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut combat = crate::test_support::blank_test_combat();
        combat.meta.is_boss_fight = true;
        session.engine_state = EngineState::CombatPlayerTurn;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoomBoss,
            }),
        ));

        let options = auto_search_options(&session, RunControlSearchCombatOptions::default());

        assert_eq!(
            options.potion_policy,
            Some(CombatSearchV2PotionPolicy::SemanticBudgeted)
        );
        assert_eq!(options.max_potions_used, Some(2));
    }

    #[test]
    fn auto_search_options_enables_single_semantic_potion_for_elite_auto_combat() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut combat = crate::test_support::blank_test_combat();
        combat.meta.is_elite_fight = true;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoomElite,
            }),
        ));

        let options = auto_search_options(&session, RunControlSearchCombatOptions::default());

        assert_eq!(
            options.potion_policy,
            Some(CombatSearchV2PotionPolicy::SemanticBudgeted)
        );
        assert_eq!(options.max_potions_used, Some(1));
    }

    #[test]
    fn auto_search_options_keeps_potions_disabled_for_ordinary_auto_combat() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            crate::test_support::blank_test_combat(),
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));

        let options = auto_search_options(&session, RunControlSearchCombatOptions::default());

        assert_eq!(options.potion_policy, None);
        assert_eq!(options.max_potions_used, None);
    }

    #[test]
    fn auto_search_options_keeps_user_potion_overrides_for_high_stakes_auto_combat() {
        let mut session = RunControlSession::new(RunControlConfig {
            search_potion_policy: Some(CombatSearchV2PotionPolicy::Never),
            ..RunControlConfig::default()
        });
        let mut combat = crate::test_support::blank_test_combat();
        combat.meta.is_elite_fight = true;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoomElite,
            }),
        ));

        let options = auto_search_options(&session, RunControlSearchCombatOptions::default());
        assert_eq!(options.potion_policy, None);

        let options = auto_search_options(
            &session,
            RunControlSearchCombatOptions {
                potion_policy: Some(CombatSearchV2PotionPolicy::All),
                max_potions_used: Some(1),
                ..RunControlSearchCombatOptions::default()
            },
        );
        assert_eq!(options.potion_policy, Some(CombatSearchV2PotionPolicy::All));
        assert_eq!(options.max_potions_used, Some(1));
    }

    #[test]
    fn auto_no_potion_first_uses_hp_loss_limit_without_saving_probe_evidence() {
        let mut session = RunControlSession::new(RunControlConfig {
            search_max_hp_loss: Some(12),
            ..RunControlConfig::default()
        });
        let mut combat = crate::test_support::blank_test_combat();
        combat.meta.is_boss_fight = true;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoomBoss,
            }),
        ));

        let probe =
            auto_no_potion_first_options(&session, &RunControlSearchCombatOptions::default())
                .expect("hp-loss-limited boss auto combat should try no-potion first");

        assert_eq!(probe.wall_ms, Some(5_000));
        assert_eq!(probe.potion_policy, Some(CombatSearchV2PotionPolicy::Never));
        assert_eq!(probe.max_potions_used, Some(0));
        assert_eq!(probe.evidence, None);

        let mut no_limit = RunControlSession::new(RunControlConfig::default());
        let mut no_limit_combat = crate::test_support::blank_test_combat();
        no_limit_combat.meta.is_boss_fight = true;
        no_limit.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            no_limit_combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoomBoss,
            }),
        ));
        assert_eq!(
            auto_no_potion_first_options(&no_limit, &RunControlSearchCombatOptions::default()),
            None
        );

        let with_evidence = RunControlSearchCombatOptions {
            max_hp_loss: Some(RunControlHpLossLimit::Limit(8)),
            evidence: Some(RunControlSearchEvidenceTarget::Path(PathBuf::from(
                "search.json",
            ))),
            ..RunControlSearchCombatOptions::default()
        };
        assert_eq!(auto_no_potion_first_options(&session, &with_evidence), None);
    }

    #[test]
    fn high_stakes_auto_search_requires_explicit_hp_loss_gate() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        let mut combat = crate::test_support::blank_test_combat();
        combat.meta.is_boss_fight = true;
        session.engine_state = EngineState::CombatPlayerTurn;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoomBoss,
            }),
        ));

        assert!(high_stakes_auto_search_requires_hp_loss_gate(
            &session,
            &RunControlSearchCombatOptions::default()
        ));
        assert!(!high_stakes_auto_search_requires_hp_loss_gate(
            &session,
            &RunControlSearchCombatOptions {
                max_hp_loss: Some(RunControlHpLossLimit::Limit(20)),
                ..RunControlSearchCombatOptions::default()
            }
        ));
        assert!(!high_stakes_auto_search_requires_hp_loss_gate(
            &session,
            &RunControlSearchCombatOptions {
                max_hp_loss: Some(RunControlHpLossLimit::Unlimited),
                ..RunControlSearchCombatOptions::default()
            }
        ));

        let outcome = apply_guarded_auto_step(&mut session, Default::default())
            .expect("guarded auto-step should reject without mutating");

        assert!(outcome.action_result.is_none());
        assert!(outcome
            .message
            .contains("Reason: high-stakes combat auto-search requires an hp-loss gate"));
        assert!(outcome.message.contains("n max_hp_loss=N"));
        assert!(matches!(
            session.engine_state,
            EngineState::CombatPlayerTurn
        ));
        assert!(session.active_combat.is_some());
    }

    #[test]
    fn ordinary_auto_search_does_not_require_hp_loss_gate() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::CombatPlayerTurn;
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            crate::test_support::blank_test_combat(),
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));

        assert!(!high_stakes_auto_search_requires_hp_loss_gate(
            &session,
            &RunControlSearchCombatOptions::default()
        ));
    }

    #[test]
    fn auto_potion_rescue_uses_one_potion_only_when_hp_loss_gate_is_set() {
        let mut session = RunControlSession::new(RunControlConfig {
            search_max_hp_loss: Some(8),
            ..RunControlConfig::default()
        });
        session.engine_state = EngineState::CombatPlayerTurn;
        let mut combat = crate::test_support::blank_test_combat();
        combat.entities.potions = vec![Some(Potion::new(PotionId::FirePotion, 42))];
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));

        let rescue =
            auto_potion_rescue_options(&session, &RunControlSearchCombatOptions::default())
                .expect("hp-loss-limited ordinary combat with potion should allow rescue attempt");

        assert_eq!(rescue.wall_ms, Some(5_000));
        assert_eq!(rescue.potion_policy, Some(CombatSearchV2PotionPolicy::All));
        assert_eq!(rescue.max_potions_used, Some(1));
    }

    #[test]
    fn auto_potion_rescue_respects_explicit_potion_policy_and_missing_gate() {
        let mut session = RunControlSession::new(RunControlConfig::default());
        session.engine_state = EngineState::CombatPlayerTurn;
        let mut combat = crate::test_support::blank_test_combat();
        combat.entities.potions = vec![Some(Potion::new(PotionId::FirePotion, 42))];
        session.active_combat = Some(ActiveCombat::new(
            EngineState::CombatPlayerTurn,
            combat,
            CombatContext::Room(RoomCombatContext {
                room_type: RoomType::MonsterRoom,
            }),
        ));

        assert_eq!(
            auto_potion_rescue_options(&session, &RunControlSearchCombatOptions::default()),
            None
        );

        let blocked_by_explicit_policy = RunControlSearchCombatOptions {
            max_hp_loss: Some(RunControlHpLossLimit::Limit(8)),
            potion_policy: Some(CombatSearchV2PotionPolicy::Never),
            ..RunControlSearchCombatOptions::default()
        };
        assert_eq!(
            auto_potion_rescue_options(&session, &blocked_by_explicit_policy),
            None
        );
    }
}
