# Rollout First-Win Provenance Design

## Goal

Make `CombatSearchV2Stats::nodes_to_first_win` report the main-search node count at which the rollout witness that is later accepted by exact replay was first observed. A root rollout witness is therefore observed at node count `0`; a child or deferred rollout witness uses the current generated-node count.

## Reliability Boundary

A rollout estimate remains estimate-only. Observing a replayable terminal-win witness must not update public win statistics. The search may publish its recorded discovery count only after the existing post-loop exact replay reproduces that same witness and verifies its terminal state and outcome facts.

If replay fails, neither `terminal_wins` nor `nodes_to_first_win` changes because of that witness.

## Design

Store the selected replayable terminal-win witness together with its `nodes_generated` discovery snapshot inside `RolloutCache`. Pass the current generated-node count into each timed rollout evaluation. When a newly evaluated witness replaces the selected best witness, its discovery snapshot replaces the previous snapshot with it.

Post-loop promotion replays the selected witness exactly as it does today. On success, it records the win using the witness's discovery snapshot. Ordinary main-frontier wins keep using the current generated-node count, and an earlier exact win remains authoritative through the existing first-write behavior.

This change does not alter rollout scheduling, cache keys, witness ranking, frontier order, early stopping, action selection, or combat outcomes.

## Alternatives Rejected

- Updating `nodes_to_first_win` when rollout evaluation sees a terminal estimate would cross the reliability boundary before exact replay.
- Inferring the discovery count during post-loop promotion is impossible after the intermediate node-count history has been discarded.
- Adding a second public rollout-only timing metric would leave the existing misleading `nodes_to_first_win` value in candidate summaries.

## Verification

Add a regression where the root rollout finds a replayable win before any main-search node is generated, while the search still reaches its node budget before post-loop promotion. The promoted exact win must report `nodes_to_first_win == Some(0)` rather than the final generated-node count.

Retain existing tests for exact replay, HP-loss acceptance, main-frontier wins, and turn-plan-generated wins. Finish with the full library, architecture-boundary, driver, formatting, and diff checks required by the repository workflow.
