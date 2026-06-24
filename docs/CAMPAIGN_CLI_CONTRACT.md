# Campaign CLI Contract

This document defines the target user-facing campaign command surface. It is a
contract for the Rust campaign application, not a description of every current
PowerShell compatibility switch.

## Shape

The stable surface should be subcommand based:

```text
branch_campaign_driver campaign <command> [options]
```

PowerShell wrappers may forward to this surface, but the Rust CLI owns
semantics.

## Source Selectors

All commands that read an artifact use one source selector:

```text
--from latest
--from run:<run-id>
--from scratch:<artifact-id>
--from scratch-latest
--from path:<path>
```

Rules:

- `latest` resolves through the normal run latest pointer.
- `scratch-latest` resolves through the scratch latest pointer.
- `run:<id>` and `scratch:<id>` resolve through `ArtifactStore`.
- explicit paths are allowed for archaeology, but should be printed as
  explicit paths in provenance.
- source resolution is read-only.

No command should infer source from output mode.

## Output Selectors

Writing commands choose one output intent:

```text
--out run
--out scratch
--out path:<dir>
```

Rules:

- normal campaign runs default to `--out run`
- experimental continuations should use `--out scratch` unless explicitly
  promoted
- a command that writes a normal run may update the normal latest pointer
- a command that writes scratch may update the scratch latest pointer
- inspecting never writes output

## Stable Commands

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

Continues an existing artifact. `--rounds` means additional campaign rounds.
`--until` means run a Rust-owned milestone loop until the target, terminal
state, or configured budget limit.

### Coverage Plan

```text
campaign coverage plan --from latest --budget key
campaign coverage plan --from latest --bucket route --limit 8
```

Plans candidate coverage from `CampaignJournal`. It does not execute
continuations.

### Coverage Execute

```text
campaign coverage execute --from latest --until Act2Start --out scratch
campaign coverage execute --from latest --budget milestone --limit 12
```

Executes continuation jobs created by the planner. Jobs must carry target
provenance.

### Inspect

```text
campaign inspect --from latest --view summary
campaign inspect --from latest --view state --index 0
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

## Presets

Presets are named request builders, not hidden policy:

```text
--mode smoke
--mode quick
--mode explore
--mode deep
```

Each preset must expand to printed typed settings: max rounds, active executor
budget, planner budget, search budget, capture policy, and output intent.

## Probes

Diagnostic probes should be namespaced:

```text
campaign inspect --from latest --view probe --probe shop-evidence
campaign inspect --from latest --view probe --probe combat-lab --index 0
```

Do not add one top-level CLI flag per probe unless it graduates into a stable
view.

## Deprecated Surface

These names should not be used in new docs, tests, scripts, or examples:

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

Compatibility may remain temporarily, but it must forward to the stable Rust
command path or fail loudly.

## Error Rules

The CLI should reject ambiguous commands:

- a command cannot both inspect and write
- a command cannot both read normal latest and scratch latest implicitly
- `--until` cannot be implemented by a wrapper loop
- `--rounds` and `--until-round` should not mean different things depending on
  whether the source is latest or scratch
- `--out scratch` must never update normal latest

## Output Rules

Every writing command prints:

```text
run_id=<id>
source=<selector>
output=<selector>
report=<path>
checkpoint=<path>
journal=<path or inline>
manifest=<path>
```

Every command should support a dry-run mode that prints the typed request and
the driver command without mutating artifacts.
