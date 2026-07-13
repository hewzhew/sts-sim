# Card-flow capability progress design

## Problem

The acquisition contract currently treats a card as improving card flow only when the projected
`PressureLevel` changes. That loses meaningful progress inside the coarse `Open / Thin / Present`
levels.

In seed `20260713002`, the deck had two small cantrips and no real draw. `Battle Trance` added the
first real-draw source, but the aggregate level remained `Thin`. Strategic scoring awarded the card
for the access gap, while acquisition called it speculative and capped its raw mainline score to a
probe. In the one-branch runner that made a 220-point card lose to a 120-point third `Cleave`.

## Decision

Keep the existing level-transition rule and add two bounded forms of within-level progress:

- gaining the first real-draw source;
- gaining the first energy-access source.

Do not treat another small cantrip, or another already-present real-draw/energy source, as progress
unless it raises the aggregate pressure level through the existing rule.

This changes only acquisition's answer to "does this candidate improve card flow?". It does not
change lane ordering, score thresholds, shop policy, or the meaning of probe/reject.

## Expected effect

High-confidence access cards can establish a missing capability without being trapped below the
mainline by the coarse pressure bucket. Repeated low-marginal access remains capped. The exact
seed-`20260713002` Battle Trance state becomes a mainline candidate; Flash of Steel and repeated
Pommel Strike remain governed by the existing small-cantrip and saturation rules.

## Verification

- Unit regression for thin flow made only from small cantrips: first Battle Trance improves flow.
- Unit regression that an additional small cantrip does not improve the same state.
- Decision-pipeline regression reconstructed from the seed: Battle Trance is mainline and beats the
  redundant Cleave.
- Focused tests, full library tests, architecture tests, formatting, and a fresh bounded rerun of the
  seed.
