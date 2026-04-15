# sts_simulator

A work-in-progress Slay the Spire engine reimplementation in Rust.

The goal is to build a headless, deterministic simulation core that can eventually
serve as a fast-forward environment for search-based AI (MCTS, RL). "Eventually"
is doing heavy lifting here — the project is nowhere near complete, and help is needed.

> **⚠️ This is an early-stage project. Lots of game mechanics are missing, incomplete,
> or subtly wrong. There is no interactive interface. Contributions are very welcome —
> see below for how you can help.**

## Why this exists (and how it differs from alternatives)

Projects like [slay-the-model](https://github.com/wkzMagician/slay-the-model) offer
a clean Python framework with TUI, localization, LLM integration, and human-friendly
design. This project aims at something different:

| | slay-the-model (Python) | this project (Rust) |
|---|---|---|
| Design goal | Clean architecture, LLM/human-friendly | Raw simulation speed for search-based AI |
| Verification | Manual / ad-hoc | Differential testing against real Java traces |
| RNG | Independent impl | Java-compatible `SplittableRandom` (same seed → same outcome) |
| UI | ✅ TUI + i18n | ❌ None (headless only) |
| LLM support | ✅ Built-in | ❌ Not yet |
| Maturity | More complete | Very incomplete |

**If you want something usable today**, use slay-the-model. This project's bet is that
a Rust simulation core with verified RNG parity and differential testing will matter
for serious AI research — but we're not there yet.

## Honest status

The biggest problem is **not knowing what we don't know**. Files exist for many
entities, but "file exists" ≠ "correct implementation":

### What has been tested against real game traces

The **differential test driver** (`diff_driver.rs`) replays `.jsonl` state traces
captured from the real Java game and compares every field (HP, block, buffs, card
positions, RNG seeds) after each player action. This is the only source of truth.

- **A few Ironclad-only full runs** have achieved 100% combat parity — meaning every
  player action in those runs produces identical output in Rust vs Java
- **Map generation** is seed-accurate, verified against
  [sts_map_oracle](https://github.com/Ru5ty0ne/sts_map_oracle)
- **RNG** (SplittableRandom, Xoshiro256**) is verified at the seed level

### What exists but is not trustworthy

- **Powers**: Files exist for many powers, but hook dispatch may be incomplete or
  fire in the wrong order. Some hooks are missing entirely. Hard to quantify because
  powers interact with actions in complex ways
- **Relics**: Similar story — files exist, but counter logic (reset-on-combat-start,
  increment-on-X) is likely wrong in many cases. Scattered engine checks
  (`hasRelic("Ginger")` buried in action code) are particularly easy to miss
- **Actions**: ~120 action variants are defined, but the Rust fields don't always
  mirror Java's fields (Rust uses enum dispatch where Java uses inheritance), so
  subtle behavior differences are expected
- **Monsters**: Exordium and City AI is implemented and partially tested; Beyond is
  incomplete
- **Events, Potions, Orbs, Stances**: Files exist, partially functional, not verified

### What is known missing

- **Silent / Defect / Watcher cards**: Barely started
- **Non-combat game flow**: The run loop has framework for map → combat → reward →
  shop → campfire → event, but the differential driver only tests combat. Whether a
  full run can complete end-to-end without crashing is unclear
- **Interactive interface**: No TUI, no text output. You literally cannot "play" the
  game — it's a pure state machine with no human-facing output
- **Engine completeness**: The core loop mirrors Java's `GameActionManager`, but we
  don't have a systematic way to know which action handlers are incomplete or which
  hook call sites are missing. This is the hardest problem and the main reason
  contributors would be valuable

## Architecture

The Rust codebase uses **enum dispatch** (flat enums + match arms) where Java uses
inheritance hierarchies. A Java `AbstractPower` subclass becomes a `PowerId` enum
variant, with hook behavior dispatched via `match` in centralized handler functions.

```
src/
├── runtime/               # Core runtime primitives
│   ├── action.rs          # Action enum definition
│   ├── combat.rs          # CombatState and runtime entities
│   └── rng.rs             # Java-compatible RNG
├── engine/                # Core game loop, action dispatch, turn management
│   ├── core.rs            # Action queue tick loop (mirrors Java GameActionManager)
│   └── action_handlers/   # Action::* variant handlers
├── content/               # Game content (per-entity logic)
│   ├── cards/             # Card use() implementations
│   ├── powers/            # Power hook dispatch functions
│   ├── relics/            # Relic hook dispatch functions
│   ├── monsters/          # Monster AI (intent selection, takeTurn)
│   └── ...                # potions, events, orbs, stances
├── state/                 # Run / combat / pending-choice state
├── diff/                  # Protocol mapping, replay, and state sync
│   ├── protocol/          # Java/protocol parsing + snapshot shaping
│   ├── replay/            # Replay execution + diff verification
│   └── state_sync/        # Protocol -> runtime state construction
├── bot/                   # Search, policy, harness, sidecars
├── cli/                   # Live-comm runtime/admin/tooling
└── map/                   # Procedural map generation (seed-deterministic)
```

The current structure is intentionally layered:

- `runtime / engine / content / state / map`
  - core runtime truth
- `diff / fixtures`
  - integration layers around protocol sync, replay, and verification
- `bot / cli / src/bin/*`
  - app-layer consumers and workbenches

For the up-to-date ownership map, read [docs/REPOSITORY_MAP.md](docs/REPOSITORY_MAP.md).
For the hard dependency rules, read [docs/LAYER_BOUNDARIES.md](docs/LAYER_BOUNDARIES.md).

## Verification methodology

The differential test driver works as follows:

1. **Capture**: Run the real Java game with a
   [modified CommunicationMod](https://github.com/ForgottenArbiter/CommunicationMod)
   that exports full game state snapshots (every entity's HP, buffs, cards, RNG state)
   as `.jsonl` after each player action
2. **Replay**: `diff_driver.rs` loads these traces. For each action, it syncs Rust
   state from the Java snapshot, executes the same player decision in the Rust engine,
   then compares the resulting state field by field
3. **Assert**: Any mismatch → failing test, early stop, detailed divergence report

**Important limitation**: The driver currently only tests combat sequences. Non-combat
flow (rewards, map, shop, events) is not covered by differential testing. This means
bugs in those systems can go unnoticed.

```bash
# Requires .jsonl trace files in tools/replays/ (not distributed — contains game data)
cargo test test_diff_rng_replay -- --nocapture
```

## Building

```bash
cargo build --release
cargo test
```

## Porting tools

The `tools/` directory contains Python utilities for analyzing Java source code,
used when porting new entities to Rust. They require a local copy of the decompiled
Java source (not distributed).

| Tool | Purpose |
|------|---------|
| `sts_tool/` | Tree-sitter based: structured AST extraction + cross-file call chain + Rust parity check |
| `hook_query.py` | Hook override finder with liveness detection |
| `coverage/` | HTML dashboard showing which hooks are implemented |

```bash
# Requires: pip install tree-sitter tree-sitter-java
# Requires: decompiled Java source
cd tools && python -m sts_tool query ApplyPower
```

## How you can help

### If you know StS mechanics
- **Audit an action handler**: Pick an `Action::*` variant in `action_handlers.rs`,
  use `python -m sts_tool query <ActionName>` to see the Java logic, and check if
  the Rust implementation matches
- **Port a missing card/power/relic**: Same workflow — tool gives you the structured
  Java logic and cross-file dependencies
- **Identify scattered logic**: The Java engine buries `hasRelic("X")` and
  `hasPower("Y")` checks in unexpected places. Finding and porting these is tedious
  but critical

### If you know Rust
- **Add a simple TUI or text dump**: Even printing "Turn 3: Player plays Strike
  on Jaw Worm" would make debugging dramatically easier
- **Run loop testing**: Try running a full simulated run (map → combat → reward → ...)
  and see where it crashes or produces nonsensical state. File issues.
- **Improve the differential driver**: Extending it to cover non-combat flow
  (rewards, events, map transitions) would catch bugs that currently go undetected

### If you know about AI/RL
- **Design an interface**: What observation/action format would make this useful as
  a gym-like environment? How should the state be exposed?
- **Integration with existing frameworks**: Similar to how
  [bottled_ai](https://github.com/xaved88/bottled_ai) interfaces with the real game,
  design how an AI would interact with this simulation

There is no active learned policy path in the repo right now. Older reward-ranker and
local combat learner experiments were removed because they had fallen behind the
current audit-driven strategy stack. If ML/RL work resumes, it should start from the
current audit truth sources documented in [docs/LEARNING_TRUTH_SOURCES.md](docs/LEARNING_TRUTH_SOURCES.md),
not from restored legacy scripts. For the current repo structure and RL-facing main
path, start with [docs/REPOSITORY_MAP.md](docs/REPOSITORY_MAP.md).

## Related Projects

- [Slay the Spire](https://www.megacrit.com/) — The original game by MegaCrit
- [CommunicationMod](https://github.com/ForgottenArbiter/CommunicationMod) — Java mod
  for exporting game state traces
- [bottled_ai](https://github.com/xaved88/bottled_ai) — Automated StS play via
  CommunicationMod
- [sts_map_oracle](https://github.com/Ru5ty0ne/sts_map_oracle) — Pitch-perfect map
  generation; used to verify our map generator
- [slay-the-model](https://github.com/wkzMagician/slay-the-model) — Python StS
  framework with TUI, localization, and LLM support

## License

MIT
