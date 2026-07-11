# Intangible Pressure Projection Design

## Problem

Fresh seed `20260711002` reaches A3F48 Time Eater at 18/39 HP with five
Apparitions. The opening draw contains an Apparition, but the bounded owner
search finds no win in 10 seconds and a focused all-potion review still finds
no win after 20 seconds and 14,133 expanded nodes. Its closest complete line
does not play the opening Apparition and dies on turn one.

Combat search computes pressure as player HP plus block minus visible incoming
monster damage. That visible damage remains the monster's raw intent after the
player gains `IntangiblePlayer`. The simulator correctly caps attack damage to
one per hit, but search continues to score the exact post-Apparition state as
`18 - 26 = -8` survival margin. Rollout probes and frontier lanes therefore
treat a genuinely protected state as forced loss.

## Approaches Considered

1. Special-case Apparition in card ordering. This is rejected because it only
   masks one producer and leaves Wraith Form, Ghost in a Jar, and other exact
   Intangible states incorrectly valued.
2. Change the global monster intent preview to post-mitigation damage. This is
   rejected because the preview is also the truthful raw intent boundary; it
   should not silently become player-specific effective damage.
3. Project effective incoming damage only inside combat-search pressure. This
   preserves raw intent and fixes every `IntangiblePlayer` source. This is the
   selected approach.

## Decision

Keep `monster_preview_total_damage_in_combat` unchanged. In
`combat_search_v2::pressure_value`, derive search-only effective incoming
damage from each visible monster preview:

- without player Intangible, retain the current raw total damage;
- with positive `IntangiblePlayer`, cap each attack hit to one and use the
  preview hit count as the effective total;
- continue combining effective incoming damage with current block and HP in
  the existing survival-margin formula.

The rule reads exact player power state, not card identity. Once an Apparition
is applied by the normal simulator transition, the existing one-step rollout
probe can observe the corrected child survival value without a new strategy
hint.

## Scope

- Change only combat-search pressure projection and its focused mechanical
  test.
- Do not change simulator damage resolution, monster intent reporting, card
  ordering tables, run-control, owner HP acceptance, or public report shape.
- Do not add a Time Eater, Apparition, Runic Pyramid, or seed-specific rule.
- Do not generalize to Buffer or unrelated defensive powers in this change.

## Verification

Add one mechanical regression test showing that active player Intangible caps
visible search pressure per attack hit while the same fixture without
Intangible retains raw damage. Verify RED before implementation and GREEN
afterward. Then run the full library and architecture boundary suites, and
rerun the frozen Time Eater case under the original 10-second owner-equivalent
profile. Success requires the post-Intangible pressure fact to be correct; a
combat win is useful acceptance evidence but is not manufactured through
additional policy exceptions.
