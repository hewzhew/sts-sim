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
