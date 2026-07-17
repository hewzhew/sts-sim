# STS Simulator Tools

`tools/` is the offline tooling layer. Runtime behavior belongs in Rust binaries
and library modules; Python and PowerShell here should launch, inspect, export,
or analyze artifacts.

## Maintained Tool Groups

| Tool | Purpose |
| --- | --- |
| `path_review.py` | Render `branch_tiny` capsule paths with selected choices and candidate pools. |
| `success_feedback_panel.py` | Compare branch/capsule outcomes for feedback-oriented inspection. |
| `frozen_case_panel.py` | Run fixed combat/search cases for review panels. |
| `build_rl_dataset_manifest.py` | Build dataset manifests from exported artifacts. |
| `label_rl_outcomes.py` | Attach outcome labels to exported decision samples. |
| `analyze_imitation_disagreements.py` | Inspect imitation-model disagreements against behavior-policy samples. |
| `train_imitation_candidate_ranker.py` | Train the current offline candidate-ranker baseline. |
| `compiled_protocol_schema.json` | Generated protocol schema snapshot used by tooling. |

`tools/ml/` contains combat/search trace extraction and baseline scripts:

| Tool | Purpose |
| --- | --- |
| `combat_tactical_trace_extract.py` | Extract tactical combat traces. |
| `combat_first_action_ranking_baseline.py` | Baseline first-action ranking experiment. |
| `run_tactical_trace_extract.ps1` | Local trace-extraction launcher. |
| `run_tactical_trace_batch.ps1` | Batch trace-extraction launcher. |
| `run_turn_plan_baseline.ps1` | Turn-plan baseline launcher. |
| `run_turn_plan_policy_compare.ps1` | Turn-plan policy comparison launcher. |

## Output Rules

- generated reports and datasets belong under `tools/artifacts/`;
- root-level one-off snapshots belong under `tools/artifacts/root_snapshots/`;
- `__pycache__/`, generated panels, model outputs, and scratch data must stay
  ignored;
- long-lived schemas or tiny sample fixtures should be committed only when they
  are intentional interfaces.

## Branch And Panel Workflow

Use Rust binaries for normal run/panel work:

```powershell
cargo run -p sts_simulator_control --bin branch_tiny -- --seed 1552225673 --ascension 0 --max-branches 1 --wall-ms 60000
cargo run -p sts_simulator_control --bin branch_panel -- panel smoke --seeds 1552225671 1552225672 1552225673 --capsule-root tools/artifacts/panels/current --max-branches 1 --slice-ms 60000
```

The retired `tools/gap_panel.py` wrapper should not return. `branch_panel` is
the maintained panel entrypoint.

## Path Review Examples

```powershell
python tools\path_review.py target\gap-panel-candidate-pool-smoke2\1552225675 --boundary Shop --interesting --summary
python tools\path_review.py target\gap-panel-candidate-pool-smoke2\1552225675 --contains "purge reserve" --summary
python tools\path_review.py target\gap-panel-candidate-pool-smoke2\1552225675 --boundary Shop --inspect-summary
```
