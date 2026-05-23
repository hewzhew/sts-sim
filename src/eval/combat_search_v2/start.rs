use std::fs;
use std::path::Path;
use std::time::Duration;

use serde::Serialize;

use crate::ai::combat_search_v2::{
    run_combat_search_v2, CombatSearchV2Config, CombatSearchV2PotionPolicy, CombatSearchV2Report,
};
use crate::eval::artifact::ArtifactTrustLevel;
use crate::eval::combat_capture::load_combat_capture_v1;
use crate::eval::fingerprint::StateFingerprintV1;
use crate::fixtures::combat_start_spec::{compile_combat_start_spec, CombatStartSpec};
use crate::sim::combat::CombatPosition;

#[derive(Clone, Debug, Default)]
pub struct CombatSearchV2RunOptions {
    pub max_nodes: Option<usize>,
    pub max_actions_per_line: Option<usize>,
    pub max_engine_steps_per_action: Option<usize>,
    pub wall_ms: Option<u64>,
    pub potion_policy: Option<CombatSearchV2PotionPolicy>,
    pub max_potions_used: Option<u32>,
}

impl CombatSearchV2RunOptions {
    pub fn to_search_config(&self, input_label: String) -> CombatSearchV2Config {
        let defaults = CombatSearchV2Config::default();
        CombatSearchV2Config {
            max_nodes: self.max_nodes.unwrap_or(defaults.max_nodes),
            max_actions_per_line: self
                .max_actions_per_line
                .unwrap_or(defaults.max_actions_per_line),
            max_engine_steps_per_action: self
                .max_engine_steps_per_action
                .unwrap_or(defaults.max_engine_steps_per_action),
            wall_time: self.wall_ms.map(Duration::from_millis),
            input_label: Some(input_label),
            potion_policy: self.potion_policy.unwrap_or(defaults.potion_policy),
            max_potions_used: self.max_potions_used.or(defaults.max_potions_used),
        }
    }
}

#[derive(Clone)]
pub struct CombatSearchV2LoadedStart {
    pub label: String,
    pub position: CombatPosition,
    pub artifact_trust_level: Option<ArtifactTrustLevel>,
    pub fingerprints: Option<StateFingerprintV1>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2SingleRun {
    pub search_report: CombatSearchV2Report,
}

pub fn load_combat_search_v2_start(path: &Path) -> Result<CombatSearchV2LoadedStart, String> {
    let payload = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let spec: CombatStartSpec = serde_json::from_str(&payload).map_err(|err| err.to_string())?;
    let (engine, combat) = compile_combat_start_spec(&spec)?;
    Ok(CombatSearchV2LoadedStart {
        label: format!("start_spec:{}", path.display()),
        position: CombatPosition::new(engine, combat),
        artifact_trust_level: None,
        fingerprints: None,
    })
}

pub fn load_combat_search_v2_snapshot(path: &Path) -> Result<CombatSearchV2LoadedStart, String> {
    let capture = load_combat_capture_v1(path)?;
    let label = match capture.label.as_deref().filter(|label| !label.is_empty()) {
        Some(label) => format!("combat_snapshot:{}:{label}", path.display()),
        None => format!("combat_snapshot:{}", path.display()),
    };
    Ok(CombatSearchV2LoadedStart {
        label,
        position: capture.position,
        artifact_trust_level: Some(capture.trust_level),
        fingerprints: capture.fingerprints,
    })
}

pub fn run_combat_search_v2_loaded_start(
    loaded: &CombatSearchV2LoadedStart,
    options: CombatSearchV2RunOptions,
) -> CombatSearchV2SingleRun {
    CombatSearchV2SingleRun {
        search_report: run_combat_search_v2(
            &loaded.position.engine,
            &loaded.position.combat,
            options.to_search_config(loaded.label.clone()),
        ),
    }
}
