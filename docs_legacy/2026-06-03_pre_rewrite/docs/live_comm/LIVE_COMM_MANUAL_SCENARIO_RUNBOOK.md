# Live Comm Manual Scenario Runbook

This runbook is for the new `CommunicationMod` `scenario` command path.
It assumes:

- `CommunicationMod` has been switched to the manual bridge launcher
- the game is started normally through ModTheSpire
- a separate console window opens for manual commands

Current helper to switch config:

```powershell
powershell -ExecutionPolicy Bypass -File D:\rust\sts_simulator\tools\live_comm\install_manual_client_config.ps1
```

To switch back to the normal Rust `play.exe` loop:

```powershell
powershell -ExecutionPolicy Bypass -File D:\rust\sts_simulator\tools\live_comm\install_communicationmod_config.ps1
```

## First boot

When the game launches, the manual console should accept raw commands.

Use this baseline first:

```text
START ironclad 0
STATE
scenario state
```

Useful local REPL commands:

- `/help`
- `/show`
- `/commands`
- `/state`
- `/monsters`
- `/monster N`
- `/find term`
- `/quit`

The latest raw frame is mirrored to:

- `logs/current/manual_client_latest.json`

To archive the current manual frame into a named sample:

```powershell
powershell -ExecutionPolicy Bypass -File D:\rust\sts_simulator\tools\live_comm\save_manual_sample.ps1 guardian_threshold
```

Optional note:

```powershell
powershell -ExecutionPolicy Bypass -File D:\rust\sts_simulator\tools\live_comm\save_manual_sample.ps1 combust -Note "player power runtime_state.hp_loss present"
```

## Current safe boundary

At the moment, the most reliable scenario path is:

- `START`
- `STATE`
- `scenario state`
- `scenario fight`

These are the commands to trust first when rebuilding strict protocol samples.

Commands that exist but should be treated as secondary helpers for now:

- `scenario deck add`
- `scenario relic add`
- `scenario hp set`
- `scenario event`

Why:

- `scenario deck add` only changes `masterDeck`; it does not backfill the
  current combat hand/draw/discard piles
- `scenario event` currently uses a more hand-built room transition path
- `scenario hp set` is useful for setup, but should not be used to force
  edge states like death
- `scenario relic add` is usable, but is less stable as a first recording path

For the first recording pass, prefer:

- entering the target fight
- observing existing runtime truth
- using `scenario power add` only when you specifically need to force a power slice

## Known encounter IDs

These are confirmed from the base game encounter list and normalize
correctly through `scenario fight`:

- `jaw_worm`
- `gremlin_gang`
- `lagavulin`
- `the_guardian`
- `hexaghost`
- `chosen`
- `cultist_and_chosen`
- `chosen_and_byrds`
- `3_darklings`
- `automaton`

## Slice 1: GuardianThreshold

Goal:

- confirm `monster.runtime_state.guardian_threshold` is exported on `TheGuardian`

Commands:

```text
START ironclad 0
scenario fight the_guardian
STATE
```

Success criteria:

- `manual_client_latest.json`
- top-level `game_state.combat_truth.monsters[*].id == "TheGuardian"`
- same monster has:
  - `runtime_state.guardian_threshold`

## Slice 2: Angry

Goal:

- confirm `monster.runtime_state.angry_amount` is exported for `GremlinWarrior`

Commands:

```text
START ironclad 0
scenario fight gremlin_gang
STATE
```

Success criteria:

- find monster with `id == "GremlinWarrior"`
- that monster has:
  - `runtime_state.angry_amount`

## Slice 3: Combust

Goal:

- confirm `power.runtime_state.hp_loss` is exported for `Combust`

Recommended path:

```text
START ironclad 0
scenario fight jaw_worm
STATE
scenario power add player combust 1
WAIT 10
STATE
```

Notes:

- `scenario power add` queues a normal in-game power application
- `WAIT` is used here only to let the queued action resolve before the next `STATE`
- do **not** use `scenario deck add combust ...` as the primary path for this slice
  during combat; `deck add` only updates `masterDeck`, not the current combat zones
- if `Combust` is still missing after one `WAIT 10`, repeat:

```text
WAIT 10
STATE
```

Success criteria:

- `game_state.combat_truth.player.powers[*].id == "Combust"`
- same power has:
  - `runtime_state.hp_loss`

## Slice 4: Stasis

Goal:

- confirm `power.runtime_state.card_uuid` is exported for `Stasis`

Recommended path:

```text
START ironclad 0
scenario fight automaton
STATE
END
STATE
```

Notes:

- `Stasis` is less deterministic than the first three slices
- use `automaton` because `Bronze Orb` is the practical source of `Stasis`
- `WAIT 10` is optional here; if the monster actions have visibly resolved and
  a fresh `STATE` already returns the next frame, you do not need an extra wait
- if the first enemy turn is not enough, repeat:

```text
STATE
END
WAIT 10
STATE
```

Success criteria:

- some monster power has `id == "Stasis"`
- that power has:
  - `runtime_state.card_uuid`

## Recording discipline

For the first pass, do not try to build long replays.

For each slice:

1. start a fresh run
2. enter the target combat
3. capture the first state frame that proves the new `runtime_state` field exists
4. archive the raw JSON or the corresponding `manual_client_latest.json`

This is enough to prove:

- `CommunicationMod` exports the new truth shape
- Rust strict importer has a real sample to consume

## Failure checklist

If `scenario` commands do not work:

1. run `/commands` and confirm `scenario` is listed
2. run `STATE` and confirm:
   - `protocol_meta.capabilities.scenario_control == true`
3. confirm `config.properties` points to:
   - `launch_manual_client.ps1`
4. confirm the bridge wrote:
   - `logs/current/manual_client_latest.json`

If `Combust` does not appear after `scenario power add`:

1. make sure you are already in `room=MonsterRoom/COMBAT`
2. run `WAIT 10`
3. run `STATE` again
4. use `/show` to inspect whether the queued action has resolved
5. do not switch to `scenario deck add` for this slice unless you also plan to
   start a fresh combat afterward

If `Stasis` does not appear quickly:

1. keep the fight running for another enemy turn
2. re-check after each `END` + `WAIT 10` + `STATE`
3. do not block the whole migration on `Stasis`; land `GuardianThreshold`, `Angry`, and `Combust` first
