use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::ai::combat_search_v2::{
    compare_outcome_metrics, run_combat_search_v2, CombatSearchV2DiagnosticsReport,
    CombatSearchV2OutcomeMetrics, CombatSearchV2OutcomeReport, CombatSearchV2Report,
    CombatSearchV2Stats, CombatSearchV2TrajectoryReport, SearchProofStatus, SearchTerminalLabel,
    WHOLE_COMBAT_OUTCOME_CRITERIA,
};
use crate::eval::artifact::ArtifactTrustLevel;
use crate::eval::fingerprint::StateFingerprintV1;
use crate::eval::run_control::load_combat_baseline_outcome_v1;
use crate::sim::combat::CombatTerminal;

use super::{
    load_combat_search_v2_snapshot, load_combat_search_v2_start, CombatSearchV2LoadedStart,
    CombatSearchV2RunOptions,
};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatSearchV2BenchmarkSpec {
    #[serde(default = "default_benchmark_schema_name")]
    pub schema_name: String,
    #[serde(default = "default_benchmark_schema_version")]
    pub schema_version: u32,
    pub name: String,
    #[serde(default = "default_benchmark_min_trust_level")]
    pub min_trust_level: ArtifactTrustLevel,
    pub cases: Vec<CombatSearchV2BenchmarkCaseSpec>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatSearchV2BenchmarkCaseSpec {
    pub id: String,
    #[serde(default)]
    pub start_spec: Option<PathBuf>,
    #[serde(default)]
    pub combat_snapshot: Option<PathBuf>,
    #[serde(default)]
    pub expected_fingerprints: Option<CombatSearchV2BenchmarkExpectedFingerprints>,
    #[serde(default)]
    pub baseline: Option<CombatSearchV2BenchmarkBaselineSpec>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatSearchV2BenchmarkExpectedFingerprints {
    pub public_observation_hash: String,
    pub legal_candidate_set_hash: String,
    #[serde(default)]
    pub legal_candidate_order_hash: Option<String>,
    #[serde(default)]
    pub exact_state_hash: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CombatSearchV2BaselineOutcomeSpec {
    pub terminal: SearchTerminalLabel,
    pub final_hp: i32,
    pub potions_used: u32,
    pub turns: u32,
    pub cards_played: u32,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(untagged)]
pub enum CombatSearchV2BenchmarkBaselineSpec {
    Inline(CombatSearchV2BaselineOutcomeSpec),
    Path(PathBuf),
}

#[derive(Clone)]
pub struct CombatSearchV2LoadedBenchmark {
    pub name: String,
    pub min_trust_level: ArtifactTrustLevel,
    pub cases: Vec<CombatSearchV2LoadedBenchmarkCase>,
}

#[derive(Clone)]
pub struct CombatSearchV2LoadedBenchmarkCase {
    pub id: String,
    pub input: CombatSearchV2LoadedBenchmarkInput,
    pub start: CombatSearchV2LoadedStart,
    pub expected_fingerprints: Option<CombatSearchV2BenchmarkExpectedFingerprints>,
    pub baseline: Option<CombatSearchV2BaselineOutcomeSpec>,
    pub baseline_path: Option<PathBuf>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CombatSearchV2LoadedBenchmarkInput {
    pub kind: CombatSearchV2BenchmarkInputKind,
    pub path: PathBuf,
}

impl CombatSearchV2LoadedBenchmarkInput {
    fn new(kind: CombatSearchV2BenchmarkInputKind, path: PathBuf) -> Self {
        Self { kind, path }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchV2BenchmarkInputKind {
    StartSpec,
    CombatSnapshot,
}

impl CombatSearchV2BenchmarkInputKind {
    fn as_label(self) -> &'static str {
        match self {
            CombatSearchV2BenchmarkInputKind::StartSpec => "start_spec",
            CombatSearchV2BenchmarkInputKind::CombatSnapshot => "combat_snapshot",
        }
    }
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2BenchmarkReport {
    pub schema_name: &'static str,
    pub schema_version: u32,
    pub benchmark_name: String,
    pub min_trust_level: ArtifactTrustLevel,
    pub case_count: usize,
    pub summary: CombatSearchV2BenchmarkSummary,
    pub cases: Vec<CombatSearchV2BenchmarkCaseReport>,
}

#[derive(Clone, Debug, Default, Serialize)]
pub struct CombatSearchV2BenchmarkSummary {
    pub wins: usize,
    pub losses: usize,
    pub unresolved: usize,
    pub exhaustive: usize,
    pub complete_trajectory_found: usize,
    pub budget_exhausted: usize,
    pub deadline_hit: usize,
    pub baseline_cases: usize,
    pub search_better: usize,
    pub search_tied: usize,
    pub baseline_better: usize,
    pub inconclusive: usize,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2BenchmarkCaseReport {
    pub id: String,
    pub start_label: String,
    pub input_kind: CombatSearchV2BenchmarkInputKind,
    pub input_path: String,
    pub start_spec_path: Option<String>,
    pub combat_snapshot_path: Option<String>,
    pub input_trust_level: Option<ArtifactTrustLevel>,
    pub input_fingerprints: Option<StateFingerprintV1>,
    pub outcome: CombatSearchV2OutcomeReport,
    pub best_complete_trajectory: Option<CombatSearchV2TrajectoryReport>,
    pub diagnostics: CombatSearchV2DiagnosticsReport,
    pub stats: CombatSearchV2Stats,
    pub baseline: Option<CombatSearchV2BaselineOutcomeSpec>,
    pub baseline_path: Option<String>,
    pub baseline_comparison: Option<CombatSearchV2BaselineComparison>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2BaselineComparison {
    pub verdict: CombatSearchV2BaselineVerdict,
    pub basis: &'static str,
    pub reason: Option<&'static str>,
    pub criteria_order: Vec<&'static str>,
    pub search_terminal: Option<SearchTerminalLabel>,
    pub baseline_terminal: SearchTerminalLabel,
    pub search_final_hp: Option<i32>,
    pub baseline_final_hp: i32,
    pub search_potions_used: Option<u32>,
    pub baseline_potions_used: u32,
    pub search_turns: Option<u32>,
    pub baseline_turns: u32,
    pub search_cards_played: Option<u32>,
    pub baseline_cards_played: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CombatSearchV2BaselineVerdict {
    SearchBetter,
    SearchTied,
    BaselineBetter,
    Inconclusive,
    InconclusiveUnresolvedSearch,
}

pub fn load_combat_search_v2_benchmark(
    path: &Path,
) -> Result<CombatSearchV2LoadedBenchmark, String> {
    let payload = fs::read_to_string(path).map_err(|err| err.to_string())?;
    let spec: CombatSearchV2BenchmarkSpec =
        serde_json::from_str(&payload).map_err(|err| err.to_string())?;
    if spec.schema_name != default_benchmark_schema_name() {
        return Err(format!(
            "unsupported combat search benchmark schema '{}'",
            spec.schema_name
        ));
    }
    if spec.schema_version != default_benchmark_schema_version() {
        return Err(format!(
            "unsupported combat search benchmark schema_version {}",
            spec.schema_version
        ));
    }
    if spec.name.trim().is_empty() {
        return Err("CombatSearchV2BenchmarkSpec requires a non-empty name".to_string());
    }
    if spec.cases.is_empty() {
        return Err("CombatSearchV2BenchmarkSpec requires at least one case".to_string());
    }

    let base_dir = path.parent().unwrap_or_else(|| Path::new(""));
    let mut ids = HashSet::new();
    let mut cases = Vec::with_capacity(spec.cases.len());
    for case in spec.cases {
        if case.id.trim().is_empty() {
            return Err("CombatSearchV2BenchmarkSpec case id cannot be empty".to_string());
        }
        if !ids.insert(case.id.clone()) {
            return Err(format!(
                "duplicate CombatSearchV2 benchmark case id '{}'",
                case.id
            ));
        }
        let (input, mut start) = load_benchmark_case_input(base_dir, &spec.name, &case)?;
        validate_benchmark_case_artifact(&case, &start, spec.min_trust_level)?;
        let (baseline, baseline_path) = load_benchmark_case_baseline(base_dir, &case)?;
        start.label = format!(
            "benchmark:{}:case:{}:{}:{}",
            spec.name,
            case.id,
            input.kind.as_label(),
            input.path.display()
        );
        cases.push(CombatSearchV2LoadedBenchmarkCase {
            id: case.id,
            input,
            start,
            expected_fingerprints: case.expected_fingerprints,
            baseline,
            baseline_path,
        });
    }

    Ok(CombatSearchV2LoadedBenchmark {
        name: spec.name,
        min_trust_level: spec.min_trust_level,
        cases,
    })
}

pub fn run_combat_search_v2_benchmark(
    loaded: &CombatSearchV2LoadedBenchmark,
    options: CombatSearchV2RunOptions,
) -> CombatSearchV2BenchmarkReport {
    let cases = loaded
        .cases
        .iter()
        .map(|case| run_combat_search_v2_benchmark_case(case, options.clone()))
        .collect::<Vec<_>>();
    let summary = summarize_benchmark_cases(&cases);

    CombatSearchV2BenchmarkReport {
        schema_name: "CombatSearchV2BenchmarkReport",
        schema_version: 1,
        benchmark_name: loaded.name.clone(),
        min_trust_level: loaded.min_trust_level,
        case_count: cases.len(),
        summary,
        cases,
    }
}

fn run_combat_search_v2_benchmark_case(
    case: &CombatSearchV2LoadedBenchmarkCase,
    options: CombatSearchV2RunOptions,
) -> CombatSearchV2BenchmarkCaseReport {
    let search_report = run_combat_search_v2(
        &case.start.position.engine,
        &case.start.position.combat,
        options.to_search_config(case.start.label.clone()),
    );
    let baseline_comparison = case
        .baseline
        .as_ref()
        .map(|baseline| compare_search_to_baseline_outcome(&search_report, baseline));

    CombatSearchV2BenchmarkCaseReport {
        id: case.id.clone(),
        start_label: case.start.label.clone(),
        input_kind: case.input.kind,
        input_path: case.input.path.display().to_string(),
        start_spec_path: (case.input.kind == CombatSearchV2BenchmarkInputKind::StartSpec)
            .then(|| case.input.path.display().to_string()),
        combat_snapshot_path: (case.input.kind == CombatSearchV2BenchmarkInputKind::CombatSnapshot)
            .then(|| case.input.path.display().to_string()),
        input_trust_level: case.start.artifact_trust_level,
        input_fingerprints: case.start.fingerprints.clone(),
        outcome: search_report.outcome.clone(),
        best_complete_trajectory: search_report.best_complete_trajectory.clone(),
        diagnostics: search_report.diagnostics.clone(),
        stats: search_report.stats.clone(),
        baseline: case.baseline.clone(),
        baseline_path: case
            .baseline_path
            .as_ref()
            .map(|path| path.display().to_string()),
        baseline_comparison,
    }
}

fn validate_benchmark_case_artifact(
    case: &CombatSearchV2BenchmarkCaseSpec,
    start: &CombatSearchV2LoadedStart,
    min_trust_level: ArtifactTrustLevel,
) -> Result<(), String> {
    if case.combat_snapshot.is_some() {
        let trust = start
            .artifact_trust_level
            .ok_or_else(|| format!("case '{}' combat_snapshot is missing trust level", case.id))?;
        if !trust.satisfies_minimum(min_trust_level) {
            return Err(format!(
                "case '{}' combat_snapshot trust level {:?} does not satisfy minimum {:?}",
                case.id, trust, min_trust_level
            ));
        }
    }
    if let Some(expected) = case.expected_fingerprints.as_ref() {
        let actual = start.fingerprints.as_ref().ok_or_else(|| {
            format!(
                "case '{}' has expected_fingerprints but input has no artifact fingerprints",
                case.id
            )
        })?;
        if actual.public_observation_hash != expected.public_observation_hash {
            return Err(format!(
                "case '{}' public_observation_hash drift: expected {}, got {}",
                case.id, expected.public_observation_hash, actual.public_observation_hash
            ));
        }
        if actual.legal_candidate_set_hash != expected.legal_candidate_set_hash {
            return Err(format!(
                "case '{}' legal_candidate_set_hash drift: expected {}, got {}",
                case.id, expected.legal_candidate_set_hash, actual.legal_candidate_set_hash
            ));
        }
        if let Some(expected_order) = expected.legal_candidate_order_hash.as_ref() {
            if actual.legal_candidate_order_hash != *expected_order {
                return Err(format!(
                    "case '{}' legal_candidate_order_hash drift: expected {}, got {}",
                    case.id, expected_order, actual.legal_candidate_order_hash
                ));
            }
        }
        if let Some(expected_exact) = expected.exact_state_hash.as_ref() {
            if actual.exact_state_hash != *expected_exact {
                return Err(format!(
                    "case '{}' exact_state_hash drift: expected {}, got {}",
                    case.id, expected_exact, actual.exact_state_hash
                ));
            }
        }
    }
    Ok(())
}

fn summarize_benchmark_cases(
    cases: &[CombatSearchV2BenchmarkCaseReport],
) -> CombatSearchV2BenchmarkSummary {
    let mut summary = CombatSearchV2BenchmarkSummary::default();
    for case in cases {
        match case.outcome.terminal {
            SearchTerminalLabel::Win => summary.wins += 1,
            SearchTerminalLabel::Loss => summary.losses += 1,
            SearchTerminalLabel::Unresolved => summary.unresolved += 1,
        }
        if case.outcome.exhaustive {
            summary.exhaustive += 1;
        }
        if case.outcome.complete_trajectory_found {
            summary.complete_trajectory_found += 1;
        }
        match case.outcome.proof_status {
            SearchProofStatus::BudgetExhausted => summary.budget_exhausted += 1,
            SearchProofStatus::DeadlineHit => summary.deadline_hit += 1,
            SearchProofStatus::Exhaustive | SearchProofStatus::FrontierUnresolved => {}
        }
        if let Some(comparison) = &case.baseline_comparison {
            summary.baseline_cases += 1;
            match comparison.verdict {
                CombatSearchV2BaselineVerdict::SearchBetter => summary.search_better += 1,
                CombatSearchV2BaselineVerdict::SearchTied => summary.search_tied += 1,
                CombatSearchV2BaselineVerdict::BaselineBetter => summary.baseline_better += 1,
                _ => summary.inconclusive += 1,
            }
        }
    }
    summary
}

fn compare_search_to_baseline_outcome(
    search_report: &CombatSearchV2Report,
    baseline: &CombatSearchV2BaselineOutcomeSpec,
) -> CombatSearchV2BaselineComparison {
    let criteria_order = WHOLE_COMBAT_OUTCOME_CRITERIA.to_vec();
    let Some(search) = search_report.best_complete_trajectory.as_ref() else {
        return inconclusive_baseline_comparison(
            baseline,
            criteria_order,
            "no_search_complete_trajectory",
        );
    };
    if !search_report.outcome.exhaustive || search.terminal == SearchTerminalLabel::Unresolved {
        return CombatSearchV2BaselineComparison {
            verdict: CombatSearchV2BaselineVerdict::InconclusiveUnresolvedSearch,
            basis: "whole_combat_outcome",
            reason: Some(
                "search has unresolved frontier and cannot claim not-weaker-than-baseline",
            ),
            criteria_order,
            search_terminal: Some(search.terminal),
            baseline_terminal: baseline.terminal,
            search_final_hp: Some(search.final_hp),
            baseline_final_hp: baseline.final_hp,
            search_potions_used: Some(search.potions_used),
            baseline_potions_used: baseline.potions_used,
            search_turns: Some(search.turns),
            baseline_turns: baseline.turns,
            search_cards_played: Some(search.cards_played),
            baseline_cards_played: baseline.cards_played,
        };
    }

    let ordering = compare_outcome_metrics(
        CombatSearchV2OutcomeMetrics::from_trajectory(search),
        CombatSearchV2OutcomeMetrics {
            terminal: baseline.terminal,
            final_hp: baseline.final_hp,
            potions_used: baseline.potions_used,
            turns: baseline.turns,
            cards_played: baseline.cards_played,
        },
    );
    CombatSearchV2BaselineComparison {
        verdict: match ordering {
            std::cmp::Ordering::Greater => CombatSearchV2BaselineVerdict::SearchBetter,
            std::cmp::Ordering::Equal => CombatSearchV2BaselineVerdict::SearchTied,
            std::cmp::Ordering::Less => CombatSearchV2BaselineVerdict::BaselineBetter,
        },
        basis: "whole_combat_outcome",
        reason: None,
        criteria_order,
        search_terminal: Some(search.terminal),
        baseline_terminal: baseline.terminal,
        search_final_hp: Some(search.final_hp),
        baseline_final_hp: baseline.final_hp,
        search_potions_used: Some(search.potions_used),
        baseline_potions_used: baseline.potions_used,
        search_turns: Some(search.turns),
        baseline_turns: baseline.turns,
        search_cards_played: Some(search.cards_played),
        baseline_cards_played: baseline.cards_played,
    }
}

fn inconclusive_baseline_comparison(
    baseline: &CombatSearchV2BaselineOutcomeSpec,
    criteria_order: Vec<&'static str>,
    reason: &'static str,
) -> CombatSearchV2BaselineComparison {
    CombatSearchV2BaselineComparison {
        verdict: CombatSearchV2BaselineVerdict::Inconclusive,
        basis: "whole_combat_outcome",
        reason: Some(reason),
        criteria_order,
        search_terminal: None,
        baseline_terminal: baseline.terminal,
        search_final_hp: None,
        baseline_final_hp: baseline.final_hp,
        search_potions_used: None,
        baseline_potions_used: baseline.potions_used,
        search_turns: None,
        baseline_turns: baseline.turns,
        search_cards_played: None,
        baseline_cards_played: baseline.cards_played,
    }
}

fn load_benchmark_case_input(
    base_dir: &Path,
    benchmark_name: &str,
    case: &CombatSearchV2BenchmarkCaseSpec,
) -> Result<
    (
        CombatSearchV2LoadedBenchmarkInput,
        CombatSearchV2LoadedStart,
    ),
    String,
> {
    match (case.start_spec.as_ref(), case.combat_snapshot.as_ref()) {
        (Some(start_spec), None) => {
            let path = resolve_manifest_relative_path(base_dir, start_spec);
            let start = load_combat_search_v2_start(&path)
                .map_err(|err| format!("case '{}' start_spec failed: {err}", case.id))?;
            Ok((
                CombatSearchV2LoadedBenchmarkInput::new(
                    CombatSearchV2BenchmarkInputKind::StartSpec,
                    path,
                ),
                start,
            ))
        }
        (None, Some(combat_snapshot)) => {
            let path = resolve_manifest_relative_path(base_dir, combat_snapshot);
            let start = load_combat_search_v2_snapshot(&path)
                .map_err(|err| format!("case '{}' combat_snapshot failed: {err}", case.id))?;
            Ok((
                CombatSearchV2LoadedBenchmarkInput::new(
                    CombatSearchV2BenchmarkInputKind::CombatSnapshot,
                    path,
                ),
                start,
            ))
        }
        (None, None) => Err(format!(
            "benchmark '{benchmark_name}' case '{}' requires exactly one of start_spec or combat_snapshot",
            case.id
        )),
        (Some(_), Some(_)) => Err(format!(
            "benchmark '{benchmark_name}' case '{}' cannot set both start_spec and combat_snapshot",
            case.id
        )),
    }
}

fn load_benchmark_case_baseline(
    base_dir: &Path,
    case: &CombatSearchV2BenchmarkCaseSpec,
) -> Result<(Option<CombatSearchV2BaselineOutcomeSpec>, Option<PathBuf>), String> {
    match case.baseline.as_ref() {
        None => Ok((None, None)),
        Some(CombatSearchV2BenchmarkBaselineSpec::Inline(inline)) => {
            Ok((Some(inline.clone()), None))
        }
        Some(CombatSearchV2BenchmarkBaselineSpec::Path(path)) => {
            let path = resolve_manifest_relative_path(base_dir, path);
            let baseline = load_combat_baseline_outcome_v1(&path)
                .map_err(|err| format!("case '{}' baseline failed: {err}", case.id))?;
            Ok((
                Some(CombatSearchV2BaselineOutcomeSpec {
                    terminal: search_terminal_from_combat_terminal(baseline.terminal()),
                    final_hp: baseline.final_hp,
                    potions_used: baseline.potions_used,
                    turns: baseline.turns,
                    cards_played: baseline.cards_played,
                }),
                Some(path),
            ))
        }
    }
}

fn search_terminal_from_combat_terminal(terminal: CombatTerminal) -> SearchTerminalLabel {
    match terminal {
        CombatTerminal::Win => SearchTerminalLabel::Win,
        CombatTerminal::Loss => SearchTerminalLabel::Loss,
        CombatTerminal::Unresolved => SearchTerminalLabel::Unresolved,
    }
}

fn resolve_manifest_relative_path(base_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn default_benchmark_schema_name() -> String {
    "CombatSearchV2BenchmarkSuiteV1".to_string()
}

fn default_benchmark_schema_version() -> u32 {
    1
}

fn default_benchmark_min_trust_level() -> ArtifactTrustLevel {
    ArtifactTrustLevel::Restorable
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::eval::combat_capture::{capture_combat_position_v1, save_combat_capture_v1};
    use crate::fixtures::combat_start_spec::{compile_combat_start_spec, CombatStartSpec};
    use crate::sim::combat::CombatPosition;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn benchmark_loader_accepts_relative_start_spec_only() {
        let dir = unique_temp_dir("combat_search_v2_benchmark_loader");
        fs::create_dir_all(&dir).expect("temp dir should be created");
        let start_path = dir.join("jaw_worm.json");
        let benchmark_path = dir.join("benchmark.json");
        fs::write(&start_path, starter_jaw_worm_start_spec())
            .expect("start spec should be written");
        fs::write(
            &benchmark_path,
            r#"{
                "name": "smoke",
                "cases": [
                    {
                        "id": "jaw_worm",
                        "start_spec": "jaw_worm.json",
                        "baseline": {
                            "terminal": "win",
                            "final_hp": 70,
                            "potions_used": 0,
                            "turns": 4,
                            "cards_played": 9
                        }
                    }
                ]
            }"#,
        )
        .expect("benchmark spec should be written");

        let loaded = load_combat_search_v2_benchmark(&benchmark_path)
            .expect("benchmark should load from relative start-spec path");

        assert_eq!(loaded.name, "smoke");
        assert_eq!(loaded.cases.len(), 1);
        assert_eq!(loaded.cases[0].id, "jaw_worm");
        assert_eq!(
            loaded.cases[0].input.kind,
            CombatSearchV2BenchmarkInputKind::StartSpec
        );
        assert!(loaded.cases[0].baseline.is_some());

        let _ = fs::remove_file(start_path);
        let _ = fs::remove_file(benchmark_path);
        let _ = fs::remove_dir(dir);
    }

    #[test]
    fn benchmark_loader_accepts_relative_combat_snapshot_only() {
        let dir = unique_temp_dir("combat_search_v2_snapshot_benchmark_loader");
        fs::create_dir_all(&dir).expect("temp dir should be created");
        let snapshot_path = dir.join("jaw_worm.capture.json");
        let benchmark_path = dir.join("benchmark.json");

        let position = jaw_worm_position();
        let capture = capture_combat_position_v1(Some("jaw_worm".to_string()), &position)
            .expect("stable position should capture");
        save_combat_capture_v1(&snapshot_path, &capture).expect("capture should be written");
        fs::write(
            &benchmark_path,
            r#"{
                "name": "smoke",
                "cases": [
                    {
                        "id": "jaw_worm_snapshot",
                        "combat_snapshot": "jaw_worm.capture.json"
                    }
                ]
            }"#,
        )
        .expect("benchmark spec should be written");

        let loaded = load_combat_search_v2_benchmark(&benchmark_path)
            .expect("benchmark should load from relative combat-snapshot path");

        assert_eq!(loaded.name, "smoke");
        assert_eq!(loaded.cases.len(), 1);
        assert_eq!(loaded.cases[0].id, "jaw_worm_snapshot");
        assert_eq!(
            loaded.cases[0].input.kind,
            CombatSearchV2BenchmarkInputKind::CombatSnapshot
        );
        assert_eq!(loaded.cases[0].start.position, position);
        assert_eq!(
            loaded.cases[0].start.artifact_trust_level,
            Some(ArtifactTrustLevel::Restorable)
        );
        assert!(loaded.cases[0].start.fingerprints.is_some());

        let _ = fs::remove_file(snapshot_path);
        let _ = fs::remove_file(benchmark_path);
        let _ = fs::remove_dir(dir);
    }

    #[test]
    fn benchmark_loader_accepts_expected_snapshot_fingerprints() {
        let dir = unique_temp_dir("combat_search_v2_expected_fingerprint_loader");
        fs::create_dir_all(&dir).expect("temp dir should be created");
        let snapshot_path = dir.join("jaw_worm.capture.json");
        let benchmark_path = dir.join("benchmark.json");

        let capture =
            capture_combat_position_v1(Some("jaw_worm".to_string()), &jaw_worm_position())
                .expect("stable position should capture");
        let fingerprints = capture
            .fingerprints
            .as_ref()
            .expect("capture should have fingerprints")
            .clone();
        save_combat_capture_v1(&snapshot_path, &capture).expect("capture should be written");
        fs::write(
            &benchmark_path,
            format!(
                r#"{{
                    "schema_name": "CombatSearchV2BenchmarkSuiteV1",
                    "schema_version": 1,
                    "name": "smoke",
                    "min_trust_level": "restorable",
                    "cases": [
                        {{
                            "id": "jaw_worm_snapshot",
                            "combat_snapshot": "jaw_worm.capture.json",
                            "expected_fingerprints": {{
                                "public_observation_hash": "{}",
                                "legal_candidate_set_hash": "{}",
                                "legal_candidate_order_hash": "{}",
                                "exact_state_hash": "{}"
                            }}
                        }}
                    ]
                }}"#,
                fingerprints.public_observation_hash,
                fingerprints.legal_candidate_set_hash,
                fingerprints.legal_candidate_order_hash,
                fingerprints.exact_state_hash
            ),
        )
        .expect("benchmark spec should be written");

        let loaded = load_combat_search_v2_benchmark(&benchmark_path)
            .expect("benchmark should accept matching fingerprints");

        assert!(loaded.cases[0].expected_fingerprints.is_some());

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn benchmark_loader_rejects_expected_snapshot_fingerprint_drift() {
        let dir = unique_temp_dir("combat_search_v2_expected_fingerprint_drift");
        fs::create_dir_all(&dir).expect("temp dir should be created");
        let snapshot_path = dir.join("jaw_worm.capture.json");
        let benchmark_path = dir.join("benchmark.json");

        let capture =
            capture_combat_position_v1(Some("jaw_worm".to_string()), &jaw_worm_position())
                .expect("stable position should capture");
        let fingerprints = capture
            .fingerprints
            .as_ref()
            .expect("capture should have fingerprints")
            .clone();
        save_combat_capture_v1(&snapshot_path, &capture).expect("capture should be written");
        fs::write(
            &benchmark_path,
            format!(
                r#"{{
                    "name": "smoke",
                    "cases": [
                        {{
                            "id": "jaw_worm_snapshot",
                            "combat_snapshot": "jaw_worm.capture.json",
                            "expected_fingerprints": {{
                                "public_observation_hash": "0000000000000000000000000000000000000000000000000000000000000000",
                                "legal_candidate_set_hash": "{}"
                            }}
                        }}
                    ]
                }}"#,
                fingerprints.legal_candidate_set_hash
            ),
        )
        .expect("benchmark spec should be written");

        let err = match load_combat_search_v2_benchmark(&benchmark_path) {
            Ok(_) => panic!("benchmark should reject fingerprint drift"),
            Err(err) => err,
        };

        assert!(err.contains("public_observation_hash drift"));

        let _ = fs::remove_dir_all(dir);
    }

    #[test]
    fn benchmark_loader_accepts_relative_baseline_path() {
        let dir = unique_temp_dir("combat_search_v2_baseline_path_loader");
        fs::create_dir_all(dir.join("captures")).expect("capture dir should be created");
        fs::create_dir_all(dir.join("baselines")).expect("baseline dir should be created");
        let snapshot_path = dir.join("captures").join("jaw.capture.json");
        let baseline_path = dir.join("baselines").join("jaw.baseline.json");
        let benchmark_path = dir.join("benchmark.json");

        let position = jaw_worm_position();
        let capture = capture_combat_position_v1(Some("jaw".to_string()), &position)
            .expect("stable position should capture");
        save_combat_capture_v1(&snapshot_path, &capture).expect("capture should be written");
        fs::write(
            &baseline_path,
            r#"{
                "schema_name": "CombatBaselineOutcomeV1",
                "schema_version": 1,
                "case_id": "jaw",
                "terminal": "win",
                "start_hp": 80,
                "final_hp": 70,
                "hp_loss": 10,
                "turns": 4,
                "potions_used": 0,
                "potions_discarded": 0,
                "cards_played": 9
            }"#,
        )
        .expect("baseline should be written");
        fs::write(
            &benchmark_path,
            r#"{
                "name": "baseline_path",
                "cases": [
                    {
                        "id": "jaw",
                        "combat_snapshot": "captures/jaw.capture.json",
                        "baseline": "baselines/jaw.baseline.json"
                    }
                ]
            }"#,
        )
        .expect("benchmark should be written");

        let loaded = load_combat_search_v2_benchmark(&benchmark_path)
            .expect("benchmark should load baseline path");

        assert_eq!(loaded.cases[0].baseline.as_ref().unwrap().final_hp, 70);
        assert_eq!(
            loaded.cases[0].baseline_path.as_ref().unwrap(),
            &baseline_path
        );

        let _ = fs::remove_dir_all(dir);
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{label}_{}_{}", std::process::id(), nanos))
    }

    fn starter_jaw_worm_start_spec() -> &'static str {
        r#"{
            "name": "jaw_worm_starter",
            "player_class": "Ironclad",
            "ascension_level": 0,
            "encounter_id": "JawWorm",
            "room_type": "monster",
            "seed": 1,
            "player_current_hp": 80,
            "player_max_hp": 80,
            "master_deck": [
                {"id": "Strike_R", "count": 5},
                {"id": "Defend_R", "count": 4},
                "Bash"
            ]
        }"#
    }

    fn jaw_worm_position() -> CombatPosition {
        let spec: CombatStartSpec = serde_json::from_str(starter_jaw_worm_start_spec())
            .expect("test start spec should parse");
        let (engine, combat) =
            compile_combat_start_spec(&spec).expect("test start spec should compile");
        CombatPosition::new(engine, combat)
    }
}
