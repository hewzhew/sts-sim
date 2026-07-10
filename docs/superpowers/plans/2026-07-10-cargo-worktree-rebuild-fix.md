# Cargo Worktree Rebuild Fix Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Remove an obsolete Git watcher that forces every filtered Cargo test command to rebuild the library test binary in linked worktrees.

**Architecture:** Keep schema generation in the existing root `build.rs`, but restrict its declared inputs to the files it actually consumes. Reuse the existing architecture-boundary integration target for a source contract, then verify Cargo fingerprint reuse dynamically with two consecutive filtered test builds.

**Tech Stack:** Rust 2021, Cargo 1.91, PowerShell, Git linked worktrees

## Global Constraints

- Do not delete behavior tests to reduce linking time.
- Do not split the crate or reorganize the test suite.
- Do not change runtime behavior, generated schema contents, dependencies, or build profiles.
- Do not restore Git SHA, build timestamp, or live-communication build metadata.
- Reuse `tests/architecture_runtime_boundaries.rs`; do not add another Rust test target.
- Preserve `build.rs` and `tools/compiled_protocol_schema.json` as the only declared build-script inputs.

---

### Task 1: Remove Obsolete Git Invalidation

**Files:**
- Modify: `tests/architecture_runtime_boundaries.rs`
- Modify: `build.rs`

**Interfaces:**
- Consumes: root Cargo build script and the existing architecture-boundary integration test target
- Produces: `build_script_only_watches_consumed_inputs()` source contract; a build script with no Git dependency

- [ ] **Step 1: Add the failing build-script input contract**

Append this test to `tests/architecture_runtime_boundaries.rs`:

```rust
#[test]
fn build_script_only_watches_consumed_inputs() {
    let build_script = std::fs::read_to_string("build.rs").expect("read root build script");

    for required in [
        "cargo:rerun-if-changed=build.rs",
        "cargo:rerun-if-changed=tools/compiled_protocol_schema.json",
    ] {
        assert!(
            build_script.contains(required),
            "build script must keep the consumed input watcher `{required}`"
        );
    }

    for obsolete in [
        "emit_git_rerun_watchers",
        "Command::new(\"git\")",
        "packed-refs",
        "refs/heads",
    ] {
        assert!(
            !build_script.contains(obsolete),
            "build script must not retain obsolete Git invalidation `{obsolete}`"
        );
    }
}
```

- [ ] **Step 2: Run the exact test and verify RED**

Run:

```powershell
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo test --test architecture_runtime_boundaries build_script_only_watches_consumed_inputs -- --exact
```

Expected: FAIL because the current `build.rs` still contains
`emit_git_rerun_watchers`.

- [ ] **Step 3: Remove only the obsolete Git watcher**

Change the imports at the top of `build.rs` to:

```rust
use serde_json::Value;
use std::collections::BTreeSet;
use std::env;
use std::fs;
use std::path::Path;
```

Delete the complete `emit_git_rerun_watchers` function and delete this call
from `main`:

```rust
emit_git_rerun_watchers();
```

Keep these declarations unchanged:

```rust
println!("cargo:rerun-if-changed=build.rs");
println!("cargo:rerun-if-changed=tools/compiled_protocol_schema.json");
```

- [ ] **Step 4: Run the exact test and architecture suite and verify GREEN**

Run:

```powershell
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo test --test architecture_runtime_boundaries build_script_only_watches_consumed_inputs -- --exact
cargo test --test architecture_runtime_boundaries
```

Expected: the exact contract and the complete architecture suite pass.

- [ ] **Step 5: Commit the root-cause fix**

```powershell
git add build.rs tests/architecture_runtime_boundaries.rs
git commit -m "Stop Cargo rebuilds in linked worktrees"
```

---

### Task 2: Prove Incremental Reuse and Preserve Behavior

**Files:**
- Modify only if verification reveals an in-scope defect

**Interfaces:**
- Consumes: the narrowed `build.rs` input contract from Task 1
- Produces: fresh Cargo fingerprint evidence and full-suite verification

- [ ] **Step 1: Run two filtered test builds in the same worktree**

Run this PowerShell block without editing files between commands:

```powershell
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
$env:CARGO_LOG='cargo::core::compiler::fingerprint=info'

$firstLines = & cargo test --lib 'eval::run_control::commands::tests' --no-run -vv 2>&1
$firstExit = $LASTEXITCODE
if ($firstExit -ne 0) {
    $firstLines
    throw "first filtered test build failed with exit code $firstExit"
}

$secondLines = & cargo test --lib 'eval::run_control::auto_step::tests' --no-run -vv 2>&1
$secondExit = $LASTEXITCODE
$secondOutput = $secondLines | Out-String
if ($secondExit -ne 0) {
    $secondLines
    throw "second filtered test build failed with exit code $secondExit"
}
if ($secondOutput -match '(?m)^\s*Compiling sts_simulator') {
    $secondLines
    throw 'second filtered test build recompiled sts_simulator'
}
if ($secondOutput -match 'refs/heads') {
    $secondLines
    throw 'Cargo still observed a Git branch-ref watcher'
}
$secondLines | Select-String -Pattern 'Fresh sts_simulator|Finished'
```

Expected: the block exits successfully; the second command prints no
`Compiling sts_simulator` and no missing `refs/heads` fingerprint reason.

- [ ] **Step 2: Run the full library suite**

Run:

```powershell
Remove-Item Env:CARGO_LOG -ErrorAction SilentlyContinue
$env:CARGO_TARGET_DIR='D:\rust\sts_simulator\target'
cargo test --lib
```

Expected: all library tests pass with zero failures.

- [ ] **Step 3: Run formatting and diff hygiene checks**

Run:

```powershell
cargo fmt --all -- --check
git diff --check master...HEAD
git status -sb
```

Expected: formatting and whitespace checks pass; the worktree is clean; the
branch contains only the documented run-control narrowing and Cargo rebuild
fix commits.

- [ ] **Step 4: Review success criteria**

Confirm from fresh output that:

```text
the second filtered test build is fresh
no missing linked-worktree refs/heads path appears
schema input watchers remain present
no behavior tests were deleted for this fix
the full library suite has zero failures
```
