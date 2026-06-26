use sts_simulator::eval::run_control::RunControlSession;
use sts_simulator::state::core::EngineState;

use super::command_inputs::InspectEvidenceDetailV1;

pub(super) fn render_checkpoint_campfire_evidence_v1(
    session: &RunControlSession,
    detail: InspectEvidenceDetailV1,
) -> Result<String, String> {
    if !matches!(session.engine_state, EngineState::Campfire) {
        return Err(format!(
            "--inspect-campfire-evidence requires Campfire engine state, got {:?}",
            session.engine_state
        ));
    }
    let context = sts_simulator::ai::campfire_policy_v1::build_campfire_decision_context_v1(
        &session.run_state,
        sts_simulator::engine::campfire_handler::get_available_options(&session.run_state),
    );
    let decision = sts_simulator::ai::campfire_policy_v1::plan_campfire_decision_v1(
        &context,
        &sts_simulator::ai::campfire_policy_v1::CampfirePolicyConfigV1::default(),
    );
    Ok(match detail {
        InspectEvidenceDetailV1::Compact => {
            render_campfire_evidence_compact_v1(session, &context, &decision)
        }
        InspectEvidenceDetailV1::Full => {
            render_campfire_evidence_full_v1(session, &context, &decision)
        }
    }
    .join("\n"))
}

fn render_campfire_evidence_full_v1(
    session: &RunControlSession,
    context: &sts_simulator::ai::campfire_policy_v1::CampfireDecisionContextV1,
    decision: &sts_simulator::ai::campfire_policy_v1::CampfireDecisionV1,
) -> Vec<String> {
    let mut lines = Vec::new();
    let formation = context.strategy.formation_summary();
    lines.push("Campfire evidence:".to_string());
    lines.push(format!(
        "facts: act={} floor={} hp={}/{} gold={} boss={:?}",
        session.run_state.act_num,
        session.run_state.floor_num,
        session.run_state.current_hp,
        session.run_state.max_hp,
        session.run_state.gold,
        session.run_state.boss_key
    ));
    lines.push(format!(
        "execution: head={}",
        render_campfire_plan_execution_head_v1(
            &decision.selected_plan,
            &render_campfire_action_debug_v1(&decision.selected_plan.action)
        )
    ));
    lines.push(format!(
        "diagnostics: candidates={} formation={:?} needs={:?}",
        context.candidates.len(),
        formation.stage,
        formation.needs
    ));
    lines.push("candidate_pool:".to_string());
    for plan in &decision.candidate_plans {
        lines.push(format!(
            "  candidate: {}",
            render_campfire_plan_candidate_line_v1(
                plan,
                &render_campfire_action_debug_v1(&plan.action)
            )
        ));
        if let Some(tag) = &plan.strategy_tag {
            lines.push(format!("      strategy_tag={tag}"));
        }
        for reason in plan.reasons.iter().take(4) {
            lines.push(format!("      reason: {reason}"));
        }
        if let Some(candidate) = context
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == plan.plan_id)
        {
            lines.push(format!(
                "      facts: class={:?} support_gate={:?} deck_mutation_execute={:?}",
                candidate.class, candidate.support_gate, candidate.deck_mutation_execute_allowed
            ));
            lines.push(format!(
                "      diagnostics: upgrade_score={:?}",
                candidate.upgrade_plan_score_hint
            ));
            for evidence in candidate.evidence.iter().take(6) {
                lines.push(format!("      evidence: {evidence}"));
            }
            for risk in candidate.risks.iter().take(4) {
                lines.push(format!("      risk: {risk}"));
            }
        }
    }
    lines
}

fn render_campfire_evidence_compact_v1(
    session: &RunControlSession,
    context: &sts_simulator::ai::campfire_policy_v1::CampfireDecisionContextV1,
    decision: &sts_simulator::ai::campfire_policy_v1::CampfireDecisionV1,
) -> Vec<String> {
    let mut lines = Vec::new();
    let formation = context.strategy.formation_summary();
    lines.push("Campfire evidence:".to_string());
    lines.push(format!(
        "facts: act={} floor={} hp={}/{} gold={} boss={:?}",
        session.run_state.act_num,
        session.run_state.floor_num,
        session.run_state.current_hp,
        session.run_state.max_hp,
        session.run_state.gold,
        session.run_state.boss_key
    ));
    lines.push(format!(
        "execution: head={}",
        render_campfire_plan_execution_head_v1(
            &decision.selected_plan,
            &render_campfire_action_label_v1(&decision.selected_plan.action)
        )
    ));
    lines.push(format!(
        "diagnostics: candidates={} formation={:?} needs={:?} rest_vs_smith={:?}",
        context.candidates.len(),
        formation.stage,
        formation.needs,
        context.rest_vs_smith.verdict
    ));
    lines.extend(render_campfire_plan_candidate_summary_v1(decision, context));

    let samples = compact_campfire_candidate_samples_v1(decision, context);
    if samples.is_empty() {
        lines.push("candidate_samples: -".to_string());
    } else {
        lines.push(format!("candidate_samples: {}", samples.len()));
        for line in samples {
            lines.push(format!("  {line}"));
        }
    }
    lines.push(
        "candidate_plan_detail: hidden; rerun with --inspect-evidence-detail full for full campfire evidence"
            .to_string(),
    );
    lines
}

fn render_campfire_plan_candidate_summary_v1(
    decision: &sts_simulator::ai::campfire_policy_v1::CampfireDecisionV1,
    context: &sts_simulator::ai::campfire_policy_v1::CampfireDecisionContextV1,
) -> Vec<String> {
    let mut by_role = std::collections::BTreeMap::<String, usize>::new();
    let mut by_class = std::collections::BTreeMap::<String, usize>::new();
    let mut by_gate = std::collections::BTreeMap::<String, usize>::new();
    let mut executable = 0usize;
    let mut branch_active = 0usize;
    for plan in &decision.candidate_plans {
        *by_role.entry(format!("{:?}", plan.role)).or_insert(0) += 1;
        if plan.execute_autopilot {
            executable += 1;
        }
        if plan.branch_active {
            branch_active += 1;
        }
        if let Some(candidate) = context
            .candidates
            .iter()
            .find(|candidate| candidate.candidate_id == plan.plan_id)
        {
            *by_class
                .entry(format!("{:?}", candidate.class))
                .or_insert(0) += 1;
            *by_gate
                .entry(format!("{:?}", candidate.support_gate))
                .or_insert(0) += 1;
        }
    }
    vec![
        format!(
            "candidate_pool: total={} by_role=[{}] by_class=[{}]",
            decision.candidate_plans.len(),
            render_count_map_v1(&by_role),
            render_count_map_v1(&by_class),
        ),
        format!(
            "scheduler: executable={} branch_active={}",
            executable, branch_active
        ),
        format!(
            "diagnostics: support_gate=[{}]",
            render_count_map_v1(&by_gate)
        ),
    ]
}

fn compact_campfire_candidate_samples_v1(
    decision: &sts_simulator::ai::campfire_policy_v1::CampfireDecisionV1,
    context: &sts_simulator::ai::campfire_policy_v1::CampfireDecisionContextV1,
) -> Vec<String> {
    let selected_id = decision.selected_plan.plan_id.as_str();
    let mut samples = Vec::new();
    let mut sampled_plan_ids = std::collections::BTreeSet::new();
    if let Some(selected) = decision
        .candidate_plans
        .iter()
        .find(|plan| plan.plan_id == selected_id)
    {
        let candidate = campfire_candidate_evidence_by_plan_id_v1(context, &selected.plan_id);
        sampled_plan_ids.insert(selected.plan_id.clone());
        samples.push(format!(
            "execution: {}",
            render_campfire_candidate_sample_line_v1(selected, candidate)
        ));
    }
    for plan in decision
        .candidate_plans
        .iter()
        .filter(|plan| plan.branch_active && plan.plan_id != selected_id)
        .take(3)
    {
        let candidate = campfire_candidate_evidence_by_plan_id_v1(context, &plan.plan_id);
        sampled_plan_ids.insert(plan.plan_id.clone());
        samples.push(format!(
            "scheduler_branch: {}",
            render_campfire_candidate_sample_line_v1(plan, candidate)
        ));
    }
    for plan in decision
        .candidate_plans
        .iter()
        .filter(|plan| {
            !sampled_plan_ids.contains(&plan.plan_id)
                && campfire_candidate_evidence_by_plan_id_v1(context, &plan.plan_id)
                    .is_some_and(|candidate| !candidate.risks.is_empty())
        })
        .take(2)
    {
        let candidate = campfire_candidate_evidence_by_plan_id_v1(context, &plan.plan_id);
        samples.push(format!(
            "diagnostic_risk: {}",
            render_campfire_candidate_sample_line_v1(plan, candidate)
        ));
    }
    samples
}

fn campfire_candidate_evidence_by_plan_id_v1<'a>(
    context: &'a sts_simulator::ai::campfire_policy_v1::CampfireDecisionContextV1,
    plan_id: &str,
) -> Option<&'a sts_simulator::ai::campfire_policy_v1::CampfireCandidateEvidenceV1> {
    context
        .candidates
        .iter()
        .find(|candidate| candidate.candidate_id == plan_id)
}

fn render_campfire_candidate_sample_line_v1(
    plan: &sts_simulator::ai::campfire_policy_v1::CampfirePlanCandidateV1,
    candidate: Option<&sts_simulator::ai::campfire_policy_v1::CampfireCandidateEvidenceV1>,
) -> String {
    let mut parts = vec![format!(
        "{} action={} role={:?}",
        plan.plan_id,
        render_campfire_action_label_v1(&plan.action),
        plan.role
    )];
    parts.push(format!(
        "scheduler=[execute={} branch_active={}]",
        plan.execute_autopilot, plan.branch_active
    ));
    parts.push(format!(
        "diagnostics=[score={} confidence={:.2}]",
        plan.score_hint, plan.confidence
    ));
    if let Some(tag) = &plan.strategy_tag {
        parts.push(format!("tag={tag}"));
    }
    if let Some(candidate) = candidate {
        parts.push(format!(
            "class={:?} diagnostic_support_gate={:?} upgrade_score={}",
            candidate.class,
            candidate.support_gate,
            candidate
                .upgrade_plan_score_hint
                .map(|value| value.to_string())
                .unwrap_or_else(|| "-".to_string())
        ));
        if !candidate.risks.is_empty() {
            parts.push(format!(
                "risks=[{}]",
                render_short_list(&candidate.risks.iter().take(2).cloned().collect::<Vec<_>>())
            ));
        }
    }
    if !plan.reasons.is_empty() {
        parts.push(format!(
            "reasons=[{}]",
            render_short_list(&plan.reasons.iter().take(2).cloned().collect::<Vec<_>>())
        ));
    }
    parts.join(" | ")
}

fn render_campfire_plan_execution_head_v1(
    plan: &sts_simulator::ai::campfire_policy_v1::CampfirePlanCandidateV1,
    action_label: &str,
) -> String {
    format!(
        "plan_id={} action={} role={:?} execute={} diagnostics=[score={} confidence={:.2}]",
        plan.plan_id,
        action_label,
        plan.role,
        plan.execute_autopilot,
        plan.score_hint,
        plan.confidence
    )
}

fn render_campfire_plan_candidate_line_v1(
    plan: &sts_simulator::ai::campfire_policy_v1::CampfirePlanCandidateV1,
    action_label: &str,
) -> String {
    format!(
        "{} action={} role={:?} scheduler=[execute={} branch_active={}] diagnostics=[score={} confidence={:.2}]",
        plan.plan_id,
        action_label,
        plan.role,
        plan.execute_autopilot,
        plan.branch_active,
        plan.score_hint,
        plan.confidence
    )
}

fn render_campfire_action_debug_v1(
    action: &sts_simulator::ai::campfire_policy_v1::CampfirePolicyActionV1,
) -> String {
    format!("{action:?}")
}

fn render_campfire_action_label_v1(
    action: &sts_simulator::ai::campfire_policy_v1::CampfirePolicyActionV1,
) -> String {
    match action {
        sts_simulator::ai::campfire_policy_v1::CampfirePolicyActionV1::Rest { .. } => {
            "Rest".to_string()
        }
        sts_simulator::ai::campfire_policy_v1::CampfirePolicyActionV1::Smith {
            deck_index, ..
        } => format!("Smith({deck_index})"),
        sts_simulator::ai::campfire_policy_v1::CampfirePolicyActionV1::Stop { .. } => {
            "Stop".to_string()
        }
    }
}

pub(super) fn render_checkpoint_route_evidence_v1(
    session: &RunControlSession,
    detail: InspectEvidenceDetailV1,
) -> Result<String, String> {
    if !session.engine_state.is_map_surface() {
        return Err(format!(
            "--inspect-route-evidence requires MapNavigation/MapOverlay engine state, got {:?}",
            session.engine_state
        ));
    }
    let trace = sts_simulator::ai::route_planner_v1::plan_route_decision_v1(
        &session.run_state,
        &session.engine_state,
        sts_simulator::ai::route_planner_v1::RoutePlannerConfigV1::default(),
    );
    Ok(match detail {
        InspectEvidenceDetailV1::Compact => render_route_evidence_compact_v1(&trace).join("\n"),
        InspectEvidenceDetailV1::Full => {
            sts_simulator::ai::route_planner_v1::render_route_decision_trace_v1(&trace)
        }
    })
}

fn render_route_evidence_compact_v1(
    trace: &sts_simulator::ai::route_planner_v1::RouteDecisionTraceV1,
) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push("Route evidence:".to_string());
    lines.push(format!(
        "facts: act={} boss={}",
        trace.context.act,
        trace.context.boss.as_deref().unwrap_or("unknown")
    ));
    lines.push(format!(
        "diagnostics: objective={:?} mode={:?} candidates={} path_budget={} label_role={}",
        trace.objective,
        trace.selection_mode,
        trace.candidates.len(),
        trace.path_budget,
        trace.label_role
    ));
    if !trace.warnings.is_empty() {
        lines.push(format!("warnings: {}", render_short_list(&trace.warnings)));
    }
    lines.push(render_route_candidate_summary_v1(trace));
    if let Some(idx) = trace.selected_index {
        if let Some(candidate) = trace.candidates.get(idx) {
            lines.push(format!(
                "execution: head=candidate_index={} x={} room={} command={}",
                idx,
                candidate.target.x,
                route_room_label_for_compact_v1(candidate.target.room_type),
                candidate
                    .suggested_command
                    .clone()
                    .unwrap_or_else(|| "-".to_string())
            ));
            lines.push(format!(
                "diagnostics: execution_safety={:?} execution_score={:.2}",
                candidate.safety, candidate.total_score
            ));
        }
    } else {
        lines.push("execution: head=-".to_string());
    }

    let samples = compact_route_candidate_samples_v1(trace);
    if samples.is_empty() {
        lines.push("candidate_samples: -".to_string());
    } else {
        lines.push(format!("candidate_samples: {}", samples.len()));
        for line in samples {
            lines.push(format!("  {line}"));
        }
    }
    lines.push(
        "route_candidate_detail: hidden; rerun with --inspect-evidence-detail full for full route evidence"
            .to_string(),
    );
    lines
}

fn render_route_candidate_summary_v1(
    trace: &sts_simulator::ai::route_planner_v1::RouteDecisionTraceV1,
) -> String {
    let mut by_room = std::collections::BTreeMap::<String, usize>::new();
    let mut by_safety = std::collections::BTreeMap::<String, usize>::new();
    let mut by_move = std::collections::BTreeMap::<String, usize>::new();
    let mut truncated = 0usize;
    for candidate in &trace.candidates {
        *by_room
            .entry(route_room_label_for_compact_v1(candidate.target.room_type).to_string())
            .or_insert(0) += 1;
        *by_safety
            .entry(format!("{:?}", candidate.safety))
            .or_insert(0) += 1;
        *by_move
            .entry(format!("{:?}", candidate.target.move_kind))
            .or_insert(0) += 1;
        if candidate.path_summary.path_budget_exhausted {
            truncated += 1;
        }
    }
    format!(
        "candidate_routes: compact total={} by_room=[{}] by_safety=[{}] by_move=[{}] path_budget_exhausted={}",
        trace.candidates.len(),
        render_count_map_v1(&by_room),
        render_count_map_v1(&by_safety),
        render_count_map_v1(&by_move),
        truncated
    )
}

fn compact_route_candidate_samples_v1(
    trace: &sts_simulator::ai::route_planner_v1::RouteDecisionTraceV1,
) -> Vec<String> {
    trace
        .candidates
        .iter()
        .enumerate()
        .take(6)
        .map(|(idx, candidate)| {
            format!(
                "candidate_index={} x={} room={} command={} scheduler=[execution_head={}] diagnostics=[safety={:?} score={:.2}] path=[{}] terms=[card={:.2} relic={:.2} shop={:.2} heal={:.2} hp={:.2} risk={:.2}]",
                idx,
                candidate.target.x,
                route_room_label_for_compact_v1(candidate.target.room_type),
                candidate
                    .suggested_command
                    .clone()
                    .unwrap_or_else(|| "-".to_string()),
                trace.selected_index == Some(idx),
                candidate.safety,
                candidate.total_score,
                route_path_compact_v1(&candidate.path_summary),
                candidate.score_terms.card_reward,
                candidate.score_terms.relic,
                candidate.score_terms.shop,
                candidate.score_terms.heal,
                candidate.score_terms.hp_loss,
                candidate.score_terms.death_risk,
            )
        })
        .collect()
}

fn route_path_compact_v1(path: &sts_simulator::ai::route_planner_v1::RoutePathSummaryV1) -> String {
    format!(
        "paths={} elites={} fires={} shops={} unknowns={} pressure={} first_elite={}",
        path.path_count,
        route_range_compact_v1(path.min_elites, path.max_elites),
        route_range_compact_v1(path.min_fires, path.max_fires),
        route_range_compact_v1(path.min_shops, path.max_shops),
        route_range_compact_v1(path.min_unknowns, path.max_unknowns),
        route_range_compact_v1(path.min_early_pressure, path.max_early_pressure),
        route_first_elite_compact_v1(&path.first_elite),
    )
}

fn route_first_elite_compact_v1(
    segment: &sts_simulator::ai::route_planner_v1::RouteFirstEliteSegmentV1,
) -> String {
    if segment.paths_with_first_elite == 0 {
        "none".to_string()
    } else if segment.forced {
        format!(
            "forced prep={}",
            route_range_compact_v1(
                segment.min_hallway_fights_before,
                segment.max_hallway_fights_before
            )
        )
    } else if segment.optional {
        format!(
            "optional prep={}",
            route_range_compact_v1(
                segment.min_hallway_fights_before,
                segment.max_hallway_fights_before
            )
        )
    } else {
        format!(
            "seen prep={}",
            route_range_compact_v1(
                segment.min_hallway_fights_before,
                segment.max_hallway_fights_before
            )
        )
    }
}

fn route_room_label_for_compact_v1(
    room_type: Option<sts_simulator::state::map::node::RoomType>,
) -> &'static str {
    match room_type {
        Some(sts_simulator::state::map::node::RoomType::EventRoom) => "?",
        Some(sts_simulator::state::map::node::RoomType::MonsterRoom) => "Monster",
        Some(sts_simulator::state::map::node::RoomType::MonsterRoomElite) => "Elite",
        Some(sts_simulator::state::map::node::RoomType::MonsterRoomBoss) => "Boss",
        Some(sts_simulator::state::map::node::RoomType::RestRoom) => "Rest",
        Some(sts_simulator::state::map::node::RoomType::ShopRoom) => "Shop",
        Some(sts_simulator::state::map::node::RoomType::TreasureRoom) => "Treasure",
        Some(sts_simulator::state::map::node::RoomType::TrueVictoryRoom) => "TrueVictory",
        None => "Unknown",
    }
}

fn route_range_compact_v1(min: usize, max: usize) -> String {
    if min == max {
        min.to_string()
    } else {
        format!("{min}-{max}")
    }
}

pub(super) fn render_checkpoint_deck_mutation_v1(
    session: &RunControlSession,
) -> Result<String, String> {
    let EngineState::RunPendingChoice(choice) = &session.engine_state else {
        return Err(format!(
            "--inspect-deck-mutation requires RunPendingChoice engine state, got {:?}",
            session.engine_state
        ));
    };
    let decision = sts_simulator::ai::deck_mutation_compiler_v1::compile_deck_mutation_decision_v1(
        &session.run_state,
        choice,
        sts_simulator::ai::deck_mutation_compiler_v1::DeckMutationCompilerModeV1::Inspect,
    );
    Ok(
        sts_simulator::ai::deck_mutation_compiler_v1::render_compiled_deck_mutation_decision_v1(
            &decision,
        ),
    )
}

pub(super) fn render_checkpoint_card_reward_evidence_v1(
    session: &RunControlSession,
) -> Result<String, String> {
    let cards = active_or_visible_reward_cards_for_inspect_v1(session).ok_or_else(|| {
        format!(
            "--inspect-card-reward-evidence requires an open or visible card reward, got {:?}",
            session.engine_state
        )
    })?;
    let reward_cards = cards.iter().map(|card| card.id).collect::<Vec<_>>();
    let context =
        sts_simulator::ai::card_reward_policy_v1::build_card_reward_decision_context_with_current_route_v1(
            &session.run_state,
            &session.engine_state,
            cards,
        );
    let trace = sts_simulator::ai::strategic::strategic_trace_for_card_reward(&context);
    let deck_cards = session
        .run_state
        .master_deck
        .iter()
        .map(|card| card.id)
        .collect::<Vec<_>>();
    let semantic_probe =
        sts_simulator::ai::strategy::reward_semantic_probe::assess_reward_semantics_from_cards(
            &deck_cards,
            &reward_cards,
        );
    let semantic_explanation =
        sts_simulator::ai::strategy::reward_semantic_probe::explain_reward_semantics_v1(
            &semantic_probe,
        );
    let mut lines = Vec::new();
    lines.push("Card reward evidence:".to_string());
    lines.push(format!(
        "facts: act={} floor={} hp={}/{} gold={} boss={:?}",
        session.run_state.act_num,
        session.run_state.floor_num,
        session.run_state.current_hp,
        session.run_state.max_hp,
        session.run_state.gold,
        session.run_state.boss_key
    ));
    lines.push(format!(
        "facts: candidates={} deck_size={} has_singing_bowl={}",
        context.candidates.len(),
        context.deck.deck_size,
        context.has_singing_bowl
    ));
    lines.push(format!(
        "diagnostics: startup_strong_draw={}->{}",
        context.startup.strong_draw_count, context.startup.effective_strong_draw_count,
    ));
    lines.push(format!(
        "semantic_probe: deck_package strength={:?} exhaust={:?} self_damage={:?} block={:?}",
        semantic_explanation.deck_package.strength,
        semantic_explanation.deck_package.exhaust,
        semantic_explanation.deck_package.self_damage,
        semantic_explanation.deck_package.block
    ));
    lines.push("candidate_pool:".to_string());
    for (candidate_position, candidate) in context.candidates.iter().enumerate() {
        let action = sts_simulator::ai::strategic::CandidateAction::TakeCard {
            index: candidate.index,
            card: candidate.card,
        };
        let compiled = trace.compiled_for_action(&action);
        let delta = trace
            .candidate_deltas
            .iter()
            .find(|delta| delta.action == action);
        lines.push(format!(
            "  candidate: id={} label={} index={} card={:?} same_card_count={}",
            action.candidate_id(),
            candidate.name,
            candidate.index,
            candidate.card,
            candidate.same_card_count
        ));
        lines.push(format!(
            "      facts: type={:?} cost={} damage={} block={} draw={} energy={} roles=[{}]",
            candidate.card_type,
            candidate.facts.cost,
            candidate.facts.damage.total_damage,
            candidate.facts.block,
            candidate.facts.draw_cards,
            candidate.facts.energy_gain,
            render_short_list(
                &sts_simulator::ai::card_reward_policy_v1::card_reward_semantic_profile_v1(
                    &sts_simulator::state::rewards::RewardCard::new(
                        candidate.card,
                        candidate.facts.upgrades,
                    ),
                )
                .roles
                .iter()
                .map(|role| format!("{role:?}"))
                .collect::<Vec<_>>(),
            )
        ));
        if let Some(probe_candidate) = semantic_explanation.candidates.get(candidate_position) {
            lines.extend(render_reward_semantic_explanation_candidate_v1(
                probe_candidate,
            ));
        }
        if let Some(compiled) = compiled {
            lines.push(format!("      verdict: {:?}", compiled.verdict));
            lines.push(format!("      diagnostics: score={:.2}", compiled.score));
        } else {
            lines.push("      verdict: -".to_string());
            lines.push("      diagnostics: score=-".to_string());
        }
        if let Some(delta) = delta {
            lines.push(format!(
                "      diagnostics: delta_role={:?} hint={:?} positive=[{}] negative=[{}] notes=[{}]",
                delta.role,
                delta.verdict_hint,
                render_ledger_deltas(&delta.positive),
                render_ledger_deltas(&delta.negative),
                render_short_list(&delta.notes)
            ));
        }
    }
    let skip_action = if context.has_singing_bowl {
        sts_simulator::ai::strategic::CandidateAction::TakeSingingBowl { max_hp_gain: 2 }
    } else {
        sts_simulator::ai::strategic::CandidateAction::SkipCardReward
    };
    if let Some(compiled) = trace.compiled_for_action(&skip_action) {
        lines.push(format!(
            "candidate: id={} label=decline",
            skip_action.candidate_id(),
        ));
        lines.push(format!("  verdict: {:?}", compiled.verdict,));
        lines.push(format!("  diagnostics: score={:.2}", compiled.score));
    }
    if let Some(action) = trace.would_choose.as_ref() {
        lines.push(format!(
            "execution_projection: current_trace_would_choose={}",
            action.candidate_id()
        ));
    } else {
        lines.push("execution_projection: current_trace_would_choose=-".to_string());
    }
    Ok(lines.join("\n"))
}

fn render_reward_semantic_explanation_candidate_v1(
    explanation: &sts_simulator::ai::strategy::reward_semantic_probe::RewardCandidateSemanticExplanationV1,
) -> Vec<String> {
    let mut lines = Vec::new();
    lines.push(format!(
        "      semantic: package_changes=[{}] closes=[{}] opens=[{}]",
        render_short_list(&explanation.package_changes),
        render_short_list(&explanation.closes),
        render_short_list(&explanation.opens)
    ));
    lines.push(format!(
        "      semantic: candidate_facts provides=[{}] damage_uses=[{}] emits=[{}] rules=[{}] handlers=[{}]",
        render_short_list(&explanation.provides),
        render_short_list(&explanation.damage_uses),
        render_short_list(&explanation.emits),
        render_short_list(&explanation.rules),
        render_short_list(&explanation.handlers)
    ));
    lines.push(format!(
        "      semantic: burdens=[{}] duplicates=[{}] new_mechanics=[{}] new_streams=[{}] new_rules=[{}]",
        render_short_list(&explanation.burdens),
        render_short_list(&explanation.duplicates),
        render_short_list(&explanation.new_mechanics),
        render_short_list(&explanation.new_streams),
        render_short_list(&explanation.new_rules)
    ));
    lines
}

fn active_or_visible_reward_cards_for_inspect_v1(
    session: &RunControlSession,
) -> Option<Vec<sts_simulator::state::rewards::RewardCard>> {
    match &session.engine_state {
        EngineState::RewardScreen(reward) => reward
            .pending_card_choice
            .clone()
            .or_else(|| first_visible_card_reward_for_inspect_v1(reward)),
        EngineState::RewardOverlay { reward_state, .. } => reward_state
            .pending_card_choice
            .clone()
            .or_else(|| first_visible_card_reward_for_inspect_v1(reward_state)),
        _ => None,
    }
}

fn first_visible_card_reward_for_inspect_v1(
    reward: &sts_simulator::state::rewards::RewardState,
) -> Option<Vec<sts_simulator::state::rewards::RewardCard>> {
    reward.items.iter().find_map(|item| match item {
        sts_simulator::state::rewards::RewardItem::Card { cards } => Some(cards.clone()),
        _ => None,
    })
}

fn render_short_list(items: &[String]) -> String {
    if items.is_empty() {
        "-".to_string()
    } else {
        items.join(", ")
    }
}

fn render_count_map_v1(map: &std::collections::BTreeMap<String, usize>) -> String {
    if map.is_empty() {
        return "-".to_string();
    }
    map.iter()
        .map(|(key, count)| format!("{key}={count}"))
        .collect::<Vec<_>>()
        .join(" ")
}

fn render_ledger_deltas(items: &[sts_simulator::ai::strategic::LedgerDelta]) -> String {
    if items.is_empty() {
        return "-".to_string();
    }
    items
        .iter()
        .map(|delta| format!("{:?}:{:.2}:{}", delta.kind, delta.amount, delta.reason))
        .collect::<Vec<_>>()
        .join("; ")
}
