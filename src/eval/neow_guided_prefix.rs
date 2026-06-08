use crate::ai::neow_policy_v1::{
    choices_from_event_options_v1, neow_followup_selection_v1, neow_map_features_from_run_state_v1,
    rank_neow_choices_v1, NeowDecisionInputV1, NeowGuidanceConfigV1,
};
use crate::content::events::neow;
use crate::eval::run_control::{parse_run_control_command, RunControlConfig, RunControlSession};
use crate::state::core::EngineState;
use crate::state::events::EventId;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct NeowGuidedPrefixConfigV1 {
    pub seed: u64,
    pub ascension_level: u8,
    pub final_act: bool,
    pub player_class: &'static str,
    pub search_max_nodes: Option<usize>,
    pub search_wall_ms: Option<u64>,
}

pub fn neow_guided_prefix_commands_v1(
    config: &NeowGuidedPrefixConfigV1,
) -> Result<Vec<String>, String> {
    let mut prefix = vec!["0".to_string()];
    let mut session = RunControlSession::new(RunControlConfig {
        seed: config.seed,
        ascension_level: config.ascension_level,
        final_act: config.final_act,
        player_class: config.player_class,
        search_max_nodes: config.search_max_nodes,
        search_wall_ms: config.search_wall_ms,
        ..RunControlConfig::default()
    });
    session.apply_command(parse_run_control_command("0")?)?;
    let Some(event_state) = session.run_state.event_state.as_ref() else {
        return Ok(prefix);
    };
    if event_state.id != EventId::Neow || event_state.current_screen != 1 {
        return Ok(prefix);
    }

    let options = neow::get_options(&session.run_state, event_state);
    let trace = rank_neow_choices_v1(NeowDecisionInputV1 {
        player_class: config.player_class.to_string(),
        map: neow_map_features_from_run_state_v1(&session.run_state),
        choices: choices_from_event_options_v1(&options),
        config: NeowGuidanceConfigV1::default(),
    });
    if let Some(selected) = trace.selected() {
        let neow_choice_command = selected.index.to_string();
        prefix.push(neow_choice_command.clone());
        session.apply_command(parse_run_control_command(&neow_choice_command)?)?;
        if is_neow_followup_selection(&session) {
            if let EngineState::RunPendingChoice(choice) = &session.engine_state {
                if let Some(decision) =
                    neow_followup_selection_v1(&session.run_state, choice, config.player_class)
                {
                    prefix.push(decision.command);
                }
            }
        }
    }
    Ok(prefix)
}

fn is_neow_followup_selection(session: &RunControlSession) -> bool {
    session.run_state.event_state.as_ref().is_some_and(|event| {
        event.id == EventId::Neow
            && event.completed
            && matches!(session.engine_state, EngineState::RunPendingChoice(_))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn neow_guided_prefix_includes_intro_and_guided_choice() {
        let prefix = neow_guided_prefix_commands_v1(&NeowGuidedPrefixConfigV1 {
            seed: 521,
            ascension_level: 0,
            final_act: false,
            player_class: "Ironclad",
            search_max_nodes: None,
            search_wall_ms: Some(100),
        })
        .expect("prefix builds");

        assert_eq!(prefix.first().map(String::as_str), Some("0"));
        assert!(prefix.len() >= 2);
    }
}
