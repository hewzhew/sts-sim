# Decision Audit Fixtures

This directory holds fixture inputs used by the `combat_decision_audit` CLI
tests and related audit notes.

Current-shape fixtures should live at the top level.

Historical fixtures that still rely on a retired protocol shape should live
under:

- `archive_pre_runtime_state/`

This also applies to companion CLI integration tests that only make sense
against those archived historical assets.
