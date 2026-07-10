# Cargo Worktree Rebuild Fix Design

**Date:** 2026-07-10

## Context

Filtered library tests repeatedly rebuild and relink the same `sts_simulator`
test binary when run from a linked Git worktree. Two consecutive no-run test
commands took 20.88 seconds and 21.07 seconds even though no source file changed.

Cargo fingerprint logging identifies the direct cause:

```text
stale: missing D:/rust/sts_simulator/.git/worktrees/run-control-narrowing/refs/heads/run-control-narrowing
```

`build.rs` obtains the worktree-specific Git directory and then appends the
symbolic `HEAD` ref path to it. Git stores branch refs in the common Git
directory, not below `.git/worktrees/<name>`, so the resulting watched path can
never exist. Cargo therefore marks the build script dirty on every invocation,
which dirties and relinks the library test target.

The Git watchers originally supported live-communication build metadata. Commit
`58ec0122` removed that metadata, but left `emit_git_rerun_watchers` and its
imports behind. The current build script does not read Git data or emit any
Git-derived compiler environment variables.

## Goals

- Stop unchanged filtered test commands from rebuilding the library test binary
  in linked worktrees.
- Remove the obsolete Git-watcher code instead of repairing an unused feature.
- Add a cheap regression contract without creating another Rust test binary.
- Preserve schema generation and its real Cargo invalidation inputs.

## Non-Goals

- Do not delete behavior tests to reduce linking time.
- Do not split the crate or reorganize the test suite in this change.
- Do not change runtime behavior, generated schema contents, or build profiles.
- Do not restore Git SHA, build timestamp, or live-communication build metadata.

## Design

### Build-script inputs

Delete `emit_git_rerun_watchers`, its call from `main`, and the now-unused
`PathBuf` and `Command` imports. The build script will declare only the inputs it
actually consumes:

```text
build.rs
tools/compiled_protocol_schema.json
```

Cargo will still rerun schema generation when either input changes. Branch
moves, commits, ref packing, and linked-worktree metadata changes will no longer
invalidate the Rust build because they do not affect generated output.

### Regression contract

Extend the existing `tests/architecture_runtime_boundaries.rs` integration test
target with one source-level build-script contract. It will verify that:

- `build.rs` keeps explicit watchers for itself and the compiled protocol
  schema;
- `build.rs` does not invoke Git or mention Git ref watcher paths such as
  `packed-refs` or `refs/heads`.

Reusing the existing architecture test target avoids adding another integration
test executable and its associated link cost. The check deliberately protects
the current boundary: a future feature that genuinely consumes Git metadata
must redesign this contract rather than silently reintroducing unconditional
Git invalidation.

### Performance verification

Verification will run two different filtered library-test build commands back
to back in the same linked worktree and target directory, with Cargo fingerprint
logging enabled. The first command may compile after the build-script edit. The
second command must report the `sts_simulator` test target as fresh and must not
emit `Compiling sts_simulator`.

The dynamic check proves the user-visible symptom is fixed; the architecture
test prevents the obsolete dependency from returning. Normal focused and full
library tests verify that schema generation and runtime behavior are unchanged.

## Error Handling

The existing hard failure for a missing or invalid
`tools/compiled_protocol_schema.json` remains unchanged. Removing Git probing
also removes its silent fallback to `.git`; builds outside a Git checkout remain
supported because schema generation has no Git dependency.

## Test Plan

1. Run the new exact architecture contract test.
2. Run two filtered `cargo test --lib <filter> --no-run` commands consecutively
   with fingerprint logging and assert only the first may compile the package.
3. Run `cargo test --test architecture_runtime_boundaries`.
4. Run `cargo test --lib`.
5. Run `cargo fmt --all -- --check` and `git diff --check`.

## Success Criteria

- Cargo no longer reports the missing linked-worktree branch-ref path.
- The second unchanged filtered test build does not compile or relink
  `sts_simulator`.
- Schema generation still reruns for its two declared inputs.
- The architecture boundary suite and full library suite pass.
- No behavior tests or production modules are removed for this performance fix.
