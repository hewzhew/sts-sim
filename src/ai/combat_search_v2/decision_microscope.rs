use crate::sim::combat::{CombatPosition, CombatStepper, EngineCombatStepper};

use super::*;

mod candidate_probe;
mod report;
mod types;
use report::{config_report, selected_first_action, trajectory_summary};
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
    let plugin_stack = CombatSearchPluginStack::from_config(&config);
    let action_ordering_plugins = CombatSearchActionOrderingPlugins::from_stack(
        config.root_action_prior.as_ref(),
        &plugin_stack,
    );
    let search_report = run_combat_search_v2_with_stepper(engine, combat, config.clone(), stepper);
    let selected_first_action =
        selected_first_action(engine, combat, action_ordering_plugins, &search_report);
    let selected_identity = selected_first_action
        .as_ref()
        .map(|action| (action.action_id, action.action_key.as_str()));
    let initial_node = SearchNode::root(engine.clone(), combat.clone());
    let position = CombatPosition::new(engine.clone(), combat.clone());
    let legal = filtered_legal_actions(
        stepper.legal_action_choices(&position),
        plugin_stack.potion.policy,
        combat,
    );
    let equivalence = compress_equivalent_actions(engine, combat, legal);
    let ordered = order_indexed_action_choices_with_plugins(
        engine,
        combat,
        equivalence.choices,
        action_ordering_plugins,
    );
    let candidate_count = ordered.choices.len();
    let candidates = ordered
        .choices
        .iter()
        .take(CANDIDATE_REPORT_LIMIT)
        .enumerate()
        .map(|(ordered_index, choice)| {
            candidate_probe::candidate_report(
                &initial_node,
                stepper,
                &config,
                action_ordering_plugins,
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
        best_win_summary: search_report
            .best_win_trajectory
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
            "one-step values explain local consequences, not whole-combat outcome ranking",
            "use this before changing global frontier ordering; if the failure is only a vague ordering preference, do not patch blindly",
        ],
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

    #[test]
    fn microscope_candidate_order_respects_key_card_setup_bias() {
        let mut combat = blank_test_combat();
        let mut monster = planned_monster(EnemyId::JawWorm, 1);
        monster.current_hp = 50;
        monster.max_hp = 50;
        combat.entities.monsters = vec![monster];
        combat.zones.hand = vec![
            CombatCard::new(CardId::Strike, 1),
            CombatCard::new(CardId::DemonForm, 2),
        ];
        let config = CombatSearchV2Config {
            max_nodes: 20,
            rollout_policy: CombatSearchV2RolloutPolicy::Disabled,
            setup_bias_policy: CombatSearchV2SetupBiasPolicy::KeyCardOnline,
            input_label: Some("microscope_key_card_bias_test".to_string()),
            ..CombatSearchV2Config::default()
        };

        let report = explain_combat_search_v2_initial_decision(
            &EngineState::CombatPlayerTurn,
            &combat,
            config,
        );

        let demon_form = report
            .candidates
            .iter()
            .find(|candidate| {
                matches!(
                    candidate.input,
                    ClientInput::PlayCard {
                        card_index: 1,
                        target: None
                    }
                )
            })
            .expect("Demon Form candidate should be reported");
        let strike = report
            .candidates
            .iter()
            .find(|candidate| {
                matches!(
                    candidate.input,
                    ClientInput::PlayCard {
                        card_index: 0,
                        target: Some(_)
                    }
                )
            })
            .expect("Strike candidate should be reported");

        assert_eq!(demon_form.action_role, "key_setup_card");
        assert!(demon_form.ordered_index < strike.ordered_index);
    }
}
