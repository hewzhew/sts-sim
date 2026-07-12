# Repository Cleanup Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Do not use subagents for this delivery.

**Goal:** Preserve the exact pre-cleanup repository state on an immutable remote branch, then establish an evidence-backed inventory of every supported Cargo surface and the safest first retirement candidate without deleting or refactoring code.

**Architecture:** The final clean documentation commit becomes the backup source of truth. A new architecture document records the verified backup hash, repository baselines, and one row for the library, build script, architecture test, and each Cargo binary. Classification is conservative: missing evidence produces `Unknown`, while `CandidateRetire` requires the complete proof contract from the approved design.

**Tech Stack:** Git/GitHub remote refs, Cargo metadata and verification, PowerShell, ripgrep, Markdown.

## Global Constraints

- Work in the stable checkout at `D:\rust\sts_simulator`; do not create a worktree and do not use subagents.
- Start from a clean worktree and make bounded local documentation commits.
- Never run `cargo clean`, delete build caches, or alter `.venv-ai`.
- Do not alter Rust source, Cargo manifests or locks, tests, run capsules, generated artifacts, or ignored files.
- Do not push `master`; this delivery is authorized to create only `origin/backup/pre-cleanup-20260712`.
- Never force-push or rewrite history. If the backup ref exists at a different hash, stop and ask the user for a new name.
- Treat file age, line count, reference count, and `v1`/`v2` naming as clues, never deletion proof.
- Classify ambiguous surfaces as `Unknown`; deletion is forbidden until a later reviewed retirement delivery.
- Keep game-mechanic, parity, real-regression, serialization, capsule, and architecture tests by default.
- This plan creates only `docs/architecture/supported-surfaces.md` during implementation.

---

### Task 1: Verify the Frozen Source State

**Files:**
- Read: `AGENTS.md`
- Read: `Cargo.toml`
- Read: `src/bin/README.md`
- Read: `tests/architecture_runtime_boundaries.rs`
- Change: none

- [ ] **Step 1: Reconfirm repository instructions and cleanliness**

Run:

```powershell
Get-Content AGENTS.md
git status --short
git branch --show-current
```

Expected: branch is `master`, `git status --short` is empty, and repository instructions still permit the approved backup flow. Stop if the worktree is dirty; do not stash or discard another owner's changes.

- [ ] **Step 2: Record the final local and public-remote state**

Run:

```powershell
$head = (git rev-parse HEAD).Trim()
$originMaster = (git ls-remote origin refs/heads/master | ForEach-Object { ($_ -split "\s+")[0] })
$aheadBehind = (git rev-list --left-right --count "${originMaster}...${head}").Trim()
"HEAD=$head"
"origin/master=$originMaster"
"origin/master...HEAD=$aheadBehind"
```

Expected: all three values are non-empty. Preserve the printed values for the inventory document. The local `HEAD` includes both the approved design and this implementation plan.

- [ ] **Step 3: Establish Cargo's actual target set**

Run:

```powershell
$metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
$package = $metadata.packages | Where-Object { $_.name -eq "sts_simulator" }
$package.targets |
  Sort-Object kind,name |
  ForEach-Object { "{0}`t{1}`t{2}" -f ($_.kind -join ","), $_.name, $_.src_path }
```

Expected: one library, one custom build target, one architecture integration-test target, and exactly these eight binaries:

```text
branch_campaign_driver
branch_panel
branch_tiny
combat_case_review
combat_search_v2_driver
decision_records
rl_dataset_export
run_play_driver
```

Stop and update the design before continuing if Cargo metadata returns a different supported-surface universe.

- [ ] **Step 4: Run the pre-cleanup verification checkpoint**

Run:

```powershell
cargo fmt --all -- --check
cargo test --lib
cargo check --bins
cargo test --test architecture_runtime_boundaries
git diff --check
git status --short
```

Expected: 2,811 library tests and all seven architecture tests pass, all binaries check, formatting and diff checks pass, and the worktree remains clean. If test counts have legitimately changed in the plan commit, record the actual passing counts rather than forcing the old number; any failure stops the backup operation for diagnosis.

---

### Task 2: Create and Verify the Immutable Remote Backup

**Files:**
- Change: none

- [ ] **Step 1: Query the exact remote backup ref before writing**

Run:

```powershell
$head = (git rev-parse HEAD).Trim()
$backupRef = "refs/heads/backup/pre-cleanup-20260712"
$existing = (git ls-remote origin $backupRef | ForEach-Object { ($_ -split "\s+")[0] })
"HEAD=$head"
"existing backup=$existing"
```

Expected: `$existing` is empty or exactly equals `$head`.

- [ ] **Step 2: Create the backup only when absent**

Run this guarded PowerShell block as one operation:

```powershell
$head = (git rev-parse HEAD).Trim()
$backupRef = "refs/heads/backup/pre-cleanup-20260712"
$existing = (git ls-remote origin $backupRef | ForEach-Object { ($_ -split "\s+")[0] })
if ($existing -and $existing -ne $head) {
    throw "Backup ref already exists at $existing, expected $head; refusing to overwrite"
}
if (-not $existing) {
    git push origin "HEAD:$backupRef"
    if ($LASTEXITCODE -ne 0) { throw "Backup push failed" }
}
```

Expected: either a new non-force branch is created or the operation is an idempotent no-op. Never add `--force`.

- [ ] **Step 3: Verify the remote object ID independently**

Run:

```powershell
$head = (git rev-parse HEAD).Trim()
$remoteBackup = (git ls-remote origin refs/heads/backup/pre-cleanup-20260712 |
  ForEach-Object { ($_ -split "\s+")[0] })
if ($remoteBackup -ne $head) {
    throw "Remote backup verification failed: local=$head remote=$remoteBackup"
}
"verified backup/pre-cleanup-20260712=$remoteBackup"
git status --short
```

Expected: local and remote hashes match and the worktree is still clean. This verified hash is the value recorded in the inventory; do not substitute `origin/master` or an earlier design hash.

---

### Task 3: Create the Supported-Surface Inventory and Classify Core Surfaces

**Files:**
- Create: `docs/architecture/supported-surfaces.md`
- Read: `Cargo.toml`
- Read: `build.rs`
- Read: `src/lib.rs`
- Read: `src/bin/README.md`
- Read: `src/bin/branch_tiny.rs`
- Read: `src/bin/branch_panel.rs`
- Read: `src/bin/combat_case_review.rs`
- Read: `tests/architecture_runtime_boundaries.rs`

- [ ] **Step 1: Recompute repository baselines without touching ignored data**

Run:

```powershell
$rustFiles = @(rg --files -g "*.rs")
$rustLines = ($rustFiles | ForEach-Object { (Get-Content -LiteralPath $_ | Measure-Object -Line).Lines } |
  Measure-Object -Sum).Sum
$trackedFiles = @(git ls-files)
$testMarkers = (rg -n "#\[test\]" -g "*.rs" | Measure-Object -Line).Lines
$cfgTestModules = (rg -n "#\[cfg\(test\)\]" -g "*.rs" | Measure-Object -Line).Lines
"tracked_files=$($trackedFiles.Count)"
"rust_files=$($rustFiles.Count)"
"rust_lines=$rustLines"
"test_markers=$testMarkers"
"cfg_test_modules=$cfgTestModules"
```

Also record tracked-byte size without staging anything:

```powershell
$trackedBytes = 0
git ls-files | ForEach-Object {
    if ($_ -and (Test-Path -LiteralPath $_ -PathType Leaf)) {
        $trackedBytes += (Get-Item -LiteralPath $_).Length
    }
}
"tracked_bytes=$trackedBytes"
```

Expected: the values are close to the approved design baseline. Differences caused by the two new Markdown files are expected and should be recorded exactly.

- [ ] **Step 2: Create the inventory structure**

Use `apply_patch` to create `docs/architecture/supported-surfaces.md` with these sections:

```markdown
# Supported Repository Surfaces

## Snapshot
## Repository Baseline
## Status Vocabulary
## Classification Method
## Supported Surface Matrix
## Surface Evidence
## First Retirement Recommendation
## Test Retention Contract
## Next Cleanup Delivery
```

The snapshot section records:

- branch `master`;
- backup ref `origin/backup/pre-cleanup-20260712`;
- the exact verified backup hash;
- `origin/master` hash and the recorded ahead/behind count;
- verification commands and actual pass counts;
- the explicit fact that ignored `artifacts/`, `target/`, and `.venv-ai/` are outside this Git backup.

The status vocabulary must define only `SupportedMainline`, `SupportedDiagnostic`, `CandidateRetire`, and `Unknown` using the approved design meanings.

- [ ] **Step 3: Add every required matrix row before making judgments**

The matrix must include these eleven rows and these columns:

```text
Cargo surface | Entry point | Owned purpose | Active callers | Written artifacts/schemas | Artifact consumers | Overlap/replacement | Evidence | Status | Next action
```

Required rows:

```text
sts_simulator library
custom build script
architecture_runtime_boundaries
branch_campaign_driver
branch_panel
branch_tiny
combat_case_review
combat_search_v2_driver
decision_records
rl_dataset_export
run_play_driver
```

Do not leave a cell blank. Use `None observed` only after a recorded search; use `Unknown` when evidence remains incomplete.

- [ ] **Step 4: Collect and write evidence for the library/build/architecture boundary**

Run:

```powershell
Get-Content Cargo.toml
Get-Content build.rs
Get-Content src\lib.rs
Get-Content tests\architecture_runtime_boundaries.rs
rg -n "architecture_runtime_boundaries|build\.rs|sts_simulator::" src tests tools docs --glob "!docs/superpowers/**"
git log -5 --oneline -- build.rs src/lib.rs tests/architecture_runtime_boundaries.rs
```

Document exact ownership and consumers. Classify the library and custom build script `SupportedMainline`; classify the architecture integration test `SupportedMainline` because it protects the declared production ownership boundary. Do not infer that architecture-test compilation makes every historical architecture choice permanent.

- [ ] **Step 5: Collect and write evidence for the three core operational tools**

Run:

```powershell
Get-Content src\bin\README.md
Get-Content src\bin\branch_tiny.rs
Get-Content src\bin\branch_panel.rs
Get-Content src\bin\combat_case_review.rs
rg -n "branch_tiny|branch_panel|combat_case_review" src tests tools docs --glob "!docs/superpowers/**"
rg -n "schema|json|jsonl|capsule|artifact|frontier|combat_case" src\bin\branch_tiny.rs src\bin\branch_panel.rs src\bin\combat_case_review.rs
git log -8 --oneline -- src/bin/branch_tiny.rs src/bin/branch_panel.rs src/bin/combat_case_review.rs
```

Record active command workflows separately from historical references. Initial classifications, subject to contradictory evidence, are:

- `branch_tiny`: `SupportedMainline`, the bounded owner-audit run entry point;
- `branch_panel`: `SupportedDiagnostic`, the durable panel inspect/continue/compare scheduler;
- `combat_case_review`: `SupportedDiagnostic`, the saved combat-case replay and review path.

- [ ] **Step 6: Commit the backup record and core inventory**

Run:

```powershell
git diff -- docs/architecture/supported-surfaces.md
git diff --check
git status --short
git add docs/architecture/supported-surfaces.md
git commit -m "docs: inventory core supported surfaces"
```

Expected: the commit contains only `docs/architecture/supported-surfaces.md` and preserves the verified pre-cleanup backup hash rather than the new post-backup documentation commit hash.

---

### Task 4: Audit Remaining Binary Tools and Their Artifact Chains

**Files:**
- Modify: `docs/architecture/supported-surfaces.md`
- Read: `src/bin/branch_campaign_driver/main.rs`
- Read: `src/bin/combat_search_v2_driver/main.rs`
- Read: `src/bin/decision_records.rs`
- Read: `src/bin/rl_dataset_export.rs`
- Read: `src/bin/run_play_driver/main.rs`
- Read: matching modules below `src/bin/branch_campaign_driver/` and `src/bin/combat_search_v2_driver/`

- [ ] **Step 1: Establish each tool's CLI and unique capability from source**

Run:

```powershell
rg -n "derive\(.*Parser|derive\(.*Subcommand|struct Cli|enum Command|fn main|about =|long_about" src/bin/branch_campaign_driver src/bin/combat_search_v2_driver src/bin/decision_records.rs src/bin/rl_dataset_export.rs src/bin/run_play_driver
rg -n "schema|schema_version|json|jsonl|capsule|dataset|decision|bookmark|capture|trace|frontier|panel" src/bin/branch_campaign_driver src/bin/combat_search_v2_driver src/bin/decision_records.rs src/bin/rl_dataset_export.rs src/bin/run_play_driver
```

For each target, write a one-sentence owned purpose and list every output path or schema literal visible at its boundary. Do not classify internal helper modules as separate Cargo surfaces.

- [ ] **Step 2: Separate active callers from historical documentation**

Run:

```powershell
$targets = @(
  "branch_campaign_driver",
  "combat_search_v2_driver",
  "decision_records",
  "rl_dataset_export",
  "run_play_driver"
)
foreach ($target in $targets) {
    "=== $target active references ==="
    rg -n $target src tests tools docs README.md Cargo.toml --glob "!docs/superpowers/**" --glob "!docs/architecture/supported-surfaces.md"
    "=== $target recent history ==="
    git log -8 --oneline -- "src/bin/$target*" "src/bin/$target/**"
}
```

The `===` labels are deliberate audit output, not a production script. In the document, distinguish:

- direct code/script invocation;
- current architecture or operator documentation;
- only historical specs/plans;
- no repository caller because the binary is a human CLI.

No-reference alone must not yield `CandidateRetire`.

- [ ] **Step 3: Trace artifacts from writer to consumer**

First extract likely schema and filename strings from each binary, then search each concrete string across active repository paths:

```powershell
rg -n '"[A-Za-z0-9_.-]+(_v[0-9]+|\.jsonl?|\.gz|\.csv|\.bin)"' src/bin/branch_campaign_driver src/bin/combat_search_v2_driver src/bin/decision_records.rs src/bin/rl_dataset_export.rs src/bin/run_play_driver
rg -n "decision_record_v0|path_observable_facts_v0|rlds|episode|bookmark|combat_case|frontier.json|final.json" src tests tools docs --glob "!docs/superpowers/**"
```

For every written artifact, record either named consumers or `No active consumer observed after <search description>`. If a schema is consumed outside this repository and that fact cannot be proven locally, record `Unknown external consumer` and keep the surface.

- [ ] **Step 4: Classify supported diagnostics conservatively**

Apply these starting hypotheses only when the collected evidence supports them:

- `combat_search_v2_driver`: `SupportedDiagnostic` for whole-combat scenario, capture, benchmark, and guidance investigation;
- `run_play_driver`: `SupportedDiagnostic` if its interactive trace/bookmark/capture workflow remains unique;
- `branch_campaign_driver`: `Unknown` unless current mainline ownership or a complete replacement can be demonstrated;
- `decision_records`: `Unknown` until both JSONL schemas and all consumers are resolved;
- `rl_dataset_export`: `Unknown` until the RL dataset consumer and replacement story are resolved.

An oversized or lightly referenced binary remains `Unknown`; do not turn cleanup discomfort into retirement evidence.

- [ ] **Step 5: Update every remaining matrix field**

Use `apply_patch` to fill callers, artifacts, consumers, overlap, evidence, status, and next action for all remaining binaries. Add a compact evidence subsection per binary so the matrix stays scannable while retaining exact searches and observations.

Run:

```powershell
git diff -- docs/architecture/supported-surfaces.md
git diff --check
```

Expected: all eleven rows are complete and no status exists outside the four-value vocabulary.

---

### Task 5: Select Exactly One Retirement Recommendation or Explicitly Defer

**Files:**
- Modify: `docs/architecture/supported-surfaces.md`

- [ ] **Step 1: Apply the complete retirement proof checklist**

Evaluate the most plausible small experimental surfaces first, especially `decision_records` and `rl_dataset_export`, against all six required facts:

1. no active source, script, current architecture document, or supported command invokes it;
2. every unique capability has a supported replacement or is explicitly no longer required;
3. every written schema has no active consumer or a complete migration path;
4. removal needs no empty compatibility shell;
5. related tests and documents have explicit keep/migrate/delete dispositions;
6. a later removal can name focused, library, remaining-bin, and architecture verification commands.

Record the evidence for each fact in `First Retirement Recommendation`. Reference count, age, size, and naming do not satisfy any fact by themselves.

- [ ] **Step 2: Make the single permitted foundation conclusion**

Choose exactly one of these document shapes:

```markdown
### Recommended: `<one target>`

Status: `CandidateRetire`

<proof for all six rules and scope for the future retirement specification>
```

or:

```markdown
### No Safe Candidate Yet

All unresolved candidates remain `Unknown`. <missing evidence per target and the next bounded audit>
```

Do not mark multiple targets `CandidateRetire`. A recommendation authorizes only a future design/specification, not deletion in this delivery.

- [ ] **Step 3: Record the first follow-on delivery**

In `Next Cleanup Delivery`, name either:

- the single candidate's retirement design, limited to that binary, its schemas, callers, tests, and docs; or
- the single smallest evidence-gathering audit needed to resolve an `Unknown` target.

Explicitly repeat that test cleanup, run-control consolidation, and disk/cache cleanup remain separate later deliveries.

- [ ] **Step 4: Commit the completed inventory**

Run:

```powershell
git diff --check
git status --short
git add docs/architecture/supported-surfaces.md
git commit -m "docs: complete supported surface inventory"
```

Expected: a documentation-only commit with exactly one recommendation or the explicit all-`Unknown` conclusion.

---

### Task 6: Verify Foundation Scope and Handoff

**Files:**
- Verify: `docs/architecture/supported-surfaces.md`
- Change: none

- [ ] **Step 1: Prove inventory coverage against Cargo metadata**

Run:

```powershell
$doc = Get-Content docs\architecture\supported-surfaces.md -Raw
$metadata = cargo metadata --no-deps --format-version 1 | ConvertFrom-Json
$package = $metadata.packages | Where-Object { $_.name -eq "sts_simulator" }
$missing = @()
foreach ($target in $package.targets) {
    if (-not $doc.Contains($target.name)) { $missing += $target.name }
}
if ($missing.Count -gt 0) { throw "Inventory misses Cargo targets: $($missing -join ', ')" }
$required = @(
  "Active callers", "Written artifacts/schemas", "Artifact consumers",
  "Overlap/replacement", "Evidence", "Status", "Next action",
  "SupportedMainline", "SupportedDiagnostic", "CandidateRetire", "Unknown"
)
foreach ($term in $required) {
    if (-not $doc.Contains($term)) { throw "Inventory misses required term: $term" }
}
"inventory coverage verified"
```

Expected: no missing target or required field vocabulary.

- [ ] **Step 2: Prove that implementation changed documentation only**

Record the implementation starting commit—the commit pushed to the backup ref—from the inventory, then run:

```powershell
$backupHead = (git ls-remote origin refs/heads/backup/pre-cleanup-20260712 |
  ForEach-Object { ($_ -split "\s+")[0] })
$changed = @(git diff --name-only $backupHead..HEAD)
$changed
if ($changed.Count -ne 1 -or $changed[0] -ne "docs/architecture/supported-surfaces.md") {
    throw "Foundation changed files outside the supported-surface inventory"
}
git diff --check $backupHead..HEAD
git status --short
```

Expected: the only post-backup path is `docs/architecture/supported-surfaces.md`, diff check passes, and the worktree is clean.

- [ ] **Step 3: Reconfirm the immutable snapshot after documentation commits**

Run:

```powershell
$recorded = (git ls-remote origin refs/heads/backup/pre-cleanup-20260712 |
  ForEach-Object { ($_ -split "\s+")[0] })
git show --no-patch --oneline $recorded
git status --short
git log -4 --oneline
```

Expected: the backup still points to the frozen pre-implementation commit, local `HEAD` contains the bounded inventory commits, and no force update or `master` push occurred.

- [ ] **Step 4: Deliver the evidence-backed handoff**

Report:

- verified backup branch and exact hash;
- library and architecture test pass counts plus binary-check result;
- actual baseline file/line/test counts;
- each `SupportedMainline` and `SupportedDiagnostic` surface;
- the single `CandidateRetire` and why, or why all candidates remain `Unknown`;
- the precise next cleanup delivery;
- confirmation that no Rust, test, Cargo, artifact, cache, environment, or public `master` state changed.

Do not describe any target as deleted, fixed, or simplified: this delivery establishes the safety boundary for those later changes.
