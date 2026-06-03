use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicU64, Ordering};

use blake2::{Blake2b512, Digest};

static TRACE_SUFFIX_COUNTER: AtomicU64 = AtomicU64::new(0);

pub(super) fn validate_trace_args(
    replay_trace: Option<&PathBuf>,
    continue_trace: Option<&PathBuf>,
    branch: Option<&str>,
) -> Result<(), String> {
    if replay_trace.is_some() && continue_trace.is_some() {
        return Err("--replay-trace and --continue-trace cannot be used together".to_string());
    }
    if branch.is_some() && continue_trace.is_none() {
        return Err("--branch is only valid with --continue-trace".to_string());
    }
    Ok(())
}

pub(super) fn trace_output_path(
    trace: Option<&PathBuf>,
    record_trace: Option<PathBuf>,
    continue_trace: Option<&PathBuf>,
    branch: Option<&str>,
) -> Option<PathBuf> {
    match (trace, record_trace, continue_trace) {
        (Some(path), _, _) => Some(path.clone()),
        (None, Some(path), _) => Some(path),
        (None, None, Some(parent)) => Some(default_continue_trace_path(parent, branch)),
        (None, None, None) => None,
    }
}

pub(super) fn default_record_trace_path(seed: u64, ascension: u8, player_class: &str) -> PathBuf {
    let class = sanitize_branch_name(player_class);
    let suffix = current_trace_suffix();
    PathBuf::from("tools/artifacts/traces").join(format!(
        "seed{seed}_{class}_a{ascension}.{suffix}.trace.json"
    ))
}

pub(super) fn reject_same_trace_path(source: &Path, output: &Path) -> Result<(), String> {
    if paths_refer_to_same_file(source, output)? {
        return Err(format!(
            "refusing to write continuation trace over source trace: {}",
            output.display()
        ));
    }
    Ok(())
}

pub(super) fn file_hash(path: &Path) -> Result<String, String> {
    let payload = fs::read(path).map_err(|err| err.to_string())?;
    let mut hasher = Blake2b512::new();
    hasher.update(&payload);
    let digest = hasher.finalize();
    Ok(hex_lower(&digest[..32]))
}

fn paths_refer_to_same_file(left: &Path, right: &Path) -> Result<bool, String> {
    if left == right {
        return Ok(true);
    }
    let left = fs::canonicalize(left)
        .map_err(|err| format!("failed to resolve trace path {}: {err}", left.display()))?;
    let Ok(right) = fs::canonicalize(right) else {
        return Ok(false);
    };
    Ok(left == right)
}

fn default_continue_trace_path(parent: &Path, branch: Option<&str>) -> PathBuf {
    let parent_dir = parent.parent().unwrap_or_else(|| Path::new(""));
    let stem = parent
        .file_stem()
        .and_then(|stem| stem.to_str())
        .unwrap_or("trace");
    let base = stem.strip_suffix(".trace").unwrap_or(stem);
    let branch = branch
        .map(sanitize_branch_name)
        .filter(|branch| !branch.is_empty())
        .unwrap_or_else(|| "continue".to_string());
    let suffix = current_trace_suffix();
    parent_dir.join(format!("{base}.{branch}.{suffix}.trace.json"))
}

fn sanitize_branch_name(raw: &str) -> String {
    raw.trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_lowercase()
            } else {
                '_'
            }
        })
        .collect::<String>()
        .split('_')
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("_")
}

fn current_trace_suffix() -> String {
    let duration = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default();
    let counter = TRACE_SUFFIX_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!(
        "{}-{:09}-{counter}",
        duration.as_secs(),
        duration.subsec_nanos()
    )
}

fn hex_lower(bytes: &[u8]) -> String {
    let mut out = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        use std::fmt::Write;
        let _ = write!(&mut out, "{byte:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_continue_trace_path_stays_next_to_parent_and_names_branch() {
        let parent = PathBuf::from("tools/artifacts/traces/seed590.trace.json");

        let path = default_continue_trace_path(&parent, Some("act1 event path"));

        assert_eq!(path.parent(), parent.parent());
        let file_name = path
            .file_name()
            .and_then(|name| name.to_str())
            .expect("path should have utf8 file name");
        assert!(file_name.starts_with("seed590.act1_event_path."));
        assert!(file_name.ends_with(".trace.json"));
    }

    #[test]
    fn default_record_trace_path_names_run_config_and_is_unique() {
        let first = default_record_trace_path(521, 0, "Ironclad");
        let second = default_record_trace_path(521, 0, "Ironclad");

        let first_name = first
            .file_name()
            .and_then(|name| name.to_str())
            .expect("trace path should have a file name");
        assert!(first_name.starts_with("seed521_ironclad_a0."));
        assert!(first_name.ends_with(".trace.json"));
        assert_ne!(first, second);
    }

    #[test]
    fn branch_without_continue_trace_is_rejected() {
        let err = validate_trace_args(None, None, Some("act1"))
            .expect_err("branch without continue should fail");

        assert!(err.contains("--branch"));
        assert!(err.contains("--continue-trace"));
    }

    #[test]
    fn replay_and_continue_trace_are_mutually_exclusive() {
        let old = PathBuf::from("old.trace.json");

        let err = validate_trace_args(Some(&old), Some(&old), None)
            .expect_err("mixed replay modes should fail");

        assert!(err.contains("--replay-trace"));
        assert!(err.contains("--continue-trace"));
    }

    #[test]
    fn rejects_overwriting_same_trace_by_canonical_path() {
        let dir = unique_temp_dir("same_trace");
        std::fs::create_dir_all(&dir).expect("temp dir should be created");
        let source = dir.join("old.trace.json");
        std::fs::write(&source, "{}").expect("source trace should be written");
        let same_file = std::fs::canonicalize(&source).expect("source should canonicalize");

        let err = reject_same_trace_path(&source, &same_file)
            .expect_err("canonical equivalent trace paths should fail");

        assert!(err.contains("refusing to write continuation trace over source trace"));
        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn default_continue_trace_path_uses_distinct_names_for_quick_repeats() {
        let parent = PathBuf::from("tools/artifacts/traces/seed590.trace.json");

        let first = default_continue_trace_path(&parent, Some("branch"));
        let second = default_continue_trace_path(&parent, Some("branch"));

        assert_ne!(first, second);
    }

    fn unique_temp_dir(prefix: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        path.push(format!(
            "sts_simulator_run_play_driver_{prefix}_{}_{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("clock works")
                .as_nanos()
        ));
        path
    }
}
