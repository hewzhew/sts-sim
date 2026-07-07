# Durable Run Panel Architecture Design

## Status

Review draft, iteration 2. This document defines the intended architecture
before implementation. It does not change runner behavior by itself.

## Problem

`tools/gap_panel.py` currently behaves like a convenience wrapper:

- build `branch_tiny`,
- delete each seed capsule,
- run each seed from Neow,
- optionally continue once after `wall_deadline`,
- collect `summary.json`.

That was useful while owner coverage was the main problem, but it is now the
wrong control surface. Five-seed panels take minutes, often redo already known
work, and blur three different concepts:

- a real blocker, such as combat gap or owner gap,
- a normal soft pause with saved frontier,
- a process or tool failure.

The result is not just slow. It makes investigation semantics depend on how a
script happened to be called.

## Design Goal

Turn the panel from a rerun script into a durable run scheduler.

```text
seed/config/code identity
  -> reusable run capsule
  -> short resumable slices
  -> panel-level scheduler
  -> typed stop classification
```

`wall_ms` remains useful, but only as a slice-level soft deadline. It must not
be the meaning of an experiment.

The closest mature analogs are:

- durable workflow engines: event history plus deterministic replay,
- data/workflow systems: task identity plus work avoidance,
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

## Core Concepts

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
- command kind and process result,
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

A run slice is one bounded invocation of `branch_tiny` against a capsule.

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
command_kind = start | continue
process_exit
before_state
after_state
```

The slice budget is a normal scheduling knob. It should usually be short enough
to keep the panel responsive, not long enough to hide progress for minutes.

### Panel Scheduler

The panel scheduler owns experiment orchestration across seeds. It does not own
game policy.

It should:

- build once,
- resolve or create one capsule per run identity,
- skip capsules that already reached a real stop,
- continue soft-paused capsules in short slices,
- use round-robin scheduling when multiple seeds remain live,
- stop when the panel budget or all real stops are reached,
- write a small structured panel summary.

It should not:

- delete capsules by default,
- parse human prose,
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
  build_failed
  run_failed
  continue_failed
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
```

This field matters because "not rerun" can be either a correct cache hit or a
bug that hid stale data.

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

## CLI Shape

The future panel command should read like a scheduler:

```powershell
python tools/gap_panel.py `
  --seeds 1552225671..1552225675 `
  --capsule-root tools/artifacts/gap_panels/current `
  --mode continue `
  --slice-ms 15000 `
  --max-slices 8 `
  --max-active 1
```

Compatibility:

- keep `--wall-ms` as an alias for `--slice-ms`,
- keep `--continue-soft-wall` temporarily as an alias for `--max-slices`,
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
    run one slice or continue one frontier
    refresh row from summary.json
```

This gives each seed a chance to advance without one slow seed hiding all other
results. `--max-active` can remain `1` until the runner and artifacts are
stable enough for parallel runs.

The scheduler should prefer fairness over depth by default. A panel that spends
all time on one seed before touching the others is a campaign run, not a panel.

## Error Handling

The scheduler must be strict about tool failures:

- build failure writes panel summary and exits non-zero,
- one seed failure produces a row and keeps the panel table complete,
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

### Phase 1: Durable Panel Semantics

- Add `--fresh`.
- Add `--mode smoke|continue|drain` with `continue` as the default for an
  existing capsule root and `smoke` as the default for a new root.
- Stop deleting existing capsules by default.
- Reuse real stops.
- Continue compatible soft pauses.
- Add `stop_class`, `reuse_decision`, `slice_count`, and elapsed fields to rows.
- Preserve the current `panel_summary.json` shape where practical.

### Phase 2: Ledger, Identity, And Compatibility

- Add a panel-level ledger.
- Add a capsule-level slice ledger if the existing capsule artifacts cannot
  already express enough history.
- Add a run identity payload to capsule summaries or manifests.
- Detect stale/incompatible capsules.
- Record binary/source fingerprint at the level available locally.
- Make panel reuse conditional on identity match.

### Phase 3: Round-Robin Slice Scheduling

- Replace per-seed start-then-continue loops with slice rounds.
- Add `--slice-ms`, `--max-slices`, and `--max-active`.
- Keep `--wall-ms` as an alias for one transition period.

### Phase 4: Runbook And Cleanup

- Update `docs/RUNBOOK.md`.
- Rename panel wording from wall deadline to slice soft pause.
- Remove or deprecate obsolete panel options after one migration window.

### Phase 5: Compare Mode

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
- process failure keeps a row with `tool_failure`,
- missing summary becomes `missing_summary`,
- `--wall-ms` and `--slice-ms` produce the same slice budget.
- incompatible identity is not silently reused,
- panel summary includes one row per requested seed,
- scheduler ledger records reuse decisions.

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

## Open Decisions Before Implementation

These should be answered in the implementation plan, not by ad hoc code:

- exact location of run identity: `manifest.json`, `summary.json`, or both,
- whether `--fresh` archives old capsules or deletes them,
- initial source fingerprint: git commit only, dirty-tree hash, or binary mtime,
- transition period for `--continue-soft-wall`,
- whether `gap_panel.py` remains the long-term scheduler or becomes a thin
  launcher over a Rust scheduler.

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
- DVC pipelines and run cache:
  https://doc.dvc.org/start/data-pipelines/data-pipelines
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
