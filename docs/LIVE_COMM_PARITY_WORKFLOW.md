# Live Comm Parity Workflow

This document records the intended workflow for Rust/Java parity debugging in
`live_comm`, so the mode split and profile usage do not regress back into
ad-hoc flag edits.

## Goals

Parity workflow must satisfy three constraints at the same time:

- `strict` mode must stop on the first real mismatch so engine bugs are obvious.
- `survey` mode must allow one run to expose multiple bugs.
- mismatch recovery must come from current Java truth rebuild, not from
  previous-state `carry`.

The key architectural rule is:

- do not use `sync/carry` as a bug-recovery mechanism
- if a run continues after mismatch, continuation must be based on Java
  snapshot truth

See also:

- [PROTOCOL_TRUTH_RULES.md](d:\rust\sts_simulator\docs\PROTOCOL_TRUTH_RULES.md)
- [STATE_SYNC_STATUS.md](d:\rust\sts_simulator\docs\STATE_SYNC_STATUS.md)

## Startup Chain

`CommunicationMod` starts Rust through the configured launcher command in
`config.properties`.

Current intended chain:

1. `CommunicationMod`
2. [launch_live_comm.ps1](d:\rust\sts_simulator\tools\live_comm\launch_live_comm.ps1)
3. [profile.json](d:\rust\sts_simulator\tools\live_comm\profile.json)
4. `play.exe` with the profile's `args`

This means parity mode should normally be chosen by switching profiles, not by
hand-editing the game config or retyping flags every run.

## Modes

### `strict`

Use this when the goal is:

- pinpoint the first engine/protocol bug
- stop as soon as a combat parse diff or combat parity fail appears
- make the symptom obvious enough that the next debugging prompt is concrete

Behavior:

- Rust exits on first combat mismatch
- run does not continue to collect later divergences

CLI form:

```text
--live-comm --live-comm-strict
```

Equivalent explicit form:

```text
--live-comm --live-comm-parity-mode strict
```

### `survey`

Use this when the goal is:

- collect several bugs from one run
- expose later-act content while bot strength is still limited
- mine multiple parity failures before stopping to fix

Behavior:

- Rust records the mismatch and continues
- continuation is based on current Java truth rebuild
- continuation must not rely on previous-state `carry`

CLI form:

```text
--live-comm --live-comm-parity-mode survey
```

This is currently the default if no strict flag is supplied.

## Engine Profiles

Checked-in engine-debug profile:

- [Ironclad_Engine_Strict.json](d:\rust\sts_simulator\tools\live_comm\profiles\Ironclad_Engine_Strict.json)
- [Ironclad_Engine_Survey.json](d:\rust\sts_simulator\tools\live_comm\profiles\Ironclad_Engine_Survey.json)

Purpose:

- use `strict` parity stop
- freeze `watch`
- keep the run surface narrow and low-noise

Activate it with:

```powershell
powershell -ExecutionPolicy Bypass -File D:\rust\sts_simulator\tools\live_comm\use_profile.ps1 Ironclad_Engine_Strict
```

Then launch the game normally through the existing `CommunicationMod`
configuration.

Survey variant:

```powershell
powershell -ExecutionPolicy Bypass -File D:\rust\sts_simulator\tools\live_comm\use_profile.ps1 Ironclad_Engine_Survey
```

Use this when one run should expose multiple parity bugs instead of stopping at
the first one.

## Why `watch` Is Frozen In Engine Debug Runs

For engine/parity debugging, `watch` is intentionally disabled by default.

Reason:

- `watch` is useful for strategy capture and later review
- `watch` is not needed to detect the first parity bug
- extra captures add noise when the current goal is "what is the first mismatch"

So engine-debug profiles should prefer:

- no `--live-watch-*` flags

Strategy-oriented profiles may still enable `watch`.

## Recommended Workflow

### First-pass engine debugging

Use `strict`:

1. switch to `Ironclad_Engine_Strict`
2. run the game
3. stop on first parity fail
4. fix the bug or missing protocol truth

### Bug harvesting

Use `survey`:

1. switch to a survey profile or add `--live-comm-parity-mode survey`
2. run one longer climb
3. collect multiple parity failures
4. fix the highest-value cluster first

## Rule For Recovery

If a run continues after mismatch:

- allowed: rebuild from current Java snapshot truth
- not allowed: recover by inheriting hidden runtime state from previous Rust
  state

This rule exists to prevent `sync/carry` from becoming a second semantic system.

## Near-Term Follow-Up

The next protocol-truth migration work should continue in this order:

1. consume already-exported runtime truth in Rust importer
2. add missing Java protocol fields for hidden monster/relic runtime
3. delete corresponding fallback/carry logic

Do not add new long-lived `carry` rules as a shortcut.

## Log Storage

`live_comm` no longer treats flat `logs/raw`, `logs/replays`, `logs/debug` as the
primary archive layout for new runs.

New intended model:

- latest mirror in `logs/current/`
- canonical archive in `logs/runs/<run_id>/`
- `manifest.json` is the source of truth for run classification and retention
- `replay.json` is a cache artifact and may be absent for clean runs

Operational commands live under:

```powershell
cargo run --bin sts_dev_tool -- logs status
```
