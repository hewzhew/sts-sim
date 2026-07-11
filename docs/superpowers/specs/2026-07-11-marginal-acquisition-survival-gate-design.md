# Marginal Acquisition And Survival Gate Design

## Goal

Prevent locally attractive setup cards from repeatedly claiming full engine or boss-scaling
credit when they do not materially improve the current deck. Under acute survival pressure,
immediate survival must not be displaced by redundant long-fight setup.

## Decision

### Deck-relative engine admission

`EngineSeed` is a deck-relative classification, not an intrinsic property of a card.
An event handler that requires a missing event stream is an unsupported payoff rather than a
live seed. A candidate that leaves an already supported package at the same maturity does not
become a new seed merely because its handler can stack.

This makes an unsupported first Rupture speculative and prevents a second Rupture from
repeating the original engine-seed credit. Mechanically stackable copies are not forbidden;
they need separate evidence that the copy repairs reliability or adds useful throughput.

### Marginal boss-scaling evidence

Full boss-scaling source credit requires one of two conditions:

- the candidate establishes the deck's first usable scaling source; or
- the candidate satisfies an explicit package-reliability repair.

An additional source in an already live package does not close the boss-scaling gap by default.
Preserve the existing strength-multiplier repair: a deck with a multiplier and exactly one
stable strength source may still admit a second stable source. Conditional sources and a third
stable source do not receive that exception.

### Acute survival gate

When the existing survival-pressure predicate is active, a candidate that supplies only setup
or long-fight scaling is capped below the mainline unless it closes a hard requirement or a
boss-specific survival plan. This is a categorical lane gate, not another large score penalty.

Immediate block is survival-relevant even when the long-run block inventory is already
adequate. Pure block therefore qualifies for the heavy-burden survival exception; block plus
draw, Weak, and enemy Strength reduction keep their existing treatment.

The intended invariant is that an upgraded immediate block card at low HP is not demoted while
a redundant setup power is automatically selected on the following reward.

## Data Flow

1. Package transition and reward quality determine whether the candidate establishes,
   supports, repairs, or merely repeats a package.
2. Reward admission assigns `EngineSeed` only to a real deck-relative establishment.
3. Boss-scaling evidence grants full gap credit only to a usable first source or an explicit
   reliability repair.
4. Strategic and acquisition lane caps apply the acute survival gate after scoring, so a high
   setup score cannot bypass the boundary.

## Stable Tests

- A Rupture without a self-damage stream is not a mainline shop acquisition.
- A second Rupture in a supported self-damage package is not classified as a new engine seed
  and is not mainline under survival pressure.
- Flame Barrier+ remains mainline under survival pressure even when starter/deck burden is
  heavy.
- A multiplier with exactly one stable strength source may still accept a second stable source.
- A third stable source and conditional strength sources do not masquerade as that reliability
  repair.

Tests assert classifications and lane relationships, not exact aggregate scores, exact seed
paths, or boss outcomes.

## Non-Goals

- Do not redesign Velvet Choker, Runic Pyramid, Apparition, or generated-startup-card
  compatibility in this pass.
- Do not change combat search.
- Do not replace the strategic-deficit schema or tune every acquisition score.
- Do not add an exact replay, frontier, checkpoint, or full-seed regression test.
- Do not forbid duplicate powers globally.
