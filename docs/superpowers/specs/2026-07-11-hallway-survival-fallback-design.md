# Hallway Survival Fallback Design

## Problem

At the saved Act 2 floor 26 hallway combat, the player starts at 20/74 HP.
The owner reserve rule requires at least one quarter of max HP after every
non-transition combat, so every search lane receives a maximum loss of two HP.
The bounded hallway quality lane finds a complete win that ends at 13 HP after
using two potions, but run-control rejects that line and the portfolio reports a
combat gap.

The reserve is a useful quality target, but it currently also acts as a
liveness veto. A combat that has an executable clean win is therefore reported
as if no usable winning line exists.

## Decision

Keep the reserve-gated primary, immediate-escalation, and hallway-quality lanes
unchanged. When a pressured hallway portfolio reaches the end of those lanes,
run one explicit `hallway_survival_fallback` lane.

The fallback:

- uses the existing bounded hallway-quality search budget;
- keeps the semantic potion policy and the existing two-potion cap;
- still accepts only a complete winning line;
- still rejects a line that adds a curse to the master deck;
- uses an unlimited HP-loss search gate, which means the complete-win
  requirement itself supplies the positive-HP survival boundary.

The fallback is not added to ordinary hallways without the existing non-boss
potion-rescue signal. Elite and boss portfolios are unchanged in this slice.

## Why this boundary

Changing the quarter-HP reserve to a different percentage would mix route
quality and combat liveness again. Relaxing every lane would also discard the
preference for safer wins. A final typed lane makes the priority explicit:

1. find a reserve-preserving win;
2. if the pressured hallway otherwise stops, accept a bounded clean survival
   win and retain its high-loss evidence.

The first implementation deliberately reruns the bounded search in the final
lane instead of teaching run-control to retain and recommit a rejected
trajectory. Reusing a rejected trajectory would cross the search/report/owner
boundary and is a separate optimization.

## Verification

Regression checks assert two stable contracts rather than a card sequence:

- the pressured hallway portfolio orders the survival fallback after the
  quality rescue;
- the strict hallway lane keeps the reserve limit while the survival fallback
  explicitly uses `RunControlHpLossLimit::Unlimited` and keeps its bounded
  potion profile.

After focused tests, review the saved A2F26 case and then run the repository's
full library and architecture suites. A full-seed rerun is deferred until the
focused implementation is verified.
