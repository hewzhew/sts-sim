# Finite-Survival Damage-Mitigation Design

## Problem

The simulator executes Transient's stable combat mechanics: positive `Fading`
ends the encounter after a bounded number of owner turns, while `Shifting`
turns actual HP damage dealt to the enemy into temporary Strength loss for the
current turn.  Combat search does not expose this combination as a typed enemy
mechanic.  `enemy_mechanics_adaptive_no_potion` therefore falls back to its
conservative rollout, which spends energy on ordinary block and evaluates the
999-HP enemy as a conventional damage race.

With immediate child rollouts, the resulting losing estimates dominate
frontier priority.  On the retained seed006 A3F39 capture, immediate adaptive
rollout found no win after 85,611 expanded nodes and 20 seconds.  The same
position with the existing phase-aware rollout found a win at node 181 within
one second, ending at 44 HP without a potion.  This isolates the missing
mechanics dispatch rather than a run-control budget or lane problem.

## Considered Approaches

1. Force `LazyOnPop` for Transient hallway lanes.  This is proven to recover a
   win, but it routes around the incorrect rollout model and couples
   run-control to an encounter-specific workaround.
2. Detect the semantic `Fading + Shifting` combination in the typed enemy
   mechanics profile and let adaptive rollout select the existing phase-aware
   policy.  This is the selected approach: it is name-independent, reuses a
   policy already validated on the exact capture, and leaves lane ownership
   unchanged.
3. Add a dedicated Transient evaluator and finite-horizon dynamic program.
   This could later improve HP optimality, but it is unnecessary for restoring
   reliable search and would duplicate existing exact engine behavior.

## Decision

Extend `EnemyMechanicsProfileV1` with two read-only facts:

- `finite_survival_damage_mitigation_target_count`: the number of living
  enemies that simultaneously own positive `Fading` and `Shifting`;
- `finite_survival_damage_mitigation_min_owner_turns`: the minimum positive
  `Fading` amount among those enemies.

The detector reads powers, not `EnemyId`, so its meaning is the combat
mechanism rather than the Transient name.  `Fading` alone and `Shifting` alone
do not activate this combined fact.  The facts are diagnostic and dispatch
inputs only: they do not prune actions, modify combat state, claim a terminal,
or replace exact replay.

`enemy_mechanics_adaptive_no_potion` selects `PhaseAwareNoPotion` when the
combined target count is positive.  Existing Guardian and Bronze Automaton
dispatch remains unchanged; all other encounters retain the conservative
fallback.  Run-control lane profiles remain unchanged because the controlled
experiment shows that immediate child rollout is reliable once the adaptive
policy selects the correct existing rollout behavior.

## Reporting and Compatibility

Expose both facts in `CombatSearchV2EnemyMechanicsReport` and update the rollout
report note so artifacts explain the additional adaptive dispatch.  Adding
serialized report fields increments `CombatSearchV2Report` schema version from
12 to 13.  No persisted input format changes.

The stale `Shifting::at_end_of_turn` MVP comment is removed or corrected: the
real restoration owner is the paired `Shackled` power.  This is documentation
cleanup only and must not change combat behavior.

## Verification

Use focused red-green contracts rather than a brittle full-line unit test:

1. a non-Transient test monster with positive `Fading` and `Shifting` exposes
   count one and the exact remaining-turn minimum in both internal and
   serialized profiles;
2. either power in isolation exposes no combined target;
3. adaptive rollout chooses phase-aware only for the combined mechanism while
   preserving the conservative fallback for the isolated cases;
4. the combat-search report advertises schema version 13;
5. existing enemy-mechanics, rollout-cache, full library, and
   `architecture_runtime_boundaries` tests remain green;
6. the retained seed006 A3F39 capture is rerun with immediate adaptive rollout
   under a one-second bound and must produce a replay-verified win without
   requiring a potion.  This acceptance run is artifact evidence, not a test
   that locks one temporary action sequence.
