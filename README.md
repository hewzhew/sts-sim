# sts_simulator

[中文说明](README.zh-CN.md)

`sts_simulator` is an unofficial Rust simulator and AI-search workspace for
Slay the Spire.

The active direction is:

```text
simulator state -> legal actions -> rollout/search -> value -> policy improvement
```

This repository is not currently focused on a live-game UI, prompt engineering,
or an LLM-driven controller. Those may become adapters later, but the main line
is simulator correctness, stable state boundaries, combat search, and
whole-combat outcome evaluation.

## Current Status

This is a work in progress. The currently maintained workflow is:

1. run a deterministic simulator session from Neow onward
2. make non-combat decisions manually or with guarded helpers
3. capture stable combat starts
4. run Combat Search V2 on whole-combat trajectories
5. compare search outcomes against whole-combat baselines

Search reports are budgeted evidence. An unresolved result or a budgeted win is
not a proof of optimal play.

## What This Is

- a Rust reimplementation of Slay the Spire run/combat state transitions
- legal action generation and apply-action execution
- a terminal run/play driver for manual and semi-automatic runs
- exact combat capture and benchmark artifact infrastructure
- Combat Search V2 experiments over complete combat trajectories
- route-planner evidence for map decisions

## What This Is Not

- not an official Mega Crit project
- not a polished game client
- not a live CommunicationMod replacement right now
- not an LLM teacher-label generator
- not a stable public API yet
- not a claim of optimal Slay the Spire play

## Quick Start

```powershell
cd D:\rust\sts_simulator
cargo test --quiet
cargo run --release --bin run_play_driver -- --seed 521 --ascension 0 --class ironclad
```

For faster iteration during development, use the debug build:

```powershell
cargo run --bin run_play_driver -- --seed 521 --ascension 0 --class ironclad
```

## Main Entrypoints

| Binary | Purpose |
| --- | --- |
| `run_play_driver` | manual or semi-automatic simulator run, combat capture, whole-combat baseline capture |
| `combat_search_v2_driver` | whole-combat search from start specs, combat captures, or benchmark suites |
| `artifact_doctor` | read-only audit over benchmark artifact directories |

See [src/bin/README.md](src/bin/README.md) for the current binary surface.

## Manual Run Workflow

Start a simulator session:

```powershell
cargo run --release --bin run_play_driver -- --seed 521 --ascension 0 --class ironclad
```

Optional diagnostic trace:

```powershell
cargo run --release --bin run_play_driver -- --seed 521 --ascension 0 --class ironclad --trace tools\artifacts\traces\seed521.trace.json
```

Optional automatic combat-start capture:

```powershell
cargo run --release --bin run_play_driver -- --seed 521 --ascension 0 --class ironclad --auto-capture-combat
```

Useful in-session commands:

| Command | Meaning |
| --- | --- |
| `n` / `next` | guarded advance to the next human strategic boundary |
| `nr` | guarded advance, allowing the route planner to choose map nodes |
| `rs` | route suggestion only |
| `rg` | route planner chooses and executes one map move |
| `sc` / `search-combat` | run Combat Search V2 from the current combat boundary |
| `cap <case_id>` | capture the current stable combat start |
| `baseline` | save the matching whole-combat baseline after the captured combat ends |
| `deck`, `map`, `relics`, `potions` | inspect visible run panels |
| `draw`, `discard`, `exhaust` | inspect combat piles |
| `details`, `raw` | inspect debug/internal views |
| `help` | show the full command list |

Reward screens and map previews intentionally preserve unclaimed rewards until a
path is actually chosen. Opening the map from a reward screen is a preview; use
`back` or `cancel` to return, and use `go <x>` or `rg` to commit to the next
room.

## Combat Search Workflow

Run search from an exact combat capture:

```powershell
cargo run --release --bin combat_search_v2_driver -- --combat-snapshot tools\artifacts\benchmarks\seed521_act1\captures\some_case.capture.json
```

Run search over a benchmark suite:

```powershell
cargo run --release --bin combat_search_v2_driver -- --benchmark-spec tools\artifacts\benchmarks\seed521_act1\benchmark.json
```

Use explicit budgets when probing hard fights:

```powershell
cargo run --release --bin combat_search_v2_driver -- --combat-snapshot tools\artifacts\benchmarks\seed521_act1\captures\some_case.capture.json --max-nodes 500000 --wall-ms 30000
```

Stop batch search after a good-enough exact win:

```powershell
cargo run --release --bin combat_search_v2_driver -- --benchmark-spec tools\artifacts\benchmarks\seed521_act1\benchmark.json --max-hp-loss 8
```

Benchmark cases with baselines ignore this early-stop gate, so a good-enough
candidate cannot hide a whole-combat baseline regression.

Potion branches are disabled unless explicitly requested:

```powershell
cargo run --release --bin combat_search_v2_driver -- --combat-snapshot tools\artifacts\benchmarks\seed521_act1\captures\some_case.capture.json --potion-policy semantic --max-potions-used 1
```

Important search output concepts:

- `Win` / `Loss` / `Unresolved` describe the reported terminal class.
- `coverage_status=node_budget_limited` or `time_budget_limited` means
  unresolved frontier remains.
- `coverage_status=accepted_complete_candidate` means search stopped after an
  exact complete win passed the configured hp-loss acceptance gate.
- `complete_trajectory_found=false` means the search did not find an executable
  complete win under the given budget.
- A budgeted complete win is useful evidence, but it is not an exhaustive
  best-line claim.

## Artifacts

Artifacts are stored under `tools/artifacts/` by default and are ignored by git.

Common artifact types:

| Artifact | Role |
| --- | --- |
| `CombatCaptureV1` | stable combat decision boundary used as a search input |
| `CombatBaselineOutcomeV1` | whole-combat baseline outcome for a matching capture |
| `BenchmarkSuiteV1` | suite manifest that registers captures and optional baselines |
| `SessionTraceV1` | diagnostic fact log for successful state-changing commands |
| `SearchBenchmarkResultV1` | search result evidence over one or more cases |

Artifacts are provenance and evaluation evidence. They are not teacher labels,
and they should not be treated as proof of policy quality without the matching
benchmark context and simulator version.

Read-only artifact audit:

```powershell
cargo run --release --bin artifact_doctor -- --root tools\artifacts\benchmarks --json
```

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
| `src/bin` | maintained command entrypoints only |
| `docs` | current notes plus historical audits and design records |

Current compatibility modules may preserve old paths, but new code should prefer
the active ownership above.

## Current Roadmap

1. keep stable simulator boundaries correct
2. improve Combat Search V2 value and rollout behavior
3. handle special combat phases and high-fanout branches without unsound pruning
4. make route planning useful for low-risk map automation
5. strengthen capture -> suite -> search -> baseline comparison loops
6. revisit live-game adapters or LLM integration only after the simulator/search
   evidence layer is reliable

## Verification

Before pushing core changes, run:

```powershell
cargo fmt --check
cargo check --all-targets
cargo test --quiet
cargo check --release --all-targets
cargo build --release --bin run_play_driver
cargo build --release --bin combat_search_v2_driver
git diff --check
```

## Documentation Notes

The repository contains many historical investigations. Treat the root README,
[src/bin/README.md](src/bin/README.md), and current code as the active entry
points. Older files under `docs/audits`, `docs/archive`, and retired live-comm
notes are useful context, but they may describe workflows that are no longer the
main path.

## License and Game Notice

No license has been declared yet.

This is an unofficial research project. Slay the Spire is developed by Mega
Crit; this repository is not affiliated with or endorsed by Mega Crit.
