# Runbook

This file is the compact entry point for maintained local workflows. Keep it
command-oriented and version-independent:

- architecture and policy rules belong in [ARCHITECTURE.md](ARCHITECTURE.md);
- test ownership belongs in [TESTING.md](TESTING.md);
- detailed operational recipes belong in the
  [runbooks](runbooks/README.md) directory;
- CLI flags and emitted schema versions are owned by the corresponding Rust
  command, not copied here.

## Command Surfaces

Use the narrowest maintained surface for the task:

| Surface | Purpose |
| --- | --- |
| `branch_tiny` | One bounded owner-audit seed or capsule continuation. |
| `branch_panel` | Small smoke, continuation, drain, or comparison panels. |
| `combat_case_review` | Diagnostic review of one saved combat case. |
| `combat_search_v2_driver` | Fixed combat starts, captures, benchmarks, and offline laboratories. |
| `cargo oracle-lab` / `cargo ol` | Build-owning offline oracle commands. |
| `.\ol.cmd` | Repeated offline oracle calls through the canonical artifact guard. |
| `cargo ol-live` / `.\ol-live.cmd` | Resident oracle sessions. |
| `.\ol-contract.cmd` | Narrow replay-verified exact-combat contracts. |

The heavy `oracle_lab` and `oracle_lab_service` targets require the internal
`canonical-oracle-artifacts` feature. Do not bypass the aliases with an
ad-hoc Cargo build. The retired `fast-run` profile and target directory are
not operational entry points.

Detailed recipes:

- [Combat Evidence And Offline Laboratories](runbooks/combat-evidence.md)
- [Oracle Operations](runbooks/oracle-operations.md)
- [Performance Investigation](runbooks/performance.md)

## Branch Tiny And Branch Panels

`branch_tiny` writes run capsules with `summary.json`, `path.json`,
optional `frontier.json`, optional `terminal.json`, and combat cases when
combat search blocks.

Run one seed:

```powershell
cd D:\rust\sts_simulator
cargo run -p sts_oracle_tools --bin branch_tiny -- --seed 1552225673 --ascension 0 --max-branches 1 --wall-ms 60000
```

Run a small panel:

```powershell
cargo run -p sts_oracle_tools --bin branch_panel -- panel smoke --seeds 1552225671 1552225672 1552225673 1552225674 1552225675 --capsule-root tools/artifacts/panels/current --max-branches 1 --slice-ms 60000
```

Use the panel to classify blockers. Do not treat one seed as a strategy
verdict.

For bounded continuation, use `drain`:

```powershell
cargo run -p sts_oracle_tools --bin branch_panel -- panel drain --seeds 1552225671 1552225672 --capsule-root tools/artifacts/panels/current --max-slices 3 --slice-ms 60000
```

The retired `tools/gap_panel.py` compatibility wrapper has been removed. Use
`branch_panel` directly.

## Continue A Capsule

When a capsule soft-stops with a frontier, continue from that capsule instead
of rerunning from Neow:

```powershell
cargo run -p sts_oracle_tools --bin branch_tiny -- --continue-capsule <capsule-dir>
```

Continuation may inherit run-contract values such as `wall_ms` from the
capsule manifest. Override them only when the investigation needs a different
contract.

## Combat Case Work

Start a diagnostic review from one saved case:

```powershell
cargo run -p sts_oracle_tools --bin combat_case_review -- --case <case.json> --ladder
```

Run the captured production owner directly from a case that carries validated
owner context:

```powershell
.\ol.cmd combat-case-owner-parity --case <case.json> --wall-ms 30000
```

The default command is in-memory and writes no workspace, timeline, or sidecar.
Use `--keep-debug <path>` only when the full advance report and resumable
analysis checkpoint are needed. Legacy, counterfactual, descendant, and
state-only cases are rejected rather than combined with inferred owner defaults.

Review output is diagnostic. It does not mutate runner policy or prove that a
deck is good or bad.

Index local combat evidence and replay exact relationships:

```powershell
.\ol.cmd combat-evidence-audit
```

Compare potion lanes from one unchanged exact root:

```powershell
.\ol.cmd combat-case-potion-expenditure-audit `
  --case <case.json> `
  --max-combination-size 2 `
  --export-witness-actions-dir <fresh-output-directory> `
  --wall-ms-per-lane 10000
```

Audit an exact descendant turn without first exporting another case. The plan
audit defaults to no potions; `--potion-slot` opens only that zero-based
concrete identity lane and disables discard:

```powershell
.\ol.cmd turn-action-audit --case <case.json> --actions <actions.json> --through <N>
.\ol.cmd turn-plan-audit --case <case.json> --actions <actions.json> --through <N> --potion-slot 2
.\ol.cmd turn-quality-corridor --case <case.json> --min-boundary-player-hp 14 --min-terminal-player-hp 20 --potion-slot 2 --max-turns 3
.\ol.cmd turn-quality-frontier --case <case.json> --checkpoint <depth-N.json.gz> --export-representatives-dir <fresh-dir> --probe-next-turn-roots 512
```

Compare two caller-supplied exact routes from the same unchanged combat root:

```powershell
.\ol.cmd combat-case-route-compare `
  --case <case.json> `
  --route-a-actions <route-a.actions.json> `
  --route-b-actions <route-b.actions.json>
```

The comparison is neutral replay evidence: it reports the shared action prefix,
first divergence, aligned typed turn boundaries, and state deltas. It does not
search, rank either route, prune alternatives, or create a teacher label.

The corridor command deduplicates exact next-turn states. Its unresolved-turn
floor and post-victory terminal floor are separate because victory relics and
combat healing can increase HP. It reports every planner or frontier cap as
censoring; only an uncensored exhausted boundary frontier supports a bounded
non-existence conclusion under the declared intermediate floor.

Use a fresh compressed checkpoint path when another turn would otherwise
repeat earlier layers. Resume against the same case and enumeration identity;
only `--max-turns` may increase:

```powershell
.\ol.cmd turn-quality-corridor <same-args> --checkpoint-out <depth-3.json.gz>
.\ol.cmd turn-quality-corridor <same-args> --max-turns 4 --checkpoint-in <depth-3.json.gz> --checkpoint-out <depth-4.json.gz>
```

`turn-quality-frontier` validates every stored exact hash, reports typed HP,
enemy-composition, potion-identity, and active-setup distributions, and exports
diagnostic descendant cases with their exact prefix actions. Its optional
next-turn probe inspects only the requested survival-ranked roots; an unprobed
remainder is reported as censoring, never as a non-existence result.

For a descendant suffix whose victory heal makes relative HP loss ambiguous,
use absolute terminal satisfaction. A retained potion requires an explicit
slot contract; an already-consumed descendant can remain potion-free:

```powershell
.\ol.cmd combat-case-local-graph --case <descendant.case.json> --satisfy-min-final-hp 20 --max-potions-used 0
.\ol.cmd combat-case-local-graph --case <root.case.json> --satisfy-min-final-hp 20 --max-potions-used 1 --potion-slot 2
```

Keep full JSON and build output below `.oracle-lab`; report aggregate lane
results and a short failure tail. A missing budget-limited witness remains
unknown. See [Combat Evidence And Offline
Laboratories](runbooks/combat-evidence.md) for manifests, typed queries,
fresh-case capture, potion interpretation, and laboratory artifacts.

## Combat Search Driver

Use `combat_search_v2_driver` for fixed combat starts, captures, and
benchmark suites:

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

If combat search reports unresolved, it failed to find an executable complete
win under the current contract. It did not prove the fight unwinnable.

The resumable Combat Laboratory and Campfire Threat Panel remain offline modes
of this driver. Their maintained commands and artifact contracts are in
[Combat Evidence And Offline Laboratories](runbooks/combat-evidence.md).

## Planner Capture Export

The retired interactive driver no longer produces live `SessionTraceV1`
captures. Existing compatible traces remain readable while capture moves to
the atomic run-job journal. Export a dataset and coverage report from an
existing typed trace with:

```powershell
cargo run -p sts_oracle_tools --bin rl_dataset_export -- --input artifacts/runs/example/trace.json --out artifacts/runs/example/planner-dataset.json --planner-coverage-out artifacts/runs/example/planner-coverage.json
```

The coverage report measures representation and linkage only. It does not rank
decision sites or promote recorded behavior to a correct-action label.

## Production Oracle Quick Start

Diagnose a suspicious late stop before changing combat policy or starting a
seed panel:

```powershell
.\ol.cmd diagnose-run-witness `
  --workspace .oracle-lab/cases/<run>.workspace.json `
  --node <node> --max-pivots 5 `
  > .oracle-lab/reports/<fresh-id>.diagnosis.json
```

Create and run one exact F0 resident workspace:

```powershell
cargo oracle-lab new --seed 20260713009 --ascension 0 --workspace .oracle-lab/cases/seed009.workspace.json
.\ol-live.cmd start --session seed009 --workspace .oracle-lab/cases/seed009.workspace.json
.\ol-live.cmd live --session seed009 run --export-continuation .oracle-lab/cases/seed009.victory.continuation.json
```

Run consecutive seeds in one process instead of launching resident services
from a PowerShell loop:

```powershell
cargo oracle-lab seed-panel `
  --seed-start 20260713006 `
  --ascension 0 `
  --output-dir .oracle-lab/panels/a0-10
```

The maintained seed-panel defaults are 10 consecutive seeds, 30 seconds of run
work per seed, and a 10-minute invocation wall. Resident lifecycle, durable
resume rules, report locations, and diagnosis continuation are documented in
[Oracle Operations](runbooks/oracle-operations.md).

## Verification

Start with the narrowest check that owns the changed surface. For code changes,
the maintained broad handoff commands are:

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

The root package is the sole default member and disables automatic bins and
integration tests. Bare `cargo test --lib` therefore checks only the core
package. Use `cargo test-core <filter>` and `cargo test-control <filter>`
for targeted tests, then broaden in proportion to risk. Compilation ownership
and dependency direction are maintained in
[ARCHITECTURE.md](ARCHITECTURE.md#cargo-package-boundary).

For documentation-only changes:

```powershell
git diff --check
```

Run targeted tests only when the changed surface has a stable structural
contract worth protecting. Do not add or preserve tests for retired probes,
temporary reports, or prose-only behavior.

## Iteration Ladder

Use the smallest evidence surface that can distinguish the current hypotheses.
More seeds do not repair a censored or ambiguous measurement.

1. **Contract test, usually under 30 seconds.** Protect the exact ownership,
   budget, replay, or stage-transition rule below the full runner.
2. **Fixed combat root, normally under 60 seconds total.** Run one unchanged
   `CombatCase` with one factor changed at a time. Compare exact root hash,
   witness status, generation work, final HP, potion UUIDs, and replay
   compliance. A missing budget-limited witness remains unknown.
3. **Production parity, normally under two minutes.** Copy a saved workspace
   to a fresh ignored path, restart only its current combat, advance the real
   resident production portfolio, and replay-verify the committed journal.
   When isolated and production witnesses diverge, compare the exact-root
   `stage_trace` first.
4. **Two to five contract-selected sentinels, normally under three minutes.**
   Select a changed contract, a known success, a known hard unknown, and any
   observed regression. Record why each sentinel is present.
5. **Distribution panel.** Run 20 or more consecutive seeds only when the
   lower layers pass and the remaining question is genuinely about outcome
   frequency, path distribution, crashes, or long-run resource use.

Every experiment should answer one declared question. Stop escalating when the
first causal divergence is localized. Do not raise budgets merely because two
algorithms both report unknown. Calibrate diagnostic lanes before interpreting
their rankings.

For a partial panel, preserve `panel.summary.json` and report completed seeds,
elapsed time, changed outcomes, repeated censoring reasons, and errors. It is
valid to stop after a durable seed boundary when the remaining samples no
longer justify their runtime.
