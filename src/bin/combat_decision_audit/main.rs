use std::path::PathBuf;
use std::sync::Arc;

use clap::{Args as ClapArgs, Parser, Subcommand, ValueEnum};
use serde::{Deserialize, Serialize};

use sts_simulator::bot::branch_family_for_card;
use sts_simulator::bot::search::{
    audit_fixture, build_fixture_from_reconstructed_step, diagnose_root_search_with_depth_and_mode,
    diagnose_root_search_with_depth_and_mode_and_root_prior, extract_preference_samples,
    load_fixture_path, render_text_report, write_fixture_path, CombatPreferenceSample,
    DecisionAuditConfig, DecisionAuditEngineState, LookupRootPriorProvider, RootPriorConfig,
    RootPriorQueryKey, SearchEquivalenceMode, SearchProfileBreakdown, TrajectoryOutcomeKind,
};
use sts_simulator::runtime::combat::CombatCard;
use sts_simulator::diff::replay::{
    derive_combat_replay_view, find_combat_step_index_by_before_frame_id,
    load_live_session_replay_path, mapped_command_to_input, reconstruct_combat_replay_step,
    CombatReplayStepStatus,
};
use sts_simulator::diff::state_sync::build_combat_state;
use sts_simulator::state::core::ClientInput;
use sts_simulator::state::EngineState;

#[derive(Parser, Debug)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

#[derive(Clone, Copy, Debug, ValueEnum)]
enum EquivalenceModeArg {
    Off,
    Safe,
    Experimental,
}

impl From<EquivalenceModeArg> for SearchEquivalenceMode {
    fn from(value: EquivalenceModeArg) -> Self {
        match value {
            EquivalenceModeArg::Off => SearchEquivalenceMode::Off,
            EquivalenceModeArg::Safe => SearchEquivalenceMode::Safe,
            EquivalenceModeArg::Experimental => SearchEquivalenceMode::Experimental,
        }
    }
}

#[derive(Subcommand, Debug)]
enum Command {
    AuditFrame {
        #[arg(long)]
        raw: PathBuf,
        #[arg(long)]
        frame: u64,
        #[arg(long)]
        json_out: Option<PathBuf>,
        #[arg(long, default_value_t = 4)]
        decision_depth: usize,
        #[arg(long, default_value_t = 3)]
        top_k: usize,
        #[arg(long, default_value_t = 6)]
        branch_cap: usize,
        #[arg(long)]
        quiet: bool,
    },
    AuditFrameBatch {
        #[arg(long)]
        raw: PathBuf,
        #[arg(long, value_delimiter = ',')]
        frames: Vec<u64>,
        #[arg(long)]
        json_out: PathBuf,
        #[arg(long, default_value_t = 4)]
        decision_depth: usize,
        #[arg(long, default_value_t = 3)]
        top_k: usize,
        #[arg(long, default_value_t = 6)]
        branch_cap: usize,
        #[arg(long)]
        quiet: bool,
    },
    AuditFixture {
        #[arg(long)]
        fixture: PathBuf,
        #[arg(long)]
        json_out: Option<PathBuf>,
        #[arg(long, default_value_t = 4)]
        decision_depth: usize,
        #[arg(long, default_value_t = 3)]
        top_k: usize,
        #[arg(long, default_value_t = 6)]
        branch_cap: usize,
        #[command(flatten)]
        root_prior: RootPriorCommandArgs,
    },
    ExtractFixture {
        #[arg(long)]
        raw: PathBuf,
        #[arg(long)]
        frame: u64,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        name: Option<String>,
    },
    ExportPreferences {
        #[arg(long)]
        raw: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        summary_out: Option<PathBuf>,
        #[arg(long, default_value_t = 4)]
        decision_depth: usize,
        #[arg(long, default_value_t = 3)]
        top_k: usize,
        #[arg(long, default_value_t = 6)]
        branch_cap: usize,
        #[arg(long, default_value_t = 8)]
        min_incoming: i32,
        #[arg(long, default_value_t = 0.55)]
        max_hp_ratio: f32,
        #[arg(long, default_value_t = 64)]
        limit: usize,
    },
    ExportPreferenceSeedSet {
        #[arg(long)]
        raw: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        summary_out: Option<PathBuf>,
        #[arg(long, value_delimiter = ',')]
        frames: Vec<u64>,
        #[arg(long, default_value_t = 4)]
        decision_depth: usize,
        #[arg(long, default_value_t = 3)]
        top_k: usize,
        #[arg(long, default_value_t = 6)]
        branch_cap: usize,
    },
    SummarizePreferences {
        #[arg(long = "in", value_delimiter = ',', required = true)]
        inputs: Vec<PathBuf>,
        #[arg(long)]
        json_out: Option<PathBuf>,
        #[arg(long, default_value_t = 5)]
        top_examples: usize,
    },
    DiagnoseSearchFrame {
        #[arg(long)]
        raw: PathBuf,
        #[arg(long)]
        frame: u64,
        #[arg(long, default_value_t = 5)]
        depth_limit: u32,
        #[arg(long, default_value_t = 5)]
        top_k: usize,
        #[arg(long, value_enum, default_value_t = EquivalenceModeArg::Safe)]
        equivalence_mode: EquivalenceModeArg,
        #[arg(long)]
        emit_profile_json: Option<PathBuf>,
        #[command(flatten)]
        root_prior: RootPriorCommandArgs,
    },
    ExportSearchBaseline {
        #[arg(long)]
        raw: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, value_delimiter = ',')]
        frames: Vec<u64>,
        #[arg(long, default_value_t = 5)]
        depth_limit: u32,
        #[arg(long, default_value_t = 3)]
        top_k: usize,
        #[arg(long, value_enum, default_value_t = EquivalenceModeArg::Safe)]
        equivalence_mode: EquivalenceModeArg,
        #[command(flatten)]
        root_prior: RootPriorCommandArgs,
    },
    AuditRecentLiveSession {
        #[arg(long)]
        raw: Option<PathBuf>,
        #[arg(long)]
        suspects: Option<PathBuf>,
        #[arg(long, default_value_t = 5)]
        depth_limit: u32,
        #[arg(long, default_value_t = 3)]
        top_k: usize,
        #[arg(long, default_value_t = 8)]
        limit: usize,
        #[arg(long, value_enum, default_value_t = EquivalenceModeArg::Safe)]
        equivalence_mode: EquivalenceModeArg,
    },
}

#[derive(ClapArgs, Debug, Clone, Default)]
struct RootPriorCommandArgs {
    #[arg(long)]
    q_local_prior: Option<PathBuf>,
    #[arg(long, default_value_t = 1.0)]
    q_local_prior_weight: f32,
    #[arg(long)]
    q_local_shadow: bool,
    #[arg(long)]
    q_local_prior_spec_name: Option<String>,
    #[arg(long)]
    q_local_prior_episode_id: Option<usize>,
    #[arg(long)]
    q_local_prior_step_index: Option<usize>,
    #[arg(long)]
    q_local_prior_source_path: Option<String>,
    #[arg(long)]
    q_local_prior_frame: Option<u64>,
}

#[derive(Debug, Default, Serialize)]
struct PreferenceExportSummary {
    raw_path: String,
    considered_steps: usize,
    candidate_steps: usize,
    audited_steps: usize,
    exported_samples: usize,
    frames_with_preferences: usize,
    preference_kind_counts: std::collections::BTreeMap<String, usize>,
}

#[derive(Debug, Default, Serialize)]
struct PreferenceSeedSetSummary {
    raw_path: String,
    requested_frames: Vec<u64>,
    exported_frame_ids: Vec<u64>,
    missing_frames: Vec<u64>,
    audited_frames: usize,
    exported_samples: usize,
    preference_kind_counts: std::collections::BTreeMap<String, usize>,
}

#[derive(Debug, Default, Serialize)]
struct PreferenceMotifSummary {
    input_paths: Vec<String>,
    total_samples: usize,
    unique_frames: usize,
    preference_kind_counts: std::collections::BTreeMap<String, usize>,
    chosen_tag_counts: std::collections::BTreeMap<String, usize>,
    preferred_tag_counts: std::collections::BTreeMap<String, usize>,
    motif_counts: std::collections::BTreeMap<String, usize>,
    chosen_action_family_counts: std::collections::BTreeMap<String, usize>,
    preferred_action_family_counts: std::collections::BTreeMap<String, usize>,
    top_action_pairs: Vec<PreferenceActionPairCount>,
    top_examples: Vec<PreferenceMotifExample>,
}

#[derive(Debug, Serialize)]
struct PreferenceActionPairCount {
    chosen_action: String,
    preferred_action: String,
    count: usize,
}

#[derive(Debug, Clone, Serialize)]
struct PreferenceMotifExample {
    motif: String,
    before_frame_id: Option<u64>,
    chosen_action: String,
    preferred_action: String,
    preference_kind: String,
    score_gap: i32,
}

#[derive(Debug, Serialize)]
struct SearchBaselineRecord {
    frame: u64,
    source_path: String,
    depth_limit: u32,
    max_depth: usize,
    root_width: usize,
    branch_width: usize,
    max_engine_steps: usize,
    equivalence_mode: String,
    legal_moves: usize,
    reduced_legal_moves: usize,
    simulations: u32,
    elapsed_ms: u128,
    chosen_move: String,
    root_prior_enabled: bool,
    root_prior_key: Option<String>,
    root_prior_weight: f32,
    root_prior_hits: usize,
    root_prior_reordered: bool,
    profile: SearchProfileBreakdown,
    top_moves: Vec<SearchBaselineMove>,
}

#[derive(Debug, Serialize)]
struct SearchBaselineMove {
    rank: usize,
    move_text: String,
    avg_score: f32,
    visits: u32,
    cluster_size: usize,
    base_order_score: f32,
    order_score: f32,
    root_prior_score: f32,
    root_prior_hit: bool,
    leaf_score: f32,
    policy_bonus: f32,
    sequence_bonus: f32,
    sequence_frontload_bonus: f32,
    sequence_defer_bonus: f32,
    sequence_branch_bonus: f32,
    sequence_downside_penalty: f32,
    survival_window_delta: f32,
    exhaust_evidence_delta: f32,
    realized_exhaust_block: i32,
    realized_exhaust_draw: i32,
    branch_family: Option<String>,
}

#[derive(Debug, Serialize)]
struct AuditFixtureShadowOutput {
    report: sts_simulator::bot::search::DecisionAuditReport,
    search_baseline: SearchBaselineRecord,
}

#[derive(Debug, Serialize)]
struct AuditFrameBatchReport {
    raw_path: String,
    decision_depth: usize,
    top_k: usize,
    branch_cap: usize,
    results: Vec<AuditFrameBatchItem>,
}

#[derive(Debug, Serialize)]
struct AuditFrameBatchItem {
    frame: u64,
    status: String,
    report: Option<sts_simulator::bot::search::DecisionAuditReport>,
    error: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
struct LiveCombatSuspectRecord {
    frame_count: u64,
    response_id: Option<i64>,
    state_frame_id: Option<i64>,
    chosen_move: String,
    heuristic_move: String,
    search_move: String,
    top_gap: Option<f32>,
    #[serde(default)]
    sequence_bonus: f32,
    #[serde(default)]
    sequence_frontload_bonus: f32,
    #[serde(default)]
    sequence_defer_bonus: f32,
    #[serde(default)]
    sequence_branch_bonus: f32,
    #[serde(default)]
    sequence_downside_penalty: f32,
    #[serde(default)]
    survival_window_delta: f32,
    #[serde(default)]
    exhaust_evidence_delta: f32,
    #[serde(default)]
    realized_exhaust_block: i32,
    #[serde(default)]
    realized_exhaust_draw: i32,
    #[serde(default)]
    branch_family: Option<String>,
    #[serde(default)]
    sequencing_rationale_key: Option<String>,
    #[serde(default)]
    branch_rationale_key: Option<String>,
    #[serde(default)]
    downside_rationale_key: Option<String>,
    heuristic_search_gap: bool,
    large_sequence_bonus: bool,
    tight_root_gap: bool,
    reasons: Vec<String>,
}

#[derive(Debug, Clone)]
struct RecentAuditEntry {
    frame: u64,
    priority: i32,
    reasons: Vec<String>,
    suspect: Option<LiveCombatSuspectRecord>,
}

fn recent_suspect_priority(suspect: &LiveCombatSuspectRecord) -> i32 {
    let mut priority = 0;
    if suspect.heuristic_search_gap {
        priority += 4;
    }
    if suspect.large_sequence_bonus {
        priority += 3;
    }
    if suspect.sequence_downside_penalty.abs() >= 8_000.0 {
        priority += 4;
    } else if suspect.sequence_downside_penalty.abs() >= 3_000.0 {
        priority += 2;
    }
    if suspect.sequence_branch_bonus.abs() >= 8_000.0 && suspect.heuristic_search_gap {
        priority += 3;
    }
    if suspect.sequence_bonus.abs() < 1_000.0
        && (suspect.sequence_frontload_bonus.abs()
            + suspect.sequence_defer_bonus.abs()
            + suspect.sequence_branch_bonus.abs()
            + suspect.sequence_downside_penalty.abs())
            < 2_000.0
    {
        priority -= 3;
    }
    if let Some(gap) = suspect.top_gap {
        if gap >= 1_000.0 {
            priority += 4;
        } else if gap >= 100.0 {
            priority += 2;
        } else if gap <= 1.0 {
            priority -= 2;
        }
    }
    if suspect.tight_root_gap {
        priority -= 1;
    }
    if suspect.reasons.len() == 1 && suspect.tight_root_gap {
        priority -= 5;
    }
    priority
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = Args::parse();
    match args.command {
        Command::AuditFrame {
            raw,
            frame,
            json_out,
            decision_depth,
            top_k,
            branch_cap,
            quiet,
        } => {
            let report = audit_frame_report(
                &raw,
                frame,
                DecisionAuditConfig {
                    decision_depth,
                    top_k,
                    branch_cap,
                },
            )?;
            if !quiet {
                println!("{}", render_text_report(&report));
            }
            if let Some(path) = json_out {
                write_json(&report, &path)?;
            }
        }
        Command::AuditFrameBatch {
            raw,
            frames,
            json_out,
            decision_depth,
            top_k,
            branch_cap,
            quiet: _,
        } => {
            let config = DecisionAuditConfig {
                decision_depth,
                top_k,
                branch_cap,
            };
            let results = frames
                .into_iter()
                .map(|frame| match audit_frame_report(&raw, frame, config) {
                    Ok(report) => AuditFrameBatchItem {
                        frame,
                        status: "ok".to_string(),
                        report: Some(report),
                        error: None,
                    },
                    Err(error) => AuditFrameBatchItem {
                        frame,
                        status: "error".to_string(),
                        report: None,
                        error: Some(error),
                    },
                })
                .collect::<Vec<_>>();
            let batch = AuditFrameBatchReport {
                raw_path: raw.display().to_string(),
                decision_depth,
                top_k,
                branch_cap,
                results,
            };
            write_json(&batch, &json_out)?;
        }
        Command::AuditFixture {
            fixture,
            json_out,
            decision_depth,
            top_k,
            branch_cap,
            root_prior,
        } => {
            let fixture = load_fixture_path(&fixture)?;
            let report = audit_fixture(
                &fixture,
                DecisionAuditConfig {
                    decision_depth,
                    top_k,
                    branch_cap,
                },
            )?;
            println!("{}", render_text_report(&report));
            let root_prior_config =
                load_root_prior_config(&root_prior, default_root_prior_key_for_fixture(&fixture)?)?;
            let search_baseline = root_prior_config
                .as_ref()
                .map(|config| {
                    build_search_baseline_record_from_fixture(
                        &fixture,
                        decision_depth as u32,
                        top_k,
                        SearchEquivalenceMode::Safe,
                        Some(config),
                    )
                })
                .transpose()?;
            if let Some(search_baseline) = &search_baseline {
                println!();
                println!("{}", render_search_baseline_record(search_baseline));
            }
            if let Some(path) = json_out {
                if let Some(search_baseline) = search_baseline {
                    write_json(
                        &AuditFixtureShadowOutput {
                            report,
                            search_baseline,
                        },
                        &path,
                    )?;
                } else {
                    write_json(&report, &path)?;
                }
            }
        }
        Command::ExtractFixture {
            raw,
            frame,
            out,
            name,
        } => {
            let fixture = fixture_from_raw_frame(&raw, frame, name)?;
            write_fixture_path(&fixture, &out)?;
            println!("wrote fixture: {}", out.display());
        }
        Command::ExportPreferences {
            raw,
            out,
            summary_out,
            decision_depth,
            top_k,
            branch_cap,
            min_incoming,
            max_hp_ratio,
            limit,
        } => {
            let summary = export_preferences_from_raw(
                &raw,
                &out,
                DecisionAuditConfig {
                    decision_depth,
                    top_k,
                    branch_cap,
                },
                min_incoming,
                max_hp_ratio,
                limit,
            )?;
            println!(
                "exported {} preference samples from {} audited frames to {}",
                summary.exported_samples,
                summary.audited_steps,
                out.display()
            );
            if let Some(path) = summary_out {
                write_json(&summary, &path)?;
            }
        }
        Command::ExportPreferenceSeedSet {
            raw,
            out,
            summary_out,
            frames,
            decision_depth,
            top_k,
            branch_cap,
        } => {
            let summary = export_preference_seed_set(
                &raw,
                &out,
                &frames,
                DecisionAuditConfig {
                    decision_depth,
                    top_k,
                    branch_cap,
                },
            )?;
            println!(
                "exported {} preference samples from {} requested frames to {}",
                summary.exported_samples,
                summary.requested_frames.len(),
                out.display()
            );
            if let Some(path) = summary_out {
                write_json(&summary, &path)?;
            }
        }
        Command::SummarizePreferences {
            inputs,
            json_out,
            top_examples,
        } => {
            let summary = summarize_preferences(&inputs, top_examples)?;
            println!("{}", render_preference_summary(&summary));
            if let Some(path) = json_out {
                write_json(&summary, &path)?;
            }
        }
        Command::DiagnoseSearchFrame {
            raw,
            frame,
            depth_limit,
            top_k,
            equivalence_mode,
            emit_profile_json,
            root_prior,
        } => {
            let root_prior_config = load_root_prior_config(
                &root_prior,
                Some(RootPriorQueryKey::ReplayFrame {
                    source_path: raw.display().to_string(),
                    frame,
                }),
            )?;
            let baseline = build_search_baseline_record(
                &raw,
                frame,
                depth_limit,
                top_k,
                equivalence_mode.into(),
                root_prior_config.as_ref(),
            )?;
            println!("{}", render_search_baseline_record(&baseline));
            if let Some(path) = emit_profile_json {
                write_json(&baseline, &path)?;
            }
        }
        Command::ExportSearchBaseline {
            raw,
            out,
            frames,
            depth_limit,
            top_k,
            equivalence_mode,
            root_prior,
        } => {
            let provider = load_root_prior_provider(root_prior.q_local_prior.as_ref())?;
            let mut records = Vec::new();
            for frame in &frames {
                let root_prior_config = build_root_prior_config(
                    &root_prior,
                    Some(RootPriorQueryKey::ReplayFrame {
                        source_path: raw.display().to_string(),
                        frame: *frame,
                    }),
                    provider.clone(),
                )?;
                records.push(build_search_baseline_record(
                    &raw,
                    *frame,
                    depth_limit,
                    top_k,
                    equivalence_mode.into(),
                    root_prior_config.as_ref(),
                )?);
            }
            write_json(&records, &out)?;
            println!(
                "exported {} search baseline records to {}",
                records.len(),
                out.display()
            );
        }
        Command::AuditRecentLiveSession {
            raw,
            suspects,
            depth_limit,
            top_k,
            limit,
            equivalence_mode,
        } => {
            let rendered = audit_recent_live_session(
                raw,
                suspects,
                depth_limit,
                top_k,
                limit,
                equivalence_mode.into(),
            )?;
            println!("{rendered}");
        }
    }
    Ok(())
}

fn fixture_from_raw_frame(
    raw: &PathBuf,
    frame: u64,
    explicit_name: Option<String>,
) -> Result<sts_simulator::bot::search::DecisionAuditFixture, String> {
    let replay = load_live_session_replay_path(raw)?;
    let view = derive_combat_replay_view(&replay);
    let step_index = find_combat_step_index_by_before_frame_id(&view, frame)
        .ok_or_else(|| format!("no executable combat step found for before frame_id={frame}"))?;
    let reconstructed = reconstruct_combat_replay_step(&view, step_index)?;
    let name = explicit_name.unwrap_or_else(|| format!("decision_audit_frame_{frame}"));
    build_fixture_from_reconstructed_step(&reconstructed, replay.source_path.clone(), name)
}

fn audit_frame_report(
    raw: &PathBuf,
    frame: u64,
    config: DecisionAuditConfig,
) -> Result<sts_simulator::bot::search::DecisionAuditReport, String> {
    let fixture = fixture_from_raw_frame(raw, frame, None)?;
    audit_fixture(&fixture, config)
}

fn write_json<T: serde::Serialize>(value: &T, path: &PathBuf) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create audit output directory '{}': {err}",
                parent.display()
            )
        })?;
    }
    let text = serde_json::to_string_pretty(value)
        .map_err(|err| format!("failed to serialize report: {err}"))?;
    std::fs::write(path, text)
        .map_err(|err| format!("failed to write audit output '{}': {err}", path.display()))
}

fn export_preferences_from_raw(
    raw: &PathBuf,
    out: &PathBuf,
    config: DecisionAuditConfig,
    min_incoming: i32,
    max_hp_ratio: f32,
    limit: usize,
) -> Result<PreferenceExportSummary, String> {
    let replay = load_live_session_replay_path(raw)?;
    let view = derive_combat_replay_view(&replay);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create combat preference output directory '{}': {err}",
                parent.display()
            )
        })?;
    }
    let mut writer = std::io::BufWriter::new(std::fs::File::create(out).map_err(|err| {
        format!(
            "failed to create preference output '{}': {err}",
            out.display()
        )
    })?);

    let mut summary = PreferenceExportSummary {
        raw_path: raw.display().to_string(),
        ..PreferenceExportSummary::default()
    };

    for (step_index, step) in view.steps.iter().enumerate() {
        summary.considered_steps += 1;
        if step.status
            != sts_simulator::diff::replay::CombatReplayStepStatus::Executable
        {
            continue;
        }
        let reconstructed = reconstruct_combat_replay_step(&view, step_index)?;
        let legal_moves = sts_simulator::bot::search::legal_moves_for_audit(
            &reconstructed.before_engine,
            &reconstructed.before_combat,
        );
        if legal_moves.len() <= 1 {
            continue;
        }
        let incoming = reconstructed
            .before_combat
            .entities
            .monsters
            .iter()
            .filter(|monster| !monster.is_dying && !monster.is_escaped && monster.current_hp > 0)
            .map(|monster| match monster.current_intent {
                sts_simulator::runtime::combat::Intent::Attack { hits, .. }
                | sts_simulator::runtime::combat::Intent::AttackBuff { hits, .. }
                | sts_simulator::runtime::combat::Intent::AttackDebuff { hits, .. }
                | sts_simulator::runtime::combat::Intent::AttackDefend { hits, .. } => {
                    monster.intent_dmg * hits as i32
                }
                _ => 0,
            })
            .sum::<i32>();
        let hp_ratio = if reconstructed.before_combat.entities.player.max_hp > 0 {
            reconstructed.before_combat.entities.player.current_hp as f32
                / reconstructed.before_combat.entities.player.max_hp as f32
        } else {
            1.0
        };
        if incoming < min_incoming || hp_ratio > max_hp_ratio {
            continue;
        }

        summary.candidate_steps += 1;
        let fixture = build_fixture_from_reconstructed_step(
            &reconstructed,
            replay.source_path.clone(),
            format!(
                "decision_audit_frame_{}",
                reconstructed
                    .before_state_frame_id
                    .unwrap_or(step_index as u64)
            ),
        )?;
        let report = audit_fixture(&fixture, config)?;
        summary.audited_steps += 1;
        let samples = extract_preference_samples(&fixture, &report, config)?;
        if !samples.is_empty() {
            summary.frames_with_preferences += 1;
        }
        for sample in samples {
            *summary
                .preference_kind_counts
                .entry(sample.preference_kind.clone())
                .or_insert(0) += 1;
            summary.exported_samples += 1;
            use std::io::Write;
            writeln!(
                writer,
                "{}",
                serde_json::to_string(&sample).map_err(|err| err.to_string())?
            )
            .map_err(|err| format!("failed writing preference sample: {err}"))?;
        }
        if summary.audited_steps >= limit {
            break;
        }
    }

    Ok(summary)
}

fn export_preference_seed_set(
    raw: &PathBuf,
    out: &PathBuf,
    frames: &[u64],
    config: DecisionAuditConfig,
) -> Result<PreferenceSeedSetSummary, String> {
    let replay = load_live_session_replay_path(raw)?;
    let view = derive_combat_replay_view(&replay);
    if let Some(parent) = out.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create combat preference seed output directory '{}': {err}",
                parent.display()
            )
        })?;
    }
    let mut writer = std::io::BufWriter::new(std::fs::File::create(out).map_err(|err| {
        format!(
            "failed to create preference seed output '{}': {err}",
            out.display()
        )
    })?);

    let mut summary = PreferenceSeedSetSummary {
        raw_path: raw.display().to_string(),
        requested_frames: frames.to_vec(),
        ..PreferenceSeedSetSummary::default()
    };

    for frame in frames {
        let Some(step_index) = find_combat_step_index_by_before_frame_id(&view, *frame) else {
            summary.missing_frames.push(*frame);
            continue;
        };
        let reconstructed = reconstruct_combat_replay_step(&view, step_index)?;
        let fixture = build_fixture_from_reconstructed_step(
            &reconstructed,
            replay.source_path.clone(),
            format!("decision_audit_frame_{frame}"),
        )?;
        let report = audit_fixture(&fixture, config)?;
        let samples = extract_preference_samples(&fixture, &report, config)?;
        summary.audited_frames += 1;
        if samples.is_empty() {
            continue;
        }
        summary.exported_frame_ids.push(*frame);
        for sample in samples {
            *summary
                .preference_kind_counts
                .entry(sample.preference_kind.clone())
                .or_insert(0) += 1;
            summary.exported_samples += 1;
            use std::io::Write;
            writeln!(
                writer,
                "{}",
                serde_json::to_string(&sample).map_err(|err| err.to_string())?
            )
            .map_err(|err| format!("failed writing preference seed sample: {err}"))?;
        }
    }

    Ok(summary)
}

fn summarize_preferences(
    inputs: &[PathBuf],
    top_examples: usize,
) -> Result<PreferenceMotifSummary, String> {
    let mut samples = Vec::new();
    for path in inputs {
        let text = std::fs::read_to_string(path).map_err(|err| {
            format!(
                "failed to read preference jsonl '{}': {err}",
                path.display()
            )
        })?;
        for (line_idx, line) in text.lines().enumerate() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            let sample: CombatPreferenceSample = serde_json::from_str(trimmed).map_err(|err| {
                format!(
                    "failed to parse preference jsonl line {} from '{}': {err}",
                    line_idx + 1,
                    path.display()
                )
            })?;
            samples.push(sample);
        }
    }

    let mut summary = PreferenceMotifSummary {
        input_paths: inputs
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        total_samples: samples.len(),
        unique_frames: {
            let mut frames = std::collections::BTreeSet::new();
            for sample in &samples {
                if let Some(frame_id) = sample.before_frame_id {
                    frames.insert(frame_id);
                }
            }
            frames.len()
        },
        ..PreferenceMotifSummary::default()
    };

    let mut pair_counts = std::collections::BTreeMap::<(String, String), usize>::new();
    let mut motif_examples = Vec::<PreferenceMotifExample>::new();

    for sample in &samples {
        *summary
            .preference_kind_counts
            .entry(sample.preference_kind.clone())
            .or_insert(0) += 1;
        for tag in &sample.chosen_tags {
            *summary.chosen_tag_counts.entry(tag.clone()).or_insert(0) += 1;
        }
        for tag in &sample.preferred_tags {
            *summary.preferred_tag_counts.entry(tag.clone()).or_insert(0) += 1;
        }

        *summary
            .chosen_action_family_counts
            .entry(action_family(&sample.chosen_action).to_string())
            .or_insert(0) += 1;
        *summary
            .preferred_action_family_counts
            .entry(action_family(&sample.preferred_action).to_string())
            .or_insert(0) += 1;

        *pair_counts
            .entry((
                sample.chosen_action.clone(),
                sample.preferred_action.clone(),
            ))
            .or_insert(0) += 1;

        for motif in sample_motifs(sample) {
            *summary.motif_counts.entry(motif.clone()).or_insert(0) += 1;
            motif_examples.push(PreferenceMotifExample {
                motif,
                before_frame_id: sample.before_frame_id,
                chosen_action: sample.chosen_action.clone(),
                preferred_action: sample.preferred_action.clone(),
                preference_kind: sample.preference_kind.clone(),
                score_gap: sample.score_gap,
            });
        }
    }

    let mut top_pairs = pair_counts
        .into_iter()
        .map(
            |((chosen_action, preferred_action), count)| PreferenceActionPairCount {
                chosen_action,
                preferred_action,
                count,
            },
        )
        .collect::<Vec<_>>();
    top_pairs.sort_by(|left, right| {
        right
            .count
            .cmp(&left.count)
            .then_with(|| left.chosen_action.cmp(&right.chosen_action))
            .then_with(|| left.preferred_action.cmp(&right.preferred_action))
    });
    top_pairs.truncate(top_examples.max(1));
    summary.top_action_pairs = top_pairs;

    motif_examples.sort_by(|left, right| {
        right
            .score_gap
            .cmp(&left.score_gap)
            .then_with(|| left.motif.cmp(&right.motif))
            .then_with(|| left.before_frame_id.cmp(&right.before_frame_id))
    });
    let mut deduped = Vec::new();
    let mut seen = std::collections::BTreeSet::new();
    for example in motif_examples {
        let key = (
            example.motif.clone(),
            example.before_frame_id,
            example.chosen_action.clone(),
            example.preferred_action.clone(),
        );
        if seen.insert(key) {
            deduped.push(example);
        }
        if deduped.len() >= top_examples.max(1) {
            break;
        }
    }
    summary.top_examples = deduped;

    Ok(summary)
}

fn render_preference_summary(summary: &PreferenceMotifSummary) -> String {
    let mut lines = Vec::new();
    lines.push("preference motif summary".to_string());
    lines.push(format!(
        "  inputs={} total_samples={} unique_frames={}",
        summary.input_paths.len(),
        summary.total_samples,
        summary.unique_frames
    ));
    if !summary.preference_kind_counts.is_empty() {
        lines.push(format!(
            "  preference_kinds={}",
            format_count_map(&summary.preference_kind_counts)
        ));
    }
    if !summary.motif_counts.is_empty() {
        lines.push(format!(
            "  motifs={}",
            format_count_map(&summary.motif_counts)
        ));
    }
    if !summary.chosen_action_family_counts.is_empty() {
        lines.push(format!(
            "  chosen_action_families={}",
            format_count_map(&summary.chosen_action_family_counts)
        ));
    }
    if !summary.preferred_action_family_counts.is_empty() {
        lines.push(format!(
            "  preferred_action_families={}",
            format_count_map(&summary.preferred_action_family_counts)
        ));
    }
    if !summary.preferred_tag_counts.is_empty() {
        lines.push(format!(
            "  preferred_tags={}",
            format_count_map(&summary.preferred_tag_counts)
        ));
    }
    if !summary.top_action_pairs.is_empty() {
        lines.push("top_action_pairs:".to_string());
        for pair in &summary.top_action_pairs {
            lines.push(format!(
                "  {}x {} -> {}",
                pair.count, pair.chosen_action, pair.preferred_action
            ));
        }
    }
    if !summary.top_examples.is_empty() {
        lines.push("top_examples:".to_string());
        for example in &summary.top_examples {
            lines.push(format!(
                "  motif={} frame={:?} gap={} {} -> {} ({})",
                example.motif,
                example.before_frame_id,
                example.score_gap,
                example.chosen_action,
                example.preferred_action,
                example.preference_kind
            ));
        }
    }
    lines.join("\n")
}

fn format_count_map(map: &std::collections::BTreeMap<String, usize>) -> String {
    let mut items = map.iter().collect::<Vec<_>>();
    items.sort_by(|(left_key, left_count), (right_key, right_count)| {
        right_count
            .cmp(left_count)
            .then_with(|| left_key.cmp(right_key))
    });
    items
        .into_iter()
        .map(|(key, count)| format!("{key}={count}"))
        .collect::<Vec<_>>()
        .join(", ")
}

fn sample_motifs(sample: &CombatPreferenceSample) -> Vec<String> {
    let mut motifs = Vec::new();
    if outcome_rank(sample.preferred_outcome) > outcome_rank(sample.chosen_outcome) {
        motifs.push("better_outcome_available".to_string());
    }
    if has_tag(&sample.preferred_tags, "survival_line") {
        motifs.push("survival_window_missed".to_string());
    }
    if has_tag(&sample.preferred_tags, "block_gained")
        && !has_tag(&sample.chosen_tags, "block_gained")
    {
        motifs.push("undervalued_block".to_string());
    }
    if has_tag(&sample.preferred_tags, "weak_applied")
        && !has_tag(&sample.chosen_tags, "weak_applied")
    {
        motifs.push("undervalued_weak".to_string());
    }
    if has_tag(&sample.preferred_tags, "used_potion")
        && !has_tag(&sample.chosen_tags, "used_potion")
    {
        motifs.push("potion_bridge_available".to_string());
    }

    let chosen_family = action_family(&sample.chosen_action);
    let preferred_family = action_family(&sample.preferred_action);
    if chosen_family == "end_turn" && preferred_family != "end_turn" {
        motifs.push("premature_end_turn".to_string());
    }
    if chosen_family == "defend_like" && preferred_family == "power_like" {
        motifs.push("heuristic_power_timing".to_string());
    }
    if chosen_family == "power_like" && preferred_family == "defend_like" {
        motifs.push("overgreedy_setup".to_string());
    }
    if chosen_family == "attack_like" && preferred_family == "defend_like" {
        motifs.push("attack_over_block".to_string());
    }
    motifs.sort();
    motifs.dedup();
    motifs
}

fn action_family(action: &str) -> &'static str {
    if action == "EndTurn" {
        "end_turn"
    } else if action.starts_with("UsePotion#") {
        "potion"
    } else if action.contains("Defend") {
        "defend_like"
    } else if looks_like_power_action(action) {
        "power_like"
    } else if action.starts_with("Play #") {
        "attack_like"
    } else {
        "other"
    }
}

fn looks_like_power_action(action: &str) -> bool {
    const POWER_HINTS: &[&str] = &[
        "Feel No Pain",
        "Inflame",
        "Corruption",
        "Dark Embrace",
        "Barricade",
        "Demon Form",
        "Juggernaut",
        "Metallicize",
        "Brutality",
        "Rupture",
        "Berserk",
        "Combust",
        "Fire Breathing",
        "Evolve",
        "Rage",
    ];
    POWER_HINTS.iter().any(|hint| action.contains(hint))
}

fn has_tag(tags: &[String], target: &str) -> bool {
    tags.iter().any(|tag| tag == target)
}

fn outcome_rank(outcome: TrajectoryOutcomeKind) -> i32 {
    match outcome {
        TrajectoryOutcomeKind::LethalWin => 3,
        TrajectoryOutcomeKind::Survives => 2,
        TrajectoryOutcomeKind::Timeout => 1,
        TrajectoryOutcomeKind::Dies => 0,
    }
}

fn resolve_recent_raw_path(explicit: Option<PathBuf>) -> Result<PathBuf, String> {
    if let Some(path) = explicit {
        return Ok(path);
    }
    sts_simulator::cli::live_comm_admin::latest_valid_raw_path(
        &sts_simulator::cli::live_comm_admin::LiveLogPaths::default_paths(),
    )
    .or_else(|| {
        sts_simulator::cli::live_comm_admin::latest_raw_path(
            &sts_simulator::cli::live_comm_admin::LiveLogPaths::default_paths(),
        )
    })
    .ok_or_else(|| "no recent livecomm raw log found".to_string())
}

fn resolve_recent_suspect_path(explicit: Option<PathBuf>) -> Result<Option<PathBuf>, String> {
    if let Some(path) = explicit {
        return Ok(Some(path));
    }
    Ok(
        sts_simulator::cli::live_comm_admin::latest_combat_suspect_path(
            &sts_simulator::cli::live_comm_admin::LiveLogPaths::default_paths(),
        ),
    )
}

fn load_live_combat_suspects(path: &PathBuf) -> Result<Vec<LiveCombatSuspectRecord>, String> {
    let text = std::fs::read_to_string(path)
        .map_err(|err| format!("failed to read suspect log '{}': {err}", path.display()))?;
    let mut suspects = Vec::new();
    for (line_idx, line) in text.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        let suspect: LiveCombatSuspectRecord = serde_json::from_str(trimmed).map_err(|err| {
            format!(
                "failed to parse suspect line {} from '{}': {err}",
                line_idx + 1,
                path.display()
            )
        })?;
        suspects.push(suspect);
    }
    Ok(suspects)
}

fn same_client_input(left: &ClientInput, right: &ClientInput) -> bool {
    match (left, right) {
        (
            ClientInput::PlayCard {
                card_index: left_card,
                target: left_target,
            },
            ClientInput::PlayCard {
                card_index: right_card,
                target: right_target,
            },
        ) => left_card == right_card && left_target == right_target,
        (
            ClientInput::UsePotion {
                potion_index: left_potion,
                target: left_target,
            },
            ClientInput::UsePotion {
                potion_index: right_potion,
                target: right_target,
            },
        ) => left_potion == right_potion && left_target == right_target,
        (ClientInput::EndTurn, ClientInput::EndTurn)
        | (ClientInput::Proceed, ClientInput::Proceed)
        | (ClientInput::Cancel, ClientInput::Cancel) => true,
        _ => false,
    }
}

fn render_search_move_detail(stat: &sts_simulator::bot::search::SearchMoveStat) -> String {
    let cluster_suffix = if stat.cluster_size > 1 {
        format!(
            " cluster_id={} cluster_size={} reduced_kind={} collapsed_members={}",
            stat.cluster_id,
            stat.cluster_size,
            stat.equivalence_kind
                .map(|kind| kind.as_str())
                .unwrap_or("none"),
            stat.collapsed_inputs.len()
        )
    } else {
        String::new()
    };
    format!(
        "order={:.1} leaf={:.1} policy={:.1} sequence={:.1} frontload={:.1} defer={:.1} branch={:.1} downside={:.1} survival_window={:.1} exhaust_evidence={:.1} projected_hp={} projected_block={} projected_unblocked={} projected_enemy_total={} survives={} exhaust_block={} exhaust_draw={}",
        stat.order_score,
        stat.leaf_score,
        stat.policy_bonus,
        stat.sequence_bonus,
        stat.sequence_frontload_bonus,
        stat.sequence_defer_bonus,
        stat.sequence_branch_bonus,
        stat.sequence_downside_penalty,
        stat.sequence_survival_bonus,
        stat.sequence_exhaust_bonus,
        stat.projected_hp,
        stat.projected_block,
        stat.projected_unblocked,
        stat.projected_enemy_total,
        stat.survives,
        stat.realized_exhaust_block,
        stat.realized_exhaust_draw
    ) + &cluster_suffix
}

fn session_outcome_label(
    replay: &sts_simulator::diff::replay::LiveSessionReplay,
) -> &'static str {
    let Some(last) = replay.steps.last() else {
        return "unknown";
    };
    let screen_name = last
        .after_root
        .get("game_state")
        .and_then(|state| state.get("screen_name"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    if screen_name.eq_ignore_ascii_case("DEATH") {
        "defeat"
    } else if screen_name.eq_ignore_ascii_case("VICTORY") {
        "victory"
    } else {
        "unknown"
    }
}

fn state_frame_id_to_u64(state_frame_id: Option<i64>) -> Option<u64> {
    state_frame_id.and_then(|value| u64::try_from(value).ok())
}

fn audit_recent_live_session(
    raw: Option<PathBuf>,
    suspects: Option<PathBuf>,
    depth_limit: u32,
    top_k: usize,
    limit: usize,
    equivalence_mode: SearchEquivalenceMode,
) -> Result<String, String> {
    let raw_path = resolve_recent_raw_path(raw)?;
    let suspect_path = resolve_recent_suspect_path(suspects)?;
    let replay = load_live_session_replay_path(&raw_path)?;
    let session_outcome = session_outcome_label(&replay);
    let view = derive_combat_replay_view(&replay);
    let suspect_records = if let Some(path) = suspect_path.as_ref() {
        if path.exists() {
            load_live_combat_suspects(path)?
        } else {
            Vec::new()
        }
    } else {
        Vec::new()
    };

    let mut shortlisted = Vec::<RecentAuditEntry>::new();
    let mut seen = std::collections::BTreeSet::<u64>::new();
    for suspect in suspect_records.into_iter().rev() {
        let Some(frame) = state_frame_id_to_u64(suspect.state_frame_id) else {
            continue;
        };
        if !seen.insert(frame) {
            continue;
        }
        shortlisted.push(RecentAuditEntry {
            frame,
            priority: recent_suspect_priority(&suspect),
            reasons: suspect.reasons.clone(),
            suspect: Some(suspect),
        });
    }
    shortlisted.sort_by(|left, right| {
        right
            .priority
            .cmp(&left.priority)
            .then_with(|| right.frame.cmp(&left.frame))
    });
    shortlisted.truncate(limit.max(1));

    if shortlisted.is_empty() {
        for step in view.steps.iter().rev() {
            if step.status != CombatReplayStepStatus::Executable {
                continue;
            }
            let Some(frame) = step.state_frame_id.or_else(|| {
                step.before_root
                    .get("protocol_meta")
                    .and_then(|meta| meta.get("state_frame_id").or_else(|| meta.get("frame_id")))
                    .and_then(serde_json::Value::as_u64)
            }) else {
                continue;
            };
            if !seen.insert(frame) {
                continue;
            }
            shortlisted.push(RecentAuditEntry {
                frame,
                priority: 0,
                reasons: vec!["fallback_recent_frame".to_string()],
                suspect: None,
            });
            if shortlisted.len() >= limit.max(1) {
                break;
            }
        }
    }

    shortlisted.sort_by_key(|entry| entry.frame);

    let coverage = sts_simulator::bot::CoverageDb::load_or_default();
    let mut lines = Vec::new();
    lines.push("recent live session audit".to_string());
    lines.push(format!(
        "  raw={} suspects={} session_outcome={} shortlisted_frames={}",
        raw_path.display(),
        suspect_path
            .as_ref()
            .map(|path| path.display().to_string())
            .unwrap_or_else(|| "<none>".to_string()),
        session_outcome,
        shortlisted.len()
    ));
    lines.push(format!("  equivalence_mode={}", equivalence_mode.as_str()));

    let mut survival_frames = 0usize;
    let mut exhaust_frames = 0usize;
    let mut heuristic_gap_frames = 0usize;
    let mut tight_gap_frames = 0usize;
    let mut loss_pivot_frames = 0usize;
    let mut clustered_frames = 0usize;

    lines.push("shortlist:".to_string());
    for entry in &shortlisted {
        let reasons = if entry.reasons.is_empty() {
            "<none>".to_string()
        } else {
            entry.reasons.join(",")
        };
        lines.push(format!(
            "  frame={} priority={} reasons={}",
            entry.frame, entry.priority, reasons
        ));
    }

    lines.push("frame_details:".to_string());
    for entry in shortlisted {
        let Some(step_index) = find_combat_step_index_by_before_frame_id(&view, entry.frame) else {
            lines.push(format!("  frame={} missing executable step", entry.frame));
            continue;
        };
        let reconstructed = reconstruct_combat_replay_step(&view, step_index)?;
        let executed_input =
            mapped_command_to_input(&reconstructed.mapped_command, &reconstructed.before_combat)?;
        let diagnostics = diagnose_root_search_with_depth_and_mode(
            &reconstructed.before_engine,
            &reconstructed.before_combat,
            &coverage,
            sts_simulator::bot::CoverageMode::Off,
            None,
            depth_limit,
            0,
            equivalence_mode,
        );
        let Some(best) = diagnostics.top_moves.first() else {
            lines.push(format!("  frame={} no root moves", entry.frame));
            continue;
        };
        let second_gap = diagnostics
            .top_moves
            .get(1)
            .map(|second| best.avg_score - second.avg_score);
        let mut detail_reasons = entry.reasons.clone();
        if entry
            .suspect
            .as_ref()
            .is_some_and(|suspect| suspect.heuristic_search_gap)
        {
            heuristic_gap_frames += 1;
        }
        if entry
            .suspect
            .as_ref()
            .is_some_and(|suspect| suspect.tight_root_gap)
        {
            tight_gap_frames += 1;
        }
        if best.sequence_survival_bonus.abs() >= best.sequence_exhaust_bonus.abs()
            && best.sequence_survival_bonus.abs() >= 120_000.0
        {
            survival_frames += 1;
        }
        if best.sequence_exhaust_bonus.abs() >= 60_000.0 || best.realized_exhaust_draw > 0 {
            exhaust_frames += 1;
        }
        if best.cluster_size > 1 {
            clustered_frames += 1;
        }
        let sequencing_conflict = entry.suspect.as_ref().is_some_and(|suspect| {
            suspect.heuristic_search_gap
                && (suspect.sequence_frontload_bonus.abs() >= 3_000.0
                    || suspect.sequence_defer_bonus.abs() >= 3_000.0
                    || suspect.sequence_downside_penalty.abs() >= 3_000.0)
        });
        let branch_opening_conflict = entry.suspect.as_ref().is_some_and(|suspect| {
            suspect.heuristic_search_gap
                && (suspect.sequence_branch_bonus.abs() >= 3_500.0
                    || suspect.sequence_downside_penalty.abs() >= 3_500.0)
        });
        if sequencing_conflict {
            detail_reasons.push("sequencing_conflict".to_string());
        }
        if branch_opening_conflict {
            detail_reasons.push("branch_opening_conflict".to_string());
        }
        if session_outcome == "defeat"
            && !same_client_input(&executed_input, &diagnostics.chosen_move)
            && second_gap.is_some_and(|gap| gap >= 3.0)
        {
            loss_pivot_frames += 1;
            detail_reasons.push("loss_pivot_candidate".to_string());
        }

        lines.push(format!(
            "  frame={} executed={} search={} top_gap={:?} reasons={} elapsed_ms={} depth_limit={} max_depth={} root_width={} branch_width={} max_engine_steps={} legal_moves={} reduced_legal_moves={}",
            entry.frame,
            describe_client_input(&reconstructed.before_combat, &executed_input),
            describe_client_input(&reconstructed.before_combat, &diagnostics.chosen_move),
            second_gap,
            if detail_reasons.is_empty() {
                "<none>".to_string()
            } else {
                detail_reasons.join(",")
            },
            diagnostics.elapsed_ms,
            diagnostics.depth_limit,
            diagnostics.max_decision_depth,
            diagnostics.root_width,
            diagnostics.branch_width,
            diagnostics.max_engine_steps,
            diagnostics.legal_moves,
            diagnostics.reduced_legal_moves
        ));
        lines.push(format!(
            "      top score={:.1} visits={} move={}",
            best.avg_score,
            best.visits,
            describe_client_input(&reconstructed.before_combat, &best.input)
        ));
        lines.push(format!("      {}", render_search_move_detail(best)));
        if best.cluster_size > 1 {
            lines.push(format!(
                "      cluster_members={}",
                best.collapsed_inputs
                    .iter()
                    .map(|input| describe_client_input(&reconstructed.before_combat, input))
                    .collect::<Vec<_>>()
                    .join(" | ")
            ));
        }
        for stat in diagnostics
            .top_moves
            .iter()
            .skip(1)
            .take(top_k.saturating_sub(1))
        {
            lines.push(format!(
                "      alt score={:.1} visits={} move={}",
                stat.avg_score,
                stat.visits,
                describe_client_input(&reconstructed.before_combat, &stat.input)
            ));
            lines.push(format!("          {}", render_search_move_detail(stat)));
            if stat.cluster_size > 1 {
                lines.push(format!(
                    "          cluster_members={}",
                    stat.collapsed_inputs
                        .iter()
                        .map(|input| describe_client_input(&reconstructed.before_combat, input))
                        .collect::<Vec<_>>()
                        .join(" | ")
                ));
            }
        }
        if let Some(suspect) = entry.suspect.as_ref() {
            lines.push(format!(
                "      live suspect chosen={} heuristic={} search={} live_top_gap={:?} live_sequence={:.1} frontload={:.1} defer={:.1} branch={:.1} downside={:.1} live_survival_window={:.1} live_exhaust_evidence={:.1} live_exhaust_block={} live_exhaust_draw={} branch_family={} rationale={} branch_rationale={} downside_rationale={}",
                suspect.chosen_move,
                suspect.heuristic_move,
                suspect.search_move,
                suspect.top_gap,
                suspect.sequence_bonus,
                suspect.sequence_frontload_bonus,
                suspect.sequence_defer_bonus,
                suspect.sequence_branch_bonus,
                suspect.sequence_downside_penalty,
                suspect.survival_window_delta,
                suspect.exhaust_evidence_delta,
                suspect.realized_exhaust_block,
                suspect.realized_exhaust_draw,
                suspect.branch_family.as_deref().unwrap_or("none"),
                suspect.sequencing_rationale_key.as_deref().unwrap_or(""),
                suspect.branch_rationale_key.as_deref().unwrap_or(""),
                suspect.downside_rationale_key.as_deref().unwrap_or("")
            ));
            lines.push(format!(
                "      live frame_count={} response_id={:?} heuristic_gap={} large_sequence={} tight_root_gap={}",
                suspect.frame_count,
                suspect.response_id,
                suspect.heuristic_search_gap,
                suspect.large_sequence_bonus,
                suspect.tight_root_gap
            ));
        }
    }

    let mut suggestions = Vec::new();
    if survival_frames > 0 {
        suggestions.push(format!(
            "survival_window / pressure timing is the leading suspect in {} shortlisted frames",
            survival_frames
        ));
    }
    if exhaust_frames > 0 {
        suggestions.push(format!(
            "setup timing / exhaust payoff evidence is active in {} shortlisted frames",
            exhaust_frames
        ));
    }
    if heuristic_gap_frames > 0 {
        suggestions.push(format!(
            "heuristic/search mismatch deserves review in {} shortlisted frames",
            heuristic_gap_frames
        ));
    }
    if tight_gap_frames > 0 {
        suggestions.push(format!(
            "root tie-breaks are thin in {} shortlisted frames; inspect ranking stability",
            tight_gap_frames
        ));
    }
    if clustered_frames > 0 {
        suggestions.push(format!(
            "equivalence reduction was active in {} shortlisted frames; verify disagreements are true strategy gaps, not representative choice",
            clustered_frames
        ));
    }
    if loss_pivot_frames > 0 {
        suggestions.push(format!(
            "defeat session had {} loss_pivot_candidate frames where deeper audit preferred another move",
            loss_pivot_frames
        ));
    }
    if suggestions.is_empty() {
        suggestions.push(
            "no dominant failure cluster; inspect the shortlisted frames directly".to_string(),
        );
    }

    lines.push("next_step:".to_string());
    for suggestion in suggestions {
        lines.push(format!("  {}", suggestion));
    }

    Ok(lines.join("\n"))
}

fn build_search_baseline_record(
    raw: &PathBuf,
    frame: u64,
    depth_limit: u32,
    top_k: usize,
    equivalence_mode: SearchEquivalenceMode,
    root_prior: Option<&RootPriorConfig>,
) -> Result<SearchBaselineRecord, String> {
    let replay = load_live_session_replay_path(raw)?;
    let view = derive_combat_replay_view(&replay);
    let step_index = find_combat_step_index_by_before_frame_id(&view, frame)
        .ok_or_else(|| format!("no executable combat step found for before frame_id={frame}"))?;
    let reconstructed = reconstruct_combat_replay_step(&view, step_index)?;
    let diagnostics = diagnose_root_search_with_depth_and_mode_and_root_prior(
        &reconstructed.before_engine,
        &reconstructed.before_combat,
        &sts_simulator::bot::CoverageDb::load_or_default(),
        sts_simulator::bot::CoverageMode::Off,
        None,
        depth_limit,
        0,
        equivalence_mode,
        sts_simulator::bot::search::SearchProfilingLevel::Summary,
        root_prior,
    );

    Ok(search_baseline_record_from_diagnostics(
        raw.display().to_string(),
        frame,
        depth_limit,
        top_k,
        &reconstructed.before_combat,
        diagnostics,
    ))
}

fn build_search_baseline_record_from_fixture(
    fixture: &sts_simulator::bot::search::DecisionAuditFixture,
    depth_limit: u32,
    top_k: usize,
    equivalence_mode: SearchEquivalenceMode,
    root_prior: Option<&RootPriorConfig>,
) -> Result<SearchBaselineRecord, String> {
    let combat = build_combat_state(&fixture.combat_snapshot, &fixture.relics);
    let engine = match fixture.engine_state {
        DecisionAuditEngineState::CombatPlayerTurn => EngineState::CombatPlayerTurn,
    };
    let diagnostics = diagnose_root_search_with_depth_and_mode_and_root_prior(
        &engine,
        &combat,
        &sts_simulator::bot::CoverageDb::load_or_default(),
        sts_simulator::bot::CoverageMode::Off,
        None,
        depth_limit,
        0,
        equivalence_mode,
        sts_simulator::bot::search::SearchProfilingLevel::Summary,
        root_prior,
    );
    Ok(search_baseline_record_from_diagnostics(
        fixture
            .source_path
            .clone()
            .unwrap_or_else(|| fixture.name.clone()),
        fixture.before_frame_id.unwrap_or(0),
        depth_limit,
        top_k,
        &combat,
        diagnostics,
    ))
}

fn search_baseline_record_from_diagnostics(
    source_path: String,
    frame: u64,
    depth_limit: u32,
    top_k: usize,
    combat: &sts_simulator::runtime::combat::CombatState,
    diagnostics: sts_simulator::bot::search::SearchDiagnostics,
) -> SearchBaselineRecord {
    SearchBaselineRecord {
        frame,
        source_path,
        depth_limit,
        max_depth: diagnostics.max_decision_depth,
        root_width: diagnostics.root_width,
        branch_width: diagnostics.branch_width,
        max_engine_steps: diagnostics.max_engine_steps,
        equivalence_mode: diagnostics.equivalence_mode.as_str().to_string(),
        legal_moves: diagnostics.legal_moves,
        reduced_legal_moves: diagnostics.reduced_legal_moves,
        simulations: diagnostics.simulations,
        elapsed_ms: diagnostics.elapsed_ms,
        chosen_move: describe_client_input(combat, &diagnostics.chosen_move),
        root_prior_enabled: diagnostics.root_prior_enabled,
        root_prior_key: diagnostics.root_prior_key,
        root_prior_weight: diagnostics.root_prior_weight,
        root_prior_hits: diagnostics.root_prior_hits,
        root_prior_reordered: diagnostics.root_prior_reordered,
        profile: diagnostics.profile,
        top_moves: diagnostics
            .top_moves
            .iter()
            .take(top_k.max(1))
            .enumerate()
            .map(|(idx, stat)| SearchBaselineMove {
                rank: idx + 1,
                move_text: describe_client_input(combat, &stat.input),
                avg_score: stat.avg_score,
                visits: stat.visits,
                cluster_size: stat.cluster_size,
                base_order_score: stat.base_order_score,
                order_score: stat.order_score,
                root_prior_score: stat.root_prior_score,
                root_prior_hit: stat.root_prior_hit,
                leaf_score: stat.leaf_score,
                policy_bonus: stat.policy_bonus,
                sequence_bonus: stat.sequence_bonus,
                sequence_frontload_bonus: stat.sequence_frontload_bonus,
                sequence_defer_bonus: stat.sequence_defer_bonus,
                sequence_branch_bonus: stat.sequence_branch_bonus,
                sequence_downside_penalty: stat.sequence_downside_penalty,
                survival_window_delta: stat.sequence_survival_bonus,
                exhaust_evidence_delta: stat.sequence_exhaust_bonus,
                realized_exhaust_block: stat.realized_exhaust_block,
                realized_exhaust_draw: stat.realized_exhaust_draw,
                branch_family: input_branch_family(combat, &stat.input),
            })
            .collect(),
    }
}

fn load_root_prior_provider(
    path: Option<&PathBuf>,
) -> Result<Option<Arc<LookupRootPriorProvider>>, String> {
    path.map(|path| LookupRootPriorProvider::load_jsonl(path).map(Arc::new))
        .transpose()
}

fn default_root_prior_key_for_fixture(
    fixture: &sts_simulator::bot::search::DecisionAuditFixture,
) -> Result<Option<RootPriorQueryKey>, String> {
    if let (Some(source_path), Some(frame)) = (&fixture.source_path, fixture.before_frame_id) {
        return Ok(Some(RootPriorQueryKey::ReplayFrame {
            source_path: source_path.clone(),
            frame,
        }));
    }
    Ok(None)
}

fn load_root_prior_config(
    args: &RootPriorCommandArgs,
    default_key: Option<RootPriorQueryKey>,
) -> Result<Option<RootPriorConfig>, String> {
    let provider = load_root_prior_provider(args.q_local_prior.as_ref())?;
    build_root_prior_config(args, default_key, provider)
}

fn build_root_prior_config(
    args: &RootPriorCommandArgs,
    default_key: Option<RootPriorQueryKey>,
    provider: Option<Arc<LookupRootPriorProvider>>,
) -> Result<Option<RootPriorConfig>, String> {
    if args.q_local_shadow && provider.is_none() {
        return Err("--q-local-shadow requires --q-local-prior".to_string());
    }
    let Some(provider) = provider else {
        return Ok(None);
    };
    let key = resolve_root_prior_key(args, default_key)?;
    let Some(key) = key else {
        return Err("q_local prior requested but no root prior key could be derived; pass explicit --q-local-prior-* key arguments".to_string());
    };
    Ok(Some(RootPriorConfig {
        provider,
        key,
        weight: args.q_local_prior_weight,
        shadow: args.q_local_shadow,
    }))
}

fn resolve_root_prior_key(
    args: &RootPriorCommandArgs,
    default_key: Option<RootPriorQueryKey>,
) -> Result<Option<RootPriorQueryKey>, String> {
    let spec_fields = (
        args.q_local_prior_spec_name.as_ref(),
        args.q_local_prior_episode_id,
        args.q_local_prior_step_index,
    );
    let replay_fields = (
        args.q_local_prior_source_path.as_ref(),
        args.q_local_prior_frame,
    );
    if spec_fields.0.is_some() || spec_fields.1.is_some() || spec_fields.2.is_some() {
        let (Some(spec_name), Some(episode_id), Some(step_index)) = spec_fields else {
            return Err(
                "explicit spec prior key requires --q-local-prior-spec-name, --q-local-prior-episode-id, and --q-local-prior-step-index".to_string(),
            );
        };
        return Ok(Some(RootPriorQueryKey::SpecEpisodeStep {
            spec_name: spec_name.clone(),
            episode_id,
            step_index,
        }));
    }
    if replay_fields.0.is_some() || replay_fields.1.is_some() {
        let (Some(source_path), Some(frame)) = replay_fields else {
            return Err(
                "explicit replay prior key requires --q-local-prior-source-path and --q-local-prior-frame".to_string(),
            );
        };
        return Ok(Some(RootPriorQueryKey::ReplayFrame {
            source_path: source_path.clone(),
            frame,
        }));
    }
    Ok(default_key)
}

fn render_search_baseline_record(record: &SearchBaselineRecord) -> String {
    let mut lines = Vec::new();
    lines.push(format!("search diagnosis: frame {}", record.frame));
    lines.push(format!(
        "  source={} depth_limit={} max_depth={} root_width={} branch_width={} max_engine_steps={} elapsed_ms={} legal_moves={} reduced_legal_moves={} equivalence_mode={} simulations={}",
        record.source_path,
        record.depth_limit,
        record.max_depth,
        record.root_width,
        record.branch_width,
        record.max_engine_steps,
        record.elapsed_ms,
        record.legal_moves,
        record.reduced_legal_moves,
        record.equivalence_mode,
        record.simulations
    ));
    lines.push(format!("  chosen_move={}", record.chosen_move));
    lines.push(format!(
        "  root_prior enabled={} key={} weight={} hits={} reordered={}",
        record.root_prior_enabled,
        record.root_prior_key.as_deref().unwrap_or("none"),
        record.root_prior_weight,
        record.root_prior_hits,
        record.root_prior_reordered
    ));
    lines.push(format!(
        "  profile search_total_ms={} advance_calls={} advance_steps={} root_reduce={}=>{} recursive_reduce={}=>{} nodes={} terminal_nodes={}",
        record.profile.search_total_ms,
        record.profile.advance_calls,
        record.profile.advance_engine_steps,
        record.profile.root.transition_reduce_inputs,
        record.profile.root.transition_reduce_outputs,
        record.profile.recursive.transition_reduce_inputs,
        record.profile.recursive.transition_reduce_outputs,
        record.profile.nodes.nodes_expanded,
        record.profile.nodes.terminal_nodes
    ));
    let frontload_dominant = record
        .top_moves
        .iter()
        .map(|stat| stat.sequence_frontload_bonus.abs())
        .sum::<f32>();
    let downside_dominant = record
        .top_moves
        .iter()
        .map(|stat| stat.sequence_downside_penalty.abs())
        .sum::<f32>();
    lines.push(format!(
        "  sequencing_summary={}{}",
        if frontload_dominant >= downside_dominant {
            "frontload_dominant"
        } else {
            "downside_dominant"
        },
        if record
            .top_moves
            .iter()
            .any(|stat| stat.sequence_branch_bonus.abs() >= 3_500.0)
        {
            ", branch_opening_active"
        } else {
            ""
        }
    ));
    lines.push("top_moves:".to_string());
    for stat in &record.top_moves {
        lines.push(format!(
            "  rank={} score={:.1} visits={} cluster_size={} move={} branch_family={}",
            stat.rank,
            stat.avg_score,
            stat.visits,
            stat.cluster_size,
            stat.move_text,
            stat.branch_family.as_deref().unwrap_or("none")
        ));
        lines.push(format!(
            "      base_order={:.1} order={:.1} prior={:.1} prior_hit={} leaf={:.1} policy={:.1} sequence={:.1} frontload={:.1} defer={:.1} branch={:.1} downside={:.1} survival_window={:.1} exhaust_evidence={:.1} exhaust_block={} exhaust_draw={}",
            stat.base_order_score,
            stat.order_score,
            stat.root_prior_score,
            stat.root_prior_hit,
            stat.leaf_score,
            stat.policy_bonus,
            stat.sequence_bonus,
            stat.sequence_frontload_bonus,
            stat.sequence_defer_bonus,
            stat.sequence_branch_bonus,
            stat.sequence_downside_penalty,
            stat.survival_window_delta,
            stat.exhaust_evidence_delta,
            stat.realized_exhaust_block,
            stat.realized_exhaust_draw
        ));
    }
    lines.join("\n")
}

fn describe_client_input(
    combat: &sts_simulator::runtime::combat::CombatState,
    input: &ClientInput,
) -> String {
    match input {
        ClientInput::PlayCard { card_index, target } => {
            let card = combat
                .zones
                .hand
                .get(*card_index)
                .map(format_card)
                .unwrap_or_else(|| format!("hand[{card_index}]"));
            match target {
                Some(target) => format!("Play #{} {card} @{target}", card_index + 1),
                None => format!("Play #{} {card}", card_index + 1),
            }
        }
        ClientInput::UsePotion {
            potion_index,
            target,
        } => match target {
            Some(target) => format!("UsePotion#{potion_index} @{target}"),
            None => format!("UsePotion#{potion_index}"),
        },
        ClientInput::EndTurn => "EndTurn".to_string(),
        other => format!("{other:?}"),
    }
}

fn format_card(card: &CombatCard) -> String {
    let mut label = sts_simulator::content::cards::get_card_definition(card.id)
        .name
        .to_string();
    if card.upgrades > 0 {
        label.push_str(&"+".repeat(card.upgrades as usize));
    }
    label
}

fn input_branch_family(
    combat: &sts_simulator::runtime::combat::CombatState,
    input: &ClientInput,
) -> Option<String> {
    let ClientInput::PlayCard { card_index, .. } = input else {
        return None;
    };
    let card = combat.zones.hand.get(*card_index)?;
    branch_family_for_card(card.id).map(|family| family.as_str().to_string())
}
