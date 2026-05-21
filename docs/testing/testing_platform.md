# Testing Platform Direction

The old `CombatCase`, `ScenarioFixture`, protocol importer, and state-sync
fixture workflow has been removed from the active code path.

Current testing infrastructure should support the simulator/search mainline:

1. build a combat start state from a start-spec
2. run local simulator/search from that state
3. report whole-combat outcome and unresolved budget state
4. compare against external baselines only at whole-combat outcome level

## Active Inputs

- `sts_simulator::fixtures::combat_start_spec`
  - compiles a JSON combat start-spec into `EngineState + CombatState`
- `cargo run --bin combat_search_v2_driver -- --start-spec <spec.json>`
  - runs Combat Search V2 from that start state

## Start-Spec Shape

The start-spec is deliberately narrow:

- player class, ascension, seed, room type, encounter id
- player hp/max hp
- master deck
- relics
- potions

It describes a combat start, not a replay window and not a human action program.

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
