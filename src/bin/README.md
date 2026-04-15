# Binary Entrypoints

Each binary now lives in its own directory under `src/bin/<name>/main.rs`.

This keeps the root of `src/bin/` navigable while preserving Cargo's auto-discovered binary names.

Rough groups:

- user / developer interaction
  - `play`
  - `combat_lab`
  - `sts_dev_tool`
  - `view_replay`
- combat validation and audits
  - `combat_author_audit`
  - `combat_boss_validate`
  - `combat_boss_validate_pack`
  - `combat_decision_audit`
  - `combat_env_driver`
  - `potion_audit`
  - `verify_live_comm_replay`
  - `verify_shop`

Supporting module ownership for these binaries now lives in:

- `sts_simulator::testing::fixtures`
  - scenario and author/start-spec inputs
- `sts_simulator::bot::harness`
  - combat env, lab, and bot-coupled validation workbenches
- `sts_simulator::testing::harness`
  - integration-side analysis helpers consumed internally by app-layer harnesses
- `sts_simulator::diff::protocol`
  - protocol parsing and live snapshot shaping
- `sts_simulator::diff::replay`
  - replay execution and diff comparison
