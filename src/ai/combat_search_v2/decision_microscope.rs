use crate::sim::combat::{CombatPosition, CombatStepLimits, CombatStepper, EngineCombatStepper};
use crate::state::core::ClientInput;

use super::*;

mod types;
pub use types::{
    CombatSearchV2ActionFactsReport, CombatSearchV2DecisionCandidateReport,
    CombatSearchV2DecisionContext, CombatSearchV2DecisionMicroscopeConfigReport,
    CombatSearchV2DecisionMicroscopeReport, CombatSearchV2DecisionOneStepReport,
    CombatSearchV2DecisionSelectedAction, CombatSearchV2DecisionTrajectorySummary,
};

const CANDIDATE_REPORT_LIMIT: usize = 24;

pub fn explain_combat_search_v2_initial_decision(
    engine: &EngineState,
    combat: &CombatState,
    config: CombatSearchV2Config,
) -> CombatSearchV2DecisionMicroscopeReport {
    explain_combat_search_v2_initial_decision_with_stepper(
        engine,
        combat,
        config,
        &EngineCombatStepper,
    )
}

fn explain_combat_search_v2_initial_decision_with_stepper(
    engine: &EngineState,
    combat: &CombatState,
    config: CombatSearchV2Config,
    stepper: &impl CombatStepper,
) -> CombatSearchV2DecisionMicroscopeReport {
    let search_report = run_combat_search_v2_with_stepper(engine, combat, config.clone(), stepper);
    let selected_first_action = selected_first_action(engine, combat, &search_report);
    let selected_identity = selected_first_action
        .as_ref()
        .map(|action| (action.action_id, action.action_key.as_str()));
    let initial_node = SearchNode {
        engine: engine.clone(),
        combat: combat.clone(),
        actions: Vec::new(),
        turn_prefix: TurnPrefixState::default(),
        initial_hp: combat.entities.player.current_hp,
        potions_used: 0,
        potions_discarded: 0,
        cards_played: 0,
        potion_tactical_priority: 0,
        last_turn_branch_priority: 0,
        rollout_estimate: RolloutNodeEstimate::unevaluated(),
    };
    let position = CombatPosition::new(engine.clone(), combat.clone());
    let legal = filtered_legal_actions(
        stepper.legal_action_choices(&position),
        config.potion_policy,
        combat,
    );
    let equivalence = compress_equivalent_actions(engine, combat, legal);
    let ordered = order_indexed_action_choices(engine, combat, equivalence.choices);
    let candidate_count = ordered.choices.len();
    let candidates = ordered
        .choices
        .iter()
        .take(CANDIDATE_REPORT_LIMIT)
        .enumerate()
        .map(|(ordered_index, choice)| {
            candidate_report(
                &initial_node,
                stepper,
                &config,
                choice,
                ordered_index,
                selected_identity,
            )
        })
        .collect();

    CombatSearchV2DecisionMicroscopeReport {
        schema_name: "CombatSearchV2DecisionMicroscopeReport",
        schema_version: 1,
        question: "why_was_this_action_selected_and_where_might_it_be_wrong",
        behavioral_scope: "diagnostic_only_no_prune_no_policy_change_no_artifact_promotion",
        input_label: config.input_label.clone(),
        config: config_report(&config),
        search_outcome: search_report.outcome.clone(),
        best_complete_summary: search_report
            .best_complete_trajectory
            .as_ref()
            .map(trajectory_summary),
        selected_first_action,
        initial_context: CombatSearchV2DecisionContext {
            state: summarize_state(engine, combat),
            phase_profile: combat_search_phase_profile_report(combat_search_phase_profile(
                engine, combat,
            )),
            frontier_value: combat_search_frontier_value_report(&initial_node),
        },
        candidate_count,
        reported_candidate_limit: CANDIDATE_REPORT_LIMIT,
        candidates,
        notes: vec![
            "this report explains the initial decision boundary only",
            "selected_first_action comes from the best complete trajectory found under the current budget",
            "candidate one-step probes are exact simulator transitions to the next stable boundary",
            "one-step values explain local consequences, not whole-combat optimality",
            "use this before changing global frontier ordering; if the failure is only a vague ordering preference, do not patch blindly",
        ],
    }
}

fn action_facts_report(facts: CombatSearchV2ActionFacts) -> CombatSearchV2ActionFactsReport {
    CombatSearchV2ActionFactsReport {
        schema_name: "CombatSearchV2ActionFactsReport",
        schema_version: 1,
        evidence_policy:
            "static_card_definition_plus_simulator_one_step_delta_no_quality_label_no_teacher_claim",
        consumer_boundary:
            "diagnostic_report_wrapper; search_value_must_consume_CombatSearchV2ActionFacts_not_report_metadata",
        facts,
        notes: vec![
            "action facts describe current-state affordances and exact one-step consequences",
            "facts do not claim the action is good or optimal",
            "one-step deltas use the supplied exact engine state and may include hidden draw/rng truth from that state",
            "long-horizon value must consume pure facts separately and remain explicit about estimate boundaries",
        ],
    }
}

fn candidate_report(
    root: &SearchNode,
    stepper: &impl CombatStepper,
    config: &CombatSearchV2Config,
    choice: &IndexedActionChoice,
    ordered_index: usize,
    selected_identity: Option<(usize, &str)>,
) -> CombatSearchV2DecisionCandidateReport {
    let input = choice.choice.input.clone();
    let role =
        combat_search_action_ordering_role_label_for_state(&root.engine, &root.combat, &input);
    let selected_by_best_complete = selected_identity
        .map(|(id, key)| id == choice.original_action_id && key == choice.choice.action_key)
        .unwrap_or(false);
    let step = stepper.apply_to_stable(
        &CombatPosition::new(root.engine.clone(), root.combat.clone()),
        input.clone(),
        CombatStepLimits {
            max_engine_steps: config.max_engine_steps_per_action,
            deadline: None,
        },
    );
    CombatSearchV2DecisionCandidateReport {
        original_action_id: choice.original_action_id,
        ordered_index,
        action_key: choice.choice.action_key.clone(),
        action_debug: choice.choice.action_debug.clone(),
        action_role: role,
        selected_by_best_complete,
        input: input.clone(),
        action_facts: action_facts_report(summarize_action_facts_from_step(
            &root.combat,
            &input,
            &step,
        )),
        one_step: one_step_report(root, &input, &step),
    }
}

fn one_step_report(
    root: &SearchNode,
    input: &ClientInput,
    step: &crate::sim::combat::CombatStepResult,
) -> CombatSearchV2DecisionOneStepReport {
    let transition = (!step.truncated && !step.timed_out && step.alive).then(|| {
        classify_turn_branch_transition(
            &root.engine,
            &root.combat,
            input,
            &step.position.engine,
            &step.position.combat,
        )
    });
    let phase_profile = combat_search_phase_profile(&step.position.engine, &step.position.combat);
    let visible_hp_loss = (phase_profile.pressure.visible_incoming_damage
        - step.position.combat.entities.player.block)
        .max(0);

    CombatSearchV2DecisionOneStepReport {
        status: step_status(&step),
        engine_steps: step.engine_steps,
        terminal: terminal_label(&step.position.engine, &step.position.combat),
        transition: transition.map(|transition| format!("{transition:?}")),
        turn_branch_priority_hint: transition.map(TurnBranchTransition::frontier_priority_hint),
        player_hp: step.position.combat.entities.player.current_hp,
        player_block: step.position.combat.entities.player.block,
        energy: step.position.combat.turn.energy,
        visible_incoming_damage: phase_profile.pressure.visible_incoming_damage,
        visible_hp_loss_if_turn_ends: visible_hp_loss,
        survival_margin: phase_profile.pressure.survival_margin,
        living_enemy_count: living_enemy_count(&step.position.combat),
        total_enemy_hp: phase_profile.enemy_phase.raw_living_enemy_hp,
        total_enemy_block: phase_profile.enemy_phase.raw_living_enemy_block,
        phase_adjusted_enemy_effort: phase_profile.enemy_phase.phase_adjusted_living_enemy_effort,
        split_debt_hp: phase_profile.enemy_phase.split_debt_hp,
        guardian_mode_shift_pending_count: phase_profile
            .enemy_mechanics
            .guardian_mode_shift_pending_count,
        lagavulin_waking_count: phase_profile.enemy_mechanics.lagavulin_waking_count,
        gremlin_nob_anger_amount_total: phase_profile
            .enemy_mechanics
            .gremlin_nob_anger_amount_total,
        sentry_dazed_pressure_count: phase_profile.enemy_mechanics.sentry_dazed_pressure_count,
        hexaghost_opening_pressure_count: phase_profile
            .enemy_mechanics
            .hexaghost_opening_pressure_count,
        pending_choice_present: phase_profile.pending_choice.present,
        pending_choice_estimated_action_fanout: phase_profile
            .pending_choice
            .estimated_action_fanout,
    }
}

fn selected_first_action(
    engine: &EngineState,
    combat: &CombatState,
    search_report: &CombatSearchV2Report,
) -> Option<CombatSearchV2DecisionSelectedAction> {
    let action = search_report
        .best_complete_trajectory
        .as_ref()?
        .actions
        .first()?;
    Some(CombatSearchV2DecisionSelectedAction {
        action_id: action.action_id,
        action_key: action.action_key.clone(),
        action_debug: action.action_debug.clone(),
        action_role: combat_search_action_ordering_role_label_for_state(
            engine,
            combat,
            &action.input,
        ),
        selection_source: "best_complete_trajectory_first_action",
    })
}

fn trajectory_summary(
    trajectory: &CombatSearchV2TrajectoryReport,
) -> CombatSearchV2DecisionTrajectorySummary {
    CombatSearchV2DecisionTrajectorySummary {
        terminal: trajectory.terminal,
        estimated: trajectory.estimated,
        final_hp: trajectory.final_hp,
        hp_loss: trajectory.hp_loss,
        turns: trajectory.turns,
        potions_used: trajectory.potions_used,
        potions_discarded: trajectory.potions_discarded,
        cards_played: trajectory.cards_played,
        action_count: trajectory.actions.len(),
    }
}

fn config_report(config: &CombatSearchV2Config) -> CombatSearchV2DecisionMicroscopeConfigReport {
    CombatSearchV2DecisionMicroscopeConfigReport {
        max_nodes: config.max_nodes,
        max_actions_per_line: config.max_actions_per_line,
        max_engine_steps_per_action: config.max_engine_steps_per_action,
        wall_time_ms: config.wall_time.map(|duration| duration.as_millis()),
        potion_policy: config.potion_policy.label(),
        max_potions_used: config.max_potions_used,
        rollout_policy: config.rollout_policy.label(),
        rollout_max_evaluations: config.rollout_max_evaluations,
        rollout_max_actions: config.rollout_max_actions,
    }
}

fn step_status(step: &crate::sim::combat::CombatStepResult) -> &'static str {
    if step.timed_out {
        "timed_out"
    } else if step.truncated {
        "engine_step_limit"
    } else if !step.alive {
        "player_dead"
    } else {
        "stable"
    }
}

#[cfg(test)]
mod tests {
    use crate::content::cards::CardId;
    use crate::content::monsters::EnemyId;
    use crate::runtime::combat::CombatCard;
    use crate::test_support::{blank_test_combat, planned_monster};

    use super::*;

    #[test]
    fn microscope_reports_selected_action_and_one_step_candidates() {
        let mut combat = blank_test_combat();
        combat.zones.hand = vec![CombatCard::new(CardId::Strike, 1)];
        combat.entities.monsters = vec![planned_monster(EnemyId::JawWorm, 1)];
        let config = CombatSearchV2Config {
            max_nodes: 200,
            rollout_policy: CombatSearchV2RolloutPolicy::Disabled,
            input_label: Some("microscope_test".to_string()),
            ..CombatSearchV2Config::default()
        };

        let report = explain_combat_search_v2_initial_decision(
            &EngineState::CombatPlayerTurn,
            &combat,
            config,
        );

        assert_eq!(
            report.question,
            "why_was_this_action_selected_and_where_might_it_be_wrong"
        );
        assert!(report.candidate_count >= 2);
        assert!(report
            .candidates
            .iter()
            .any(|candidate| candidate.one_step.status == "stable"));
        assert!(report.selected_first_action.is_some());
    }
}
