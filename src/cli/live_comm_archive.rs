use crate::cli::live_comm::{
    ArchiveOutcome, ARCHIVE_ROOT, LOG_PATH, MAX_DEBUG_ARCHIVES, MAX_RAW_ARCHIVES,
    MAX_SIGNATURE_ARCHIVES, RAW_PATH, SIG_PATH,
};
use std::path::Path;

fn timestamp_string() -> String {
    let out = std::process::Command::new("powershell")
        .args(["-NoProfile", "-Command", "Get-Date -Format yyyyMMdd_HHmmss"])
        .output();
    match out {
        Ok(output) if output.status.success() => {
            String::from_utf8_lossy(&output.stdout).trim().to_string()
        }
        _ => "unknown_time".to_string(),
    }
}

fn ensure_archive_dirs() -> std::io::Result<()> {
    std::fs::create_dir_all(Path::new(ARCHIVE_ROOT).join("debug"))?;
    std::fs::create_dir_all(Path::new(ARCHIVE_ROOT).join("raw"))?;
    std::fs::create_dir_all(Path::new(ARCHIVE_ROOT).join("signatures"))?;
    Ok(())
}

fn prune_old_archives(dir: &Path, max_files: usize) -> std::io::Result<()> {
    let mut files: Vec<_> = std::fs::read_dir(dir)?
        .filter_map(|entry| entry.ok())
        .filter_map(|entry| {
            let meta = entry.metadata().ok()?;
            if !meta.is_file() {
                return None;
            }
            Some((entry.path(), meta.modified().ok()))
        })
        .collect();
    files.sort_by_key(|(_, modified)| *modified);
    let remove_count = files.len().saturating_sub(max_files);
    for (path, _) in files.into_iter().take(remove_count) {
        let _ = std::fs::remove_file(path);
    }
    Ok(())
}

pub(crate) fn maybe_archive_live_comm_logs(
    engine_bug_total: usize,
    content_gap_total: usize,
    game_over_seen: bool,
    victory: bool,
) -> std::io::Result<ArchiveOutcome> {
    let should_archive =
        engine_bug_total > 0 || content_gap_total > 0 || (game_over_seen && !victory);
    if !should_archive {
        return Ok(ArchiveOutcome {
            should_archive: false,
            reason: "no archival trigger".to_string(),
            archived: Vec::new(),
        });
    }

    ensure_archive_dirs()?;
    let stamp = timestamp_string();
    let reason = if engine_bug_total > 0 || content_gap_total > 0 {
        format!(
            "engine_bug_total={} content_gap_total={}",
            engine_bug_total, content_gap_total
        )
    } else {
        format!("game_over victory={}", victory)
    };

    let debug_target = Path::new(ARCHIVE_ROOT)
        .join("debug")
        .join(format!("live_comm_debug_{}.txt", stamp));
    let raw_target = Path::new(ARCHIVE_ROOT)
        .join("raw")
        .join(format!("live_comm_raw_{}.jsonl", stamp));
    let sig_target = Path::new(ARCHIVE_ROOT)
        .join("signatures")
        .join(format!("live_comm_signatures_{}.jsonl", stamp));

    std::fs::copy(LOG_PATH, &debug_target)?;
    std::fs::copy(RAW_PATH, &raw_target)?;
    std::fs::copy(SIG_PATH, &sig_target)?;

    let _ = prune_old_archives(&Path::new(ARCHIVE_ROOT).join("debug"), MAX_DEBUG_ARCHIVES);
    let _ = prune_old_archives(&Path::new(ARCHIVE_ROOT).join("raw"), MAX_RAW_ARCHIVES);
    let _ = prune_old_archives(
        &Path::new(ARCHIVE_ROOT).join("signatures"),
        MAX_SIGNATURE_ARCHIVES,
    );

    Ok(ArchiveOutcome {
        should_archive: true,
        reason,
        archived: vec![debug_target, raw_target, sig_target],
    })
}
