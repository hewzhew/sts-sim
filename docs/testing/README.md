# Testing

Testing platform and execution workflow notes live here.

Keep this directory focused on test infrastructure, capture workflow, and validation harness usage rather than architecture or protocol design.

Additional layout guidance:

- [MONSTER_SEMANTIC_TEST_LAYOUT.md](./MONSTER_SEMANTIC_TEST_LAYOUT.md)
  - target placement and factoring rules for monster semantic tests
- [testing_platform.md](./testing_platform.md)
  - canonical `CombatCase` workflow, bridge status, and default runners

## Validation Entrypoints

Treat these as the default validation entrypoints rather than optional buried
tools:

- [../TEST_ORACLE_STRATEGY.md](../TEST_ORACLE_STRATEGY.md)
  - classify oracle strength before writing correctness-sensitive tests
- `tests/live_comm_replay_driver.rs`
  - differential parity against structured live-comm replays
- `tests/protocol_truth_samples.rs`
  - checked-in protocol truth fixtures
- `tests/combat_case_driver.rs`
  - checked-in canonical combat regressions under `tests/combat_cases/`
- `tests/state_sync_strictness.rs`
  - importer strictness and missing-field guardrails
- `tests/guardian_threshold_behavior.rs`
  - example behavior test file with explicit oracle annotation
- `tests/stasis_behavior.rs`
  - example behavior test file for a power-driven hidden-state mechanic
- [../live_comm/LIVE_COMM_MANUAL_SCENARIO_RUNBOOK.md](../live_comm/LIVE_COMM_MANUAL_SCENARIO_RUNBOOK.md)
  - targeted live spot-check and sample capture workflow
- `tools/sts_tool`
  - investigation tool for scattered Java/source tracing when exact behavior is
    difficult to infer from one class alone
- `tools/run_high_value_tests.ps1`
  - default command entrypoint for the current high-value correctness suite

## Default Correctness Suite

Run the current fast high-value suite with:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\run_high_value_tests.ps1
```

Include the slower parity layer explicitly:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\run_high_value_tests.ps1 -IncludeParity
```

If a correctness-sensitive test cannot name its oracle source, stop and classify
it first instead of writing a guessed expected value.
