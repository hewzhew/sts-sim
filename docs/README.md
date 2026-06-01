# Docs Index

This directory is split between current entrypoints and historical notes.

## Read These First

- [../README.md](../README.md)
- [AI_DIRECTION.md](AI_DIRECTION.md)
- [REPOSITORY_MAP.md](REPOSITORY_MAP.md)
- [LAYER_BOUNDARIES.md](LAYER_BOUNDARIES.md)
- [TEST_ORACLE_STRATEGY.md](TEST_ORACLE_STRATEGY.md)
- [CODEX_CLI_RESUME.md](CODEX_CLI_RESUME.md)
- [NEXT_AI_HANDOFF.md](NEXT_AI_HANDOFF.md)

Then branch by task:

- LLM controller / demo route:
  - [LLM_INTEGRATION_HANDOFF.md](LLM_INTEGRATION_HANDOFF.md)
  - [../tools/llm/README.md](../tools/llm/README.md)
- Java-source-backed mechanics parity:
  - [MECHANICS_ACCEPTANCE_STANDARD.md](MECHANICS_ACCEPTANCE_STANDARD.md)
  - [MECHANICS_AUDIT_LEDGER.md](MECHANICS_AUDIT_LEDGER.md)
  - [JAVA_SOURCE_MAP.md](JAVA_SOURCE_MAP.md)
  - [JAVA_MECHANICS_DEBUG_HANDOFF.md](JAVA_MECHANICS_DEBUG_HANDOFF.md)
- legacy `live_comm` / parity / fixture capture:
  - [live_comm/README.md](live_comm/README.md)
  - [live_comm/LEGACY_FIXTURE_ONLY.md](live_comm/LEGACY_FIXTURE_ONLY.md)
  - [live_comm/LIVE_COMM_RUNBOOK.md](live_comm/LIVE_COMM_RUNBOOK.md)
  - [live_comm/LIVE_COMM_PARITY_WORKFLOW.md](live_comm/LIVE_COMM_PARITY_WORKFLOW.md)
- testing / start-spec fixture work:
  - [testing/README.md](testing/README.md)
  - [BUGFIX_WORKFLOW.md](BUGFIX_WORKFLOW.md)
- local debug binary usage:
  - [PLAY_GUIDE.md](PLAY_GUIDE.md)
- current AI/eval infrastructure:
  - [../tools/learning/README.md](../tools/learning/README.md)
  - [decision_records/README.md](decision_records/README.md)

## Directory Roles

- root `docs/`
  - repo-wide rules and default entry docs
- `docs/live_comm/`
  - legacy bridge notes and future adapter boundary
- `docs/testing/`
  - active testing workflow and start-spec notes
- `docs/design/`
  - engine and runtime design notes
- `docs/decision_records/`
  - short current decisions that prevent repeating invalidated experiment paths
- `docs/audits/`
  - dated investigations and validation reports
- `docs/archive/`
  - retired handoffs and historical notes
- `docs/templates/`
  - reusable templates

## Canonical Versus Historical

- if a file is linked from an active `README.md`, treat it as current workflow
- if a file lives under `audits/` or `archive/`, treat it as historical context
- if a dated note disagrees with the root `README`, `AI_DIRECTION.md`, or
  live-comm boundary docs, the current entrypoint wins
