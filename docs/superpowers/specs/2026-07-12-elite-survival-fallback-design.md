# Elite Survival Fallback Design

## Problem

The fresh seed `20260711004` run reaches Book of Stabbing at Act 2 floor 23
with 45/74 HP. Both the elite primary search and its post-primary quality lane
find the same complete win: 13 HP remaining after 32 damage and one potion.
The non-boss reserve allows at most 27 damage, so both attempts are rejected and
the run stops even though the combat has an executable clean win.

The current elite portfolio contains exactly one post-primary lane named
`NonBossPotionRescue`. Despite that broad name, it is only scheduled for elite
stakes. It already uses the desired final-fallback mechanics:

- a bounded 300,000-node / 5,000-ms quality budget;
- immediate child rollout and round-robin quality frontier;
- semantic potion use with a one-potion maximum;
- complete-line-only commit behavior;
- rejection of wins that add a curse.

Its remaining problem is semantic: it inherits the same quarter-max-HP reserve
as the strict elite primary lane.

## Decision

Rename `NonBossPotionRescue` to `EliteSurvivalFallback` and rename its label
from `nonboss_potion_rescue` to `elite_survival_fallback`.

Keep the elite portfolio at two total attempts:

1. the primary elite search retains the quarter-max-HP reserve and its existing
   three-second rescue budget;
2. `elite_survival_fallback` retains the existing five-second quality profile
   and one-potion cap, but receives `RunControlHpLossLimit::Unlimited`.

`Unlimited` removes the quality reserve as a commit veto. The existing complete
win requirement still guarantees that the player survives the combat.

This reuses the current final lane instead of appending a third search, so the
worst-case elite portfolio budget does not increase.

## Evidence And Responsibility

The accepted-high-loss diagnostic remains active for committed fallback wins.
Its trigger is based on observed loss relative to max HP, independently of the
hard HP-loss limit. The Book line's 32 damage exceeds that trigger, so a
successful commit will retain a combat capture, selected trajectory, search
summary, and attrition evidence under the lane label
`elite_survival_fallback`.

This keeps responsibilities separate:

- route policy decides whether an optional elite should be entered;
- combat owner preserves liveness after combat has already begun;
- accepted-high-loss evidence supports later route review.

The current capsule does not retain the automatic route candidate pool for the
floor 22 to floor 23 transition, so whether this Book was avoidable cannot be
proved from that artifact. Route provenance is a separate future slice and is
not required for combat liveness.

## Non-goals

- Do not add a third elite search lane.
- Do not increase elite search budgets or potion allowance.
- Do not change hallway or boss portfolios.
- Do not change the quarter-max-HP reserve formula.
- Do not reuse rejected trajectories across lane boundaries in this slice.
- Do not change route selection or route artifact schemas.

## Verification

Stable regression checks will assert that:

1. the elite post-primary portfolio contains exactly one lane named
   `elite_survival_fallback`;
2. the elite primary lane keeps the reserve gate while the fallback uses
   `RunControlHpLossLimit::Unlimited`;
3. the fallback keeps the five-second bounded quality profile, semantic potion
   policy, one-potion maximum, and clean-win acceptance;
4. hallway and boss portfolio plans remain unchanged.

After focused tests, an owner-exact Book case probe will confirm that the
existing fallback profile still finds the 13 HP complete win. Completion
verification will run the full library and architecture suites, followed by one
fresh bounded seed run to observe whether the owner advances past Book of
Stabbing and which real blocker appears next.
