# Docs Index

This directory is split between current entrypoints and historical notes.

## Read These First

- [../README.md](../README.md)
- [AI_DIRECTION.md](AI_DIRECTION.md)
- [REPOSITORY_MAP.md](REPOSITORY_MAP.md)
- [LAYER_BOUNDARIES.md](LAYER_BOUNDARIES.md)
- [TEST_ORACLE_STRATEGY.md](TEST_ORACLE_STRATEGY.md)
- [CODEX_CLI_RESUME.md](CODEX_CLI_RESUME.md)

Then branch by task:

- `live_comm` / parity / run archives:
  - [live_comm/README.md](live_comm/README.md)
  - [live_comm/LIVE_COMM_RUNBOOK.md](live_comm/LIVE_COMM_RUNBOOK.md)
  - [live_comm/LIVE_COMM_PARITY_WORKFLOW.md](live_comm/LIVE_COMM_PARITY_WORKFLOW.md)
- protocol / importer / CommunicationMod:
  - [protocol/README.md](protocol/README.md)
  - [protocol/PROTOCOL_TRUTH_RULES.md](protocol/PROTOCOL_TRUTH_RULES.md)
  - [protocol/STATE_SYNC_STATUS.md](protocol/STATE_SYNC_STATUS.md)
- testing / fixtures / scenario work:
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
  - runbooks, mode selection, parity workflow, manual scenario capture
- `docs/protocol/`
  - protocol truth rules, importer status, truth samples, test matrices
- `docs/testing/`
  - testing workflow and fixture/platform notes
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
- if a dated note disagrees with the root `README`, `AI_DIRECTION.md`, protocol
  docs, or live-comm runbook, the current entrypoint wins
