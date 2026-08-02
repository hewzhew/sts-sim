# Operational Runbooks

[../RUNBOOK.md](../RUNBOOK.md) is the compact entry point for maintained
commands. These topic pages hold recipes that need more context:

- [Combat Evidence And Offline Laboratories](combat-evidence.md): exact combat
  replay, typed evidence queries, potion lanes, and fixed-input laboratories.
- [Oracle Operations](oracle-operations.md): late-stop diagnosis, resident
  sessions, durable seed panels, and state-root safety.
- [Performance Investigation](performance.md): exact-contract benchmarks,
  native CPU traces, symbol identity, and measurement acceptance.

Keep every page operational:

- command flags are owned by Clap definitions and `--help`;
- emitted schema versions are owned by the Rust producer and must not be
  copied into recipe names;
- architecture and policy semantics are linked from
  [../ARCHITECTURE.md](../ARCHITECTURE.md), not duplicated here;
- machine-specific timings and one-off experiment conclusions belong in a
  dated design or report, not a maintained runbook;
- new mutable artifacts go below the ignored `.oracle-lab/` state root unless
  the producing command has a different explicit artifact contract.
