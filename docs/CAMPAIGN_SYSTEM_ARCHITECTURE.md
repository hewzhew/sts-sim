# Campaign System Architecture

This document is the target architecture for the campaign system. It is not a
description of the current compatibility wrapper. When current code disagrees
with this document, treat the code as migration debt.

The goal is not to make the existing PowerShell campaign wrapper tidier. The
goal is to remove the design that made the wrapper, report, checkpoint,
journal, active/frozen worksets, and ad hoc inspect modes fight each other.

## Why This Redesign Exists

The old campaign workflow grew from an interactive helper:

```text
run a little
save latest
continue with -More
inspect whatever broke
add another wrapper flag
add another report field
```

That was useful while the simulator needed frequent manual intervention. It is
now the wrong shape. The project needs deliberate campaign experiments, typed
decision coverage, exact replay state, and later learning data. Those needs
cannot be built reliably on a wrapper that owns semantics by accident.

The recurring failures all share one cause: two or more layers claimed ownership
of the same concept.

Examples:

- PowerShell and Rust both interpreted `latest`, `scratch`, `continue`, and
  `rounds`.
- Reports became a mix of user display, checkpoint data, planner diagnostics,
  training-like samples, and large combat traces.
- Branch scheduling exposed `active` and `frozen` as if they were the experiment
  model, then later tools had to guess which frozen branches mattered.
- Candidate identity leaked through display labels, command prefixes, synthetic
  marker strings, and replay snippets.
- Inspect tools were added as one-off switches instead of read-only views over a
  stable artifact model.
- Tests locked transitional behavior, names, or prose rather than durable
  ownership boundaries.

The fix is a campaign application architecture, not another patch to the
wrapper.

## Design Rule

Every campaign feature must answer one ownership question before code is
written:

```text
Is this simulator state, experiment planning, artifact lifecycle, decision
fact, diagnostic explanation, presentation, or learning export?
```

If the answer is unclear, the feature is not ready to implement.

## Target Shape

```text
User
  -> Campaign CLI
      -> CampaignApp
          -> ArtifactStore
          -> CampaignEngine
          -> ExperimentPlanner
          -> InspectRenderer
          -> Exporter
```

PowerShell may exist as a launcher. PowerShell must not be a campaign
application.

## Non-Negotiable Invariants

1. Rust owns campaign semantics.
2. A user-facing campaign operation maps to one typed Rust request.
3. Writing commands go through `CampaignApp` and `ArtifactStore`.
4. Inspect commands are read-only.
5. PowerShell may choose a build profile and forward arguments only.
6. `latest` and `scratch-latest` are typed pointers, not magic report files.
7. `rounds` means additional engine rounds. It must not mean "total rounds" in
   one path and "append rounds" in another.
8. Milestone loops are engine behavior, not wrapper loops.
9. Coverage execution starts from journaled candidate pools, not from
   active/frozen branch ordering.
10. Candidate identity is typed. Labels are display text only.
11. Reports are bounded projections. They are not checkpoint, journal,
    diagnostics, or training data.
12. Learning samples are exported sidecars. They are not silently accumulated in
    reports or checkpoints.
13. Strategy policy must not live in wrappers, report prose, display labels, or
    string reason parsing.
14. Tests protect schema, ownership, replay, and simulator mechanics. They must
    not protect temporary wording or questionable strategy outcomes.

## Public User Model

The public campaign model is:

```text
run
continue
coverage plan
coverage execute
inspect
artifacts
export
```

The user should not have to reason about:

```text
active branch pool
frozen branch pool
PowerShell scratch mode
latest report path
checkpoint sidecar path
prefix command replay
wrapper milestone loop
```

Those may exist internally during migration, but they are not the product model.

## Rust Components

### Campaign CLI

The CLI parses stable commands into typed requests:

```text
campaign run
campaign continue
campaign coverage plan
campaign coverage execute
campaign inspect
campaign artifacts
campaign export
```

Responsibilities:

- parse command names and flags
- reject ambiguous combinations
- print typed dry-run requests
- call `CampaignApp`

Non-responsibilities:

- resolve artifact paths by convention
- decide latest or scratch behavior outside `ArtifactStore`
- implement loops
- inspect JSON by hand
- render policy explanations directly from wrapper-specific fields

### CampaignApp

`CampaignApp` is the top-level Rust service boundary.

Responsibilities:

- resolve source and output intent
- choose whether a request reads, writes, or exports
- call `ArtifactStore`, `CampaignEngine`, `ExperimentPlanner`,
  `InspectRenderer`, or `Exporter`
- write command provenance through `ArtifactStore`

Hard rule: if a workflow mutates campaign artifacts, it must pass through
`CampaignApp`.

### ArtifactStore

`ArtifactStore` owns artifact lifecycle.

Responsibilities:

- resolve `latest`, `scratch-latest`, `run:<id>`, `scratch:<id>`, and explicit
  archaeology paths
- allocate run and scratch output artifacts
- read and write checkpoint, state, journal, report, diagnostics, manifest, and
  command provenance
- update latest pointers
- record encoding and size metadata
- list and prune artifacts

`ArtifactStore` is allowed to use gzip, compact schemas, content-addressed
objects, or sidecars. Callers must not care which storage layout is used.

Forbidden:

- PowerShell-written latest pointers
- PowerShell-written manifests
- direct report-path construction outside the store
- using a report as the only source of truth for resume or analysis

### CampaignEngine

`CampaignEngine` executes simulator campaigns.

Responsibilities:

- start a new campaign from seed/class/ascension/preset
- continue from exact checkpoint state
- run until round budget, milestone, terminal result, or explicit blocker
- emit progress events
- write exact resume state through `ArtifactStore`

Internal executor queues may use names like scheduled, parked, active, or
frozen. Those names are implementation details and must not become the public
experiment language.

### ExperimentPlanner

`ExperimentPlanner` owns deliberate branch exploration.

Input:

```text
CampaignJournal candidate pools
existing outcome observations
budget profile
milestone target
```

Output:

```text
ContinuationJob {
  source_artifact,
  replay_root,
  target_decision,
  target_candidate,
  budget,
  milestone,
  provenance,
}
```

Responsibilities:

- classify key decision nodes
- pick which candidates need observation
- allocate budget across decision type, candidate group, and milestone
- record whether candidates are unobserved, target-only, continued, terminal,
  censored, blocked by combat budget, or invalid

Non-responsibilities:

- deciding that a card, route, shop item, or boss relic is globally correct
- hiding candidates because they are inconvenient to schedule
- using active/frozen rank as a substitute for candidate coverage

### InspectRenderer

Inspect is a read-only projection over stored artifacts.

Views should be named and typed:

```text
summary
state
journal
coverage
lineage
decision
route
shop
combat
final-boss
artifact
```

Inspect must not:

- mutate latest pointers
- allocate scratch artifacts
- perform continuation implicitly
- rely on PowerShell-only paths
- parse display labels as identity

### Exporter

`Exporter` produces learning or analysis datasets from campaign artifacts.

Responsibilities:

- read checkpoint, journal, state, diagnostics, and outcome data
- produce explicit JSONL/Parquet/CSV sidecars
- record schema version and source artifact ids

Non-responsibilities:

- changing campaign execution
- adding training fields to default reports
- treating autopilot decisions as teacher labels

## Artifact Ownership

Campaign artifacts are split by purpose:

```text
checkpoint  exact simulator resume state
state       campaign executor bookkeeping
journal     durable decision facts and candidate pools
report      bounded human/tool inspection projection
diagnostic  optional large explanations and traces
export      learning or analysis dataset
manifest    provenance, encoding, sizes, and references
```

### Checkpoint

Checkpoint answers:

```text
Can the engine resume exactly from here?
```

It may store opaque simulator state. It must not be optimized for human reading.
If it becomes large, optimize storage through object references, pooling, or
content-addressed chunks, not by deleting resume truth.

### State

State answers:

```text
What campaign jobs and executor bookkeeping remain?
```

State can include internal queues. Public reports should translate those queues
into candidate coverage and outcomes rather than exposing active/frozen as the
main story.

### Journal

Journal answers:

```text
What decisions existed, what candidates existed, and where did later branches
come from?
```

Journal entries must use typed identity:

```text
decision_id
candidate_id
candidate_kind
source_artifact
replay_root
run_prelude
admission_category
target_origin
```

Journal entries may include compact facts. Large diagnostics belong in
diagnostic sidecars.

### Report

Report answers:

```text
What should a reader or tool inspect first?
```

Reports are allowed to summarize:

- run status
- terminal outcomes
- milestone progress
- candidate coverage
- notable blockers
- references to state, checkpoint, journal, diagnostics, and exports

Reports must not inline:

- full simulator sessions
- full route maps for every candidate
- full combat traces by default
- repeated planner score term tables
- learning samples

### Diagnostic

Diagnostic sidecars answer:

```text
Why did a planner, policy, or search component rank or reject something?
```

Diagnostics are opt-in and may be large. They should be referenced, not
silently embedded in default reports.

### Export

Exports answer:

```text
What dataset should another process train on or analyze?
```

Exports must be explicit. A campaign report is not a dataset.

## Decision Model

A decision is not a rendered command string.

Target shape:

```text
DecisionNode {
  decision_id,
  decision_kind,
  floor,
  act,
  source_artifact,
  replay_root,
  run_prelude,
  candidates,
}

DecisionCandidate {
  candidate_id,
  candidate_kind,
  action_ir,
  facts,
  admission,
  diagnostics_ref,
}
```

Display labels may exist, but they are never identity.

### Candidate Admission

Candidate admission should be structured:

```text
admission.status   scheduled | deferred | rejected | unavailable | unknown
admission.category policy | legality | budget | duplicate | dominated | bug
admission.reason   typed enum or stable code
```

Free-text explanations are comments. They are not control flow.

### Replay Root

Replay root is typed provenance:

```text
source_artifact
checkpoint_id
run_prelude
decision_path
```

The target design must not require long command-prefix strings to reconstruct a
decision. Command strings are for user reproduction only.

## Experiment Model

The old model was:

```text
keep a few active branches
freeze the rest
hope the active branches were right
inspect after failure
```

The target model is:

```text
journal candidate pools
choose coverage targets
continue candidates to milestones
compare outcomes by decision and candidate
refine only where evidence is censored, close, or strategically important
```

Budget profiles should mean:

```text
smoke      shortest health check
focused    one policy head plus major blockers
coverage   broad key-candidate observation
milestone  continue selected candidates to act/boss/death/budget blocker
deep       refine close or strategically important branches
```

This is still allowed to use queues internally. It is not allowed to present a
random internal queue as the experiment result.

## PowerShell Boundary

`tools/campaign.ps1` should end as:

```text
parse convenience aliases
choose build profile
build driver if needed
call Rust campaign CLI
print returned artifact refs
```

It should not:

- decide source semantics
- decide output artifact paths
- own scratch/latest behavior
- own milestone loops
- own coverage planning or execution
- write manifests
- parse reports as workflow control
- add a new switch for every probe

If a new feature seems to require more PowerShell code, the feature belongs in
Rust first.

## Deprecated Concepts

These concepts should not appear in new design, tests, examples, or public
output except as explicit legacy notes:

```text
-More
latest.campaign.json as a semantic source
scratch mode as a wrapper behavior
PowerShell milestone loop
PowerShell coverage-gap orchestration
active/frozen as the public experiment model
prefix command strings as decision identity
selected_plan as the only branch truth
report as checkpoint
report as training dataset
strategy encoded in display labels
tests that assert human wording
tests that assert one questionable card/shop choice is globally correct
```

## Testing Policy

Good tests:

- parser rejects ambiguous commands
- artifact store resolves and allocates typed refs
- writing commands create manifests
- inspect commands are read-only
- journal records candidate pools with stable ids
- coverage planner creates jobs from candidate ids
- replay root restores the intended decision boundary
- simulator mechanics match game rules

Bad tests:

- asserting prose wording
- asserting a transitional wrapper alias forever
- asserting one strategy choice as universally correct
- asserting active/frozen ordering as a product feature
- preserving a bug because a report field once printed it

If a test makes architecture worse to satisfy, delete or rewrite the test.

## Migration Strategy

Implementation can be incremental. The design cannot be incremental.

Each migration step must remove one semantic owner from the wrong layer. A step
that only renames files, splits PowerShell modules, or formats reports is not a
campaign architecture migration unless it also transfers ownership or deletes a
bad surface.

### Required Milestones

1. Rust `ArtifactStore` owns source resolution, output allocation, latest
   pointers, manifests, and artifact pruning.
2. Rust `CampaignApp` owns run, continue, inspect, coverage, artifact, and
   export request routing.
3. Rust `CampaignEngine` owns round and milestone continuation.
4. Rust `ExperimentPlanner` owns coverage target selection and continuation job
   creation from journaled candidates.
5. Rust `InspectRenderer` owns maintained inspect views.
6. PowerShell wrapper is reduced to build-and-launch.
7. Reports stop being large data stores; checkpoint/state/journal/diagnostic
   boundaries are enforced by writers.
8. Public output talks about candidate coverage and milestone outcomes, not
   active/frozen branch pools.

### Compatibility Rules

Compatibility is allowed only as a forwarding layer:

- legacy aliases may map to the stable Rust command path
- legacy readers may hydrate old artifacts
- legacy writers should not be extended
- ambiguous legacy behavior should fail loudly
- no new feature may depend on wrapper-only semantics

### Stop Conditions

Pause implementation and redesign if any step requires:

- parsing display labels for identity
- adding another report field that is actually checkpoint/journal/export data
- adding another PowerShell switch for a new experiment type
- making `rounds`, `latest`, or `scratch` mean different things by context
- testing strategy quality through a fixed card/relic/shop assertion

## Definition Of Done

The campaign architecture migration is done when:

- a direct Rust CLI command can perform every maintained workflow
- PowerShell contains no campaign semantic logic
- `latest` and `scratch-latest` are artifact-store pointers only
- coverage execution can target historical decision candidates without
  active/frozen guessing
- reports are small bounded projections
- checkpoint/state/journal/diagnostic/export ownership is visible in code
- inspect views are read-only and typed
- old compatibility names are absent from normal help and examples
- new strategy work can be developed without touching wrapper lifecycle code
