use crate::ai::combat_search_v2::CombatSearchV2PotionPolicy;

use super::commands::{
    RunControlHpLossLimit, RunControlSearchCombatOptions, RunControlSearchDefaultsCommand,
};
use super::session::{RunControlCommandOutcome, RunControlSession};

pub(super) fn apply_search_defaults(
    session: &mut RunControlSession,
    command: RunControlSearchDefaultsCommand,
) -> Result<RunControlCommandOutcome, String> {
    match command {
        RunControlSearchDefaultsCommand::Status => Ok(RunControlCommandOutcome::message(
            render_search_defaults(session),
        )),
        RunControlSearchDefaultsCommand::Clear => {
            session.search_max_nodes = None;
            session.search_wall_ms = None;
            session.search_max_hp_loss = None;
            session.search_potion_policy = None;
            session.search_max_potions_used = None;
            Ok(RunControlCommandOutcome::message(format!(
                "search defaults cleared\n{}",
                render_search_defaults(session)
            )))
        }
        RunControlSearchDefaultsCommand::Update(options) => {
            apply_search_default_options(session, options);
            Ok(RunControlCommandOutcome::message(render_search_defaults(
                session,
            )))
        }
    }
}

fn apply_search_default_options(
    session: &mut RunControlSession,
    options: RunControlSearchCombatOptions,
) {
    if let Some(max_nodes) = options.max_nodes {
        session.search_max_nodes = Some(max_nodes);
    }
    if let Some(wall_ms) = options.wall_ms {
        session.search_wall_ms = Some(wall_ms);
    }
    if let Some(max_hp_loss) = options.max_hp_loss {
        session.search_max_hp_loss = match max_hp_loss {
            RunControlHpLossLimit::Limit(limit) => Some(limit),
            RunControlHpLossLimit::Unlimited => None,
        };
    }
    if let Some(potion_policy) = options.potion_policy {
        session.search_potion_policy = Some(potion_policy);
    }
    if let Some(max_potions_used) = options.max_potions_used {
        session.search_max_potions_used = Some(max_potions_used);
    }
}

pub(super) fn render_search_defaults(session: &RunControlSession) -> String {
    format!(
        "\
search defaults:
  max_nodes: {}
  wall_ms: {}
  max_hp_loss: {}
  potion: {}
  max_potions: {}

Commands:
  sd max_hp_loss=8
  sd max_nodes=500000 wall_ms=30000
  sd potion=never max_potions=0
  sd max_hp_loss=off
  sd clear",
        option_usize(session.search_max_nodes),
        option_u64(session.search_wall_ms),
        option_u32(session.search_max_hp_loss),
        potion_policy_label(session.search_potion_policy),
        option_u32(session.search_max_potions_used)
    )
}

fn option_usize(value: Option<usize>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "default".to_string())
}

fn option_u64(value: Option<u64>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "default".to_string())
}

fn option_u32(value: Option<u32>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "default".to_string())
}

fn potion_policy_label(policy: Option<CombatSearchV2PotionPolicy>) -> &'static str {
    match policy {
        Some(CombatSearchV2PotionPolicy::Never) => "never",
        Some(CombatSearchV2PotionPolicy::All) => "all",
        Some(CombatSearchV2PotionPolicy::SemanticBudgeted) => "semantic",
        None => "auto high-stakes",
    }
}
