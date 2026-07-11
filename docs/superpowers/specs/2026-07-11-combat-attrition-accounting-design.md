# Combat Attrition Accounting Design

## Context

Accepted high-loss diagnostics currently preserve the exact combat start, the
selected executable trajectory, and the selected line's terminal HP loss. That
terminal loss is a persistent run outcome: it compares combat-entry HP with HP
after victory hooks. It does not say how low the player fell during combat or
how much post-combat healing masked the encounter's pressure.

For seed `20260711004`, the accepted Snake Plant line entered at 44 HP, fell to
8 HP, and finished at 20 HP after Black Blood. Describing this only as 24 HP
loss hides both the proven 36 HP combat drawdown and the 12 HP terminal rebound.
Conversely, treating all 36 HP as persistent loss exaggerates the damage carried
into the next floor.

## Considered Approaches

### Add diagnostic attrition accounting

Derive an explicit HP ledger from the exact accepted trajectory. This preserves
facts without deciding whether the deck needs more defense. This is the selected
approach.

### Classify repeated defensive failure immediately

Aggregate several combats and label the run as defense-deficient. This requires
unsettled choices about comparable encounter types, observation windows, and
unavoidable bad draws. It is deferred until the factual ledger exists.

### Feed recent attrition directly into card rewards

Use recent losses to change reward scoring. This risks turning one encounter or
one draw order into a permanent defensive bias and crosses the branch-runtime to
core-AI boundary. It is out of scope.

## Decision

Add a typed accepted-combat attrition summary with these fields:

- `start_hp`: HP at the captured stable combat start;
- `lowest_observed_hp`: the minimum HP in the captured start and every available
  selected-trajectory step snapshot;
- `observed_combat_drawdown`: `max(0, start_hp - lowest_observed_hp)`, explicitly
  a lower bound when observation is incomplete;
- `terminal_hp`: the selected winning line's final HP after victory processing;
- `terminal_rebound_from_observed_low`:
  `max(0, terminal_hp - lowest_observed_hp)`, without claiming that all of the
  rebound came from one particular relic or hook;
- `persistent_net_hp_loss`: `max(0, start_hp - terminal_hp)`;
- `observation_complete`: whether every selected action supplied an HP snapshot
  needed for the minimum calculation.

The terminal killing action may legitimately have no active-combat snapshot
because combat state has already transitioned through damage, victory hooks,
and room exit. That absence marks the minimum observation incomplete: the
ledger reports only the lowest HP it can prove and never invents an unseen
intra-action minimum.

The existing original/selected terminal summaries remain unchanged. The new
ledger describes only the line actually executed. It is written into the typed
diagnostic draft, evidence sidecar, and capsule result projection.

The diagnostic trigger expands to retain a committed accepted combat when any
existing original/selected net-loss trigger fires or when the selected line's
observed combat drawdown reaches 25% of maximum HP. Because the observed value
is a lower bound, this trigger can miss an unseen dip but cannot fire from an
invented one. This only changes artifact retention; it does not reject the line
or affect gameplay.

Evidence sidecars advance to `accepted_high_loss_combat_evidence_v2`. Capsule
discovery accepts both v1 and v2 so existing capsules remain readable. No v1
artifact is rewritten.

## Boundaries

- Do not change combat search ordering, budgets, acceptance, repair, or potion
  policy.
- Do not change reward, shop, route, campfire, deck-deficit, or owner policy.
- Do not label one combat as avoidable, unavoidable, or defense-deficient.
- Do not aggregate combat history or introduce a repeated-loss threshold yet.
- Do not special-case Black Blood, Burning Blood, Snake Plant, or the current
  seed.
- Do not rerun a full seed as a regression test; use typed fixtures and the
  existing exact capture for post-implementation inspection.

## Verification

Use test-driven development to prove:

1. `44 -> 8 -> 20` yields observed drawdown 36, terminal rebound 12, and
   persistent loss 24.
2. Ordinary no-heal damage reports zero terminal rebound and equal observed/net
   loss.
3. Any missing action snapshot, including a terminal transition, marks the
   ledger incomplete without discarding the proven minimum.
4. Observed drawdown alone can retain a diagnostic even when terminal recovery
   brings net loss below the existing threshold.
5. Capsule writing emits v2 evidence and discovery continues to recognize v1.
6. No gameplay decision records or search outcomes change in focused fixtures.

Run formatting, focused run-control and owner-audit tests, the full library
suite, and architecture-boundary tests. Then replay only the preserved Snake
Plant capture/evidence path to confirm the expected ledger; do not rerun the
preceding floors.

## Success Criteria

- High-loss evidence distinguishes encounter drawdown from persistent run loss.
- Post-combat healing is visible rather than silently folded into one HP number.
- Observed pressure can preserve a diagnostic without becoming a policy verdict.
- Existing v1 capsules remain discoverable.
- No combat or noncombat policy behavior changes.
