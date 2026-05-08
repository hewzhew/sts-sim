# Neutral Engine Query V0

## Status

Accepted as the new search/evidence substrate.

## Decision

The main AI path should not treat `legacy_frontier` or `exact_turn best_line` as policy evidence. The neutral query layer answers only engine-transition questions:

- force this candidate for one engine tick;
- force this candidate to a stable engine boundary;
- summarize the resulting state delta;
- group equivalent observed deltas by `BranchEffectSignature`.

It does not output `best_move`, `chosen_move`, `takeover`, `frontier_score`, or `exact_turn best_line`.

## Current Implementation

- `src/verification/neutral_engine_query.rs` defines `SearchExecutionContext`, `NeutralEngineQueryService`, `NeutralEngineQueryResult`, `BranchEffectVector`, `BranchEffectSignature`, and `BranchEffectGroup`.
- `NeutralEngineQueryResult::to_search_evidence` converts neutral transition facts into `SearchEvidence` with `neutral_*` search kinds.
- Branch compression is currently deterministic over observed transition results. Future random/draw branches should be added as more `NeutralEngineQueryResult`s and then grouped by the same signature mechanism.

## Non-Goals

- It is not a policy.
- It is not a value function.
- It does not rank actions.
- It does not use legacy frontier scoring.
- It does not use exact-turn heuristic ordering.
