# sts_simulator

[中文说明](README.zh-CN.md)

`sts_simulator` is an unofficial Rust simulator and AI-search workspace for
Slay the Spire.

Current main line:

```text
simulator correctness
  -> Rust-owned campaign application
  -> source/output/continuation lifecycle
  -> search/rollout evidence when needed
```

The project is not currently focused on old watch UI, Workbench,
DecisionFrame, prompt engineering, or an LLM-driven controller. Those may return
later as adapters, but they do not define simulator truth or search quality.

## Current Workflow

The maintained campaign wrapper direction is:

1. resolve a source artifact when continuing or inspecting
2. allocate a new output artifact for each run/continue invocation
3. run a new campaign or continue a source for a small explicit round budget

Autopilot, route planning, card reward policy, traces, and search-assisted
combat are convenience/evidence tools. They are not teacher labels.

The campaign system is owned by typed Rust application boundaries. The
PowerShell wrapper is a local source/output/continuation launcher, not the
architecture. See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md).

## Quick Start

Use the Rust campaign surface directly when checking architecture, CLI
behavior, or artifact semantics:

```powershell
cd D:\rust\sts_simulator
cargo run --profile fast-run --bin branch_campaign_driver -- campaign run --preset quick --seed 1 --rounds 0
```

Use the local launcher when you want the current short aliases:

```powershell
.\tools\campaign.ps1 -Mode quick
.\tools\campaign.ps1 -From latest -Continue -Mode quick -Rounds 2
.\tools\campaign.ps1 -From latest -Inspect
```

Treat wrapper commands as launch aliases, not architecture. See
[docs/RUNBOOK.md](docs/RUNBOOK.md) for branch-tiny panels, combat case review,
manual REPL usage, search drivers, and verification commands.

## Main Entrypoints

| Binary | Purpose |
| --- | --- |
| `branch_campaign_driver` | current automated branch campaign, checkpoint inspection, outcome export, and continuation experiments |
| `branch_tiny` | lightweight owner-audit runner with run capsules, frontier continuation, and gap-panel diagnostics |
| `run_play_driver` | manual and semi-automatic simulator runs, traces, bookmarks, captures, baselines |
| `combat_search_v2_driver` | whole-combat search from start specs, combat captures, or benchmark suites |
| `combat_case_review` | review ladder for saved combat cases from branch-tiny combat gaps |

See [src/bin/README.md](src/bin/README.md) for binary details.

## Active Docs

- [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md): current ownership boundaries
  and design rules.
- [docs/RUNBOOK.md](docs/RUNBOOK.md): maintained local commands and
  verification.

Retired docs are not kept searchable in the working tree. Use git history for
archaeology.

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

Use [docs/RUNBOOK.md](docs/RUNBOOK.md) for maintained verification commands.
Run targeted tests only when the changed surface has a stable structural
contract worth protecting.

## License and Game Notice

No license has been declared yet.

This is an unofficial research project. Slay the Spire is developed by Mega
Crit; this repository is not affiliated with or endorsed by Mega Crit.
