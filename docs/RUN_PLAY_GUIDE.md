# Run Play Guide

`run_play_driver` is the maintained manual and semi-automatic run driver.

## Start A New Recorded Run

Use a random seed when probing new behavior:

```powershell
cd D:\rust\sts_simulator
$seed = Get-Random -Minimum 1 -Maximum 2147483647
echo "seed=$seed"
.\target\release\run_play_driver.exe --seed $seed --ascension 0 --class ironclad --record --search-wall-ms 100
```

`--record` writes a unique trace under `tools/artifacts/traces/`. It does not
make the trace a benchmark or a teacher label.

## Common Commands

| Command | Meaning |
| --- | --- |
| `ar` | auto-run with guarded route/card/search helpers until human input is needed |
| `n` | guarded advance without route planning |
| `nr` | guarded advance with route planning |
| `rs` | show route suggestion only |
| `rg` | execute one route-planner map choice |
| `sc` | run combat search from the current combat boundary |
| `sd` | inspect or update search defaults |
| `mark <name>` | save a named replay bookmark while recording |
| `marks` | list known bookmarks |
| `q` | quit cleanly |

Use panels as needed:

```text
deck | map | mf | relics | potions | draw | discard | exhaust | inspect <id> | details | raw
```

`Ctrl+C` exits the process immediately. Prefer `q` when recording traces so the
session can finish cleanly.

## Resume From A Bookmark

After saving a mark:

```text
mark before_we_meet_again
q
```

Resume later:

```powershell
.\target\release\run_play_driver.exe --goto before_we_meet_again --search-wall-ms 100
```

This replays the recorded prefix, creates a new continuation trace, and stops at
the bookmarked boundary. Read-only panel commands such as `deck` and `map` are
not replayed as state changes.

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

## Search Budgets

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
