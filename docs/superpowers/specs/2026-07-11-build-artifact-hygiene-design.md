# Build and Artifact Hygiene Design

## Context

The repository is large but the recent development delay is not explained by
test count alone. The current checkout contains about 340,000 Rust lines in
1,827 Rust files and 2,840 `#[test]` annotations. All library unit tests compile
into one harness. The current default harness is about 53 MiB and its PDB is
about 316 MiB.

The larger practical problem is that development has alternated between the
main checkout and short-lived worktrees while sharing one Cargo target
directory. Cargo treats those source roots as different crate inputs. A fresh
worktree baseline took about 98 seconds, individual rebuilds commonly took
40–90 seconds, and the post-merge rebuild in the main checkout took 183
seconds. By contrast, touching one file and rebuilding repeatedly in the same
checkout took about 19–20 seconds.

Disabling test debug information reduced the PDB from about 316 MiB to 28 MiB,
but did not materially change the same-checkout incremental rebuild. Using
`rust-lld` reduced a cold experimental build but left the incremental rebuild at
about 20 seconds. The first hygiene pass therefore needs to stabilize the
source root and reduce cache size; it should not add a linker wrapper that does
not improve the common loop.

Cargo caches and experiment output are also mixed under `target`. Known Cargo
profile/cache directories occupy roughly 42 GiB, led by `target/debug` at
about 31 GiB and `target/fast-run` at about 8 GiB. Durable run capsules are much
smaller but cannot safely be subjected to `cargo clean`, because their JSON
contains absolute paths to sibling capture and evidence files.

The route-reliability baseline created at commit `e19c4557` must be preserved.
On seed `20260711004`, the new route reached a campfire before the Book of
Stabbing, defeated the Book on floor 24, and advanced to a new floor-26 combat
gap. The capsule lives at
`target/route-reliability-seed-20260711004` and is about 1 MiB.

## Considered Approaches

### Build and artifact hygiene first

Use one stable checkout path, shrink test debug output, establish a separate
location for future run artifacts, and remove only positively identified Cargo
cache directories. This directly addresses repeated cold builds and disk
growth without changing gameplay or deleting evidence. This is the selected
approach.

### Delete large groups of tests immediately

Some policy tests probably lock transitional scores, but deleting them before a
semantic audit would discard useful mechanics, schema, and regression coverage.
It would also leave the 340,000-line single-crate frontend and most link cost
unchanged. This is deferred to a separate test-semantics audit.

### Split the package into a Cargo workspace immediately

Separating core simulation, AI, evaluation, and drivers is the strongest
long-term compile-boundary improvement. Current dependencies and compatibility
paths make that a substantial architecture migration. It is deferred until the
workflow and artifact boundaries are safe and measured.

## Decision

### Stable single-checkout workflow

Development for this repository will use local feature branches in
`D:\rust\sts_simulator` rather than linked worktrees. Work starts only from a
clean checkout and uses frequent local commits. Switching branches and
fast-forward merging in the same filesystem path preserves Cargo's crate-root
identity and avoids rebuilding the same commit once per worktree path.

A short root `AGENTS.md` will record this repository-specific rule for future
agent sessions. It will also record the artifact and verification rules below.
This is an explicit project preference that overrides generic worktree advice.

### Smaller default test debug output

Add a `[profile.test]` section to `Cargo.toml` with `debug = 0`. Tests retain
debug assertions and overflow checks; only full debug information is removed.
Developers can temporarily override it with `CARGO_PROFILE_TEST_DEBUG=1` when a
debugger or richer native backtrace is genuinely needed.

Do not configure `rust-lld` in this phase. It is not on `PATH`, a portable
repository configuration would require a wrapper, and the measured incremental
loop did not improve.

### Separate future run artifacts from Cargo caches

New branch-run capsules, combat reviews, and other durable experiment output
will be written under `artifacts/runs`, which will be ignored by Git. Existing
capsules remain where they are because moving them would invalidate absolute
paths embedded in their manifests and summaries.

The repository guidance will explicitly forbid broad `cargo clean` while
legacy durable artifacts remain under `target`. Future commands must pass an
explicit `--run-capsule artifacts/runs/<name>` or equivalent output path.

### Guarded cache cleanup

Delete only directories positively identified as Cargo build caches:

- `target/debug`;
- `target/fast-run`;
- `target/release`;
- `target/dev-opt`;
- `target/release-final`;
- top-level `target/codex-verify-run-play*` directories that contain Cargo
  cache markers such as `.rustc_info.json`, `.fingerprint`, or a profile
  subdirectory.

Before deletion, resolve every absolute path and prove it is a direct child of
the repository's `target` directory. Unknown directories, run capsules,
combat cases, datasets, reports, and the new route baseline are preserved.
Do not delete `target` itself and do not call `cargo clean`.

### Verification workflow

After cleanup, rebuild and run:

1. focused route-window and route-planner tests;
2. the full library suite;
3. `architecture_runtime_boundaries`;
4. `cargo fmt -- --check`.

Then measure:

- cold focused-test build time;
- same-checkout incremental focused-test build time after a timestamp-only
  source touch;
- resulting unit-test EXE/PDB size;
- remaining sizes of the known Cargo cache directories.

The timestamp touch must leave `git status` clean. The existing route capsule
and its manifest identity must still be readable. No full seed rerun is needed
because this phase changes only build and repository workflow configuration.

## Boundaries

- Do not delete or rewrite any test in this phase.
- Do not change simulator, route, combat, reward, shop, owner, or run-control behavior.
- Do not move existing capsules whose evidence contains absolute paths.
- Do not add a linker wrapper or a second test command that creates a competing cache identity.
- Do not split crates or introduce Cargo features in this phase.
- Do not run `cargo clean` or recursively delete unknown `target` children.
- Do not delete source, design history, datasets, or run evidence merely because it is old.

## Deferred Cleanup

The next cleanup phase will classify tests by meaning:

- mechanics/parity facts;
- schema and architecture contracts;
- causal bug regressions;
- temporary exact-score or ranking locks;
- duplicated coverage.

Only the final two groups are deletion candidates, and each deletion must show
which stronger invariant replaces it. A later architecture study can then
evaluate splitting simulation core, AI policy, evaluation, and binaries into
separate crates using measured dependency boundaries.

## Success Criteria

- At least 35 GiB of positively identified Cargo cache is reclaimed.
- Existing run capsules, including the route-reliability baseline, remain readable.
- The rebuilt library test PDB is below 50 MiB.
- Same-checkout incremental focused-test compilation is no worse than 25 seconds.
- All 2,685 library tests and 7 architecture tests pass after cleanup.
- `git status` is clean and no gameplay source file changes in this phase.
- Future repository work uses one checkout path and writes new durable runs outside `target`.
