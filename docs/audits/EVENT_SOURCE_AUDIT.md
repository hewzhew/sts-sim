# Event Source Audit

This audit starts the Java-source-driven event pass. Events are run-level
mechanics, not policy; every implemented choice should preserve Java rewards,
costs, RNG streams, selection constraints, and follow-up screens closely enough
for replay and future training data.

## Source Roots

- Java: `D:/rust/cardcrawl/events`
- Rust event content: `src/content/events`
- Rust event routing: `src/engine/event_handler.rs`
- Rust run state and RNG helpers: `src/state/run.rs`

## Coverage Shape

Java has 52 concrete event classes after excluding abstract/dialog base classes.
Rust currently has 52 event modules, but the counts are not one-to-one:

- Rust splits Java `shrines/Bonfire.java` into `bonfire_spirits` and
  `bonfire_elementals`.
- Rust includes `neow`, which is not under Java `events`.
- Java has `beyond/SecretPortal.java` and `beyond/SpireHeart.java`; these are
  not represented as normal Rust event modules yet.
- Java `shrines/GremlinMatchGame.java` maps to Rust `match_and_keep`.
- Java `exordium/GoldenIdolEvent.java` maps to Rust `golden_idol`.
- Java `exordium/GoopPuddle.java` maps to Rust `goop_puddle` / EventId
  `WorldOfGoop`.
- Java `shrines/FountainOfCurseRemoval.java` maps to Rust `fountain`.
- Java `shrines/GoldShrine.java` maps to Rust `golden_shrine`.
- Java `city/TheMausoleum.java` maps to Rust `mausoleum`.

Do not treat equal module counts as proof of complete event parity.

## Fixed In This Pass

### Match and Keep start card

Java `GremlinMatchGame.initializeCards()` calls
`AbstractDungeon.player.getStartCardForEvent()`.

The class-specific Java results are:

- Ironclad: `Bash`
- Silent: `Neutralize`
- Defect: `Zap`
- Watcher: `Eruption`

Rust previously mapped non-Ironclad/Silent classes to `Strike`. This is now
fixed in `src/content/events/match_and_keep.rs`.

Test:

- `match_and_keep_start_card_matches_java_player_get_start_card_for_event`

### Event random-card RNG streams

Java source paths:

- `GremlinMatchGame.initializeCards()` uses `AbstractDungeon.getCard(rarity)`.
- `AbstractDungeon.getCard(rarity)` calls rarity pools through
  `CardGroup.getRandomCard(true)`, which consumes `cardRng`.
- `AbstractDungeon.returnColorlessCard(rarity)` shuffles
  `colorlessCardPool.group` with `shuffleRng.randomLong()`, then picks the
  first card of the requested rarity.
- `GremlinMatchGame` later shuffles the board with `miscRng.randomLong()`.

Rust `RunState::random_card_by_rarity()` and `random_colorless_card()` were
using `misc_rng`, which could shift event board RNG and later run replay. They
now use `card_rng` and `shuffle_rng` respectively.

Test:

- `event_random_card_helpers_use_java_rng_streams`

## Current High-Risk Event Areas

- `Match and Keep` still deserves deeper review for board serialization,
  duplicate-card handling, and how upgraded/generated card instances are
  represented after a match.
- Selection-heavy events need source-by-source checks: `Designer`, `Falling`,
  `WeMeetAgain`, `The Library`, `Bonfire Spirits`, `Nloth`, and
  `Note For Yourself`.
- Event combat return states need continued scrutiny: `Colosseum`,
  `Masked Bandits`, `Mushrooms`, and `Mysterious Sphere`.
- `SecretPortal` and `SpireHeart` need an explicit classification: unsupported,
  modeled elsewhere, or normal event module.
- Event reward generation and domain-event source tagging must remain separate
  from combat reward generation.

## Validation

- `cargo test --all-targets`
- Current result after this pass: `772 passed`.

