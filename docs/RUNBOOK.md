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
cargo run -p sts_simulator_control --bin branch_tiny -- --seed 1552225673 --ascension 0 --max-branches 1 --wall-ms 60000
```

Run a small panel:

```powershell
cargo run -p sts_simulator_control --bin branch_panel -- panel smoke --seeds 1552225671 1552225672 1552225673 1552225674 1552225675 --capsule-root tools/artifacts/panels/current --max-branches 1 --slice-ms 60000
```

Use the panel to classify blockers. Do not treat one seed as a strategy verdict.

For bounded continuation, use `drain`:

```powershell
cargo run -p sts_simulator_control --bin branch_panel -- panel drain --seeds 1552225671 1552225672 --capsule-root tools/artifacts/panels/current --max-slices 3 --slice-ms 60000
```

The retired `tools/gap_panel.py` compatibility wrapper has been removed. Use
`branch_panel` directly for all panel runs.

## Continue A Capsule

When a capsule soft-stops with a frontier, continue from the capsule instead of
rerunning from Neow:

```powershell
cargo run -p sts_simulator_control --bin branch_tiny -- --continue-capsule <capsule-dir>
```

Continuation may inherit relevant run-contract values such as `wall_ms` from
the capsule manifest. Override only when the investigation needs a different
contract.

## Combat Case Review

For saved combat gaps, start from the case:

```powershell
cargo run -p sts_simulator_control --bin combat_case_review -- --case <case.json> --ladder
```

Review output is diagnostic. It does not mutate runner policy and does not
prove a deck is good or bad by itself.

## Combat Search Driver

Use `combat_search_v2_driver` for fixed combat starts, captures, and benchmark
suites:

```powershell
cargo run -p sts_simulator_control --release --bin combat_search_v2_driver -- --start-spec <path>
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
cargo run -p sts_simulator_control --bin combat_search_v2_driver -- --lab-spec fixtures/combat_lab/seed006_reptomancer_8x2.lab.json --lab-output artifacts/runs/combat-lab-seed006-pilot --lab-samples 8
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
cargo run -p sts_simulator_control --release --bin combat_search_v2_driver -- --threat-panel-spec fixtures/campfire_threat_panel/seed006_pre_transient_reconstructed.panel.json --threat-panel-output artifacts/runs/campfire-threat-panel-seed006-pilot --threat-panel-samples 1
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
cargo run -p sts_simulator_control --bin rl_dataset_export -- --input artifacts/runs/example/trace.json --out artifacts/runs/example/planner-dataset.json --planner-coverage-out artifacts/runs/example/planner-coverage.json
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
cargo build -p sts_simulator_control --release --bin combat_search_v2_driver
git diff --check
```

On `x86_64-pc-windows-msvc`, the repository uses rustup's bundled `rust-lld`
through `.cargo/config.toml`. Keep that override: it remains useful, but the
primary rebuild fix is now the workspace boundary rather than a linker flag.

### Compilation Boundaries

The workspace has two production compilation units:

- `sts_simulator` owns the stable simulator/domain and lower policy layers;
- `sts_simulator_control` owns combat search, evaluation, run-control,
  `runtime::branch`, and all supported binaries.

The root package deliberately keeps `autobins = false`, `autotests = false`,
and remains the sole default member. Therefore bare `cargo test --lib` tests
only the core package; it is not the complete workspace check. Use
`cargo test-core <filter>` for core tests and `cargo test-control <filter>` for
search/evaluation/run-control tests. Use both aliases plus `cargo architecture`
before handoff.

On the migration baseline, the old 2,808-test / 54.76 MiB harness became a
1,889-test / 17.86 MiB core harness and a 919-test / 46.80 MiB control harness.
A real `combat_search_v2` source edit rebuilt the control harness in 8.23
seconds while the core package stayed fresh; two unchanged filtered commands
then completed in 0.40 and 0.37 seconds. These numbers are evidence from one
Windows machine, not performance assertions for CI.

Use `fast-run` for iterative optimized runs, build a binary once for repeated
panel cells, and reserve `release` or `release-final` for final confirmation.
Further package splits should be justified by a new measured invalidation
boundary; do not replace this boundary with test features or many integration
test executables.

For documentation-only changes:

```powershell
git diff --check
```

Run targeted tests only when the changed surface has a stable structural
contract worth protecting. Do not add or preserve tests for retired probes,
temporary reports, or prose-only behavior.
