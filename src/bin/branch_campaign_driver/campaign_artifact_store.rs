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
