# Campaign Migration Gates

This file tracks migration gates for the campaign architecture. The target
architecture is defined in
[Campaign System Architecture](CAMPAIGN_SYSTEM_ARCHITECTURE.md). This file does
not redefine that architecture.

A migration step counts only when it transfers semantic ownership to the right
Rust component, deletes a misleading public surface, or enforces an artifact
boundary.

A step does not count merely because it:

- splits a large file
- renames functions
- compresses a payload
- formats output
- moves PowerShell logic into another PowerShell file

## Gate 1: Wrapper Quarantine

Goal: stop `tools/campaign.ps1` from growing into a second campaign
application.

Done when:

- PowerShell docs call the wrapper compatibility launch code.
- New wrapper changes are limited to build/profile/invocation compatibility.
- New campaign semantics are rejected unless they already exist in Rust.
- Old aliases either fail loudly or forward to Rust without changing semantics.

Evidence:

```powershell
rg -n --glob '*.ps1' "Invoke-CampaignUntilMilestone|PowerShell-owned|latest\.campaign|FromScratchLatest|\\bMore\\b" tools
```

Remaining hits must be compatibility notes, loud failures, or legacy readers.

## Gate 2: Rust ArtifactStore And CampaignApp

Goal: make Rust the owner of artifact lifecycle and request routing.

Done when:

- Rust resolves `latest`, `scratch-latest`, `run:<id>`, `scratch:<id>`, and
  explicit archaeology paths.
- Rust allocates run and scratch outputs.
- Rust writes manifests and latest pointers.
- Rust has typed requests for run, continue, inspect, coverage plan, coverage
  execute, artifact, and export workflows.
- PowerShell no longer constructs artifact paths for maintained workflows.

Evidence:

```powershell
cargo test --bin branch_campaign_driver campaign_artifact_store --quiet
cargo check --bin branch_campaign_driver
.\tools\campaign.ps1 -Mode quick -DryRun
```

The dry-run should show a Rust request or Rust driver command, not a
PowerShell-owned lifecycle.

## Gate 3: Rust CampaignEngine Continuation

Goal: make continuation behavior one engine concept.

Done when:

- Rust owns additional-round continuation.
- Rust owns milestone continuation.
- `rounds` means additional rounds in every maintained path.
- No PowerShell file calls the driver repeatedly to implement milestone
  behavior.

Evidence:

```powershell
rg -n --glob '*.ps1' "Invoke-CampaignUntilMilestone|MilestoneLoop|milestone-loop-command" tools
.\tools\campaign.ps1 -Mode quick -UntilMilestone Act1Boss -MilestoneStepRounds 1 -MilestoneMaxRounds 1 -DryRun
```

The dry-run should contain one Rust invocation with milestone flags, not a loop
template.

## Gate 4: Rust ExperimentPlanner

Goal: replace active/frozen guessing with journal candidate coverage.

Done when:

- `coverage plan` selects targets from journaled decision candidates.
- `coverage execute` runs `ContinuationJob`s with target provenance.
- Reports can classify candidate progress as unobserved, target-only,
  continued, terminal, censored, budget-blocked, or invalid.
- Route, reward, shop, event, boss relic, and major deck-mutation candidates can
  enter coverage planning through typed ids.
- Active/frozen ordering is not the primary coverage mechanism.

Evidence:

```powershell
branch_campaign_driver campaign coverage plan --from latest --budget key
branch_campaign_driver campaign coverage execute --from latest --until Act2Start --out scratch
branch_campaign_driver campaign inspect --from scratch-latest --view coverage
```

Until the stable subcommand exists, equivalent legacy driver paths are allowed
only as migration scaffolding.

## Gate 5: Rust Inspect And Artifact Commands

Goal: make inspect and artifact lifecycle independent of wrapper state.

Done when:

- Rust owns maintained inspect views.
- Rust owns artifact list/show/prune.
- Inspect commands are read-only by construction.
- Wrapper inspect code is deleted or only forwards to Rust.

Evidence:

```powershell
branch_campaign_driver campaign inspect --from latest --view summary
branch_campaign_driver campaign artifacts list
branch_campaign_driver campaign artifacts prune --dry-run
```

No maintained inspect path should parse report prose or wrapper-only paths.

## Gate 6: Artifact Boundary Enforcement

Goal: stop report/checkpoint/journal/diagnostic/export from collapsing into one
large JSON blob.

Done when:

- default reports are bounded projections
- checkpoint stores exact resume state only
- state stores executor bookkeeping only
- journal stores decision facts and candidate pools only
- diagnostics store optional large explanations and traces
- exports store learning/analysis datasets explicitly
- manifests record refs, encodings, sizes, and provenance

Evidence:

```powershell
branch_campaign_driver campaign artifacts show --from latest
branch_campaign_driver campaign inspect --from latest --view artifact
```

The artifact view should show refs and sizes. It should not require loading a
large report to understand artifact ownership.

## Gate 7: Compatibility Retirement

Goal: remove the old public surface from normal use.

Done when:

- normal help and examples use stable Rust campaign commands
- `tools/campaign.ps1` is small enough to audit as a launcher
- retired names appear only in deprecation notes or legacy readers
- old artifact readers do not write new artifacts

Evidence:

```powershell
rg -n "-More|FromScratchLatest|InspectScratchLatest|latest\.campaign|selected_plan|active/frozen" README.md docs tools src
```

Hits in current docs should be explicit deprecation or migration notes, not
instructions for normal use.

## Stop Rules

Stop a migration step and redesign it if it requires:

- parsing display labels as identity
- adding a wrapper switch for a new experiment type
- adding report fields because no better artifact owner exists
- making source/output/rounds/scratch semantics context-dependent
- using active/frozen ordering as the answer to a coverage question
- writing a strategy-quality test for one uncertain card, shop, route, or relic
- preserving a confusing alias because old smoke tests expect it

## Current Priority Order

1. Finish retiring PowerShell-owned campaign workflow semantics.
2. Move artifact list/show/prune into Rust.
3. Move maintained inspect views into Rust.
4. Make coverage planning use journal candidate ids as the normal exploration
   interface.
5. Enforce report/checkpoint/state/journal/diagnostic/export boundaries in
   writers.
6. Return to strategy quality only after lifecycle and experiment surfaces stop
   fighting each other.
