# CombatSearchImprovementContractV1

Status: **Not Implemented**

Qualification status: **No current witness engine is a certified teacher.**

This document defines the boundary that a combat search must satisfy before
its output may become a policy-improvement target. It does not select an
implementation engine, authorize a learner, or reinterpret existing witness
artifacts as teacher data.

## Problem

The repository currently has two useful exact witness producers:

- `AtomicExactV2`, an atomic-action fixed-root search;
- `TurnGraphPortfolioV1`, the resident combination of
  `LocalTurnGraphWitnessSession` and `PolicyDiscrepancySession`.

They can find and replay action sequences. That proves reachability under a
declared exact root and budget. It does not prove that the first action is
better than the current policy under the information and randomness available
to the deployed actor. A best witness, first win, frontier score, rollout, or
accepted production action therefore remains evidence, not a teacher target.

The missing owner is an explicit stochastic policy-improvement operator:

```text
public-information root
  + typed legal root candidates
  + current policy prior
  + explicit selection chance population
  + fair bounded root allocation
  -> proposed improved policy
  + independent evaluation chance population
  -> qualified or rejected improvement evidence
```

## Non-Goals

Version 1 does not:

- train PPO, a critic, a strategic policy, or a run-value model;
- assign hand-written HP or potion exchange rates;
- certify a complete combat solver from a single successful trajectory;
- use visits as a target merely because a search emitted them;
- preserve or auto-upgrade old combat cases, capsules, or teacher-like lab
  artifacts;
- choose `AtomicExactV2`, `TurnGraphPortfolioV1`, or another engine in advance.

## Information Boundary

### Public root

`PublicInformationCombatRootV1` must contain only facts available to the actor
at the decision boundary: the typed visible combat/run context, public action
history required to reproduce that observation, typed legal candidate
identities, and the versions of the behavior policy and public schema.

The policy-visible root must not contain or derive features from:

- the actual hidden RNG state or future draw/intent sequence;
- a chance-particle identifier;
- an exact-state hash whose distinctions expose hidden RNG;
- a transposition or dominance key that lets action selection condition on a
  hidden particle.

The simulator may retain an exact private state inside a particle to execute
transitions. Search statistics and subsequent decisions must aggregate exact
particles that share the same public information state. Particle identity is
provenance only and never model input.

### Exact roots

An exact production root remains valuable for replay, debugging, matched
experiments, and sampling chance particles. It is not passed through as the
teacher's realized future. The improvement request records the public-root
projection digest and the private source provenance separately.

## Chance Contract

Every request declares a `ChancePopulationContractV1` with:

- the public-history conditioning boundary;
- the sampler and simulator versions;
- the number of particles;
- the provenance digest of the sampled population;
- whether outcomes are paired across candidate and baseline actions;
- a role of either `selection` or `independent_evaluation`.

Selection and evaluation populations must be disjoint. Randomness used to
choose a candidate cannot also certify that candidate. This prevents the
winner's curse from turning selection noise into a teacher claim.

For a given particle, candidate and baseline continuations should use matched
environment randomness after their root action whenever the simulator can
define that coupling without changing game semantics. Both then use the same
declared continuation policy version.

## Typed Root Candidate Identity

The request owns one canonical ordered set of legal root candidates. Each
candidate has:

- a typed semantic action identity sufficient for exact application;
- a public legality/context digest;
- the current policy logit/probability;
- no display-string identity and no hidden-particle-dependent fields.

Search may internally expand many atomic actions or complete-turn plans, but
all allocation, Q estimates, uncertainty, and output probability mass project
back to this root candidate set. Missing or duplicate projection is an invalid
result, not an implicit zero value.

## Fair Selection Allocation

Version 1 uses a declared root-action fair-allocation schedule, initially
sequential halving or an equivalent sampling-without-replacement schedule.

The scheduler must record:

1. the initial candidate set and any typed pre-search exclusions;
2. a first comparable grant to every admitted root action;
3. every round's allocation, completed samples, estimate, uncertainty, and
   elimination decision;
4. unused or censored grants;
5. the total charged work per root action.

The current policy prior may regularize estimates or break a fully declared
tie. It must not create the self-confirming loop "high prior -> only visited
action -> more stable estimate -> teacher". A candidate with no comparable
sample cannot be called inferior.

The improvement budget has its own unit and accounting contract. Atomic-v2
expanded nodes and turn-graph generation work may appear as engine-private
diagnostics, but they are never silently compared or merged. An adapter must
state how its engine work realizes each improvement sample.

## Search Selection Output

The selection phase returns a `CombatSearchImprovementProposalV1` containing:

- `improved_policy`: one normalized probability for every legal root
  candidate;
- `per_action_outcomes`: completed outcome samples or sufficient typed
  statistics;
- `per_action_q_mean` and uncertainty/variance or standard error;
- per-action sample counts and allocation history;
- the selected candidate;
- the current-policy baseline candidate;
- selection population and continuation-policy provenance;
- engine-private diagnostics and optional replayable best witnesses.

The primary target is the improved policy distribution, not a one-hot best
witness. Per-action outcomes and uncertainty are first-class evidence. Visit
fractions are usable only if the scheduler contract makes visits comparable;
otherwise they remain diagnostics. Best witnesses are always replay/debug
provenance and never teacher authority.

The first implementation may use a regularized engineering target such as
`softmax((log(policy_prior) + beta * q_mean) / temperature)`, provided the
formula and all parameters are versioned. No theorem from a different search
setting is implied by that choice.

## Independent Evaluation Output

`CombatSearchImprovementEvaluationV1` evaluates the proposed action and the
current-policy baseline on fresh particles. It records:

- candidate and baseline action identities;
- independent evaluation population provenance;
- continuation policy version;
- paired typed outcomes;
- aggregate candidate/baseline estimates, paired delta, uncertainty, and
  sample count;
- censoring, mechanics gaps, and replay failures without converting them to
  losses.

Selection estimates cannot fill missing evaluation values. A best witness
cannot replace the paired outcomes.

## Teacher Qualification Gate

`CombatSearchTeacherQualificationV1` is a separate result owned by the gate,
not by the witness engine. A proposal is qualified only when all of the
following hold:

- the public-information and hidden-RNG audits pass;
- every selected root candidate has valid typed identity and comparable fair
  allocation;
- selection and evaluation particle populations are independent;
- evaluation uses the declared matched continuation policy;
- replay/mechanics gaps and budget censoring remain below declared limits;
- the fresh paired estimate supports a positive improvement over the current
  policy under the versioned statistical rule;
- the result is not dependent on one encounter family or one repeated exact
  root in the qualification collection;
- a higher-budget diagnostic subset does not systematically reverse the
  claimed direction.

The gate emits `qualified`, `rejected`, or `unknown`. `unknown` includes
insufficient samples, censored work, and unresolved mechanics. Only
`qualified` results may set `teacher_valid = true` in a future training
record. Witness-engine identity alone can never do so.

## Natural A20 Qualification Collection

The first collection samples natural combat roots from trajectories induced
by one frozen current policy on A20:

- start with 32-64 roots from at least 16-32 independent run seeds;
- cap roots per run and per combat so one episode cannot dominate;
- preserve the natural occurrence weight and source episode identity;
- do not select only elites, named encounters, wins, losses, or hand-picked
  frontier states;
- record every typed exclusion and its frequency;
- exclude forced single-action and terminal boundaries;
- for the first resource-insensitive slice, admit only roots satisfying a
  typed no-potion/potion-irrelevant contract rather than guessing HP/potion
  exchange rates.

Deck, relic, potion, encounter, ascension, HP, public RNG history, legal action
surface, and policy version remain explicit context. A bad deck is not removed
because it later loses; only predeclared root-level eligibility may exclude it.

Evaluation is distribution-weighted and block-resampled by independent run
seed, not by treating correlated decisions as independent roots.

## First Vertical Slice

The first implementation milestone is **Natural-A20 Combat Search Improvement
Qualification**. It ends before learner work.

1. Project natural exact roots to audited public-information roots.
2. Sample explicit selection particles without using the actual hidden future.
3. Give each root action a fair low budget (initial diagnostic target: 128
   declared improvement units) and propose an improved distribution/action.
4. On 16-32 fresh evaluation particles, compare the proposed action with the
   frozen policy action under matched continuation.
5. Repeat a diagnostic subset at a larger budget (initially 512 units).
6. Report disagreement frequency, fresh paired outcome delta with block
   uncertainty, censoring/gaps, encounter dependence, and budget reversal.

Interpretation is intentionally sharp:

- neither budget improves over policy: fix objective, pruning, chance handling,
  or search semantics; do not train;
- only the larger budget improves: search may be valid but budget-inefficient;
  improve scheduling/value/prior before training;
- both pass the gate: the improvement operator is eligible for a later,
  separately reviewed one-step policy distillation experiment.

That later experiment first checks held-out natural-root target prediction,
then fresh closed-loop combat outcomes. PPO, strategic training, and run-level
resource targets remain out of scope until this boundary is qualified.

## Relationship To Current Code

| Current component | Status under this contract |
| --- | --- |
| exact simulator and replay | Required foundation |
| typed public trajectory and legal candidates | Required foundation |
| `AtomicExactV2` | Witness/challenger engine; not certified |
| `LocalTurnGraphWitnessSession` | Witness/rescue/diagnostic engine; not certified |
| `PolicyDiscrepancySession` | Witness/rescue/diagnostic engine; not certified |
| `TurnGraphPortfolioV1` | Production witness portfolio; not certified |
| existing frontier/rollout/case evidence | Diagnostics/provenance only |
| current PPO/critic pipelines | Not authorized by this contract |

An engine adapter may implement this contract only after passing the
information-state, fair-allocation, and independent-evaluation audits. Existing
artifacts are not upgraded by renaming them.

## Implementation Order

1. Implement public-root projection and a hidden-RNG leakage audit.
2. Implement typed chance-population provenance and information-state
   aggregation tests.
3. Implement the engine-independent fair root allocator and proposal schema.
4. Add one engine adapter with explicit work-unit translation.
5. Implement independent matched evaluation and the three-state qualification
   gate.
6. Run the natural A20 vertical slice and publish only qualification evidence.
7. Design learner records only after the operator passes.

Until step 6 passes, maintained documentation and commands must call current
outputs witnesses, search evidence, or diagnostics—never certified teachers.
