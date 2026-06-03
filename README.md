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

1. run a deterministic simulator session from Neow onward
2. make or automate low-risk non-combat decisions under explicit boundaries
3. capture stable combat starts when needed
4. run Combat Search V2 over complete combat trajectories
5. compare whole-combat outcomes, not step-by-step action agreement

Autopilot, route planning, card reward policy, traces, and search-assisted
combat are convenience/evidence tools. They are not teacher labels.

## Quick Start

Build once:

```powershell
cd D:\rust\sts_simulator
cargo build --release --bin run_play_driver
```

Start a fresh recorded run with a random seed:

```powershell
$seed = Get-Random -Minimum 1 -Maximum 2147483647
echo "seed=$seed"
.\target\release\run_play_driver.exe --seed $seed --ascension 0 --class ironclad --record --search-wall-ms 100
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
.\target\release\run_play_driver.exe --goto <name> --search-wall-ms 100
```

See [docs/RUN_PLAY_GUIDE.md](docs/RUN_PLAY_GUIDE.md) for the maintained play
workflow.

## Main Entrypoints

| Binary | Purpose |
| --- | --- |
| `run_play_driver` | manual and semi-automatic simulator runs, traces, bookmarks, captures, baselines |
| `combat_search_v2_driver` | whole-combat search from start specs, combat captures, or benchmark suites |
| `artifact_doctor` | read-only audit over benchmark artifact directories |

See [src/bin/README.md](src/bin/README.md) for binary details.

## Active Docs

Start here:

- [docs/CURRENT_DIRECTION.md](docs/CURRENT_DIRECTION.md)
- [docs/RUN_PLAY_GUIDE.md](docs/RUN_PLAY_GUIDE.md)
- [docs/AUTOPILOT_BOUNDARY.md](docs/AUTOPILOT_BOUNDARY.md)
- [docs/ARTIFACTS.md](docs/ARTIFACTS.md)
- [docs/KNOWN_LIMITS.md](docs/KNOWN_LIMITS.md)

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
