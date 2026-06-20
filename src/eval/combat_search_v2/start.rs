use std::fs;
use std::path::Path;
use std::time::Duration;

use serde::Serialize;

use crate::ai::combat_search_v2::{
    high_stakes_semantic_potion_budget, run_combat_search_v2, CombatSearchV2Config,
    CombatSearchV2FrontierPolicy, CombatSearchV2PotionPolicy, CombatSearchV2Report,
    CombatSearchV2RolloutPolicy, CombatSearchV2TurnPlanPolicy,
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
    pub stop_on_win_hp_loss_at_most: Option<u32>,
    pub potion_policy: Option<CombatSearchV2PotionPolicy>,
    pub max_potions_used: Option<u32>,
    pub high_stakes_semantic_potions: bool,
    pub rollout_policy: Option<CombatSearchV2RolloutPolicy>,
    pub rollout_max_evaluations: Option<usize>,
    pub rollout_max_actions: Option<usize>,
    pub rollout_beam_width: Option<usize>,
    pub turn_plan_policy: Option<CombatSearchV2TurnPlanPolicy>,
    pub frontier_policy: Option<CombatSearchV2FrontierPolicy>,
    pub turn_plan_probe_max_inner_nodes: Option<usize>,
    pub turn_plan_probe_max_end_states: Option<usize>,
    pub turn_plan_probe_per_bucket_limit: Option<usize>,
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
            stop_on_win_hp_loss_at_most: self
                .stop_on_win_hp_loss_at_most
                .or(defaults.stop_on_win_hp_loss_at_most),
            input_label: Some(input_label),
            potion_policy: self.potion_policy.unwrap_or(defaults.potion_policy),
            max_potions_used: self.max_potions_used.or(defaults.max_potions_used),
            rollout_policy: self.rollout_policy.unwrap_or(defaults.rollout_policy),
            child_rollout_policy: defaults.child_rollout_policy,
            rollout_max_evaluations: self
                .rollout_max_evaluations
                .unwrap_or(defaults.rollout_max_evaluations),
            rollout_max_actions: self
                .rollout_max_actions
                .unwrap_or(defaults.rollout_max_actions),
            rollout_beam_width: self
                .rollout_beam_width
                .unwrap_or(defaults.rollout_beam_width),
            turn_plan_policy: self.turn_plan_policy.unwrap_or(defaults.turn_plan_policy),
            frontier_policy: self.frontier_policy.unwrap_or(defaults.frontier_policy),
            turn_plan_probe_max_inner_nodes: self
                .turn_plan_probe_max_inner_nodes
                .or(defaults.turn_plan_probe_max_inner_nodes),
            turn_plan_probe_max_end_states: self
                .turn_plan_probe_max_end_states
                .or(defaults.turn_plan_probe_max_end_states),
            turn_plan_probe_per_bucket_limit: self
                .turn_plan_probe_per_bucket_limit
                .or(defaults.turn_plan_probe_per_bucket_limit),
        }
    }

    pub fn to_search_config_for_position(
        &self,
        input_label: String,
        position: &CombatPosition,
    ) -> CombatSearchV2Config {
        let mut config = self.to_search_config(input_label);
        if self.high_stakes_semantic_potions && self.potion_policy.is_none() {
            if let Some(potion_budget) = high_stakes_semantic_potion_budget(&position.combat) {
                config.potion_policy = CombatSearchV2PotionPolicy::SemanticBudgeted;
                if self.max_potions_used.is_none() {
                    config.max_potions_used = Some(potion_budget);
                }
            }
        }
        config
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
            options.to_search_config_for_position(loaded.label.clone(), &loaded.position),
        ),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ai::combat_search_v2::CombatSearchV2PotionPolicy;

    fn combat_position_with_flags(is_boss: bool, is_elite: bool) -> CombatPosition {
        let mut combat = crate::test_support::blank_test_combat();
        combat.meta.is_boss_fight = is_boss;
        combat.meta.is_elite_fight = is_elite;
        CombatPosition::new(crate::state::core::EngineState::CombatPlayerTurn, combat)
    }

    #[test]
    fn run_options_auto_high_stakes_potions_enable_boss_semantic_budget() {
        let options = CombatSearchV2RunOptions {
            high_stakes_semantic_potions: true,
            ..CombatSearchV2RunOptions::default()
        };

        let config = options.to_search_config_for_position(
            "test".to_string(),
            &combat_position_with_flags(true, false),
        );

        assert_eq!(
            config.potion_policy,
            CombatSearchV2PotionPolicy::SemanticBudgeted
        );
        assert_eq!(config.max_potions_used, Some(2));
    }

    #[test]
    fn run_options_auto_high_stakes_potions_enable_elite_single_budget() {
        let options = CombatSearchV2RunOptions {
            high_stakes_semantic_potions: true,
            ..CombatSearchV2RunOptions::default()
        };

        let config = options.to_search_config_for_position(
            "test".to_string(),
            &combat_position_with_flags(false, true),
        );

        assert_eq!(
            config.potion_policy,
            CombatSearchV2PotionPolicy::SemanticBudgeted
        );
        assert_eq!(config.max_potions_used, Some(1));
    }

    #[test]
    fn run_options_auto_high_stakes_potions_leave_ordinary_combat_default() {
        let options = CombatSearchV2RunOptions {
            high_stakes_semantic_potions: true,
            ..CombatSearchV2RunOptions::default()
        };

        let config = options.to_search_config_for_position(
            "test".to_string(),
            &combat_position_with_flags(false, false),
        );

        assert_eq!(
            config.potion_policy,
            CombatSearchV2Config::default().potion_policy
        );
        assert_eq!(config.max_potions_used, None);
    }

    #[test]
    fn run_options_auto_high_stakes_potions_respect_explicit_policy() {
        let options = CombatSearchV2RunOptions {
            high_stakes_semantic_potions: true,
            potion_policy: Some(CombatSearchV2PotionPolicy::Never),
            max_potions_used: Some(0),
            ..CombatSearchV2RunOptions::default()
        };

        let config = options.to_search_config_for_position(
            "test".to_string(),
            &combat_position_with_flags(true, false),
        );

        assert_eq!(config.potion_policy, CombatSearchV2PotionPolicy::Never);
        assert_eq!(config.max_potions_used, Some(0));
    }

    #[test]
    fn run_options_accept_complete_win_hp_loss_gate() {
        let options = CombatSearchV2RunOptions {
            stop_on_win_hp_loss_at_most: Some(8),
            ..CombatSearchV2RunOptions::default()
        };

        let config = options.to_search_config_for_position(
            "test".to_string(),
            &combat_position_with_flags(false, false),
        );

        assert_eq!(config.stop_on_win_hp_loss_at_most, Some(8));
    }
}
