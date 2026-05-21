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
- `cargo test --quiet`
  - current checked-in Rust validation suite
- `tests/protocol_truth_samples/`
  - checked-in protocol truth fixture data
- [../live_comm/LIVE_COMM_MANUAL_SCENARIO_RUNBOOK.md](../live_comm/LIVE_COMM_MANUAL_SCENARIO_RUNBOOK.md)
  - historical targeted live spot-check and sample capture workflow
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

`-IncludeParity` is currently a compatibility switch only; live-comm parity is
legacy fixture work until the adapter is rebuilt.

If a correctness-sensitive test cannot name its oracle source, stop and classify
it first instead of writing a guessed expected value.
