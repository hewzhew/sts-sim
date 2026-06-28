use std::fs;
use std::path::{Path, PathBuf};

use serde_json::{json, Value};
use sts_simulator::content::monsters::EnemyId;
use sts_simulator::runtime::combat::CombatState;
use sts_simulator::runtime::rng::RngPool;
use sts_simulator::sim::combat::CombatPosition;
use sts_simulator::state::core::EngineState;

use super::{Args, Branch, BranchPathStep, BranchStatus};

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
        BranchStatus::CombatGap { boundary, reason } => (boundary.as_str(), reason.as_str()),
        _ => return Ok(None),
    };
    fs::create_dir_all(dir).map_err(|err| err.to_string())?;
    let path = dir.join(case_filename(
        args.seed,
        generation,
        branch,
        &position.combat,
    ));
    let payload = serde_json::to_string_pretty(&json!({
        "schema": "combat_gap_case",
        "source": {
            "seed": args.seed,
            "ascension": args.ascension,
            "generation": generation,
            "branch_id": branch.id,
            "parent_id": branch.parent_id,
        },
        "gap": {
            "boundary": boundary,
            "reason": reason,
            "search_nodes": args.search_nodes,
            "search_ms": args.search_ms,
            "rescue_search_nodes": args.rescue_search_nodes,
            "rescue_search_ms": args.rescue_search_ms,
        },
        "run": {
            "act": branch.session.run_state.act_num,
            "floor": branch.session.run_state.floor_num,
            "hp": branch.session.run_state.current_hp,
            "max_hp": branch.session.run_state.max_hp,
            "gold": branch.session.run_state.gold,
            "deck_size": branch.session.run_state.master_deck.len(),
            "relic_count": branch.session.run_state.relics.len(),
            "potion_slots": branch.session.run_state.potions.len(),
        },
        "combat": combat_summary(&position),
        "failed_search": branch.combat_search.last(),
        "path": branch.path.iter().map(path_step).collect::<Vec<_>>(),
        "run_rng": rng_summary(&branch.session.run_state.rng_pool),
        "position": position,
    }))
    .map_err(|err| err.to_string())?;
    fs::write(&path, payload).map_err(|err| err.to_string())?;
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

fn combat_summary(position: &CombatPosition) -> Value {
    let combat = &position.combat;
    json!({
        "engine_state": format!("{:?}", position.engine),
        "turn": combat.turn.turn_count,
        "hp": combat.entities.player.current_hp,
        "max_hp": combat.entities.player.max_hp,
        "block": combat.entities.player.block,
        "energy": combat.turn.energy,
        "enemies": living_enemy_names(combat),
        "hand": combat.zones.hand.iter().map(|card| json!({
            "id": card.id,
            "uuid": card.uuid,
            "upgrades": card.upgrades,
        })).collect::<Vec<_>>(),
        "draw_count": combat.zones.draw_pile.len(),
        "discard_count": combat.zones.discard_pile.len(),
        "exhaust_count": combat.zones.exhaust_pile.len(),
    })
}

fn path_step(step: &BranchPathStep) -> Value {
    json!({
        "key": step.key,
        "label": step.label,
    })
}

fn rng_summary(rng: &RngPool) -> Value {
    json!({
        "monster_rng": rng.monster_rng.counter,
        "event_rng": rng.event_rng.counter,
        "merchant_rng": rng.merchant_rng.counter,
        "card_rng": rng.card_rng.counter,
        "treasure_rng": rng.treasure_rng.counter,
        "relic_rng": rng.relic_rng.counter,
        "potion_rng": rng.potion_rng.counter,
        "monster_hp_rng": rng.monster_hp_rng.counter,
        "ai_rng": rng.ai_rng.counter,
        "shuffle_rng": rng.shuffle_rng.counter,
        "card_random_rng": rng.card_random_rng.counter,
        "misc_rng": rng.misc_rng.counter,
        "math_rng": rng.math_rng.counter,
    })
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

fn living_enemy_names(combat: &CombatState) -> Vec<String> {
    combat
        .entities
        .monsters
        .iter()
        .filter(|monster| monster.is_alive_for_action())
        .take(3)
        .map(|monster| {
            EnemyId::from_id(monster.monster_type)
                .map(|id| format!("{id:?}"))
                .unwrap_or_else(|| format!("monster{}", monster.monster_type))
        })
        .collect()
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
