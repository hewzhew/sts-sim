use std::path::PathBuf;

use crate::ai::combat_search_v2::{CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy};
use crate::state::core::ClientInput;

use super::super::reward_auto::{parse_on_off, parse_reward_automation_target};
use super::{
    RunControlAutoStepOptions, RunControlCommand, RunControlRouteAutomationMode,
    RunControlSearchCombatOptions, RunControlSearchEvidenceTarget,
};

pub(super) fn parse_search_combat_options(
    rest: &[&str],
) -> Result<RunControlSearchCombatOptions, String> {
    let mut options = RunControlSearchCombatOptions::default();
    for token in rest {
        let (key, value) = token
            .split_once('=')
            .ok_or_else(|| format!("search-combat option must be key=value, got '{token}'"))?;
        match key.to_ascii_lowercase().as_str() {
            "max_nodes" | "nodes" => {
                options.max_nodes = Some(parse_usize_value(value, "max_nodes")?);
            }
            "max_actions" | "max_actions_per_line" => {
                options.max_actions_per_line =
                    Some(parse_usize_value(value, "max_actions_per_line")?);
            }
            "max_steps" | "max_engine_steps_per_action" => {
                options.max_engine_steps_per_action =
                    Some(parse_usize_value(value, "max_engine_steps_per_action")?);
            }
            "wall_ms" | "ms" => {
                options.wall_ms = Some(parse_u64_value(value, "wall_ms")?);
            }
            "potion" | "potion_policy" => {
                options.potion_policy = Some(parse_potion_policy(value)?);
            }
            "max_potions" | "max_potions_used" | "potions_used" => {
                options.max_potions_used = Some(parse_u32_value(value, "max_potions_used")?);
            }
            "rollout" | "rollout_policy" => {
                options.rollout_policy = Some(parse_rollout_policy(value)?);
            }
            "rollouts" | "rollout_max_evaluations" | "max_rollouts" => {
                options.rollout_max_evaluations =
                    Some(parse_usize_value(value, "rollout_max_evaluations")?);
            }
            "rollout_actions" | "rollout_max_actions" => {
                options.rollout_max_actions =
                    Some(parse_usize_value(value, "rollout_max_actions")?);
            }
            "save" | "evidence" | "output" | "out" => {
                options.evidence = Some(parse_search_evidence_target(value));
            }
            other => return Err(format!("unknown search-combat option '{other}'")),
        }
    }
    Ok(options)
}

fn parse_search_evidence_target(value: &str) -> RunControlSearchEvidenceTarget {
    match value.to_ascii_lowercase().as_str() {
        "case" | "capture" | "last_capture" | "last-capture" => {
            RunControlSearchEvidenceTarget::LastCaptureCase
        }
        _ => RunControlSearchEvidenceTarget::Path(PathBuf::from(value)),
    }
}

pub(super) fn parse_auto_step_command(rest: &[&str]) -> Result<RunControlCommand, String> {
    let mut options = RunControlAutoStepOptions::default();
    let mut search_tokens = Vec::new();
    for token in rest {
        let Some((key, value)) = token.split_once('=') else {
            return Err(format!("advance option must be key=value, got '{token}'"));
        };
        match key.to_ascii_lowercase().as_str() {
            "max_ops" | "max_operations" | "max_steps" => {
                options.max_operations = Some(parse_usize_value(value, "max_ops")?);
            }
            "route" | "route_policy" | "route-policy" => {
                options.route = parse_route_automation_mode(value)?;
            }
            _ => search_tokens.push(*token),
        }
    }
    options.search = parse_search_combat_options(&search_tokens)?;
    Ok(RunControlCommand::AutoStep(options))
}

pub(super) fn parse_route_auto_step_command(rest: &[&str]) -> Result<RunControlCommand, String> {
    if rest.iter().any(|token| {
        token.split_once('=').is_some_and(|(key, _)| {
            matches!(
                key.to_ascii_lowercase().as_str(),
                "route" | "route_policy" | "route-policy"
            )
        })
    }) {
        return Err("nr/next-route already means route=planner; do not pass route=".to_string());
    }
    let RunControlCommand::AutoStep(mut options) = parse_auto_step_command(rest)? else {
        unreachable!("parse_auto_step_command always returns AutoStep")
    };
    options.route = RunControlRouteAutomationMode::Planner;
    Ok(RunControlCommand::AutoStep(options))
}

pub(super) fn parse_auto_reward_command(rest: &[&str]) -> Result<RunControlCommand, String> {
    match rest {
        [] | ["status"] => Ok(RunControlCommand::RewardAutomationStatus),
        [target, enabled] => Ok(RunControlCommand::SetRewardAutomation {
            target: parse_reward_automation_target(target)?,
            enabled: parse_on_off(enabled)?,
        }),
        _ => Err("auto-reward expects no args or: auto-reward gold|potion|all on|off".to_string()),
    }
}

fn parse_route_automation_mode(value: &str) -> Result<RunControlRouteAutomationMode, String> {
    match value.to_ascii_lowercase().as_str() {
        "manual" | "off" | "stop" | "human" => Ok(RunControlRouteAutomationMode::Manual),
        "planner" | "route_planner" | "route-planner" | "auto" => {
            Ok(RunControlRouteAutomationMode::Planner)
        }
        _ => Err(format!(
            "invalid route automation mode '{value}', expected manual|planner"
        )),
    }
}

fn parse_potion_policy(value: &str) -> Result<CombatSearchV2PotionPolicy, String> {
    match value.to_ascii_lowercase().as_str() {
        "never" => Ok(CombatSearchV2PotionPolicy::Never),
        "all" | "all_legal_potion_actions" => Ok(CombatSearchV2PotionPolicy::All),
        "semantic" | "semantic_budgeted" | "semantic_budgeted_potion_actions" => {
            Ok(CombatSearchV2PotionPolicy::SemanticBudgeted)
        }
        _ => Err(format!(
            "invalid potion policy '{value}', expected never|all|semantic"
        )),
    }
}

fn parse_rollout_policy(value: &str) -> Result<CombatSearchV2RolloutPolicy, String> {
    match value.to_ascii_lowercase().as_str() {
        "disabled" | "off" | "none" => Ok(CombatSearchV2RolloutPolicy::Disabled),
        "conservative" | "conservative_no_potion" | "conservative-no-potion" | "no_potion" => {
            Ok(CombatSearchV2RolloutPolicy::ConservativeNoPotion)
        }
        "phase-aware" | "phase_aware" | "phase-aware-no-potion" | "phase_aware_no_potion" => {
            Ok(CombatSearchV2RolloutPolicy::PhaseAwareNoPotion)
        }
        _ => Err(format!(
            "invalid rollout policy '{value}', expected disabled|conservative_no_potion|phase_aware_no_potion"
        )),
    }
}

pub(super) fn parse_buy_command(rest: &[&str]) -> Result<RunControlCommand, String> {
    let kind = rest
        .first()
        .ok_or_else(|| "buy requires card|relic|potion".to_string())?
        .to_ascii_lowercase();
    let index = parse_usize_arg(rest.get(1), "shop index")?;
    match kind.as_str() {
        "card" => Ok(RunControlCommand::Input(ClientInput::BuyCard(index))),
        "relic" => Ok(RunControlCommand::Input(ClientInput::BuyRelic(index))),
        "potion" => Ok(RunControlCommand::Input(ClientInput::BuyPotion(index))),
        _ => Err("buy requires card|relic|potion".to_string()),
    }
}

fn parse_usize_value(value: &str, name: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .map_err(|_| format!("invalid {name} '{value}'"))
}

fn parse_u64_value(value: &str, name: &str) -> Result<u64, String> {
    value
        .parse::<u64>()
        .map_err(|_| format!("invalid {name} '{value}'"))
}

fn parse_u32_value(value: &str, name: &str) -> Result<u32, String> {
    value
        .parse::<u32>()
        .map_err(|_| format!("invalid {name} '{value}'"))
}

pub(super) fn parse_usize_arg(value: Option<&&str>, name: &str) -> Result<usize, String> {
    value
        .ok_or_else(|| format!("missing {name}"))?
        .parse::<usize>()
        .map_err(|_| format!("invalid {name} '{}'", value.unwrap()))
}

pub(super) fn parse_optional_usize_arg(
    value: Option<&&str>,
    name: &str,
) -> Result<Option<usize>, String> {
    value
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| format!("invalid {name} '{value}'"))
        })
        .transpose()
}

pub(super) fn parse_usize_list(values: &[&str], name: &str) -> Result<Vec<usize>, String> {
    values
        .iter()
        .map(|value| {
            value
                .parse::<usize>()
                .map_err(|_| format!("invalid {name} '{value}'"))
        })
        .collect()
}

pub(super) fn parse_u32_list(values: &[&str], name: &str) -> Result<Vec<u32>, String> {
    values
        .iter()
        .map(|value| {
            value
                .parse::<u32>()
                .map_err(|_| format!("invalid {name} '{value}'"))
        })
        .collect()
}
