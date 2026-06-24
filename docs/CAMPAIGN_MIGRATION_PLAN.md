# Campaign Migration Plan

This plan migrates campaign ownership from the PowerShell wrapper back into the
Rust campaign application. It is intentionally larger than a wrapper cleanup:
the goal is to remove the two-application-layer failure mode.

The phases below are ownership transfers. A change does not count as progress
on this plan merely because it splits files, renames functions, compresses a
payload, or makes output prettier. It counts only when a campaign semantic
leaves the wrong layer, an obsolete public surface is deleted or loudly
deprecated, or an artifact boundary becomes enforceable.

## Completion Definition

The migration is complete when:

- `tools/campaign.ps1` is a launcher, not a campaign orchestrator
- Rust owns source/output/latest/scratch semantics
- Rust owns milestone loops
- Rust owns coverage-gap planning and execution
- Rust owns artifact manifests and pointers
- report, checkpoint, journal, and diagnostics follow
  [Campaign Artifact Architecture](CAMPAIGN_ARTIFACT_ARCHITECTURE.md)
- public reports describe decision candidate coverage, not active/frozen as the
  user-facing experiment model

## Phase 0: Freeze Wrapper Semantics

Goal: prevent the current wrapper from growing.

Tasks:

- mark `tools/campaign.ps1` and `tools/campaign_*.ps1` as compatibility
  launcher code
- reject new PowerShell-owned campaign semantics
- update docs so new work targets the Rust campaign app
- remove examples that encourage wrapper-only concepts

Done when:

- docs point to `CAMPAIGN_SYSTEM_ARCHITECTURE.md`
- new wrapper changes are limited to launch/build compatibility

Not done by:

- moving PowerShell functions into more `campaign_*.ps1` files
- adding a new wrapper probe switch
- adding a new compatibility alias for a Rust feature that does not exist yet
- documenting the current wrapper behavior as if it were the target design

## Phase 1: Rust ArtifactStore

Goal: make Rust the only owner of artifact source and output resolution.

Tasks:

- add a Rust `ArtifactStore` facade for run, scratch, latest, explicit path,
  report, checkpoint, journal, state, diagnostics, and manifest paths
- move latest and scratch pointer resolution into Rust
- move output directory creation into Rust
- move manifest writing into Rust
- make PowerShell pass source/output selectors without interpreting them

Done when:

- `campaign run`, `campaign continue`, and `campaign inspect` can resolve
  sources without PowerShell path logic
- PowerShell does not read or write campaign manifest or latest pointer files
- dry-run can display the same typed artifact resolution Rust will use

Not done by:

- calling Rust for only one path while PowerShell still owns the surrounding
  lifecycle
- preserving separate PowerShell meanings for latest, scratch, continue, or
  rounds

## Phase 2: Rust CampaignApp

Goal: provide one top-level Rust application boundary.

Tasks:

- introduce typed request structs for run, continue, inspect, coverage plan,
  coverage execute, and artifact commands
- route all campaign workflows through `CampaignApp`
- ensure invalid command combinations are rejected in Rust
- make PowerShell forward to `CampaignApp` rather than branching per workflow

Done when:

- a direct Rust command can perform every maintained workflow currently used
  through the wrapper
- wrapper code no longer chooses workflow semantics after parsing basic
  convenience options

## Phase 3: Rust Milestone Continuation

Goal: remove wrapper-owned milestone loops.

Tasks:

- implement milestone continuation inside `CampaignEngine`
- define milestone targets as typed values
- make `--rounds` mean additional campaign rounds
- make `--until` mean engine-owned milestone loop
- record milestone loop provenance in the manifest

Done when:

- `campaign continue --from latest --until Act2Start` works without wrapper
  loops
- no PowerShell file calls the driver repeatedly to implement a milestone
- milestone status is computed from Rust artifact state

## Phase 4: Rust Coverage Planner

Goal: make candidate coverage the default exploration model.

Tasks:

- define `CoverageTarget` from journal event id, candidate id, candidate
  command, source checkpoint, and intended milestone
- classify key decision nodes: boss relic, route, major shop, event, high-impact
  reward, critical campfire/deck mutation
- implement budgets: smoke, key, milestone, deep
- execute coverage jobs through `CampaignEngine`
- record target provenance in produced branches and reports

Done when:

- coverage plan and execute commands do not depend on active/frozen ordering
- reports can answer which key candidates are untried, target-only, continued,
  blocked by combat budget, terminal loss, or victory
- active/frozen are executor internals in output, not the primary user model

Not done by:

- increasing the active branch count
- renaming active/frozen to scheduled/parked in public output
- adding more rank terms to decide which frozen branch to thaw
- treating the current best policy head as the only branch worth continuing

## Phase 5: Rust Inspect And Artifact Commands

Goal: make inspect and artifact lifecycle independent of wrapper state.

Tasks:

- implement inspect views as Rust subcommands
- implement artifact list/show/prune in Rust
- remove wrapper-written summaries that duplicate Rust inspect output
- ensure inspect is read-only

Done when:

- `campaign inspect --from latest --view ...` covers maintained inspect needs
- artifact pruning protects latest and scratch latest through Rust
- wrapper inspect functions are deleted or only call Rust

## Phase 6: Retire Compatibility Surface

Goal: remove misleading old entry points.

Tasks:

- remove or hard-fail `-More`
- remove wrapper-only scratch/latest shortcuts
- remove top-level probe switches that have stable Rust inspect equivalents
- remove stale docs and tests that lock old wrapper behavior

Done when:

- `rg "-More|FromScratchLatest|InspectScratchLatest|latest.campaign"` only finds
  explicit deprecation notes or legacy readers
- `tools/campaign.ps1` is small enough to audit as a launcher

## Non-Goals

This migration does not solve strategy quality by itself. It creates the
architecture needed to study strategy without wrapper confusion.

Out of scope for this migration:

- tuning card reward or shop policy
- changing combat search behavior
- training models
- adding new game mechanics

## Risk Controls

- Do not preserve compatibility by duplicating semantics in both Rust and
  PowerShell.
- Prefer loud failure over surprising fallback.
- Keep old artifact readers only as readers.
- New writers should emit the target artifact shape.
- Every phase must reduce the amount of campaign semantic code in PowerShell.
- If a migration step adds more wrapper semantic code than it removes, stop and
  redesign the step.
- If a migration step requires parsing labels, command prefixes, or report prose
  as identity, stop and add a typed field instead.
- If a migration step makes a report larger because it lacks a better owner for
  the data, stop and define the owner before writing the field.

## First Implementation Cut

The first code migration should be Phase 1:

```text
Rust ArtifactStore source/output resolver
  -> PowerShell forwards selectors
  -> Rust writes manifest
  -> wrapper manifest/path code begins deletion
```

Do not start by renaming wrapper functions, splitting wrapper files, or
polishing existing PowerShell modules. That preserves the wrong architecture.
