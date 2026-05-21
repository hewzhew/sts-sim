# Testing

Testing platform and execution workflow notes live here.

Keep this directory focused on active test infrastructure and validation usage,
not historical protocol/case artifact workflows.

Additional layout guidance:

- [MONSTER_SEMANTIC_TEST_LAYOUT.md](./MONSTER_SEMANTIC_TEST_LAYOUT.md)
  - target placement and factoring rules for monster semantic tests
- [testing_platform.md](./testing_platform.md)
  - current start-spec search testing boundary

## Validation Entrypoints

Treat these as the default validation entrypoints rather than optional buried
tools:

- [../TEST_ORACLE_STRATEGY.md](../TEST_ORACLE_STRATEGY.md)
  - classify oracle strength before writing correctness-sensitive tests
- `cargo test --quiet`
  - current checked-in Rust validation suite
- [../live_comm/LIVE_COMM_MANUAL_SCENARIO_RUNBOOK.md](../live_comm/LIVE_COMM_MANUAL_SCENARIO_RUNBOOK.md)
  - historical targeted live spot-check workflow, not an active fixture path
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

`-IncludeParity` is currently a compatibility switch only; live-comm parity and
protocol sample import are retired until the adapter is rebuilt.

If a correctness-sensitive test cannot name its oracle source, stop and classify
it first instead of writing a guessed expected value.
