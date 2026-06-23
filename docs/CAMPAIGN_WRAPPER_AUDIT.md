# Campaign Wrapper Audit

`tools/campaign.ps1` started as a friendly runner for campaign experiments. It
now also owns scratch artifacts, continuation planning, coverage-gap execution,
milestone loops, manifest writing, and many inspect commands. That is why the
file grew past a maintainable size.

This file is the cleanup contract. The goal is not to make the wrapper smaller
by moving code around blindly. The goal is to stop one script from owning too
many concepts.

## Post-Split Diagnosis

The recent split reduced local reading cost, but it is not sufficient by
itself. A split is healthy only when it also makes ownership clearer. The next
cleanup work should therefore be judged by these boundaries, not by line count:

- **Public command surface:** `campaign.ps1` still exposes many switches. Some
  are real user workflows; others are thin driver adapters or historical
  debugging affordances. The wrapper needs an explicit decision about which
  workflows it owns.
- **Entry context shape:** dispatch now groups wrapper switches into
  `RunSwitchContext`, `CoverageGapSwitchContext`, and `InspectSwitchContext`.
  That is better than one flat field bag, but the boundary is still dynamic
  `pscustomobject` plumbing. Future changes should favor smaller
  request-specific contexts instead of adding more generic fields.
- **Side-effect boundaries:** building, driver execution, manifest writes,
  latest pointer writes, and artifact inspection are now in separate files, but
  they still communicate mostly by convention. Any new behavior should define
  which function owns the side effect.
- **Smoke coverage:** wrapper smoke tests now protect source/output/milestone
  routing. They do not prove strategic correctness, and they should not grow
  into a second driver test suite.

In short: file splitting was a scaffold, not the architecture goal. The next
phase is to shrink or classify the wrapper API and replace loose cross-module
field passing where it causes confusion.

## Current Size

Approximate physical line count after the request/source/output cleanup and
inspect probe consolidation:

- `tools/campaign_inspect.ps1`: 395 lines
- `tools/campaign.ps1`: 374 lines
- `tools/campaign_request.ps1`: 329 lines
- `tools/campaign_continuation.ps1`: 318 lines
- `tools/campaign_run_execution.ps1`: 298 lines
- `tools/campaign_wrapper_smoke.ps1`: 289 lines
- `tools/campaign_coverage_gaps.ps1`: 269 lines
- `tools/campaign_invocation.ps1`: 212 lines
- `tools/campaign_manifest.ps1`: 198 lines
- `tools/campaign_preflight.ps1`: 197 lines
- `tools/campaign_source.ps1`: 190 lines
- `tools/campaign_rounds.ps1`: 181 lines
- `tools/campaign_artifact_summary.ps1`: 170 lines
- `tools/campaign_milestones.ps1`: 165 lines
- `tools/campaign_artifact_refs.ps1`: 156 lines
- `tools/campaign_entry_dispatch.ps1`: 136 lines

Major regions:

| Region | Approx lines | Status |
| --- | ---: | --- |
| Help examples and synopsis | 35 | Useful as a quick reference |
| Parameter block | 180 | Too many feature flags in one entrypoint |
| Path globals and helper import | 20 | Fine |
| Source/build/output resolution | 130 | Narrower, but still in the wrapper |
| Entry dispatch | 130 | Moved out of main wrapper; switch fields are grouped, but still dynamic |
| Inspect helpers | 348 | Biggest remaining module; likely needs semantic pruning, not just splitting |
| Invocation helpers | 331 | Shared option and driver argument construction are still mixed |

## Why It Got This Large

The wrapper currently does all of these jobs:

- user-facing command shortcut
- build profile selector
- latest artifact locator
- scratch artifact locator
- run source selector
- output target selector
- resume and round-budget normalizer
- normal campaign runner
- coverage-gap planner/executor
- milestone continuation loop
- manifest writer
- command-file writer
- artifact shape inspector
- general inspect dispatcher
- detailed inspect flag adapter

Some of these belong in the wrapper. Most do not.

## Concepts That Must Stay Clear

These names are now the boundary:

```text
latest:
  A pointer to the current default campaign artifact. It is not the artifact
  itself and must not be inferred from scattered sidecar files.

scratch:
  A side artifact for experiments. It must not overwrite latest. Scratch has
  its own pointer at tools/artifacts/campaigns/scratch/latest.json.

source:
  The artifact a command reads from.

output:
  The artifact a command writes to.
```

Any command that reads one artifact and writes another must print and record:

```text
source=...
source-report=...
source-checkpoint=...
report=...
checkpoint=...
```

Dangerous behavior is forbidden:

```text
read from scratch, then silently write back to latest
```

Use clear aliases for new user-facing commands:

```powershell
-FromScratchLatest  # read latest scratch artifact
-OutScratch         # write new scratch artifact
```

The old names can remain as compatibility aliases, but new docs and examples
should prefer the clearer names.

`-More` is retired. It used one mutable `latest` location as source, output,
and round-budget context at the same time. That was the root cause behind
ambiguous "continue latest", "scratch", and "coverage-gap" behavior. New
continuation commands must spell out their source:

```powershell
.\tools\campaign.ps1 -From latest -Continue
.\tools\campaign.ps1 -From run:<id> -Continue -Rounds 1
```

Normal campaign runs now write to:

```text
tools/artifacts/campaigns/runs/<run-id>/
  campaign.json
  checkpoint.json
  manifest.json
  command.txt
  log.txt
```

`tools/artifacts/campaigns/latest.json` is the only mutable latest pointer for
`-From latest`. The older `latest.campaign.json`, `latest.checkpoint.json`, and
sidecar text files are available only through explicit `-From legacy-latest`;
new code should not silently fall back to them or write them as source of truth.
`tools/artifacts/campaigns/scratch/latest.json` is the corresponding pointer
for `-FromScratchLatest`; scratch source selection no longer guesses by newest
file modification time.

## Keep In Wrapper

These are the wrapper's real job:

- parameter parsing
- choosing build profile
- choosing source and output artifact refs
- rendering the driver command
- launching the driver
- printing a short preflight summary
- writing a wrapper manifest

If a feature does not fit this list, it needs a strong reason to remain here.

## Public Command Surface

The wrapper should expose a small set of workflows, not every Rust driver flag.
Current parameters fall into these groups.

Maintained workflow surface:

- **Run selection:** positional `Seed`, `-Last`, `-Mode`, `-Ascension`,
  `-Domain`, and `-Class`.
- **Source and output:** `-From`, `-FromScratchLatest`, `-Scratch` /
  `-OutScratch`, and `-RunLabel`.
- **Continuation:** `-Continue`, `-Rounds`, `-UntilRound`, `-UntilMilestone`,
  `-MilestoneStepRounds`, `-MilestoneMaxRounds`, and `-MilestoneStop`.
- **Coverage-gap workflow:** `-PlanCoverageGaps`, `-ContinueCoverageGaps`,
  `-CoverageGapRoute`, `-CoverageGapRouteMissing`,
  `-CoverageGapEventBoundary`, `-CoverageGapEventBoundaryMissing`,
  `-CoverageGapLimit`, `-CoverageGapCandidatesPerDecision`,
  `-CoverageGapBucket`, `-CoverageGapEventId`, `-CoverageGapLane`,
  `-CoverageGapOriginSource`, `-CoverageGapProgress`,
  `-CoverageGapIntent`, `-CoverageGapExecution`, and
  `-CoverageGapMilestoneTarget`.
- **Inspect workflow:** `-Inspect`, `-InspectArtifacts`, `-InspectState`,
  `-InspectDecisionObservations`, `-InspectJournal`,
  `-InspectLineageDecisions`, `-InspectCoverageGapMilestoneSummary`,
  `-InspectCoverageGapTargetState`, `-ExportLearningDataset`, and inspect
  filters `-InspectIndex`, `-InspectAct`, `-InspectFloor`,
  `-InspectBoundary`, `-InspectQuery`.
- **Driver probes:** `-Probe` and `-ProbeBoss`. These are debugging entrypoints,
  but `-Probe` is the maintained way to reach thin Rust inspect probes without
  growing the top-level switch list.
- **Operational controls:** `-DryRun`, `-Log`, `-NoProgress`,
  `-VerboseProgress`, `-Diagnose`, `-Perf`, `-DebugBuild`, `-Build`,
  `-BuildProfile`, and `-DriverArgs`.

Advanced campaign knobs:

- `-ExperimentWallMs`, `-SearchWallMs`, `-SearchMaxNodes`,
  `-CombatRetryWallMs`, `-ActiveLineageDiversity`, `-BranchExamples`,
  `-VictoryHpPercent`, `-BossRelicAxes`, `-BossSegments`,
  `-AutoCaptureCombat`, and `-AutoCaptureRoot`.

These are still wrapper-visible because they are frequently used to shape a
campaign run. They should not multiply into many strategy-specific switches;
if a setting is only for one driver experiment, prefer `-DriverArgs` or a Rust
driver command.

Compatibility or retiring surface:

- `-More`: retired; kept only to emit a clear error.
- `-InspectScratchLatest`: compatibility alias for `-FromScratchLatest`.
- `-InspectShopEvidence`, `-InspectCardRewardEvidence`,
  `-InspectCampfireEvidence`, `-InspectDeckMutation`,
  `-InspectRouteEvidence`, `-InspectLastAutoCombat`, `-InspectCombatLab`, and
  `-InspectFinalBossCombat`: compatibility aliases for `-Probe <kind>`.
- `-MaxRounds`: legacy round-budget spelling. Prefer `-Rounds` or
  `-UntilRound` for continuation-style commands.
- positional remaining `ExtraArgs`: compatibility capture only. New usage
  should use explicit `-DriverArgs`.

Mixed surface:

- `-InspectShopChallenge` and `-ChallengeMaxPlans` / `-ChallengeDepth` /
  `-ChallengeMaxBranches`: currently useful, but still shaped like a one-off
  probe. It should either become a named shop experiment or move to direct
  driver usage; do not copy this pattern for new probes.

## Moved Out Of Wrapper

Artifact helpers are now loaded through a facade:

```text
tools/campaign_artifacts.ps1
```

The facade owns only shared artifact state and imports smaller helpers:

```text
tools/campaign_artifact_paths.ps1
tools/campaign_artifact_refs.ps1
tools/campaign_artifact_pointers.ps1
tools/campaign_artifact_legacy.ps1
tools/campaign_artifact_source.ps1
```

These helpers own:

- latest/scratch artifact refs
- legacy latest sidecar path compatibility
- run latest and scratch latest pointer files
- run source artifact selection
- run/scratch output artifact selection from typed request output intent
- latest campaign mode/config reads
- explicit `CampaignPathContext` initialization; artifact helpers no longer
  depend on `$CampaignDir` / `$ScratchCampaignDir` being present in the parent
  dot-source scope

Artifact summary helpers now live in:

```text
tools/campaign_artifact_summary.ps1
```

This helper owns:

- artifact size rendering
- artifact JSON shape summaries
- `-InspectArtifacts` contract output formatting

Invocation helpers now live in:

```text
tools/campaign_invocation.ps1
```

This helper owns:

- command-line rendering
- shared campaign driver option context extraction
- shared campaign driver option rendering
- shared driver option context is passed explicitly to campaign, coverage-gap,
  and milestone driver command builders
- continuation round-budget argument rendering

Normal run execution helpers now live in:

```text
tools/campaign_run_execution.ps1
```

This helper owns:

- run driver identity argument rendering
- normal run driver argument context construction, including resume/output,
  round budget, learning export, shared options, and combat segment labeling
- normal campaign run command context construction
- normal campaign run command execution through an explicit run context
- logged driver invocation

Manifest helpers now live in:

```text
tools/campaign_manifest.ps1
```

This helper owns:

- wrapper manifest JSON writing
- common wrapper manifest fields
- normal campaign run manifest shape
- structured driver passthrough provenance in manifests
- primary driver command-file recording through an explicit run context
- primary driver command-file recording now requires an output artifact; the old
  `latest.seed.txt` / `latest.command.txt` sidecar write fallback is gone

Preflight helpers now live in:

```text
tools/campaign_preflight.ps1
```

This helper owns:

- normal campaign run preflight output through an explicit run context
- continuation preflight context shape
- coverage-gap continuation preflight output rendering

Continuation entry helpers now live in:

```text
tools/campaign_continuation.ps1
```

This helper owns:

- an explicit continuation entry context boundary
- continuation entry context construction
- continuation operation dispatch through `CampaignEntryRequestV1`, with old
  context boolean fallback removed
- continuation source context validation
- coverage-gap continuation command context assembly
- continuation preflight context handoff
- continuation dry-run dispatch
- continuation execution dispatch
- explicit record/manifest context handoff for continuation helpers
- continuation round-budget consumption from `RunRoundContext`; continuation
  no longer reinterprets raw `-Rounds` / `-UntilRound` / `-MaxRounds` inputs

Round-budget helpers now live in:

```text
tools/campaign_rounds.ps1
```

This helper owns:

- round-budget role checks from typed request kind when available
- mutual exclusion validation for `-Rounds`, `-UntilRound`, and legacy `-MaxRounds`
- `-UntilMilestone` round-budget validation and stop-mode normalization
- normal campaign resume source validation
- normal campaign resume and round-budget driver argument rendering
- coverage-gap continuation default/additional round-budget rendering, so
  continuation commands share the same round-budget context instead of
  re-running round interpretation

Milestone helpers now live in:

```text
tools/campaign_milestones.ps1
```

This helper owns:

- milestone status extraction from a report
- milestone resume driver argument rendering
- the wrapper-level milestone continuation loop
- milestone loop execution through an explicit context containing report,
  checkpoint, driver, target, stop mode, and step/max round settings; milestone
  helpers no longer read outer wrapper globals such as `$RunOutputCampaignPath`
  or `$DriverExe`

Coverage-gap wrapper helpers now live in:

```text
tools/campaign_coverage_gaps.ps1
```

This helper owns:

- coverage-gap preset compatibility checks
- coverage-gap preset normalization into filter context
- coverage-gap execution-mode normalization
- coverage-gap filter argument rendering
- coverage-gap plan/continue driver argument rendering
- coverage-gap plan/continue driver argument builders take explicit limit,
  candidate, intent, and filter inputs; they no longer read outer wrapper
  globals

Coverage-gap execution helpers now live in:

```text
tools/campaign_coverage_gap_execution.ps1
```

This helper owns:

- coverage-gap continuation dry-run command rendering
- coverage-gap continuation execution orchestration
- coverage-gap milestone summary commands
- coverage-gap command recording and manifest write handoff from explicit
  contexts, not outer wrapper globals

Coverage-gap manifest helpers now live in:

```text
tools/campaign_coverage_gap_manifest.ps1
```

This helper owns:

- coverage-gap wrapper manifest shape
- coverage-gap source, filter, execution, and milestone metadata in manifests

Inspect argument helpers now live in:

```text
tools/campaign_inspect.ps1
```

This helper owns:

- inspect option context construction and driver-argument rendering without
  implicit global inspect-switch reads
- inspect entry context construction
- inspect entry dispatch from a resolved campaign source context
- inspect preflight and command execution
- deciding whether an inspect command should use summary mode
- mapping wrapper inspect switches to Rust driver flags
- rendering dataset export inspect arguments

Build freshness helpers now live in:

```text
tools/campaign_build.ps1
```

This helper owns:

- resolving wrapper build profile selectors
- rendering the driver executable path and cargo build args
- deciding whether the Rust driver binary needs rebuilding

Source/run identity helpers now live in:

```text
tools/campaign_source.ps1
```

This helper owns:

- resolving a wrapper source selector into a campaign artifact and run config
- resolving the source/mode/seed/ascension/class run context as one boundary
- source-read behavior from typed request source intent
- inheriting mode from source artifacts for continuation-style commands
- resolving seed, ascension, class, and domain defaults
- run identity inheritance only reads the selected source artifact; it no longer
  falls back to unrelated global `latest.checkpoint.json`

`tools/campaign.ps1` should consume the resolved source context from this helper.
It should not call artifact/run-config source lookup helpers directly in
continuation or inspect branches; otherwise `latest`, `scratch`, and `source`
semantics will drift again.

Main wrapper dispatch now consumes the typed request kind directly:

```text
plan_coverage_gaps / continue_coverage_gaps
  -> continuation entry

inspect
  -> inspect entry

run / continue_run
  -> normal campaign run entry
```

Unknown request kinds fail closed. The old implicit fall-through to a normal
campaign run is gone.

Request helpers now live in:

```text
tools/campaign_request.ps1
```

This helper owns:

- typed entry request classification (`run`, `continue_run`, `inspect`,
  `plan_coverage_gaps`, and `continue_coverage_gaps`)
- main-wrapper dispatch consumes the resolved request object directly instead
  of rebinding request switches into a second set of mutable variables
- source/output intent derivation for wrapper manifests
- derived operation switches used by the main wrapper after request resolution
- retired `-More` rejection
- wrapper parameter value normalization for manifests and command records
- wrapper bound-parameter context extraction
- inspect flag folding
- inspect selector switch classification
- `-FromScratchLatest` source/read interpretation; the old
  `-InspectScratchLatest` name remains only as a compatibility alias
- scratch output eligibility
- the derived `ReadsCampaignSource` flag

Run-only driver argument construction now happens only inside the `run` /
`continue_run` dispatch branch. Plan-only and inspect-only commands do not build
normal-run driver args as incidental state.

Invocation helpers now also own driver passthrough normalization. The main
wrapper only declares `-DriverArgs` and the compatibility remaining-argument
capture; `tools/campaign_invocation.ps1` combines, validates, and records the
effective passthrough args. Manifests write the split explicitly:

```text
driver_passthrough.explicit_driver_args
driver_passthrough.compatibility_extra_args
driver_passthrough.effective_args
```

Output artifact allocation happens after typed round/request validation. For
example, invalid `-UntilMilestone` use on a plan-only command now fails before
the wrapper creates scratch output directories or renders continuation
preflight. Output artifact resolution also consumes the request object directly
instead of receiving a second set of request-derived booleans.

The wrapper no longer exposes targeted continuation. The Rust driver still
supports `--plan-targeted-continuation` / `--execute-targeted-continuation` for
manual archaeology, but `tools/campaign.ps1` is now centered on the current
journal/coverage-gap continuation path.

Raw driver passthrough is intentionally narrow. Prefer the explicit
`-DriverArgs @("--driver-flag", "value")` parameter for temporary direct Rust
driver flags. The remaining positional `ExtraArgs` capture exists only as
compatibility for older ad-hoc invocations and should not be documented as a
primary interface. Unknown single-dash arguments such as `-SomeWrapperTypo`
fail at the wrapper boundary instead of being silently forwarded. Driver
passthrough should use Rust-style `--flag` syntax. PowerShell still accepts
unambiguous abbreviations for declared wrapper parameters, so this is not a
full replacement for a future typed subcommand surface.

## Still Move Out Of Wrapper

These pieces are useful but should not live in the main script long term:

- residual compatibility switches that may no longer earn wrapper-level
  visibility after the latest/source/output cleanup
- targeted continuation remains available only through direct Rust driver flags
  if old data archaeology needs it; it is not a maintained wrapper workflow

## Candidates To Delete Or Degrade

Do not preserve every old feature just because it exists.

Review these before adding more wrapper logic:

- very specific inspect flags that only forward one driver flag; they may be
  better as direct driver examples or a single `-InspectKind` style adapter.
- long comment-help examples; active docs can carry examples instead of the
  wrapper header carrying every workflow.

### Inspect Surface Audit

`tools/campaign_inspect.ps1` is now structurally separate from the main
wrapper, but its public switch surface is still too broad. The important
boundary is not the file split; it is whether an inspect command is a wrapper
workflow or only a thin Rust-driver probe.

Wrapper-owned inspect workflows:

- `-Inspect`: the normal saved-artifact summary. It selects the resolved source
  artifact, prints wrapper preflight, and asks the driver for a compact report.
- `-InspectArtifacts`: the artifact contract view. This is entirely wrapper
  owned because it explains report/checkpoint/manifest/log/command paths.
- `-InspectCoverageGapMilestoneSummary` and
  `-InspectCoverageGapTargetState`: these belong with coverage-gap
  continuation and milestone orchestration. They are not generic driver probes.
- `-InspectJournal`, `-InspectLineageDecisions`,
  `-InspectDecisionObservations`, and `-ExportLearningDataset`: these are
  current campaign-learning and journal workflows. They should stay visible
  while the journal/coverage-gap loop is active, but they should converge toward
  one journal/learning inspect surface instead of accumulating flags forever.

Thin driver probes:

- `-InspectShopEvidence`
- `-InspectCardRewardEvidence`
- `-InspectCampfireEvidence`
- `-InspectDeckMutation`
- `-InspectRouteEvidence`
- `-InspectLastAutoCombat`
- `-InspectCombatLab`
- `-InspectFinalBossCombat`
- `-ProbeBoss`

These are useful while debugging, but they are not separate wrapper concepts.
They are now available through a typed probe selector, for example:

```powershell
.\tools\campaign.ps1 -Inspect -Probe shop-evidence
.\tools\campaign.ps1 -Inspect -Probe combat-lab -ProbeBoss
```

The older `-InspectShopEvidence`-style switches remain compatibility aliases
for now. Do not add more of them.

`-InspectState` is different: the Rust driver has no `--inspect-state` flag.
It means "run checkpoint inspect without summary mode", so it should stay with
the core inspect surface unless the driver grows a real typed state subcommand.

`-InspectShopChallenge` is mixed. It is currently a driver rollout probe with
several wrapper-exposed tuning knobs. It should not grow further as a public
wrapper workflow; either the driver owns it directly, or the wrapper exposes a
small named experiment preset.

Near-term rule:

```text
Do not add another top-level inspect switch unless it is wrapper-owned.
For driver-only diagnostics, prefer a typed probe selector or a direct driver
command in docs.
```

This keeps `campaign.ps1` from becoming an index of every temporary Rust
debugging flag.

Deletion rule:

```text
If a wrapper feature is not used by current campaign, journal, coverage-gap, or
artifact workflows, either move it to docs as a direct driver command or delete
it.
```

## No More Hidden Expansion

New wrapper behavior should pass this checklist:

- Does it define source and output explicitly?
- Can it be explained without knowing the internals of `latest` sidecar files?
- Is it a wrapper responsibility, not a driver responsibility?
- Does it add less than roughly 40 lines to the main script?
- If larger, does it belong in a helper script or Rust driver subcommand?
- Does it have one dry-run command that proves the rendered source/output?

If the answer is no, do not add it to `tools/campaign.ps1`.

## Next Cleanup Order

1. Consider replacing many specific inspect switches with a smaller typed
   inspect adapter only after current callers are audited.
2. Review residual compatibility switches and remove any that belong only in
   direct driver commands.

This sequence reduces cognitive load without changing campaign strategy.
