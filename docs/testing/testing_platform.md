# Testing Platform Direction

The old `CombatCase`, `ScenarioFixture`, protocol importer, and state-sync
fixture workflow has been removed from the active code path.

Current testing infrastructure should support the simulator/search mainline:

1. build a combat start state from a start-spec
2. capture exact stable combat positions when a run reaches a useful fight
3. save whole-combat baseline outcomes after a human-played fight completes
4. report search outcome and unresolved budget state
5. compare against external baselines only at whole-combat outcome level

## Active Inputs

- `sts_simulator::fixtures::combat_start_spec`
  - compiles a JSON combat start-spec into `EngineState + CombatState`
- `sts_simulator::eval::combat_capture::CombatCaptureV1`
  - stores a stable-boundary `CombatPosition` with schema/version,
    integrity fingerprints, a human-readable summary, and the exact typed
    simulator state used by search
- `sts_simulator::eval::run_control::CombatBaselineOutcomeV1`
  - stores the completed whole-combat baseline outcome: terminal label, final
    HP, HP loss, turns, potions used/discarded, and cards played
- `cargo run --bin combat_search_v2_driver -- --start-spec <spec.json>`
  - runs Combat Search V2 from that start state
- `cargo run --bin combat_search_v2_driver -- --combat-snapshot <capture.json>`
  - runs Combat Search V2 from an exact captured combat position
- `cargo run --bin run_play_driver -- --seed <seed> --ascension <n>`
  - opens the simulator run-control shell from real Neow by default
  - `capture-case <benchmark_dir> <case_id> [label]` writes
    `captures/<case_id>.capture.json`
  - `save-baseline-case <benchmark_dir> <case_id>` writes
    `baselines/<case_id>.baseline.json` from the last completed combat
  - `bench-add <benchmark_dir> <case_id>` registers both files in
    `benchmark.json`

## Start-Spec Shape

The start-spec is deliberately narrow:

- player class, ascension, seed, room type, encounter id
- player hp/max hp
- master deck
- relics
- potions

It describes a combat start, not a replay window and not a human action program.

## Combat Capture Shape

`CombatCaptureV1` is the durable capture format for real search starts. The
summary is for review; the executable payload is the typed `CombatPosition`.
Validation rejects unknown schema versions, non-stable combat boundaries,
fingerprint drift, and summaries that no longer match the position.

## Benchmark Case Shape

The preferred real-run benchmark case is a path-based pair:

```json
{
  "id": "f03_lagavulin",
  "combat_snapshot": "captures/f03_lagavulin.capture.json",
  "baseline": "baselines/f03_lagavulin.baseline.json"
}
```

Inline baseline objects remain accepted for old manifests, but new run-control
case registration writes a separate `CombatBaselineOutcomeV1` file.

## Removed Active Paths

These are not current testing architecture:

- `CombatCase`
- `ScenarioFixture`
- live protocol truth samples
- state-sync snapshot import
- replay support for captured Java windows
- case reduction or case verification binaries

If Java-connected capture is revived later, it should be rebuilt as an external
adapter that emits start-specs or a new explicit oracle fixture format. It should
not restore the old case/scenario/state-sync stack.
