use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{to_value, Value};
use sts_simulator::eval::combat_case::{
    living_enemy_names, save_combat_case, CombatCase, CombatCaseGap, CombatCasePathStep,
    CombatCaseRngSummary, CombatCaseRunSummary, CombatCaseSource,
};
use sts_simulator::runtime::combat::CombatState;
use sts_simulator::sim::combat::CombatPosition;
use sts_simulator::state::core::EngineState;

use super::branch_path::BranchPathStep;
use super::{Args, Branch, BranchStatus};

pub(super) fn save_combat_gap_case(
    dir: &Path,
    args: Args,
    generation: usize,
    branch: &Branch,
) -> Result<Option<PathBuf>, String> {
    let Some(position) = current_stable_combat_position(branch) else {
        return Ok(None);
    };
    let (boundary, reason) = match &branch.status {
        BranchStatus::CombatGap { boundary, reason }
        | BranchStatus::OperationBudgetExhausted { boundary, reason }
        | BranchStatus::BudgetGap { boundary, reason } => (boundary.as_str(), reason.as_str()),
        _ => return Ok(None),
    };
    fs::create_dir_all(dir).map_err(|err| err.to_string())?;
    let path = dir.join(case_filename(
        args.seed,
        generation,
        branch,
        &position.combat,
    ));
    let case = CombatCase::new(
        CombatCaseSource {
            seed: args.seed,
            ascension: args.ascension,
            generation,
            branch_id: branch.id,
            parent_id: branch.parent_id,
        },
        CombatCaseGap {
            boundary: boundary.to_string(),
            reason: reason.to_string(),
            search_nodes: args.search_nodes,
            search_ms: args.search_ms,
            rescue_search_nodes: args.rescue_search_nodes,
            rescue_search_ms: args.rescue_search_ms,
        },
        CombatCaseRunSummary {
            act: branch.session.run_state.act_num,
            floor: branch.session.run_state.floor_num,
            hp: branch.session.run_state.current_hp,
            max_hp: branch.session.run_state.max_hp,
            gold: branch.session.run_state.gold,
            deck_size: branch.session.run_state.master_deck.len(),
            relic_count: branch.session.run_state.relics.len(),
            potion_slots: branch.session.run_state.potions.len(),
        },
        branch.combat_search.clone(),
        branch.combat_search.last().cloned(),
        branch.path.iter().map(path_step).collect(),
        CombatCaseRngSummary::from_pool(&branch.session.run_state.rng_pool),
        position,
    );
    save_combat_case(&path, &case)?;
    Ok(Some(path))
}

fn current_stable_combat_position(branch: &Branch) -> Option<CombatPosition> {
    let active = branch.session.active_combat.as_ref()?;
    if !matches!(
        active.engine_state,
        EngineState::CombatPlayerTurn | EngineState::PendingChoice(_)
    ) {
        return None;
    }
    Some(CombatPosition::new(
        active.engine_state.clone(),
        active.combat_state.clone(),
    ))
}

fn path_step(step: &BranchPathStep) -> CombatCasePathStep {
    CombatCasePathStep {
        key: to_value(&step.key).unwrap_or(Value::Null),
        label: step.label.clone(),
    }
}

fn case_filename(seed: u64, generation: usize, branch: &Branch, combat: &CombatState) -> String {
    let enemies = living_enemy_names(combat)
        .into_iter()
        .map(|name| slug(&name))
        .collect::<Vec<_>>()
        .join("_");
    format!(
        "seed{}_g{:02}_b{:04}_a{}f{}{}.json",
        seed,
        generation,
        branch.id,
        branch.session.run_state.act_num,
        branch.session.run_state.floor_num,
        if enemies.is_empty() {
            String::new()
        } else {
            format!("_{enemies}")
        }
    )
}

fn slug(raw: &str) -> String {
    let mut out = String::new();
    let mut last_sep = false;
    for ch in raw.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_alphanumeric() {
            out.push(ch);
            last_sep = false;
        } else if !last_sep {
            out.push('_');
            last_sep = true;
        }
    }
    out.trim_matches('_').to_string()
}
