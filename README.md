# sts_simulator

[中文说明](README.zh-CN.md)

`sts_simulator` is an unofficial Rust simulator and AI-search workspace for
Slay the Spire.

Current main line:

```text
simulator -> state representation -> search/rollout -> value -> policy improvement
```

The project is not currently focused on old watch UI, Workbench,
DecisionFrame, prompt engineering, or an LLM-driven controller. Those may return
later as adapters, but they do not define simulator truth or search quality.

## Current Workflow

The maintained loop is:

1. run a deterministic simulator campaign from Neow onward
2. keep several noncombat branches alive under explicit budgets
3. use Combat Search V2 for complete combat trajectories inside those branches
4. inspect checkpoints, final boss combats, and outcome datasets when a branch fails
5. compare whole-run outcomes and branch siblings, not step-by-step action agreement

Autopilot, route planning, card reward policy, traces, and search-assisted
combat are convenience/evidence tools. They are not teacher labels.

## Quick Start

Run the current campaign workflow:

```powershell
cd D:\rust\sts_simulator
.\tools\campaign.ps1 -Mode quick
.\tools\campaign.ps1 -From latest -Continue -Rounds 1
.\tools\campaign.ps1 -Inspect
```

Build the main campaign driver directly when debugging the binary:

```powershell
cd D:\rust\sts_simulator
cargo build --profile fast-run --bin branch_campaign_driver
```

Manual REPL runs are still supported when you want to play or inspect the
simulator interactively:

```powershell
$seed = Get-Random -Minimum 1 -Maximum 2147483647
echo "seed=$seed"
cargo run --profile fast-run --bin run_play_driver -- --seed $seed --ascension 0 --class ironclad --record --search-wall-ms 100
```

Useful in-session commands:

| Command | Meaning |
| --- | --- |
| `ar` | auto-run with guarded route/card/search helpers until human input is needed |
| `n` | guarded advance without route planning |
| `nr` | guarded advance with route planning |
| `rs` / `rg` | route suggestion / execute one route choice |
| `sc` | run combat search from the current combat boundary |
| `sd` | inspect or update search defaults |
| `mark <name>` | save a replay bookmark while recording |
| `q` | quit cleanly |

Resume from a bookmark:

```powershell
cargo run --profile fast-run --bin run_play_driver -- --goto <name> --search-wall-ms 100
```

See [docs/RUN_PLAY_GUIDE.md](docs/RUN_PLAY_GUIDE.md) for the maintained play
workflow.

## Main Entrypoints

| Binary | Purpose |
| --- | --- |
| `branch_campaign_driver` | current automated branch campaign, checkpoint inspection, outcome export, and continuation experiments |
| `run_play_driver` | manual and semi-automatic simulator runs, traces, bookmarks, captures, baselines |
| `combat_search_v2_driver` | whole-combat search from start specs, combat captures, or benchmark suites |
| `artifact_doctor` | read-only audit over benchmark artifact directories |

See [src/bin/README.md](src/bin/README.md) for binary details.

## Active Docs

Start here:

- [docs/CURRENT_DIRECTION.md](docs/CURRENT_DIRECTION.md)
- [docs/RUN_PLAY_GUIDE.md](docs/RUN_PLAY_GUIDE.md)
- [docs/AUTOPILOT_BOUNDARY.md](docs/AUTOPILOT_BOUNDARY.md)

The old documentation tree was moved to:

```text
docs_legacy/2026-06-03_pre_rewrite/docs/
```

Legacy docs are for archaeology only. They may mention retired LLM, live-comm,
watch UI, Workbench, or stale command paths.

## Architecture

| Directory | Role |
| --- | --- |
| `src/content` | Java-game content reimplementation; avoid casual churn |
| `src/state` | run, combat, map, event, reward, and engine state |
| `src/engine` | state transitions and action handlers |
| `src/runtime` | runtime support for run/combat execution |
| `src/sim` | simulator-facing legal action and apply/search boundaries |
| `src/ai` | combat search, state keys, route planner, value/rollout work |
| `src/eval` | run-control, benchmark artifacts, diagnostics, reports |
| `src/bin` | maintained command entrypoints |

## Verification

For core code changes:

```powershell
cargo fmt --check
cargo check --all-targets
cargo test --quiet
cargo check --release --all-targets
cargo build --profile fast-run --bin branch_campaign_driver
cargo build --release --bin run_play_driver
cargo build --release --bin combat_search_v2_driver
git diff --check
```

For documentation-only changes, at minimum run:

```powershell
git diff --check
```

## License and Game Notice

No license has been declared yet.

This is an unofficial research project. Slay the Spire is developed by Mega
Crit; this repository is not affiliated with or endorsed by Mega Crit.
