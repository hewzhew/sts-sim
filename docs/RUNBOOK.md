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
| `.\ol.cmd drive` | Bounded current-owner and ordinary-combat progression in one process. |
| `cargo ol-live` | Build or rebuild the lightweight resident client, then run it. |
| `.\ol-live.cmd` | Invoke the validated canonical resident client directly for low-latency repeated calls. |
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

For one exact workspace, avoid shell-level `owner`/`advance` alternation with
the bounded drive surface:

```powershell
.\ol.cmd drive --workspace <workspace.json> `
  --max-steps 64 --wall-ms 60000 `
  --output .oracle-lab/reports/<fresh-id>.drive.json
```

`drive` recomputes each non-combat decision through the current typed owner,
uses ordinary non-improving combat advance, and saves after every mutation.
It stops on its total wall/step limit, a boundary without an owner answer, or
an unresolved combat attempt. Pass `--stop-at shop` (or another typed boundary)
to stop before the first step at that boundary. Its event ledger is execution
evidence, not a teacher label or a strategy verdict. Owner events retain the
exact choice ref, typed action, source fingerprint, and resource delta; combat
events retain the typed source encounter and exact combat-root identity. One
typed initial deck/relic/potion snapshot makes those deltas self-contained
without repeating full inventories on every event. Stdout is always a compact
execution receipt; pass `--output` when the complete event ledger is needed.

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

Before combining a run witness with a saved combat case, lock their exact
origin instead of trusting a seed, node number, floor, or filename:

```powershell
.\ol.cmd diagnose-run-witness `
  --workspace <run.workspace.json> --node <workspace-local-node> `
  --case <case.json>
```

Witness verification, policy audit, diagnosis, prefix export, and historical
combat export default to the workspace cursor when `--node` is omitted. Use
`--node 0` explicitly when the Neow root is actually intended.

The case must carry exact production context and match one unambiguous combat
root by both combat-state hash and run-session fingerprint. A node id is local
to one workspace. `timeline` emits the selected line fingerprint and embeds
the corresponding exact root identity in every returned combat entry.
An unresolved combat at the selected final checkpoint is represented as the
typed `final_active_combat` origin with no fabricated journal-entry number.

Workspace nodes retain the owner ranks that were materialized when the node
was created. `status`, `owner`, and `choose --owner-rank` therefore recompute
the current owner ordering from the node's exact session and join it back to
the persisted choice surface by candidate id. Compact status keeps the old
value as `materialized_owner_rank` and emits the current value as
`owner_rank`; raw `view` remains a materialized tree view. A missing or
ambiguous candidate-id join fails instead of falling back to a label.
Compact status also carries the exact state fingerprint, deck, relics, potion
slots, keys, and current typed reward state. Policy investigation should not
reopen the workspace checkpoint to recover those facts.

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

For one-factor attribution inside the local graph, `--omit-guide-lane <N>`
removes one positive typed guide lane without changing action legality or
terminal truth. `--boost-guide-lane <N> --boost-guide-extra-services <K>` gives
that existing lane `K` additional boundary-selection turns per rotation while
keeping every entry one-shot and every service quantum unchanged. These are
lab controls: they are recorded in `execution_profile` / `search_spec`, cannot
be combined with `--anchor-only`, and the boosted lane cannot also be omitted.

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

At a retained combat node, a quality-reaching witness does not prove that its
local HP is optimal. Spend one deliberately small diagnostic grant in the
current exact potion stage without advancing to another identity or committing
the witness:

```powershell
.\ol-live.cmd live --session seed009 probe-combat `
  --generation-work 4096 --quantum-nodes 256 --wall-ms 1000
```

The result reports `work_budget_reached`, `wall_reached`, `stage_exhausted`, or
`no_progress`, plus the exact work consumed and current incumbent. The resident
frontier remains in memory. Repeat another bounded probe only when its result
can distinguish a concrete hypothesis; use `live ... accept` to materialize
the retained incumbent explicitly. `advance --improve-incumbent` has a
different contract: it stops once configured strategic quality is reached and
is not a request to exhaust the whole caller budget.

Offline parity is available for a single invocation:

```powershell
cargo oracle-lab probe-combat `
  --workspace .oracle-lab/cases/seed009.workspace.json `
  --generation-work 4096 --quantum-nodes 256 --wall-ms 1000
```

At an exact combat node, open one resident combat line lab from the complete
verified incumbent. The normal loop uses observed turn/action coordinates and
typed card ids; it never requires scratch node ids or action-list JSON:

```powershell
.\ol-live.cmd live --session seed009 lab open --node <combat-node> --baseline incumbent
.\ol-live.cmd live --session seed009 lab goto --line current --turn <observed-turn> --before <action-ordinal>
.\ol-live.cmd live --session seed009 lab back
.\ol-live.cmd live --session seed009 lab play --card PowerThrough [--copy <occurrence>] [--target <monster-index>]
.\ol-live.cmd live --session seed009 lab potion --potion FearPotion [--copy <occurrence>] [--target <monster-index>]
.\ol-live.cmd live --session seed009 lab end
.\ol-live.cmd live --session seed009 lab search --max-quanta 4 --quantum-nodes 1024 --wall-ms 1000
.\ol-live.cmd live --session seed009 lab compare
.\ol-live.cmd live --session seed009 lab commit
```

`ol-live.cmd` bypasses Cargo's per-process startup on the repeated card loop.
The canonical client still validates its own path and depfile before contacting
the resident service; if it is missing or stale, rebuild it once with
`cargo build --release -p oracle_lab_client --bin oracle_lab_client`.
When exactly one validated resident endpoint names a process running its
recorded immutable host image, routine `live` calls may omit `--session
<name>`; stale endpoints, dead PIDs, and reused PIDs are ignored, while zero or
multiple active sessions require an explicit session or endpoint:

```powershell
.\ol-live.cmd live lab observe
.\ol-live.cmd live lab play --card PowerThrough
```

`open`, `goto`, `back`, and `observe` return a complete decision frame. `goto`
defaults to `--line current`; use `--line baseline` to recover an imported
incumbent frame. A normal `play`, `potion`, or `end` returns only the typed state
delta. Duplicate copies of one card or potion id return candidate occurrences;
an action with several legal targets returns target ambiguity. Neither case
mutates the line. A unique copy and unique target resolve automatically.
`compare` reports the common prefix, first semantic divergence, both exact
divergent tails, per-turn HP/block/enemy totals, potion use, and whether each
suffix is terminally known. The semantic action projection contains card/potion
ids and local selectors, not UUID-bearing diagnostic action keys.
Resident execute/autosave timing is carried separately in the raw service
response envelope and does not inflate normal typed `ol-live` results. Use the
low-level `call` surface only when transport timing itself is under diagnosis.
Structured Hand/Grid/Scry inputs use local domain indices and are returned in
bounded pages; use `--selection-offset` and `--selection-limit` without
materializing the complete combination space.
Lab action deltas carry semantic before/after locations and only changed typed
fields. Card piles use one validated prefix/remove/insert splice; potions use
stable slot removals/upserts; monsters use indexed field updates unless topology
changed. Internal DAG ids remain in the persisted checkpoint and low-level
scratch adapter, not in the normal lab response.
CLI JSON is emitted on one compact line; pipe it through a formatter only for
ad-hoc human inspection.
The longer exact-hash action refs and hidden UUID selectors remain accepted by
the scratch adapter for old diagnostic callers, but are not part of the normal
lab interface.

Ask the existing portfolio for one deliberately small potion-free suffix from
the current line only when manual play needs help:

```powershell
.\ol-live.cmd live --session seed009 lab search `
  --max-quanta 4 --quantum-nodes 1024 --wall-ms 1000
```

A found suffix is appended to the line lab; a missing bounded witness remains
unresolved. `lab commit` succeeds only at a terminal win,
replays the complete prefix from the unchanged run combat root, creates one
atomic combat-witness child, and clears the lab. `lab clear` discards only the
temporary DAG.

The older `scratch` commands remain a low-level compatibility surface for
structured selections and exact diagnostic selectors during the cutover. Do
not add new normal workflows there; move a missing semantic operation into
`lab` and then retire the overlapping scratch command. Offline scratch parity
remains available through `.\ol.cmd combat-scratch --workspace <workspace>
<subcommand>` until that migration completes.

Audit every materialized card-reward choice on the current exact path without
guessing node ids or opening the workspace checkpoint:

```powershell
cargo oracle-lab card-reward-path `
  --workspace .oracle-lab/cases/<run>.workspace.json `
  --output .oracle-lab/reports/<fresh-id>.card-rewards.json
```

The complete typed deck, relic, potion, candidate, owner-band, and applied-edge
evidence is written to the fresh output path. Capability rule changes retain
the evaluator's typed before/after inputs even when coverage does not improve,
so a count-only change can be audited without reopening strategy source.
Stdout stays a compact artifact receipt. For an active resident session, use
`card-reward-path` through the matching `ol-live` surface with the same
optional `--output` behavior.

Explain the current route owner's full typed context and candidate ordering at
one exact retained map node without starting a resident service:

```powershell
cargo oracle-lab route-policy-audit `
  --workspace .oracle-lab/cases/<run>.workspace.json `
  --node <workspace-local-node>
```

Omit `--node` to audit the current cursor. This command is read-only and emits
the same route-policy evidence as the resident `route` command.

The matching exact shop-owner audit is also available without a resident
service:

```powershell
cargo oracle-lab shop-policy-audit `
  --workspace .oracle-lab/cases/<run>.workspace.json `
  --node <workspace-local-node>
```

Omit `--node` to audit the cursor. This is the same read-only typed evidence as
the resident `shop` command; it does not buy, remove, or leave.

When an analysis workspace has accumulated many historical variations, create
a fresh active workbench from one exact committed node without modifying or
overwriting the source:

```powershell
cargo oracle-lab compact-workspace `
  --workspace .oracle-lab/cases/<historical>.workspace.json `
  --node <workspace-local-node> `
  --output .oracle-lab/cases/<fresh-active>.workspace.json
```

The output path must be fresh. The command exact-replays the committed journal,
rebuilds a one-node workspace, reloads the written artifact, and requires the
same journal and final-state fingerprints before reporting success. Historical
variations remain only in the unchanged source workspace. A selected node with
resident combat search is rejected because ordinary run continuations do not
preserve that search frontier.

To migrate an older large workspace to pooled checkpoint storage while
preserving every variation, use a fresh output path:

```powershell
cargo oracle-lab repack-workspace `
  --workspace .oracle-lab/cases/<historical>.workspace.json `
  --output .oracle-lab/cases/<pooled>.workspace.json
```

`repack-workspace` writes the current content-addressed payload format, verifies
the written JSON against the exact generated artifact, restores it through the
typed workspace loader, and requires the complete variation tree to match.
Unlike `compact-workspace`, it retains the entire variation DAG. Both commands
leave the source untouched and reject an existing output path.

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
