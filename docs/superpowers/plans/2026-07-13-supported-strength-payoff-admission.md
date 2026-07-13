# Supported Strength Payoff Admission

## Problem

After the shop scaling repair, seed `20260713003` defeats The Champ and reaches
Awakened One.  The first Act 3 construction discontinuity appears at floor 33:
the deck already owns upgraded Demon Form and no dedicated strength payoff, yet
Heavy Blade and Sword Boomerang are capped to probe despite raw mainline scores.
The next reward again skips Heavy Blade+, while the following shop buys Flex+.

The boss-scaling model already says one stable strength source makes the first
strength payoff relevant.  Acquisition's duplicate `fragile_supported_payoff`
predicate instead requires two stable sources, so two policy layers disagree.

## Design

Make the open payoff slot explicit on `DeckPlanSnapshot`:

- at least one stable strength source is present;
- no strength payoff has been acquired yet.

A strength-dependent `BuildsSupportedPackage` candidate is no longer fragile
when that slot is open.  This lets the reward acquisition contract admit the
first payoff as `ContextTake`.  Once a payoff exists, further payoff cards remain
speculative and continue through duplicate/role-saturation controls.  Conditional
sources alone do not open the slot.

Both acquisition and decision-pipeline scoring must call the same deck-plan
predicate so their thresholds cannot drift again.

## Non-goals

- Do not give Heavy Blade or Sword Boomerang a card-id-specific score bonus.
- Do not admit repeated payoff copies.
- Do not treat conditional strength sources as stable support.
- Do not change block-payoff readiness in this slice.

## Verification

1. Reproduce the A3 floor-33 deck with Demon Form and no payoff; Heavy Blade
   must remain mainline and executable.
2. Verify a second payoff remains non-mainline.
3. Verify conditional-only strength support does not open the payoff slot.
4. Run strategy tests, rerun seed `20260713003`, and inspect the new first
   blocker rather than assuming the final boss is fixed.

