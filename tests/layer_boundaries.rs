use std::fs;
use std::path::{Path, PathBuf};

fn collect_rs_files(path: &Path, out: &mut Vec<PathBuf>) {
    if path.is_file() {
        if path.extension().is_some_and(|ext| ext == "rs") {
            out.push(path.to_path_buf());
        }
        return;
    }

    let entries =
        fs::read_dir(path).unwrap_or_else(|err| panic!("read_dir failed for {path:?}: {err}"));
    for entry in entries {
        let entry = entry.unwrap_or_else(|err| panic!("dir entry failed for {path:?}: {err}"));
        collect_rs_files(&entry.path(), out);
    }
}

fn layer_files(rel_paths: &[&str]) -> Vec<PathBuf> {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    for rel in rel_paths {
        collect_rs_files(&root.join(rel), &mut files);
    }
    files.sort();
    files
}

fn source_lines(path: &Path) -> Vec<String> {
    fs::read_to_string(path)
        .unwrap_or_else(|err| panic!("read_to_string failed for {path:?}: {err}"))
        .lines()
        .map(|line| line.to_string())
        .collect()
}

fn assert_forbidden_deps(files: &[PathBuf], forbidden: &[&str], label: &str) {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));
    let mut violations = Vec::new();

    for path in files {
        let rel = path.strip_prefix(root).unwrap_or(path);
        for (line_no, line) in source_lines(path).into_iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") {
                continue;
            }
            for dep in forbidden {
                if line.contains(dep) {
                    violations.push(format!("{}:{} -> {}", rel.display(), line_no + 1, dep));
                }
            }
        }
    }

    assert!(
        violations.is_empty(),
        "{label} layer boundary violations:\n{}",
        violations.join("\n")
    );
}

#[test]
fn core_does_not_depend_on_integration_or_app_layers() {
    let files = layer_files(&[
        "src/content",
        "src/core",
        "src/engine",
        "src/map",
        "src/projection",
        "src/rewards",
        "src/runtime",
        "src/semantics",
        "src/state",
    ]);

    assert_forbidden_deps(
        &files,
        &["crate::testing", "crate::diff", "crate::bot", "crate::cli"],
        "core",
    );
}

#[test]
fn integration_does_not_depend_on_app_layer() {
    let files = layer_files(&[
        "src/diff",
        "src/protocol",
        "src/testing",
        "src/verification",
    ]);

    assert_forbidden_deps(&files, &["crate::bot", "crate::cli"], "integration");
}
