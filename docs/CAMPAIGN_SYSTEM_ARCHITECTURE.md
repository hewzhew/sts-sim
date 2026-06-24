# Campaign System Architecture

This document defines the target campaign architecture. It is intentionally
stricter than the current compatibility wrapper. When current code disagrees
with this document, treat the disagreement as migration debt unless a later
design explicitly changes the target.

## Problem Statement

The campaign workflow grew from a convenience PowerShell wrapper into two
application layers:

```text
PowerShell wrapper:
  source/latest/scratch selection
  continuation defaults
  milestone loops
  coverage-gap orchestration
  manifest and command provenance
  build and inspect dispatch

Rust driver:
  campaign execution
  checkpoint/report/journal IO
  coverage-gap planning/execution
  inspect rendering
  scheduler and combat search integration
```

This split is the source of repeated confusion. The same words, such as
`latest`, `scratch`, `continue`, `rounds`, and `milestone`, can be interpreted
in two places. That makes command behavior hard to predict, artifacts hard to
audit, and later strategy work dependent on wrapper accidents.

## Target Ownership

There must be one campaign application layer.

```text
User command
  -> Campaign CLI
      -> CampaignApp
          -> ArtifactStore
          -> ExperimentPlanner
          -> CampaignEngine
          -> InspectRenderer
```

PowerShell may launch this CLI. PowerShell must not own campaign semantics.

## Components

### Campaign CLI

The CLI parses stable campaign commands and turns them into typed requests. It
does not run campaign logic directly.

Responsibilities:

- parse subcommands and flags
- validate that command combinations are meaningful
- render clear errors for invalid usage
- pass typed requests to `CampaignApp`

Non-responsibilities:

- infer artifact lifecycle from file names
- implement milestone loops
- implement coverage-gap selection
- mutate latest or scratch pointers directly

### CampaignApp

`CampaignApp` is the single top-level Rust service for campaign workflows.

Responsibilities:

- resolve source and output artifact intent
- decide whether a command reads, writes, or only inspects artifacts
- route requests to the engine, planner, artifact store, or renderer
- write command provenance through the artifact store

Hard rule: if a workflow changes campaign state, it must pass through
`CampaignApp`.

### ArtifactStore

`ArtifactStore` owns campaign artifact lifecycle.

Responsibilities:

- resolve `latest`, `run:<id>`, `scratch:<id>`, and explicit paths
- create run and scratch output directories
- read and write checkpoint, report, journal, state, diagnostics, and manifest
- update latest pointers
- record encoding and size metadata
- implement artifact list and prune operations

PowerShell and inspect renderers must not independently update pointers or
manifests.

### CampaignEngine

`CampaignEngine` executes campaign jobs.

Responsibilities:

- start new campaign runs from seed/domain/class configuration
- continue from checkpoint/report/journal artifacts
- run until a round budget or milestone target is reached
- emit progress events
- write exact resume state through `ArtifactStore`

The engine may internally use scheduled and parked work queues. Those queues are
implementation details, not the public experiment model.

### ExperimentPlanner

`ExperimentPlanner` owns deliberate branch exploration.

Responsibilities:

- consume `CampaignJournal` decision candidate pools
- classify key decision nodes
- assign continuation jobs under an explicit budget
- track whether each candidate is untried, target-only, continued to a
  milestone, terminal, or blocked by combat budget
- balance coverage across decision type and candidate lane

The planner does not decide card or route quality by itself. It decides which
candidate needs observation next.

### InspectRenderer

Inspect renderers are read-only views over artifacts.

Responsibilities:

- summarize run status
- inspect checkpoint state
- inspect journal candidate pools
- inspect lineage and coverage
- inspect combat trajectories when present

Inspect renderers must not mutate campaign state or rely on PowerShell-specific
paths.

### PowerShell Launcher

`tools/campaign.ps1` should become a launcher only.

Allowed:

- choose build profile
- build `branch_campaign_driver` when needed
- pass arguments to the Rust campaign CLI
- print the exact command and artifact paths returned by Rust

Forbidden:

- parse or write campaign reports/checkpoints/journals
- decide source/latest/scratch semantics
- implement milestone loops
- implement coverage-gap orchestration
- write campaign manifests
- add one switch per temporary inspect probe

## Experiment Model

The target model is decision-candidate coverage, not active/frozen branch
guessing.

```text
CampaignJournal candidate pool
  -> key decision classifier
  -> coverage target
  -> continuation job
  -> milestone outcome
  -> outcome table
```

Small budgets should cover only key decision nodes. Larger budgets should
progressively refine within those nodes.

Example budget behavior:

```text
smoke:
  run one policy head to a short milestone

key:
  cover boss relic, route, major shop, event, and high-impact reward candidates

milestone:
  continue each selected key candidate to the next boss, act start, death, or
  combat-budget blocker

deep:
  refine child decisions under candidates whose outcomes are close, censored,
  or strategically important
```

The public report should explain this as candidate coverage. It should not ask
the user to reason in terms of active and frozen branch pools.

## Artifact Model

Campaign artifacts keep separate ownership:

```text
checkpoint  exact simulator resume state
state       scheduler/executor bookkeeping
journal     decision facts and candidate pools
report      bounded inspection projection
diagnostic  opt-in large explanations and traces
manifest    artifact provenance and references
```

Reports may reference state, journal, checkpoint, and diagnostics. Reports must
not become the only copy of those facts.

## Hard Invariants

1. Rust owns campaign semantics.
2. PowerShell is not allowed to interpret artifact lifecycle.
3. A user-visible campaign operation maps to one typed Rust command.
4. Every writing command produces a manifest through `ArtifactStore`.
5. Every inspect command is read-only.
6. `latest` and `scratch/latest` are pointers, not special report files.
7. `scheduled` and `parked` are executor internals, not the experiment model.
8. Coverage planning starts from journal candidates, not from branch rank.
9. Command strings are replay inputs; typed journal ids are analysis inputs.
10. Strategy policy must not be implemented in wrappers, labels, or report text.

## Migration Compatibility

Existing commands and artifacts may be supported while migrating, but
compatibility must be explicit:

- compatibility aliases should call the same Rust command path as the stable
  command
- compatibility code should not add new semantics
- deprecated wrapper-only behavior should fail loudly rather than silently map
  to a surprising new behavior
- old artifact readers may hydrate legacy shapes, but new writers should emit
  the target shape

Compatibility is not a reason to preserve the two-application-layer design.
