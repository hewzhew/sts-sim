# Performance Investigation

Measure the smallest production-shaped surface that can answer the question.
Timing is admissible only when source, executable, symbols, workload, exact
counters, and witness identity are all controlled.

Architecture and compilation ownership are maintained in
[../ARCHITECTURE.md](../ARCHITECTURE.md#cargo-package-boundary). This page owns
commands and acceptance rules, not historical machine timings.

## Native CPU Trace

Record a symbolized trace of the narrow combat contract with:

```powershell
.\tools\perf\profile_combat_cpu.ps1
```

Analyze an existing trace without recording again or requesting elevation:

```powershell
.\tools\perf\summarize_combat_cpu.ps1 -Trace .profiles\combat-cpu-<id>.etl
```

The summary normalizes samples across only the recorded `combat_contract`
processes. The recorder publishes the matching Rust PDB into an ignored,
identity-keyed local symbol cache before WPR starts so an older trace remains
symbolizable after later builds. The summarizer rejects insufficient symbol
resolution instead of producing a hotspot table dominated by unknown frames.

PerfView is an ignored local analysis dependency below `.profiles/tools`; it
is not a repository artifact.

## Exact Contract Benchmark

Use the canonical workload without WPR or elevation:

```powershell
.\tools\perf\benchmark_combat_contract.ps1
```

The script builds the narrow target once, warms it, reports batched process
wall time, and rejects iterations whose exact counters or witness differ. The
benchmark and profiler source the same workload definition.

Both tools write a content-addressed build receipt beside the profiling
executable. `-SkipBuild` verifies the complete Rust/Cargo source scope plus the
executable and PDB. Rebuild without `-SkipBuild` after any relevant source
change.

## Identity-Locked Combat Panel

Before changing state ownership or another cross-cutting hot path, run:

```powershell
.\tools\perf\benchmark_combat_panel.ps1 -SkipBuild
```

The panel interleaves maintained light, long, large-state, and
replay-verified combat cases under one fixed work contract. Timing is
observational; deterministic counters and witness identity must match the
panel specification before accepting a result.

To diagnose transition ownership costs with sparse sampling:

```powershell
.\tools\perf\benchmark_combat_panel.ps1 -SkipBuild -ProfileTransitionCloneCost
```

The diagnostic reports engine clone, combat clone, and transition execution
costs. Its wall time is instrumented and must not be used for performance
acceptance; use the ordinary panel for before/after timing.

## Build Boundaries

Oracle work uses one canonical incremental `release` artifact. The narrow
`.\ol-contract.cmd` surface excludes the run explorer and resident runtime, so
planner changes do not require linking the full oracle host. Use
`release-final` only for a deliberately requested fully optimized deployment
artifact.

Splitting a Rust source file into modules changes ownership and review scope,
not the Cargo compilation unit. Claim rebuild improvements only after a
measured crate-boundary change with matched source invalidation.

Do not configure a project-wide `sccache` wrapper for the canonical iterative
build. That profile deliberately uses incremental compilation, which sccache
does not cache. Reconsider it only for a separately measured non-incremental CI
or same-path recovery workflow.

## Reporting Results

Report:

- source commit and dirty state;
- exact command and workload identity;
- whether a build was performed or receipt-verified;
- accepted and rejected iteration counts;
- deterministic counter and witness agreement;
- aggregate timing and a short hotspot table;
- environmental caveats that could affect comparison.

Store full traces and logs in ignored artifact roots and surface only
aggregates plus a short failure tail. Machine-specific observations are not CI
thresholds unless a separate maintained contract explicitly promotes them.
