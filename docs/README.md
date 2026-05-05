# Docs Index

This directory is split between active entrypoints and dated working notes.

## Read These First

- [../README.md](../README.md)
  - current high-level project status
- [REPOSITORY_MAP.md](REPOSITORY_MAP.md)
  - ownership map and active repo surfaces
- [LAYER_BOUNDARIES.md](LAYER_BOUNDARIES.md)
  - hard dependency direction for `core / integration / app`
- [TEST_ORACLE_STRATEGY.md](TEST_ORACLE_STRATEGY.md)
  - oracle discipline for correctness-sensitive tests

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
- learning and RL-facing experiments:
  - [RL_READINESS_CHECKLIST.md](RL_READINESS_CHECKLIST.md)
  - [design/README.md](design/README.md)
  - [../tools/learning/README.md](../tools/learning/README.md)

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
  - active design docs and experiment contracts
- `docs/audits/`
  - dated investigations and validation reports
- `docs/archive/`
  - retired handoffs and historical notes
- `docs/templates/`
  - reusable templates

## Canonical Versus Historical

Use this rule when reading docs:

- if a file is linked from an active `README.md`, treat it as current workflow
- if a file lives under `audits/` or `archive/`, treat it as historical context unless a current doc explicitly promotes it
- if a dated note disagrees with the root `README`, protocol docs, or live-comm runbook, the dated note loses

Root `docs/` should stay small. If a document is mainly a topic-specific runbook,
deep design thread, or one-off investigation, it belongs in a subdirectory.
