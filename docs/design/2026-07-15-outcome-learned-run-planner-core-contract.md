# Outcome-Learned Run Planner Core Contract

## Status

Typed core implemented; live capture integration remains provisional. The
first bounded calibration showed that command-level capture misses decisions
performed inside aggregate run-control automation and that the current
coverage denominator counts only finalized captures. Gate 1 is therefore not
accepted yet. The required atomic execution migration and physical deletion of
live auto-run are defined in
`docs/design/2026-07-15-atomic-run-decision-execution-design.md`.

These contracts do not change production decisions and do not authorize a
model or a new planner to control a run. Gates 2 through 4 remain future work
and require faithful evidence from atomic decision traces before an owner is
selected.

The implemented capture boundary lives in `src/ai/planner_core` and
`src/eval/run_control/planner_capture.rs`. SessionTrace schema v15 stores its
observations, legal candidate sets, and outcome attachments as
content-addressed payloads; behavior events contain references rather than
duplicating those payloads. The legacy policy is recorded explicitly as a
behavior policy, never as teacher authority.

This document supersedes the assumption in the deletion-driven Campfire
prospect design that Campfire should be the first production boundary or that a
Campfire-specific prospect vocabulary should become the seed of a global
planner. Existing work on engine-owned legality, stable candidate identity,
exact deterministic projection, and hidden-information safety remains valid.

## Decision

The next planning architecture will be built around four contracts:

```text
public observation
  -> complete legal candidate set
  -> exact or public-chance successor kernel
  -> learned outcome distribution + bounded planning
```

The core does not contain `pressure`, `prospect`, deck archetype verdicts,
scene-specific score terms, or a catalogue of strategic reasons. Those may
remain diagnostics outside the core while they are useful, but they are not
inputs that the new planner must preserve and are not labels for learning.

The current policy may generate behavior data. It is never the teacher merely
because it currently owns production.

## Problem

The current project has accumulated many useful local fixes, but they have the
same structural weakness:

- a local symptom becomes a named strategic fact;
- the fact gains thresholds and score terms;
- several scene owners consume it differently;
- tests protect the temporary interpretation;
- a later migration has to preserve the vocabulary before it can replace the
  decision.

This can make the code explain more while learning no more. It also makes a
single successful or failed seed carry too much architectural weight.

Three evidence errors are especially dangerous:

1. A behavior policy action is treated as a correct action label.
2. A bounded search miss is treated as an unwinnable outcome.
3. A realized future from the live RNG is treated as information available at
   the earlier decision.

The new core exists primarily to make those errors unrepresentable.

## External Design Evidence

The contract is informed by public Slay the Spire projects, but does not copy
their implementations:

- Bottled AI demonstrates that manual priorities plus combat graph traversal
  can work, while also showing the continuing maintenance cost of many local
  comparators and capability exceptions:
  <https://github.com/xaved88/bottled_ai>.
- Slay-I predicts fight HP loss from human runs and compares deck mutations,
  but its published average error and player-data dependence show why a fight
  predictor is a useful sensor rather than a whole-run authority:
  <https://medium.com/data-science/bringing-deep-neural-networks-to-slay-the-spire-a2971d5a5115>.
- Miles Oram's STS agent separates combat control, fight-outcome prediction,
  and macro state value. Its shop cost and single expected-floor target show
  both the value and the limits of successor-state evaluation:
  <https://github.com/MilesOram/STS>.
- `sts-rl-agent` scores variable non-combat candidates with a shared network
  while retaining search for combat. Its failed combat distillation documents
  the danger of training from a search teacher that can see future draw order:
  <https://huggingface.co/Jialeiv/sts-rl-agent>.
- Floor-level win prediction from human runs shows that observational run
  strength becomes much easier to separate late in a run than early in one:
  <https://github.com/JoeyRussoniello/sts-win-prediction>.

The general research lesson is also narrow. Set models are appropriate for
unordered owned objects, distributional value models retain outcome variation,
and policy/value estimates can guide rather than replace bounded search:

- <https://proceedings.mlr.press/v97/lee19d.html>
- <https://arxiv.org/abs/1707.06887>
- <https://www.nature.com/articles/s41586-020-03051-4>

These sources motivate the interface shape. They do not justify implementing a
large transformer, MuZero, or an end-to-end reinforcement-learning system as
the first slice.

## Goals

The core must:

- represent the information available to a real player at a decision;
- represent every legal candidate with stable typed identity;
- distinguish deterministic, random, and random-then-decide transitions;
- learn or estimate distributions of future outcomes rather than a universal
  hand-authored strategic score;
- let a policy prior allocate search without owning legality;
- record behavior, search evidence, counterfactual experiments, and realized
  outcomes with different label types;
- keep the continuation policy and mechanics version in every outcome claim;
- support one-owner cutover with deletion instead of permanent fallback;
- remain small enough to train and evaluate on the local machine.

## Non-Goals

The first contract does not:

- train a model;
- choose Campfire as the first migrated owner;
- solve route, reward, shop, event, relic, and combat jointly;
- require full-run tree search;
- treat human run history as causal action-value data;
- use the current policy's score, tags, reasons, or pressure facts as truth;
- encode a preferred action for seed006 or any other named seed;
- replace engine mechanics with learned transitions;
- preserve an old owner as a semantic fallback after cutover;
- require one particular ML library or network architecture.

## Core Invariants

1. The engine owns legality and mechanics.
2. The planner observes only public information, even if the in-memory
   `RunState` also contains hidden queues or RNG state.
3. Every legal candidate is representable before ordering or budget allocation.
4. Candidate labels and display text are never parsed for control flow.
5. The current owner is recorded as a behavior policy, not a teacher.
6. Realized behavior outcomes, simulated counterfactuals, search estimates,
   and human historical observations are different label kinds.
7. Actual outcome variation and uncertainty about an estimate are different
   fields.
8. A policy prior may order and allocate; it may not make an action illegal.
9. No candidate is evaluated by peeking at the live future RNG cursor.
10. Every value claim names the mechanics, observation schema, objective,
    horizon, and continuation-policy manifest under which it was produced.
11. A production cutover deletes the replaced semantic owner in the same
    delivery. An unsupported planner result becomes a typed gap, not a call to
    the old policy.
12. The first migrated owner is selected by evidence after the core exists. No
    scene is privileged by this contract.

## Public Observation Contract

The planner consumes a sanitized `PlannerObservation`, not raw `RunState`.
Conceptually:

```text
PlannerObservation:
  observation_id
  schema_id
  mechanics_id
  information_cutoff
  run_goal
  decision_site
  run_scalars
  owned_cards[]
  owned_relics[]
  owned_potions[]
  public_map
  public_encounter_context
  public_history
```

Owned cards, relics, and potions are typed entities. Their stable mechanics
identity, upgrade level, counters, and publicly visible state may be encoded.
Their array position must not accidentally become strategic meaning. An
implementation must use canonical aggregation or a permutation-invariant
encoder and must test that harmless reordering does not change inference.

`run_scalars` contains raw public facts such as HP, max HP, gold, act, floor,
keys, ascension, and visible counters. `public_map` contains only revealed
nodes, edges, room information, and boss identity available to the player.

The observation contract deliberately excludes:

- live RNG streams or cursors;
- realized future encounter queues;
- future card, relic, event, or potion rolls;
- old policy scores and action ranks;
- `pressure`, `prospect`, admission, deficit, package, archetype, or repair
  verdicts produced by current strategy modules;
- a prior owner's selected action.

Stable domain facts derived directly from mechanics are allowed. Strategic
features should be learned from those facts or added later only as documented
auxiliary observations with ablation evidence. Adding a field because a
current heuristic already uses it is not sufficient evidence.

## Candidate Contract

Every decision site exposes a complete `LegalCandidateSet`:

```text
LegalCandidateSet:
  decision_id
  observation_id
  site
  candidates[]

LegalCandidate:
  candidate_id
  action
  target_ids[]
  mechanics_fingerprint
  public_descriptor
```

`action` is a typed tagged union. Examples include:

- `TakeCard { card_offer_id }` and `SkipReward`;
- `Rest`, `Smith { card_uuid }`, `Toke { card_uuid }`, `Dig`, `Lift`, and
  `Recall`;
- `Buy { shop_item_id }`, `Remove { card_uuid }`, and `LeaveShop`;
- `ChooseRouteEdge { edge_id }`;
- `ChooseEventOption { option_id }`;
- `TakeBossRelic { offer_id }` and `SkipBossRelic` when legal.

Smith and Toke produce one candidate per legal target. A large candidate set is
not collapsed into a family score before the shared evaluator can see it.

Shop combinations are represented sequentially. Buying one item produces a
new shop state where buying another item or leaving becomes a new decision.
The core does not enumerate every purchase bundle up front.

`public_descriptor` contains typed mechanics needed to encode the candidate,
not a strategic verdict. A rendered card or relic name is display-only.

Candidate generation may adapt existing engine-owned legal boundaries. It must
not call a current policy owner to decide which legal options are worth
showing.

## Successor Kernel Contract

A candidate is evaluated through one of four typed transition forms:

```text
SuccessorKernel:
  Deterministic { successor }
  PublicChance { distribution_id, support_or_sampler }
  ChanceThenDecision { distribution_id, revealed_site }
  Unsupported { gap }
```

### Deterministic

The exact engine transition is applied to a scratch state without mutating the
live run. The resulting public observation is rebuilt through the sanitized
observation boundary. Smith, Toke, normal Rest healing, Lift, Recall, and a
specific purchase are typical deterministic prefixes.

### Public chance

The outcome is drawn from publicly eligible possibilities using an analysis
RNG independent of the live run. Dig is a typical example. The kernel records
the distribution definition, exclusions visible to the player, sampling
schedule, and mechanics version.

### Chance then decision

Random revelation and the later choice remain separate nodes. Dream Catcher
heals exactly, reveals a public random reward screen, and only then invokes the
card-reward decision policy. It is not reduced to a random scalar benefit.

### Unsupported

Failure to model a suffix is explicit. It does not become zero value, rejection,
or permission to run an old semantic owner.

The successor kernel models game mechanics and public chance. It contains no
preference for the candidate.

## Outcome Distribution Contract

The learned evaluator predicts a distribution over `RunOutcome`, not a
`pressure_score` or action-specific utility:

```text
RunOutcome:
  goal_reached
  terminal_act
  terminal_floor
  terminal_kind
  terminal_hp
  persistent_resources

OutcomeDistribution:
  outcome_schema_id
  horizon
  continuation_policy_manifest
  goal_success_probability
  terminal_progress_distribution
  terminal_hp_distribution
  resource_use_distribution
  aleatoric_summary
  epistemic_summary
  applicability
  provenance
```

`goal_success_probability` refers to the declared `run_goal`, such as ordinary
Act 3 victory or Heart victory. `terminal_progress_distribution` is an
auxiliary learning and diagnostic target; it must not silently replace winning
as the production objective. HP and resource distributions provide useful
credit and risk information without declaring that HP, damage, defense, or
potions have one context-free exchange rate.

`terminal_kind`, `applicability`, provenance, and every planner gap are typed
enums or tagged records. They are not free-form reason strings used later for
control flow.

The first useful implementation may predict quantiles or a compact categorical
distribution. It need not generate raw full-run samples. Whatever form is
chosen must retain lower-tail behavior and be calibratable on held-out data.

`aleatoric_summary` describes variation in possible game outcomes.
`epistemic_summary` describes finite data, model disagreement, search coverage,
or distribution shift. These values may not share one numeric field.

The production objective is a small, repository-wide `ObjectiveProfile` that
maps terminal game outcomes to preference. It is versioned independently from
the model and is shared across non-combat sites. It may express the declared
run goal and a risk attitude over terminal outcomes. It may not contain card
names, encounter names, scene-specific thresholds, or old strategy tags.

If the evaluator is not calibrated well enough to support that objective, it
is not promoted. A handcrafted fallback objective is not added to compensate
for an unready model.

## Policy Prior Contract

The policy prior scores a pair rather than owning a fixed action vector:

```text
PolicyPrior(observation, candidate) -> prior
```

This supports variable reward screens, arbitrary Smith and Toke targets,
shops, events, and route edges through one interface. Candidate-specific
encoders may exist, but the shared state representation and outcome objective
remain common.

The prior may be initialized from behavior data, but imitation quality is not
the promotion target. It is used to:

- order candidates;
- allocate a bounded search budget;
- provide a deterministic tie preference when outcome evidence is genuinely
  indistinguishable and the prior is in-domain.

It may not:

- remove a legal candidate;
- turn an unvisited candidate into a proven bad candidate;
- claim that the incumbent action was correct;
- inspect hidden future information available only to a search teacher.

## Bounded Planner Contract

The planner combines legal successors, public chance, a policy prior, and an
outcome distribution:

```text
PlannerRequest:
  observation
  legal_candidates
  objective_profile
  search_profile
  model_manifest

PlannerDecision:
  status
  selected_candidate
  candidate_evidence[]
  search_coverage
  model_applicability
  decision_gap
  provenance
```

The first planner may be shallow. It should expand exact afterstates and a
bounded number of chance or later-decision nodes, then use the outcome model at
the leaves. A perfect full-run rollout is not required.

Equivalent public states may share a transposition entry. States that differ
only because of hidden simulator identity must not be treated as different
information sets.

Search profiles own wall time, node limits, chance-sample limits, and widening.
They do not contain scene strategy. A profile may spend more compute on a
high-impact site, but it uses the same objective and value contract.

Decision status is typed:

```text
Selected
EvidenceGap
UnsupportedObservation
BudgetExhaustedWithoutDecision
InternalError
```

The runner advances only on `Selected`. Before cutover, every other status is
shadow evidence. After cutover, it is a planner gap and stops that automation
path; it does not call the deleted owner.

## Trajectory And Label Contract

Learning data is append-only evidence linked by stable ids. The maintained
campaign-artifact contract keeps one authority for decision history. In the
current implementation, that authority is carried by `SessionTraceV1` steps,
boundary annotations, and outcome attachments; it is not a separate generic
`journal.jsonl` file. This contract extends that typed decision history and
does not create a second learning journal. A behavior decision event contains
at least:

```text
BehaviorDecisionRecord:
  trajectory_id
  run_id
  seed_group_id
  decision_id
  observation_ref
  legal_candidate_set_ref
  selected_candidate_id
  behavior_policy_manifest
  selection_probability
  continuation_policy_manifest
  mechanics_id
  timestamp_or_sequence
```

The referenced normalized observation and candidate payloads may be inline when
small or content-addressed when large. The authoritative decision history owns
their ids and hashes. Dataset files and analysis reports are rebuildable
projections from typed decision events, referenced immutable payloads,
capsules, and outcome attachments; they are never a second evidence authority.
If storage later moves from `SessionTraceV1` to an append-only journal, that is
an authority migration with compatibility reading, not permanent dual-write.

`selection_probability` is recorded as known stochastic, known deterministic,
or unknown. It is never fabricated. Deterministic incumbent behavior provides
no off-policy support for alternatives merely because those alternatives were
listed.

Later records attach transitions and outcomes:

```text
TransitionRecord:
  decision_id
  selected_candidate_id
  realized_public_successor
  transition_kind
  live_rng_not_exposed

OutcomeAttachment:
  trajectory_id
  label_kind
  horizon
  outcome
  continuation_policy_manifest
  search_or_sampling_coverage
  provenance
```

`label_kind` is one of:

- `RealizedBehaviorRun`;
- `ExactScenarioReplay`;
- `SampledCounterfactualScenario`;
- `BoundedSearchEstimate`;
- `HumanHistoricalObservation`.

These label kinds are not interchangeable. In particular:

- only the selected behavior action receives the realized run outcome;
- an exact replay is exact for its scenario, not for the public outcome
  distribution;
- a search miss is coverage evidence, not a loss label;
- human data describes outcomes under human behavior and selection bias;
- counterfactual siblings stay grouped with their root in dataset splitting.

Every outcome records the combat and non-combat continuation policies used
after the decision. When combat search changes, old outcomes do not silently
become labels for the new combat policy. They are conditioned on their manifest,
reweighted under a defensible method, or excluded.

## Learning Loop

The intended loop is policy evaluation and improvement, not one-shot imitation:

```text
incumbent behavior produces visited states
  -> hidden-free trajectories and typed outcomes
  -> outcome model + auxiliary heads
  -> bounded shadow planner
  -> new trajectories from planned and exploratory behavior
  -> held-out calibration and policy evaluation
  -> one-owner cutover when earned
```

The incumbent is useful because it reaches real states. Its selected actions
are not correctness targets. Imitation may warm-start a prior, but later
improvement must be supported by outcome evidence or planner targets that obey
the same information boundary.

Useful auxiliary heads may include fight survival, HP-loss quantiles, resource
use, terminal floor, and encounter-conditioned outcomes. They help ground the
representation and diagnose failures. They do not become independent policy
owners and are not manually summed into a new strategic score.

Training and evaluation partitions are assigned by seed and root state before
counterfactual children are generated. Siblings, reruns, and derived samples
from one root remain in the same partition.

## Relationship To Existing Code

The new core may reuse:

- domain and engine facts;
- engine-owned legal candidate enumeration;
- stable card, relic, offer, route, and event identities;
- exact deterministic scratch transitions;
- public chance-pool definitions;
- journals, capsules, fingerprints, and exact replay infrastructure.

It must not import as strategic inputs:

- current `*_policy_v1` decisions or scores;
- `strategy`, `strategic`, or `noncombat_strategy_v1` verdicts;
- pressure assessments, candidate responses, package labels, debt scores, or
  admission reasons;
- Campfire prospect comparison or threat-panel summaries;
- branch-retention scores or human-facing reason strings.

The current policy modules may appear only in a behavior-policy manifest and as
production callers before cutover. An architecture test should enforce this
clean-room dependency direction.

Campfire candidate and projection work that already lives with engine mechanics
remains useful. Campfire threat-panel and prospect work may remain an offline
counterfactual evidence producer while it has value, but it does not define the
core outcome schema and does not receive production authority through this
contract.

## Selecting The First Owner

The first owner is chosen only after observation and trajectory capture can
measure candidate surfaces. Candidate sites are compared on:

- complete legal enumeration;
- correctness of deterministic or public-chance successors;
- frequency and diversity in collected trajectories;
- delayed-credit length;
- candidate count and search cost;
- availability of held-out outcome evidence;
- size of the old semantic surface that can be deleted;
- rate of unsupported or out-of-domain observations.

Campfire, card reward, and route are candidates for the first cutover. None is
selected by this document. Shop should normally wait until sequential action
planning is demonstrated because bundle enumeration is explicitly rejected.

## Migration And Deletion Rule

Migration has four gates:

### Gate 1: Contract capture

- hidden-free observations serialize deterministically;
- every legal candidate at the measured sites has stable identity;
- behavior manifests and typed label provenance are present;
- no live decision changes.

### Gate 2: Offline value feasibility

- a simple baseline and the candidate model are evaluated on grouped held-out
  roots;
- goal probability reports Brier or log loss plus calibration and sharpness;
- quantile outputs report pinball loss or CRPS and empirical coverage;
- distribution-shift and mechanics-version strata are visible;
- no threshold is selected from the final held-out partition.

### Gate 3: Shadow planning

- the planner covers every legal root candidate before widening;
- hidden RNG access is statically and dynamically rejected;
- search estimates and exact outcomes retain separate provenance;
- latency, node use, gap rate, and action-family coverage are measured;
- paired incumbent-versus-planner evaluation uses held-out seed groups and
  confidence intervals.

### Gate 4: Production cutover

- one decision site changes to the planner;
- the replaced semantic owner, its production readers, leaked configuration,
  and behavior-pinning tests are deleted in the same change;
- no semantic fallback remains;
- unsupported production cases stop with a typed gap;
- source guards prove the old owner is unreachable.

A long-lived shadow period is allowed only before any production authority is
granted. Once a site cuts over, shadowing the deleted owner is not a reason to
keep it in production source; frozen comparison evidence belongs in artifacts.

## Evaluation Policy

Named seeds are case studies, never training examples selected for their
failure and never promotion gates. In particular, seed006 may explain a failure
mode but cannot prove that the planner is better.

Evaluation reports at least:

- held-out seed and root grouping;
- run-goal success interval;
- terminal-floor distribution;
- planner gap and out-of-domain rates;
- per-site action-family coverage;
- compute distribution, not only mean latency;
- model and continuation-policy manifests;
- results both with and without any imported human data.

Small seed panels are workflow smoke tests. End-to-end policy claims require a
sample size justified before inspecting the result and a confidence interval
reported afterward.

## First Implementation Slice

The first code slice implements contracts and capture only. It does not train a
model or alter a decision:

1. Add an unversioned planner-core module for `PlannerObservation`,
   `LegalCandidateSet`, behavior manifests, and typed label provenance.
2. Build one sanitized observation serializer from public facts and prove that
   hidden RNG and scheduled encounter queues are absent.
3. Add a typed behavior annotation and outcome linkage to the existing
   `SessionTraceV1` decision-history boundary without importing current
   strategic scores or reasons.
4. Write immutable payloads and rebuildable dataset exports under
   `artifacts/runs`, outside Cargo target directories; do not add another
   decision-history, checkpoint, or report authority.
5. Produce a coverage report showing which decision sites can already provide
   complete candidates and which still have typed gaps.
6. Use that report to select, in a later reviewed change, the first owner and
   the smallest outcome-prediction experiment.

The slice should prefer a transparent tabular or small-set baseline before a
large neural model. Architecture is earned by data and ablation, not by model
size.

## Verification

Tests protect contracts rather than strategic opinions:

- identical public states serialize identically;
- harmless reordering of owned entities does not alter the canonical
  observation;
- hidden RNG cursors and future queues cannot affect the observation;
- engine legality and recorded candidate completeness agree;
- candidate ids survive display reordering;
- deterministic scratch successors agree with execution mechanics;
- public chance sampling does not read or advance the live RNG;
- chance-then-decision keeps revelation and recourse separate;
- behavior actions are not serialized as teacher labels;
- label kinds cannot be deserialized into one another;
- search misses cannot become terminal-loss labels;
- counterfactual siblings cannot cross dataset partitions;
- policy priors cannot change legality;
- the planner core cannot import current strategic verdict modules;
- no regression test pins a named seed to a preferred action;
- production cutover guards prove the replaced owner and fallback are gone.

There should be no tests asserting prose reasons, temporary score values, or a
specific card choice solely because the incumbent currently makes it.

## Rejection Criteria

Stop and redesign if:

- the core needs current pressure, prospect, package, or archetype verdicts to
  produce its first useful prediction;
- a model is trained primarily to reproduce incumbent actions and is then
  called an improvement;
- candidate completeness depends on a heuristic top-K;
- a search teacher sees future RNG that the deployed policy cannot see;
- outcome labels cannot name the continuation policy that produced them;
- the first owner is chosen because a named seed currently fails there;
- a new scene-specific score is introduced to cover model uncertainty;
- cutover requires keeping the old owner as fallback;
- the data pipeline grows a second decision-history, checkpoint, or report
  authority.

## Completion Criteria

This core contract is implemented only when:

1. hidden-free observations, complete candidates, successor kinds, behavior
   records, and outcome-label provenance exist as typed interfaces;
2. current owner output is recorded only as behavior provenance;
3. exact, random, counterfactual, search-estimated, and human outcomes remain
   distinguishable through storage and training;
4. one offline outcome model is calibrated on grouped held-out roots without
   consuming current strategic verdicts;
5. one bounded shadow planner compares variable candidates under the same
   objective and information boundary;
6. the first production owner is selected by the measured feasibility report;
7. that owner is cut over with deletion and no semantic fallback;
8. end-to-end evaluation reports uncertainty, policy manifests, compute, and
   gaps without using seed006 as the success criterion.

Until those conditions hold, existing automation remains an incumbent behavior
policy and the new planner remains an experiment. The distinction is explicit
and temporary; it is not a permanent dual-owner architecture.
