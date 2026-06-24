# Campaign System Architecture

This is the authority document for the campaign system.

It describes the target architecture, not the current accident. Current scripts,
old artifacts, legacy flags, tests, reports, and helper files are subordinate to
this document. If code disagrees with this document, either migrate the code or
make the incompatibility explicit as legacy archaeology. Do not document the
accident as normal behavior.

## Purpose

The campaign system is an experiment runner for the Slay the Spire simulator.
It must support this workflow:

```text
start or continue an exact simulator campaign
  -> record every important non-combat decision boundary
  -> store the full candidate pool at that boundary
  -> deliberately continue selected historical candidates
  -> run to milestones, terminal outcomes, or explicit blockers
  -> inspect lineage, state, and outcomes without mutating artifacts
  -> export explicit datasets for analysis or learning
```

The campaign system is not:

```text
a PowerShell workflow with a Rust binary attached
a report JSON database
an active/frozen branch queue exposed as experiment truth
a pile of latest/scratch/prefix conveniences
a strategy system hidden inside display labels, scores, or tests
```

## Root Problem To Prevent

The previous architecture failed because the same concepts had multiple owners.

Examples:

- PowerShell and Rust both interpreted source, output, continuation, milestone,
  scratch, latest, and coverage behavior.
- Reports became checkpoint, journal, diagnostics, combat trace, and
  training-like data at once.
- Internal executor queues leaked as public concepts: active, frozen, best hp,
  cleanest, furthest.
- Candidate identity was reconstructed from labels, command prefixes, synthetic
  marker strings, or replay snippets.
- Inspect modes grew as one-off switches instead of typed read-only views.
- Tests preserved transitional wording or uncertain strategy choices.

Architecture progress means removing one of these ownership conflicts. A change
that only splits a file, renames a field, compresses JSON, or makes output
prettier is not progress unless it also moves ownership to the correct layer or
deletes a wrong public surface.

## Non-Negotiable Invariants

1. Rust owns campaign semantics.
2. PowerShell is a launcher only.
3. Every maintained operation maps to one typed Rust request.
4. Every artifact write passes through `CampaignApp` and `ArtifactStore`.
5. Inspect is read-only by construction.
6. `latest` and `scratch-latest` are artifact-store pointers, not magic paths.
7. `rounds` means additional campaign rounds everywhere.
8. Milestone continuation is Rust engine behavior, not a wrapper loop.
9. Coverage starts from journaled decision candidates, not active/frozen queues.
10. Candidate identity is typed. Labels are display text only.
11. Reports are bounded projections, not checkpoint, journal, diagnostics, or
    learning datasets.
12. Learning data is produced by explicit export commands.
13. Strategy policy must not live in wrappers, report prose, display labels,
    command prefixes, or string reason parsing.
14. Tests protect schema, ownership, replay, and simulator mechanics. They do
    not protect temporary wording or one uncertain card/shop/relic choice.

## Public Product Surface

The maintained Rust surface is:

```text
branch_campaign_driver campaign run
branch_campaign_driver campaign continue
branch_campaign_driver campaign coverage plan
branch_campaign_driver campaign coverage execute
branch_campaign_driver campaign inspect
branch_campaign_driver campaign artifacts
branch_campaign_driver campaign export
```

Everything else is either:

- a local launcher alias that forwards to this surface
- an internal implementation detail
- legacy archaeology

The public product is not:

```text
-More
-FromScratchLatest
-InspectScratchLatest
latest.campaign.json as a semantic source
PowerShell milestone loops
PowerShell coverage-gap orchestration
prefix command replay
active/frozen as experiment truth
selected_plan as the only branch truth
report-as-checkpoint
report-as-training-dataset
```

## Target Runtime Shape

```text
User or wrapper
  -> Campaign CLI
      -> CampaignApp
          -> ArtifactStore
          -> CampaignEngine
          -> CampaignJournal
          -> ExperimentPlanner
          -> InspectRenderer
          -> Exporter
```

Each component has one owner role. Cross-layer shortcuts are bugs.

### Campaign CLI

The CLI parses commands into typed request structs.

Owns:

- stable command names
- option parsing
- preset expansion into visible typed settings
- dry-run request display

Does not own:

- artifact path resolution
- latest/scratch pointer semantics
- continuation loops
- report parsing
- strategy explanation

### CampaignApp

`CampaignApp` is the Rust service boundary for campaign workflows.

Owns:

- request dispatch
- read/write/export intent
- calling the correct subsystem
- command provenance
- enforcing read-only inspect behavior

Rule:

```text
If a workflow mutates campaign artifacts, it goes through CampaignApp.
```

### ArtifactStore

`ArtifactStore` owns artifact identity, lifecycle, encoding, and references.

Owns:

- resolving `latest`, `scratch-latest`, `run:<id>`, `scratch:<id>`, and
  explicit `path:<path>`
- allocating new run and scratch outputs
- writing checkpoint, state, journal, report, diagnostics, exports, manifest,
  command provenance, and pointer files
- recording schema versions, encoding, raw/compressed sizes, and references
- listing, showing, and pruning artifacts

Allowed implementation choices:

- `.json.gz`
- compact schemas
- id pools
- content-addressed objects
- diagnostic sidecars

Forbidden:

- PowerShell-written latest pointers
- PowerShell-written manifests
- report path construction outside the store
- using a report as the only resume or analysis source

### CampaignEngine

`CampaignEngine` executes campaign jobs.

Owns:

- new campaign execution from seed, character, ascension, and preset
- exact continuation from checkpoint state
- round-budget execution
- milestone execution
- terminal outcome detection
- explicit blocker detection
- progress events
- exact resume state emission

Internal queues are allowed. They are not the experiment model.

### CampaignJournal

`CampaignJournal` records decision facts.

Owns:

- decision ids
- replay roots
- branch coordinates
- typed candidate pools
- candidate ids
- candidate admission facts
- applied candidate provenance
- links from later observations back to historical candidates

Does not own:

- ranking policy
- branch scheduling
- combat traces
- training labels
- large score decompositions

### ExperimentPlanner

`ExperimentPlanner` chooses deliberate continuation work from journaled
candidates.

Input:

```text
journaled decision candidate pools
existing observations
budget profile
milestone target
coverage policy
```

Output:

```text
ContinuationJob {
  source_artifact,
  replay_root,
  target_decision_id,
  target_candidate_id,
  target_origin,
  budget,
  milestone,
}
```

Owns:

- candidate coverage planning
- budget allocation across decision types and candidate groups
- target provenance
- censored/terminal/invalid/superseded outcome classification

Forbidden:

- thawing frozen branches as a substitute for candidate coverage
- hiding candidates because the executor did not keep them active
- deciding a card, shop item, event, route, or boss relic is globally correct

### InspectRenderer

Inspect renders typed read-only views.

Stable view families:

```text
summary
artifact
state
journal
coverage
lineage
decision
route
shop
combat
final-boss
diagnostic
```

Inspect must not:

- allocate artifacts
- mutate latest pointers
- perform continuation implicitly
- parse labels as ids
- treat report fields as hidden source truth

### Exporter

`Exporter` produces explicit datasets.

Owns:

- learning JSONL
- analysis JSONL/CSV/Parquet
- source artifact references
- schema version
- censored-outcome metadata

Forbidden:

- changing campaign execution
- turning reports into training tables
- treating autopilot decisions as teacher labels

## Core Domain Model

These concepts must be represented as typed structures in maintained code.

### ArtifactRef

```text
ArtifactRef {
  selector: latest | scratch-latest | run:<id> | scratch:<id> | path:<path>
  run_id
  checkpoint_ref
  state_ref
  journal_ref
  report_ref
  diagnostics_refs
  manifest_ref
}
```

Artifact refs are the only maintained way to pass artifact identity between
components.

### RunPrelude

```text
RunPrelude {
  seed
  character
  ascension
  neow_context
  initial_map
  deterministic_rng_context
}
```

Run prelude replaces hidden CLI prefixes and implicit process-start assumptions.

### ReplayRoot

```text
ReplayRoot {
  source_artifact
  checkpoint_id
  run_prelude
  decision_path
}
```

Replay roots are canonical. Command strings may be displayed for humans, but
they are never identity.

### DecisionNode

```text
DecisionNode {
  decision_id
  decision_kind
  act
  floor
  branch_coordinate
  replay_root
  candidate_pool_id
}
```

### CandidatePool

```text
CandidatePool {
  pool_id
  decision_id
  candidates: Vec<DecisionCandidate>
}
```

### DecisionCandidate

```text
DecisionCandidate {
  candidate_id
  candidate_kind
  action_ir
  facts
  admission
  diagnostics_ref
  display_label
}
```

`display_label` is optional projection. It must not be parsed.

### CandidateAdmission

```text
CandidateAdmission {
  status: scheduled | deferred | rejected | unavailable | unknown
  category: legality | policy | budget | duplicate | dominated | invalid | bug
  reason_code: stable code
  source: compiler | scheduler | engine | legacy
  explanation: optional display text
}
```

Free text is commentary, not control flow.

### CoverageTarget

```text
CoverageTarget {
  origin_decision_id
  origin_candidate_id
  replay_root
  target_milestone
  budget_profile
  target_origin
}
```

### ContinuationJob

```text
ContinuationJob {
  target
  source_artifact
  output_intent
  engine_budget
  search_budget
  capture_policy
}
```

Continuation jobs are the unit the engine runs.

## Artifact Boundaries

Campaign storage is split by purpose.

```text
checkpoint  exact simulator resume state
state       campaign executor bookkeeping
journal     durable decision facts and candidate pools
report      bounded inspection projection
diagnostic  optional large explanations and traces
export      explicit learning or analysis dataset
manifest    provenance, schema, encoding, sizes, and refs
```

### Checkpoint

Question answered:

```text
Can the engine resume exactly from here?
```

Checkpoint may be opaque and compact. It must not be made human-readable at the
cost of resume correctness.

### State

Question answered:

```text
What campaign executor work remains?
```

State may contain internal queues. Reports translate those queues into coverage
and outcome views.

### Journal

Question answered:

```text
What decision boundaries and candidates existed?
```

Journal stores compact typed facts. Large explanations belong in diagnostics.

### Report

Question answered:

```text
What should a reader or tool inspect first?
```

Reports contain bounded summaries and references. They do not inline full
checkpoint, full journal, full route maps, full combat traces, or learning rows.

### Diagnostic

Question answered:

```text
Why did a component rank, reject, fail, or stop?
```

Diagnostics are opt-in and may be large. They link back to ids.

### Export

Question answered:

```text
What dataset should another process consume?
```

Exports are explicit. A campaign report is not a dataset.

## Experiment Model

Old public model:

```text
keep two active branches
freeze the rest
hope active was right
inspect after failure
```

Target model:

```text
record candidate pools
select coverage targets from candidate ids
continue targets to milestones
classify each candidate observation
compare by decision, candidate group, and outcome
```

Internal scheduling may still use queues, beams, parking lots, or priority
heaps. Public reports must describe candidate coverage, not internal queue
names as if they were conclusions.

Observation states:

```text
unobserved
target_only
continued
milestone_reached
terminal_victory
terminal_death
combat_budget_blocked
censored_by_budget
invalid_replay
superseded
```

## Strategy Boundary

Policy compilers may generate facts, candidate deltas, risks, and diagnostics.
They do not get to own campaign lifecycle.

Examples:

- Card reward policy may say a card provides frontload, setup, scaling,
  exhaust fuel, or debt.
- Shop compiler may emit a frontier of purchase plans.
- Route planner may emit route facts and risk projections.
- Boss pressure models may describe pressure and missing answer facts.

But campaign scheduling asks:

```text
Which candidates need observation under this budget?
```

It must not ask:

```text
Which string label sounds like the winner?
Which frozen branch should be thawed?
Which selected_plan did the old shop policy approve?
```

If a strategy concept is needed in multiple places, create or repair a typed
domain profile. Do not copy card/relic/potion names into several policies.

## PowerShell Boundary

PowerShell may:

- locate the repository
- choose a local build profile
- run `cargo build` when requested
- call one Rust campaign command
- print stdout/stderr and returned refs

PowerShell may not:

- resolve source/output semantics
- own latest/scratch behavior
- implement milestone loops
- implement coverage planning or execution
- write manifests
- write pointer files
- parse reports for workflow control
- add a new switch for every probe

If a feature seems to need PowerShell semantics, the feature belongs in Rust.

## Deprecated Surfaces

These names may appear only in explicit legacy notes or archaeology paths:

```text
-More
-Last as semantic state
-FromScratchLatest as ordinary continue
-InspectScratchLatest
-MaxRounds meaning total rounds
latest.campaign.json as source truth
PowerShell milestone loop
PowerShell coverage-gap orchestration
active/frozen as public experiment model
prefix command strings as identity
selected_plan as the branch truth
strategy encoded in display labels
tests that assert wording
tests that assert one uncertain strategy choice
```

## Testing Policy

Good tests protect boundaries:

- parser rejects ambiguous commands
- typed requests are built correctly
- inspect requests are read-only
- artifact store resolves selectors and writes refs
- journal records candidate pools with stable ids
- coverage planner creates jobs from candidate ids
- replay roots restore the intended boundary
- simulator mechanics match game rules

Bad tests protect accidents:

- exact human wording
- one temporary wrapper alias
- active/frozen order as product behavior
- one uncertain card/shop/relic decision as universal truth
- a report field that exists only because an old inspect tool needed it

Delete or rewrite tests that make architecture worse.

## Migration Standard

Implementation can be staged. The target design cannot be contradictory.

A migration step is valid only if it does at least one of these:

- moves a concept to its correct owner
- deletes a wrong public surface
- quarantines legacy behavior behind explicit archaeology naming
- replaces string identity with typed identity
- converts wrapper orchestration into one Rust request
- separates checkpoint/state/journal/report/diagnostic/export responsibilities

Changes that do not count:

- splitting a large file without changing ownership
- renaming active/frozen while still using it as public experiment truth
- compressing JSON while storing the wrong payload
- adding report fields because a probe lacks a real view
- adding magic scores because no domain owner exists
- adding another wrapper switch for a new experiment

## Stop-And-Redesign Triggers

Stop implementation and redesign when a change requires:

- parsing display text for identity
- storing training-only data in report or journal
- making `rounds`, `latest`, `scratch`, or `source` mean different things by
  context
- adding a PowerShell semantic branch
- using active/frozen to answer a coverage question
- adding a magic score because a typed profile does not exist
- preserving a legacy alias as the normal path
- writing a test that asserts uncertain strategic taste

## Definition Of Done

The campaign architecture migration is complete when:

- every maintained workflow has a direct Rust `campaign ...` command
- PowerShell contains no campaign semantic logic
- `latest` and `scratch-latest` are artifact-store pointers only
- all artifact writes pass through `CampaignApp` and `ArtifactStore`
- checkpoint, state, journal, report, diagnostic, export, and manifest are
  distinct in code and storage
- inspect views are typed and read-only
- coverage planning targets historical candidate ids
- route, reward, shop, event, boss relic, campfire, and deck mutation decisions
  all record candidate pools in the journal
- coverage execution can continue historical candidates without active/frozen
  guessing
- normal help and docs do not teach retired aliases
- reports stay bounded for long runs
- learning data is exported explicitly
- new strategy work can be built without touching wrapper lifecycle code
