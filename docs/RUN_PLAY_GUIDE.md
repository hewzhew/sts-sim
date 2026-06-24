# Run Play Guide

`run_play_driver` is the maintained manual and semi-automatic REPL for a single
simulator run. Use it to inspect a run, record a trace, create bookmarks, capture
combat starts, and save whole-combat baselines.

It is not the campaign scheduler. Campaign reports, checkpoints, journals, and
scratch runs are owned by
[Campaign Artifact Architecture](CAMPAIGN_ARTIFACT_ARCHITECTURE.md).

## Start A Recorded Run

Use `fast-run` for normal local probing:

```powershell
cd D:\rust\sts_simulator
$seed = Get-Random -Minimum 1 -Maximum 2147483647
echo "seed=$seed"
cargo run --profile fast-run --bin run_play_driver -- --seed $seed --ascension 0 --class ironclad --record --search-wall-ms 100
```

`--record` writes a unique `SessionTraceV1` under
`tools/artifacts/traces/`. A trace is provenance, not a teacher label, not a
benchmark suite, and not proof that a policy is good.

## Common Commands

| Command | Meaning |
| --- | --- |
| `ar` | auto-run with guarded route/card/search helpers until a boundary stops it |
| `n` | guarded advance without route planning |
| `nr` | guarded advance with route planning |
| `rs` | show route suggestion only |
| `rg` | execute one route-planner map choice |
| `bd` | show the current `NonCombatDecisionRecordV1` summary |
| `sc` | run combat search from the current combat boundary |
| `sd` | inspect or update search defaults |
| `mark <name>` | save a named replay bookmark while recording |
| `marks` | list known bookmarks |
| `q` | quit cleanly |

Useful panels:

```text
deck | map | mf | bd | relics | potions | draw | discard | exhaust | inspect <id> | details | raw
```

`Ctrl+C` exits immediately. Prefer `q` when recording so the trace can finish
cleanly.

## Trace And Bookmark

`SessionTraceV1` records successful state-changing commands. It may also include
non-combat boundary records when automation stops without changing state, such
as a shop or campfire boundary. Those records are provenance evidence and do not
replay as actions.

Bookmarks point into recorded traces and avoid long replay prefixes.

```text
mark before_reward
marks
q
```

Resume later:

```powershell
cargo run --profile fast-run --bin run_play_driver -- --goto before_reward --search-wall-ms 100
```

`--goto` replays the recorded prefix, creates a new continuation trace beside
the source trace, and stops at the bookmarked boundary. Read-only panel commands
such as `deck` and `map` are not replayed as state changes.

## Combat Capture And Baseline

`CombatCaptureV1` is the stable combat decision boundary used by combat search
benchmarks.

```text
cap act1_elite_lagavulin
```

A capture may include privileged simulator state for exact restoration. Keep
public observation and hidden simulator state conceptually separate when using
captures for analysis.

`CombatBaselineOutcomeV1` records the matching completed combat outcome.

```text
baseline
```

Only save a baseline when the combat was actually played as the intended
baseline. If combat search took over, treat the result as search evidence
instead.

`BenchmarkSuiteV1` registers captures and optional baselines for repeated search
evaluation over fixed combat starts. It does not prove the original seed can
still reach that combat; that is trace or campaign provenance.

## Reward Screens

Outer reward screens can contain a card reward item:

```text
0 | Card reward [Twin Strike, Sword Boomerang, Warcry]
skip | Open map preview
```

Open the card reward first:

```text
0
```

Then choose a card or skip on the card reward screen:

```text
0 | Twin Strike
1 | Sword Boomerang
2 | Warcry
3 | Skip card reward
```

`skip` from the outer reward screen opens a map preview while unclaimed rewards
remain. It is not the same as permanently abandoning rewards before choosing a
next room.

## Combat Search Budgets

Use a small wall-clock budget for quick manual runs:

```powershell
--search-wall-ms 100
```

Use a larger budget when inspecting a hard combat:

```text
sc max_nodes=500000 wall_ms=30000
```

If combat search reports unresolved, it did not prove the fight unwinnable. It
only failed to find an executable complete win under the current budget.
