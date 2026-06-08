use crate::eval::run_control::{
    parse_run_control_command, RunControlCommand, RunControlSearchCombatOptions,
};

pub fn parse_branch_experiment_search_options_v1(
    option_tokens: &[String],
) -> Result<RunControlSearchCombatOptions, String> {
    if option_tokens.is_empty() {
        return Ok(RunControlSearchCombatOptions::default());
    }
    let command = format!("search-combat {}", option_tokens.join(" "));
    match parse_run_control_command(&command)? {
        RunControlCommand::SearchCombat(options) => Ok(options),
        _ => Err("internal error: parsed search options as non-search command".to_string()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::combat_search_v2::{
        CombatSearchV2FrontierPolicy, CombatSearchV2RolloutPolicy, CombatSearchV2TurnPlanPolicy,
    };

    #[test]
    fn parses_branch_experiment_search_option_overrides_with_run_control_parser() {
        let options = parse_branch_experiment_search_options_v1(&[
            "rollout=turn_beam".to_string(),
            "beam=4".to_string(),
            "turn_plan=root_frontier_seed".to_string(),
            "frontier=single_queue".to_string(),
        ])
        .expect("search options parse");

        assert_eq!(
            options.rollout_policy,
            Some(CombatSearchV2RolloutPolicy::TurnBeamNoPotion)
        );
        assert_eq!(options.rollout_beam_width, Some(4));
        assert_eq!(
            options.turn_plan_policy,
            Some(CombatSearchV2TurnPlanPolicy::RootFrontierSeed)
        );
        assert_eq!(
            options.frontier_policy,
            Some(CombatSearchV2FrontierPolicy::SingleQueue)
        );
    }
}
