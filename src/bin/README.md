# Binary Entrypoints

`src/bin` is only for maintained command entrypoints. Library ownership belongs
in `src/ai`, `src/eval`, `src/testing`, `src/state`, or another library module.
One-off experiments should not grow long-lived binaries here.

Use [../../docs/RUNBOOK.md](../../docs/RUNBOOK.md) for command examples. This
file only records binary ownership and boundaries.

## Active Binaries

| Binary | Boundary |
| --- | --- |
| `branch_campaign_driver` | Rust-owned campaign application: run, inspect, dataset, continuation, and self-check commands. |
| `branch_tiny` | Lightweight owner-audit runner with run capsules, frontier continuation, and seed-panel diagnostics. |
| `combat_search_v2_driver` | Whole-combat search from start specs, combat captures, and benchmark suites. |
| `combat_case_review` | Review ladder for saved `CombatCase` artifacts from branch-tiny combat gaps; CLI owns IO, `combat_case_review/review_pipeline.rs` owns probe orchestration. |
| `run_play_driver` | Manual and semi-automatic REPL over `eval::run_control`: traces, bookmarks, captures, baselines, and interactive panels. |

## Ownership Rules

- `branch_campaign_driver` subcommands are the campaign application surface.
  Top-level compatibility flags may parse, but new tooling should call explicit
  subcommands.
- `branch_tiny` owner modules produce typed decisions. The runner applies those
  decisions without parsing rendered labels.
- `combat_search_v2_driver` and `combat_case_review` are combat investigation
  tools. They do not decide non-combat policy.
- `run_play_driver` is an interactive shell over the run-control kernel. Its
  detailed command help belongs in the binary and in `docs/RUNBOOK.md`, not in
  this map.

## File Boundaries

For large binaries, keep parsing, command selection, input conversion,
artifact IO, and dispatch separate:

- parsing/defaults belong near CLI args,
- command selection belongs near command dispatch,
- command handlers should take narrow input structs,
- artifact readers/writers should not own policy,
- `main.rs` should stay dispatch-oriented.

If a binary needs enough internal structure to require a second map, move the
shared logic into a library module and keep this README short.
