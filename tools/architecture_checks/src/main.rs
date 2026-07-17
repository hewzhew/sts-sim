use std::env;
use std::ffi::OsString;
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    match run() {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("architecture check runner failed: {error}");
            ExitCode::FAILURE
        }
    }
}

fn run() -> Result<u8, String> {
    let repo_root = repo_root()?;
    let suite = repo_root.join("tests/architecture_runtime_boundaries.rs");
    let output_dir = repo_root.join("target/architecture-checks");
    std::fs::create_dir_all(&output_dir)
        .map_err(|error| format!("create {}: {error}", output_dir.display()))?;
    let executable = output_dir.join(format!(
        "architecture_runtime_boundaries{}",
        env::consts::EXE_SUFFIX
    ));

    compile_suite(&repo_root, &suite, &executable)?;
    let status = Command::new(&executable)
        .args(env::args_os().skip(1))
        .current_dir(&repo_root)
        .status()
        .map_err(|error| format!("run {}: {error}", executable.display()))?;

    Ok(status.code().unwrap_or(1).clamp(0, u8::MAX as i32) as u8)
}

fn repo_root() -> Result<PathBuf, String> {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .and_then(Path::parent)
        .map(Path::to_path_buf)
        .ok_or_else(|| "architecture helper must live two levels below the repository".to_string())
}

fn compile_suite(repo_root: &Path, suite: &Path, executable: &Path) -> Result<(), String> {
    let mut rustc_args = vec![
        OsString::from("--edition=2021"),
        OsString::from("--test"),
        suite.as_os_str().to_owned(),
        OsString::from("-C"),
        OsString::from("debuginfo=0"),
        OsString::from("-o"),
        executable.as_os_str().to_owned(),
    ];
    if cfg!(target_env = "msvc") {
        rustc_args.push(OsString::from("-C"));
        rustc_args.push(OsString::from("linker=rust-lld"));
    }

    let status = Command::new("rustc")
        .args(rustc_args)
        .current_dir(repo_root)
        .status()
        .map_err(|error| format!("compile {}: {error}", suite.display()))?;
    if status.success() {
        Ok(())
    } else {
        Err(format!(
            "rustc failed for {} with status {status}",
            suite.display()
        ))
    }
}
