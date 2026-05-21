# Testing Platform Direction

Legacy `live_comm` workflow lives under:

- `docs/live_comm/LEGACY_FIXTURE_ONLY.md`
- `docs/live_comm/LIVE_COMM_RUNBOOK.md`
- `docs/live_comm/LIVE_COMM_MODES.md`

This document defines the canonical combat-debug artifact and the current cutover
rules.

## Current Layering

- `sts_simulator::fixtures::combat_case`
  - canonical combat-debug schema
  - shared lowering, replay, and assertion engine
  - owns `protocol_snapshot`, `encounter_template`, and transitional `live_window`
- `src/bin/combat_case`
  - CLI for verifying, reducing, compiling, and converting combat cases
- `sts_simulator::fixtures::scenario`
  - legacy bridge for old fixture workflows
  - still used at import boundaries and by some older tools

## Canonical Artifact

All new combat regressions should converge on `CombatCase`.

- `id`
  - stable bug or scenario identifier
- `domain`
  - currently always `combat`
- `basis`
  - `protocol_snapshot`
  - `encounter_template`
  - `live_window`
    - witness-only transitional basis
    - not the preferred checked-in end state
- `delta`
  - typed runtime overrides for player, monsters, relics, zones, potions, and engine state
- `program`
  - structured steps only
- `oracle`
  - `java_source`
  - `live_runtime`
  - `differential`
  - `invariant`
- `expectations`
  - typed expectations first
  - path assertions remain allowed as an import bridge
- `provenance`
  - source path, response range, failure frame, notes, audit context
- `tags`
  - human and agent discoverability

## Basis Rules

- Checked-in combat regressions should be `protocol_snapshot` or `encounter_template`.
- `live_window` is an intermediate witness used to preserve provenance from raw
  `live_comm` logs before reduction.
- `ScenarioFixture` is no longer canonical. Keep it only as a migration bridge.
- If a case cannot be reduced below `live_window`, treat it as `needs_lab_support`
  work rather than the final regression artifact.

## Producers

### Live Witness Producer

Input:

- `live_comm` raw/debug logs
- failure snapshot selection or explicit response window

Output:

- extracted bridge fixture
- minimized bridge fixture
- extracted `CombatCase { basis = live_window }`
- minimized `CombatCase { basis = live_window }`
- reduced `CombatCase { basis = protocol_snapshot }`

Default entrypoint:

```powershell
python tools\analysis\bugfix_workflow.py from-snapshot `
  --run-dir logs\runs\20260420_001126 `
  --snapshot-id f216_r216_s216_engine_bug
```

The wrapper currently still uses `live_regression.py` as the extraction/minimize
bridge, but the canonical outputs are the witness and reduced combat cases.

### Synthetic Producer

Input:

- human-authored or agent-authored declarative combat spec

Output:

- compiled `CombatCase`

Entrypoints:

- `cargo run --bin combat_case -- compile-author-spec --author-spec <spec.json> --out <case.json>`
- `cargo test --quiet`

Current implementation still compiles through the legacy author-spec fixture path
before converting to `CombatCase`, but callers should consume the case output.

### Protocol Sample Producer

Input:

- checked-in protocol truth samples
- hand-authored protocol snapshot basis

Output:

- `CombatCase { basis = protocol_snapshot }`

Use this when the bug is fundamentally importer/state-sync driven and does not
need a live witness window.

### Java Combat Lab Producer

Status:

- planned
- do not design against a generic debug console

Target output:

- `CombatCase`-compatible base state
- optionally a short structured program
- protocol snapshot or live witness export

## Default Validation Entrypoints

- `cargo test --quiet`
  - current checked-in Rust validation suite
- `cargo run --bin combat_case -- verify --case <path>`
  - validate a case outside the test harness
- `cargo run --bin combat_case -- reduce --case <witness.json> --out <case.json>`
  - materialize or reduce a witness case

Legacy bridge:

- old live regression drivers are retired
- keep raw Java captures only as fixture provenance, not as active strategy code

## Cutover Rules

1. New combat regressions should land under `tests/combat_cases/`.
2. New workflow/docs should talk about `CombatCase`, not `ScenarioFixture`.
3. If a legacy tool still emits fixtures, convert at the boundary and keep the
   bridge local.
4. Prefer reducing a witness into `protocol_snapshot` before checking it in.
5. Only extend Java lab/debug surface when an unreduced witness demonstrates a
   concrete gap.
