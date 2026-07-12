# Repository Cleanup Foundation Design

**Date:** 2026-07-12

## Purpose

The repository has reached a size where undirected cleanup would be dangerous. It contains about
347,000 lines of Rust across 1,844 Rust files, 2,971 `#[test]` markers, eight binary targets, and
471 paths with explicit `v1` or `v2` naming. Much of that volume is real game-mechanic coverage;
file age, file size, version suffixes, and test count are not sufficient evidence for deletion.

The local `master` is also 194 commits ahead of `origin/master`. Those commits touch 314 files and
contain roughly 39,700 added lines. Before removing any source, the project needs an independently
verifiable remote snapshot and a declared supported product surface.

This delivery creates that cleanup foundation. It does not delete source code, tests, generated
run evidence, or build caches.

## Current Inventory

The inventory at design time records:

- 1,973 tracked files totaling about 14.4 MiB;
- 1,844 Rust files totaling about 347,364 lines;
- 2,971 `#[test]` attributes and 499 inline test modules;
- 40 design specs and 42 implementation plans;
- eight Cargo binary targets;
- 13 ignored run capsules totaling about 20 MiB;
- about 9.8 GiB under `target` and 875 MiB under `.venv-ai`.

The largest source areas include:

- `src/content/cards`: about 37,753 lines;
- `src/ai/combat_search_v2`: about 32,102 lines;
- `src/eval/run_control`: about 25,032 lines;
- `src/content/monsters`: about 23,420 lines;
- `src/content/events`: about 21,990 lines;
- `src/bin/branch_campaign_driver`: about 17,794 lines;
- `src/runtime/branch`: about 17,144 lines;
- `src/ai/strategy`: about 11,288 lines.

These numbers are baseline evidence, not reduction targets.

## Considered Approaches

### Delete by age, size, or version suffix

This could remove many lines quickly, but none of those signals proves that a mechanism, artifact
schema, or diagnostic workflow is unused. It would especially endanger rare content behavior and
backward-compatible run evidence. This approach is rejected.

### Only split files and reorganize directories

This improves local readability but preserves every historical product surface and dependency.
It also does not reduce the Rust library test binary merely by moving tests between source files.
Reorganizing dead code before deciding whether it should exist is wasted work. This approach is
rejected as the first step.

### Freeze, classify, prove, retire, then reshape

The selected approach first preserves the exact source state, then declares supported entry
points, and only later retires one independently verified chain at a time. Surviving code is
refactored after deletion evidence is complete.

## Cleanup Program Decomposition

Repository cleanup is not one implementation project. It is divided into independent deliveries:

1. **Cleanup foundation**: remote snapshot, supported-surface inventory, deletion proof rules, and
   selection of the first retirement candidate. This specification covers only this delivery.
2. **Experimental tool retirement**: one candidate binary/tool chain per reviewed specification.
3. **Test contract cleanup**: remove tests only with explicit redundant or retired-contract proof.
4. **Architecture consolidation**: narrow surviving run-control, branch, and versioned strategy
   boundaries.
5. **Disk/cache cleanup**: separately audit safe direct children of `target` and local environments
   without `cargo clean`.

Each future delivery needs its own design, plan, verification, and local commit history.

## Remote Snapshot Contract

The snapshot source is the final clean local `HEAD` immediately before the first cleanup
implementation begins. It includes this design and its implementation plan. It must not point to
the earlier design-time hash if documentation commits have advanced the branch.

The implementation creates a remote branch named:

`backup/pre-cleanup-20260712`

The snapshot procedure is:

1. require a clean worktree;
2. record local `HEAD`, `origin/master`, and ahead/behind state;
3. run the repository completion verification required by the current workflow;
4. query the remote backup ref before writing it;
5. if the ref is absent, push local `HEAD` to that exact remote branch without force;
6. query the remote ref again and require its object ID to equal recorded local `HEAD`;
7. record the verified hash in the supported-surface inventory.

If the remote branch already exists at the same hash, the operation is idempotently complete. If
it exists at a different hash, stop and request a new backup name. Never force-update the backup
ref and never rewrite local history for cleanup aesthetics.

Updating public `origin/master` is not part of the foundation. Once the backup ref is verified,
the user may separately authorize a normal fast-forward push of `master`. A compact-looking commit
history is not sufficient reason to squash or rebase 194 validated local commits.

## Ignored Data Boundary

Git remote backup includes tracked source and history only. It does not include:

- `artifacts/`, including the 13 current run capsules;
- `target/` build caches;
- `.venv-ai/`;
- other ignored logs, temporary data, and generated outputs.

The cleanup foundation does not modify those directories. Run capsules remain reproducible or
diagnostic evidence, not repository source. If the user later requests remote preservation of run
evidence, use a separate archive or release-asset design rather than committing generated capsules
to the main repository.

## Supported Surface Inventory

Create `docs/architecture/supported-surfaces.md` as the human-readable cleanup authority. It lists
the library, custom build script, architecture integration test, and all eight current binary
targets:

- `branch_campaign_driver`;
- `branch_panel`;
- `branch_tiny`;
- `combat_case_review`;
- `combat_search_v2_driver`;
- `decision_records`;
- `rl_dataset_export`;
- `run_play_driver`;
- the implicit library target is recorded separately from binaries.

The table records for every target:

- Cargo target and source entry point;
- one-sentence owned purpose;
- known human, script, documentation, or code callers;
- artifact files or schemas it writes;
- consumers of those artifacts;
- overlapping or replacement target, if any;
- most recent supporting evidence;
- status;
- next action.

Allowed statuses are:

- `SupportedMainline`: required for the current mainline run workflow;
- `SupportedDiagnostic`: intentionally retained for investigation or replay;
- `CandidateRetire`: deletion evidence is complete enough to write a retirement specification;
- `Unknown`: evidence is incomplete, so deletion is forbidden.

The inventory also records the verified remote backup ref and hash at its top.

## Evidence Collection

Classification uses repository evidence rather than intuition:

1. Cargo metadata establishes actual targets.
2. `rg` finds callers in source, tools, tracked documentation, and scripts.
3. CLI definitions and entry points establish unique capabilities.
4. Artifact writers and readers establish schema dependencies.
5. Git history establishes recent maintenance but cannot alone prove support or retirement.
6. Focused smoke commands establish whether a declared supported target still starts or inspects
   its intended input.

Reference count is not sufficient by itself. A zero-reference binary may be a user-facing entry
point; a highly referenced binary may appear only in historical plans. Evidence notes must
distinguish active workflow references from historical documentation.

## Retirement Proof Rules

A future delivery may mark a target `CandidateRetire` only when all of these are documented:

- no active source, script, current architecture document, or supported command invokes it;
- every unique capability is mapped to a supported replacement or explicitly declared no longer
  required;
- every artifact schema it writes has no active consumer, or the consumer migration is part of
  the same retirement delivery;
- removing it does not require a permanent compatibility shell with no behavior;
- related tests and documentation have an explicit keep, migrate, or delete disposition;
- focused tests, full library tests, all remaining target compilation, and architecture boundary
  tests define the post-removal verification contract.

Ambiguous evidence yields `Unknown`, not `CandidateRetire`.

## Test Deletion Rules

Future test cleanup may remove only:

- tests whose production target is retired in the same reviewed delivery;
- exact duplicate contracts already protected by a named surviving test;
- assertions that lock a temporary seed order, transient numeric score, or private structure while
  protecting no public or architectural behavior.

The following are retained by default:

- game-mechanic fidelity and Java parity tests;
- regression tests for previously observed real failures;
- serialized checkpoint, capsule, JSON, and source-identity compatibility;
- architecture and ownership boundary tests;
- tests that distinguish legal execution from diagnostic evidence.

Every removed test lists its replacement contract or the retired production behavior. Test-count
reduction and line-count reduction are observations, not acceptance criteria.

## Change and Rollback Discipline

Each future retirement removes one bounded chain and uses its own commits. The handoff records:

- deleted production paths;
- deleted or migrated tests;
- deleted or updated documentation;
- supported replacement command;
- before/after target, file, line, and test counts;
- exact verification results.

If a retirement is later found harmful, use `git revert` on its bounded commits. Do not reset,
force-push, or rewrite the verified backup history.

## Foundation Deliverables

This delivery produces only:

1. the verified remote backup branch and hash;
2. `docs/architecture/supported-surfaces.md` covering every Cargo target;
3. one evidence-backed recommendation for the first retirement candidate;
4. baseline repository counts and verification results in that inventory.

It does not change Rust source, Cargo targets, tests, run artifacts, or caches.

## Verification

The foundation is complete when:

1. local and remote backup hashes match;
2. the supported-surface table contains the library, build script, architecture test, and all eight
   binaries returned by Cargo metadata;
3. every target has callers, artifacts, replacement, status, evidence, and next action recorded;
4. exactly one first retirement candidate is recommended, or the inventory explicitly concludes
   that all candidates remain `Unknown`;
5. no Rust, Cargo, test, artifact, target-cache, or virtual-environment file changed;
6. documentation formatting and `git diff --check` pass;
7. the worktree is clean after committing the inventory.

## Failure Handling

- Remote unavailable: stop before cleanup; local commits alone do not satisfy remote backup.
- Existing backup ref has another hash: do not overwrite it; choose a new dated/suffixed name only
  after user direction.
- Dirty worktree: stop and identify the owner of changes before snapshotting.
- Unknown target purpose or artifact consumer: classify `Unknown` and retain it.
- Smoke command fails: record the failure as evidence; do not delete the target in the foundation.
- Generated or ignored data appears in Git staging: unstage it and keep it outside the source
  snapshot.

## Non-Goals

- Do not delete or refactor Rust source in the foundation.
- Do not remove tests or reduce target count yet.
- Do not push or rewrite public `master` without separate authorization.
- Do not commit generated run capsules.
- Do not clean `target`, `.venv-ai`, or ignored artifact directories.
- Do not split giant files merely to report smaller file sizes.
- Do not set a code-line or test-count reduction quota.
