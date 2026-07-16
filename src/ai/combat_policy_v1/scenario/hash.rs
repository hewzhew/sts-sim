use blake2::{Blake2b512, Digest};
use serde::Serialize;

pub(super) fn stable_hash<T: Serialize>(value: &T) -> String {
    let bytes = serde_json::to_vec(value).expect("public combat policy input should serialize");
    let mut hasher = Blake2b512::new();
    hasher.update(bytes);
    let digest = hasher.finalize();
    digest[..32]
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}
