# Live Comm Modes

This document defines the default working modes for `live_comm`.

The main goal is to keep mode selection simple:

- the user chooses the mode
- Codex chooses the concrete watch target, extraction focus, and follow-up

This avoids pushing low-level watch decisions onto the user every run.

## Modes

There are four primary modes.

### `engine`

Use this when the goal is:

- fix Rust/Java parity bugs
- improve truth rebuild / state sync
- investigate hidden runtime mismatches

Typical watch focus:

- current monster or boss family
- suspected relic or power family
- truth-rebuild-sensitive runtime state
- spawn/order/identity issues

Typical outputs:

- replayable fixture
- field-scoped extract/minimize
- aspect-preserving minimization

Default aspects to preserve:

- `energy_relics`
- `damage_mod_relics`
- `draw_exhaust_engine`
- `boss_mechanics`

What not to do in this mode:

- do not broaden into “general strategy improvement”
- do not optimize for quantity of captures

### `survival`

Use this when the goal is:

- help the bot live longer
- reduce high-frequency tactical mistakes
- improve reward choice quality enough to reach later acts

Typical watch focus:

- powers not being frontloaded
- exhaust selectivity mistakes
- bad ordering on block-to-damage turns
- status-engine misses
- obvious reward-choice mistakes

Typical outputs:

- strategy-oriented captures
- targeted comparisons around a few high-value mistakes
- small policy/heuristic changes

Default aspects to preserve:

- `draw_exhaust_engine`
- `damage_mod_relics`
- `boss_mechanics`

What not to do in this mode:

- do not chase every parity diff
- do not widen to rare late-game situations if the bot still dies early

### `assisted_progression`

Use this when the goal is:

- push runs deeper with selective human help
- collect later-act data before the bot is strong enough to reach it alone
- turn high-leverage human decisions into future strategy work

Typical human intervention points:

- card rewards
- boss relic choice
- shops
- events
- optional elite or boss combat handoff when depth matters more than autonomy

Usually not worth prioritizing as a separate intervention point:

- ordinary relic reward screens that are effectively take-or-skip only

Typical watch focus:

- reward and shop decisions with large downstream impact
- event choices that strongly affect survivability
- late-act combats reached because the run was manually stabilized
- high-value strategy misses seen after the run is pushed deeper

Typical outputs:

- deeper-run audit data
- later-act bug exposure
- strategy candidates for reward/shop/event policy

Default aspects to preserve:

- `draw_exhaust_engine`
- `damage_mod_relics`
- `boss_mechanics`

What not to do in this mode:

- do not fully replace the bot in ordinary hallway fights
- do not treat every human intervention as something to automate immediately
- do not stop fixing engine bugs that obviously poison the collected data

### `handoff`

Use this when the goal is:

- improve human/bot switching
- improve reward audit and session continuity
- prepare future debug or scenario-control workflow

Typical watch focus:

- reward session behavior
- human card reward audit
- temporarily offscreen transitions
- session continuity across deck/map/boss inspection

Typical outputs:

- audit logs
- protocol field validation
- workflow fixes rather than combat fixes

Default aspects to preserve:

- `reward_session`
- `boss_mechanics`

What not to do in this mode:

- do not treat it like a combat balance pass
- do not broaden into generic parity hunting unless the handoff bug depends on it

## Default Decision Rule

When choosing a mode:

1. If Rust and Java disagree in a clear way, use `engine`
2. Else if you want to push runs deeper with selective human help, use `assisted_progression`
3. Else if the bot is simply making poor decisions and dying too early without heavy manual help, use `survival`
4. Else if the main frustration is human intervention / control-flow / switching, use `handoff`

If more than one mode seems relevant, pick only one for the run.

## Operator Split

Expected split of responsibility:

- user:
  - chooses the mode
  - runs the game
  - gives high-level goals or observations

- Codex:
  - chooses concrete watch targets
  - decides which fixture to extract
  - decides which field to minimize
  - decides whether the next step is code, protocol, or workflow

This is the intended operating model. The user should not need to manually
invent watch flags every run.

## Current Mapping To Existing Run Profiles

Current checked-in run profiles are still operational configs, not a full mode
system. For now:

- `Ironclad_Engine_Strict`
  - closest to `engine`
  - strict parity stop
  - no watch capture
- `Ironclad_Engine_Survey`
  - closest to `engine`
  - survey parity continuation
  - no watch capture
- `Ironclad_Progression`
  - closest to `survival`
- `Ironclad_Assisted_Progression`
  - closest to `assisted_progression`
- `Ironclad_Assisted_Progression_BossHandoff`
  - closest to `assisted_progression`
  - explicitly enables boss-combat handoff
- `Ironclad_HumanPrimary_Capture`
  - closest to `handoff`
  - explicitly enables human-primary noncombat hold while preserving bot shadow evaluation
- `Ironclad_Reaper`
  - targeted `survival`
- `Ironclad_Barricade`
  - targeted `survival`

For engine debugging, prefer `Ironclad_Engine_Strict` first. It keeps the run
surface small and avoids mixing parity debugging with watch/capture noise.

## Near-Term Migration

Near-term workflow:

- keep current launcher/profile behavior unchanged
- add lightweight metadata to run profiles:
  - `purpose`
  - `aspects`
  - `capture_policy`
- use this document as the decision layer

Longer-term workflow:

- let run profiles reference a named watch preset
- let watch presets reference a detector preset and capture policy

See also:

- `docs/live_comm/LIVE_COMM_RUNBOOK.md`
- `docs/live_comm/WATCH_PRESET_SCHEMA_DRAFT.md`
