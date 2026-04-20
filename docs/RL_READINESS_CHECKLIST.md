# RL Readiness Checklist

This document answers one question:

What must become true before this repository should be treated as a stable RL
environment instead of a still-moving simulator project?

It is intentionally practical. Items here should map to tests, fixtures,
protocol assets, or explicit interface contracts.

## Current Thesis

The repo already has active offline learning experiments. What it still lacks is
a thick enough **correctness flywheel** to justify treating those experiments as
a stable environment instead of a moving research harness:

- protocol truth samples
- strict importer coverage
- targeted mechanic behavior tests
- live/manual spot-checks
- stable training-facing contracts

That is the path from "architecture feels better" to "safe to train on".

## Must

These are blockers for treating the environment as RL-ready.

### 1. High-Risk Mechanic Behavior Coverage

Protocol truth is not enough. A mechanic can import the right hidden state and
still resolve incorrectly.

At minimum, the project needs targeted behavior coverage for the highest-risk
state machines and combat pivots, such as:

- `The Guardian`
- `Bronze Automaton` / `Stasis`
- `Hexaghost`
- `Time Eater`
- `Awakened One`
- high-impact relic/power edge cases

Completion signal:

- each high-risk mechanic has at least one checked-in behavior test file
- the tests cover state transitions, not just field existence
- the tests are stable enough to run in the default CI/local test path

### 2. Stable Training Interface Contract

Before serious RL work resumes, the environment-facing contract needs to stop
moving casually.

This means explicitly stabilizing:

- observation schema
- action schema
- reward semantics
- terminal/episode boundary semantics
- reset and seed behavior

Completion signal:

- these semantics are written down in one canonical place
- changing them requires an intentional doc + test update
- offline data generation and live rollouts use the same meaning

### 3. Reproducible Scenario Construction

When a mechanic drifts, reproducing it must be cheap.

The project already has a good start:

- `scenario` commands in `CommunicationMod`
- manual live-comm client
- `combat_start_spec`
- synthetic scenario support

The remaining requirement is operational, not theoretical:

- key encounters and key state setups must be easy to reproduce repeatedly

Completion signal:

- high-risk mechanic bugs can be recreated from a short script/spec or short
  manual runbook
- protocol truth samples can be refreshed without long ad hoc gameplay sessions

## Should

These should be done soon because they reduce drift and make RL work cheaper.

### 4. Canonical vs Historical Sample Separation

The repo must keep current truth assets separate from old-world captures.

Desired state:

- canonical assets live under active test/sample paths
- pre-`runtime_state` or other retired protocol shapes are archived explicitly
- docs do not imply that archived assets are current truth

Completion signal:

- active tests only depend on canonical assets
- historical fixtures are either archived or clearly marked

### 5. Regular Live Spot-Check Loop

Live/manual testing should not be the primary verification path, but it should
remain a routine secondary check.

Use it to confirm:

- `CommunicationMod` still exports current truth
- manual `scenario` tooling still works
- Java and Rust still agree on a few narrow, meaningful slices

Completion signal:

- every major protocol/importer change is followed by a short live spot-check
- runbooks stay current because they are exercised occasionally

### 6. Comparator and Audit Assumption Cleanup

When protocol truth moves forward, diff/audit logic must stop assuming the old
world.

Examples:

- fields once treated as Rust-only may no longer be Rust-only
- archived protocol shapes should not quietly drive active comparison logic

Completion signal:

- active comparator/audit code reflects current protocol truth assumptions
- old assumptions move to archive docs instead of remaining implicit in code

## Later

These matter, but they should not outrank correctness and interface stability.

### 7. More Scenario Power

Examples:

- `scenario monster-runtime ...`
- `scenario power-runtime ...`
- `scenario relic-runtime ...`

These will make reproduction and targeted capture cheaper, but the current
minimal scenario tooling is already enough to justify continuing on the
correctness path first.

### 8. More Repository Cleanup

Examples:

- more artifact directory stratification
- more documentation pruning
- more surface-area narrowing

These improve maintainability, but they are no longer the main source of
confidence.

### 9. Performance Maximization

Performance remains a core goal, but correctness and contract stability come
first.

Fast wrong environments are worse than slower correct ones for RL.

## Readiness Gate

The project is in a good position to restart serious RL work when all of the
following are true:

1. high-risk mechanics have meaningful behavior coverage
2. protocol/importer truth is effectively single-source for active slices
3. the training interface contract is written and stable
4. scenario construction is cheap enough for recurring bug reproduction
5. live spot-checks confirm the protocol toolchain has not drifted

## Suggested Order

1. Continue landing high-risk mechanic behavior tests
2. Write and freeze the training interface contract
3. Keep protocol truth samples and strict importer tests current
4. Use live/manual spot-checks as narrow confirmation, not the primary harness
5. Only then resume larger-scale RL iteration

## Related Docs

- [../README.md](../README.md)
- [LAYER_BOUNDARIES.md](LAYER_BOUNDARIES.md)
- [protocol/PROTOCOL_TRUTH_RULES.md](protocol/PROTOCOL_TRUTH_RULES.md)
- [protocol/STATE_SYNC_STATUS.md](protocol/STATE_SYNC_STATUS.md)
- [protocol/GUARDIAN_THRESHOLD_TEST_MATRIX.md](protocol/GUARDIAN_THRESHOLD_TEST_MATRIX.md)
- [live_comm/LIVE_COMM_MANUAL_SCENARIO_RUNBOOK.md](live_comm/LIVE_COMM_MANUAL_SCENARIO_RUNBOOK.md)
- [design/COMBAT_RL_CONTRACT_V0.md](design/COMBAT_RL_CONTRACT_V0.md)
- [../tools/learning/README.md](../tools/learning/README.md)
