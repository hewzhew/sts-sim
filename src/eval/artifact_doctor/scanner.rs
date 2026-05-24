use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use blake2::{Blake2b512, Digest};

use crate::eval::combat_capture::{load_combat_capture_v1, CombatCaptureV1};
use crate::eval::combat_search_v2::{
    load_combat_search_v2_benchmark, load_combat_search_v2_snapshot, load_combat_search_v2_start,
    CombatSearchV2BenchmarkBaselineSpec, CombatSearchV2BenchmarkCaseSpec,
    CombatSearchV2BenchmarkSpec,
};
use crate::eval::run_control::{load_combat_baseline_outcome_v1, load_combat_search_evidence_v1};

use super::report::{
    ArtifactAuditCheckV1, ArtifactAuditReportV1, ArtifactAuditStatus, ArtifactAuditSummaryV1,
    ARTIFACT_AUDIT_SCHEMA_NAME, ARTIFACT_AUDIT_SCHEMA_VERSION,
};

pub fn audit_artifacts(root: &Path) -> ArtifactAuditReportV1 {
    let mut auditor = ArtifactAuditor::new(root);
    auditor.run()
}

struct ArtifactAuditor {
    root: PathBuf,
    checks: Vec<ArtifactAuditCheckV1>,
    suites_found: usize,
    cases_found: usize,
    captures_referenced: BTreeSet<PathBuf>,
    baselines_referenced: BTreeSet<PathBuf>,
    search_evidence_found: BTreeSet<PathBuf>,
}

impl ArtifactAuditor {
    fn new(root: &Path) -> Self {
        Self {
            root: root.to_path_buf(),
            checks: Vec::new(),
            suites_found: 0,
            cases_found: 0,
            captures_referenced: BTreeSet::new(),
            baselines_referenced: BTreeSet::new(),
            search_evidence_found: BTreeSet::new(),
        }
    }

    fn run(&mut self) -> ArtifactAuditReportV1 {
        if !self.root.exists() {
            self.push_check(
                "root:exists",
                ArtifactAuditStatus::Error,
                Some(self.root.clone()),
                "root_missing",
                "artifact audit root does not exist",
            );
            return self.report();
        }

        let benchmark_manifests = find_named_files(&self.root, "benchmark.json");
        if benchmark_manifests.is_empty() {
            self.push_check(
                "root:benchmark_scan",
                ArtifactAuditStatus::Warn,
                Some(self.root.clone()),
                "no_benchmark_manifests",
                "no benchmark.json files were found under audit root",
            );
        }

        for manifest_path in benchmark_manifests {
            self.audit_suite(&manifest_path);
        }

        self.report()
    }

    fn audit_suite(&mut self, manifest_path: &Path) {
        self.suites_found += 1;
        let suite_dir = manifest_path.parent().unwrap_or_else(|| Path::new(""));
        let suite_id = self.suite_id(suite_dir);

        match load_combat_search_v2_benchmark(manifest_path) {
            Ok(_) => self.push_check(
                format!("suite:{suite_id}:benchmark_load"),
                ArtifactAuditStatus::Ok,
                Some(manifest_path.to_path_buf()),
                "benchmark_load_ok",
                "benchmark suite loads with current schema and fingerprints",
            ),
            Err(err) => self.push_check(
                format!("suite:{suite_id}:benchmark_load"),
                ArtifactAuditStatus::Error,
                Some(manifest_path.to_path_buf()),
                "benchmark_load_failed",
                err,
            ),
        }

        let Some(spec) = self.parse_manifest(&suite_id, manifest_path) else {
            self.audit_orphan_directories(&suite_id, suite_dir, &BTreeSet::new(), &BTreeSet::new());
            self.audit_search_evidence(&suite_id, suite_dir, &BTreeSet::new());
            return;
        };

        let mut known_case_ids = BTreeSet::new();
        let mut suite_capture_paths = BTreeSet::new();
        let mut suite_baseline_paths = BTreeSet::new();
        self.cases_found += spec.cases.len();

        for case in &spec.cases {
            known_case_ids.insert(case.id.clone());
            self.audit_case_inputs(
                &suite_id,
                suite_dir,
                case,
                &mut suite_capture_paths,
                &mut suite_baseline_paths,
            );
            self.audit_case_baseline(&suite_id, suite_dir, case, &mut suite_baseline_paths);
        }

        self.audit_orphan_directories(
            &suite_id,
            suite_dir,
            &suite_capture_paths,
            &suite_baseline_paths,
        );
        self.audit_search_evidence(&suite_id, suite_dir, &known_case_ids);
    }

    fn parse_manifest(
        &mut self,
        suite_id: &str,
        manifest_path: &Path,
    ) -> Option<CombatSearchV2BenchmarkSpec> {
        let payload = match fs::read_to_string(manifest_path) {
            Ok(payload) => payload,
            Err(err) => {
                self.push_check(
                    format!("suite:{suite_id}:manifest_parse"),
                    ArtifactAuditStatus::Error,
                    Some(manifest_path.to_path_buf()),
                    "manifest_read_failed",
                    err.to_string(),
                );
                return None;
            }
        };
        match serde_json::from_str::<CombatSearchV2BenchmarkSpec>(&payload) {
            Ok(spec) => {
                self.push_check(
                    format!("suite:{suite_id}:manifest_parse"),
                    ArtifactAuditStatus::Ok,
                    Some(manifest_path.to_path_buf()),
                    "manifest_parse_ok",
                    "benchmark manifest parsed for full audit",
                );
                Some(spec)
            }
            Err(err) => {
                self.push_check(
                    format!("suite:{suite_id}:manifest_parse"),
                    ArtifactAuditStatus::Error,
                    Some(manifest_path.to_path_buf()),
                    "manifest_parse_failed",
                    err.to_string(),
                );
                None
            }
        }
    }

    fn audit_case_inputs(
        &mut self,
        suite_id: &str,
        suite_dir: &Path,
        case: &CombatSearchV2BenchmarkCaseSpec,
        suite_capture_paths: &mut BTreeSet<PathBuf>,
        suite_baseline_paths: &mut BTreeSet<PathBuf>,
    ) {
        match (case.start_spec.as_ref(), case.combat_snapshot.as_ref()) {
            (Some(start_spec), None) => {
                let path = resolve_manifest_relative_path(suite_dir, start_spec);
                match load_combat_search_v2_start(&path) {
                    Ok(_) => self.push_check(
                        format!("case:{suite_id}:{}:start_spec_load", case.id),
                        ArtifactAuditStatus::Ok,
                        Some(path),
                        "start_spec_load_ok",
                        "start spec loads as a combat search input",
                    ),
                    Err(err) => self.push_check(
                        format!("case:{suite_id}:{}:start_spec_load", case.id),
                        ArtifactAuditStatus::Error,
                        Some(path),
                        "start_spec_load_failed",
                        err,
                    ),
                }
            }
            (None, Some(combat_snapshot)) => {
                let path = resolve_manifest_relative_path(suite_dir, combat_snapshot);
                suite_capture_paths.insert(path.clone());
                self.captures_referenced.insert(path.clone());
                match load_combat_capture_v1(&path) {
                    Ok(capture) => {
                        self.push_check(
                            format!("case:{suite_id}:{}:capture_load", case.id),
                            ArtifactAuditStatus::Ok,
                            Some(path.clone()),
                            "capture_load_ok",
                            "combat capture loads and validates",
                        );
                        self.audit_capture_expectations(suite_id, case, &path, &capture);
                        self.audit_combat_snapshot_search_input(suite_id, case, &path);
                    }
                    Err(err) => self.push_check(
                        format!("case:{suite_id}:{}:capture_load", case.id),
                        ArtifactAuditStatus::Error,
                        Some(path),
                        "capture_load_failed",
                        err,
                    ),
                }
            }
            (None, None) => self.push_check(
                format!("case:{suite_id}:{}:input_shape", case.id),
                ArtifactAuditStatus::Error,
                None,
                "case_input_missing",
                "case requires exactly one of start_spec or combat_snapshot",
            ),
            (Some(_), Some(_)) => self.push_check(
                format!("case:{suite_id}:{}:input_shape", case.id),
                ArtifactAuditStatus::Error,
                None,
                "case_input_ambiguous",
                "case cannot set both start_spec and combat_snapshot",
            ),
        }

        if let Some(CombatSearchV2BenchmarkBaselineSpec::Path(path)) = case.baseline.as_ref() {
            let path = resolve_manifest_relative_path(suite_dir, path);
            suite_baseline_paths.insert(path.clone());
            self.baselines_referenced.insert(path);
        }
    }

    fn audit_combat_snapshot_search_input(
        &mut self,
        suite_id: &str,
        case: &CombatSearchV2BenchmarkCaseSpec,
        path: &Path,
    ) {
        match load_combat_search_v2_snapshot(path) {
            Ok(_) => self.push_check(
                format!("case:{suite_id}:{}:search_input_load", case.id),
                ArtifactAuditStatus::Ok,
                Some(path.to_path_buf()),
                "search_input_load_ok",
                "combat snapshot loads through the combat search input boundary",
            ),
            Err(err) => self.push_check(
                format!("case:{suite_id}:{}:search_input_load", case.id),
                ArtifactAuditStatus::Error,
                Some(path.to_path_buf()),
                "search_input_load_failed",
                err,
            ),
        }
    }

    fn audit_capture_expectations(
        &mut self,
        suite_id: &str,
        case: &CombatSearchV2BenchmarkCaseSpec,
        path: &Path,
        capture: &CombatCaptureV1,
    ) {
        let Some(expected) = case.expected_fingerprints.as_ref() else {
            self.push_check(
                format!("case:{suite_id}:{}:expected_fingerprints", case.id),
                ArtifactAuditStatus::Warn,
                Some(path.to_path_buf()),
                "expected_fingerprints_missing",
                "benchmark case has no expected capture fingerprints",
            );
            return;
        };
        let Some(actual) = capture.fingerprints.as_ref() else {
            self.push_check(
                format!("case:{suite_id}:{}:expected_fingerprints", case.id),
                ArtifactAuditStatus::Error,
                Some(path.to_path_buf()),
                "capture_fingerprints_missing",
                "combat capture loaded without state fingerprints",
            );
            return;
        };
        let exact_ok = expected
            .exact_state_hash
            .as_ref()
            .is_none_or(|expected| expected == &actual.exact_state_hash);
        let order_ok = expected
            .legal_candidate_order_hash
            .as_ref()
            .is_none_or(|expected| expected == &actual.legal_candidate_order_hash);
        let ok = expected.public_observation_hash == actual.public_observation_hash
            && expected.legal_candidate_set_hash == actual.legal_candidate_set_hash
            && order_ok
            && exact_ok;
        if ok {
            self.push_check(
                format!("case:{suite_id}:{}:expected_fingerprints", case.id),
                ArtifactAuditStatus::Ok,
                Some(path.to_path_buf()),
                "expected_fingerprints_match",
                "registered expected fingerprints match the capture",
            );
        } else {
            self.push_check(
                format!("case:{suite_id}:{}:expected_fingerprints", case.id),
                ArtifactAuditStatus::Error,
                Some(path.to_path_buf()),
                "expected_fingerprints_drift",
                "registered expected fingerprints do not match the capture",
            );
        }
    }

    fn audit_case_baseline(
        &mut self,
        suite_id: &str,
        suite_dir: &Path,
        case: &CombatSearchV2BenchmarkCaseSpec,
        suite_baseline_paths: &mut BTreeSet<PathBuf>,
    ) {
        match case.baseline.as_ref() {
            None => {}
            Some(CombatSearchV2BenchmarkBaselineSpec::Inline(_)) => self.push_check(
                format!("case:{suite_id}:{}:baseline_inline", case.id),
                ArtifactAuditStatus::Ok,
                None,
                "baseline_inline_ok",
                "case uses an inline baseline outcome",
            ),
            Some(CombatSearchV2BenchmarkBaselineSpec::Path(path)) => {
                let path = resolve_manifest_relative_path(suite_dir, path);
                suite_baseline_paths.insert(path.clone());
                self.baselines_referenced.insert(path.clone());
                match load_combat_baseline_outcome_v1(&path) {
                    Ok(baseline) => {
                        self.push_check(
                            format!("case:{suite_id}:{}:baseline_load", case.id),
                            ArtifactAuditStatus::Ok,
                            Some(path.clone()),
                            "baseline_load_ok",
                            "baseline outcome loads and validates",
                        );
                        if baseline.case_id == case.id {
                            self.push_check(
                                format!("case:{suite_id}:{}:baseline_case_id", case.id),
                                ArtifactAuditStatus::Ok,
                                Some(path),
                                "baseline_case_id_ok",
                                "baseline case_id matches benchmark case id",
                            );
                        } else {
                            self.push_check(
                                format!("case:{suite_id}:{}:baseline_case_id", case.id),
                                ArtifactAuditStatus::Error,
                                Some(path),
                                "baseline_case_id_mismatch",
                                format!(
                                    "baseline case_id '{}' does not match benchmark case id '{}'",
                                    baseline.case_id, case.id
                                ),
                            );
                        }
                    }
                    Err(err) => self.push_check(
                        format!("case:{suite_id}:{}:baseline_load", case.id),
                        ArtifactAuditStatus::Error,
                        Some(path),
                        "baseline_load_failed",
                        err,
                    ),
                }
            }
        }
    }

    fn audit_orphan_directories(
        &mut self,
        suite_id: &str,
        suite_dir: &Path,
        suite_capture_paths: &BTreeSet<PathBuf>,
        suite_baseline_paths: &BTreeSet<PathBuf>,
    ) {
        for path in list_matching_files(&suite_dir.join("captures"), |path| {
            file_name_ends_with(path, ".capture.json")
        }) {
            if !suite_capture_paths.contains(&path) {
                self.push_check(
                    format!(
                        "suite:{suite_id}:orphan_capture:{}",
                        file_stem_for_check(&path)
                    ),
                    ArtifactAuditStatus::Warn,
                    Some(path),
                    "orphan_capture",
                    "capture file exists but is not referenced by benchmark.json",
                );
            }
        }

        for path in list_matching_files(&suite_dir.join("baselines"), |path| {
            file_name_ends_with(path, ".baseline.json")
        }) {
            if suite_baseline_paths.contains(&path) {
                continue;
            }
            let code = match load_combat_baseline_outcome_v1(&path) {
                Ok(_) => "baseline_file_not_registered",
                Err(_) => "orphan_baseline_invalid",
            };
            self.push_check(
                format!(
                    "suite:{suite_id}:orphan_baseline:{}",
                    file_stem_for_check(&path)
                ),
                ArtifactAuditStatus::Warn,
                Some(path),
                code,
                "baseline file exists but is not referenced by benchmark.json",
            );
        }
    }

    fn audit_search_evidence(
        &mut self,
        suite_id: &str,
        suite_dir: &Path,
        known_case_ids: &BTreeSet<String>,
    ) {
        let evidence_files = list_matching_files(&suite_dir.join("search_evidence"), |path| {
            path.extension()
                .and_then(|ext| ext.to_str())
                .is_some_and(|ext| ext.eq_ignore_ascii_case("json"))
        });
        for path in evidence_files {
            self.search_evidence_found.insert(path.clone());
            match load_combat_search_evidence_v1(&path) {
                Ok(evidence) => {
                    self.push_check(
                        format!(
                            "suite:{suite_id}:search_evidence_load:{}",
                            file_stem_for_check(&path)
                        ),
                        ArtifactAuditStatus::Ok,
                        Some(path.clone()),
                        "search_evidence_load_ok",
                        "search evidence envelope loads and validates",
                    );
                    self.audit_search_evidence_context(
                        suite_id,
                        suite_dir,
                        known_case_ids,
                        &path,
                        &evidence,
                    );
                }
                Err(err) => self.push_check(
                    format!(
                        "suite:{suite_id}:search_evidence_load:{}",
                        file_stem_for_check(&path)
                    ),
                    ArtifactAuditStatus::Error,
                    Some(path),
                    "search_evidence_load_failed",
                    err,
                ),
            }
        }
    }

    fn audit_search_evidence_context(
        &mut self,
        suite_id: &str,
        suite_dir: &Path,
        known_case_ids: &BTreeSet<String>,
        path: &Path,
        evidence: &serde_json::Value,
    ) {
        let context = evidence
            .get("context")
            .and_then(serde_json::Value::as_object);
        let case_id = context
            .and_then(|context| context.get("capture_case_id"))
            .and_then(serde_json::Value::as_str)
            .filter(|case_id| !case_id.trim().is_empty());
        match case_id {
            Some(case_id) if known_case_ids.contains(case_id) => self.push_check(
                format!(
                    "suite:{suite_id}:search_evidence_link:{}",
                    file_stem_for_check(path)
                ),
                ArtifactAuditStatus::Ok,
                Some(path.to_path_buf()),
                "search_evidence_case_link_ok",
                "search evidence links to a known benchmark case",
            ),
            Some(case_id) => self.push_check(
                format!(
                    "suite:{suite_id}:search_evidence_link:{}",
                    file_stem_for_check(path)
                ),
                ArtifactAuditStatus::Warn,
                Some(path.to_path_buf()),
                "search_evidence_unknown_case",
                format!("search evidence references unknown capture_case_id '{case_id}'"),
            ),
            None => self.push_check(
                format!(
                    "suite:{suite_id}:search_evidence_link:{}",
                    file_stem_for_check(path)
                ),
                ArtifactAuditStatus::Warn,
                Some(path.to_path_buf()),
                "search_evidence_unlinked",
                "search evidence has no capture_case_id link",
            ),
        }

        let capture_path = context
            .and_then(|context| context.get("capture_path"))
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.trim().is_empty());
        if let Some(capture_path) = capture_path {
            let resolved = resolve_recorded_path(suite_dir, capture_path);
            if resolved.exists() {
                self.push_check(
                    format!(
                        "suite:{suite_id}:search_evidence_capture_ref:{}",
                        file_stem_for_check(path)
                    ),
                    ArtifactAuditStatus::Ok,
                    Some(path.to_path_buf()),
                    "search_evidence_capture_ref_ok",
                    "search evidence capture_path exists",
                );
            } else {
                self.push_check(
                    format!(
                        "suite:{suite_id}:search_evidence_capture_ref:{}",
                        file_stem_for_check(path)
                    ),
                    ArtifactAuditStatus::Warn,
                    Some(path.to_path_buf()),
                    "search_evidence_capture_ref_missing",
                    format!("search evidence capture_path does not exist: {capture_path}"),
                );
            }
        }
    }

    fn push_check(
        &mut self,
        check_id: impl Into<String>,
        status: ArtifactAuditStatus,
        artifact_path: Option<PathBuf>,
        code: impl Into<String>,
        message: impl Into<String>,
    ) {
        let artifact_hash = artifact_path.as_ref().and_then(|path| file_hash(path).ok());
        let artifact_path = artifact_path
            .as_ref()
            .map(|path| normalize_report_path(&self.root, path));
        self.checks.push(ArtifactAuditCheckV1 {
            check_id: check_id.into(),
            status,
            artifact_path,
            artifact_hash,
            code: code.into(),
            message: message.into(),
        });
    }

    fn suite_id(&self, suite_dir: &Path) -> String {
        let value = normalize_report_path(&self.root, suite_dir);
        if value.is_empty() || value == "." {
            "root".to_string()
        } else {
            value
        }
    }

    fn report(&mut self) -> ArtifactAuditReportV1 {
        self.checks
            .sort_by(|left, right| left.check_id.cmp(&right.check_id));
        let mut summary = ArtifactAuditSummaryV1 {
            suites_found: self.suites_found,
            cases_found: self.cases_found,
            captures_referenced: self.captures_referenced.len(),
            baselines_referenced: self.baselines_referenced.len(),
            search_evidence_found: self.search_evidence_found.len(),
            checks_total: self.checks.len(),
            ..ArtifactAuditSummaryV1::default()
        };
        for check in &self.checks {
            match check.status {
                ArtifactAuditStatus::Ok => summary.checks_ok += 1,
                ArtifactAuditStatus::Warn => summary.checks_warn += 1,
                ArtifactAuditStatus::Error => summary.checks_error += 1,
            }
        }
        ArtifactAuditReportV1 {
            schema_name: ARTIFACT_AUDIT_SCHEMA_NAME,
            schema_version: ARTIFACT_AUDIT_SCHEMA_VERSION,
            root: self.root.display().to_string(),
            summary,
            checks: self.checks.clone(),
        }
    }
}

fn find_named_files(root: &Path, name: &str) -> Vec<PathBuf> {
    list_matching_files(root, |path| {
        path.file_name()
            .and_then(|file_name| file_name.to_str())
            .is_some_and(|file_name| file_name.eq_ignore_ascii_case(name))
    })
}

fn list_matching_files(root: &Path, predicate: impl Fn(&Path) -> bool) -> Vec<PathBuf> {
    let mut files = Vec::new();
    if root.exists() {
        collect_matching_files(root, &predicate, &mut files);
    }
    files.sort();
    files
}

fn collect_matching_files(
    root: &Path,
    predicate: &impl Fn(&Path) -> bool,
    files: &mut Vec<PathBuf>,
) {
    let Ok(entries) = fs::read_dir(root) else {
        return;
    };
    let mut entries = entries
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    entries.sort();
    for path in entries {
        if path.is_dir() {
            collect_matching_files(&path, predicate, files);
        } else if predicate(&path) {
            files.push(path);
        }
    }
}

fn resolve_manifest_relative_path(base_dir: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base_dir.join(path)
    }
}

fn resolve_recorded_path(suite_dir: &Path, path: &str) -> PathBuf {
    let raw = PathBuf::from(path);
    if raw.is_absolute() || raw.exists() {
        raw
    } else {
        suite_dir.join(raw)
    }
}

fn file_name_ends_with(path: &Path, suffix: &str) -> bool {
    path.file_name()
        .and_then(|file_name| file_name.to_str())
        .is_some_and(|file_name| file_name.ends_with(suffix))
}

fn file_stem_for_check(path: &Path) -> String {
    path.file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("unknown")
        .replace(['\\', '/', ':', ' '], "_")
}

fn normalize_report_path(root: &Path, path: &Path) -> String {
    let value = path
        .strip_prefix(root)
        .unwrap_or(path)
        .display()
        .to_string()
        .replace('\\', "/");
    if value.is_empty() {
        ".".to_string()
    } else {
        value
    }
}

fn file_hash(path: &Path) -> Result<String, String> {
    let bytes = fs::read(path).map_err(|err| err.to_string())?;
    let digest = Blake2b512::digest(bytes);
    Ok(hex_encode(&digest[..32]))
}

fn hex_encode(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        out.push(HEX[(byte >> 4) as usize] as char);
        out.push(HEX[(byte & 0x0f) as usize] as char);
    }
    out
}
