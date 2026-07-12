# Decision Closure Challenger Policy Design

**Date:** 2026-07-12

## Goal

Restore a reliable non-combat decision loop without optimizing for one seed.

The system must be able to test a genuinely different construction policy that changes the deck repeatedly over time. It must not infer that HP loss means "take more defense," and it must not evaluate one changed card choice while reverting every later choice to the legacy policy.

The first delivery is an evaluation path, not an automatic replacement for the production mainline.

## Problem

The current strategy stack contains useful local facts but gives them authority through disconnected coarse verdicts:

- `DeckStrategicDeficit` converts card-role counts into `Missing`, `Thin`, `Adequate`, or `Surplus` labels;
- `heavy-burden` treats an `Adequate` label as permission to suppress further additions;
- `Probe` mixes evidence uncertainty, policy caps, and actual alternative execution;
- a one-branch owner ranks `Skip` ahead of every Probe when no mainline take exists.

This can produce a closed failure loop. One Shrug It Off plus starter Defends can make static block supply appear adequate, future mitigation is capped, the deck continues to lose HP, and the resulting combat loss is then observed only on the same stale construction path.

Pure shadow diagnostics do not solve the problem because they never create a different deck. A single changed decision also does not solve it because many cards need later supporting choices. Exhaust, Strength, block engines, generated-action decks, and upgrade plans are sequences rather than isolated picks.

## Design Principles

### Survival is a pressure contract

Do not model attack and defense as independent totals. Model whether the run can resolve an encounter before its pressure becomes unacceptable.

Pressure has several interacting axes:

- **resolution tempo**: frontload, scaling, and target removal speed;
- **delay capacity**: Block, Weak, Intangible, healing, and other ways to buy turns;
- **multi-target control**: area damage, target control, and protection against simultaneous threats;
- **growth horizon**: how quickly the enemy punishes a long fight;
- **deployability**: draw, energy, ordering, retention, and the probability that an answer can be used in time.

Fast lethal may satisfy a survival contract with little Block. Intangible may buy time without solving the fight. A card can improve multiple axes, and no single axis is named "defense score."

### Outcomes open hypotheses; they do not assign blame

HP loss, death, or a failed search proves only that some pressure remained unresolved under the observed line and search contract.

Outcome evidence may support hypotheses such as:

- resolution tempo was too slow;
- delay capacity was too thin;
- multi-target pressure remained live;
- an answer existed but was not deployable;
- search coverage was insufficient to judge the state.

No outcome directly emits "missing defense" or "missing damage." Multiple hypotheses may remain live, and missing evidence is `Unknown`, not `Adequate`.

### Evaluate policies, not isolated actions

A challenger that diverges once must keep using its own policy at every later non-combat boundary. It must not take Corruption and then use the legacy policy to reject every Exhaust support card.

The primary unit of evidence is therefore a bounded policy trajectory. Individual-decision attribution happens only after a challenger trajectory has produced meaningful evidence.

## Architecture

### Pressure assessment

Introduce a pressure assessment that joins three sources without replacing their ownership:

1. encounter or route threat facts describe burst, growth, multi-target, and control horizons;
2. deck semantic facts describe available response supply and its reliability;
3. observed combat facts describe what remained unresolved and how complete the search coverage was.

The assessment returns structured pressure hypotheses with:

- axis;
- status: `Open`, `PartiallyCovered`, `Covered`, or `Unknown`;
- confidence;
- supporting and contradicting evidence;
- the horizon over which the claim is meaningful.

Existing `DeckStrategicDeficit` remains a static inventory summary during this delivery. Its `Adequate` labels cannot by themselves close a pressure hypothesis or certify survival.

### Policy lanes

An evaluation run contains at most three non-combat policy lanes:

- one **baseline** lane using the current production policy;
- up to two **challenger** lanes using pressure-aware construction policies.

The lane limit is global for the evaluation trajectory. A challenger does not create a fresh child for every candidate. At a decision boundary it chooses one action, advances, and keeps its identity.

Two challenger lanes exist to preserve materially different solution hypotheses, for example:

- increase resolution tempo and scaling;
- buy time while improving deployability.

They are not "the two highest card scores."

### Challenger policy memory

Every challenger carries a small serializable policy state:

- active pressure hypotheses;
- strategy commitments opened by prior decisions;
- requirements still needed by each commitment;
- burden already paid for those commitments;
- evidence that supports continuing, completing, or abandoning them;
- the last exact non-combat divergence checkpoint.

A commitment is not an archetype lock. It is a hypothesis with an explicit expiry condition. If support does not appear, deployability remains poor, or the relevant encounter horizon passes, the challenger may abandon it and record the failed investment.

Examples:

- Corruption opens a commitment to evaluate skill density, Exhaust access, draw payoff, and deck-thinning value;
- Rupture opens a requirement for repeatable card self-damage rather than treating Offering as stable support;
- Apparition records finite delay coverage and asks whether the deck can convert the purchased turns into resolution.

### Continuous challenger rollout

At the first material disagreement, clone the exact in-memory run state before applying the baseline choice. Each lane inherits identical run RNG state and the same combat-search contract.

After the fork:

- the baseline continues with the production policy;
- each challenger reevaluates every later reward, shop, campfire, event, boss relic, and route boundary using its own deck and policy memory;
- challengers may diverge repeatedly;
- lane state and run state persist in the ordinary capsule/checkpoint mechanism so later slices continue instead of restarting from Neow.

This is not prefix replay. A lane starts from an exact state clone and advances normally.

### Challenger creation and deduplication

Create a challenger only when a candidate expresses a materially different response to an open pressure hypothesis and passes factual legality and acquisition safety checks.

A challenger signature contains:

- pressure axes it intends to improve;
- active strategy commitments;
- relevant package maturity;
- coarse deck burden and deployability shape.

If two challengers converge to the same signature, retain the one with stronger evidence and better run state. Do not preserve both merely because they selected different card IDs.

If two existing challengers remain semantically distinct, neither is dropped solely for having lower current HP while its declared horizon is still open.

### Bounded execution

The existing run slice wall-time and generation contracts remain the hard execution bounds. Challenger lanes are resumable across slices; they do not receive an unbounded nested search.

Evaluation checkpoints are:

- terminal win or death;
- act boss completion or failure;
- an explicit pressure horizon expiring;
- a commitment completing or expiring;
- an externally requested bounded stop.

Search deadline or coverage-limited results remain `Unknown`. They cannot prove a challenger is bad or promote it over baseline.

## Comparing Trajectories

Do not collapse the result into one universal score.

Compare paired trajectories in ordered evidence layers:

1. terminal and progression evidence: win, death, boss reached, boss completed;
2. pressure evidence: which horizons were resolved, extended, or left unknown;
3. deployability evidence: whether claimed answers were drawn and playable in time;
4. resource evidence: HP after known recovery, max HP, potions, gold, and route flexibility;
5. construction evidence: deck burden, completed commitments, abandoned investments, and open requirements.

When layers conflict without a clear dominance relation, the result is `Inconclusive`. A few extra HP cannot automatically outweigh an unresolved boss plan, and a speculative engine cannot automatically outweigh immediate death.

## Attribution After a Challenger Succeeds

A better challenger trajectory proves that a policy sequence deserves further study; it does not prove that the first divergent card was individually correct.

Record every divergence checkpoint. For a materially better challenger, perform bounded leave-one-decision-out checks from the nearest saved checkpoint:

- preserve the challenger policy after the tested decision;
- replace only that decision with the baseline action;
- continue under the same contract;
- classify the decision as `Essential`, `Supporting`, `Neutral`, `Harmful`, or `Unknown`.

Attribution is diagnostic. It does not directly update card scores or create card-ID rules.

## Relationship to Existing Boundaries

### `DeckStrategicDeficit`

It remains a static supply inventory and risk summary. It no longer has conceptual authority to certify that encounter pressure is solved. A later implementation may rename or narrow its public labels, but this delivery does not require a schema migration.

### `heavy-burden`

The production baseline remains unchanged initially. Challenger policy may override the cap only when it records:

- the open pressure being addressed;
- why the candidate changes the response contract;
- the burden added;
- the horizon and abandonment condition.

Promotion of this override into production requires a later reviewed change backed by challenger evidence.

### `Probe`

Existing Probe remains readable for compatibility but does not define challenger identity. A challenger may inspect a Probe candidate, yet it executes the candidate only through pressure evidence and policy memory.

A later specification will split candidate eligibility from execution disposition. That schema change is not required to begin policy-lane evaluation.

### Run-control and owner boundaries

The owner remains responsible for legal boundary execution. The new evaluator proposes lane-specific decisions and persists lane state; it does not take ownership of rewards, shops, routes, campfires, events, or combat.

Run-control may start, continue, inspect, and compare evaluation lanes. It must not contain strategy rules or reinterpret pressure evidence.

## Artifact Contract

Write durable evaluation evidence under `artifacts/runs`, never under a Cargo target profile.

Each evaluation records:

- common origin checkpoint and source identity;
- baseline and challenger lane identities;
- pressure hypotheses and challenger signatures;
- each divergence decision and policy-memory delta;
- execution contract and coverage status;
- comparison checkpoints;
- attribution results when requested.

Artifacts distinguish observed, counterfactual, estimated, and coverage-limited evidence. Missing fields do not imply failure or adequacy.

## Error and Uncertainty Handling

- Missing encounter context yields `Unknown` pressure, not a guessed threat profile.
- Search timeout yields coverage-limited evidence and keeps the lane resumable.
- An illegal or stale candidate invalidates that divergence without damaging the origin checkpoint.
- A commitment that passes its horizon without support expires explicitly.
- A challenger that cannot be distinguished semantically from another is merged, not arbitrarily ranked.
- Source-identity mismatch prevents silent capsule reuse.
- Conflicting trajectory layers produce `Inconclusive`, not a forced winner.

## Delivery Sequence

This architecture is delivered as three reviewed slices:

1. **Pressure and policy-state foundation**: structured pressure hypotheses, challenger signatures, serializable policy memory, and evidence-only candidate explanation.
2. **Continuous challenger execution**: exact-state fork, baseline plus at most two persistent challenger lanes, semantic deduplication, capsule continuation, and paired comparison.
3. **Attribution and promotion evidence**: divergence checkpoints, leave-one-decision-out diagnostics, and reports suitable for a later production-policy change.

Each slice must be independently testable. Slice one does not claim policy improvement. Slice two is the first point at which new deck distributions are actually observed. Slice three does not automatically promote a challenger.

## Test Design

Use a small number of structural contracts rather than seed-outcome tests:

1. HP loss opens unresolved pressure but does not label damage or defense as the cause.
2. Fast resolution can cover a survival horizon without high Block supply.
3. Apparition contributes finite delay and cannot certify permanent pressure coverage.
4. A challenger continues using its own policy after multiple sequential divergences.
5. A commitment changes later candidate interpretation and can expire without support.
6. Baseline and challengers inherit the same exact origin and RNG state.
7. Semantically equivalent challengers merge while distinct pressure hypotheses survive.
8. Coverage-limited search produces `Unknown` and preserves resumability.
9. Capsule continuation restores both run state and challenger policy memory.
10. Attribution removes one divergence from the nearest checkpoint without replaying from Neow.

Do not assert that seed `20260712002` must win, that a named card must be selected, or that one fixed HP delta proves a policy is better.

## Validation

During implementation, use focused tests for each slice. At permanent-code completion checkpoints run:

- `cargo fmt --all -- --check`;
- `cargo test --lib`;
- `cargo test --test architecture_runtime_boundaries`;
- `git diff --check`.

A bounded evaluation smoke test must demonstrate that a challenger can make at least two sequential non-combat decisions, persist them, and resume from its capsule without prefix replay. It need not win a run.

## Non-Goals

- Do not optimize directly for the investigated seed.
- Do not introduce a universal defense score or infer defense shortage from HP loss.
- Do not expand every card, shop bundle, route, or event combination.
- Do not grant challenger evidence automatic authority over production mainline.
- Do not add card-ID-specific acquisition exceptions.
- Do not make run-control own strategy.
- Do not require source-code replay or a full run restart to continue a challenger.
- Do not replace combat search with the non-combat policy evaluator.
