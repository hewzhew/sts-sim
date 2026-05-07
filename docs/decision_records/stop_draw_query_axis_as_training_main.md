# Stop Draw Query-Axis As Training Main

Date: 2026-05-06

## Observation

The draw marginal / query-axis branch looked useful under the earlier split:

```text
card + query baseline ~= 0.66
```

After switching to trace-step grouped splits and hash-based assignment, the same
shallow proxy collapsed:

```text
action-key grouped+hash:
card + query baseline ~= 0.51
card only baseline ~= 0.43
query only baseline ~= 0.62
leakage groups = 0
```

The branch proved that card/action id plus query name does not force
state-conditioned or plan-conditioned understanding.

## Decision

Demote the draw marginal / query-axis dataset to diagnostic status.

It is not a primary training signal for card value, combat plan evaluation, or
draw/search understanding.

## Keep

- forced-target and no-target branch runner
- action-key granularity
- query evaluator plumbing
- trace-step grouped split keys
- hash split hygiene checks
- reports as leakage detectors and negative controls

## Stop

- expanding the draw/query-axis dataset as a main corpus
- training card/action id plus query-name classifiers as a project milestone
- treating `needs_rollout`, query-local deltas, or static cashout tags as
  global action/card value
- tuning features to recover the old shallow baseline

## Replacement Gate

The next training-facing V0 must be a minimal trainability test, not a bigger
schema.

Allowed primary targets:

- engine-executed outcome vectors
- exact or bounded same-state counterfactual outcomes
- within-state candidate comparisons derived from explicit utility protocols

Disallowed primary targets:

- plan-query status
- card/query labels
- `PlanScoreBreakdown.total_score`
- static tag or cashout scores

The V0 fails unless full state plus candidate features beat candidate-only,
card/action-only, and state-only ablations under grouped splits, and unless the
trained evaluator improves engine closed-loop candidate selection.
