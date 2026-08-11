# CombatSearchImprovementContractV1

Status: **Partially implemented: exact finite-frame conditioning and natural-root distillation feasibility, no qualified teacher**

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

Semantic schema v9 now supplies the combat actor with the available public run
context: run goal, act, floor, keys, typed encounter, and public map. Earlier
semantic rows omitted that context even though the Rust public snapshot owned
it. Results that grouped natural roots under schema v8 therefore measured a
model-input omission, not a valid public-information chance population.

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

The first low-level feasibility primitive is
`combat_public_chance_particle_checkpoints_v1`. It preserves the complete
model-visible combat boundary and legal candidate surface, independently
resamples hidden draw order and combat-consumed RNG streams while retaining
their consumption counters, and rejects hidden current intents. Its streams
are sampled independently rather than reconstructed as a run-seed-consistent
posterior conditioned on public history. It is therefore suitable for wiring
and sensitivity experiments only, not teacher qualification.

The natural-entry feasibility primitive is
`combat_entry_floor_chance_population_v1`. It accepts visible-intent,
potion-empty room combats at turn zero, keeps the exact already-realized
upstream run and all seven persistent RNG streams, re-seeds only the five
floor-local streams, rebuilds the encounter through the production combat-start
constructor, and retains only complete public-decision matches. This gives a
seed-consistent sample of alternative floor-local combat-entry randomness
conditioned on one exact upstream run. It does not regenerate alternative
upstream histories and its candidate floor-seed bases are not complete run
seeds. It is therefore more physically faithful than independent stream
re-seeding but still not the public-history run-seed posterior required for
teacher qualification.

This strict floor-seed rejection path is the reference implementation for
checking exact reconstruction and measuring conditional sparsity. It is not
assumed to scale: a multi-monster public entry may admit no alternative in a
large bounded interval even when the source seed reconstructs exactly. Such a
scan is `unknown`, not a loss or permission to fall back silently to the
realized future. An accepted-seed cache may save repeated scanning, but cannot
replace the missing public-history posterior.

`combat_public_history_chance` is the exact finite-frame run-seed reference.
It starts every candidate from a fresh production run using one complete run
seed, compares every captured public decision snapshot, replays the same typed
public candidate identity only after an exact match, and accepts only the same
combat-entry snapshot. It neither fixes the source run state nor independently
re-seeds any stream. Its receipt names the seed-only partition, raw candidate
range, complete scanned frame, accepted run seeds, deterministic retained
sample, and whether the source seed reconstructed. The result is exact for that
declared finite prior frame; it is not a claim about all `2^64` seeds.

On an A0 Jaw Worm root and an A20 Two Louse root, independent 2,048-seed
training-partition frames each retained exactly one candidate: the original run
seed. The retained opaque root digest was identical to the source root digest,
and every other seed diverged at the first public decision snapshot before the
combat-entry comparison. Thus full public map/history conditioning is already
near-degenerate at these early roots. A multi-particle same-public-root teacher
cannot be obtained by scanning harder or by falling back to floor-local seed
repair.

The learning caller can also compose sanitized decision snapshots and their
selected public candidate ids into a decision-boundary prefix identity. This
contains neither run seed nor private simulator handles, but it is not yet a
complete transcript of non-decision public events. A bounded natural-entry
census found every such prefix unique in 2,048 A0 roots and independently in
2,048 A20 roots. After schema v9 admitted the missing combat run context, the
model information rows were also 2,048/2,048 unique in both censuses. Natural
seed collisions are therefore rejected as a practical particle sampler; a
repeated coarse model row must not be relabeled as public-history conditioning.

The first 2026-08-12 natural-entry spike exported opaque roots directly from that
census in declared seed-partition order, with no encounter or outcome filter.
On two A0 Jaw Worm roots, four selection and four disjoint evaluation particles
plus 5,000 equal generation-work units per action selected Bash over the frozen
Defend and reproduced `+7` and `+1` mean winning-HP deltas on evaluation.
Three other completed roots produced no strict improvement and therefore no
proposal. The strict rejection sampler also accepted zero alternative public
matches for one A20 Two Louse root after 1,000,000 floor-seed candidates.
These are feasibility and sparsity measurements with `teacher_valid = false`;
by themselves they neither qualify a teacher nor justify a policy update.

The subsequent natural-root path treats independent production seeds as the
sampling unit instead. `natural_combat_search_census` gives every no-potion
model candidate exactly 5,000 LocalTurnGraph generation-work units and records
a proposal only when exact-win count and then winning final HP strictly exceed
the frozen baseline. The no-potion contract covers the root candidate surface
and the entire successor search; a potion-bearing A20 root exposed and fixed a
previous mismatch where Rust searched potion actions hidden from the model.

Across 8 natural A0 plus 8 natural A20 training roots, search produced five
strict proposals and no budget-unknown result. A fixed 16-epoch, `3e-4`
non-publishing distillation used the proposal action where strict improvement
existed and otherwise anchored the frozen baseline. On a fresh disjoint
8-A0/8-A20 held-out collection, the updated in-memory scorer selected two
strictly better actions, fourteen equal actions, zero worse actions, and zero
unknown actions under the same equal-work search evidence. The two improvements
were `+2` and `+14` winning HP; mean held-out HP delta over all sixteen roots was
`+1.0`, and mean best-search HP regret fell from `1.9375` to `0.9375`.
This is the first positive evidence that cross-root search proposals form a
learnable signal. It remains a small realized-private-future feasibility result:
no model was published, `teacher_valid` remains false, and PPO is not
authorized.

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

## Development Staging

A0 may be used as a low-noise mechanism-development domain before the A20
qualification collection. The useful claim there is only that public-chance
sampling, fair root-action comparison, frozen continuation, and fresh paired
evaluation work and can expose stable action differences. It is not an A0 PPO
benchmark, a complete-run competence claim, or permission to train on noisy
single-root proposals. Once that mechanism is stable, qualification transfers
directly to natural early-act A20 roots; it need not climb every ascension.

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
| independent-stream public-chance sampler | Feasibility primitive; public-equivalent but not a run-seed-consistent posterior |
| conditioned combat-entry floor-chance sampler | Feasibility primitive; production combat start with exact upstream state fixed, not a posterior over complete run histories |
| finite-frame public-history run-seed scan | Exact for its declared seed frame; complete production replay; observed early-root posterior was source-only |
| natural combat-entry information census/export | Seed-partition-ordered opaque bootstrap roots plus sparsity/model-input diagnostics; not a chance sampler or teacher evidence |
| natural-root equal-work search/distillation spike | Positive cross-root learnability evidence; realized private futures, non-publishing, not a certified teacher |
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
