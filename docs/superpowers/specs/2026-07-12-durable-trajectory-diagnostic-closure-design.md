# Durable Trajectory Diagnostic Closure Design

## Purpose

The paired trajectory comparison currently exists only while multiple branches remain in the
frontier. A challenger that reaches a non-resumable gap before the baseline is removed from the
frontier, and a later final result overwrites the capsule summary with the last stopped branch.
The run therefore loses the strongest cross-policy evidence exactly when the investigation reaches
its blocker.

Combat cases have a related loss. `BranchPathStep` already carries policy identity, candidate-pool
evidence, annotations, decision deltas, and shop preview evidence, but the combat-case projection
keeps only the decision key, label, and state before the decision. The saved fight remains
replayable, but it cannot answer whether the owner saw and rejected a relevant construction repair.

This delivery closes those two diagnostic gaps without changing branch execution, frontier
retention, owner decisions, combat search, or policy promotion.

## Scope

The delivery must:

- durably retain the latest observation for every baseline or challenger policy lane across
  generations and bounded slices;
- update that evidence when a frontier is saved and when a branch becomes non-resumable;
- expose the latest paired evaluation in final gap, completed, and terminal capsule output;
- preserve branch policy identity and complete recorded decision evidence in new combat cases;
- continue loading combat cases written before these additive fields existed;
- remain evidence-only: no scheduler, frontier, owner, or search module may consume a comparison
  verdict.

The delivery must not:

- keep stopped branches alive in the runtime frontier;
- use an artifact file as resumable execution state;
- change policy-lane selection, branch retention, combat budgets, or production choice ordering;
- infer pressure coverage or deployability from HP, static deck adequacy, or a missing search win;
- rerun the bounded seed as an implementation requirement.

## Considered Approaches

### Retain stopped branches in the frontier

This would make all observations available at finalization, but it would mix diagnostic lifetime
with scheduler lifetime. Frontier capacity, retention, and later expansion behavior could change.
This approach is rejected.

### Widen one generation result from one branch to a collection

This would prevent same-generation overwrite, but a stopped challenger would still disappear when
a later generation or resumed slice writes its result. It fixes the immediate symptom but not the
durability contract. This approach is rejected.

### Persist an evidence-only latest-lane state

The capsule owns a small `trajectory_state.json` artifact containing the latest observation for
each policy lane. Frontier saves refresh all live lanes. Stopped-branch observation refreshes that
lane before it leaves runtime state. Final formatting derives the paired evaluation from the latest
observations. This approach is selected because it crosses slices without affecting execution.

## Durable Trajectory State

The owner-audit capsule adds `trajectory_state.json` with schema
`branch_tiny_trajectory_state_v0`.

Each observation contains:

- `generation`;
- `branch_id` and optional `parent_id`;
- the structured branch status used when the observation was recorded;
- the existing typed `TrajectorySnapshot`, including its policy-lane label.

There is at most one current observation per lane label. A newer `(generation, branch_id)` replaces
an older observation for the same lane. The file serializes observations in deterministic order:
baseline first, followed by challenger lane number.

The state also serializes the derived `FrontierTrajectoryEvaluation` so the artifact is directly
inspectable. Derivation uses the existing conservative comparator:

- the baseline is compared independently with every challenger;
- missing baseline evidence yields no comparisons;
- pressure and deployability remain `Unknown` until real instrumentation supplies them;
- this delivery does not promote, prune, or rank a branch from the verdict.

`trajectory_state.json` is an output artifact, not a checkpoint. Frontier resumption continues to
use `frontier.json`; execution must remain correct if the trajectory artifact is absent. When a
capsule resumes and the file exists, new observations merge with it only for diagnostic continuity.

## Recording Lifecycle

The evidence store has two write paths:

1. Before a frontier summary is written, record every branch currently in the frontier.
2. When a branch is observed as terminal or otherwise non-resumable, record that branch before it
   is discarded or written as the selected result.

Final `summary.json` and `result.json` include a top-level `trajectory_evaluation` derived from the
durable state. Frontier summaries keep their existing nested frontier evaluation and use the same
state-derived facts. Terminal entries include the branch-local snapshot as today; the capsule-level
summary retains the cross-lane evaluation.

Writing a selected result records that branch again as an idempotent safety measure. A failed
trajectory evidence write is a capsule artifact error, not a silent omission, because durable
diagnostics are part of the requested run contract.

## Combat-Case Evidence

`CombatCase` receives an additive optional `branch_evidence` object. It contains:

- an internal evidence schema identifier;
- the serialized `BranchPolicyLane`, including challenger policy memory;
- the typed `TrajectorySnapshot` captured at the combat gap.

`CombatCasePathStep` receives additive optional decision evidence containing every field already
recorded by `BranchPathStep` but previously dropped by the projection:

- `policy_lane`;
- selected-choice annotation;
- decision delta;
- the complete ordinary candidate pool;
- shop boss-preview candidates;
- shop boss-preview bundles.

The existing `key`, `label`, and `state_before` fields remain unchanged so current review tooling
and old cases continue to work. New fields use serde defaults and omission rules. Loading an old
case produces absent branch evidence and empty decision evidence rather than an error or fabricated
facts.

The combat-case file remains replay input. Decision evidence is read-only context and must not alter
the reconstructed `CombatPosition` or search configuration.

## Ownership Boundaries

- `ai::strategy::trajectory_comparison` continues to own pure comparison vocabulary and rules.
- owner-audit trajectory adaptation owns conversion from `Branch` into typed snapshots.
- a focused owner-audit trajectory evidence store owns `trajectory_state.json` merging and writing.
- the capsule artifact store coordinates evidence writes with existing summary/result writes.
- `eval::combat_case` owns backward-compatible public combat-case evidence types.
- owner-audit combat-gap projection fills those types from private branch-path records.
- run-control, owners, policy expansion, branch frontier, and combat search do not read the new
  artifact or verdict.

## Failure and Compatibility Rules

- Missing `trajectory_state.json` starts an empty diagnostic state; it does not block a fresh or
  legacy capsule resume.
- Malformed existing trajectory state returns a descriptive capsule artifact error instead of
  overwriting evidence silently.
- Old combat cases without the additive fields continue to deserialize and replay.
- Unknown pressure/deployability stays unknown in all final artifacts.
- Candidate evidence is serialized from the already-recorded `BranchPathStep`; the combat-case
  writer does not reconstruct past choices from current deck state.

## Verification

Red-green tests must prove:

1. a baseline and challenger frontier creates two durable lane observations;
2. a challenger gap followed in a later generation by a baseline gap leaves both latest snapshots
   in the final evaluation;
3. reopening the evidence store, as a resumed slice would, preserves and updates prior lanes;
4. final summary/result formatting exposes the state-derived paired evaluation without a live
   frontier;
5. a new challenger combat case round-trips policy memory, trajectory snapshot, candidate pool,
   annotation, decision delta, and shop preview evidence;
6. a legacy combat case without new fields still loads;
7. static architecture checks find no trajectory verdict or evidence-state consumer in run-control,
   frontier retention, branch generation policy, owners, or combat search.

Completion runs focused tests, the complete library suite, `branch_tiny` compilation, and
`architecture_runtime_boundaries`. No seed rerun is required because the saved Awakened One cases
remain valid combat inputs and the missing historical candidate pools cannot be reconstructed
honestly after the fact.
