# Oracle Operations

This runbook covers the canonical offline and resident oracle surfaces. Mutable
workspaces, endpoints, reports, and new research cases belong below the ignored
`.oracle-lab/` state root.

## Choose The Command Host

- Use `cargo oracle-lab` or `cargo ol` when Cargo should own the build.
- Use `.\ol.cmd` for repeated offline calls; its canonical artifact guard
  rejects stale source identity.
- Use `cargo ol-live` or `.\ol-live.cmd` for resident sessions.
- Use `.\ol-contract.cmd` for the narrow replay-verified combat contract.

Do not directly build or execute the heavy host outside these surfaces. The
canonical `release` profile is the iterative artifact; use `release-final`
only for a deliberately requested deployment build.

## Diagnose A Suspicious Late Stop

Replay the committed run history before changing combat policy or starting a
distribution panel:

```powershell
.\ol.cmd diagnose-run-witness `
  --workspace .oracle-lab/cases/<run>.workspace.json `
  --node <node> --case .oracle-lab/cases/<combat>.case.json `
  --max-pivots 5 `
  --export-first-divergence-continuation `
    .oracle-lab/cases/<fresh-id>.continuation.json `
  > .oracle-lab/reports/<fresh-id>.diagnosis.json
```

This command performs exact journal replay without search. Its compact output
reports the largest combat HP losses, lowest post-combat HP boundaries,
recoveries, potion identity, nearby typed run choices, HP lineage since the
latest full-HP reset, and the first current-owner ranking divergence. It also
reports the first unclassified divergence after preserving but stepping past
a narrow typed `same_potion_kind_discard` identity difference. Use `--details`
only when the complete combat timeline and all divergences are needed.

`--case` is optional when no cross-artifact claim is being made. When present,
it is a fail-closed origin check: the case needs exact production context and
must match one combat timeline entry by both exact combat-state hash and
run-session fingerprint. Seed, floor, a workspace-local node id, and filenames
are not sufficient identity. A mismatch reports only a bounded set of
same-floor candidate roots and does not continue as if the case belonged to
the selected history.

The selected checkpoint's unresolved combat uses the typed
`final_active_combat` origin rather than a fabricated journal entry. If an old
witness stops replaying before the requested case boundary, the command names
both facts: no validated root matched before the replay failure, and the
specific replay failure remains visible. That is a fail-closed unknown, not a
claim that the unreachable remainder was audited.

A divergence is a counterfactual candidate, not causal proof. Import the exact
prefix into a fresh workspace, apply one alternative, and give only that
branch a bounded downstream wall:

```powershell
.\ol.cmd import --continuation .oracle-lab/cases/<fresh-id>.continuation.json `
  --workspace .oracle-lab/cases/<fresh-id>.workspace.json
.\ol.cmd choose --workspace .oracle-lab/cases/<fresh-id>.workspace.json `
  --owner-rank 0
```

When the decision under investigation is not a current-owner divergence,
export its exact historical boundary directly instead of editing a
continuation or reconstructing state from display text:

```powershell
.\ol.cmd export-run-witness-prefix `
  --workspace .oracle-lab/cases/<run>.workspace.json `
  --node <node> --journal-entry <entry> `
  --output .oracle-lab/cases/<fresh-id>.continuation.json
```

The exported continuation stops immediately before the selected committed
journal entry and clears combat-local diagnostic state. Import it into a fresh
workspace before creating counterfactual children.

## Run One Resident Seed

Create and start one exact F0 workspace, then send one typed resident run
transaction:

```powershell
cargo oracle-lab new --seed 20260713009 --ascension 0 --workspace .oracle-lab/cases/seed009.workspace.json
.\ol-live.cmd start --session seed009 --workspace .oracle-lab/cases/seed009.workspace.json
.\ol-live.cmd live --session seed009 run --export-continuation .oracle-lab/cases/seed009.victory.continuation.json
```

The resident runtime owns owner/search/acceptance and saves at terminal or an
explicit stop. Maintained hallway, elite, and Boss wall defaults come from the
CLI budget contract. It accepts only replay-verified incumbents that satisfy
the applicable strategic survival contract. A budget stop preserves the exact
resident workspace instead of restarting or selecting a historical donor.

At terminal victory the service replays the committed journal from canonical
seed state and exports the exact continuation when requested. Combat staging,
potion admission, and run-wall materialization semantics are maintained in
[../ARCHITECTURE.md](../ARCHITECTURE.md#runner-and-combat).

## Resident Lifecycle

Resident services autosave after every mutation and on shutdown. Startup
accepts mutable workspaces only below `.oracle-lab/`; committed fixtures and
historical files below `target/` remain offline inputs and are never rewritten
by resident startup.

On Windows the lightweight client detaches a newly launched resident host from
one-shot callers. `cargo ol-live` is the build-owning entry point;
`.\ol-live.cmd` invokes the canonical client directly for repeated calls. The
client validates its own canonical path and depfile before contacting a
resident, so source drift fails before a mutation and requires one explicit
`cargo build --release -p oracle_lab_client --bin oracle_lab_client`. `live
start` compares an existing endpoint's immutable service image with the current
canonical host: a matching image is reused; a stale image is saved and replaced
while exact run state and charged historical work remain durable. In-memory
tactical frontier work belongs to the old image and is deliberately restarted.
With exactly one validated endpoint below `.oracle-lab/sessions` whose process
runs the endpoint's recorded immutable host image, repeated `live` calls may
omit the session name. Stale endpoints, dead PIDs, and reused PIDs are ignored;
no active endpoint or more than one active endpoint is an explicit error rather
than an implicit selection.

## Consecutive Seed Panels

Do not launch resident services from a PowerShell loop. Use the single-process
panel owner so each completed seed drops search memory, writes its report, and
retains a full workspace only for a real stop:

```powershell
cargo oracle-lab seed-panel `
  --seed-start 20260713006 `
  --ascension 0 `
  --output-dir .oracle-lab/panels/a0-10
```

The maintained daily defaults are 10 consecutive seeds, 30 seconds of total
run work per seed, and a 10-minute invocation wall. Larger counts remain
resumable but stop at that invocation bound. Use `--invocation-wall-ms 0` only
for an explicitly monitored uninterrupted run.

Panel artifacts are partitioned by outcome:

- verified victories keep compact reports and exact continuations under
  `reports/` and `witnesses/`;
- budget or correctness stops retain resumable workspaces under `incomplete/`;
- `panel.summary.json` records source identity, budgets, per-seed outcome,
  remaining count, timings, persistence phases, and artifact paths.

Rerunning the same command skips verified victories and stable exhausted
stops. Wall or durable-boundary interruptions resume automatically. Use
`--retry-stopped` only after deliberately changing an allowance, and `--force`
only when the selected seeds should restart from F0.

A top-level invocation-wall interruption after a durable boundary is a
successful slice, not a seed failure. For partial panels, report completed
seeds, elapsed time, changed outcomes, repeated censoring reasons, and errors.

## Artifact Safety

Resident endpoints, immutable service images, mutable cases, and research
checkpoints live outside Cargo's `target/` tree so `cargo clean` cannot erase
them. Historical `target/oracle-cases/` inputs remain readable but must not
receive new output.

When a replay-verified combat witness must be inspected later, pass
`--write-witness-actions <path>` to save the exact action array or
`--write-witness-trace <path>` to retain resolved card/potion identity and
compact before/after state. Feed the action artifact back through
`--watch-actions <path>` or another replay surface instead of reconstructing
actions from display text.
