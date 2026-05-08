# DecisionEnv Contract V0

## Status

Accepted.

## Context

The simulator must become a reproducible training platform instead of another place to hide bot logic. Recent work showed that mixing `frontier_eval`, exact-turn evidence, verified teacher output, live decisions, and ad-hoc JSON rows makes it difficult to know which component is truth, evidence, label, or action authority.

## Decision

`DecisionEnv` is the canonical integration boundary for AI training and evaluation:

- `core` remains deterministic simulator truth.
- `DecisionEnv` exposes `reset`, `current_timestep`, `step`, `snapshot`, and `restore`.
- `TimeStep` contains versioned public observation, legal `ActionCandidate`s, reward event, terminal/truncation flags, and debug `info`.
- `DecisionRecord` is the canonical versioned dataset row for behavior, teacher, model, human, and baseline decisions.
- Offline teacher output belongs in `teacher_label`; it is not a live command path.
- The initial FullRun adapter exposes a filtered public payload schema. Legacy heuristic fields such as planner scores, plan deltas, reward structure hints, and dominance markers remain available only through debug/audit paths.

## Non-Goals

This does not make `frontier_eval`, exact-turn search, verified teacher, or live bot smarter. It makes their roles explicit so they can stop writing incompatible data and stop silently becoming policy authority.

## Rules

- Training code should consume `DecisionRecord` or `TimeStep`, not legacy planner-specific payloads.
- Policy inference should consume public observation plus action candidates.
- Oracle/debug state may appear in `info`, but not in policy observation.
- `terminated` and `truncated` must remain distinct.
- Legacy planners may remain as fallbacks and diagnostics, but should be named as legacy or evidence when surfaced through this contract.

## Current Implementation

- `src/verification/decision_env.rs` defines the contract.
- `FullRunEnv` implements the contract in `src/cli/full_run_smoke/decision_env.rs`.
- `full_run_env_driver` exposes contract payloads with `decision_env_observation`, `decision_env_step`, and `decision_record_step`.
- `tools/learning/collect_decision_records.py` collects behavior-policy trajectories as `DecisionRecord` JSONL through the driver contract.
- `decision_record_step` can optionally attach `candidate_evaluation_teacher_v0` labels. These labels evaluate candidates and populate `teacher_label`; they do not choose the live action.
- `teacher_label.payload.training_eligibility` records whether the label can be used for training. Fixed-decision horizon labels are marked audit/screening only by default.
- `tools/learning/audit_decision_record_teacher_quality.py` audits `DecisionRecord` JSONL and can fail before training if no eligible labels are present.
- `tools/learning/audit_decision_record_contract.py` checks that records keep public observations public, keep behavior actions legal, and keep legacy heuristic keys out of public observation/candidate payloads.
- `tools/learning/verify_decision_records_replay.py` replays `DecisionRecord` JSONL through the DecisionEnv commands and verifies state hashes, candidate lists, rewards, and terminal flags. It requires the same env config used during collection.

## Next Work

- Replace the filtered JSON public observation with typed public observation structs.
- Split live policy input so it cannot read oracle/debug payloads by construction.
- Move legacy combat planners and frontier heuristics behind explicitly named fallback/evidence interfaces.
