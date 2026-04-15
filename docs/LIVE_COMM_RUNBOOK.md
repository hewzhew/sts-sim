# Live Comm Runbook

This is the default workflow for Rust `live_comm` development and bug-fixing.

If the workflow changes, update this file in the same change.

Also update these when relevant:
- `tools/live_comm/README.md`
- `docs/testing_platform.md`
- `docs/COMM_PROTOCOL_REWARD_SESSION_DRAFT.md`
- `docs/LIVE_COMM_MODES.md`
- `docs/PROTOCOL_TRUTH_RULES.md`
- `tools/live_comm/profile.json` examples

## One-Time Setup

Do this once per machine, not per run.

1. Point `CommunicationMod` at the launcher script instead of a hard-coded Rust command:

```powershell
powershell -ExecutionPolicy Bypass -File D:\rust\sts_simulator\tools\live_comm\install_communicationmod_config.ps1
```

2. After that, do not hand-edit:

`C:\Users\17239\AppData\Local\ModTheSpire\CommunicationMod\config.properties`

Day-to-day changes should go through:

- `tools/live_comm/profile.json`
- `tools/live_comm/profiles/*.json`
- Rust code
- Java mod code when needed

## Single Source of Truth

To avoid forgetting scattered updates, use these ownership rules:

- Live launch args:
  - `tools/live_comm/profile.json`
  - `tools/live_comm/profiles/*.json`
- CommunicationMod command wrapper:
  - `tools/live_comm/launch_live_comm.ps1`
  - `tools/live_comm/use_profile.ps1`
- Rust live loop:
  - `src/cli/live_comm/`
- Live testing / watch / capture docs:
  - this file
  - `docs/testing_platform.md`
  - `docs/LIVE_COMM_MODES.md`

## Mode First

Before changing watch targets, choose the mode for the run:

- `engine`
- `assisted_progression`
- `survival`
- `handoff`

See:

- `docs/LIVE_COMM_MODES.md`

Expected workflow:

- user chooses the mode
- Codex chooses the concrete watch targets, fixture extraction, and follow-up

Do not default to inventing low-level watch flags by hand every run.

## Before You Run

Use this checklist every time.

1. If you changed Rust code that affects live behavior:
   - `cargo fmt`
   - `cargo test cli::live_comm -- --nocapture`

2. If you changed Rust code that `CommunicationMod` will execute:
   - `cargo build --release --bin play`

3. If you changed Java `CommunicationMod` code:
   - rebuild the mod jar
   - deploy it to the game `mods` directory
   - make sure the game is actually loading the new jar

4. If you changed the active watch/capture target:
   - update `tools/live_comm/profile.json`
   - or switch with `tools/live_comm/use_profile.ps1`

5. Validate what the launcher will actually run:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File D:\rust\sts_simulator\tools\live_comm\launch_live_comm.ps1 -DryRun
```

Do not skip this if you recently changed:
- `profile.json`
- `play.exe`
- launcher paths

## Default Rust Edit Checklist

When adding or moving Rust modules, check these immediately:

1. Parent `mod.rs` updated
2. Imports updated
3. `cargo fmt`
4. Minimal targeted tests
5. `cargo build --release --bin play` if live will use it

Common places to forget:
- `src/cli/live_comm/mod.rs`
- `src/diff/state_sync/mod.rs`
- new `tests/...` driver coverage if a new workflow entrypoint was added

## Default Java Edit Checklist

When changing `CommunicationMod`:

1. Rebuild the jar
2. Confirm the mod path the game is loading
3. Run one real `live_comm`
4. Check Rust logs to verify the new protocol field/command actually arrived

Current Java build/deploy path:

```powershell
cd D:\rust\CommunicationMod
mvn -q -DskipTests package
Copy-Item -LiteralPath 'D:\rust\CommunicationMod\target\CommunicationMod.jar' -Destination 'C:\Program Files (x86)\Steam\steamapps\common\SlayTheSpire\mods\CommunicationMod.jar' -Force
```

Do not trust a successful Java compile by itself.

## Running a Live Comm Session

1. Update `tools/live_comm/profile.json`
   - or activate a template with:

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File D:\rust\sts_simulator\tools\live_comm\use_profile.ps1 Ironclad_Progression
```

2. Dry-run the launcher
3. Start the game
4. Let `CommunicationMod` launch Rust
5. Run the scenario you care about

Useful first-pass profiles:

- `Ironclad_Progression`
  - broader normal-climb sampling
  - intentionally noisier
- `Ironclad_Reaper`
  - narrow combat/play capture for a high-value rare card
- `Ironclad_Barricade`
  - narrow combat/play capture for a high-value rare card

## After Each Run

Always inspect these in order:

1. `live_comm_debug.txt`
2. `live_comm_watch_audit.jsonl`
3. `live_comm_watch_noncombat.jsonl`
4. `tests/live_captures/`

Then list captures:

```powershell
python D:\rust\sts_simulator\tools\analysis\live_regression.py watch-list --limit 10
```

If a capture looks useful:

```powershell
python D:\rust\sts_simulator\tools\analysis\live_regression.py watch-minimize-latest
```

Or copy the `suggested_minimize_cmd` from `live_comm_watch_audit.jsonl`.

If the current watch profile missed the bug, extract directly from the raw response window:

```powershell
python D:\rust\sts_simulator\tools\analysis\live_regression.py extract `
  --from-response-id 524 `
  --to-response-id 543 `
  --failure-frame 543 `
  --field player.energy `
  --out D:\rust\sts_simulator\tests\live_captures\champ_energy.json
```

Notes:
- `--field ...` now auto-crops the extracted response window around the latest matching diff with a short lookback
- extracted assertions carry `response_id` / `frame_id` scope, so replay can target intermediate-state bugs
- extracted fixtures now include `provenance.debug_context_summary` and `provenance.aspect_summary`
- minimization now preserves aspect-tagged causal context by default
  - `energy_relics`, `damage_mod_relics`, `draw_exhaust_engine`, and `boss_mechanics` are protected during state shrinking
  - this is especially useful for long boss-fight bugs where naive shrinking would delete the relics or powers that actually explain the failure

Then minimize that targeted fixture:

```powershell
python D:\rust\sts_simulator\tools\analysis\live_regression.py minimize `
  --fixture D:\rust\sts_simulator\tests\live_captures\champ_energy.json `
  --failure-frame 543 `
  --field player.energy `
  --out D:\rust\sts_simulator\tests\live_captures\champ_energy.min.json
```

## How To Decide What Broke

Use this split:

- No captures at all:
  - check `profile.json`
  - run launcher `-DryRun`
  - verify `[CONFIG]` in `live_comm_debug.txt`
  - confirm you rebuilt `target/release/play.exe`

- Captures exist but are noisy:
  - consider switching from `Ironclad_Progression` to a narrow template
  - tighten watch flags
  - increase `--live-watch-match all`
  - tune `--live-watch-dedupe-window`

- Captures exist but are useless:
  - check whether the active profile is too broad for the current goal
  - improve watch conditions
  - improve audit payload/notes/assertions
  - do not immediately broaden to more targets

- Rust and Java disagree:
  - minimize first
  - fix engine/importer/content bug second
  - only then broaden capture scope

## Truth Rebuild Rule

Current live truth rebuild is importer-first.

Do not restore hidden runtime state by default through previous-state carry in:

- live comparator truth build
- replay rebuild
- reward audit combat rebuild

If a field is missing and parity now fails, treat that as:

- missing protocol truth
- importer gap
- or actual engine bug

Do not silently patch it with carry.

## Recommended Work Order

For current project state, prefer:

1. Run `Ironclad live_comm`
2. Confirm watch capture and minimize are actually helping
3. Fix real parity bugs
4. Adjust tooling only when the real run exposes friction
5. Return to `Silent` after the workflow proves useful

## Do Not Rely On Memory

If you changed any of these, explicitly ask whether the runbook/docs need an update:

- launch args
- required rebuild steps
- watch flags
- audit files
- capture locations
- minimize commands
- module ownership or moved files

This is a maintenance rule, not optional cleanup.
