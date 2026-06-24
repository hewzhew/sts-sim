# Campaign CLI Contract

This document defines the target command shape for the Rust campaign
application. It is subordinate to
[Campaign System Architecture](CAMPAIGN_SYSTEM_ARCHITECTURE.md). If there is a
conflict, the architecture document wins.

The target surface is subcommand based:

```text
branch_campaign_driver campaign <command> [options]
```

PowerShell wrappers may forward to this surface. They do not own semantics.

## Source Selectors

Every command that reads an artifact uses one source selector:

```text
--from latest
--from scratch-latest
--from run:<run-id>
--from scratch:<artifact-id>
--from path:<report-path>
```

Rules:

- `latest` and `scratch-latest` resolve through `ArtifactStore` pointers.
- `run:<id>` and `scratch:<id>` resolve through `ArtifactStore` conventions.
- `path:<report-path>` is for archaeology and must remain explicit in
  provenance.
- source resolution is read-only.
- a command must not infer source from output mode.

## Output Selectors

Every writing command chooses one output intent:

```text
--out run
--out scratch
--out path:<dir>
```

Rules:

- new normal campaign runs default to `--out run`
- experimental continuations default to `--out scratch`
- `--out run` may update the normal latest pointer
- `--out scratch` may update the scratch latest pointer
- inspect commands never write output

## Commands

### Run

```text
campaign run --seed <seed> --class ironclad --ascension <n> --mode <preset>
campaign run --random-seed --mode explore
```

Creates a new campaign artifact.

### Continue

```text
campaign continue --from latest --rounds <n>
campaign continue --from run:<id> --until Act2Start
campaign continue --from scratch-latest --out scratch --rounds <n>
```

`--rounds` means additional campaign rounds. `--until` means a Rust-owned
engine milestone loop, not a wrapper loop.

### Coverage Plan

```text
campaign coverage plan --from latest --budget key
campaign coverage plan --from latest --bucket route --limit 8
```

Plans targets from `CampaignJournal`. It does not execute continuations.

### Coverage Execute

```text
campaign coverage execute --from latest --until Act2Start --out scratch
campaign coverage execute --from latest --budget milestone --limit 12
```

Executes `ContinuationJob`s produced from journaled candidates. Jobs must carry
target provenance.

### Inspect

```text
campaign inspect --from latest --view summary
campaign inspect --from latest --view decision --decision-id <id>
campaign inspect --from latest --view journal --query shop
campaign inspect --from latest --view final-boss
campaign inspect --from scratch-latest --view coverage
```

Inspect commands are read-only.

### Artifacts

```text
campaign artifacts list
campaign artifacts show --from latest
campaign artifacts prune --dry-run
campaign artifacts prune --apply --keep-runs 10 --keep-scratch 1
```

Artifact commands are owned by `ArtifactStore`.

### Export

```text
campaign export --from latest --kind learning-jsonl --out <path>
campaign export --from run:<id> --kind combat-episodes --out <path>
```

Exports are explicit datasets. Reports are not datasets.

## Presets

Presets are named request builders:

```text
--mode smoke
--mode quick
--mode focused
--mode explore
--mode deep
```

Each preset must be printable as typed settings:

```text
round budget
coverage budget
search budget
capture policy
output intent
```

## Probes

Diagnostic probes are inspect views, not top-level wrapper switches:

```text
campaign inspect --from latest --view probe --probe shop-evidence
campaign inspect --from latest --view probe --probe combat-lab --index 0
```

A probe becomes a stable command only after it has a named typed view.

## Dry Run

Every command should support dry-run output that prints:

```text
typed request
source selector
output selector
expanded preset
artifact refs that would be read or written
```

Dry-run must not mutate artifacts.

## Deprecated Surface

These names must not appear in new examples or tests except as deprecation
notes:

```text
-More
-FromScratchLatest as a semantic shortcut
-InspectScratchLatest
-MaxRounds as continuation shorthand
PowerShell-owned milestone loops
PowerShell-owned coverage-gap orchestration
wrapper-written manifests
latest.campaign.json / latest.checkpoint.json as default source
```

Compatibility may exist temporarily only as a forwarder to the stable Rust
command path or as a loud failure.

## Error Rules

The CLI rejects ambiguous requests:

- a command cannot both inspect and write
- a command cannot read normal latest and scratch latest implicitly
- `--until` cannot be implemented by a wrapper loop
- `--rounds` cannot mean total rounds in one path and additional rounds in
  another
- `--out scratch` must never update normal latest

## Standard Write Output

Every writing command prints stable artifact refs:

```text
run_id=<id>
source=<selector>
output=<selector>
report=<path>
checkpoint=<path>
state=<path>
journal=<path>
manifest=<path>
```
