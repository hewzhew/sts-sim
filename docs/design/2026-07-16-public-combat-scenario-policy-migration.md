# Public Combat Scenario Policy Migration

## Decision

Combat policy will migrate from exact-state trajectory ownership to a sampled
scenario policy tree constrained by public observation history.

The exact simulator remains the world model and witness verifier. It must not
directly expose draw order, RNG state, hidden intent, or scenario-specific
identities to the policy boundary.

## Core Boundary

```text
exact scenario worlds
  -> public history + public observation + public legal candidates
  -> one information-set group
  -> one public action for the whole group
  -> exact action binding inside each world
  -> simulator step
  -> regroup by newly observed history
```

Two worlds may share a policy group only when:

1. their public history identifiers match;
2. their public observation payloads match;
3. their public legal candidate sets match.

The policy receives only the group view. Exact `CombatPosition`, card UUIDs,
monster entity IDs, RNG streams, and hidden draw order remain private world
data. One public action is bound separately to the exact legal input in every
scenario.

## First Slice

The first executable slice:

- moves the combat public-observation schema to an AI-owned boundary;
- defines public player-turn action identity without card UUIDs or monster
  entity IDs;
- groups exact combat scenarios by public history, observation, and candidate
  set;
- fails closed on unsupported boundaries, unsupported actions, duplicate
  scenario identity, hash disagreement, or ambiguous action binding;
- proves that hidden draw-order variants share a group without Frozen Eye and
  separate with Frozen Eye;
- proves that one public target action binds to different exact entity IDs
  across otherwise indistinguishable scenarios.

This slice does not change production combat behavior and does not claim that
the current public observation schema is complete enough for production
policy. Under-observation may reduce policy strength, but it must never reveal
hidden state.

## Second Slice

The second executable slice establishes a policy-specific observation and the
first closed-loop transition:

- keeps `CombatPublicObservationV1` stable as the compatibility evidence and
  fingerprint boundary;
- adds `CombatPolicyObservationV1` for public, mechanically relevant combat
  state: detailed card runtime without UUIDs, visible pile contents, turn
  counters, stance, orbs, relic counters, powers, and public power timing;
- keeps RNG streams, card UUIDs, monster entity IDs, and power instance IDs
  outside the policy payload;
- rejects half-resolved player turns with pending action queues, queued cards,
  limbo cards, or active card resolution;
- binds one public action across every exact world, steps each world to a
  stable boundary, records a public-only history transition, and regroups by
  the newly observed state;
- proves that hidden RNG and hidden draw order stay grouped until an action
  reveals a different public result, at which point the successor information
  sets separate.

Pending combat choices remain a typed unsupported successor in this slice.
The closed loop supports only quiescent `CombatPlayerTurn` boundaries and
terminal win/loss outcomes. Production combat ownership remains unchanged.

## Third Slice

The third executable slice extends the same information-set loop across every
combat-local pending-choice kind:

- hand and grid selections use public card-state multiplicities rather than
  card UUIDs;
- Scry uses the publicly revealed card order and public reveal indices;
- Discovery, combat card rewards, Foreign Influence, Choose One, and stance
  choices expose typed public options;
- exact selection UUIDs remain private bindings inside each scenario;
- duplicate exact selections that represent the same public card multiset are
  collapsed before the policy sees them;
- pending-choice action enumeration is complete up to an explicit 4096-action
  safety boundary and returns `CandidateSpaceTooLarge` above it;
- the scenario policy no longer inherits the legacy Hand/Grid enumeration
  path that silently retained at most sixteen combinations.

The closed loop can now transition from a player action into a pending choice,
commit one public selection across all grouped scenarios, resume exact engine
resolution, and regroup at the next public boundary. This remains laboratory
infrastructure; production combat ownership is still unchanged.

## Fourth Slice

The fourth executable slice connects compiled combat-laboratory samples to a
shared public-policy scenario bank:

- all samples enter one information-set queue rather than one exact-search
  cell per sample;
- the policy interface receives only the public information-set view, decision
  index, and public-history depth;
- one selected public action is applied to every exact world in the group;
- newly revealed observations split later groups naturally, while hidden-only
  differences continue sharing one decision;
- per-sample outcomes retain public action history, observed HP loss, turns,
  cards played, and potion usage without exposing exact UUIDs or RNG state;
- the report is explicitly marked
  `PublicHistoryScenarioPolicy` and separates win, loss, and typed unresolved
  coverage;
- distribution output includes resolution and win rates plus terminal, win,
  and loss HP-loss summaries with median, p90, and maximum tail loss.

This slice does not yet replace the existing journal/cell runner. The old
runner still owns `ExactStateOracle` artifacts until the public-policy bank has
its own durable manifest and a real scenario-aware action-selection policy.
That cutover must replace the old executor rather than leave both as permanent
owners.

## Migration Gates

1. **Information-set foundation:** public grouping and single-action binding
   are executable and tested.
2. **Closed-loop lab:** the combat laboratory executes one public policy tree
   over paired sampled scenarios rather than optimizing each exact seed
   independently.
3. **Distributional evidence:** comparisons report win/death rates, HP-loss
   distribution, tail loss, potion use, and unresolved coverage under the same
   scenario bank.
4. **Production cutover:** `BoundedRunDriver` commits only the current public
   policy action. Exact-state combat lanes lose direct commit authority in the
   same delivery.
5. **Old-owner deletion:** exact search remains an internal oracle, simulator,
   and witness checker; its production semantic-owner path is removed rather
   than retained as fallback.

## Non-Goals

This migration does not:

- add another decision microscope;
- encode seed006, Awakened One, or a preferred opening action;
- treat a bounded search miss as an unwinnable combat;
- train a model or introduce reinforcement learning;
- preserve current heuristic scores or string reasons as planner inputs;
- enumerate shop bundles or every future noncombat combination;
- increase stack size or search time as an architectural fix.

## Deletion Rule

No new public scenario planner may become a permanent parallel owner. A
production cutover is complete only when the replaced exact-state commit path
is deleted and unsupported results become typed gaps.
