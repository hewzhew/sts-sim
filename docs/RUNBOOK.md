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
| `cargo combat-search` / `.\cs.cmd` | Lightweight combat-search frontend and reusable dedicated worker. |
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

Current continuation is deliberately breaking: `frontier.json` must use
`branch_tiny_frontier_checkpoint_v3` and embeds the source identity that wrote
the exact checkpoint. `--continue-capsule` additionally requires a V5 manifest
whose run contract and source identity match that frontier before the old
trajectory run id is inherited. Earlier frontiers/capsules are not upgraded;
start a fresh capsule or import the exact state through a maintained case
surface.

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

## Atomic Combat Search

This is the fixed-root `AtomicExactV2` diagnostic/challenger surface. It does
not run the production resident `TurnGraphPortfolioV1`; see
[Combat Search Ownership](architecture/combat-search.md) before comparing its
budgets or results with an oracle run.

Use the capability-scoped combat-search frontend for fixed starts, captures,
and benchmark suites:

```powershell
cargo combat-search --help
cargo combat-search --start-spec <path>
```

The frontend owns only parsing and help. `--help` therefore never compiles the
simulator or evaluation backend. A real request builds the optimized
`combat_search_v2_worker` when necessary and then runs it.

For repeated calls against one deliberately frozen build, bypass Cargo:

```powershell
cargo combat-search-build
.\cs.cmd --start-spec <path>
```

`cs.cmd --help` invokes the already-built lightweight frontend. Other
`cs.cmd` requests invoke the already-built worker directly; rebuild once with
`cargo combat-search-build` after source changes. The old
`combat_search_v2_driver` target remains only as a compatibility adapter.

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
  --generation-work 4096 --quantum-generation-work 256 --wall-ms 1000
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
  --generation-work 4096 --quantum-generation-work 256 --wall-ms 1000
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
.\ol-live.cmd live --session seed009 lab search --max-quanta 4 --quantum-generation-work 1024 --wall-ms 1000
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
  --max-quanta 4 --quantum-generation-work 1024 --wall-ms 1000
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
cargo check --workspace --release --all-targets
cargo test -p sts_combat_search_driver --lib
cargo check -p sts_combat_search_driver --features backend --bin combat_search_v2_worker
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
.\learning\dev.ps1 train-combat -Artifact <roots.bin> -Behavior <optional-warm-start-dir> -Output <fresh-dir> -Roots <count> -Updates <count> -CombatLearningRate 0.001 -PotionLane never -CombatPolicyUpdate ppo-clip -CombatAllLossAxis none
.\learning\dev.ps1 train-combat-recovery -Artifact <roots.bin> -SourceExpectedRoots <artifact-root-count> -SourceRootSlot <zero-based-slot> -Behavior <warm-start-dir> -Output <fresh-dir> -Roots 4 -Replicates 8 -Updates 1 -CombatLearningRate 0.001 -PotionLane root-slots -PotionSlots 0 -CombatPolicyUpdate ppo-clip
.\learning\dev.ps1 evaluate-combat -Artifact <held-out-roots.bin> -Behavior <training-dir> -Output <fresh-dir> -Roots <count> -Replicates <count> [-CombatDecisionRule sampled|greedy] [-TraceReplicatesPerRoot 1]
.\learning\dev.ps1 audit-combat-policy -Artifact <roots.bin> -BaselineBehavior <baseline-dir> -CandidateBehavior <candidate-dir> -Output <fresh-dir> -Roots <count> -RootSlot <slot> [-DecisionOrdinals <ordinal-prefix>] -PotionLane never
.\learning\dev.ps1 compare-combat-paired -Artifact <held-out-roots.bin> -BaselineBehavior <baseline-dir> -CandidateBehavior <candidate-dir> -Output <fresh-dir> -Roots <count> -Replicates 2 -BehaviorSeedBase <seed> [-CombatDecisionRule greedy|sampled] -PotionLane never
.\learning\dev.ps1 evaluate-combat-potions -Artifact <held-out-roots.bin> -Behavior <training-dir> -Output <fresh-dir> -Roots <count> -Replicates <count>
.\learning\dev.ps1 evaluate-run -Behavior <training-dir> -Output <fresh-dir> -Ascension <0..20> -Attempts 8 -MaxBatchSteps 4096 -BehaviorSeed 10000 -HeldOutSeedStart 0 -RunPotionLane trained
.\learning\dev.ps1 evaluate-run-potions -Behavior <training-dir> -Output <fresh-dir> -Ascension <0..20> -Attempts 8 -MaxBatchSteps 4096 -BehaviorSeed 10000 -HeldOutSeedStart 0
.\learning\dev.ps1 compare-run-paired -BaselineBehavior <baseline-dir> -CandidateBehavior <candidate-dir> [-StrategicBehavior <shared-strategic-dir>] -Output <fresh-dir> -Ascension <0..20> -Attempts 8 -MaxBatchSteps 4096 -BehaviorSeed 10000 -HeldOutSeedStart 0 -RunPotionLane never
.\learning\dev.ps1 probe-run-critic -Behavior <completed-run-publication-with-scalar-critic> -Output <fresh-dir> -Ascension <0..20> -ProbeTrainAttempts 24 -ProbeHeldOutAttempts 8 -ProbeHeadFitSteps 256 -MaxBatchSteps 32768 -BehaviorSeed 10000 -HeldOutSeedStart 1000000 -RunPotionLane trained
.\learning\dev.ps1 collect-run-roots -Behavior <training-dir-or-combat-anchor> [-StrategicBehavior <shared-strategic-dir>] -Output <fresh.bin> -Ascension <0..20> -Roots 2 -MaxBatchSteps 4096 -WallMs 60000 -BehaviorSeed 120000 -RootSeedStart 10000000 -RootSeedPartition training -RootHeldOutNumerator 1 -RootPartitionDenominator 10 -MinFloor 2 -MinUsablePotions 1 [-CombatFightClass any|ordinary|elite|boss] -RunPotionLane trained
.\learning\dev.ps1 train-run -Behavior <combat-training-dir> -Output <fresh-dir> -Ascension <0..20> -Slots 4 -Generations 1 -AttemptsPerUpdate 32 -MaxBatchSteps 4096 -EvaluationAttempts 16 -HeldOutSeedStart 1000000 -AdvantageMode decision-local-gae -DecisionScope strategic -CombatDecisionRule greedy -SamplingMode independent-cohorts -RunPolicyUpdate ppo-clip-value -RunPotionLane trained
.\learning\dev.ps1 train-run -Behavior <combat-training-dir> -Output <fresh-dir> -Ascension <0..20> -Slots 4 -Generations 1 -AttemptsPerUpdate 8 -MaxBatchSteps 4096 -EvaluationAttempts 4 -HeldOutSeedStart 1000000 -AdvantageMode decision-local-gae -DecisionScope strategic -CombatDecisionRule greedy -SamplingMode independent-cohorts -RunPolicyUpdate critic-calibration -CriticFitSteps 256 -RunPotionLane trained
```

Every learning command that creates a fresh run requires an explicit
`-Ascension`; there is no implicit A0 default. The collector receipt, run
training journal, and run held-out evaluation all retain that value. Combat
training and combat evaluation inherit ascension from each exact opaque root
instead of accepting a second override. Do not infer ascension from an artifact
or directory name.

`train-combat` and `train-combat-recovery` may warm start from either a verified
combat- or run-training publication. Cross-objective handoffs copy only actor
parameters: a run continuation critic is never reused as a combat outcome
critic, and a combat critic is never reused as a run continuation critic.
Combat-to-combat warm starts still require exact model configuration, semantic
schema, behavior rule, and checkpoint tensor compatibility. Historical
model-definition, optimizer, or trainer-provenance digest differences are
recorded as `warm_start_provenance_mismatches` and force an actor-only copy;
they are compatibility evidence, not a resume claim. A fresh optimizer,
destination critic, manifest, and step-zero training lineage are always used.
Both commands bind `-CombatLearningRate` into the destination Adam optimizer
and durable trainer provenance; its default is `0.001`. A smaller value is an
explicit optimizer experiment, not an implicit continuation of the source
optimizer or a change to reward semantics.
`train-combat -Updates 0` publishes the seeded initialization without collecting
experience or applying an optimizer step. Use it for a paired untrained
baseline under the same schema, model seed, evaluation roots, and behavior RNG;
it is not a training result. This initialization-only mode may bind one exact
root for a narrow action-surface audit. Any nonzero update still requires at
least two distinct source roots.

`configure` installs the tool requirements declared by the local
`sts-learning[test]` extra into the selected runtime, then verifies Python,
pytest, NumPy, PyTorch, the repository caller, and the installed bridge. The
caller continues to load from the repository source path, so configuration
does not build or install the local package into its own worktree. No separate
requirements file or global pytest installation is needed.
`test` keeps pytest's small reusable cache below `.oracle-lab/pytest/cache/`
and gives every invocation a fresh `.oracle-lab/pytest/runs/` base temporary
directory. A successful run removes its temporary tree and log immediately; a
failed run preserves the full log below `.oracle-lab/reports/` and its temporary
tree for diagnosis. It does not write a `.pytest_cache` into `learning/` or
depend on access to the process-wide system temporary directory.

Use `check-bridge` for the routine bridge edit loop. It builds a dev-profile
wheel and verifies it through the isolated Python smoke and caller suite in
seconds without changing the configured training runtime. A lightweight
pytest tool environment is cached by Python ABI under ignored `.oracle-lab/`
storage and revalidated against the `sts-learning[test]` requirement on every
run. Each bridge wheel and smoke environment remains fresh and is removed after
a successful check; failures preserve their isolated directory and complete
logs. Optional training dependencies may stay absent from the pytest tool and
their tests may skip. Use
`refresh-bridge` when a real experiment needs the new bridge: it verifies and
installs a release-profile wheel with `--no-deps`, but defers the second Rust
test-binary link. For a functional experiment that does not measure throughput,
`refresh-bridge -BridgeProfile dev` installs the already verified dev-profile
semantics without paying the optimized rebuild; use the default `release`
profile for timing or milestone evidence. Neither command mutates NumPy or
PyTorch. `verify` is the release-profile milestone gate and additionally runs
the Rust bridge contract tests. The standalone bridge workspace explicitly
mirrors the root release ownership profiles: exact environments, model/pool
adapters, and the NumPy bridge are separate optimized units. A model-view edit
therefore rebuilds only the downstream adapter and bridge; it does not
recompile exact episode transitions, run-control, or combat evaluation.
The wheel and release Rust contract test reuse the same ignored Cargo target;
fresh wheel directories and isolated Python environments remain per-run.
`release-final` remains the explicit all-O3/thin-LTO deployment profile and is
not the routine edit or verification path. Ordinary Python-only edits still
use `test`. For first setup, pass
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
To sample potion-bearing later combats from the actual trajectories of one
published frozen behavior, use the typed run collector instead of reconstructing
continuations or probing JSON:

```powershell
.\learning\dev.ps1 collect-run-roots `
  -Behavior <completed-training-directory> `
  -Output <fresh.combat-roots.bin> `
  -Ascension <0..20> `
  -Roots 2 -MaxBatchSteps 4096 -WallMs 60000 `
  -BehaviorSeed 120000 -RootSeedStart 10000000 `
  -RootSeedPartition training `
  -RootHeldOutNumerator 1 -RootPartitionDenominator 10 `
  -MinFloor 2 -MinUsablePotions 1 -RunPotionLane trained
```

The command uses one run slot, advances only seeds in the declared stable
seed-only partition, captures at most one undecoded combat root from each seed,
and keeps concrete inventory inside the opaque checkpoint even when
the behavior's model-facing lane is `never`. Floor and usable-potion filters
are typed root facts. It merges in Rust and writes only after all requested
roots have been collected; an incomplete bound leaves the output absent. The
single-line receipt reports seed/site/resource facts and artifact identity.
`-RequiredPriorCombats` comes from the typed run resource trace, and
`-MaxFloor` is an exact admission bound. These selectors do not currently
abandon an active run after it has passed the bound, so a narrow ceiling can
spend the remaining collection budget advancing an ineligible run. For a small
strict tier, first collect a wider reservoir with the required prior-combat
count, choose only receipt rows at the intended floor, then use
`learning-root select` to publish those opaque slots as a fresh batch. For
example, prior-combat count 1 plus selected floor-2 rows means the captured
combat immediately follows exactly one completed combat without admitting a
long event/shop route. Use direct `-MinFloor 2 -MaxFloor 2` collection only
when that bounded inefficiency is acceptable.
Use `-RootSeedPartition held-out` with the same numerator and denominator for a
disjoint evaluation corpus; do not reconstruct the partition from outcomes.
To collect from the same policy surface as a combat-anchor-only run comparison,
pass the candidate combat publication as `-Behavior` and the fixed strategic
publication as `-StrategicBehavior`. This scoped mode requires greedy combat
decisions and an explicit `-RunPotionLane all|never`; `trained` is rejected
because two publication histories cannot own one implicit lane. The V7 receipt
records the strategic source, combat anchor, and combined collection manifest
identities. Root admission remains seed/context based and does not become an
outcome selector.
Use `-CombatFightClass ordinary` for a general later-combat curriculum before
introducing elite-specific mechanisms. The selector reads the simulator's typed
elite/boss flags; it does not infer class from encounter names. `any` preserves
the trajectory distribution, while `elite` and `boss` remain explicit separate
corpora rather than silently entering an ordinary batch.
Set `-MinUsablePotions 0` to collect ordinary run-derived combat roots without
conditioning the corpus on potion ownership. An exact potion rescue corpus
still supplies both `-RequiredPotionId` and `-RequiredPotionSlot`.
Use `-MinHpPercent <0..100>` when the curriculum question requires viable
entry states; the filter compares exact integer HP against max HP and is
recorded in the receipt. Keep low-HP roots in a separate recovery corpus rather
than silently mixing them into ordinary competence training.
Run-derived curriculum collection defaults to `-CombatDecisionRule greedy`:
argmax applies only while the bridge's typed public run context says the current
row is combat, while strategic rows retain the source categorical temperature
and RNG. The collector records the distinct combined manifest identity. This
prevents combat exploration noise from manufacturing low-HP later roots without
pretending that untrained route or reward decisions have become competent.
Pass `-CombatDecisionRule sampled` explicitly only when the source behavior's
combat exploration distribution is itself the subject of the collection.
Add `-DistinctEncounters` for a small diversity census or training batch; the
collector then admits at most one root for each canonical `EncounterId` and
reports the active contract in its receipt.
For a concrete encounter corpus, pass `-RequiredEncounterId GremlinGang` (or
another canonical ID). The installed bridge normalizes the ID before any run
advances, and the receipt records the normalized contract. Variable member
composition remains part of the exact root rather than the selector identity.
For one shared curriculum with several explicit encounter targets, supply
fixed quotas instead of collecting and merging shards by hand:

```powershell
.\learning\dev.ps1 collect-run-roots `
  -Behavior <completed-training-directory> `
  -Output <fresh.combat-roots.bin> `
  -Ascension <0..20> `
  -EncounterQuota ThreeSentries=4,Lagavulin=4,ExordiumThugs=4 `
  -MaxBatchSteps 4096 -WallMs 60000 `
  -BehaviorSeed 120000 -RootSeedStart 10000000 `
  -MinFloor 6 -MinUsablePotions 0 -RunPotionLane trained
```

The quota total determines the artifact root count; an optional `-Roots` must
match it exactly. Quotas cannot be combined with `-DistinctEncounters` or
`-RequiredEncounterId`. Every admitted root still comes from a distinct seed,
each encounter stops accepting roots when its own quota is full, and any
incomplete quota leaves the output absent. The receipt reports requested and
captured roots for every canonical encounter.
Encounter-aware roots use opaque artifact format version 2. Version 1 roots
fail explicitly as incompatible and must be recollected from their recorded
seed/behavior provenance.
It reports both the requested run potion lane and the resolved combat potion
lane, so `trained` inheritance never has to be inferred from a lone `never` or
`all` value.
Each captured root also lists its earlier completed same-seed combat resource
transitions from the existing typed `RunResourceTrace`: start/end HP and gold,
ordered canonical enemy identities, concrete potion inventories, and terminal
status. The captured combat itself is
excluded even if the final collector step resolves it; later same-seed combats
are also excluded when another slot keeps a multi-root collection alive. This is enough to locate
upstream attrition without retaining action history or decoding the artifact.
Use a fresh path and a small root count first. This is a corpus sampler, not a
trainer or evidence that the collected combats are representative.
For a concrete rescue lane, add both `-RequiredPotionId FirePotion` and
`-RequiredPotionSlot 0`. The installed Rust bridge validates the canonical
potion id before any run advances; each captured root must contain that exact
identity in that exact zero-based slot. Supplying only one selector field is an
error. This aligns the resulting artifact with `train-combat -PotionLane
root-slots -PotionSlots 0` without treating other potion identities as the same
training intervention.
When a combat case has validated exact production context and a typed action
file already replays to a win, derive a bounded reverse-curriculum batch with:

```powershell
cargo oracle-lab learning-root case --artifact <combat-roots.bin> --expected-roots <count> --root-slot <zero-based-slot> --output <fresh.case.json>
cargo oracle-lab learning-root recover --case <case.json> --actions <win.actions.json> --output <fresh.combat-roots.bin> --max-roots 8 --max-bytes 16777216
```

`learning-root case` is the read-only bridge from one validated opaque training
root into the existing bounded combat-search and witness tools. It decodes the
batch once, requires the declared root count and slot, writes a fresh case with
exact production-state replay context, and reports both learning-root and case
identity. It neither samples the policy nor creates action labels.
Rust restores the production session, replays every input, requires a new typed
combat win, retains at most the terminal-nearest requested roots in memory, and
writes the same opaque root-batch format. The compact receipt reports the
actual root count, which can be smaller than `--max-roots` for a short line.
The action sequence is verification evidence only; it is not included in the
artifact and must not be used as a supervised policy label.
Rare contract-selected roots may instead be collected as separate canonical
single-root artifacts and combined without decoding them:

```powershell
cargo oracle-lab learning-root merge --input <first.bin> --input <second.bin> --output <fresh.combat-roots.bin> --max-bytes 16777216
```

With no extra counts, every input must contain exactly one distinct root. To
compose already canonical batches for one small joint frontier/rehearsal
curriculum, declare one count per input in the same order:

```powershell
cargo oracle-lab learning-root merge `
  --input <three-root-rehearsal.bin> --input <twenty-four-root-frontier.bin> `
  --input-roots 3 --input-roots 24 `
  --output <fresh-27-root-curriculum.bin> --max-bytes 16777216
```

Rust requires the declared width of every input, revalidates artifact versions
and exact identities, rejects duplicate roots across inputs, caps the combined
batch at 64 roots, and writes only to a fresh output after all checks pass. The
operation preserves input/root order; it does not select or weight curriculum
roots.
To retain an exact typed subset before composition, select source slots without
decoding the artifact:

```powershell
cargo oracle-lab learning-root select `
  --artifact <source-batch.bin> --expected-roots <source-count> `
  --root-slot <first-slot> --root-slot <second-slot> `
  --output <fresh-subset.bin> --max-bytes 16777216
```

The selected artifact follows caller slot order. Duplicate or out-of-range
slots, source-width mismatch, malformed input, byte overflow, or an existing
output path all fail before publication. Selection is curriculum configuration,
not a policy score or teacher label.

For the combat-search improvement laboratory, first collect natural roots in
declared seed-partition order without encounter or outcome filtering:

```powershell
python -m sts_learning.combat_information_census `
  --output <fresh-census.json> --root-artifact <fresh-roots.bin> --root-count 8 `
  --seed-start <start> --seed-count 8 --ascension <0|20> --partition <training|held_out>
```

To measure the true finite-frame public-history posterior for one exported
root, replay complete production run seeds instead of changing floor RNGs:

```powershell
python -m sts_learning.combat_public_history_chance `
  --census <census.json> --root-slot <slot> `
  --candidate-seed-start <start> --candidate-seed-count 2048 `
  --partition <training|held_out> --output <fresh-scan.json> `
  --root-artifact <fresh-matching-roots.bin>
```

Every candidate must match each captured public snapshot, replay the same typed
public candidate identity, and reach the same combat snapshot. The receipt is
exact only for its declared finite seed frame. A complete source-only result is
a degenerate posterior, not a failure and not permission to manufacture chance
particles.

When that posterior is degenerate, measure cross-root Expert Iteration on an
untouched natural batch:

```powershell
python -m sts_learning.natural_combat_search_census `
  --artifact <roots.bin> --root-count <count> `
  --candidate <experimental-candidate> `
  --oracle-binary target\release\oracle_lab.exe --output-dir <fresh-dir> `
  --solve-work-per-candidate 5000 --candidate-jobs 4
```

This command uses `potion_lane=never` for both the model candidate surface and
the entire successor search. It records one strict proposal only when exact-win
count and then winning final HP exceed the frozen greedy action; equal results
retain the anchor. A published behavior is the initial anchor; an explicitly
unqualified search-distillation candidate may be used as a residual-search
anchor without promoting it. The manifest records the anchor kind, exact
manifest identity, and candidate provenance when applicable. An entrance-only
result is a first-action diagnostic, not a combat-policy result, because its
measured suffix is still supplied by search.

For a source root with a replayable exact-win proposal, expand the verified
winning line into bounded suffix decision roots and independently search every
derived root:

```powershell
python -m sts_learning.combat_search_trajectory_census `
  --artifact <natural-roots.bin> --root-count <count> `
  --search-manifest <natural-search/manifest.json> `
  --root-slot <strict-proposal-slot> [--root-slot <another-slot>] `
  --candidate <experimental-candidate> --oracle-binary target\release\oracle_lab.exe `
  --output-dir <fresh-dir> --max-recovery-roots 8 `
  --solve-work-per-candidate 5000 --candidate-jobs 4
```

The Rust search corpus owns the exact action witness used only to reconstruct
suffix states. `learning-root recover-search` reads it beside the unchanged
opaque source artifact and verifies their root identity, candidate, witness,
and terminal HP before writing the bounded suffix batch. The Python caller
writes neither a `CombatCase` nor an action file. The witness never crosses
into the training target; every suffix root receives a fresh equal-work search
comparison. `combat_search_distillation_spike` may consume several such
recovery artifact/search pairs plus disjoint natural held-out pairs. Strict
proposal rows use cross-entropy; retained rows preserve the complete frozen
baseline distribution through forward KL instead of contributing baseline-top
hard labels. By default it writes no checkpoint. Pass
`--candidate-output <fresh-dir>` only when the
bounded update should be retained as an explicitly unqualified experiment. The
candidate directory contains one exact checkpoint and greedy manifest plus
`candidate.json`; it deliberately contains no production `training.jsonl` and
normal combat behavior recovery rejects it. The command immediately restores
the candidate and requires exact training/held-out logits, complete-combat
greedy action traces, and terminal outcomes to match the live scorer.

Once the training corpus is fixed, candidate creation does not need held-out
search evidence. Use the training-only surface instead:

```powershell
python -m sts_learning.train_combat_search_candidate `
  --training-artifact <recovery-roots.bin> `
  --training-search <suffix-search/manifest.json> `
  [--training-artifact <another.bin> `
   --training-search <another/manifest.json>] `
  --behavior <frozen-source-behavior> `
  --candidate-output <fresh-candidate-dir> `
  --output <fresh-training-result.json>
```

The default is one bounded Adam step at learning rate `3e-4`; larger epoch
counts must be requested explicitly because earlier feasibility runs showed
that later epochs can overwrite unseen attack-vs-attack ordering. The command
restores the written candidate and requires exact training logits and greedy ordinals before it
returns. Its result records proposal cross-entropy and retained forward KL for
every update. It performs no held-out evaluation and makes no teacher claim.

After that parity check, collect a fresh natural root artifact and compare the
reloaded candidate without running successor search:

```powershell
python -m sts_learning.evaluate_combat_search_candidate `
  --artifact <fresh-natural-roots.bin> --roots <count> `
  --baseline-behavior <frozen-source-behavior> `
  --candidate <experimental-candidate-dir> `
  --output <fresh-evaluation.json> --replicates 2 `
  [--max-experience-payload-bytes <explicit-bound>]
```

Both scorers greedily play every complete combat with no search suffix and no
potion actions. The result reports exact root audits, terminal outcomes, and
win-first/final-HP comparisons. Search labels are neither needed nor accepted
for this independent behavior check. The default experience-payload bound is
64 MiB; a larger explicit bound is recorded in the result, and an exhausted
bound reports whether the baseline or candidate failed plus the exact root
slot. The experiment remains
`teacher_valid=false`; PPO remains out of scope until the operator is qualified
over broader decks, relics, potions, encounters, and repeated independent
cohorts.

To discover where that reloaded candidate first changes attack ordering on
several independent natural artifacts, follow only actions on which both frozen
scorers have the same greedy ordinal:

```powershell
python -m sts_learning.combat_policy_divergence_collect `
  --artifact <first-natural-roots.bin> --root-count <first-count> `
  --artifact <second-natural-roots.bin> --root-count <second-count> `
  --baseline-behavior <frozen-source-behavior> `
  --candidate <experimental-candidate-dir> `
  --output-dir <fresh-divergence-dir>
```

At the first differing ordinal, the collector retains only a same-card-profile
different-target choice or two damaging card actions. It records both complete
typed candidate surfaces, logits, ranks, margins, and the shared prefix, then
exports the current boundary directly as one canonical opaque root. A shared
structured-selection flow is identified by its enclosing exact combat root and
typed selection prefix; it is never exported as an invented root. A first
selection, defense, potion, or end-turn divergence rejects that source root
instead of following one policy and searching for a later convenient
disagreement. The merged `divergence-roots.bin` is discovery input for a fresh
equal-work successor search, not a teacher corpus or evidence that the
candidate action is better.

For a bounded fixed-behavior coverage check over every root, use
`CombatWinSignalCensusRunner` with the same `expected_roots`, one shared model
seed, one explicit behavior seed per root, and `max_roots` equal to the intended
census bound. Bind the same verified warm start and potion lane that the
destination trainer will use. It returns compact per-root generations and a
signal census, imports the opaque batch only once, publishes nothing, and is
not a cross-root training scheduler.

`train-combat` performs that fixed-behavior census automatically whenever
`-Updates` is nonzero. It reuses the already validated root source and trains
only mixed-win survival roots plus all-win roots with terminal-HP variation.
Default all-loss roots are recorded in `frontier_rescue_slots`; solved roots are
recorded separately and neither class enters the optimizer. The generation
journal keeps source artifact slot indices even when only one frontier root is
trainable. `-Updates 0` skips census and remains an initialization-only
publication.

For one bounded shared update, construct
`CombatWinBatchSessionFactory` with a `CombatWinBatchSessionConfig` whose
`expected_roots` exactly equals the selected frontier's
`profile.objective.groups_per_update` and does not exceed the separately
declared source `max_roots`. Pass one distinct explicit
behavior seed per root. `advance()` loads no additional artifact, collects every
root under the same frozen manifest, attempts one group-balanced optimizer
delivery, and promotes at most once. `-CombatPolicyUpdate reinforce` (the
compatibility default) performs one exact on-policy step. `ppo-clip` uses the
recorded selection probabilities to apply at most four clipped epochs with
entropy and gradient-norm regularization, stopping before an epoch whose
approximate KL already exceeds the preset target. The generation receipt and
journal report the actual optimizer steps, approximate KL, clip fraction, and
entropy. It writes nothing;
`publish_active_behavior()` is the only durable publication boundary. Treat the
returned loss and promotion as training accounting, not held-out evidence of
improvement.

`-CombatPolicyUpdate ppo-clip-value` selects the separate actor-critic profile.
Its three zero-output value columns have fixed win, future player-HP-change,
and future enemy-HP-change meanings. The typed win-first selector chooses the
matching column for a group; every decision uses its own undiscounted
return-to-go minus its matching pre-update value, without centering residuals
across turns. Actor advantages are frozen across PPO epochs and the journal
reports `value_loss`. It may warm-start shared actor weights from a
policy-only or differently shaped publication, but publishes a distinct model
identity. Keep it opt-in until held-out evidence justifies making it the
maintained default.

`train-combat-recovery` selects one explicit canonical root from an artifact;
`-SourceExpectedRoots` validates the artifact width and `-SourceRootSlot`
selects the zero-based root without copying or extracting it. Under one frozen
behavior it samples the requested
replicates, chooses the verified win with highest final HP (lowest replicate
index breaks ties), exactly replays its recorded ordinals, and derives the
requested number of terminal-nearest roots. A shorter win or an all-loss source
fails before training. It then performs ordinary group-balanced on-policy
updates over those immutable suffix roots and publishes the result in the same
recoverable combat-training layout. `-Roots` is the exact suffix width and
`-Replicates` applies independently to source discovery and every suffix group.
The declared potion lane is reapplied to discovery, replay, and derived groups;
a Fire-only source therefore cannot silently become an unrestricted-potion
curriculum. The replay ordinals establish provenance only and never become
supervised action targets.

The batch and recovery sessions bind `PotionLane all|never|root-slots` into
their root sources.
Prefer `never` when the selected roots already have no-potion winning coverage,
so the all-win terminal-HP axis cannot learn to burn inventory for local HP. An
all-loss `never` group stays no-signal by default. The explicit
`-CombatAllLossAxis enemy-hp-progress` mode selects varying enemy-HP progress
only when every replicate is an exact loss; Smoke Bomb escapes and all other
unresolved terminals remain ineligible. Use that mode as a separately
provenanced bounded damage-support experiment, not as a victory claim or an
HP/potion price. A concrete-potion rescue question still moves to `root-slots`
with one `-PotionSlots <zero-based-slot>` at a time instead of reopening
unrestricted `all`; the selected slot binds its exact root potion UUID, and a
later replacement in that slot remains unavailable.

Use `evaluate-combat` on a distinct opaque root artifact after publication. The
behavior directory may be an exact completed combat- or run-training output
containing its bounded durable behavior stores. A combat publication owns one
checkpoint and manifest; an anchored run publication additionally owns its
immutable combat anchor. The evaluator verifies complete provenance and records
the training kind. For a combat-trained source
it also recovers the training artifact digest and rejects an evaluation artifact
with the same digest before constructing combat groups; a run-trained source
instead records its run objective, sampling contract, and combat-anchor
identity. Combat evaluation output V16 carries those anchor fields explicitly,
adds the root-by-replicate policy RNG seed matrix, and retains `root_audits`
read directly from every opaque evaluation root. Each
audit records seed, act/floor, ascension, encounter and ordered monsters, entry
HP, canonical card/upgrade counts, relics, and potion slots; filenames are not
accepted as root identity.
It gives every root and replicate an independent explicit behavior RNG stream and, by
default, writes only `evaluation.json` with per-replicate win, HP/max HP, gold, turn,
concrete starting/final/lost/gained potion identities, potion use/discard
counts, and card facts plus compact aggregates. Each root retains its seed,
canonical encounter identity, and ordered monster identities; the top-level
`encounters` rows aggregate wins, losses, HP loss, enemy HP, and potion actions
without inventing a scalar score. Use these rows as the fixed-cohort combat
school view before interpreting a whole-run floor metric. The completion line includes each root's
site and starting potion identities, so routine resource-loss classification
does not require reopening the JSON. Identity loss is an inventory fact, not a
static potion-value score; strategic retained value requires an exact run
continuation outside this evaluator. It creates no optimizer, trainer,
experience buffer, or behavior promotion. A result measures the exact frozen
manifest on that bounded sample; it does not establish improvement without a
comparable baseline using the same roots and RNG streams.

The default `-CombatDecisionRule sampled` evaluates the exact published
temperature-scaled behavior and retains its independent RNG streams. Use
`-CombatDecisionRule greedy` only as a paired ranking diagnostic. It derives a
separate in-memory behavior manifest over the effective immutable combat
checkpoint with the explicit greedy rule. For an anchored run publication this
is the combat anchor, not the promoted strategic scorer. The result records both
source and evaluation manifest ids and uses deterministic selection
probabilities. It does not relabel the published sampled behavior or silently
promote a deployment policy.

For one bounded diagnosis, pass `-TraceReplicatesPerRoot <count>`. The count
cannot exceed `-Replicates`; zero is the default. The evaluator then writes
`combat-traces.jsonl` with one compact pre-action row whenever those selected
replicates finish symbolic action decoding. Each row retains root and replicate
identity, turn, energy, HP/block, player and monster powers, hand and pile
counts, potions, monster intent, the decoded action, model round, selected
ordinal, and selection probability. Trace schema V2 adds those power rows so
Artifact, Vulnerable, Strength, and similar state cannot disappear from manual
diagnosis.
`evaluation.json` keeps only the sidecar schema, filename, record count, and
bound, so ordinary aggregate reads never ingest the trace. Treat it as
diagnostic evidence, not training experience or an action-quality label.

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
published combat- or run-trained scorer, creates a fresh held-out seed population, disables
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
lanes finish the exact same ordered episode seeds, retains both complete V10
evaluations, and writes a compact per-seed `potion-comparison.json`. A combat-
trained scorer has not thereby learned non-combat strategy; the command is a
bounded end-to-end diagnostic of that complete policy surface, not a claim that
all decisions were trained.
Use `compare-run-paired` for two different frozen publications. It runs the
same ordinary one-slot evaluator on both sides under one held-out seed prefix,
initial policy RNG seed, ascension, potion action surface, terminal target, and
step bound. The command rejects mismatched executed model, behavior-rule, or
semantic-schema identities, incomplete sides, and different terminal seed
sets. Without `-StrategicBehavior`, this is a full-behavior comparison: both
combat and strategic rows use their publication's scorer. With
`-StrategicBehavior <shared-strategic-dir>`, the baseline and candidate
directories instead name distinct combat anchors. Combat rows select greedily
from the corresponding anchor, while route, reward, shop, event, and other
strategic rows remain sampled from the same recovered strategic publication.
The scoped mode verifies matching model/configuration, semantic schema, and
categorical rule identities; it rejects a shared anchor or a changed strategic
source.

The command retains both V10 evaluations and writes a per-seed
`paired-comparison.json` with win transitions and separate act/floor, HP, gold,
combat-count, and potion facts. `-RunPotionLane never` is the default for this
comparison. The policy RNG contract is the same initial stream on each side;
after action paths diverge, subsequent draws may be consumed at different
decisions and are not claimed as stepwise shared randomness. In the scoped
mode this statement applies only to strategic sampling: greedy combat actions
consume no policy RNG. The scope and strategic/combat manifest identities are
recorded in both the V10 evaluations and the V2 paired contract.
Run-trained publications are recovered directly from their completed V2
training journal and durable behavior stores, so a new held-out seed block does
not require repeating the optimizer update. Evaluation output V10 records
whether the source behavior was combat- or run-trained, includes the run
objective when applicable, records the immutable combat-anchor provenance for
anchored run policies, and preserves each combat's typed encounter and monster
identities for encounter-level resource analysis.
Use `train-run` for the first whole-run on-policy handoff. `-Behavior` is a
verified completed `train-combat` directory whose scorer becomes an independent
generation-zero parameter copy. `-Device cpu|cuda` selects the scorer and
categorical RNG device; the default is `cpu`. An unavailable CUDA runtime fails
before session setup, and the selected device is retained in the journal and
publication. Training uses the `TRAINING` seed partition,
the terminal floor-progress return, and exactly
`-AttemptsPerUpdate` complete runs per generation. The attempt count must be a
multiple of `-Slots`: faster slots park at terminal until the complete slot
cohort finishes, and the next cohort is reset only before another old-behavior
cohort or after promotion to the new behavior. No run may cross a behavior
manifest boundary. This default `independent-cohorts` mode uses zero recovery.
Each generation reports its
own terminal-floor histogram. The command publishes only after every requested
generation completes, but publishes only the frozen behavior checkpoint and
manifest—not optimizer or environment resume state. It then evaluates the
result on the disjoint `HELD_OUT` partition and writes a compact `summary.json`.
Each generation journal row also records a bounded credit diagnostic: current
terminal-broadcast decision targets beside decision-local remaining-floor
targets and their matched-floor leave-one-out advantages, including sign counts
and per-decision-floor, combat/strategic scope, and typed strategic-context
aggregates. Context rows include their strategic-scope attempt-equal weight and a
non-authoritative floor-plus-context leave-one-out comparison; unsupported
context groups remain zero. The overall diagnostic also reports an episode-plus-
floor-plus-context comparison. This is a target-distribution comparison only;
it never selects the optimizer target. REINFORCE uses its configured terminal
advantage, while value PPO uses the decision-local target described below. The
diagnostic does not price HP, gold, or potions.
A generation that
hits `-MaxBatchSteps` before an optimizer step fails without publishing its
partial live update.
The default `-RunPotionLane trained` preserves the warm-start behavior's combat
potion candidate surface for both training and held-out evaluation. This avoids
injecting untrained potion actions at the handoff. No-potion run sessions do
not yet support cross-process resume; this command publishes only the frozen
behavior and fails explicitly if resume serialization is requested elsewhere.
`-AdvantageMode auto` is the command default: it resolves to `raw-return` for
REINFORCE and `decision-local-gae` for value PPO. The explicit
`leave-one-out` ablation subtracts, for each attempt, the mean return of the
other attempts in that update; it requires at least two attempts and is bound
into trainer provenance. Compare modes on identical training and held-out seed
blocks instead of treating the ablation as an automatic improvement. The
`matched-floor` ablation instead uses the recorded decision-time floor and
centers each remaining-progress target only against other attempts that reached
that floor. A floor reached by one attempt contributes zero advantage. It also
requires at least two attempts, is bound into trainer provenance, and must be
evaluated as a separate fresh behavior.
`matched-floor-context` tightens the same comparison to decision floor plus the
typed combat/strategic context. It therefore gives zero advantage to a context
observed in only one attempt instead of borrowing signal from a different site
on the same floor. It is separately bound into trainer provenance and remains
an ablation, not the maintained default.
`-SamplingMode episode-root-retries` is a single-slot training ablation. It
restores the exact episode root after each defeat up to the explicit
`-EpisodeRootAttempts` cap; this allows one update to cover multiple roots.
Victories finish normally, and the boundary defeat is completed so no live
episode crosses promotion. It requires
`-AdvantageMode matched-episode-floor-context`, which compares only retries from
the same episode seed and generation at the same floor and typed context. The
training recovery budget is derived as `EpisodeRootAttempts - 1`; held-out
evaluation still uses zero recovery. Treat the cap and recovery count as
sampling provenance, not reward or a competence metric.
`-DecisionScope all` and `-CombatDecisionRule sampled` remain the compatibility
defaults. The explicit `strategic` scope removes combat-boundary rows from the
whole-run loss and renormalizes each attempt over its remaining strategic
decisions; it does not erase combat actions or their state transitions from
complete-attempt evidence. Pairing it with `-CombatDecisionRule greedy` uses
the verified warm-start scorer as an immutable combat argmax anchor while the
separate run scorer supplies categorical strategic choices. That pairing has
its own behavior manifest bound to the anchor manifest identity, records combat
propensity as `1.0`, records the real strategic propensity, and keeps the same
anchor across every strategic promotion. Publication copies both the anchor
and the final strategic scorer into the run directory; durable recovery rejects
missing or mismatched anchor manifest, checkpoint, or scorer configuration.
`greedy` is rejected with the `all` scope because categorical PPO cannot claim
deterministic combat choices as on-policy samples. Compare the explicit pair on
the same training and held-out seed blocks.
`-RunPolicyUpdate reinforce` preserves the compatibility whole-run update and
remains the default. The opt-in `ppo-clip-value` profile adds a zero-initialized
value head and requires `-AdvantageMode decision-local-gae`. It predicts the
typed continuation return after each decision, freezes attempt-equal normalized
GAE advantages and pre-update value predictions, and applies at most four PPO
epochs with separate actor/value clipping, target-KL stop, entropy
regularization, and gradient clipping. Forced single-candidate rows train the
critic but contribute no actor, entropy, KL, or normalization weight. The
training journal records optimizer epochs, KL, actor/value clip fractions,
entropy, value loss, gradient norm, and attempt-weighted explained variance.
Its per-generation `rollout_value_diagnostics` identifies the active target as
`decision_local_return_to_go`; there is no terminal-broadcast shadow optimizer.
The journal reports eligible actor residuals both before and after advantage
normalization, whether normalization ran, and how many directions changed sign
(including each positive-from-nonpositive and negative-from-nonnegative case).
These are signal-formation facts, not automatic rejection criteria. Raw sign
counts remain distinguishable from attempt-equal sign weight and weighted
moments, so a long attempt cannot silently dominate the diagnostic.
`-RunAdvantageNormalization auto` preserves the selected update profile. For a
single-variable value-PPO ablation, `on` or `off` explicitly selects global
attempt-weighted advantage normalization and is retained in trainer provenance
and the journal. `on` is rejected for REINFORCE. This switch changes the policy
gradient estimator, not the environment return or critic target; compare it on
identical frozen training cohorts and disjoint held-out cohorts.
`-RunPolicyUpdate critic-calibration` is the behavior-neutral value warmup. It
uses the same decision-local return-to-go target but applies only scalar value
loss: every shared semantic encoder and actor tensor is frozen, actor decision
count and trained-decision count stay zero, and the publication receives a
distinct trainer identity. The maintained CLI profile uses one fixed complete-
attempt cohort; `-CriticFitSteps` defaults to 256 supervised updates with value-
loss coefficient `1.0`, no value clipping, and no finite gradient clipping.
Values above 1024 are rejected. The journal's legacy `run_policy_epochs` and
optimizer-step fields therefore mean critic fit steps for this profile; actor
clip, entropy, and KL fields are inert. It is calibration evidence, not a
policy-improvement claim. A later `ppo-clip-value` run may name it explicitly
with `-CriticInitializationBehavior`; that run still names the original combat
publication with `-Behavior`, verifies identical actor tensors plus matching
ascension, potion lane, decision scope, combat rule and immutable combat anchor,
then collects a fresh actor cohort under its own seeds and behavior RNG. No
calibration attempt is reused as PPO actor experience.
`probe-run-critic` is the non-publishing learnability diagnostic for that
boundary. It collects one fixed no-recovery `HELD_OUT` cohort under the immutable
published behavior, then splits its ordered complete attempts into probe-train
and probe-held-out partitions without shuffling decision rows. It compares a
constant predictor, a direct public run-feature ridge baseline, the published
scalar critic, and a fresh head-only capacity fit over the unchanged actor
encoder.
The head-only fit may deliberately reuse its fixed training trajectories for
the configured supervised steps because it never becomes behavior and writes
no checkpoint. `challenge.json` reports attempt-equal train/held-out MSE,
explained variance, prediction spread, and attempt-pair-equal concordance after
collapsing repeated decisions within each attempt/floor/context group. Prediction
ties contribute `0.5` to that concordance instead of being called discordant.
It is a representation/optimization diagnostic, not an actor update or a
capability publication.
It also aggregates the first four completed
combats by ordinal:
net post-combat HP already includes relic recovery such as Burning Blood, so
these rows diagnose premature low-HP state occupancy without inventing an HP
reward or declaring one combat result sufficient evidence of improvement.
Missing or malformed decision-time progress fails value PPO before optimizer
mutation; the caller never reconstructs rollout floors from semantic tensors.
Whole-run publication schema V6 is intentionally incompatible with V5, whose
strategic-only update still shared one mutable scorer with combat argmax. V6
binds the immutable combat anchor in both journal boundaries and durable local
stores. V5 had already replaced V4 to record the combat/strategic selection
rule; V4 replaced V3 to require explicit ascension provenance, and V3 replaced
the older terminal-broadcast value-PPO contract.

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
