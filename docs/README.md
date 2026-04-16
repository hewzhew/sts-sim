# Docs Index

This directory now separates long-lived entrypoints from topic-specific working sets:

- root `docs/`
  - canonical repo entrypoints and always-on workflows
- `docs/live_comm/`
  - live communication runbooks, modes, parity workflow, and watch/schema notes
- `docs/protocol/`
  - protocol truth, `state_sync`, manual truth samples, and protocol debt tracking
- `docs/design/`
  - active design docs, experiments, and workboards that still inform current code
- `docs/audits/`
  - one-off investigations, validation reports, and dated findings
- `docs/testing/`
  - testing platform workflow notes
- `docs/templates/`
  - reusable templates or scaffolds
- `docs/archive/`
  - historical handoffs and retired notes

Read these first:

- `REPOSITORY_MAP.md`
  - top-level repository map, ownership tags, and RL main path
- `LAYER_BOUNDARIES.md`
  - hard dependency direction for `core / integration / app`
- `architecture.md`
  - system architecture and verification context
- `TEST_ORACLE_STRATEGY.md`
  - how correctness tests should declare and source their oracle, and when to
    use Java source, live samples, parity, invariants, or `tools/sts_tool`
- `RL_READINESS_CHECKLIST.md`
  - what still needs to be true before the simulator should be treated as a
    stable RL environment
- `protocol/PROTOCOL_TRUTH_RULES.md`
  - hard rules for Java truth, protocol export, and importer boundaries
- `protocol/STATE_SYNC_STATUS.md`
  - current live-path importer / protocol status
- `PLAY_GUIDE.md`
  - terminal and developer-facing usage
- `BUGFIX_WORKFLOW.md`
  - parity bug workflow
- `live_comm/LIVE_COMM_RUNBOOK.md`
  - operational live-comm workflow

Current structural anchors:

- `runtime`
  - base runtime primitives (`action`, `combat`, `rng`)
- `diff`
  - protocol / replay / state_sync integration layer
- `bot`
  - search, harness, policy, sidecar app layer
- `cli`
  - live-comm runtime/admin/tooling app layer
- `fixtures`
  - integration fixture/spec entry exported from `lib.rs`

Keep a doc in the root only when at least one of these is true:

- it is part of the default developer workflow
- other docs or tools should link to it directly
- it defines repo-wide architecture or boundary rules

Move a doc out of the root when it is mostly one of:

- a topic-specific runbook
- a protocol/state-sync deep dive
- an active but narrow design thread
- a dated audit or experiment
