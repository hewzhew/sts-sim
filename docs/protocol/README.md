# Protocol

This directory is the rulebook for the Java protocol and the Rust importer.

The active combat schema is split:

- `game_state.combat_truth`
- `game_state.combat_observation`
- `protocol_meta.combat_action_space`
- `protocol_meta.noncombat_action_space`
- `protocol_meta.continuation_state`
- protocol/session metadata such as `reward_session` and `combat_session`

Rust screen-command routing treats `noncombat_action_space` as the preferred
source for ordinary non-combat screens, and falls back to `combat_action_space`
for combat-internal pending screens such as grid or discovery choices.

The legacy merged `combat_state` payload is historical and should not be treated
as the live contract.

## Read These First

- [PROTOCOL_TRUTH_RULES.md](PROTOCOL_TRUTH_RULES.md)
  - hard rules for Java truth, protocol export, and importer boundaries
- [STATE_SYNC_STATUS.md](STATE_SYNC_STATUS.md)
  - what the current live importer already consumes and what debt remains
- [MANUAL_SCENARIO_SAMPLE_INDEX.md](MANUAL_SCENARIO_SAMPLE_INDEX.md)
  - checked-in protocol truth sample inventory

Behavior and sample matrices:

- [GUARDIAN_THRESHOLD_TEST_MATRIX.md](GUARDIAN_THRESHOLD_TEST_MATRIX.md)
- [STASIS_TEST_MATRIX.md](STASIS_TEST_MATRIX.md)

Java-side migration reference:

- [../../../CommunicationMod/PROTOCOL_SCHEMA_MIGRATION.md](../../../CommunicationMod/PROTOCOL_SCHEMA_MIGRATION.md)

## What Belongs Here

- CommunicationMod truth rules
- importer status and debt
- manual scenario truth samples
- protocol-facing behavior matrices
- dated protocol audits when they still explain current constraints

## How To Read Older Notes

Files with dated names such as `*_2026-04-12.md` are historical protocol notes.
They are useful when you need prior rationale, but they do not outrank:

- `PROTOCOL_TRUTH_RULES.md`
- `STATE_SYNC_STATUS.md`
- the current `CommunicationMod` migration document
