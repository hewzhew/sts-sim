# Durable Run Panel Architecture Design

## Status

Review draft, iteration 4. This document defines the intended architecture
before implementation. It does not change runner behavior by itself.

## Problem

`tools/gap_panel.py` currently behaves like a convenience wrapper:

- build `branch_tiny`,
- delete each seed capsule,
- run each seed from Neow,
- optionally continue once after `wall_deadline`,
- collect `summary.json`.

`branch_tiny --continue-capsule` also uses a transitional shape: it starts
another `branch_tiny` process for each continuation slice. That was acceptable
while the runner was still being made resumable, but it is now the wrong core
abstraction. Process boundaries force state and errors through files, stdout,
stderr, and exit codes even when the caller is Rust code in the same crate.

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

## Implementation Phases

The design is complete, but implementation can be staged.

### Phase 1: Extract BranchRuntime

- Introduce typed `RunSliceRequest`, `ContinueSliceRequest`, `RunSliceResult`,
  and `RunStop`.
- Move initial frontier creation out of CLI startup into runtime construction.
- Move `run_loop::run` behind a runtime API that returns a typed result.
- Keep `branch_tiny` behavior stable by making it a CLI adapter over runtime.
- Preserve current capsule artifacts where practical.

### Phase 2: Remove Child-Process Continuation

- Rewrite `branch_tiny --continue-capsule` to call `BranchRuntime` directly.
- Delete or retire child-process continuation from `run_chain`.
- Record slice results through typed runtime values, not subprocess status.
- Keep the CLI command as a compatibility surface only.

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
  capsule set.
- Write comparison rows without mutating the base capsule unless the mode
  explicitly materializes new capsules.
- Keep this separate from normal smoke/continue/drain usage.

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

## Open Decisions Before Implementation

These should be answered in the implementation plan, not by ad hoc code:

- exact location of run identity: `manifest.json`, `summary.json`, or both,
- whether `--fresh` archives old capsules or deletes them,
- initial source fingerprint: git commit only, dirty-tree hash, or binary mtime,
- transition period for `--continue-soft-wall`,
- exact migration rule for old capsules without identity fields.
- exact crate/module location for `BranchRuntime` and `PanelScheduler`,
- how much of current `src/bin/branch_tiny/*` moves into library modules before
  `branch_panel` is introduced.

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
