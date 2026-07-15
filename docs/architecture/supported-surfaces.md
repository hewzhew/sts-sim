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

## Post-Retirement Baseline

Counts below describe the working tree after the three-layer legacy campaign retirement. The
tracked-file and byte totals include the cleanup design and implementation plan, which were written
after the frozen baseline above.

| Measure | Value | Change evidence |
| --- | ---: | --- |
| Tracked files | 1,928 | 50 tracked production/tool files retired; later cleanup documents remain tracked |
| Tracked bytes | 13,458,284 | Exact sum of existing working-tree files returned by `git ls-files` |
| Rust files | 1,796 | 49 retired Rust files |
| Physical Rust lines | 328,197 | 45,898 fewer than the frozen 374,095-line source tree |
| `#[test]` markers | 2,720 | 251 campaign-only markers retired |
| `#[cfg(test)]` markers | 424 | 76 campaign-only test modules retired |
| Passing library tests | 2,642 | 169 campaign-only library tests retired from the 2,811-test suite |
| Cargo binaries | 6 | `decision_records` and `branch_campaign_driver` retired |

The physical-line reduction exceeds the 45,799 lines in the file-deletion closure by 99 lines:
four removed `src/eval/mod.rs` declarations plus 95 lines of decision-axis composition helpers that
became unreferenced when campaign learning datasets were removed. The two shared shop-axis key
helpers remain because `branch_experiment_boundary::shop` still calls them.

## Post-Lens-Retirement Baseline

Counts below describe the working tree after retiring the orphan `combat_case_review` experiment
families and their Collector-only search policy. Two cleanup documents added after the campaign
baseline remain tracked, so tracked-file and byte changes are cumulative tree measurements rather
than the raw size of deleted source alone.

| Measure | Value | Change from post-campaign baseline |
| --- | ---: | --- |
| Tracked files | 1,910 | 20 Rust files retired and two cleanup documents added |
| Tracked bytes | 13,417,421 | 40,863 fewer bytes after including the new design and plan |
| Rust files | 1,776 | 20 retired Rust files |
| Physical Rust lines | 325,864 | 2,333 fewer lines |
| `#[test]` markers | 2,704 | 16 experiment-only markers retired |
| `#[cfg(test)]` markers | 419 | 5 experiment-only test modules retired |
| Passing library tests | 2,631 | 11 Collector-policy library tests retired |
| Passing `combat_case_review` tests | 20 | 5 adapter-only binary tests retired |
| Cargo binaries | 6 | Binary target set unchanged |

## Post-Branch-Experiment-Retirement Baseline

Counts below describe the 2026-07-15 working tree after retiring the unreachable legacy branch
experiment product. This is a dependency-closure retirement: repository-wide active-source
searches found no caller of any branch experiment runner, and removing those runners made the
boundary, retention, trajectory, decision-path, and branch-only auto-run adapters compiler-visible
as unused code. The retained `branch_tiny`, `branch_panel`, owner-audit, ordinary run-control, and
game-mechanic surfaces compile without warnings and pass their existing tests.

| Measure | Value | Change evidence |
| --- | ---: | --- |
| Rust files under `src` | 1,788 | 25 branch-experiment closure files retired |
| Physical Rust lines under `src` | 333,442 | Current tree measurement after intervening mainline development |
| `#[test]` markers under `src` | 2,838 | 138 library tests retired from the pre-delivery 2,891-test suite |
| Rust files containing `#[test]` | 415 | 10 self-testing legacy product files retired |
| Passing library tests | 2,753 | 0 failed |
| Linked library test binary | 49.07 MiB | Down from approximately 51.6 MiB; size is observational only |

The delivery removes 14,595 net Rust lines. It does not prune tests from cards, relics, monsters,
events, combat search, owner-audit, or ordinary run-control. The still-live
`BranchSkipCardReward` command is also retained despite its historical name because the current
run-control decision surface and owner-audit renderer consume it.

## Post-Legacy-Shop-Bundle-Retirement Baseline

Counts below describe the 2026-07-15 working tree after removing the legacy
`shop_purchase_bundle` policy from the generic decision pipeline and ShopTiny owner. The retired
module mixed candidate evidence, whole-shop opportunity costs, owner verdicts, score weights, and
string reason codes. Whole-shop planning now has an explicit owner boundary: the generic pipeline
may expose candidate evidence, while an owner/compiler must choose between purchases, cleanup,
future liquidity, and leaving.

| Measure | Value | Change evidence |
| --- | ---: | --- |
| Rust files under `src` | 1,787 | The 936-line legacy bundle module retired |
| Physical Rust lines under `src` | 331,957 | Current exact physical-line measurement |
| `#[test]` markers under `src` | 2,814 | 24 bundle/string-owner tests retired |
| Rust files containing `#[test]` | 414 | The bundle self-test file retired |
| Passing library tests | 2,729 | 0 failed |
| Linked library test binary | 49.01 MiB | Size is observational only |

The delivery removes 1,527 net Rust lines. Boss scaling and survival repair remain typed evidence
in `decision_pipeline`; the hard-checkpoint boss bundle preview now explicitly promotes the next
owner-selected executable step instead of requiring prior generic-pipeline approval. Maw Bank and
future-shop opportunity costs are intentionally no longer hidden filters in the candidate
pipeline. If restored, they must be modeled by the whole-shop plan compiler rather than by string
reason codes. The existing `shop_policy_v1` compiler passes its focused tests but is not yet the
production ShopTiny owner.

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

The cleanup foundation used CLI `--help` smoke checks as one input to classification. Later
retirement deliveries removed `branch_campaign_driver` and `decision_records`; neither remains a
Cargo target. Help output establishes only that a retained CLI boundary starts, not that every
nested diagnostic is supported.

File size, modification date, reference count, and `v1`/`v2` naming are never sufficient to mark a
surface `CandidateRetire`. Human-invoked CLIs may legitimately have no source caller. An artifact
with an unresolved external consumer keeps its writer `Unknown`.

## Supported Surface Matrix

| Cargo surface | Entry point | Owned purpose | Active callers | Written artifacts/schemas | Artifact consumers | Overlap/replacement | Evidence | Status | Next action |
| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| `sts_simulator` library | `src/lib.rs` | Owns game content, state transitions, simulation, AI, evaluation, and reusable run-time APIs. | All six binaries, Rust tests, and downstream code using crate modules. | Typed modules own run capsules, panels, combat cases, datasets, and other JSON/JSONL contracts; the crate root itself performs no IO. | Maintained binaries, repository tests and tools, and human diagnostics. | None; binaries are adapters over this surface. | Cargo metadata plus public module exports in `src/lib.rs`. | `SupportedMainline` | Keep; consolidate internals only through later architecture deliveries. |
| Custom build script | `build.rs` (`build-script-build`) | Converts the compiled protocol schema into Rust enum-name adapters during every build. | Cargo automatically; watches `build.rs` and `tools/compiled_protocol_schema.json`. | `$OUT_DIR/generated_schema.rs`. | `src/testing/combat_start_spec.rs` includes the generated Rust source. | No replacement observed. | Direct writer/reader trace and architecture test `build_script_only_watches_consumed_inputs`. | `SupportedMainline` | Keep the input/watch boundary narrow. |
| `architecture_runtime_boundaries` | `tests/architecture_runtime_boundaries.rs` | Protects seven source-ownership and persistence delegation boundaries. | Completion verification and developer test runs. | None observed; assertions read source files only. | Developers and future cleanup/refactor work. | No replacement observed. | Cargo metadata and seven passing named tests. | `SupportedMainline` | Keep; revise individual assertions only with an approved ownership change. |
| `branch_panel` | `src/bin/branch_panel.rs` | Inspects and schedules bounded multi-seed smoke, continuation, drain, and compare work over durable owner-audit capsules. | Root README, `docs/RUNBOOK.md`, `tools/README.md`, current durable-panel design, and human CLI use. | `panel_summary.json` (`branch_panel_summary_v0`), `panel_ledger.jsonl` (`branch_panel_ledger_event_v0`), profile capsule trees, and the underlying `branch_tiny` capsule set. | Humans, panel continuation/inspection, Rust panel tests, and follow-on diagnostics. | Replaces the retired Python `gap_panel.py`; shares `BranchRuntime` with `branch_tiny` without replacing the single-run CLI. | CLI source, `BranchArtifactStore`, current runbook, and active runtime tests. | `SupportedDiagnostic` | Keep as the supported bounded panel scheduler; do not move policy into it. |
| `branch_tiny` | `src/bin/branch_tiny.rs` | Thin mainline CLI adapter over `OwnerAuditRuntime` for a bounded owner-audit run or continuation. | Root README, `docs/RUNBOOK.md`, `tools/README.md`, capsule next-command generation, and direct human runs. | Capsule manifest/summary/result/path/terminal/chain/ledger, frontier checkpoint, trace, trajectory evidence, combat cases, and accepted-high-loss evidence; schemas include `branch_tiny_run_capsule`, `branch_tiny_capsule_summary`, `branch_tiny_run_result`, `branch_tiny_run_path`, `branch_tiny_terminal_results`, `branch_tiny_run_chain`, `branch_tiny_frontier_checkpoint`, `branch_tiny_trace_v1`, and `branch_tiny_trajectory_state_v0`. | `branch_panel`, continuation logic, `combat_case_review`, `tools/path_review.py`, dataset exporters, tests, and humans. | `BranchRuntime` is the reusable API, not a CLI replacement; `branch_panel` adds multi-seed scheduling. | Eight-line entry point, current runbook, generated next commands, schema readers, and recent bounded-mainline use. | `SupportedMainline` | Keep thin; future run-control work belongs in library ownership. |
| `combat_case_review` | `src/bin/combat_case_review.rs` and `src/bin/combat_case_review/` | Replays a saved `CombatCase` through the supported review ladder, named panels, counterfactual HP, and boss evidence. | Capsule next-command generation, root README, `docs/RUNBOOK.md`, `tools/frozen_case_panel.py`, `tools/success_feedback_panel.py`, and humans. | Standard output or `--write-review` JSON with root schema `combat_case_review`, plus nested quality, frozen-panel, line-lab, HP, boss, lifecycle, and strategic-feedback evidence. | Frozen-case panel, success-feedback panel, frozen-panel tests, and human combat diagnosis. | `combat_search_v2_driver` starts broader whole-combat scenarios; it does not replace saved-case review. | CLI and case loader, active Python consumers, retained binary tests, and current runbook. | `SupportedDiagnostic` | Keep review-only; never let its lanes silently become runner policy. |
| `combat_search_v2_driver` | `src/bin/combat_search_v2_driver/main.rs` | Runs exact whole-combat starts, captures, benchmark gates, policy comparisons, explanations, guidance labs, and the resumable offline Combat Laboratory V1. | Current `docs/RUNBOOK.md`, maintained combat-lab fixtures, root READMEs, `tools/ml/run_turn_plan_policy_compare.ps1`, `tools/ml/run_tactical_trace_batch.ps1`, and humans. | Standard output or `--output` JSON reports; laboratory mode writes `manifest.json`, authoritative append-only `cells.jsonl`, rebuildable `checkpoint.json`, and reproducible `summary.json` under `artifacts/runs`. | ML batch scripts, benchmark/guidance analysis, offline laboratory resume/extension and summary regeneration, and human combat diagnosis. | `combat_case_review` specializes in saved branch-gap cases; Combat Laboratory V1 remains a mode of this driver and does not replace or feed live run-control, route, or acquisition policy. | Active scripts and runbook, maintained seed006-derived fixture, CLI mode validation, artifact/resume tests, and the offline dependency-direction guard. | `SupportedDiagnostic` | Keep fixed-input experiments sequential and descriptive; preserve outcome/coverage separation and require recorded commits to rerun removed historical profiles. |
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
with frozen-panel tests protecting the active nested schema and success-feedback receiving a CLI
smoke check. The generic ladder, quality/frozen lanes, line-lab, HP probe, and boss/lifecycle
evidence remain; the one-off setup, potion, key-card, root-action, and Collector lenses do not.

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

## Retirement History

### `decision_records`

- Removal commit: `bd69e90afeb30a90c5b6c93de77e96640a0d6dc2`
  (`chore: retire decision records exporter`).
- Removed contracts: `learning_decision_record_v0` and `path_observable_facts_v0`.
- Replacements: `rl_dataset_export` for per-step behavior-policy data and
  `tools/path_review.py` for human path inspection.
- Recovery: `origin/backup/pre-cleanup-20260712` at
  `1ee108d0f53806f6b53c5169b74949b28e8648ce`.

### `branch_campaign_driver` and campaign-only library

- Application removal commit: `aed59982611d7db25aca8d36aea09956f323d8c7`
  (`chore: retire legacy campaign application`).
- Library closure: `008326e63cb8b9e471e409dfa9d9ba8d6f941b81`
  (`chore: remove legacy campaign library stack`).
- Removed contracts: `BranchCampaignV1`, `BranchCampaignCheckpointV2`, campaign journal,
  campaign artifact pointers/manifests, targeted continuation, and campaign learning datasets.
- Replacement: none; the product boundary was explicitly retired. `branch_tiny` and
  `branch_panel` remain the supported mainline rather than compatibility readers.
- Recovery: `origin/backup/pre-cleanup-20260712` at
  `1ee108d0f53806f6b53c5169b74949b28e8648ce`.

### Orphan `combat_case_review` lenses and Collector policy

- Adapter removal commit: `a1f71d4b` (`chore: retire orphan combat review lenses`).
- Collector policy closure: removed in the commit containing this record.
- Removed flags and payloads: Boss setup, forced potion opening, key-card counterfactual,
  key-card decision microscope, root-action role duel, Collector tactic lanes, and the optional
  turn-plan ladder row.
- Removed library policy: `collector_single_head_control` and `collector_boss_race` action priors,
  their action/frontier ranking, and their experiment-only tests.
- Replacement: none. The generic ladder, Frozen/Quality panels, line-lab, HP probe, key-card
  lifecycle, and boss evidence remain the supported saved-case review boundary.
- Recovery: `origin/backup/pre-cleanup-20260712` at
  `1ee108d0f53806f6b53c5169b74949b28e8648ce`.

### Legacy branch experiment product

- Removal delivery: the 2026-07-15 branch-experiment dependency-closure cleanup recorded here.
- Removed contracts: the `BranchExperimentV1` report and runner, shared-start profile runner,
  branch retention portfolio, branch boundary enumerator, branch trajectory and decision-path
  envelopes, branch-only event auto-policy, the unconstructable `InputSequence` command, the
  `event-select` retirement tombstone, and their private candidate/report schemas.
- Removed tests: 138 self-tests whose only production owner was the retired product.
- Replacement: none. `branch_tiny` and `branch_panel` remain the supported portfolio mainline;
  owner-audit and ordinary run-control retain their own automation and evidence contracts.
- Recovery: repository history and `origin/backup/pre-cleanup-20260712`.

### Legacy shop purchase bundle policy

- Removal delivery: the 2026-07-15 shop-bundle dependency-closure cleanup recorded here.
- Removed contracts: `ShopGoldOpportunity`, `ShopPurchaseBundleDecision`, bundle verdict and fact
  schemas, bundle filter/score passes, Maw Bank and future-shop string reason codes, and the
  ShopTiny route-to-bundle adapter.
- Removed tests: 24 tests that owned the retired bundle or asserted its string reasons and magic
  cross-candidate ordering weights.
- Surviving contracts: typed boss scaling/survival evidence, ordinary candidate admission,
  ShopTiny hard-checkpoint boss preview, and the separately tested `shop_policy_v1` compiler.
- Replacement boundary: whole-shop opportunity costs belong in a shop plan owner/compiler;
  `decision_pipeline` must not regain cross-candidate purchase policy.
- Recovery: repository history and `origin/backup/pre-cleanup-20260712`.

## Test Retention Contract

Keep game-mechanic and Java-parity tests, regressions for observed failures, serialized checkpoint
and artifact compatibility tests, and architecture/ownership tests by default. A future retirement
may remove a test only when the same delivery names the retired production behavior or a surviving
test that protects the exact contract. Test count and linked-binary size are observations, not
acceptance criteria.

## Next Cleanup Delivery

The legacy campaign and branch-experiment stacks are retired. Future cleanup may separately
address run-control consolidation, combat-review lens pruning, or disk/cache management; none is
authorized by this retirement.
