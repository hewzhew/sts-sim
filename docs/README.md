# Current Documentation

This directory contains the active documentation for `sts_simulator`.

Retired notes are not kept in the working tree. Use git history for archaeology
instead of keeping stale command paths searchable.

## Read First

1. [CURRENT_DIRECTION.md](CURRENT_DIRECTION.md)
2. [CAMPAIGN_SYSTEM_ARCHITECTURE.md](CAMPAIGN_SYSTEM_ARCHITECTURE.md)

`CAMPAIGN_SYSTEM_ARCHITECTURE.md` is the authority document for campaign
ownership. If another campaign doc disagrees with it, update that doc or fix
the code.

## Campaign Supporting Contracts

- [CAMPAIGN_CLI_CONTRACT.md](CAMPAIGN_CLI_CONTRACT.md): target Rust command
  shape
- [CAMPAIGN_MIGRATION_PLAN.md](CAMPAIGN_MIGRATION_PLAN.md): migration gates and
  stop rules
- [CAMPAIGN_ARTIFACT_ARCHITECTURE.md](CAMPAIGN_ARTIFACT_ARCHITECTURE.md):
  artifact ownership boundaries
- [CAMPAIGN_JOURNAL.md](CAMPAIGN_JOURNAL.md): decision candidate journal
  semantics
- [REPORT_FIELD_ADMISSION.md](REPORT_FIELD_ADMISSION.md): rules for adding
  report, journal, or export fields

## Other Maintained Docs

- [CAMPAIGN_WRAPPER_USAGE.md](CAMPAIGN_WRAPPER_USAGE.md): compatibility wrapper
  usage while migration is incomplete
- [RUN_PLAY_GUIDE.md](RUN_PLAY_GUIDE.md): manual/semi-automatic play driver
- [AUTOPILOT_BOUNDARY.md](AUTOPILOT_BOUNDARY.md): non-combat autopilot boundary

## Current Rule

If active docs disagree with current code behavior, update the active docs or
fix the code in the same change.
