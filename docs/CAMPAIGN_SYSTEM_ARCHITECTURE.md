# Campaign System Architecture

This document is the authority contract for the campaign system.

It describes the target architecture. If current scripts, old artifacts, tests,
or older docs disagree with this file, treat them as migration debt. Do not
document accidental behavior as desired behavior.

## Design Goal

The campaign system is an experiment runner for the Slay the Spire simulator.
It should make it possible to:

```text
run deterministic campaign branches
record every important decision candidate pool
continue deliberately selected candidates to milestones
inspect lineage and outcomes without mutating artifacts
export explicit learning or analysis datasets
```

It must not be a pile of wrapper shortcuts around "latest", "scratch",
"active", "frozen", or "run a few more rounds and inspect whatever happened".
That shape was useful while automation still needed frequent manual help. It is
now the main source of confusion.

## Root Failure Pattern

Nearly every recent campaign tooling failure came from the same mistake:

```text
two layers claimed ownership of the same concept
```

Examples:

- PowerShell and Rust both interpreted source, output, rounds, milestone, and
  coverage behavior.
- Reports became display, checkpoint, journal, diagnostics, combat trace, and
  training-like data at once.
- Internal executor queues (`active` / `frozen`) leaked into the public
  experiment model.
- Candidate identity leaked through labels, command prefixes, synthetic marker
  strings, and replay snippets.
- Inspect modes grew as one-off switches instead of typed read-only views.
- Tests preserved transitional wording, aliases, or uncertain strategy choices.

The target architecture exists to prevent these failure modes. A change that
only renames, splits, compresses, or prettifies code without fixing ownership is
not architecture progress.

## Non-Negotiable Invariants

1. Rust owns campaign semantics.
2. PowerShell is a launcher only: build profile, local convenience aliases,
   process invocation, and stdout/stderr display.
3. Every maintained user operation maps to one typed Rust request.
4. Every artifact-writing workflow passes through `CampaignApp` and
   `ArtifactStore`.
5. Inspect is read-only by construction.
6. `latest` and `scratch-latest` are typed artifact pointers, not magic paths.
7. `rounds` means additional campaign rounds everywhere.
8. Milestone continuation is engine behavior, not a wrapper loop.
9. Coverage execution starts from journaled decision candidates, not from
   active/frozen ordering.
10. Candidate identity is typed. Labels are display text only.
11. Reports are bounded projections, not checkpoint, journal, diagnostics, or
    training data.
12. Learning data is produced by explicit export commands.
13. Strategy policy must not live in wrappers, report prose, display labels, or
    string reason parsing.
14. Tests protect schema, ownership, replay, and simulator mechanics. They do
    not protect temporary wording or one uncertain card/shop/relic choice.

## Public Product Model

The public campaign product is:

```text
campaign run
campaign continue
campaign coverage plan
campaign coverage execute
campaign inspect
campaign artifacts
campaign export
```

The public product is not:

```text
active branch pool
frozen branch pool
latest report path
checkpoint sidecar path
PowerShell scratch mode
PowerShell milestone loop
prefix command replay
selected_plan as the only branch truth
report-as-database
```

Internal queues may continue to exist while the engine is rewritten. They must
not be the vocabulary used for experiment coverage, learning, or user-facing
analysis.

## Target Runtime Shape

```text
User or script
  -> Campaign CLI
      -> CampaignApp
          -> ArtifactStore
          -> CampaignEngine
          -> ExperimentPlanner
          -> InspectRenderer
          -> Exporter
```

### Campaign CLI

The CLI parses stable commands into typed request structs and rejects ambiguous
requests.

Allowed:

- parse command names and options
- expand named presets into visible typed settings
- print typed dry-run requests
- call `CampaignApp`

Forbidden:

- resolving artifact paths by convention
- deciding source/output semantics outside `ArtifactStore`
- implementing continuation loops
- inspecting JSON by hand
- rendering strategy explanations from wrapper-only fields

### CampaignApp

`CampaignApp` is the Rust service boundary for campaign workflows.

Responsibilities:

- resolve source and output intent
- decide whether a request reads, writes, or exports
- dispatch to `ArtifactStore`, `CampaignEngine`, `ExperimentPlanner`,
  `InspectRenderer`, and `Exporter`
- record command provenance through `ArtifactStore`

Hard rule:

```text
If a workflow mutates campaign artifacts, it passes through CampaignApp.
```

### ArtifactStore

`ArtifactStore` owns artifact lifecycle.

Responsibilities:

- resolve `latest`, `scratch-latest`, `run:<id>`, `scratch:<id>`, and explicit
  archaeology paths
- allocate run and scratch outputs
- read and write checkpoint, state, journal, report, diagnostics, manifest,
  command provenance, and exports
- update latest pointers
- record encoding, schema version, and size metadata
- list and prune artifacts

Allowed implementation details:

- `.json.gz`
- compact schema encodings
- content-addressed objects
- sidecars
- id pools and object references

Callers must not care which storage layout is used.

Forbidden:

- PowerShell-written latest pointers
- PowerShell-written manifests
- direct report-path construction outside the store
- using a report as the only source of truth for resume or analysis

### CampaignEngine

`CampaignEngine` executes campaign jobs.

Responsibilities:

- start a new campaign from seed, character, ascension, and preset
- continue from exact checkpoint state
- run until round budget, milestone, terminal result, or explicit blocker
- emit progress events
- write exact resume state through `ArtifactStore`

Internal executor queues are allowed. They are not the experiment model.

### ExperimentPlanner

`ExperimentPlanner` owns deliberate branch exploration.

Input:

```text
CampaignJournal candidate pools
existing observations
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

- classify important decision nodes
- choose candidates that need observation
- allocate budget across decision kind, candidate group, and milestone
- track whether each candidate is unobserved, target-only, continued,
  terminal, censored, combat-budget-blocked, invalid, or superseded

Forbidden:

- hiding candidates because the executor did not keep them active
- treating active/frozen rank as candidate coverage
- deciding a card, shop item, route, event, or boss relic is globally correct

### InspectRenderer

Inspect is a read-only projection over stored artifacts.

Stable views are typed:

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

`Exporter` produces learning and analysis datasets.

Responsibilities:

- read checkpoint, state, journal, diagnostics, and outcome data
- produce explicit JSONL, Parquet, or CSV sidecars
- record schema version and source artifact ids

Forbidden:

- changing campaign execution
- adding training fields to default reports
- treating autopilot choices as teacher labels

## Request Lifecycle

Every maintained write command follows this lifecycle:

```text
parse CLI
  -> build typed request
  -> CampaignApp resolves source and output intent
  -> ArtifactStore resolves source refs and allocates output refs
  -> CampaignEngine or ExperimentPlanner executes
  -> ArtifactStore writes checkpoint/state/journal/report/manifest
  -> command prints stable artifact refs
```

Every inspect command follows this lifecycle:

```text
parse CLI
  -> build typed inspect request
  -> CampaignApp resolves source ref read-only
  -> InspectRenderer reads necessary artifacts through ArtifactStore
  -> render bounded view
```

Dry-run prints the typed request and resolved read/write refs. It never writes
artifacts.

## Core Domain Objects

### ArtifactRef

```text
ArtifactRef {
  kind: run | scratch | path
  id
  report_ref
  checkpoint_ref
  state_ref
  journal_ref
  manifest_ref
  diagnostics_ref
}
```

An artifact ref is the only maintained way to pass artifact identity between
components.

### ReplayRoot

```text
ReplayRoot {
  source_artifact
  checkpoint_id
  run_prelude
  decision_path
}
```

Replay roots replace long prefix command strings as identity. Command strings
may exist for human reproduction, never as the canonical replay key.

### DecisionNode

```text
DecisionNode {
  decision_id
  decision_kind
  act
  floor
  source_artifact
  replay_root
  candidates
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
}
```

Display text may be derived from a candidate. It must not define identity.

### CandidateAdmission

```text
CandidateAdmission {
  status: scheduled | deferred | rejected | unavailable | unknown
  category: policy | legality | budget | duplicate | dominated | bug
  reason_code: stable enum or stable string code
  explanation: optional display text
}
```

Free text is commentary. It is not control flow.

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

Coverage targets are selected from journaled candidates, not from current
active/frozen queues.

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

Continuation jobs are the unit that `CampaignEngine` runs.

## Artifact Model

Campaign artifacts are split by purpose:

```text
checkpoint  exact simulator resume state
state       campaign executor bookkeeping
journal     durable decision facts and candidate pools
report      bounded inspection projection
diagnostic  optional large explanations and traces
export      explicit learning or analysis dataset
manifest    provenance, encoding, sizes, and references
```

### Checkpoint

Answers:

```text
Can the engine resume exactly from here?
```

Checkpoint may be opaque and compact. It must not be optimized for casual
reading. If it becomes large, use pooling, references, or content-addressed
objects. Do not delete resume truth to make the file pretty.

### State

Answers:

```text
What campaign executor work remains?
```

State may include internal queues. Public reports translate queues into
candidate coverage and outcomes.

### Journal

Answers:

```text
What decisions existed, what candidates existed, and where did later branches
come from?
```

Journal entries are compact facts with typed ids. Large explanations belong in
diagnostics.

### Report

Answers:

```text
What should a reader or tool inspect first?
```

Reports may summarize run status, milestone progress, terminal outcomes,
candidate coverage, notable blockers, and artifact references.

Reports must not inline full simulator sessions, full route maps, full combat
traces, planner score tables, or learning samples by default.

### Diagnostic

Answers:

```text
Why did a component rank, reject, or fail something?
```

Diagnostics are opt-in and may be large. They link back to artifact ids,
decision ids, candidate ids, branch ids, or checkpoint ids.

### Export

Answers:

```text
What dataset should another process train on or analyze?
```

Exports are explicit. A campaign report is not a dataset.

## Stable Command Contract

The target Rust surface is:

```text
branch_campaign_driver campaign run --seed <seed> --class ironclad --ascension <n> --mode <preset>
branch_campaign_driver campaign run --random-seed --mode explore

branch_campaign_driver campaign continue --from latest --rounds <n>
branch_campaign_driver campaign continue --from run:<id> --until Act2Start
branch_campaign_driver campaign continue --from scratch-latest --out scratch --rounds <n>

branch_campaign_driver campaign coverage plan --from latest --budget key
branch_campaign_driver campaign coverage execute --from latest --until Act2Start --out scratch

branch_campaign_driver campaign inspect --from latest --view summary
branch_campaign_driver campaign inspect --from latest --view decision --decision-id <id>
branch_campaign_driver campaign inspect --from scratch-latest --view coverage

branch_campaign_driver campaign artifacts list
branch_campaign_driver campaign artifacts show --from latest
branch_campaign_driver campaign artifacts prune --dry-run

branch_campaign_driver campaign export --from latest --kind learning-jsonl --out <path>
```

`tools/campaign.ps1` may remain only as:

```text
choose build profile
build driver if needed
translate a small set of convenience aliases
call the Rust command
print returned artifact refs
```

The wrapper must not own a command that Rust cannot run directly.

## Presets

Presets are typed request builders, not hidden policy:

```text
smoke     short health check
quick     small local run
focused   current policy head plus key blockers
explore   broad candidate coverage
deep      expensive continuation of important candidates
```

Every preset prints its expanded settings:

```text
round budget
coverage budget
search budget
capture policy
output intent
```

## Experiment Model

Old model:

```text
keep a few active branches
freeze the rest
hope the active branches were right
inspect after failure
```

Target model:

```text
record candidate pools in the journal
select coverage targets from candidate ids
continue targets to milestones
compare outcomes by decision and candidate
refine censored, close, or strategically important cases
```

This can still use internal queues. Public output describes candidate coverage
and milestone outcomes, not an internal queue as if it were the experiment
result.

## PowerShell Boundary

PowerShell is allowed to:

- locate the repository
- choose a build profile
- run `cargo build` when needed
- forward a stable command to Rust
- print returned stdout/stderr

PowerShell is not allowed to:

- decide artifact source semantics
- decide output artifact paths
- own scratch/latest behavior
- own milestone loops
- own coverage planning or execution
- write manifests or latest pointers
- parse reports as workflow control
- add one top-level switch per probe

If a new feature seems to require more PowerShell semantic code, implement the
feature in Rust first.

## Deprecated Concepts

These concepts must not appear in new design, normal help, or public examples
except as explicit legacy notes:

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
tests that assert one uncertain card/shop/relic choice is globally correct
```

## Testing Policy

Good tests:

- parser rejects ambiguous commands
- artifact store resolves and allocates typed refs
- writing commands create manifests through Rust
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

If a test makes the architecture worse to satisfy, delete or rewrite it.

## Migration Standard

Implementation can be staged. The design cannot be split into contradictory
intermediate truths.

Every migration step must remove one semantic owner from the wrong layer. A
change that only splits files, renames functions, compresses a payload, or
formats output is not architecture progress unless it also transfers ownership
or deletes a bad surface.

## Stop And Redesign Triggers

Stop implementation and redesign if a step requires:

- parsing display labels for identity
- adding a report field that is actually checkpoint, journal, diagnostic, or
  export data
- adding a PowerShell switch for a new experiment type
- making `rounds`, `latest`, `scratch`, or `source` mean different things by
  context
- testing strategy quality through a fixed card/relic/shop assertion
- adding a score or magic number because no owner exists for the concept
- using active/frozen as the answer to a candidate coverage question

## Definition Of Done

The campaign architecture migration is complete when:

- a direct Rust command can perform every maintained workflow
- PowerShell contains no campaign semantic logic
- `latest` and `scratch-latest` are artifact-store pointers only
- coverage execution can target historical decision candidates without
  active/frozen guessing
- reports are small bounded projections
- checkpoint, state, journal, diagnostic, export, and manifest ownership is
  visible in code
- inspect views are read-only and typed
- old compatibility names are absent from normal help and examples
- new strategy work can be developed without touching wrapper lifecycle code
