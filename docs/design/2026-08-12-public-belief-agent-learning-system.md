# Public-Belief Agent Learning System

Status: **active clean-room architecture**

This is the single maintained design for the learned agent. It replaces the
former combat-search-improvement contract. Existing search, trainer, case, and
experiment schemas may be deleted or migrated when they obstruct this design;
they are evidence and bootstrap material, not compatibility requirements.

## The Problem We Are Actually Solving

Slay the Spire is a long-horizon, partially observed stochastic control
problem. A deployed policy observes public run and combat history, chooses one
legal typed action, and eventually receives the only authoritative objective:
whether the complete run wins. Deck, relic, potion, map, encounter, ascension,
draw order, monster intent rules, and future chance all change the value of an
action. A combat witness or local HP score cannot replace that continuation
value.

The learned agent is therefore one architecture at every ascension:

```text
exact simulator mechanics
  -> public information state + private action-resolution table
  -> belief environment over hidden state and future chance
  -> information-set search with a policy/value prior
  -> visit policy + complete-run value targets
  -> occurrence-weighted replay and reanalysis
  -> recurrent typed policy/value model
  -> natural complete runs
  -> paired full-run candidate promotion
```

A0 is a lower-variance curriculum distribution. A20 is the final distribution.
Changing ascension must not switch action schemas, objectives, or agent
architecture.

## Formal Boundary

At decision time the environment owns an exact simulator state `x_t`, while
the agent receives public information `i_t = O(h_t)`, where `h_t` is the public
action-observation history. The agent never receives the exact hidden draw
order, private RNG state, hidden intent, or simulator allocation handles.

The belief environment represents

```text
b_t(x) = P(x_t = x | public history h_t)
```

through typed particles or another explicitly declared approximation. Search
compares root actions on matched belief particles. It returns a distribution
over the complete public legal-action set and a run-win value estimate. It
does not return a privileged exact-future witness disguised as a label.

## Ownership

### `src/agent/information`

Owns semantic public state. `PublicCombatStateV1` contains cards, visible or
unordered piles, powers, relics, potion identities and slots, monster order,
public intent/history, encounter facts, and current resources. It contains no
RNG state, card UUID, potion UUID, or monster entity ID.

Public candidate identity is an ordinal plus typed semantics over public
indices: hand ordinal, monster order, potion slot, choice ordinal, or public
selection-domain ordinal. Simulator handles belong in a parallel private
resolution table. A model may score public candidates but cannot inspect that
table.

### `src/agent/belief`

Owns hidden-state/chance sampling and its provenance. A sampler must state what
distribution it approximates, which public history it conditions on, and
which gaps it rejects. It returns exact simulator particles only to the belief
environment/search executor; particles never become model observations.

The currently implemented `IndependentStreams` sampler is intentionally a
mechanics feasibility sampler, not a run-seed posterior. It resamples hidden
draw order and future combat RNG independently while preserving the public
boundary, potion inventory, legal potion actions, and RNG counters. It rejects
hidden current intent. A later seed/history-consistent sampler will be a new
typed origin, not a silent reinterpretation of these particles.

### Belief environment and search

The belief environment owns particle stepping, public observation aggregation,
chance refresh, and action application. Information-set MCTS/POMCP owns search
statistics keyed by canonical public history, not by one exact private state.
Every admitted action receives comparable root exploration before prior-guided
exploitation. Chance particles are shared across root-action comparisons when
possible so variance does not masquerade as action quality.

Exact witness engines may provide rollout implementations or demonstrations,
but their old frontier score, first-win stopping rule, and work units do not
define the new search contract.

### Replay, model, and learner

Replay stores decision occurrences, not a hand-selected bag of named fights.
One row binds public recurrent history, the complete public candidate set,
search visit distribution, eventual complete-run outcome, behavior/search
versions, and occurrence weight. Reanalysis may refresh policy/value targets
without rewriting the episode provenance.

The model has a shared typed recurrent/relational encoder with:

- a ragged policy head over the current public candidate set;
- a scalar complete-run win-probability head;
- optional auxiliary heads for next public draw, next visible intent, and
  combat-terminal resources, used only to improve representation.

Potion actions exist in the action schema from the first training run. Potion
opportunity cost is learned through run continuation value and later outcomes,
not a static rarity or retained-value penalty.

## Current Implemented Slice

The following is repository fact as of this design:

- `src/agent/information/combat.rs` owns the compact public observation used by
  fingerprints and public-boundary checks.
- `src/agent/information/state.rs` owns `PublicCombatStateV1`, the richer
  model-facing state. Potion UUIDs and monster entity IDs were removed.
- `LearningCombatPrivateResolutionV1` holds those exact handles beside, not
  inside, the public state.
- `src/agent/belief/combat.rs` owns the `IndependentStreams` mechanics sampler.
  The old learning-env helper delegates to it and still performs its richer
  complete-boundary check.
- the former `combat_belief` module was deleted because it read exact monster
  plans and was an unused damage profile, not a belief state.

The remaining known gaps are explicit:

1. `CombatLegalActionSurfaceV2` still carries private card UUIDs inside
   symbolic selection domains. The next information migration must produce a
   canonical public action surface plus a complete private resolution table.
2. `IndependentStreams` is not conditioned on a complete public run history.
3. There is no canonical belief environment or information-set search owner.
4. Current replay/trainer formats do not carry visit policies and recurrent
   public histories as the authoritative target.
5. No current learned behavior is promoted for complete potion-aware A0 or A20
   runs.

These are implementation gaps, not reasons to revive the retired local
teacher contract.

## Delivery Order

1. Finish public candidate identity and private resolution separation for all
   atomic and symbolic combat actions. Migrate the model caller, then delete
   the mixed surface from the learned-agent boundary.
2. Introduce a canonical belief environment and a history-conditioned sampler
   interface. Keep unsupported hidden-intent/history cases typed and visible.
3. Implement the smallest potion-aware information-set search vertical slice.
   Its output is visits and run-win value samples, never a hard witness label.
4. Replace combat-proposal replay with occurrence-weighted recurrent replay and
   reanalysis. Bootstrap only enough non-random behavior to make search useful.
5. Train and evaluate on natural complete A0 runs, including potion acquisition,
   use, discard, combat, and non-combat decisions. Promote only by paired
   complete-run outcomes.
6. Expand the same system toward A20 by curriculum distribution and capacity,
   not by adding encounter-specific teachers.

Each step must have a real downstream consumer in the next step. Small tests
protect public/private separation, exact transition semantics, and replayable
action resolution; temporary reports and prose guards do not qualify as
architecture.

## Deletion Policy

Breaking migration is the default. Once the new learned-agent boundary serves
a consumer, remove the replaced entry point, schema, helper, and documentation
in the same change. Frozen legacy artifacts may stop loading. Git history is
the archive; the active tree should describe only the system we intend to
build.
