# sts_simulator

[中文说明](README.zh-CN.md)

`sts_simulator` is an unofficial Rust simulator and AI-search workspace for
Slay the Spire.

The project is currently a research and automation codebase, not a polished
library crate. Its main goal is to make simulator state, run decisions, combat
search, and experiment artifacts explicit enough that failures can be replayed
and improved instead of explained from terminal logs.

## Current Focus

The implemented learned-agent boundary now covers public information, private
action resolution, and belief transitions. The choice of search, replay,
model, and A0-to-A20 curriculum remains experimental rather than prescribed.
Final evaluation is still potion-aware natural complete runs; existing search,
owners, cases, and trainers are not automatic teachers.

See [docs/ARCHITECTURE.md](docs/ARCHITECTURE.md) for the maintained boundary
contract.

## Quick Start

These are existing simulator/oracle diagnostic entry points. The new
learned-agent loop is not yet presented as a finished training command.

Run one owner-audit seed:

```powershell
cd D:\rust\sts_simulator
cargo run -p sts_oracle_tools --bin branch_tiny -- --seed 1552225673 --ascension 0 --max-branches 1 --wall-ms 60000
```

Run a small seed panel:

```powershell
cargo run -p sts_oracle_tools --bin branch_panel -- panel smoke --seeds 1552225671 1552225672 1552225673 1552225674 1552225675 --capsule-root tools/artifacts/panels/current --max-branches 1 --slice-ms 60000
```

Review a saved combat case:

```powershell
cargo run -p sts_oracle_tools --bin combat_case_review -- --case <case.json> --ladder
```

Use [docs/RUNBOOK.md](docs/RUNBOOK.md) for maintained bounded-run commands,
continuation examples, combat search drivers, and verification.

## Main Entrypoints

| Binary | Purpose |
| --- | --- |
| `oracle_lab` | canonical heavyweight oracle workspace, exact witness verification, and resumable consecutive seed panels |
| `oracle_lab_service` | resident owner/search runtime for one exact workspace |
| `oracle_lab_client` | lightweight typed client for repeated resident inspection and mutation |
| `branch_tiny` / `branch_panel` | retained owner-audit/capsule diagnostics; not the current production oracle mainline |
| `combat_case_review` | diagnostic review ladder for saved combat cases |
| `combat_search_v2` / `combat_search_v2_worker` | `AtomicExactV2` fixed-root diagnostic/challenger; not the production resident witness engine |
| `rl_dataset_export` | offline decision-sample export for imitation/RL experiments |

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
- [src/agent/README.md](src/agent/README.md): learned-agent public-information
  and belief ownership.

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
| `src/agent` | learned-agent public information, private resolution, and belief contracts |
| `crates/sts_agent` | independent Cargo owner for that source; agent edits do not relink simulator core |
| `src/ai` | policies, strategic facts, deck mutation, combat search, route/search work |
| `src/eval` | Historical physical source tree for combat eval, run-control, the analysis workbench, and learning adapters; the crates below define their Cargo owners |
| `src/bin` | maintained command entrypoints |
| `crates/sts_oracle_eval` | Optimized Cargo owner for combat evaluation and exact-search orchestration |
| `crates/sts_oracle_run_control` | Independent Cargo owner for exact run sessions, decision application, and run evidence |
| `crates/sts_oracle_learning_env` | Optimized exact single-episode learning environments and opaque combat-root artifacts |
| `crates/sts_oracle_learning` | Downstream Cargo owner for model inputs and batched learning adapters |
| `crates/sts_oracle_runtime` | Cheaply rebuildable Cargo owner for the analysis session/workbench, owner parity, branch execution, persistence, and service orchestration |
| `crates/sts_combat_search_driver` | Lightweight combat-search frontend and capability-scoped optimized worker |
| `crates/sts_oracle_tools` | Library-free Cargo host for maintained legacy oracle command adapters and integration contracts |
| `learning` | online-training callers, curricula, seed schedules, models, and evaluation accounting; never simulator mechanics |
| `tools` | offline scripts, datasets, panels, and generated artifacts |
| `docs` | maintained architecture, runbook, testing notes, and current drafts |

Generated outputs belong under ignored locations such as `target/` and
`tools/artifacts/`.

## Combat Search Names

The repository has no version-ordered "highest" search. The current resident
oracle witness producer uses `TurnGraphPortfolioV1`: run control composes complete-turn
`LocalTurnGraphWitnessSession` and `PolicyDiscrepancySession` behind
`OracleResidentCombatWitnessJobV1`. `combat_search_v2` is the atomic-action
`AtomicExactV2` engine used for fixed-root diagnostics, challengers,
benchmarks, and the legacy `branch_tiny` owner-audit path. Their configuration
and evidence identities are distinct; see
[Combat Search Ownership](docs/architecture/combat-search.md). Neither engine
is the learned agent or an authoritative teacher; the new complete direction
is [Public-Belief Agent Learning System](docs/design/2026-08-12-public-belief-agent-learning-system.md).

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
