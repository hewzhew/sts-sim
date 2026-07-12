# Legacy Campaign Stack Retirement Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Do not use subagents for this delivery.

**Goal:** Remove the unused decision projection and the complete legacy campaign product stack while preserving the owner-audit mainline, interactive run-control, combat diagnostics, and supported RLDS/path-review tooling.

**Architecture:** Retirement proceeds in dependency order across three bounded commits: remove the independent exporter, remove the campaign CLI/launcher and current operator surface, then remove the unreachable campaign-only library closure. Compilation and focused replacement checks gate every layer; full library, Python, binary, and architecture verification gates completion.

**Tech Stack:** Rust 2021, Cargo auto-discovered binaries, serde artifact schemas, Python `unittest`, PowerShell, Git.

## Global Constraints

- Work in the stable checkout at `D:\rust\sts_simulator`; do not create a worktree and do not use subagents.
- Require a clean worktree before the first deletion and make exactly three bounded retirement commits.
- Verify `origin/backup/pre-cleanup-20260712` still points to `1ee108d0f53806f6b53c5169b74949b28e8648ce` before deleting anything.
- Use the command-local V2Ray proxy `http.proxy=http://127.0.0.1:10808` only when direct GitHub access fails; do not change repository, global Git, V2Ray, or ProxyBridge configuration.
- Never run `cargo clean`, delete `target`, alter `.venv-ai`, or delete ignored/local campaign artifacts.
- Never force-push, rewrite history, or push public `master` in this delivery.
- Use `apply_patch` for every tracked-file edit and deletion; do not use shell deletion commands.
- Do not add compatibility aliases, migration readers, deprecated shells, or owner-audit replacements for retired campaign APIs.
- Preserve `branch_tiny`, `branch_panel`, `branch_experiment*`, `run_control`, `run_play_driver`, owner-audit, combat search/review, `rl_dataset_export`, and `tools/path_review.py`.
- Preserve game-mechanic, Java-parity, retained serialization, and architecture tests.
- Historical documents under `docs/superpowers` remain unchanged except for this approved design and plan.
- Test-driven development does not apply to pure retirement: no replacement behavior is introduced. Compilation, focused replacement tests, structural absence checks, and full retained-contract suites are the gates.

---

### Task 1: Revalidate the Backup and Pre-Deletion Baseline

**Files:**
- Read: `AGENTS.md`
- Read: `docs/superpowers/specs/2026-07-12-legacy-campaign-stack-retirement-design.md`
- Read: `docs/architecture/supported-surfaces.md`
- Change: none

**Interfaces:**
- Consumes: approved product boundary and immutable backup ref.
- Produces: a verified clean implementation base and current passing test evidence.

- [ ] **Step 1: Confirm branch, cleanliness, and implementation base**

Run:

```powershell
Get-Content AGENTS.md
git branch --show-current
git status --short
$implementationBase = (git rev-parse HEAD).Trim()
"implementation_base=$implementationBase"
```

Expected: branch `master`, empty status, and `HEAD` containing this implementation plan. Stop if any unrelated change exists; do not stash or discard it.

- [ ] **Step 2: Revalidate the immutable remote backup without writing it**

Run:

```powershell
$proxy = 'http://127.0.0.1:10808'
$backup = (git -c "http.proxy=$proxy" ls-remote origin refs/heads/backup/pre-cleanup-20260712 |
  ForEach-Object { ($_ -split '\s+')[0] })
if ($backup -ne '1ee108d0f53806f6b53c5169b74949b28e8648ce') {
    throw "Backup ref mismatch: $backup"
}
"backup_verified=$backup"
```

Expected: exact hash match. If the local proxy is unavailable, retry direct `git ls-remote`; if neither works, stop before deletion.

- [ ] **Step 3: Recompute the exact retirement closure**

Run:

```powershell
$retiredRust = @('src/bin/decision_records.rs')
$retiredRust += @(rg --files src/bin/branch_campaign_driver -g '*.rs')
$retiredRust += @('src/eval/branch_campaign.rs')
$retiredRust += @(rg --files src/eval/branch_campaign -g '*.rs')
$retiredRust += @(
  'src/eval/campaign_journal.rs',
  'src/eval/branch_outcome_dataset_v1.rs',
  'src/eval/learning_dataset_v1.rs'
)
$retiredRust = @($retiredRust | Sort-Object -Unique)
$retiredLines = ($retiredRust | ForEach-Object { @(Get-Content -LiteralPath $_).Count } |
  Measure-Object -Sum).Sum
$retiredTests = ($retiredRust | ForEach-Object {
    (rg -n '#\[test\]' $_ | Measure-Object -Line).Lines
  } | Measure-Object -Sum).Sum
"retired_files=$($retiredRust.Count)"
"retired_physical_lines=$retiredLines"
"retired_test_markers=$retiredTests"
```

Expected: 49 Rust files, 45,799 physical lines, and 251 test markers. Stop and update the design if the source closure changed.

- [ ] **Step 4: Run the complete pre-deletion contract**

Run:

```powershell
cargo fmt --all -- --check
cargo test --lib --quiet
cargo check --bins
cargo test --test architecture_runtime_boundaries --quiet
python -m unittest discover -s tests -p 'test_*.py'
git diff --check
git status --short
```

Expected: 2,811 library tests, all seven architecture tests, all eight binaries, and the Python suite pass; worktree remains clean. Any failure is a pre-existing blocker and stops retirement.

---

### Task 2: Retire `decision_records`

**Files:**
- Delete: `src/bin/decision_records.rs`
- Modify: `README.md`
- Modify: `README.zh-CN.md`
- Modify: `src/bin/README.md`
- Modify: `docs/architecture/supported-surfaces.md`

**Interfaces:**
- Consumes: supported replacements `rl_dataset_export` and `tools/path_review.py`.
- Produces: seven Cargo binaries and no writer for `learning_decision_record_v0` or `path_observable_facts_v0`.

- [ ] **Step 1: Delete the implicit Cargo binary source**

Use `apply_patch` with:

```text
*** Delete File: D:/rust/sts_simulator/src/bin/decision_records.rs
```

Do not add an explicit `[[bin]]`, stub executable, schema alias, or migration reader.

- [ ] **Step 2: Remove the maintained binary-list rows**

Use `apply_patch` to delete exactly these rows:

```markdown
| `decision_records` | decision-record inspection utility |
```

from `README.md`, this row:

```markdown
| `decision_records` | decision record 检查工具 |
```

from `README.zh-CN.md`, and this row:

```markdown
| `decision_records` | Decision-record inspection utility; reports typed decision artifacts without owning policy. |
```

from `src/bin/README.md`.

- [ ] **Step 3: Remove the live inventory row and obsolete recommendation prose**

In `docs/architecture/supported-surfaces.md`, use `apply_patch` to remove:

- the `decision_records` row from `Supported Surface Matrix`;
- the `### decision_records` subsection from `Surface Evidence`;
- the complete `First Retirement Recommendation` section that recommends `decision_records`;
- the old `Next Cleanup Delivery` paragraph that asks for a future decision-record retirement design.

Replace the next-delivery section with this exact transitional content:

```markdown
## Next Cleanup Delivery

Legacy campaign stack retirement is in progress under the approved layered plan. The completed
decision-record layer will be recorded in retirement history with its commit ID during the next
layer. Run-control consolidation, combat-review pruning, and disk/cache cleanup remain separate.
```

Do not add a `Retired` value to the live status vocabulary.

- [ ] **Step 4: Prove the supported replacements still work**

Run:

```powershell
cargo test --bin rl_dataset_export --quiet
python tests\test_path_review.py
cargo check --bins
```

Expected: all commands pass. The Cargo check compiles seven remaining binaries.

- [ ] **Step 5: Prove structural removal**

Run:

```powershell
$metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
$bins = @($metadata.packages |
  Where-Object { $_.name -eq 'sts_simulator' } |
  ForEach-Object { $_.targets } |
  Where-Object { $_.kind -contains 'bin' } |
  Select-Object -ExpandProperty name |
  Sort-Object)
$bins
if ($bins.Count -ne 7 -or $bins -contains 'decision_records') {
    throw "Unexpected binary set after decision_records retirement"
}
$activeRefs = @(rg -n 'decision_records|learning_decision_record_v0|path_observable_facts_v0' `
  README.md README.zh-CN.md src tests tools docs/RUNBOOK.md docs/ARCHITECTURE.md `
  docs/architecture --glob '!docs/architecture/supported-surfaces.md')
if ($activeRefs.Count) { $activeRefs; throw 'Active decision-record references remain' }
git diff --check
```

Expected: seven binaries, no retired target, no active source/operator reference, and a clean diff check. Historical `docs/superpowers` references are intentionally outside the search.

- [ ] **Step 6: Review and commit Layer 1**

Run:

```powershell
git status --short
git diff --stat
git diff --check
git add src/bin/decision_records.rs README.md README.zh-CN.md src/bin/README.md docs/architecture/supported-surfaces.md
git diff --cached --check
git commit -m "chore: retire decision records exporter"
$decisionRetirementCommit = (git rev-parse HEAD).Trim()
"decision_retirement_commit=$decisionRetirementCommit"
```

Expected: one bounded commit containing only the exporter deletion and its maintained documentation.

---

### Task 3: Retire the Legacy Campaign Application and Launcher

**Files:**
- Delete: `src/bin/branch_campaign_driver/main.rs`
- Delete: `src/bin/branch_campaign_driver/campaign_app.rs`
- Delete: `src/bin/branch_campaign_driver/campaign_artifact_source_info.rs`
- Delete: `src/bin/branch_campaign_driver/campaign_artifact_store.rs`
- Delete: `src/bin/branch_campaign_driver/campaign_artifacts.rs`
- Delete: `src/bin/branch_campaign_driver/campaign_milestones.rs`
- Delete: `src/bin/branch_campaign_driver/campaign_run.rs`
- Delete: `src/bin/branch_campaign_driver/checkpoint_evidence.rs`
- Delete: `src/bin/branch_campaign_driver/checkpoint_inspection.rs`
- Delete: `src/bin/branch_campaign_driver/checkpoint_shop_evidence.rs`
- Delete: `src/bin/branch_campaign_driver/cli_args.rs`
- Delete: `src/bin/branch_campaign_driver/combat_timeline_inspection.rs`
- Delete: `src/bin/branch_campaign_driver/command_inputs.rs`
- Delete: `src/bin/branch_campaign_driver/coverage_gap_milestone_summary.rs`
- Delete: `src/bin/branch_campaign_driver/decision_observations.rs`
- Delete: `src/bin/branch_campaign_driver/driver_command.rs`
- Delete: `src/bin/branch_campaign_driver/inspect_summary.rs`
- Delete: `src/bin/branch_campaign_driver/journal_inspection.rs`
- Delete: `src/bin/branch_campaign_driver/outcome_dataset.rs`
- Delete: `src/bin/branch_campaign_driver/shop_challenge.rs`
- Delete: `tools/campaign.ps1`
- Modify: `README.md`
- Modify: `README.zh-CN.md`
- Modify: `src/bin/README.md`
- Modify: `docs/RUNBOOK.md`
- Modify: `tools/README.md`
- Modify: `docs/architecture/supported-surfaces.md`

**Interfaces:**
- Consumes: Layer 1 commit ID and the user's explicit decision to drop campaign artifact compatibility.
- Produces: six Cargo binaries, no maintained campaign command, and temporarily orphaned campaign-only library modules for Layer 3.

- [ ] **Step 1: Capture the completed Layer 1 commit**

Run:

```powershell
$decisionRetirementCommit = (git rev-parse HEAD).Trim()
git show --no-patch --oneline $decisionRetirementCommit
git status --short
```

Expected: `chore: retire decision records exporter` and an empty status.

- [ ] **Step 2: Delete all 20 campaign binary files and the launcher**

Use one `apply_patch` operation containing `*** Delete File` entries for every path listed in this task's Files section under `src/bin/branch_campaign_driver/` and for `tools/campaign.ps1`.

Afterward run:

```powershell
if (Test-Path src/bin/branch_campaign_driver) {
    $remaining = @(Get-ChildItem src/bin/branch_campaign_driver -Force)
    if ($remaining.Count) { throw "Campaign binary files remain: $($remaining.Name -join ', ')" }
}
if (Test-Path tools/campaign.ps1) { throw 'campaign.ps1 still exists' }
```

Expected: no tracked campaign binary source and no launcher. An empty directory, if left by the filesystem, is irrelevant to Git.

- [ ] **Step 3: Remove campaign rows and ownership prose from binary maps**

Use `apply_patch` to delete these exact table rows:

```markdown
| `branch_campaign_driver` | older Rust campaign application surface for campaign artifacts and continuation experiments |
| `branch_campaign_driver` | 较旧但仍保留的 Rust campaign application surface，用于 campaign artifact 和 continuation 实验 |
| `branch_campaign_driver` | Rust-owned campaign application: run, inspect, dataset, continuation, and self-check commands. |
```

Remove this complete ownership bullet from `src/bin/README.md`:

```markdown
- `branch_campaign_driver` subcommands are the campaign application surface.
  Top-level compatibility flags may parse, but new tooling should call explicit
  subcommands.
```

Do not alter the six surviving binary descriptions.

- [ ] **Step 4: Remove campaign commands from the runbook**

In `docs/RUNBOOK.md`, use `apply_patch` to remove the entire `## Campaign Launcher` section through the paragraph ending with `artifact schema semantics.`

In the `## Verification` PowerShell block, delete:

```powershell
cargo build --profile fast-run --bin branch_campaign_driver
```

Keep the run-play and combat-search build commands.

- [ ] **Step 5: Remove the campaign launcher boundary from the tools map**

In `tools/README.md`, delete the `campaign.ps1` table row and the complete `## Campaign Launcher Boundary` section through its final architecture-boundary sentence. Preserve panel, path-review, combat, and ML tool groups.

- [ ] **Step 6: Update live supported surfaces and record Layer 1 history**

In `docs/architecture/supported-surfaces.md`, use `apply_patch` to remove:

- the `branch_campaign_driver` row from the live matrix;
- the `### branch_campaign_driver` evidence subsection.

Insert a `## Retirement History` section immediately before `## Test Retention Contract`. Its first
subsection is `### decision_records`. The first bullet begins `- Removal commit:`, contains the
literal 40-character hash printed in Step 1, and ends
`(chore: retire decision records exporter).` Add these remaining bullets verbatim:

```markdown
- Removed contracts: `learning_decision_record_v0` and `path_observable_facts_v0`.
- Replacements: `rl_dataset_export` for per-step behavior-policy data and
  `tools/path_review.py` for human path inspection.
- Recovery: `origin/backup/pre-cleanup-20260712` at
  `1ee108d0f53806f6b53c5169b74949b28e8648ce`.
```

Verify the committed Markdown contains the literal Layer 1 hash and the exact heading/bullets.

- [ ] **Step 7: Prove the application surface is gone while the library still compiles**

Run:

```powershell
$metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
$bins = @($metadata.packages |
  Where-Object { $_.name -eq 'sts_simulator' } |
  ForEach-Object { $_.targets } |
  Where-Object { $_.kind -contains 'bin' } |
  Select-Object -ExpandProperty name |
  Sort-Object)
$expected = @('branch_panel','branch_tiny','combat_case_review','combat_search_v2_driver','rl_dataset_export','run_play_driver')
if (($bins -join ',') -ne ($expected -join ',')) {
    throw "Unexpected binaries: $($bins -join ',')"
}
cargo check --bins
cargo check --lib
```

Expected: exactly six binaries and successful compilation. Campaign library modules still exist at this checkpoint by design.

- [ ] **Step 8: Prove current commands no longer advertise the campaign application**

Run:

```powershell
$activeRefs = @(rg -n 'branch_campaign_driver|campaign\.ps1' `
  README.md README.zh-CN.md src/bin/README.md docs/RUNBOOK.md tools/README.md `
  src tests tools --glob '!src/eval/branch_campaign/**' --glob '!src/eval/branch_campaign.rs')
if ($activeRefs.Count) { $activeRefs; throw 'Active campaign application references remain' }
git diff --check
```

Expected: no maintained command or source caller. Historical `docs/superpowers` files are outside the search.

- [ ] **Step 9: Review and commit Layer 2**

Run:

```powershell
git status --short
git diff --stat
git diff --check
git add src/bin/branch_campaign_driver tools/campaign.ps1 README.md README.zh-CN.md src/bin/README.md docs/RUNBOOK.md tools/README.md docs/architecture/supported-surfaces.md
git diff --cached --check
git commit -m "chore: retire legacy campaign application"
$campaignApplicationCommit = (git rev-parse HEAD).Trim()
"campaign_application_commit=$campaignApplicationCommit"
```

Expected: a bounded application/launcher/documentation deletion commit. The campaign-only library still exists for the next layer.

---

### Task 4: Remove the Campaign-Only Library Closure

**Files:**
- Delete: `src/eval/branch_campaign.rs`
- Delete: `src/eval/branch_campaign/assessment.rs`
- Delete: `src/eval/branch_campaign/branch_display.rs`
- Delete: `src/eval/branch_campaign/discard_trace.rs`
- Delete: `src/eval/branch_campaign/intervention.rs`
- Delete: `src/eval/branch_campaign/lineage.rs`
- Delete: `src/eval/branch_campaign/model.rs`
- Delete: `src/eval/branch_campaign/parent_batch.rs`
- Delete: `src/eval/branch_campaign/performance.rs`
- Delete: `src/eval/branch_campaign/progress.rs`
- Delete: `src/eval/branch_campaign/report_render.rs`
- Delete: `src/eval/branch_campaign/retry.rs`
- Delete: `src/eval/branch_campaign/route_evidence.rs`
- Delete: `src/eval/branch_campaign/run_domain.rs`
- Delete: `src/eval/branch_campaign/scheduler.rs`
- Delete: `src/eval/branch_campaign/selection_key.rs`
- Delete: `src/eval/branch_campaign/state_graph.rs`
- Delete: `src/eval/branch_campaign/strategic_signals.rs`
- Delete: `src/eval/branch_campaign/summary.rs`
- Delete: `src/eval/branch_campaign/tests.rs`
- Delete: `src/eval/branch_campaign/tests/intervention_tests.rs`
- Delete: `src/eval/branch_campaign/tests/report_tests.rs`
- Delete: `src/eval/branch_campaign/tests/resume_tests.rs`
- Delete: `src/eval/branch_campaign/tests/retry_tests.rs`
- Delete: `src/eval/branch_campaign/tests/state_store_tests.rs`
- Delete: `src/eval/campaign_journal.rs`
- Delete: `src/eval/branch_outcome_dataset_v1.rs`
- Delete: `src/eval/learning_dataset_v1.rs`
- Modify: `src/eval/mod.rs`
- Modify: `docs/architecture/supported-surfaces.md`

**Interfaces:**
- Consumes: no remaining production callers after Task 3.
- Produces: a six-binary repository without campaign types, schemas, journals, datasets, or tests.

- [ ] **Step 1: Capture and verify the Layer 2 commit**

Run:

```powershell
$campaignApplicationCommit = (git rev-parse HEAD).Trim()
git show --no-patch --oneline $campaignApplicationCommit
git status --short
```

Expected: `chore: retire legacy campaign application` and empty status.

- [ ] **Step 2: List the exact library files before deletion**

Run:

```powershell
$campaignLibrary = @('src/eval/branch_campaign.rs')
$campaignLibrary += @(rg --files src/eval/branch_campaign -g '*.rs')
$campaignLibrary += @(
  'src/eval/campaign_journal.rs',
  'src/eval/branch_outcome_dataset_v1.rs',
  'src/eval/learning_dataset_v1.rs'
)
$campaignLibrary = @($campaignLibrary | Sort-Object -Unique)
$campaignLibrary
if ($campaignLibrary.Count -ne 28) {
    throw "Expected 28 campaign library files, found $($campaignLibrary.Count)"
}
```

Expected: 28 files: the `branch_campaign` root and 24 subordinate files plus the three standalone dataset/journal files.

- [ ] **Step 3: Delete the 28 library files**

Use `apply_patch` with one `*** Delete File` entry for each path printed in Step 2. Do not use `Remove-Item`, and do not include any `branch_experiment*`, `run_control`, owner-audit, combat, or RLDS path.

- [ ] **Step 4: Remove only the four module declarations**

In `src/eval/mod.rs`, use `apply_patch` to delete exactly:

```rust
pub mod branch_campaign;
pub mod branch_outcome_dataset_v1;
pub mod campaign_journal;
pub mod learning_dataset_v1;
```

The retained beginning must still contain:

```rust
pub mod artifact;
pub mod branch_experiment;
pub(crate) mod branch_experiment_boundary;
pub mod branch_experiment_retention;
pub mod branch_experiment_search_options;
pub mod branch_experiment_trajectory;
```

- [ ] **Step 5: Add the campaign retirement record and final baseline**

In `docs/architecture/supported-surfaces.md`, append a
`### branch_campaign_driver and campaign-only library` subsection below the decision-record
retirement record. Its first bullet begins `- Application removal commit:`, contains the literal
40-character Layer 2 hash printed in Step 1, and ends
`(chore: retire legacy campaign application).` Add these remaining bullets verbatim:

```markdown
- Library closure: removed in the commit containing this record.
- Removed contracts: `BranchCampaignV1`, `BranchCampaignCheckpointV2`, campaign journal,
  campaign artifact pointers/manifests, targeted continuation, and campaign learning datasets.
- Replacement: none; the product boundary was explicitly retired. `branch_tiny` and
  `branch_panel` remain the supported mainline rather than compatibility readers.
- Recovery: `origin/backup/pre-cleanup-20260712` at
  `1ee108d0f53806f6b53c5169b74949b28e8648ce`.
```

Replace the transitional `Next Cleanup Delivery` text with:

```markdown
## Next Cleanup Delivery

The legacy campaign stack is retired. Future cleanup may separately address run-control
consolidation, combat-review lens pruning, or disk/cache management; none is authorized by this
retirement.
```

After recomputing counts in Step 8, add a `## Post-Retirement Baseline` table containing exact values for tracked files/bytes, Rust files, physical Rust lines, `#[test]` markers, `#[cfg(test)]` markers, and Cargo binaries. Keep the original frozen baseline unchanged above it.

- [ ] **Step 6: Prove no retained source depends on the removed modules**

Run:

```powershell
$sourceRefs = @(rg -n 'branch_campaign|BranchCampaign|campaign_journal|CampaignJournal|branch_outcome_dataset_v1|learning_dataset_v1|LearningDecisionOutcomeSampleV1' `
  src tests tools --glob '!src/bin/README.md')
if ($sourceRefs.Count) { $sourceRefs; throw 'Campaign library references remain' }
```

Expected: no source, test, or tool references. Documentation history is intentionally not searched.

- [ ] **Step 7: Run the retained Rust contracts before documenting final counts**

Run:

```powershell
cargo fmt --all -- --check
cargo test --lib --quiet
cargo check --bins
cargo test --test architecture_runtime_boundaries --quiet
```

Expected: all retained library tests and all seven architecture tests pass; all six binaries compile. Record the actual remaining library-test count from Cargo output.

- [ ] **Step 8: Recompute and write the post-retirement baseline**

Run:

```powershell
$rustFiles = @(rg --files -g '*.rs')
$physicalRustLines = ($rustFiles | ForEach-Object { @(Get-Content -LiteralPath $_).Count } |
  Measure-Object -Sum).Sum
$trackedFiles = @(git ls-files)
$trackedBytes = 0
$trackedFiles | ForEach-Object {
    if ($_ -and (Test-Path -LiteralPath $_ -PathType Leaf)) {
        $trackedBytes += (Get-Item -LiteralPath $_).Length
    }
}
$testMarkers = (rg -n '#\[test\]' -g '*.rs' | Measure-Object -Line).Lines
$cfgTestMarkers = (rg -n '#\[cfg\(test\)\]' -g '*.rs' | Measure-Object -Line).Lines
$metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
$binaryCount = @($metadata.packages |
  Where-Object { $_.name -eq 'sts_simulator' } |
  ForEach-Object { $_.targets } |
  Where-Object { $_.kind -contains 'bin' }).Count
"tracked_files=$($trackedFiles.Count)"
"tracked_bytes=$trackedBytes"
"rust_files=$($rustFiles.Count)"
"physical_rust_lines=$physicalRustLines"
"test_markers=$testMarkers"
"cfg_test_markers=$cfgTestMarkers"
"cargo_binaries=$binaryCount"
```

Expected structural reductions from the frozen source baseline are 49 Rust files, 45,799 physical Rust lines, 251 test markers, and two binaries. The exact tracked-file and byte values include this design and plan and are recorded as printed.

- [ ] **Step 9: Review and commit Layer 3**

Run:

```powershell
git status --short
git diff --stat
git diff --check
git add src/eval/branch_campaign.rs src/eval/branch_campaign src/eval/campaign_journal.rs src/eval/branch_outcome_dataset_v1.rs src/eval/learning_dataset_v1.rs src/eval/mod.rs docs/architecture/supported-surfaces.md
git diff --cached --check
git commit -m "chore: remove legacy campaign library stack"
```

Expected: the third and final retirement commit contains only the campaign-only library closure, four module declarations, and final inventory evidence.

---

### Task 5: Final Scope, Test, and Recovery Verification

**Files:**
- Verify: all surviving source and documentation
- Change: none

**Interfaces:**
- Consumes: three clean retirement commits.
- Produces: evidence that the reduced repository preserves every declared retained surface.

- [ ] **Step 1: Verify exactly three retirement commits and a clean tree**

Run:

```powershell
git log -4 --oneline
git status --short
```

Expected: the newest three commit subjects, in order, are
`chore: remove legacy campaign library stack`,
`chore: retire legacy campaign application`, and
`chore: retire decision records exporter`; the fourth is this implementation-plan commit. Status
must be empty.

- [ ] **Step 2: Verify the six-target Cargo surface**

Run:

```powershell
$metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
$bins = @($metadata.packages |
  Where-Object { $_.name -eq 'sts_simulator' } |
  ForEach-Object { $_.targets } |
  Where-Object { $_.kind -contains 'bin' } |
  Select-Object -ExpandProperty name |
  Sort-Object)
$expected = @('branch_panel','branch_tiny','combat_case_review','combat_search_v2_driver','rl_dataset_export','run_play_driver')
$bins
if (($bins -join ',') -ne ($expected -join ',')) {
    throw "Unexpected final binary set: $($bins -join ',')"
}
```

Expected: exactly the six declared retained binaries.

- [ ] **Step 3: Verify every explicit keep-boundary path exists**

Run:

```powershell
$kept = @(
  'src/bin/branch_tiny.rs',
  'src/bin/branch_panel.rs',
  'src/bin/combat_case_review.rs',
  'src/bin/combat_search_v2_driver/main.rs',
  'src/bin/rl_dataset_export.rs',
  'src/bin/run_play_driver/main.rs',
  'src/eval/branch_experiment.rs',
  'src/eval/branch_experiment_boundary.rs',
  'src/eval/run_control',
  'src/runtime/branch',
  'src/ai/combat_search_v2',
  'tools/path_review.py'
)
$missing = @($kept | Where-Object { -not (Test-Path -LiteralPath $_) })
if ($missing.Count) { throw "Keep-boundary paths missing: $($missing -join ', ')" }
"keep_boundary=$($kept.Count)/$($kept.Count)"
```

Expected: every retained path exists.

- [ ] **Step 4: Run the complete final verification from the reduced tree**

Run:

```powershell
cargo fmt --all -- --check
cargo test --lib --quiet
cargo check --bins
cargo test --test architecture_runtime_boundaries --quiet
cargo test --bin rl_dataset_export --quiet
python -m unittest discover -s tests -p 'test_*.py'
$implementationBase = (git rev-parse HEAD~3).Trim()
git diff --check "$implementationBase..HEAD"
git diff --name-status "$implementationBase..HEAD"
git status --short
```

Expected: every command passes, all seven architecture tests pass, six binaries compile, and the worktree is clean. Report the actual retained library and Python test counts.

- [ ] **Step 5: Reverify remote recovery and public-master immutability**

Run:

```powershell
$proxy = 'http://127.0.0.1:10808'
$backup = (git -c "http.proxy=$proxy" ls-remote origin refs/heads/backup/pre-cleanup-20260712 |
  ForEach-Object { ($_ -split '\s+')[0] })
$publicMaster = (git -c "http.proxy=$proxy" ls-remote origin refs/heads/master |
  ForEach-Object { ($_ -split '\s+')[0] })
if ($backup -ne '1ee108d0f53806f6b53c5169b74949b28e8648ce') {
    throw "Backup changed: $backup"
}
if ($publicMaster -ne '5643238ad85af6f11833452ab78c15a9df975a42') {
    throw "Public master changed unexpectedly: $publicMaster"
}
"backup=$backup"
"origin_master=$publicMaster"
```

Expected: recovery ref and public master are unchanged; no push occurs.

- [ ] **Step 6: Deliver the cleanup handoff**

Report:

- all three local retirement commit hashes;
- before/after Rust files, physical lines, test markers, actual tests, and Cargo binaries;
- exact surviving mainline and diagnostic binaries;
- final full verification results;
- recovery branch and hash;
- confirmation that no mechanics, content, owner-audit, run-control, combat, RLDS, ignored artifact, cache, environment, or public remote state changed;
- the next optional cleanup area, without beginning it automatically.
