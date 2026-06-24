# Binary Entrypoints

`src/bin` is only for active command entrypoints. Library ownership belongs in
`src/ai`, `src/eval`, `src/testing`, or `src/state`; one-off experiments should
not grow new long-lived binaries here.

Active binaries:

- `branch_campaign_driver`
  - current automated branch-campaign engine used by `tools/campaign.ps1`
  - primary subcommands:
    - `run`: advance scheduled/parked campaign branches from a seed, report, and
      checkpoint
    - `inspect`: inspect checkpoint sessions, decks, route/shop/card/campfire
      evidence, combat lab packets, and final boss timelines
    - `dataset`: export or analyze branch/outcome/learning JSONL
    - `continue`: run targeted sibling continuation experiments
    - `self-check`: run internal replay/cache checks
  - legacy top-level flags remain temporarily parseable, but new tooling should
    call a subcommand explicitly; subcommand help is intentionally scoped to
    that command, while the top-level flattened flags are compatibility only
  - implementation boundary: `cli_args.rs` owns parsing, legacy `Args`, and
    preset/default application; `driver_command.rs` owns command selection;
    `command_inputs.rs` converts CLI args into narrow handler inputs plus
    campaign/search config; `campaign_artifacts.rs` owns report/checkpoint
    JSON IO; `main.rs` owns dispatch only
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
  - `--max-hp-loss <N|off>`: stop once an exact complete winning candidate
    with at most that hp loss is found; this is an acceptance gate for budgeted
    batch runs, not an exhaustive best-line claim. Benchmark cases with
    baselines keep searching normally so this early acceptance cannot create a
    baseline regression.
  - `--benchmark-spec <path> --explain-case <case_id>`: diagnostic-only
    initial-decision microscope; reports the selected first action, current
    candidate ordering, and exact one-step consequences without writing
    artifacts or changing search policy
- `auto_run_batch_driver`
  - diagnostic-only batch smoke for the current `run_control` auto-run chain
    across seed lists or contiguous seed ranges
  - this is not a separate bot; it reuses the same `RunControlSession` and
    `auto-run` command as `run_play_driver`, then reports the next human
    boundary
  - examples:
    - `auto_run_batch_driver --seed 521 --seed 590093712 --search-wall-ms 100`
    - `auto_run_batch_driver --seed-start 1000 --count 20 --json-lines`
    - `auto_run_batch_driver --seed 521 --prefix-script tools/tmp/opening.txt`
  - without a prefix script it will usually stop at Neow Bonus, because Neow
    remains an explicit strategy boundary
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
  - `--record`: convenience form for a new manual/auto-run session. It writes
    to a unique auto-named trace under `tools/artifacts/traces`, so the REPL can
    use `mark <name>` without typing a trace path.
  - `mark <name>` / `marks`: while trace recording is enabled, save or list a
    named replay bookmark in `tools/artifacts/traces/bookmarks.json`
  - `--goto <name>`: resume from a named bookmark. This automatically replays
    the bookmarked trace prefix, records a new continuation trace beside the
    source trace, and uses the bookmark name as the branch name. It should not
    be combined with `--trace`, `--replay-trace`, `--continue-trace`,
    `--branch`, or `--replay-steps`.
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
    Start `run_play_driver` with `--search-max-nodes N`, `--search-wall-ms N`,
    `--search-max-hp-loss N`, `--search-potion-policy semantic`, or
    `--search-max-potions-used N` to make those defaults for `sc`/`n`/`nr`;
    command-local `max_nodes=`, `wall_ms=`, `max_hp_loss=`, `potion=`, and
    `max_potions=` override them.
    Inside a running REPL, use `sd` / `search-defaults` to inspect or update
    the same session defaults, for example `sd max_hp_loss=8`,
    `sd max_nodes=500000 wall_ms=30000`, or `sd clear`.
    The default turn-plan policy is
    `tactical_enemy_turn_boundary_frontier_seed`: exact same-turn end states are
    seeded only for typed tactical multi-enemy fights; use
    `turn_plan=diagnostic_only` to disable that gate for one command, or
    `turn_plan=turn_boundary_frontier_seed` for explicit broad experiments.
    Use command-local `max_hp_loss=off` to disable the hp-loss gate once.
    `potion=semantic max_potions=N` for semantic resource-bounded potion probes,
    or `potion=all max_potions=N` for a broader comparison.
  - `n` / `next` / `advance-to-human-boundary [max_nodes=N] [wall_ms=N] [max_hp_loss=N|off] [potion=never|all|semantic]`:
    advances routine or forced-safe screens, claims low-risk rewards, and uses
    combat search only when a complete winning candidate is available, then
    stops at the next human strategic choice. Automated combat search uses the
    same default search strategy as `sc`, with only an interactive wall-clock
    default added when no session or command budget is set. Pass command-local
    `turn_plan=` or `frontier=` only for explicit experiments.
    When `max_hp_loss=N` is active, search may stop early after an exact
    complete win within that loss limit; this is a practical acceptance gate,
    not an optimality claim.
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
  - reward screens auto-claim gold/stolen gold, safe relic rewards without a
    same-screen `SapphireKey`, and potion rewards only when an empty potion slot
    exists; use `auto-reward gold|potion|relic|all on|off` to inspect or change
    this convenience setting
  - `save-baseline-case <benchmark_dir> <case_id>`: write the last completed
    whole-combat `CombatBaselineOutcomeV1`; if the matching capture exists,
    refresh `benchmark.json`
  - `bench-add <benchmark_dir> <case_id>`: refresh a suite case from the saved
    capture and optional baseline

Experimental or legacy binaries:

- `branch_experiment_driver`
  - standalone branch experiment microscope from a seed, bookmark, prefix, or
    replay trace
  - current campaign work should prefer `branch_campaign_driver` checkpoint
    inspection or challenge modes unless a focused local experiment is needed

Removed from the active binary surface:

- JSONL action-env drivers
- micro/toy RL environment drivers
- live/workbench/prompt controller entrypoints
- `decision_lab_driver`, now archived under
  `tools/_dormant/rust_bins/decision_lab_driver`
- `card_reward_value_loop_driver`, now archived under
  `tools/_dormant/rust_bins/card_reward_value_loop_driver`
