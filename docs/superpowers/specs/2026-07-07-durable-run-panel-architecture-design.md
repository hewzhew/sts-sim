# Durable Run Panel Architecture Design

## Status

Implementation-ready review draft. This document defines the intended
architecture before implementation. It does not change runner behavior by
itself.

## Implementation Progress

Current implementation has established the first durable panel path:

- `branch_panel panel inspect` reads existing capsule artifacts and writes a
  structured `panel_summary.json`.
- `branch_panel panel smoke` runs in-process through the Rust runtime facade;
  it does not shell out to `branch_tiny`.
- `branch_panel panel continue` advances only compatible soft-paused capsules;
  it does not start missing capsules.
- `branch_panel panel drain` runs bounded repeated slices for longer
  continuation experiments.
- `panel_summary.json` now records run mode, max slices, row status, reuse
  decision, scheduler action, artifact facts including capsule ledger
  presence, and tool errors.
- `panel_ledger.jsonl` records each executed/skipped/failed slice with run
  mode and slice index.
- `capsule_ledger.jsonl` records capsule `slice_started` and
  `slice_finished` events with stop kind and typed `ArtifactRef` entries, so a
  capsule has its own execution history separate from scheduler decisions.
- `--slice-ms` is the preferred panel deadline option; legacy `--wall-ms`
  remains accepted for compatibility.
- `--fresh` archives an existing seed capsule under `_archive/` before
  starting a replacement run, and the summary records
  `fresh_replaced_capsule` plus the archived capsule path.
- `branch_panel panel compare` materializes named search profiles under
  `_compare/<profile>/<seed>` and writes one combined comparison summary. V0
  supports `baseline` and `double-search`.
- `branch_tiny` is now a thin CLI adapter over `OwnerAuditRuntime::run_cli`.
- `branch_tiny --continue-capsule` runs continuation slices in-process through
  `BranchRuntime`; it no longer spawns nested `branch_tiny` child processes.
- Owner-audit implementation files now live under
  `src/runtime/branch/owner_audit/`; the runtime facade no longer imports
  implementation modules from `src/bin/branch_tiny`.
- `BranchArtifactStore` owns panel seed artifact presence reads; panel
  resolution consumes typed artifact facts instead of reading capsule files
  directly, including `capsule_ledger.jsonl` presence.
- `panel.rs` no longer knows concrete capsule artifact file names in
  production code; capsule file-layout reads are routed through
  `BranchArtifactStore`.
- `RunSliceResult` now carries an `ArtifactWriteSummary` for core capsule
  writes observed by the slice path, so in-process callers no longer need to
  infer manifest/frontier/result/summary writes from the filesystem.
- `ArtifactWriteSummary` now includes typed `ArtifactRef` entries for observed
  capsule writes, including kind, path, schema, and creator metadata.
- `panel_ledger.jsonl` now records `ArtifactRef` entries from executed
  `RunSliceResult` values, so ledger rows no longer rely only on capsule file
  existence checks.
- `panel_summary.json` rows now also carry executed slice `ArtifactRef`
  entries, keeping the human-facing summary tied to the typed runtime result.
- Capsule JSON/path persistence for owner-audit runs is now isolated in
  `capsule_artifact_store.rs`; `run_capsule.rs` is a runtime handle that
  delegates concrete filesystem writes to that adapter.
- `run_persistence.rs` is now limited to recovery/soft-wall persistence;
  objective completion artifact handling lives with branch observation instead
  of the recovery helper.
- `run_loop.rs` now delegates `RunSliceResult` construction to
  `owner_audit/run_slice_result.rs`, keeping slice result assembly out of the
  main expansion loop.
- `run_loop.rs` now delegates capsule result persistence to
  `RunStopRecorder`, so the loop no longer directly writes result artifacts or
  formats capsule result output.
- `tools/gap_panel.py` is now a deprecated compatibility wrapper over
  `branch_panel`; it no longer owns seed deletion, continuation, or
  `branch_tiny` process orchestration.

Still open:

- `--fresh --discard-old` explicit destructive replacement, if it is still
  wanted.
- richer named policy/search config comparison beyond the current
  `baseline` / `double-search` V0.
- completing the capsule artifact store boundary with a more public store
  facade; concrete owner-audit capsule writes have been moved out of
  `run_capsule.rs`, capsule ledgers are emitted by the store adapter, panel
  capsule reads are routed through `BranchArtifactStore`, and panel
  ledger/summary rows now consume runtime artifact refs.
- narrowing the remaining owner-audit facade surface so persistence and
  run-slice result construction continue to stay outside owner/search
  internals.

## Problem

`tools/gap_panel.py` currently behaves like a convenience wrapper:

- build `branch_tiny`,
- delete each seed capsule,
- run each seed from Neow,
- optionally continue once after `wall_deadline`,
- collect `summary.json`.

`branch_tiny --continue-capsule` previously used a transitional shape: it
started another `branch_tiny` process for each continuation slice. That has
been removed. The remaining transitional shape is internal layering:
owner-audit implementation files now live under runtime code, but persistence,
run-slice construction, owner policy, and combat portfolio internals still sit
inside the same owner-audit implementation subtree.

That was useful while owner coverage was the main problem, but it is now the
wrong control surface. Five-seed panels take minutes, often redo already known
work, and blur three different concepts:

- a real blocker, such as combat gap or owner gap,
- a normal soft pause with saved frontier,
- a process or tool failure.

The result is not just slow. It makes investigation semantics depend on how a
script happened to be called.

## Design Goal

Turn the panel from a rerun script into a durable in-process run scheduler.

```text
seed/config/code identity
  -> reusable run capsule
  -> short resumable slices
  -> Rust BranchRuntime
  -> Rust PanelScheduler
  -> typed stop classification
```

`wall_ms` remains useful, but only as a slice-level soft deadline. It must not
be the meaning of an experiment.

The final target is not:

```text
Python -> branch_tiny.exe -> files -> Python summary parsing
```

and not:

```text
Rust panel -> branch_tiny.exe child process -> files -> Rust summary parsing
```

The target is:

```text
Rust PanelScheduler -> Rust BranchRuntime -> typed RunSliceResult
                                      \-> ArtifactStore persists capsule
```

The closest mature analogs are:

- durable workflow engines: event history plus deterministic replay,
- data/workflow systems: task identity plus work avoidance,
- build caches: strict keys, partial restore, and cache-hit visibility,
- experiment trackers: runs with params, metrics, artifacts, and resume ids,
- HPC schedulers: wall-time signal, checkpoint, requeue,
- anytime/contract algorithms: bounded slices with meaningful partial results.

The project should not copy any one of these systems. It should copy their
separation of concerns.

## Non-Goals

- No new reward, shop, event, or combat strategy.
- No ML training or vector model.
- No HTML dashboard.
- No large human report system.
- No attempt to make five seeds statistically meaningful.
- No hidden reruns from Neow unless explicitly requested.
- No Python ownership of run compatibility, resume, reuse, or stop semantics.
- No new child-process continuation path as the target architecture.

## Core Concepts

### Ownership Rule

Runtime semantics live in Rust.

```text
Rust owns:
  run identity, compatibility, slice execution, resume, stop classes,
  artifact persistence, panel scheduling, and ledger semantics.

Python owns:
  offline analysis, ML training, plotting, notebooks, batch post-processing,
  and optional wrapper convenience.
```

Python tools may consume Rust artifacts. They must not decide whether a capsule
is reusable, whether a frontier is resumable, or what stop class a slice
produced.

### Branch Runtime

`BranchRuntime` is the reusable Rust API that replaces `branch_tiny` as the
semantic owner of a run slice.

It should expose typed operations like:

```text
start_slice(RunSliceRequest) -> RunSliceResult
continue_slice(ContinueSliceRequest) -> RunSliceResult
```

It owns:

- creating the initial run-control session,
- loading a frontier from an artifact store,
- advancing one bounded slice,
- applying soft-stop and generation contracts,
- returning a typed stop,
- asking the artifact store to persist state.

It must not parse CLI args, print human reports, or spawn `branch_tiny` child
processes.

### CLI Adapters

CLI binaries are adapters over Rust runtime APIs:

```text
branch_tiny:
  parse one-run CLI args -> BranchRuntime

branch_panel:
  parse panel CLI args -> PanelScheduler -> BranchRuntime
```

CLI output is for humans. It is not the control surface between scheduler and
runner.

### Panel Run

A panel run is the experiment object. It is not just a folder containing seed
subdirectories.

It owns:

- panel id,
- panel mode,
- requested run identities,
- scheduler options,
- slice ledger,
- final panel summary.

This mirrors experiment managers where a run/study is the stable object and
individual artifacts are attached to it. Without this object, a "5-seed panel"
is too vague to compare across days.

### Panel Mode

Every panel invocation must declare its intent. The scheduler should not infer
intent from incidental flags.

```text
smoke:
  Touch each seed briefly and classify immediate blockers. Low total budget.

continue:
  Reuse existing capsules and advance soft-paused seeds. No fresh rerun unless
  explicitly requested.

drain:
  Keep slicing until every seed reaches a real stop or a panel budget ends.

compare:
  Run the same compatible capsule set under named policy/search configs and
  write rows that can be compared.
```

The current manual panel is closest to `smoke`, but recent usage has drifted
toward `continue` and `drain`. Naming the mode prevents accidental misuse.

### Run Capsule

A run capsule is the durable state container for one run identity.

It owns:

- exact resume frontier,
- manifest and run contract,
- path and terminal artifacts,
- combat case sidecars,
- cheap `summary.json` projection.

It is not a strategy report. It is the state that lets tools continue or inspect
the run without replaying from Neow.

### Capsule Ledger

The capsule needs an append-only ledger in addition to summary projections.

The ledger records:

- start and continue slice attempts,
- request kind and runtime result,
- identity checks,
- compatibility decisions,
- artifact writes,
- stop-class transitions.

`summary.json` is allowed to be overwritten because it is a projection.
The ledger is not overwritten. This follows the durable-workflow lesson:
history is the source of recovery truth; summaries are cheap views.

### Run Identity

Panel reuse needs a stable identity:

```text
seed
class
ascension
objective
max_branches
generations
runner contract version
policy/search config fingerprint
binary/source fingerprint
```

V1 can begin with a conservative subset, but the architecture target is clear:
a capsule is reusable only when its identity matches the requested run
contract. If identity is missing or mismatched, the scheduler must mark it
`stale_or_incompatible` unless the user explicitly asks for `--fresh`.

### Run Slice

A run slice is one bounded call into `BranchRuntime` against a capsule.

It may:

- start a new capsule,
- resume an existing frontier,
- produce a real stop,
- produce another soft pause,
- fail as a process/tool error.

Each slice records:

```text
slice_index
started_at
finished_at
elapsed_ms
request_kind = start | continue
before_state
after_state
```

The slice budget is a normal scheduling knob. It should usually be short enough
to keep the panel responsive, not long enough to hide progress for minutes.

### Panel Scheduler

The panel scheduler owns experiment orchestration across seeds. It does not own
game policy.

It should:

- resolve or create one capsule per run identity,
- skip capsules that already reached a real stop,
- continue soft-paused capsules in short slices,
- use round-robin scheduling when multiple seeds remain live,
- stop when the panel budget or all real stops are reached,
- write a small structured panel summary.

It should not:

- delete capsules by default,
- parse human prose,
- parse `branch_tiny` stdout/stderr,
- classify a run from process exit code when a typed `RunSliceResult` exists,
- infer why a deck is bad,
- decide strategy from blocker distribution,
- silently hide process failures.

### Materialized Artifacts

Panel artifacts should be treated like materialized assets:

```text
identity + command contract + source fingerprint -> capsule artifacts
```

If the identity is unchanged, work can be avoided. If any dependency changes,
the artifact is stale. This is stricter than "path exists" and less wasteful
than "always rerun".

## Stop Classes

Panel rows should separate stop reason from artifact availability.

```text
real_stop:
  terminal
  combat_gap
  automation_gap
  event_gap
  run_choice_gap
  owner_gap
  advance_failed

soft_pause:
  wall_deadline_with_frontier
  slice_budget_with_frontier

tool_failure:
  runtime_error
  artifact_write_failed
  artifact_read_failed
  missing_summary
  malformed_capsule
  stale_or_incompatible
```

`frontier_saved` is not a stop kind. It is an artifact fact attached to a stop
or pause.

## Source And Identity Fingerprints

Identity should be split into visible fields instead of one opaque hash:

```text
game_identity:
  seed, class, ascension

runner_contract:
  objective, max_branches, generations, slice_ms, panel_mode

policy_identity:
  owner policy version, reward/shop/acquisition policy version,
  combat portfolio/profile version

source_identity:
  git commit, dirty flag, optional dirty tree hash, branch_tiny binary mtime
```

The panel can start with a smaller implementation, but the design target is
fielded identity. Opaque hashes are useful for equality checks; fielded identity
is useful for review.

### Identity Strictness

The scheduler should distinguish exact identity matches from weaker matches.

```text
identity_match = exact
               | compatible_partial
               | incompatible
               | unknown
```

Only `exact` may silently reuse a real stop. `compatible_partial` may be used
for inspection or explicit continuation, but the row must show that it was not
a strict cache hit. This follows cache systems where a fallback/restore-key hit
is useful but not equivalent to the primary key.

Initial strictness should be conservative:

```text
exact:
  game identity, runner contract, policy/search versions, and source identity
  all match.

compatible_partial:
  game identity matches and the capsule has a valid frontier, but one
  non-state-changing display/report field changed.

incompatible:
  game identity differs, policy/search behavior differs, source is newer, or
  artifact schema is not understood.

unknown:
  old capsule lacks enough identity fields.
```

For V1, `unknown` should not be silently reused. It can be continued only under
an explicit compatibility flag or after a one-time migration stamps enough
identity into the capsule.

### Input-Addressed, Not Output-Addressed

Run identity should be input-addressed:

```text
requested run contract + policy/search/source identity -> reusable capsule
```

It should not be output-addressed:

```text
capsule happened to reach A3 or victory -> reuse
```

This matters because two different policies can both win while producing
different evidence. The panel must preserve which policy produced the result.

## Budget Model

Four budgets must stay separate:

```text
combat/search budget:
  How much a single combat search or portfolio attempt may spend.

run slice budget:
  How long one branch_tiny invocation may run before saving frontier.

panel budget:
  How many slices or how much total elapsed time the panel may spend.

outer hard timeout:
  Last-resort process safety fuse, not normal control flow.
```

The current name `--wall-ms` is acceptable for backward compatibility, but the
panel should present it as `slice_ms` in summaries and documentation.

## Capsule Reuse Rules

Default behavior should be reuse-first:

1. If a compatible capsule has a real stop, read it and do not rerun.
2. If a compatible capsule has `wall_deadline` and a frontier, continue it.
3. If a compatible capsule is running/stale/incomplete but resumable, continue
   only when the state is structurally valid.
4. If a capsule is missing, create it.
5. If a capsule is incompatible, mark it instead of deleting it.

Explicit rerun behavior should require:

```text
--fresh
```

`--fresh` may archive or replace the old capsule, but that must be visible in
the panel summary.

Reuse must be an explicit scheduler decision:

```text
reuse_decision = reused_real_stop
               | continued_soft_pause
               | created_new_capsule
               | rejected_stale_capsule
               | fresh_replaced_capsule
               | inspected_partial_match
               | rejected_unknown_identity
```

This field matters because "not rerun" can be either a correct cache hit or a
bug that hid stale data.

### Fresh And Archive Semantics

`--fresh` should not silently delete prior evidence. The preferred behavior is:

```text
--fresh:
  move existing capsule to an archive directory with timestamp and old identity,
  then create a new capsule.

--fresh --discard-old:
  delete old capsule after writing a short tombstone row in the panel ledger.
```

The default should be archive, not delete. Deletion is acceptable only when the
caller explicitly asks for disposable output.

## Panel Output Contract

`panel_summary.json` should be the stable interface for callers.

Each row should include:

```text
profile
seed
capsule_path
run_identity
identity_status
stop_class
blocker_kind
act
floor
hp
max_hp
gold
deck_size
subject
frontier_exists
terminal_exists
result_exists
slice_count
total_elapsed_ms
last_slice_elapsed_ms
last_boundary
last_owner
next_recommended_command
next_recommended_reason
```

`next_recommended_command` is allowed only as a mechanical diagnostic pointer,
for example a `combat_case_review` command for a combat gap. It must not become
a strategy recommendation system.

The panel summary should include aggregate counts, but only over typed fields:

```text
counts_by_stop_class
counts_by_blocker_kind
counts_by_reuse_decision
total_elapsed_ms
total_slices
```

Do not add prose conclusions such as "reward is the problem" to the panel
summary. A later analysis tool may interpret rows, but the scheduler should not.

## Ledger Placement

There should be two ledgers with different scopes.

### Panel Ledger

`panel_ledger.jsonl` records scheduler decisions:

```text
panel_started
build_started
build_finished
seed_identity_resolved
capsule_reuse_decision
slice_scheduled
slice_finished
row_refreshed
panel_finished
```

It answers:

```text
Why did this panel run, skip, continue, or reject a seed?
```

### Capsule Ledger

`capsule_ledger.jsonl` records one capsule's execution history:

```text
capsule_created
slice_started
manifest_written
frontier_written
result_written
terminal_written
summary_written
slice_finished
```

It answers:

```text
What happened to this run identity over time?
```

Neither ledger should contain large path snapshots, full decks, candidate pools,
or combat traces. They should reference existing artifacts by path and schema.
This prevents ledgers from becoming another report system.

### Summary Is A Projection

`summary.json` is derived from the latest valid capsule state plus selected
ledger facts. Tools may read it for convenience. Tools must not assume it is
the recovery source of truth.

If a summary disagrees with ledger/artifacts, the repair path is:

```text
read ledger/artifacts -> regenerate summary
```

not:

```text
edit summary by hand
```

## CLI Shape

The future panel command should call a Rust scheduler, not a Python scheduler:

```powershell
cargo run --bin branch_panel -- panel run `
  --seeds 1552225671..1552225675 `
  --capsule-root tools/artifacts/gap_panels/current `
  --mode continue `
  --slice-ms 15000 `
  --max-slices 8 `
  --max-active 1
```

Compatibility:

- keep `branch_tiny --wall-ms` as an alias for `--slice-ms` during migration,
- keep `branch_tiny --continue-capsule` temporarily, but implement it through
  `BranchRuntime`, not by spawning a child process,
- keep `tools/gap_panel.py` only as a temporary wrapper or retire it,
- add `--fresh` for explicit reruns,
- add `--mode smoke|continue|drain|compare`,
- reject ambiguous aliases such as `--output-root` unless intentionally added
  as documented compatibility.

## Scheduling Policy

Default scheduling should be deterministic and boring:

```text
for slice_round in 0..max_slices:
  for seed in seed_order:
    if seed has real_stop:
      skip
    if seed has tool_failure:
      skip unless retry requested
    call BranchRuntime for one slice
    persist artifacts through ArtifactStore
    refresh row from RunSliceResult and capsule projection
```

This gives each seed a chance to advance without one slow seed hiding all other
results. `--max-active` can remain `1` until the runner and artifacts are
stable enough for parallel runs.

The scheduler should prefer fairness over depth by default. A panel that spends
all time on one seed before touching the others is a campaign run, not a panel.

## Error Handling

The scheduler must be strict about tool failures:

- one seed runtime failure produces a row and keeps the panel table complete,
- missing `summary.json` becomes `missing_summary`,
- malformed JSON becomes `malformed_capsule`,
- incompatible identity becomes `stale_or_incompatible`,
- no seed disappears from the panel output.

This avoids the worst failure mode: confusing a broken tool with an interesting
game blocker.

## Maturity Scorecard

This design should be judged against mature systems by concrete properties, not
by whether it has many features.

```text
Durability:
  Can the process die between slices without losing meaning?

Reproducibility:
  Can a row explain which config/source produced it?

Work avoidance:
  Does a second run skip compatible completed work?

Fair scheduling:
  Can one slow seed avoid starving the rest?

Separation:
  Does scheduler code avoid game strategy?

Observability:
  Can a caller tell real stop, soft pause, and tool failure apart?

Reviewability:
  Can a human inspect why work was reused or rejected?
```

The target for implementation is not feature parity with workflow engines. The
target is passing this scorecard in a small local tool.

## Detailed Migration Design

The following sections define the implementation-ready runtime boundaries.

## Migration Shape From Current `branch_tiny`

Current `branch_tiny` code mixes three concerns that must be separated before
`branch_panel` can be healthy:

```text
CLI args:
  human command-line parsing, aliases, probe flags, output paths.

Run contract:
  seed, ascension, objective, generations, max branches, auto/search budgets,
  slice budget, runtime feature flags.

Runtime state:
  frontier, next branch id, run-control sessions, branch status, artifact
  store, trace sinks.
```

The first migration must extract `RunContract` from the current `Args` shape.
`Args` may remain in `branch_tiny` as a CLI adapter type, but checkpoints,
capsules, and runtime APIs should carry `RunContract`.

### Target Module Ownership

The final module direction should be:

```text
src/runtime/branch/
  contract.rs          RunContract, RunBudgets, RunObjective
  model.rs             Branch, BranchStatus, Owner, BoundarySite
  runtime.rs           BranchRuntime
  slice.rs             RunSliceRequest, RunSliceResult, RunStop
  deadline.rs          RunDeadline / SliceDeadline
  frontier.rs          frontier retain/expand/checkpoint state
  artifact_store.rs    capsule/frontier/result/summary persistence interface
  capsule_json.rs      current JSON projection compatibility

src/runtime/panel/
  mode.rs              PanelMode
  scheduler.rs         PanelScheduler
  identity.rs          RunIdentity, IdentityMatch
  ledger.rs            panel ledger events
  summary.rs           panel summary projection

src/bin/branch_tiny.rs
  CLI parse -> RunContract / artifact options -> BranchRuntime

src/bin/branch_panel.rs
  CLI parse -> PanelRunRequest -> PanelScheduler
```

This does not require a physical move in one commit. It defines the ownership
target so that each mechanical extraction moves code in the right direction.

### Stay In Bin During Extraction

To avoid a massive one-shot move, the first implementation can keep files under
`src/bin/branch_tiny/` while making the boundaries real:

```text
Step A:
  introduce runtime-shaped types beside existing code.

Step B:
  make branch_tiny CLI convert Args -> RunContract.

Step C:
  make run_loop return RunSliceResult.

Step D:
  make run_chain call runtime directly.

Step E:
  move stabilized modules from bin to src/runtime/branch.
```

The important part is the dependency direction, not the first file path.

### Runtime API Contract

The runtime should not return `Result<(), String>` as the primary success path.
It should return typed slice information:

```text
RunSliceResult:
  contract: RunContract
  generation_start
  generation_end
  next_branch_id
  stop: RunStop
  frontier_summary
  selected_branch_summary
  artifacts_written
  elapsed_ms
```

Errors are reserved for runtime/tool failures such as malformed checkpoints,
artifact write failures, or invariant violations. A combat gap, owner gap,
terminal result, or soft pause is a successful `RunSliceResult`.

### Artifact Store Boundary

The runtime should write artifacts through an interface, even if the first
implementation has only a filesystem implementation:

```text
ArtifactStore:
  load_frontier(capsule) -> FrontierCheckpoint
  write_manifest(...)
  write_frontier(...)
  write_result(...)
  write_terminal(...)
  write_summary(...)
  append_capsule_ledger(...)
```

This prevents future panel code from reading and writing capsule JSON directly.
The filesystem remains the persistence backend, but not the semantic owner.

### Trace And Human Output

Trace writers and human printing are sinks, not runtime semantics.

`BranchRuntime` may accept optional sinks:

```text
trace_sink
human_log_sink
```

but typed `RunSliceResult` must be complete without reading either sink. This
is the rule that prevents the new runtime from becoming another stdout parser.

### Migration Cut Line

Do not start with `branch_panel`.

The first useful cut is:

```text
branch_tiny CLI still works
but internally calls BranchRuntime for one slice
and receives RunSliceResult
```

Only after that is true should `branch_tiny --continue-capsule` be rewritten.
Only after continuation is in-process should `branch_panel` be added.

## RunContract Extraction Design

`Args` currently appears throughout the runner, search portfolio, capsule,
trace, checkpoint, and rendering code. Most uses read stable runtime contract
fields, but some fields are CLI-only or per-slice derived state. A mechanical
rename would preserve the wrong shape, so the first cut must classify fields.

### Field Classification

Stable run contract:

```text
seed
ascension
objective
generations
max_branches
auto_ops
search_nodes
search_ms
rescue_search_nodes
rescue_search_ms
boss_search_nodes
boss_search_ms
slice_ms        // current wall_ms semantics renamed at the contract boundary
```

Runtime feature flags:

```text
checkpoint_before_combat_portfolio
```

Per-slice derived budget facts:

```text
wall_capped_search_budget
wall_capped_boss_budget
```

CLI/artifact/probe fields that must not enter `RunContract`:

```text
trace_jsonl
combat_gap_case_dir
frontier_checkpoint
resume_frontier
run_capsule
resume_capsule
continue_capsule
continue_slices
probe_event_owner
probe_event_screen
```

### Contract Types

The first runtime extraction should use small nested types rather than one
large flat struct:

```text
RunContract:
  game: GameRunContract
  objective: RunObjective
  branching: BranchingContract
  automation: AutomationContract
  combat_search: CombatSearchContract
  slice: SliceContract
  features: RuntimeFeatureContract

GameRunContract:
  seed
  ascension

BranchingContract:
  generations
  max_branches

AutomationContract:
  auto_ops

CombatSearchContract:
  primary_nodes
  primary_ms
  rescue_nodes
  rescue_ms
  boss_nodes
  boss_ms

SliceContract:
  slice_ms: Option<u64>

RuntimeFeatureContract:
  checkpoint_before_combat_portfolio
```

The nested shape makes identity and future config diffs readable. It also
prevents every call site from depending on one giant `Args`-like object.

### SliceBudgetView

`RunDeadline::cap_args` currently mutates an `Args` copy by reducing search
budgets and setting `wall_capped_*` flags. In the runtime design, this should
become a derived view:

```text
RunContract
  + SliceDeadline
  + child_count
  -> SliceBudgetView
```

`SliceBudgetView` carries:

```text
effective_search_ms
effective_rescue_search_ms
effective_boss_search_ms
search_budget_was_capped
boss_budget_was_capped
```

This avoids polluting the stable run identity with per-slice wall pressure.
The contract says what was requested. The slice view says what this slice could
afford.

### Compatibility During Migration

For the first implementation, `Args` can remain as:

```text
Args:
  contract: RunContract
  cli/runtime compatibility flags needed by old call sites
```

or it can remain flat with conversion helpers:

```text
impl From<Args> for RunContract
impl Args {
  fn from_contract_for_cli(contract: RunContract) -> Args
}
```

The preferred first step is conversion helpers, not a full `Args` rewrite.
That lets checkpoints and capsule JSON keep reading old payloads while new
runtime code starts accepting `RunContract`.

### Artifact Migration

Existing artifacts store:

```text
manifest.args
frontier.args
```

New artifacts should store:

```text
manifest.run_contract
frontier.run_contract
```

During migration, readers should accept both:

```text
if run_contract exists:
  use run_contract
else if args exists:
  convert args -> run_contract
  mark identity_match = unknown_or_legacy
else:
  malformed_capsule
```

Writers should write `run_contract` and may temporarily also write `args` as a
legacy projection. The legacy projection must be marked as compatibility data,
not as the primary identity source.

### First Implementation Cut

The first code cut should do only this:

```text
1. Add RunContract and nested contract structs.
2. Add Args -> RunContract conversion.
3. Add run_contract to manifest/frontier JSON while preserving args.
4. Update no behavior.
5. Add focused tests for conversion and artifact compatibility.
```

It should not yet:

```text
- move files into src/runtime,
- rewrite run_loop,
- introduce branch_panel,
- delete Args,
- delete child-process continuation.
```

This cut gives later work a typed contract without risking behavior drift.

## RunSliceResult And Stop Semantics

After `RunContract`, the next important boundary is `RunSliceResult`. Today,
stop facts are spread across:

```text
run_loop:
  decides loop exits and soft stops.

run_stop_recorder / run_persistence:
  saves frontier/result and prints messages.

branch_observer:
  records terminal branches and objective completion.

run_capsule_format:
  projects status into summary/report JSON.
```

This is why later tools must inspect artifacts or logs to understand what
happened. Runtime should instead produce one typed result first, then let sinks
persist or render it.

### RunStop Shape

`RunStop` should classify the slice outcome, not the cause of the whole run:

```text
RunStop:
  RealStop(RealStop)
  SoftPause(SoftPause)
  FrontierExhausted(FrontierExhausted)
```

`RealStop`:

```text
Terminal { outcome, branch_id }
ObjectiveSatisfied { objective, reason, branch_id }
CombatGap { branch_id, boundary, reason, combat_case }
AutomationGap { branch_id, boundary, site }
BudgetGap { branch_id, boundary, reason }
ApplyFailed { branch_id, reason }
AdvanceFailed { branch_id, reason }
AwaitingUnsupportedAuto { branch_id, boundary, reason }
```

`SoftPause`:

```text
SliceDeadline { generation, frontier_running_count }
AwaitingAutoBoundary { generation, frontier_running_count }
SearchBudgetCappedBeforeGeneration { generation, frontier_running_count }
```

`FrontierExhausted`:

```text
NoRunningBranches { generation }
NoExpandableBranches { generation }
```

Tool/runtime errors stay outside `RunStop`:

```text
Result<RunSliceResult, RunRuntimeError>
```

Malformed checkpoint, artifact read/write failure, and invariant violation are
runtime errors. Combat gap, owner gap, terminal defeat, and soft deadline are
successful slice results.

### RunSliceResult Shape

`RunSliceResult` should be enough for `branch_tiny`, `branch_panel`, tests, and
future analysis without reading stdout or `summary.json`:

```text
RunSliceResult:
  contract: RunContract
  request_kind: Start | Continue
  generation_start
  generation_end
  next_branch_id
  stop: RunStop
  frontier: FrontierSummary
  selected_branch: Option<BranchSummary>
  artifacts: ArtifactWriteSummary
  budget: SliceBudgetSummary
  elapsed_ms
```

`FrontierSummary`:

```text
total_count
running_count
expandable_count
terminal_count
gap_count
```

`BranchSummary`:

```text
branch_id
parent_id
status_kind
boundary
owner
act
floor
hp
max_hp
gold
deck_size
subject
```

`ArtifactWriteSummary`:

```text
manifest_written
frontier_written
result_written
terminal_written
summary_written
combat_case_written
ledger_appended
```

`SliceBudgetSummary`:

```text
slice_ms
elapsed_ms
remaining_ms
search_budget_was_capped
boss_budget_was_capped
```

### Stop Selection Rule

When several facts are true, runtime should select the stop in this order:

```text
1. objective satisfied
2. real terminal/gap/failure branch selected as result
3. unsupported/awaiting auto boundary
4. soft deadline with resumable frontier
5. frontier exhausted
```

This order keeps "victory found" from being hidden by a simultaneous deadline,
and keeps a true gap from being reported as a generic frontier condition.

### Sink Rule

Artifact and human outputs are projections:

```text
RunSliceResult -> capsule manifest/frontier/result/summary
RunSliceResult -> terminal table/log lines
RunSliceResult -> panel row
```

No sink should discover a new stop kind by re-reading files or parsing strings.
If a sink needs a field that is not in `RunSliceResult`, add the field to the
typed result or to a referenced artifact, not to the sink's private parser.

### First RunSliceResult Cut

The first implementation should not rewrite all stop handling at once. It
should:

```text
1. Add RunStop and RunSliceResult types.
2. Make run_loop construct a best-effort RunSliceResult before returning.
3. Keep existing artifact writes in place.
4. Keep existing prints in place.
5. Add tests that map current summary cases to RunStop.
```

Then later cuts can move artifact writes behind `ArtifactStore` and remove
summary/log parsing.

## ArtifactStore Design

`ArtifactStore` is the only runtime component allowed to know the concrete
filesystem shape of a capsule. Runtime and panel code should talk to the store
through typed operations, not by constructing JSON paths directly.

### Store Responsibilities

`ArtifactStore` owns:

```text
capsule directory allocation
manifest read/write
frontier checkpoint read/write
result/path/terminal artifact write
summary projection write
combat case sidecar write
capsule ledger append
old-capsule archive/tombstone
legacy args/run_contract migration reads
```

It does not own:

```text
run scheduling
branch expansion
owner policy
combat search policy
panel-level reuse decisions
human report interpretation
```

### Typed Store Interface

The runtime-facing store should be narrow:

```text
ArtifactStore:
  open_capsule(CapsuleRef) -> CapsuleHandle

CapsuleHandle:
  read_manifest() -> Option<CapsuleManifest>
  read_frontier() -> FrontierCheckpoint
  write_running_manifest(RunContract, CapsuleRunState)
  write_frontier(RunContract, FrontierCheckpoint, RunSliceResult)
  write_result(RunContract, BranchResultArtifact, RunSliceResult)
  write_terminal_entry(RunContract, TerminalArtifact)
  write_summary(CapsuleSummary)
  write_combat_case(CombatCaseArtifact) -> Option<ArtifactRef>
  append_ledger(CapsuleLedgerEvent)
  archive_existing(ArchiveReason) -> ArchivedCapsuleRef
```

The exact Rust names can differ. The important rule is that callers pass typed
payloads and receive typed artifact references.

### ArtifactRef

Artifacts should be referenced through a small typed reference:

```text
ArtifactRef:
  kind
  path
  schema
  created_by
```

This keeps ledgers and summaries from embedding large payloads while still
letting tools find the underlying detail.

### Summary Generation

`CapsuleSummary` should be produced from:

```text
RunSliceResult
  + latest branch/run state projection
  + artifact refs
```

It should not independently inspect result/frontier files to rediscover the
stop. That is how summaries become a parallel logic layer.

### Legacy Compatibility

The store should be the compatibility boundary for old artifacts:

```text
old manifest.args -> RunContract
old frontier.args -> RunContract
missing run_contract -> LegacyIdentity
```

Runtime code should not contain scattered legacy JSON parsing. If a legacy
field needs to be understood, `ArtifactStore` converts it into typed data and
marks its provenance.

### First ArtifactStore Cut

The first cut should be deliberately modest:

```text
1. Introduce ArtifactRef and ArtifactWriteSummary.
2. Wrap existing RunCapsule writes behind a CapsuleArtifactStore adapter.
3. Keep existing JSON schemas stable.
4. Make RunSliceResult receive ArtifactWriteSummary from the adapter.
5. Keep direct JSON format code private to the store adapter.
```

Current implementation has completed the first `ArtifactWriteSummary` plumbing
for core capsule writes in `RunSliceResult`; those summaries now carry typed
`ArtifactRef` values. Owner-audit JSON writes live behind a
`CapsuleArtifactStore` adapter. The adapter also appends
`capsule_ledger.jsonl` `slice_started` / `slice_finished` events with typed
artifact refs. Panel ledger events and summary rows consume those refs from
executed `RunSliceResult` values. A fully public store facade remains open; the
current adapter still preserves the legacy JSON schemas.

It should not yet redesign every artifact schema. The goal is to put a wall
around persistence semantics first.

## PanelScheduler Design

`PanelScheduler` is a Rust orchestrator over `BranchRuntime`. It should never
shell out to `branch_tiny`, parse `branch_tiny` output, or infer strategy from
game-specific labels.

### Panel Request

Panel scheduling starts from a typed request:

```text
PanelRunRequest:
  panel_id
  mode
  capsule_root
  seeds
  run_contract_template
  max_slices
  max_active
  fresh_policy
  identity_policy
```

`run_contract_template` carries all non-seed run settings. Each seed produces a
concrete `RunContract`.

### Panel Row Lifecycle

Each seed row moves through a simple lifecycle:

```text
Requested
  -> IdentityResolved
  -> ReusedRealStop
  -> Scheduled
  -> SoftPaused
  -> RealStopped
  -> ToolFailed
  -> Skipped
```

This lifecycle is scheduler state, not a human report. It is what makes
round-robin, resume, and partial failure understandable.

### Scheduling Loop

The scheduler loop should be:

```text
resolve identities for all seeds
for round in 0..max_slices:
  for row in seed order:
    if row is terminal for panel purposes:
      continue
    if no runnable capsule action exists:
      mark skipped or tool_failed
      continue
    call BranchRuntime for one slice
    persist through ArtifactStore
    update row from RunSliceResult
    append panel ledger event
  if all rows are terminal for panel purposes:
    break
write panel_summary.json
```

Panel purposes vary by mode:

```text
smoke:
  one slice per seed unless real stop appears first.

continue:
  run only missing or soft-paused compatible capsules.

drain:
  continue until real stop, tool failure, or panel budget.

compare:
  materialize separate compatible result sets by named config.
```

### Identity Resolution

Before any slice runs, scheduler resolves:

```text
requested RunIdentity
existing capsule identity
identity_match
reuse_decision
```

`PanelScheduler` chooses reuse or scheduling. `ArtifactStore` only reports what
exists. `BranchRuntime` only executes a requested slice.

### Failure Handling

A failed seed must not remove the row:

```text
runtime error -> ToolFailed row with error kind
artifact error -> ToolFailed row with artifact ref if available
identity mismatch -> Skipped or ToolFailed depending on policy
```

The panel may return non-zero for tool failures, but it must still write a full
summary with one row per requested seed.

### First PanelScheduler Cut

The first cut should come after `BranchRuntime` and in-process continuation:

```text
1. Add branch_panel binary.
2. Support mode=smoke, continue, drain, and compare.
3. Support max_active=1 only.
4. Use BranchRuntime directly.
5. Write panel_summary.json and panel_ledger.jsonl.
6. Keep tools/gap_panel.py as deprecated wrapper or leave it untouched until
   branch_panel covers current usage.
```

It should not yet implement parallelism, HTML, or ML export.

## Design Completion Checklist

This design is complete enough to start implementation when these statements
are true:

```text
Run identity is defined as input-addressed typed data.
RunContract is separated from CLI Args.
RunSliceResult is the typed result of a slice.
RunStop distinguishes real stop, soft pause, and frontier exhaustion.
ArtifactStore owns capsule filesystem semantics.
PanelScheduler calls BranchRuntime in-process.
CLI binaries are adapters.
Python tools only consume outputs.
Legacy artifacts have a migration path.
Implementation phases start with low-risk compatibility cuts.
```

The design is not trying to solve search quality, reward strategy, or ML. It is
trying to make those future experiments sit on a clean runtime surface.

## Implementation Phases

The design is complete, but implementation should be staged.

### Phase 1: Extract BranchRuntime

- Introduce typed `RunSliceRequest`, `ContinueSliceRequest`, `RunSliceResult`,
  and `RunStop`.
- Move initial frontier creation out of CLI startup into runtime construction.
- Move `run_loop::run` behind a runtime API that returns a typed result.
- Keep `branch_tiny` behavior stable by making it a CLI adapter over runtime.
- Preserve current capsule artifacts where practical.
- Current status: implemented. `branch_tiny` is a thin adapter and the
  owner-audit implementation files are runtime-owned.

### Phase 2: Remove Child-Process Continuation

- Rewrite `branch_tiny --continue-capsule` to call `BranchRuntime` directly.
- Delete or retire child-process continuation from `run_chain`.
- Record slice results through typed runtime values, not subprocess status.
- Keep the CLI command as a compatibility surface only.
- Current status: implemented. `run_chain` calls `BranchRuntime::run_slice`
  directly and writes slice results from typed `RunSliceResult` values.

### Phase 3: Ledger, Identity, And Compatibility

- Add a panel-level ledger.
- Add a capsule-level slice ledger if the existing capsule artifacts cannot
  already express enough history.
- Add a run identity payload to capsule summaries or manifests.
- Detect stale/incompatible capsules.
- Record binary/source fingerprint at the level available locally.
- Make panel reuse conditional on identity match.
- Treat unknown identity as non-reusable by default.
- Implement `--fresh` archival before destructive replacement.

### Phase 4: Rust Panel Scheduler

- Add `PanelScheduler`, `PanelRun`, `PanelMode`, and `PanelRunResult`.
- Add `branch_panel` as the CLI adapter over `PanelScheduler`.
- Add `--fresh`.
- Add `--mode smoke|continue|drain` with `continue` as the default for an
  existing capsule root and `smoke` as the default for a new root.
- Stop deleting existing capsules by default.
- Reuse real stops.
- Continue compatible soft pauses.
- Add `stop_class`, `reuse_decision`, `slice_count`, and elapsed fields to rows.

### Phase 5: Round-Robin Slice Scheduling

- Replace per-seed start-then-continue loops with slice rounds.
- Add `--slice-ms`, `--max-slices`, and `--max-active`.
- Keep `--wall-ms` as an alias for one transition period.

### Phase 6: Runbook And Cleanup

- Update `docs/RUNBOOK.md`.
- Rename panel wording from wall deadline to slice soft pause.
- Deprecate or delete `tools/gap_panel.py` after `branch_panel` covers the
  workflow.
- Remove or deprecate obsolete panel options after one migration window.

### Phase 7: Compare Mode

- Allow named policy/search profiles to be compared over the same compatible
  seed set.
- Materialize comparison capsules under `_compare/<profile>/<seed>` so compare
  runs do not mutate the base capsule namespace.
- Write a combined `panel_summary.json` with `run_mode = compare` and
  `profiles = [...]`.
- Keep this separate from normal smoke/continue/drain usage.
- V0 supports `baseline` and `double-search`; later profile work should move
  toward typed policy/search profiles rather than ad hoc CLI switches.

## Tests

Tests should cover stable scheduler contracts, not prose:

- existing compatible real-stop capsule is not rerun,
- existing compatible wall-deadline capsule is continued,
- `--fresh` replaces or archives a capsule explicitly,
- runtime failure keeps a row with `tool_failure`,
- missing summary becomes `missing_summary`,
- `--wall-ms` and `--slice-ms` produce the same slice budget.
- incompatible identity is not silently reused,
- unknown identity is not silently reused,
- panel summary includes one row per requested seed,
- scheduler ledger records reuse decisions.
- `--fresh` archives or tombstones the previous capsule.

Do not add tests for exact human table spacing.

## Success Criteria

The design is working when:

- running the same panel twice does not restart solved seeds,
- a soft-paused seed continues from its capsule by default,
- the panel table distinguishes real blockers from soft pauses,
- a five-seed panel can be interrupted and resumed without losing meaning,
- outer process timeout is no longer part of normal experiment control,
- future search experiments can consume `panel_summary.json` without parsing
  logs or human summaries.
- a second invocation of the same panel produces mostly reuse decisions, not
  duplicate work.
- a fallback/partial identity match is visibly marked and never counted as an
  exact reuse.
- `branch_tiny --continue-capsule` no longer spawns another `branch_tiny`
  process.
- panel scheduling can run multiple slices inside one Rust process.

## Implementation Decisions

These decisions are fixed for the first implementation plan.

### Run Identity Location

`manifest.json` is the primary identity location.

```text
manifest.run_contract
manifest.run_identity
manifest.source_identity
```

`summary.json` may include the same identity as a projection for convenient
readers. `frontier.json` must carry `run_contract` because it is the exact
resume artifact. The panel ledger records identity resolution decisions but is
not the primary identity store.

### Fresh Policy

`--fresh` archives by default.

```text
--fresh:
  archive old capsule, then create a new capsule.

--fresh --discard-old:
  delete old capsule only after writing a tombstone event.
```

Silent deletion is not allowed.

### Source Fingerprint

The initial source identity is:

```text
git_commit
git_dirty
dirty_tree_hash: optional, present only when cheap to compute
```

Binary mtime may be recorded as diagnostic information, but it is not part of
the primary identity. The scheduler is in-process Rust, so source identity is
more meaningful than executable timestamp.

### Legacy Capsule Rule

Capsules without `run_contract` are `unknown_legacy`.

They may be:

```text
inspected
archived by --fresh
continued only with an explicit legacy compatibility flag
```

They may not be silently reused as exact matches. If continued, the new slice
must write modern identity fields.

### CLI Transition

`branch_tiny --continue-capsule` remains during migration, but it must be
rewritten to call `BranchRuntime` in-process before `branch_panel` is added.

`tools/gap_panel.py` gets no new semantics. It may remain temporarily as a thin
launcher or be retired once `branch_panel` supports `smoke` and `continue`.

### Module Location

The final locations are:

```text
src/runtime/branch
src/runtime/panel
```

During the first cuts, runtime-shaped types may be introduced under
`src/bin/branch_tiny` to keep diffs mechanical. Before `branch_panel` is added,
the reusable runtime facade must move into library code under
`src/runtime/branch`.

### Move Scope Before `branch_panel`

Move only the minimal runtime surface before introducing `branch_panel`:

```text
RunContract
RunSliceRequest
RunSliceResult
RunStop
BranchRuntime facade
ArtifactStore facade
frontier checkpoint compatibility
```

Scene owners, reward/shop policy, and combat portfolio internals may remain as
dependencies behind `BranchRuntime` until there is a separate reason to move
them. `branch_panel` should depend on the runtime facade, not on every owner
module directly.

## External References Considered

These references informed the architecture; they are not dependencies.

- Temporal durable execution and event history:
  https://learn.temporal.io/tutorials/go/background-check/durable-execution/
  and https://docs.temporal.io/encyclopedia/event-history/event-history-java
- Flink checkpointing:
  https://nightlies.apache.org/flink/flink-docs-stable/docs/dev/datastream/fault-tolerance/checkpointing/
- Nextflow resume/cache:
  https://docs.seqera.io/nextflow/cache-and-resume
- Bazel remote caching:
  https://bazel.build/remote/caching
- Nix input-addressed and content-addressed derivations:
  https://nix.dev/manual/nix/2.24/language/advanced-attributes.html
- DVC pipelines and run cache:
  https://doc.dvc.org/start/data-pipelines/data-pipelines
  and https://doc.dvc.org/command-reference/repro
- ccache direct/preprocessor cache key modes:
  https://ccache.dev/manual/4.13.6.html
- GitHub Actions dependency cache keys and restore keys:
  https://docs.github.com/en/actions/reference/workflows-and-actions/dependency-caching
- Argo Workflows work avoidance and memoization:
  https://argo-workflows.readthedocs.io/en/latest/work-avoidance/
  and https://argo-workflows.readthedocs.io/en/latest/memoization/
- Ray Tune fault tolerance:
  https://docs.ray.io/en/latest/tune/tutorials/tune-fault-tolerance.html
- Optuna RDB-backed study resume:
  https://optuna.readthedocs.io/en/stable/tutorial/20_recipes/001_rdb.html
- MLflow run tracking:
  https://mlflow.org/docs/latest/ml/tracking/
- Hydra multirun and output directories:
  https://hydra.cc/docs/tutorials/basic/running_your_app/multi-run/
  and https://hydra.cc/docs/configure_hydra/workdir/
- SLURM checkpoint/requeue patterns:
  https://it.sci.utah.edu/slurm-job-re-queuing/
