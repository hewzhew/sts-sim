use std::path::{Path, PathBuf};

use blake2::{Blake2b512, Digest};
use serde_json::{json, Value};

pub(super) fn validate(canonical_oracle: bool) -> Result<(), String> {
    const REQUIRED_PROFILE: &str = "release";
    const BUILT_PROFILE: &str = env!("STS_CARGO_PROFILE");
    const REPOSITORY_ROOT: &str = env!("STS_REPOSITORY_ROOT");

    if !canonical_oracle {
        return Err(
            "oracle_lab refuses direct execution; run `cargo oracle-lab <command> ...`".to_string(),
        );
    }
    if BUILT_PROFILE != REQUIRED_PROFILE {
        return Err(format!(
            "oracle_lab was built with forbidden profile `{BUILT_PROFILE}`; \
             run `cargo oracle-lab <command> ...`"
        ));
    }
    let executable_name = if cfg!(windows) {
        "oracle_lab.exe"
    } else {
        "oracle_lab"
    };
    let expected = PathBuf::from(REPOSITORY_ROOT)
        .join("target")
        .join(REQUIRED_PROFILE)
        .join(executable_name);
    let current = std::env::current_exe()
        .and_then(|path| path.canonicalize())
        .map_err(|error| format!("failed to identify running oracle_lab: {error}"))?;
    let expected = expected.canonicalize().map_err(|error| {
        format!(
            "canonical oracle_lab artifact is missing at {}: {error}; \
             run `cargo oracle-lab <command> ...`",
            expected.display()
        )
    })?;
    if current != expected {
        return Err(format!(
            "oracle_lab refuses non-canonical artifact {}; expected {}; \
             run `cargo oracle-lab <command> ...`",
            current.display(),
            expected.display()
        ));
    }
    validate_source_freshness(&expected)?;
    Ok(())
}

fn validate_source_freshness(executable: &Path) -> Result<(), String> {
    let executable_modified = std::fs::metadata(executable)
        .and_then(|metadata| metadata.modified())
        .map_err(|error| {
            format!(
                "failed to inspect canonical oracle laboratory '{}': {error}",
                executable.display()
            )
        })?;
    let depfile = executable.with_extension("d");
    let depfile_text = std::fs::read_to_string(&depfile).map_err(|error| {
        format!(
            "canonical oracle dependency manifest is missing at '{}': {error}; rebuild with `cargo oracle-lab --help`",
            depfile.display()
        )
    })?;
    let repository = PathBuf::from(env!("STS_REPOSITORY_ROOT"));
    let dependencies = canonical_dependencies(&repository, &depfile_text);
    let stale = dependencies.iter().find(|dependency| {
        std::fs::metadata(dependency)
            .and_then(|metadata| metadata.modified())
            .is_ok_and(|modified| modified > executable_modified)
    });
    if let Some(stale) = stale {
        return Err(format!(
            "canonical oracle laboratory is stale: '{}' changed after '{}'; rebuild once with \
             `cargo oracle-lab --help`",
            stale.display(),
            executable.display()
        ));
    }
    Ok(())
}

fn depfile_dependencies(depfile: &str) -> Vec<PathBuf> {
    depfile
        .lines()
        .filter_map(|line| line.split_once(": ").map(|(_, dependencies)| dependencies))
        .flat_map(str::split_whitespace)
        .filter(|dependency| !dependency.ends_with(':'))
        .map(PathBuf::from)
        .collect()
}

fn canonical_dependencies(repository: &Path, depfile: &str) -> Vec<PathBuf> {
    let mut dependencies = depfile_dependencies(depfile);
    dependencies.extend([
        repository.join("Cargo.toml"),
        repository.join("Cargo.lock"),
        repository.join(".cargo/config.toml"),
        repository.join("crates/sts_combat_planner/Cargo.toml"),
    ]);
    dependencies
}

pub(super) fn source_content_fingerprint(
    repository: &Path,
    dependencies: &[PathBuf],
) -> Result<String, String> {
    let mut dependencies = dependencies
        .iter()
        .map(|dependency| {
            if dependency.is_absolute() {
                dependency.clone()
            } else {
                repository.join(dependency)
            }
        })
        .collect::<Vec<_>>();
    dependencies.sort();
    dependencies.dedup();
    let mut digest = Blake2b512::new();
    for dependency in dependencies {
        let bytes = std::fs::read(&dependency).map_err(|error| {
            format!(
                "failed to fingerprint canonical dependency '{}': {error}",
                dependency.display()
            )
        })?;
        digest.update(dependency.to_string_lossy().as_bytes());
        digest.update([0]);
        digest.update((bytes.len() as u64).to_le_bytes());
        digest.update(bytes);
    }
    Ok(digest
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect())
}

pub(super) fn runtime_identity() -> Value {
    let repository = PathBuf::from(env!("STS_REPOSITORY_ROOT"));
    let executable = std::env::current_exe().ok();
    let metadata = executable
        .as_ref()
        .and_then(|path| std::fs::metadata(path).ok());
    let modified_unix_ms = metadata
        .as_ref()
        .and_then(|metadata| metadata.modified().ok())
        .and_then(|modified| {
            modified
                .duration_since(std::time::SystemTime::UNIX_EPOCH)
                .ok()
        })
        .map(|duration| duration.as_millis());
    let git_head = read_git_head_fast(&repository);
    json!({
        "profile": env!("STS_CARGO_PROFILE"),
        "executable": executable,
        "artifact_bytes": metadata.map(|metadata| metadata.len()),
        "artifact_modified_unix_ms": modified_unix_ms,
        "git_head": git_head,
        "git_dirty": Value::Null,
        "dirty_scan": "omitted_in_compact_mode",
    })
}

/// Content identity of every source dependency recorded for the canonical
/// executable. Unlike `git_head`, this also distinguishes binaries built from
/// different uncommitted worktrees at the same revision.
pub(super) fn runtime_source_content_fingerprint() -> Result<String, String> {
    let repository = PathBuf::from(env!("STS_REPOSITORY_ROOT"));
    let executable = std::env::current_exe()
        .map_err(|error| format!("failed to identify running oracle_lab: {error}"))?;
    let depfile = executable.with_extension("d");
    let depfile_text = std::fs::read_to_string(&depfile).map_err(|error| {
        format!(
            "canonical oracle dependency manifest is missing at '{}': {error}",
            depfile.display()
        )
    })?;
    let dependencies = canonical_dependencies(&repository, &depfile_text);
    source_content_fingerprint(&repository, &dependencies)
}

fn read_git_head_fast(repository: &Path) -> Option<String> {
    let dot_git = repository.join(".git");
    let git_dir = if dot_git.is_dir() {
        dot_git
    } else {
        let pointer = std::fs::read_to_string(dot_git).ok()?;
        let relative = pointer.trim().strip_prefix("gitdir:")?.trim();
        repository.join(relative)
    };
    let head = std::fs::read_to_string(git_dir.join("HEAD")).ok()?;
    let revision = if let Some(reference) = head.trim().strip_prefix("ref: ") {
        std::fs::read_to_string(git_dir.join(reference))
            .ok()
            .or_else(|| {
                std::fs::read_to_string(git_dir.join("packed-refs"))
                    .ok()?
                    .lines()
                    .find_map(|line| {
                        let (hash, name) = line.split_once(' ')?;
                        (name == reference).then(|| hash.to_owned())
                    })
            })?
    } else {
        head
    };
    Some(revision.trim().chars().take(12).collect())
}
