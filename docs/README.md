# Current Documentation

This directory only keeps docs that are meant to guide current work. Retired
notes are not kept searchable; use git history for archaeology.

## Read These

1. [ARCHITECTURE.md](ARCHITECTURE.md): ownership boundaries and design rules.
2. [RUNBOOK.md](RUNBOOK.md): maintained commands and local verification.
3. [TESTING.md](TESTING.md): test ownership, cleanup, and review standards.

## Rules

- If docs and active code disagree, update the doc or fix the code in the same
  change.
- Do not add a new doc for a temporary investigation. Use a run capsule,
  combat case, thread note, or commit message.
- A new maintained doc should replace or summarize a current boundary. It
  should not create a second source of truth.
