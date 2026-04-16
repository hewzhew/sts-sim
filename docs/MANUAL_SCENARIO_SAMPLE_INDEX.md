# Manual Scenario Sample Index

This index tracks manually recorded `CommunicationMod` scenario samples that
already prove strict `runtime_state` slices in the new protocol path.

These samples are not yet full replay fixtures. They are first-pass truth
captures used to confirm that:

- `scenario` can enter the target scene
- `CommunicationMod` exports the expected `runtime_state`
- Rust strict importer has a concrete sample to consume

## Recorded Samples

### GuardianThreshold

- slice: `GuardianThreshold`
- encounter: `the_guardian`
- path:
  - `START ironclad 0`
  - `scenario fight the_guardian`
  - `STATE`
- proof:
  - monster `id == "TheGuardian"`
  - `monster.runtime_state.guardian_threshold`
- sample:
  - [guardian_threshold_20260416_123846](/d:/rust/sts_simulator/logs/manual_scenario_samples/guardian_threshold_20260416_123846)
  - [frame.json](/d:/rust/sts_simulator/logs/manual_scenario_samples/guardian_threshold_20260416_123846/frame.json:1)

### Angry

- slice: `Angry`
- encounter: `gremlin_gang`
- path:
  - `START ironclad 0`
  - `scenario fight gremlin_gang`
  - `STATE`
- proof:
  - monster `id == "GremlinWarrior"`
  - `monster.runtime_state.angry_amount`
- sample:
  - [angry_20260416_124115](/d:/rust/sts_simulator/logs/manual_scenario_samples/angry_20260416_124115)
  - [frame.json](/d:/rust/sts_simulator/logs/manual_scenario_samples/angry_20260416_124115/frame.json:1)

### Combust

- slice: `Combust`
- encounter: `jaw_worm`
- path:
  - `START ironclad 0`
  - `scenario fight jaw_worm`
  - `STATE`
  - `scenario power add player combust 1`
  - `WAIT 10`
  - `STATE`
- proof:
  - player power `id == "Combust"`
  - `power.runtime_state.hp_loss`
- sample:
  - [combust_20260416_124432](/d:/rust/sts_simulator/logs/manual_scenario_samples/combust_20260416_124432)
  - [frame.json](/d:/rust/sts_simulator/logs/manual_scenario_samples/combust_20260416_124432/frame.json:1)

## Current Status

These three slices are the current stable manual truth set:

- `GuardianThreshold`
- `Angry`
- `Combust`

`Stasis` is intentionally not listed yet. It remains a later slice because the
shortest stable path is less deterministic than the three above.

## Next Expected Use

This index should be updated when one of these happens:

1. a manual sample is promoted into a more formal replay/fixture asset
2. a newer sample replaces the current one
3. an additional strict `runtime_state` slice is manually captured

## Related Docs

- [LIVE_COMM_MANUAL_SCENARIO_RUNBOOK.md](/d:/rust/sts_simulator/docs/LIVE_COMM_MANUAL_SCENARIO_RUNBOOK.md:1)
- [STATE_SYNC_STATUS.md](/d:/rust/sts_simulator/docs/STATE_SYNC_STATUS.md:1)
- [PROTOCOL_TRUTH_RULES.md](/d:/rust/sts_simulator/docs/PROTOCOL_TRUTH_RULES.md:1)
