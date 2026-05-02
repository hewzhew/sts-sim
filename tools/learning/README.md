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
- production full-run RL control
- a second source of engine truth

## Setup

```powershell
python -m venv .venv-rl
.\.venv-rl\Scripts\python -m pip install -r tools/learning/requirements-hybrid-rl.txt
cargo build --release --bin combat_env_driver
cargo build --release --bin full_run_env_driver
```

Use `.\.venv-rl\Scripts\python.exe` for learning scripts. The system Python is
intentionally not treated as the RL environment. The default environment is
CPU-first. Treat CUDA as a separate install profile.

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

### Full-Run Environment Contract

The first full-run learning surface is intentionally only an environment
contract:

- Rust episode wrapper:
  - `sts_simulator::cli::full_run_smoke::FullRunEnv`
- Rust bridge:
  - `full_run_env_driver`
- Python bridge:
  - `tools/learning/full_run_env.py`
- smoke script:
  - `tools/learning/smoke_full_run_env.py`

This surface exists to prove offline full-run `reset/step/action_mask/done`
behavior before training. It is not yet a policy or a claim that PPO is the main
solution.

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

### Smoke-test the full-run Gym bridge

```powershell
cargo build --release --bin full_run_env_driver
.\.venv-rl\Scripts\python.exe tools/learning/smoke_full_run_env.py `
  --episodes 100 `
  --seed 14000 `
  --max-steps 5000
```

Expected contract-level failures are `crash_count`, `illegal_action_count`, or
`no_progress_count` greater than zero. A normal random policy should usually die;
the smoke only validates the bridge and action contract.

### Run a tiny full-run PPO sanity check

```powershell
.\.venv-rl\Scripts\python.exe tools/learning/train_full_run_maskable_ppo.py `
  --timesteps 2048 `
  --n-envs 2 `
  --eval-episodes 50 `
  --seed 30000 `
  --eval-seed 40000
```

This is only a bridge/training-loop sanity check. Treat its output as evidence
that `MaskablePPO` can consume the full-run environment and legal action mask,
not as a meaningful policy-strength result.

### Run a start-spec curriculum sweep

```powershell
.\.venv-rl\Scripts\python tools/learning/run_structured_start_spec_curriculum.py `
  --stages hexaghost_op_v1,hexaghost_v3,hexaghost_v2 `
  --timesteps 32768
```

This generates the synthetic strong Hexaghost start spec under ignored learning
artifacts, then runs separate structured PPO probes for each requested stage and
writes a compact summary JSON next to the per-stage metrics.

### Build a structured BC greedy-transition warmup

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

The first warmup dataset builder is intentionally local and conservative: it
replays each sampled prefix from the same start spec and seed, evaluates legal
root candidates with a short branch score, and writes structured observation
tensors plus the preferred multi-head action. These labels are policy warmup
priors, not engine truth. The historical script and CLI still use `teacher` in
some names, but this source is only a greedy one-step transition judge.

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
candidate set, pads it to a fixed candidate axis, marks all greedy-judge top ties,
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
observations with short greedy-transition continuation outcomes such as discounted return,
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
  --continuation-policy greedy_transition `
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
executed, then the greedy-transition policy continues for a short horizon; the model learns
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
  --continuation-policy greedy_transition `
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
of re-running expensive greedy-transition continuations while rejecting groups online:

```powershell
.\.venv-rl\Scripts\python tools/learning/filter_structured_candidate_value_dataset.py `
  --dataset tools/artifacts/learning_dataset/structured_candidate_value_hexaghost_v2_256.npz `
  --min-top2-gap 0.10 `
  --out tools/artifacts/learning_dataset/structured_candidate_value_hexaghost_v2_256_top2gap010.npz
```

The filtered rows renumber `sample_index` and `group_index`, while preserving
`source_sample_index` and `source_group_index` for provenance.

### Triage run-built state corpus rows

Fixed combat specs are useful for smoke tests and regressions, but they should
not be treated as the main training distribution. Full-run `live_comm` frames are
also not labels; they are only a more realistic candidate pool. Before spending
rollout/oracle budget, triage them into cheap buckets:

```powershell
cargo run --bin sts_dev_tool -- combat build-state-corpus `
  --label loss_clean `
  --latest-runs 5 `
  --limit-per-raw 64 `
  --depth 2 `
  --out tmp/run_state_corpus_loss_clean/state_corpus.jsonl `
  --summary-out tmp/run_state_corpus_loss_clean/state_corpus_summary.json

.\.venv-rl\Scripts\python tools/learning/triage_state_corpus.py `
  --input tmp/run_state_corpus_loss_clean/state_corpus.jsonl
```

The triage step writes:

- `*.triage.jsonl`: compact per-state features, tags, and routing decision
- `*.counterfactual_candidates.jsonl`: broad cheap-triage candidate set
- `*.oracle_needed.jsonl`: high-priority states worth expensive candidate counterfactuals
- `*.macro_backtrack.jsonl`: states that look already lost and need provenance
- `*.calibration_only.jsonl`: unstable rows useful for uncertainty, not top-1 labels
- `*.rejected_or_background.jsonl`: forced or low-signal states
- `*.triage.summary.json` / `*.triage.md`: coverage and sample-efficiency review

The report also loads each run's `manifest.json` and compares it with the
current repository state. If the run commit, binary provenance, or current dirty
state means the rows are stale, the report is marked `STALE` and its evidence
scope is `historical_replay_only`, not current-policy evidence.

By default `*.oracle_needed.jsonl` keeps only high-priority rows and caps each
run+encounter group at four rows, because consecutive combat frames are highly
correlated. Use `--oracle-min-priority medium` or `--max-oracle-per-encounter 0`
for broader offline review, but keep the defaults for expensive rollout labeling
until the yield is known.

This keeps the data contract explicit:

```text
live run state corpus -> cheap triage -> selective oracle -> hard candidate groups
```

Do not train action heads directly from raw `live_comm` bot choices. A
`live_snapshot` row solves only the "fake state distribution" problem; it does
not solve weak labels, low-decision states, or failures caused by earlier macro
choices.

### Audit oracle-needed live states

After triage, the next step is an audit loop, not a training dataset. The
consumer below takes `*.oracle_needed.jsonl`, runs `combat_decision_audit` on the
matching live replay frames, and asks whether the current bot action was clearly
bad under the configured local audit protocol:

```powershell
.\.venv-rl\Scripts\python tools/learning/audit_oracle_needed_states.py `
  --input tmp/run_state_corpus_loss_clean/state_corpus.oracle_needed.jsonl `
  --mode fast `
  --require-current
```

It writes:

- `*.audit.jsonl`: one row per audited state, with legal candidates, candidate
  outcomes, bot rank, best candidate, and recommendation
- `*.audit.md`: human review table
- `*.audit.summary.json`: counts for bot top-rank, clearly bad decisions,
  safety-rule candidates, and regression candidates
- `*.regression_cases.jsonl` / `*.safety_rule_cases.jsonl`: focused audit
  buckets for local replay/regression work

This script intentionally does not write `.npz` files and should not be used as
a training shortcut. The acceptance question is:

```text
Did the audit find real live-run states where the bot action is clearly worse,
and can those cases be turned into search/frontier regressions?
```

Rows where every candidate still dies are routed as macro-provenance candidates,
not local combat regressions, even if one dying line scores better than another.
In `--mode fast`, actionable findings are rerun with the slow audit budget by
default; pass `--no-refine-actionable` only when profiling the cheap pass itself.
Use `--require-current` for current-policy diagnosis. Without it, stale rows are
allowed only as historical replay audit and the report is marked accordingly.

### Build combat rescue decision groups

Failure-centered decision groups are generated from failed episodes by replaying
back to recent combat decision points and evaluating every legal root
candidate. One JSONL row is one same-state group, not one bad/rescue pair:

```powershell
.\.venv-rl\Scripts\python tools/learning/build_combat_rescue_counterfactual_dataset.py `
  --start-spec data/boss_validation/hexaghost_v2/start_spec.json `
  --seeds 2009,2010,2011,2012 `
  --episodes 4 `
  --state-policy random `
  --max-backtrack-steps 8 `
  --label-horizon 12 `
  --continuation-policy greedy_transition `
  --rescue-mode survival `
  --out tools/artifacts/learning_dataset/combat_rescue_decision_groups.jsonl
```

Each group records `public_observation`, `failed_action`, `candidate_outcomes`,
`rescue_candidate_indices`, `label_mode`, `continuation_policy`,
`intervention_depth`, and per-candidate `filter_reason`. The current labels use
`label_mode=fixed_seed_replay`, `continuation_policy=greedy_transition`, and a
`judge_protocol` such as `root_candidate_plus_greedy_transition_h12`: they are
counterfactual certificates for this replay protocol, not global optimal-action
labels.

The default `--rescue-mode survival` keeps only groups where the failed action
dies within the short horizon and at least one alternative survives it. Episodes
with no combat rescue groups are written to the `.macro_backtrack.jsonl`
manifest and marked as `needs_macro_backtrack`, which makes them candidates for
deeper rollback to card rewards, shops, rests, paths, or human review. Use
`--rescue-mode root_or_survival`, `return`, or `any` only for broader audit
corpora; those modes intentionally admit weaker rescue notions.

Combat rescue groups can be bucketed before training:

```powershell
.\.venv-rl\Scripts\python tools/learning/audit_combat_rescue_decision_groups.py `
  --groups tools/artifacts/learning_dataset/combat_rescue_decision_groups.jsonl
```

The audit writes a JSON summary, a Markdown review, tagged groups, and focused
JSONL buckets for `hard_survival`, `greedy_transition_bad`, `random_trivial`, and
`candidate_value_recommended`. The recommended bucket keeps hard survival groups
while holding out trivial random `EndTurn` failures so they can be capped
separately.

Candidate value prediction failures can be audited with:

```powershell
.\.venv-rl\Scripts\python tools/learning/audit_candidate_value_predictions.py `
  --rows tools/artifacts/learning_dataset/structured_candidate_value_hexaghost_v2_32.jsonl `
  --predictions tools/artifacts/learning_dataset/structured_candidate_value_hexaghost_v2_32_e80_predictions.jsonl `
  --split val `
  --out tools/artifacts/learning_dataset/structured_candidate_value_hexaghost_v2_32_audit.json
```

The audit groups predictions by root state, compares predicted top candidate
against the greedy-transition continuation target, and separates small-gap mistakes from
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
