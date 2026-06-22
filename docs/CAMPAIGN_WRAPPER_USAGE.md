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
`-From run:<id>`. The retired `-More` shortcut mixed source, output, and round
budget semantics and should not be used.

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
.\tools\campaign.ps1 -InspectShopEvidence -InspectIndex 0
.\tools\campaign.ps1 -InspectShopChallenge -InspectIndex 0
.\tools\campaign.ps1 -InspectCardRewardEvidence -InspectIndex 0
.\tools\campaign.ps1 -InspectDeckMutation -InspectIndex 0
.\tools\campaign.ps1 -InspectCampfireEvidence -InspectIndex 0
.\tools\campaign.ps1 -InspectRouteEvidence -InspectIndex 0
```

These are debugging tools for compiler/evidence layers. They should explain
inputs and candidate structure, not create new policy behavior in PowerShell.

## Combat And Learning Inspectors

```powershell
.\tools\campaign.ps1 -InspectLastAutoCombat -InspectIndex 0
.\tools\campaign.ps1 -InspectCombatLab -InspectIndex 0
.\tools\campaign.ps1 -InspectCombatLab -ProbeBoss -InspectIndex 0
.\tools\campaign.ps1 -Inspect -ExportLearningDataset tools\artifacts\learning\latest.learning.jsonl
```

## Coverage-Gap Inspectors

```powershell
.\tools\campaign.ps1 -InspectCoverageGapMilestoneSummary -CoverageGapRouteMissing
.\tools\campaign.ps1 -InspectScratchLatest -InspectCoverageGapMilestoneSummary -CoverageGapRouteMissing
.\tools\campaign.ps1 -InspectScratchLatest -InspectCoverageGapMilestoneSummary -CoverageGapRoute
.\tools\campaign.ps1 -InspectScratchLatest -InspectCoverageGapTargetState -CoverageGapRoute -InspectIndex 1
```

## Targeted Continuation

```powershell
.\tools\campaign.ps1 -PlanTargets
.\tools\campaign.ps1 -From latest -ContinueTargets -Rounds 1
.\tools\campaign.ps1 -From latest -ContinueTargets -Scratch -Rounds 1
```

This is an older sibling-continuation workflow. Prefer coverage-gap
continuation unless specifically investigating the legacy targeted path.
Execution-style targeted continuation writes before/after decision-outcome
datasets into the selected output artifact. Use `-Scratch` for experiments that
should not update latest.

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
```

Use `-DryRun` first when checking source/output semantics. Use `-Scratch` for
experiments that should not update latest.

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
