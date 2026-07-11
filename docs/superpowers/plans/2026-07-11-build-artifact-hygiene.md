# Build and Artifact Hygiene Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Reclaim stale Cargo cache safely, keep durable run evidence intact, and make future test builds use one checkout path with substantially smaller debug artifacts.

**Architecture:** Repository guidance owns the single-checkout and artifact-location rules, while Cargo's test profile owns debug-output size. Cache cleanup is a one-time guarded filesystem operation that accepts only known direct children of `target` with Cargo markers; verification then rebuilds one canonical cache and records cold and incremental behavior.

**Tech Stack:** Rust 2021/Cargo, PowerShell 7, Git, existing route and architecture test suites.

## Global Constraints

- Do not delete or rewrite any test in this phase.
- Do not change simulator, route, combat, reward, shop, owner, or run-control behavior.
- Do not move existing capsules whose evidence contains absolute paths.
- Do not add a linker wrapper or a second test command that creates a competing cache identity.
- Do not configure `rust-lld` in this phase.
- Do not split crates or introduce Cargo features in this phase.
- Do not run `cargo clean` or recursively delete unknown `target` children.
- Do not delete source, design history, datasets, or run evidence merely because it is old.
- Use the existing checkout at `D:\rust\sts_simulator`; do not create a worktree.

---

### Task 1: Record the stable-checkout and artifact rules

**Files:**
- Create: `AGENTS.md`
- Modify: `.gitignore`
- Create: `docs/superpowers/plans/2026-07-11-build-artifact-hygiene.md`

**Interfaces:**
- Produces: repository-scoped instructions consumed by future agents.
- Produces: ignored `artifacts/runs` namespace for new durable run output.

- [ ] **Step 1: Run the rule-contract check and verify RED**

Run:

```powershell
$agents = Test-Path AGENTS.md
$ignored = git check-ignore -q artifacts/runs/probe.json
if ($agents -and $LASTEXITCODE -eq 0) { exit 0 }
Write-Error "stable workflow rules or artifacts/runs ignore rule are missing"
exit 1
```

Expected: exit 1 because `AGENTS.md` does not exist and `artifacts/runs` is not ignored.

- [ ] **Step 2: Add the repository instructions**

Create `AGENTS.md` with exactly this policy content:

```markdown
# Repository Workflow

- Work in the stable checkout at `D:\rust\sts_simulator` on a local feature branch. Do not create Git worktrees for this repository; switching source roots duplicates the large Cargo test build.
- Start changes only from a clean Git status and make frequent local commits.
- Never run `cargo clean` in this repository while legacy run capsules remain under `target`.
- Write new durable run capsules and experiment evidence under `artifacts/runs`, never under a Cargo profile directory.
- Delete build caches only when the resolved target is a known direct child of `target` and Cargo marker files prove it is a cache.
- Use focused tests during red/green work. Run the full library and `architecture_runtime_boundaries` suites at completion checkpoints.
```

Append this generated-output rule to `.gitignore` beside `tools/artifacts/`:

```gitignore
/artifacts/
```

- [ ] **Step 3: Re-run the rule-contract check and verify GREEN**

Run the Step 1 command again.

Expected: exit 0; `AGENTS.md` exists and `artifacts/runs/probe.json` is ignored.

- [ ] **Step 4: Verify the change is documentation/configuration only**

Run:

```powershell
git diff --check
git status --short
```

Expected: only `AGENTS.md`, `.gitignore`, and this implementation plan are listed; no file under `src` is modified.

- [ ] **Step 5: Commit the workflow boundary**

```powershell
git add -- AGENTS.md .gitignore docs/superpowers/plans/2026-07-11-build-artifact-hygiene.md
git commit -m "chore: define stable build artifact workflow"
```

---

### Task 2: Shrink default test debug output

**Files:**
- Modify: `Cargo.toml`

**Interfaces:**
- Produces: Cargo's standard `test` profile with `debug = 0`.
- Preserves: debug assertions, overflow checks, optimization level, and the normal `cargo test` command.

- [ ] **Step 1: Capture the failing artifact-size baseline**

Run:

```powershell
$pdb = Get-ChildItem target\debug\deps -Filter 'sts_simulator-*.pdb' -File |
    Sort-Object LastWriteTime -Descending |
    Select-Object -First 1
if (-not $pdb) { throw "library test PDB is missing" }
$mib = [math]::Round($pdb.Length / 1MB, 1)
Write-Output "DEFAULT_TEST_PDB_MIB=$mib"
if ($mib -lt 50) { exit 0 }
Write-Error "default test PDB exceeds the 50 MiB target"
exit 1
```

Expected: exit 1 with the existing default PDB around 316 MiB.

- [ ] **Step 2: Add the minimal Cargo profile configuration**

Append to `Cargo.toml` before the custom run profiles:

```toml
[profile.test]
debug = 0
```

Do not add linker, rustflags, strip, optimization, panic, or codegen-unit settings.

- [ ] **Step 3: Validate Cargo metadata without rebuilding**

Run:

```powershell
cargo metadata --no-deps --format-version 1 | Out-Null
cargo fmt -- --check
git diff --check
```

Expected: all commands exit 0. The artifact-size check remains red until Task 3 removes the old cache and Task 4 rebuilds it.

- [ ] **Step 4: Commit the profile boundary**

```powershell
git add -- Cargo.toml
git commit -m "build: reduce test debug artifacts"
```

---

### Task 3: Remove only proven Cargo cache directories

**Files:**
- Delete cache contents only under the exact guarded paths listed below.
- Preserve every other `target` child, especially `target/route-reliability-seed-20260711004` and `target/seed-fix-diagnosis-20260711004`.

**Interfaces:**
- Consumes: direct-child path and Cargo-marker evidence.
- Produces: reclaimed disk space while leaving durable artifacts untouched.

- [ ] **Step 1: Verify the preserved-capsule precondition**

Run:

```powershell
$preserved = @(
    'target\route-reliability-seed-20260711004\manifest.json',
    'target\route-reliability-seed-20260711004\summary.json',
    'target\seed-fix-diagnosis-20260711004\manifest.json',
    'target\seed-fix-diagnosis-20260711004\summary.json'
)
$missing = @($preserved | Where-Object { -not (Test-Path -LiteralPath $_) })
if ($missing.Count -gt 0) { throw "preserved capsule files missing: $($missing -join ', ')" }
$preserved | ForEach-Object { Get-Content -LiteralPath $_ | ConvertFrom-Json -Depth 100 | Out-Null }
Write-Output 'preserved capsule JSON is readable'
```

Expected: exit 0.

- [ ] **Step 2: Enumerate and prove every deletion candidate**

Run this read-only guard:

```powershell
$repo = (Resolve-Path '.').Path
$target = (Resolve-Path 'target').Path
$candidates = @(
    'target\debug',
    'target\fast-run',
    'target\release',
    'target\dev-opt',
    'target\release-final'
) + @(
    Get-ChildItem target -Directory -Filter 'codex-verify-run-play*' |
        ForEach-Object { $_.FullName }
)

$approved = @()
foreach ($candidate in $candidates) {
    if (-not (Test-Path -LiteralPath $candidate)) { continue }
    $resolved = (Resolve-Path -LiteralPath $candidate).Path
    if ((Split-Path -Parent $resolved) -ne $target) {
        throw "cache candidate is not a direct target child: $resolved"
    }
    $profileCache =
        (Test-Path (Join-Path $resolved '.fingerprint')) -and
        (Test-Path (Join-Path $resolved 'deps'))
    $cargoRoot =
        (Test-Path (Join-Path $resolved '.rustc_info.json')) -and
        (Test-Path (Join-Path $resolved 'CACHEDIR.TAG'))
    if (-not ($profileCache -or $cargoRoot)) {
        throw "Cargo cache markers missing: $resolved"
    }
    $measure = Get-ChildItem $resolved -Recurse -File -ErrorAction Stop |
        Measure-Object Length -Sum
    $approved += [pscustomobject]@{
        Path = $resolved
        GiB = [math]::Round($measure.Sum / 1GB, 2)
    }
}
$approved | Sort-Object GiB -Descending | Format-Table -AutoSize
$total = [math]::Round(($approved | Measure-Object GiB -Sum).Sum, 2)
Write-Output "APPROVED_CACHE_GIB=$total"
if ($total -lt 35) { throw "approved cache is below the expected 35 GiB floor" }
```

Expected: every candidate is a direct child of `target`, every candidate has one accepted Cargo marker pair, and approved size is at least 35 GiB.

- [ ] **Step 3: Delete exactly the approved paths in one PowerShell process**

Run this complete guarded deletion command:

```powershell
$target = (Resolve-Path 'target').Path
$candidates = @(
    'target\debug',
    'target\fast-run',
    'target\release',
    'target\dev-opt',
    'target\release-final'
) + @(
    Get-ChildItem target -Directory -Filter 'codex-verify-run-play*' |
        ForEach-Object { $_.FullName }
)

$approved = @()
foreach ($candidate in $candidates) {
    if (-not (Test-Path -LiteralPath $candidate)) { continue }
    $resolved = (Resolve-Path -LiteralPath $candidate).Path
    if ((Split-Path -Parent $resolved) -ne $target) {
        throw "cache candidate is not a direct target child: $resolved"
    }
    $profileCache =
        (Test-Path (Join-Path $resolved '.fingerprint')) -and
        (Test-Path (Join-Path $resolved 'deps'))
    $cargoRoot =
        (Test-Path (Join-Path $resolved '.rustc_info.json')) -and
        (Test-Path (Join-Path $resolved 'CACHEDIR.TAG'))
    if (-not ($profileCache -or $cargoRoot)) {
        throw "Cargo cache markers missing: $resolved"
    }
    $measure = Get-ChildItem $resolved -Recurse -File -ErrorAction Stop |
        Measure-Object Length -Sum
    $approved += [pscustomobject]@{
        Path = $resolved
        GiB = [math]::Round($measure.Sum / 1GB, 2)
    }
}

foreach ($entry in $approved) {
    Write-Output "removing Cargo cache: $($entry.Path) ($($entry.GiB) GiB)"
    Remove-Item -LiteralPath $entry.Path -Recurse -Force -ErrorAction Stop
}
```

The `$approved` objects used for deletion are produced and consumed in the same
PowerShell process. Do not reconstruct deletion paths after validation.

Expected: only the approved directories are removed.

- [ ] **Step 4: Re-verify capsules and reclaimed space**

Run the Step 1 capsule check again, then:

```powershell
$remaining = @(
    'target\debug',
    'target\fast-run',
    'target\release',
    'target\dev-opt',
    'target\release-final'
) + @(Get-ChildItem target -Directory -Filter 'codex-verify-run-play*' -ErrorAction SilentlyContinue)
if (@($remaining | Where-Object { Test-Path -LiteralPath $_ }).Count -gt 0) {
    throw 'one or more approved Cargo cache directories remain'
}
Write-Output 'approved Cargo caches removed; preserved capsules remain readable'
```

Expected: exit 0.

---

### Task 4: Rebuild one canonical cache and verify the project

**Files:**
- No additional source or configuration files.
- Timestamp-only diagnostic touch: `src/ai/route_planner_v1/render.rs`.

**Interfaces:**
- Consumes: the canonical checkout and `[profile.test] debug = 0`.
- Produces: focused/full test evidence, artifact-size evidence, and a single canonical Cargo cache.

- [ ] **Step 1: Measure the cold focused build and run route tests**

Run:

```powershell
$sw = [Diagnostics.Stopwatch]::StartNew()
cargo test --lib ai::route_window_facts::tests -- --nocapture
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo test --lib ai::route_planner_v1::tests -- --nocapture
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
$sw.Stop()
Write-Output ("COLD_FOCUSED_SECONDS={0:N1}" -f $sw.Elapsed.TotalSeconds)
```

Expected: 5 route-window tests and 25 route-planner tests pass.

- [ ] **Step 2: Verify the rebuilt PDB-size target**

Run the Task 2 artifact-size command again.

Expected: exit 0 with the newest library test PDB below 50 MiB.

- [ ] **Step 3: Measure same-checkout incremental cost**

Run:

```powershell
$file = 'src\ai\route_planner_v1\render.rs'
(Get-Item $file).LastWriteTime = Get-Date
$sw = [Diagnostics.Stopwatch]::StartNew()
cargo test --lib ai::route_planner_v1::tests --no-run
$code = $LASTEXITCODE
$sw.Stop()
Write-Output ("INCREMENTAL_FOCUSED_SECONDS={0:N1}" -f $sw.Elapsed.TotalSeconds)
if ($code -ne 0) { exit $code }
if ($sw.Elapsed.TotalSeconds -gt 25) {
    throw 'incremental focused build exceeded 25 seconds'
}
if ((git status --porcelain).Length -ne 0) {
    throw 'timestamp-only benchmark left tracked content changes'
}
```

Expected: build completes within 25 seconds and Git remains clean.

- [ ] **Step 4: Run full verification**

Run:

```powershell
cargo fmt -- --check
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo test --lib --quiet
if ($LASTEXITCODE -ne 0) { exit $LASTEXITCODE }
cargo test --test architecture_runtime_boundaries --quiet
```

Expected: formatting passes, all 2,685 library tests pass, and all 7 architecture tests pass.

- [ ] **Step 5: Verify final repository and artifact state**

Run:

```powershell
git status --short
git log -4 --oneline
Get-Content target\route-reliability-seed-20260711004\manifest.json |
    ConvertFrom-Json -Depth 100 |
    Select-Object -ExpandProperty source_identity |
    Format-List
```

Expected: Git is clean; the manifest remains readable and identifies clean commit `e19c4557` as the route behavior baseline.

No full seed rerun is required because no gameplay source file changed.
