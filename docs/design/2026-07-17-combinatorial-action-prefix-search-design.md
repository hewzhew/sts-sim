# Combinatorial action-prefix search

Status: implemented vertical slice, with explicit completeness boundaries.

## Problem

Some combat decisions are one atomic game action but have a combinatorial input
surface. Examples include choosing a subset of cards from hand, draw/discard,
or a Scry window. Materializing every complete input before search can spend the
entire time and memory budget before the simulator evaluates even one action.

`Win / Loss / Unknown` does not solve this problem. It describes the outcome of
work already attempted; it does not provide a representation for the enormous
set of work that has not yet been generated.

The search representation therefore follows operator decomposition: a large
atomic action is represented by search-only partial assignments. This is the
same broad pattern used by Operator Decomposition and partial-expansion search:

```text
concrete engine state
  -> search-only action prefix
  -> search-only action prefix
  -> complete ClientInput
  -> one atomic simulator transition
  -> next concrete engine state
```

References:

- Trevor Standley, [Finding Optimal Solutions to Cooperative Pathfinding Problems](https://ojs.aaai.org/index.php/AAAI/article/view/7564), AAAI 2010.
- Felner et al., [Partial-Expansion A* with Selective Node Generation](https://ai.dmi.unibas.ch/research/reading_group/felner-et-al-aaai2012.pdf), AAAI 2012.

## Correctness contract

1. A prefix is not an engine state and is never passed to the simulator.
2. A prefix does not consume a combat action, produce reward, change RNG, or
   appear in a replay trace.
3. Only a complete prefix is compiled to one `ClientInput`; the existing atomic
   engine transition applies it exactly once.
4. Candidate identity and order are frozen when the pending-choice transaction
   starts. Prefix work cannot observe a later mutable pile.
5. An unfinished residual remains in the frontier. A budget may postpone it,
   but heuristic ordering must not silently delete it.
6. Prefixes do not enter concrete-state transposition tables, dominance tables,
   rollout caches, or action-length accounting. Those keys describe real engine
   states and would incorrectly merge all prefixes.
7. Legality membership is separate from candidate enumeration. A complete
   action can be legal even when it was outside a legacy bounded candidate list.
8. `Cancel` is an atomic leaf scheduled first inside the same transaction
   residual as the subset tree. A shallow prefix budget cannot starve it, and
   the frontier does not clone the concrete parent merely to schedule it.
9. A timed-out or engine-step-limited partial transition is not a concrete
   child. It cannot acquire an action trace or enter the state frontier.

## Implemented action families

| Pending choice | Search representation | Current coverage |
| --- | --- | --- |
| `HandSelect` | include/exclude over frozen card UUIDs | every canonical subset satisfying `min..=max`, plus a first-scheduled `Cancel` leaf when allowed |
| `GridSelect` | include/exclude over frozen card UUIDs | every canonical subset satisfying `min..=max`, plus a first-scheduled `Cancel` leaf when allowed |
| `ScrySelect` | include/exclude over frozen candidate indices | every canonical discard subset |
| discovery, card reward, foreign influence, choose-one, stance | existing atomic enumeration | linear, small discrete surfaces; no combinatorial prefix needed |

Feasibility propagation forces the remaining suffix when it is already
determined by the lower or upper cardinality bound. Candidate storage is shared
between prefixes, so branching does not copy the entire UUID domain at every
depth.

The frontier uses an EPEA-style residual: one transaction work item yields at most one
complete action, then requeues its exact remaining tree. Real child states can
therefore compete with ungenerated sibling actions instead of waiting for the
whole power set to be enumerated.

Reports keep these units separate: `remaining_states` counts unique concrete
engine states, while `pending_choice_work_items` counts virtual residual work.
Rejected complete prefixes are counted as action-surface diagnostics; if an
entire transaction has no legal input, its concrete parent is retained once.

## Potion boundary

Potion actions are not one homogeneous combinatorial family:

- `UsePotion { slot, target }` is a small atomic action surface (one action per
  legal target). If using it opens a Hand/Grid/Scry choice, that later stable
  boundary is handled by the action-prefix layer.
- `DiscardPotion(slot)` is also atomic and permanently reduces the state. It is
  usually unattractive, but is not globally dominated: freeing a slot before
  `Alchemize`, `Entropic Brew`, or another potion-generation effect can change
  the outcome. Discarding Fairy Potion can also deliberately expose an unused
  Lizard Tail, because the death handler consumes Fairy before trying the relic.
- The default/semantic potion policies already omit discard actions; the
  explicit `All` policy retains them for an all-legal oracle surface. No global
  legality rule may remove them merely because most uses are bad.

A future proposal policy may normally put discard actions to sleep and wake
them for a concrete slot-pressure continuation or a revive-priority override.
That is a scheduling optimization, not a legality claim. Multiple independent
discard actions may eventually use partial-order reduction to keep one canonical
slot order, but every discard remains a real replayed action.

## Explicit completeness boundary

The implemented family enumerates subsets in frozen candidate order. It does
not enumerate every permutation of the same selected cards. This preserves the
old generator's canonical surface and removes its hidden pool/result caps, but
it must not be reported as the set of every ordered input the engine happens to
accept.

Order can be observable for effects such as moving multiple cards to the top of
a pile. Potion-created choices supply concrete counterexamples: Gambler's Brew
and Elixir process selected cards in submission order, and Sacred Bark Liquid
Memories can choose two cards when only one hand slot remains. If ordered choice
becomes decision-critical, extend the family in two stages:

```text
choose semantic members -> order only the order-sensitive members -> submit
```

Do not blindly generate all permutations for every choice reason. First classify
the resolution reason and prove when order is irrelevant, canonical, partially
ordered, or fully ordered.

Until that second stage exists, visiting every canonical member set sets
`ActionSurfaceIncomplete` and can never produce `exhaustive = true`.

## Remaining legacy consumers

Production combat search, complete-line fallback, turn-pool rescue, and replay
membership use the new lazy/membership boundary. Fingerprint and combat capture
now use `StateFingerprintV2` / `CombatCaptureV2` and a linear-size typed legal
action language from `sim::combat_action_surface`; they no longer call the
legacy eager generator. The unused raw action-list renderer was deleted rather
than preserved as another powerset entry point.

The V2 legal language and search coverage are deliberately separate:

- the simulator surface describes every ordered payload accepted by membership
  using `OrderedDistinctSequence` plus typed uniqueness and availability facts;
- `legal_input_language_hash` canonicalizes that membership language, while
  `action_enumeration_domain_hash` separately records ordered semantic atomic
  addresses and the frozen selection domain used to construct work; card
  identity and upgrades may change that domain hash but are not smuggled into
  the legal-language hash;
- the current search still schedules only canonical member sets and reports
  `ActionSurfaceIncomplete` when submitted order permutations remain uncovered.

Search coverage remains a property of a concrete search run, not of a capture:
the same complete legal language may be explored by different schedules and
budgets. Captures therefore retain the legal-language and frozen-domain facts,
while search reports own the `Complete` versus `ActionSurfaceIncomplete` claim.

V1 action hashes are not migrated as authoritative evidence. Scry V1 could be
exponential, Hand/Grid V1 was capped and incomplete, and V1 omitted legal
`Cancel` and ordered payloads. Production writers and loaders therefore accept
V2 captures only; an offline V1 migration, if ever required, must discard the
derived V1 action cache and rebuild V2 from the authoritative exact position.

The engine and production combat-search path no longer expose an eager
`get_legal_moves` API. One deliberately bounded exception remains in the V1
scenario exact-world policy: `scenario::pending_choice::enumerate` still
materializes exact Hand/Grid/Scry candidates up to
`MAX_PENDING_CHOICE_EXACT_ACTIONS = 4_096`. Larger domains fail closed with the
typed `CandidateSpaceTooLarge` gap instead of being truncated or silently
treated as complete. This scenario-only exception must be migrated separately;
it is not evidence that the engine or production search still owns an eager
action list.

## Required evidence

- A large Scry state can spend a small prefix budget without any simulator
  transition and retains residual frontier work.
- A small Scry state submits exactly one simulator transition per complete
  subset.
- Hand/Grid choices outside the legacy candidate cap pass structured legality
  membership and can be replayed.
- `Cancel` is scheduled before the subset residual without cloning the parent.
- Timeout never turns a zero-step or partial transition into a traced child;
  the exact complete leaf remains in residual work.
- An infeasible or wholly rejected transaction retains one concrete unresolved
  parent rather than one pseudo-leaf per rejected prefix.
- Custom `CombatStepper` implementations do not inherit engine-specific
  factorization unless they explicitly opt in.
