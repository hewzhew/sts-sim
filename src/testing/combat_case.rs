use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::content::cards::{get_card_definition, upgraded_base_cost_override, CardId};
use crate::content::powers::store::set_powers_for;
use crate::diff::replay::tick_until_stable;
use crate::diff::state_sync::build_combat_state_from_snapshots;
use crate::protocol::java::{card_id_from_java, power_id_from_java, relic_id_from_java};
use crate::runtime::combat::{CombatCard, CombatState, Power};
use crate::runtime::rng::StsRng;
use crate::state::core::{ClientInput, EngineState, PendingChoice};
use crate::testing::fixtures::author_spec::{
    compile_combat_author_spec, AuthorCardEntry, AuthorCardSpec, AuthorRelicEntry, AuthorRelicSpec,
    CombatAuthorSpec,
};
use crate::testing::fixtures::combat_start_spec::{
    compile_combat_start_spec_with_seed, CombatStartSpec,
};
use crate::testing::fixtures::scenario::{
    build_live_observation_snapshot, build_live_truth_snapshot, extract_field_value,
    parse_expected, ActualFieldValue, ScenarioAssertion, ScenarioCardSelector, ScenarioFixture,
    ScenarioOracleKind, ScenarioStep, StructuredScenarioStep,
};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
#[serde(rename_all = "snake_case")]
pub enum CombatCaseDomain {
    #[default]
    Combat,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CombatCaseOracleKind {
    JavaSource,
    LiveRuntime,
    Differential,
    Invariant,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatCaseOracle {
    pub primary: CombatCaseOracleKind,
    #[serde(default)]
    pub evidence: Vec<CombatCaseOracleKind>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CombatCaseProvenance {
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub response_id_range: Option<(u64, u64)>,
    #[serde(default)]
    pub failure_frame: Option<u64>,
    #[serde(default)]
    pub assertion_source_frames: Vec<u64>,
    #[serde(default)]
    pub assertion_source_response_ids: Vec<u64>,
    #[serde(default)]
    pub debug_context_summary: Option<Value>,
    #[serde(default)]
    pub aspect_summary: Option<Value>,
    #[serde(default)]
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatCase {
    pub id: String,
    #[serde(default)]
    pub domain: CombatCaseDomain,
    pub basis: CombatCaseBasis,
    #[serde(default)]
    pub delta: CombatCaseDelta,
    #[serde(default)]
    pub program: Vec<CombatCaseStep>,
    pub oracle: CombatCaseOracle,
    #[serde(default)]
    pub expectations: Vec<CombatCaseExpectation>,
    #[serde(default)]
    pub provenance: CombatCaseProvenance,
    #[serde(default)]
    pub tags: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CombatCaseBasis {
    ProtocolSnapshot(CombatCaseProtocolSnapshotBasis),
    EncounterTemplate(CombatCaseEncounterTemplateBasis),
    LiveWindow(CombatCaseLiveWindowBasis),
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CombatCaseProtocolSnapshotBasis {
    pub combat_truth: Value,
    pub combat_observation: Value,
    pub relics: Value,
    #[serde(default)]
    pub protocol_meta: Option<Value>,
    #[serde(default)]
    pub root_meta: CombatCaseRootMeta,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CombatCaseRootMeta {
    #[serde(default)]
    pub player_class: Option<String>,
    #[serde(default)]
    pub ascension_level: Option<i32>,
    #[serde(default)]
    pub seed_hint: Option<u64>,
    #[serde(default)]
    pub screen_type: Option<String>,
    #[serde(default)]
    pub screen_state: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatCaseEncounterTemplateBasis {
    pub player_class: String,
    pub ascension_level: i32,
    pub encounter_id: String,
    #[serde(default = "default_room_type")]
    pub room_type: String,
    #[serde(default = "default_seed_hint")]
    pub seed_hint: u64,
    #[serde(default)]
    pub player_current_hp: Option<i32>,
    #[serde(default)]
    pub player_max_hp: Option<i32>,
    #[serde(default)]
    pub relics: Vec<AuthorRelicSpec>,
    #[serde(default)]
    pub potions: Vec<String>,
    #[serde(default)]
    pub master_deck: Vec<AuthorCardSpec>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatCaseLiveWindowBasis {
    pub raw_path: String,
    #[serde(default)]
    pub debug_path: Option<String>,
    pub from_response_id: u64,
    pub to_response_id: u64,
    #[serde(default)]
    pub failure_frame: Option<u64>,
    #[serde(default)]
    pub run_id: Option<String>,
    #[serde(default)]
    pub target_field: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CombatCaseDelta {
    #[serde(default)]
    pub player: Option<CombatCasePlayerDelta>,
    #[serde(default)]
    pub monsters: Vec<CombatCaseMonsterDelta>,
    #[serde(default)]
    pub relics: Vec<CombatCaseRelicDelta>,
    #[serde(default)]
    pub zones: Option<CombatCaseZonesDelta>,
    #[serde(default)]
    pub potions: Option<Vec<String>>,
    #[serde(default)]
    pub runtime: Option<CombatCaseRuntimeDelta>,
    #[serde(default)]
    pub rng: Option<CombatCaseRngDelta>,
    #[serde(default)]
    pub engine: Option<CombatCaseEngineDelta>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CombatCasePlayerDelta {
    #[serde(default)]
    pub current_hp: Option<i32>,
    #[serde(default)]
    pub max_hp: Option<i32>,
    #[serde(default)]
    pub block: Option<i32>,
    #[serde(default)]
    pub energy: Option<u8>,
    #[serde(default)]
    pub gold: Option<i32>,
    #[serde(default)]
    pub powers: Option<Vec<CombatCasePowerSpec>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CombatCaseMonsterDelta {
    pub monster: usize,
    #[serde(default)]
    pub current_hp: Option<i32>,
    #[serde(default)]
    pub max_hp: Option<i32>,
    #[serde(default)]
    pub block: Option<i32>,
    #[serde(default)]
    pub move_id: Option<u8>,
    #[serde(default)]
    pub move_history: Option<Vec<u8>>,
    #[serde(default)]
    pub powers: Option<Vec<CombatCasePowerSpec>>,
    #[serde(default)]
    pub runtime: Option<CombatCaseMonsterRuntimeDelta>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CombatCaseMonsterRuntimeDelta {
    #[serde(default)]
    pub values: BTreeMap<String, Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CombatCaseRelicDelta {
    pub id: String,
    #[serde(default)]
    pub counter: Option<i32>,
    #[serde(default)]
    pub used_up: Option<bool>,
    #[serde(default)]
    pub amount: Option<i32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CombatCaseZonesDelta {
    #[serde(default)]
    pub hand: Option<Vec<CombatCaseCardEntry>>,
    #[serde(default)]
    pub draw_pile: Option<Vec<CombatCaseCardEntry>>,
    #[serde(default)]
    pub discard_pile: Option<Vec<CombatCaseCardEntry>>,
    #[serde(default)]
    pub exhaust_pile: Option<Vec<CombatCaseCardEntry>>,
    #[serde(default)]
    pub limbo: Option<Vec<CombatCaseCardEntry>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CombatCaseRuntimeDelta {
    #[serde(default)]
    pub colorless_combat_pool: Option<Vec<String>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CombatCaseRngDelta {
    #[serde(default)]
    pub ai_rng: Option<CombatCaseRngChannel>,
    #[serde(default)]
    pub shuffle_rng: Option<CombatCaseRngChannel>,
    #[serde(default)]
    pub card_rng: Option<CombatCaseRngChannel>,
    #[serde(default)]
    pub misc_rng: Option<CombatCaseRngChannel>,
    #[serde(default)]
    pub monster_hp_rng: Option<CombatCaseRngChannel>,
    #[serde(default)]
    pub potion_rng: Option<CombatCaseRngChannel>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CombatCaseRngChannel {
    #[serde(default)]
    pub seed0: Option<i64>,
    #[serde(default)]
    pub seed1: Option<i64>,
    #[serde(default)]
    pub counter: Option<u32>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct CombatCaseEngineDelta {
    #[serde(default)]
    pub response_id: Option<u64>,
    #[serde(default)]
    pub frame_id: Option<u64>,
    #[serde(default)]
    pub turn_count: Option<u32>,
    #[serde(default)]
    pub screen_type: Option<String>,
    #[serde(default)]
    pub screen_state: Option<Value>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatCasePowerSpec {
    pub id: String,
    pub amount: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatCaseCardEntry {
    pub id: String,
    #[serde(default)]
    pub uuid: Option<String>,
    #[serde(default)]
    pub upgrades: u8,
    #[serde(default)]
    pub cost: Option<i32>,
    #[serde(default)]
    pub misc: Option<i32>,
    #[serde(default = "default_count")]
    pub count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatCaseStep {
    pub step: CombatCaseProgramStep,
    #[serde(default)]
    pub label: Option<String>,
    #[serde(default)]
    pub response_id: Option<u64>,
    #[serde(default)]
    pub frame_id: Option<u64>,
    #[serde(default)]
    pub command_kind: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CombatCaseProgramStep {
    Play {
        selector: CombatCaseCardSelector,
        #[serde(default)]
        target: Option<usize>,
    },
    End,
    Cancel,
    Choose {
        index: usize,
    },
    PotionUse {
        slot: usize,
        #[serde(default)]
        target: Option<usize>,
    },
    HandSelect {
        selectors: Vec<CombatCaseCardSelector>,
    },
    GridSelect {
        selectors: Vec<CombatCaseCardSelector>,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CombatCaseCardSelector {
    Index {
        index: usize,
    },
    JavaId {
        id: String,
        #[serde(default = "default_occurrence")]
        occurrence: usize,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CombatCaseExpectation {
    pub check: CombatCaseCheck,
    #[serde(default)]
    pub response_id: Option<u64>,
    #[serde(default)]
    pub frame_id: Option<u64>,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CombatCaseCheck {
    Path {
        field: String,
        expected: CombatCaseScalarValue,
    },
    PlayerStat {
        stat: String,
        value: i64,
    },
    PlayerPower {
        id: String,
        amount: i64,
    },
    MonsterStat {
        monster: usize,
        stat: String,
        value: i64,
    },
    MonsterPower {
        monster: usize,
        id: String,
        amount: i64,
    },
    MonsterRuntime {
        monster: usize,
        field: String,
        expected: CombatCaseScalarValue,
    },
    PileContains {
        pile: String,
        id: String,
        present: bool,
    },
    PileCount {
        pile: String,
        id: String,
        count: i64,
    },
    PileSize {
        pile: String,
        count: i64,
    },
    RelicPresent {
        id: String,
        present: bool,
    },
    RelicCount {
        id: String,
        count: i64,
    },
    RelicRuntime {
        id: String,
        field: String,
        expected: CombatCaseScalarValue,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum CombatCaseScalarValue {
    Missing,
    Number { value: i64 },
    String { value: String },
    Bool { value: bool },
}

#[derive(Debug, Clone)]
pub struct CombatCaseRuntimeSeed {
    pub combat: CombatState,
    pub engine_state: EngineState,
    pub response_id: Option<u64>,
    pub frame_id: Option<u64>,
    pub seed_hint: Option<u64>,
    pub ascension_level: Option<u8>,
    pub player_class: Option<String>,
}

#[derive(Debug, Clone)]
pub struct CombatCaseReplaySnapshot {
    pub response_id: Option<u64>,
    pub frame_id: Option<u64>,
    pub combat: CombatState,
}

#[derive(Debug)]
pub struct CombatCaseReplay {
    pub combat: CombatState,
    pub engine_state: EngineState,
    pub snapshots: Vec<CombatCaseReplaySnapshot>,
}

pub struct CombatCaseReducer;

impl CombatCaseReducer {
    pub fn materialize(case: &CombatCase) -> Result<CombatCase, String> {
        match &case.basis {
            CombatCaseBasis::LiveWindow(live) => {
                let protocol = protocol_basis_from_live_window(live)?;
                let mut materialized = case.clone();
                materialized.basis = CombatCaseBasis::ProtocolSnapshot(protocol);
                materialized
                    .provenance
                    .notes
                    .push("materialized_from_live_window".to_string());
                Ok(materialized)
            }
            _ => Ok(case.clone()),
        }
    }

    pub fn reduce(case: &CombatCase) -> Result<CombatCase, String> {
        let materialized = Self::materialize(case)?;
        let reduced = match &materialized.basis {
            CombatCaseBasis::ProtocolSnapshot(protocol) => {
                if let Some(reduced) = try_reduce_protocol_snapshot_case(&materialized, protocol)? {
                    reduced
                } else {
                    materialized
                }
            }
            _ => materialized,
        };

        minimize_encounter_template_case(&reduced)
    }
}

pub fn lower_case(case: &CombatCase) -> Result<CombatCaseRuntimeSeed, String> {
    let materialized = CombatCaseReducer::materialize(case)?;
    let mut seed = match &materialized.basis {
        CombatCaseBasis::ProtocolSnapshot(protocol) => lower_protocol_snapshot_basis(protocol)?,
        CombatCaseBasis::EncounterTemplate(template) => lower_encounter_template_basis(template)?,
        CombatCaseBasis::LiveWindow(_) => {
            return Err(
                "live_window basis should have been materialized before lowering".to_string(),
            )
        }
    };

    apply_case_delta(&mut seed, &materialized.delta)?;
    Ok(seed)
}

pub fn replay_case(case: &CombatCase) -> Result<CombatCaseReplay, String> {
    let seed = lower_case(case)?;
    let mut combat = seed.combat;
    let mut engine_state = seed.engine_state;
    let mut snapshots = vec![CombatCaseReplaySnapshot {
        response_id: seed.response_id,
        frame_id: seed.frame_id,
        combat: combat.clone(),
    }];

    for step in &case.program {
        let input = input_for_case_step(step, &engine_state, &combat).ok_or_else(|| {
            format!(
                "combat case '{}' contains unsupported or invalid step {:?}",
                case.id, step.step
            )
        })?;
        let alive = tick_until_stable(&mut engine_state, &mut combat, input);
        if !alive {
            return Err(format!(
                "combat case '{}' died while executing {:?}",
                case.id, step.step
            ));
        }
        snapshots.push(CombatCaseReplaySnapshot {
            response_id: step.response_id,
            frame_id: step.frame_id,
            combat: combat.clone(),
        });
    }

    Ok(CombatCaseReplay {
        combat,
        engine_state,
        snapshots,
    })
}

pub fn assert_case(case: &CombatCase) -> Result<(), String> {
    let replay = replay_case(case)?;
    for expectation in &case.expectations {
        let combat = combat_for_expectation(&replay, expectation).map_err(|err| {
            format!(
                "combat case '{}' could not resolve expectation scope: {err}",
                case.id
            )
        })?;
        let actual = extract_expectation_value(combat, &expectation.check)?;
        let expected = expected_value_for_check(&expectation.check)?;
        if actual != expected {
            return Err(format!(
                "combat case '{}' mismatch on {:?}{}: actual={actual:?} expected={expected:?}",
                case.id,
                expectation.check,
                format_expectation_scope(expectation),
            ));
        }
    }
    Ok(())
}

pub fn case_from_scenario_fixture(fixture: &ScenarioFixture) -> Result<CombatCase, String> {
    let protocol_meta = fixture.initial_protocol_meta.clone();
    let initial_game_state = &fixture.initial_game_state;
    let truth_snapshot = build_live_truth_snapshot(initial_game_state);
    let observation_snapshot = build_live_observation_snapshot(initial_game_state);
    let relics = initial_game_state
        .get("relics")
        .cloned()
        .unwrap_or(Value::Null);
    let root_meta = CombatCaseRootMeta {
        player_class: initial_game_state
            .get("class")
            .and_then(|value| value.as_str())
            .map(ToString::to_string),
        ascension_level: initial_game_state
            .get("ascension_level")
            .and_then(|value| value.as_i64())
            .map(|value| value as i32),
        seed_hint: initial_game_state
            .get("seed")
            .and_then(|value| value.as_u64().or_else(|| value.as_i64().map(|v| v as u64))),
        screen_type: initial_game_state
            .get("screen_type")
            .and_then(|value| value.as_str())
            .map(ToString::to_string),
        screen_state: initial_game_state.get("screen_state").cloned(),
    };

    Ok(CombatCase {
        id: fixture.name.clone(),
        domain: CombatCaseDomain::Combat,
        basis: CombatCaseBasis::ProtocolSnapshot(CombatCaseProtocolSnapshotBasis {
            combat_truth: truth_snapshot,
            combat_observation: observation_snapshot,
            relics,
            protocol_meta,
            root_meta,
        }),
        delta: CombatCaseDelta::default(),
        program: fixture
            .steps
            .iter()
            .map(case_step_from_scenario_step)
            .collect::<Result<Vec<_>, _>>()?,
        oracle: oracle_from_scenario_kind(fixture.oracle_kind.clone()),
        expectations: fixture
            .assertions
            .iter()
            .map(expectation_from_scenario_assertion)
            .collect::<Result<Vec<_>, _>>()?,
        provenance: provenance_from_scenario(fixture.provenance.clone()),
        tags: fixture.tags.clone(),
    })
}

pub fn compile_combat_author_case(spec: &CombatAuthorSpec) -> Result<CombatCase, String> {
    let fixture = compile_combat_author_spec(spec)?;
    case_from_scenario_fixture(&fixture)
}

pub fn load_case_from_path(path: &Path) -> Result<CombatCase, String> {
    let payload = std::fs::read_to_string(path).map_err(|err| err.to_string())?;
    serde_json::from_str(&payload).map_err(|err| err.to_string())
}

pub fn write_case_to_path(case: &CombatCase, path: &Path) -> Result<(), String> {
    let payload = serde_json::to_string_pretty(case).map_err(|err| err.to_string())?;
    std::fs::write(path, payload).map_err(|err| err.to_string())
}

fn lower_protocol_snapshot_basis(
    basis: &CombatCaseProtocolSnapshotBasis,
) -> Result<CombatCaseRuntimeSeed, String> {
    let mut combat = build_combat_state_from_snapshots(
        &basis.combat_truth,
        &basis.combat_observation,
        &basis.relics,
    );
    let engine_state = build_engine_state_from_root_meta(
        &basis.root_meta,
        basis.protocol_meta.as_ref(),
        &mut combat,
    );
    let player_class = basis
        .root_meta
        .player_class
        .clone()
        .or_else(|| Some(combat.meta.player_class.to_string()));
    let ascension_level = basis
        .root_meta
        .ascension_level
        .and_then(|value| u8::try_from(value).ok())
        .or(Some(combat.meta.ascension_level));
    Ok(CombatCaseRuntimeSeed {
        combat,
        engine_state,
        response_id: basis
            .protocol_meta
            .as_ref()
            .and_then(|meta| meta.get("response_id"))
            .and_then(json_u64),
        frame_id: basis
            .protocol_meta
            .as_ref()
            .and_then(|meta| meta.get("state_frame_id").or_else(|| meta.get("frame_id")))
            .and_then(json_u64),
        seed_hint: basis.root_meta.seed_hint,
        ascension_level,
        player_class,
    })
}

fn lower_encounter_template_basis(
    basis: &CombatCaseEncounterTemplateBasis,
) -> Result<CombatCaseRuntimeSeed, String> {
    let player_max_hp = basis
        .player_max_hp
        .unwrap_or_else(|| base_max_hp_for_class(&basis.player_class));
    let player_current_hp = basis.player_current_hp.unwrap_or(player_max_hp);
    let start_spec = CombatStartSpec {
        name: basis.encounter_id.clone(),
        player_class: basis.player_class.clone(),
        ascension_level: basis.ascension_level,
        encounter_id: basis.encounter_id.clone(),
        room_type: basis.room_type.clone(),
        seed: basis.seed_hint,
        player_current_hp,
        player_max_hp,
        relics: basis.relics.clone(),
        potions: basis.potions.clone(),
        master_deck: basis.master_deck.clone(),
    };
    let (engine_state, combat) = compile_combat_start_spec_with_seed(&start_spec, basis.seed_hint)?;
    Ok(CombatCaseRuntimeSeed {
        combat,
        engine_state,
        response_id: None,
        frame_id: None,
        seed_hint: Some(basis.seed_hint),
        ascension_level: u8::try_from(basis.ascension_level).ok(),
        player_class: Some(basis.player_class.clone()),
    })
}

fn protocol_basis_from_live_window(
    basis: &CombatCaseLiveWindowBasis,
) -> Result<CombatCaseProtocolSnapshotBasis, String> {
    let raw_path = PathBuf::from(&basis.raw_path);
    let records = load_raw_records(&raw_path)?;
    let start = records.get(&basis.from_response_id).ok_or_else(|| {
        format!(
            "missing response_id {} in {}",
            basis.from_response_id,
            raw_path.display()
        )
    })?;
    let game_state = start
        .get("game_state")
        .ok_or_else(|| format!("response_id {} missing game_state", basis.from_response_id))?;
    Ok(CombatCaseProtocolSnapshotBasis {
        combat_truth: build_live_truth_snapshot(game_state),
        combat_observation: build_live_observation_snapshot(game_state),
        relics: game_state.get("relics").cloned().unwrap_or(Value::Null),
        protocol_meta: start.get("protocol_meta").cloned(),
        root_meta: CombatCaseRootMeta {
            player_class: game_state
                .get("class")
                .and_then(|value| value.as_str())
                .map(ToString::to_string),
            ascension_level: game_state
                .get("ascension_level")
                .and_then(|value| value.as_i64())
                .map(|value| value as i32),
            seed_hint: game_state
                .get("seed")
                .and_then(|value| value.as_u64().or_else(|| value.as_i64().map(|v| v as u64))),
            screen_type: game_state
                .get("screen_type")
                .and_then(|value| value.as_str())
                .map(ToString::to_string),
            screen_state: game_state.get("screen_state").cloned(),
        },
    })
}

fn try_reduce_protocol_snapshot_case(
    case: &CombatCase,
    basis: &CombatCaseProtocolSnapshotBasis,
) -> Result<Option<CombatCase>, String> {
    let template_basis = match infer_encounter_template_basis(basis)? {
        Some(template_basis) => template_basis,
        None => return Ok(None),
    };
    let delta = build_encounter_template_delta(basis)?;
    let mut reduced = case.clone();
    reduced.basis = CombatCaseBasis::EncounterTemplate(template_basis);
    reduced.delta = delta;
    reduced
        .provenance
        .notes
        .push("reduced_protocol_snapshot_to_encounter_template".to_string());
    match assert_case(&reduced) {
        Ok(()) => Ok(Some(reduced)),
        Err(_err) => Ok(None),
    }
}

fn minimize_encounter_template_case(case: &CombatCase) -> Result<CombatCase, String> {
    if !matches!(case.basis, CombatCaseBasis::EncounterTemplate(_)) {
        return Ok(case.clone());
    }

    assert_case(case)?;
    let mut current = case.clone();
    let mut changed = false;
    loop {
        let mut progress = false;
        progress |= minimize_encounter_template_relic_delta(&mut current)?;
        progress |= minimize_encounter_template_runtime_delta(&mut current)?;
        progress |= minimize_encounter_template_rng_delta(&mut current)?;
        progress |= minimize_encounter_template_zone_delta(&mut current)?;
        if !progress {
            break;
        }
        changed = true;
    }

    if changed
        && !current
            .provenance
            .notes
            .iter()
            .any(|note| note == "minimized_encounter_template_delta")
    {
        current
            .provenance
            .notes
            .push("minimized_encounter_template_delta".to_string());
    }

    Ok(current)
}

fn try_accept_case_mutation<F>(case: &mut CombatCase, mutate: F) -> Result<bool, String>
where
    F: FnOnce(&mut CombatCase) -> bool,
{
    let mut candidate = case.clone();
    if !mutate(&mut candidate) {
        return Ok(false);
    }
    if assert_case(&candidate).is_ok() {
        *case = candidate;
        return Ok(true);
    }
    Ok(false)
}

fn minimize_encounter_template_relic_delta(case: &mut CombatCase) -> Result<bool, String> {
    let mut progress = false;
    let mut index = 0usize;
    while index < case.delta.relics.len() {
        if try_accept_case_mutation(case, |candidate| {
            let Some(relic) = candidate.delta.relics.get_mut(index) else {
                return false;
            };
            if relic.counter.take().is_some() {
                return true;
            }
            false
        })? {
            progress = true;
            continue;
        }
        if try_accept_case_mutation(case, |candidate| {
            let Some(relic) = candidate.delta.relics.get_mut(index) else {
                return false;
            };
            if relic.used_up.take().is_some() {
                return true;
            }
            false
        })? {
            progress = true;
            continue;
        }
        if try_accept_case_mutation(case, |candidate| {
            let Some(relic) = candidate.delta.relics.get_mut(index) else {
                return false;
            };
            if relic.amount.take().is_some() {
                return true;
            }
            false
        })? {
            progress = true;
            continue;
        }
        if try_accept_case_mutation(case, |candidate| {
            if index >= candidate.delta.relics.len() {
                return false;
            }
            candidate.delta.relics.remove(index);
            true
        })? {
            progress = true;
            continue;
        }
        index += 1;
    }
    Ok(progress)
}

fn minimize_encounter_template_runtime_delta(case: &mut CombatCase) -> Result<bool, String> {
    let mut progress = false;
    if try_accept_case_mutation(case, |candidate| {
        if candidate.delta.runtime.is_none() {
            return false;
        }
        candidate.delta.runtime = None;
        true
    })? {
        return Ok(true);
    }

    loop {
        let Some(runtime) = case.delta.runtime.as_ref() else {
            break;
        };
        let Some(pool) = runtime.colorless_combat_pool.as_ref() else {
            break;
        };
        if pool.is_empty() {
            if try_accept_case_mutation(case, |candidate| {
                let Some(runtime) = candidate.delta.runtime.as_mut() else {
                    return false;
                };
                runtime.colorless_combat_pool = None;
                if runtime.colorless_combat_pool.is_none() {
                    candidate.delta.runtime = None;
                }
                true
            })? {
                progress = true;
                continue;
            }
            break;
        }

        let mut index = 0usize;
        let mut removed = false;
        while index
            < case
                .delta
                .runtime
                .as_ref()
                .and_then(|runtime| runtime.colorless_combat_pool.as_ref().map(Vec::len))
                .unwrap_or(0)
        {
            if try_accept_case_mutation(case, |candidate| {
                let Some(runtime) = candidate.delta.runtime.as_mut() else {
                    return false;
                };
                let Some(pool) = runtime.colorless_combat_pool.as_mut() else {
                    return false;
                };
                if index >= pool.len() {
                    return false;
                }
                pool.remove(index);
                if pool.is_empty() {
                    runtime.colorless_combat_pool = None;
                    if runtime.colorless_combat_pool.is_none() {
                        candidate.delta.runtime = None;
                    }
                }
                true
            })? {
                progress = true;
                removed = true;
                break;
            }
            index += 1;
        }
        if !removed {
            break;
        }
    }

    Ok(progress)
}

fn minimize_encounter_template_rng_delta(case: &mut CombatCase) -> Result<bool, String> {
    let mut progress = false;
    if try_accept_case_mutation(case, |candidate| {
        if candidate.delta.rng.is_none() {
            return false;
        }
        candidate.delta.rng = None;
        true
    })? {
        return Ok(true);
    }

    for channel in [
        "ai_rng",
        "shuffle_rng",
        "card_rng",
        "misc_rng",
        "monster_hp_rng",
        "potion_rng",
    ] {
        if try_accept_case_mutation(case, |candidate| {
            let Some(rng) = candidate.delta.rng.as_mut() else {
                return false;
            };
            let Some(slot) = rng_channel_slot_mut(rng, channel) else {
                return false;
            };
            if slot.is_none() {
                return false;
            }
            *slot = None;
            if rng_delta_is_empty(rng) {
                candidate.delta.rng = None;
            }
            true
        })? {
            progress = true;
            continue;
        }

        for field in ["counter", "seed0", "seed1"] {
            loop {
                let changed = try_accept_case_mutation(case, |candidate| {
                    let Some(rng) = candidate.delta.rng.as_mut() else {
                        return false;
                    };
                    let Some(channel_state) =
                        rng_channel_slot_mut(rng, channel).and_then(Option::as_mut)
                    else {
                        return false;
                    };
                    let removed = match field {
                        "counter" => channel_state.counter.take().is_some(),
                        "seed0" => channel_state.seed0.take().is_some(),
                        "seed1" => channel_state.seed1.take().is_some(),
                        _ => false,
                    };
                    if !removed {
                        return false;
                    }
                    if rng_channel_is_empty(channel_state) {
                        let slot = rng_channel_slot_mut(rng, channel)
                            .expect("channel slot should exist after mutation");
                        *slot = None;
                    }
                    if rng_delta_is_empty(rng) {
                        candidate.delta.rng = None;
                    }
                    true
                })?;
                if !changed {
                    break;
                }
                progress = true;
            }
        }
    }

    Ok(progress)
}

fn minimize_encounter_template_zone_delta(case: &mut CombatCase) -> Result<bool, String> {
    let mut progress = false;
    if try_accept_case_mutation(case, |candidate| {
        if candidate.delta.zones.is_none() {
            return false;
        }
        candidate.delta.zones = None;
        true
    })? {
        return Ok(true);
    }

    for pile in ["hand", "draw_pile", "discard_pile", "exhaust_pile", "limbo"] {
        if try_accept_case_mutation(case, |candidate| {
            let Some(zones) = candidate.delta.zones.as_mut() else {
                return false;
            };
            let Some(slot) = zone_pile_slot_mut(zones, pile) else {
                return false;
            };
            if slot.is_none() {
                return false;
            }
            *slot = None;
            if zones_delta_is_empty(zones) {
                candidate.delta.zones = None;
            }
            true
        })? {
            progress = true;
            continue;
        }

        let mut index = 0usize;
        while index
            < case
                .delta
                .zones
                .as_ref()
                .and_then(|zones| zone_pile_len(zones, pile))
                .unwrap_or(0)
        {
            if try_accept_case_mutation(case, |candidate| {
                let Some(zones) = candidate.delta.zones.as_mut() else {
                    return false;
                };
                let Some(cards) = zone_pile_slot_mut(zones, pile).and_then(Option::as_mut) else {
                    return false;
                };
                if index >= cards.len() {
                    return false;
                }
                cards.remove(index);
                if cards.is_empty() {
                    let slot = zone_pile_slot_mut(zones, pile)
                        .expect("zone slot should exist after card removal");
                    *slot = None;
                }
                if zones_delta_is_empty(zones) {
                    candidate.delta.zones = None;
                }
                true
            })? {
                progress = true;
                continue;
            }

            if try_accept_case_mutation(case, |candidate| {
                let Some(zones) = candidate.delta.zones.as_mut() else {
                    return false;
                };
                let Some(card) = zone_pile_slot_mut(zones, pile)
                    .and_then(Option::as_mut)
                    .and_then(|cards| cards.get_mut(index))
                else {
                    return false;
                };
                if card.uuid.take().is_some() {
                    return true;
                }
                false
            })? {
                progress = true;
                continue;
            }

            if try_accept_case_mutation(case, |candidate| {
                let Some(zones) = candidate.delta.zones.as_mut() else {
                    return false;
                };
                let Some(card) = zone_pile_slot_mut(zones, pile)
                    .and_then(Option::as_mut)
                    .and_then(|cards| cards.get_mut(index))
                else {
                    return false;
                };
                if card.misc.take().is_some() {
                    return true;
                }
                false
            })? {
                progress = true;
                continue;
            }

            index += 1;
        }
    }

    Ok(progress)
}

fn rng_channel_slot_mut<'a>(
    rng: &'a mut CombatCaseRngDelta,
    channel: &str,
) -> Option<&'a mut Option<CombatCaseRngChannel>> {
    match channel {
        "ai_rng" => Some(&mut rng.ai_rng),
        "shuffle_rng" => Some(&mut rng.shuffle_rng),
        "card_rng" => Some(&mut rng.card_rng),
        "misc_rng" => Some(&mut rng.misc_rng),
        "monster_hp_rng" => Some(&mut rng.monster_hp_rng),
        "potion_rng" => Some(&mut rng.potion_rng),
        _ => None,
    }
}

fn rng_channel_is_empty(channel: &CombatCaseRngChannel) -> bool {
    channel.seed0.is_none() && channel.seed1.is_none() && channel.counter.is_none()
}

fn rng_delta_is_empty(rng: &CombatCaseRngDelta) -> bool {
    rng.ai_rng.is_none()
        && rng.shuffle_rng.is_none()
        && rng.card_rng.is_none()
        && rng.misc_rng.is_none()
        && rng.monster_hp_rng.is_none()
        && rng.potion_rng.is_none()
}

fn zone_pile_slot_mut<'a>(
    zones: &'a mut CombatCaseZonesDelta,
    pile: &str,
) -> Option<&'a mut Option<Vec<CombatCaseCardEntry>>> {
    match pile {
        "hand" => Some(&mut zones.hand),
        "draw_pile" => Some(&mut zones.draw_pile),
        "discard_pile" => Some(&mut zones.discard_pile),
        "exhaust_pile" => Some(&mut zones.exhaust_pile),
        "limbo" => Some(&mut zones.limbo),
        _ => None,
    }
}

fn zone_pile_len(zones: &CombatCaseZonesDelta, pile: &str) -> Option<usize> {
    match pile {
        "hand" => zones.hand.as_ref().map(Vec::len),
        "draw_pile" => zones.draw_pile.as_ref().map(Vec::len),
        "discard_pile" => zones.discard_pile.as_ref().map(Vec::len),
        "exhaust_pile" => zones.exhaust_pile.as_ref().map(Vec::len),
        "limbo" => zones.limbo.as_ref().map(Vec::len),
        _ => None,
    }
}

fn zones_delta_is_empty(zones: &CombatCaseZonesDelta) -> bool {
    zones.hand.is_none()
        && zones.draw_pile.is_none()
        && zones.discard_pile.is_none()
        && zones.exhaust_pile.is_none()
        && zones.limbo.is_none()
}

fn infer_encounter_template_basis(
    basis: &CombatCaseProtocolSnapshotBasis,
) -> Result<Option<CombatCaseEncounterTemplateBasis>, String> {
    let truth = basis
        .combat_truth
        .as_object()
        .ok_or_else(|| "combat_truth basis is not an object".to_string())?;
    if truth
        .get("using_card")
        .and_then(Value::as_bool)
        .unwrap_or(false)
    {
        return Ok(None);
    }
    if truth
        .get("card_queue")
        .and_then(Value::as_array)
        .is_some_and(|queue| !queue.is_empty())
    {
        return Ok(None);
    }
    let player_class = match basis.root_meta.player_class.clone() {
        Some(player_class) => player_class,
        None => return Ok(None),
    };
    let ascension_level = basis.root_meta.ascension_level.unwrap_or(0);
    let encounter_id = match infer_encounter_id_from_snapshot_truth(&basis.combat_truth) {
        Some(encounter_id) => encounter_id,
        None => return Ok(None),
    };
    let room_type = truth
        .get("room_type")
        .and_then(Value::as_str)
        .unwrap_or("MonsterRoom")
        .to_string();
    let player = truth
        .get("player")
        .and_then(Value::as_object)
        .ok_or_else(|| "combat_truth.player missing or not an object".to_string())?;
    let relics = relic_specs_from_snapshot_value(protocol_relics_snapshot_value(basis))?;
    let potions =
        occupied_potion_ids_from_snapshot_value(truth.get("potions").unwrap_or(&Value::Null));
    let master_deck = collect_master_deck_specs_from_truth(&basis.combat_truth)?;
    if master_deck.is_empty() {
        return Ok(None);
    }
    Ok(Some(CombatCaseEncounterTemplateBasis {
        player_class,
        ascension_level,
        encounter_id: encounter_id.to_string(),
        room_type,
        seed_hint: basis.root_meta.seed_hint.unwrap_or_else(default_seed_hint),
        player_current_hp: Some(
            player
                .get("current_hp")
                .and_then(Value::as_i64)
                .unwrap_or(player.get("max_hp").and_then(Value::as_i64).unwrap_or(0))
                as i32,
        ),
        player_max_hp: Some(player.get("max_hp").and_then(Value::as_i64).unwrap_or(0) as i32),
        relics,
        potions,
        master_deck,
    }))
}

fn build_encounter_template_delta(
    basis: &CombatCaseProtocolSnapshotBasis,
) -> Result<CombatCaseDelta, String> {
    let truth = basis
        .combat_truth
        .as_object()
        .ok_or_else(|| "combat_truth basis is not an object".to_string())?;
    let player = truth
        .get("player")
        .and_then(Value::as_object)
        .ok_or_else(|| "combat_truth.player missing or not an object".to_string())?;
    let player_delta = CombatCasePlayerDelta {
        current_hp: None,
        max_hp: None,
        block: Some(player.get("block").and_then(Value::as_i64).unwrap_or(0) as i32),
        energy: Some(player.get("energy").and_then(Value::as_u64).unwrap_or(3) as u8),
        gold: player
            .get("gold")
            .and_then(Value::as_i64)
            .map(|value| value as i32),
        powers: Some(powers_from_snapshot_value(player.get("powers"))),
    };
    let monsters = truth
        .get("monsters")
        .and_then(Value::as_array)
        .ok_or_else(|| "combat_truth.monsters missing or not an array".to_string())?
        .iter()
        .enumerate()
        .map(|(index, monster)| monster_delta_from_snapshot(index, monster))
        .collect::<Result<Vec<_>, _>>()?;
    let zones = CombatCaseZonesDelta {
        hand: Some(zone_cards_from_snapshot_value(
            truth.get("hand").unwrap_or(&Value::Null),
        )?),
        draw_pile: Some(zone_cards_from_snapshot_value(
            truth.get("draw_pile").unwrap_or(&Value::Null),
        )?),
        discard_pile: Some(zone_cards_from_snapshot_value(
            truth.get("discard_pile").unwrap_or(&Value::Null),
        )?),
        exhaust_pile: Some(zone_cards_from_snapshot_value(
            truth.get("exhaust_pile").unwrap_or(&Value::Null),
        )?),
        limbo: Some(zone_cards_from_snapshot_value(
            truth.get("limbo").unwrap_or(&Value::Null),
        )?),
    };
    let engine = CombatCaseEngineDelta {
        response_id: basis
            .protocol_meta
            .as_ref()
            .and_then(|meta| meta.get("response_id"))
            .and_then(json_u64),
        frame_id: basis
            .protocol_meta
            .as_ref()
            .and_then(|meta| meta.get("state_frame_id").or_else(|| meta.get("frame_id")))
            .and_then(json_u64),
        turn_count: truth
            .get("turn")
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok()),
        screen_type: basis.root_meta.screen_type.clone(),
        screen_state: basis.root_meta.screen_state.clone(),
    };
    let runtime = combat_runtime_delta_from_snapshot(&basis.combat_truth)?;
    let rng = rng_delta_from_snapshot(&basis.combat_truth)?;
    Ok(CombatCaseDelta {
        player: Some(player_delta),
        monsters,
        relics: relic_deltas_from_snapshot_value(protocol_relics_snapshot_value(basis))?,
        zones: Some(zones),
        potions: Some(potion_ids_from_snapshot_value(
            truth.get("potions").unwrap_or(&Value::Null),
        )),
        runtime,
        rng,
        engine: Some(engine),
    })
}

fn infer_encounter_id_from_snapshot_truth(truth_snapshot: &Value) -> Option<&'static str> {
    let monsters = truth_snapshot.get("monsters")?.as_array()?;
    let mut roster = monsters
        .iter()
        .filter_map(|monster| monster.get("id").and_then(Value::as_str))
        .map(normalize_identifier)
        .collect::<Vec<_>>();
    roster.sort();
    let key = roster.join("+");
    match key.as_str() {
        "awakenedone+cultist+cultist" => Some("awakened_one"),
        "blueslaver" | "slaverblue" => Some("blue_slaver"),
        "bookofstabbing" => Some("book_of_stabbing"),
        "bronzeautomaton" => Some("bronze_automaton"),
        "champ" => Some("champ"),
        "corruptheart" => Some("heart"),
        "deca+donu" => Some("donu_and_deca"),
        "gremlinnob" => Some("gremlin_nob"),
        "hexaghost" => Some("hexaghost"),
        "jawworm" => Some("jaw_worm"),
        "lagavulin" => Some("lagavulin"),
        "sentry+sentry+sentry" => Some("three_sentries"),
        "slimeboss" => Some("slime_boss"),
        "spireshield+spirespear" => Some("shield_and_spear"),
        "thecollector" => Some("collector"),
        "theguardian" => Some("guardian"),
        "timeeater" => Some("time_eater"),
        _ => None,
    }
}

fn collect_master_deck_specs_from_truth(
    truth_snapshot: &Value,
) -> Result<Vec<AuthorCardSpec>, String> {
    let truth = truth_snapshot
        .as_object()
        .ok_or_else(|| "combat_truth basis is not an object".to_string())?;
    let mut grouped = BTreeMap::<String, AuthorCardEntry>::new();
    for zone_key in ["hand", "draw_pile", "discard_pile", "exhaust_pile", "limbo"] {
        let Some(cards) = truth.get(zone_key).and_then(Value::as_array) else {
            continue;
        };
        for card in cards {
            let entry = author_card_entry_from_snapshot_card(card)?;
            let key = format!(
                "{}|{}|{}|{}",
                entry.id,
                entry.upgrades,
                entry.misc.unwrap_or_default(),
                entry.cost.unwrap_or_default()
            );
            grouped
                .entry(key)
                .and_modify(|existing| existing.count += 1)
                .or_insert(entry);
        }
    }
    Ok(grouped
        .into_values()
        .map(AuthorCardSpec::Detailed)
        .collect::<Vec<_>>())
}

fn author_card_entry_from_snapshot_card(card: &Value) -> Result<AuthorCardEntry, String> {
    let java_id = card
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| "snapshot card missing id".to_string())?;
    let card_id =
        card_id_from_java(java_id).ok_or_else(|| format!("unknown Java card id '{java_id}'"))?;
    let upgrades = card.get("upgrades").and_then(Value::as_u64).unwrap_or(0) as u8;
    if let Some(explicit_cost) = card
        .get("cost")
        .and_then(Value::as_i64)
        .map(|value| value as i32)
    {
        let mut runtime_card = CombatCard::new(card_id, 0);
        runtime_card.upgrades = upgrades;
        let base_cost = upgraded_base_cost_override(&runtime_card)
            .unwrap_or_else(|| get_card_definition(card_id).cost) as i32;
        if explicit_cost != base_cost {
            return Err(format!(
                "snapshot card '{}' has unsupported modified cost {} (base {})",
                java_id, explicit_cost, base_cost
            ));
        }
    }
    Ok(AuthorCardEntry {
        id: java_id.to_string(),
        upgrades,
        cost: None,
        misc: card
            .get("misc")
            .and_then(Value::as_i64)
            .map(|value| value as i32),
        count: 1,
    })
}

fn relic_specs_from_snapshot_value(value: &Value) -> Result<Vec<AuthorRelicSpec>, String> {
    let Some(relics) = value.as_array() else {
        return Ok(Vec::new());
    };
    let mut parsed = Vec::new();
    for relic in relics {
        let Some(java_id) = relic.get("id").and_then(Value::as_str) else {
            continue;
        };
        let counter = relic
            .get("counter")
            .and_then(Value::as_i64)
            .map(|value| value as i32)
            .unwrap_or(-1);
        if counter == -1 {
            parsed.push(AuthorRelicSpec::Simple(java_id.to_string()));
        } else {
            parsed.push(AuthorRelicSpec::Detailed(AuthorRelicEntry {
                id: java_id.to_string(),
                counter,
            }));
        }
    }
    Ok(parsed)
}

fn protocol_relics_snapshot_value<'a>(basis: &'a CombatCaseProtocolSnapshotBasis) -> &'a Value {
    if basis
        .relics
        .as_array()
        .is_some_and(|relics| !relics.is_empty())
    {
        &basis.relics
    } else {
        basis.combat_truth.get("relics").unwrap_or(&basis.relics)
    }
}

fn relic_deltas_from_snapshot_value(value: &Value) -> Result<Vec<CombatCaseRelicDelta>, String> {
    let Some(relics) = value.as_array() else {
        return Ok(Vec::new());
    };
    let mut parsed = Vec::new();
    for relic in relics {
        let Some(java_id) = relic.get("id").and_then(Value::as_str) else {
            continue;
        };
        parsed.push(CombatCaseRelicDelta {
            id: java_id.to_string(),
            counter: relic
                .get("counter")
                .and_then(Value::as_i64)
                .map(|value| value as i32),
            used_up: relic.get("used_up").and_then(Value::as_bool),
            amount: relic
                .get("amount")
                .and_then(Value::as_i64)
                .map(|value| value as i32),
        });
    }
    Ok(parsed)
}

fn potion_ids_from_snapshot_value(value: &Value) -> Vec<String> {
    let Some(potions) = value.as_array() else {
        return Vec::new();
    };
    potions
        .iter()
        .map(|potion| {
            potion
                .get("id")
                .and_then(Value::as_str)
                .map(ToString::to_string)
                .unwrap_or_else(|| "Potion Slot".to_string())
        })
        .collect()
}

fn occupied_potion_ids_from_snapshot_value(value: &Value) -> Vec<String> {
    potion_ids_from_snapshot_value(value)
        .into_iter()
        .filter(|id| id != "Potion Slot")
        .collect()
}

fn combat_runtime_delta_from_snapshot(
    snapshot: &Value,
) -> Result<Option<CombatCaseRuntimeDelta>, String> {
    let colorless_combat_pool =
        colorless_pool_ids_from_snapshot_value(snapshot.get("colorless_combat_pool"))?;
    if colorless_combat_pool.is_none() {
        return Ok(None);
    }
    Ok(Some(CombatCaseRuntimeDelta {
        colorless_combat_pool,
    }))
}

fn colorless_pool_ids_from_snapshot_value(
    value: Option<&Value>,
) -> Result<Option<Vec<String>>, String> {
    let Some(cards) = value.and_then(Value::as_array) else {
        return Ok(None);
    };
    let mut ids = Vec::with_capacity(cards.len());
    for card in cards {
        let java_id = card
            .get("id")
            .and_then(Value::as_str)
            .ok_or_else(|| "colorless_combat_pool card missing id".to_string())?;
        card_id_from_java(java_id)
            .ok_or_else(|| format!("unknown Java card id '{java_id}' in colorless_combat_pool"))?;
        ids.push(java_id.to_string());
    }
    Ok(Some(ids))
}

fn rng_delta_from_snapshot(snapshot: &Value) -> Result<Option<CombatCaseRngDelta>, String> {
    let Some(rng_state) = snapshot.get("rng_state").and_then(Value::as_object) else {
        return Ok(None);
    };
    if rng_state.is_empty() {
        return Ok(None);
    }
    Ok(Some(CombatCaseRngDelta {
        ai_rng: rng_channel_from_snapshot_value(rng_state.get("ai_rng"))?,
        shuffle_rng: rng_channel_from_snapshot_value(rng_state.get("shuffle_rng"))?,
        card_rng: rng_channel_from_snapshot_value(rng_state.get("card_rng"))?,
        misc_rng: rng_channel_from_snapshot_value(rng_state.get("misc_rng"))?,
        monster_hp_rng: rng_channel_from_snapshot_value(rng_state.get("monster_hp_rng"))?,
        potion_rng: rng_channel_from_snapshot_value(rng_state.get("potion_rng"))?,
    }))
}

fn rng_channel_from_snapshot_value(
    value: Option<&Value>,
) -> Result<Option<CombatCaseRngChannel>, String> {
    let Some(channel) = value.and_then(Value::as_object) else {
        return Ok(None);
    };
    Ok(Some(CombatCaseRngChannel {
        seed0: channel.get("seed0").and_then(Value::as_i64).or_else(|| {
            channel
                .get("seed0")
                .and_then(Value::as_u64)
                .map(|value| value as i64)
        }),
        seed1: channel.get("seed1").and_then(Value::as_i64).or_else(|| {
            channel
                .get("seed1")
                .and_then(Value::as_u64)
                .map(|value| value as i64)
        }),
        counter: channel
            .get("counter")
            .and_then(Value::as_u64)
            .and_then(|value| u32::try_from(value).ok()),
    }))
}

fn powers_from_snapshot_value(value: Option<&Value>) -> Vec<CombatCasePowerSpec> {
    let Some(powers) = value.and_then(Value::as_array) else {
        return Vec::new();
    };
    powers
        .iter()
        .filter_map(|power| {
            Some(CombatCasePowerSpec {
                id: power.get("id")?.as_str()?.to_string(),
                amount: power.get("amount").and_then(Value::as_i64).unwrap_or(0) as i32,
            })
        })
        .collect()
}

fn zone_cards_from_snapshot_value(value: &Value) -> Result<Vec<CombatCaseCardEntry>, String> {
    let Some(cards) = value.as_array() else {
        return Ok(Vec::new());
    };
    cards.iter().map(card_entry_from_snapshot_card).collect()
}

fn card_entry_from_snapshot_card(card: &Value) -> Result<CombatCaseCardEntry, String> {
    let java_id = card
        .get("id")
        .and_then(Value::as_str)
        .ok_or_else(|| "snapshot card missing id".to_string())?;
    let card_id =
        card_id_from_java(java_id).ok_or_else(|| format!("unknown Java card id '{java_id}'"))?;
    let upgrades = card.get("upgrades").and_then(Value::as_u64).unwrap_or(0) as u8;
    if let Some(explicit_cost) = card
        .get("cost")
        .and_then(Value::as_i64)
        .map(|value| value as i32)
    {
        let mut runtime_card = CombatCard::new(card_id, 0);
        runtime_card.upgrades = upgrades;
        let base_cost = upgraded_base_cost_override(&runtime_card)
            .unwrap_or_else(|| get_card_definition(card_id).cost) as i32;
        if explicit_cost != base_cost {
            return Err(format!(
                "snapshot card '{}' has unsupported modified cost {} (base {})",
                java_id, explicit_cost, base_cost
            ));
        }
    }
    Ok(CombatCaseCardEntry {
        id: java_id.to_string(),
        uuid: card
            .get("uuid")
            .and_then(Value::as_str)
            .map(ToString::to_string),
        upgrades,
        cost: None,
        misc: card
            .get("misc")
            .and_then(Value::as_i64)
            .map(|value| value as i32),
        count: 1,
    })
}

fn monster_delta_from_snapshot(
    index: usize,
    monster: &Value,
) -> Result<CombatCaseMonsterDelta, String> {
    Ok(CombatCaseMonsterDelta {
        monster: index,
        current_hp: monster
            .get("current_hp")
            .or_else(|| monster.get("hp"))
            .and_then(Value::as_i64)
            .map(|value| value as i32),
        max_hp: monster
            .get("max_hp")
            .and_then(Value::as_i64)
            .map(|value| value as i32),
        block: monster
            .get("block")
            .and_then(Value::as_i64)
            .map(|value| value as i32),
        move_id: monster
            .get("move_id")
            .and_then(Value::as_u64)
            .map(|value| value as u8),
        move_history: Some(monster_move_history_from_snapshot(monster)),
        powers: Some(powers_from_snapshot_value(monster.get("powers"))),
        runtime: runtime_delta_from_snapshot(monster),
    })
}

fn monster_move_history_from_snapshot(monster: &Value) -> Vec<u8> {
    let mut history = Vec::new();
    for key in ["second_last_move_id", "last_move_id", "move_id"] {
        let move_id = monster.get(key).and_then(Value::as_u64).unwrap_or(0) as u8;
        if move_id != 0 {
            history.push(move_id);
        }
    }
    history
}

fn runtime_delta_from_snapshot(monster: &Value) -> Option<CombatCaseMonsterRuntimeDelta> {
    let prefix = runtime_prefix_for_monster(
        monster
            .get("id")
            .and_then(Value::as_str)
            .unwrap_or_default(),
    )?;
    let runtime_state = monster.get("runtime_state")?.as_object()?;
    let mut values = BTreeMap::new();
    for (key, value) in runtime_state {
        values.insert(format!("{prefix}.{key}"), value.clone());
    }
    Some(CombatCaseMonsterRuntimeDelta { values })
}

fn runtime_prefix_for_monster(java_id: &str) -> Option<&'static str> {
    match normalize_identifier(java_id).as_str() {
        "awakenedone" => Some("awakened_one"),
        "bookofstabbing" => Some("book_of_stabbing"),
        "bronzeautomaton" => Some("bronze_automaton"),
        "bronzeorb" => Some("bronze_orb"),
        "byrd" => Some("byrd"),
        "champ" => Some("champ"),
        "chosen" => Some("chosen"),
        "corruptheart" => Some("corrupt_heart"),
        "darkling" => Some("darkling"),
        "hexaghost" => Some("hexaghost"),
        "lagavulin" => Some("lagavulin"),
        "lousedefensive" | "lousenormal" => Some("louse"),
        "looter" | "mugger" => Some("thief"),
        "shelledparasite" => Some("shelled_parasite"),
        "snecko" => Some("snecko"),
        "thecollector" => Some("collector"),
        "theguardian" => Some("guardian"),
        _ => None,
    }
}

fn apply_case_delta(
    seed: &mut CombatCaseRuntimeSeed,
    delta: &CombatCaseDelta,
) -> Result<(), String> {
    if let Some(player) = &delta.player {
        if let Some(current_hp) = player.current_hp {
            seed.combat.entities.player.current_hp = current_hp;
        }
        if let Some(max_hp) = player.max_hp {
            seed.combat.entities.player.max_hp = max_hp;
        }
        if let Some(block) = player.block {
            seed.combat.entities.player.block = block;
        }
        if let Some(gold) = player.gold {
            seed.combat.entities.player.gold = gold;
        }
        if let Some(energy) = player.energy {
            seed.combat.turn.energy = energy;
        }
        if let Some(powers) = &player.powers {
            set_powers_for(&mut seed.combat, 0, compile_powers(powers)?);
        }
    }

    for monster_delta in &delta.monsters {
        let monster = seed
            .combat
            .entities
            .monsters
            .get_mut(monster_delta.monster)
            .ok_or_else(|| format!("monster delta index {} out of range", monster_delta.monster))?;
        if let Some(current_hp) = monster_delta.current_hp {
            monster.current_hp = current_hp;
        }
        if let Some(max_hp) = monster_delta.max_hp {
            monster.max_hp = max_hp;
        }
        if let Some(block) = monster_delta.block {
            monster.block = block;
        }
        if let Some(move_id) = monster_delta.move_id {
            monster.set_planned_move_id(move_id);
        }
        if let Some(move_history) = &monster_delta.move_history {
            monster.move_history_mut().clear();
            monster
                .move_history_mut()
                .extend(move_history.iter().copied());
        }
        if let Some(runtime) = &monster_delta.runtime {
            apply_monster_runtime_delta(monster, runtime)?;
        }
    }

    for monster_delta in &delta.monsters {
        if let Some(powers) = &monster_delta.powers {
            let monster_id = seed
                .combat
                .entities
                .monsters
                .get(monster_delta.monster)
                .ok_or_else(|| {
                    format!("monster delta index {} out of range", monster_delta.monster)
                })?
                .id;
            set_powers_for(&mut seed.combat, monster_id, compile_powers(powers)?);
        }
    }

    for relic_delta in &delta.relics {
        let relic_id = relic_id_from_java(&relic_delta.id)
            .ok_or_else(|| format!("unknown Java relic id '{}'", relic_delta.id))?;
        let relic = seed
            .combat
            .entities
            .player
            .relics
            .iter_mut()
            .find(|relic| relic.id == relic_id)
            .ok_or_else(|| format!("relic '{}' not present in combat state", relic_delta.id))?;
        if let Some(counter) = relic_delta.counter {
            relic.counter = counter;
        }
        if let Some(used_up) = relic_delta.used_up {
            relic.used_up = used_up;
        }
        if let Some(amount) = relic_delta.amount {
            relic.amount = amount;
        }
    }

    if let Some(zones) = &delta.zones {
        if let Some(cards) = &zones.hand {
            seed.combat.zones.hand = compile_zone_cards(cards, "hand", false)?;
        }
        if let Some(cards) = &zones.draw_pile {
            seed.combat.zones.draw_pile = compile_zone_cards(cards, "draw", true)?;
        }
        if let Some(cards) = &zones.discard_pile {
            seed.combat.zones.discard_pile = compile_zone_cards(cards, "discard", false)?;
        }
        if let Some(cards) = &zones.exhaust_pile {
            seed.combat.zones.exhaust_pile = compile_zone_cards(cards, "exhaust", false)?;
        }
        if let Some(cards) = &zones.limbo {
            seed.combat.zones.limbo = compile_zone_cards(cards, "limbo", false)?;
        }
    }

    if let Some(potions) = &delta.potions {
        seed.combat.entities.potions = compile_potions(potions)?;
    }

    if let Some(runtime) = &delta.runtime {
        if let Some(colorless_combat_pool) = &runtime.colorless_combat_pool {
            seed.combat.runtime.colorless_combat_pool = colorless_combat_pool
                .iter()
                .map(|java_id| {
                    card_id_from_java(java_id)
                        .ok_or_else(|| format!("unknown Java card id '{java_id}'"))
                })
                .collect::<Result<Vec<_>, _>>()?;
        }
    }

    if let Some(rng) = &delta.rng {
        if let Some(channel) = &rng.ai_rng {
            apply_rng_channel_delta(&mut seed.combat.rng.ai_rng, channel);
        }
        if let Some(channel) = &rng.shuffle_rng {
            apply_rng_channel_delta(&mut seed.combat.rng.shuffle_rng, channel);
        }
        if let Some(channel) = &rng.card_rng {
            apply_rng_channel_delta(&mut seed.combat.rng.card_random_rng, channel);
        }
        if let Some(channel) = &rng.misc_rng {
            apply_rng_channel_delta(&mut seed.combat.rng.misc_rng, channel);
        }
        if let Some(channel) = &rng.monster_hp_rng {
            apply_rng_channel_delta(&mut seed.combat.rng.monster_hp_rng, channel);
        }
        if let Some(channel) = &rng.potion_rng {
            apply_rng_channel_delta(&mut seed.combat.rng.potion_rng, channel);
        }
    }

    if let Some(engine) = &delta.engine {
        if let Some(response_id) = engine.response_id {
            seed.response_id = Some(response_id);
        }
        if let Some(frame_id) = engine.frame_id {
            seed.frame_id = Some(frame_id);
        }
        if let Some(turn_count) = engine.turn_count {
            seed.combat.turn.turn_count = turn_count;
        }
        if engine.screen_type.is_some() || engine.screen_state.is_some() {
            let meta = CombatCaseRootMeta {
                screen_type: engine.screen_type.clone(),
                screen_state: engine.screen_state.clone(),
                ..Default::default()
            };
            seed.engine_state = build_engine_state_from_root_meta(&meta, None, &mut seed.combat);
        }
    }

    Ok(())
}

fn apply_rng_channel_delta(rng: &mut StsRng, delta: &CombatCaseRngChannel) {
    if let Some(seed0) = delta.seed0 {
        rng.seed0 = seed0 as u64;
    }
    if let Some(seed1) = delta.seed1 {
        rng.seed1 = seed1 as u64;
    }
    if let Some(counter) = delta.counter {
        rng.counter = counter;
    }
}

fn build_engine_state_from_root_meta(
    root_meta: &CombatCaseRootMeta,
    protocol_meta: Option<&Value>,
    combat: &mut CombatState,
) -> EngineState {
    let screen_type = root_meta.screen_type.as_deref().unwrap_or("NONE");
    if screen_type != "CARD_REWARD" {
        return EngineState::CombatPlayerTurn;
    }

    let Some(screen_state) = root_meta.screen_state.as_ref().and_then(Value::as_object) else {
        return EngineState::CombatPlayerTurn;
    };
    let Some(cards) = screen_state.get("cards").and_then(Value::as_array) else {
        return EngineState::CombatPlayerTurn;
    };
    let offered = cards
        .iter()
        .filter_map(screen_card_to_rust_id)
        .collect::<Vec<_>>();
    if offered.is_empty() {
        return EngineState::CombatPlayerTurn;
    }

    let last_command_kind = protocol_meta
        .and_then(|meta| meta.get("last_command_kind"))
        .and_then(Value::as_str);
    let last_command = protocol_meta
        .and_then(|meta| meta.get("last_command"))
        .and_then(Value::as_str)
        .unwrap_or("");
    let skip_available = screen_state
        .get("skip_available")
        .and_then(Value::as_bool)
        .unwrap_or(false);

    if matches!(last_command_kind, Some("potion")) || last_command.starts_with("POTION USE ") {
        combat.turn.counters.discovery_cost_for_turn = Some(0);
        return EngineState::PendingChoice(PendingChoice::DiscoverySelect(offered));
    }

    EngineState::PendingChoice(PendingChoice::CardRewardSelect {
        cards: offered,
        destination: crate::runtime::action::CardDestination::Hand,
        can_skip: skip_available,
    })
}

fn compile_powers(specs: &[CombatCasePowerSpec]) -> Result<Vec<Power>, String> {
    specs
        .iter()
        .map(|spec| {
            let power_type = power_id_from_java(&spec.id)
                .ok_or_else(|| format!("unknown Java power id '{}'", spec.id))?;
            Ok(Power {
                power_type,
                instance_id: None,
                amount: spec.amount,
                extra_data: 0,
                just_applied: false,
            })
        })
        .collect()
}

fn compile_zone_cards(
    specs: &[CombatCaseCardEntry],
    zone_prefix: &str,
    reverse_output: bool,
) -> Result<Vec<CombatCard>, String> {
    let mut cards = Vec::new();
    let mut zone_index = 0usize;
    for spec in specs {
        let card_id = card_id_from_java(&spec.id)
            .ok_or_else(|| format!("unknown Java card id '{}'", spec.id))?;
        let def = get_card_definition(card_id);
        for local_index in 0..spec.count.max(1) {
            let uuid = spec
                .uuid
                .clone()
                .unwrap_or_else(|| format!("{zone_prefix}-{zone_index}-{local_index}"));
            let mut card = CombatCard::new(card_id, stable_uuid(&uuid));
            card.upgrades = spec.upgrades;
            card.misc_value = spec.misc.unwrap_or(0);
            if let Some(explicit_cost) = spec.cost {
                let base_cost =
                    crate::content::cards::upgraded_base_cost_override(&card).unwrap_or(def.cost);
                if explicit_cost != base_cost as i32 {
                    return Err(format!(
                        "combat case does not support per-card cost override yet: {} cost {} != base {}",
                        spec.id, explicit_cost, base_cost
                    ));
                }
            }
            cards.push(card);
        }
        zone_index += 1;
    }
    if reverse_output {
        cards.reverse();
    }
    Ok(cards)
}

fn compile_potions(
    potions: &[String],
) -> Result<Vec<Option<crate::content::potions::Potion>>, String> {
    let mut compiled = Vec::new();
    for (index, id) in potions.iter().enumerate() {
        if id == "Potion Slot" {
            compiled.push(None);
            continue;
        }
        let potion_id = crate::protocol::java::java_potion_id_to_rust(id)
            .ok_or_else(|| format!("unknown Java potion id '{}'", id))?;
        compiled.push(Some(crate::content::potions::Potion::new(
            potion_id,
            50_000 + index as u32,
        )));
    }
    while compiled.len() < 3 {
        compiled.push(None);
    }
    Ok(compiled)
}

fn apply_monster_runtime_delta(
    monster: &mut crate::runtime::combat::MonsterEntity,
    delta: &CombatCaseMonsterRuntimeDelta,
) -> Result<(), String> {
    for (field, value) in &delta.values {
        match field.as_str() {
            "hexaghost.activated" => monster.hexaghost.activated = json_bool(value, field)?,
            "hexaghost.orb_active_count" => {
                monster.hexaghost.orb_active_count = json_u8(value, field)?
            }
            "hexaghost.burn_upgraded" => monster.hexaghost.burn_upgraded = json_bool(value, field)?,
            "hexaghost.divider_damage" => {
                monster.hexaghost.divider_damage = json_option_i32(value, field)?
            }
            "louse.bite_damage" => monster.louse.bite_damage = json_option_i32(value, field)?,
            "thief.protocol_seeded" => monster.thief.protocol_seeded = json_bool(value, field)?,
            "thief.slash_count" => monster.thief.slash_count = json_u8(value, field)?,
            "thief.stolen_gold" => monster.thief.stolen_gold = json_i32(value, field)?,
            "byrd.protocol_seeded" => monster.byrd.protocol_seeded = json_bool(value, field)?,
            "byrd.first_move" => monster.byrd.first_move = json_bool(value, field)?,
            "byrd.is_flying" => monster.byrd.is_flying = json_bool(value, field)?,
            "chosen.protocol_seeded" => monster.chosen.protocol_seeded = json_bool(value, field)?,
            "chosen.first_turn" => monster.chosen.first_turn = json_bool(value, field)?,
            "chosen.used_hex" => monster.chosen.used_hex = json_bool(value, field)?,
            "snecko.protocol_seeded" => monster.snecko.protocol_seeded = json_bool(value, field)?,
            "snecko.first_turn" => monster.snecko.first_turn = json_bool(value, field)?,
            "shelled_parasite.protocol_seeded" => {
                monster.shelled_parasite.protocol_seeded = json_bool(value, field)?
            }
            "shelled_parasite.first_move" => {
                monster.shelled_parasite.first_move = json_bool(value, field)?
            }
            "bronze_automaton.protocol_seeded" => {
                monster.bronze_automaton.protocol_seeded = json_bool(value, field)?
            }
            "bronze_automaton.first_turn" => {
                monster.bronze_automaton.first_turn = json_bool(value, field)?
            }
            "bronze_automaton.num_turns" => {
                monster.bronze_automaton.num_turns = json_u8(value, field)?
            }
            "bronze_orb.protocol_seeded" => {
                monster.bronze_orb.protocol_seeded = json_bool(value, field)?
            }
            "bronze_orb.used_stasis" => monster.bronze_orb.used_stasis = json_bool(value, field)?,
            "book_of_stabbing.protocol_seeded" => {
                monster.book_of_stabbing.protocol_seeded = json_bool(value, field)?
            }
            "book_of_stabbing.stab_count" => {
                monster.book_of_stabbing.stab_count = json_u8(value, field)?
            }
            "collector.protocol_seeded" => {
                monster.collector.protocol_seeded = json_bool(value, field)?
            }
            "collector.initial_spawn" => monster.collector.initial_spawn = json_bool(value, field)?,
            "collector.ult_used" => monster.collector.ult_used = json_bool(value, field)?,
            "collector.turns_taken" => monster.collector.turns_taken = json_u8(value, field)?,
            "champ.protocol_seeded" => monster.champ.protocol_seeded = json_bool(value, field)?,
            "champ.first_turn" => monster.champ.first_turn = json_bool(value, field)?,
            "champ.num_turns" => monster.champ.num_turns = json_u8(value, field)?,
            "champ.forge_times" => monster.champ.forge_times = json_u8(value, field)?,
            "champ.threshold_reached" => monster.champ.threshold_reached = json_bool(value, field)?,
            "awakened_one.protocol_seeded" => {
                monster.awakened_one.protocol_seeded = json_bool(value, field)?
            }
            "awakened_one.form1" => monster.awakened_one.form1 = json_bool(value, field)?,
            "awakened_one.first_turn" => monster.awakened_one.first_turn = json_bool(value, field)?,
            "corrupt_heart.protocol_seeded" => {
                monster.corrupt_heart.protocol_seeded = json_bool(value, field)?
            }
            "corrupt_heart.first_move" => {
                monster.corrupt_heart.first_move = json_bool(value, field)?
            }
            "corrupt_heart.move_count" => monster.corrupt_heart.move_count = json_u8(value, field)?,
            "corrupt_heart.buff_count" => monster.corrupt_heart.buff_count = json_u8(value, field)?,
            "darkling.first_move" => monster.darkling.first_move = json_bool(value, field)?,
            "darkling.nip_dmg" => monster.darkling.nip_dmg = json_i32(value, field)?,
            "lagavulin.is_out" => monster.lagavulin.is_out = json_bool(value, field)?,
            "lagavulin.idle_count" => monster.lagavulin.idle_count = json_u8(value, field)?,
            "lagavulin.debuff_turn_count" => {
                monster.lagavulin.debuff_turn_count = json_u8(value, field)?
            }
            "lagavulin.is_out_triggered" => {
                monster.lagavulin.is_out_triggered = json_bool(value, field)?
            }
            "guardian.damage_threshold" => {
                monster.guardian.damage_threshold = json_i32(value, field)?
            }
            "guardian.damage_taken" => monster.guardian.damage_taken = json_i32(value, field)?,
            "guardian.is_open" => monster.guardian.is_open = json_bool(value, field)?,
            "guardian.close_up_triggered" => {
                monster.guardian.close_up_triggered = json_bool(value, field)?
            }
            other => {
                return Err(format!("unsupported monster runtime delta field '{other}'"));
            }
        }
    }
    Ok(())
}

fn case_step_from_scenario_step(step: &ScenarioStep) -> Result<CombatCaseStep, String> {
    let structured = if let Some(structured) = &step.structured {
        case_program_step_from_structured(structured.clone())
    } else {
        parse_case_program_step(&step.command)?
    };
    Ok(CombatCaseStep {
        step: structured,
        label: step.label.clone(),
        response_id: step.response_id,
        frame_id: step.frame_id,
        command_kind: step.command_kind.clone(),
    })
}

fn case_program_step_from_structured(step: StructuredScenarioStep) -> CombatCaseProgramStep {
    match step {
        StructuredScenarioStep::Play { selector, target } => CombatCaseProgramStep::Play {
            selector: selector_from_scenario(selector),
            target,
        },
        StructuredScenarioStep::End => CombatCaseProgramStep::End,
        StructuredScenarioStep::Cancel => CombatCaseProgramStep::Cancel,
        StructuredScenarioStep::Choose { index } => CombatCaseProgramStep::Choose { index },
        StructuredScenarioStep::PotionUse { slot, target } => {
            CombatCaseProgramStep::PotionUse { slot, target }
        }
        StructuredScenarioStep::HandSelect { selectors } => CombatCaseProgramStep::HandSelect {
            selectors: selectors.into_iter().map(selector_from_scenario).collect(),
        },
        StructuredScenarioStep::GridSelect { selectors } => CombatCaseProgramStep::GridSelect {
            selectors: selectors.into_iter().map(selector_from_scenario).collect(),
        },
    }
}

fn selector_from_scenario(selector: ScenarioCardSelector) -> CombatCaseCardSelector {
    match selector {
        ScenarioCardSelector::Index { index } => CombatCaseCardSelector::Index { index },
        ScenarioCardSelector::JavaId { id, occurrence } => {
            CombatCaseCardSelector::JavaId { id, occurrence }
        }
    }
}

fn parse_case_program_step(command: &str) -> Result<CombatCaseProgramStep, String> {
    let parts: Vec<&str> = command.split_whitespace().collect();
    match parts.as_slice() {
        ["END"] => Ok(CombatCaseProgramStep::End),
        ["CANCEL"] => Ok(CombatCaseProgramStep::Cancel),
        ["PLAY", card_idx] => Ok(CombatCaseProgramStep::Play {
            selector: CombatCaseCardSelector::Index {
                index: parse_index(card_idx, "PLAY")?,
            },
            target: None,
        }),
        ["PLAY", card_idx, target] => Ok(CombatCaseProgramStep::Play {
            selector: CombatCaseCardSelector::Index {
                index: parse_index(card_idx, "PLAY")?,
            },
            target: Some(
                target
                    .parse::<usize>()
                    .map_err(|_| format!("invalid PLAY target '{target}'"))?,
            ),
        }),
        ["POTION", "USE", slot] => Ok(CombatCaseProgramStep::PotionUse {
            slot: slot
                .parse::<usize>()
                .map_err(|_| format!("invalid potion slot '{slot}'"))?,
            target: None,
        }),
        ["POTION", "USE", slot, target] => Ok(CombatCaseProgramStep::PotionUse {
            slot: slot
                .parse::<usize>()
                .map_err(|_| format!("invalid potion slot '{slot}'"))?,
            target: Some(
                target
                    .parse::<usize>()
                    .map_err(|_| format!("invalid potion target '{target}'"))?,
            ),
        }),
        ["HUMAN_CARD_REWARD", "SKIP"] => Ok(CombatCaseProgramStep::Cancel),
        ["HUMAN_CARD_REWARD", choice_idx] => Ok(CombatCaseProgramStep::Choose {
            index: choice_idx
                .parse::<usize>()
                .map_err(|_| format!("invalid reward choice '{choice_idx}'"))?,
        }),
        _ => Err(format!("unsupported raw scenario command '{command}'")),
    }
}

fn parse_index(value: &str, context: &str) -> Result<usize, String> {
    let parsed = value
        .parse::<usize>()
        .map_err(|_| format!("invalid {context} card index '{value}'"))?;
    if parsed == 0 {
        return Err(format!("{context} card index is 1-based; got 0"));
    }
    Ok(parsed)
}

fn expectation_from_scenario_assertion(
    assertion: &ScenarioAssertion,
) -> Result<CombatCaseExpectation, String> {
    let expected = match parse_expected(assertion)? {
        ActualFieldValue::Missing => CombatCaseScalarValue::Missing,
        ActualFieldValue::Number(value) => CombatCaseScalarValue::Number { value },
        ActualFieldValue::String(value) => CombatCaseScalarValue::String { value },
        ActualFieldValue::Bool(value) => CombatCaseScalarValue::Bool { value },
    };
    Ok(CombatCaseExpectation {
        check: CombatCaseCheck::Path {
            field: assertion.field.clone(),
            expected,
        },
        response_id: assertion.response_id,
        frame_id: assertion.frame_id,
        note: assertion.note.clone(),
    })
}

fn oracle_from_scenario_kind(kind: ScenarioOracleKind) -> CombatCaseOracle {
    let primary = match kind {
        ScenarioOracleKind::Synthetic => CombatCaseOracleKind::Invariant,
        ScenarioOracleKind::Live => CombatCaseOracleKind::Differential,
        ScenarioOracleKind::JavaHarness => CombatCaseOracleKind::LiveRuntime,
    };
    CombatCaseOracle {
        primary: primary.clone(),
        evidence: vec![primary],
        note: None,
    }
}

fn provenance_from_scenario(
    provenance: Option<crate::testing::fixtures::scenario::ScenarioProvenance>,
) -> CombatCaseProvenance {
    let Some(provenance) = provenance else {
        return CombatCaseProvenance::default();
    };
    CombatCaseProvenance {
        source: provenance.source,
        source_path: provenance.source_path,
        response_id_range: provenance.response_id_range,
        failure_frame: provenance.failure_frame,
        assertion_source_frames: provenance.assertion_source_frames,
        assertion_source_response_ids: provenance.assertion_source_response_ids,
        debug_context_summary: provenance.debug_context_summary,
        aspect_summary: provenance.aspect_summary,
        notes: provenance.notes,
    }
}

fn input_for_case_step(
    step: &CombatCaseStep,
    engine_state: &EngineState,
    combat: &CombatState,
) -> Option<ClientInput> {
    let structured = scenario_step_from_case_step(step);
    crate::testing::fixtures::scenario::input_for_step(&structured, engine_state, combat)
}

fn scenario_step_from_case_step(step: &CombatCaseStep) -> ScenarioStep {
    ScenarioStep {
        command: describe_case_step(&step.step),
        label: step.label.clone(),
        response_id: step.response_id,
        frame_id: step.frame_id,
        command_kind: step.command_kind.clone(),
        structured: Some(match &step.step {
            CombatCaseProgramStep::Play { selector, target } => StructuredScenarioStep::Play {
                selector: scenario_selector_from_case(selector.clone()),
                target: *target,
            },
            CombatCaseProgramStep::End => StructuredScenarioStep::End,
            CombatCaseProgramStep::Cancel => StructuredScenarioStep::Cancel,
            CombatCaseProgramStep::Choose { index } => {
                StructuredScenarioStep::Choose { index: *index }
            }
            CombatCaseProgramStep::PotionUse { slot, target } => {
                StructuredScenarioStep::PotionUse {
                    slot: *slot,
                    target: *target,
                }
            }
            CombatCaseProgramStep::HandSelect { selectors } => StructuredScenarioStep::HandSelect {
                selectors: selectors
                    .iter()
                    .cloned()
                    .map(scenario_selector_from_case)
                    .collect(),
            },
            CombatCaseProgramStep::GridSelect { selectors } => StructuredScenarioStep::GridSelect {
                selectors: selectors
                    .iter()
                    .cloned()
                    .map(scenario_selector_from_case)
                    .collect(),
            },
        }),
    }
}

fn scenario_selector_from_case(selector: CombatCaseCardSelector) -> ScenarioCardSelector {
    match selector {
        CombatCaseCardSelector::Index { index } => ScenarioCardSelector::Index { index },
        CombatCaseCardSelector::JavaId { id, occurrence } => {
            ScenarioCardSelector::JavaId { id, occurrence }
        }
    }
}

fn describe_case_step(step: &CombatCaseProgramStep) -> String {
    match step {
        CombatCaseProgramStep::Play { selector, target } => {
            let mut base = match selector {
                CombatCaseCardSelector::Index { index } => format!("PLAY {index}"),
                CombatCaseCardSelector::JavaId { id, occurrence } => {
                    if *occurrence == 1 {
                        format!("PLAY_ID {id}")
                    } else {
                        format!("PLAY_ID {id} #{occurrence}")
                    }
                }
            };
            if let Some(target) = target {
                base.push_str(&format!(" -> {target}"));
            }
            base
        }
        CombatCaseProgramStep::End => "END".to_string(),
        CombatCaseProgramStep::Cancel => "CANCEL".to_string(),
        CombatCaseProgramStep::Choose { index } => format!("HUMAN_CARD_REWARD {index}"),
        CombatCaseProgramStep::PotionUse { slot, target } => match target {
            Some(target) => format!("POTION USE {slot} {target}"),
            None => format!("POTION USE {slot}"),
        },
        CombatCaseProgramStep::HandSelect { selectors } => format!(
            "HAND_SELECT {}",
            selectors
                .iter()
                .map(describe_selector)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        CombatCaseProgramStep::GridSelect { selectors } => format!(
            "GRID_SELECT {}",
            selectors
                .iter()
                .map(describe_selector)
                .collect::<Vec<_>>()
                .join(", ")
        ),
    }
}

fn describe_selector(selector: &CombatCaseCardSelector) -> String {
    match selector {
        CombatCaseCardSelector::Index { index } => format!("#{index}"),
        CombatCaseCardSelector::JavaId { id, occurrence } => {
            if *occurrence == 1 {
                id.clone()
            } else {
                format!("{id}#{occurrence}")
            }
        }
    }
}

fn combat_for_expectation<'a>(
    replay: &'a CombatCaseReplay,
    expectation: &CombatCaseExpectation,
) -> Result<&'a CombatState, String> {
    if expectation.response_id.is_none() && expectation.frame_id.is_none() {
        return Ok(&replay.combat);
    }

    replay
        .snapshots
        .iter()
        .rev()
        .find(|snapshot| {
            expectation
                .response_id
                .map_or(true, |response_id| snapshot.response_id == Some(response_id))
                && expectation
                    .frame_id
                    .map_or(true, |frame_id| snapshot.frame_id == Some(frame_id))
        })
        .map(|snapshot| &snapshot.combat)
        .ok_or_else(|| {
            let available_response_ids = replay
                .snapshots
                .iter()
                .filter_map(|snapshot| snapshot.response_id)
                .collect::<Vec<_>>();
            let available_frame_ids = replay
                .snapshots
                .iter()
                .filter_map(|snapshot| snapshot.frame_id)
                .collect::<Vec<_>>();
            format!(
                "no snapshot for response_id={:?} frame_id={:?}; available response_ids={available_response_ids:?}, frame_ids={available_frame_ids:?}",
                expectation.response_id, expectation.frame_id
            )
        })
}

fn extract_expectation_value(
    combat: &CombatState,
    check: &CombatCaseCheck,
) -> Result<ActualFieldValue, String> {
    Ok(match check {
        CombatCaseCheck::Path { field, .. } => extract_field_value(combat, field),
        CombatCaseCheck::PlayerStat { stat, .. } => extract_field_value(
            combat,
            &format!("player.{}", normalize_player_stat_name(stat)?),
        ),
        CombatCaseCheck::PlayerPower { id, .. } => {
            extract_field_value(combat, &format!("player.power[{id}].amount"))
        }
        CombatCaseCheck::MonsterStat { monster, stat, .. } => extract_field_value(
            combat,
            &format!("monster[{monster}].{}", normalize_monster_stat_name(stat)?),
        ),
        CombatCaseCheck::MonsterPower { monster, id, .. } => {
            extract_field_value(combat, &format!("monster[{monster}].power[{id}].amount"))
        }
        CombatCaseCheck::MonsterRuntime { monster, field, .. } => {
            extract_monster_runtime_value(combat, *monster, field)?
        }
        CombatCaseCheck::PileContains { pile, id, .. } => extract_field_value(
            combat,
            &format!("{}.contains[{id}]", normalize_pile_name(pile)?),
        ),
        CombatCaseCheck::PileCount { pile, id, .. } => extract_field_value(
            combat,
            &format!("{}.count[{id}]", normalize_pile_name(pile)?),
        ),
        CombatCaseCheck::PileSize { pile, .. } => {
            extract_field_value(combat, &format!("{}_size", normalize_pile_name(pile)?))
        }
        CombatCaseCheck::RelicPresent { id, .. } => {
            extract_field_value(combat, &format!("relics.contains[{id}]"))
        }
        CombatCaseCheck::RelicCount { id, .. } => {
            extract_field_value(combat, &format!("relics.count[{id}]"))
        }
        CombatCaseCheck::RelicRuntime { id, field, .. } => {
            extract_relic_runtime_value(combat, id, field)?
        }
    })
}

fn expected_value_for_check(check: &CombatCaseCheck) -> Result<ActualFieldValue, String> {
    Ok(match check {
        CombatCaseCheck::Path { expected, .. }
        | CombatCaseCheck::MonsterRuntime { expected, .. }
        | CombatCaseCheck::RelicRuntime { expected, .. } => scalar_to_actual(expected),
        CombatCaseCheck::PlayerStat { value, .. }
        | CombatCaseCheck::MonsterStat { value, .. }
        | CombatCaseCheck::PileCount { count: value, .. }
        | CombatCaseCheck::PileSize { count: value, .. }
        | CombatCaseCheck::RelicCount { count: value, .. } => ActualFieldValue::Number(*value),
        CombatCaseCheck::PlayerPower { amount, .. }
        | CombatCaseCheck::MonsterPower { amount, .. } => ActualFieldValue::Number(*amount),
        CombatCaseCheck::PileContains { present, .. }
        | CombatCaseCheck::RelicPresent { present, .. } => ActualFieldValue::Bool(*present),
    })
}

fn scalar_to_actual(value: &CombatCaseScalarValue) -> ActualFieldValue {
    match value {
        CombatCaseScalarValue::Missing => ActualFieldValue::Missing,
        CombatCaseScalarValue::Number { value } => ActualFieldValue::Number(*value),
        CombatCaseScalarValue::String { value } => ActualFieldValue::String(value.clone()),
        CombatCaseScalarValue::Bool { value } => ActualFieldValue::Bool(*value),
    }
}

fn format_expectation_scope(expectation: &CombatCaseExpectation) -> String {
    let mut parts = Vec::new();
    if let Some(response_id) = expectation.response_id {
        parts.push(format!("response_id={response_id}"));
    }
    if let Some(frame_id) = expectation.frame_id {
        parts.push(format!("frame_id={frame_id}"));
    }
    if parts.is_empty() {
        String::new()
    } else {
        format!(" [{}]", parts.join(", "))
    }
}

fn extract_monster_runtime_value(
    combat: &CombatState,
    monster_index: usize,
    field: &str,
) -> Result<ActualFieldValue, String> {
    let Some(monster) = combat.entities.monsters.get(monster_index) else {
        return Ok(ActualFieldValue::Missing);
    };
    Ok(match field {
        "hexaghost.activated" => ActualFieldValue::Bool(monster.hexaghost.activated),
        "hexaghost.orb_active_count" => {
            ActualFieldValue::Number(monster.hexaghost.orb_active_count as i64)
        }
        "hexaghost.burn_upgraded" => ActualFieldValue::Bool(monster.hexaghost.burn_upgraded),
        "hexaghost.divider_damage" => monster
            .hexaghost
            .divider_damage
            .map(|value| ActualFieldValue::Number(value as i64))
            .unwrap_or(ActualFieldValue::Missing),
        "louse.bite_damage" => monster
            .louse
            .bite_damage
            .map(|value| ActualFieldValue::Number(value as i64))
            .unwrap_or(ActualFieldValue::Missing),
        "thief.protocol_seeded" => ActualFieldValue::Bool(monster.thief.protocol_seeded),
        "thief.slash_count" => ActualFieldValue::Number(monster.thief.slash_count as i64),
        "thief.stolen_gold" => ActualFieldValue::Number(monster.thief.stolen_gold as i64),
        "byrd.protocol_seeded" => ActualFieldValue::Bool(monster.byrd.protocol_seeded),
        "byrd.first_move" => ActualFieldValue::Bool(monster.byrd.first_move),
        "byrd.is_flying" => ActualFieldValue::Bool(monster.byrd.is_flying),
        "chosen.protocol_seeded" => ActualFieldValue::Bool(monster.chosen.protocol_seeded),
        "chosen.first_turn" => ActualFieldValue::Bool(monster.chosen.first_turn),
        "chosen.used_hex" => ActualFieldValue::Bool(monster.chosen.used_hex),
        "snecko.protocol_seeded" => ActualFieldValue::Bool(monster.snecko.protocol_seeded),
        "snecko.first_turn" => ActualFieldValue::Bool(monster.snecko.first_turn),
        "shelled_parasite.protocol_seeded" => {
            ActualFieldValue::Bool(monster.shelled_parasite.protocol_seeded)
        }
        "shelled_parasite.first_move" => {
            ActualFieldValue::Bool(monster.shelled_parasite.first_move)
        }
        "bronze_automaton.protocol_seeded" => {
            ActualFieldValue::Bool(monster.bronze_automaton.protocol_seeded)
        }
        "bronze_automaton.first_turn" => {
            ActualFieldValue::Bool(monster.bronze_automaton.first_turn)
        }
        "bronze_automaton.num_turns" => {
            ActualFieldValue::Number(monster.bronze_automaton.num_turns as i64)
        }
        "bronze_orb.protocol_seeded" => ActualFieldValue::Bool(monster.bronze_orb.protocol_seeded),
        "bronze_orb.used_stasis" => ActualFieldValue::Bool(monster.bronze_orb.used_stasis),
        "book_of_stabbing.protocol_seeded" => {
            ActualFieldValue::Bool(monster.book_of_stabbing.protocol_seeded)
        }
        "book_of_stabbing.stab_count" => {
            ActualFieldValue::Number(monster.book_of_stabbing.stab_count as i64)
        }
        "collector.protocol_seeded" => ActualFieldValue::Bool(monster.collector.protocol_seeded),
        "collector.initial_spawn" => ActualFieldValue::Bool(monster.collector.initial_spawn),
        "collector.ult_used" => ActualFieldValue::Bool(monster.collector.ult_used),
        "collector.turns_taken" => ActualFieldValue::Number(monster.collector.turns_taken as i64),
        "champ.protocol_seeded" => ActualFieldValue::Bool(monster.champ.protocol_seeded),
        "champ.first_turn" => ActualFieldValue::Bool(monster.champ.first_turn),
        "champ.num_turns" => ActualFieldValue::Number(monster.champ.num_turns as i64),
        "champ.forge_times" => ActualFieldValue::Number(monster.champ.forge_times as i64),
        "champ.threshold_reached" => ActualFieldValue::Bool(monster.champ.threshold_reached),
        "awakened_one.protocol_seeded" => {
            ActualFieldValue::Bool(monster.awakened_one.protocol_seeded)
        }
        "awakened_one.form1" => ActualFieldValue::Bool(monster.awakened_one.form1),
        "awakened_one.first_turn" => ActualFieldValue::Bool(monster.awakened_one.first_turn),
        "corrupt_heart.protocol_seeded" => {
            ActualFieldValue::Bool(monster.corrupt_heart.protocol_seeded)
        }
        "corrupt_heart.first_move" => ActualFieldValue::Bool(monster.corrupt_heart.first_move),
        "corrupt_heart.move_count" => {
            ActualFieldValue::Number(monster.corrupt_heart.move_count as i64)
        }
        "corrupt_heart.buff_count" => {
            ActualFieldValue::Number(monster.corrupt_heart.buff_count as i64)
        }
        "darkling.first_move" => ActualFieldValue::Bool(monster.darkling.first_move),
        "darkling.nip_dmg" => ActualFieldValue::Number(monster.darkling.nip_dmg as i64),
        "lagavulin.is_out" => ActualFieldValue::Bool(monster.lagavulin.is_out),
        "lagavulin.idle_count" => ActualFieldValue::Number(monster.lagavulin.idle_count as i64),
        "lagavulin.debuff_turn_count" => {
            ActualFieldValue::Number(monster.lagavulin.debuff_turn_count as i64)
        }
        "lagavulin.is_out_triggered" => ActualFieldValue::Bool(monster.lagavulin.is_out_triggered),
        "guardian.damage_threshold" => {
            ActualFieldValue::Number(monster.guardian.damage_threshold as i64)
        }
        "guardian.damage_taken" => ActualFieldValue::Number(monster.guardian.damage_taken as i64),
        "guardian.is_open" => ActualFieldValue::Bool(monster.guardian.is_open),
        "guardian.close_up_triggered" => {
            ActualFieldValue::Bool(monster.guardian.close_up_triggered)
        }
        other => return Err(format!("unsupported monster runtime field '{other}'")),
    })
}

fn extract_relic_runtime_value(
    combat: &CombatState,
    java_id: &str,
    field: &str,
) -> Result<ActualFieldValue, String> {
    let relic_id =
        relic_id_from_java(java_id).ok_or_else(|| format!("unknown Java relic id '{java_id}'"))?;
    let Some(relic) = combat
        .entities
        .player
        .relics
        .iter()
        .find(|relic| relic.id == relic_id)
    else {
        return Ok(ActualFieldValue::Missing);
    };
    Ok(match field {
        "counter" => ActualFieldValue::Number(relic.counter as i64),
        "used_up" => ActualFieldValue::Bool(relic.used_up),
        "amount" => ActualFieldValue::Number(relic.amount as i64),
        other => return Err(format!("unsupported relic runtime field '{other}'")),
    })
}

fn normalize_player_stat_name(stat: &str) -> Result<&'static str, String> {
    match normalize_identifier(stat).as_str() {
        "hp" | "currenthp" => Ok("hp"),
        "block" => Ok("block"),
        "energy" => Ok("energy"),
        other => Err(format!("unsupported player stat '{other}'")),
    }
}

fn normalize_monster_stat_name(stat: &str) -> Result<&'static str, String> {
    match normalize_identifier(stat).as_str() {
        "hp" | "currenthp" => Ok("hp"),
        "block" => Ok("block"),
        other => Err(format!("unsupported monster stat '{other}'")),
    }
}

fn normalize_pile_name(pile: &str) -> Result<&'static str, String> {
    match normalize_identifier(pile).as_str() {
        "hand" => Ok("hand"),
        "draw" | "drawpile" => Ok("draw_pile"),
        "discard" | "discardpile" => Ok("discard_pile"),
        "exhaust" | "exhaustpile" => Ok("exhaust_pile"),
        "limbo" => Ok("limbo"),
        other => Err(format!("unsupported pile '{other}'")),
    }
}

fn load_raw_records(path: &Path) -> Result<BTreeMap<u64, Value>, String> {
    let payload = std::fs::read_to_string(path).map_err(|err| err.to_string())?;
    let mut records = BTreeMap::new();
    for line in payload.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let root: Value = serde_json::from_str(trimmed).map_err(|err| err.to_string())?;
        let response_id = root
            .get("protocol_meta")
            .and_then(|meta| meta.get("response_id"))
            .and_then(json_u64)
            .ok_or_else(|| "raw record missing protocol_meta.response_id".to_string())?;
        records.insert(response_id, root);
    }
    Ok(records)
}

fn screen_card_to_rust_id(card: &Value) -> Option<CardId> {
    let java_id = card.get("id").and_then(Value::as_str)?;
    card_id_from_java(java_id)
}

fn json_u64(value: &Value) -> Option<u64> {
    value
        .as_u64()
        .or_else(|| value.as_i64().and_then(|number| u64::try_from(number).ok()))
}

fn json_i32(value: &Value, field: &str) -> Result<i32, String> {
    value
        .as_i64()
        .and_then(|number| i32::try_from(number).ok())
        .ok_or_else(|| format!("field '{field}' requires i32"))
}

fn json_u8(value: &Value, field: &str) -> Result<u8, String> {
    value
        .as_u64()
        .and_then(|number| u8::try_from(number).ok())
        .ok_or_else(|| format!("field '{field}' requires u8"))
}

fn json_bool(value: &Value, field: &str) -> Result<bool, String> {
    value
        .as_bool()
        .ok_or_else(|| format!("field '{field}' requires bool"))
}

fn json_option_i32(value: &Value, field: &str) -> Result<Option<i32>, String> {
    if value.is_null() {
        return Ok(None);
    }
    json_i32(value, field).map(Some)
}

fn normalize_identifier(raw: &str) -> String {
    raw.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn stable_uuid(seed: &str) -> u32 {
    let mut hash = 2_166_136_261u32;
    for byte in seed.as_bytes() {
        hash ^= u32::from(*byte);
        hash = hash.wrapping_mul(16_777_619);
    }
    hash
}

fn base_max_hp_for_class(player_class: &str) -> i32 {
    match normalize_identifier(player_class).as_str() {
        "silent" => 70,
        "defect" => 75,
        "watcher" => 72,
        _ => 80,
    }
}

fn default_room_type() -> String {
    "monster".to_string()
}

fn default_seed_hint() -> u64 {
    1
}

fn default_count() -> usize {
    1
}

fn default_occurrence() -> usize {
    1
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::testing::fixtures::author_spec::CombatAuthorSpec;
    use std::fs;

    #[test]
    fn protocol_snapshot_case_round_trips() {
        let case = CombatCase {
            id: "roundtrip".to_string(),
            domain: CombatCaseDomain::Combat,
            basis: CombatCaseBasis::ProtocolSnapshot(CombatCaseProtocolSnapshotBasis {
                combat_truth: serde_json::json!({"turn": 1, "player": {"current_hp": 80, "max_hp": 80, "block": 0, "energy": 3, "powers": []}, "monsters": [], "hand": [], "draw_pile": [], "discard_pile": [], "exhaust_pile": [], "limbo": [], "card_queue": [], "potions": []}),
                combat_observation: serde_json::json!({"player": {"current_hp": 80, "max_hp": 80, "block": 0, "energy": 3, "powers": []}, "monsters": [], "hand": [], "discard_pile": [], "exhaust_pile": [], "limbo": [], "draw_pile_count": 0}),
                relics: serde_json::json!([]),
                protocol_meta: Some(serde_json::json!({"response_id": 1, "state_frame_id": 1})),
                root_meta: CombatCaseRootMeta {
                    player_class: Some("Ironclad".to_string()),
                    ascension_level: Some(0),
                    seed_hint: Some(1),
                    screen_type: Some("NONE".to_string()),
                    screen_state: Some(serde_json::json!({})),
                },
            }),
            delta: CombatCaseDelta::default(),
            program: vec![],
            oracle: CombatCaseOracle {
                primary: CombatCaseOracleKind::Invariant,
                evidence: vec![CombatCaseOracleKind::Invariant],
                note: None,
            },
            expectations: vec![],
            provenance: CombatCaseProvenance::default(),
            tags: vec!["roundtrip".to_string()],
        };

        let payload = serde_json::to_string(&case).expect("serialize case");
        let decoded: CombatCase = serde_json::from_str(&payload).expect("deserialize case");
        assert_eq!(decoded.id, case.id);
        match decoded.basis {
            CombatCaseBasis::ProtocolSnapshot(_) => {}
            other => panic!("expected protocol_snapshot basis, got {other:?}"),
        }
    }

    #[test]
    fn compile_author_case_replays() {
        let spec: CombatAuthorSpec = serde_json::from_value(serde_json::json!({
            "name": "jaw_worm_strike",
            "player_class": "Ironclad",
            "room_type": "MonsterRoom",
            "turn": 1,
            "player": { "energy": 3 },
            "monsters": [{ "id": "JawWorm", "current_hp": 40, "intent": "ATTACK", "move_adjusted_damage": 11, "move_base_damage": 11, "move_hits": 1 }],
            "hand": ["Strike_R"],
            "draw_pile": [],
            "discard_pile": [],
            "exhaust_pile": [],
            "steps": [{ "play": { "card": 1, "target": 0 } }],
            "assertions": [{ "monster_stat": { "monster": 0, "stat": "hp", "value": 34 } }]
        }))
        .expect("author spec");

        let case = compile_combat_author_case(&spec).expect("compile case");
        assert_case(&case).expect("author case should replay");
    }

    #[test]
    fn encounter_template_case_lowers() {
        let case = CombatCase {
            id: "lagavulin_start".to_string(),
            domain: CombatCaseDomain::Combat,
            basis: CombatCaseBasis::EncounterTemplate(CombatCaseEncounterTemplateBasis {
                player_class: "Ironclad".to_string(),
                ascension_level: 0,
                encounter_id: "lagavulin".to_string(),
                room_type: "elite".to_string(),
                seed_hint: 7,
                player_current_hp: Some(80),
                player_max_hp: Some(80),
                relics: vec![],
                potions: vec![],
                master_deck: vec![
                    AuthorCardSpec::Simple("Strike_R".to_string()),
                    AuthorCardSpec::Simple("Strike_R".to_string()),
                    AuthorCardSpec::Simple("Strike_R".to_string()),
                    AuthorCardSpec::Simple("Strike_R".to_string()),
                    AuthorCardSpec::Simple("Strike_R".to_string()),
                    AuthorCardSpec::Simple("Defend_R".to_string()),
                    AuthorCardSpec::Simple("Defend_R".to_string()),
                    AuthorCardSpec::Simple("Defend_R".to_string()),
                    AuthorCardSpec::Simple("Defend_R".to_string()),
                    AuthorCardSpec::Simple("Bash".to_string()),
                ],
            }),
            delta: CombatCaseDelta::default(),
            program: vec![],
            oracle: CombatCaseOracle {
                primary: CombatCaseOracleKind::Invariant,
                evidence: vec![CombatCaseOracleKind::Invariant],
                note: None,
            },
            expectations: vec![],
            provenance: CombatCaseProvenance::default(),
            tags: vec![],
        };

        let lowered = lower_case(&case).expect("lower encounter template");
        assert_eq!(lowered.combat.meta.player_class, "Ironclad");
        assert!(!lowered.combat.entities.monsters.is_empty());
    }

    #[test]
    fn protocol_snapshot_case_reduces_to_encounter_template() {
        let spec: CombatAuthorSpec = serde_json::from_value(serde_json::json!({
            "name": "jaw_worm_reduce",
            "player_class": "Ironclad",
            "room_type": "MonsterRoom",
            "turn": 1,
            "player": { "energy": 3, "block": 0 },
            "monsters": [{ "id": "JawWorm", "current_hp": 40, "intent": "ATTACK", "move_adjusted_damage": 11, "move_base_damage": 11, "move_hits": 1 }],
            "hand": ["Strike_R", "Defend_R"],
            "draw_pile": ["Strike_R", "Defend_R", "Bash"],
            "discard_pile": [],
            "exhaust_pile": [],
            "steps": [{ "play": { "card": 1, "target": 0 } }],
            "assertions": [{ "monster_stat": { "monster": 0, "stat": "hp", "value": 34 } }]
        }))
        .expect("author spec");

        let case = compile_combat_author_case(&spec).expect("compile case");
        let protocol = match &case.basis {
            CombatCaseBasis::ProtocolSnapshot(protocol) => protocol.clone(),
            other => panic!("expected protocol snapshot basis, got {other:?}"),
        };
        let template = infer_encounter_template_basis(&protocol)
            .expect("infer template basis")
            .expect("template basis should be inferable");
        let candidate = CombatCase {
            basis: CombatCaseBasis::EncounterTemplate(template),
            delta: build_encounter_template_delta(&protocol).expect("build delta"),
            ..case.clone()
        };
        if let Err(err) = assert_case(&candidate) {
            panic!("candidate encounter_template case should replay: {err}");
        }
        let reduced = CombatCaseReducer::reduce(&case).expect("reduce case");
        match reduced.basis {
            CombatCaseBasis::EncounterTemplate(_) => {}
            other => panic!("expected encounter_template reduction, got {other:?}"),
        }
        assert_case(&reduced).expect("reduced case should replay");
    }

    #[test]
    fn protocol_snapshot_reduction_preserves_runtime_rng_and_truth_relics() {
        let protocol = CombatCaseProtocolSnapshotBasis {
            combat_truth: serde_json::json!({
                "turn": 1,
                "room_type": "MonsterRoom",
                "player": {
                    "current_hp": 85,
                    "max_hp": 87,
                    "block": 5,
                    "energy": 0,
                    "powers": []
                },
                "monsters": [{
                    "id": "SlaverBlue",
                    "current_hp": 33,
                    "max_hp": 50,
                    "block": 0,
                    "move_id": 1,
                    "powers": []
                }],
                "hand": [{
                    "id": "Strike_R",
                    "uuid": "00000000-0000-0000-0000-000000000001",
                    "upgrades": 0,
                    "cost": 1
                }],
                "draw_pile": [{
                    "id": "Bash",
                    "uuid": "00000000-0000-0000-0000-000000000002",
                    "upgrades": 0,
                    "cost": 2
                }],
                "discard_pile": [],
                "exhaust_pile": [],
                "limbo": [],
                "card_queue": [],
                "potions": [
                    { "id": "ColorlessPotion" },
                    { "id": "Potion Slot" },
                    { "id": "Potion Slot" }
                ],
                "relics": [
                    { "id": "Toy Ornithopter" }
                ],
                "colorless_combat_pool": [
                    { "id": "Madness" },
                    { "id": "Discovery" }
                ],
                "rng_state": {
                    "card_rng": {
                        "seed0": 11,
                        "seed1": 29,
                        "counter": 7
                    }
                }
            }),
            combat_observation: serde_json::json!({
                "player": {
                    "current_hp": 85,
                    "max_hp": 87,
                    "block": 5,
                    "energy": 0,
                    "powers": []
                },
                "monsters": [{
                    "id": "SlaverBlue",
                    "current_hp": 33,
                    "max_hp": 50,
                    "block": 0,
                    "powers": []
                }],
                "hand": [{
                    "id": "Strike_R",
                    "uuid": "00000000-0000-0000-0000-000000000001",
                    "upgrades": 0,
                    "cost": 1
                }],
                "discard_pile": [],
                "exhaust_pile": [],
                "limbo": [],
                "draw_pile_count": 1
            }),
            relics: serde_json::json!([]),
            protocol_meta: Some(serde_json::json!({
                "response_id": 158,
                "state_frame_id": 158
            })),
            root_meta: CombatCaseRootMeta {
                player_class: Some("IRONCLAD".to_string()),
                ascension_level: Some(0),
                seed_hint: Some(42),
                screen_type: Some("NONE".to_string()),
                screen_state: Some(serde_json::json!({})),
            },
        };

        let template = infer_encounter_template_basis(&protocol)
            .expect("infer template basis")
            .expect("template basis should be inferable");
        assert_eq!(template.encounter_id, "blue_slaver");
        assert!(template.relics.iter().any(|spec| match spec {
            AuthorRelicSpec::Simple(id) => id == "Toy Ornithopter",
            AuthorRelicSpec::Detailed(entry) => entry.id == "Toy Ornithopter",
        }));

        let delta = build_encounter_template_delta(&protocol).expect("build delta");
        assert_eq!(
            delta
                .runtime
                .as_ref()
                .and_then(|runtime| runtime.colorless_combat_pool.as_ref())
                .expect("colorless pool delta"),
            &vec!["Madness".to_string(), "Discovery".to_string()]
        );
        assert_eq!(
            delta
                .rng
                .as_ref()
                .and_then(|rng| rng.card_rng.as_ref())
                .and_then(|channel| channel.counter),
            Some(7)
        );

        let case = CombatCase {
            id: "blue_slaver_runtime_rng".to_string(),
            domain: CombatCaseDomain::Combat,
            basis: CombatCaseBasis::EncounterTemplate(template),
            delta,
            program: vec![],
            oracle: CombatCaseOracle {
                primary: CombatCaseOracleKind::Invariant,
                evidence: vec![CombatCaseOracleKind::Invariant],
                note: None,
            },
            expectations: vec![],
            provenance: CombatCaseProvenance::default(),
            tags: vec![],
        };
        let lowered = lower_case(&case).expect("lower reduced case");
        assert_eq!(
            lowered.combat.runtime.colorless_combat_pool,
            vec![CardId::Madness, CardId::Discovery]
        );
        assert_eq!(lowered.combat.rng.card_random_rng.seed0, 11);
        assert_eq!(lowered.combat.rng.card_random_rng.seed1, 29);
        assert_eq!(lowered.combat.rng.card_random_rng.counter, 7);
        assert!(lowered
            .combat
            .entities
            .player
            .relics
            .iter()
            .any(|relic| relic.id == relic_id_from_java("Toy Ornithopter").unwrap()));
    }

    #[test]
    fn encounter_template_reduction_minimizes_redundant_delta_sections() {
        let mut case = CombatCase {
            id: "redundant_delta_minimize".to_string(),
            domain: CombatCaseDomain::Combat,
            basis: CombatCaseBasis::EncounterTemplate(CombatCaseEncounterTemplateBasis {
                player_class: "IRONCLAD".to_string(),
                ascension_level: 0,
                encounter_id: "jaw_worm".to_string(),
                room_type: "MonsterRoom".to_string(),
                seed_hint: 7,
                player_current_hp: Some(80),
                player_max_hp: Some(80),
                relics: vec![AuthorRelicSpec::Simple("Burning Blood".to_string())],
                potions: vec![],
                master_deck: vec![
                    AuthorCardSpec::Simple("Strike_R".to_string()),
                    AuthorCardSpec::Simple("Strike_R".to_string()),
                    AuthorCardSpec::Simple("Strike_R".to_string()),
                    AuthorCardSpec::Simple("Strike_R".to_string()),
                    AuthorCardSpec::Simple("Strike_R".to_string()),
                    AuthorCardSpec::Simple("Defend_R".to_string()),
                    AuthorCardSpec::Simple("Defend_R".to_string()),
                    AuthorCardSpec::Simple("Defend_R".to_string()),
                    AuthorCardSpec::Simple("Defend_R".to_string()),
                    AuthorCardSpec::Simple("Bash".to_string()),
                ],
            }),
            delta: CombatCaseDelta {
                relics: vec![CombatCaseRelicDelta {
                    id: "Burning Blood".to_string(),
                    counter: None,
                    used_up: None,
                    amount: None,
                }],
                zones: Some(CombatCaseZonesDelta {
                    hand: Some(vec![CombatCaseCardEntry {
                        id: "Strike_R".to_string(),
                        uuid: Some("00000000-0000-0000-0000-000000000001".to_string()),
                        upgrades: 0,
                        cost: None,
                        misc: None,
                        count: 1,
                    }]),
                    draw_pile: Some(vec![CombatCaseCardEntry {
                        id: "Bash".to_string(),
                        uuid: Some("00000000-0000-0000-0000-000000000002".to_string()),
                        upgrades: 0,
                        cost: None,
                        misc: None,
                        count: 1,
                    }]),
                    discard_pile: None,
                    exhaust_pile: None,
                    limbo: None,
                }),
                runtime: Some(CombatCaseRuntimeDelta {
                    colorless_combat_pool: Some(vec!["Madness".to_string()]),
                }),
                rng: Some(CombatCaseRngDelta {
                    card_rng: Some(CombatCaseRngChannel {
                        seed0: Some(11),
                        seed1: Some(29),
                        counter: Some(7),
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            },
            program: vec![],
            oracle: CombatCaseOracle {
                primary: CombatCaseOracleKind::Invariant,
                evidence: vec![CombatCaseOracleKind::Invariant],
                note: None,
            },
            expectations: vec![],
            provenance: CombatCaseProvenance::default(),
            tags: vec![],
        };

        let baseline = lower_case(&case).expect("lower redundant case");
        let monster_hp = baseline.combat.entities.monsters[0].current_hp as i64;
        case.expectations.push(CombatCaseExpectation {
            check: CombatCaseCheck::MonsterStat {
                monster: 0,
                stat: "hp".to_string(),
                value: monster_hp,
            },
            response_id: None,
            frame_id: None,
            note: None,
        });

        let reduced = CombatCaseReducer::reduce(&case).expect("reduce redundant delta case");
        assert!(reduced.delta.relics.is_empty());
        assert!(reduced.delta.runtime.is_none());
        assert!(reduced.delta.rng.is_none());
        assert!(reduced.delta.zones.is_none());
        assert!(reduced
            .provenance
            .notes
            .iter()
            .any(|note| note == "minimized_encounter_template_delta"));
        assert_case(&reduced).expect("minimized case should still verify");
    }

    #[test]
    fn live_window_case_materializes_to_protocol_snapshot() {
        let nonce = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .expect("system clock before unix epoch")
            .as_nanos();
        let raw_path = std::env::temp_dir().join(format!(
            "combat_case_live_window_{}_{}.jsonl",
            std::process::id(),
            nonce
        ));
        let raw_record = serde_json::json!({
            "game_state": {
                "class": "Ironclad",
                "ascension_level": 0,
                "seed": 42,
                "screen_type": "NONE",
                "screen_state": {},
                "combat_truth": {
                    "turn": 1,
                    "player": { "current_hp": 80, "max_hp": 80, "block": 0, "powers": [] },
                    "monsters": [],
                    "hand": [],
                    "draw_pile": [],
                    "discard_pile": [],
                    "exhaust_pile": [],
                    "limbo": [],
                    "card_queue": [],
                    "potions": [],
                    "relics": []
                },
                "combat_observation": {
                    "player": { "current_hp": 80, "max_hp": 80, "block": 0, "energy": 3, "powers": [] },
                    "monsters": [],
                    "hand": [],
                    "discard_pile": [],
                    "exhaust_pile": [],
                    "limbo": [],
                    "draw_pile_count": 0
                },
                "relics": [],
                "potions": []
            },
            "protocol_meta": {
                "response_id": 10,
                "state_frame_id": 10
            }
        });
        fs::write(
            &raw_path,
            format!(
                "{}\n",
                serde_json::to_string(&raw_record).expect("serialize raw record")
            ),
        )
        .expect("write raw log");

        let case = CombatCase {
            id: "live_window".to_string(),
            domain: CombatCaseDomain::Combat,
            basis: CombatCaseBasis::LiveWindow(CombatCaseLiveWindowBasis {
                raw_path: raw_path.display().to_string(),
                debug_path: None,
                from_response_id: 10,
                to_response_id: 10,
                failure_frame: Some(10),
                run_id: Some("test".to_string()),
                target_field: Some("player.current_hp".to_string()),
            }),
            delta: CombatCaseDelta::default(),
            program: vec![],
            oracle: CombatCaseOracle {
                primary: CombatCaseOracleKind::Differential,
                evidence: vec![CombatCaseOracleKind::Differential],
                note: None,
            },
            expectations: vec![],
            provenance: CombatCaseProvenance::default(),
            tags: vec![],
        };

        let materialized = CombatCaseReducer::materialize(&case).expect("materialize live window");
        match &materialized.basis {
            CombatCaseBasis::ProtocolSnapshot(protocol) => {
                assert_eq!(protocol.root_meta.player_class.as_deref(), Some("Ironclad"));
                assert_eq!(protocol.root_meta.seed_hint, Some(42));
            }
            other => panic!("expected protocol_snapshot basis, got {other:?}"),
        }
        let lowered = lower_case(&materialized).expect("lower materialized case");
        assert_eq!(lowered.player_class.as_deref(), Some("Ironclad"));

        let _ = fs::remove_file(&raw_path);
    }
}
