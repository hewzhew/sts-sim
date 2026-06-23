# Campaign Wrapper Usage

`tools/campaign.ps1` is the maintained friendly wrapper for
`branch_campaign_driver`. It should stay a thin entrypoint: source/output
selection, build profile selection, dry-run rendering, and common convenience
defaults. Campaign semantics belong in the Rust driver and compiler layers.

## Primary Commands

```powershell
.\tools\campaign.ps1
.\tools\campaign.ps1 -Mode quick
.\tools\campaign.ps1 521
.\tools\campaign.ps1 -Last
```

- no arguments: run an explore campaign on a random seed
- positional seed: run the same campaign on a fixed seed
- `-Mode quick`: shorter random-seed smoke run
- `-Last`: reuse the last non-dry-run campaign seed

## Continuation

```powershell
.\tools\campaign.ps1 -From latest -Continue
.\tools\campaign.ps1 -From latest -Continue -Rounds 1
.\tools\campaign.ps1 -From latest -Continue -UntilRound 33
.\tools\campaign.ps1 -From latest -Continue -UntilMilestone Act2Start
```

Continuation must explicitly state its source with `-From latest` or
`-From run:<id>`. `-From latest` reads the `latest.json` pointer written by new
campaign runs. Scratch experiments have their own `scratch/latest.json` pointer
and can be selected with `-FromScratchLatest` or `-From scratch:<id>`. Old
`latest.campaign.json` / `latest.checkpoint.json` sidecars must be selected
explicitly with `-From legacy-latest`. The retired `-More` shortcut mixed
source, output, and round budget semantics and should not be used.

## Coverage-Gap Continuation

```powershell
.\tools\campaign.ps1 -From latest -PlanCoverageGaps
.\tools\campaign.ps1 -From latest -PlanCoverageGaps -CoverageGapRouteMissing
.\tools\campaign.ps1 -From latest -ContinueCoverageGaps -Rounds 1
.\tools\campaign.ps1 -From latest -ContinueCoverageGaps -Scratch -RunLabel gap-probe -Rounds 1
.\tools\campaign.ps1 -From latest -ContinueCoverageGaps -CoverageGapExecution milestone -UntilMilestone Act2Start -Scratch
.\tools\campaign.ps1 -FromScratchLatest -ContinueCoverageGaps -OutScratch -RunLabel gap-probe -CoverageGapExecution target_only
```

Coverage-gap continuation is the preferred way to revisit unobserved journal
candidates. Use scratch output when probing without updating latest.

Older targeted continuation wrapper switches (`-PlanTargets` and
`-ContinueTargets`) were removed from `tools/campaign.ps1`. The Rust driver
still has `--plan-targeted-continuation` and `--execute-targeted-continuation`
for direct archaeology, but maintained wrapper workflows should use
coverage-gap continuation.

## Inspect

```powershell
.\tools\campaign.ps1 -Inspect
.\tools\campaign.ps1 -InspectArtifacts
.\tools\campaign.ps1 -InspectState -InspectIndex 0
.\tools\campaign.ps1 -InspectScratchLatest -InspectState -InspectIndex 0
.\tools\campaign.ps1 -InspectDecisionObservations -InspectQuery "Iron Wave"
.\tools\campaign.ps1 -InspectJournal -InspectQuery "shop"
.\tools\campaign.ps1 -InspectLineageDecisions -InspectIndex 0
.\tools\campaign.ps1 -InspectLineageDecisions -InspectQuery "CompleteWithinBudget"
```

Inspect commands read an artifact and render a report; they should not mutate
campaign state.

## Strategy Evidence Inspectors

```powershell
.\tools\campaign.ps1 -Probe shop-evidence -InspectIndex 0
.\tools\campaign.ps1 -Probe shop-challenge -InspectIndex 0
.\tools\campaign.ps1 -Probe card-reward-evidence -InspectIndex 0
.\tools\campaign.ps1 -Probe deck-mutation -InspectIndex 0
.\tools\campaign.ps1 -Probe campfire-evidence -InspectIndex 0
.\tools\campaign.ps1 -Probe route-evidence -InspectIndex 0
```

These are debugging tools for compiler/evidence layers. They should explain
inputs and candidate structure, not create new policy behavior in PowerShell.
The older `-InspectShopEvidence`-style switches remain compatibility aliases,
but new examples should prefer `-Probe <kind>` so the wrapper does not keep
adding one top-level switch per Rust driver probe.

## Combat And Learning Inspectors

```powershell
.\tools\campaign.ps1 -Probe last-auto-combat -InspectIndex 0
.\tools\campaign.ps1 -Probe combat-lab -InspectIndex 0
.\tools\campaign.ps1 -Probe combat-lab -ProbeBoss -InspectIndex 0
.\tools\campaign.ps1 -Inspect -ExportLearningDataset tools\artifacts\learning\latest.learning.jsonl
```

## Coverage-Gap Inspectors

```powershell
.\tools\campaign.ps1 -InspectCoverageGapMilestoneSummary -CoverageGapRouteMissing
.\tools\campaign.ps1 -InspectScratchLatest -InspectCoverageGapMilestoneSummary -CoverageGapRouteMissing
.\tools\campaign.ps1 -InspectScratchLatest -InspectCoverageGapMilestoneSummary -CoverageGapRoute
.\tools\campaign.ps1 -InspectScratchLatest -InspectCoverageGapTargetState -CoverageGapRoute -InspectIndex 1
```

## Build And Diagnostics

```powershell
.\tools\campaign.ps1 -DryRun
.\tools\campaign.ps1 -NoProgress
.\tools\campaign.ps1 -VerboseProgress
.\tools\campaign.ps1 -Diagnose
.\tools\campaign.ps1 -Perf
.\tools\campaign.ps1 -DebugBuild
.\tools\campaign.ps1 -Build
.\tools\campaign.ps1 -BuildProfile release-final
.\tools\campaign.ps1 -Mode quick -Scratch -Log
.\tools\campaign.ps1 -Mode quick -AutoCaptureCombat -AutoCaptureRoot tools\artifacts\tmp\ml_capture_seed123
.\tools\campaign.ps1 -Mode quick -DriverArgs @("--combat-search-option", "segment=turn")
```

Use `-DryRun` first when checking source/output semantics. Use `-Scratch` for
experiments that should not update latest. Use `-DriverArgs` only for explicit
Rust driver passthrough. Driver passthrough flags should use Rust-style
`--flag` syntax; new common workflows should become typed wrapper parameters
instead of accumulating raw passthrough examples. Wrapper manifests record
driver passthrough provenance under `driver_passthrough`, split into explicit
`-DriverArgs`, compatibility remaining args, and the effective forwarded args.

## High Ascension Presets

```powershell
.\tools\campaign.ps1 -Ascension 20 -Mode quick
.\tools\campaign.ps1 -Domain a20 -Mode quick
.\tools\campaign.ps1 -Domain a20 -Mode explore -BossRelicAxes
```

`-BossRelicAxes` gives boss relic lineages separate active/frozen branch
budgets and is useful when comparing high-impact relic choices.

## Combat Segment Compatibility

```powershell
.\tools\campaign.ps1 -BossSegments
```

Boss combats already stay on complete-win search by default.
`-BossSegments` allows turn-segment continuation inside boss combats while
debugging combat strategy; it can be slower.
