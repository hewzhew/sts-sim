# Learning Truth Sources

Learning work is active again, but the truth policy is still strict:

- learning stays downstream of engine truth
- `live_comm` provides state truth, not automatic labels
- stronger labels come from offline oracles, audits, and explicit contracts

## Truth Hierarchy

### 1. Engine and Protocol Truth

Highest-priority sources:

- Java source in `cardcrawl`
- current `CommunicationMod` protocol export
- Rust importer plus parity/test surfaces

If this layer is unstable, learning output is not allowed to define correctness.

### 2. Checked-In Protocol Samples and Behavior Tests

Use these to pin importer-sensitive mechanics:

- `tests/protocol_truth_samples/`
- `tests/protocol_truth_samples.rs`
- behavior tests such as `stasis_behavior.rs`

These are the small, explicit truth anchors that keep dataset assumptions honest.

### 3. Validated Archived Runs

Approved real-state sources:

- archived `live_comm` runs whose validation passed
- frozen baselines created by `sts_dev_tool logs freeze-baseline`

Use them for:

- state distributions
- audit streams
- replay-first frame extraction

Do not use tainted runs as if they were ground truth.

### 4. Offline Combat Label Sources

Approved stronger combat label sources:

- `combat_decision_audit`
- local oracle rollouts
- structured benchmark cases
- curated curriculum or seed datasets

Important rule:

- archived or live bot choices are baseline behavior
- offline oracle outputs are candidate strong labels

### 5. Local Curriculum and Contract Surfaces

Approved local experiment sources:

- `combat_lab` specs
- `CombatEnv`
- `combat_env_driver`
- structured combat observation/action contract

These are for controlled learning experiments, not for replacing protocol truth.

## Approved Data Families

Reward:

- `reward_audit.jsonl`
- derived hindsight/counterfactual datasets

Event:

- `event_audit.jsonl`
- macro counterfactual datasets

Combat:

- baseline manifests and archived clean runs
- `combat_suspects.jsonl`
- `failure_snapshots.jsonl`
- `combat_lab` trace exports
- local oracle datasets

## What Not To Do

Do not:

- treat raw `live_comm` bot actions as oracle labels
- restore stale legacy learning formats just because a script still exists
- invent a second combat ontology outside the current contract docs
- let a learner output dictate protocol or engine semantics

## Canonical Learning Entry Docs

- [COMBAT_RL_CONTRACT_V0.md](COMBAT_RL_CONTRACT_V0.md)
- [../RL_READINESS_CHECKLIST.md](../RL_READINESS_CHECKLIST.md)
- [../../tools/learning/README.md](../../tools/learning/README.md)
