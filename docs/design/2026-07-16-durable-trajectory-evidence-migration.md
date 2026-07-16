# Durable Trajectory Evidence Migration

## Status

Accepted implementation direction. The 2026-07-16 live calibration proved
that planner boundary visits and committed progress records pair faithfully
inside one bounded drive, but also proved that the evidence is not durable
across capsule slices. This migration replaces the current recent-only handoff
with an immutable trajectory-segment DAG owned by the capsule artifact store.

This is an authority migration, not a new strategy feature. It must not change
candidate enumeration, policy selection, combat search, or run outcomes.

## Calibration Finding

The calibration capsule for seed `20260715001` produced two branch nodes:

- branch 0 contained one selected Neow proceed visit, one yielded Neow reward
  visit, and one matching decision transaction;
- branch 1 contained four selected visits, one yielded card-reward visit, four
  matching decision transactions, and one independently typed combat
  resolution.

All five selected visits matched exactly one decision transaction by decision
step, selection source, and run candidate id. Seven visit occurrences used six
semantic visit ids because the Neow reward boundary was first yielded by the
parent and then selected by its child. That duplicate semantic id is correct.

The durable gap is outside the bounded driver:

- `Branch` carries only `recent_progress_journal` and
  `recent_planner_capture`;
- `frontier.json` does not store either field and resume resets both to empty;
- `capsule_ledger.jsonl` records slice metadata and artifact references, not
  trajectory evidence;
- terminal results serialize only the latest in-memory segment;
- optional `trace.jsonl` happens to contain node evidence, but it is CLI-owned,
  truncates on creation, and is not a capsule authority.

## Ownership Decision

There are three different authorities and they must remain separate:

1. **Run state authority**: the frontier checkpoint contains only the state
   required to resume execution plus a trajectory head reference.
2. **Committed mutation authority**: `RunProgressJournalV1` remains the ordered
   typed truth for mutations within a segment.
3. **Observation evidence sidecar**: planner boundary visits remain typed
   public observations linked to the committed mutation authority.

`RunTrajectorySegmentV1` is the durable envelope that links authorities 2 and
3 to branch lineage. It persists existing facts; it does not reinterpret or
duplicate their semantics.

## Durable Shape

Each branch points to one immutable segment head:

```text
root segment S0
  -> child segment S1A
  -> child segment S1B
  -> child segment S1C
```

Children reference `S0`; they do not copy it. A checkpoint stores only the
selected branch's `trajectory_head_id` and depth.

The segment contract is:

```text
RunTrajectorySegmentV1:
  schema_name
  schema_version
  segment_id
  run_id
  branch_id
  parent_segment_id
  generation
  depth
  disposition
  progress_journal
  boundary_visit_occurrences
```

`segment_id` is a deterministic content id over every field except itself.
`run_id` is stable for the capsule's original run identity and does not change
when a continuation overrides a per-slice budget.

The branch keeps an in-memory queue of immutable, already-identified segment
drafts that have not yet been written. Every call that installs a new advance
result must append a draft before it can overwrite `recent_*`. Observation or
terminal recording flushes the queue in parent-before-child order.

## Boundary Identity

Two identities are required:

- `visit_id` is the semantic public boundary identity already produced by the
  planner capture layer;
- `occurrence_id` identifies that visit inside one trajectory segment and
  branch.

A parent `yielded` occurrence and a child `selected` occurrence may share a
`visit_id`. They must never share an `occurrence_id`.

Only a selected occurrence projects to a behavior event. Yielded occurrences
remain coverage and orchestration evidence.

## Content-Addressed Payloads

Planner observations and legal candidate sets are large and already carry
stable content ids. The durable store writes each payload once:

```text
trajectory/
  observations/<observation-id>.json
  candidate_sets/<candidate-set-id>.json
  segments/<segment-id>.json
```

Segment occurrences retain only the observation id, candidate-set id,
candidate links, outcome, decision step, semantic visit id, and occurrence id.
This prevents full public-map snapshots from being copied into every branch.

An existing payload path is accepted only when its decoded id and value match
the value being committed. A mismatch is a typed integrity failure, never an
overwrite.

## Commit Order And Recovery

A segment commit uses this order:

1. atomically write any missing observation payloads;
2. atomically write any missing candidate-set payloads;
3. atomically write the immutable segment;
4. append a typed segment-committed record or artifact reference to the
   capsule ledger;
5. write a frontier whose branch checkpoint references the committed head.

A crash before step 4 may leave unreferenced immutable payloads. They are safe
orphans and may be garbage-collected. A ledger reference to a missing or
hash-mismatched segment is corruption and must fail closed. Resume must verify
that every non-empty checkpoint head exists and belongs to the same run.

Runtime continuation does not load ancestral segment payloads into `Branch`.
It restores only the verified head id and depth.

## Ordering Contract

`decision_step` orders decisions but does not order every mutation: combat and
forced transitions deliberately preserve the decision-step counter. Durable
ordering is therefore:

```text
segment depth -> journal entry ordinal
```

Behavior projection may use decision step for matching, but it must use the
segment chain and journal ordinal for total trajectory order.

## Segment Integrity

Before a segment can be committed:

- every selected boundary occurrence matches exactly one decision transaction
  in the same segment by decision step, selection source, and run candidate id;
- every decision transaction matches exactly one selected occurrence, unless
  the segment carries an explicit typed representation gap;
- a combat resolution does not require a planner decision occurrence;
- a forced transition is represented by the capture layer's typed
  mutation-without-selection outcome;
- occurrence ids are unique within the segment;
- referenced observations and candidate sets have matching ids;
- the parent id equals the branch's previous trajectory head;
- depth is parent depth plus one, or zero for a root;
- no stop record appears in the committed progress journal.

Integrity failures are typed data gaps. They are not string strategy reasons
and must not be converted into policy fallbacks.

## Behavior Projection

Behavior records are read-only projections from committed segments. A
projector walks one head to its root, reverses the chain, and emits one record
for each selected occurrence after matching it to its decision transaction.

The projection records:

- run and trajectory identity;
- segment and occurrence identity;
- decision, observation, candidate-set, and selected planner candidate ids;
- the actual policy-selection source;
- known deterministic, known stochastic, or unknown selection probability;
- mechanics and provenance manifests;
- segment depth and journal ordinal.

The incumbent policy is behavior evidence, never teacher authority.

## Outcome Projection

Outcome attachments are derived facts, not mutations of behavior events. The
first implementation supports typed horizons without a scalar reward:

- immediate committed successor;
- next combat resolution;
- act terminal;
- run terminal.

If the selected trajectory ends before a horizon is observed, the projector
records a typed censored outcome. A bounded search miss, soft pause, or absent
continuation is never fabricated as defeat.

Attribution and learned value remain future work. Raw horizon facts must be
available before any model or heuristic consumes them.

## Capsule And Trace Migration

The capsule owns durable segments by default. No CLI flag is required.

- the capsule ledger references committed segment artifacts;
- frontier checkpoints store only trajectory head id and depth;
- terminal and summary files reference trajectory heads and derived summaries;
- `trace.jsonl` becomes an optional projection/debug rendering of durable
  segments rather than an evidence authority;
- once the durable reader and projector are accepted, evidence completeness
  must no longer depend on `--trace-jsonl`.

Temporary dual-write is permitted only during this migration and must have an
explicit deletion test. It may not become a permanent compatibility owner.

## Delivery Sequence

1. Add the typed segment, occurrence, payload-reference, disposition, and
   integrity contracts with deterministic-id tests.
2. Add the capsule trajectory store with immutable collision checks and branch
   head verification.
3. Make every advance producer append a segment draft before replacing recent
   evidence; flush drafts at branch observation and terminal boundaries.
4. Persist and restore checkpoint head references without loading history.
5. Add DAG reconstruction plus behavior and raw-horizon projection.
6. Project result/summary/trace output from durable heads and remove the
   evidence-authority role of the optional trace CLI.

## Acceptance

The migration is accepted only when:

- a one-slice run and an equivalent multi-slice continuation reconstruct the
  same trajectory;
- every selected calibration visit links to exactly one transaction;
- parent yield and child selection occurrences with the same semantic visit id
  coexist correctly;
- branch forks share immutable ancestors without copying payloads;
- a resumed branch preserves and verifies its trajectory head;
- terminal, soft-pause, progress-budget, and wall-deadline exits preserve the
  final segment;
- hidden RNG state and scheduled future encounters never enter planner
  payloads;
- behavior and outcome artifacts rebuild solely from committed segments;
- the full library and architecture-boundary suites pass;
- the old trace path is no longer required for evidence completeness.

