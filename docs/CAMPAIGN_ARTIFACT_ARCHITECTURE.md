# Campaign Artifact Architecture

Campaign artifacts must keep four responsibilities separate:

```text
checkpoint  = authoritative state needed to resume execution
journal     = append-only decision facts and candidate pools
report      = compact projection for humans, audits, and scripts
diagnostic  = optional sidecar for large explanation data
```

This boundary is more important than any individual field name. If one file
tries to be all four things, it will grow until it becomes hard to inspect,
hard to replay, and easy to accidentally treat as policy truth.

## Design References

This project should follow mature storage patterns:

- Workflow engines keep event history separate from execution state and cut
  long histories into new runs instead of growing one object forever.
- Event-sourced systems use materialized views for reports. The view is not
  the source of truth.
- Build systems and VCS tools store large repeated objects once and refer to
  them by id or content hash.
- Experiment trackers separate metrics, parameters, and artifacts instead of
  placing every diagnostic payload in a single report.
- Telemetry systems link traces, logs, and metrics with ids. Span attributes
  are bounded facts, not unlimited dumps.

These are design constraints, not external dependencies.

## Checkpoint

The checkpoint is the only campaign artifact that must restore execution.

Allowed:

- exact simulator state required to resume a branch
- branch command coordinates needed to locate that state
- content-addressed or id-addressed pools for repeated state objects
- references from session entries into those pools

Forbidden:

- human-readable report summaries as the only source of resume truth
- duplicated large state objects when an id pool can restore the same state
- policy explanations that are not required to resume execution

Current pattern:

```text
BranchCampaignCheckpointV1
  nodes[]
  decision_parent_anchor_commands[]
  run_state_maps[]                  # pooled repeated RunState.map objects
  combat_automation_trajectories[]  # pooled repeated last-combat traces
  sessions[]
    commands
    run_state_map_id?
    session                         # hydrated before restore
```

Normal `RunControlSessionCheckpointV1` remains a complete checkpoint when used
outside campaign export. Campaign export may externalize fields into pools, but
must hydrate them before `into_session()`.

## Journal

The journal records decision facts. It is the source for coverage-gap,
candidate auditing, and learning-sample construction.

Allowed:

- decision id, event id, branch id, parent branch commands
- candidate id, command, typed summary, admission, disposition
- enough typed route/event/shop/reward fields to replay or target a candidate
- stable provenance references to checkpoint state or sidecar diagnostics

Forbidden:

- full run state or combat state
- large route score decomposition in the default campaign report
- selected-candidate payloads duplicated from a candidate pool
- fields whose only purpose is a temporary text rendering

Route candidate pools are allowed to keep typed route origin data required for
coverage-gap continuation. Internal route planner explanations such as score
terms, node-feature dumps, value factors, needs vectors, and free-form reasons
belong in a diagnostic sidecar unless a specific consumer needs them as typed
facts.

## Report

The report is a projection. It should be cheap to regenerate from checkpoint
and journal when possible.

Allowed:

- compact branch summaries
- active/frozen/abandoned/victory slices
- aggregate counters
- links or ids into journal/checkpoint/diagnostic artifacts
- short examples for orientation

Forbidden:

- full checkpoint state
- full route planner diagnostics
- combat trajectories
- repeated copies of candidate pools already present in the journal
- winner-like fields unless the evidence really supports a winner claim

Report fields should pass [Report Field Admission](REPORT_FIELD_ADMISSION.md):
classify them as fact, diagnostic, verdict, or label before adding them.

## Diagnostic Sidecars

Large diagnostic data should be sidecar artifacts, not default report content.

Examples:

```text
route-diagnostics.jsonl
combat-trajectory.jsonl
shop-evidence.jsonl
campfire-evidence.jsonl
learning-sample.jsonl
```

Sidecars may be verbose because they are opt-in. They must include provenance
back to the journal event or checkpoint session they explain.

Use a sidecar when:

- the payload is large and not needed for normal resume
- the payload explains a decision but does not define the decision fact
- the payload is useful only for one inspect command or offline analysis

## Object References

Prefer typed references over command-string markers.

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

String markers can remain for compatibility, but new storage should use typed
fields and hydrate through explicit references.

## Compaction Rules

Default campaign artifact export should apply these projections:

1. Pool repeated checkpoint objects before writing sessions.
2. Drop large route planner diagnostics from journal route candidates.
3. Drop selected route candidate copies when selected id/index/target remain.
4. Keep candidate facts needed for coverage-gap continuation.
5. Keep sidecar-worthy diagnostics out of the report unless explicitly
   requested.

If a future feature needs a dropped diagnostic field, add a sidecar or typed
reference first. Do not re-expand the default report.

## Review Checklist

Before adding a new artifact field, answer:

1. Is this resume state, decision fact, report projection, or diagnostic?
2. Which artifact owns it?
3. Can it be regenerated from an existing source?
4. Is it duplicated across many branches or sessions?
5. Does it need a typed reference instead of embedding the payload?
6. Is it bounded in size for a long campaign run?

If the answer is unclear, do not add the field to the default campaign report.
