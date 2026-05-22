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

Removed from the active binary surface:

- JSONL action-env drivers
- micro/toy RL environment drivers
- live/workbench/prompt controller entrypoints
