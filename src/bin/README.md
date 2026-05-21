# Binary Entrypoints

Each binary now lives in its own directory under `src/bin/<name>/main.rs`.

This keeps the root of `src/bin/` navigable while preserving Cargo's auto-discovered binary names.

Rough groups:

- user / developer interaction
  - `view_replay`
- combat validation and audits
  - `combat_case`
  - `combat_env_driver`
  - `combat_search_v2_driver`
  - `verify_live_comm_replay`

Supporting module ownership for these binaries now lives in:

- `sts_simulator::fixtures`
  - scenario and author/start-spec inputs
- `sts_simulator::eval::combat_env`
  - combat env surfaces for explicit external action selection
- `crate::testing::harness`
  - integration-side analysis helpers consumed internally by app-layer harnesses
- `sts_simulator::diff::protocol`
  - protocol parsing and live snapshot shaping
- `sts_simulator::diff::replay`
  - replay execution and diff comparison
