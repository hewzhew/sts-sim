# Deletion-Driven Oracle Turn-Option Planner

## Status

Accepted architecture direction. Slice 1 exact turn-option generation and replay
are complete. Slice 2 now has a resumable evidence agenda, terminal-witness
verification, one exact future-turn refinement, and the first typed comparator
and decision result. Atomic production cutover remains to be implemented.

This contract defines the production replacement for `combat_search_v2`. It
does not define another search profile, rescue path, diagnostic lane, or
versioned wrapper around the existing owner.

The first implementation is deliberately full-information. It may use the
exact combat position, including the realized draw order, while the project
establishes a reliable planning and evidence loop. Public-information scenario
branching remains a later extension of the same successor contract, not a
prerequisite for the Oracle cutover.

## Decision

Production combat planning will be owned by one unversioned
`crates/sts_combat_planner` crate whose semantic candidate is a **complete
turn option**. The separate crate is a compilation and ownership boundary: a
planner edit must not rebuild the core crate's monolithic unit-test binary.

A complete turn option is an executable decision program from one stable
player decision boundary to exactly one of:

- the next stable player-turn boundary;
- a terminal combat win;
- a terminal combat loss;
- a legal escape;
- a typed unsupported mechanics boundary.

It includes action order, targets, potions, and every structured selection
needed to reach that boundary. A card play, potion use, choice prefix, or raw
search node is not independently selectable merely because the simulator can
step it.

The planner compares typed prospects for complete options, progressively
refines their continuations, selects one option, executes only that option,
and observes the resulting boundary again. It does not precommit the rest of
the combat.

## Why The Current Unit Is Wrong

`combat_search_v2` globally schedules atomic exact states. A strong local
priority can therefore spend nearly all available work below one first action
before other first actions have produced comparable continuation evidence.
The observed seed006 Guardian run made this concrete: the Clothesline lineage
generated 44,450 nodes while six other materialized root actions generated one
node each.

That observation proves a comparability failure, but it does not prove that
each first action deserves an equal rollout. First actions are usually not
meaningful alternatives on their own:

- attack order can determine whether Guardian changes mode before attacking;
- Armaments changes the value of later plays in the same hand;
- a potion can open a structured target or card-selection transaction;
- zero-cost generation and draw can change the remaining action surface;
- ending the turn is meaningful only relative to the complete preceding
  sequence.

Giving every atomic root one rollout would make the old unit fairer without
making the evidence semantically comparable. The replacement changes the unit
instead of repairing that scheduler.

## Ownership Boundaries

```text
exact combat simulator
        |
        v
turn-option generator ----> complete executable options
        |                              |
        |                              v
        +--------------------> option prospects
                                       |
                                       v
                              computation agenda
                                       |
                                       v
                                 one decision
                                       |
                                       v
                       execute one option, then observe again
```

### Mechanics Kernel

The mechanics kernel owns only truth about the game:

- legal atomic inputs and structured choices;
- exact transition to a stable engine boundary;
- terminal classification;
- deterministic state identity;
- exact resource and persistent-state deltas;
- engine-step accounting.

It never ranks candidates, invents strategic reasons, or decides how much
planning work a state deserves.

Existing simulator and stepper code should be reused. Exact action binding,
pending-choice enumeration, and state-key code may be extracted from current
modules when their semantics are sound. Their current module ownership is not
preserved merely for compatibility.

### Turn-Option Generator

The generator owns legal coupled actions within one player turn. Internally it
may use exact states, prefix work, transpositions, and sound same-turn
dominance. Its public output has only two kinds of item:

- `CompleteTurnOption`: selectable and replayable through a supported boundary;
- `TurnOptionGenerationGap`: typed evidence that option generation is partial.

Partial action prefixes are private work. They are never ranked against
complete options and can never be executed as a planner decision.

Generation is monotone and resumable. A later budget grant continues retained
work without replaying already accepted exact transitions. Its status is
either complete or partial with a typed cause such as node budget, engine-step
budget, deadline, unresolved combinatorial choice, or unsupported boundary.
Partial means unknown, not bad.

Structured selection is represented as a typed transaction cursor. A
choose-X action may enumerate one decision at a time, but the planner sees a
candidate only after the complete legal input has reached a supported stable
boundary. Combinatorial size therefore consumes explicit generation work
without manufacturing millions of fake top-level candidates.

### Option Prospect

An option prospect is the consequence of:

- one exact root position;
- one complete turn option;
- one named continuation contract;
- one evaluation horizon;
- the evidence and compute actually obtained under that contract.

It is not an intrinsic card score and not one scalar combat value.

A prospect carries independent typed fields:

- exact immediate successor or terminal outcome;
- exact HP, block, potion, gold, card, relic, and persistent payoff deltas;
- exact enemy state and turn boundary reached;
- continuation evidence, if any;
- provenance for every estimate or model output;
- remaining generation or continuation gaps;
- work consumed to obtain the evidence.

Continuation evidence is one of:

- `VerifiedTerminal`: an exact replayable terminal witness;
- `ExactHorizon`: exact planning through a named number of future turn
  boundaries, followed by an unresolved boundary;
- `Estimated`: an explicitly named evaluator or learned-model result with its
  calibration identity;
- `Unavailable`: no supported continuation evidence;
- `Interrupted`: a retained computation stopped by a typed budget cause.

These states must not be collapsed into win/loss/unknown or a string reason.
An estimate cannot become an exact terminal fact without replay.

### Computation Agenda

The agenda schedules questions that can produce decision evidence. It does not
globally schedule anonymous search nodes.

The initial work kinds are:

- `DiscoverTurnOption`: advance the resumable generator until it produces one
  more complete option or a typed gap;
- `ExtendContinuation`: plan one named option through an additional player-turn
  boundary;
- `VerifyTerminalWitness`: replay a proposed terminal continuation exactly;
- `RefreshStaleEvidence`: recompute evidence whose root or continuation
  fingerprint no longer matches.

Each work item names:

- the decision and option it can affect;
- the evidence field it is expected to produce;
- its hard engine/node/deadline allowance;
- its prerequisites;
- the reason it is currently admissible.

The first production scheduler may be simple, but it must report its actual
admission rule. It may admit work because root option generation is incomplete,
a currently nondominated option lacks comparable continuation evidence, a
terminal witness needs verification, or an explicit exploration budget was
granted. It may not claim value-of-computation calibration until measured cost
and decision-change probabilities exist.

Node count, number of sampled wins, stable incumbent duration, or an empty
local queue do not by themselves earn more compute or prove that planning is
finished.

### Decision Owner

There is one production comparator and one production decision owner. The
owner receives only complete options and their prospects. It returns:

- the selected option;
- nondominated alternatives still supported by the same comparison contract;
- the typed basis of selection;
- unresolved evidence gaps;
- the exact evaluation-context and root fingerprints.

The selection basis is explicit, for example:

- verified terminal win;
- proven dominance under exact comparable evidence;
- preferred exact finite-horizon prospect;
- preferred calibrated estimate;
- budget-bounded incumbent with unresolved alternatives.

The owner never calls a rescue planner after making its decision. If required
evidence is missing, it either requests an admissible agenda item or returns a
typed planner gap. Run-control may stop or request a new external budget; it
may not silently ask `combat_search_v2`, a rollout repair, or a segment fallback
for a second verdict.

## Budget And Resumption

One `CombatPlanningBudget` owns all work for one decision:

- exact engine steps;
- option-generation transitions;
- continuation expansions;
- witness replay steps;
- one optional wall deadline.

Internal helpers receive only the remaining allowance. A nested turn generator
or continuation planner cannot multiply the budget by candidate count, future
turn count, scenario count, or rescue attempt.

Every quantum records before/after counters and retained open work. Increasing
a budget must extend prior evidence monotonically. It must not restart the
decision, replay completed option generation, or discard a previous verified
witness.

Budget exhaustion is not a combat loss and not a policy decision. It produces
`Interrupted` evidence plus a resumable session.

## Oracle First, Public Scenarios Later

The first owner receives a scenario set containing exactly the realized exact
position. This is an explicit Oracle policy, not an accidental hidden-state
leak.

The future public-information migration replaces that one exact successor with
groups of publicly indistinguishable successors. A complete turn option then
becomes a closed-loop policy tree whose continuation may differ after public
information is revealed.

The following contracts must remain unchanged during that extension:

- complete turn options are the selectable unit;
- partial prefixes are private work;
- prospects retain field-level provenance and gaps;
- one agenda owns all computation;
- one owner selects and one executor commits;
- estimates never become exact facts without verification.

The current `combat_policy_v1` contains useful experiments for public action
identity, scenario grouping, pending-choice projection, shared expansion
budgets, and turn-option composition. Those pieces are mechanism donors, not a
second production owner. Information-set scheduling and hidden-world
branching stay out of the Oracle cutover.

## Production Execution Contract

The planner executes exactly one selected complete turn option. Before the
first action it verifies the root fingerprint; after every action it verifies
the expected stable transition or returns a mechanics/integration error.

At the next player-turn boundary, run-control records the committed option and
asks the same planner owner to create a new decision. It does not execute a
whole-combat trajectory that was chosen several turns earlier.

This receding-horizon boundary lets newly drawn cards, random outcomes,
generated cards, enemy moves, potion changes, and phase transitions enter the
next decision as observed state instead of requiring a brittle precommitted
line.

## Deletion-Driven Migration

There will be no `combat_search_v3`. The target module is unversioned because
it is the final owner boundary, not another competing policy generation.

### Current Source Cut Line

The migration begins from these concrete owners:

| Current source | Current responsibility | Migration disposition |
| --- | --- | --- |
| `src/eval/run_control/combat_search.rs` | Creates the whole-combat session, selects a line, and invokes fallbacks | Replace with one `combat_planner` decision per stable player-turn boundary |
| `src/eval/run_control/combat_no_win_fallback.rs` | Calls turn-plan rescue, turn-pool rescue, Smoke Bomb rescue, and segment fallback | Delete from the production decision path at cutover; supported escape is a normal complete option |
| `src/eval/run_control/combat_line_selector.rs` | Produces a second adjudication over a whole-combat search line | Replace with the single option-prospect comparator |
| `src/eval/run_control/combat_line_executor.rs` | Replays a selected whole-combat or rescue line | Narrow or replace with one-option execution and boundary verification |
| `src/ai/combat_search_v2/search/` and `frontier/` | Own the global atomic search lifecycle | Outgoing owner; no types cross into `combat_planner` |
| `src/ai/combat_search_v2/turn_planner/` | Enumerates and bucket-ranks exact same-turn plans | Extract only sound enumeration mechanics; delete buckets, portfolios, seeding, and old ownership |
| `src/ai/combat_search_v2/rollout*` | Supplies estimate ordering, repair, and terminal promotion | May temporarily donate named estimate evidence; never becomes the new agenda or proof owner |
| `src/ai/combat_policy_v1/scenario/` | Experiments with public scenario binding and option composition | Extract sound transaction/budget mechanics later; do not import hidden-information scheduling into the Oracle slice |

The table names dependency roots, not a promise to preserve every helper below
them. Compile-visible dependency closure determines the eventual deletion.

### Slice 1: Exact Turn-Option Core

Create `crates/sts_combat_planner` with:

- exact `CombatDecisionRoot` identity;
- resumable `TurnOptionGenerator`;
- typed structured-choice transaction work;
- `CompleteTurnOption` and exact replay;
- one shared budget ledger;
- exact immediate `OptionProspect` fields.

Reuse mechanics only after moving them behind these boundaries. Do not expose
`SearchNode`, turn-plan buckets, rollout scores, or `CombatSearchV2Config` in
the new API.

This slice is temporary implementation scaffolding, not a supported parallel
product. It gains no CLI, artifact schema, laboratory mode, or run-control
fallback.

### Slice 2: Resumable Continuation And Decision

Add the computation agenda, future-turn continuation refinement, exact witness
verification, one comparator, and one decision result. Demonstrate that a
single decision and an equivalent sequence of work quanta retain the same
evidence and selected option.

Root discovery has priority over continuation refinement: the agenda first
finishes enumerating complete options at the current decision root, then gives
their continuations comparable exact horizon work. This prevents the successor
from recreating the outgoing search's failure mode where one early lineage
consumes the budget before sibling options exist.

The first comparator may use a documented finite-horizon criterion, but its
unresolved continuation remains explicit. Existing rollout or bucket scores
may be used only as named estimated evidence during migration; they may not
define core types or pruning safety.

The initial Oracle comparator is deliberately narrow:

1. any retained agenda work or typed generation/verification gap defers;
2. a shorter exact terminal-win horizon is preferred;
3. prospects with the same exact immediate successor are equivalent;
4. one complete legal option is selectable without inventing a ranking;
5. all other exact state differences remain nondominated and defer.

This gives production integration an honest decision/gap boundary without
smuggling the outgoing HP, damage, rollout, or action-count scores into the new
owner. A later evaluator must enter as named evidence with its own comparison
contract.

### Slice 3: Atomic Production Cutover

In one delivery:

1. make run-control call `combat_planner` at stable player-turn boundaries;
2. execute one selected complete option;
3. make planner gaps stop or yield with typed evidence;
4. remove all production imports of `combat_search_v2`;
5. remove no-win rescue, segment fallback, and second-verdict selection from
   the production combat path;
6. add an architecture check that prevents those imports and fallbacks from
   returning.

There is no production dual-run period. A short-lived test-only equivalence
harness may compare exact transitions before the cutover, but it is deleted in
the cutover delivery.

### Slice 4: Retire The Outgoing Search Product

After production cutover, classify the remaining `combat_search_v2` consumers.
Move only mechanics or explicitly supported diagnostic capabilities that have
real consumers. Then retire the dependency closure of:

- global atomic frontier and node priority;
- rollout schedulers, rollout repair, and terminal promotion;
- root/turn-plan seeding and macro portfolios;
- turn-pool and turn-plan rescue owners;
- whole-combat line selection/adjudication paths;
- obsolete search profiles, policy comparison flags, reports, fixtures, and
  tests whose only purpose was the outgoing owner.

Historical artifacts remain readable only where a maintained consumer needs
them. Compatibility readers cannot call old policy code.

The current `combat_policy_v1` public scenario experiment is reviewed in the
same retirement pass. Sound mechanism pieces move under the new successor
boundary; unused policy-bank, widening-schedule, and observational product
surfaces are deleted rather than retained as a second planner.

## First Vertical-Slice Acceptance

The new planner is ready for production cutover only when:

- every selected item is a complete option ending at a supported boundary;
- partial prefixes and truncated transitions are structurally unselectable;
- play order, targets, potions, and structured selections survive exact replay;
- option generation resumes without replaying completed transitions;
- a larger quantum extends prior evidence rather than replacing it;
- exact terminal wins remain exact witnesses and estimates remain estimates;
- one-shot and split-quantum planning are deterministic under the same total
  engine and generation budgets, except for explicitly recorded wall-time
  interruption points;
- unsupported mechanics, incomplete generation, and exhausted budgets produce
  typed gaps rather than low scores or losses;
- selecting and executing an option changes the exact state as its retained
  transition predicted;
- the next player turn causes a fresh decision instead of continuing a stale
  whole-combat line;
- no run-control code imports `combat_search_v2` or invokes a fallback owner;
- the old production path is deleted in the same delivery as the cutover;
- core, control, and architecture suites pass.

Passing seed006 is not an acceptance requirement. After cutover, seed006 and a
small encounter set are useful behavioral probes: they may reveal weak option
generation, continuation evidence, or comparison, but they do not define the
architecture and cannot turn a bounded miss into a failed correctness test.

## Non-Goals Of The First Cutover

- hidden-information optimal play;
- a calibrated learned value model;
- proof of global combat optimality without admissible bounds;
- exhaustive enumeration of every combinatorial turn option;
- a universal scalar combat reward;
- keeping old search reports stable for convenience;
- preserving legacy strategy reason strings, plugin stacks, or lane names;
- making one seed pass by encounter-specific rules.

## Architecture Invariants

The migration is incomplete if any of these become false:

1. The production candidate is a complete turn option, not an atomic action.
2. The production owner sees prospects, not internal generator nodes.
3. Partial generation is unknown, never losing or dominated by default.
4. Evidence kind and provenance survive comparison and serialization.
5. One budget owns nested option and continuation work.
6. One owner selects and one executor commits one option.
7. A planner gap cannot invoke an old or secondary verdict owner.
8. Oracle exact state is explicit and replaceable by scenario groups later.
9. Every production cutover deletes the superseded path.
10. No benchmark seed, win count, or elapsed-time target becomes a correctness
    definition for the planner architecture.
