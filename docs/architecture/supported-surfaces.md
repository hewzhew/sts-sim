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

As an audit smoke check, `branch_campaign_driver`, `combat_search_v2_driver`, `decision_records`,
`rl_dataset_export`, and `run_play_driver` each executed their current `--help` path with exit code
0 after compilation. This establishes that the inspected CLI boundaries start; it does not by
itself prove support or retirement.

File size, modification date, reference count, and `v1`/`v2` naming are never sufficient to mark a
surface `CandidateRetire`. Human-invoked CLIs may legitimately have no source caller. An artifact
with an unresolved external consumer keeps its writer `Unknown`.

## Supported Surface Matrix

| Cargo surface | Entry point | Owned purpose | Active callers | Written artifacts/schemas | Artifact consumers | Overlap/replacement | Evidence | Status | Next action |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `sts_simulator` library | `src/lib.rs` | Owns game content, state transitions, simulation, AI, evaluation, and reusable run-time APIs. | All eight binaries, Rust tests, and downstream code using crate modules. | Typed modules own run capsules, panels, combat cases, datasets, and other JSON/JSONL contracts; the crate root itself performs no IO. | Maintained binaries, repository tests and tools, and human diagnostics. | None; binaries are adapters over this surface. | Cargo metadata plus public module exports in `src/lib.rs`. | `SupportedMainline` | Keep; consolidate internals only through later architecture deliveries. |
| Custom build script | `build.rs` (`build-script-build`) | Converts the compiled protocol schema into Rust enum-name adapters during every build. | Cargo automatically; watches `build.rs` and `tools/compiled_protocol_schema.json`. | `$OUT_DIR/generated_schema.rs`. | `src/testing/combat_start_spec.rs` includes the generated Rust source. | No replacement observed. | Direct writer/reader trace and architecture test `build_script_only_watches_consumed_inputs`. | `SupportedMainline` | Keep the input/watch boundary narrow. |
| `architecture_runtime_boundaries` | `tests/architecture_runtime_boundaries.rs` | Protects seven source-ownership and persistence delegation boundaries. | Completion verification and developer test runs. | None observed; assertions read source files only. | Developers and future cleanup/refactor work. | No replacement observed. | Cargo metadata and seven passing named tests. | `SupportedMainline` | Keep; revise individual assertions only with an approved ownership change. |
| `branch_campaign_driver` | `src/bin/branch_campaign_driver/main.rs` and 19 sibling modules | Runs the older Rust campaign application, artifact store, inspection, dataset, and targeted-continuation experiments. | Current `docs/RUNBOOK.md`, `tools/campaign.ps1`, `tools/README.md`, root READMEs, and humans. | Campaign report/checkpoint/state/journal JSON or JSON.GZ, manifests, command/log sidecars, latest pointers, and outcome/learning/decision JSONL; principal schemas include `BranchCampaignV1`, `BranchCampaignCheckpointV2`, `CampaignArtifactManifestV1`, and `CampaignLatestPointerV1`. | The same binary's inspect/continue/dataset commands, `tools/campaign.ps1`, and human campaign experiments. | `branch_tiny` is the newer mainline owner-audit runner, but it does not read campaign artifacts or replace targeted campaign continuation. | Active launcher and runbook commands, typed request enum, artifact store, and recent maintenance. | `SupportedDiagnostic` | Keep as a legacy diagnostic application; do not expand it into the mainline runner. |
| `branch_panel` | `src/bin/branch_panel.rs` | Inspects and schedules bounded multi-seed smoke, continuation, drain, and compare work over durable owner-audit capsules. | Root README, `docs/RUNBOOK.md`, `tools/README.md`, current durable-panel design, and human CLI use. | `panel_summary.json` (`branch_panel_summary_v0`), `panel_ledger.jsonl` (`branch_panel_ledger_event_v0`), profile capsule trees, and the underlying `branch_tiny` capsule set. | Humans, panel continuation/inspection, Rust panel tests, and follow-on diagnostics. | Replaces the retired Python `gap_panel.py`; shares `BranchRuntime` with `branch_tiny` without replacing the single-run CLI. | CLI source, `BranchArtifactStore`, current runbook, and active runtime tests. | `SupportedDiagnostic` | Keep as the supported bounded panel scheduler; do not move policy into it. |
| `branch_tiny` | `src/bin/branch_tiny.rs` | Thin mainline CLI adapter over `OwnerAuditRuntime` for a bounded owner-audit run or continuation. | Root README, `docs/RUNBOOK.md`, `tools/README.md`, capsule next-command generation, and direct human runs. | Capsule manifest/summary/result/path/terminal/chain/ledger, frontier checkpoint, trace, trajectory evidence, combat cases, and accepted-high-loss evidence; schemas include `branch_tiny_run_capsule`, `branch_tiny_capsule_summary`, `branch_tiny_run_result`, `branch_tiny_run_path`, `branch_tiny_terminal_results`, `branch_tiny_run_chain`, `branch_tiny_frontier_checkpoint`, `branch_tiny_trace_v1`, and `branch_tiny_trajectory_state_v0`. | `branch_panel`, continuation logic, `combat_case_review`, `tools/path_review.py`, dataset exporters, tests, and humans. | `BranchRuntime` is the reusable API, not a CLI replacement; `branch_panel` adds multi-seed scheduling. | Eight-line entry point, current runbook, generated next commands, schema readers, and recent bounded-mainline use. | `SupportedMainline` | Keep thin; future run-control work belongs in library ownership. |
| `combat_case_review` | `src/bin/combat_case_review.rs` and `src/bin/combat_case_review/` | Replays a saved `CombatCase` through review-only search ladders, counterfactuals, and tactical lenses. | Capsule next-command generation, root README, `docs/RUNBOOK.md`, `tools/frozen_case_panel.py`, `tools/success_feedback_panel.py`, and humans. | Standard output or `--write-review` JSON with root schema `combat_case_review`, plus nested review-only schemas such as quality, frozen-panel, Collector tactic, and strategic-feedback evidence. | Frozen-case panel, success-feedback panel, their tests, and human combat diagnosis. | `combat_search_v2_driver` starts broader whole-combat scenarios; it does not replace saved-case review. | CLI and case loader, active Python consumers, tests, runbook, and recent Collector review-lane history. | `SupportedDiagnostic` | Keep review-only; never let its lanes silently become runner policy. |
| `combat_search_v2_driver` | `src/bin/combat_search_v2_driver/main.rs` | Runs exact whole-combat starts, captures, benchmark gates, policy comparisons, explanations, and guidance labs. | Current `docs/RUNBOOK.md`, root READMEs, `tools/ml/run_turn_plan_policy_compare.ps1`, `tools/ml/run_tactical_trace_batch.ps1`, and humans. | Standard output or `--output` JSON reports, including input validation, benchmark runs/gates, comparison reports, decision microscopes, and turn-plan guidance harnesses. | ML batch scripts, benchmark/guidance analysis, and human combat diagnosis. | `combat_case_review` specializes in saved branch-gap cases; neither replaces the driver's benchmark and guidance-lab modes. | Active scripts and runbook, CLI mode validation, and recent authoritative-search/guidance commits. | `SupportedDiagnostic` | Keep as the fixed-input combat laboratory; keep non-combat policy out. |
| `rl_dataset_export` | `src/bin/rl_dataset_export.rs` | Converts one branch path, capsule, frontier, or panel tree into behavior-policy RLDS-style episodes. | Root READMEs and the active offline-ML tool chain. | `rlds_episode_dataset_v0` JSON with `observation_features_v0`, `action_features_v0`, and `candidate_group_features_v0`. | `tools/build_rl_dataset_manifest.py`, `tools/label_rl_outcomes.py`, `tools/train_imitation_candidate_ranker.py`, and downstream analysis tools. | Campaign learning datasets target observed sibling outcomes; they do not replace RLDS-style per-step episodes. | Direct writer/consumer trace, active ML help text, and recent frontier/imitation feature commits. | `SupportedDiagnostic` | Keep the behavior-policy warning and versioned feature contracts explicit. |
| `run_play_driver` | `src/bin/run_play_driver/main.rs`, `terminal.rs`, and `trace_cli.rs` | Provides the manual/semi-automatic simulator REPL, deterministic trace replay/branching, bookmarks, captures, baselines, and calibration experiments. | Current `docs/RUNBOOK.md`, root READMEs, run-control diagnostic source labels, and humans. | `SessionTraceV1`, `RunPlayBookmarkRegistryV1`, `CombatCaptureV1`, `sts_simulator.run_decision_case`, `CombatBaselineOutcomeV1`, and benchmark case files. | The same REPL's replay/goto flow, `combat_search_v2_driver`, run-control calibration extraction, benchmark tooling, and humans. | Campaign and owner-audit CLIs automate different workflows; neither replaces interactive command execution and trace branching. | Active runbook examples, schema loaders/writers, terminal tests, and recent run-control boundary maintenance. | `SupportedDiagnostic` | Keep the CLI thin over `eval::run_control`; narrow that kernel in a separate architecture delivery. |

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

### `branch_campaign_driver`

The current root README labels this as an older campaign application rather than the mainline
runner, but it is not abandoned. `docs/RUNBOOK.md` still gives campaign run and artifact-resolve
commands, while `tools/campaign.ps1` builds and invokes the executable. Its request enum covers
campaign run/continue, checkpoint and journal inspection, dataset analysis/export, targeted sibling
continuation, coverage-gap continuation, artifact management, and an ancestor-replay self-check.

Its artifact store is a distinct compatibility surface: run and scratch directories contain
campaign report, checkpoint, state, journal, manifest, command, and log files, with latest-pointer
resolution and guarded pruning. The owner-audit capsule tools do not currently consume or replace
those schemas. This makes the binary intentionally supported for diagnostics and experiments, but
not a model for new mainline functionality.

### `combat_search_v2_driver`

The driver accepts exactly one start spec, combat capture/snapshot, or benchmark suite. It can
validate input, run or gate a benchmark, compare rollout/turn-plan/frontier policies, explain a
case, and execute guidance labs. Reports are printed or written through `--output` and retain typed
schema names and versions. Current ML PowerShell batches build and invoke this exact binary, so its
use is independent of historical design documents.

### `rl_dataset_export`

This binary recursively accepts branch capsule and panel directories in addition to individual
result, frontier, or path files. It emits RLDS-style episodes with explicit terminal/truncation,
reward, action-index, observation, and feature contracts. The output is named directly by the
dataset-manifest builder and imitation-ranker CLI, and outcome-label tooling reads the associated
manifest. These are active consumers, not speculative external users.

### `run_play_driver`

The runbook still defines this as the manual or semi-automatic one-run inspection path. The binary
can record and branch `SessionTraceV1`, resume named bookmarks, auto-capture exact combat inputs,
save decision cases and combat baselines, and derive runtime card-reward calibration from traces.
Its REPL is unique among current Cargo targets. Although the underlying run-control kernel needs a
separate narrowing pass, that architecture concern is not evidence for deleting its supported
diagnostic adapter.

## Test Retention Contract

Keep game-mechanic and Java-parity tests, regressions for observed failures, serialized checkpoint
and artifact compatibility tests, and architecture/ownership tests by default. A future retirement
may remove a test only when the same delivery names the retired production behavior or a surviving
test that protects the exact contract. Test count and linked-binary size are observations, not
acceptance criteria.

## Next Cleanup Delivery

Legacy campaign stack retirement is in progress under the approved layered plan. The completed
decision-record layer will be recorded in retirement history with its commit ID during the next
layer. Run-control consolidation, combat-review pruning, and disk/cache cleanup remain separate.
