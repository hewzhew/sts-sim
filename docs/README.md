# Docs Index

This directory now uses a lighter structure:

- root `docs/`
  - canonical docs, active workflows, and still-live design notes
- `docs/audits/`
  - one-off investigations, validation reports, and topic-specific findings
- `docs/templates/`
  - reusable templates or document scaffolds
- `docs/archive/`
  - historical handoffs and retired notes

Read these first:

- `REPOSITORY_MAP.md`
  - top-level repository map, ownership tags, and RL main path
- `LAYER_BOUNDARIES.md`
  - hard dependency direction for `core / integration / app`
- `architecture.md`
  - system architecture and verification context
- `PROTOCOL_TRUTH_RULES.md`
  - hard rules for Java truth, protocol export, and importer boundaries
- `STATE_SYNC_STATUS.md`
  - current live-path importer / protocol status
- `PLAY_GUIDE.md`
  - terminal and developer-facing usage
- `BUGFIX_WORKFLOW.md`
  - parity bug workflow
- `LIVE_COMM_RUNBOOK.md`
  - operational live-comm workflow

Active design docs still kept in root:

- `COMBAT_STATE_REFACTOR.md`
- `DRAW_HAND_SIZE_DESIGN.md`
- `COMM_PROTOCOL_REWARD_SESSION_DRAFT.md`
- `LEARNING_TRUTH_SOURCES.md`
- `WATCH_PRESET_SCHEMA_DRAFT.md`
- `MINIMAL_COMBAT_LOCAL_RL_EXPERIMENT.md`

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

Move a doc out of the root when it is mostly one of:

- a dated audit
- a narrow experiment report
- a temporary finding for a single mechanics cluster
- a handoff tied to a specific debugging window

Keep a doc in the root only when at least one of these is true:

- it is part of the default developer workflow
- other docs or tools should link to it directly
- it defines an active design the codebase still follows
