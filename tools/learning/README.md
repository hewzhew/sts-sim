# Learning Tools

This directory no longer contains an active "verified teacher", BranchTrace,
candidate rollout, or DecisionRecord teacher-label pipeline.

Those paths were removed because they promoted weak baseline continuation and
seed counterfactuals into reusable evidence. The full-run driver now exposes
only environment stepping, public observations, policy input snapshots,
baseline policy stepping, preview, and raw DecisionRecord transition capture.

Current allowed uses:

- simulator smoke tests
- replay and contract checks
- explicit full-run policy evaluation where the final run outcome is the metric
- local diagnostic scripts that do not create action labels from branch traces,
  candidate rollout returns, or teacher-label fields

Tracked files in this directory are intentionally limited to:

- `full_run_env.py`
- `smoke_full_run_env.py`
- `analyze_full_run_policy_matrix.py`
- `evaluate_full_run_capabilities.py`
- `audit_decision_record_contract.py`
- `collect_decision_records.py`
- `collect_decision_records_batch.py`
- `verify_decision_records_replay.py`

Do not reintroduce scripts that:

- call `branch_trace`
- call `evaluate_candidates`
- call `run_verified_adv_override_*`
- collect `neutral_policy_trace`
- train from `teacher_label`
- convert single-seed counterfactuals into policy labels

Deleted files remain recoverable from Git history and from the backup branches
created before this cleanup.
