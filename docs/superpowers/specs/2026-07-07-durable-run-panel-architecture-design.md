# Durable Run Panel Architecture Design

## Status

Review draft. This document defines the intended architecture before
implementation. It does not change runner behavior by itself.

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

## Non-Goals

- No new reward, shop, event, or combat strategy.
- No ML training or vector model.
- No HTML dashboard.
- No large human report system.
- No attempt to make five seeds statistically meaningful.
- No hidden reruns from Neow unless explicitly requested.

## Core Concepts

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

## CLI Shape

The future panel command should read like a scheduler:

```powershell
python tools/gap_panel.py `
  --seeds 1552225671..1552225675 `
  --capsule-root tools/artifacts/gap_panels/current `
  --slice-ms 15000 `
  --max-slices 8 `
  --max-active 1
```

Compatibility:

- keep `--wall-ms` as an alias for `--slice-ms`,
- keep `--continue-soft-wall` temporarily as an alias for `--max-slices`,
- add `--fresh` for explicit reruns,
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

## Implementation Phases

The design is complete, but implementation can be staged.

### Phase 1: Durable Panel Semantics

- Add `--fresh`.
- Stop deleting existing capsules by default.
- Reuse real stops.
- Continue compatible soft pauses.
- Add `stop_class`, `slice_count`, and elapsed fields to rows.
- Preserve the current `panel_summary.json` shape where practical.

### Phase 2: Identity And Compatibility

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

## Tests

Tests should cover stable scheduler contracts, not prose:

- existing compatible real-stop capsule is not rerun,
- existing compatible wall-deadline capsule is continued,
- `--fresh` replaces or archives a capsule explicitly,
- process failure keeps a row with `tool_failure`,
- missing summary becomes `missing_summary`,
- `--wall-ms` and `--slice-ms` produce the same slice budget.

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

## Open Decisions Before Implementation

These should be answered in the implementation plan, not by ad hoc code:

- exact location of run identity: `manifest.json`, `summary.json`, or both,
- whether `--fresh` archives old capsules or deletes them,
- initial source fingerprint: git commit only, dirty-tree hash, or binary mtime,
- transition period for `--continue-soft-wall`,
- whether `gap_panel.py` remains the long-term scheduler or becomes a thin
  launcher over a Rust scheduler.
