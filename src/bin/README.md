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
  - `--validate-only`: validate a start spec, capture, or benchmark suite
    without running search
- `run_play_driver`
  - thin shell over the `eval::run_control` kernel
  - starts in a game-like main screen; `deck`, `map`, `relics`, `potions`,
    `draw`, `discard`, `exhaust`, and `inspect <id>` open on-demand panels
  - `map` is read-only outside actual map navigation; `go <x>` is only valid
    after the current room is complete
  - `d` shows legacy details, `r` shows raw debug output, `h` shows the full
    categorized help
  - `--trace <path>`: write a `SessionTraceV1` diagnostic fact log for
    successful state-changing commands and benchmark artifact refs; omitted by
    default, with no implicit trace path
  - `case [path]`: save a diagnostic `RunDecisionCaseV1` with no teacher-label
    or policy-quality claim
  - `capture <path> [label]`: save `CombatCaptureV1` only from active stable
    combat decision boundaries
  - `capture-case <benchmark_dir> <case_id> [label]`: write the standard
    `captures/<case_id>.capture.json` and refresh `benchmark.json` with trust
    and expected fingerprints
  - `baseline`: after a `capture-case` and the matching combat finishes, save
    the last completed whole-combat baseline to that same case
  - `sc` / `search-combat [max_nodes=N] [wall_ms=N] [potion=never|all]`: run Combat
    Search V2 from the current active combat boundary, dry-run the selected
    complete winning trajectory, and apply it only if the dry-run still wins;
    budgeted wins are reported as no optimality claim
  - `n` / `next` / `advance-to-human-boundary [max_nodes=N] [wall_ms=N] [potion=never|all]`:
    advances routine or forced-safe screens, claims low-risk rewards, and uses
    combat search only when a complete winning trajectory is available, then
    stops at the next human strategic choice
  - reward screens auto-claim gold/stolen gold and only claim potion rewards
    when an empty potion slot exists; use `auto-reward gold|potion|all on|off`
    to inspect or change this convenience setting
  - `save-baseline-case <benchmark_dir> <case_id>`: write the last completed
    whole-combat `CombatBaselineOutcomeV1`; if the matching capture exists,
    refresh `benchmark.json`
  - `bench-add <benchmark_dir> <case_id>`: refresh a suite case from the saved
    capture and optional baseline

Removed from the active binary surface:

- JSONL action-env drivers
- micro/toy RL environment drivers
- live/workbench/prompt controller entrypoints
