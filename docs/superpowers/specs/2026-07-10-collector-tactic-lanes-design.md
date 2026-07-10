# Collector Tactic Lanes Design

## Goal

Test whether the saved Act 2 Collector failure is primarily a search-strategy
problem by comparing two deliberately different tactical priors under one
fixed total budget: keep exactly one Torch Head alive, or race The Collector
directly.

## Decision

Add an opt-in `combat_case_review` experiment with exactly two lanes:

1. `collector_single_head_control` concentrates damage into one Torch Head,
   then values the one-head formation and redirects single-target damage to
   The Collector while the remaining head stays alive.
2. `collector_boss_race` values damage to The Collector ahead of damage to a
   Torch Head.

Both lanes use the same search profile except for their typed tactical-prior
plugin. The configured total node and wall-clock budgets are split evenly
between the two lanes. The result records total and per-lane budgets, plugin
labels, search reviews, and focused witness summaries.

The tactical prior must affect both local action ordering and frontier node
ordering. A root-only prior is insufficient because Torch Heads spawn after
the root state. For tactical frontier ordering, exact terminal outcomes and
rollout safety remain gates; the Collector formation/target preference is
inserted before generic enemy-progress comparison. The default plugin retains
the existing comparator byte-for-byte in behavior.

## Stable Tests

Add only structural and semantic contracts:

- both new plugin variants survive profile-to-config conversion and have
  stable labels;
- default frontier ordering remains unchanged;
- the control prior prefers one living Torch Head, and focuses the weaker head
  while two are alive;
- the race prior prefers lower Collector HP when generic state facts tie;
- local card ordering targets the intended enemy for each formation;
- the review experiment contains exactly two lanes and splits one total budget
  evenly.

Do not assert that the saved b0094 case must win, must reach a fixed turn, or
must emit a particular card sequence. Those are experiment results, not stable
software contracts.

## Non-Goals

- Do not enable either tactic in the main runner or default combat search.
- Do not change card rewards, campfire decisions, map routing, or owner policy.
- Do not add a general Collector strategy to production policy in this change.
- Do not force scripted action prefixes or use a root-only action prior.
- Do not add seed replay, checkpoint, or panel regression tests.
