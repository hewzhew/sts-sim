# Play Guide — `sts_simulator` Terminal Client

A reference for running, controlling, and debugging the terminal-based Slay the Spire
simulator. This is **not** a polished game — it's a testing/debugging harness for the
simulation engine.

## Quick Start

```bash
# Build
cargo build --release

# Run with defaults (seed=12345, asc=0, manual mode)
cargo run --release --bin play

# Run with specific seed and ascension
cargo run --release --bin play -- 42 15

# Full auto-play (bot plays everything, verbose output)
cargo run --release --bin play -- 42 0 --auto

# Full auto-play (silent, just see the result)
cargo run --release --bin play -- 42 0 --auto --silent
```

## Command Line Arguments

```
cargo run --bin play -- [SEED] [ASCENSION] [FLAGS...]
```

| Argument | Default | Description |
|----------|---------|-------------|
| `SEED` | `12345` | RNG seed (u64). Same seed = same run. |
| `ASCENSION` | `0` | Ascension level (0–20). |
| `--auto` | off | Bot plays from the start. |
| `--silent` | off | Suppress ALL output (only game-over summary prints). |
| `--debug` | on (default) | Full verbose output (every card, every action). |
| `--fast-act1` | off | Summary mode until Act 2, then switches to DeepDebug. |
| `--panic-watch` | off | Summary mode until HP drops below 50%, then switches to DeepDebug. |

### Display Modes

| Mode | What you see |
|------|-------------|
| **DeepDebug** | Full combat state every tick: hand, monsters, powers, intents, costs. |
| **Summary** | One-liner per floor + combat end summary (HP delta, cards played). |
| **TotalSilence** | Nothing until game over. |

## In-Game Commands

### Mode Switching

| Command | Effect |
|---------|--------|
| `auto` | Bot takes over everything. (Ctrl+C to stop.) |
| `auto run` | Bot takes over + switch to Summary display. |
| `silent run` / `sr` | Bot takes over + TotalSilence (fastest). |
| `manual` | Return to manual control. |
| `skip` | Bot finishes the current combat, then returns to manual. |
| `fast` | Toggle between Summary and DeepDebug display. |
| `a` / `step` | Bot makes ONE decision, you see what it picked, then you're back in control. |

### Combat Commands

| Command | Example | Description |
|---------|---------|-------------|
| `play <idx> [target]` / `p <idx> [target]` | `play 0 1` | Play hand card at index `idx` targeting monster at index `target`. Target is optional for non-targeted cards. |
| `end` / `e` | `end` | End your turn. |
| `potion <slot> [target]` | `potion 0 1` | Use potion in slot `slot`, optionally targeting monster index `target`. |
| `choose <idx1> <idx2> ...` | `choose 0 2` | Submit a hand-select choice (e.g., discard, exhaust, retain). Indices are hand positions. |
| `cancel` | `cancel` | Cancel a pending choice (if cancellable). |

### Map Navigation

| Command | Example | Description |
|---------|---------|-------------|
| `go <x>` | `go 3` | Travel to map node at column `x` in the next row. |
| `<number>` | `3` | Shorthand for `go <number>` when on the map screen. |

### Event Rooms

| Command | Example | Description |
|---------|---------|-------------|
| `<number>` | `0` | Choose event option at the given index. |

### Reward Screen

| Command | Example | Description |
|---------|---------|-------------|
| `claim <idx>` | `claim 0` | Claim reward item at index (gold, relic, potion, cards, key). |
| `pick <idx>` | `pick 2` | Pick a card from the card choice screen. |
| `proceed` / `leave` / `skip` | `proceed` | Leave the reward screen / skip card choice. |

### Campfire

| Command | Example | Description |
|---------|---------|-------------|
| `rest` | `rest` | Heal 30% of max HP. |
| `smith <deck_idx>` | `smith 3` | Upgrade the card at index `<deck_idx>` in your deck. |

### Shop

| Command | Example | Description |
|---------|---------|-------------|
| `buy card <idx>` | `buy card 0` | Buy a card from the shop. |
| `buy relic <idx>` | `buy relic 1` | Buy a relic from the shop. |
| `buy potion <idx>` | `buy potion 0` | Buy a potion from the shop. |
| `purge <deck_idx>` | `purge 5` | Remove a card from your deck (costs gold). |
| `leave` | `leave` | Leave the shop. |

### Boss Relic Select

| Command | Example | Description |
|---------|---------|-------------|
| `relic <idx>` | `relic 0` | Pick a boss relic reward. |
| `skip` | `skip` | Skip boss relic (proceed). |

### Deck Select (out-of-combat)

| Command | Example | Description |
|---------|---------|-------------|
| `select <idx1> <idx2> ...` | `select 3 7` | Select card(s) from your master deck (for purge/upgrade/transform/duplicate events). |
| `cancel` | `cancel` | Cancel selection. |

### Inspection Commands

| Command | Description |
|---------|-------------|
| `state` / `s` | Print detailed RunState: deck, relics, potions. |
| `relics` | List all relics with counters. |
| `potions` | List all potion slots. |
| `draw` | Show draw pile (combat only). |
| `discard` | Show discard pile (combat only). |
| `exhaust` | Show exhaust pile (combat only). |
| `help` / `h` | Print the in-game help summary. |
| `quit` / `q` | Exit immediately. |

## Typical Workflows

### "I want to watch the bot play a full run"

```bash
cargo run --release --bin play -- 42 0 --auto
```

This runs seed 42 at Ascension 0 with the bot making all decisions. Full DeepDebug
output — you'll see every card play, every monster action, every reward decision.

### "I want the bot to speedrun and just tell me the result"

```bash
cargo run --release --bin play -- 42 0 --auto --silent
```

Only the final GAME OVER summary prints. Good for batch testing seeds.

### "I want to play manually but skip boring combats"

Start in manual mode, navigate the map yourself, then type `skip` when combat starts.
The bot will finish the fight and return control to you.

```
> go 3
  [Encounter: JawWorm]
  COMBAT — Turn 0 | Energy: 3
  ...
> skip
  [MODE] Skipping combat (bot plays, summary mode)...
  [COMBAT END] HP: 80 → 72 | Cards played: 14
  [MODE] Returning to manual mode.
  REWARDS:
  ...
```

### "I want the bot to play Act 1 fast, then debug Act 2"

```bash
cargo run --release --bin play -- 42 0 --auto --fast-act1
```

Summary mode through Act 1, switches to DeepDebug when Act 2 starts.

### "I want to step through bot decisions one at a time"

Start in manual mode. At any decision point, type `a` or `step`:

```
> a
  [BOT] Decided: PlayCard { card_index: 0, target: Some(1) }
```

The bot makes one decision, you see what it chose, and you're back in control.

## The Bot (Agent)

The built-in bot uses **MCTS (Monte Carlo Tree Search)** with 4000 simulations per
combat decision and heuristic rollouts for non-combat screens.

### Combat AI
- UCB1 selection with exploration constant 150.0
- Smart rollouts with defense bias (6× weight for block cards when facing attack intents)
- Searches exhaustively within the current turn

### Non-Combat AI
- **Map**: Dynamic programming over the full act map, weighted by room type
  (Act 1: favor Rest rooms, Act 2: favor Elites, Act 3: favor Rest/Shop)
- **Rewards**: Score-based card evaluation, always claims gold/relics/keys
- **Campfire**: Rest if HP < 50%, otherwise smith the highest-value upgradeable card
- **Events**: Hardcoded per-event choice table
- **Shop**: Always leaves (TODO: implement shop buying logic)
- **Boss Relics**: Hardcoded priority list (Sozu > CursedKey > Astrolabe > ...)

## Known Bugs & Issues

> These are known problems you may encounter during testing. If you find new ones,
> please file an issue!

### 🔴 Crashes / Panics

1. **EventCombat crashes**: Some events that trigger combat (Colosseum, Dead Adventurer,
   Masked Bandits, Mysterious Sphere) may crash because the `EventCombat` state
   initialization path in `play.rs` doesn't fully handle the `EventCombat` variant.
   The engine's `tick_run` expects `combat_state` to be pre-populated, but `play.rs`
   only initializes combat for `CombatPlayerTurn`.

2. **Processing loop infinite spin**: Occasionally the "processing loop exceeded 2000
   ticks" warning fires. This usually indicates a power hook or action handler is
   re-queuing itself infinitely (e.g., a mistaken `ApplyPower` that triggers another
   `ApplyPower`).

### 🟡 Incorrect Behavior

3. **Event choices display as generic `[Option 0]`**: Most events only show `[Option 0]`,
   `[Option 1]`, etc., because `get_event_choices()` in `display.rs` only has readable
   text for the Neow event. All other events fall through to the generic placeholder.

4. **Bot skips shops entirely**: The agent always returns `Proceed` for shop screens,
   never buying cards, relics, or potions. This significantly hurts run success rate.

5. **SkipCombat mode display glitch**: When using `skip`, the base display mode gets
   overwritten to Summary. If you were previously in DeepDebug, you need to type `fast`
   to toggle back.

6. **Campfire options incomplete**: Only `rest` and `smith` are shown. Missing options:
   `dig` (Shovel relic), `lift` (Girya relic), `toke` (Peace Pipe relic),
   `recall` (Ruby Key). The campfire_handler may support them but the display doesn't
   show them and the CLI parser doesn't accept them.

### 🟢 Missing / Incomplete

7. **No Act 4 support in play.rs**: `RunState::new()` is called with `final_act: false`.
   The run always ends at Act 3 boss.

8. **Floor seed generation not called**: `rng_pool.generate_floor_seeds()` may not be
   called at every floor transition in `play.rs`, causing per-floor RNG to use stale
   seeds.

9. **No save/load**: Can't save mid-run and resume later.

10. **No deck view during combat**: The `state` command shows master deck, but there's
    no quick way to see your full combat deck (draw + hand + discard + exhaust) sorted.

## Future Directions

### Short-term: Make Testing Easier

- **Custom starting conditions**: Allow specifying starting deck, relics, and HP via
  command-line flags or a config file:
  ```bash
  cargo run --bin play -- 42 0 --deck "Strike x3, Demon Form, Offering" --relics "SneckoEye, DeadBranch"
  ```

- **Jump to floor**: Skip early floors to test specific encounters:
  ```bash
  cargo run --bin play -- 42 0 --start-floor 15
  ```

- **Force encounter**: Test a specific monster fight:
  ```bash
  cargo run --bin play -- 42 0 --encounter Hexaghost
  ```

- **Readable event text**: Implement `get_event_choices()` for all 53 events so players
  can understand what each option does.

- **Replay mode**: Record all inputs to a file, replay from file. Useful for reproducing
  bugs ("here's the exact input sequence that crashes").

### Medium-term: Community Engagement

- **WASM/web build**: Compile to WebAssembly + simple HTML frontend. Anyone can test in
  a browser without installing Rust.

- **Structured logging**: Output game state as JSON at each decision point. Makes it easy
  to build visualizers, analyzers, and alternative frontends.

- **Scenario test harness**: Define test scenarios as data files:
  ```yaml
  seed: 42
  ascension: 15
  deck: [Strike, Strike, Bash, Demon Form]
  relics: [BurningBlood, SneckoEye]
  encounter: SlimeBoss
  expected_outcome: Victory
  ```

- **Diff test dashboard**: Web UI showing which seeds pass/fail differential testing,
  with divergence reports.

### Long-term: AI Research Platform

- **Gym-like Python API**: `pip install sts-sim`, then:
  ```python
  import sts_sim
  env = sts_sim.CombatEnv(seed=42, ascension=15)
  obs = env.reset()
  while not done:
      action = my_agent.act(obs)
      obs, reward, done, info = env.step(action)
  ```

- **Multi-character support**: Silent, Defect, Watcher card pools + AI evaluation.

- **Distributed seed scanning**: Run thousands of seeds in parallel to find engine bugs
  via statistical anomalies.

## Contributing

Found a bug? Here's how to report it:

1. Note the **seed** and **ascension level**
2. Note the **floor number** and **room type** where the bug occurred
3. Copy the terminal output around the bug (a few lines before and after)
4. If possible, note the **exact command** that triggered it
5. File an issue with the above info, or just paste it in chat

Even "the bot chose something obviously stupid" is useful feedback — it helps improve
the AI heuristics.
