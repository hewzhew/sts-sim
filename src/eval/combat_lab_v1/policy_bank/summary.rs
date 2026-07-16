use super::types::{
    CombatLabPolicyBankSummaryV1, CombatLabPolicyNumericSummaryV1,
    CombatLabPolicyScenarioOutcomeV1, CombatLabPolicyScenarioResolutionV1,
};

pub(super) fn summarize_policy_bank(
    outcomes: &[CombatLabPolicyScenarioOutcomeV1],
) -> CombatLabPolicyBankSummaryV1 {
    let wins = outcomes
        .iter()
        .filter(|outcome| matches!(outcome.resolution, CombatLabPolicyScenarioResolutionV1::Win))
        .count();
    let losses = outcomes
        .iter()
        .filter(|outcome| {
            matches!(
                outcome.resolution,
                CombatLabPolicyScenarioResolutionV1::Loss
            )
        })
        .count();
    let unresolved = outcomes.len().saturating_sub(wins + losses);
    let terminal_hp_loss = outcomes
        .iter()
        .filter(|outcome| {
            matches!(
                outcome.resolution,
                CombatLabPolicyScenarioResolutionV1::Win
                    | CombatLabPolicyScenarioResolutionV1::Loss
            )
        })
        .map(|outcome| outcome.observed_hp_loss)
        .collect::<Vec<_>>();
    let win_hp_loss = outcomes
        .iter()
        .filter(|outcome| matches!(outcome.resolution, CombatLabPolicyScenarioResolutionV1::Win))
        .map(|outcome| outcome.observed_hp_loss)
        .collect::<Vec<_>>();
    let loss_hp_loss = outcomes
        .iter()
        .filter(|outcome| {
            matches!(
                outcome.resolution,
                CombatLabPolicyScenarioResolutionV1::Loss
            )
        })
        .map(|outcome| outcome.observed_hp_loss)
        .collect::<Vec<_>>();
    let scenario_count = outcomes.len();
    let resolved = wins + losses;

    CombatLabPolicyBankSummaryV1 {
        scenario_count,
        wins,
        losses,
        unresolved,
        resolution_rate: rate(resolved, scenario_count),
        win_rate_all_scenarios: rate(wins, scenario_count),
        win_rate_resolved: rate(wins, resolved),
        terminal_hp_loss: numeric_summary(terminal_hp_loss),
        win_hp_loss: numeric_summary(win_hp_loss),
        loss_hp_loss: numeric_summary(loss_hp_loss),
        observed_actions: numeric_summary(
            outcomes
                .iter()
                .map(|outcome| saturating_i32(outcome.actions))
                .collect(),
        ),
        observed_turns: numeric_summary(
            outcomes
                .iter()
                .map(|outcome| saturating_i32(outcome.turn_count))
                .collect(),
        ),
        observed_potions_used: numeric_summary(
            outcomes
                .iter()
                .map(|outcome| saturating_i32(outcome.potions_used))
                .collect(),
        ),
    }
}

fn numeric_summary(mut values: Vec<i32>) -> CombatLabPolicyNumericSummaryV1 {
    values.sort_unstable();
    let count = values.len();
    let mean = (count > 0)
        .then(|| values.iter().map(|value| f64::from(*value)).sum::<f64>() / count as f64);
    let median = if count == 0 {
        None
    } else if count % 2 == 1 {
        Some(f64::from(values[count / 2]))
    } else {
        Some((f64::from(values[count / 2 - 1]) + f64::from(values[count / 2])) / 2.0)
    };
    let p90_nearest_rank = if count == 0 {
        None
    } else {
        let rank = ((count * 9) + 9) / 10;
        values.get(rank.saturating_sub(1)).copied()
    };
    CombatLabPolicyNumericSummaryV1 {
        count,
        mean,
        median,
        p90_nearest_rank,
        max: values.last().copied(),
    }
}

fn rate(numerator: usize, denominator: usize) -> Option<f64> {
    (denominator > 0).then_some(numerator as f64 / denominator as f64)
}

fn saturating_i32(value: impl TryInto<i32>) -> i32 {
    value.try_into().unwrap_or(i32::MAX)
}
