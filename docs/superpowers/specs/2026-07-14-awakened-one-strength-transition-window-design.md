# Awakened One Strength-Transition Window Design

## Problem

The combat engine correctly executes the stable Slay the Spire interaction:
temporary enemy Strength loss applies both negative Strength and `Shackled`;
when Awakened One's first phase dies, the immediate debuff purge removes
`Shackled`, removes negative Strength, and preserves positive Strength.  If the
pre-card positive Strength is `S` and the temporary loss is `D`, phase two
therefore retains `max(0, S - D)` Strength.

Combat search observes temporary versus persistent Strength loss and values
mitigation against the currently visible attack.  It does not identify the
two-action ordering window in which temporary Strength loss is played before
the first-phase killing blow.  Exact search can discover the result after both
actions, but action ordering may discard the setup branch before reaching that
state.

## Considered Approaches

1. Add a `Dark Shackles` card-name bonus against Awakened One.  This is narrow
   but duplicates card mechanics and cannot cover another temporary
   Strength-down source.
2. Add unrestricted two-action lookahead to action ordering.  This is more
   exact but repeats the combat stepper inside every ordering comparison and
   expands a hot search path.
3. Add a typed, conservative transition opportunity derived from existing
   action-effect facts, Awakened One runtime state, and a remaining-hand
   damage upper bound.  Use it only for ordering and diagnostics; let the real
   stepper establish the resulting state.  This is the selected approach.

## Decision

Extend the read-only enemy mechanics profile with the minimum Awakened One
runtime facts needed by consumers:

- whether a targetable Awakened One is still in form one;
- its current HP plus block;
- its current positive Strength.

For a playable action that applies temporary enemy Strength loss to that
target, compute a transition opportunity only when the remaining playable
hand has enough damage upper-bound capacity to finish form one after the
setup.  The opportunity reports:

- temporary Strength loss applied;
- positive Strength convertible through the purge, capped at the target's
  current positive Strength;
- whether the remaining-hand damage upper bound reaches the phase-one HP and
  block threshold.

The upper bound is deliberately evidence of an opportunity, not proof of a
line.  It must account for the setup card's cost and removal from hand, but it
does not perform nested search.

## Search Consumption

The phase-action ordering layer gives a bounded setup bonus to a positive
transition opportunity so the temporary-strength action is explored before a
direct killing blow.  The fact:

- does not prune any action;
- does not declare a terminal or a guaranteed phase transition;
- does not modify combat state;
- does not replace the existing visible-attack mitigation value.

After the action is explored, normal search and the real combat engine own all
subsequent results.  If the projected damage cannot actually be assembled in
the required order, the branch receives no fabricated state benefit.  If the
boss is already in form two, has no positive Strength to convert, or the hand
cannot plausibly finish form one, no transition bonus is emitted.

## Disarm Interaction

Persistent Strength loss already present on the boss reduces the remaining
positive Strength and therefore the numeric conversion amount, but it does
not disable the window.  This preserves the real complementarity: `Disarm`
improves every later attack while temporary Strength loss can cover a peak
multi-hit or finish reducing retained phase-two Strength to zero.

## Verification

Add focused red-green regressions for:

1. temporary Strength loss followed by reachable form-one lethal producing a
   positive transition opportunity and ordering bonus;
2. the same opportunity remaining available after persistent Strength loss,
   with only the convertible amount reduced;
3. form two, zero positive Strength, insufficient remaining damage, and
   persistent-only Strength loss producing no temporary transition bonus;
4. an exact engine sequence applying temporary Strength loss before lethal and
   preserving `max(0, S - D)` Strength after the immediate purge;
5. existing Time Eater, Guardian, Lagavulin, and split-transition ordering
   tests remaining unchanged.

At completion, run the full library suite and
`architecture_runtime_boundaries` as required by the repository workflow.
