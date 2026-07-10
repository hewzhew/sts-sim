# Strength Package Reliability Design

## Problem

The construction policy can deadlock a live strength package. A strength multiplier
such as Limit Break needs reliable strength access, but once the deck has one stable
strength source the boss-scaling deficit is only `Thin`. The shop policy therefore
rejects a second stable source because it is not repairing a `Missing` hard gap.

This leaves the policy asking for two sources before trusting the payoff while refusing
the card that would supply source two.

## Decision

Track strength multipliers explicitly in `DeckRoleInventory` and recognize a narrow
package-reliability repair when all of these are true:

- the deck already has a strength multiplier;
- it has exactly one stable strength source;
- boss scaling is still `Missing` or `Thin`; and
- the candidate is another stable strength source.

Such a shop card receives `ContextTake`, not `AutoAcquire`. Affordability still applies.
The repair may cross the generic purge-reserve threshold: completing a live package is
the intended strategic purchase, while the fixed reserve is only an opportunity-cost
heuristic.

## Boundaries

- Do not name or special-case Inflame or Limit Break.
- Do not promote a third stable source.
- Do not treat conditional sources such as Spot Weakness as stable.
- Do not promote an ordinary strength payoff such as Heavy Blade.
- Do not change combat search or Collector-specific behavior.

## Verification

Add focused tests for multiplier inventory, the positive second-source case, and the
negative third-source/conditional-source/payoff cases. Then run the complete library
and architecture-sensitive suites before committing.
