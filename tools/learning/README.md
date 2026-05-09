# Learning Tools

This directory is now DecisionRecord infrastructure only.

It does not contain an active verified teacher, BranchTrace, candidate rollout,
Gym/PPO, return-Q, pairwise preference, or teacher-label pipeline. Those paths
were removed because they turned weak baseline continuation and single-seed
counterfactuals into reusable labels.

Allowed uses:

- collect raw DecisionRecord transitions from `full_run_env_driver`
- audit that records expose legal/public payloads only
- replay records through the driver and compare hashes/candidates/outcomes
- run explicit full-run policy evaluation where the final run outcome is the
  metric

Tracked scripts:

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
created before cleanup.
