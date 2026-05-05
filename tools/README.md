# STS Simulator Tools

`tools/` is now treated as the offline tooling layer, not as an extension of the runtime code.

## Directory Map

- `analysis/`
  - cache-first Java and parity analysis scripts
- `analysis_cache/`
  - machine-readable cached truth used by renderers and audits
- `artifacts/`
  - generated reports, datasets, coverage outputs, and other derived files
- `combat_lab/`
  - local batch helpers for combat-lab style experiments and policy A/B runs
- `coverage/`
  - coverage dashboard and parsers
- `learning/`
  - RL / dataset build scripts and learning-side utilities
- `legacy/`
  - retained old scripts and retired implementation snapshots
- `live_comm/`
  - launch scripts, profiles, and operational helpers
- `manual/`
  - hand-run helper scripts
- `rust_ast_extractor/`
  - Rust AST extraction helper crate
- `schema_builder/`
  - schema generation and comparison helpers
- `source_extractor/`
  - broader Java source report generation
- `sts_tool/`
  - primary structured analysis CLI

## Output Rules

- generated reports and datasets belong under `tools/artifacts/`
- cache files belong under `tools/analysis_cache/`
- live replay captures belong under `logs/replays/` or `logs/runs/`
- loose live-comm captures do not belong in the repo root; they now live under `logs/live_comm/`
- root-level one-off snapshots such as `coverage.json` or `ledger.jsonl` belong under `tools/artifacts/root_snapshots/`

## Primary Workflow

The Java analysis toolchain is cache-first:

```powershell
cd tools
python -m sts_tool cache
python -m sts_tool query ApplyPower
python -m sts_tool query ApplyPower --json
python -m sts_tool find Corruption
python -m sts_tool overrides onApplyPower
python -m sts_tool family power_lifecycle
python -m sts_tool inspect ApplyPower --method update
python hook_query.py onApplyPower
```

`analysis_cache/*.json` is the canonical machine-readable truth layer. Markdown reports are renderers over that cache.

## Active Tools

### `run_high_value_tests.ps1`

Default entrypoint for the current high-value correctness suite.

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\run_high_value_tests.ps1
powershell -ExecutionPolicy Bypass -File .\tools\run_high_value_tests.ps1 -IncludeParity
```

Runs:

- `protocol_truth_samples`
- `state_sync_strictness`
- `guardian_threshold_behavior`
- `stasis_behavior`
- optional `live_comm_replay_driver` when `-IncludeParity` is set

### `sts_tool/`

Primary entrypoint for Java→Rust analysis.

Commands:
- `cache`
- `query`
- `find`
- `overrides`
- `family`
- `inspect`

### `cargo run --bin sts_dev_tool -- logs analyze-decisions`

Lightweight strategy-experiment report over `live_comm` run logs. It clusters
`exact_turn` disagreements and idle `EndTurn` decisions from `debug.txt`
without needing a new live run.

```powershell
cargo run --bin sts_dev_tool -- logs analyze-decisions 20260421_145156
cargo run --bin sts_dev_tool -- logs analyze-decisions --label loss_clean --json-out tools/artifacts/decision_experiment.json
```

### `cargo run --bin sts_dev_tool -- logs export-disagreement-fixtures`

Exports real `live_comm` disagreement frames into `ScenarioFixture` files that
`combat_lab` can consume via `--fixture`. This lets local A/B runs target actual
bad frames from a run instead of hand-authored representative specs.

```powershell
cargo run --bin sts_dev_tool -- logs export-disagreement-fixtures 20260421_145156 `
  --categories high_threat_exact_disagree_not_taken,idle_energy_end_turn `
  --limit 6 `
  --out-dir tmp/live_comm_disagreement_fixtures `
  --json-out tmp/live_comm_disagreement_fixtures/export_report.json
```

### `cargo run --bin sts_dev_tool -- logs export-decision-training-set`

Builds a JSONL dataset from exported disagreement fixtures. Each record carries:
- observed command text
- current bot choice
- exact-turn preferred choice
- a transitional `preferred_action_source` for backward compatibility
- frame-level auxiliary targets such as:
  - `needs_exact_trigger_target`
  - `has_high_threat_target`
  - `has_strict_disagreement_target`
  - `frontier_self_consistent_target`
- top root moves
- `decision_trace`, `root_pipeline`, and `exact_turn_verdict`

This is the first learning-friendly data path: real bad frames, not hand-authored
surrogate specs. The intended downstream use is no longer "predict one preferred
action", but support tasks such as:
- proposal prior / pruning
- screened-out veto prediction
- exact-trigger / regime classification

When archived live combat shadows are available, the exporter prefers the live
`decision_audit` over fixture re-run diagnostics. This preserves `screened_out`
and other proposal-level evidence that can be lost when replaying a fixture
offline under slightly different runtime conditions.

```powershell
cargo run --bin sts_dev_tool -- logs export-decision-training-set `
  --fixture-dirs tmp/live_comm_disagreement_fixtures_145156,tmp/live_comm_disagreement_fixtures_145156_idle,tmp/live_comm_disagreement_fixtures_145156_strict `
  --out tmp/live_comm_frame_ab_145156/decision_training.jsonl `
  --summary-out tmp/live_comm_frame_ab_145156/decision_training_summary.json `
  --proposal-out tmp/live_comm_frame_ab_145156/proposal_training.jsonl `
  --proposal-summary-out tmp/live_comm_frame_ab_145156/proposal_training_summary.json `
  --depth 6
```

### `cargo run --bin sts_dev_tool -- logs build-decision-corpus`

Batch wrapper that scans multiple `live_comm` runs, exports real disagreement
fixtures per run, and writes one combined frame-level and proposal-level corpus.
Use this instead of manually chaining:
- `analyze-decisions`
- `export-disagreement-fixtures`
- `export-decision-training-set`

```powershell
cargo run --bin sts_dev_tool -- logs build-decision-corpus `
  --label loss_clean `
  --latest-runs 5 `
  --categories high_threat_exact_disagree_not_taken,idle_energy_end_turn,strict_better_same_survival,survival_upgrade_not_taken `
  --limit-per-run 8 `
  --out-dir tmp/decision_corpus_loss_clean `
  --depth 6
```

Outputs:
- `fixtures/<run_id>/*.fixture.json`
- `decision_training.jsonl`
- `proposal_training.jsonl`
- `decision_training_summary.json`
- `proposal_training_summary.json`
- `corpus_summary.json`

### `cargo run --bin sts_dev_tool -- combat build-state-corpus`

Build a **state-level combat corpus** from existing `ScenarioFixture`s,
`CombatCase`s, and/or raw `live_comm` logs. This is the preferred data path for
state-centric curriculum work when full-run trajectories are too weak or too
biased by early deaths.

Each JSONL record contains:
- source metadata (`source_kind`, `source_path`, `run_id`, `response_id`, `frame_id`)
- a compact `combat_snapshot`
- engine/screen metadata (`engine_state`, `screen_type`, `player_class`, `ascension_level`)
- curriculum-facing bucket tags (`curriculum_buckets`), currently including:
  - `elite`
  - `boss`
  - `regime_crisis`
  - `regime_fragile`
  - `status_heavy`
  - `setup_window`
- lightweight decision-probe fields from `diagnose_root_search_with_depth_and_runtime`
  - `regime`
  - `legal_moves`
  - `reduced_legal_moves`
  - `needs_exact_trigger_target`
  - `screened_out_count`
  - `decision_audit`

Notes:
- `--fixtures` expects real `ScenarioFixture` JSON files, not authored
  `combat_lab` specs.
- `--combat-cases` expects `.case.json` files.
- raw snapshots are only included when you explicitly pass `--raw`, `--run-ids`,
  or `--label`.
- `--include-buckets` keeps states matching **any** listed bucket.
- `--exclude-buckets` drops states matching **any** listed bucket.
- the builder filters terminal states (`GAME_OVER`, dead player, no living
  monsters) and deduplicates equivalent `run_id + response_id + frame_id`
  samples, preferring `combat_case > scenario_fixture > live_snapshot`

```powershell
cargo run --bin sts_dev_tool -- combat build-state-corpus `
  --fixtures tmp/decision_corpus_213431/fixtures/20260421_213431/20260421_213431_f47_high_threat_exact_disagree_not_taken.fixture.json `
  --combat-cases tests/combat_cases/lagavulin_metallicize.case.json `
  --run-ids 20260421_213431 `
  --limit-per-raw 4 `
  --depth 4 `
  --out tmp/state_corpus_smoke/state_corpus.jsonl `
  --summary-out tmp/state_corpus_smoke/state_corpus_summary.json
```

Example bucketed slice:

```powershell
cargo run --bin sts_dev_tool -- combat build-state-corpus `
  --fixture-dirs tmp/decision_corpus_213431/fixtures/20260421_213431 `
  --combat-case-dirs tests/combat_cases `
  --run-ids 20260421_213431 `
  --limit-per-raw 12 `
  --depth 4 `
  --include-buckets elite,setup_window `
  --exclude-buckets regime_crisis `
  --out tmp/state_corpus_trial_213431_bucketed/state_corpus.jsonl `
  --summary-out tmp/state_corpus_trial_213431_bucketed/state_corpus_summary.json
```

Outputs:
- `state_corpus.jsonl`
- `state_corpus_summary.json`

### `cargo run --bin sts_dev_tool -- combat split-state-corpus`

Split an existing `state_corpus.jsonl` into deterministic `train/val/test`
partitions. Records are grouped before splitting so related states stay
together; the current grouping key prefers:
- `combat_case_id`
- otherwise `run_id + encounter_signature`
- otherwise `fixture_name`

The command can also re-apply bucket filters at split time.

```powershell
cargo run --bin sts_dev_tool -- combat split-state-corpus `
  --input tmp/state_corpus_trial_213431_bucketed/state_corpus.jsonl `
  --out-dir tmp/state_corpus_trial_213431_bucketed/split `
  --include-buckets elite,setup_window `
  --exclude-buckets regime_crisis `
  --train-pct 80 `
  --val-pct 10
```

Outputs:
- `train.jsonl`
- `val.jsonl`
- `test.jsonl`
- `split_summary.json`

Note:
- on small corpora, a split can legitimately end up with no `val` examples if no
  groups hash into that bucket.

### `python tools/learning/train_state_corpus_aux_baseline.py`

Train the first **state-level auxiliary baseline** over a split state corpus.
This baseline does **not** predict final combat actions. It only learns:
- `needs_exact_trigger_target`
- `regime`

If one target is single-class in the train split, the trainer now skips just
that target instead of aborting the whole run. It only hard-fails when **all
requested** targets are unsupported.

Use it as:
- a signal check for the new state-centric corpus
- a first offline probe for trigger/regime learning
- a warm-start surface before any larger online or RL integration

```powershell
python tools/learning/train_state_corpus_aux_baseline.py `
  --split-dir tmp/state_corpus_trial_213431_bucketed/split
```

Request just one auxiliary target:

```powershell
python tools/learning/train_state_corpus_aux_baseline.py `
  --split-dir tmp/state_corpus_bundle_elite_setup_preserve/split `
  --targets trigger

python tools/learning/train_state_corpus_aux_baseline.py `
  --split-dir tmp/state_corpus_bundle_fragile_boss_preserve/split `
  --targets regime
```

Outputs:
- `<prefix>_metrics.json`
- `<prefix>_predictions.jsonl`

The metrics payload includes:
- trigger/regime train/val/test metrics
- class coverage
- top positive/negative logistic feature weights for both tasks

### `tools/learning/build_state_corpus_bundle.ps1`

Convenience wrapper for the new state-centric pipeline. It:
- discovers existing disagreement `ScenarioFixture` directories under `tmp/`
- builds one combined `state_corpus.jsonl`
- splits it into deterministic `train/val/test`
- optionally runs the auxiliary trigger/regime baseline
- writes `split/aux_training_preflight.json` with train-split label coverage for:
  - `needs_exact_trigger_target`
  - `regime`
- skips auxiliary training entirely when the train split supports none of the
  requested targets

By default it uses:
- fixture dirs from `tmp/decision_corpus_*/fixtures/*`
- fixture dirs from `tmp/live_comm_disagreement_fixtures*`
- combat cases from `tests/combat_cases`

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\learning\build_state_corpus_bundle.ps1 `
  -OutDir tmp/state_corpus_bundle `
  -TrainAuxBaseline
```

Optional bucket filtering is forwarded through to both build and split:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\learning\build_state_corpus_bundle.ps1 `
  -OutDir tmp/state_corpus_bundle_elite `
  -IncludeBuckets elite,setup_window `
  -ExcludeBuckets regime_crisis
```

If a bucketed bundle would otherwise lose all trigger-negative rows, you can ask
the split step to preserve a small negative reservoir from outside the include
buckets:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\learning\build_state_corpus_bundle.ps1 `
  -OutDir tmp/state_corpus_bundle_fragile_status `
  -IncludeBuckets regime_fragile,status_heavy `
  -PreserveTriggerNegativeRows 8 `
  -TrainAuxBaseline
```

In preserve mode, the build step keeps the corpus broad and the split step adds
up to the requested number of trigger-negative rows that missed the include
buckets but did not hit excluded buckets.

Request only one auxiliary target from the bundle wrapper:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\learning\build_state_corpus_bundle.ps1 `
  -OutDir tmp/state_corpus_bundle_trigger_only `
  -IncludeBuckets elite,setup_window `
  -PreserveTriggerNegativeRows 8 `
  -AuxTargets needs_exact_trigger_target `
  -TrainAuxBaseline
```

### `tools/learning/build_trigger_bundle.ps1`

Trigger-focused convenience wrapper over `build_state_corpus_bundle.ps1`.
Defaults:
- `IncludeBuckets = elite,setup_window`
- `PreserveTriggerNegativeRows = 8`
- `AuxTargets = needs_exact_trigger_target`
- `TrainAuxBaseline = true`

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\learning\build_trigger_bundle.ps1 `
  -OutDir tmp/state_corpus_bundle_trigger
```

### `tools/learning/build_regime_bundle.ps1`

Regime-focused convenience wrapper over `build_state_corpus_bundle.ps1`.
Defaults:
- `IncludeBuckets = regime_fragile,boss`
- `PreserveTriggerNegativeRows = 8`
- `AuxTargets = regime`
- `TrainAuxBaseline = true`

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\learning\build_regime_bundle.ps1 `
  -OutDir tmp/state_corpus_bundle_regime
```

### `tools/learning/evaluate_aux_bundle_suite.ps1`

Run the trigger-focused and regime-focused bundle entrypoints together and write
one stable scoreboard artifact. This is the preferred offline evaluation entry
for the current auxiliary-learning phase.

It produces:
- `trigger/` bundle outputs
- `regime/` bundle outputs
- `aux_bundle_suite_summary.json`

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\learning\evaluate_aux_bundle_suite.ps1 `
  -OutDir tmp/aux_bundle_suite
```

The suite summary records:
- requested and supported targets per bundle
- split counts and trigger label coverage
- preserved negative-row counts
- held-out trigger/regime metrics

### `tools/combat_lab/run_policy_compare.ps1`

Fixed-seed local A/B runner for `combat_lab` policy variants. By default it now
compares:
- `heuristic`
- `bot`
- `bot_contested_takeover`
- `bot_no_idle_end_turn`
- `bot_combined`

and writes a single `comparison.json` with per-variant deltas against baseline
`bot`.

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\combat_lab\run_policy_compare.ps1 `
  -AuthorSpec data/combat_lab/specs/jaw_worm_opening.json `
  -Episodes 20 `
  -Depth 6 `
  -BaseSeed 1 `
  -OutDir tmp/combat_lab_compare
```

It can also compare directly against an exported live disagreement fixture:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\combat_lab\run_policy_compare.ps1 `
  -Fixture tmp/live_comm_disagreement_fixtures/20260421_145156_f307_high_threat_exact_disagree_not_taken.fixture.json `
  -Episodes 20 `
  -Depth 6 `
  -BaseSeed 1 `
  -OutDir tmp/combat_lab_compare_from_fixture
```

### `tools/combat_lab/run_decision_triplet.ps1`

Small targeted experiment set for the current chooser questions. By default it
runs these 3 specs:
- `spot_weakness_attack_intent_window`
- `survival_override_guardrail`
- `power_through_not_on_lagavulin_debuff_turn`

and writes one combined `decision_triplet_summary.json`.

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\combat_lab\run_decision_triplet.ps1 `
  -Episodes 20 `
  -Depth 6 `
  -BaseSeed 1 `
  -OutDir tmp/combat_lab_decision_triplet
```

### `hook_query.py`

Thin wrapper that renders a focused hook report from the shared cache.

### `analysis.quick_smoke` / `analysis.full_smoke`

Validation tiers for the cache-first workflow.

```powershell
python -m analysis.quick_smoke
python -m analysis.full_smoke
```

### `analysis.live_regression`

Live log extraction and minimization for `live_comm` fixtures.

### `analysis.bugfix_workflow`

Opinionated wrapper over `analysis.live_regression` for parity bug work.

### `learning/`

Dataset-build and learning-side workflow for future RL work.

## Canonical Artifacts

Machine-readable:
- `analysis_cache/java_entities.json`
- `analysis_cache/java_methods.json`
- `analysis_cache/java_hooks.json`
- `analysis_cache/java_callsites.json`
- `analysis_cache/rust_dispatch.json`
- `analysis_cache/schema_aliases.json`
- `analysis_cache/manifest.json`
- `compiled_protocol_schema.json`

Rendered:
- `analysis_cache/family_audit/<family>.json`
- `analysis_cache/family_audit/<family>.md`
- `artifacts/hook_query_output/<hook>.md`
- `artifacts/coverage_report.html`

## Legacy

`source_extractor/` remains available for broad report rendering and compatibility checks, but it is not the preferred first stop when cache-backed analysis exists.
