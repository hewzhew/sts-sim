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

Add a typed repair classification to `BossSurvivalEvidence`:

- `PlanRepair` identifies the existing concrete boss-survival answers.
- `TimedBridge` identifies a one-shot answer whose value depends on reliable
  timing and must not masquerade as a durable solution.

The early shop candidate evidence and bundle facts carry this optional repair
kind alongside the existing boss-scaling flag.  A recognized survival repair
may cross the future-shop-liquidity guard only when the shop owner already
reports a boss answer need and there is no more urgent survival-purchase
emergency.  Maw Bank rejection remains earlier and therefore unchanged.

The bundle continues to use `StrategicBossRepairBuy`, but emits distinct
reasons and score labels for a survival plan repair and a timed bridge.  This
keeps ordering compact while preserving diagnostic truth.

## Dark Shackles Classification

Against a known Awakened One with an open survival pressure, `Dark Shackles`
is a `TimedBridge` only when at least one timing-confidence fact holds:

- the card is upgraded, providing 15 temporary Strength loss; or
- the run has Runic Pyramid, allowing the card to be retained for a critical
  multi-hit, Dark Echo, or first-phase transition turn.

An unupgraded copy without Runic Pyramid may receive score-only recognition,
but it does not justify spending protected future-shop liquidity.  Existing
survival answers still close the open-pressure predicate and prevent duplicate
bridge credit.  `RunStrategicFacts` gains a factual `has_runic_pyramid` field;
it contains no policy judgment.

## Verification

Use focused red-green regressions for:

1. upgraded or Pyramid-supported `Dark Shackles` producing `TimedBridge`
   evidence, while an unupgraded untimed copy remains score-only;
2. an Awakened One shop purchase crossing the liquidity guard and exposing
   both the survival-evidence and bundle-evidence labels;
3. the bridge not overriding Maw Bank or an urgent survival-purchase state;
4. existing boss-scaling, reward survival, and saturated-deck boundaries
   remaining unchanged.

At completion, run the full library suite and
`architecture_runtime_boundaries` as required by the repository workflow.
