# Learning Sidecar Workflow

This directory is the current home for offline learning experiments. It is active,
but still intentionally narrow.

What it is for:

- combat-only environment and bridge work
- offline dataset construction from approved truth sources
- baseline transition/value/policy experiments
- structured PPO probes
- local oracle and macro counterfactual reference experiments

What it is not for:

- runtime inference in `live_comm`
- full-run RL control
- a second source of engine truth

## Setup

```powershell
python -m venv .venv-rl
.\.venv-rl\Scripts\python -m pip install -r tools/learning/requirements-hybrid-rl.txt
cargo build --release --bin combat_env_driver
```

The default environment is CPU-first. Treat CUDA as a separate install profile.

## Current Learning Surfaces

### Structured Combat Contract

The cleanest current combat learning surface is the structured contract:

- Rust truth:
  - `sts_simulator::bot::harness::combat_env::CombatEnv`
- Rust bridge:
  - `combat_env_driver`
- Python bridge:
  - `tools/learning/structured_combat_env.py`
- policy and trainer:
  - `tools/learning/structured_policy.py`
  - `tools/learning/train_structured_combat_ppo.py`

This is the most explicit observation/action contract in the repo right now.

### Simulator-Outcome Baselines

Current baseline dataset and trainer stack:

- `build_combat_rl_datasets.py`
- `train_combat_transition_baseline.py`
- `train_combat_transition_torch.py`
- `train_combat_value_baseline.py`
- `train_combat_value_torch.py`
- `train_combat_policy_baseline.py`
- `train_combat_ppo.py`

This path is for offline control and modeling baselines, not live runtime policy.

### Local Oracle and Search-Prior Experiments

Current tactical local-oracle stack:

- `build_combat_local_oracle.py`
- `train_q_local_torch.py`
- `export_q_local_root_prior.py`
- `evaluate_q_local_search.py`
- `evaluate_q_local_rust_search.py`

Use this when the goal is to test small-horizon tactical priors before touching
runtime control.

### Macro Counterfactual Experiments

Current macro hindsight/counterfactual stack:

- `build_macro_counterfactual_dataset.py`
- `train_macro_counterfactual_choice_torch.py`
- `train_macro_counterfactual_choice_tower_torch.py`
- `train_macro_counterfactual_pairwise.py`
- `train_macro_counterfactual_xgb_ranker.py`

This is for reward/shop/event style choice analysis, not combat stepping.

## Truth Source Rules

Learning must stay downstream of current engine truth policy.

State truth comes from:

- validated archived `live_comm` runs
- checked-in protocol truth samples
- `combat_lab` / fixture specs
- Rust simulator truth from `CombatEnv`

Stronger labels come from:

- `combat_decision_audit`
- local oracle rollouts
- structured benchmarks and review scripts
- curated seed/curriculum datasets

Do not treat raw baseline bot actions from `live_comm` as oracle labels by default.

## Common Workflows

### Freeze a baseline from recent runs

```powershell
cargo run --bin sts_dev_tool -- logs freeze-baseline `
  --out tools/artifacts/learning_baseline.json `
  --latest-runs 3
```

### Build offline datasets

```powershell
.\.venv-rl\Scripts\python tools/learning/build_sidecar_datasets.py `
  --baseline tools/artifacts/learning_baseline.json `
  --out-dir tools/artifacts/learning_dataset
```

### Run the bridge-based PPO baseline

```powershell
cargo build --release --bin combat_env_driver
.\.venv-rl\Scripts\python tools/learning/train_combat_ppo.py --timesteps 4096
```

### Run the structured PPO experiment

```powershell
.\.venv-rl\Scripts\python tools/learning/train_structured_combat_ppo.py
```

### Run a start-spec curriculum sweep

```powershell
.\.venv-rl\Scripts\python tools/learning/run_structured_start_spec_curriculum.py `
  --stages hexaghost_op_v1,hexaghost_v3,hexaghost_v2 `
  --timesteps 32768
```

This generates the synthetic strong Hexaghost start spec under ignored learning
artifacts, then runs separate structured PPO probes for each requested stage and
writes a compact summary JSON next to the per-stage metrics.

### Build a structured BC teacher warmup

```powershell
.\.venv-rl\Scripts\python tools/learning/build_structured_bc_teacher_dataset.py `
  --start-spec data/boss_validation/hexaghost_v2/start_spec.json `
  --seeds 2009,2010,2011,2012,2013,2014,2015,2016 `
  --samples 128 `
  --out tools/artifacts/learning_dataset/structured_bc_teacher_hexaghost_v2_128.npz

.\.venv-rl\Scripts\python tools/learning/train_structured_combat_ppo.py `
  --spec-source start_spec `
  --start-spec data/boss_validation/hexaghost_v2/start_spec.json `
  --draw-order-variant reshuffle_draw `
  --enemy-hp-delta-scale 0.01 `
  --bc-dataset tools/artifacts/learning_dataset/structured_bc_teacher_hexaghost_v2_128.npz `
  --bc-warmup-epochs 5 `
  --timesteps 8192
```

The first teacher dataset builder is intentionally local and conservative: it
replays each sampled prefix from the same start spec and seed, evaluates legal
root candidates with a short branch score, and writes structured observation
tensors plus the preferred multi-head action. These labels are policy warmup
priors, not engine truth.

### Train a structured candidate ranker

```powershell
.\.venv-rl\Scripts\python tools/learning/build_structured_candidate_ranker_dataset.py `
  --start-spec data/boss_validation/hexaghost_v2/start_spec.json `
  --seeds 2009,2010,2011,2012,2013,2014,2015,2016 `
  --samples 128 `
  --out tools/artifacts/learning_dataset/structured_candidate_ranker_hexaghost_v2_128.npz

.\.venv-rl\Scripts\python tools/learning/train_structured_candidate_ranker.py `
  --dataset tools/artifacts/learning_dataset/structured_candidate_ranker_hexaghost_v2_128.npz `
  --start-spec data/boss_validation/hexaghost_v2/start_spec.json `
  --eval-seeds 2009,2010,2011,2012,2013,2014,2015,2016 `
  --epochs 60 `
  --batch-size 32 `
  --output-prefix structured_candidate_ranker_hexaghost_v2_128
```

This is the first quotient-action experiment. The dataset keeps the legal root
candidate set, pads it to a fixed candidate axis, marks all teacher-top ties,
and stores abstract action classes such as damage, mitigation, setup, draw, and
end turn. The ranker scores candidates from state tensors plus candidate
features instead of directly predicting a concrete hand slot. It is useful as an
offline prior/diagnostic surface; it is not yet a production combat policy.

### Train a structured state evaluator

```powershell
.\.venv-rl\Scripts\python tools/learning/build_structured_state_evaluator_dataset.py `
  --start-spec data/boss_validation/hexaghost_v2/start_spec.json `
  --seeds 2009,2010,2011,2012,2013,2014,2015,2016 `
  --samples 128 `
  --label-horizon 8 `
  --state-policy mixed `
  --out tools/artifacts/learning_dataset/structured_state_evaluator_hexaghost_v2_128.npz

.\.venv-rl\Scripts\python tools/learning/train_structured_state_evaluator.py `
  --dataset tools/artifacts/learning_dataset/structured_state_evaluator_hexaghost_v2_128.npz `
  --epochs 80 `
  --batch-size 32 `
  --output-prefix structured_state_evaluator_hexaghost_v2_128
```

This is the first structured value-learning probe. It labels structured
observations with short teacher-continuation outcomes such as discounted return,
HP delta, enemy HP progress, visible unblocked damage, survival, victory, and
defeat. The point is to test whether the encoder can learn state quality before
asking a policy head to choose exact card slots.

### Train a structured candidate value evaluator

```powershell
.\.venv-rl\Scripts\python tools/learning/build_structured_candidate_value_dataset.py `
  --start-spec data/boss_validation/hexaghost_v2/start_spec.json `
  --seeds 2009,2010,2011,2012 `
  --states 32 `
  --label-horizon 10 `
  --min-visible-unblocked 6 `
  --min-step-index 3 `
  --state-policy mixed `
  --out tools/artifacts/learning_dataset/structured_candidate_value_hexaghost_v2_32.npz

.\.venv-rl\Scripts\python tools/learning/train_structured_candidate_value_evaluator.py `
  --dataset tools/artifacts/learning_dataset/structured_candidate_value_hexaghost_v2_32.npz `
  --epochs 80 `
  --batch-size 32 `
  --rank-loss-coef 0.10 `
  --output-prefix structured_candidate_value_hexaghost_v2_32
```

This is the first candidate-after-state value probe. Each root candidate is
executed, then the teacher policy continues for a short horizon; the model learns
candidate outcome heads and evaluates candidate ranking within the same root
state group. This is slower to build than state-only value data, but it is much
closer to asking why one root choice is better than another. `--rank-loss-coef`
adds an optional listwise loss over candidates from the same root state; leave it
at `0.0` to reproduce pure pointwise value regression. Early Hexaghost probes
showed `0.10` preserving value calibration better than a more aggressive `0.25`,
while still improving root candidate ranking.

When the root ranking signal is too soft, the candidate value builder can keep
only harder groups before writing samples:

```powershell
.\.venv-rl\Scripts\python tools/learning/build_structured_candidate_value_dataset.py `
  --start-spec data/boss_validation/hexaghost_v2/start_spec.json `
  --seeds 2009,2010,2011,2012 `
  --states 32 `
  --label-horizon 10 `
  --min-visible-unblocked 6 `
  --min-step-index 3 `
  --min-top2-gap 0.10 `
  --out tools/artifacts/learning_dataset/structured_candidate_value_hexaghost_v2_hard32.npz
```

`--min-top2-gap`, `--min-return-range`,
`--require-survival-disagreement`, `--require-terminal-disagreement`, and
`--require-root-terminal-disagreement` are group-level filters. With the default
`--hard-group-match any`, a group is kept when any enabled criterion matches.
Use `--hard-group-match all` when building a narrow corpus that must satisfy
every enabled criterion. The summary JSON reports how many candidate groups were
considered, accepted, and rejected, plus accepted-group gap and disagreement
rates.

For larger experiments, prefer filtering an already-built broad dataset instead
of re-running expensive teacher continuations while rejecting groups online:

```powershell
.\.venv-rl\Scripts\python tools/learning/filter_structured_candidate_value_dataset.py `
  --dataset tools/artifacts/learning_dataset/structured_candidate_value_hexaghost_v2_256.npz `
  --min-top2-gap 0.10 `
  --out tools/artifacts/learning_dataset/structured_candidate_value_hexaghost_v2_256_top2gap010.npz
```

The filtered rows renumber `sample_index` and `group_index`, while preserving
`source_sample_index` and `source_group_index` for provenance.

Candidate value prediction failures can be audited with:

```powershell
.\.venv-rl\Scripts\python tools/learning/audit_candidate_value_predictions.py `
  --rows tools/artifacts/learning_dataset/structured_candidate_value_hexaghost_v2_32.jsonl `
  --predictions tools/artifacts/learning_dataset/structured_candidate_value_hexaghost_v2_32_e80_predictions.jsonl `
  --split val `
  --out tools/artifacts/learning_dataset/structured_candidate_value_hexaghost_v2_32_audit.json
```

The audit groups predictions by root state, compares predicted top candidate
against the teacher-continuation target, and separates small-gap mistakes from
large-gap failures. It also reports class confusion, true/predicted value deltas,
and worst mistake examples for deciding whether to improve labels, data balance,
or the evaluator architecture.

## Recommended Reading

- [../../docs/design/COMBAT_RL_CONTRACT_V0.md](../../docs/design/COMBAT_RL_CONTRACT_V0.md)
- [../../docs/design/LEARNING_TRUTH_SOURCES.md](../../docs/design/LEARNING_TRUTH_SOURCES.md)
- [../../docs/RL_READINESS_CHECKLIST.md](../../docs/RL_READINESS_CHECKLIST.md)

If you are adding a new learning experiment, make it explicit which of these it is:

- contract experiment
- dataset builder
- offline baseline
- evaluation probe

If it does not fit one of those buckets, it probably does not belong in the current stack.
