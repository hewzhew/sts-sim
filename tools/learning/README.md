# Learning Tools

This directory is now DecisionRecord infrastructure only.

It does not contain an active verified teacher, BranchTrace, candidate rollout,
Gym/PPO, return-Q, pairwise preference, or teacher-label pipeline. Those paths
were removed because they turned weak baseline continuation and single-seed
counterfactuals into reusable labels.

Allowed uses:

- collect raw DecisionRecord transitions from `full_run_env_driver` using only
  externally selected legal actions or random-masked smoke actions
- audit that records expose legal/public payloads only
- replay records through the driver and compare hashes/candidates/outcomes
- run explicit full-run outcome evaluation for externally supplied controllers

Replay checks must use the same env config as collection. `max_steps`, class,
ascension, and final-act status are part of the state hash.

Tracked scripts:

- `audit_decision_record_contract.py`
- `collect_decision_records.py`
- `collect_decision_records_batch.py`
- `verify_decision_records_replay.py`

Do not reintroduce scripts that:

- call `branch_trace`
- call `evaluate_candidates`
- call `run_verified_adv_override_*`
- collect controller-shadow traces
- collect search-allocation traces
- train from `teacher_label`
- convert single-seed counterfactuals into controller labels

Deleted files remain recoverable from Git history and from the backup branches
created before cleanup.
