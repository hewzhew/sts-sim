# Weak Evidence Paths Removed

## Status

Accepted.

## Decision

The repo no longer treats baseline continuation, single-seed counterfactuals, or
local branch rollout scores as policy evidence.

Removed from active code/docs:

- BranchTrace datasets
- `evaluate_candidates`
- verified advantage override teacher
- live snapshot teacher shadow/takeover entrypoint
- DecisionRecord `teacher_label`
- return-Q / pairwise teacher training
- Gym/PPO full-run scripts
- combat preference export from `combat_decision_audit`

## Reason

These paths made weak local signals look more authoritative than they were. The
result was seed-driven policy pollution: a death or local rollout would become a
general rule without a trustworthy source of truth.

## Current Boundary

The only active AI-facing data path is:

```text
full_run_env_driver
-> DecisionRecord capture
-> contract audit
-> deterministic replay verification
-> explicit full-run outcome evaluation
```

The baseline bot remains useful as a comparator and simulator stress test. It is
not a teacher.
