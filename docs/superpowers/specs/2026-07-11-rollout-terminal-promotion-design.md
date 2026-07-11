# Rollout Terminal Promotion Design

## Problem

Combat search can execute a complete winning rollout and still return no
complete trajectory. Rollouts currently retain only an estimate and an action
preview for frontier ordering. `SearchTrajectoryBook`, and therefore final
reporting and owner commit, only sees nodes reached by exact frontier
expansion.

On seed `20260711001`, the Time Eater rollout produced a 62-action win ending
at 7 HP. The review tool replayed every action exactly and reached the same
terminal state, while the bounded main search reported no complete trajectory.
A witness-prior rerun then found the same exact win quickly. The failure is the
missing handoff from a replayable rollout witness to the exact trajectory book,
not deck construction, owner HP policy, or simulator correctness.

## Approaches Considered

1. Treat a terminal rollout estimate as a complete trajectory. This crosses the
   existing evidence boundary without checking that the stored preview is
   complete and executable, so it is rejected.
2. Restart search with the rollout actions as priors. This works, but repeats a
   substantial search budget and does not guarantee that the witness itself is
   returned.
3. At search finish, replay one complete terminal witness from the original
   root and promote it only if exact replay agrees. This is bounded, preserves
   the exact-evidence boundary, and is selected.

## Decision

`RolloutCache` retains its best replayable terminal-win estimate. A candidate is
replayable only when it stopped at a terminal state, was not truncated, and its
entire root-to-terminal action sequence fits in the stored preview.

After frontier search ends, and only when no exact win has already been found,
combat search attempts one promotion:

- start from the original root node;
- for each previewed action, find the exact legal action by both input and
  action key;
- apply it with the normal per-action engine-step limit and no expired search
  deadline;
- rebuild the normal action trace and resource counters;
- reject on an illegal action, truncation, timeout, early terminal, leftover
  actions, or a terminal/final-HP/enemy-state mismatch;
- otherwise insert the replayed `SearchNode` through the existing
  `remember_win` path.

The verification tail is bounded to one candidate and at most the existing
96-action preview limit. It does not rerun search and does not trust estimated
terminal state.

## Scope

Change only generic combat-search rollout evidence handling. Do not change
rollout policy, frontier ranking, owner HP acceptance, run-control commit
semantics, deck construction, or add a seed-specific rule.

## Verification

Use a tiny deterministic stepper where the root rollout wins but a one-node
frontier budget cannot expand the winning child. The current code must report
no complete trajectory; the new code must return the exactly replayed win.
Add rejection coverage for an incomplete preview. Then run the full library,
architecture boundary tests, formatting, diff checks, and the same bounded
seed review.
