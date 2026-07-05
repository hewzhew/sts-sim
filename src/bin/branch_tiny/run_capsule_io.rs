use std::fs;
use std::path::Path;

use serde_json::Value;

pub(super) fn write_json(path: &Path, value: Value) -> Result<(), String> {
    if let Some(parent) = path
        .parent()
        .filter(|parent| !parent.as_os_str().is_empty())
    {
        ensure_dir(parent)?;
    }
    let payload = serde_json::to_string_pretty(&value).map_err(|err| err.to_string())?;
    let tmp = path.with_extension("tmp");
    fs::write(&tmp, payload).map_err(|err| format!("failed to write {}: {err}", tmp.display()))?;
    let _ = fs::remove_file(path);
    fs::rename(&tmp, path).map_err(|err| {
        format!(
            "failed to replace {} with {}: {err}",
            path.display(),
            tmp.display()
        )
    })
}

pub(super) fn remove_if_exists(path: &Path) -> Result<(), String> {
    match fs::remove_file(path) {
        Ok(()) => Ok(()),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(format!("failed to remove {}: {err}", path.display())),
    }
}

pub(super) fn read_terminal_entries(path: &Path) -> Result<Vec<Value>, String> {
    let Ok(payload) = fs::read_to_string(path) else {
        return Ok(Vec::new());
    };
    let value: Value = serde_json::from_str(&payload)
        .map_err(|err| format!("failed to parse {}: {err}", path.display()))?;
    Ok(value
        .get("terminals")
        .and_then(Value::as_array)
        .cloned()
        .unwrap_or_default())
}

pub(super) fn ensure_dir(path: &Path) -> Result<(), String> {
    fs::create_dir_all(path).map_err(|err| format!("failed to create {}: {err}", path.display()))
}
