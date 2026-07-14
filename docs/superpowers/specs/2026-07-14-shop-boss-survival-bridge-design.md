# Shop Boss-Survival Bridge Design

## Problem

On seed `20260713006`, the A3F35 shop knows that the next boss is Awakened
One and describes `Dark Shackles+` as immediate enemy-strength reduction, but
rejects it before scoring with `SpendsFutureShopLiquidityWithoutHardNeed`.
`Heavy Blade` crosses the same liquidity guard because the early shop bundle
evidence carries boss-scaling repair, while boss-survival repair is only
computed by later acquisition and scoring passes.

There is a second, card-specific omission: Awakened One survival evidence
recognizes persistent or substantial answers such as `Disarm`, `Shockwave`,
and `Impervious`, but not `Dark Shackles`.  The combat engine already models
its temporary Strength loss, `Shackled` restoration, and phase-transition
debuff removal correctly.

The current survival predicate also treats every Weak or Strength-down card as
the same `mitigation_units` coverage.  In the seed deck, `Clothesline+` alone
therefore closes the predicate before `Dark Shackles+` is considered.  A
single `Disarm` would do the same, even though persistent Strength loss and a
one-turn peak-damage answer are complementary against multi-hit attacks.

## Considered Approaches

1. Hard-code `Dark Shackles` in the shop liquidity guard.  This is small but
   duplicates boss knowledge at the wrong layer and would not fix the same
   failure for other survival repairs.
2. Move the acquisition filter before the bundle filter.  This still leaves
   bundle evidence incomplete and risks weakening Maw Bank and future-shop
   preservation for every upgraded or premium card.
3. Carry typed boss-survival evidence through the existing bundle boundary.
   This reuses the authoritative survival evaluator and preserves the current
   ordering of economic safety checks.  This is the selected approach.

## Decision

Split deck mitigation inventory into factual coverage while retaining the
aggregate counter for generic consumers:

- Weak coverage;
- persistent enemy-Strength-down coverage;
- temporary enemy-Strength-down coverage.

Card semantics expose temporary enemy Strength loss separately from persistent
enemy Strength loss.  `Disarm` remains persistent; `Dark Shackles` becomes
temporary.  Broad survival consumers may treat both as mitigation, but
Awakened One evidence uses the distinct coverage slots.

Add a typed repair classification to `BossSurvivalEvidence`:

- `PlanRepair` identifies the existing concrete boss-survival answers.
- `TimedBridge` identifies a one-shot answer whose value depends on reliable
  timing and must not masquerade as a durable solution.

The global `mitigation_units == 0` gate is replaced by a boss-pressure gate
that does not claim saturation.  Each candidate then checks its own coverage
slot.  Existing Weak or persistent Strength loss does not close the temporary
bridge slot, and existing temporary Strength loss does not erase the value of
a persistent answer.  A duplicate in the same slot may receive score-only
credit instead of another strategic repair classification.

The early shop candidate evidence and bundle facts carry the optional repair
kind alongside the existing boss-scaling flag.  A recognized survival repair
may cross the future-shop-liquidity guard only when the shop owner already
reports a boss answer need and there is no more urgent survival-purchase
emergency.  Maw Bank rejection remains earlier and therefore unchanged.

The bundle continues to use `StrategicBossRepairBuy`, but emits distinct
reasons and score labels for a survival plan repair and a timed bridge.  This
keeps ordering compact while preserving diagnostic truth.

## Dark Shackles Classification

Against a known Awakened One with boss-survival pressure and no existing
temporary Strength-down coverage, `Dark Shackles` is a `TimedBridge` even when
the deck already contains Weak or `Disarm`.  Its zero cost, one-turn peak
coverage, and phase-transition interaction are distinct from persistent
mitigation.

Upgrade status and Runic Pyramid increase its evidence score rather than
acting as hard admission gates.  Pyramid makes the critical turn more
reachable; the upgrade increases the covered per-hit Strength from 9 to 15.
Neither fact is allowed to turn the card into a durable plan repair.
`RunStrategicFacts` gains a factual `has_runic_pyramid` field with no policy
judgment.

The acquisition layer does not predict the exact combat turn.  Exact
Awakened One transition-window ordering belongs to the separate
`awakened-one-strength-transition-window` design so shop policy and combat
search remain independently testable.

## Verification

Use focused red-green regressions for:

1. `Dark Shackles` producing `TimedBridge` evidence alongside existing Weak or
   one `Disarm`, while a duplicate temporary bridge receives reduced credit;
2. upgrade and Pyramid increasing score without changing the repair kind;
3. an Awakened One shop purchase crossing the liquidity guard and exposing
   both the survival-evidence and bundle-evidence labels;
4. the bridge not overriding Maw Bank or an urgent survival-purchase state;
5. existing boss-scaling, reward survival, and saturated-deck boundaries
   remaining unchanged.

At completion, run the full library suite and
`architecture_runtime_boundaries` as required by the repository workflow.
