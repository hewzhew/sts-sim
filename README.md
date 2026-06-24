# sts_simulator

[中文说明](README.zh-CN.md)

`sts_simulator` is an unofficial Rust simulator and AI-search workspace for
Slay the Spire.

Current main line:

```text
simulator correctness
  -> Rust-owned campaign application
  -> journaled decision candidate coverage
  -> search/rollout evidence
  -> explicit exports for learning or analysis
```

The project is not currently focused on old watch UI, Workbench,
DecisionFrame, prompt engineering, or an LLM-driven controller. Those may return
later as adapters, but they do not define simulator truth or search quality.

## Current Workflow

The maintained campaign direction is:

1. run or continue a deterministic simulator campaign from Neow onward
2. record typed decision candidate pools in `CampaignJournal`
3. use coverage planning to continue historically unobserved candidates
4. use Combat Search V2 for complete combat trajectories inside branches
5. inspect read-only artifact views when a run fails or reaches a milestone
6. export learning or analysis data through explicit exporters

Autopilot, route planning, card reward policy, traces, and search-assisted
combat are convenience/evidence tools. They are not teacher labels.

The campaign system is being migrated to a Rust-owned application boundary. The
PowerShell wrapper remains a local launcher, not the architecture. See
[docs/CAMPAIGN_SYSTEM_ARCHITECTURE.md](docs/CAMPAIGN_SYSTEM_ARCHITECTURE.md).

## Quick Start

Run the current compatibility campaign launcher:

```powershell
cd D:\rust\sts_simulator
.\tools\campaign.ps1 -Mode quick
.\tools\campaign.ps1 -From latest -Continue -Rounds 1
.\tools\campaign.ps1 -Inspect
```

Treat wrapper commands as transitional conveniences. New campaign semantics
belong in the Rust `branch_campaign_driver` campaign app, not in PowerShell.

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
- [docs/CAMPAIGN_SYSTEM_ARCHITECTURE.md](docs/CAMPAIGN_SYSTEM_ARCHITECTURE.md)

`CAMPAIGN_SYSTEM_ARCHITECTURE.md` is the campaign authority contract. The next
three campaign docs are supporting contracts, not competing designs:

- [docs/CAMPAIGN_CLI_CONTRACT.md](docs/CAMPAIGN_CLI_CONTRACT.md)
- [docs/CAMPAIGN_MIGRATION_PLAN.md](docs/CAMPAIGN_MIGRATION_PLAN.md)
- [docs/CAMPAIGN_ARTIFACT_ARCHITECTURE.md](docs/CAMPAIGN_ARTIFACT_ARCHITECTURE.md)
- [docs/CAMPAIGN_JOURNAL.md](docs/CAMPAIGN_JOURNAL.md)
- [docs/AUTOPILOT_BOUNDARY.md](docs/AUTOPILOT_BOUNDARY.md)
- [docs/RUN_PLAY_GUIDE.md](docs/RUN_PLAY_GUIDE.md)
- [docs/CAMPAIGN_WRAPPER_USAGE.md](docs/CAMPAIGN_WRAPPER_USAGE.md)

Retired docs were removed from the working tree to keep search results usable.
Use git history for archaeology.

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

## Editing Hygiene

The repository stores source, docs, and PowerShell scripts with LF line endings.
After mechanical edits on Windows, check that a small source change did not
become a whole-file CRLF rewrite:

```powershell
git diff --stat
git diff --ignore-space-at-eol --stat
git ls-files --eol $(git diff --name-only)
```

Prefer `apply_patch` for source edits. If a one-off PowerShell migration must
rewrite files, write UTF-8 without BOM and normalize text to LF before saving.

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
