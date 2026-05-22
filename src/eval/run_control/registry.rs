use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BenchmarkCasePaths {
    pub benchmark_manifest: PathBuf,
    pub capture_path: PathBuf,
    pub baseline_path: PathBuf,
}

impl BenchmarkCasePaths {
    pub fn for_case(root: &Path, case_id: &str) -> Self {
        Self {
            benchmark_manifest: root.join("benchmark.json"),
            capture_path: root
                .join("captures")
                .join(format!("{case_id}.capture.json")),
            baseline_path: root
                .join("baselines")
                .join(format!("{case_id}.baseline.json")),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RegistryManifest {
    name: String,
    cases: Vec<RegistryCase>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
struct RegistryCase {
    id: String,
    combat_snapshot: String,
    baseline: String,
}

pub fn add_case_to_benchmark_registry(
    root: &Path,
    case_id: &str,
) -> Result<BenchmarkCasePaths, String> {
    if case_id.trim().is_empty() {
        return Err("benchmark case id cannot be empty".to_string());
    }
    let paths = BenchmarkCasePaths::for_case(root, case_id);
    if !paths.capture_path.exists() {
        return Err(format!(
            "combat capture does not exist: {}",
            paths.capture_path.display()
        ));
    }
    if !paths.baseline_path.exists() {
        return Err(format!(
            "combat baseline does not exist: {}",
            paths.baseline_path.display()
        ));
    }

    let mut manifest = load_or_new_manifest(&paths.benchmark_manifest, root)?;
    let capture_rel = format!("captures/{case_id}.capture.json");
    let baseline_rel = format!("baselines/{case_id}.baseline.json");
    let case = RegistryCase {
        id: case_id.to_string(),
        combat_snapshot: capture_rel,
        baseline: baseline_rel,
    };
    match manifest.cases.iter_mut().find(|case| case.id == case_id) {
        Some(existing) => *existing = case,
        None => manifest.cases.push(case),
    }
    manifest.cases.sort_by(|left, right| left.id.cmp(&right.id));

    if let Some(parent) = paths
        .benchmark_manifest
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        fs::create_dir_all(parent).map_err(|err| err.to_string())?;
    }
    let payload = serde_json::to_string_pretty(&manifest).map_err(|err| err.to_string())?;
    fs::write(&paths.benchmark_manifest, payload).map_err(|err| err.to_string())?;
    Ok(paths)
}

fn load_or_new_manifest(path: &Path, root: &Path) -> Result<RegistryManifest, String> {
    if path.exists() {
        let payload = fs::read_to_string(path).map_err(|err| err.to_string())?;
        let manifest: RegistryManifest =
            serde_json::from_str(&payload).map_err(|err| err.to_string())?;
        if manifest.name.trim().is_empty() {
            return Err("benchmark manifest name cannot be empty".to_string());
        }
        return Ok(manifest);
    }

    Ok(RegistryManifest {
        name: root
            .file_name()
            .and_then(|name| name.to_str())
            .filter(|name| !name.trim().is_empty())
            .unwrap_or("combat_search_v2_benchmark")
            .to_string(),
        cases: Vec::new(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    fn registry_adds_case_with_relative_capture_and_baseline_paths() {
        let root = unique_temp_dir("run_control_registry");
        let paths = BenchmarkCasePaths::for_case(&root, "case_a");
        fs::create_dir_all(paths.capture_path.parent().unwrap()).expect("capture dir");
        fs::create_dir_all(paths.baseline_path.parent().unwrap()).expect("baseline dir");
        fs::write(&paths.capture_path, "{}").expect("capture placeholder");
        fs::write(&paths.baseline_path, "{}").expect("baseline placeholder");

        let written =
            add_case_to_benchmark_registry(&root, "case_a").expect("registry should update");
        let payload = fs::read_to_string(&written.benchmark_manifest).expect("manifest readable");

        assert!(payload.contains("\"combat_snapshot\": \"captures/case_a.capture.json\""));
        assert!(payload.contains("\"baseline\": \"baselines/case_a.baseline.json\""));

        let _ = fs::remove_dir_all(root);
    }

    fn unique_temp_dir(label: &str) -> PathBuf {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system time should be after unix epoch")
            .as_nanos();
        std::env::temp_dir().join(format!("{label}_{}_{}", std::process::id(), nanos))
    }
}
