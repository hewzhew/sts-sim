# Neutral Engine Query V0

## Status

Accepted as the new search/evidence substrate.

## Decision

The main AI path should not treat `legacy_frontier` or `exact_turn best_line` as policy evidence. The neutral query layer answers only engine-transition questions:

- force this candidate for one engine tick;
- force this candidate to a stable engine boundary;
- force this candidate to a named/aligned boundary;
- compare two candidates under the same neutral transition contract;
- sample draw-order branch effects as outcome samples, not observations;
- summarize the resulting state delta;
- group equivalent observed deltas by `BranchEffectSignature`.

It does not output `best_move`, `chosen_move`, `takeover`, `frontier_score`, or `exact_turn best_line`.

## Current Implementation

- `src/verification/neutral_engine_query.rs` defines `SearchExecutionContext`, `NeutralEngineQueryService`, `NeutralEngineQueryResult`, `BranchEffectVector`, `BranchEffectSignature`, and `BranchEffectGroup`.
- `NeutralEngineQueryResult::to_search_evidence` converts neutral transition facts into `SearchEvidence` with `neutral_*` search kinds.
- `NeutralEngineQueryResult` records `boundary_kind`, `observability`, `scenario_debug`, exactness, truncation, before/after summaries, deltas, and branch effect vectors.
- Branch compression is deterministic over observed transition results. Random/draw branches are represented as additional `NeutralEngineQueryResult`s and grouped by the same signature mechanism.
- `draw_top_card_branch_effects` can sample draw-pile alternatives by forcing each selected draw-pile card to the top. These results are marked `FutureSample`; they may guide search allocation or audit, but they are not public observation.
- Pending choices such as Headbutt-style follow-up choices are represented as `BoundaryKind::PendingChoice`. They are not flattened into root action products.
- `src/app/policy_runner` contains `NeutralProbeEvaluator`, the non-legacy audit component over this substrate. It emits a deliberation trace from neutral evidence and records only short-horizon diagnostic signals; it does not select actions.

## Non-Goals

- It is not a policy.
- It is not a value function.
- It does not rank actions.
- It does not use legacy frontier scoring.
- It does not use exact-turn heuristic ordering.
