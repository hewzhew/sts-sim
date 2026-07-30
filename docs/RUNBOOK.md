# Runbook

This file keeps current local commands in one place. It is command-oriented;
architecture rules belong in [ARCHITECTURE.md](ARCHITECTURE.md).

## Branch Tiny And Branch Panels

`branch_tiny` is the lightweight owner-audit runner. It writes run capsules
with `summary.json`, `path.json`, optional `frontier.json`, optional
`terminal.json`, and combat cases when combat search blocks.

Run one seed:

```powershell
cd D:\rust\sts_simulator
cargo run -p sts_oracle_tools --bin branch_tiny -- --seed 1552225673 --ascension 0 --max-branches 1 --wall-ms 60000
```

Run a small panel:

```powershell
cargo run -p sts_oracle_tools --bin branch_panel -- panel smoke --seeds 1552225671 1552225672 1552225673 1552225674 1552225675 --capsule-root tools/artifacts/panels/current --max-branches 1 --slice-ms 60000
```

Use the panel to classify blockers. Do not treat one seed as a strategy verdict.

For bounded continuation, use `drain`:

```powershell
cargo run -p sts_oracle_tools --bin branch_panel -- panel drain --seeds 1552225671 1552225672 --capsule-root tools/artifacts/panels/current --max-slices 3 --slice-ms 60000
```

The retired `tools/gap_panel.py` compatibility wrapper has been removed. Use
`branch_panel` directly for all panel runs.

## Continue A Capsule

When a capsule soft-stops with a frontier, continue from the capsule instead of
rerunning from Neow:

```powershell
cargo run -p sts_oracle_tools --bin branch_tiny -- --continue-capsule <capsule-dir>
```

Continuation may inherit relevant run-contract values such as `wall_ms` from
the capsule manifest. Override only when the investigation needs a different
contract.

## Combat Case Review

For saved combat gaps, start from the case:

```powershell
cargo run -p sts_oracle_tools --bin combat_case_review -- --case <case.json> --ladder
```

Review output is diagnostic. It does not mutate runner policy and does not
prove a deck is good or bad by itself.

### Potion Expenditure Audit

Compare no-potion, each initial potion, and optionally small potion
combinations from one unchanged exact combat root:

```powershell
.\ol.cmd combat-case-potion-expenditure-audit `
  --case <case.json> `
  --max-combination-size 2 `
  --wall-ms-per-lane 10000
```

Every lane receives the same independent search allowance. The command filters
explicit use/discard inputs by exact slot without deleting potions from the
root, then replay-attributes actual consumption by potion UUID. This preserves
potion-sensitive state and detects passive Fairy Potion use. A passive
expenditure outside a lane's allowed slots marks that witness non-compliant
instead of silently treating it as a no-potion result.

The report exposes final HP, final turn, action count, exact potion identities,
an optional `--survival-reserve-hp`, and a Pareto frontier. A missing witness is
budget-unknown unless the lane reports `frontier_exhausted`. Continuation value
such as forced-rest avoidance, future elite plans, potion-slot overflow, and
encounter-specific preservation remains a run-level decision and is listed as
unobserved rather than invented into one combat score.

### Fresh Potion Continuation Cases

Use a fresh capsule path and two bounded phases when the investigation needs a
current owner-generated combat case with
`PotionRunContinuationContextV1`. The first phase lets the run naturally
acquire cards, relics, and potions. The second phase deliberately lowers search
allowance so the next unresolved fight becomes a diagnostic case:

```powershell
$capsule = ".oracle-lab/collections/potion-v5/<fresh-id>"
if (Test-Path -LiteralPath $capsule) {
  throw "choose a fresh capsule path: $capsule"
}

cargo run --quiet -p sts_oracle_tools --bin branch_tiny -- `
  --seed <seed> --ascension 0 --objective first-terminal `
  --generations 14 --max-branches 1 --auto-ops 512 `
  --search-nodes 100 --search-ms 50 `
  --rescue-search-nodes 300 --rescue-search-ms 100 `
  --boss-search-nodes 300 --boss-search-ms 100 `
  --wall-ms 10000 --run-capsule $capsule

if (Test-Path -LiteralPath "$capsule/frontier.json") {
  cargo run --quiet -p sts_oracle_tools --bin branch_tiny -- `
    --continue-capsule $capsule --continue-slices 1 `
    --generations 10 --max-branches 1 `
    --search-nodes 1 --search-ms 1 `
    --rescue-search-nodes 1 --rescue-search-ms 1 `
    --boss-search-nodes 1 --boss-search-ms 1 `
    --wall-ms 10000
}
```

The 1-node phase is a capture mechanism, not a claim that the combat is hard
or unwinnable. Before auditing, require every saved search summary to contain
one identical `before_combat_search` context. Current captures should also
contain one identical `PotionContinuationPressureV1`; its absence is expected
only for legacy cases. The V5 audit must report `validated_exact_root` with no
mismatches:

```powershell
$case = Get-ChildItem -LiteralPath "$capsule/combat_cases" `
  -Filter *.json -File | Select-Object -First 1

cargo oracle-lab combat-case-potion-expenditure-audit `
  --case $case.FullName `
  --max-combination-size 1 --survival-reserve-hp 30 `
  --max-nodes 5000 --max-selections 20000 `
  --wall-ms-per-lane 500 `
  > .oracle-lab/reports/<fresh-id>-potion-v5.json `
  2> .oracle-lab/reports/<fresh-id>-potion-v5.log
```

Keep complete JSON and build output under `.oracle-lab`; report only aggregate
lane results. Do not upgrade a legacy case by guessing missing route, Boss, or
supply facts, and do not treat a budget-limited missing witness as potion
evidence.

## Combat Search Driver

Use `combat_search_v2_driver` for fixed combat starts, captures, and benchmark
suites:

```powershell
cargo run -p sts_oracle_tools --release --bin combat_search_v2_driver -- --start-spec <path>
```

Common investigation switches include:

```text
--combat-snapshot <path>
--benchmark-spec <path>
--validate-only
--potion-policy all --max-potions-used <n>
--max-hp-loss <n|off>
```

If combat search reports unresolved, it only failed to find an executable
complete win under the current contract. It did not prove the fight unwinnable.

### Combat Laboratory V1

The Combat Laboratory is an offline mode of `combat_search_v2_driver`, not a
new binary or a live run-control component. Run the maintained seed006-derived
Reptomancer `8 x 2` pilot with:

```powershell
cargo run -p sts_oracle_tools --bin combat_search_v2_driver -- --lab-spec fixtures/combat_lab/seed006_reptomancer_8x2.lab.json --lab-output artifacts/runs/combat-lab-seed006-pilot --lab-samples 8
```

Rerun the same command and output directory to resume without repeating journaled
cells. To extend the deterministic schedule, increase only `--lab-samples` (for
example, from 8 to 16 or 32). A smaller requested target does not delete existing
evidence. Resume rejects changes to the scenario, schedule, profiles, common
budget, schema, or source identity.

Each laboratory directory contains four contract/evidence files:

- `manifest.json`: the immutable resolved experiment and source provenance;
- `cells.jsonl`: the append-only raw evidence journal and evidence authority;
- `checkpoint.json`: a rebuildable resume accelerator derived from the journal;
- `summary.json`: a reproducible aggregate derived from the manifest and journal.

`resolved_win` and `resolved_loss` are exact-replayed outcomes. A deadline, node
cap, or missing complete replay is `coverage_limited`, not a proven loss;
infrastructure errors are separate again. Read outcome rates together with the
reported coverage denominators.

V1 runs sequentially in one process: it compiles each shuffle sample once,
clones that position across the two profiles, gives both profiles the same
resource limits, records the row, and then advances. It does not invoke Cargo or
relink per cell. Results are descriptive offline evidence only; they do not
automatically update combat policy, route planning, card acquisition, or any
other live decision.

The pilot preserves the selected seed006 deck, resources, encounter, and a fresh
laboratory base seed. It is explicitly `seed006_derived`: it does not infer the
campaign RNG history that had already been consumed before the original combat.
Both profiles are `exact_state_oracle` searches that may inspect hidden state,
not human-visible-information policies.

### Campfire Threat Panel V1

The Campfire Threat Panel is the wider, offline Campfire microscope. It expands
every alignable exact Campfire candidate against every encounter in a declared
public pool, with matched analysis RNG and shuffle samples. It never reads the
live run's hidden encounter queue and never updates live Campfire policy.
The contract rejects wall-clock budgets: comparisons use deterministic node
limits, and cells with identical exact-state fingerprints reuse one explicitly
recorded search result rather than measuring scheduler noise twice.

Run the reconstructed seed006 pre-Transient pilot with:

```powershell
cargo run -p sts_oracle_tools --release --bin combat_search_v2_driver -- --threat-panel-spec fixtures/campfire_threat_panel/seed006_pre_transient_reconstructed.panel.json --threat-panel-output artifacts/runs/campfire-threat-panel-seed006-pilot --threat-panel-samples 1
```

The fixture is explicitly reconstructed from recorded public deck/resources;
it is not claimed to restore the campaign's consumed hidden RNG or route map.
The manifest records this public context, the resolved encounter pool, all
alignable subjects, typed candidate gaps, source identity, and fixed search
contract. `cells.jsonl` is again the append-only evidence authority. Repeating
the identical command resumes completed cells; increasing only the sample
target extends the fixed shuffle schedule.

Read the two lenses separately:

- `actual_consequence` keeps each candidate's real post-Campfire HP/resources;
- `root_hp_capability` resets only current HP to the public root, isolating what
  the resulting deck can mechanically do at equal starting HP.

Summaries remain stratified by encounter and subject. Pair deltas and direction
reversals are evidence that a choice changes with the threat, not a hidden
global Campfire score. Coverage-limited rows remain usable exact-replayed best
candidates, but they are not proofs that search found the optimum.

Historical artifacts remain readable and valid when a profile implementation is
later removed. Rerunning that historical profile requires the Git commit recorded
in its manifest; the current tree must not silently substitute a newer profile.

## Planner Capture Export

The retired interactive driver no longer produces live `SessionTraceV1`
captures. Existing schema-v6-through-v15 traces remain readable; schema v16
keeps legacy exact-frontier evidence distinct from current work-item counts
while capture moves to the
atomic run-job journal. A rebuildable dataset and coverage report can still be
exported from an existing typed trace under `artifacts/runs` with:

```powershell
cargo run -p sts_oracle_tools --bin rl_dataset_export -- --input artifacts/runs/example/trace.json --out artifacts/runs/example/planner-dataset.json --planner-coverage-out artifacts/runs/example/planner-coverage.json
```

The coverage report measures representation and linkage only. It does not rank
decision sites, declare policy quality, or promote the recorded behavior to a
correct-action label.

## Verification

For code changes:

```powershell
cargo fmt --all -- --check
cargo check-workspace
cargo test-core
cargo test-control
cargo architecture
cargo check --workspace --release --all-targets
cargo build -p sts_oracle_tools --release --bin combat_search_v2_driver
git diff --check
```

On `x86_64-pc-windows-msvc`, the repository uses rustup's bundled `rust-lld`
through `.cargo/config.toml`. Keep that override: it remains useful, but the
primary rebuild fix is now the workspace boundary rather than a linker flag.

### Compilation Boundaries

The workspace has several deliberate production compilation units:

- `sts_simulator` owns the stable simulator/domain and lower policy layers;
- `sts_combat_strategy`, `sts_combat_planner`, and `sts_combat_legacy` isolate
  typed combat knowledge, the current exact planner, and the retained legacy
  capability surface;
- `sts_combat_knowledge` owns the shared tactical priors consumed by both
  run-control and exact combat tools;
- `sts_combat_contract` is the narrow, replay-verified Boss-regression runner;
  it intentionally excludes the run explorer, routes, shops, and continuations;
- `sts_oracle_eval` owns optimized evaluation, exact-search orchestration, and
  run-control;
- `sts_oracle_runtime` owns branch execution, persistence, and resident service
  orchestration;
- `sts_oracle_lab` owns heavyweight offline and resident command hosts;
- `oracle_lab_client` owns the lightweight repeated-command surface;
- `sts_oracle_tools` is the thin Cargo host for maintained command adapters and
  their cross-layer integration contracts; it has no library facade or policy
  implementation.

The root package deliberately keeps `autobins = false`, `autotests = false`,
and remains the sole default member. Therefore bare `cargo test --lib` tests
only the core package; it is not the complete workspace check. Use
`cargo test-core <filter>` for core tests and `cargo test-control <filter>` for
search/evaluation/run-control tests. Use both aliases plus `cargo architecture`
before handoff.

For a symbolized native CPU trace of the narrow combat contract, run
`tools/perf/profile_combat_cpu.ps1`. Analyze an existing trace without another
recording or elevation prompt with:

```powershell
.\tools\perf\summarize_combat_cpu.ps1 -Trace .profiles\combat-cpu-<id>.etl
```

The summary normalizes samples across only the recorded `combat_contract`
processes, so unrelated machine activity cannot distort the reported hotspot
percentages. The portable Microsoft-signed PerfView executable remains an
ignored local analysis dependency under `.profiles\tools`.

Each capture publishes its matching Rust PDB into an ignored, GUID-keyed local
symbol cache before WPR starts. This keeps old traces symbolizable after later
builds and avoids PerfView's unreliable adjacent-PDB matcher. The summarizer
rejects reports with less than 95% resolved executable-exclusive samples rather
than emitting a plausible-looking hotspot table made from unknown symbols.

For the same canonical workload without WPR or elevation, use:

```powershell
.\tools\perf\benchmark_combat_contract.ps1
```

It builds the narrow profiling target once, performs warmup, reports batched
process-wall timing, and rejects any iteration whose exact counters or witness
differ. Both tools source `combat_contract_workload.ps1`, so their workload
cannot silently drift apart.

Both tools also write a content-addressed build receipt beside the profiling
executable. `-SkipBuild` hashes the combat runner's complete Rust/Cargo source
scope plus the executable and PDB, and refuses to run when any identity differs
from that receipt. Rebuild without `-SkipBuild` after a source change; do not
interpret a previously built experimental binary as the current checkout.

Before changing state ownership or another cross-cutting hot path, run the
small identity-locked combat panel:

```powershell
.\tools\perf\benchmark_combat_panel.ps1 -SkipBuild
```

It interleaves Hexaghost, Champ, Bronze Automaton, and Collector samples under
the same 20,000-work contract. The cases cover a light state, a long combat, a
large/expensive state, and a replay-verified witness. Timing is observational;
deterministic counters and witness identity are checked against
`combat_performance_panel.json` before a result is accepted.

To diagnose transition ownership costs without putting clocks on the
production stepper, run the same locked panel with sparse sampling:

```powershell
.\tools\perf\benchmark_combat_panel.ps1 -SkipBuild -ProfileTransitionCloneCost
```

The diagnostic samples one of every 16 applied transitions and reports
`engine_clone_ns`, `combat_clone_ns`, and `transition_execution_ns`. Its wall
time is instrumented; use the ordinary panel for performance acceptance.

Oracle work uses one canonical `release` artifact. Build-owning commands use
`cargo oracle-lab` or `cargo ol`; repeated offline calls use `.\ol.cmd`, and
resident work uses `cargo ol-live` or `.\ol-live.cmd`. The retired `fast-run`
profile and target directory are not valid operational entrypoints.
Use `.\ol-contract.cmd` for the maintained exact-combat contracts. It compiles
only simulator/core combat, the planner, shared tactical knowledge, and its
thin runner; a planner edit therefore does not invalidate or link the full
oracle run explorer.
The heavy `oracle_lab` and `oracle_lab_service` targets require the internal
`canonical-oracle-artifacts` feature. This is intentional: an ad-hoc
`cargo build -p sts_oracle_lab --bin oracle_lab` is rejected during Cargo
target selection, before it can spend tens of seconds linking an artifact that
the runtime profile guard would later refuse to execute.

For a production-owner run, create and start one exact F0 workspace. The thin
client sends one typed `run` transaction; the resident runtime owns the
owner/search/accept loop in memory and saves once at the terminal or explicit
stop boundary:

```powershell
cargo oracle-lab new --seed 20260713009 --ascension 0 --workspace .oracle-lab/cases/seed009.workspace.json
.\ol-live.cmd start --session seed009 --workspace .oracle-lab/cases/seed009.workspace.json
.\ol-live.cmd live --session seed009 run --export-continuation .oracle-lab/cases/seed009.victory.continuation.json
```

`live run` uses the maintained 5/15/30-second hallway/elite/boss budgets. It
accepts only replay-verified combat incumbents. If a combat has no witness
within its budget, the command saves the resident workspace and stops at that
exact combat with `combat_budget_unknown_without_witness`; it does not restart,
switch algorithms, or select a historical donor. At terminal victory it
replays the complete committed journal from the canonical seed state inside
the resident service, exports the exact continuation when requested, and
returns `victory_verified`.

Resident services autosave their workspace in place after every mutation and
on shutdown. `live start` therefore accepts mutable workspaces only below the
ignored `.oracle-lab/` state root. Committed witness fixtures and historical
files below `target/` remain valid offline replay inputs, but resident startup
rejects them before building or launching the service so verification cannot
silently rewrite golden evidence.

On Windows, the lightweight client parents a newly launched resident host to
the interactive shell process. This lets one-shot Cargo, PowerShell, and tool
callers return their first `started` response without remaining attached to the
long-lived service process tree.

For a consecutive multi-seed panel, do not launch resident services from a
PowerShell loop. Use the single-process panel owner so each finished seed drops
all search memory before the next seed, writes its report immediately, and
retains a full workspace only for a real stop:

```powershell
cargo oracle-lab seed-panel `
  --seed-start 20260713006 `
  --ascension 0 `
  --output-dir .oracle-lab/panels/a0-10
```

The safe daily default is 10 consecutive seeds, 30 seconds of total run work
per seed, and at most 10 minutes for one process invocation. A deliberately
larger `--count` remains resumable but still stops at that invocation cap.
Pass `--invocation-wall-ms 0` only for an explicitly monitored uninterrupted
run, or raise the cap to a known bound.

Victories keep a compact report and exact replayable continuation under
`reports/` and `witnesses/`. Budget or correctness stops keep a resumable
workspace under `incomplete/`. Re-running the same command skips verified
victories and stable exhausted stops; only wall/boundary interruptions resume
automatically. Use `--retry-stopped` to re-enter a deterministic stop after
deliberately changing its allowance, or `--force` to start the selected seeds
from F0 again. `panel.summary.json` records the source commit, dirty state,
budgets, per-seed outcome, remaining count, separate run/total timings, the
workspace prepare, report write, workspace persistence, teardown, and residual
phases, including checkpoint materialization versus atomic JSON write, and
artifact paths after every completed seed. A top-level
`interrupted / invocation_wall_budget` result is a successful durable slice,
not a seed failure.

Full `combat-case` reports also include a read-only `storage` census. It
separates live generator work from retained slot, exact-state index, scheduling
heap, and graph-edge capacities, including the subset owned by already
finished turn generators. It also distinguishes live from stale scheduling
entries and reports the active-generator live-work distribution, so lazy queue
garbage can be measured without changing service order. These values diagnose
ownership and allocation shape only; no search budget, ordering, or witness
contract reads them.

Resident endpoints, immutable service images, and new local case artifacts
live below the ignored `.oracle-lab/` state root. They are deliberately outside
Cargo's `target/` tree so `cargo clean` cannot erase a live session or research
checkpoint. Historical files below `target/oracle-cases/` remain readable but
should not receive new output.

`.\ol-live.cmd` always lets Cargo validate the lightweight client before
execution; it never directly runs a possibly stale client binary. `live start`
also compares an already-running endpoint's immutable service image with the
current canonical host. A matching image is reused. A stale or legacy image is
saved and shut down, then the same workspace is relaunched automatically with
status `restarted_stale_runtime`. Exact run state and charged historical work
survive this transition; an in-memory tactical frontier is deliberately
restarted because it belongs to the old executable image.

The command hosts in `sts_oracle_lab` are physically separate from the branch
runtime, and optimized evaluation/run-control is a third compilation unit.
On one Windows machine, an actual public run-control metadata edit rebuilt the
former combined O2 runtime and host in 60.24 seconds: 53.1 of the runtime's
57.8 seconds were code generation. After the measured split, the same class of
warm edit rebuilt `sts_oracle_eval` in 8.37 seconds, the branch runtime in 2.55
seconds, and the host in 1.95 seconds, with 10.76 seconds total wall time. A
matched three-seed production panel retained identical boundaries, HP, combat
counts, and owner-decision counts; total wall time changed from 10.606 to
10.720 seconds, within persistence noise. These local numbers verify the
invalidation boundary; they are not CI performance thresholds.

The first build after introducing a new package still has to create its
incremental cache and is not expected to match a warm edit. Use `release-final`
only when the fully optimized deployment artifact is specifically required.
Further package splits require another measured source-invalidation boundary.

The exact-combat contract has a still narrower measured boundary. On the same
Windows checkout, an actual source edit in `sts_combat_planner` followed by the
managed-T4 contract rebuilt and ran in 3.85 seconds; the exact search itself
took 0.38 seconds. The former heavyweight route spent about 20.8 seconds
end-to-end after the same class of planner invalidation. These figures are
local observations, not CI thresholds; the enforced architectural fact is
that `sts_combat_contract` does not depend on `sts_oracle_runtime`.

Splitting one Rust source file into modules improves ownership and review
scope, but it does not create a new compilation unit: every module of one
crate is still parsed and code-generated together. Claim a build-time
improvement only after a measured crate-boundary change; do not infer one from
smaller files.

Do not configure a project-wide `sccache` wrapper for the canonical iterative
build. A bounded Windows experiment restored the same empty target path in
10.96 seconds after a 74.97-second cache fill (51 hits, 3 misses), but two
different target paths produced zero hits even with normalized base
directories. More importantly, the canonical `release` profile deliberately
uses incremental compilation, which `sccache` cannot cache. Reconsider it for
non-incremental CI or same-path disaster recovery, not routine local edits.

For documentation-only changes:

```powershell
git diff --check
```

Run targeted tests only when the changed surface has a stable structural
contract worth protecting. Do not add or preserve tests for retired probes,
temporary reports, or prose-only behavior.
