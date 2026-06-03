# Artifacts

Artifacts live under `tools/artifacts/` by default and are ignored by git.

## Trace

`SessionTraceV1` records successful state-changing commands from a run session.

Use it for:

- replaying a prefix
- creating bookmarks
- explaining how a test point was reached
- debugging drift

Do not use it as:

- a teacher label
- a benchmark suite
- a proof that a policy is good

## Bookmark

Bookmarks point into recorded traces. They are convenience handles for returning
to a known boundary without typing a long command.

Use:

```text
mark <name>
marks
```

Resume:

```powershell
.\target\release\run_play_driver.exe --goto <name> --search-wall-ms 100
```

## Combat Capture

`CombatCaptureV1` is a stable combat decision boundary. It is the preferred
input for combat search benchmarking.

Use:

```text
cap <case_id>
```

A capture may include privileged simulator state for restoration. Public
observation and hidden simulator state must stay conceptually separate.

## Baseline

`CombatBaselineOutcomeV1` records the outcome of the matching captured combat.

Use:

```text
baseline
```

Only save a human baseline when the combat was actually played as the intended
baseline. If combat search took over, record it as search evidence instead.

## Benchmark Suite

`BenchmarkSuiteV1` registers captures and optional baselines.

It is for repeated search evaluation over fixed combat starts. It should not
directly mean "this seed can still reach this point"; that is a trace/replay
provenance question.
