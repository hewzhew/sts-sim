use std::fs;
use std::path::Path;

/// Emits one compatibility identity for every crate that compiles the shared
/// combat action-imitation implementation.
pub fn emit(repository_root: &Path) {
    // FNV-1a is used only as a deterministic change detector, not for
    // adversarial integrity.
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const FNV_PRIME: u64 = 0x100000001b3;
    const CONTRACT_FILES: &[&str] = &[
        "src/eval/combat_action_imitation.rs",
        "src/sim/combat_action.rs",
        "src/sim/combat_action_surface.rs",
        "src/ai/combat_search_v2/pending_choice_action_prefix.rs",
    ];

    let mut hash = FNV_OFFSET_BASIS;
    for relative_path in CONTRACT_FILES {
        let path = repository_root.join(relative_path);
        println!("cargo:rerun-if-changed={}", path.display());
        for byte in relative_path
            .as_bytes()
            .iter()
            .copied()
            .chain([0])
            .chain(
                fs::read(&path)
                    .unwrap_or_else(|error| {
                        panic!(
                            "failed to read combat action imitation contract file {}: {error}",
                            path.display()
                        )
                    })
                    .into_iter(),
            )
            .chain([0xff])
        {
            hash ^= u64::from(byte);
            hash = hash.wrapping_mul(FNV_PRIME);
        }
    }
    println!("cargo:rustc-env=STS_COMBAT_ACTION_IMITATION_RUNTIME_ID=fnv1a64:{hash:016x}");
}
