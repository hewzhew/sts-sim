# Campaign Wrapper Audit

`tools/campaign.ps1` started as a friendly runner for campaign experiments. It
now also owns scratch artifacts, continuation planning, coverage-gap execution,
milestone loops, manifest writing, and many inspect commands. That is why the
file grew past a maintainable size.

This file is the cleanup contract. The goal is not to make the wrapper smaller
by moving code around blindly. The goal is to stop one script from owning too
many concepts.

## Current Size

Approximate physical line count: 2300 lines.

Major regions:

| Region | Approx lines | Status |
| --- | ---: | --- |
| Help examples and synopsis | 170 | Too long, but useful as a quick reference |
| Parameter block | 180 | Too many feature flags in one entrypoint |
| Path globals | 20 | Fine |
| Helpers plus pre-main setup | 1150 | Too broad; mixes helpers, normalization, build/run state, milestone support |
| Continuation plan/execute | 470 | Too much logic in the wrapper |
| Inspect execution | 190 | Useful, but dispatch is growing by flag count |
| Normal run execution | 145 | This is the core wrapper responsibility |

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
  The default saved campaign line.

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

## Move Out Of Wrapper

These pieces are useful but should not live in the main script long term:

- artifact shape inspection
- manifest and command-file helpers
- milestone loop helpers
- coverage-gap filter construction
- coverage-gap continuation orchestration
- inspect flag to driver flag mapping

The first extraction target should be artifact helpers because they are already
shared by normal run, inspect, and continuation:

```text
tools/campaign_artifacts.ps1
  latest/scratch artifact refs
  artifact size and shape summaries
  source/output validation
```

The second extraction target should be invocation helpers:

```text
tools/campaign_invocation.ps1
  command rendering
  manifest writing
  logged driver command files
```

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

1. Extract artifact helpers into `tools/campaign_artifacts.ps1`.
2. Update `tools/campaign.ps1` to dot-source that helper.
3. Re-run normal latest, scratch inspect, and coverage-gap dry-run checks.
4. Extract manifest/command rendering helpers into `tools/campaign_invocation.ps1`.
5. Reassess whether targeted continuation still earns its wrapper surface.

This sequence reduces cognitive load without changing campaign strategy.
