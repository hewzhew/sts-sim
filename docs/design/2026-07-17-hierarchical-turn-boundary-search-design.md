# Hierarchical Turn-Boundary Search

## Status

Frozen exact-state laboratory prototype.

This document records the experiment that established planning-granularity
evidence. It is not the migration target for production combat policy. The
production replacement direction is defined in
`2026-07-17-deletion-driven-oracle-turn-option-planner.md`; the later
public-information extension retains the useful mechanism evidence in
`2026-07-17-progressive-public-turn-option-design.md`.

## Decision

The combat search needs a second expansion topology, not another turn-plan
frontier hint.

Atomic action expansion remains the exact refinement mechanism. Hierarchical
turn-boundary expansion becomes the owner at stable player-turn boundaries:
it asks the existing exact turn planner for a small portfolio of complete-turn
outcomes and inserts those outcomes as macro edges into the global combat
frontier.

The first executable slice is laboratory-only. It is enabled explicitly by a
combat-search expansion policy and is not wired into production run-control
profiles.

## Evidence

The seed `20260713006` Guardian case did not produce a win under the atomic
search after a long bounded review. The same exact position produced
replayable wins when complete-turn states were inserted at player-turn
boundaries.

The decisive state required several individually incomplete attacks to cross
Mode Shift and cancel visible incoming damage. This is a planning-granularity
failure: the useful unit is a coupled action sequence ending at a stable
boundary, not its first action.

## Expansion Contract

At an unresolved, stable `CombatPlayerTurn` node with an empty turn prefix:

1. derive the remaining global node budget;
2. enumerate exact complete-turn candidates using that remaining budget and
   the global wall deadline;
3. retain a bounded, coverage-diverse portfolio of terminal, next-turn, and
   pending-choice outcomes;
4. charge all turn-planner inner expansions and generated states to the same
   global search counters;
5. insert selected exact end states into the global frontier without an
   additional rollout;
6. skip atomic expansion of the source node when the portfolio contains at
   least one candidate.

Pending choices and non-player-turn boundaries continue through atomic exact
expansion. If the turn planner produces no supported candidate without
exhausting the node or wall budget, the source node falls back to atomic
expansion and records the gap.

## Portfolio Contract

Each candidate is an exact executable action sequence plus its exact stable end
state. Candidate purposes are typed:

- terminal win;
- survival;
- progress;
- setup;
- balanced;
- pending choice.

Terminal losses, engine-step truncations, no-legal-action artifacts, and
unsupported other boundaries are not macro candidates.

The portfolio preserves objective diversity before filling remaining slots by
the existing exact-state evaluation. It is not a teacher label and does not
claim long-horizon optimality.

## Budget Ownership

There is one combat-search node budget and one wall deadline.

Turn-planner inner nodes are no longer hidden side work. They increment the
global expanded/generated counters and reduce the remaining node allowance for
later macro or atomic expansion. The turn planner receives the same deadline as
the outer search.

Macro end states use their exact boundary evaluation for frontier placement.
They do not trigger an additional rollout during portfolio insertion.

## Deduplication

Each exact source state may be macro-expanded at most once. Exact end states
still pass through the global transposition and dominance gates when popped
from the frontier.

The macro layer does not introduce a second terminal authority. Only the
existing global search terminal gate may accept a complete combat result.

## First-Slice Acceptance

The slice is accepted when:

- atomic expansion remains the default;
- hierarchical mode never atomically expands a source after successfully
  producing its macro portfolio;
- inner turn-planner work is visible in and constrained by the global node
  budget;
- pending-choice and unsupported gaps retain atomic refinement;
- the fixed Guardian combat case finds a replayable win under a bounded shared
  budget;
- focused combat-search tests pass.

Passing the Guardian case validates the planning mechanism only. It does not
prove seed `20260713006`, deck policy, or production combat policy.

## Later Migration

After the first slice is stable:

1. replace encounter-name activation with typed planning demand;
2. cache portfolios by exact state and planning contract;
3. tune portfolio width and per-purpose limits from scenario evidence;
4. compare the hierarchical policy across a small public scenario bank;
5. wire it into one production lane only when it replaces, rather than
   supplements, the old owner;
6. delete duplicated turn-plan frontier seeding and rescue loops after their
   responsibilities have moved.
