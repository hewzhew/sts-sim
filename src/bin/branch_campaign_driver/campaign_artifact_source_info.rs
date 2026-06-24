use std::path::PathBuf;

use serde::Serialize;
use serde_json::Value;

use crate::campaign_artifact_store::{CampaignArtifactRefV1, CampaignArtifactStoreV1};
use crate::campaign_artifacts::{
    read_campaign_artifact_text_v1, read_campaign_checkpoint_v1, read_campaign_report_v1,
};

const CAMPAIGN_ARTIFACT_SOURCE_INFO_SCHEMA_NAME: &str = "CampaignArtifactSourceInfoV1";
const CAMPAIGN_ARTIFACT_SOURCE_INFO_SCHEMA_VERSION: u32 = 1;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
pub(super) struct CampaignArtifactSourceInfoV1 {
    pub(super) schema_name: String,
    pub(super) schema_version: u32,
    pub(super) artifact: CampaignArtifactRefV1,
    pub(super) run_config: CampaignArtifactRunConfigV1,
    pub(super) progress: CampaignArtifactProgressV1,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub(super) struct CampaignArtifactRunConfigV1 {
    pub(super) seed: Option<u64>,
    pub(super) ascension: Option<u8>,
    pub(super) class: Option<String>,
    pub(super) mode: Option<String>,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize)]
pub(super) struct CampaignArtifactProgressV1 {
    pub(super) rounds_completed: Option<usize>,
    pub(super) stop_reason: Option<String>,
}

impl CampaignArtifactRunConfigV1 {
    fn merge_missing_from(&mut self, other: Self) {
        if self.seed.is_none() {
            self.seed = other.seed;
        }
        if self.ascension.is_none() {
            self.ascension = other.ascension;
        }
        if self.class.is_none() {
            self.class = other.class;
        }
        if self.mode.is_none() {
            self.mode = other.mode;
        }
    }
}

pub(super) fn campaign_artifact_source_info_v1(
    store: &CampaignArtifactStoreV1,
    selector: &str,
) -> Result<CampaignArtifactSourceInfoV1, String> {
    let artifact = store.resolve_source_selector_v1(selector)?;
    let mut run_config = CampaignArtifactRunConfigV1::default();
    run_config.merge_missing_from(campaign_artifact_manifest_run_config_v1(
        &artifact.manifest_path,
    ));
    run_config.merge_missing_from(campaign_artifact_checkpoint_run_config_v1(
        &artifact.checkpoint_path,
    ));
    run_config.merge_missing_from(campaign_artifact_command_run_config_v1(
        &artifact.command_path,
    ));
    let progress = campaign_artifact_report_progress_v1(&artifact.report_path);
    Ok(CampaignArtifactSourceInfoV1 {
        schema_name: CAMPAIGN_ARTIFACT_SOURCE_INFO_SCHEMA_NAME.to_string(),
        schema_version: CAMPAIGN_ARTIFACT_SOURCE_INFO_SCHEMA_VERSION,
        artifact,
        run_config,
        progress,
    })
}

pub(super) fn render_campaign_artifact_source_info_v1(
    info: &CampaignArtifactSourceInfoV1,
    json: bool,
) -> Result<String, String> {
    if json {
        return serde_json::to_string_pretty(info)
            .map_err(|err| format!("failed to serialize campaign source info: {err}"));
    }
    Ok(format!(
        "CampaignArtifactSourceInfoV1 label={} seed={} ascension={} class={} mode={} rounds={} stop={}\n  report={}\n  checkpoint={}\n  manifest={}",
        info.artifact.label,
        option_display_v1(info.run_config.seed),
        option_display_v1(info.run_config.ascension),
        info.run_config.class.as_deref().unwrap_or("-"),
        info.run_config.mode.as_deref().unwrap_or("-"),
        option_display_v1(info.progress.rounds_completed),
        info.progress.stop_reason.as_deref().unwrap_or("-"),
        info.artifact.report_path.display(),
        info.artifact.checkpoint_path.display(),
        info.artifact.manifest_path.display(),
    ))
}

fn campaign_artifact_report_progress_v1(path: &PathBuf) -> CampaignArtifactProgressV1 {
    let Ok(report) = read_campaign_report_v1(path) else {
        return CampaignArtifactProgressV1::default();
    };
    CampaignArtifactProgressV1 {
        rounds_completed: Some(report.rounds_completed),
        stop_reason: Some(report.stop_reason),
    }
}

fn campaign_artifact_manifest_run_config_v1(path: &PathBuf) -> CampaignArtifactRunConfigV1 {
    let Ok(text) = read_campaign_artifact_text_v1(path, "campaign manifest") else {
        return CampaignArtifactRunConfigV1::default();
    };
    let Ok(value) = serde_json::from_str::<Value>(&text) else {
        return CampaignArtifactRunConfigV1::default();
    };
    CampaignArtifactRunConfigV1 {
        seed: manifest_u64_field_v1(&value, "seed"),
        ascension: manifest_u8_field_v1(&value, "ascension"),
        class: manifest_string_field_v1(&value, "class")
            .or_else(|| manifest_string_field_v1(&value, "player_class"))
            .map(|value| canonical_campaign_class_label_v1(&value)),
        mode: manifest_string_field_v1(&value, "mode").map(|value| value.to_ascii_lowercase()),
    }
}

fn campaign_artifact_checkpoint_run_config_v1(path: &PathBuf) -> CampaignArtifactRunConfigV1 {
    let Ok(checkpoint) = read_campaign_checkpoint_v1(path) else {
        return CampaignArtifactRunConfigV1::default();
    };
    CampaignArtifactRunConfigV1 {
        seed: Some(checkpoint.seed),
        ascension: Some(checkpoint.run_domain.ascension_level),
        class: Some(canonical_campaign_class_label_v1(
            &checkpoint.run_domain.player_class,
        )),
        mode: None,
    }
}

fn campaign_artifact_command_run_config_v1(path: &PathBuf) -> CampaignArtifactRunConfigV1 {
    let Ok(text) = read_campaign_artifact_text_v1(path, "campaign command") else {
        return CampaignArtifactRunConfigV1::default();
    };
    CampaignArtifactRunConfigV1 {
        mode: campaign_command_preset_v1(&text),
        ..CampaignArtifactRunConfigV1::default()
    }
}

fn manifest_string_field_v1(value: &Value, name: &str) -> Option<String> {
    manifest_field_v1(value, name)
        .and_then(Value::as_str)
        .map(str::to_string)
        .filter(|value| !value.is_empty())
}

fn manifest_u64_field_v1(value: &Value, name: &str) -> Option<u64> {
    manifest_field_v1(value, name).and_then(Value::as_u64)
}

fn manifest_u8_field_v1(value: &Value, name: &str) -> Option<u8> {
    manifest_u64_field_v1(value, name).and_then(|value| u8::try_from(value).ok())
}

fn manifest_field_v1<'a>(value: &'a Value, name: &str) -> Option<&'a Value> {
    value
        .get(name)
        .or_else(|| value.get("compatibility").and_then(|value| value.get(name)))
        .or_else(|| value.get("payload").and_then(|value| value.get(name)))
}

fn campaign_command_preset_v1(text: &str) -> Option<String> {
    let tokens = text.split_whitespace().collect::<Vec<_>>();
    tokens
        .windows(2)
        .find_map(|window| {
            if window[0] == "--preset" {
                Some(window[1].trim_matches('\'').trim_matches('"'))
            } else {
                None
            }
        })
        .filter(|value| matches!(*value, "quick" | "focused" | "explore" | "deep"))
        .map(str::to_string)
}

fn canonical_campaign_class_label_v1(value: &str) -> String {
    match value.to_ascii_lowercase().as_str() {
        "ironclad" => "ironclad",
        "silent" => "silent",
        "defect" => "defect",
        "watcher" => "watcher",
        _ => value,
    }
    .to_string()
}

fn option_display_v1<T: ToString>(value: Option<T>) -> String {
    value
        .map(|value| value.to_string())
        .unwrap_or_else(|| "-".to_string())
}
