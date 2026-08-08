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
| `cargo oracle-lab contract --help` | Rebuild the canonical oracle host and show the compact V2 contract surface. |
| `.\ol.cmd contract combat` | One bounded exact-combat experiment with compact stdout and automatic full evidence. |
| `.\ol.cmd artifact summary/summaries/search/trace/compare/turn/rerun` | Read one or several results, inspect search service, replay/compare candidates, inspect one exact turn, or reproduce a V2 request without parsing full reports. |
| `.\ol.cmd case import/list` | Admit and query exact roots in the explicit V2 catalog. |
| `.\ol.cmd drive` | Bounded current-owner and ordinary-combat progression in one process. |
| `cargo ol-live` | Build or rebuild the lightweight resident client, then run it. |
| `.\ol-live.cmd` | Invoke the validated canonical resident client directly for low-latency repeated calls. |
| `.\ol-contract.cmd` | Narrow replay-verified exact-combat contracts. |

The heavy `oracle_lab` and `oracle_lab_service` targets require the internal
`canonical-oracle-artifacts` feature. Do not bypass the aliases with an
ad-hoc Cargo build. The retired `fast-run` profile and target directory are
not operational entry points.

`.\ol.cmd --help` is a deliberately non-enumerating gateway to the maintained
command groups; it must not duplicate their subcommand inventory. Use
`.\ol.cmd <command> --help` for one current typed surface or `.\ol.cmd help`
to delegate the full top-level help to the canonical binary.

Detailed recipes:

- [Combat Evidence And Offline Laboratories](runbooks/combat-evidence.md)
- [Oracle Operations](runbooks/oracle-operations.md)
- [Performance Investigation](runbooks/performance.md)

Routine combat experiments must not use PowerShell to list guessed filenames,
select JSON paths, or construct report summaries. Admit one exact root once,
then run and reproduce it through the stable V2 protocol:

```powershell
.\ol.cmd case import --case <case.json>
.\ol.cmd contract combat --case-id <unique-root-prefix> `
  --min-final-hp 20 --max-potions-used 0 `
  --require-recovered-stolen-gold --generation-work 4096
.\ol.cmd artifact summary <artifact-directory>
.\ol.cmd artifact summaries <artifact-a> <artifact-b> <artifact-c>
.\ol.cmd artifact search <artifact-directory>
.\ol.cmd artifact search <artifact-directory> --state <exact-hash-or-prefix>
.\ol.cmd artifact trace <artifact-directory>
.\ol.cmd artifact trace <artifact-directory> --detail checkpoints
.\ol.cmd artifact trace <artifact-directory> --detail policy
.\ol.cmd artifact compare <artifact-directory>
.\ol.cmd artifact turn <artifact-directory> --candidate contract --turn 1
.\ol.cmd artifact turn <artifact-directory> --candidate contract --turn 1 --follow-plan 2
.\ol.cmd artifact turn <artifact-directory> --candidate contract --turn 1 `
  --follow-state <exact-successor-prefix> --reached-only
.\ol.cmd artifact turn <artifact-directory> --candidate contract --turn 1 `
  --follow-state <exact-successor-prefix> --successor-state <next-successor-prefix>
.\ol.cmd artifact branch <artifact-directory> --candidate contract --turn 1 `
  --follow-state <exact-successor-prefix> --generation-work 4096 --wall-ms 2000
.\ol.cmd artifact rerun <artifact-directory>
```

`contract combat` writes a fresh `.oracle-lab/v2/contracts/<id>/` directory.
`manifest.json` owns the typed request, compact result, compact per-depth search
service accounting (including typed proposal root/continuation enqueue,
generation, and independent service counts), source identity,
and paths to `report.json`,
the compact exact-state service index, plus one
replay-exact action sidecar for every retained non-dominated terminal candidate.
Summary, summaries, search, and rerun read only V2 manifests. `summaries`
returns one typed result set in caller order so comparisons do not require a
PowerShell JSON aggregation pipeline. Trace replays the
contract-aligned candidate and emits only compact action keys, policy ranks,
and turn-boundary checkpoints by default. `--detail checkpoints` omits action
policy payloads entirely; full per-action inputs, probabilities, and top
choices require the explicit `--detail policy` opt-in.
Compare authoritatively replays both the contract-aligned and
local-HP candidates, then reports their first exact action divergence and both
turn-boundary histories. Turn resolves one of those retained candidates by
semantic role and enumerates a bounded exact complete-turn surface at the
requested observed turn; it never asks callers for action sidecar paths or
scratch ids. Repeat `--follow-plan <displayed-index>` to walk exact complete-turn
successors and inspect the reached turn directly; the command replay-validates
each successor and does not export an intermediate case. Prefer
`--follow-state <exact-hash-or-prefix>` when the successor identity is already
known, and use `--successor-state <exact-hash-or-prefix>` to return only one
matching plan from the reached surface instead of printing and reparsing every
sibling. Use `--reached-only` when the selected successor itself is the answer:
it replay-checks the navigation and returns the source, followed plan summary,
and reached state without enumerating the next surface. Callers never join case
and action paths or restate the contract.
`artifact search --state` reports whether one exact state was retained and
whether its complete-turn generator received service, its current anchor,
proposal-root, and proposal-continuation queue positions, and their service
counts. It also reports whether the boundary's typed proposal was applicable,
root-eligible or continuation-only, attempted, completed, or rejected, the
exact successor identities it
materialized, the boundary service source that actually consumed work, and
whether the generator's internal lane used anchor or guide service; it never
parses the opaque full report.
`artifact branch` inherits the artifact's exact original root and complete
contract, replays the retained candidate to `--turn`, follows the requested
plan indices or exact successor identities, and starts one fresh bounded suffix
search there. It does not export a descendant case or ask the caller to restate
HP, potion, or stolen-gold constraints. Every retained suffix is concatenated
with the bounded diagnostic prefix and replayed from the unchanged original
root before it can enter the new ordinary V2 artifact. Prefix potion
expenditures reduce the suffix allowance; the prefix itself is reported
separately from charged generation work. `artifact summary`, `search`, `trace`,
`turn`, and `rerun` work unchanged on the result. Search-state queries are
relative to the manifest's explicit `search_root_exact_state_hash`, while
trace and terminal candidates remain relative to the original exact root.
These commands deliberately reject earlier manifest schemas and legacy reports
instead of guessing their fields.

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
use absolute terminal satisfaction. The V2 contract starts from an exact case
root, enumerates a bounded exact complete-turn frontier, enforces its typed
potion and stolen-gold contract during terminal admission, and classifies a
missing budget-limited result without expanding stdout:

```powershell
.\ol.cmd contract combat --case <descendant.case.json> `
  --min-final-hp 20 --max-potions-used 0 --generation-work 4096
```

Concrete potion-identity comparisons belong to
`combat-case-potion-expenditure-audit`; the compact V2 contract does not expose
local-graph scheduler, guide, or slot-ablation knobs. `artifact search` exposes
read-only per-depth service accounting, not knobs; add `--states` only when a
few highest-service exact-state samples are needed. Add a typed contract owner
when a repeated causal question truly needs one instead of reopening the old
all-flags command.

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
.\ol-live.cmd live --session seed009 lab keep
.\ol-live.cmd live --session seed009 lab restore
.\ol-live.cmd live --session seed009 lab play --card PowerThrough [--copy <occurrence>] [--target <monster-index>]
.\ol-live.cmd live --session seed009 lab potion --potion FearPotion [--copy <occurrence>] [--target <monster-index>]
.\ol-live.cmd live --session seed009 lab select --indices 0,2,3
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
incumbent frame. Before `goto` or `back` leaves the first non-baseline terminal
win, the lab keeps that exact line automatically. `keep` deliberately replaces
the one-slot recovery point with the current line; `restore` returns to it. A
kept summary in full frames is recovery state, not a best-line verdict. A normal
`play`, `potion`, or `end` returns only the typed state delta. Duplicate copies
of one card or potion id return candidate occurrences; an action with several
legal targets returns target ambiguity. Neither case mutates the line. A unique
copy and unique target resolve automatically.
The typed ambiguity response is still printed, but the canonical client exits
nonzero so a chained shell command cannot mistake "needs an explicit selector"
for an applied action.
Each living monster keeps its raw `intent` for base mechanics and also reports
`move_preview.damage_per_hit`, `hits`, and `total_damage` after the simulator's
Strength/Weak/Vulnerable and protocol-visible damage pipeline. Dead or escaped
monsters have no move preview.
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

Configure one stable Python 3.12 training runtime once, then use the small
`learning/dev.ps1` surface for routine learning work:

```powershell
.\learning\dev.ps1 configure -Python <python-3.12-with-numpy-torch-and-bridge>
.\learning\dev.ps1 doctor
.\learning\dev.ps1 check-bridge
.\learning\dev.ps1 refresh-bridge
.\learning\dev.ps1 test
.\learning\dev.ps1 verify -MaturinPython <python-with-maturin>
.\learning\dev.ps1 train-combat -Artifact <roots.bin> -Output <fresh-dir> -Roots <count> -Updates <count> -PotionLane never
.\learning\dev.ps1 evaluate-combat -Artifact <held-out-roots.bin> -Behavior <training-dir> -Output <fresh-dir> -Roots <count> -Replicates <count>
.\learning\dev.ps1 evaluate-combat-potions -Artifact <held-out-roots.bin> -Behavior <training-dir> -Output <fresh-dir> -Roots <count> -Replicates <count>
.\learning\dev.ps1 evaluate-run -Behavior <training-dir> -Output <fresh-dir> -Attempts 8 -MaxBatchSteps 4096 -BehaviorSeed 10000 -HeldOutSeedStart 0 -RunPotionLane trained
.\learning\dev.ps1 evaluate-run-potions -Behavior <training-dir> -Output <fresh-dir> -Attempts 8 -MaxBatchSteps 4096 -BehaviorSeed 10000 -HeldOutSeedStart 0
.\learning\dev.ps1 train-run -Behavior <combat-training-dir> -Output <fresh-dir> -Slots 4 -Generations 1 -AttemptsPerUpdate 8 -MaxBatchSteps 4096 -EvaluationAttempts 16 -HeldOutSeedStart 1000000 -AdvantageMode raw-return -RunPotionLane trained
```

`configure` installs the tool requirements declared by the local
`sts-learning[test]` extra into the selected runtime, then verifies Python,
pytest, NumPy, PyTorch, the repository caller, and the installed bridge. The
caller continues to load from the repository source path, so configuration
does not build or install the local package into its own worktree. No separate
requirements file or global pytest installation is needed.

Use `check-bridge` for the routine bridge edit loop. It builds a dev-profile
wheel and verifies it through the isolated Python smoke and caller suite in
seconds without changing the configured training runtime. A lightweight
pytest tool environment is cached by Python ABI under ignored `.oracle-lab/`
storage and revalidated against the `sts-learning[test]` requirement on every
run. Each bridge wheel and smoke environment remains fresh; optional training
dependencies may stay absent from the pytest tool and their tests may skip. Use
`refresh-bridge` when a real experiment needs the new bridge: it verifies and
installs a release-profile wheel with `--no-deps`, but defers the second Rust
test-binary link. For a functional experiment that does not measure throughput,
`refresh-bridge -BridgeProfile dev` installs the already verified dev-profile
semantics without paying the optimized rebuild; use the default `release`
profile for timing or milestone evidence. Neither command mutates NumPy or
PyTorch. `verify` is the release-profile milestone gate and additionally runs
the Rust bridge contract tests. Ordinary Python-only edits still use `test`. For first setup, pass
`-Python <python.exe>` to refresh and record that runtime only after the guarded
installation and final doctor both succeed.

To collect the first production-owned combat from a fresh seed without an
intermediate workspace or continuation JSON, write one fresh opaque root:

```powershell
cargo oracle-lab learning-root collect --seed <first-seed> --seed <second-seed> --ascension 0 --output <fresh.combat-roots.bin> --max-progress-steps 64 --wall-ms 10000 --max-bytes 16777216
```

The collector accepts one to 64 distinct explicit seeds, uses current
production non-combat owners, and stops each seed before combat search. The
step and wall bounds apply independently to each seed. It writes no artifact
when any seed hits a bound or automation gap, and its compact receipt preserves
the input seed associated with each root. To seed a bounded corpus from
production-owned later combat boundaries, convert one or more public
continuations into a fresh opaque root batch. Every continuation must already
be at an active combat input boundary; private capsule/cutpoint schemas are not
accepted:

```powershell
cargo oracle-lab learning-root export --continuation <first.continuation.json> --continuation <second.continuation.json> --output <fresh.combat-roots.bin> --max-bytes 16777216
```

Pass the file bytes unchanged to
`LearningBatchEnv.from_combat_root_artifact_bytes(payload,
expected_roots=2, max_bytes=16777216)`. Rust validates the exact root count,
canonical envelope, recomputed root identity/context, and current combat
boundary before constructing the batch. Python must not inspect the payload.
When a combat case has validated exact production context and a typed action
file already replays to a win, derive a bounded reverse-curriculum batch with:

```powershell
cargo oracle-lab learning-root recover --case <case.json> --actions <win.actions.json> --output <fresh.combat-roots.bin> --max-roots 8 --max-bytes 16777216
```

Rust restores the production session, replays every input, requires a new typed
combat win, retains at most the terminal-nearest requested roots in memory, and
writes the same opaque root-batch format. The compact receipt reports the
actual root count, which can be smaller than `--max-roots` for a short line.
The action sequence is verification evidence only; it is not included in the
artifact and must not be used as a supervised policy label.
For a bounded fixed-behavior coverage check over every root, use
`CombatWinSignalCensusRunner` with the same `expected_roots`, one shared model
seed, one explicit behavior seed per root, and `max_roots` equal to the intended
census bound. It returns compact per-root generations and a signal census,
imports the opaque batch only once, publishes nothing, and is not a cross-root
training scheduler.

For one bounded shared update, construct
`CombatWinBatchSessionFactory` with a `CombatWinBatchSessionConfig` whose
`expected_roots` exactly equals `profile.objective.groups_per_update` and does
not exceed the separately declared `max_roots`. Pass one distinct explicit
behavior seed per root. `advance()` loads no additional artifact, collects every
root under the same frozen manifest, attempts one group-balanced optimizer
update, and promotes at most once. It writes nothing;
`publish_active_behavior()` is the only durable publication boundary. Treat the
returned loss and promotion as training accounting, not held-out evidence of
improvement.

The batch session binds `PotionLane all|never|root-slots` into its root source.
Prefer `never` when the selected roots already have no-potion winning coverage,
so the all-win terminal-HP axis cannot learn to burn inventory for local HP. An
all-loss `never` group stays no-signal; move it to `root-slots` with one
`-PotionSlots <zero-based-slot>` at a time instead of rerunning unrestricted
`all` by default. The selected slot binds its exact root potion UUID; a later
replacement in that slot remains unavailable.

Use `evaluate-combat` on a distinct opaque root artifact after publication. The
behavior directory must be an exact completed `train-combat` output containing
one durable checkpoint and manifest. The evaluator verifies their complete
provenance, recovers the training artifact digest and potion lane, and rejects
an evaluation artifact with the same digest before constructing combat groups.
It gives every root an independent explicit behavior RNG stream and writes only
`evaluation.json` with per-replicate win, HP/max HP, gold, turn,
concrete starting/final/lost/gained potion identities, potion use/discard
counts, and card facts plus compact aggregates. The completion line includes each root's
site and starting potion identities, so routine resource-loss classification
does not require reopening the JSON. Identity loss is an inventory fact, not a
static potion-value score; strategic retained value requires an exact run
continuation outside this evaluator. It creates no optimizer, trainer,
experience buffer, or behavior promotion. A result measures the exact frozen
manifest on that bounded sample; it does not establish improvement without a
comparable baseline using the same roots and RNG streams.

Select the model-facing potion action surface explicitly with
`-PotionLane all|never|root-slots` (default `all`). `never` removes potion use
and discard from the model candidates without changing simulator legality. Run
an `all` and a `never` evaluation into separate fresh output directories with
the same artifact, behavior seed base, and replicate count to ask whether the
frozen behavior can win while preserving the starting inventory. This is an
evaluation ablation only: it does not price a potion, change training, or
establish continuation value.
If the no-potion result lacks acceptable survival coverage, run one bounded
identity lane at a time, for example
`-PotionLane root-slots -PotionSlots 0`. The output records both the slot
contract and every root's starting potion identities; empty selected slots
admit nothing. Multiple slots are accepted as a PowerShell array only for an
explicit combined fallback, not as the default rescue probe.

For the routine complete comparison, use `evaluate-combat-potions` instead of
manually coordinating those commands. It runs `never`, one separate lane for
every filled root potion slot discovered from the exact root contexts, and
`all`, with the same roots, frozen checkpoint, and behavior seeds. Each lane
keeps its own `evaluation.json`; the top-level `potion-sweep.json` is only a
compact typed index and aggregate comparison. The output directory must be
fresh. Slot lanes remain concrete-identity counterfactuals, not potion prices.
The completion output also emits one bounded line per root with starting site,
HP, potion identities, and lane win/HP/use/discard aggregates, so routine
classification does not require reopening or probing the JSON schema.

Each root also reports an observed-resource Pareto frontier among its winning
replicates. The order compares final HP, max HP, gold, and exact potion
multisets coordinate-wise. HP/potion tradeoffs and unlike potion identities are
incomparable rather than silently converted. Treat this as local classification
of the recorded facts, not proof of run-level continuation value.

Use `evaluate-run` for the separate whole-run question. It recovers the exact
published combat scorer, creates a fresh held-out seed population, disables
recovery and training, and runs until the terminal-attempt target or explicit
batch-step bound. Maintained whole-run evaluation deliberately uses one slot:
an atomic multi-slot step can finish several episodes at once and overshoot a
requested target, invalidating floor-sum and fixed-prefix comparisons. The one
completion line and `evaluation.json` report victory,
defeat, terminal floor sum/range/histogram, act counts, and execution bounds.
The same evaluation records each completed combat's start/end HP, max HP, gold,
and concrete potion-slot identities, plus one compact summary line per observed
seed. A combat still open at the stopping boundary remains explicitly open;
HP, gold, and potion identities remain separate facts rather than a combined
resource score. The default `-RunPotionLane trained` inherits the published
behavior's training potion surface; a root-slot-trained behavior must instead
choose an explicit whole-run lane. Use the same behavior, held-out seed range,
behavior RNG, and bounds with `-RunPotionLane all` and
`-RunPotionLane never` for a bounded
counterfactual: `never` removes combat potion use and discard from the model
surface without changing simulator legality or non-combat potion decisions.
It measures how much observed progress depends on combat potion access; it
does not establish the value of a consumed identity. Root-slot lanes are not
defined for complete runs. For the routine comparison, use
`evaluate-run-potions`: it deliberately uses one environment slot so both
lanes finish the exact same ordered episode seeds, retains both complete V2
evaluations, and writes a compact per-seed `potion-comparison.json`. A combat-
trained scorer has not thereby learned non-combat strategy; the command is a
bounded end-to-end diagnostic of that complete policy surface, not a claim that
all decisions were trained.

Use `train-run` for the first whole-run on-policy handoff. `-Behavior` is a
verified completed `train-combat` directory whose scorer becomes an independent
generation-zero parameter copy. Training uses the `TRAINING` seed partition,
zero recovery, the terminal floor-progress return, and exactly
`-AttemptsPerUpdate` complete runs per generation. Each generation reports its
own terminal-floor histogram. The command publishes only after every requested
generation completes, but publishes only the frozen behavior checkpoint and
manifest—not optimizer or environment resume state. It then evaluates the
result on the disjoint `HELD_OUT` partition and writes a compact `summary.json`.
Each generation journal row also records a bounded credit diagnostic: current
terminal-broadcast decision targets beside decision-local remaining-floor
targets and their matched-floor leave-one-out advantages, including sign counts
and per-decision-floor aggregates. This is a target-distribution comparison
only; training still uses the configured terminal objective and the diagnostic
does not price HP, gold, or potions.
A generation that
hits `-MaxBatchSteps` before an optimizer step fails without publishing its
partial live update.
The default `-RunPotionLane trained` preserves the warm-start behavior's combat
potion candidate surface for both training and held-out evaluation. This avoids
injecting untrained potion actions at the handoff. No-potion run sessions do
not yet support cross-process resume; this command publishes only the frozen
behavior and fails explicitly if resume serialization is requested elsewhere.
`-AdvantageMode raw-return` is the maintained default. The explicit
`leave-one-out` ablation subtracts, for each attempt, the mean return of the
other attempts in that update; it requires at least two attempts and is bound
into trainer provenance. Compare modes on identical training and held-out seed
blocks instead of treating the ablation as an automatic improvement.

`test` requires PyTorch and the installed bridge and runs the complete learning
suite; missing training dependencies are failures, not skips. `verify` runs
that suite and then invokes `bindings/python_learning/verify.ps1` for the fresh
wheel, Rust bridge, smoke, and isolated minimal caller contracts. The lower-level
bridge command intentionally permits optional PyTorch tests to skip and is not
the maintained training-verification entrypoint.

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
