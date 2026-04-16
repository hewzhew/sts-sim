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
- `/quit`

The latest raw frame is mirrored to:

- `logs/current/manual_client_latest.json`

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
- `bronze_automaton`

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
- top-level `game_state.combat_state.monsters[*].id == "TheGuardian"`
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
scenario power add player combust 1
WAIT 5
STATE
```

Notes:

- `scenario power add` queues a normal in-game power application
- `WAIT` is used here only to let the queued action resolve before the next `STATE`

Success criteria:

- `game_state.combat_state.player.powers[*].id == "Combust"`
- same power has:
  - `runtime_state.hp_loss`

## Slice 4: Stasis

Goal:

- confirm `power.runtime_state.card_uuid` is exported for `Stasis`

Recommended path:

```text
START ironclad 0
scenario fight bronze_automaton
STATE
END
WAIT 10
STATE
```

Notes:

- `Stasis` is less deterministic than the first three slices
- use `bronze_automaton` because `Bronze Orb` is the practical source of `Stasis`
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

1. run `WAIT 10`
2. run `STATE` again
3. use `/show` to inspect whether the queued action has resolved

If `Stasis` does not appear quickly:

1. keep the fight running for another enemy turn
2. re-check after each `END` + `WAIT 10` + `STATE`
3. do not block the whole migration on `Stasis`; land `GuardianThreshold`, `Angry`, and `Combust` first
