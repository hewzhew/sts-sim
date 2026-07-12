# Legacy Campaign Stack Retirement Design

**Date:** 2026-07-12

## Purpose

The repository cleanup foundation proved that the current mainline is `branch_tiny` plus the
library-owned owner-audit runtime, while `branch_campaign_driver` is an older, separate campaign
application. The user has now explicitly chosen to stop supporting that campaign workflow and its
serialized artifact compatibility in exchange for substantial repository reduction.

This delivery retires the complete legacy campaign product stack and the already-proven unused
`decision_records` exporter. It is expected to remove 49 Rust files, about 45,799 physical Rust
lines, 251 `#[test]` markers, and two Cargo binary targets. Exact post-removal counts are recorded
from the resulting tree rather than treated as quotas.

The cleanup is a product-boundary decision, not a claim that every removed file was unused.
`branch_campaign_driver` still has current documentation and a launcher; those callers are removed
with the product surface. The immutable remote backup preserves the complete pre-cleanup history at
`backup/pre-cleanup-20260712`, commit
`1ee108d0f53806f6b53c5169b74949b28e8648ce`.

## Current Scope Evidence

The measured retirement closure is:

| Component | Rust files | Physical lines | `#[test]` markers |
| --- | ---: | ---: | ---: |
| `src/bin/decision_records.rs` | 1 | 822 | 0 |
| `src/bin/branch_campaign_driver/` | 20 | 18,996 | 82 |
| `src/eval/branch_campaign.rs` and `src/eval/branch_campaign/` | 25 | 14,644 | 105 |
| `src/eval/campaign_journal.rs` | 1 | 2,111 | 5 |
| `src/eval/branch_outcome_dataset_v1.rs` | 1 | 1,172 | 1 |
| `src/eval/learning_dataset_v1.rs` | 1 | 8,054 | 58 |
| **Total** | **49** | **45,799** | **251** |

`tools/campaign.ps1` is the only maintained launcher dedicated to the old campaign application.
No campaign artifacts below `tools/artifacts` are tracked by Git. Existing ignored or external
campaign artifacts are not deleted by this delivery, but current source will no longer read them.
They remain recoverable by checking out the verified backup.

Searches found no consumers of `branch_campaign`, `campaign_journal`,
`branch_outcome_dataset_v1`, or `learning_dataset_v1` outside this campaign closure and their
module declarations. `branch_experiment` is not part of the closure: it has active consumers in
run-control, engine reward handling, decision paths, owner-audit trace output, and other mainline
code.

## Considered Approaches

### Delete the complete stack in one commit

This reaches the final size fastest, but a hidden coupling would appear among roughly 46,000 lines
of deletion at once. Failure localization and rollback would be unnecessarily coarse. This
approach is rejected.

### Migrate campaign features into the mainline before deletion

This could preserve old checkpoint, journal, targeted-continuation, and learning-dataset behavior,
but it would carry the legacy product boundary into the owner-audit runtime. The user explicitly
accepts loss of that compatibility, so migration would preserve debt that the cleanup is meant to
remove. This approach is rejected.

### Retire the stack in dependency order with verification gates

The selected approach removes the independent exporter first, then the campaign adapter and its
human-facing launcher, and finally the now-unreachable library closure. Each layer receives its own
commit and verification checkpoint. The final result is the same large reduction as a one-shot
deletion, with precise rollback boundaries.

## Product Boundary After Retirement

### Retained mainline

- `branch_tiny` remains the bounded owner-audit CLI.
- `branch_panel` remains the multi-seed inspect/continue/drain/compare scheduler.
- `src/runtime/branch` remains the durable mainline branch runtime.
- Challenger strategy repair, trajectory evidence, combat-gap capture, and capsule continuation
  remain supported.

### Retained diagnostics

- `combat_case_review` remains the saved-case review ladder.
- `combat_search_v2_driver` remains the fixed-input benchmark and guidance laboratory.
- `run_play_driver` and `eval::run_control` remain the interactive trace/bookmark/capture path.
- `rl_dataset_export` remains the supported per-step behavior-policy learning export.
- `tools/path_review.py` remains the supported human-readable owner-audit path inspector.

### Retired product behavior

- campaign run/resume and campaign-specific scheduling;
- `BranchCampaignV1` and `BranchCampaignCheckpointV2` loading and writing;
- campaign state, journal, manifest, latest-pointer, command, and log artifact management;
- campaign checkpoint/journal/coverage/decision inspection;
- campaign targeted and coverage-gap continuation workflows;
- campaign outcome, sibling-outcome, and `LearningDecisionOutcomeSampleV1` dataset workflows;
- `learning_decision_record_v0` and `path_observable_facts_v0` export.

No compatibility executable, deprecated module alias, schema reader, or migration shell is kept.

## Explicit Keep Boundary

The following are excluded even when names or concepts overlap with the legacy campaign:

- `src/eval/branch_experiment.rs` and all `branch_experiment*` modules;
- `src/eval/run_control/` and `src/eval/run_play.rs`;
- `src/runtime/branch/`;
- `src/ai/combat_search_v2/` and `src/eval/combat_search_v2/`;
- `src/bin/combat_case_review/` and frozen combat fixtures;
- `src/bin/rl_dataset_export.rs` and its Python ML consumers;
- game content, mechanics, state, engine, Java-parity, and serialization tests unrelated to the
  retired schemas;
- historical specifications and implementation plans under `docs/superpowers`.

The implementation must not replace campaign types with new owner-audit abstractions. If a
retained module unexpectedly depends on a campaign type, stop and reassess that dependency instead
of silently migrating the type into mainline ownership.

## Layered Retirement

### Layer 1: Unused decision projection

Delete `src/bin/decision_records.rs`. Remove its maintained binary-list entries from `README.md`,
`README.zh-CN.md`, and `src/bin/README.md`; update the supported-surface inventory with a retirement
record. Do not delete historical plan references.

Verification proves that:

- Cargo metadata returns seven binaries and no `decision_records` target;
- `rl_dataset_export` still compiles and its focused tests pass;
- `tests/test_path_review.py` passes;
- all remaining binaries compile.

### Layer 2: Legacy campaign application adapter

Delete `src/bin/branch_campaign_driver/` and `tools/campaign.ps1`. Remove current campaign command
examples and ownership claims from both root READMEs, `src/bin/README.md`, `docs/RUNBOOK.md`, and
`tools/README.md`. Record the retired surface in the supported-surface inventory.

The library campaign modules remain temporarily in this layer so the binary/launcher/product
boundary has an isolated commit. Cargo metadata must then return six binaries. All six must compile
before library deletion begins.

### Layer 3: Campaign-only library closure

Delete:

- `src/eval/branch_campaign.rs`;
- `src/eval/branch_campaign/`;
- `src/eval/campaign_journal.rs`;
- `src/eval/branch_outcome_dataset_v1.rs`;
- `src/eval/learning_dataset_v1.rs`.

Remove only their four module declarations from `src/eval/mod.rs`. Do not modify or rename
`branch_experiment`, `run_control`, owner-audit, RLDS, or combat-search APIs to make the deletion
compile.

The layer is complete only when repository-wide active-source searches find no remaining campaign
type or module references outside historical documentation.

## Documentation Contract

Current operator documentation must describe the six surviving binaries and the owner-audit
mainline. Historical design/specification documents remain unchanged so Git history and abandoned
architectural reasoning stay inspectable.

`docs/architecture/supported-surfaces.md` remains a current-state authority. Its Cargo matrix must
contain only the six surviving binary targets plus the library, build script, and architecture
test. A separate retirement-history section records `decision_records` and
`branch_campaign_driver`, the removal commit IDs, and the backup recovery ref; it does not add a
fifth status value to the live matrix.

## Verification Strategy

Deletion does not need replacement production tests. Safety comes from proving the retained
contracts at each layer and from compiling after the dependency direction is broken.

Focused verification includes:

```powershell
cargo test --bin rl_dataset_export
python tests\test_path_review.py
cargo check --bins
```

Final verification includes:

```powershell
cargo fmt --all -- --check
cargo test --lib
cargo check --bins
cargo test --test architecture_runtime_boundaries
python -m unittest discover -s tests -p 'test_*.py'
git diff --check
```

Additional structural checks must prove:

- Cargo metadata returns exactly these six binaries:
  `branch_panel`, `branch_tiny`, `combat_case_review`, `combat_search_v2_driver`,
  `rl_dataset_export`, and `run_play_driver`;
- no current source or operator document invokes `decision_records` or
  `branch_campaign_driver`;
- `src/eval/mod.rs` exports none of the five retired modules;
- the explicit keep-boundary paths still exist;
- no ignored campaign artifacts, `artifacts/`, `target/`, `.venv-ai/`, or remote refs changed.

Post-removal physical line, Rust file, test-marker, actual test, and Cargo-target counts are recorded
in the handoff. Count reduction is evidence that the intended closure was removed, not a reason to
delete additional code when the measured difference varies.

## Commit and Rollback Discipline

Use three bounded local commits:

1. retire the decision-record exporter;
2. retire the legacy campaign application and launcher;
3. remove the campaign-only library closure.

Documentation changes travel with the layer whose command or module they describe. If a committed
layer proves harmful, use `git revert` in reverse dependency order. Do not reset, force-push,
rewrite the immutable backup, or fold all three layers into an opaque deletion commit.

No public `master` push is authorized by this design. The existing remote backup is sufficient for
implementation safety; pushing the final cleaned `master` remains a separate user decision.

## Failure Handling

- Hidden retained-code dependency on a campaign type: stop that layer and reassess; do not migrate
  campaign types automatically.
- Remaining schema or command consumer: classify the consumer as retained or retired and update
  the design before deletion if it crosses the declared boundary.
- Focused or full test failure: diagnose the first failure before continuing to the next layer.
- Unexpected dirty or generated files: leave them untouched and identify their owner.
- Missing remote backup ref or changed hash: stop before deletion; never recreate it by force.
- Need to inspect an old campaign artifact later: use the verified backup in a separate checkout,
  not a compatibility reader added back to current mainline.

## Non-Goals

- Do not narrow `run_control` in this delivery.
- Do not prune individual combat review lenses or combat search strategies.
- Do not retire offline RLDS tooling.
- Do not delete game code or mechanic/parity tests.
- Do not clean build caches, virtual environments, or ignored artifacts.
- Do not reorganize surviving files merely to make the deletion diff larger.
- Do not migrate old campaign schemas into the owner-audit capsule format.
- Do not delete historical design and implementation documents.
