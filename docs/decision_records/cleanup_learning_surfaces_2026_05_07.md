# Cleanup Learning Surfaces

Date: 2026-05-07

## Observation

The project had too many experimental learning routes in the active tool
surface. This made it easy to confuse negative baselines with current strategy.

The most confusing cases were:

- shallow draw/query labels
- candidate-pack local oracle labels
- absolute return-Q direct selectors
- learned advantage override selectors
- learned proposer pruning
- verifier margin grids treated as if they were algorithmic progress

The Rust `full_run_env_driver` had also grown past 5k lines. That is a real
maintenance smell, especially because verified teacher logic was mixed with the
driver protocol and candidate evaluation code.

## Decision

The active learning surface is now:

```text
verified teacher diagnostics
-> harmful override audit
-> leaf / continuation / evidence protocol improvement
-> optional future distillation
```

Archived or downgraded routes are moved under:

```text
tools/learning/_archive/2026_05_failed_routes/
```

They are preserved for reproduction but are not active mainline code.

## Current Mainline Files

- `tools/learning/return_q_common.py`
- `tools/learning/eval_verified_adv_override_rust_runner.py`
- `tools/learning/run_verified_teacher_diagnostics.py`
- `tools/learning/audit_verified_teacher_pending_coverage.py`

## Non-Mainline

- return-Q direct selector scripts
- learned override selector scripts
- learned proposer training/pruning scripts
- candidate-pack trainability/dominance audits
- draw/query-axis datasets as training labels

## Next Refactor Target

`src/bin/full_run_env_driver/main.rs` has been mechanically split so verified
override implementation lives in:

```text
src/bin/full_run_env_driver/verified_override_impl.rs
```

The split is intentionally low-risk: it uses `include!` to preserve the old
scope and avoid a large visibility refactor while the API is still moving.

The next cleanup step, once behavior stabilizes, is to replace the include split
with real modules:

```text
protocol.rs
candidate_evaluation.rs
verified_override.rs
verified_stats.rs
counterfactual_pending.rs
session.rs
```

This should be a mechanical move with no behavior change and should be done only
with compile checks before and after.
