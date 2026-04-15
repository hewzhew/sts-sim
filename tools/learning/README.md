# Learning Sidecar Workflow

This directory is intentionally narrow.

It does **not** contain a runtime inference path or a placeholder `src/ml` scaffold.
It contains the workflow needed to:

1. freeze a reproducible symbolic baseline
2. build reward / event / combat datasets from approved audit truth sources
3. oracle-label battle-local combat rows with a stronger offline search
4. review and pack those rows for the first offline combat reranker experiments
5. run a lightweight baseline trainer and an optional small neural reranker
6. build combat transition / value / policy datasets from simulator outcomes
7. run transition / value / PPO-ready policy baselines
8. build macro hindsight datasets for reward / shop / event

## RL Environment

Create the dedicated training environment first:

```powershell
python -m venv .venv-rl
.\.venv-rl\Scripts\python -m pip install -r tools/learning/requirements-hybrid-rl.txt
```

This environment is the default for:

- PyTorch transition/value training
- Gymnasium combat env wrapping
- SB3 / `sb3-contrib` `MaskablePPO` combat policy training

The current default is CPU-first. If you later want CUDA, treat that as a
separate install profile rather than mutating the baseline requirements file.

## Q_local Tactical Evaluator

The current post-reranker direction is a local tactical evaluator:

- freeze explicit combat card semantics
- build a small-horizon local rollout oracle over replay-first combat sources
- train a multi-head `Q_local(s, a)` model
- evaluate it first as an offline root-ordering prior over replay/raw frames before touching PPO or runtime search

The first-pass commands are:

```powershell
.\.venv-rl\Scripts\python tools/learning/build_combat_local_oracle.py `
  --limit-groups 90 `
  --rollout-seeds 4 `
  --min-replay-test-groups 16

.\.venv-rl\Scripts\python tools/learning/train_q_local_torch.py `
  --dataset-prefix combat_q_local `
  --epochs 60

.\.venv-rl\Scripts\python tools/learning/export_q_local_root_prior.py

cargo run --quiet --bin combat_decision_audit -- export-search-baseline `
  --raw D:\rust\sts_simulator\logs\runs\<run_id>\raw.jsonl `
  --frames 8,12,24 `
  --out tools/artifacts/learning_dataset\search_baseline.json

cargo run --quiet --bin combat_decision_audit -- export-search-baseline `
  --raw D:\rust\sts_simulator\logs\runs\<run_id>\raw.jsonl `
  --frames 8,12,24 `
  --out tools/artifacts/learning_dataset\search_with_q_local_prior.json `
  --q-local-prior tools/artifacts/learning_dataset\q_local_root_prior.jsonl `
  --q-local-shadow

.\.venv-rl\Scripts\python tools/learning/evaluate_q_local_rust_search.py `
  --baseline tools/artifacts/learning_dataset\search_baseline.json `
  --prior tools/artifacts/learning_dataset\search_with_q_local_prior.json
```

Notes:

- `q_local_root_prior.jsonl` is a static lookup artifact for Rust offline shadow.
- replay-first exports now retain uncertain `replay_frame` rows by default for shadow coverage.
- current replay/raw validation should prefer `DiagnoseSearchFrame` and `ExportSearchBaseline` over spec-only fixture checks.
- `audit-fixture` can still be exercised with explicit key overrides:
  - `--q-local-prior-spec-name`
  - `--q-local-prior-episode-id`
  - `--q-local-prior-step-index`

Outputs:

- `combat_q_local_{train,val,test}.jsonl`
- `combat_q_local_summary.json`
- `q_local_metrics.json`
- `q_local_predictions.jsonl`
- `q_local_search_eval.json`
- `q_local_search_eval_samples.jsonl`
- `q_local_root_prior.jsonl`
- `q_local_root_prior_summary.json`
- `q_local_rust_search_eval.json`
- `q_local_rust_search_eval_samples.jsonl`
- `combat_q_local_torch_model.pt`

This path is intentionally:

- combat-only
- offline-first
- local-oracle driven
- search-first in evaluation

It is **not** a runtime policy path yet.

## Current Main Loop

`livecomm` is no longer the preferred engine for growing combat training data.

Use it for:

- protocol / engine canary runs
- discovering new real-state distributions
- final acceptance checks

Use the local offline loop for actual combat learning iteration:

1. `policy_seed_set` as the first offline preference teacher
2. `combat_lab/specs` as curriculum expansion
3. archived clean runs as real-state supplements, relabeled by offline oracle

The current pivot is broader than reranking but still offline-first:

- **combat transition / value / PPO-ready policy baselines**
- `combat_lab` and archived offline sources as the main learning loop
- macro hindsight rows for reward / shop / event
- no runtime learner control yet

## Workflow

### 1. Freeze a baseline

```powershell
cargo run --bin sts_dev_tool -- logs freeze-baseline `
  --out tools/artifacts/learning_baseline.json `
  --latest-runs 3
```

This writes a manifest that points at:

- validated livecomm runs
- archived `reward_audit`, `event_audit`, `combat_suspects`, `failure_snapshots`, and `validation` artifacts
- a small default set of `combat_lab` fixture specs
- a `known_noise` section so later experiments can see which recent runs were still tainted

### 1.5. Build the local combat source manifest

```powershell
python tools/learning/build_local_combat_training_sources.py
```

Outputs:

- `local_combat_source_manifest.json`
- `local_combat_seed_rows.jsonl`
- `local_combat_spec_rows.jsonl`
- `local_combat_archived_rows.jsonl`

This is the canonical inventory for the offline combat loop. It makes the
source split explicit:

- `policy_seed_set`
- `combat_lab_spec`
- `archived_clean_run`

### 2. Build datasets

```powershell
python tools/learning/build_sidecar_datasets.py `
  --baseline tools/artifacts/learning_baseline.json `
  --out-dir tools/artifacts/learning_dataset
```

Outputs:

- `reward_rows.jsonl`
- `event_rows.jsonl`
- `combat_rows.jsonl`
- `summary.json`

### 2.5. Convert policy seed sets into trainer-ready candidate rows

```powershell
python tools/learning/prepare_policy_seed_reranker_dataset.py
```

Outputs:

- `policy_seed_reranker_train.jsonl`
- `policy_seed_reranker_val.jsonl`
- `policy_seed_reranker_test.jsonl`
- `policy_seed_reranker_dataset_summary.json`

This is the first preferred teacher path for combat reranking.

It treats:

- `preferred_action` as the positive candidate
- `chosen_action` as the negative/reference candidate

To build a mixed dataset that appends the archived strong-only oracle rows:

```powershell
python tools/learning/prepare_policy_seed_reranker_dataset.py `
  --append-dataset-prefix combat_reranker_strong_only `
  --mixed-dataset-prefix seed_plus_strong_reranker
```

Outputs:

- `seed_plus_strong_reranker_train.jsonl`
- `seed_plus_strong_reranker_val.jsonl`
- `seed_plus_strong_reranker_test.jsonl`
- `seed_plus_strong_reranker_dataset_summary.json`

Use this to compare:

- `seed_only`
- `seed_plus_strong`

### 2.6. Generate more policy seed teachers from archived clean runs

```powershell
python tools/learning/export_archived_preference_seed_sets.py
```

Outputs:

- `generated_policy_seed_sets/policy_seed_set_<run_id>.jsonl`
- `generated_policy_seed_sets/policy_seed_set_<run_id>.summary.json`
- `generated_policy_seed_sets/generated_policy_seed_manifest.json`

This grows the offline teacher set **without** new `livecomm` sessions.

It:

- reads `learning_baseline.json`
- picks high-value combat suspect frames from archived clean runs
- calls `combat_decision_audit export-preference-seed-set`
- writes additional `policy_seed_set`-style teacher data

To merge the original teacher set with the generated archived teacher set:

```powershell
python tools/learning/prepare_policy_seed_reranker_dataset.py `
  --seed-glob "D:\rust\sts_simulator\data\combat_lab\policy_seed_set_*.jsonl,D:\rust\sts_simulator\tools\artifacts\learning_dataset\generated_policy_seed_sets\policy_seed_set_*.jsonl" `
  --dataset-prefix offline_teacher_reranker `
  --append-dataset-prefix combat_reranker_strong_only `
  --mixed-dataset-prefix offline_teacher_plus_strong_reranker
```

### 3. Review combat reranker rows

```powershell
python tools/learning/review_combat_reranker.py `
  --dataset-dir tools/artifacts/learning_dataset
```

Outputs:

- `combat_reranker_review.json`
- `combat_reranker_samples.jsonl`

This step is the current approved starting point for the first supervised
combat reranker. It does not train a model. It:

- checks that combat rows have stable top-k candidate context
- summarizes label strength, equivalent-best tie states, and conflict reasons
- writes a compact bundle of high-value rows for manual inspection before any
  offline training run
- can optionally summarize packed datasets by source:
  - `policy_seed_set`
  - `archived_clean_run`
  - mixed datasets

Example:

```powershell
python tools/learning/review_combat_reranker.py `
  --packed-prefixes policy_seed_reranker,combat_reranker_strong_only,seed_plus_strong_reranker
```

### 4. Oracle-label combat rows

```powershell
python tools/learning/oracle_label_combat_rows.py `
  --baseline tools/artifacts/learning_baseline.json `
  --dataset-dir tools/artifacts/learning_dataset `
  --mode fast `
  --quiet `
  --profile-out tools/artifacts/learning_dataset/oracle_labeled_combat_profile.json
```

Outputs:

- `oracle_labeled_combat_rows.jsonl`
- `oracle_labeled_combat_summary.json`
- optional `oracle_labeled_combat_profile.json`

This is the first approved stronger label path for combat:

- `livecomm` contributes the state rows
- offline `combat_decision_audit` contributes stronger battle-local labels
- the resulting oracle rows are the intended input to the first combat reranker

### 4.5. Export combat_lab curriculum rows

```powershell
python tools/learning/export_combat_lab_curriculum.py `
  --episodes 4 `
  --sample-limit-per-spec 4
```

Outputs:

- `combat_lab_curriculum_rows.jsonl`
- `combat_lab_curriculum_summary.json`

This is the preferred local curriculum expansion path.

It:

- runs `combat_lab` locally on `data/combat_lab/specs/*.json`
- keeps tagged high-value steps
- groups them by tactical curriculum theme such as:
  - `attack_over_defend`
  - `setup_before_payoff`
  - `potion_bridge`
  - `survival_override`
  - `status_exhaust_draw`

The exporter now keeps:

- normalized candidate move labels
- `top_candidate_move`
- `chosen_matches_top_candidate`

so the rows can feed an offline curriculum teacher packer instead of staying
review-only.

### 4.6. Pack combat_lab curriculum rows into a weak teacher dataset

```powershell
python tools/learning/prepare_combat_lab_curriculum_dataset.py `
  --append-dataset-prefix offline_teacher_plus_strong_reranker `
  --mixed-dataset-prefix offline_teacher_plus_curriculum_reranker
```

Outputs:

- `combat_lab_curriculum_reranker_train.jsonl`
- `combat_lab_curriculum_reranker_val.jsonl`
- `combat_lab_curriculum_reranker_test.jsonl`
- `combat_lab_curriculum_reranker_dataset_summary.json`
- optional mixed:
  - `offline_teacher_plus_curriculum_reranker_train.jsonl`
  - `offline_teacher_plus_curriculum_reranker_val.jsonl`
  - `offline_teacher_plus_curriculum_reranker_test.jsonl`
  - `offline_teacher_plus_curriculum_reranker_dataset_summary.json`

This dataset is intentionally weaker than archived oracle labels.

It only keeps curriculum rows where:

- there is a real multi-candidate choice
- a curriculum guardrail rule prefers a different move than the baseline

Current guardrails focus on the most tactical motifs:

- `kill_now_missed`
- `setup_flex_missed`
- `power_through_played_without_incoming`
- `survival_override_played_status_or_curse`

Use this to compare:

- `offline_teacher_plus_strong`
- `offline_teacher_plus_curriculum`

Modes:

- `--mode fast`
  - optimized for local iteration
  - uses lighter search defaults and may refine only disagreement-heavy rows
- `--mode slow`
  - optimized for higher-quality offline labeling
  - uses heavier search defaults and larger batch budgets

The script now prefers `target/release/combat_decision_audit(.exe)` automatically.
If no release binary exists, it falls back to `cargo run`.

## Truth Source Rules

The dataset builder deliberately follows the current project truth-source policy.

### Reward

Source:

- `reward_audit.jsonl`

Current row policy:

- only consume `kind=bot_reward_decision`
- keep the full reward breakdown already emitted by the live path:
  - `delta_prior`
  - `delta_bias`
  - `delta_rollout`
  - `delta_context`
  - `delta_context_rationale_key`
  - `delta_rule_context_summary`

### Event

Source:

- `event_audit.jsonl`

Current row policy:

- copy the structured family decision fields directly
- do not revive legacy option re-parsing exporters

### Combat

Source:

- `combat_suspects.jsonl`
- `failure_snapshots.jsonl` when present in the frozen baseline

Current row policy:

- keep sequencing / branch-opening / downside fields
- keep serialized root top-k candidates for reranker input
- keep heuristic-vs-search conflict labels
- attach snapshot-normalized state for high-risk or failed frames when a matching snapshot exists
- down-weight `tight_root_gap` rows instead of treating them as strong labels
- mark baseline choices as `baseline_weak` labels, not oracle truth

## Battle-Local Environment

The current approved battle-local training entrypoint is:

- `sts_simulator::testing::combat_env::CombatEnv`

It exposes:

- `reset(...)`
- `step(...)`
- `legal_actions()`
- `action_mask()`
- `observation()`

This is the environment to use for fixture replay, shadow evaluation, and PPO-ready
combat experiments. It does not cover map, reward, event, or shop flow.

## Filtering Rules

The builder excludes runs when any of these are true:

- `validation.status` is not `ok*`
- `trace_incomplete = true`
- `reward_loop_detected = true`
- `bootstrap_protocol_ok = false`
- run manifest reports `engine_bugs > 0`
- run manifest reports `replay_failures > 0`
- classification label contains `tainted`

This is stricter than the baseline freeze step on purpose. The baseline manifest is allowed
to record recent noisy runs; the dataset builder is the component that decides whether those
runs are acceptable as learning truth.

## Hybrid RL Pivot

The new offline-first combat learning loop adds simulator-outcomes datasets and
true PyTorch/SB3 training on top of the existing reranker pipeline.

### Combat transition / value / policy datasets

```powershell
python tools/learning/build_combat_rl_datasets.py
```

Outputs:

- `combat_transition_{train,val,test}.jsonl`
- `combat_value_{train,val,test}.jsonl`
- `combat_policy_{train,val,test}.jsonl`
- `combat_rl_dataset_summary.json`

This is the first simulator-outcomes path for the Hybrid RL pivot:

- transition rows come from `combat_lab` trace step state changes
- value rows use discounted and short-horizon rollout returns
- policy rows mix:
  - `combat_lab` trace warm-start rows
  - `policy_seed_set`
  - archived oracle policy supplements

### Transition / value / PPO-ready baselines

```powershell
python tools/learning/train_combat_transition_baseline.py
python tools/learning/train_combat_value_baseline.py
python tools/learning/train_combat_policy_baseline.py
```

Outputs:

- `transition_metrics.json`
- `value_metrics.json`
- `ppo_eval_metrics.json`

These are offline baselines for the new path:

- transition predicts next-state summaries and terminal flags
- value predicts return, survival, and kill signals
- policy is a PPO-ready control baseline over candidate actions

### PyTorch transition / value models

```powershell
.\.venv-rl\Scripts\python tools/learning/train_combat_transition_torch.py
.\.venv-rl\Scripts\python tools/learning/train_combat_value_torch.py
```

Outputs:

- `transition_torch_metrics.json`
- `value_torch_metrics.json`
- `combat_transition_torch_model.pt`
- `combat_value_torch_model.pt`

These are the first neural models for the Hybrid RL path:

- transition predicts next-state summary, reward, done, and terminal outcome
- value predicts discounted return, short-horizon return, survival, and kill probability

### Gymnasium bridge + true PPO

The PPO path now uses a dedicated Rust bridge binary:

- `target/release/combat_env_driver(.exe)`

It wraps `CombatEnv` over line-delimited JSON and is consumed by:

- `tools/learning/gym_combat_env.py`

Train PPO with:

```powershell
cargo build --release --bin combat_env_driver
.\.venv-rl\Scripts\python tools/learning/train_combat_ppo.py --timesteps 4096
```

Outputs:

- `combat_maskable_ppo_model.zip`
- `ppo_eval_metrics.json`
- `ppo_eval_episodes.jsonl`

This path is now a **real** PPO experiment:

- uses `Gymnasium`
- uses `sb3-contrib` `MaskablePPO`
- consumes the Rust `ActionMask`
- remains offline-first and fixed-spec only
- emits fixed tactical bucket metrics in `ppo_eval_metrics.json`
  - `attack_over_defend`
  - `setup_before_payoff`
  - `potion_bridge`
  - `survival_override`
  - `status_exhaust_draw`

### Macro hindsight rows

```powershell
python tools/learning/build_macro_hindsight_datasets.py
python tools/learning/evaluate_macro_hindsight_baselines.py
```

Outputs:

- `reward_hindsight_rows.jsonl`
- `shop_hindsight_rows.jsonl`
- `event_hindsight_rows.jsonl`
- `macro_hindsight_summary.json`
- `macro_hindsight_metrics.json`

These rows are future-window enriched archived clean-run samples for reward,
shop, and event analysis. They are not behavior-cloning labels and they do not
drive runtime policy.

### 5. Pack oracle-labeled combat rows for training

```powershell
python tools/learning/prepare_combat_reranker_dataset.py `
  --dataset-dir tools/artifacts/learning_dataset `
  --out-dir tools/artifacts/learning_dataset
```

Outputs:

- `combat_reranker_train.jsonl`
- `combat_reranker_val.jsonl`
- `combat_reranker_test.jsonl`
- `combat_reranker_dataset_summary.json`

This step converts frame-level oracle labels into candidate-level reranker examples.
Important rules:

- `livecomm` contributes state truth, not strong labels
- oracle rows are the main combat trainer label source
- `baseline_weak` stays in the packed dataset for analysis, but is not training-eligible

For the first-pass "stop being obviously stupid" subset, use:

```powershell
python tools/learning/prepare_combat_reranker_dataset.py `
  --dataset-dir tools/artifacts/learning_dataset `
  --out-dir tools/artifacts/learning_dataset `
  --dataset-prefix combat_reranker_strong_only `
  --only-label-strengths oracle_strong `
  --only-oracle-disagreements
```

That subset keeps only rows where:

- oracle labels are `oracle_strong`
- baseline chosen move is outside the oracle equivalent-best bucket

### 6. Train the lightweight baseline combat reranker

```powershell
python tools/learning/train_combat_reranker_baseline.py `
  --dataset-dir tools/artifacts/learning_dataset
```

Outputs:

- `combat_reranker_baseline_metrics.json`
- `combat_reranker_baseline_predictions.jsonl`
- `combat_reranker_baseline_review.json`

This is the primary completion target for the first offline combat reranker round.
It trains a lightweight candidate scorer for root top-k reranking.

To train on the strong-only subset:

```powershell
python tools/learning/train_combat_reranker_baseline.py `
  --dataset-dir tools/artifacts/learning_dataset `
  --dataset-prefix combat_reranker_strong_only `
  --output-prefix combat_reranker_strong_only_baseline
```

To train on the seed-only teacher path:

```powershell
python tools/learning/train_combat_reranker_baseline.py `
  --dataset-dir tools/artifacts/learning_dataset `
  --dataset-prefix policy_seed_reranker `
  --output-prefix seed_only_baseline
```

To train on the mixed offline-first path:

```powershell
python tools/learning/train_combat_reranker_baseline.py `
  --dataset-dir tools/artifacts/learning_dataset `
  --dataset-prefix seed_plus_strong_reranker `
  --output-prefix seed_plus_strong_baseline
```

### 7. Optionally run the small neural reranker entrypoint

```powershell
python tools/learning/train_combat_reranker_nn.py `
  --dataset-dir tools/artifacts/learning_dataset
```

Outputs:

- `combat_reranker_nn_metrics.json`
- `combat_reranker_nn_predictions.jsonl`
- `combat_reranker_nn_review.json`

This entrypoint reuses the exact same packed dataset and split. It is present to
validate that the current dataset contract already supports a small neural reranker.

Seed-only smoke:

```powershell
python tools/learning/train_combat_reranker_nn.py `
  --dataset-dir tools/artifacts/learning_dataset `
  --dataset-prefix policy_seed_reranker `
  --output-prefix seed_only_nn
```

Mixed smoke:

```powershell
python tools/learning/train_combat_reranker_nn.py `
  --dataset-dir tools/artifacts/learning_dataset `
  --dataset-prefix seed_plus_strong_reranker `
  --output-prefix seed_plus_strong_nn
```

## Near-Term Targets

- reward reranker
- event reranker
- combat root reranker
- combat pressure / downside sidecar

The intended first stronger label source for combat remains:

- `combat_decision_audit audit-frame-batch`

The intended first preferred teacher source for combat now also explicitly includes:

- `data/combat_lab/policy_seed_set_*.jsonl`
