# Campaign Workspace V2

This document defines the next campaign lifecycle design. It is not a rename of
`run`, `continue`, or `latest`. The goal is to stop treating each driver
invocation as the durable unit of work.

The current toolchain has one repeated failure mode:

```text
we run a campaign attempt
  -> it produces report/checkpoint/journal files
  -> later code changes
  -> we want to keep useful progress without rerunning everything
  -> source/latest/rounds/scratch/report semantics become overloaded
  -> generated artifacts accumulate and become hard to trust
```

Workspace V2 makes the durable unit a campaign workspace. Individual executions
become disposable attempts inside that workspace.

## Design Goals

- Keep useful campaign progress across frequent code changes.
- Avoid rerunning from seed zero when a compatible state snapshot already
  exists.
- Make generated data easy to discard without losing the experiment context.
- Separate exact recovery state from reports, diagnostics, and learning data.
- Make compatibility explicit instead of pretending all old artifacts can be
  resumed.
- Keep the PowerShell wrapper as a launcher, not a lifecycle owner.

## Non-Goals

- This does not solve strategy quality, combat search quality, or branch ranking.
- This does not make old artifacts permanently compatible.
- This does not make reports a source of truth.
- This does not preserve `scratch-latest`, wrapper manifests, command replay, or
  coverage-gap orchestration as campaign lifecycle concepts.

## Core Model

### Workspace

A workspace is the long-lived object for one investigation.

```text
CampaignWorkspaceV2
  workspace_id
  created_at
  game_identity
  compatibility_policy
  frontier_index
  pinned_refs
  attempts
```

`game_identity` contains stable game facts such as seed, character, ascension,
and simulator ruleset identity. It does not contain `quick`, `deep`, or any other
launcher convenience template.

`frontier_index` points to snapshots that can be used as future sources. It is
not "latest run output".

### Attempt

An attempt is one execution of the driver.

```text
CampaignAttemptV2
  attempt_id
  workspace_id
  source_ref
  request
  engine_fingerprint
  produced_snapshot_refs
  produced_observation_refs
  status
  retention
```

Attempts are cache entries by default. They may be deleted if their useful
snapshots or observations have been promoted or pinned elsewhere.

An attempt records its budget request for audit, but the budget request is not
campaign identity. Continuing a workspace does not mean "repeat this attempt's
arguments".

### Snapshot

A snapshot is the exact recoverable state.

```text
CampaignSnapshotV2
  snapshot_id
  workspace_id
  parent_snapshot_id
  decision_origin
  engine_state_ref
  scheduler_state_ref
  rng_state_ref
  compatibility_fingerprint
```

Snapshots are the only valid resume sources. Reports, command text, path
prefixes, and human-readable decision labels are not resume sources.

If the current binary cannot prove compatibility with a snapshot, that snapshot
is read-only historical context. It may still be inspected, but it is not
resumed.

### Journal Event

Journal events are typed observations, not recovery state.

```text
CampaignJournalEventV2
  event_id
  workspace_id
  attempt_id
  snapshot_id
  event_type
  payload_ref
```

Examples:

- candidate pool observed
- candidate selected
- candidate not explored
- combat result observed
- shop inventory observed
- route candidate pool observed

Journal events help answer "what happened?" and "what was not explored?" They do
not restore the simulator.

### Report

A report is a derived view.

Reports can be regenerated from workspace metadata, snapshots, and journal
events. A report may be stored for convenience, but it must not be required to
continue an experiment.

## Object Storage

The workspace should use object references for large or repeated data.

```text
workspace/
  workspace.json
  attempts/
    <attempt_id>.json
  objects/
    sha256/
      ab/
        <hash>.json.gz
  views/
    latest-summary.json
```

Objects are content-addressed when practical. Candidate pools, map graphs,
combat traces, and repeated state fragments should be stored once and referenced
by ID.

This is not primarily a compression trick. It is a boundary rule:

```text
core index files stay small
large facts live behind object refs
views are disposable
```

## Execution Requests

Workspace V2 should not expose a single overloaded `rounds` field.

There are separate request types:

```text
CreateWorkspaceRequest
  game_identity
  initial_source

RunAttemptRequest
  workspace_id
  source_selector
  budget
  retention_policy

InspectWorkspaceRequest
  workspace_id
  view
```

`budget` is explicit about what is being limited:

```text
CampaignAttemptBudget
  max_wall_ms
  max_new_rounds
  max_new_snapshots
  max_search_nodes
```

The first implementation may only use `max_new_rounds`, but the schema must not
pretend that "rounds" is the campaign lifecycle.

Convenience templates such as `quick` or `deep` may exist only as client-side
request builders. After expansion, the attempt stores the concrete budget it
received for audit. The workspace does not inherit a template name.

## Source Selection

Source selection chooses a snapshot or frontier set from the workspace.

Allowed selectors:

```text
current
snapshot:<snapshot_id>
frontier:<frontier_name>
decision-gap:<decision_id>
```

Legacy selectors such as `latest`, `run:<id>`, and `path:<report>` can remain as
import adapters during migration. They should resolve into workspace snapshots
or fail explicitly.

## Compatibility

Each snapshot carries a compatibility fingerprint. The fingerprint should be
declared by the simulator, not guessed from command text.

Minimum contents:

```text
state_schema_version
scheduler_schema_version
journal_schema_version
rules_semantics_version
policy_bundle_version
binary_commit
```

`binary_commit` is provenance. It is not automatically a compatibility breaker.
The schema and semantic version fields decide whether exact resume is allowed.

If compatibility fails, the system may:

- keep the snapshot for inspection,
- replay from a compatible ancestor if one exists,
- or require a new workspace/attempt.

It must not silently resume with changed semantics.

## Retention and Garbage Collection

Every attempt starts as cache.

Retention classes:

```text
cache
promoted
pinned
obsolete
```

Rules:

- Cache attempts can be deleted.
- Promoted snapshots survive even if their producing attempt is deleted.
- Pinned attempts or snapshots survive until explicitly unpinned.
- Derived reports and summaries can always be regenerated or deleted.
- Old incompatible attempts are historical observations, not continuation
  sources.

Garbage collection should walk from:

- workspace root,
- current frontier snapshots,
- pinned refs,
- promoted snapshots,
- retained journal events,

and delete unreferenced objects.

## Relationship to Active/Frozen

Workspace V2 does not by itself solve branch scheduling. It changes the storage
contract so the scheduler can improve without rewriting artifact semantics.

The old active/frozen language should not be a lifecycle concept. A scheduler may
still maintain queues internally, but the workspace should store:

```text
frontier candidates
coverage gaps
promoted snapshots
attempt observations
```

This makes it possible to later replace the scheduler without invalidating the
storage layer.

## PowerShell Wrapper Boundary

`tools/campaign.ps1` remains a launcher only.

Allowed:

- build or locate the Rust binary,
- pass command-line arguments,
- print the command being run.

Not allowed:

- choose continuation semantics,
- infer source identity from command text,
- write manifests,
- implement milestone loops,
- implement coverage-gap execution,
- manage scratch/latest state.

Those must live in Rust workspace commands or be retired.

## Migration Plan

### Phase 1: Add Workspace Store Beside Current Artifacts

- Add workspace root allocation in Rust.
- Write `workspace.json`.
- Write one `attempt.json` per campaign run.
- Let existing report/checkpoint files continue to be emitted as compatibility
  views.

No behavior change is required in this phase.

### Phase 2: Make Snapshots First-Class

- Store exact campaign snapshots as object refs.
- Let continuation take `snapshot:<id>` internally.
- Make report/checkpoint paths optional export views rather than required source
  inputs.

### Phase 3: Replace Latest Run With Current Workspace

- `latest` points to a workspace, not a report artifact.
- "Continue latest" means "run an attempt from the workspace's current frontier".
- If no compatible snapshot exists, fail with a compatibility explanation.

### Phase 4: Move Inspect to Workspace Views

- Inspect reads workspace index and object refs.
- Reports are generated views.
- Large diagnostic tables stay in sidecar objects or on-demand renderers.

### Phase 5: Garbage Collection

- Implement reachability-based cleanup.
- Delete unpinned cache attempts and orphan objects.
- Keep only promoted/pinned state.

## Acceptance Criteria

The design is working when these are true:

- A new run creates or updates a workspace without relying on command text.
- A follow-up attempt can run from a compatible snapshot without restating seed,
  class, ascension, or old launcher template names.
- Reports can be deleted without losing the ability to inspect or continue from
  promoted snapshots.
- Old incompatible attempts are clearly marked read-only and are not resumed.
- Generated artifacts can be garbage-collected without manually guessing what is
  safe to delete.
- `tools/campaign.ps1` remains a launcher and does not regain lifecycle logic.

## Explicit Anti-Patterns

Do not implement Workspace V2 by:

- adding another `mode` field to old reports,
- renaming `rounds` to `add_rounds` while keeping the same overloaded command,
- storing command lines as the recovery source,
- making `latest` point to whichever report happened to finish last,
- preserving every old artifact shape forever,
- treating gzip compression as a lifecycle design,
- adding PowerShell switches for new campaign semantics.

The durable model is:

```text
workspace -> attempts -> snapshots/observations -> derived views
```

Everything else is compatibility or presentation.
