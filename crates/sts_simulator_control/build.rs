fn main() {
    let profile = std::env::var("PROFILE").unwrap_or_else(|_| "unknown".to_string());
    let manifest_dir = std::path::PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo should provide CARGO_MANIFEST_DIR"),
    );
    let repository_root = manifest_dir
        .join("../..")
        .canonicalize()
        .expect("control package should live below the repository root");
    println!("cargo:rustc-env=STS_CARGO_PROFILE={profile}");
    println!(
        "cargo:rustc-env=STS_REPOSITORY_ROOT={}",
        repository_root.display()
    );
    println!("cargo:rerun-if-env-changed=PROFILE");
}
