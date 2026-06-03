# Collateral Exhaust Cost Of Immediate Conversion

This note defines one narrow candidate dimension for `decision_audit`.

It is intentionally small.
It is not a general exhaust-engine evaluator.

## Problem

Current local terms can reward:

- immediate threat relief
- immediate defense stabilization
- immediate burden removal

They do not express:

> when an immediate conversion action clears junk but also burns valuable
> non-attack cards that were still near-term live resources

This is the specific blind spot exposed by the `Power Through -> Second Wind`
net-value cases.

## Intended Meaning

`collateral_exhaust_cost_of_immediate_conversion` should mean:

> if the current line immediately plays a conversion action that exhausts a set
> of non-attack cards, what is the near-term opportunity cost of the valuable
> collateral cards that get burned along with the junk?

This is not:

- a generic "Second Wind bonus"
- a generic "status relief" term
- a long-horizon deck engine score

## Scope

The first implementation should stay narrow.

It should only answer three questions:

1. What counts as an immediate conversion action?
2. Which exhausted cards count as collateral?
3. Which collateral cards are valuable enough to charge a cost?

## First-Pass Definitions

### 1. Immediate conversion action

An action in the current line that:

- is played immediately in the current decision window
- exhausts or converts a set of non-attack cards as part of the action

The motivating example is `Second Wind`, but the interface should not be named
after a specific card.

### 2. Collateral exhausted card

A non-attack card exhausted by that conversion action that is **not** the main
garbage payload the action was intended to clean up.

Examples of likely garbage payload:

- `Wound`
- `Slimed`
- other obviously dead non-attack clutter in the same immediate window

Collateral means:

- cards burned incidentally while taking the conversion line

### 3. High-value collateral cost

A collateral exhausted card should contribute cost only if it is both:

- plausibly valuable
- plausibly near-term live

The first pass should stay conservative:

- use a very small explicit whitelist of already-validated motif cards
- avoid a large card-knowledge table

The motivating cards are:

- `Barricade`
- `Entrench`

This is acceptable for v1 because the experimental evidence is currently only
about this motif.

## What The First Version Should Not Try To Solve

Do not try to solve:

- all exhaust synergies
- all power/setup valuation
- long-horizon future draw quality
- deck-specific engine evaluation
- global "important card" theory

The first version only needs to improve ranking on the validated immediate
conversion motif.

## Integration Shape

The cleanest shape is:

- detect an immediate conversion action in the audited line
- inspect the exhausted non-attack set
- separate obvious junk from valuable collateral
- add a cost for the valuable collateral subset

Conceptually:

`net immediate conversion value = junk relief + block/threat benefits - collateral exhaust cost`

This note only defines the last term.

## Success Criteria

The first implementation is good enough if, on the existing three net-value
cases:

- `pure_gain` still ranks correctly
- `pure_loss` pushes `Second Wind first` / `Power Through -> Second Wind` down
- `mixed` no longer treats valuable collateral burn as invisible

## Non-Goals

This design note does not authorize:

- changing the broader search stack
- redesigning `decision_audit`
- adding multiple new local dimensions at once

One narrow blind spot was identified.
This note exists to keep the fix narrow too.
