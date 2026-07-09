# sts_simulator

[中文说明](README.zh-CN.md)

`sts_simulator` is an unofficial Rust simulator and AI-search workspace for
Slay the Spire.

The project is currently a research and automation codebase, not a polished
library crate. Its main goal is to make simulator state, run decisions, combat
search, and experiment artifacts explicit enough that failures can be replayed
and improved instead of explained from terminal logs.

## Current Focus

```text
typed simulator state
  -> typed non-combat owners and deck mutation bridges
  -> branch-tiny run capsules and seed panels
  -> combat cases for search review
  -> offline datasets and diagnostics when useful
```

The active direction is to keep strategy, execution, and diagnostics separate:

- owners choose typed non-combat decisions;
- runtime applies those decisions without parsing display text;
- combat search only solves combat;
- panels and review tools expose evidence, not teacher labels.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the maintained boundary
contract.

## Quick Start

Run one owner-audit seed:

```powershell
cd D:\rust\sts_simulator
cargo run --bin branch_tiny -- --seed 1552225673 --ascension 0 --max-branches 1 --wall-ms 60000
```

Run a small seed panel:

```powershell
cargo run --bin branch_panel -- panel smoke --seeds 1552225671 1552225672 1552225673 1552225674 1552225675 --capsule-root tools/artifacts/panels/current --max-branches 1 --slice-ms 60000
```

Review a saved combat case:

```powershell
cargo run --bin combat_case_review -- --case <case.json> --ladder
```

Use [docs/RUNBOOK.md](docs/RUNBOOK.md) for maintained commands, continuation
examples, combat search drivers, manual REPL usage, and verification.

## Main Entrypoints

| Binary | Purpose |
| --- | --- |
| `branch_tiny` | lightweight run runner for owner coverage, run capsules, frontier continuation, and combat-case capture |
| `branch_panel` | Rust seed-panel scheduler for smoke/drain runs across several seeds |
| `combat_case_review` | diagnostic review ladder for saved combat cases |
| `combat_search_v2_driver` | fixed combat search from start specs, captures, or benchmark suites |
| `run_play_driver` | manual and semi-automatic simulator REPL |
| `branch_campaign_driver` | older Rust campaign application surface for campaign artifacts and continuation experiments |
| `rl_dataset_export` | offline decision-sample export for imitation/RL experiments |
| `decision_records` | decision-record inspection utility |

See [src/bin/README.md](src/bin/README.md) for binary ownership boundaries.

## Documentation Map

- [docs/README.md](docs/README.md): current documentation index.
- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): ownership boundaries and design
  rules.
- [docs/RUNBOOK.md](docs/RUNBOOK.md): maintained local commands.
- [docs/TESTING.md](docs/TESTING.md): test ownership and cleanup standards.
- [tools/README.md](tools/README.md): offline tool boundaries and artifact
  rules.
- [src/ai/README.md](src/ai/README.md): AI module map and cleanup direction.

Retired docs are not kept searchable in the working tree. Use git history for
archaeology.

## Repository Layout

| Directory | Role |
| --- | --- |
| `src/content` | Java-game content reimplementation; avoid casual churn |
| `src/state` | run, combat, map, event, reward, and engine state |
| `src/engine` | state transitions and action handlers |
| `src/runtime` | runtime support for run/combat execution |
| `src/sim` | simulator-facing legal action and apply/search boundaries |
| `src/ai` | policies, strategic facts, deck mutation, combat search, route/search work |
| `src/eval` | run-control, benchmark artifacts, diagnostics, reports |
| `src/bin` | maintained command entrypoints |
| `tools` | offline scripts, datasets, panels, and generated artifacts |
| `docs` | maintained architecture, runbook, testing notes, and current drafts |

Generated outputs belong under ignored locations such as `target/` and
`tools/artifacts/`.

## Development Hygiene

The repository stores source, docs, and PowerShell scripts with LF line endings.
After mechanical edits on Windows, check that a small source change did not
become a whole-file CRLF rewrite:

```powershell
git diff --stat
git diff --ignore-space-at-eol --stat
git ls-files --eol $(git diff --name-only)
```

Prefer small commits with honest names. Do not preserve duplicate policy modules
only because migration is uncomfortable; when a boundary is ready, delete the
old entrypoint instead of keeping a compatibility layer.

## Verification

For documentation-only changes:

```powershell
git diff --check
```

For core code changes, start from the commands in
[docs/RUNBOOK.md](docs/RUNBOOK.md). Run targeted tests only when the changed
surface has a stable structural contract worth protecting.

## License and Game Notice

No license has been declared yet.

This is an unofficial research project. Slay the Spire is developed by Mega
Crit; this repository is not affiliated with or endorsed by Mega Crit.
