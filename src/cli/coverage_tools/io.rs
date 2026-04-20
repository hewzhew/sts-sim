use std::path::Path;

use crate::bot::infra::coverage_signatures::ObservedInteractionRecord;

pub fn load_live_comm_records(path: &Path) -> std::io::Result<Vec<ObservedInteractionRecord>> {
    let content = std::fs::read_to_string(path)?;
    content
        .lines()
        .enumerate()
        .map(|(line_idx, line)| {
            serde_json::from_str::<ObservedInteractionRecord>(line).map_err(|err| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidData,
                    format!(
                        "failed to parse live_comm signature record {}:{}: {}",
                        path.display(),
                        line_idx + 1,
                        err
                    ),
                )
            })
        })
        .collect()
}
