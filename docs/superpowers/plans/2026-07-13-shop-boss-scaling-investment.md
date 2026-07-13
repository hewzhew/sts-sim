# Shop Boss-Scaling Investment

## Problem

The bounded seed `20260713003` reaches the Act 2 shop with the known boss The
Champ, 313 gold, and a deck whose boss-scaling plan is still missing or thin.
The decision pipeline correctly recognizes Demon Form as the first stable
scaling repair on the run and admits it to the mainline.  The shop owner still
removes a Strike first because cleanup receives a larger single-action score.
After that purchase, the future-shop liquidity filter rejects Demon Form, even
though it is the identified boss-plan repair.

This is an information-boundary failure: semantic boss-scaling evidence reaches
card scoring, but shop purchase planning only recognizes a fixed list of
front-loaded damage cards as boss answers.

## Design

Introduce candidate-specific shop purchase evidence with one narrow fact:
`repairs_boss_scaling_plan`.  The decision pipeline derives it from the existing
`assess_boss_scaling_evidence` result, only when the deck actually needs a boss
scaling plan and the candidate is a non-fragile relevant repair.

The shop bundle contract will give that candidate a distinct
`StrategicBossRepairBuy` verdict.  This verdict:

- is allowed to spend future-shop liquidity because it addresses an observed
  strategic deficit;
- ranks above efficient cleanup, so the scarce repair is purchased before a
  generic purge can make it unaffordable;
- remains below immediate survival purchases and hard imminent-boss answers;
- does not bypass acquisition, setup-risk, duplicate, or survival-pressure
  filters.

The static shop preview will classify deterministic stable scaling cards such
as Demon Form as boss-repair candidates.  That change improves diagnostics and
optional bundle exploration, but the live owner decision remains driven by the
contextual semantic evidence rather than the static allow-list alone.

## Non-goals

- Do not encode a The Champ-only rule.
- Do not globally increase Demon Form's card score.
- Do not weaken liquidity protection for ordinary cards, relics, potions, or
  cleanup.
- Do not claim the current Champ fixture is mathematically unwinnable; bounded
  searches only show that more HP alone does not close the gap.

## Verification

1. Add a failing decision-pipeline regression matching the Act 2 shop shape:
   Demon Form remains executable after a hypothetical cleanup and outranks an
   ordinary Strike purge before it.
2. Add a shop-purchase-bundle unit test proving semantic strategic repair may
   spend liquidity while an otherwise identical ordinary purchase remains
   rejected.
3. Add a preview classification test for Demon Form.
4. Run focused strategy and owner tests, then rerun seed `20260713003` from the
   beginning and inspect the new first blocker.

