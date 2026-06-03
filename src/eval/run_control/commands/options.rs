use std::path::PathBuf;

use crate::ai::combat_search_v2::{
    CombatSearchV2FrontierPolicy, CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy,
    CombatSearchV2TurnPlanPolicy,
};
use crate::state::core::ClientInput;

use super::super::reward_auto::{parse_on_off, parse_reward_automation_target};
use super::{
    RunControlAutoStepOptions, RunControlCommand, RunControlHpLossLimit,
    RunControlRouteAutomationMode, RunControlSearchCombatOptions, RunControlSearchDefaultsCommand,
    RunControlSearchEvidenceTarget,
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
            "max_hp_loss" | "hp_loss" | "hp_loss_limit" => {
                options.max_hp_loss = Some(parse_hp_loss_limit(value)?);
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
            "beam" | "beam_width" | "rollout_beam_width" => {
                options.rollout_beam_width = Some(parse_usize_value(value, "rollout_beam_width")?);
            }
            "turn_plan" | "turn_plan_policy" | "turn-plan-policy" => {
                options.turn_plan_policy = Some(parse_turn_plan_policy(value)?);
            }
            "frontier" | "frontier_policy" | "frontier-policy" => {
                options.frontier_policy = Some(parse_frontier_policy(value)?);
            }
            "save" | "evidence" | "output" | "out" => {
                options.evidence = Some(parse_search_evidence_target(value));
            }
            other => return Err(format!("unknown search-combat option '{other}'")),
        }
    }
    Ok(options)
}

fn parse_hp_loss_limit(value: &str) -> Result<RunControlHpLossLimit, String> {
    match value.to_ascii_lowercase().as_str() {
        "off" | "none" | "unlimited" | "no_limit" | "no-limit" => {
            Ok(RunControlHpLossLimit::Unlimited)
        }
        _ => Ok(RunControlHpLossLimit::Limit(parse_u32_value(
            value,
            "max_hp_loss",
        )?)),
    }
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

pub(super) fn parse_search_defaults_command(rest: &[&str]) -> Result<RunControlCommand, String> {
    match rest {
        [] | ["status"] => Ok(RunControlCommand::SearchDefaults(
            RunControlSearchDefaultsCommand::Status,
        )),
        ["clear"] | ["reset"] => Ok(RunControlCommand::SearchDefaults(
            RunControlSearchDefaultsCommand::Clear,
        )),
        _ => {
            let options = parse_search_combat_options(rest)?;
            validate_search_default_options(&options)?;
            Ok(RunControlCommand::SearchDefaults(
                RunControlSearchDefaultsCommand::Update(options),
            ))
        }
    }
}

fn validate_search_default_options(options: &RunControlSearchCombatOptions) -> Result<(), String> {
    let mut unsupported = Vec::new();
    if options.max_actions_per_line.is_some() {
        unsupported.push("max_actions_per_line");
    }
    if options.max_engine_steps_per_action.is_some() {
        unsupported.push("max_engine_steps_per_action");
    }
    if options.rollout_policy.is_some() {
        unsupported.push("rollout");
    }
    if options.rollout_max_evaluations.is_some() {
        unsupported.push("rollouts");
    }
    if options.rollout_max_actions.is_some() {
        unsupported.push("rollout_actions");
    }
    if options.rollout_beam_width.is_some() {
        unsupported.push("beam");
    }
    if options.turn_plan_policy.is_some() {
        unsupported.push("turn_plan");
    }
    if options.frontier_policy.is_some() {
        unsupported.push("frontier");
    }
    if options.evidence.is_some() {
        unsupported.push("save");
    }
    if unsupported.is_empty() {
        Ok(())
    } else {
        Err(format!(
            "search-defaults only stores max_nodes, wall_ms, max_hp_loss, potion, and max_potions; unsupported: {}",
            unsupported.join(", ")
        ))
    }
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

pub(super) fn parse_auto_run_command(rest: &[&str]) -> Result<RunControlCommand, String> {
    if rest.iter().any(|token| {
        token.split_once('=').is_some_and(|(key, _)| {
            matches!(
                key.to_ascii_lowercase().as_str(),
                "route" | "route_policy" | "route-policy"
            )
        })
    }) {
        return Err("auto-run already means route=planner; do not pass route=".to_string());
    }
    let RunControlCommand::AutoStep(mut options) = parse_auto_step_command(rest)? else {
        unreachable!("parse_auto_step_command always returns AutoStep")
    };
    options.route = RunControlRouteAutomationMode::Planner;
    Ok(RunControlCommand::AutoRun(options))
}

pub(super) fn parse_auto_reward_command(rest: &[&str]) -> Result<RunControlCommand, String> {
    match rest {
        [] | ["status"] => Ok(RunControlCommand::RewardAutomationStatus),
        [target, enabled] => Ok(RunControlCommand::SetRewardAutomation {
            target: parse_reward_automation_target(target)?,
            enabled: parse_on_off(enabled)?,
        }),
        _ => Err(
            "auto-reward expects no args or: auto-reward gold|potion|relic|all on|off".to_string(),
        ),
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
        "adaptive"
        | "adaptive_no_potion"
        | "adaptive-no-potion"
        | "enemy_mechanics_adaptive_no_potion"
        | "enemy-mechanics-adaptive-no-potion" => {
            Ok(CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion)
        }
        "conservative" | "conservative_no_potion" | "conservative-no-potion" | "no_potion" => {
            Ok(CombatSearchV2RolloutPolicy::ConservativeNoPotion)
        }
        "phase-aware" | "phase_aware" | "phase-aware-no-potion" | "phase_aware_no_potion" => {
            Ok(CombatSearchV2RolloutPolicy::PhaseAwareNoPotion)
        }
        "turn-beam" | "turn_beam" | "turn-beam-no-potion" | "turn_beam_no_potion" => {
            Ok(CombatSearchV2RolloutPolicy::TurnBeamNoPotion)
        }
        _ => Err(format!(
            "invalid rollout policy '{value}', expected disabled|enemy_mechanics_adaptive_no_potion|conservative_no_potion|phase_aware_no_potion|turn_beam_no_potion"
        )),
    }
}

fn parse_turn_plan_policy(value: &str) -> Result<CombatSearchV2TurnPlanPolicy, String> {
    match value.to_ascii_lowercase().as_str() {
        "diagnostic" | "diagnostic_only" | "diagnostic-only" | "off" => {
            Ok(CombatSearchV2TurnPlanPolicy::DiagnosticOnly)
        }
        "root_seed" | "root-seed" | "root_frontier_seed" | "root-frontier-seed" | "seed" => {
            Ok(CombatSearchV2TurnPlanPolicy::RootFrontierSeed)
        }
        "turn_boundary"
        | "turn-boundary"
        | "turn_boundary_seed"
        | "turn-boundary-seed"
        | "turn_boundary_frontier_seed"
        | "turn-boundary-frontier-seed" => {
            Ok(CombatSearchV2TurnPlanPolicy::TurnBoundaryFrontierSeed)
        }
        "tactical_enemy_turn_boundary_frontier_seed"
        | "tactical-enemy-turn-boundary-frontier-seed"
        | "tactical_turn_boundary_seed"
        | "tactical-turn-boundary-seed"
        | "tactical_seed"
        | "tactical-seed"
        | "support_enemy_turn_boundary_frontier_seed"
        | "support-enemy-turn-boundary-frontier-seed"
        | "support_turn_boundary_seed"
        | "support-turn-boundary-seed"
        | "support_seed"
        | "support-seed" => {
            Ok(CombatSearchV2TurnPlanPolicy::TacticalEnemyTurnBoundaryFrontierSeed)
        }
        _ => Err(format!(
            "invalid turn plan policy '{value}', expected diagnostic_only|root_frontier_seed|turn_boundary_frontier_seed|tactical_enemy_turn_boundary_frontier_seed"
        )),
    }
}

fn parse_frontier_policy(value: &str) -> Result<CombatSearchV2FrontierPolicy, String> {
    match value.to_ascii_lowercase().as_str() {
        "single" | "single_queue" | "single-queue" => Ok(CombatSearchV2FrontierPolicy::SingleQueue),
        "round_robin"
        | "round-robin"
        | "round_robin_eval_buckets"
        | "round-robin-eval-buckets"
        | "eval_buckets"
        | "eval-buckets" => Ok(CombatSearchV2FrontierPolicy::RoundRobinEvalBuckets),
        _ => Err(format!(
            "invalid frontier policy '{value}', expected single_queue|round_robin_eval_buckets"
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
