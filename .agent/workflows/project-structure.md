---
description: >
  Reference for the sts_sim Rust project directory structure. Consult this
  when adding new files, modules, or deciding where code belongs. Also covers
  naming conventions and what NOT to commit.
---

# sts_sim — Project Structure Reference

## Source Code Layout (`src/`)

```
src/
├── lib.rs              ← Crate root. Module declarations + pub use aliases.
├── main.rs             ← Demo binary (uses sts_sim:: lib crate).
├── interop.rs          ← PyO3 Python bindings (single file, boundary layer).
│
├── core/               ← Foundational types everything depends on
│   ├── schema.rs       ←   CardInstance, CardDefinition, CardCommand, CardType
│   ├── state.rs        ←   GameState, GamePhase, Player, Enemy
│   └── loader.rs       ←   CardLibrary, MonsterLibrary (JSON loading)
│
├── engine/             ← Game engine (split from 3500-line monolith)
│   ├── mod.rs          ←   Shared types (CommandResult, Condition), pub use re-exports
│   ├── commands.rs     ←   apply_command — card command interpreter
│   ├── combat.rs       ←   Combat flow, card play, enemy turns
│   ├── navigation.rs   ←   Map traversal, room transitions
│   ├── events.rs       ←   Event system integration (engine-side)
│   ├── potions_use.rs  ←   Potion effect application
│   └── tests.rs        ←   Engine unit tests
│
├── powers_mod/         ← Power system (buffs/debuffs)
│   ├── power_set.rs    ←   PowerSet, PowerDefinition, PowerLibrary
│   └── hooks.rs        ←   PowerId enum, hook dispatch (on_attack, on_block, etc.)
│
├── monsters/           ← Enemy definitions and encounter pools
│   ├── enemy.rs        ←   MonsterState, MonsterDefinition, Intent, AI logic
│   ├── encounters.rs   ←   Encounter pools, spawn_encounter (was dungeon.rs)
│   └── act_config.rs   ←   Per-act configuration tables
│
├── rooms/              ← Non-combat room logic
│   ├── shop.rs         ←   Shop generation, pricing, buy/sell
│   ├── campfire.rs     ←   Rest site options (rest, smith, lift, etc.)
│   └── events.rs       ←   Event definitions, event pool, event commands
│
├── dungeon_mod/        ← Map and progression
│   ├── map.rs          ←   Procedural map generation, SimpleMap, RoomType
│   └── rewards.rs      ←   Combat reward generation
│
├── items/              ← Collectible items
│   ├── relics.rs       ←   RelicDefinition, RelicInstance, trigger system
│   └── potions.rs      ←   PotionDefinition, PotionSlots, PotionLibrary
│
├── ai/                 ← RL training infrastructure
│   ├── encoding.rs     ←   State encoding for neural networks
│   ├── features.rs     ←   Observation space, action mask
│   ├── card_features.rs ←  Per-card feature extraction
│   └── mcts.rs         ←   Monte Carlo Tree Search
│
├── bin/                ← Additional binary targets
│   ├── e2e_test.rs     ←   End-to-end integration test runner
│   ├── interactive_combat.rs ← Browser-based combat UI
│   └── ...
│
└── card_tests/         ← Card-specific test suites
    └── ...
```

## Path Aliasing Convention

`lib.rs` uses `pub use` aliases so internal code keeps flat paths:

```rust
// In lib.rs:
pub use core::schema;      // allows `use crate::schema::CardInstance;`
pub use monsters::enemy;   // allows `use crate::enemy::MonsterState;`
```

**Rule**: When adding a new module to a directory, add a `pub use` alias in
`lib.rs` if ANY existing code uses `crate::module_name`.

## Where to Put New Code

| Type of code | Directory | Example |
|---|---|---|
| New card mechanic / command | `engine/commands.rs` | Adding `CardCommand::Scry` |
| New power (buff/debuff) | `powers_mod/hooks.rs` | Adding `PowerId::Thorns` |
| New enemy / encounter | `monsters/` | Adding `enemy.rs` entry + `encounters.rs` pool |
| New relic | `items/relics.rs` | Adding `RelicDefinition` for new relic |
| New potion | `items/potions.rs` | Adding `PotionCommand` variant |
| New room type logic | `rooms/` | Adding a new room handler |
| New AI feature | `ai/` | Adding reward shaping |
| New test | `card_tests/` or `engine/tests.rs` | Card interaction tests |
| Python bindings | `interop.rs` | Adding new `#[pymethods]` |

## Project Root (`sts_sim/`)

```
sts_sim/
├── Cargo.toml          ← Rust dependencies
├── pyproject.toml      ← Python/maturin build config
├── data/               ← Game data JSONs (cards, monsters, events, etc.)
├── scripts/            ← Python helper scripts (analysis, coverage, etc.)
├── static/             ← Static assets (HTML for interactive combat)
├── _archive/           ← Old/experimental code, kept for reference
├── tools/              ← Dev tools
├── src/                ← Source code (see above)
└── .agent/workflows/   ← Antigravity workflow definitions
```

## What NOT to Commit

`.gitignore` covers these, but as a reminder:
- `*_results*.txt` — simulation output logs (regenerable)
- `build_output.txt` — compiler output captures
- `/target` — Rust build artifacts
- `/.venv` — Python virtual environment

## Data Scripts (`data/scripts/`)

| Script | Purpose |
|--------|---------|
| `card_db.py` | Cross-reference query tool (power-cards, turn-trigger, power-names, validate, search, sql) |
| `extract_powers.py` | Parse Java source → generate `data/powers_spec.json` |

### Power Naming Convention

- **In card JSON `ApplyPower`**: Use the Java `POWER_ID` string (e.g. `"Demon Form"` with space)
- **In `hooks.rs` `from_str`**: Both camelCase and spaced forms should be accepted (e.g. `"DemonForm" | "Demon Form"`)
- **Stack values**: Must match Java card's `ApplyPowerAction` 4th arg, NOT the card description
  - Example: Combust stacks = `magicNumber` (5/7), NOT hpLoss (1)

### Power Card JSON Rules

Power cards with TurnTrigger effects should:
1. Have an explicit `ApplyPower` command with correct stacks
2. NOT include recurring commands (GainEnergy, DealDamageAll, etc.) — those belong in hooks.rs
3. Only include truly immediate effects alongside ApplyPower
