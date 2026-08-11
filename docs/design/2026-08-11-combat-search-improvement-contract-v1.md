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

The natural-root path treats independent production seeds as the sampling unit.
`natural_combat_search_census` gives every no-potion model candidate exactly
5,000 LocalTurnGraph generation-work units and records a proposal only when
exact-win count and then winning final HP strictly exceed the frozen baseline.
The no-potion contract covers the root candidate surface and the entire
successor search; a potion-bearing A20 root exposed and fixed a previous
mismatch where Rust searched potion actions hidden from the model.

The first 8-A0/8-A20 update produced a small positive first-action result, but a
larger 16-A0/16-A20 replication localized why that result was not yet a combat
policy: only five training roots had strict proposals, and the evaluator let
search provide the complete suffix after the model's first action. Alternative
one-hot, KL-anchor, and set-valued entrance targets could change proposal
agreement but did not produce a stable independent first-action update. Those
failed development results are not teacher evidence.

`combat_search_trajectory_census` closes the more important execution gap. It
takes a strict natural-root exact-win proposal and gives its unchanged opaque
source artifact plus Rust-owned successor corpus to `learning-root
recover-search`. Rust binds the exact source identity and terminal witness,
retains a bounded terminal-nearest chain of unique opaque decision roots, and
the caller then runs a fresh equal-work search on every retained root. No
production `CombatCase` or caller-written action file is created. The witness
actions create states only; every label still comes from an independent search
comparison at that state. Five A0/A20 source wins produced 39 decision roots
and 15 strict proposals, raising useful target density from `5/32` entrance
roots to `15/39` trajectory roots.

With the original proposal-else-frozen-baseline cross-entropy target, fixed 16
epochs, and learning rate `3e-4`, an in-memory scorer was evaluated on a final
untouched 16-A0/16-A20 natural-root cohort. Unlike the entrance proxy, this
evaluation let the scorer choose every combat action and used no search suffix.
The frozen initialization won `16/64` deterministic replicates and the updated
scorer won `54/64`: A0 improved from `8/32` to `32/32`, while A20 improved from
`8/32` to `22/32`. At the exact-root level, 20 improved, 10 were equal, and two
regressed only in both-win final HP (`-6` and `-10`); no winning root became a
loss, and 19 roots changed from loss to win. This is the first positive evidence
for search-derived multi-decision combat behavior rather than a search-completed
first-action proxy. It is still a small realized-private-future feasibility
result: no model was published, `teacher_valid` remains false, and PPO is not
authorized.

The same update now has an explicit non-production persistence boundary. Its
tensor-only checkpoint and greedy behavior manifest restore with identical
logit bytes on all 39 training and 32 held-out entry rows; all 32 held-out
complete-combat greedy action traces and terminal outcomes also match the live
scorer. The artifact is marked `experimental_unqualified`, has no production
training journal, and is rejected by normal combat behavior recovery.

One further untouched natural cohort used 8 A0 and 8 A20 roots and no successor
search. The frozen source won `8/32` deterministic replicates; the reloaded
candidate won `28/32`. A0 moved from `0/16` to `14/16` and A20 from `8/16` to
`14/16`; 11 exact roots improved, four were equal, and one already-winning A20
Small Slimes root regressed by 15 final HP per replicate without losing. This
independently reproduces the survival improvement while also exposing the
remaining resource-preservation weakness. It does not qualify the search
operator or authorize PPO.

The Small Slimes regression starts at the first decision: the frozen scorer
chooses `Immolate` and wins in one turn at 74 HP, while the 16-epoch scorer puts
`Strike` ahead by only `0.058` logit and wins in six turns at 59 HP. The 39-row
training corpus contains no `Immolate` surface at all. Its frozen targets are
only 28 `Defend` and 11 `end_turn` choices; the 15 strict proposals are mostly
`Strike` or `Bash`. The update therefore learned a useful broad attack bias but
had no evidence for preserving unseen attack-vs-attack ordering.

An epoch sweep on that diagnostic cohort separated useful learning from
over-update. One optimizer step already produced the same `28/32` wins as 16
steps and preserved all four baseline all-win A20 roots exactly. Regressions
appeared only from epoch 8 onward. A second untouched 8-A0/8-A20 cohort then
compared persisted one-step and 16-step candidates. Both won `28/32` versus the
frozen source's `16/32`, but the one-step candidate had 8 improved, 8 equal,
and 0 regressed exact roots; the 16-step candidate had 8 improved, 5 equal,
and 3 regressed roots. The one-step candidate also had higher mean final HP at
both ascensions (`67.625` versus `67.25` on A0 and `46.125` versus `44.875` on
A20). Candidate generation therefore defaults to one bounded optimizer step.
This is an evidence-backed trust bound, not HP reward shaping, and still does
not qualify the search teacher.

The persisted one-step candidate was then tested beyond floor-one entry roots.
A fresh held-out A0/A20 cohort required one completed production combat, its
reward and non-combat decisions, at least 50% HP, and a distinct ordinary
encounter. Across three roots per ascension, the frozen source won `4/12`
deterministic replicates and the candidate won `12/12`. The two equal roots
were Neow's Lament auto-resolutions; on all four roots that required combat
decisions, the source lost and the candidate won. These roots included added
cards and, at A20, a Pandora's Box deck, so the result is not a floor-one deck
memorization check.

Two further held-out sentinels required three already-completed combats, so
Neow's Lament's three possible auto-resolutions were already consumed.
On A0 seed `132000099`, the source lost a floor-five Gremlin Gang in nine turns
while the candidate won in seven turns at 37 HP. On A20 seed `142000182`, both
policies beat a floor-five Blue Slaver, but the candidate finished in six turns
at 66 HP instead of thirteen turns at 39 HP. This is narrow positive evidence
that the one-step search signal repairs the source's excessive passivity and
can preserve resources in a later natural combat. It is not evidence that
attack-vs-attack ordering, potion use, deeper-run distributions, or the search
operator as a whole are qualified; `teacher_valid` remains false and PPO is
still unauthorized.

An exact-prefix audit then separated that broad attack bias from target
ordering. The later-combat policies first diverged only when the frozen source
ended its turn and the candidate continued attacking. In the retained floor-one
A20 seed `82000029` Two Louse root, however, both policies first played
`Defend` and struck monster slot 0 before choosing the same remaining `Strike`
against different targets. At root
`2a6d268c572a65a3a2fd57f483cf830df3df69c33705fd58642cced84ec0e4ae`
and exact combat hash
`d692bda8e87957e5bb6d5cda94e9d9c6f5e5dbfcf878ed7ffe77856e7a60b868`,
slot 0 had 6 HP plus 12 block while slot 1 had 12 HP, no block, and intended
to gain 4 Strength. The candidate preferred slot 1 by only `0.000056` logit.
The direct opaque recovery-root export gave end turn, striking the blocked
Louse, and striking the unblocked Louse 5,000 LocalTurnGraph generation-work
units each. All three searches completed with exact wins at respectively
`71`, `71`, and `73` final HP. This is one real resource-preserving
attack-target example and evidence that same-card/same-enemy candidates require
ordered target state; it is not yet a target-order training distribution or a
qualified teacher.
The artifact-native trajectory path replayed that same exact win into four
terminal-nearest roots without case or action intermediates. Fresh equal-work
search found two strict proposals and two baseline-retained roots; one strict
proposal was the original target-order decision. This verifies the durable
collection path, but does not turn the repeated decision into an independent
second target-order example.

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
| multi-decision search-trajectory distillation spike | Positive independent full-combat learnability evidence; optional exact unqualified candidate with reload parity, not a production publication or certified teacher |
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
