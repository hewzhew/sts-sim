fn main() {
    // Cargo's PROFILE environment variable reports the inherited base
    // (`release`) for custom profiles. OUT_DIR retains the actual profile
    // directory: target/<profile>/build/<package-hash>/out.
    let out_dir = std::path::PathBuf::from(
        std::env::var_os("OUT_DIR").expect("Cargo should provide OUT_DIR"),
    );
    let cargo_profile = out_dir
        .ancestors()
        .nth(3)
        .and_then(std::path::Path::file_name)
        .and_then(std::ffi::OsStr::to_str)
        .expect("OUT_DIR should contain the Cargo profile directory");
    let manifest_dir = std::path::PathBuf::from(
        std::env::var_os("CARGO_MANIFEST_DIR").expect("Cargo should provide CARGO_MANIFEST_DIR"),
    );
    let repository_root = manifest_dir
        .join("../..")
        .canonicalize()
        .expect("control package should live below the repository root");
    println!("cargo:rustc-env=STS_CARGO_PROFILE={cargo_profile}");
    println!(
        "cargo:rustc-env=STS_REPOSITORY_ROOT={}",
        repository_root.display()
    );
}
