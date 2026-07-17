# Progressive Public Turn-Option Migration

## Decision

Combat planning will grow from the existing public scenario-policy boundary,
not from the exact-state hierarchical turn portfolio.

A turn option is a closed-loop public policy over combat information sets. It
chooses one public action at each visited information set and may choose a
different continuation after an action reveals different public information.
Exact scenario states remain private transition witnesses.

The exact hierarchical turn-boundary search is frozen as a laboratory
prototype. It proved that coupled within-turn planning matters, but its fixed
exact action sequences, fixed portfolio width, and exact-state ownership are
not the production architecture.

## Public Option Shape

```text
public information set
  -> one public action
  -> exact binding and simulator step in every scenario
  -> zero or more terminal outcomes
  -> regroup by newly revealed public history
  -> one continuation decision per successor information set
```

The complete option eventually terminates at the next player-turn boundary, a
terminal win/loss/Smoke Bomb escape outcome, or an explicitly supported pending-choice boundary.
It is a policy tree, not a precommitted exact input sequence.

## Resumable Widening Contract

Candidate expansion is monotone and resumable:

1. an expansion session owns one public information-set group;
2. canonical public candidates are immutable action keys, while successful
   expansion and retained exact transitions are tracked separately;
3. a policy-controlled scheduler may select one unopened public action from a
   context containing only public observation, public results, action state, and
   a budget snapshot;
4. opened actions retain their exact successor groups for later closed-loop
   continuation, and consuming a transition does not make that action unopened;
5. later widening uses the updated public action-key state without replaying
   earlier actions;
6. status is one tagged value: either `exhausted` or `partially_expanded` with
   one typed cause;
7. unopened candidates remain unknown and must never be interpreted as losing,
   dominated, or irrelevant.

Stable public candidate enumeration remains the compatibility schedule. It is
deterministic but is not a quality ranking or an implicit strategy owner. The
scheduler selects an action key scoped to one information set, never an array
cursor or an exact simulator input.

## Shared Budget Contract

One runtime budget is passed through root and successor information-set
expansion. It owns:

- candidate evaluations;
- total exact engine steps;
- one optional wall deadline.

The engine-step allowance applies to the whole information-set action, not
once per hidden scenario world. As each exact world is stepped, it receives
only the remaining allowance. A larger scenario bank therefore cannot
silently multiply the configured work limit.

Budget stops are public and typed:

- `candidate_evaluation_budget`;
- `engine_step_budget`;
- `deadline`.

They produce a partially expanded result. The candidate that could not reach a
stable public boundary remains unopened and unknown. A later explicit budget
grant adds candidate/engine allowance and refreshes the deadline without
resetting cumulative accounting or replaying exact open leaves. No fallback or
loss verdict is created.

All scenario-step failures carry consumed engine work. Public TurnOption
reports expose only a typed stop; exact scenario identity and UUID-bearing
boundary diagnostics remain crate-private. A public-boundary failure also names
the attempted public action, while leaving that action unopened for an explicit
retry or a different scheduler choice. Budget grants validate every new limit
and deadline before atomically changing the shared budget.

## First Executable Slice

`CombatTurnOptionPrefixExpansionSessionV1` opens one-decision public policy
prefixes. Every opened prefix reports:

- its public root action;
- terminal win/loss/escape counts;
- continuing scenario count;
- public successor information sets and turn boundaries;
- exact engine work charged by the transition;
- the complete public action-key expansion order chosen so far.

The session privately retains the successor `CombatScenarioGroupV1` values so
the same mechanism can be applied recursively after public information
branches. `CombatTurnOptionWideningScheduleV1` lives beside `scenario`, not
inside it. An architecture check prevents that scheduler from naming exact
groups, positions, inputs, bindings, or scenario identities. Unknown actions,
already-expanded actions, and false exhaustion are rejected before budget or
session state changes.

## Observable Effect Evidence

After a candidate expands successfully, its session caches a public-only
observable-effect projection. An unopened candidate has no effect evidence;
the scheduler receives only an optional immutable reference to evidence that
already exists.

Each canonical successor bucket contains only:

- a typed public boundary kind;
- public turn count;
- public observation hash;
- public candidate-set hash;
- scenario multiplicity.

The complete projection also carries input, continuing, and terminal
win/loss/escape multiplicities. It deliberately excludes the root action,
`public_history_id`, exact inputs and worlds, exact engine work, card UUIDs,
RNG state, and scenario identity. Different public history edges may therefore
produce the same observable successor shape without becoming the same policy
edge. Evidence is serializable for diagnostics but intentionally cannot be
deserialized into a trusted value; any future persistence or IPC reader must
revalidate and canonicalize its input.

Comparison has three typed results:

- unequal complete public shapes are `observably_different`;
- equal nonterminal public shapes are `observably_same`;
- missing or malformed evidence is `inconclusive`;
- matching candidates with any terminal outcome are also `inconclusive` until
  the terminal handoff exposes authoritative post-combat HP, potions, gold,
  deck changes, and persistent relic state.

`observably_same` is novelty evidence only. It does not authorize policy-node
merging, exact-transition reuse, pruning, dominance, value equivalence, belief
equivalence, or an inference that unopened candidates are irrelevant.

`CombatPublicTurnOptionCompositionSessionV1` then consumes already-opened
transitions without replaying the simulator. It owns the one
`CombatTurnOptionExpansionBudgetV1` and every exact prefix session for its open
leaves; no transition-bearing leaf token is issued outside that owner:

- same-turn player boundaries and pending choices remain open leaves;
- newly revealed public groups remain separate open leaves;
- the next player turn and typed win/loss/escape outcomes close their scenario leaves;
- the option is complete only when no open leaf remains.

Composition records only public information-set keys, public actions, public
successor boundaries, and scenario counts. Commits preflight every successor
key before consuming the selected exact transition, so collision errors are
transactional. Decisions are serialized in information-set-key order, making
the same policy tree independent of sibling traversal order. Composition does
not rank novelty or alter production combat behavior.

## Migration Hold And Oracle Gate

The public observable-effect evidence slice is the stopping point for this
migration. It is executable but does not alter scheduling or production combat
behavior. Authoritative terminal handoff, bounded-search metareasoning, richer
public schedules, and replacement of exact-state combat ownership are deferred.

Work returns first to the Oracle benchmark: the planner may use the complete
simulator state and fixed hidden realization while diagnosing whether it can
find a known winning trajectory within a bounded cost. The hidden-information
migration resumes only after that benchmark can repeatedly:

- reproduce known winning continuations on a small curated combat/run set;
- distinguish a missing action or transition from a bad selection decision;
- report a typed evidence or budget gap when it cannot decide;
- stay within an explicit engine-step and wall-time envelope.

Passing this gate does not prove general game strength. It establishes the
smaller prerequisite that adding information-set branching will not merely
magnify an unresolved full-information planning failure.

Semantic labels such as threat interruption or phase control may remain
diagnostic observations. They do not own candidate enumeration, termination,
or fallback.

## Acceptance

The first slice is accepted when:

- widening one action and then one more does not replay the first action;
- a scheduler may expand a non-prefix action first, and the report preserves
  that actual action-key order;
- selecting an unknown or already-expanded action, or falsely reporting
  exhaustion, changes neither budget nor session state;
- exhausting a session and widening again performs zero engine work;
- hidden draw-order worlds share the root information set;
- a revealing action may split them into distinct successor information sets;
- each successor can independently continue through the same public expansion
  interface;
- root and successor information sets consume one candidate/engine/deadline
  budget;
- a scenario group cannot multiply the engine-step limit by its hidden-world
  count;
- budget exhaustion is typed and leaves unopened actions inconclusive;
- an explicit budget grant retries the same unknown candidate without resetting
  prior accounting;
- Smoke Bomb closes through a typed `escape` leaf rather than an unsupported
  boundary or false victory;
- sibling public branches remain open independently until each one closes;
- reversing sibling commit order produces the same canonical public option;
- duplicate successor rejection is transactional and leaves the selected
  transition available for a later valid commit;
- already-opened transitions compose into a complete next-turn option without
  another simulator step;
- serialized expansion reports contain no exact card UUID, RNG state, or
  scenario identity;
- different identical-Strike actions may keep distinct public histories while
  comparing `observably_same`;
- Strike and Defend successor effects compare `observably_different`;
- a terminal effect compared with itself remains typed `inconclusive` until an
  authoritative terminal handoff exists;
- malformed multiplicities produce typed inconclusive evidence rather than a
  panic;
- serialized observable-effect evidence contains no root action,
  `public_history_id`, UUID, RNG state, scenario identity, or exact engine work;
- unopened widening candidates expose no observable-effect evidence and cannot
  be inferred losing or duplicate;
- production search configuration and run-control behavior remain unchanged.
