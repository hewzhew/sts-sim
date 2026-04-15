# Testing Platform Direction

Operational workflow for live runs:

- `docs/LIVE_COMM_RUNBOOK.md`
- `docs/LIVE_COMM_MODES.md`
- watch-preset terminology and schema draft:
  - `docs/WATCH_PRESET_SCHEMA_DRAFT.md`

## Current Layering

- `sts_simulator::testing::scenario`
  - Canonical fixture schema
  - Shared replay/assertion logic
  - Backward-compatible with existing live regression fixtures
- `tests/live_regression_driver.rs`
  - File-based fixture discovery only
  - Delegates execution to the shared scenario module

## Canonical Fixture Shape

All producers should converge on `ScenarioFixture`.

- `kind`
  - `combat`
- `oracle_kind`
  - `live`
  - `synthetic`
  - `java_harness`
- `initial_game_state`
  - Current wire-compatible Java snapshot payload
- `initial_protocol_meta`
  - Optional protocol context from `live_comm`
- `steps`
  - Command strings for now
- `assertions`
  - Stable field assertions
  - Optional response/frame-scoped assertions for intermediate-state bugs
- `provenance`
  - Source path, response range, failure frame, notes
  - Optional debug context summary and aspect summary for human triage
- `tags`
  - Human and agent discoverability

## Planned Producers

### Live Producer

Input:
- `live_comm` logs
- optional debug assertion hints
- optional live watch flags from `play --live-comm`

Output:
- `ScenarioFixture { oracle_kind = live }`

Notes:
- Should support both failure-driven extraction and watchpoint-driven capture.
- Python can remain a wrapper, but schema validation should stay in Rust.
- The first Rust-side watchpoint implementation now exists under `src/cli/live_comm/`.
- If launch steps, watch flags, capture locations, or minimize workflow change, update `docs/LIVE_COMM_RUNBOOK.md` in the same change.
- Current flags:
  - `--live-watch-card <java_id>`
  - `--live-watch-relic <java_id>`
  - `--live-watch-power <java_id>`
  - `--live-watch-monster <java_id>`
  - `--live-watch-screen <screen>`
  - `--live-watch-room-phase <phase>`
  - `--live-watch-command-kind <kind>`
  - `--live-watch-match <any|all>`
  - `--live-watch-window <response_count>`
  - `--live-watch-dedupe-window <response_count>`
  - `--live-watch-max <capture_count>`
  - `--live-watch-dir <path>`
- Current output:
  - captured fixtures under `tests/live_captures/` by default
  - audit sidecar at `live_comm_watch_audit.jsonl`
  - noncombat sidecar audit at `live_comm_watch_noncombat.jsonl`
  - replayable combat captures are deduped by a lightweight signature over matched watch tags/assertions plus screen/room-phase/command-kind; the default cooldown is `3` responses and can be tuned with `--live-watch-dedupe-window`
  - `python tools/analysis/live_regression.py watch-list` now lists recent watch captures from both audit files
    - supports `--tag`, `--response-id`, `--path-contains`, and `--require-existing-path`
  - `python tools/analysis/live_regression.py minimize --fixture <watch_fixture.json> ...` now accepts Rust-generated watch fixtures by reading `provenance.response_id_range`
  - `python tools/analysis/live_regression.py watch-minimize-latest` now minimizes the newest replayable combat watch capture into a sibling `.min.json`
    - supports `--tag`, `--response-id`, and `--path-contains` to target a specific capture
  - `python tools/analysis/live_regression.py extract --from-response-id ... --to-response-id ... --field <field>` can now target a specific engine-bug field by picking the latest matching diff inside the selected response window
    - useful when the final frame shows one diff (for example `monster[0].hp`) but the real bug you want to minimize happened earlier in the same window (for example `player.energy`)
    - extracted assertions now carry `response_id` / `frame_id` scope, so replay can assert intermediate-state bugs instead of only the final state
    - when `--field` is used, extraction now auto-crops the response window around the latest matching diff using a short lookback
    - fixtures now also include `provenance.debug_context_summary` and `provenance.aspect_summary` for faster human triage
  - minimization is now aspect-aware by default
    - `energy_relics`, `damage_mod_relics`, `draw_exhaust_engine`, and `boss_mechanics` from `provenance.aspect_summary` are treated as protected causal context during state shrinking
    - this keeps fixtures closer to the original bug family instead of over-minimizing into a different failure with the same final field
  - combat audit rows include `suggested_minimize_out_path` and `suggested_minimize_cmd` for one-step follow-up
- Current limitation:
  - combat frames emit replayable fixtures with extra context assertions
  - noncombat frames now emit sidecar JSON records instead of replay fixtures
  - sidecars include a `screen_summary` for common screens such as `EVENT`, `CARD_REWARD`, `COMBAT_REWARD`, `SHOP_SCREEN`, `MAP`, `REST`, and `GRID`

### Synthetic Producer

Input:
- human-authored or agent-authored declarative spec

Output:
- compiled `ScenarioFixture { oracle_kind = synthetic }`

Notes:
- Prefer a higher-level author spec over direct raw snapshot editing.
- The first version can stay combat-only.

Example author spec:

```json
{
  "name": "silent_neutralize",
  "player_class": "Silent",
  "player": {
    "energy": 3
  },
  "monsters": [
    { "id": "JawWorm", "current_hp": 40 }
  ],
  "hand": ["Neutralize"],
  "steps": ["PLAY 1 0"],
  "expect": [
    { "field": "monster[0].hp", "number": 37 },
    { "field": "monster[0].power[Weakened].amount", "number": 1 }
  ],
  "tags": ["silent", "starter"]
}
```

Current authoring conveniences:
- `hand` / `draw_pile` / `discard_pile` / `exhaust_pile` accept either a Java card id string or an object with `id`, `upgrades`, `cost`, `misc`, `count`
- `draw_pile` is authored in natural top-first order; the compiler reverses it into Java snapshot order
- assertions use `number`, `string`, or `missing: true` instead of the lower-level replay fields
- assertions also support higher-level structured forms that still compile into the shared low-level `ScenarioAssertion` layer:
  - `{ "monster_count": 2 }`
  - `{ "pile_contains": { "pile": "discard", "id": "Survivor" } }`
  - `{ "pile_count": { "pile": "hand", "id": "Shiv", "count": 2 } }`
  - `{ "pile_size": { "pile": "draw", "count": 5 } }`
  - `{ "player_stat": { "stat": "energy", "value": 3 } }`
  - `{ "player_power": { "id": "Strength", "amount": 2 } }`
  - `{ "monster_stat": { "monster": 0, "stat": "hp", "value": 40 } }`
  - `{ "monster_power": { "monster": 0, "id": "Poison", "amount": 10 } }`
  - `{ "monster_missing": 2 }`
  - `{ "has_relic": "SnakeRing" }`
  - `{ "relic_count": { "id": "SnakeRing", "count": 1 } }`
- raw command strings are still accepted for `steps`
- structured steps are now supported:
  - `{ "play": { "card": 1, "target": 0 } }`
  - `{ "play": { "card": "Acrobatics" } }`
  - `{ "play": { "card": { "id": "Strike_R", "occurrence": 2 }, "target": 0 } }`
  - `{ "end": true }`
  - `{ "potion_use": { "slot": 0, "target": 0 } }`
  - `{ "human_card_reward": { "choice": 1 } }`
  - `{ "human_card_reward": { "skip": true } }`
  - `{ "hand_select": { "cards": ["Survivor"] } }`
  - `{ "grid_select": { "cards": [{ "id": "Burn", "occurrence": 2 }] } }`
  - `{ "cancel": true }`
- card selectors support:
  - 1-based indices
  - Java card ids
  - Java card id + `occurrence` for duplicates in the current visible order

Current test entrypoints:
- `cargo test --test synthetic_scenario_driver`
- `SYNTHETIC_SCENARIO=... cargo test --test synthetic_scenario_driver replay_single_synthetic_scenario_from_env`
- sample file: `tests/synthetic_scenarios/silent_neutralize.json`

## Local Combat Lab

For fixed boss-state local experiments, use the combat lab on top of a live-captured fixture.

Workflow:

1. Extract or identify a boss handoff fixture from `tests/live_captures/`
2. Sanitize it into a pure start-state fixture:
   - `python tools/analysis/live_regression.py sanitize-lab --fixture <handoff_fixture.json> --out <start_fixture.json>`
3. Run local bot rollouts:
   - `cargo run --bin combat_lab -- --fixture <start_fixture.json> --episodes 10 --policy bot --depth 6 --variant-mode reshuffle-draw --base-seed 1 --out-dir <dir>`
4. Render any trace to Markdown:
   - `python tools/analysis/render_combat_lab_trace.py --trace <dir>/trace_0000.json --out <dir>/trace_0000.md`

Outputs:
- `episodes.jsonl`
  - one line per episode with `won`, final HP totals, turn count, path score, and trace path
- `trace_XXXX.json`
  - full per-step trace with hand, draw/discard sizes, monster snapshots, chosen action, evaluator scores, and a compact `state_features_preview`
- `summary.json`
  - aggregate win rate / average final HP / best episode ids
- `best_win_trace.md` or `best_attempt_trace.md`
  - automatically rendered review artifact for the strongest run in the batch

Current scope:
- v1 is intentionally narrow:
  - fixed fixture start state only
  - `policy=bot` only
  - `variant_mode=exact|reshuffle_draw`
- `reshuffle_draw` only perturbs the initial draw pile ordering; it does not randomize the full combat RNG stream

### Java Harness Producer

Input:
- explicit debug commands in `CommunicationMod`

Output:
- Java-created state snapshot exported as `ScenarioFixture { oracle_kind = java_harness }`

Notes:
- This should reuse the same fixture schema rather than inventing another replay format.
- This remains a planned producer. The command surface below is a target shape, not a
  currently implemented `CommunicationMod` feature set.

## BaseMod-Inspired Debug Command Surface

These commands are the best first-stage target for `CommunicationMod`.

Status:
- Treat this section as a proposal for a future debug surface.
- Do not assume these commands already exist in the current Java mod.
- Current day-to-day debugging still relies primarily on `live_comm`, replay fixtures,
  watch captures, and scenario extraction rather than an implemented Java debug console.

### Combat State

- `debug hp <current> [max]`
- `debug block <amount>`
- `debug energy <amount>`
- `debug gold <amount>`
- `debug relic add <java_id>`
- `debug relic remove <java_id>`
- `debug power add player <java_id> <amount>`
- `debug power add monster <idx> <java_id> <amount>`

### Card Zones

- `debug hand add <java_id> [upgrades] [cost]`
- `debug draw add <java_id> [upgrades] [cost]`
- `debug discard add <java_id> [upgrades] [cost]`
- `debug exhaust add <java_id> [upgrades] [cost]`
- `debug hand clear`
- `debug draw clear`
- `debug discard clear`

### Monsters

- `debug monster spawn <java_id> [slot]`
- `debug monster hp <idx> <current> [max]`
- `debug monster block <idx> <amount>`
- `debug monster power add <idx> <java_id> <amount>`

### Export / Capture

- `debug snapshot`
- `debug scenario save <name>`
- `debug watch card <java_id>`
- `debug watch relic <java_id>`
- `debug watch power <java_id>`

## Near-Term Implementation Order

1. Keep moving replay/assertion code into `testing::scenario`.
2. Add a synthetic author spec that compiles into `ScenarioFixture`.
3. Add watchpoint-based live capture.
4. Add the first `debug_*` commands in `CommunicationMod`.

For protocol evolution around human reward audit and future debug/scenario hooks, see:

- [COMM_PROTOCOL_REWARD_SESSION_DRAFT.md](D:/rust/sts_simulator/docs/COMM_PROTOCOL_REWARD_SESSION_DRAFT.md)
5. Only then consider a richer interactive UI.
