# Supported Repository Surfaces

This document is the cleanup authority for Cargo-facing product and diagnostic surfaces. It is
an inventory, not a promise that current internal boundaries are ideal. A surface remains present
until a later reviewed delivery proves that its complete production, artifact, test, and
documentation chain can be retired safely.

## Snapshot

- Local branch at freeze: `master`.
- Frozen local commit: `1ee108d0f53806f6b53c5169b74949b28e8648ce`.
- Immutable backup ref: `origin/backup/pre-cleanup-20260712`.
- Independently read remote backup hash:
  `1ee108d0f53806f6b53c5169b74949b28e8648ce`.
- Public `origin/master` at freeze:
  `5643238ad85af6f11833452ab78c15a9df975a42`.
- `origin/master...HEAD` at freeze: 0 commits behind, 196 commits ahead.
- Verification at the frozen commit:
  - `cargo fmt --all -- --check`: passed;
  - `cargo test --lib --quiet`: 2,811 passed, 0 failed;
  - `cargo check --bins`: passed for all eight binaries;
  - `cargo test --test architecture_runtime_boundaries --quiet`: 7 passed, 0 failed;
  - `git diff --check`: passed;
  - worktree: clean.

The backup contains tracked Git source and history only. Ignored `artifacts/`, `target/`,
`.venv-ai/`, local logs, and generated outputs are not part of it and were not modified by the
cleanup foundation. Design-time observations of roughly 20 MiB of run capsules, 9.8 GiB of Cargo
cache, and 875 MiB of Python environment data are disk-management evidence only.

The initial HTTPS push could not reach the system-DNS address for `github.com`. Windows user proxy
settings pointed to the active V2Ray/Xray listener at `127.0.0.1:10808`, while Git and WinHTTP had
no proxy configured. The backup was therefore pushed and verified with a command-local
`http.proxy=http://127.0.0.1:10808`; repository, global Git, V2Ray, and ProxyBridge configuration
were not changed.

## Repository Baseline

Counts were taken immediately before creating this file, after the design and implementation plan
had been committed:

| Measure | Value | Counting contract |
| --- | ---: | --- |
| Tracked files | 1,975 | `git ls-files` |
| Tracked bytes | 15,124,422 (about 14.42 MiB) | Sum of working-tree file lengths for `git ls-files` |
| Rust files | 1,845 | `rg --files -g '*.rs'`, respecting ignore rules |
| Physical Rust lines | 374,095 | Count of `Get-Content` records, including blank lines |
| Nonblank-style Rust line measure | 347,457 | PowerShell `Measure-Object -Line`; retained for comparison with the design-time estimate |
| `#[test]` markers | 2,971 | `rg -n '#\[test\]' -g '*.rs'` |
| `#[cfg(test)]` markers | 500 | `rg -n '#\[cfg\(test\)\]' -g '*.rs'` |
| Design specifications | 41 | Files under `docs/superpowers/specs` |
| Implementation plans | 43 | Files under `docs/superpowers/plans` |
| Cargo binaries | 8 | `cargo metadata --no-deps --format-version 1` |

The physical and nonblank-style line counts intentionally coexist: the earlier cleanup design used
the latter measurement, while physical lines are the less ambiguous baseline for future diffs.
Neither count is a cleanup quota.

## Status Vocabulary

- `SupportedMainline`: required to build, execute, or protect the current mainline run workflow.
- `SupportedDiagnostic`: intentionally retained to inspect, replay, compare, or explain run and
  combat evidence.
- `CandidateRetire`: all retirement-proof rules are satisfied; this permits a later retirement
  design, not deletion in the foundation.
- `Unknown`: evidence is incomplete or an external consumer may exist; deletion is forbidden.

No other status has cleanup meaning.

## Classification Method

Classification combines Cargo metadata, current source and operator documentation, CLI ownership,
artifact writers and readers, focused tests, and recent Git history. Searches exclude historical
`docs/superpowers` plans when deciding whether a caller is active. Historical plans remain useful
design evidence but cannot establish current support by themselves.

File size, modification date, reference count, and `v1`/`v2` naming are never sufficient to mark a
surface `CandidateRetire`. Human-invoked CLIs may legitimately have no source caller. An artifact
with an unresolved external consumer keeps its writer `Unknown`.

## Supported Surface Matrix

| Cargo surface | Entry point | Owned purpose | Active callers | Written artifacts/schemas | Artifact consumers | Overlap/replacement | Evidence | Status | Next action |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `sts_simulator` library | `src/lib.rs` | Owns game content, state transitions, simulation, AI, evaluation, and reusable run-time APIs. | All eight binaries, Rust tests, and downstream code using crate modules. | Typed modules own run capsules, panels, combat cases, datasets, and other JSON/JSONL contracts; the crate root itself performs no IO. | Maintained binaries, repository tests and tools, and human diagnostics. | None; binaries are adapters over this surface. | Cargo metadata plus public module exports in `src/lib.rs`. | `SupportedMainline` | Keep; consolidate internals only through later architecture deliveries. |
| Custom build script | `build.rs` (`build-script-build`) | Converts the compiled protocol schema into Rust enum-name adapters during every build. | Cargo automatically; watches `build.rs` and `tools/compiled_protocol_schema.json`. | `$OUT_DIR/generated_schema.rs`. | `src/testing/combat_start_spec.rs` includes the generated Rust source. | No replacement observed. | Direct writer/reader trace and architecture test `build_script_only_watches_consumed_inputs`. | `SupportedMainline` | Keep the input/watch boundary narrow. |
| `architecture_runtime_boundaries` | `tests/architecture_runtime_boundaries.rs` | Protects seven source-ownership and persistence delegation boundaries. | Completion verification and developer test runs. | None observed; assertions read source files only. | Developers and future cleanup/refactor work. | No replacement observed. | Cargo metadata and seven passing named tests. | `SupportedMainline` | Keep; revise individual assertions only with an approved ownership change. |
| `branch_campaign_driver` | `src/bin/branch_campaign_driver/main.rs` | Declared campaign application for run, inspect, dataset, continuation, and self-check commands. | Pending complete active-caller audit; listed in `src/bin/README.md`. | Pending Task 4 schema and path audit. | Unknown. | Potential overlap with `branch_tiny`, `branch_panel`, and exporters is not yet mapped. | Cargo metadata and binary ownership README only; insufficient for retirement or support proof. | `Unknown` | Trace each subcommand and artifact consumer before classification. |
| `branch_panel` | `src/bin/branch_panel.rs` | Inspects and schedules bounded multi-seed smoke, continuation, drain, and compare work over durable owner-audit capsules. | Root README, `docs/RUNBOOK.md`, `tools/README.md`, current durable-panel design, and human CLI use. | `panel_summary.json` (`branch_panel_summary_v0`), `panel_ledger.jsonl` (`branch_panel_ledger_event_v0`), profile capsule trees, and the underlying `branch_tiny` capsule set. | Humans, panel continuation/inspection, Rust panel tests, and follow-on diagnostics. | Replaces the retired Python `gap_panel.py`; shares `BranchRuntime` with `branch_tiny` without replacing the single-run CLI. | CLI source, `BranchArtifactStore`, current runbook, and active runtime tests. | `SupportedDiagnostic` | Keep as the supported bounded panel scheduler; do not move policy into it. |
| `branch_tiny` | `src/bin/branch_tiny.rs` | Thin mainline CLI adapter over `OwnerAuditRuntime` for a bounded owner-audit run or continuation. | Root README, `docs/RUNBOOK.md`, `tools/README.md`, capsule next-command generation, and direct human runs. | Capsule manifest/summary/result/path/terminal/chain/ledger, frontier checkpoint, trace, trajectory evidence, combat cases, and accepted-high-loss evidence; schemas include `branch_tiny_run_capsule`, `branch_tiny_capsule_summary`, `branch_tiny_run_result`, `branch_tiny_run_path`, `branch_tiny_terminal_results`, `branch_tiny_run_chain`, `branch_tiny_frontier_checkpoint`, `branch_tiny_trace_v1`, and `branch_tiny_trajectory_state_v0`. | `branch_panel`, continuation logic, `combat_case_review`, `tools/path_review.py`, dataset exporters, tests, and humans. | `BranchRuntime` is the reusable API, not a CLI replacement; `branch_panel` adds multi-seed scheduling. | Eight-line entry point, current runbook, generated next commands, schema readers, and recent bounded-mainline use. | `SupportedMainline` | Keep thin; future run-control work belongs in library ownership. |
| `combat_case_review` | `src/bin/combat_case_review.rs` and `src/bin/combat_case_review/` | Replays a saved `CombatCase` through review-only search ladders, counterfactuals, and tactical lenses. | Capsule next-command generation, root README, `docs/RUNBOOK.md`, `tools/frozen_case_panel.py`, `tools/success_feedback_panel.py`, and humans. | Standard output or `--write-review` JSON with root schema `combat_case_review`, plus nested review-only schemas such as quality, frozen-panel, Collector tactic, and strategic-feedback evidence. | Frozen-case panel, success-feedback panel, their tests, and human combat diagnosis. | `combat_search_v2_driver` starts broader whole-combat scenarios; it does not replace saved-case review. | CLI and case loader, active Python consumers, tests, runbook, and recent Collector review-lane history. | `SupportedDiagnostic` | Keep review-only; never let its lanes silently become runner policy. |
| `combat_search_v2_driver` | `src/bin/combat_search_v2_driver/main.rs` | Declared whole-combat runner for start specs, captures, benchmarks, and guidance labs. | Pending complete active-caller audit; listed in `src/bin/README.md`. | Pending Task 4 schema and path audit. | Unknown. | Possible diagnostic overlap with `combat_case_review` remains unmapped. | Cargo metadata and ownership README only. | `Unknown` | Trace scenario inputs, outputs, and unique labs before classification. |
| `decision_records` | `src/bin/decision_records.rs` | Declared typed decision-record inspection/export utility without policy ownership. | Pending complete active-caller audit; listed in `src/bin/README.md`. | Known candidates include decision/path JSONL; exact schemas and consumers pending Task 4. | Unknown. | Possible overlap with `rl_dataset_export` and campaign dataset commands remains unmapped. | Cargo metadata and ownership README only. | `Unknown` | Resolve both artifact schemas and every consumer before considering retirement. |
| `rl_dataset_export` | `src/bin/rl_dataset_export.rs` | Declared offline imitation/RL behavior-policy sample exporter. | Pending complete active-caller audit; listed in `src/bin/README.md`. | Pending Task 4 dataset schema and path audit. | Unknown, including possible external ML consumers. | Possible overlap with campaign dataset commands and `decision_records` remains unmapped. | Cargo metadata and ownership README only. | `Unknown` | Trace source artifacts, output contract, and external-consumer risk. |
| `run_play_driver` | `src/bin/run_play_driver/main.rs` | Declared manual/semi-automatic REPL over `eval::run_control` with traces, bookmarks, captures, baselines, and panels. | Pending complete active-caller audit; listed in `src/bin/README.md`. | Pending Task 4 trace/bookmark/capture schema audit. | Unknown. | Possible overlap with campaign and owner-audit run surfaces remains unmapped. | Cargo metadata and ownership README only. | `Unknown` | Establish whether the interactive workflow remains unique and supported. |

## Surface Evidence

### Library, Build, and Architecture Boundary

Cargo metadata returns the library, `build-script-build`, and
`architecture_runtime_boundaries` as distinct targets. `build.rs` reads only
`tools/compiled_protocol_schema.json`, emits `$OUT_DIR/generated_schema.rs`, and is consumed by
`src/testing/combat_start_spec.rs`. The architecture test confirms seven boundaries: runtime code
does not path-import old binary modules; capsule filesystem writes are delegated; recovery
persistence is separated; the panel scheduler does not know capsule filenames; slice-result
construction and persistence are delegated; and the build script watches only consumed inputs.

These tests protect current ownership, not every historical file arrangement. A later approved
architecture change may update them together with the boundary it intentionally replaces.

### `branch_tiny`

`src/bin/branch_tiny.rs` contains only error handling around `OwnerAuditRuntime::run_cli`; campaign
logic no longer lives below the binary directory. It is the root README's primary bounded-run
command, and run capsules synthesize `branch_tiny --continue-capsule` as their supported
continuation command. Its artifacts are actively read by panel scheduling, path review, combat-case
review, and dataset tooling, so the thin executable remains the mainline human entry point even
though its implementation is deliberately tiny.

### `branch_panel`

The binary calls library-owned `PanelSmokeRunner`, `PanelInspectConfig`, and
`BranchArtifactStore` directly; it does not spawn or parse `branch_tiny`. Its `inspect`, `smoke`,
`continue`, `drain`, and `compare` commands are documented in the current runbook. The artifact
store owns `panel_summary.json`, `panel_ledger.jsonl`, seed capsule paths, and compare-profile
subtrees. Current documentation explicitly says the retired Python `gap_panel.py` must not return.

### `combat_case_review`

The binary loads the library's `combat_case`/legacy `combat_gap_case` input and emits a typed review
payload. Owner-audit capsule summaries generate this CLI as the next recommended command for combat
gaps. `tools/frozen_case_panel.py` and `tools/success_feedback_panel.py` invoke or parse its output,
with Python tests protecting the root schema. Recent Git history adds Collector and Awakened One
review evidence, showing current diagnostic maintenance rather than historical-only references.

## First Retirement Recommendation

Pending the remaining binary and artifact-chain audit. No surface is `CandidateRetire` at this
checkpoint; unresolved entries are `Unknown` and deletion is forbidden.

## Test Retention Contract

Keep game-mechanic and Java-parity tests, regressions for observed failures, serialized checkpoint
and artifact compatibility tests, and architecture/ownership tests by default. A future retirement
may remove a test only when the same delivery names the retired production behavior or a surviving
test that protects the exact contract. Test count and linked-binary size are observations, not
acceptance criteria.

## Next Cleanup Delivery

Pending the first retirement-proof conclusion. Tool retirement, test-contract cleanup, run-control
architecture consolidation, and disk/cache cleanup remain separate deliveries with independent
design, verification, and rollback boundaries.
