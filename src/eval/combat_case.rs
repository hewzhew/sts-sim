use std::fs;
use std::path::Path;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ai::strategy::trajectory_comparison::TrajectorySnapshot;
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub branch_evidence: Option<CombatCaseBranchEvidence>,
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
    #[serde(default)]
    pub rescue_search_nodes: usize,
    #[serde(default)]
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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub state_before: Option<Value>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub decision_evidence: Option<Value>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct CombatCaseBranchEvidence {
    pub schema: String,
    pub policy_lane: Value,
    pub trajectory_snapshot: TrajectorySnapshot,
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
            branch_evidence: None,
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

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::*;
    use crate::ai::strategy::challenger_signature::DeckBurdenBand;
    use crate::ai::strategy::trajectory_comparison::{
        TrajectoryConstruction, TrajectoryDeployabilityEvidence, TrajectoryPressureEvidence,
        TrajectoryProgress, TrajectoryResources, TrajectorySnapshot, TrajectoryTerminal,
    };
    use crate::state::core::EngineState;

    fn sample_snapshot() -> TrajectorySnapshot {
        TrajectorySnapshot {
            lane: "challenger-1".to_string(),
            terminal: TrajectoryTerminal::CoverageLimited,
            progress: TrajectoryProgress { act: 3, floor: 48 },
            pressure: TrajectoryPressureEvidence::Unknown,
            deployability: TrajectoryDeployabilityEvidence::Unknown,
            resources: TrajectoryResources {
                hp: 47,
                max_hp: 81,
                gold: 595,
                potion_count: 2,
            },
            construction: TrajectoryConstruction {
                burden: DeckBurdenBand::Clean,
                completed_commitments: 0,
                active_commitments: 0,
                failed_commitments: 0,
            },
        }
    }

    fn sample_case() -> CombatCase {
        let run = crate::state::run::RunState::new(7, 0, false, "IRONCLAD");
        let position = CombatPosition::new(
            EngineState::CombatPlayerTurn,
            crate::test_support::blank_test_combat(),
        );
        CombatCase::new(
            CombatCaseSource {
                seed: 7,
                ascension: 0,
                generation: 4,
                branch_id: 2,
                parent_id: Some(1),
            },
            CombatCaseGap {
                boundary: "Combat".to_string(),
                reason: "no win".to_string(),
                search_nodes: 100,
                search_ms: 10,
                rescue_search_nodes: 200,
                rescue_search_ms: 20,
            },
            CombatCaseRunSummary {
                act: 3,
                floor: 48,
                hp: 47,
                max_hp: 81,
                gold: 595,
                deck_size: 14,
                relic_count: 11,
                potion_slots: 3,
            },
            Vec::new(),
            None,
            vec![CombatCasePathStep {
                key: Value::Null,
                label: "Skip card reward".to_string(),
                state_before: None,
                decision_evidence: None,
            }],
            CombatCaseRngSummary::from_pool(&run.rng_pool),
            position,
        )
    }

    fn sample_branch_evidence() -> CombatCaseBranchEvidence {
        CombatCaseBranchEvidence {
            schema: "branch_policy_combat_evidence_v0".to_string(),
            policy_lane: json!({"kind": "challenger", "policy": {"lane_id": 1}}),
            trajectory_snapshot: sample_snapshot(),
        }
    }

    #[test]
    fn legacy_case_without_branch_evidence_still_deserializes() {
        let value = serde_json::to_value(sample_case()).unwrap();
        let mut object = value.as_object().unwrap().clone();
        object.remove("branch_evidence");
        for step in object["path"].as_array_mut().unwrap() {
            step.as_object_mut().unwrap().remove("decision_evidence");
        }

        let restored: CombatCase = serde_json::from_value(Value::Object(object)).unwrap();

        assert!(restored.branch_evidence.is_none());
        assert!(restored
            .path
            .iter()
            .all(|step| step.decision_evidence.is_none()));
    }

    #[test]
    fn branch_and_decision_evidence_round_trip_without_changing_position() {
        let mut case = sample_case();
        let original_position = serde_json::to_value(&case.position).unwrap();
        case.branch_evidence = Some(sample_branch_evidence());
        case.path[0].decision_evidence = Some(json!({
            "policy_lane": "challenger-1",
            "candidate_pool": [{"rank": 1, "selected": true}],
            "annotation": {"kind": "candidate"},
            "decision_delta": {"gold_delta": -50},
            "shop_boss_preview_candidates": [{"rank": 1}],
            "shop_boss_preview_bundles": [{"rank": 1}]
        }));

        let restored: CombatCase =
            serde_json::from_value(serde_json::to_value(&case).unwrap()).unwrap();

        assert_eq!(
            serde_json::to_value(&restored.position).unwrap(),
            original_position
        );
        assert_eq!(
            restored.branch_evidence.unwrap().trajectory_snapshot.lane,
            "challenger-1"
        );
        assert_eq!(
            restored.path[0].decision_evidence.as_ref().unwrap()["candidate_pool"][0]["selected"],
            true
        );
    }
}
