# Binary Entrypoints

`src/bin` is only for active command entrypoints. Library ownership belongs in
`src/ai`, `src/eval`, `src/testing`, or `src/state`; one-off experiments should
not grow new long-lived binaries here.

Active binaries:

- `combat_search_v2_driver`
  - `--start-spec <path>`: single whole-combat search report
  - `--benchmark-spec <path>`: bounded multi-start-spec benchmark summary

Removed from the active binary surface:

- JSONL action-env drivers
- micro/toy RL environment drivers
- live/workbench/prompt controller entrypoints
