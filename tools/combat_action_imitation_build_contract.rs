use std::fs;
use std::path::Path;

/// Emits one compatibility identity for every crate that compiles the shared
/// combat action-imitation implementation.
pub fn emit(repository_root: &Path) {
    // FNV-1a is used only as a deterministic change detector, not for
    // adversarial integrity.
    const FNV_OFFSET_BASIS: u64 = 0xcbf29ce484222325;
    const ACTION_CONTRACT_FILES: &[&str] = &[
        "src/eval/combat_action_imitation.rs",
        "src/sim/combat_action.rs",
        "src/sim/combat_action_surface.rs",
        "src/ai/combat_search_v2/pending_choice_action_prefix.rs",
    ];
    const GUIDANCE_CONTRACT_FILES: &[&str] = &[
        "src/eval/combat_guidance_bundle.rs",
        "src/ai/combat_search_v2/oracle_action_policy.rs",
    ];

    fn contract_hash(repository_root: &Path, relative_paths: &[&str], mut hash: u64) -> u64 {
        const FNV_PRIME: u64 = 0x100000001b3;
        for relative_path in relative_paths {
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
                                "failed to read combat guidance contract file {}: {error}",
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
        hash
    }

    let action_hash = contract_hash(repository_root, ACTION_CONTRACT_FILES, FNV_OFFSET_BASIS);
    println!("cargo:rustc-env=STS_COMBAT_ACTION_IMITATION_RUNTIME_ID=fnv1a64:{action_hash:016x}");

    let guidance_hash = contract_hash(repository_root, GUIDANCE_CONTRACT_FILES, FNV_OFFSET_BASIS);
    println!("cargo:rustc-env=STS_COMBAT_GUIDANCE_RUNTIME_ID=fnv1a64:{guidance_hash:016x}");
}
