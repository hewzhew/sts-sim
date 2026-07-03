# Current Documentation

This directory contains the active documentation for `sts_simulator`.

Retired notes are not kept in the working tree. Use git history for archaeology
instead of keeping stale command paths searchable.

## Read First

1. [CURRENT_DIRECTION.md](CURRENT_DIRECTION.md)

## Campaign Supporting Contracts

- [CAMPAIGN_WORKSPACE_V2.md](CAMPAIGN_WORKSPACE_V2.md): next campaign
  lifecycle design centered on workspaces, attempts, snapshots, and disposable
  views
- [CAMPAIGN_ARTIFACT_ARCHITECTURE.md](CAMPAIGN_ARTIFACT_ARCHITECTURE.md):
  artifact ownership boundaries
- [CAMPAIGN_JOURNAL.md](CAMPAIGN_JOURNAL.md): decision candidate journal
  semantics
- [REPORT_FIELD_ADMISSION.md](REPORT_FIELD_ADMISSION.md): rules for adding
  report, journal, or export fields
- [RUNNER_COMBAT_BOUNDARY.md](RUNNER_COMBAT_BOUNDARY.md): contract between
  run-level automation, combat search, combat cases, and review diagnostics

## Other Maintained Docs

- [NEW_AI_ARCHITECTURE.md](NEW_AI_ARCHITECTURE.md): current AI layering and
  ownership rules for new strategy/policy/runtime work
- [RUN_PLAY_GUIDE.md](RUN_PLAY_GUIDE.md): manual/semi-automatic play driver
- [AUTOPILOT_BOUNDARY.md](AUTOPILOT_BOUNDARY.md): non-combat autopilot boundary

## Current Rule

If active docs disagree with current code behavior, update the active docs or
fix the code in the same change.
