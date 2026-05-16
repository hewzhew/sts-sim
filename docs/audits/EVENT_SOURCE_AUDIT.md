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
  not normal Rust event modules. They are classified below instead of being
  counted as ordinary event-content parity.
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

### We Meet Again trade sources

Java `events/shrines/WeMeetAgain.java` preselects a potion slot, a gold amount,
and one non-basic non-curse card, then any accepted trade grants a screenless
random relic. Rust already had the constructor RNG shape, but the trade effects
were too generic.

Fixes:

- The card trade option now exposes the selected card UUID and card id in
  `EventEffect::RemoveCard`.
- Giving a card removes it with `DomainEventSource::Event(WeMeetAgain)`.
- The relic obtained from potion / gold / card trades now uses
  `obtain_relic_with_source(..., Event(WeMeetAgain))` rather than the generic
  deck-mutation source.

Tests:

- `card_trade_option_exposes_specific_remove_effect`
- `card_trade_removes_card_and_obtains_relic_with_event_source`

### The Library previewed card rewards

Java `events/city/TheLibrary.java` builds a 20-card `CardGroup` using
`rollRarity()` plus `getCard()`, dedupes by `cardID`, and then calls every
player relic's `onPreviewObtainCard(card)` before adding each candidate to the
grid. When the player selects a candidate, Java obtains
`selectedCards.get(0).makeCopy()`, so preview-time changes such as Egg upgrades
belong to the selected copy.

Fixes:

- Library offerings now store card id plus previewed upgrade count instead of
  only a raw `CardId` discriminant.
- Library card selection now obtains the previewed copy through
  `add_card_to_deck_with_upgrades_from(..., Event(TheLibrary))`, preserving Egg
  preview upgrades without re-upgrading the card on obtain.
- The unsafe `transmute::<i32, CardId>` path was replaced by class-pool-based
  decoding.
- Sleep now goes through `RunState::heal_with_source`, giving it an event
  source and preserving Java `AbstractCreature.heal` behavior for
  `MarkOfTheBloom`.

Tests:

- `read_preserves_preview_obtain_upgrades_and_event_source`
- `sleep_heals_through_player_heal_semantics_and_event_source`
- `sleep_is_blocked_by_mark_of_the_bloom_like_java_player_heal`

### Big Fish HP rewards

Java `events/exordium/BigFish.java` handles the two HP options through player
resource methods, not direct field writes:

- `Banana` calls `AbstractDungeon.player.heal(maxHealth / 3, true)`.
- `Donut` calls `AbstractDungeon.player.increaseMaxHp(5, true)`.

`AbstractCreature.increaseMaxHp` first increments max HP and then calls
`heal(amount, true)`, so healing hooks such as `Mark of the Bloom` can block
the attached heal without blocking the max-HP gain itself.

Fixes:

- `BigFish` Banana now calls `RunState::heal_with_source(...,
  Event(BigFish))`.
- `BigFish` Donut now calls `RunState::gain_max_hp_with_source(...,
  Event(BigFish))`.
- `RunState::gain_max_hp_with_source` now follows Java's increase-then-heal
  shape instead of hard-mutating current HP.

Tests:

- `banana_uses_java_player_heal_semantics_and_event_source`
- `banana_heal_is_blocked_by_mark_of_the_bloom`
- `donut_increase_max_hp_uses_java_increase_then_heal_semantics`
- `donut_max_hp_gain_survives_mark_but_attached_heal_is_blocked`

### Cleric heal payment

Java `events/exordium/Cleric.java` computes the heal amount as
`(int)(maxHealth * 0.25f)` and then executes the choice as two separate player
resource calls:

- `AbstractDungeon.player.loseGold(35)`
- `AbstractDungeon.player.heal(healAmt)`

Rust previously rounded the heal amount and then directly mutated
`current_hp`.

Fixes:

- Cleric heal amount now uses Java's float-cast truncation.
- Cleric heal now calls `RunState::heal_with_source(..., Event(Cleric))`.
- Paying gold remains separate from healing, so `Mark of the Bloom` blocks only
  the heal and not the gold cost.

Tests:

- `heal_amount_uses_java_float_cast_not_rounding`
- `heal_cost_is_paid_even_when_mark_of_the_bloom_blocks_heal`

### Golden Wing remove damage

Java `events/exordium/GoldenWing.java` handles the remove-card option by first
calling:

```text
AbstractDungeon.player.damage(new DamageInfo(AbstractDungeon.player, damage))
```

That is normal player damage, not a direct HP assignment. In practice, the
out-of-combat simulator currently needs at least the `onLoseHpLast` portion
that affects HP loss such as `Tungsten Rod`.

Fixes:

- Golden Wing remove-path damage now emits an `HpChanged` event with
  `Event(GoldenWing)` source.
- The same path now applies the Java `Tungsten Rod` one-point reduction before
  opening the purge selection.

Tests:

- `remove_path_damage_uses_event_source_before_purge_selection`
- `remove_path_damage_respects_tungsten_rod_like_java_player_damage`

### N'loth relic trade

Java `events/shrines/Nloth.java` shuffles a copy of the player's relic list with
`miscRng.randomLong()`, stores two offered relic objects, and then handles trade
clicks asymmetrically:

- if the player does not already have `Nloth's Gift`, Java calls
  `player.loseRelic(choice.relicId)` and then obtains `Nloth's Gift`;
- if the player already has `Nloth's Gift`, Java obtains a `Circlet` and does
  not call `loseRelic` on the offered relic.

Fixes:

- N'loth trades now remove the offered relic through
  `remove_relic_at_with_source(..., Event(Nloth))`, preserving relic-lost
  events and unequip hooks.
- Existing `Nloth's Gift` now grants `Circlet` without losing the offered relic,
  matching Java's branch.
- The obtained Gift/Circlet now uses `obtain_relic_with_source(...,
  Event(Nloth))`.

Tests:

- `trade_removes_offered_relic_and_obtains_gift_with_event_source`
- `trade_with_existing_gift_grants_circlet_without_losing_offered_relic`

### Note For Yourself profile card

Java `events/shrines/NoteForYourself.java` reads the offered card from
`CardCrawlGame.playerPref` keys `NOTE_CARD` and `NOTE_UPGRADE`, defaulting to
`Iron Wave`. Taking the card manually calls relic `onObtainCard`, adds the card
to `masterDeck`, opens
`CardGroup.getGroupWithoutBottledCards(masterDeck.getPurgeableCards())`, and
later stores the removed card back into the same profile preference keys.

Fixes:

- `RunState` now carries explicit `note_for_yourself_card` and
  `note_for_yourself_upgrades` fields, making the Java profile preference an
  auditable simulator input/output rather than a hidden global or hardcoded
  event constant.
- The event now offers the configured note card instead of always hardcoding
  unupgraded `Iron Wave`.
- Taking the card uses a manual-obtain path that runs ordinary relic
  `onObtainCard` effects but skips Soul/obtain interception. This preserves the
  Java behavior where `Omamori` does not block a curse received from the note.
- The obtained card and removed/saved card now use
  `DomainEventSource::Event(NoteForYourself)` through the existing event
  selection source path.
- The card removed by the follow-up grid selection updates
  `RunState.note_for_yourself_*`, matching Java's delayed profile write without
  requiring the Rust simulator to touch disk preferences.

Tests:

- `take_uses_profile_note_card_and_event_source`
- `take_manual_obtain_is_not_blocked_by_omamori`
- `selected_saved_card_updates_note_profile_before_removal`

### Match and Keep board card identity

Java `events/shrines/GremlinMatchGame.java` builds six card objects, calls every
relic's `onPreviewObtainCard(c)` on each, adds `makeStatEquivalentCopy()` for
the matching pair, then shuffles the 12 concrete cards. The match check compares
`chosenCard.cardID` to `hoveredCard.cardID`, not an internal pair index.
Successful matches obtain `chosenCard.makeCopy()` through
`ShowCardAndObtainEffect`.

Fixes:

- Board serialization now stores card id plus previewed upgrade count for each
  of the six generated card types.
- Matching now compares decoded `CardId`, preserving Java behavior when two
  generated card types happen to share the same id.
- Successful matches now obtain the previewed copy with
  `DomainEventSource::Event(MatchAndKeep)` instead of using the generic reward
  source.
- The old unsafe `transmute::<i32, CardId>` path was replaced by decoding
  against the class, colorless, curse, and event starter-card pools used by this
  event.

Tests:

- `generated_board_stores_preview_obtain_upgrades_like_java`
- `matching_cards_obtain_previewed_copy_with_event_source`
- `matching_uses_card_id_not_board_type_index_like_java`

### Mushrooms and Masked Bandits event-combat boundaries

Java `events/exordium/Mushrooms.java` does not enter combat on the first
fight click. It first changes to the fight confirmation text, then generates
gold/relic rewards and calls `enterCombat()` on the next click. The eat path
uses `AbstractPlayer.heal()` and `ShowCardAndObtainEffect(new Parasite())`.

Java `events/city/MaskedBandits.java` uses event id `Masked Bandits` as the
monster encounter key. Paying gold advances through three dialogue screens and
the third continue opens the map immediately; it does not add an extra leave
screen.

Fixes:

- `Mushrooms` now preserves the Java two-step fight confirmation boundary.
- `Mushrooms` now heals through `RunState::heal_with_source`, so Mark of the
  Bloom blocks the heal, and obtains `Parasite` through the normal event card
  obtain pipeline.
- `Mushrooms` and `Masked Bandits` event-combat keys now use the Java encounter
  names, while CLI/full-run adapters still accept the older local aliases.
- `Masked Bandits` paid dialogue now completes on the same click Java uses to
  open the map instead of requiring one extra `[Leave]`.

Tests:

- `fight_path_requires_java_confirm_screen_before_combat`
- `eat_uses_player_heal_and_show_card_obtain_semantics`
- `eat_heal_is_blocked_by_mark_of_the_bloom_but_curse_obtain_still_runs`
- `pay_path_opens_map_after_java_dialog_sequence_without_extra_leave_click`
- `fight_uses_java_event_encounter_key_and_event_rewards`

### Mysterious Sphere and Colosseum event-combat boundaries

Java `events/beyond/MysteriousSphere.java` moves from INTRO to END text when
the player ignores the sphere; only the following click opens the map. Its
fight path generates gold plus a rare screenless relic immediately before
`enterCombat()`.

Java `events/city/Colosseum.java` uses two distinct event combats. The Slavers
fight has `rewardAllowed = false` and reopens the event afterward. The Nobs
fight preloads a rare relic, an uncommon relic, and 100 gold, then sets
`AbstractDungeon.getCurrRoom().eliteTrigger = true` before combat.

Fixes:

- `Mysterious Sphere` now preserves the Java END screen on the ignore path.
- `Mysterious Sphere` has tests for the preloaded event rewards before
  `EventCombat`.
- `EventCombatState` now carries `elite_trigger` separately from reward
  generation. This lets Colosseum Nobs behave like Java for combat-start relics
  such as Preserved Insect, Sling, and Slaver's Collar without generating
  ordinary elite rewards.
- CLI/full-run event combat initialization now passes `elite_trigger` into
  `CombatMeta::is_elite_fight`.

Tests:

- `leave_path_preserves_java_end_screen_before_map`
- `fight_path_generates_java_event_rewards_before_event_combat`
- `first_fight_returns_to_event_room_without_rewards_or_elite_trigger`
- `second_fight_preserves_java_elite_trigger_without_normal_elite_rewards`

### SecretPortal and SpireHeart classification

Java `events/beyond/SecretPortal.java` is a special one-time Act 3 portal event,
available only in The Beyond after `CardCrawlGame.playtime >= 800.0f`. Accepting
it does not behave like a normal event reward or combat. It marks the current
room complete, constructs a `MonsterRoomBoss` at map node `(-1, 15)`, appends
`pathX/pathY`, and starts the next-room transition.

Rust currently does not model player playtime or boss-room teleport nodes in
the event generator, and `EventId` deliberately has no `SecretPortal` variant.
This is an explicit unsupported special event, not an accidental missing normal
event module. To implement it later, add a run-level transition primitive rather
than an ordinary `src/content/events/*` choice handler.

Java `events/beyond/SpireHeart.java` is the post-Act-3 heart scene and final-act
gate. It computes score/heart damage, either sends the player to death/game-over
when the keys are missing, or opens the Door Unlock screen when all keys are
present. It is UI/stat heavy; the mechanics-relevant part is the run transition.

Rust compresses this into run-loop state:

- Act 3 boss with all three keys and final act enabled directly creates the
  Act 4 map.
- Act 3 boss without the full key set ends the run as victory in the current
  simplified outcome model.
- Act 4 `TrueVictoryRoom` ends the run as victory after the Heart.

This preserves the run-progression boundary needed by the simulator, but does
not model Java score upload, heart damage animation, death screen text, or Door
Unlock UI.

### Event card obtain source unification

Several event modules still used `RunState::add_card_to_deck`, which defaults
to `DomainEventSource::RewardScreen`. That is wrong for event choices: it makes
event curses/cards look like normal combat or reward-screen claims in trace
data, even when the mechanical obtain pipeline itself is otherwise correct.

Fixes:

- Added `content::events::obtain_event_card(run_state, event_id, card_id)` as a
  narrow helper for event-owned card obtains.
- Replaced all production `src/content/events/*` direct `add_card_to_deck`
  calls with the event-source helper.
- The only remaining direct event-module `add_card_to_deck` occurrence is a
  `falling.rs` unit-test setup helper, not production event logic.

Validation:

- Static scan: `rg "run_state\\.add_card_to_deck\\(|\\.add_card_to_deck\\(" src/content/events`
- `cargo test --all-targets`

## Current High-Risk Event Areas

- Selection choice preconditions still need deeper event-by-event review.
  Some Java handlers check candidate availability only when clicked, not when
  drawing the button, and several Rust modules still simplify those UI states.
- Event HP/max-HP/gold direct mutations still need the same domain-source pass
  that card obtains just received. `BigFish`, `Cleric`, and `GoldenWing` are
  now covered; the remaining direct writes should be handled event-by-event
  against Java source.

## Validation

- `cargo test --all-targets`
- Current result after this pass: `810 passed`.
