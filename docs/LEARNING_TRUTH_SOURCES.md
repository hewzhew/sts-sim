# Learning Truth Sources

This repo does not currently ship an active ML or RL policy path.

Previous reward-ranker and local combat learner experiments were removed because they had all of these problems at once:

- not wired into the production bot
- trained on tiny or stale datasets
- built against features that no longer match the current strategy stack
- misleading names like `LocalRl` that implied capability that did not exist

## What Still Matters

If learning work resumes later, the only approved data sources should be current audit outputs from the live strategy stack.

### Reward

Use:

- `live_comm_reward_audit.jsonl`
- reward evaluation breakdown already emitted by:
  - `reward_heuristics`
  - `deck_delta_eval`
  - `run_rule_context`

Important fields now come from the live path itself:

- `delta_prior`
- `delta_bias`
- `delta_rollout`
- `delta_context`
- `delta_context_rationale_key`
- `delta_rule_context_summary`

### Event

Use:

- `event_audit.jsonl`

Only consume samples from runs whose validation passed and whose event trace fields are present.

### Combat

Use:

- `combat_decision_audit`
- validated `livecomm` runs
- root diagnosis / suspect output
- `failure_snapshots.jsonl`
- belief summaries
- sequencing breakdowns

For battle-local training work, the current approved environment entrypoint is:

- `sts_simulator::bot::harness::combat_env::CombatEnv`

This is a headless Rust-only combat environment. It is intended for fixture-driven
training and evaluation, not as a `livecomm` replacement.

Important distinction:

- validated `livecomm` runs are the approved **state truth** source
- stronger combat labels should come from offline oracle work, starting with
  `combat_decision_audit audit-frame`

Do not treat the raw baseline bot action from `livecomm` as oracle truth by default.

Do not revive old hand-authored ranker formats or training-example dumps from `combat_lab`.

## Current Approved Infrastructure

The repo still intentionally keeps:

- `combat_decision_audit`
- validated `livecomm` run logs
- reward / event audit streams
- `combat_lab` as a regression and fixture-comparison harness

These are the only supported foundations for future learned sidecars.

## Readiness Gate Before New Learning Work

Do not restart ML/RL work until these are true:

- engine/protocol truth is stable enough that `VERDICT: ❌ Engine Bugs Found` is no longer common
- `livecomm` validation is stable
- audit schemas for reward/event/combat are no longer churning
- the current symbolic policy layers are the ones we actually want to learn on top of:
  - belief-driven combat pressure
  - sequencing breakdowns
  - regime-conditioned deck delta

## Explicit Non-Goals

This repo is not currently maintaining:

- a reward-ranker training pipeline
- a local combat RL policy
- a production learned reranker
- a placeholder ML scaffold

Any future learning line should be reintroduced from current audit truth, not by restoring removed legacy scripts.

## Current Dataset Workflow

If learning work resumes, the approved first step is:

1. freeze a baseline
2. build datasets from existing audit artifacts
3. evaluate offline before any runtime inference

Commands:

```powershell
cargo run --bin sts_dev_tool -- logs freeze-baseline `
  --out tools/artifacts/learning_baseline.json `
  --latest-runs 3

python tools/learning/build_sidecar_datasets.py `
  --baseline tools/artifacts/learning_baseline.json `
  --out-dir tools/artifacts/learning_dataset
```

This workflow is intentionally narrow:

- no new `src/ml`
- no placeholder trainer
- no production inference path
- no mixing rows from different frozen baselines

It only converts validated audit truth into offline rows for future sidecar experiments.
