# Current Documentation

This directory only keeps docs that are meant to guide current work. Retired
notes are not kept searchable; use git history for archaeology.

## Read These

1. [ARCHITECTURE.md](ARCHITECTURE.md): ownership boundaries and design rules.
2. [RUNBOOK.md](RUNBOOK.md): maintained commands and local verification.
3. [TESTING.md](TESTING.md): test ownership, cleanup, and review standards.

## Maintained Designs

- [Durable Run Panel Architecture](design/2026-07-07-durable-run-panel-architecture-design.md):
  proposed scheduler/capsule contract for replacing rerun-style gap panels.
- [Outcome-Learned Run Planner Core Contract](design/2026-07-15-outcome-learned-run-planner-core-contract.md):
  clean-room public-state, candidate, trajectory, and outcome-distribution
  boundary for replacing heuristic non-combat owners through measured cutover.
- [Atomic Run Decision Execution and REPL Retirement](design/2026-07-15-atomic-run-decision-execution-design.md):
  active deletion-driven migration from human command transactions to typed
  jobs, atomic progress steps, and an append-only run journal.
- [Durable Trajectory Evidence Migration](design/2026-07-16-durable-trajectory-evidence-migration.md):
  implemented capsule segment DAG, verified checkpoint heads, and rebuildable
  behavior/outcome projections across bounded slices.

## Rules

- If docs and active code disagree, update the doc or fix the code in the same
  change.
- Do not add a new doc for a temporary investigation. Use a run capsule,
  combat case, thread note, or commit message.
- A new maintained doc should replace or summarize a current boundary. It
  should not create a second source of truth.
