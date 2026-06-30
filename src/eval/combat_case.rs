use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::content::monsters::EnemyId;
use crate::eval::run_control::CombatSearchTraceSummary;
use crate::runtime::combat::{CombatCard, CombatState};
use crate::runtime::rng::RngPool;
use crate::sim::combat::CombatPosition;

pub const COMBAT_CASE_SCHEMA: &str = "combat_case";
const LEGACY_COMBAT_GAP_CASE_SCHEMA: &str = "combat_gap_case";

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatCase {
    pub schema: String,
    pub source: CombatCaseSource,
    pub gap: CombatCaseGap,
    pub run: CombatCaseRunSummary,
    pub combat: CombatCaseCombatSummary,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub combat_search_attempts: Vec<CombatSearchTraceSummary>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub failed_search: Option<CombatSearchTraceSummary>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub path: Vec<CombatCasePathStep>,
    pub run_rng: CombatCaseRngSummary,
    pub position: CombatPosition,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatCaseSource {
    pub seed: u64,
    pub ascension: u8,
    pub generation: usize,
    pub branch_id: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parent_id: Option<usize>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatCaseGap {
    pub boundary: String,
    pub reason: String,
    pub search_nodes: usize,
    pub search_ms: u64,
    pub rescue_search_nodes: usize,
    pub rescue_search_ms: u64,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatCaseRunSummary {
    pub act: u8,
    pub floor: i32,
    pub hp: i32,
    pub max_hp: i32,
    pub gold: i32,
    pub deck_size: usize,
    pub relic_count: usize,
    pub potion_slots: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatCaseCombatSummary {
    pub engine_state: String,
    pub turn: u32,
    pub hp: i32,
    pub max_hp: i32,
    pub block: i32,
    pub energy: u8,
    pub enemies: Vec<String>,
    pub hand: Vec<CombatCaseCardSummary>,
    pub draw_count: usize,
    pub discard_count: usize,
    pub exhaust_count: usize,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatCaseCardSummary {
    pub id: String,
    pub uuid: u32,
    #[serde(default, skip_serializing_if = "is_zero_u8")]
    pub upgrades: u8,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatCasePathStep {
    pub key: Value,
    pub label: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatCaseRngSummary {
    pub monster_rng: u32,
    pub event_rng: u32,
    pub merchant_rng: u32,
    pub card_rng: u32,
    pub treasure_rng: u32,
    pub relic_rng: u32,
    pub potion_rng: u32,
    pub monster_hp_rng: u32,
    pub ai_rng: u32,
    pub shuffle_rng: u32,
    pub card_random_rng: u32,
    pub misc_rng: u32,
    pub math_rng: u32,
}

impl CombatCase {
    pub fn new(
        source: CombatCaseSource,
        gap: CombatCaseGap,
        run: CombatCaseRunSummary,
        combat_search_attempts: Vec<CombatSearchTraceSummary>,
        failed_search: Option<CombatSearchTraceSummary>,
        path: Vec<CombatCasePathStep>,
        run_rng: CombatCaseRngSummary,
        position: CombatPosition,
    ) -> Self {
        Self {
            schema: COMBAT_CASE_SCHEMA.to_string(),
            source,
            gap,
            run,
            combat: combat_summary(&position),
            combat_search_attempts,
            failed_search,
            path,
            run_rng,
            position,
        }
    }
}

impl CombatCaseRngSummary {
    pub fn from_pool(rng: &RngPool) -> Self {
        Self {
            monster_rng: rng.monster_rng.counter,
            event_rng: rng.event_rng.counter,
            merchant_rng: rng.merchant_rng.counter,
            card_rng: rng.card_rng.counter,
            treasure_rng: rng.treasure_rng.counter,
            relic_rng: rng.relic_rng.counter,
            potion_rng: rng.potion_rng.counter,
            monster_hp_rng: rng.monster_hp_rng.counter,
            ai_rng: rng.ai_rng.counter,
            shuffle_rng: rng.shuffle_rng.counter,
            card_random_rng: rng.card_random_rng.counter,
            misc_rng: rng.misc_rng.counter,
            math_rng: rng.math_rng.counter,
        }
    }
}

pub fn load_combat_case(path: &Path) -> Result<CombatCase, String> {
    let payload = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let case: CombatCase = serde_json::from_str(&payload).map_err(|err| err.to_string())?;
    if case.schema != COMBAT_CASE_SCHEMA && case.schema != LEGACY_COMBAT_GAP_CASE_SCHEMA {
        return Err(format!("expected combat case, got {}", case.schema));
    }
    Ok(case)
}

pub fn save_combat_case(path: &Path, case: &CombatCase) -> Result<(), String> {
    let payload = serde_json::to_string_pretty(case).map_err(|err| err.to_string())?;
    fs::write(path, payload).map_err(|err| err.to_string())
}

pub fn combat_summary(position: &CombatPosition) -> CombatCaseCombatSummary {
    let combat = &position.combat;
    CombatCaseCombatSummary {
        engine_state: format!("{:?}", position.engine),
        turn: combat.turn.turn_count,
        hp: combat.entities.player.current_hp,
        max_hp: combat.entities.player.max_hp,
        block: combat.entities.player.block,
        energy: combat.turn.energy,
        enemies: living_enemy_names(combat),
        hand: combat.zones.hand.iter().map(card_summary).collect(),
        draw_count: combat.zones.draw_pile.len(),
        discard_count: combat.zones.discard_pile.len(),
        exhaust_count: combat.zones.exhaust_pile.len(),
    }
}

pub fn living_enemy_names(combat: &CombatState) -> Vec<String> {
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

pub fn card_summary(card: &CombatCard) -> CombatCaseCardSummary {
    CombatCaseCardSummary {
        id: format!("{:?}", card.id),
        uuid: card.uuid,
        upgrades: card.upgrades,
    }
}

fn is_zero_u8(value: &u8) -> bool {
    *value == 0
}
