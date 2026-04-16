# Guardian Threshold Test Matrix

This document is **not** about protocol existence.

It is about the actual Guardian mechanic:

- threshold consumption
- mode switch timing
- overflow damage behavior
- post-switch intent/state updates
- threshold reset / growth rules

The manual scenario sample for `guardian_threshold` only proves that the new
protocol path exports and imports the field. It does **not** prove that the
full Guardian mechanic is behaviorally correct in Rust.

## Scope

Questions this matrix is meant to answer:

1. Does Rust receive the correct initial threshold value?
2. Does Guardian remain in attack mode while below threshold?
3. On the threshold-crossing hit, how is damage split between:
   - pre-switch HP loss
   - the 20 block from Defensive Mode
   - any overflow damage
4. Does the intent change at the correct time after switching?
5. Does the next threshold value reset or increase correctly?
6. Is there an upper bound or repeated-step rule, and does Rust match it?

## What Is Already Proven

Already proven by the manual protocol sample:

- `monster.runtime_state.guardian_threshold` exists in protocol truth
- `state_sync` strict importer reads it

Reference sample:

- [guardian_threshold_20260416_123846](/d:/rust/sts_simulator/logs/manual_scenario_samples/guardian_threshold_20260416_123846)

## What Is Not Yet Proven

Not yet proven by current samples:

- exact-threshold crossing combat resolution
- full Java parity for the post-switch turn boundary
- next threshold progression across multiple switches

## First Implemented Behavior Coverage

The first Rust behavior tests now live in:

- [tests/guardian_threshold_behavior.rs](../../tests/guardian_threshold_behavior.rs)

Currently covered:

- below-threshold hit does not switch mode
- exact-threshold hit queues the Defensive Mode transition after full HP loss
- overflow hit applies full HP loss before queued Defensive Mode transition resolves
- queued switch grants 20 block and updates the visible intent to `Buff`
- after the defensive sequence reaches Twin Slam, `Mode Shift` is reapplied from the
  increased internal threshold (`40` at A0 after one trigger)
- a second trigger after the next offensive cycle raises the internal threshold
  from `40` to `50`, and the next `Mode Shift` is reapplied at `50`

Still not covered:

- Java parity for the visible turn boundary after switching
- repeated multi-cycle growth beyond the second confirmed trigger
- a dedicated confirmation of whether any practical cap exists; current Java
  source inspection shows `dmgThreshold += dmgThresholdIncrease` with no cap
  branch in `TheGuardian.java`

## Recommended Behavior Cases

### Case A: Below Threshold

Setup:

- Guardian in initial attack mode
- threshold known
- hit for less than threshold

Expected:

- no mode switch
- no Defensive Mode block granted yet
- threshold decreases or internal tracking updates exactly as base game expects

### Case B: Exact Threshold Hit

Setup:

- Guardian threshold known
- hit for exactly the remaining threshold amount

Expected:

- switch occurs at the correct point in resolution
- no ambiguous extra overflow damage
- next visible intent/state matches base game

### Case C: Threshold Overflow Hit

Setup:

- threshold known
- hit for more than remaining threshold

Expected questions to pin down:

- does overflow continue into HP before the switch?
- does overflow get eaten by the 20 Defensive Mode block?
- is overflow discarded entirely after the switch trigger?

This is the highest-value unresolved case right now.

### Case D: Post-Switch Intent

Setup:

- force or reach Defensive Mode

Expected:

- visible intent changes on the correct turn boundary
- Rust and Java agree on both:
  - current visible intent
  - hidden threshold/runtime state

### Case E: Repeated Threshold Cycles

Setup:

- run the fight long enough to trigger multiple threshold transitions

Expected:

- next threshold value matches base game after each cycle
- any growth/reset rule is reproduced exactly
- any upper bound is respected

## Suggested Execution Order

1. Case C: threshold overflow hit
2. Case D: post-switch intent
3. Case E: repeated threshold cycles
4. Case A/B as control cases if needed

Why:

- overflow behavior is the least obvious and easiest place for Rust/Java drift
- intent timing is the next most visible regression
- repeated cycles validate the long-tail state machine

## Recommended Test Vehicle

Do not try to prove these cases with protocol-only samples.

Preferred vehicles:

- controlled combat fixture / author spec
- `combat_lab` style deterministic harness
- targeted live/manual scenario recording only when necessary

Protocol samples should remain narrow:

- field exists
- field imports

Behavior tests should remain separate:

- mechanic resolves correctly

## Related Docs

- [MANUAL_SCENARIO_SAMPLE_INDEX.md](MANUAL_SCENARIO_SAMPLE_INDEX.md)
- [STATE_SYNC_STATUS.md](STATE_SYNC_STATUS.md)
- [LIVE_COMM_MANUAL_SCENARIO_RUNBOOK.md](../live_comm/LIVE_COMM_MANUAL_SCENARIO_RUNBOOK.md)
