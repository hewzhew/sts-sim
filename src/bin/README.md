# Binary Entrypoints

`src/bin` is only for maintained command entrypoints. Library ownership belongs
in `src/ai`, `src/eval`, `src/testing`, `src/state`, or another library module.
One-off experiments should not grow long-lived binaries here.

Use [../../docs/RUNBOOK.md](../../docs/RUNBOOK.md) for command examples. This
file only records binary ownership and boundaries.

## Active Binaries

| Binary | Boundary |
| --- | --- |
| `branch_tiny` | Lightweight owner-audit runner with run capsules, frontier continuation, seed-panel diagnostics, and combat-case capture. |
| `branch_panel` | Rust seed-panel scheduler for smoke/drain runs over multiple `branch_tiny` capsules. |
| `combat_case_review` | Review ladder for saved `CombatCase` artifacts from branch-tiny combat gaps; CLI owns IO, `combat_case_review/review_pipeline.rs` owns probe orchestration. |
| `combat_search_v2_driver` | Whole-combat search from start specs and captures, benchmark suites, the resumable Combat Laboratory V1, and the offline Campfire Threat Panel. |
| `run_play_driver` | Manual and semi-automatic REPL over `eval::run_control`: traces, bookmarks, captures, baselines, and interactive panels. |
| `rl_dataset_export` | Offline decision-sample export for imitation/RL experiments; exported labels are behavior-policy data, not truth. |

## Ownership Rules

- `branch_tiny` owner modules produce typed decisions. The runner applies those
  decisions without parsing rendered labels.
- `branch_panel` schedules and resumes `branch_tiny` capsules. It should not
  reinterpret owner policy or combat strategy.
- `combat_search_v2_driver` and `combat_case_review` are combat investigation
  tools. They do not decide non-combat policy. Combat Laboratory and Campfire
  Threat Panel artifacts are descriptive evidence and never feed live policy
  automatically.
- `run_play_driver` is an interactive shell over the run-control kernel. Its
  detailed command help belongs in the binary and in `docs/RUNBOOK.md`, not in
  this map.
- `rl_dataset_export` may read journals, summaries, and capsules, but it should
  not create policy conclusions that are not present in the source artifacts.

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
