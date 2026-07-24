# Exact Model, Policy, and Lazy Run Search Migration

Status: architecture decision for the oracle A0 mainline. Slice 1 through
Slice 4 are implemented for the shop site, and the card-reward production
cutover is implemented. Run-wide witness acceptance and retirement of the
card-reward diagnostic/calibration stack remain open.

This document replaces the assumption that the July 15 per-surface owner
migrations define the long-term run-planning architecture. Those migrations
removed duplicate execution authority, but they did not prove that
`shop_policy_v1`, `card_reward_policy_v1`, or the other site-specific scoring
stacks are suitable strategic models.

The target is a smaller and stricter separation:

1. an exact game model owns legal actions and state transitions;
2. policy and value components guide search but never change legality;
3. lazy run search owns exploration and accepts only exactly replayable
   terminal witnesses as success.

## Evidence behind the decision

### Mature game engines separate rules from search

OpenSpiel represents a game as states, legal actions, and action application.
Algorithms consume that interface rather than embedding game rules inside each
search policy:

<https://github.com/google-deepmind/open_spiel/blob/master/docs/concepts.md>

Stockfish similarly keeps position state, move generation, evaluation, and
search as distinct components. AlphaZero combines a learned policy/value model
with tree search while masking illegal moves from the rules engine:

<https://arxiv.org/abs/1712.01815>

The project does not need to copy chess search. It should copy this ownership
boundary.

### Slay the Spire does not have a context-free item value

Expert play treats cards as ways to perform jobs required by upcoming fights:
frontloaded damage, multi-target handling, defense, scaling, and access. Which
job is urgent changes during the run:

<https://sts2.untapped.gg/en/articles/slay-the-spire-deckbuilding-strategy-solving-the-spire-with-jobs>

Deck-building research reaches the same structural conclusion from another
direction: strong combinations may require rejecting policies with better
immediate reward:

<https://doi.org/10.1609/aiide.v19i1.27518>

Therefore a permanent item score, including a more elaborate typed item score,
cannot be the final source of strategic truth.

### Existing public Spire bots are useful cautions, not target designs

Bottled AI is a substantial public Slay the Spire bot with replay and testing
infrastructure. Its own documentation says its decisions are manually
constructed, card and relic choices use prioritized lists, and combat outcomes
use hand-weighted values. Its capabilities document still lists missing shop
and potion behavior:

<https://github.com/xaved88/bottled_ai/>

This is evidence that a disciplined handcrafted bot can work. It is not
evidence that another larger collection of priority lists will become the
desired planner.

### Combat is an expensive edge

Most run decisions are cheap exact transitions. Combat verification can consume
nearly the entire run budget. Planning research on expensive edge evaluation
supports selecting which edge to validate lazily rather than treating every
edge as equal-cost work:

<https://doi.org/10.1609/icaps.v26i1.13788>

This matches the already successful single-active-combat direction in the
oracle explorer.

## Current architectural fault

The current production shop path illustrates the fault:

```text
DecisionSurface
  -> shop_policy_v1 candidate facts
  -> legacy scalar estimate
  -> strategic adapter and pressure ledger
  -> evaluator admission and score
  -> compiler lane/frontier
  -> selected plan head
  -> owner ordering
  -> oracle policy rank
```

The compiler is described as whole-visit planning, but its production candidate
enumerator currently creates only single-action plans. It does not execute
successor states while comparing them. A purchase that changes prices, restocks
inventory, or opens another choice is understood only after execution and a
complete recompilation.

The generic decision pipeline also annotates the same action. The compiled shop
head can override that annotation. This is one execution owner, but it is still
multiple overlapping strategic models.

The same pattern recurs across route, reward, campfire, and shop code. Thousands
of lines classify, gate, score, adapt, compile, and re-rank candidates before
the run search receives a preferred ordering.

## Target architecture

```text
                  typed knowledge
                        |
                        v
ExactRunModel --> PolicyPrior + StateValue --> LazyRunSearch
     |                                        |
     |                                        v
     +---- exact cheap transitions       expensive combat verifier
                                              |
                                              v
                                      exact replay witness
```

### 1. ExactRunModel

The model owns:

- the exact run state;
- legal action enumeration;
- exact successor application;
- chance and oracle information boundaries;
- terminal-state classification;
- stable state fingerprints.

The existing `RunControlSession`, `DecisionSurface`, and atomic decision
transaction already implement most of this contract. The migration must wrap
and reuse them, not create another simulator or another action vocabulary.

Conceptually:

```rust
trait ExactRunModel {
    type State;
    type Action;

    fn legal_actions(&self, state: &Self::State) -> Vec<Self::Action>;
    fn successor(
        &self,
        state: &Self::State,
        action: &Self::Action,
    ) -> Result<Self::State, TransitionError>;
    fn terminal(&self, state: &Self::State) -> Option<RunTerminal>;
}
```

Every state-changing operation must be obtained by cloning a real session and
executing a public candidate transaction. A policy may not synthesize a deck,
gold total, shop restock, selection result, or combat outcome.

### 2. Typed knowledge

Knowledge extraction describes the current state and upcoming obligations. It
does not select an action.

Useful facts include:

- upcoming known boss and committed route encounters;
- encounter-pool risks visible at the current information boundary;
- deck jobs already supported;
- access and deployability of those jobs;
- liabilities such as draw burden, status burden, energy congestion, and
  duplicate low-marginal functions;
- resources and irreversible commitments.

Knowledge must be expressed as typed facts with provenance. String reasons are
rendered from those facts and are not inputs to policy.

The first implementation can reuse correct existing extractors. It must not
reuse their verdicts, hard gates, or accumulated scalar score as strategic
truth.

### 3. PolicyPrior

The policy maps one exact state and all legal actions to a normalized prior:

```rust
trait RunPolicyPrior {
    fn priors(
        &self,
        state: &RunControlSession,
        legal: &[RunDecisionAction],
    ) -> Vec<ActionPrior>;
}

struct ActionPrior {
    action: RunDecisionAction,
    probability: f64,
    evidence: Vec<TypedEvidence>,
}
```

Oracle completeness requires every legal action to receive positive
probability. A low prior delays an action; it does not delete it. This makes a
bad heuristic recoverable.

The initial prior may be handcrafted from typed job and obligation facts. It is
explicitly a replaceable guide. Exact replay trajectories can later train a
policy model without changing the game model or search API.

### 4. StateValue

State value is separate from action prior. It is not one universal integer.

The first representation is a vector:

- terminal status or exact witness evidence;
- survival/resource envelope;
- near obligation feasibility;
- deck-job support and deployability;
- unresolved uncertainty.

Search may use a calibrated projection of this vector for scheduling. The
vector remains available for audit, and `BudgetUnknown` is never converted into
loss.

Learned value can replace the initial projection later. It must not require
another state-transition system.

### 5. LazyRunSearch

The run search owns:

- the transposition graph;
- policy-guided scheduling;
- resumable work;
- discrepancy or exploration accounting;
- expensive combat-edge validation;
- replay witness retention.

Cheap noncombat actions use the exact model directly. Combat is a lazy,
expensive edge. A combat successor exists only after an exact combat witness is
validated.

The first success contract remains:

```text
seed + ascension + initial state
  -> exact action/transition journal
  -> Act 3 Boss defeated
  -> full replay reaches the same terminal state
```

No policy score, probe, or partial trajectory can substitute for that contract.

## Shop semantics under the target architecture

The shop is not a static collection of independently scored items and is not an
unordered purchase basket. It is an ordinary sequential part of the game
graph.

Examples:

- buying Membership Card changes the prices visible in the next exact state;
- buying Courier changes later inventory and prices;
- buying Orrery or Cauldron opens reward decisions;
- buying Dolly's Mirror opens a deck-card selection;
- purge consumes gold, changes the deck, and becomes unavailable;
- potion purchases depend on exact slots and may require a discard decision.

The original Java implementation performs these mutations during the purchase
itself. The Rust simulator already exposes them as subsequent decision
boundaries. The planner must observe those real boundaries.

Therefore the first shop migration uses atomic actions directly:

```text
Shop state
  -> buy Membership Card
Discounted Shop state
  -> buy Armaments
Shop state
  -> leave
```

It does not predict the discounted state inside a special membership formula
and then execute a stored tail.

If profiling later proves that repeated cheap shop nodes dominate run-search
cost, a resumable shop-closure generator may compress exact internal paths into
shop-exit macro edges. That is an optimization over the same exact graph, not a
new policy owner. It is not part of the first migration.

## What happens to Armaments

The architecture does not add a rule saying "never buy the second
Armaments."

After buying a first copy, the exact successor state has:

- one more card;
- combat upgrade access;
- a changed draw distribution;
- less gold.

After buying a second copy, the successor contains another source of block and
upgrade access, but the marginal upgrade-access job may already be supported
and the extra card may add draw burden. The policy prior can represent those
typed marginal changes. It must keep the action legal and nonzero.

The oracle search is then free to discover a line where a second copy is useful.
The strategic model no longer needs a global Armaments exception or a duplicate
card veto.

## Oracle and non-oracle information

The model API is shared; the information state differs.

For the current oracle target:

- exact seed state and future RNG are available to the search;
- exact triggered rewards and restocks can be explored;
- every accepted successor is still produced by the simulator.

For a future public-information planner:

- the same action produces a chance or belief-state transition;
- policy and value consume only public observations;
- oracle-only evidence is prohibited by the information-state type.

The oracle implementation should be direct and unapologetic. It should not
weaken itself with imagined hidden-information restrictions, but it must not
hide oracle facts inside general knowledge extractors.

## Deletion-driven migration

The cutover is incomplete until old authority is removed.

### Slice 1: exact model boundary

- Add one reusable exact-successor API around `RunControlSession`.
- Rewire oracle decision expansion through it.
- Preserve transaction journals and state fingerprints.
- No policy behavior changes.

### Slice 2: policy prior boundary

- Replace the raw `fn(&RunControlSession) -> Vec<String>` ordering callback
  with an explicit prior interface.
- Give every legal action positive probability.
- Record prior evidence separately from exact transition evidence.
- Adapt the current owner order only as a temporary named legacy prior.

Implemented on 2026-07-24:

- `RunPolicyPriorFnV1` receives the complete exact legal surface;
- every candidate must appear exactly once with finite positive probability;
- invalid, duplicate, missing, zero-support, and non-normalized priors fail
  closed;
- the former hidden rank-decay calculation is now an explicit legacy adapter;
- the candidate view borrows exact actions, so large selection actions are not
  cloned merely to obtain a prior.
- `exact_run_policy_decision_v1` executes every currently executable action
  from the same immutable parent session and returns the exact transaction,
  child session, and typed before/after strategic facts;
- state-mutating purchases are therefore observed through their real successor
  boundary rather than represented by a second policy-side effect schema.

The current `legacy_oracle_policy_prior_v1` deliberately preserves old owner
behavior and is not the target policy. Slice 3 replaces that adapter for the
shop before deleting the old compiler.

### Slice 3: shop cutover

- Implement a shop prior from typed state/action deltas.
- Execute only exact public shop actions.
- Remove `compiled_shop_rollout_step` from production.
- Delete same-shop Membership arithmetic that predicts successor legality or
  inventory instead of observing the successor.
- Keep existing shop mechanics legality in the model layer.

Implemented on 2026-07-24:

- `exact_shop_policy_decision_v1` derives the full executable shop surface and
  executes every action independently from the same immutable parent;
- `exact_shop_policy_prior_v1` orders those exact successors from typed
  acquisition, capability, liability, resource, and nested-followup evidence;
- every legal shop action retains finite positive support;
- both the production shop owner and the oracle adapter consume that one
  shared prior;
- potion purchase legality lives with exact shop mechanics rather than a
  policy-side compatibility module.

### Slice 4: shop retirement

Once no production or supported diagnostic consumer remains:

- delete `shop_policy_v1`;
- delete the shop strategic adapter whose only purpose is translating that
  schema;
- delete compatibility projections and plan-head vocabulary;
- delete tests that protect those retired representations;
- retain tests for exact mechanics, positive policy support, and replay.

Implemented on 2026-07-24:

- `shop_policy_v1`, its strategic adapter, and the owner-audit investment
  adapter were physically deleted;
- the compiled plan-head owner was replaced instead of retained as a fallback;
- obsolete Membership investment projections and their generic pipeline
  hooks were removed;
- the remaining tests protect exact successor effects, rebuilt nested
  decisions, marginal duplicate-card evidence, and complete positive policy
  support.

### Later slices

### Slice 5: card-reward production cutover

Implemented on 2026-07-24:

- `exact_card_reward_policy_decision_v1` orders every exact typed card-reward
  action from real before/after state deltas and card semantic roles;
- Battle Trance access, known-boss scaling, Pyramid status burden, duplicate
  marginality, skip, and Singing Bowl are represented as typed evidence rather
  than one legacy score;
- production and oracle share the same relative prior over the card-reward
  sub-surface, while generic reward-screen actions remain owned by the reward
  composition layer;
- every typed card-reward action remains auto-expandable with positive support;
- the former 71 KiB production owner, its lane/admission ordering, and its
  challenger smoke test were deleted rather than retained as fallback.

### Slice 6: card-reward diagnostic retirement

Implemented on 2026-07-24:

- reusable reward facts, semantic roles, profiles, and dependencies now live
  under `card_semantics_v1`;
- production reward and shop priors, deck analysis, mutation, repair, opening
  access, Pandora, coverage, and strategic compilation consume that shared
  factual boundary directly;
- the old value loop, calibration pipeline, strategic adapter, replay packet,
  and `card_reward_policy_v1` were physically deleted;
- calibration objects no longer travel inside every `RunControlSession` or
  checkpoint;
- a human card pick records the exact action result but does not fabricate a
  policy decision or teacher-like annotation;
- Singing Bowl validation remains with the exact reward boundary.

### Slice 7: campfire cutover and retirement

Implemented on 2026-07-24:

- Rest, every legal Smith target, Dig, Lift, every legal Toke target, and
  Recall now carry typed candidate keys on the exact public surface;
- `exact_campfire_policy_decision_v1` executes every action from the same
  immutable parent and orders the successors without changing legality;
- Rest distinguishes an exact zero-heal action, immediate survival, and
  non-urgent preservation instead of making any missing HP defeat every Smith;
- Smith consumes the shared upgrade-debt and deck-repair evidence; Toke
  consumes the shared exact target-loss evidence instead of a local
  curse/status/starter fallback;
- exact Dig, Lift, and Recall successors expose their real persistent state
  changes;
- the production owner returns the complete typed action surface with positive
  support, while a mechanically empty campfire remains a typed forced
  transition;
- `campfire_policy_v1`, its single-answer `Stop` contract, local score
  thresholds, and owner fallbacks were physically deleted.

Then migrate boss relic, route, and event policy through the same interface.
Each slice must delete its old production authority before the next site is
started.

## Explicit non-goals

This migration does not:

- introduce another generic score such as 340;
- declare a Pareto frontier to be a complete decision procedure;
- run combat search for every card or shop item;
- train a model before stable state/action/outcome records exist;
- require hidden-information play;
- restore unordered shop bundles;
- keep two production owners for safety;
- preserve a compatibility API without a named deletion slice.

## Acceptance

The architecture is accepted only when:

1. simulator legal actions and transitions are the sole mechanics authority;
2. all oracle-legal actions retain positive search support;
3. policy evidence cannot change legality or fabricate a successor;
4. combat successors require exact replayable witnesses;
5. buying a state-mutating shop item causes the next decision to be built from
   the resulting exact state;
6. the seed009 first and second Armaments decisions can be inspected as
   marginal successor differences, without an Armaments-specific production
   rule;
7. the shop production path no longer imports `shop_policy_v1`;
8. `shop_policy_v1` is physically deleted after the last supported consumer is
   migrated;
9. every exact Campfire action is typed, positively supported, and
   auto-expandable, with no `campfire_policy_v1` fallback;
10. seed006 through seed009 preserved witnesses replay exactly;
11. a new run can still find an A0 Act 3 Boss witness under the oracle contract.
