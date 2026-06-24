use std::collections::{BTreeMap, HashSet};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::{SystemTime, UNIX_EPOCH};

use serde::{Deserialize, Serialize};
use serde_json::Value;

const CAMPAIGN_LATEST_POINTER_SCHEMA_NAME: &str = "CampaignLatestPointerV1";
const CAMPAIGN_SCRATCH_LATEST_POINTER_SCHEMA_NAME: &str = "CampaignScratchLatestPointerV1";
const CAMPAIGN_LATEST_POINTER_SCHEMA_VERSION: u32 = 1;
const CAMPAIGN_ARTIFACT_MANIFEST_SCHEMA_NAME: &str = "CampaignArtifactManifestV1";
const CAMPAIGN_ARTIFACT_MANIFEST_SCHEMA_VERSION: u32 = 1;
static CAMPAIGN_ARTIFACT_SUFFIX_COUNTER: AtomicU64 = AtomicU64::new(0);

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) enum CampaignArtifactKindV1 {
    Run,
    Scratch,
    Path,
}

impl CampaignArtifactKindV1 {
    fn as_str(&self) -> &'static str {
        match self {
            Self::Run => "run",
            Self::Scratch => "scratch",
            Self::Path => "path",
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct CampaignArtifactRefV1 {
    pub(super) kind: CampaignArtifactKindV1,
    pub(super) id: String,
    pub(super) label: String,
    pub(super) dir: PathBuf,
    pub(super) report_path: PathBuf,
    pub(super) state_path: PathBuf,
    pub(super) journal_path: PathBuf,
    pub(super) checkpoint_path: PathBuf,
    pub(super) manifest_path: PathBuf,
    pub(super) command_path: PathBuf,
    pub(super) log_path: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct CampaignArtifactStoreV1 {
    campaign_dir: PathBuf,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct CampaignArtifactManifestRefV1 {
    pub(super) path: PathBuf,
    pub(super) schema_name: String,
    pub(super) schema_version: u32,
    pub(super) payload_schema_name: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct CampaignArtifactPruneReportV1 {
    pub(super) schema_name: String,
    pub(super) mode: String,
    pub(super) candidates: usize,
    pub(super) reclaim_bytes: u64,
    pub(super) keep_runs: usize,
    pub(super) keep_scratch: usize,
    pub(super) deleted_files: usize,
    pub(super) class_totals: Vec<CampaignArtifactPruneClassTotalV1>,
    pub(super) largest_candidates: Vec<CampaignArtifactPruneCandidateV1>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct CampaignArtifactPruneClassTotalV1 {
    pub(super) class: String,
    pub(super) files: usize,
    pub(super) bytes: u64,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct CampaignArtifactPruneCandidateV1 {
    pub(super) class: String,
    pub(super) path: PathBuf,
    pub(super) relative_path: String,
    pub(super) bytes: u64,
    pub(super) last_write_unix_ms: u128,
}

impl CampaignArtifactStoreV1 {
    pub(super) fn new(campaign_dir: PathBuf) -> Self {
        Self { campaign_dir }
    }

    pub(super) fn run_artifact_ref_v1(&self, artifact_id: &str) -> CampaignArtifactRefV1 {
        let id = campaign_artifact_slug_v1(artifact_id);
        let dir = self.campaign_dir.join("runs").join(&id);
        let report_path = dir.join("campaign.json.gz");
        CampaignArtifactRefV1 {
            kind: CampaignArtifactKindV1::Run,
            id: id.clone(),
            label: format!("run:{id}"),
            dir: dir.clone(),
            state_path: campaign_sidecar_path_v1(&report_path, "state"),
            journal_path: campaign_sidecar_path_v1(&report_path, "journal"),
            checkpoint_path: dir.join("checkpoint.json.gz"),
            manifest_path: dir.join("manifest.json"),
            command_path: dir.join("command.txt"),
            log_path: dir.join("log.txt"),
            report_path,
        }
    }

    pub(super) fn scratch_artifact_ref_v1(&self, artifact_id: &str) -> CampaignArtifactRefV1 {
        let id = campaign_artifact_slug_v1(artifact_id);
        let dir = self.campaign_dir.join("scratch");
        let report_path = dir.join(format!("{id}.campaign.json.gz"));
        CampaignArtifactRefV1 {
            kind: CampaignArtifactKindV1::Scratch,
            id: id.clone(),
            label: format!("scratch:{id}"),
            dir: dir.clone(),
            state_path: campaign_sidecar_path_v1(&report_path, "state"),
            journal_path: campaign_sidecar_path_v1(&report_path, "journal"),
            checkpoint_path: dir.join(format!("{id}.checkpoint.json.gz")),
            manifest_path: dir.join(format!("{id}.manifest.json")),
            command_path: dir.join(format!("{id}.command.txt")),
            log_path: dir.join(format!("{id}.log")),
            report_path,
        }
    }

    pub(super) fn run_output_ref_v1(
        &self,
        label: &str,
        stamp: &str,
        suffix: &str,
    ) -> CampaignArtifactRefV1 {
        self.run_artifact_ref_v1(&campaign_output_artifact_id_v1(label, stamp, suffix))
    }

    pub(super) fn scratch_output_ref_v1(
        &self,
        label: &str,
        stamp: &str,
        suffix: &str,
    ) -> CampaignArtifactRefV1 {
        self.scratch_artifact_ref_v1(&campaign_output_artifact_id_v1(label, stamp, suffix))
    }

    pub(super) fn allocate_output_ref_v1(
        &self,
        kind: CampaignArtifactKindV1,
        label: &str,
        stamp: Option<&str>,
        suffix: Option<&str>,
    ) -> Result<CampaignArtifactRefV1, String> {
        let generated_stamp;
        let generated_suffix;
        let stamp = match stamp {
            Some(stamp) if !stamp.trim().is_empty() => stamp.trim(),
            _ => {
                generated_stamp = campaign_generated_artifact_stamp_v1();
                generated_stamp.as_str()
            }
        };
        let suffix = match suffix {
            Some(suffix) if !suffix.trim().is_empty() => suffix.trim(),
            _ => {
                generated_suffix = campaign_generated_artifact_suffix_v1();
                generated_suffix.as_str()
            }
        };
        match kind {
            CampaignArtifactKindV1::Run => Ok(self.run_output_ref_v1(label, stamp, suffix)),
            CampaignArtifactKindV1::Scratch => Ok(self.scratch_output_ref_v1(label, stamp, suffix)),
            CampaignArtifactKindV1::Path => {
                Err("path artifacts cannot be allocated as campaign outputs".to_string())
            }
        }
    }

    pub(super) fn resolve_source_selector_v1(
        &self,
        selector: &str,
    ) -> Result<CampaignArtifactRefV1, String> {
        let selector = selector.trim();
        if selector == "latest" {
            let pointer = self.read_latest_pointer_v1()?;
            return Ok(self.run_artifact_ref_v1(&pointer.artifact_id));
        }
        if selector == "scratch-latest" || selector == "scratch:latest" {
            let pointer = self.read_scratch_latest_pointer_v1()?;
            return Ok(self.scratch_artifact_ref_v1(&pointer.artifact_id));
        }
        if let Some(id) = selector.strip_prefix("run:") {
            return Ok(self.run_artifact_ref_v1(id));
        }
        if let Some(id) = selector.strip_prefix("scratch:") {
            return Ok(self.scratch_artifact_ref_v1(id));
        }
        if let Some(path) = selector.strip_prefix("path:") {
            return Ok(campaign_path_artifact_ref_v1(path));
        }
        Err(format!(
            "unknown campaign artifact selector `{selector}`; expected latest, scratch-latest, run:<id>, scratch:<id>, or path:<report>"
        ))
    }

    pub(super) fn write_latest_pointer_v1(
        &self,
        artifact: &CampaignArtifactRefV1,
        updated_at: &str,
    ) -> Result<(), String> {
        if artifact.kind != CampaignArtifactKindV1::Run {
            return Err("latest pointer requires a run artifact".to_string());
        }
        let pointer = CampaignLatestPointerV1::from_artifact_ref_v1(
            CAMPAIGN_LATEST_POINTER_SCHEMA_NAME,
            artifact,
            updated_at,
        );
        write_json_file_v1(&self.latest_pointer_path_v1(), &pointer)
    }

    pub(super) fn write_scratch_latest_pointer_v1(
        &self,
        artifact: &CampaignArtifactRefV1,
        updated_at: &str,
    ) -> Result<(), String> {
        if artifact.kind != CampaignArtifactKindV1::Scratch {
            return Err("scratch latest pointer requires a scratch artifact".to_string());
        }
        let pointer = CampaignLatestPointerV1::from_artifact_ref_v1(
            CAMPAIGN_SCRATCH_LATEST_POINTER_SCHEMA_NAME,
            artifact,
            updated_at,
        );
        write_json_file_v1(&self.scratch_latest_pointer_path_v1(), &pointer)
    }

    fn read_latest_pointer_v1(&self) -> Result<CampaignLatestPointerV1, String> {
        let pointer = read_json_file_v1::<CampaignLatestPointerV1>(&self.latest_pointer_path_v1())?;
        pointer.validate_schema_v1(CAMPAIGN_LATEST_POINTER_SCHEMA_NAME)?;
        Ok(pointer)
    }

    fn read_scratch_latest_pointer_v1(&self) -> Result<CampaignLatestPointerV1, String> {
        let pointer =
            read_json_file_v1::<CampaignLatestPointerV1>(&self.scratch_latest_pointer_path_v1())?;
        pointer.validate_schema_v1(CAMPAIGN_SCRATCH_LATEST_POINTER_SCHEMA_NAME)?;
        Ok(pointer)
    }

    fn latest_pointer_path_v1(&self) -> PathBuf {
        self.campaign_dir.join("latest.json")
    }

    fn scratch_latest_pointer_path_v1(&self) -> PathBuf {
        self.campaign_dir.join("scratch").join("latest.json")
    }

    pub(super) fn prune_campaign_artifacts_v1(
        &self,
        keep_runs: usize,
        keep_scratch: usize,
        apply: bool,
    ) -> Result<CampaignArtifactPruneReportV1, String> {
        let candidates = self.campaign_artifact_prune_candidates_v1(keep_runs, keep_scratch)?;
        let deleted_files = if apply {
            self.delete_campaign_artifact_prune_candidates_v1(&candidates)?
        } else {
            0
        };
        Ok(campaign_artifact_prune_report_v1(
            candidates,
            keep_runs,
            keep_scratch,
            apply,
            deleted_files,
        ))
    }

    fn campaign_artifact_prune_candidates_v1(
        &self,
        keep_runs: usize,
        keep_scratch: usize,
    ) -> Result<Vec<CampaignArtifactPruneCandidateV1>, String> {
        if !self.campaign_dir.exists() {
            return Ok(Vec::new());
        }
        let root = canonicalize_campaign_artifact_path_v1(&self.campaign_dir)?;
        let mut protected = HashSet::<PathBuf>::new();
        self.add_campaign_artifact_pointer_protection_v1(&mut protected);
        self.add_recent_run_artifact_protection_v1(&mut protected, keep_runs)?;
        self.add_recent_scratch_artifact_protection_v1(&mut protected, keep_scratch)?;

        let mut files = Vec::new();
        collect_campaign_artifact_files_v1(&root, &mut files)?;
        let mut candidates = Vec::new();
        for file in files {
            let path = canonicalize_campaign_artifact_path_v1(&file)?;
            if !path.starts_with(&root) {
                return Err(format!(
                    "refusing to inspect path outside campaign artifact root: {}",
                    path.display()
                ));
            }
            if protected.contains(&path) {
                continue;
            }
            let metadata = std::fs::metadata(&path).map_err(|err| {
                format!(
                    "failed to read campaign artifact metadata {}: {err}",
                    path.display()
                )
            })?;
            let relative_path = campaign_relative_path_string_v1(&root, &path);
            candidates.push(CampaignArtifactPruneCandidateV1 {
                class: campaign_artifact_prune_class_v1(&relative_path),
                path,
                relative_path,
                bytes: metadata.len(),
                last_write_unix_ms: campaign_file_modified_unix_ms_v1(&metadata),
            });
        }
        candidates.sort_by(|left, right| {
            left.class
                .cmp(&right.class)
                .then(left.relative_path.cmp(&right.relative_path))
        });
        Ok(candidates)
    }

    fn add_campaign_artifact_pointer_protection_v1(&self, protected: &mut HashSet<PathBuf>) {
        add_campaign_protected_path_v1(protected, &self.latest_pointer_path_v1());
        add_campaign_protected_path_v1(protected, &self.scratch_latest_pointer_path_v1());

        if self.latest_pointer_path_v1().exists() {
            if let Ok(pointer) = self.read_latest_pointer_v1() {
                add_campaign_pointer_artifacts_v1(protected, &pointer);
            }
        }
        if self.scratch_latest_pointer_path_v1().exists() {
            if let Ok(pointer) = self.read_scratch_latest_pointer_v1() {
                add_campaign_pointer_artifacts_v1(protected, &pointer);
            }
        }

        for legacy_name in [
            "latest.mode.txt",
            "latest.command.txt",
            "latest.manifest.json",
            "latest.log",
            "latest.campaign.json",
            "latest.checkpoint.json",
        ] {
            add_campaign_protected_path_v1(protected, &self.campaign_dir.join(legacy_name));
        }
    }

    fn add_recent_run_artifact_protection_v1(
        &self,
        protected: &mut HashSet<PathBuf>,
        keep_runs: usize,
    ) -> Result<(), String> {
        if keep_runs == 0 {
            return Ok(());
        }
        let runs_dir = self.campaign_dir.join("runs");
        if !runs_dir.exists() {
            return Ok(());
        }
        let mut dirs = campaign_child_dirs_with_modified_time_v1(&runs_dir)?;
        dirs.sort_by(|left, right| right.1.cmp(&left.1).then(left.0.cmp(&right.0)));
        for (dir, _) in dirs.into_iter().take(keep_runs) {
            add_campaign_protected_directory_v1(protected, &dir)?;
        }
        Ok(())
    }

    fn add_recent_scratch_artifact_protection_v1(
        &self,
        protected: &mut HashSet<PathBuf>,
        keep_scratch: usize,
    ) -> Result<(), String> {
        if keep_scratch == 0 {
            return Ok(());
        }
        let scratch_dir = self.campaign_dir.join("scratch");
        if !scratch_dir.exists() {
            return Ok(());
        }
        let mut groups = BTreeMap::<String, (u128, Vec<PathBuf>)>::new();
        for file in campaign_child_files_v1(&scratch_dir)? {
            if file.file_name().and_then(|name| name.to_str()) == Some("latest.json") {
                continue;
            }
            let Some(file_name) = file.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let metadata = std::fs::metadata(&file).map_err(|err| {
                format!(
                    "failed to read scratch artifact metadata {}: {err}",
                    file.display()
                )
            })?;
            let group_id = campaign_scratch_artifact_group_id_v1(file_name);
            let modified = campaign_file_modified_unix_ms_v1(&metadata);
            let entry = groups.entry(group_id).or_insert((0, Vec::new()));
            entry.0 = entry.0.max(modified);
            entry.1.push(file);
        }
        let mut groups = groups.into_iter().collect::<Vec<_>>();
        groups.sort_by(
            |(left_id, (left_modified, _)), (right_id, (right_modified, _))| {
                right_modified
                    .cmp(left_modified)
                    .then(left_id.cmp(right_id))
            },
        );
        for (_, (_, files)) in groups.into_iter().take(keep_scratch) {
            for file in files {
                add_campaign_protected_path_v1(protected, &file);
            }
        }
        Ok(())
    }

    fn delete_campaign_artifact_prune_candidates_v1(
        &self,
        candidates: &[CampaignArtifactPruneCandidateV1],
    ) -> Result<usize, String> {
        if !self.campaign_dir.exists() {
            return Ok(0);
        }
        let root = canonicalize_campaign_artifact_path_v1(&self.campaign_dir)?;
        let mut deleted = 0usize;
        for candidate in candidates {
            let path = canonicalize_campaign_artifact_path_v1(&candidate.path)?;
            if !path.starts_with(&root) {
                return Err(format!(
                    "refusing to delete path outside campaign artifact root: {}",
                    path.display()
                ));
            }
            if path.exists() {
                std::fs::remove_file(&path).map_err(|err| {
                    format!(
                        "failed to delete campaign artifact {}: {err}",
                        path.display()
                    )
                })?;
                deleted += 1;
            }
        }
        remove_empty_campaign_artifact_dirs_v1(&root)?;
        Ok(deleted)
    }
}

pub(super) fn render_campaign_artifact_ref_v1(
    artifact: &CampaignArtifactRefV1,
    json: bool,
) -> Result<String, String> {
    if json {
        return serde_json::to_string_pretty(artifact)
            .map_err(|err| format!("failed to serialize campaign artifact ref: {err}"));
    }
    Ok(format!(
        "CampaignArtifactRefV1 label={} kind={} id={}\n  dir={}\n  report={}\n  state={}\n  journal={}\n  checkpoint={}\n  manifest={}\n  command={}\n  log={}",
        artifact.label,
        artifact.kind.as_str(),
        artifact.id,
        artifact.dir.display(),
        artifact.report_path.display(),
        artifact.state_path.display(),
        artifact.journal_path.display(),
        artifact.checkpoint_path.display(),
        artifact.manifest_path.display(),
        artifact.command_path.display(),
        artifact.log_path.display(),
    ))
}

pub(super) fn render_campaign_artifact_manifest_ref_v1(
    manifest: &CampaignArtifactManifestRefV1,
    json: bool,
) -> Result<String, String> {
    if json {
        return serde_json::to_string_pretty(manifest)
            .map_err(|err| format!("failed to serialize campaign manifest ref: {err}"));
    }
    Ok(format!(
        "CampaignArtifactManifestRefV1 schema={} v{} payload_schema={} path={}",
        manifest.schema_name,
        manifest.schema_version,
        manifest.payload_schema_name,
        manifest.path.display()
    ))
}

pub(super) fn render_campaign_artifact_prune_report_v1(
    report: &CampaignArtifactPruneReportV1,
    json: bool,
) -> Result<String, String> {
    if json {
        return serde_json::to_string_pretty(report)
            .map_err(|err| format!("failed to serialize campaign prune report: {err}"));
    }
    let mut lines = Vec::new();
    lines.push(format!(
        "CampaignArtifactPruneV1 mode={} candidates={} reclaim={} keep_runs={} keep_scratch={} deleted={}",
        report.mode,
        report.candidates,
        format_campaign_artifact_size_v1(report.reclaim_bytes),
        report.keep_runs,
        report.keep_scratch,
        report.deleted_files
    ));
    for class in &report.class_totals {
        lines.push(format!(
            "  {:<12} files={:4} bytes={:>10}",
            class.class,
            class.files,
            format_campaign_artifact_size_v1(class.bytes)
        ));
    }
    lines.push("Largest candidates:".to_string());
    for candidate in &report.largest_candidates {
        lines.push(format!(
            "  {:>10} | {:<12} | {}",
            format_campaign_artifact_size_v1(candidate.bytes),
            candidate.class,
            candidate.relative_path
        ));
    }
    if report.mode == "dry-run" {
        lines.push(
            "No files deleted. Re-run with --apply, or wrapper -PruneApply, to remove these candidates."
                .to_string(),
        );
    }
    Ok(lines.join("\n"))
}

pub(super) fn write_campaign_artifact_manifest_from_payload_text_v1(
    path: &Path,
    payload_schema_name: &str,
    created_at: &str,
    payload_text: &str,
) -> Result<CampaignArtifactManifestRefV1, String> {
    let payload: Value = serde_json::from_str(payload_text)
        .map_err(|err| format!("failed to parse campaign manifest payload JSON: {err}"))?;
    let payload = match payload {
        Value::Object(object) => Value::Object(object),
        _ => return Err("campaign manifest payload must be a JSON object".to_string()),
    };
    let compatibility = CampaignArtifactManifestCompatibilityV1::from_payload_v1(&payload);
    let manifest = CampaignArtifactManifestEnvelopeV1 {
        schema_name: CAMPAIGN_ARTIFACT_MANIFEST_SCHEMA_NAME.to_string(),
        schema_version: CAMPAIGN_ARTIFACT_MANIFEST_SCHEMA_VERSION,
        created_at: created_at.to_string(),
        writer: "branch_campaign_driver artifact write-manifest".to_string(),
        payload_schema_name: payload_schema_name.to_string(),
        compatibility,
        payload,
    };
    write_json_file_v1(path, &manifest)?;
    Ok(CampaignArtifactManifestRefV1 {
        path: path.to_path_buf(),
        schema_name: CAMPAIGN_ARTIFACT_MANIFEST_SCHEMA_NAME.to_string(),
        schema_version: CAMPAIGN_ARTIFACT_MANIFEST_SCHEMA_VERSION,
        payload_schema_name: payload_schema_name.to_string(),
    })
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct CampaignArtifactManifestEnvelopeV1 {
    schema_name: String,
    schema_version: u32,
    created_at: String,
    writer: String,
    payload_schema_name: String,
    compatibility: CampaignArtifactManifestCompatibilityV1,
    payload: Value,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct CampaignArtifactManifestCompatibilityV1 {
    stage: Option<String>,
    exit_code: Option<i64>,
    command_kind: Option<String>,
    mode: Option<String>,
    seed: Option<i64>,
    ascension: Option<i64>,
    player_class: Option<String>,
    output_artifact: Option<String>,
    output_report: Option<String>,
    output_checkpoint: Option<String>,
    command_file: Option<String>,
}

impl CampaignArtifactManifestCompatibilityV1 {
    fn from_payload_v1(payload: &Value) -> Self {
        Self {
            stage: payload_string_field_v1(payload, "stage"),
            exit_code: payload_i64_field_v1(payload, "exit_code"),
            command_kind: payload_string_field_v1(payload, "command_kind"),
            mode: payload_string_field_v1(payload, "mode"),
            seed: payload_i64_field_v1(payload, "seed"),
            ascension: payload_i64_field_v1(payload, "ascension"),
            player_class: payload_string_field_v1(payload, "class"),
            output_artifact: payload_string_field_v1(payload, "output_artifact"),
            output_report: payload_string_field_v1(payload, "output_report"),
            output_checkpoint: payload_string_field_v1(payload, "output_checkpoint"),
            command_file: payload_string_field_v1(payload, "command_file"),
        }
    }
}

fn payload_string_field_v1(payload: &Value, field: &str) -> Option<String> {
    payload
        .get(field)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|value| !value.is_empty())
}

fn payload_i64_field_v1(payload: &Value, field: &str) -> Option<i64> {
    payload.get(field).and_then(Value::as_i64)
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
struct CampaignLatestPointerV1 {
    schema_name: String,
    schema_version: u32,
    updated_at: String,
    artifact_id: String,
    report: PathBuf,
    state: PathBuf,
    journal: PathBuf,
    checkpoint: PathBuf,
    manifest: PathBuf,
    command: PathBuf,
    log: PathBuf,
}

impl CampaignLatestPointerV1 {
    fn from_artifact_ref_v1(
        schema_name: &str,
        artifact: &CampaignArtifactRefV1,
        updated_at: &str,
    ) -> Self {
        Self {
            schema_name: schema_name.to_string(),
            schema_version: CAMPAIGN_LATEST_POINTER_SCHEMA_VERSION,
            updated_at: updated_at.to_string(),
            artifact_id: artifact.id.clone(),
            report: artifact.report_path.clone(),
            state: artifact.state_path.clone(),
            journal: artifact.journal_path.clone(),
            checkpoint: artifact.checkpoint_path.clone(),
            manifest: artifact.manifest_path.clone(),
            command: artifact.command_path.clone(),
            log: artifact.log_path.clone(),
        }
    }

    fn validate_schema_v1(&self, expected_schema_name: &str) -> Result<(), String> {
        if self.schema_name != expected_schema_name
            || self.schema_version != CAMPAIGN_LATEST_POINTER_SCHEMA_VERSION
        {
            return Err(format!(
                "campaign latest pointer uses {} v{}; expected {} v{}",
                self.schema_name,
                self.schema_version,
                expected_schema_name,
                CAMPAIGN_LATEST_POINTER_SCHEMA_VERSION
            ));
        }
        if self.artifact_id.is_empty() {
            return Err("campaign latest pointer is missing artifact_id".to_string());
        }
        Ok(())
    }
}

fn campaign_path_artifact_ref_v1(path: &str) -> CampaignArtifactRefV1 {
    let report_path = PathBuf::from(path);
    let dir = report_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(PathBuf::new);
    CampaignArtifactRefV1 {
        kind: CampaignArtifactKindV1::Path,
        id: String::new(),
        label: format!("path:{path}"),
        dir: dir.clone(),
        state_path: campaign_sidecar_path_v1(&report_path, "state"),
        journal_path: campaign_sidecar_path_v1(&report_path, "journal"),
        checkpoint_path: dir.join("checkpoint.json.gz"),
        manifest_path: dir.join("manifest.json"),
        command_path: dir.join("command.txt"),
        log_path: dir.join("log.txt"),
        report_path,
    }
}

fn campaign_sidecar_path_v1(report_path: &Path, sidecar: &str) -> PathBuf {
    let directory = report_path
        .parent()
        .map(Path::to_path_buf)
        .unwrap_or_else(PathBuf::new);
    let name = report_path
        .file_name()
        .map(|name| name.to_string_lossy().to_string())
        .unwrap_or_else(|| "campaign.json.gz".to_string());
    let sidecar_name = if let Some(prefix) = name.strip_suffix(".campaign.json.gz") {
        format!("{prefix}.campaign.{sidecar}.json.gz")
    } else if let Some(prefix) = name.strip_suffix(".campaign.json") {
        format!("{prefix}.campaign.{sidecar}.json")
    } else if let Some(prefix) = name.strip_suffix(".json.gz") {
        format!("{prefix}.{sidecar}.json.gz")
    } else if let Some(prefix) = name.strip_suffix(".json") {
        format!("{prefix}.{sidecar}.json")
    } else {
        format!("{name}.{sidecar}.json.gz")
    };
    directory.join(sidecar_name)
}

fn campaign_artifact_slug_v1(value: &str) -> String {
    let mut slug = String::new();
    let mut previous_dash = false;
    for character in value.trim().chars() {
        let keep = character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-');
        if keep {
            slug.push(character);
            previous_dash = false;
        } else if !previous_dash {
            slug.push('-');
            previous_dash = true;
        }
    }
    let slug = slug.trim_matches('-').to_string();
    if slug.is_empty() {
        "scratch".to_string()
    } else {
        slug
    }
}

fn campaign_output_artifact_id_v1(label: &str, stamp: &str, suffix: &str) -> String {
    let mut pieces = Vec::new();
    for value in [label, stamp, suffix] {
        let slug = campaign_artifact_slug_v1(value);
        if !slug.is_empty() && slug != "scratch" {
            pieces.push(slug);
        }
    }
    if pieces.is_empty() {
        "campaign".to_string()
    } else {
        pieces.join("-")
    }
}

fn campaign_generated_artifact_stamp_v1() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    campaign_format_utc_stamp_v1(duration.as_secs())
}

fn campaign_generated_artifact_suffix_v1() -> String {
    let duration = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let counter = CAMPAIGN_ARTIFACT_SUFFIX_COUNTER.fetch_add(1, Ordering::Relaxed);
    let nanos = duration.as_nanos() as u64;
    let mixed = nanos ^ ((std::process::id() as u64) << 32) ^ counter.rotate_left(17);
    format!("{:08x}", mixed & 0xffff_ffff)
}

fn campaign_format_utc_stamp_v1(unix_seconds: u64) -> String {
    let days = (unix_seconds / 86_400) as i64;
    let second_of_day = unix_seconds % 86_400;
    let hour = second_of_day / 3_600;
    let minute = (second_of_day % 3_600) / 60;
    let second = second_of_day % 60;
    let (year, month, day) = civil_from_unix_days_v1(days);
    format!("{year:04}{month:02}{day:02}-{hour:02}{minute:02}{second:02}")
}

fn civil_from_unix_days_v1(days: i64) -> (i32, u32, u32) {
    let z = days + 719_468;
    let era = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let year = year_of_era + era * 400;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_phase = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_phase + 2) / 5 + 1;
    let month = month_phase + if month_phase < 10 { 3 } else { -9 };
    let year = year + if month <= 2 { 1 } else { 0 };
    (year as i32, month as u32, day as u32)
}

fn write_json_file_v1<T: Serialize>(path: &Path, value: &T) -> Result<(), String> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|err| {
            format!(
                "failed to create campaign artifact directory {}: {err}",
                parent.display()
            )
        })?;
    }
    let text = serde_json::to_string_pretty(value)
        .map_err(|err| format!("failed to serialize campaign artifact JSON: {err}"))?;
    std::fs::write(path, format!("{text}\n")).map_err(|err| {
        format!(
            "failed to write campaign artifact JSON {}: {err}",
            path.display()
        )
    })
}

fn read_json_file_v1<T: for<'de> Deserialize<'de>>(path: &Path) -> Result<T, String> {
    let text = std::fs::read_to_string(path).map_err(|err| {
        format!(
            "failed to read campaign artifact JSON {}: {err}",
            path.display()
        )
    })?;
    serde_json::from_str(&text).map_err(|err| {
        format!(
            "failed to parse campaign artifact JSON {}: {err}",
            path.display()
        )
    })
}

fn campaign_artifact_prune_report_v1(
    candidates: Vec<CampaignArtifactPruneCandidateV1>,
    keep_runs: usize,
    keep_scratch: usize,
    apply: bool,
    deleted_files: usize,
) -> CampaignArtifactPruneReportV1 {
    let reclaim_bytes = candidates
        .iter()
        .map(|candidate| candidate.bytes)
        .sum::<u64>();
    let mut totals = BTreeMap::<String, (usize, u64)>::new();
    for candidate in &candidates {
        let entry = totals.entry(candidate.class.clone()).or_insert((0, 0));
        entry.0 += 1;
        entry.1 += candidate.bytes;
    }
    let mut class_totals = totals
        .into_iter()
        .map(
            |(class, (files, bytes))| CampaignArtifactPruneClassTotalV1 {
                class,
                files,
                bytes,
            },
        )
        .collect::<Vec<_>>();
    class_totals.sort_by(|left, right| {
        right
            .bytes
            .cmp(&left.bytes)
            .then(left.class.cmp(&right.class))
    });
    let mut largest_candidates = candidates.clone();
    largest_candidates.sort_by(|left, right| {
        right
            .bytes
            .cmp(&left.bytes)
            .then(left.relative_path.cmp(&right.relative_path))
    });
    largest_candidates.truncate(12);
    CampaignArtifactPruneReportV1 {
        schema_name: "CampaignArtifactPruneV1".to_string(),
        mode: if apply {
            "apply".to_string()
        } else {
            "dry-run".to_string()
        },
        candidates: candidates.len(),
        reclaim_bytes,
        keep_runs,
        keep_scratch,
        deleted_files,
        class_totals,
        largest_candidates,
    }
}

fn add_campaign_pointer_artifacts_v1(
    protected: &mut HashSet<PathBuf>,
    pointer: &CampaignLatestPointerV1,
) {
    for path in [
        &pointer.report,
        &pointer.state,
        &pointer.journal,
        &pointer.checkpoint,
        &pointer.manifest,
        &pointer.command,
        &pointer.log,
    ] {
        add_campaign_protected_path_v1(protected, path);
    }
}

fn add_campaign_protected_path_v1(protected: &mut HashSet<PathBuf>, path: &Path) {
    if path.exists() {
        if let Ok(path) = canonicalize_campaign_artifact_path_v1(path) {
            protected.insert(path);
        }
    }
}

fn add_campaign_protected_directory_v1(
    protected: &mut HashSet<PathBuf>,
    path: &Path,
) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    let mut files = Vec::new();
    collect_campaign_artifact_files_v1(path, &mut files)?;
    for file in files {
        add_campaign_protected_path_v1(protected, &file);
    }
    Ok(())
}

fn collect_campaign_artifact_files_v1(path: &Path, files: &mut Vec<PathBuf>) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(path).map_err(|err| {
        format!(
            "failed to read artifact directory {}: {err}",
            path.display()
        )
    })? {
        let entry = entry.map_err(|err| {
            format!(
                "failed to read artifact directory entry under {}: {err}",
                path.display()
            )
        })?;
        let entry_path = entry.path();
        let metadata = entry.metadata().map_err(|err| {
            format!(
                "failed to read artifact metadata {}: {err}",
                entry_path.display()
            )
        })?;
        if metadata.is_dir() {
            collect_campaign_artifact_files_v1(&entry_path, files)?;
        } else if metadata.is_file() {
            files.push(entry_path);
        }
    }
    Ok(())
}

fn campaign_child_files_v1(path: &Path) -> Result<Vec<PathBuf>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut files = Vec::new();
    for entry in std::fs::read_dir(path).map_err(|err| {
        format!(
            "failed to read artifact directory {}: {err}",
            path.display()
        )
    })? {
        let entry = entry.map_err(|err| {
            format!(
                "failed to read artifact directory entry under {}: {err}",
                path.display()
            )
        })?;
        let metadata = entry.metadata().map_err(|err| {
            format!(
                "failed to read artifact metadata {}: {err}",
                entry.path().display()
            )
        })?;
        if metadata.is_file() {
            files.push(entry.path());
        }
    }
    Ok(files)
}

fn campaign_child_dirs_with_modified_time_v1(path: &Path) -> Result<Vec<(PathBuf, u128)>, String> {
    if !path.exists() {
        return Ok(Vec::new());
    }
    let mut dirs = Vec::new();
    for entry in std::fs::read_dir(path).map_err(|err| {
        format!(
            "failed to read artifact directory {}: {err}",
            path.display()
        )
    })? {
        let entry = entry.map_err(|err| {
            format!(
                "failed to read artifact directory entry under {}: {err}",
                path.display()
            )
        })?;
        let metadata = entry.metadata().map_err(|err| {
            format!(
                "failed to read artifact metadata {}: {err}",
                entry.path().display()
            )
        })?;
        if metadata.is_dir() {
            dirs.push((entry.path(), campaign_file_modified_unix_ms_v1(&metadata)));
        }
    }
    Ok(dirs)
}

fn remove_empty_campaign_artifact_dirs_v1(root: &Path) -> Result<(), String> {
    if !root.exists() {
        return Ok(());
    }
    let mut dirs = Vec::new();
    collect_campaign_artifact_dirs_v1(root, &mut dirs)?;
    dirs.sort_by(|left, right| right.components().count().cmp(&left.components().count()));
    for dir in dirs {
        if dir == root {
            continue;
        }
        let path = canonicalize_campaign_artifact_path_v1(&dir)?;
        if !path.starts_with(root) {
            return Err(format!(
                "refusing to remove directory outside campaign artifact root: {}",
                path.display()
            ));
        }
        let is_empty = std::fs::read_dir(&path)
            .map_err(|err| {
                format!(
                    "failed to read artifact directory {}: {err}",
                    path.display()
                )
            })?
            .next()
            .is_none();
        if is_empty {
            std::fs::remove_dir(&path).map_err(|err| {
                format!(
                    "failed to remove empty campaign artifact directory {}: {err}",
                    path.display()
                )
            })?;
        }
    }
    Ok(())
}

fn collect_campaign_artifact_dirs_v1(path: &Path, dirs: &mut Vec<PathBuf>) -> Result<(), String> {
    if !path.exists() {
        return Ok(());
    }
    for entry in std::fs::read_dir(path).map_err(|err| {
        format!(
            "failed to read artifact directory {}: {err}",
            path.display()
        )
    })? {
        let entry = entry.map_err(|err| {
            format!(
                "failed to read artifact directory entry under {}: {err}",
                path.display()
            )
        })?;
        let metadata = entry.metadata().map_err(|err| {
            format!(
                "failed to read artifact metadata {}: {err}",
                entry.path().display()
            )
        })?;
        if metadata.is_dir() {
            let entry_path = entry.path();
            dirs.push(entry_path.clone());
            collect_campaign_artifact_dirs_v1(&entry_path, dirs)?;
        }
    }
    Ok(())
}

fn canonicalize_campaign_artifact_path_v1(path: &Path) -> Result<PathBuf, String> {
    std::fs::canonicalize(path).map_err(|err| {
        format!(
            "failed to resolve campaign artifact path {}: {err}",
            path.display()
        )
    })
}

fn campaign_relative_path_string_v1(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .trim_start_matches(|c| c == '\\' || c == '/')
        .to_string()
}

fn campaign_artifact_prune_class_v1(relative_path: &str) -> String {
    let top = relative_path.split(['\\', '/']).next().unwrap_or_default();
    if top == "runs" {
        "old_run"
    } else if top == "scratch" {
        "old_scratch"
    } else if top == "perf" {
        "perf"
    } else if top == "diagnostics" {
        "diagnostic"
    } else if top.starts_with("samples-") {
        "sample"
    } else if !relative_path.contains('\\') && !relative_path.contains('/') {
        "loose_root"
    } else {
        "other"
    }
    .to_string()
}

fn campaign_scratch_artifact_group_id_v1(file_name: &str) -> String {
    for suffix in [
        ".decision_outcomes.after.jsonl",
        ".campaign.state.json.gz",
        ".campaign.state.json",
        ".campaign.journal.json.gz",
        ".campaign.journal.json",
        ".campaign.json.gz",
        ".campaign.json",
        ".checkpoint.json.gz",
        ".checkpoint.json",
        ".manifest.json",
        ".command.txt",
        ".log",
    ] {
        if let Some(prefix) = file_name.strip_suffix(suffix) {
            return prefix.to_string();
        }
    }
    file_name.to_string()
}

fn campaign_file_modified_unix_ms_v1(metadata: &std::fs::Metadata) -> u128 {
    metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis())
        .unwrap_or_default()
}

fn format_campaign_artifact_size_v1(bytes: u64) -> String {
    if bytes >= 1_073_741_824 {
        format!("{:.1} GB", bytes as f64 / 1_073_741_824.0)
    } else if bytes >= 1_048_576 {
        format!("{:.1} MB", bytes as f64 / 1_048_576.0)
    } else if bytes >= 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{bytes} B")
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use super::CampaignArtifactKindV1;
    use super::CampaignArtifactStoreV1;

    #[test]
    fn campaign_artifact_store_builds_run_and_scratch_paths() {
        let root = PathBuf::from(r"D:\repo\tools\artifacts\campaigns");
        let store = CampaignArtifactStoreV1::new(root.clone());

        let run = store.run_artifact_ref_v1("abc");
        assert_eq!(run.kind, CampaignArtifactKindV1::Run);
        assert_eq!(run.id, "abc");
        assert_eq!(run.label, "run:abc");
        assert_eq!(run.dir, root.join("runs").join("abc"));
        assert_eq!(
            run.report_path,
            root.join("runs").join("abc").join("campaign.json.gz")
        );
        assert_eq!(
            run.checkpoint_path,
            root.join("runs").join("abc").join("checkpoint.json.gz")
        );
        assert_eq!(
            run.state_path,
            root.join("runs").join("abc").join("campaign.state.json.gz")
        );
        assert_eq!(
            run.journal_path,
            root.join("runs")
                .join("abc")
                .join("campaign.journal.json.gz")
        );
        assert_eq!(
            run.manifest_path,
            root.join("runs").join("abc").join("manifest.json")
        );
        assert_eq!(
            run.command_path,
            root.join("runs").join("abc").join("command.txt")
        );
        assert_eq!(run.log_path, root.join("runs").join("abc").join("log.txt"));

        let scratch = store.scratch_artifact_ref_v1("probe");
        assert_eq!(scratch.kind, CampaignArtifactKindV1::Scratch);
        assert_eq!(scratch.id, "probe");
        assert_eq!(scratch.label, "scratch:probe");
        assert_eq!(scratch.dir, root.join("scratch"));
        assert_eq!(
            scratch.report_path,
            root.join("scratch").join("probe.campaign.json.gz")
        );
        assert_eq!(
            scratch.checkpoint_path,
            root.join("scratch").join("probe.checkpoint.json.gz")
        );
        assert_eq!(
            scratch.state_path,
            root.join("scratch").join("probe.campaign.state.json.gz")
        );
        assert_eq!(
            scratch.journal_path,
            root.join("scratch").join("probe.campaign.journal.json.gz")
        );
        assert_eq!(
            scratch.manifest_path,
            root.join("scratch").join("probe.manifest.json")
        );
        assert_eq!(
            scratch.command_path,
            root.join("scratch").join("probe.command.txt")
        );
        assert_eq!(scratch.log_path, root.join("scratch").join("probe.log"));
    }

    #[test]
    fn campaign_artifact_store_resolves_basic_source_selectors() {
        let root = PathBuf::from(r"D:\repo\tools\artifacts\campaigns");
        let store = CampaignArtifactStoreV1::new(root.clone());

        assert_eq!(
            store.resolve_source_selector_v1("run:abc").unwrap(),
            store.run_artifact_ref_v1("abc")
        );
        assert_eq!(
            store.resolve_source_selector_v1("scratch:probe").unwrap(),
            store.scratch_artifact_ref_v1("probe")
        );

        let path_ref = store
            .resolve_source_selector_v1(r"path:D:\tmp\campaign.json.gz")
            .unwrap();
        assert_eq!(path_ref.kind, CampaignArtifactKindV1::Path);
        assert_eq!(path_ref.label, r"path:D:\tmp\campaign.json.gz");
        assert_eq!(
            path_ref.report_path,
            PathBuf::from(r"D:\tmp\campaign.json.gz")
        );
        assert_eq!(
            path_ref.state_path,
            PathBuf::from(r"D:\tmp\campaign.state.json.gz")
        );
        assert_eq!(
            path_ref.journal_path,
            PathBuf::from(r"D:\tmp\campaign.journal.json.gz")
        );
        assert_eq!(
            path_ref.checkpoint_path,
            PathBuf::from(r"D:\tmp\checkpoint.json.gz")
        );
    }

    #[test]
    fn campaign_artifact_store_reads_and_writes_latest_pointers() {
        let temp = std::env::temp_dir().join(format!(
            "sts-campaign-artifact-store-test-{}",
            std::process::id()
        ));
        let _ = std::fs::remove_dir_all(&temp);
        std::fs::create_dir_all(&temp).unwrap();
        let store = CampaignArtifactStoreV1::new(temp.clone());

        let run = store.run_artifact_ref_v1("abc");
        store
            .write_latest_pointer_v1(&run, "2026-06-24T00:00:00Z")
            .unwrap();
        assert_eq!(store.resolve_source_selector_v1("latest").unwrap(), run);

        let scratch = store.scratch_artifact_ref_v1("probe");
        store
            .write_scratch_latest_pointer_v1(&scratch, "2026-06-24T00:00:01Z")
            .unwrap();
        assert_eq!(
            store.resolve_source_selector_v1("scratch-latest").unwrap(),
            scratch
        );
        assert_eq!(
            store.resolve_source_selector_v1("scratch:latest").unwrap(),
            scratch
        );

        let _ = std::fs::remove_dir_all(&temp);
    }

    #[test]
    fn campaign_artifact_store_allocates_deterministic_output_refs() {
        let root = PathBuf::from(r"D:\repo\tools\artifacts\campaigns");
        let store = CampaignArtifactStoreV1::new(root.clone());

        let run = store.run_output_ref_v1("continue seed 521", "20260624-010203", "abcdef12");
        assert_eq!(run.kind, CampaignArtifactKindV1::Run);
        assert_eq!(run.id, "continue-seed-521-20260624-010203-abcdef12");
        assert_eq!(
            run.report_path,
            root.join("runs")
                .join("continue-seed-521-20260624-010203-abcdef12")
                .join("campaign.json.gz")
        );

        let scratch = store.scratch_output_ref_v1("gap probe", "20260624-010203", "abcdef12");
        assert_eq!(scratch.kind, CampaignArtifactKindV1::Scratch);
        assert_eq!(scratch.id, "gap-probe-20260624-010203-abcdef12");
        assert_eq!(
            scratch.report_path,
            root.join("scratch")
                .join("gap-probe-20260624-010203-abcdef12.campaign.json.gz")
        );
    }

    #[test]
    fn campaign_artifact_store_can_generate_output_refs_without_wrapper_stamp() {
        let root = PathBuf::from(r"D:\repo\tools\artifacts\campaigns");
        let store = CampaignArtifactStoreV1::new(root.clone());

        let artifact = store
            .allocate_output_ref_v1(CampaignArtifactKindV1::Run, "seed 1", None, None)
            .expect("allocation should generate stamp and suffix");

        assert_eq!(artifact.kind, CampaignArtifactKindV1::Run);
        assert!(artifact.id.starts_with("seed-1-"));
        assert_eq!(
            artifact.report_path,
            root.join("runs")
                .join(&artifact.id)
                .join("campaign.json.gz")
        );
    }

    #[test]
    fn campaign_artifact_store_formats_known_utc_stamp() {
        assert_eq!(super::campaign_format_utc_stamp_v1(0), "19700101-000000");
        assert_eq!(
            super::campaign_format_utc_stamp_v1(1_782_304_496),
            "20260624-123456"
        );
    }

    #[test]
    fn campaign_artifact_manifest_writer_wraps_payload_in_rust_schema() {
        let temp = std::env::temp_dir().join(format!(
            "sts-campaign-manifest-writer-test-{}.json",
            std::process::id()
        ));
        let _ = std::fs::remove_file(&temp);
        let payload = r#"{
            "stage": "completed",
            "exit_code": 0,
            "command_kind": "campaign_run",
            "mode": "quick",
            "seed": 521,
            "ascension": 0,
            "class": "ironclad",
            "output_artifact": "run:abc",
            "output_report": "campaign.json.gz",
            "output_checkpoint": "checkpoint.json.gz",
            "command_file": "command.txt"
        }"#;

        super::write_campaign_artifact_manifest_from_payload_text_v1(
            &temp,
            "CampaignWrapperManifestPayloadV1",
            "2026-06-24T12:34:56Z",
            payload,
        )
        .expect("manifest should write");

        let text = std::fs::read_to_string(&temp).expect("manifest text");
        let value: serde_json::Value = serde_json::from_str(&text).expect("manifest json");
        let _ = std::fs::remove_file(&temp);

        assert_eq!(value["schema_name"], "CampaignArtifactManifestV1");
        assert_eq!(value["schema_version"], 1);
        assert_eq!(
            value["writer"],
            "branch_campaign_driver artifact write-manifest"
        );
        assert_eq!(
            value["payload_schema_name"],
            "CampaignWrapperManifestPayloadV1"
        );
        assert_eq!(value["compatibility"]["mode"], "quick");
        assert_eq!(value["compatibility"]["seed"], 521);
        assert_eq!(value["compatibility"]["player_class"], "ironclad");
        assert_eq!(value["payload"]["command_kind"], "campaign_run");
    }
}
