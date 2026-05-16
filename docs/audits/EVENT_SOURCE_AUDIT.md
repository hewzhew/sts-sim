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

### Designer selection and mutation sources

Java `events/shrines/Designer.java` has several non-obvious boundaries:

- Constructor randomness consumes `miscRng.randomBoolean()` twice.
- `Adjust` uses `masterDeck.hasUpgradableCards()` / `getUpgradableCards()`,
  so normal already-upgraded cards, Status, and Curse cards are not eligible;
  `Searing Blow` remains upgradeable.
- `Clean Up` and `Full Service` button disabling checks
  `CardGroup.getGroupWithoutBottledCards(masterDeck)`, while the actual grid
  selection opens `CardGroup.getGroupWithoutBottledCards(masterDeck.getPurgeableCards())`.
  Rust now preserves that source-level distinction instead of smoothing it into
  one "reasonable" predicate.
- `Adjust` random upgrades and `Full Service` follow-up upgrades use
  `Collections.shuffle(upgradableCards, new Random(miscRng.randomLong()))`.
- `Punch` applies HP_LOSS damage; Rust records this through a Designer-sourced
  HP domain event instead of directly mutating `current_hp`.

Fixes:

- `RunPendingChoiceReason::Upgrade` now filters to Java `canUpgrade()`-eligible
  master-deck cards.
- `RunPendingChoiceReason::Transform` now uses Java `getPurgeableCards()`
  filtering, rejecting `AscendersBane`, `CurseOfTheBell`, and `Necronomicurse`.
- Added `PurgeNonBottled` and `TransformNonBottled` run-selection reasons for
  Designer-style `getGroupWithoutBottledCards(getPurgeableCards())` flows.
- Designer random upgrades now call `upgrade_card_with_source(...,
  Event(Designer))` instead of mutating `upgrades` directly.
- Designer Punch now calls `change_hp_with_source(..., Event(Designer))`.

Tests:

- `designer_adjust_upgrade_one_selection_uses_java_can_upgrade`
- `designer_cleanup_remove_selection_excludes_bottled_and_unpurgeable_cards`
- `designer_random_upgrade_uses_can_upgrade_and_domain_event_source`
- `designer_punch_emits_hp_loss_source`
- `designer_full_service_followup_upgrade_uses_domain_event_source`
- `designer_run_pending_choice_rejects_invalid_direct_deck_input`

### Non-bottled card selection sweeps

Java frequently opens deck selection through:

```text
CardGroup.getGroupWithoutBottledCards(masterDeck.getPurgeableCards())
```

Rust previously had one generic `RunPendingChoiceReason::Purge` /
`Transform`, so these event flows could allow bottled cards through the
selection wrapper. This pass added explicit non-bottled variants and moved the
shared count helper next to the run pending-choice predicate.

Updated event modules:

- `BackToBasics`
- `Beggar`
- `BonfireElementals`
- `BonfireSpirits`
- `Cleric`
- `GoldenWing`
- `GremlinWheelGame`
- `LivingWall`
- `NoteForYourself`
- `PurificationShrine`
- `Transmogrifier`

The source distinction matters because Java does not apply the non-bottled
filter uniformly. `DrugDealer`, Neow transform/remove rewards, `EmptyCage`, and
`Astrolabe` use `masterDeck.getPurgeableCards()` directly, so they remain on
the ordinary `Purge` / `Transform` variants.

### Falling card preselection

Java `events/beyond/Falling.java` calls `CardHelper.hasCardWithType()` and
`CardHelper.returnCardOfType()`. Both helpers iterate
`CardGroup.getGroupWithoutBottledCards(masterDeck)`, so bottled Attack/Skill/
Power cards must not be preselected for the event's remove choices.

Fixes:

- `init_falling_state()` now excludes cards attached to Bottled Flame,
  Bottled Lightning, or Bottled Tornado when sampling the Skill / Power /
  Attack candidates with `miscRng`.
- Falling removal now emits `DomainEventSource::Event(Falling)` instead of
  using the generic deck-mutation source.

Tests:

- `falling_init_ignores_bottled_cards_like_java_card_helper`
- `falling_removal_uses_event_domain_source`

## Current High-Risk Event Areas

- `Match and Keep` still deserves deeper review for board serialization,
  duplicate-card handling, and how upgraded/generated card instances are
  represented after a match.
- Selection-heavy events need source-by-source checks: `WeMeetAgain`,
  `The Library`, `Nloth`, and remaining `Note For Yourself` persistence gaps.
- Selection choice preconditions still need deeper event-by-event review.
  Some Java handlers check candidate availability only when clicked, not when
  drawing the button, and several Rust modules still simplify those UI states.
- Event combat return states need continued scrutiny: `Colosseum`,
  `Masked Bandits`, `Mushrooms`, and `Mysterious Sphere`.
- `SecretPortal` and `SpireHeart` need an explicit classification: unsupported,
  modeled elsewhere, or normal event module.
- Event reward generation and domain-event source tagging must remain separate
  from combat reward generation.

## Validation

- `cargo test --all-targets`
- Current result after this pass: `780 passed`.
