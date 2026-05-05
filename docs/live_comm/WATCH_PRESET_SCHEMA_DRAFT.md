# Watch Preset Schema Draft

This document defines the next-step schema and terminology for `live_comm`
watch configuration.

The goal is to stop overloading the word `profile` to mean all of these at
once:

- how Rust is launched
- what signals are watched
- what kinds of bugs or strategy mistakes should be captured
- what should be preserved during extraction/minimization

## Terminology

Use these terms consistently.

### Run Profile

A `run profile` is the full launcher/runtime config currently stored in:

- `tools/live_comm/profile.json`
- `tools/live_comm/profiles/*.json`

It answers:

- which executable to launch
- what CLI args to pass
- which class / mode / watch flags are active

This is the operational unit for starting a session.

### Watch Preset

A `watch preset` is the semantic watch target configuration embedded inside a
run profile.

It answers:

- what the run is trying to observe
- whether the purpose is engine, strategy, handoff, or debug
- which aspects matter enough to preserve as causal context

This is the conceptual unit for capture design.

### Detector Preset

A `detector preset` is a named set of detection rules used during capture.

It answers:

- what counts as a notable event
- whether the event is a parity bug, strategy miss, handoff event, or future
  debug hook

This is the rule-selection unit.

### Capture Policy

A `capture policy` defines what to do once a detector or watch condition hits.

It answers:

- whether to write a replay fixture or audit-only record
- whether to auto-extract by field
- whether to preserve aspect-tagged context during minimization

This is the post-hit handling unit.

## Current State

Today the repo mostly has `run profiles` that also act as rough watch presets.

Example:

- `Ironclad_Progression`
- `Ironclad_Reaper`
- `Ironclad_Barricade`

These are useful, but they blur:

- launch configuration
- watch target set
- capture intent

That is acceptable for the first stage, but it is too weak for:

- engine parity triage
- strategy miss capture
- human/bot handoff workflows
- future debug/session-control hooks

## Proposed Layering

Use this layering going forward.

### Layer 1: Run Profile

Example shape:

```json
{
  "exe_path": "D:\\rust\\sts_simulator\\target\\release\\play.exe",
  "notes": "Boss-fight engine parity run",
  "args": [
    "--class",
    "ironclad",
    "--live-comm",
    "--live-comm-human-card-reward"
  ],
  "watch_preset": "Ironclad_Champ_Engine"
}
```

This file remains the launcher source of truth.

### Layer 2: Watch Preset

Proposed shape:

```json
{
  "name": "Ironclad_Champ_Engine",
  "purpose": "engine",
  "notes": "Boss-fight parity preset with energy/damage context preservation",
  "target": {
    "class": "ironclad",
    "watch_match": "any",
    "cards": ["Shrug It Off", "Offering", "Reaper"],
    "relics": ["SlaversCollar", "Boot", "Art of War"],
    "powers": ["Corruption", "Dark Embrace", "Feel No Pain"],
    "room_phases": ["COMBAT"],
    "screens": [],
    "command_kinds": []
  },
  "detector_preset": "engine_parity_truth_rebuild",
  "capture_policy": "replay_preserve_aspects",
  "aspects": [
    "energy_relics",
    "damage_mod_relics",
    "draw_exhaust_engine",
    "boss_mechanics"
  ]
}
```

### Layer 3: Detector Preset

Proposed shape:

```json
{
  "name": "engine_parity_truth_rebuild",
  "purpose": "engine",
  "rules": [
    "parity_fail",
    "truth_rebuild_sensitive",
    "spawn_identity_sensitive"
  ]
}
```

Example future detector presets:

- `engine_parity`
- `engine_parity_truth_rebuild`
- `strategy_power_frontload`
- `strategy_exhaust_selectivity`
- `strategy_status_engine`
- `reward_handoff`

### Layer 4: Capture Policy

Proposed shape:

```json
{
  "name": "replay_preserve_aspects",
  "capture_kind": "replay",
  "dedupe": {
    "enabled": true,
    "window": 6
  },
  "minimize": {
    "auto_suggest": true,
    "preserve_aspects": true,
    "suggest_field_extract": true
  },
  "sidecars": {
    "audit": true,
    "noncombat": true
  }
}
```

Example future capture policies:

- `audit_only`
- `replay_minimal`
- `replay_preserve_aspects`
- `debug_heavy_context`

## Purpose Values

`watch preset.purpose` should be one of:

- `engine`
- `strategy`
- `handoff`
- `debug`

These are for human intent, documentation, and future automation. They do not
need to change runtime semantics immediately.

## Aspect Values

Current recommended aspect keys:

- `energy_relics`
- `damage_mod_relics`
- `draw_exhaust_engine`
- `boss_mechanics`

These already map well onto current extraction/minimization context.

Future aspect keys may include:

- `spawn_identity`
- `hidden_monster_state`
- `queue_sensitive_runtime`
- `reward_session`
- `card_transform_runtime`

## Minimal Migration Plan

Do not implement the full layering at once.

Stage 1:

- keep current `run profile` files
- document that they are `run profiles`
- allow a conceptual `watch preset` section in docs first

Stage 2:

- add optional metadata fields to current profile JSON:
  - `purpose`
  - `aspects`
  - `capture_policy`
- no launcher behavior change required yet

Stage 3:

- move repeated watch target sets into named `watch preset` files
- let run profiles reference a preset by name

Stage 4:

- add detector preset wiring in Rust
- keep runtime behavior backward-compatible with raw CLI flags

## Why This Upgrade Is Worth Doing

This layering solves several current problems:

- `profile` stops meaning three different things
- strategy-oriented capture stops being hacked into engine-oriented presets
- minimization and extraction can use declared aspects instead of guessing only
  from raw context
- future reward-session and debug-command workflows get a clean place to hang
  intent metadata

## Near-Term Recommendation

For the current project stage:

- keep launcher behavior unchanged
- keep current `tools/live_comm/profiles/*.json`
- refer to them as `run profiles`
- start using `watch preset` as the conceptual term for the watch/capture part
- if a future schema change is made, prefer adding metadata fields before
  splitting files

That gives better terminology immediately without forcing a large migration.
