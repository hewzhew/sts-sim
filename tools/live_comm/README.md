# Live Comm Launcher

Use this wrapper so `CommunicationMod` does not need a new command in
`config.properties` every time the Rust-side `live_comm` args change.

The launcher now treats stale runtime binaries as a first-class failure mode:

- by default it checks whether `play.exe` is older than Rust-side build inputs
- if stale, it rebuilds `play.exe` before launching
- run provenance records both the binary git sha and the repo head sha, plus
  whether the launcher considered the binary fresh

Full day-to-day workflow:

- [docs/live_comm/LIVE_COMM_RUNBOOK.md](D:\rust\sts_simulator\docs\live_comm\LIVE_COMM_RUNBOOK.md)
- [docs/live_comm/LIVE_COMM_MANUAL_SCENARIO_RUNBOOK.md](D:\rust\sts_simulator\docs\live_comm\LIVE_COMM_MANUAL_SCENARIO_RUNBOOK.md)
- [docs/live_comm/LIVE_COMM_MODES.md](D:\rust\sts_simulator\docs\live_comm\LIVE_COMM_MODES.md)
- [docs/live_comm/WATCH_PRESET_SCHEMA_DRAFT.md](D:\rust\sts_simulator\docs\live_comm\WATCH_PRESET_SCHEMA_DRAFT.md)

## One-time `config.properties` setup

Either set the command once to:

```text
powershell -ExecutionPolicy Bypass -File D:\rust\sts_simulator\tools\live_comm\launch_live_comm.ps1
```

or use the helper:

```powershell
powershell -ExecutionPolicy Bypass -File D:\rust\sts_simulator\tools\live_comm\install_communicationmod_config.ps1
```

After that, leave `C:\Users\17239\AppData\Local\ModTheSpire\CommunicationMod\config.properties` alone.

## Manual scenario console

For protocol-truth recording and `scenario ...` testing, switch
`CommunicationMod` to the manual bridge instead of the normal Rust
`play.exe` loop:

```powershell
powershell -ExecutionPolicy Bypass -File D:\rust\sts_simulator\tools\live_comm\install_manual_client_config.ps1
```

This points `config.properties` at:

```text
powershell -ExecutionPolicy Bypass -File D:\rust\sts_simulator\tools\live_comm\launch_manual_client.ps1
```

When the game starts, the bridge opens a separate console window. That
console accepts raw `CommunicationMod` commands and keeps a live copy of
the latest frame at:

- `logs/current/manual_client_latest.json`

To archive the current manual frame into a named sample:

```powershell
powershell -ExecutionPolicy Bypass -File D:\rust\sts_simulator\tools\live_comm\save_manual_sample.ps1 guardian_threshold
```

Useful first commands:

```text
START ironclad 0
STATE
scenario state
scenario fight jaw_worm
scenario deck add combust 1 0
```

Local REPL commands:

- `/help`
- `/show`
- `/commands`
- `/state`
- `/quit`

To switch back to the normal Rust live-comm client:

```powershell
powershell -ExecutionPolicy Bypass -File D:\rust\sts_simulator\tools\live_comm\install_communicationmod_config.ps1
```

## Day-to-day usage

Edit [profile.json](D:\rust\sts_simulator\tools\live_comm\profile.json):

- `purpose`
  - optional metadata for the current working mode
  - current intended values:
    - `engine`
    - `assisted_progression`
    - `survival`
    - `handoff`
- `aspects`
  - optional metadata for causal context worth preserving during extract/minimize
- `capture_policy`
  - optional metadata for how captures should be handled conceptually
- `exe_path`
  - optional explicit `play.exe` path
  - if missing or invalid, the launcher falls back to:
    - `target\release\play.exe`
    - `target\debug\play.exe`
- `args`
  - exact argument list passed to `play.exe`

Or switch to a checked-in template:

```powershell
powershell -ExecutionPolicy Bypass -File D:\rust\sts_simulator\tools\live_comm\use_profile.ps1 Ironclad_Progression
```

Available templates live under:

- `tools/live_comm/profiles/`

Current intended rotation:

- `Ironclad_Progression`
  - broader normal-climb sampling
- `Ironclad_Engine_Strict`
  - engine/parity debugging
  - enables strict mode and intentionally disables watch capture
- `Ironclad_Engine_Survey`
  - engine/parity bug harvesting
  - enables survey mode and intentionally disables watch capture
- `Ironclad_Assisted_Progression`
  - higher-leverage noncombat sampling with selective human help expected
  - current template keeps manual help on card rewards only
- `Ironclad_Assisted_Progression_BossHandoff`
  - same as assisted progression, but also enables boss-combat handoff via `--live-comm-human-boss-combat`
- `Ironclad_Reaper`
  - narrow rare-card capture
- `Ironclad_Barricade`
  - narrow rare-card capture

Terminology:

- these JSON files are currently best thought of as `run profiles`
- the watch/capture portion inside them is the beginning of a `watch preset`
- mode selection is documented separately in `docs/live_comm/LIVE_COMM_MODES.md`
- future schema direction is documented in `docs/live_comm/WATCH_PRESET_SCHEMA_DRAFT.md`

Example:

```json
{
  "exe_path": "D:\\rust\\sts_simulator\\target\\release\\play.exe",
  "args": [
    "--class",
    "ironclad",
    "--live-comm",
    "--live-comm-human-card-reward",
    "--live-comm-human-boss-combat",
    "--live-watch-match",
    "all",
    "--live-watch-room-phase",
    "COMBAT",
    "--live-watch-command-kind",
    "play",
    "--live-watch-card",
    "Reaper"
  ]
}
```

Strict engine-debug example:

```powershell
powershell -ExecutionPolicy Bypass -File D:\rust\sts_simulator\tools\live_comm\use_profile.ps1 Ironclad_Engine_Strict
```

This profile runs with:

- `--live-comm`
- `--live-comm-strict`

and no `--live-watch-*` flags. That means:

- the game still starts through `CommunicationMod` as usual
- Rust stops on the first combat parse/parity mismatch
- watch capture is effectively frozen for the run

Survey engine-debug example:

```powershell
powershell -ExecutionPolicy Bypass -File D:\rust\sts_simulator\tools\live_comm\use_profile.ps1 Ironclad_Engine_Survey
```

This profile runs with:

- `--live-comm`
- `--live-comm-parity-mode survey`

and no `--live-watch-*` flags. That means:

- the game still starts through `CommunicationMod` as usual
- Rust continues after mismatches so one run can expose multiple bugs
- watch capture is effectively frozen for the run

## Log Layout

`live_comm` now uses a run-first archive model:

- current mirror:
  - `logs/current/live_comm_*.{txt,json,jsonl}`
- canonical per-run archive:
  - `logs/runs/<run_id>/manifest.json`
  - `logs/runs/<run_id>/raw.jsonl`
  - `logs/runs/<run_id>/focus.txt`
  - `logs/runs/<run_id>/findings.json`
  - `logs/runs/<run_id>/combat_suspects.jsonl`
  - `logs/runs/<run_id>/signatures.jsonl`

Artifact roles:

- `focus.txt`
  - human triage entrypoint
  - starts with grouped findings and “where to look next”
  - keeps a chronological appendix underneath
- `findings.json`
  - machine-readable problem index
  - grouped by engine bug/content gap/validation failure family
- `debug.txt`, `raw.jsonl`, `replay.json`
  - evidence layer for deep inspection

Heuristic/search disagreement records such as `high_risk_suspect` stay in
`combat_suspects.jsonl` and `failure_snapshots.jsonl`, not in `focus.txt`.

Derived artifacts such as `replay.json` are now cache-like:

- retained automatically for tainted/failing runs
- disposable for clean runs
- regenerable from `raw.jsonl`

Operator entrypoint:

```powershell
cargo run --bin sts_dev_tool -- logs status
```

Useful commands:

```powershell
cargo run --bin sts_dev_tool -- logs gc
cargo run --bin sts_dev_tool -- logs latest --artifact raw
cargo run --bin sts_dev_tool -- logs latest --artifact findings
cargo run --bin sts_dev_tool -- logs inspect-findings
cargo run --bin sts_dev_tool -- logs inspect-findings --category engine_bug --family Strength
cargo run --bin sts_dev_tool -- logs replay <run_id>
```

`inspect-findings` behavior:

- prefers archived `findings.json`
- if a run predates `findings.json` but still has `failure_snapshots.jsonl`, it synthesizes a temporary findings view on the fly
- prints both suggested artifacts and suggested Rust/Java source files for the family

Recommended triage flow:

1. open archived `focus.txt`
2. use `logs inspect-findings` to narrow to one family
3. only then open `debug.txt`, `failure_snapshots.jsonl`, or `replay.json`

## Why this is better

- `config.properties` only changes once
- the active `live_comm` profile lives inside the repo
- standard templates reduce “forgot to update the right watch args” mistakes
- different watch setups become normal file edits instead of AppData edits
- this avoids fighting `CommunicationMod`'s current whitespace-only command split

## Safe validation

Before launching the game, you can verify what the wrapper will run:

```powershell
powershell -ExecutionPolicy Bypass -File D:\rust\sts_simulator\tools\live_comm\launch_live_comm.ps1 -DryRun
```

`-DryRun` now also shows:

- `repo_head_short`
- `latest_input_path`
- `latest_input_write_utc`
- `binary_is_fresh`

If you intentionally want to bypass the freshness gate:

```powershell
powershell -ExecutionPolicy Bypass -File D:\rust\sts_simulator\tools\live_comm\launch_live_comm.ps1 -SkipFreshBuild
```

That should be rare. The default workflow is to let the launcher rebuild a stale
`target/release/play.exe` or `target/debug/play.exe` automatically.

You can also preview the exact `config.properties` update without writing:

```powershell
powershell -ExecutionPolicy Bypass -File D:\rust\sts_simulator\tools\live_comm\install_communicationmod_config.ps1 -DryRun
```
