# Campaign Wrapper Audit

`tools/campaign.ps1` started as a friendly runner for campaign experiments. It
now also owns scratch artifacts, continuation planning, coverage-gap execution,
milestone loops, manifest writing, and many inspect commands. That is why the
file grew past a maintainable size.

This file is the cleanup contract. The goal is not to make the wrapper smaller
by moving code around blindly. The goal is to stop one script from owning too
many concepts.

## Current Size

Approximate physical line count after the wrapper split:

- `tools/campaign.ps1`: 679 lines
- `tools/campaign_artifacts.ps1`: 520 lines
- `tools/campaign_invocation.ps1`: 494 lines
- `tools/campaign_coverage_gaps.ps1`: 391 lines
- `tools/campaign_inspect.ps1`: 184 lines
- `tools/campaign_targets.ps1`: 144 lines
- `tools/campaign_source.ps1`: 118 lines
- `tools/campaign_milestones.ps1`: 101 lines
- `tools/campaign_request.ps1`: 68 lines
- `tools/campaign_build.ps1`: 63 lines

Major regions:

| Region | Approx lines | Status |
| --- | ---: | --- |
| Help examples and synopsis | 35 | Useful as a quick reference |
| Parameter block | 180 | Too many feature flags in one entrypoint |
| Path globals and helper import | 20 | Fine |
| Source/build/output resolution | 130 | Narrower, but still in the wrapper |
| Continuation dispatch | 180 | Uses helpers, still has too much preflight text |
| Inspect dispatch | 20 | Delegated to inspect helper |
| Normal run dispatch | 25 | Delegated to invocation helper |

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
- targeted continuation planner/executor
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
  A side artifact for experiments. It must not overwrite latest.

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

`tools/artifacts/campaigns/latest.json` is the only mutable latest pointer.
The older `latest.campaign.json`, `latest.checkpoint.json`, and sidecar text
files are fallback inputs for old artifacts only; new code should not write
them as source of truth.

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

## Moved Out Of Wrapper

Artifact helpers now live in:

```text
tools/campaign_artifacts.ps1
```

This helper owns:

- latest/scratch artifact refs
- legacy latest sidecar path compatibility
- run source artifact selection
- run/scratch output artifact selection
- artifact size and shape summaries
- latest campaign mode/config reads

Invocation helpers now live in:

```text
tools/campaign_invocation.ps1
```

This helper owns:

- wrapper parameter value normalization for manifests
- wrapper bound-parameter context extraction
- command-line rendering
- run driver identity argument rendering
- shared campaign driver option context extraction
- shared campaign driver option rendering
- shared driver option context is passed explicitly to campaign, targeted,
  coverage-gap, and milestone driver command builders
- continuation round-budget argument rendering
- normal campaign run preflight and command execution
- manifest writing
- primary driver command-file recording
- logged driver invocation
- common wrapper manifest fields

Milestone helpers now live in:

```text
tools/campaign_milestones.ps1
```

This helper owns:

- milestone status extraction from a report
- milestone resume driver argument rendering
- the wrapper-level milestone continuation loop

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
- coverage-gap continuation dry-run command rendering
- coverage-gap continuation execution orchestration
- coverage-gap milestone summary commands
- coverage-gap wrapper manifest shape

Inspect argument helpers now live in:

```text
tools/campaign_inspect.ps1
```

This helper owns:

- inspect preflight and command execution
- deciding whether an inspect command should use summary mode
- mapping wrapper inspect switches to Rust driver flags
- rendering dataset export inspect arguments

Targeted continuation helpers now live in:

```text
tools/campaign_targets.ps1
```

This helper owns:

- targeted continuation dataset export command rendering
- targeted continuation plan/execute/effect command rendering
- targeted continuation dry-run command rendering
- targeted continuation execution orchestration

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
- inheriting mode from source artifacts for continuation-style commands
- resolving seed, ascension, class, and domain defaults

`tools/campaign.ps1` should consume the resolved source context from this helper.
It should not call artifact/run-config source lookup helpers directly in
continuation or inspect branches; otherwise `latest`, `scratch`, and `source`
semantics will drift again.

Request helpers now live in:

```text
tools/campaign_request.ps1
```

This helper owns:

- retired `-More` rejection
- inspect flag folding
- `-InspectScratchLatest` source/read interpretation
- targeted-vs-coverage-gap mutual exclusion
- scratch output eligibility
- the derived `ReadsCampaignSource` flag

## Still Move Out Of Wrapper

These pieces are useful but should not live in the main script long term:

- residual compatibility switches that may no longer earn wrapper-level
  visibility after the latest/source/output cleanup

## Candidates To Delete Or Degrade

Do not preserve every old feature just because it exists.

Review these before adding more wrapper logic:

- `-PlanTargets` / `-ContinueTargets`: older sibling-continuation path; may be
  superseded by journal/coverage-gap continuation.
- very specific inspect flags that only forward one driver flag; they may be
  better as direct driver examples or a single `-InspectKind` style adapter.
- long comment-help examples; active docs can carry examples instead of the
  wrapper header carrying every workflow.

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

1. Reassess whether targeted continuation still earns its wrapper surface.
2. Consider replacing many specific inspect switches with a smaller typed
   inspect adapter only after current callers are audited.
3. Review residual compatibility switches and remove any that belong only in
   direct driver commands.

This sequence reduces cognitive load without changing campaign strategy.
