# Construction Reliability Boundary Design

## Goal

Move the mainline failure boundary upstream from Collector combat tactics to
deck construction. The policy should distinguish real first-cycle access from
small cantrips, avoid adding ordinary cards that deepen an existing energy
problem, and let an Act 2 energy relic solve that problem when its constraint
is not contradicted by the run.

## Decision

### Effective access

Use the existing construction-pressure model as the source of truth for deck
access. One real draw card plus small cantrips remains `Thin`; two real draw
sources, or real draw plus energy access, becomes `Adequate`. Pommel Strike and
Shrug It Off are useful cantrips, but do not by themselves claim that a deck
can deploy its setup reliably.

Card acquisition uses the same distinction. A real draw card can improve an
access gap. A small cantrip improves the axis only when it actually moves the
construction pressure to a better level; its `CardDraw` mechanic alone is not
enough.

### Deployability debt

When energy is missing or thin and the deck already contains expensive cards,
another ordinary two-or-more-cost reward is speculative unless it fixes a
hard strategic gap or supplies energy. This is a narrow admission guard, not a
general card tier list and not an archetype prescription.

### Act 2 energy relic boundary

Sozu may enter the mainline Act 2 energy-gap lane when the run has no relic
that specifically depends on acquiring more potions. White Beast Statue,
Sacred Bark, or Potion Belt keeps Sozu in the constrained/probe lane. Existing
held potions do not block Sozu because they remain usable.

## Stable Tests

- A representative 18-card deck with Burning Pact, Pommel Strike, and Shrug It
  Off has thin access; adding Battle Trance makes access adequate.
- A small cantrip does not receive access-gap credit when it does not improve
  construction pressure; real draw still does.
- An ordinary expensive reward becomes speculative in an already
  energy-constrained deck, while hard-gap and energy solutions remain allowed.
- Sozu outranks Black Blood for an unconstrained Act 2 energy gap, but remains
  a probe when potion-synergy relics make its drawback strategically live.

Do not lock an exact seed path, card sequence, boss outcome, or deck size.

## Non-Goals

- Do not add more Collector-specific combat policy.
- Do not force a card archetype or fixed pick list.
- Do not replace the construction-pressure model with another schema.
- Do not redesign every acquisition axis or potion valuation in this pass.
- Do not add checkpoint, panel, frontier JSON, or exact replay tests.
