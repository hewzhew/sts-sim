# Binary Entrypoints

`src/bin` is only for active command entrypoints. Library ownership belongs in
`src/ai`, `src/eval`, `src/testing`, or `src/state`; one-off experiments should
not grow new long-lived binaries here.

Active binaries:

- `combat_search_v2_driver`
  - `--start-spec <path>`: single whole-combat search report
  - `--combat-snapshot <path>`: single search report from an exact
    `CombatCaptureV1` position
  - `--benchmark-spec <path>`: bounded benchmark summary over start-spec or
    combat-snapshot cases
- `run_play_driver`
  - thin shell over the `eval::run_control` kernel
  - `capture <path> [label]`: save `CombatCaptureV1` only from active stable
    combat decision boundaries
  - `capture-case <benchmark_dir> <case_id> [label]`: write the standard
    `captures/<case_id>.capture.json`
  - `save-baseline-case <benchmark_dir> <case_id>`: write the last completed
    whole-combat `CombatBaselineOutcomeV1`
  - `bench-add <benchmark_dir> <case_id>`: register the capture/baseline pair
    in `benchmark.json`

Removed from the active binary surface:

- JSONL action-env drivers
- micro/toy RL environment drivers
- live/workbench/prompt controller entrypoints
