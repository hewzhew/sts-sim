# Campaign Artifact Architecture

Campaign artifacts are storage and replay surfaces, not strategy authority. Keep
these responsibilities separate:

```text
checkpoint  exact simulator state needed to resume execution
state       scheduler/workset state needed to continue a campaign
journal     append-only decision facts and candidate pools
report      bounded projection for inspection and tools
diagnostic  opt-in sidecar data for large explanations and traces
```

This boundary matters more than any single field name. When one artifact tries
to be checkpoint, journal, report, and training dataset at once, it becomes
large, hard to replay, and easy to mistake for policy truth.

## Storage Rules

- Campaign artifacts are JSON by schema, normally written as `.json.gz`.
- Readers should accept `.json` and `.json.gz` when practical; new default
  outputs should prefer `.json.gz`.
- Wrapper manifests should record artifact paths, encoding, and useful size
  metadata such as raw and compressed bytes when available.
- Compression is not a license to store unbounded payloads. If the raw JSON is
  large, fix the ownership boundary instead of relying on gzip.

## Checkpoint

Checkpoint owns exact resume state. It may use id pools or sidecars for repeated
large objects, but loading a checkpoint must hydrate the same simulator session
semantics before execution resumes.

Allowed:

- simulator state required to resume a branch
- branch coordinates needed to locate that state
- pooled repeated state objects, referenced by stable ids
- active combat state only when it is required for resume

Forbidden:

- report summaries as the source of resume truth
- policy explanations that are not needed to resume
- duplicated large state objects when an id pool can restore the same state

Current campaign exports may pool repeated `RunState.map` objects, combat
automation trajectories, and active-combat state. A plain
`RunControlSessionCheckpointV1` can remain self-contained outside campaign
export; campaign export is allowed to externalize and hydrate.

## State

State owns campaign scheduling data: scheduled branches, parked branches,
victories, dead branches, abandoned work, round summaries, discard accounting,
retry ledgers, and other continuation bookkeeping.

Allowed:

- compact branch metadata needed by the next scheduler step
- round-budget and continuation counters
- retry/intervention bookkeeping

Forbidden:

- exact simulator sessions; those belong in checkpoint
- candidate pools and decision facts; those belong in journal
- human-only render text that can be regenerated

## Journal

Journal owns decision facts. It is the source for candidate auditing,
coverage-gap continuation, lineage inspection, and later learning sample export.

Allowed:

- decision id, event id, branch id, and parent branch coordinates
- candidate id, command, typed summary, admission, and disposition
- typed route/event/shop/reward fields needed to target a candidate later
- provenance references to checkpoint state or diagnostic sidecars

Forbidden:

- full run state or combat state
- selected-candidate payloads duplicated from the same candidate pool
- large score decompositions, node dumps, or free-form route explanations
- fields whose only purpose is one temporary text rendering

Route and map candidate pools should keep compact typed facts needed for
coverage-gap execution. Planner internals such as score terms, value factors,
node-feature dumps, needs vectors, and long reasons belong in diagnostics unless
a concrete consumer needs them as typed facts.

## Report

Report is a projection. It should be cheap to inspect and, when possible, cheap
to regenerate from checkpoint, state, and journal.

Allowed:

- compact run status and aggregate counters
- bounded branch slices and short examples
- links or ids into checkpoint, state, journal, and diagnostics
- small fields that pass [Report Field Admission](REPORT_FIELD_ADMISSION.md)

Forbidden:

- full checkpoint state
- full journal candidate pools
- combat trajectories
- large route planner diagnostics
- winner-like fields unless the evidence really supports a winner claim

Default reports should carry `state_artifact` / `journal_artifact` references
instead of inlining those payloads. Compatibility readers may hydrate referenced
sidecars into memory, but new report fields should not depend on inlining.

## Diagnostics

Diagnostics are opt-in sidecars for large or narrow-use explanations.

Examples:

```text
route-diagnostics.jsonl
combat-trajectory.jsonl
shop-evidence.jsonl
campfire-evidence.jsonl
learning-sample.jsonl
```

Use a diagnostic sidecar when the payload:

- is large and not needed for normal resume
- explains a decision but does not define the decision fact
- is useful for one inspect command or offline analysis

Diagnostics must link back to a journal event, candidate id, branch id, or
checkpoint session. They should never be the only place where a decision fact is
stored.

## Learning Data

Learning data should be exported from campaign artifacts; it should not turn the
report or journal into a training table.

Preferred shape:

```text
campaign artifacts  ->  inspected/validated export  ->  JSONL or columnar data
```

The campaign artifact owns replay, provenance, and auditability. Learning
exports own model-facing feature rows, labels, censored outcomes, and sampling
metadata. If a feature exists only for a model experiment, keep it in the export
or a diagnostic sidecar until it becomes a stable domain fact.

## Lifecycle

The maintained lifecycle is:

```text
runs/<run-id>/          normal campaign history, addressed by latest pointer
scratch/<artifact-id>/  disposable experiments, addressed by scratch/latest
diagnostics/ or perf/   opt-in analysis buckets
root loose files        old/debug debris, outside maintained storage
```

`tools/campaign.ps1` is only a minimal source/output/continuation launcher.
This document defines ownership. Normal runs should not write raw campaign
artifacts into the root artifact directory.

## References

Prefer typed references over command-string markers or parseable labels.

Good:

```text
run_state_map_id: run_state_map:17
trajectory_id: combat_trajectory:42
journal_event_id: route-pool:candidate_set
candidate_id: route_move:normal_edge:x1:y7
```

Bad:

```text
commands[] contains "__decision_parent:..."
label contains "CoffeeDripper | adds debt rest_lock"
summary text must be parsed to find a route candidate
```

String markers can remain for old artifacts, but new storage should use typed
fields and hydrate through explicit references.

## Compaction Rules

Default campaign export should:

1. Pool repeated checkpoint objects before writing sessions.
2. Store scheduler state and journal candidate pools as sidecars or references.
3. Keep route/map candidates as compact facts, not planner debug dumps.
4. Store selected ids or target references instead of duplicating selected
   candidate payloads.
5. Keep combat trajectories and large evidence tables out of the default report.
6. Prefer sidecar diagnostics or learning exports over new report payloads.

Stop micro-compacting when the schema boundary is already correct. Continue only
when the raw JSON cost is material or the change clarifies ownership.

## Field Review

Before adding a new artifact field, answer:

1. Is this resume state, scheduler state, decision fact, report projection,
   diagnostic, or learning feature?
2. Which artifact owns it?
3. Can it be regenerated from an existing source?
4. Is it duplicated across branches, sessions, or candidates?
5. Does it need a typed reference instead of embedding the payload?
6. Is it bounded for a long campaign run?

If the answer is unclear, do not add the field to the default report.
