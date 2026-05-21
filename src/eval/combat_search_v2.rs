use std::fs;
use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::ai::combat_search_v2::{
    compare_trajectory_reports, run_combat_search_v2, trajectory_from_state,
    CombatSearchV2ActionTrace, CombatSearchV2Config, CombatSearchV2PotionPolicy,
    CombatSearchV2Report, CombatSearchV2TrajectoryReport, SearchTerminalLabel,
};
use crate::fixtures::combat_case::{
    describe_case_step, input_for_case_step, load_case_from_path, lower_case, CombatCase,
};
use crate::fixtures::combat_start_spec::{compile_combat_start_spec, CombatStartSpec};
use crate::sim::combat::CombatPosition;
use crate::state::core::ClientInput;
use crate::testing::replay_support::tick_until_stable;

#[derive(Clone, Debug, Default)]
pub struct CombatSearchV2RunOptions {
    pub max_nodes: Option<usize>,
    pub max_actions_per_line: Option<usize>,
    pub max_engine_steps_per_action: Option<usize>,
    pub wall_ms: Option<u64>,
    pub potion_policy: Option<CombatSearchV2PotionPolicy>,
}

impl CombatSearchV2RunOptions {
    pub fn to_search_config(&self, input_label: String) -> CombatSearchV2Config {
        CombatSearchV2Config {
            max_nodes: self.max_nodes.unwrap_or(50_000),
            max_actions_per_line: self.max_actions_per_line.unwrap_or(200),
            max_engine_steps_per_action: self.max_engine_steps_per_action.unwrap_or(250),
            wall_time: self.wall_ms.map(Duration::from_millis),
            input_label: Some(input_label),
            potion_policy: self
                .potion_policy
                .unwrap_or(CombatSearchV2PotionPolicy::Never),
        }
    }

    fn merge_bench_config(
        &self,
        default_config: &CombatSearchV2BenchSearchConfig,
        case_config: &CombatSearchV2BenchSearchConfig,
    ) -> CombatSearchV2BenchSearchConfig {
        CombatSearchV2BenchSearchConfig {
            max_nodes: self
                .max_nodes
                .or(case_config.max_nodes)
                .or(default_config.max_nodes),
            max_actions_per_line: self
                .max_actions_per_line
                .or(case_config.max_actions_per_line)
                .or(default_config.max_actions_per_line),
            max_engine_steps_per_action: self
                .max_engine_steps_per_action
                .or(case_config.max_engine_steps_per_action)
                .or(default_config.max_engine_steps_per_action),
            wall_ms: self
                .wall_ms
                .or(case_config.wall_ms)
                .or(default_config.wall_ms),
            potion_policy: self
                .potion_policy
                .or(case_config.potion_policy)
                .or(default_config.potion_policy),
            random_boundary: case_config
                .random_boundary
                .clone()
                .or_else(|| default_config.random_boundary.clone()),
        }
    }
}

#[derive(Clone, Debug)]
pub enum CombatSearchV2StartSource {
    Case(PathBuf),
    StartSpec(PathBuf),
}

#[derive(Clone)]
pub struct CombatSearchV2LoadedStart {
    pub label: String,
    pub position: CombatPosition,
    pub case_baseline: Option<CombatSearchV2TrajectoryReport>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2SingleRun {
    pub search_report: CombatSearchV2Report,
    pub baseline_trajectory: Option<CombatSearchV2TrajectoryReport>,
    pub baseline_comparison: Option<Value>,
}

impl CombatSearchV2SingleRun {
    pub fn to_legacy_output_value(&self) -> Result<Value, serde_json::Error> {
        let mut output = serde_json::to_value(&self.search_report)?;
        if let Some(object) = output.as_object_mut() {
            if let Some(baseline) = &self.baseline_trajectory {
                object.insert(
                    "baseline_trajectory".to_string(),
                    serde_json::to_value(baseline)?,
                );
            }
            if let Some(comparison) = &self.baseline_comparison {
                object.insert("baseline_comparison".to_string(), comparison.clone());
            }
        }
        Ok(output)
    }
}

#[derive(Clone, Debug, Deserialize)]
pub struct CombatSearchV2BenchManifest {
    #[serde(default)]
    pub schema_name: Option<String>,
    #[serde(default)]
    pub schema_version: Option<u32>,
    #[serde(default)]
    pub default_search_config: CombatSearchV2BenchSearchConfig,
    pub cases: Vec<CombatSearchV2BenchCaseEntry>,
}

#[derive(Clone, Debug, Deserialize)]
pub struct CombatSearchV2BenchCaseEntry {
    pub case_id: String,
    pub combat_case_path: PathBuf,
    #[serde(default)]
    pub source: Option<String>,
    #[serde(default)]
    pub deck_summary: Option<Value>,
    #[serde(default)]
    pub run_context: Option<Value>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub search_config: CombatSearchV2BenchSearchConfig,
}

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct CombatSearchV2BenchSearchConfig {
    #[serde(default)]
    pub max_nodes: Option<usize>,
    #[serde(default)]
    pub max_actions_per_line: Option<usize>,
    #[serde(default)]
    pub max_engine_steps_per_action: Option<usize>,
    #[serde(default)]
    pub wall_ms: Option<u64>,
    #[serde(default)]
    pub potion_policy: Option<CombatSearchV2PotionPolicy>,
    #[serde(default)]
    pub random_boundary: Option<String>,
}

impl CombatSearchV2BenchSearchConfig {
    fn to_run_options(&self) -> CombatSearchV2RunOptions {
        CombatSearchV2RunOptions {
            max_nodes: self.max_nodes,
            max_actions_per_line: self.max_actions_per_line,
            max_engine_steps_per_action: self.max_engine_steps_per_action,
            wall_ms: self.wall_ms,
            potion_policy: self.potion_policy,
        }
    }
}

#[derive(Debug, Serialize)]
pub struct CombatSearchV2BenchSummaryReport {
    pub schema_name: &'static str,
    pub schema_version: u32,
    pub manifest_path: String,
    pub manifest_schema_name: Option<String>,
    pub manifest_schema_version: Option<u32>,
    pub case_count: usize,
    pub human_baseline_available: usize,
    pub human_wins: usize,
    pub search_found_complete: usize,
    pub search_found_win: usize,
    pub candidate_search_better: usize,
    pub candidate_search_tied: usize,
    pub candidate_human_better: usize,
    pub formal_inconclusive: usize,
    pub errors: usize,
    pub median_nodes_to_first_win: Option<f64>,
    pub median_best_complete_final_hp: Option<f64>,
    pub cases: Vec<CombatSearchV2BenchCaseSummary>,
}

#[derive(Debug, Serialize)]
pub struct CombatSearchV2BenchCaseSummary {
    pub case_id: String,
    pub status: CombatSearchV2BenchCaseStatus,
    pub source: Option<String>,
    pub tags: Vec<String>,
    pub human_baseline_available: bool,
    pub human_terminal: Option<SearchTerminalLabel>,
    pub human_final_hp: Option<i32>,
    pub search_best_terminal: Option<SearchTerminalLabel>,
    pub search_best_final_hp: Option<i32>,
    pub search_proof_status: Option<String>,
    pub nodes_to_first_win: Option<u64>,
    pub frontier_remaining: Option<usize>,
    pub candidate_verdict: Option<String>,
    pub formal_verdict: Option<String>,
    pub random_boundary: String,
    pub error: Option<String>,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchV2BenchCaseStatus {
    Ok,
    Error,
}

#[derive(Debug, Serialize)]
pub struct CombatSearchV2BenchCaseDetail {
    pub case_id: String,
    pub source: Option<String>,
    pub tags: Vec<String>,
    pub deck_summary: Option<Value>,
    pub run_context: Option<Value>,
    pub search_config: CombatSearchV2BenchSearchConfig,
    pub random_boundary: String,
    pub human_baseline: Option<CombatSearchV2TrajectoryReport>,
    pub search_report: Option<CombatSearchV2Report>,
    pub candidate_comparison: Option<Value>,
    pub formal_comparison: Option<Value>,
    pub error: Option<String>,
}

pub fn load_combat_search_v2_start(
    source: &CombatSearchV2StartSource,
) -> Result<CombatSearchV2LoadedStart, String> {
    match source {
        CombatSearchV2StartSource::Case(path) => load_case_start(path),
        CombatSearchV2StartSource::StartSpec(path) => load_start_spec(path),
    }
}

pub fn run_combat_search_v2_loaded_start(
    loaded: &CombatSearchV2LoadedStart,
    baseline_override: Option<CombatSearchV2TrajectoryReport>,
    options: CombatSearchV2RunOptions,
) -> CombatSearchV2SingleRun {
    let baseline = baseline_override.or_else(|| loaded.case_baseline.clone());
    let report = run_combat_search_v2(
        &loaded.position.engine,
        &loaded.position.combat,
        options.to_search_config(loaded.label.clone()),
    );
    let baseline_comparison = baseline.as_ref().map(|baseline| {
        compare_trajectory_reports(
            report.best_complete_trajectory.as_ref(),
            report.outcome.exhaustive,
            baseline,
        )
    });
    CombatSearchV2SingleRun {
        search_report: report,
        baseline_trajectory: baseline,
        baseline_comparison,
    }
}

pub fn load_complete_baseline_from_case_path(
    path: &Path,
) -> Result<Option<CombatSearchV2TrajectoryReport>, String> {
    let case = load_case_from_path(path)?;
    complete_baseline_from_case(&case, true)
}

pub fn replay_case_baseline_trajectory(
    case: &CombatCase,
) -> Result<Option<CombatSearchV2TrajectoryReport>, String> {
    if case.program.is_empty() {
        return Ok(None);
    }

    let seed = lower_case(case)?;
    let initial_hp = seed.combat.entities.player.current_hp;
    let mut engine = seed.engine_state;
    let mut combat = seed.combat;
    let mut actions = Vec::new();
    let mut potions_used = 0u32;
    let mut potions_discarded = 0u32;
    let mut cards_played = 0u32;

    for (step_index, step) in case.program.iter().enumerate() {
        let input = input_for_case_step(step, &engine, &combat).ok_or_else(|| {
            format!(
                "combat case '{}' contains unsupported or invalid step {:?}",
                case.id, step.step
            )
        })?;
        match input {
            ClientInput::UsePotion { .. } => potions_used = potions_used.saturating_add(1),
            ClientInput::DiscardPotion(_) => {
                potions_discarded = potions_discarded.saturating_add(1)
            }
            ClientInput::PlayCard { .. } => cards_played = cards_played.saturating_add(1),
            _ => {}
        }
        actions.push(CombatSearchV2ActionTrace {
            step_index,
            action_id: step_index,
            action_key: format!("baseline/{}", describe_case_step(&step.step)),
            action_debug: format!("{input:?}"),
        });
        let alive = tick_until_stable(&mut engine, &mut combat, input);
        if !alive {
            break;
        }
    }

    Ok(Some(trajectory_from_state(
        engine,
        combat,
        initial_hp,
        actions,
        potions_used,
        potions_discarded,
        cards_played,
        false,
    )))
}

pub fn run_combat_search_v2_benchmark_manifest<E>(
    manifest_path: &Path,
    overrides: CombatSearchV2RunOptions,
    require_baseline: bool,
    mut on_detail: impl FnMut(&CombatSearchV2BenchCaseDetail) -> Result<(), E>,
) -> Result<CombatSearchV2BenchSummaryReport, String>
where
    E: ToString,
{
    let manifest_text = fs::read_to_string(manifest_path).map_err(|err| err.to_string())?;
    let manifest: CombatSearchV2BenchManifest =
        serde_json::from_str(&manifest_text).map_err(|err| err.to_string())?;
    let manifest_dir = manifest_path.parent().unwrap_or_else(|| Path::new("."));
    let mut case_summaries = Vec::new();

    for entry in &manifest.cases {
        let merged_config =
            overrides.merge_bench_config(&manifest.default_search_config, &entry.search_config);
        let detail =
            run_benchmark_case(entry, manifest_dir, merged_config.clone(), require_baseline);
        let summary = summarize_bench_detail(&detail);
        on_detail(&detail).map_err(|err| err.to_string())?;
        case_summaries.push(summary);
    }

    Ok(build_bench_summary(
        manifest_path.display().to_string(),
        manifest.schema_name,
        manifest.schema_version,
        case_summaries,
    ))
}

fn load_case_start(path: &Path) -> Result<CombatSearchV2LoadedStart, String> {
    let case = load_case_from_path(path)?;
    let seed = lower_case(&case)?;
    let case_baseline = complete_baseline_from_case(&case, false)?;
    Ok(CombatSearchV2LoadedStart {
        label: format!("case:{}", path.display()),
        position: CombatPosition::new(seed.engine_state, seed.combat),
        case_baseline,
    })
}

fn load_start_spec(path: &Path) -> Result<CombatSearchV2LoadedStart, String> {
    let payload = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let spec: CombatStartSpec = serde_json::from_str(&payload).map_err(|err| err.to_string())?;
    let (engine, combat) = compile_combat_start_spec(&spec)?;
    Ok(CombatSearchV2LoadedStart {
        label: format!("start_spec:{}", path.display()),
        position: CombatPosition::new(engine, combat),
        case_baseline: None,
    })
}

fn complete_baseline_from_case(
    case: &CombatCase,
    require_baseline: bool,
) -> Result<Option<CombatSearchV2TrajectoryReport>, String> {
    let Some(baseline) = replay_case_baseline_trajectory(case)? else {
        if require_baseline {
            return Err(
                "case has no human program, and a complete baseline is required".to_string(),
            );
        }
        return Ok(None);
    };
    if baseline.terminal == SearchTerminalLabel::Unresolved {
        if require_baseline {
            return Err(format!(
                "case '{}' human program did not reach a terminal whole-combat outcome",
                case.id
            ));
        }
        return Ok(None);
    }
    Ok(Some(baseline))
}

fn run_benchmark_case(
    entry: &CombatSearchV2BenchCaseEntry,
    manifest_dir: &Path,
    search_config: CombatSearchV2BenchSearchConfig,
    require_baseline: bool,
) -> CombatSearchV2BenchCaseDetail {
    let random_boundary = search_config
        .random_boundary
        .clone()
        .unwrap_or_else(|| "engine_truth_v0".to_string());
    let path = resolve_manifest_path(manifest_dir, &entry.combat_case_path);
    let result = run_benchmark_case_inner(entry, &path, &search_config, require_baseline);
    match result {
        Ok((human_baseline, search_report, candidate_comparison, formal_comparison)) => {
            CombatSearchV2BenchCaseDetail {
                case_id: entry.case_id.clone(),
                source: entry.source.clone(),
                tags: entry.tags.clone(),
                deck_summary: entry.deck_summary.clone(),
                run_context: entry.run_context.clone(),
                search_config,
                random_boundary,
                human_baseline,
                search_report: Some(search_report),
                candidate_comparison,
                formal_comparison,
                error: None,
            }
        }
        Err(error) => CombatSearchV2BenchCaseDetail {
            case_id: entry.case_id.clone(),
            source: entry.source.clone(),
            tags: entry.tags.clone(),
            deck_summary: entry.deck_summary.clone(),
            run_context: entry.run_context.clone(),
            search_config,
            random_boundary,
            human_baseline: None,
            search_report: None,
            candidate_comparison: None,
            formal_comparison: None,
            error: Some(error),
        },
    }
}

fn run_benchmark_case_inner(
    entry: &CombatSearchV2BenchCaseEntry,
    case_path: &Path,
    search_config: &CombatSearchV2BenchSearchConfig,
    require_baseline: bool,
) -> Result<
    (
        Option<CombatSearchV2TrajectoryReport>,
        CombatSearchV2Report,
        Option<Value>,
        Option<Value>,
    ),
    String,
> {
    let case = load_case_from_path(case_path)?;
    let seed = lower_case(&case)?;
    let human_baseline = complete_baseline_from_case(&case, require_baseline)?;

    let report = run_combat_search_v2(
        &seed.engine_state,
        &seed.combat,
        search_config
            .to_run_options()
            .to_search_config(entry.case_id.clone()),
    );

    let candidate_comparison = human_baseline.as_ref().and_then(|human| {
        report
            .best_complete_trajectory
            .as_ref()
            .map(|search| candidate_comparison(search, human))
    });
    let formal_comparison = human_baseline.as_ref().map(|human| {
        compare_trajectory_reports(
            report.best_complete_trajectory.as_ref(),
            report.outcome.exhaustive,
            human,
        )
    });
    Ok((
        human_baseline,
        report,
        candidate_comparison,
        formal_comparison,
    ))
}

fn resolve_manifest_path(manifest_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        manifest_dir.join(path)
    }
}

fn candidate_comparison(
    search: &CombatSearchV2TrajectoryReport,
    human: &CombatSearchV2TrajectoryReport,
) -> Value {
    let verdict = compare_candidate(search, human);
    serde_json::json!({
        "verdict": verdict,
        "basis": "best_complete_candidate_whole_combat_outcome",
        "not_a_proof": true,
        "criteria_order": [
            "win_over_loss",
            "higher_final_hp",
            "fewer_potions_used",
            "fewer_turns",
            "fewer_cards_played"
        ],
        "search_terminal": search.terminal,
        "human_terminal": human.terminal,
        "search_final_hp": search.final_hp,
        "human_final_hp": human.final_hp,
        "search_potions_used": search.potions_used,
        "human_potions_used": human.potions_used,
        "search_turns": search.turns,
        "human_turns": human.turns,
    })
}

fn compare_candidate(
    search: &CombatSearchV2TrajectoryReport,
    human: &CombatSearchV2TrajectoryReport,
) -> &'static str {
    match terminal_rank(search.terminal).cmp(&terminal_rank(human.terminal)) {
        std::cmp::Ordering::Greater => return "search_better",
        std::cmp::Ordering::Less => return "human_better",
        std::cmp::Ordering::Equal => {}
    }
    match search.final_hp.cmp(&human.final_hp) {
        std::cmp::Ordering::Greater => return "search_better",
        std::cmp::Ordering::Less => return "human_better",
        std::cmp::Ordering::Equal => {}
    }
    match human.potions_used.cmp(&search.potions_used) {
        std::cmp::Ordering::Greater => return "search_better",
        std::cmp::Ordering::Less => return "human_better",
        std::cmp::Ordering::Equal => {}
    }
    match human.turns.cmp(&search.turns) {
        std::cmp::Ordering::Greater => return "search_better",
        std::cmp::Ordering::Less => return "human_better",
        std::cmp::Ordering::Equal => {}
    }
    match human.cards_played.cmp(&search.cards_played) {
        std::cmp::Ordering::Greater => "search_better",
        std::cmp::Ordering::Less => "human_better",
        std::cmp::Ordering::Equal => "tied",
    }
}

fn terminal_rank(label: SearchTerminalLabel) -> i32 {
    match label {
        SearchTerminalLabel::Win => 2,
        SearchTerminalLabel::Unresolved => 1,
        SearchTerminalLabel::Loss => 0,
    }
}

fn summarize_bench_detail(
    detail: &CombatSearchV2BenchCaseDetail,
) -> CombatSearchV2BenchCaseSummary {
    let search = detail.search_report.as_ref();
    let best = search.and_then(|report| report.best_complete_trajectory.as_ref());
    let human = detail.human_baseline.as_ref();
    CombatSearchV2BenchCaseSummary {
        case_id: detail.case_id.clone(),
        status: if detail.error.is_some() {
            CombatSearchV2BenchCaseStatus::Error
        } else {
            CombatSearchV2BenchCaseStatus::Ok
        },
        source: detail.source.clone(),
        tags: detail.tags.clone(),
        human_baseline_available: human.is_some(),
        human_terminal: human.map(|trajectory| trajectory.terminal),
        human_final_hp: human.map(|trajectory| trajectory.final_hp),
        search_best_terminal: best.map(|trajectory| trajectory.terminal),
        search_best_final_hp: best.map(|trajectory| trajectory.final_hp),
        search_proof_status: search.map(|report| format!("{:?}", report.outcome.proof_status)),
        nodes_to_first_win: search.and_then(|report| report.stats.nodes_to_first_win),
        frontier_remaining: search.map(|report| report.frontier.remaining_states),
        candidate_verdict: detail
            .candidate_comparison
            .as_ref()
            .and_then(|value| value.get("verdict"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        formal_verdict: detail
            .formal_comparison
            .as_ref()
            .and_then(|value| value.get("verdict"))
            .and_then(Value::as_str)
            .map(ToOwned::to_owned),
        random_boundary: detail.random_boundary.clone(),
        error: detail.error.clone(),
    }
}

fn build_bench_summary(
    manifest_path: String,
    manifest_schema_name: Option<String>,
    manifest_schema_version: Option<u32>,
    cases: Vec<CombatSearchV2BenchCaseSummary>,
) -> CombatSearchV2BenchSummaryReport {
    let human_baseline_available = cases
        .iter()
        .filter(|case| case.human_baseline_available)
        .count();
    let human_wins = cases
        .iter()
        .filter(|case| case.human_terminal == Some(SearchTerminalLabel::Win))
        .count();
    let search_found_complete = cases
        .iter()
        .filter(|case| case.search_best_terminal.is_some())
        .count();
    let search_found_win = cases
        .iter()
        .filter(|case| case.search_best_terminal == Some(SearchTerminalLabel::Win))
        .count();
    let candidate_search_better = cases
        .iter()
        .filter(|case| case.candidate_verdict.as_deref() == Some("search_better"))
        .count();
    let candidate_search_tied = cases
        .iter()
        .filter(|case| case.candidate_verdict.as_deref() == Some("tied"))
        .count();
    let candidate_human_better = cases
        .iter()
        .filter(|case| case.candidate_verdict.as_deref() == Some("human_better"))
        .count();
    let formal_inconclusive = cases
        .iter()
        .filter(|case| {
            case.formal_verdict
                .as_deref()
                .is_some_and(|verdict| verdict.starts_with("inconclusive"))
        })
        .count();
    let errors = cases
        .iter()
        .filter(|case| matches!(case.status, CombatSearchV2BenchCaseStatus::Error))
        .count();
    let nodes_to_first_win = cases
        .iter()
        .filter_map(|case| case.nodes_to_first_win.map(|value| value as f64))
        .collect::<Vec<_>>();
    let best_hp = cases
        .iter()
        .filter_map(|case| case.search_best_final_hp.map(|value| value as f64))
        .collect::<Vec<_>>();
    CombatSearchV2BenchSummaryReport {
        schema_name: "CombatSearchV2BenchReport",
        schema_version: 1,
        manifest_path,
        manifest_schema_name,
        manifest_schema_version,
        case_count: cases.len(),
        human_baseline_available,
        human_wins,
        search_found_complete,
        search_found_win,
        candidate_search_better,
        candidate_search_tied,
        candidate_human_better,
        formal_inconclusive,
        errors,
        median_nodes_to_first_win: median(nodes_to_first_win),
        median_best_complete_final_hp: median(best_hp),
        cases,
    }
}

fn median(mut values: Vec<f64>) -> Option<f64> {
    if values.is_empty() {
        return None;
    }
    values.sort_by(|left, right| left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal));
    let mid = values.len() / 2;
    if values.len() % 2 == 0 {
        Some((values[mid - 1] + values[mid]) / 2.0)
    } else {
        Some(values[mid])
    }
}
