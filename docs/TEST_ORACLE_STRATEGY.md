# Test Oracle Strategy

This project cannot treat every correctness test as "assert the exact Java value"
by default. Some tests have strong external truth, some only have invariants, and
some are differential checks against the Java runtime.

The failure mode to avoid is simple:

- inventing expected values from Rust behavior and calling that an oracle
- writing strong correctness claims without a clear evidence source
- forgetting existing validation tools because they are not part of the default
  workflow

## Oracle Classes

### 1. Java Source Oracle

Use when the original game code directly defines the behavior and the relevant
logic can be inspected.

Examples:

- boss phase transitions
- relic counters that are incremented by explicit Java actions
- power behavior with dedicated Java hidden state

Use this for:

- high-confidence behavior tests
- protocol export decisions
- disputes about exact edge semantics

### 2. Live Runtime Oracle

Use when the behavior is easier to observe from the real game than to infer from
source alone.

Examples:

- `manual scenario` live spot-checks
- `protocol truth samples`
- targeted `CommunicationMod` captures

Use this for:

- protocol truth fixtures
- exact hidden-state snapshots
- behavior cases that need observable Java sequencing

Primary entrypoints:

- [live_comm/LIVE_COMM_MANUAL_SCENARIO_RUNBOOK.md](live_comm/LIVE_COMM_MANUAL_SCENARIO_RUNBOOK.md)
- [protocol/MANUAL_SCENARIO_SAMPLE_INDEX.md](protocol/MANUAL_SCENARIO_SAMPLE_INDEX.md)

### 3. Differential Oracle

Use when Rust and Java can be driven from equivalent inputs and compared without
manually writing exact expected values first.

Examples:

- replay parity
- live-comm parity checks

Use this for:

- state parity
- replay consistency
- identifying which field or transition diverged

Primary entrypoints:

- `tests/live_comm_replay_driver.rs`
- [live_comm/LIVE_COMM_PARITY_WORKFLOW.md](live_comm/LIVE_COMM_PARITY_WORKFLOW.md)

### 4. Invariant Oracle

Use when a test should assert universal constraints rather than an exact Java
value.

Examples:

- deterministic replay for the same seed and reset path
- imported IDs and references remain internally consistent
- card UUIDs referenced by `Stasis` actually exist
- a power that should be re-applied after a state transition exists and is
  positive

Use this for:

- guardrail tests
- fast behavior sanity checks
- checks that are valuable even before exact Java parity is fully captured

## Rules of Use

Before adding or extending a correctness-sensitive test, classify its oracle.

Use one of:

- `Oracle: Java source`
- `Oracle: live runtime sample`
- `Oracle: differential parity`
- `Oracle: invariant only`

If a test uses multiple oracle sources, say so explicitly.

## What Not To Do

Do not:

- derive the expected value from current Rust behavior and present it as external
  truth
- promote an invariant test into a full Java parity claim
- write a behavior test for an exact boundary case when the oracle is still
  unknown

If the needed exact behavior is not yet confirmed, mark the case as:

- `needs_java_oracle`
- `needs_live_spot_check`

and defer the exact assertion until evidence exists.

## Workflow Anchors

When you need external evidence, the default order is:

1. inspect Java source
2. use `tools/sts_tool` to accelerate source tracing when the logic is scattered
3. capture or confirm with `manual scenario` / live spot-check if behavior still
   needs runtime evidence
4. encode the result as a protocol truth sample, behavior test, or parity check

`tools/sts_tool` should be treated as a normal investigation entrypoint, not a
buried optional utility:

- path: `tools/sts_tool`
- use it when Java logic is spread across multiple powers, actions, or helper
  classes

## Test File Convention

Behavior-sensitive tests should carry a short header comment that states the
oracle source and evidence path.

Example:

```rust
// Oracle: Java source + live runtime sample
// Evidence:
// - docs/protocol/GUARDIAN_THRESHOLD_TEST_MATRIX.md
// - tests/protocol_truth_samples/guardian_threshold/frame.json
```

This is intentionally repetitive. The goal is to make the oracle visible in the
place where the test is written, so it is harder to forget later.
