# Imminent Boss Campfire Recovery Design

## Problem

The campfire owner treats `Rest` as automatic only when the generic
`RestVsSmithPlanV1` favors rest or route `RecoveryPressure` is strong.  At the
last Act 1 campfire, 42/80 HP is above the generic 45% emergency threshold and
there is no ordinary route segment left to raise recovery pressure.  The owner
therefore smiths immediately before a known boss.

The seed `20260713003` Guardian fixture provides a bounded counterfactual:

- at 42 HP, no quality lane found a win and the best complete loss left 21 HP;
- at the post-rest 66 HP, a win appeared after 358 nodes and ended at 16 HP;
- replaying that winning line from 42 HP loses.

This is a campfire decision-boundary defect, not evidence for globally raising
combat search budgets.

## Decision

Expose whether the boss node is immediately reachable in
`CampfireDecisionContextV1`.  When the boss is next, rest is available, HP is at
or below 60%, and rest heals a positive amount, treat recovery as an autopilot
reason that blocks smithing and selects rest.

The rule stays local to campfire policy.  It does not change generic route
pressure, ordinary low-HP card admission, or the upgrade planner's route-agnostic
emergency threshold.

## Verification

Add one regression test for the observed boundary: a 42/80 Ironclad at the
final-row campfire with a useful smith target must rest.  Then run the focused
campfire tests and rerun the bounded seed.
