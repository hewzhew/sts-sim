use std::fs;
use std::path::Path;

use serde::Serialize;

pub const ARTIFACT_AUDIT_SCHEMA_NAME: &str = "ArtifactAuditReportV1";
pub const ARTIFACT_AUDIT_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactAuditReportV1 {
    pub schema_name: &'static str,
    pub schema_version: u32,
    pub root: String,
    pub summary: ArtifactAuditSummaryV1,
    pub checks: Vec<ArtifactAuditCheckV1>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactAuditSummaryV1 {
    pub suites_found: usize,
    pub cases_found: usize,
    pub captures_referenced: usize,
    pub baselines_referenced: usize,
    pub search_evidence_found: usize,
    pub checks_total: usize,
    pub checks_ok: usize,
    pub checks_warn: usize,
    pub checks_error: usize,
}

#[derive(Clone, Debug, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ArtifactAuditCheckV1 {
    pub check_id: String,
    pub status: ArtifactAuditStatus,
    pub artifact_path: Option<String>,
    pub artifact_hash: Option<String>,
    pub code: String,
    pub message: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ArtifactAuditStatus {
    Ok,
    Warn,
    Error,
}

pub fn save_artifact_audit_report(
    path: &Path,
    report: &ArtifactAuditReportV1,
) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let payload = serde_json::to_string_pretty(report).map_err(|err| err.to_string())?;
    fs::write(path, payload).map_err(|err| err.to_string())
}

pub fn render_artifact_audit_summary(report: &ArtifactAuditReportV1) -> String {
    let mut lines = vec![
        format!("Artifact audit: {}", report.root),
        format!(
            "  suites={} cases={} captures={} baselines={} search_evidence={}",
            report.summary.suites_found,
            report.summary.cases_found,
            report.summary.captures_referenced,
            report.summary.baselines_referenced,
            report.summary.search_evidence_found
        ),
        format!(
            "  checks={} ok={} warn={} error={}",
            report.summary.checks_total,
            report.summary.checks_ok,
            report.summary.checks_warn,
            report.summary.checks_error
        ),
    ];

    let notable = report
        .checks
        .iter()
        .filter(|check| check.status != ArtifactAuditStatus::Ok)
        .take(20)
        .collect::<Vec<_>>();
    if !notable.is_empty() {
        lines.push("  issues:".to_string());
        for check in notable {
            lines.push(format!(
                "    {:?} {} [{}] {}",
                check.status, check.check_id, check.code, check.message
            ));
        }
        let remaining = report
            .checks
            .iter()
            .filter(|check| check.status != ArtifactAuditStatus::Ok)
            .count()
            .saturating_sub(20);
        if remaining > 0 {
            lines.push(format!("    ... {remaining} more issue(s) in JSON report"));
        }
    }
    lines.join("\n")
}
