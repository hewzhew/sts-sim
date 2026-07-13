# Strength Source Is Not a Strength Payoff

## Observed gap

Fresh seed `20260713004` reaches Donu and Deca with the strategic deficit
`boss_scaling_plan=missing`.  On Act 3 floor 42 it is offered upgraded Inflame.
The candidate has raw mainline score 300 and records that it closes the
strength requirement, but both RoleSaturation and Acquisition cap it to a
probe.  The recorded marginal penalty is `duplicate-strength-payoff` even
though the deck has no stable strength source.

The same conflation appears earlier for Spot Weakness in the Act 2 shop.

## Root cause

`Supports(Strength)` and `Closes(WantsMechanic(Strength))` describe a package
transition.  They do not say whether the candidate is the source or the damage
payoff.  A strength source offered to a deck containing a payoff naturally
emits those reasons.  Treating either reason as payoff identity makes the
first source look like a duplicate payoff and also makes acquisition call it a
fragile supported payoff.

## Design

Define a strength payoff only from the candidate's own damage semantics:

- `DamageUses(Strength)`, or
- `DamageScalesWith(PerHitStrength)`.

Do not infer payoff identity from `Supports` or `Closes`.  Use the shared
predicate in role saturation so boss-scaling, acquisition, and saturation do
not maintain different meanings.

This preserves duplicate limits for Heavy Blade, Sword Boomerang, Pummel, and
other real strength-scaled damage while allowing Inflame, Demon Form, Spot
Weakness, and Limit Break to be evaluated as sources or multipliers.

## Verification

- In a deck with a strength payoff but no source, upgraded Inflame must remain
  mainline and receive neither RoleSaturation nor Acquisition caps.
- A real strength-scaled damage card must still satisfy the payoff predicate.
- Existing strategy, acquisition, and boss-scaling tests remain green.
- A fresh seed run determines the resulting path; no whole-seed behavior is
  frozen as a unit test.
