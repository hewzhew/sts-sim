# Reproducible Search Comparability Design

## Goal

Make branch counterfactual reports distinguish deterministic search evidence from
results decided by machine-speed-dependent wall deadlines. The first consumer is
the seed006 Act 2 Boss Relic comparison, resumed from one exact cutpoint and forked
into Black Blood, Coffee Dripper, and Philosopher's Stone arms.

This work classifies evidence only. It does not change search ordering, line
acceptance, strategy scores, or game policy.

## Execution Addendum: Exact-Fork Scope

The first seed006 execution exposed two assumptions that the initial design had
not verified:

1. the generic challenger planner only created extra lanes for card-candidate
   annotations, so a Boss Relic boundary still expanded only its first choice;
2. each child retained the complete shared search history, so wall-limited
   evidence from before the exact cutpoint incorrectly excluded every relic pair.

Boss Relic experiments therefore use the existing three policy-lane identities
for the three presented relic picks: the production pick remains `baseline`, and
the other two picks become `challenger-1` and `challenger-2`. Skip is not a fourth
experimental arm. This changes experimental branching only when branch capacity
is greater than one; it does not change relic scores or the one-branch mainline.

When the first multi-lane fork is created, every child records the same search
history index as its comparison horizon. The full history remains persisted and
is classified as `full_search_comparability`; pair eligibility uses
`search_comparability`, which classifies only evidence at or after that shared
horizon. A legacy checkpoint without a horizon continues to classify its full
history, so compatibility fails closed rather than inventing a clean suffix.

Two persistence/retention invariants are part of this boundary:

- omitted relic counters deserialize to the engine sentinel `-1`, not the Rust
  integer default `0`, so a written path round-trips to the same control hash;
- frontier deduplication includes the first divergent decision key when one is
  present, so semantically similar policies do not merge distinct relic arms.
  Repeated branches with the same divergence key and strategy signature may
  still merge normally.

## Existing Evidence and the Missing Boundary

`CombatSearchV2Report` already records:

- configured node and wall limits;
- expanded/generated nodes and the first exact-replayed win node;
- `deadline_hit` and `node_budget_hit`;
- coverage status;
- exact execution adjudication after replay.

Run-control traces retain most of this information and branch checkpoints retain
the complete combat-search history. The important schema loss is that
`node_budget_hit` is dropped when the search report becomes a run-control trace.
The trajectory comparison layer also has no explicit eligibility field, so a
wall-decided arm can still be displayed beside a deterministic arm without a
machine-readable exclusion reason.

## Chosen Approach

Preserve the missing node-budget bit, derive one pure comparability classification
from retained search traces, and attach that classification to durable trajectory
snapshots and pair comparisons.

This is narrower than a dedicated experiment runner and safer than merely raising
wall limits. It reuses exact evidence that already exists and does not introduce a
second search-reporting system.

Alternatives not chosen:

1. A dedicated Boss Relic experiment CLI would isolate the workflow strongly but
   duplicate checkpoint, branching, artifact, and comparison plumbing.
2. Very large wall limits plus fixed node limits would usually work, but a slow or
   contended run could still become wall-decided without failing closed.
3. Making the search itself wall-clock deterministic is neither realistic nor
   necessary; the contract only needs to reject invalid comparisons.

## Attempt Classification

Each retained `CombatSearchTraceSummary` is classified independently. A later
rescue lane does not erase the provenance of an earlier primary lane because a
faster primary search could have selected a different line before rescue began.

The order is deliberate:

1. `comparable_exact_accepted`: execution adjudication is `Accepted`. Exact replay
   makes this attempt usable even if the search loop also observed its safety
   deadline after discovering the accepted witness.
2. `wall_safety_limited`: no exact accepted adjudication and either
   `deadline_hit == true` or coverage is `TimeBudgetLimited`.
3. `comparable_node_bounded`: no wall stop and either `node_budget_hit == true` or
   coverage is `NodeBudgetLimited`.
4. `comparable_exhaustive`: coverage is `Exhaustive` with no wall stop.
5. `insufficient_evidence`: replay failed, coverage remains open, an accepted
   candidate lacks execution adjudication, or the coverage vocabulary is unknown.

An exact rejection under an exhaustive or node-bounded attempt is still
comparable negative evidence. A raw or estimated win without exact execution
adjudication is not promoted to comparable evidence.

## Arm Classification

One branch arm aggregates every retained attempt with fail-closed precedence:

1. any `wall_safety_limited` attempt makes the arm `wall_safety_limited`;
2. otherwise any `insufficient_evidence` attempt makes the arm
   `insufficient_evidence`;
3. otherwise the arm is `comparable`, including a branch with no relevant combat
   search yet.

The summary records total, exact-accepted, node-bounded, exhaustive,
wall-limited, and insufficient attempt counts. Counts make the classification
auditable without embedding full trace payloads in every trajectory snapshot.

## Data Model

Add `node_budget_hit: bool` with serde defaults to both
`CombatSearchPerformanceSnapshotV1` and `CombatSearchTraceSummary`, and populate it
from `CombatSearchV2Report::stats`.

Add generic diagnostic vocabulary to
`ai::strategy::trajectory_comparison`:

- `TrajectorySearchComparabilityStatus`:
  `Comparable`, `WallSafetyLimited`, `InsufficientEvidence`;
- `TrajectorySearchComparability`: status plus attempt counters;
- `TrajectoryPairEligibility`:
  `Comparable`, `ExcludedWallSafetyLimited`,
  `ExcludedInsufficientEvidence`.

`TrajectorySnapshot` gains a serde-defaulted `search_comparability` field.
`TrajectoryComparison` gains a serde-defaulted `eligibility` field. Missing fields
in old trajectory snapshots deserialize as `InsufficientEvidence` with zero
attempt counts. Missing pair eligibility deserializes as
`ExcludedInsufficientEvidence`. Legacy artifacts therefore remain readable without
being silently certified.

The run-control-aware classifier lives under owner-audit, not in the generic
strategy comparison module. It consumes `combat_search_history` and produces the
generic comparability summary. This keeps the dependency direction one-way:

```text
CombatSearchV2Report
  -> run-control performance trace
  -> Branch.combat_search_history
  -> owner-audit search comparability classifier
  -> generic TrajectorySnapshot evidence
  -> generic TrajectoryComparison eligibility
```

## Comparison Behavior

The existing progression, pressure, deployability, resource, and construction
layers are still calculated for inspection.

- If both arms are comparable, the current verdict calculation is unchanged.
- If either arm is wall-safety-limited, pair eligibility is
  `excluded_wall_safety_limited` and the verdict is `Inconclusive`.
- Otherwise, if either arm has insufficient evidence, pair eligibility is
  `excluded_insufficient_evidence` and the verdict is `Inconclusive`.

This is diagnostic-only behavior. No scheduler, owner, card/relic policy, or combat
search module may consume pair eligibility to choose actions.

## Persistence and Compatibility

New checkpoints and trajectory artifacts preserve the explicit node-budget bit and
comparability summary. Old frontier checkpoints remain loadable because new trace
fields use serde defaults. Old trajectory-state artifacts remain readable, but
their missing comparability evidence is conservatively excluded when the state is
refreshed.

No new CLI flag is required. Normal run behavior is unchanged; only diagnostic
comparison artifacts gain stricter eligibility semantics.

## Error Handling

- Unknown coverage labels classify as insufficient evidence rather than panicking.
- Replay failure is insufficient evidence even when a raw winning trajectory was
  observed.
- A wall-limited attempt cannot be overridden by a later lane in the same arm.
- Serialization compatibility never manufactures a comparable label for missing
  legacy evidence.
- Existing exact cutpoint fingerprint validation remains the authority for the
  experiment start state.

## Verification

Test-first coverage will prove:

1. run-control traces preserve `node_budget_hit` from the search report;
2. an exact accepted attempt is comparable even when the loop also reports a
   deadline;
3. a wall-limited attempt without accepted adjudication is excluded;
4. a later accepted rescue does not erase an earlier wall-limited primary;
5. a node-bounded exact rejection remains comparable negative evidence;
6. replay failure and unknown/legacy coverage are insufficient evidence;
7. comparable pairs retain the existing verdict, while excluded pairs are forced
   to `Inconclusive` with an explicit eligibility reason;
8. old trace and trajectory JSON remains readable and fails closed.

Finish with focused tests, formatting, `architecture_runtime_boundaries`, and the
full library suite.

## Seed006 Handoff

After implementation passes:

1. run one bounded seed006 mainline with a run capsule until the exact Act 2 Boss
   Relic cutpoint is written;
2. verify the cutpoint manifest and the expected HP, gold, deck, relic/RNG state,
   and candidate order;
3. resume the same cutpoint with enough branches to retain the three relic arms;
4. use fixed node limits and wall limits only as safety stops;
5. rank arms only if all relevant pair eligibility values are comparable;
6. if any arm is wall-safety-limited or insufficient, report the experiment as
   inconclusive and retain its resumable cutpoint instead of rerunning the seed
   prefix.

## Non-Goals

- No global claim about the best Boss Relic from one seed.
- No strategy-score changes for relics, cards, routes, campfires, shops, or
  potions.
- No replacement of exact replay with estimated rollout evidence.
- No attempt to equalize execution speed across machines.
- No new unbounded journal or duplicated combat laboratory.
