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

use super::{load_combat_search_v2_start, CombatSearchV2LoadedStart, CombatSearchV2RunOptions};

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatSearchV2BenchmarkSpec {
    pub name: String,
    pub cases: Vec<CombatSearchV2BenchmarkCaseSpec>,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CombatSearchV2BenchmarkCaseSpec {
    pub id: String,
    pub start_spec: PathBuf,
    #[serde(default)]
    pub baseline: Option<CombatSearchV2BaselineOutcomeSpec>,
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

#[derive(Clone)]
pub struct CombatSearchV2LoadedBenchmark {
    pub name: String,
    pub cases: Vec<CombatSearchV2LoadedBenchmarkCase>,
}

#[derive(Clone)]
pub struct CombatSearchV2LoadedBenchmarkCase {
    pub id: String,
    pub start_spec_path: PathBuf,
    pub start: CombatSearchV2LoadedStart,
    pub baseline: Option<CombatSearchV2BaselineOutcomeSpec>,
}

#[derive(Clone, Debug, Serialize)]
pub struct CombatSearchV2BenchmarkReport {
    pub schema_name: &'static str,
    pub schema_version: u32,
    pub benchmark_name: String,
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
    pub start_spec_path: String,
    pub outcome: CombatSearchV2OutcomeReport,
    pub best_complete_trajectory: Option<CombatSearchV2TrajectoryReport>,
    pub diagnostics: CombatSearchV2DiagnosticsReport,
    pub stats: CombatSearchV2Stats,
    pub baseline: Option<CombatSearchV2BaselineOutcomeSpec>,
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
        let start_spec_path = resolve_manifest_relative_path(base_dir, &case.start_spec);
        let mut start = load_combat_search_v2_start(&start_spec_path)
            .map_err(|err| format!("case '{}' start_spec failed: {err}", case.id))?;
        start.label = format!(
            "benchmark:{}:case:{}:start_spec:{}",
            spec.name,
            case.id,
            start_spec_path.display()
        );
        cases.push(CombatSearchV2LoadedBenchmarkCase {
            id: case.id,
            start_spec_path,
            start,
            baseline: case.baseline,
        });
    }

    Ok(CombatSearchV2LoadedBenchmark {
        name: spec.name,
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
        start_spec_path: case.start_spec_path.display().to_string(),
        outcome: search_report.outcome.clone(),
        best_complete_trajectory: search_report.best_complete_trajectory.clone(),
        diagnostics: search_report.diagnostics.clone(),
        stats: search_report.stats.clone(),
        baseline: case.baseline.clone(),
        baseline_comparison,
    }
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

fn resolve_manifest_relative_path(base_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
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
        assert!(loaded.cases[0].baseline.is_some());

        let _ = fs::remove_file(start_path);
        let _ = fs::remove_file(benchmark_path);
        let _ = fs::remove_dir(dir);
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
}
