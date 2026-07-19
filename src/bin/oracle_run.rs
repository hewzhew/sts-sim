use std::path::PathBuf;

use clap::Parser;
use sts_simulator::eval::combat_case::save_combat_case;
use sts_simulator::eval::run_control::{
    run_strategic_checkpoint_probe_decomposition_v1, run_strategic_encounter_probes_v1,
    run_strategic_mechanism_probes_v1, run_strategic_probe_calibration_v1,
    strategic_encounter_probe_plan_v1, strategic_mechanism_probe_plan_v1,
    StrategicCheckpointReferenceRelationV1, StrategicEncounterProbeBudgetV1,
    StrategicEncounterProbeHpBasisV1, StrategicEncounterProbePotionUseV1, StrategicProbeFidelityV1,
    StrategicProbeShadowFidelityV1,
};
use sts_simulator::runtime::branch::{
    load_oracle_run_continuation_v1, run_oracle_run, run_oracle_run_from_continuation,
    save_oracle_run_continuation_v1, OracleRunBudget, OracleRunConfig,
};

#[derive(Debug, Parser)]
#[command(
    name = "oracle_run",
    about = "Explore bounded exact run branches until an Act-3-boss victory witness is found"
)]
struct Cli {
    #[arg(long)]
    seed: u64,

    #[arg(long, default_value_t = 0)]
    ascension: u8,

    #[arg(long, default_value_t = 2_048)]
    max_work_items: usize,

    #[arg(long)]
    wall_ms: Option<u64>,

    #[arg(long, default_value_t = 250_000)]
    hallway_nodes: usize,

    #[arg(long, default_value_t = 5_000)]
    hallway_ms: u64,

    #[arg(long, default_value_t = 750_000)]
    elite_nodes: usize,

    #[arg(long, default_value_t = 15_000)]
    elite_ms: u64,

    #[arg(long, default_value_t = 2_000_000)]
    boss_nodes: usize,

    #[arg(long, default_value_t = 30_000)]
    boss_ms: u64,

    #[arg(long, default_value_t = 50_000)]
    combat_quantum_nodes: usize,

    #[arg(long, default_value_t = 1_000)]
    combat_quantum_ms: u64,

    /// Save the first exact unresolved combat, or the selected active combat
    /// at a bounded stop, as a standalone combat case.
    #[arg(long)]
    combat_case_out: Option<PathBuf>,

    /// Continue from one exact state previously written by --continuation-out.
    #[arg(long)]
    resume: Option<PathBuf>,

    /// Save the furthest exact state and its full committed journal.
    #[arg(long)]
    continuation_out: Option<PathBuf>,

    /// Diagnose the exact run state stored in --resume against the fixed
    /// semantic encounter suite without advancing the run.
    #[arg(long)]
    strategic_probe_only: bool,

    /// Pair bounded shadow envelopes with low/high whole-encounter budgets.
    /// Requires --strategic-probe-only and remains offline calibration only.
    #[arg(long)]
    strategic_probe_calibration: bool,

    /// Run the finite-horizon offline mechanism battery without advancing the
    /// run or changing any owner decision.
    #[arg(long)]
    strategic_mechanism_probe_only: bool,

    #[arg(long, default_value_t = 25_000)]
    strategic_probe_nodes: usize,

    #[arg(long, default_value_t = 500)]
    strategic_probe_ms: u64,

    /// Normalize the diagnostic combat to max HP. This removes the current HP
    /// debt for one counterfactual sample; it does not isolate deck quality.
    #[arg(long)]
    strategic_probe_full_hp: bool,

    /// Optional earlier checkpoint used only for controlled potion/deck
    /// counterfactuals. The observed checkpoint remains --resume.
    #[arg(long)]
    strategic_probe_reference: Option<PathBuf>,

    /// Restrict the offline battery to one or more stable probe ids.
    #[arg(long)]
    strategic_probe_id: Vec<String>,
}

fn main() -> Result<(), String> {
    let cli = Cli::parse();
    if cli.strategic_probe_only && cli.strategic_mechanism_probe_only {
        return Err(
            "choose either --strategic-probe-only or --strategic-mechanism-probe-only".to_string(),
        );
    }
    if cli.strategic_probe_calibration && !cli.strategic_probe_only {
        return Err("--strategic-probe-calibration requires --strategic-probe-only".to_string());
    }
    if cli.strategic_mechanism_probe_only {
        if cli.strategic_probe_reference.is_some() || cli.strategic_probe_full_hp {
            return Err(
                "mechanism probes already normalize HP and do not accept checkpoint references"
                    .to_string(),
            );
        }
        let path = cli
            .resume
            .as_ref()
            .ok_or_else(|| "--strategic-mechanism-probe-only requires --resume".to_string())?;
        let continuation = load_oracle_run_continuation_v1(path)?;
        if continuation.seed != cli.seed || continuation.ascension != cli.ascension {
            return Err(format!(
                "probe configuration {}/A{} does not match continuation {}/A{}",
                cli.seed, cli.ascension, continuation.seed, continuation.ascension
            ));
        }
        let session = continuation.session.into_session()?;
        let mut probes = strategic_mechanism_probe_plan_v1();
        if !cli.strategic_probe_id.is_empty() {
            probes.retain(|probe| {
                cli.strategic_probe_id
                    .iter()
                    .any(|requested| requested == probe.probe_id)
            });
        }
        if probes.is_empty() {
            return Err("strategic mechanism probe selection is empty".to_string());
        }
        let report = run_strategic_mechanism_probes_v1(&session.run_state, &probes);
        println!(
            "{}",
            serde_json::to_string_pretty(&report)
                .map_err(|error| format!("failed to serialize mechanism probe: {error}"))?
        );
        return Ok(());
    }
    if cli.strategic_probe_only {
        let path = cli
            .resume
            .as_ref()
            .ok_or_else(|| "--strategic-probe-only requires --resume".to_string())?;
        let continuation = load_oracle_run_continuation_v1(path)?;
        if continuation.seed != cli.seed || continuation.ascension != cli.ascension {
            return Err(format!(
                "probe configuration {}/A{} does not match continuation {}/A{}",
                cli.seed, cli.ascension, continuation.seed, continuation.ascension
            ));
        }
        let session = continuation.session.into_session()?;
        let mut probes = strategic_encounter_probe_plan_v1(&session.run_state);
        if !cli.strategic_probe_id.is_empty() {
            probes.retain(|probe| {
                cli.strategic_probe_id
                    .iter()
                    .any(|requested| requested == probe.probe_id)
            });
        }
        if probes.is_empty() {
            return Err("strategic probe selection is empty".to_string());
        }
        let budget = StrategicEncounterProbeBudgetV1 {
            max_nodes_per_encounter: cli.strategic_probe_nodes,
            wall_ms_per_encounter: cli.strategic_probe_ms,
            hp_basis: if cli.strategic_probe_full_hp {
                StrategicEncounterProbeHpBasisV1::Full
            } else {
                StrategicEncounterProbeHpBasisV1::Current
            },
            // Checkpoint decomposition is the only current caller that asks
            // a potion counterfactual.  All paired variants receive the same
            // bounded semantic potion policy; ordinary fixed probes remain
            // potion-free so the experimental variable is explicit.
            potion_use: if cli.strategic_probe_reference.is_some() {
                StrategicEncounterProbePotionUseV1::SemanticBudgeted { max_uses: 3 }
            } else {
                StrategicEncounterProbePotionUseV1::Disabled
            },
        };
        let payload = if let Some(reference_path) = cli.strategic_probe_reference.as_ref() {
            if cli.strategic_probe_calibration {
                return Err(
                    "calibration and checkpoint decomposition are separate offline layers"
                        .to_string(),
                );
            }
            if cli.strategic_probe_full_hp {
                return Err(
                    "--strategic-probe-full-hp is redundant with checkpoint decomposition"
                        .to_string(),
                );
            }
            let reference = load_oracle_run_continuation_v1(reference_path)?;
            if reference.seed != cli.seed || reference.ascension != cli.ascension {
                return Err(format!(
                    "probe reference {}/A{} does not match requested {}/A{}",
                    reference.seed, reference.ascension, cli.seed, cli.ascension
                ));
            }
            let reference_relation = if reference.journal.entries().len()
                <= continuation.journal.entries().len()
                && reference.journal.entries()
                    == &continuation.journal.entries()[..reference.journal.entries().len()]
            {
                StrategicCheckpointReferenceRelationV1::ExactJournalAncestor
            } else {
                StrategicCheckpointReferenceRelationV1::StateOnlyCounterfactual
            };
            let reference_session = reference.session.into_session()?;
            let decomposition = run_strategic_checkpoint_probe_decomposition_v1(
                &session.run_state,
                Some(&reference_session.run_state),
                Some(reference_relation),
                &probes,
                budget,
            )?;
            serde_json::to_string_pretty(&decomposition).map_err(|error| {
                format!("failed to serialize strategic checkpoint decomposition: {error}")
            })?
        } else if cli.strategic_probe_calibration {
            let low = StrategicProbeFidelityV1 {
                max_nodes: (cli.strategic_probe_nodes / 5).max(1),
                wall_ms: (cli.strategic_probe_ms / 5).max(1),
            };
            let high = StrategicProbeFidelityV1 {
                max_nodes: cli.strategic_probe_nodes,
                wall_ms: cli.strategic_probe_ms,
            };
            let fidelities = if low == high {
                vec![high]
            } else {
                vec![low, high]
            };
            let report = run_strategic_probe_calibration_v1(
                &session.run_state,
                &probes,
                budget.hp_basis,
                &fidelities,
                &[
                    StrategicProbeShadowFidelityV1 {
                        horizon_turns: 1,
                        max_active_states_per_depth: 24,
                        max_inner_nodes_per_turn: 128,
                        max_end_states_per_turn: 16,
                        per_bucket_limit: 4,
                    },
                    StrategicProbeShadowFidelityV1 {
                        horizon_turns: 2,
                        max_active_states_per_depth: 8,
                        max_inner_nodes_per_turn: 64,
                        max_end_states_per_turn: 8,
                        per_bucket_limit: 2,
                    },
                ],
            )?;
            serde_json::to_string_pretty(&report)
                .map_err(|error| format!("failed to serialize strategic calibration: {error}"))?
        } else {
            let report = run_strategic_encounter_probes_v1(&session.run_state, &probes, budget);
            serde_json::to_string_pretty(&report)
                .map_err(|error| format!("failed to serialize strategic probe: {error}"))?
        };
        println!("{payload}");
        return Ok(());
    }
    let config = OracleRunConfig {
        seed: cli.seed,
        ascension: cli.ascension,
        budget: OracleRunBudget {
            max_work_items: cli.max_work_items,
            wall_ms: cli.wall_ms,
            hallway_nodes: cli.hallway_nodes,
            hallway_ms: cli.hallway_ms,
            elite_nodes: cli.elite_nodes,
            elite_ms: cli.elite_ms,
            boss_nodes: cli.boss_nodes,
            boss_ms: cli.boss_ms,
            combat_quantum_nodes: cli.combat_quantum_nodes,
            combat_quantum_ms: cli.combat_quantum_ms,
        },
    };
    let report = if let Some(path) = cli.resume.as_ref() {
        run_oracle_run_from_continuation(config, load_oracle_run_continuation_v1(path)?)?
    } else {
        run_oracle_run(config)?
    };
    if let Some(path) = cli.combat_case_out.as_ref() {
        let case = report
            .first_unresolved_combat_case
            .as_ref()
            .or(report.selected_active_combat_case.as_ref())
            .ok_or_else(|| {
                "oracle run did not retain an unresolved or active combat to export".to_string()
            })?;
        save_combat_case(path, case)?;
    }
    if let Some(path) = cli.continuation_out.as_ref() {
        save_oracle_run_continuation_v1(path, &report.continuation)?;
    }
    println!(
        "{}",
        serde_json::to_string_pretty(&report)
            .map_err(|error| format!("failed to serialize oracle report: {error}"))?
    );
    Ok(())
}
