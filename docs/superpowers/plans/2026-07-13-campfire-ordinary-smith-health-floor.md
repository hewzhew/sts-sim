# Campfire Ordinary Smith Health Floor

## Observed gap

After the hallway-primary rollout repair, seed `20260713003` reaches Act 3
floor 39 at `47/85` HP.  The campfire owner upgrades `Battle Trance` instead
of resting, then loses 23 HP to Transient and enters the next hallway at
`24/85` HP.

The policy option `allow_clear_core_smith_when_healthy` currently has no
health check.  Clear-core tags can therefore execute at any HP above the
separate rest-vs-smith emergency boundary.  Combat-patch smith targets already
have a 70-percent safety floor, but consistency, scaling, and other clear-core
targets have none.

## Design

Add one ordinary-smith health floor to `CampfirePolicyConfigV1`, defaulting to
60 percent:

- below the floor, an available effective Rest is an executable recovery
  action;
- below the same floor, ordinary Smith actions are not executable;
- at or above the floor, the existing clear-core and combat-patch score/tag
  gates remain unchanged;
- stronger existing recovery rules and the imminent-boss rule remain intact;
- relic-only campfire actions and owner fallbacks remain unchanged.

This is a generic campfire safety boundary.  It does not mention the seed,
card, next encounter, or accepted combat loss, and it preserves the separate
70-percent floor for lower-priority combat patches.

## Verification

- An exact-shaped `47/85` state with a Battle Trance consistency upgrade and a
  visible Rest must choose Rest and leave the Smith candidate inspect-only.
- At the 60-percent boundary, the same clear-core upgrade remains executable.
- Existing campfire policy, upgrade planner, and owner tests remain green.
- A fresh bounded seed run must show whether the recovered HP changes the next
  real blocker; it is evidence, not a frozen whole-seed regression test.
