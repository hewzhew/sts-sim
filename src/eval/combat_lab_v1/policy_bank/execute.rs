use std::collections::{BTreeMap, BTreeSet, VecDeque};

use crate::ai::combat_policy_v1::{
    group_combat_scenarios_v1, step_combat_scenario_group_v1, CombatPublicActionV1,
    CombatScenarioActionPortfolioSessionV1, CombatScenarioGroupV1, CombatScenarioParticleV1,
    CombatScenarioTerminalV1,
};
use crate::sim::combat::CombatStepLimits;

use super::summary::summarize_policy_bank;
use super::types::{
    CombatLabPolicyBankErrorV1, CombatLabPolicyBankLimitsV1, CombatLabPolicyBankReportV1,
    CombatLabPolicyGapRecordV1, CombatLabPolicyInformationScopeV1,
    CombatLabPolicyScenarioOutcomeV1, CombatLabPolicyScenarioResolutionV1,
    CombatLabPolicyUnresolvedReasonV1, CombatLabPublicPolicyDecisionV1, CombatLabPublicPolicyV1,
    COMBAT_LAB_POLICY_BANK_REPORT_SCHEMA_VERSION,
};
use crate::eval::combat_lab_v1::CombatLabCompiledSampleV1;

struct ScenarioAccumulator {
    sample_index: u64,
    shuffle_seed: u64,
    start_hp: i32,
    current_hp: i32,
    turn_count: u32,
    cards_played: u32,
    potions_used: u32,
    public_action_history: Vec<CombatPublicActionV1>,
    resolution: Option<CombatLabPolicyScenarioResolutionV1>,
}

pub fn execute_combat_lab_public_policy_bank_v1<P: CombatLabPublicPolicyV1>(
    samples: &[CombatLabCompiledSampleV1],
    policy: &mut P,
    limits: CombatLabPolicyBankLimitsV1,
) -> Result<CombatLabPolicyBankReportV1, CombatLabPolicyBankErrorV1> {
    validate_inputs(samples, limits)?;

    let mut accumulators = BTreeMap::new();
    let mut particles = Vec::with_capacity(samples.len());
    for sample in samples {
        let scenario_id = scenario_id(sample.sample_index);
        let start_hp = sample.start.combat.entities.player.current_hp;
        accumulators.insert(
            scenario_id.clone(),
            ScenarioAccumulator {
                sample_index: sample.sample_index,
                shuffle_seed: sample.shuffle_seed,
                start_hp,
                current_hp: start_hp,
                turn_count: sample.start.combat.turn.turn_count,
                cards_played: sample
                    .start
                    .combat
                    .turn
                    .counters
                    .card_ids_played_this_combat
                    .len()
                    .try_into()
                    .unwrap_or(u32::MAX),
                potions_used: 0,
                public_action_history: Vec::new(),
                resolution: None,
            },
        );
        particles.push(CombatScenarioParticleV1::root(
            scenario_id,
            sample.start.clone(),
        ));
    }

    let groups = group_combat_scenarios_v1(particles).map_err(|error| {
        CombatLabPolicyBankErrorV1::ScenarioBoundary {
            message: error.to_string(),
        }
    })?;
    let mut queue = groups
        .into_iter()
        .map(|group| (0usize, group))
        .collect::<VecDeque<_>>();
    let mut information_set_decisions = 0usize;
    let mut policy_evaluation_engine_steps = 0usize;
    let mut policy_proof_information_sets = 0usize;
    let mut policy_proof_candidate_evaluations = 0usize;
    let mut execution_engine_steps = 0usize;
    let mut max_frontier_information_sets = queue.len();
    let mut gaps = Vec::new();

    while let Some((depth, group)) = queue.pop_front() {
        sync_group_public_state(&group, &mut accumulators)?;

        if depth >= limits.max_actions_per_scenario {
            mark_group_unresolved(
                &group,
                depth,
                CombatLabPolicyUnresolvedReasonV1::ActionLimit,
                &mut accumulators,
                &mut gaps,
            )?;
            continue;
        }

        if information_set_decisions >= limits.max_information_set_decisions {
            mark_group_unresolved(
                &group,
                depth,
                CombatLabPolicyUnresolvedReasonV1::DecisionBudget,
                &mut accumulators,
                &mut gaps,
            )?;
            while let Some((queued_depth, queued_group)) = queue.pop_front() {
                sync_group_public_state(&queued_group, &mut accumulators)?;
                mark_group_unresolved(
                    &queued_group,
                    queued_depth,
                    CombatLabPolicyUnresolvedReasonV1::DecisionBudget,
                    &mut accumulators,
                    &mut gaps,
                )?;
            }
            break;
        }

        let decision_index = information_set_decisions;
        information_set_decisions = information_set_decisions.saturating_add(1);
        let portfolio_session = CombatScenarioActionPortfolioSessionV1::new();
        let policy_result = policy.choose_action(CombatLabPublicPolicyDecisionV1 {
            decision_index,
            depth,
            information_set: group.view(),
            action_portfolio: portfolio_session.evaluator(&group),
        });
        policy_evaluation_engine_steps =
            policy_evaluation_engine_steps.saturating_add(portfolio_session.engine_steps());
        policy_proof_information_sets = policy_proof_information_sets
            .saturating_add(portfolio_session.proof_information_sets());
        policy_proof_candidate_evaluations = policy_proof_candidate_evaluations
            .saturating_add(portfolio_session.proof_candidate_evaluations());
        let action = match policy_result {
            Ok(action) => action,
            Err(gap) => {
                mark_group_unresolved(
                    &group,
                    depth,
                    CombatLabPolicyUnresolvedReasonV1::PolicyGap { gap },
                    &mut accumulators,
                    &mut gaps,
                )?;
                continue;
            }
        };
        if !group.view().candidates.contains(&action) {
            return Err(
                CombatLabPolicyBankErrorV1::PolicyReturnedUnavailableAction {
                    information_set: group.view().key.clone(),
                    action: format!("{action:?}"),
                },
            );
        }

        let uses_potion = matches!(action, CombatPublicActionV1::UsePotion { .. });
        for scenario_id in group.scenario_ids() {
            let accumulator = accumulators.get_mut(scenario_id).ok_or_else(|| {
                CombatLabPolicyBankErrorV1::MissingScenarioAccumulator {
                    scenario_id: scenario_id.to_string(),
                }
            })?;
            accumulator.public_action_history.push(action.clone());
            if uses_potion {
                accumulator.potions_used = accumulator.potions_used.saturating_add(1);
            }
        }

        let stepped = match portfolio_session.take_step(
            &group,
            &action,
            limits.max_engine_steps_per_action,
        ) {
            Some(stepped) => stepped,
            None => {
                let stepped = step_combat_scenario_group_v1(
                    &group,
                    &action,
                    CombatStepLimits {
                        max_engine_steps: limits.max_engine_steps_per_action,
                        deadline: None,
                    },
                )
                .map_err(|error| CombatLabPolicyBankErrorV1::ScenarioBoundary {
                    message: error.to_string(),
                })?;
                execution_engine_steps =
                    execution_engine_steps.saturating_add(stepped.view.engine_steps);
                stepped
            }
        };

        for terminal in stepped.terminal_outcomes {
            let accumulator = accumulators.get_mut(&terminal.scenario_id).ok_or_else(|| {
                CombatLabPolicyBankErrorV1::MissingScenarioAccumulator {
                    scenario_id: terminal.scenario_id.clone(),
                }
            })?;
            if accumulator.resolution.is_some() {
                return Err(CombatLabPolicyBankErrorV1::DuplicateScenarioResolution {
                    scenario_id: terminal.scenario_id,
                });
            }
            accumulator.current_hp = terminal.final_hp;
            accumulator.turn_count = terminal.turn_count;
            accumulator.cards_played = terminal.cards_played;
            accumulator.resolution = Some(match terminal.terminal {
                CombatScenarioTerminalV1::Win => CombatLabPolicyScenarioResolutionV1::Win,
                CombatScenarioTerminalV1::Loss => CombatLabPolicyScenarioResolutionV1::Loss,
                CombatScenarioTerminalV1::Escape => CombatLabPolicyScenarioResolutionV1::Escape,
            });
        }

        for next_group in stepped.next_groups {
            sync_group_public_state(&next_group, &mut accumulators)?;
            queue.push_back((depth.saturating_add(1), next_group));
        }
        max_frontier_information_sets = max_frontier_information_sets.max(queue.len());
    }

    let mut outcomes = accumulators
        .into_iter()
        .map(|(scenario_id, accumulator)| {
            let resolution = accumulator
                .resolution
                .ok_or_else(|| CombatLabPolicyBankErrorV1::IncompleteScenario { scenario_id })?;
            Ok(CombatLabPolicyScenarioOutcomeV1 {
                sample_index: accumulator.sample_index,
                shuffle_seed: accumulator.shuffle_seed,
                resolution,
                start_hp: accumulator.start_hp,
                final_observed_hp: accumulator.current_hp,
                observed_hp_loss: accumulator.start_hp - accumulator.current_hp,
                turn_count: accumulator.turn_count,
                actions: accumulator.public_action_history.len(),
                cards_played: accumulator.cards_played,
                potions_used: accumulator.potions_used,
                public_action_history: accumulator.public_action_history,
            })
        })
        .collect::<Result<Vec<_>, CombatLabPolicyBankErrorV1>>()?;
    outcomes.sort_by_key(|outcome| outcome.sample_index);
    let summary = summarize_policy_bank(&outcomes);

    Ok(CombatLabPolicyBankReportV1 {
        schema_version: COMBAT_LAB_POLICY_BANK_REPORT_SCHEMA_VERSION,
        information_scope: CombatLabPolicyInformationScopeV1::PublicHistoryScenarioPolicy,
        scenario_count: outcomes.len(),
        information_set_decisions,
        engine_steps: policy_evaluation_engine_steps.saturating_add(execution_engine_steps),
        policy_evaluation_engine_steps,
        policy_proof_information_sets,
        policy_proof_candidate_evaluations,
        execution_engine_steps,
        max_frontier_information_sets,
        gaps,
        outcomes,
        summary,
    })
}

fn validate_inputs(
    samples: &[CombatLabCompiledSampleV1],
    limits: CombatLabPolicyBankLimitsV1,
) -> Result<(), CombatLabPolicyBankErrorV1> {
    if samples.is_empty() {
        return Err(CombatLabPolicyBankErrorV1::EmptyScenarioBank);
    }
    for (field, value) in [
        (
            "max_information_set_decisions",
            limits.max_information_set_decisions,
        ),
        ("max_actions_per_scenario", limits.max_actions_per_scenario),
        (
            "max_engine_steps_per_action",
            limits.max_engine_steps_per_action,
        ),
    ] {
        if value == 0 {
            return Err(CombatLabPolicyBankErrorV1::InvalidLimit { field });
        }
    }
    let mut indices = BTreeSet::new();
    for sample in samples {
        if !indices.insert(sample.sample_index) {
            return Err(CombatLabPolicyBankErrorV1::DuplicateSampleIndex {
                sample_index: sample.sample_index,
            });
        }
    }
    Ok(())
}

fn sync_group_public_state(
    group: &CombatScenarioGroupV1,
    accumulators: &mut BTreeMap<String, ScenarioAccumulator>,
) -> Result<(), CombatLabPolicyBankErrorV1> {
    let observation = &group.view().observation;
    let current_hp = observation.observation.compatibility_public.player.hp;
    let turn_count = observation.turn_count;
    let cards_played = observation
        .observation
        .turn
        .counters
        .card_ids_played_this_combat
        .len()
        .try_into()
        .unwrap_or(u32::MAX);
    for scenario_id in group.scenario_ids() {
        let accumulator = accumulators.get_mut(scenario_id).ok_or_else(|| {
            CombatLabPolicyBankErrorV1::MissingScenarioAccumulator {
                scenario_id: scenario_id.to_string(),
            }
        })?;
        accumulator.current_hp = current_hp;
        accumulator.turn_count = turn_count;
        accumulator.cards_played = cards_played;
    }
    Ok(())
}

fn mark_group_unresolved(
    group: &CombatScenarioGroupV1,
    depth: usize,
    reason: CombatLabPolicyUnresolvedReasonV1,
    accumulators: &mut BTreeMap<String, ScenarioAccumulator>,
    gaps: &mut Vec<CombatLabPolicyGapRecordV1>,
) -> Result<(), CombatLabPolicyBankErrorV1> {
    for scenario_id in group.scenario_ids() {
        let accumulator = accumulators.get_mut(scenario_id).ok_or_else(|| {
            CombatLabPolicyBankErrorV1::MissingScenarioAccumulator {
                scenario_id: scenario_id.to_string(),
            }
        })?;
        if accumulator.resolution.is_some() {
            return Err(CombatLabPolicyBankErrorV1::DuplicateScenarioResolution {
                scenario_id: scenario_id.to_string(),
            });
        }
        accumulator.resolution = Some(CombatLabPolicyScenarioResolutionV1::Unresolved {
            reason: reason.clone(),
        });
    }
    gaps.push(CombatLabPolicyGapRecordV1 {
        information_set: group.view().key.clone(),
        depth,
        scenario_count: group.view().scenario_count,
        reason,
    });
    Ok(())
}

fn scenario_id(sample_index: u64) -> String {
    format!("combat_lab_sample:{sample_index:020}")
}
