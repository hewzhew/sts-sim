# Binary Entrypoints

`src/bin` is only for active command entrypoints. Library ownership belongs in
`src/ai`, `src/eval`, `src/testing`, or `src/state`; one-off experiments should
not grow new long-lived binaries here.

Active binaries:

- `artifact_doctor`
  - read-only audit over benchmark artifact directories
  - `--root <path>` scans for `benchmark.json` suites, registered captures,
    baselines, and search evidence links
  - `--output <path>` writes compact `ArtifactAuditReportV1` JSON with stable
    `check_id` plus artifact content hashes; stdout remains a short summary
  - no replay, no search execution, no artifact mutation, and no Markdown log
- `combat_search_v2_driver`
  - `--start-spec <path>`: single whole-combat search report
  - `--combat-snapshot <path>`: single search report from an exact
    `CombatCaptureV1` position
  - `--benchmark-spec <path>`: bounded benchmark summary over start-spec or
    combat-snapshot cases
  - `--validate-only`: validate a start spec, capture, or benchmark suite
    without running search
  - `--potion-policy all --max-potions-used <N>`: allow potion branches while
    bounding potion resource use for budgeted search experiments
  - `--benchmark-spec <path> --explain-case <case_id>`: diagnostic-only
    initial-decision microscope; reports the selected first action, current
    candidate ordering, and exact one-step consequences without writing
    artifacts or changing search policy
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
  - `cap <case_id> [label]`: short form for capturing current combat under
    `tools/artifacts/benchmarks/seed<seed>_act<act>`
  - `capture-case <benchmark_dir> <case_id> [label]`: write the standard
    `captures/<case_id>.capture.json` and refresh `benchmark.json` with trust
    and expected fingerprints
  - `b` / `baseline`: after a `capture-case` and the matching combat finishes, save
    the last completed whole-combat baseline to that same case
  - `sc` / `search-combat [max_nodes=N] [wall_ms=N] [max_hp_loss=N|off] [potion=never|all|semantic] [max_potions=N]`:
    run Combat Search V2 from the current active combat boundary, dry-run the
    selected complete winning candidate, and apply it only if the dry-run
    still wins; budget/frontier coverage is reported separately. Use
    `max_hp_loss=N` to inspect but refuse high-loss complete candidates.
    Start `run_play_driver` with `--search-max-hp-loss N` to make that the
    default for `sc`/`n`/`nr`; use command-local `max_hp_loss=off` to override.
    `potion=semantic max_potions=N` for semantic resource-bounded potion probes,
    or `potion=all max_potions=N` for a broader comparison.
  - `n` / `next` / `advance-to-human-boundary [max_nodes=N] [wall_ms=N] [max_hp_loss=N|off] [potion=never|all|semantic]`:
    advances routine or forced-safe screens, claims low-risk rewards, and uses
    combat search only when a complete winning candidate is available, then
    stops at the next human strategic choice
  - `n route=planner ...`: same guarded auto-step, but allows the route planner
    to choose map nodes; each route choice is tagged as
    `behavior_policy_not_teacher`
  - `nr` / `next-route`: short form for `n route=planner`; accepts the same
    search and budget options except an explicit `route=...`
  - `rs` / `route-suggest`: read-only route evidence; `rg` / `route-go`:
    execute one selected route planner move from the current map screen
  - `--auto-capture-combat [--auto-capture-combat-root <benchmark_dir>]`:
    automatically saves each new combat at the first stable player-turn
    boundary; with `--trace`, these captures are recorded as trace annotations
    and artifact refs
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
