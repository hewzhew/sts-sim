use std::path::PathBuf;
use std::time::{Duration, Instant};

use clap::Args;
use serde_json::json;
use sts_oracle_runtime::ai::combat_search_v2::{
    CombatSearchV2PotionPolicy, CombatSearchV2RolloutPolicy,
};
use sts_oracle_runtime::eval::combat_case::load_combat_case;
use sts_oracle_runtime::eval::combat_search_v2::{
    run_combat_root_proposal_probe_v1, CombatRootProposalProbeV1Report, CombatSearchV2LoadedStart,
    CombatSearchV2RunOptions,
};
use sts_oracle_runtime::sim::combat::{CombatStepLimits, CombatStepper, EngineCombatStepper};

use super::exact_turn_corridor::load as load_exact_turn_corridor;
use super::print_json;

#[derive(Debug, Args)]
pub(super) struct V2CapabilityAuditArgs {
    #[arg(long)]
    case: PathBuf,
    /// Optional verified witness used only to identify the expected first
    /// turn successor in both runs.
    #[arg(long)]
    corridor_actions: Option<PathBuf>,
    #[arg(long, default_value_t = 250_000)]
    max_nodes: usize,
    #[arg(long, default_value_t = 5_000)]
    wall_ms: u64,
    #[arg(long, default_value_t = 1_024)]
    quantum_nodes: usize,
    #[arg(long, default_value_t = 250)]
    max_engine_steps_per_transition: usize,
    /// Maximum atomic inputs allowed in the standalone deterministic
    /// rollout proposal. Exposed so production proposal bounds can be
    /// reproduced exactly.
    #[arg(long, default_value_t = 80)]
    root_rollout_max_actions: usize,
    /// Save the exact replayable winner found by the no-rollout control.
    /// The compact audit never embeds action arrays in its JSON report.
    #[arg(long)]
    export_without_rollout_witness_actions: Option<PathBuf>,
}

pub(super) fn run(args: V2CapabilityAuditArgs) -> Result<(), String> {
    let V2CapabilityAuditArgs {
        case,
        corridor_actions,
        max_nodes,
        wall_ms,
        quantum_nodes,
        max_engine_steps_per_transition,
        root_rollout_max_actions,
        export_without_rollout_witness_actions,
    } = args;
    let loaded_case = load_combat_case(&case)?;
    let expected_first_turn_successor = corridor_actions
        .as_ref()
        .map(|actions| {
            load_exact_turn_corridor(
                &case,
                std::slice::from_ref(actions),
                max_engine_steps_per_transition,
            )
        })
        .transpose()?
        .and_then(|corridor| corridor.positions_by_rank.get(1).cloned())
        .map(|position| {
            sts_oracle_runtime::ai::combat_state_key::combat_exact_state_hash_v2(
                &position.engine,
                &position.combat,
            )
        });
    let loaded = CombatSearchV2LoadedStart {
        label: format!("oracle_lab:{}", case.display()),
        position: loaded_case.position,
        artifact_trust_level: None,
        fingerprints: None,
    };
    let run = |rollout_policy| {
        run_combat_root_proposal_probe_v1(
            &loaded,
            CombatSearchV2RunOptions {
                max_nodes: Some(max_nodes),
                max_engine_steps_per_action: Some(max_engine_steps_per_transition),
                wall_ms: Some(wall_ms),
                potion_policy: Some(CombatSearchV2PotionPolicy::Never),
                max_potions_used: Some(0),
                rollout_policy: Some(rollout_policy),
                ..CombatSearchV2RunOptions::default()
            },
            quantum_nodes,
        )
    };
    let baseline = run(CombatSearchV2RolloutPolicy::EnemyMechanicsAdaptiveNoPotion)?;
    let without_rollout = run(CombatSearchV2RolloutPolicy::Disabled)?;
    if let (Some(path), Some(actions)) = (
        export_without_rollout_witness_actions.as_ref(),
        without_rollout.final_best_actions.as_ref(),
    ) {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|error| error.to_string())?;
        }
        std::fs::write(
            path,
            serde_json::to_vec_pretty(actions).map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())?;
    }
    let root_rollout_started = Instant::now();
    let root_rollout = sts_oracle_runtime::ai::combat_search_v2::oracle_rollout_witness_proposal_v1(
        &loaded.position,
        root_rollout_max_actions,
        Instant::now().checked_add(Duration::from_millis(wall_ms)),
    );
    let root_rollout_report = root_rollout.map(|proposal| {
        let stepper = EngineCombatStepper;
        let mut position = loaded.position.clone();
        let mut replay_valid = true;
        for input in &proposal.actions {
            if stepper.choice_for_legal_input(&position, input).is_none() {
                replay_valid = false;
                break;
            }
            let step = stepper.apply_to_stable(
                &position,
                input.clone(),
                CombatStepLimits {
                    max_engine_steps: max_engine_steps_per_transition,
                    deadline: None,
                },
            );
            if step.truncated || step.timed_out {
                replay_valid = false;
                break;
            }
            position = step.position;
        }
        json!({
            "elapsed_ms": root_rollout_started.elapsed().as_millis(),
            "action_count": proposal.actions.len(),
            "final_hp_hint": proposal.final_hp_hint,
            "replay_valid": replay_valid,
            "replay_terminal": format!("{:?}", stepper.terminal(&position)),
            "replay_final_hp": position.combat.entities.player.current_hp,
        })
    });
    let compact = |report: &CombatRootProposalProbeV1Report| {
        let expected_observation = expected_first_turn_successor.as_ref().and_then(|expected| {
            report
                .proposals
                .iter()
                .find(|proposal| proposal.successor_exact_state_hash == *expected)
        });
        json!({
            "rollout_policy": report.config.rollout_policy,
            "proposal_count": report.proposals.len(),
            "expected_first_turn_successor_seen": expected_observation.is_some(),
            "expected_first_turn_successor": expected_observation,
            "summary": report.summary,
        })
    };
    print_json(&json!({
        "schema_name": "OracleV2CapabilityAuditV1",
        "schema_version": 1,
        "authority": "diagnostic_only_no_production_seeding",
        "case": case,
        "expected_first_turn_successor_hash": expected_first_turn_successor,
        "root_rollout": root_rollout_report,
        "baseline": compact(&baseline),
        "without_rollout": compact(&without_rollout),
        "exported_without_rollout_witness_actions":
            without_rollout.final_best_actions.is_some()
                .then_some(export_without_rollout_witness_actions.as_ref())
                .flatten(),
    }))
}
