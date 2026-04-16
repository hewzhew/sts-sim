# Archived Pre-Runtime-State Decision Audit Fixtures

These decision-audit fixtures predate the removal of legacy top-level monster
hidden-state exports from `CommunicationMod`.

They are preserved because the current CLI audit tests and the associated
Hexaghost audit notes still refer to these exact historical states.

The companion legacy CLI integration test has also been moved here:

- `combat_decision_audit_cli.rs`

It depended on these archived fixtures plus a deleted raw live-comm log, so it
no longer belongs in the default active test surface.

They should be treated as archived historical inputs, not as canonical examples
of the current protocol shape.
