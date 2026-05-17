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

### EventHelper room-roll array semantics

Java `helpers/EventHelper.roll(Random)` does not classify `?` room results with a simple cumulative
threshold expression. It fills a 100-slot `RoomResult[]` initialized to `EVENT`, then overwrites
ranges for Monster, Shop, and Treasure. The source clamps fill starts with `Math.min(99, fillIndex)`,
so when ramped chances overflow the 100-slot table, later categories can still overwrite the final
slot. For example, `monsterSize=90`, `shopSize=30`, and `treasureSize=2` leaves index 99 as
`TREASURE`, not `SHOP`.

Rust now mirrors that table-fill behavior instead of using direct cumulative comparisons. This keeps
high-ramp event-room replay aligned with Java. The implementation still intentionally ignores
`DeadlyEvents` and endless `MimicInfestation` because those mod/endless branches are outside the
current simulator scope.

Test:

- `event_room_roll_uses_java_final_slot_overwrite_when_chances_overflow`

### Event pool exhaustion

Java `AbstractDungeon.generateEvent(Random)` does not repopulate event pools or
choose a backup event when both ordinary events and shrine/one-time events are
exhausted. If the shrine branch sees both pools empty it returns `null`; if
`getEvent` cannot find candidates it delegates to `getShrine`, which also has
no fabricated fallback.

Rust previously returned one of `Cleric`, `Golden Idol`, or `Golden Shrine` in
that state. That was a simulator-invented event source and could pollute long
run distributions. `EventGenerator::try_generate_event` now returns `None` when
Java has no event candidate, and the ordinary `generate_event` wrapper fails
explicitly instead of silently inventing an event.

Test:

- `exhausted_event_and_shrine_pools_do_not_fabricate_fallback_event`

### Event ID mapping

Java event identity comes from each event class's static `ID` string, not from
the visible title used by a bridge or UI. The simulator's live/trace rebuild
path must accept those exact IDs, including odd source names such as
`Liars Game`, `MindBloom`, `SensoryStone`, `The Moai Head`,
`Tomb of Lord Red Mask`, `FaceTrader`, `N'loth`, `NoteForYourself`,
`WeMeetAgain`, `Match and Keep!`, `Wheel of Change`, and Java's typo
`Transmorgrifier`.

Rust previously accepted several friendly aliases but missed a number of exact
Java IDs. That could make a real event trace look unsupported even though the
simulator had the event implementation. `event_id_from_name` now accepts the
exact Java IDs for all generated events and shrines while keeping the existing
aliases.

Test:

- `event_id_from_name_accepts_exact_java_event_ids`

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

### Golden Idol trap damage

Java `events/exordium/GoldenIdolEvent.java` handles the `[Fight]` trap option
with `AbstractDungeon.player.damage(new DamageInfo(null, this.damage))`.
This is normal DEFAULT damage with no owner: event rooms have no block, Torii's
owner-based `onAttacked` reduction does not apply, but Tungsten Rod still
applies later through `onLoseHpLast`.

Fix:

- `GoldenIdol` now uses the shared Java ownerless DEFAULT damage helper instead
  of duplicating a local Tungsten-only branch.

Test:

- `trap_damage_uses_java_ownerless_default_damage_hooks`

### Event DEFAULT damage helper unification

Java event damage paths use both ownerless default damage and player-owned
default damage:

- `FaceTrader` Touch, `Nest` Join, `ForgottenAltar` Shed Blood,
  `WindingHalls` Embrace Madness, `GoldenIdol` trap damage, and `ScrapOoze`
  Reach In use `DamageInfo(null, amount)`.
- `GoopPuddle` Gather Gold and `ShiningLight` Enter the Light use
  `DamageInfo(AbstractDungeon.player, amount)`.

Rust now routes `FaceTrader`, `Nest`, `ForgottenAltar`, `WindingHalls`, and
`ShiningLight` through the shared `apply_player_default_damage` helper instead
of duplicating relic-hook logic in each event. The owner flag remains explicit,
preserving the Java distinction where ownerless event damage skips Torii while
player-owned event damage can trigger Torii before Tungsten Rod.

Tests:

- `touch_damage_respects_tungsten_rod`
- `join_cult_damage_applies_tungsten_rod`
- `shed_blood_damage_respects_tungsten_after_max_hp_heal`
- `embrace_madness_damage_applies_tungsten_rod`
- `enter_light_normal_damage_applies_torii_then_tungsten`

### Event grid-selection callback timing

Several Java events open `AbstractDungeon.gridSelectScreen` and then consume
`gridSelectScreen.selectedCards` from the event's `update()` method. Rust must
not translate that pattern into a blanket "resume every event after
RunPendingChoice" rule:

- Pure deck mutations are already represented by the selected
  `RunPendingChoiceReason`: remove, transform, upgrade, duplicate, or obtain a
  selected card copy. Examples include `Cleric`, `GoldenWing`, `LivingWall`,
  `GremlinWheelGame`, `BackToBasics`, `AccursedBlacksmith`, `Duplicator`,
  `Purifier`, `UpgradeShrine`, `Transmorgrifier`, and `DrugDealer`.
- Events whose Java `update()` callback applies additional event-local effects
  after the deck selection need an explicit Rust post-selection callback. The
  current audited whitelist is `BonfireElementals` / `BonfireSpirits` for the
  offered-card rarity reward and `Designer` for the final event completion plus
  Full Service's random follow-up upgrade.
- Events such as `NoteForYourself` intentionally stay out of this whitelist
  because their selection side effect is handled by the shared deck-selection
  resolver and they should not be advanced by a synthetic extra event choice.

This whitelist is deliberately narrow. Adding an event here requires checking
the Java event source first and adding an integration test that submits the deck
selection through `tick_run`.

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
- `Punch` applies `DamageInfo(null, hpLoss, HP_LOSS)`; it bypasses block but
  still reaches player `onLoseHpLast` relic hooks such as `Tungsten Rod`.

Fixes:

- `RunPendingChoiceReason::Upgrade` now filters to Java `canUpgrade()`-eligible
  master-deck cards.
- `RunPendingChoiceReason::Transform` now uses Java `getPurgeableCards()`
  filtering, rejecting `AscendersBane`, `CurseOfTheBell`, and `Necronomicurse`.
- Added `PurgeNonBottled` and `TransformNonBottled` run-selection reasons for
  Designer-style `getGroupWithoutBottledCards(getPurgeableCards())` flows.
- Designer random upgrades now call `upgrade_card_with_source(...,
  Event(Designer))` instead of mutating `upgrades` directly.
- Designer Punch now uses the shared Java HP_LOSS helper with
  `Event(Designer)` source instead of directly mutating HP.
- Submitted `Clean Up` remove and transform selections now have regression
  coverage for `Event(Designer)` removal/transform sources.
- Direct calls to Java-disabled `Adjust`, `Clean Up`, and `Full Service`
  choices now stay inert. The guards intentionally reuse the Java button
  predicates, including the distinction where `Clean Up` / `Full Service`
  disabling checks non-bottled master-deck size while the opened grid later uses
  non-bottled purgeable cards.
- Java performs Designer grid-selection callbacks in `update()` after the grid
  closes. Rust now resumes only the whitelisted Designer screen-2 callback after
  `RunPendingChoice` returns to `EventRoom`, so selected Clean Up / Adjust /
  Full Service choices complete without requiring an extra synthetic event
  click. Full Service's random follow-up upgrade now happens in that same return
  step.

Tests:

- `designer_adjust_upgrade_one_selection_uses_java_can_upgrade`
- `designer_disabled_adjust_without_gold_does_not_pay_or_open_selection`
- `designer_disabled_adjust_without_upgradable_card_does_not_pay_or_advance`
- `designer_cleanup_remove_selection_excludes_bottled_and_unpurgeable_cards`
- `designer_cleanup_remove_selected_card_uses_event_source`
- `designer_cleanup_transform_selection_excludes_bottled_and_unpurgeable_cards`
- `designer_cleanup_transform_selected_cards_use_event_source`
- `designer_disabled_cleanup_without_gold_does_not_pay_or_open_selection`
- `designer_disabled_cleanup_transform_requires_two_non_bottled_cards`
- `designer_disabled_full_service_does_not_pay_or_open_selection`
- `designer_random_upgrade_uses_can_upgrade_and_domain_event_source`
- `designer_punch_emits_hp_loss_source`
- `designer_punch_hp_loss_applies_tungsten_rod`
- `designer_full_service_followup_upgrade_uses_domain_event_source`
- `designer_full_service_selection_auto_runs_followup_upgrade_like_java_update`
- `designer_run_pending_choice_rejects_invalid_direct_deck_input`

### Back to Basics starter upgrade semantics

Java `events/city/BackToBasics.java` implements `[Basics]` by scanning the
master deck and upgrading only cards that:

```text
have STARTER_STRIKE or STARTER_DEFEND tag
and canUpgrade()
```

Rust previously incremented `upgrades` directly for every locally classified
starter basic card. That could over-upgrade an already upgraded Strike/Defend
and bypass the normal master-deck upgrade path.

Java also always exposes the `[Simplicity]` remove-card button. If there are no
non-bottled purgeable cards, clicking it advances to the complete screen without
opening a grid selection. Rust previously presented that button as disabled and
still opened a pending purge selection if called directly.

Fixes:

- `[Basics]` now filters through Java-equivalent starter-basic plus
  `master_deck_card_can_upgrade`.
- Upgrades now go through `upgrade_card_with_source(...,
  Event(BackTotheBasics))`.
- `[Simplicity]` is no longer marked disabled solely because no removable card
  exists; the no-candidate path now advances without opening pending selection.
- `[Simplicity]` selection is covered as Java
  `CardGroup.getGroupWithoutBottledCards(masterDeck.getPurgeableCards())`, and
  the shared resolver emits `Event(BackTotheBasics)` sourced removal events.

Tests:

- `basics_upgrades_only_upgradeable_starter_strikes_and_defends`
- `simplicity_without_purgeable_cards_advances_without_pending_like_java`
- `simplicity_selection_excludes_bottled_and_unpurgeable_cards_like_java`
- `simplicity_removes_selected_card_with_event_source`

### Fountain bottled curse removal semantics

Java `events/shrines/FountainOfCurseRemoval.java` removes curses by scanning the
master deck backwards and skipping cards that are:

```text
not Curse
or inBottleFlame
or inBottleLightning
or AscendersBane / CurseOfTheBell / Necronomicurse
```

The Drink option is always clickable in Java. If no card passes that filter,
the event removes nothing and advances to the result screen.

Rust previously treated every non-special Curse as removable, regardless of
bottled attachment, and emitted generic `DeckMutation` removal events.

Fixes:

- Fountain drink availability and actual removal now share one source-backed
  removable-curse predicate.
- Bottled curses are excluded from Fountain removal.
- Removed curses now emit `CardRemoved` with
  `Event(FountainOfCurseCleansing)`.
- Drink is no longer disabled when only bottled/special curses exist.

Tests:

- `fountain_removes_only_non_bottled_removable_curses_with_event_source`
- `fountain_drink_without_removable_curses_is_still_clickable_like_java`

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

### Neow deck-selection rewards

Java `neow/NeowReward.java` has four deck-selection rewards whose selection
sources are easy to accidentally smooth into the ordinary event-shrine
patterns:

- `REMOVE_CARD` opens `masterDeck.getPurgeableCards()` for one card.
- `REMOVE_TWO` opens `masterDeck.getPurgeableCards()` for two cards.
- `TRANSFORM_CARD` opens `masterDeck.getPurgeableCards()` for one card.
- `TRANSFORM_TWO_CARDS` opens `masterDeck.getPurgeableCards()` for two cards.
- `UPGRADE_CARD` opens `masterDeck.getUpgradableCards()`.

Unlike many shrine events, Neow does not wrap these selection groups with
`CardGroup.getGroupWithoutBottledCards(...)`. Bottled cards therefore remain
eligible for Neow remove and transform rewards as long as they are otherwise
purgeable. Special non-purgeable cards such as `AscendersBane`,
`CurseOfTheBell`, and `Necronomicurse` still stay out of remove/transform
candidate lists.

Audit result:

- `RunState` now carries an explicit `neow_rng`, mirroring Java
  `NeowEvent.rng`, initialized from `Settings.seed` when the blessing choices
  are generated and reused by later Neow reward activation.
- Neow remove rewards stay on `RunPendingChoiceReason::Purge`, not
  `PurgeNonBottled`.
- Neow transform rewards stay on `RunPendingChoiceReason::Transform`, not
  `TransformNonBottled`.
- Neow upgrade rewards use the shared Java `canUpgrade()` master-deck filter.
- Submitted remove, upgrade, and transform selections emit domain events with
  `DomainEventSource::Event(Neow)`.
- Neow random rare card reward, ordinary card-reward generation, colorless
  generation, and Neow transform rewards now consume `neow_rng` rather than
  `card_rng` or `misc_rng`.
- Neow ordinary class-card rewards now match Java `rollRarity()`: 33% Uncommon,
  otherwise Common; they do not roll Rare unless the reward is explicitly
  `THREE_RARE_CARDS`.
- Neow ordinary colorless rewards now match Java's `getColorlessRewardCards`
  path: `rollRarity()` consumes Neow RNG, then Common is promoted to Uncommon,
  so the non-rare colorless reward offers Uncommon colorless cards only.

Tests:

- `remove_selection_uses_java_purgeable_cards_including_bottled`
- `transform_selection_uses_java_purgeable_cards_including_bottled`
- `upgrade_selection_uses_java_upgradable_cards`
- `remove_two_selection_removes_selected_cards_with_event_source`
- `selected_upgrade_uses_event_source`
- `transform_two_selection_transforms_selected_cards_with_event_source`
- `setup_preserves_java_neow_rng_counter_after_choice_generation`
- `one_random_rare_card_uses_neow_rng_not_card_rng`
- `normal_class_card_reward_uses_neow_rng_and_never_rolls_rare`
- `normal_colorless_reward_is_uncommon_only_like_java`

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
- Disabled missing-type choices now stay inert instead of advancing to the
  result screen; the all-missing `Land Safely` path still advances like Java.

Tests:

- `falling_init_ignores_bottled_cards_like_java_card_helper`
- `falling_removal_uses_event_domain_source`
- `disabled_missing_type_choice_does_not_advance_or_remove_card`
- `land_safely_without_any_candidates_advances_like_java`

### Living Wall choice guards

Java `events/exordium/LivingWall.java` has two separate guard layers:

- The `Grow` dialog option is disabled only when
  `masterDeck.hasUpgradableCards()` is false.
- When any of `Forget`, `Change`, or `Grow` is clicked, the handler first
  checks `CardGroup.getGroupWithoutBottledCards(masterDeck.getPurgeableCards())`.
  If that group is empty, the event still advances to the result screen, but no
  grid selection opens. This also affects `Grow`, even though the grid it would
  open is `masterDeck.getUpgradableCards()`.

Fix:

- Direct calls to disabled `Grow` now stay inert when no card can upgrade,
  instead of opening an empty upgrade selection.
- The Java non-bottled-purgeable guard before `Grow` is kept and covered, so a
  bottled-only upgradable deck advances without opening the upgrade prompt.
- `Forget` and `Change` are now covered as non-bottled purgeable selections,
  and their shared selection resolver must emit `Event(LivingWall)` sourced
  remove/transform domain events.

Tests:

- `disabled_grow_does_not_open_empty_upgrade_selection`
- `grow_keeps_java_non_bottled_purgeable_guard_before_upgrade_prompt`
- `forget_and_change_selection_exclude_bottled_and_unpurgeable_cards_like_java`
- `forget_removes_selected_card_with_event_source`
- `change_transforms_selected_card_with_event_source`

### We Meet Again trade sources

Java `events/shrines/WeMeetAgain.java` preselects a potion slot, a gold amount,
and one non-basic non-curse card, then any accepted trade grants a screenless
random relic. Rust already had the constructor RNG shape, but the trade effects
were too generic.

Fixes:

- The card trade option now exposes the selected card UUID and card id in
  `EventEffect::RemoveCard`.
- Added `DomainEvent::PotionLost` and `RunState::remove_potion_at_with_source`
  so giving a potion is visible as an event-sourced resource cost.
- Giving a card removes it with `DomainEventSource::Event(WeMeetAgain)`.
- The relic obtained from potion / gold / card trades now uses
  `obtain_relic_with_source(..., Event(WeMeetAgain))` rather than the generic
  deck-mutation source.
- Direct calls to disabled potion, gold, or card trades now stay inert instead
  of granting a free event relic without paying the corresponding resource.

Tests:

- `card_trade_option_exposes_specific_remove_effect`
- `card_trade_removes_card_and_obtains_relic_with_event_source`
- `potion_trade_removes_selected_potion_and_obtains_relic_with_event_source`
- `disabled_potion_trade_does_not_grant_free_relic`
- `disabled_gold_trade_does_not_grant_free_relic`
- `disabled_card_trade_does_not_grant_free_relic`

### Knowing Skull HP_LOSS costs

Java `events/city/KnowingSkull.java` applies every cost through
`AbstractDungeon.player.damage(new DamageInfo(null, cost, HP_LOSS))`: potion,
gold, card, and leave all bypass block but still run player relic
`onLoseHpLast`. `Tungsten Rod` therefore reduces each cost by 1.

Rust previously used direct sourced HP changes for these costs, preserving the
event source but skipping the HP_LOSS damage semantics.

Fixes:

- Potion, gold, card, and leave costs now use
  `content::events::apply_player_hp_loss_damage(..., Event(KnowingSkull))`.
- Added regression coverage for independent cost increments, gold reward order,
  random colorless card event source, and leave transition under Tungsten Rod.

Tests:

- `potion_reward_hp_loss_respects_tungsten_and_increments_only_potion_cost`
- `gold_reward_hp_loss_respects_tungsten_then_grants_gold`
- `card_reward_hp_loss_and_random_colorless_card_use_event_source`
- `leave_hp_loss_respects_tungsten_and_moves_to_complete_screen`

### Dead Adventurer encounter roll

Java `events/exordium/DeadAdventurer.java` consumes `miscRng` twice during
construction before the player searches:

- shuffle the hidden reward list with `miscRng.randomLong()`;
- choose the elite corpse encounter with `miscRng.random(0, 2)`.

Rust was preserving the reward shuffle but did not consume or store the enemy
roll, and event-combat adapters treated `"Dead Adventurer"` as a fixed
Lagavulin event fight. The initialized event state now stores the Java enemy
index and the combat trigger emits the corresponding encounter key:

- `0` -> `3 Sentries`
- `1` -> `Gremlin Nob`
- `2` -> `Lagavulin Event`

The full-run and play adapters now resolve those event-combat keys explicitly.

Java also splits the fight branch across two clicks: the search click reveals
the fight and rolls/adds the 25-35 gold combat reward, then the `[Fight!]`
click adds any remaining corpse rewards and enters elite combat. Rust previously
entered combat immediately on the search click and generated all remaining
rewards at that earlier boundary.

Fixes:

- Dead Adventurer now stops on the fight prompt after a triggered search.
- The pre-combat gold roll is stored in event state and reused when the fight is
  actually entered.
- Remaining unclaimed corpse rewards are generated on the `[Fight!]` click.
- The event combat now marks `elite_trigger=true`, matching Java.

Tests:

- `init_consumes_java_enemy_roll_and_stores_enemy_in_state`
- `combat_trigger_first_stops_on_java_fight_prompt`
- `fight_prompt_enters_combat_with_stored_java_enemy_key`
- `enemy_key_mapping_matches_java_get_monster_cases`

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

### Big Fish HP rewards and Box timing

Java `events/exordium/BigFish.java` handles the two HP options through player
resource methods, not direct field writes:

- `Banana` calls `AbstractDungeon.player.heal(maxHealth / 3, true)`.
- `Donut` calls `AbstractDungeon.player.increaseMaxHp(5, true)`.
- `Box` constructs `ShowCardAndObtainEffect(Regret)` before obtaining the random
  relic through `spawnRelicAndObtain`.

`AbstractCreature.increaseMaxHp` first increments max HP and then calls
`heal(amount, true)`, so healing hooks such as `Mark of the Bloom` can block
the attached heal without blocking the max-HP gain itself.

The Box ordering means Omamori interception uses the pre-relic state, while
later obtain-card hooks see the newly obtained relic.

Fixes:

- `BigFish` Banana now calls `RunState::heal_with_source(...,
  Event(BigFish))`.
- `BigFish` Donut now calls `RunState::gain_max_hp_with_source(...,
  Event(BigFish))`.
- `RunState::gain_max_hp_with_source` now follows Java's increase-then-heal
  shape instead of hard-mutating current HP.
- `BigFish` Box now obtains `Regret` with a pre-relic Omamori snapshot, while
  preserving post-relic obtain-card hooks such as `Darkstone Periapt`.

Tests:

- `banana_uses_java_player_heal_semantics_and_event_source`
- `banana_heal_is_blocked_by_mark_of_the_bloom`
- `donut_increase_max_hp_uses_java_increase_then_heal_semantics`
- `donut_max_hp_gain_survives_mark_but_attached_heal_is_blocked`
- `box_new_omamori_does_not_block_regret_from_same_choice`
- `box_new_darkstone_still_triggers_on_regret_after_relic_obtain`

### Sssserpent gold and curse

Java `events/exordium/Sssserpent.java` uses event id `"Liars Game"` and is a
two-step event:

- first click `Agree` only advances to the agree/confirm screen;
- confirm adds `ShowCardAndObtainEffect(Doubt)`, then grants gold with
  `player.gainGold(goldReward)`;
- gold reward is `175`, or `150` at ascension 15+.

`ShowCardAndObtainEffect` routes the curse through the normal obtain pipeline,
so `Omamori` can prevent `Doubt` without preventing the gold gain.

Tests:

- `agree_is_two_step_and_confirm_grants_java_gold_and_doubt`
- `ascension_15_uses_java_lower_gold_reward`
- `omamori_blocks_doubt_but_not_gold`

### Cleric heal payment

Java `events/exordium/Cleric.java` computes the heal amount as
`(int)(maxHealth * 0.25f)` and then executes the choice as two separate player
resource calls:

- `AbstractDungeon.player.loseGold(35)`
- `AbstractDungeon.player.heal(healAmt)`

Rust previously rounded the heal amount and then directly mutated
`current_hp`.

Java disables the Heal and Purify buttons when the player cannot pay, so those
branches must not be executable through direct event calls. Java also leaves
Purify clickable when the player can pay but has no non-bottled purgeable card;
that path advances to the proceed screen without paying gold or opening a grid.

Fixes:

- Cleric heal amount now uses Java's float-cast truncation.
- Cleric heal now calls `RunState::heal_with_source(..., Event(Cleric))`.
- Paying gold remains separate from healing, so `Mark of the Bloom` blocks only
  the heal and not the gold cost.
- Disabled Heal and Purify direct calls now stay inert instead of creating
  negative-gold states.
- Purify with enough gold but no removable card now advances without payment or
  pending selection, and its option semantics no longer claim a removal
  selection exists.
- Purify selection is covered as Java
  `CardGroup.getGroupWithoutBottledCards(masterDeck.getPurgeableCards())`, and
  the shared resolver must emit `Event(Cleric)` sourced removal events.

Tests:

- `heal_amount_uses_java_float_cast_not_rounding`
- `heal_cost_is_paid_even_when_mark_of_the_bloom_blocks_heal`
- `disabled_heal_does_not_pay_or_advance`
- `disabled_purify_does_not_pay_or_open_selection`
- `purify_without_removable_card_is_enabled_but_advances_without_payment_like_java`
- `purify_selection_excludes_bottled_and_unpurgeable_cards_like_java`
- `purify_removes_selected_card_with_event_source`

### Beggar donation and purge boundary

Java `events/city/Beggar.java` separates the paid donation from the card-removal
grid:

```text
INTRO click:
  loseGold(75)
  screen = GAVE_MONEY

GAVE_MONEY click:
  gridSelectScreen.open(getGroupWithoutBottledCards(masterDeck.getPurgeableCards()))
  screen = LEAVE
```

Rust previously paid gold and opened the purge selection on the same choice.

Fixes:

- Donation now only pays 75 gold and advances to the paid prompt.
- The next event click opens `RunPendingChoiceReason::PurgeNonBottled`.
- Direct calls to disabled donation now stay inert when the player has less
  than 75 gold.
- The paid purge prompt is covered as Java
  `CardGroup.getGroupWithoutBottledCards(masterDeck.getPurgeableCards())`, and
  the shared resolver must emit `Event(Beggar)` sourced removal events.

Tests:

- `donate_pays_gold_before_opening_purge_prompt_like_java`
- `paid_continue_opens_non_bottled_purge_selection`
- `disabled_donate_does_not_pay_or_advance`
- `paid_continue_selection_excludes_bottled_and_unpurgeable_cards_like_java`
- `paid_continue_removes_selected_card_with_event_source`

### Golden Wing remove damage and attack gate

Java `events/exordium/GoldenWing.java` handles the remove-card option by first
calling:

```text
AbstractDungeon.player.damage(new DamageInfo(AbstractDungeon.player, damage))
```

That is normal player damage, not a direct HP assignment. In practice, the
out-of-combat simulator currently needs the same normal-damage relic hook shape:
player-owned reductions such as `Torii` when applicable, then `onLoseHpLast`
such as `Tungsten Rod`.

The attack option is gated by `CardHelper.hasCardWithXDamage(10)`. The helper
ignores its parameter name in practice and checks `c.type == ATTACK` plus the
master-deck card instance's upgraded `baseDamage >= 10`. Rust must therefore
inspect the upgraded master-deck card instance, not only the card definition's
unupgraded `base_damage`.

Fixes:

- Golden Wing remove-path damage now emits an `HpChanged` event with
  `Event(GoldenWing)` source.
- The same path now applies the Java `Tungsten Rod` one-point reduction before
  opening the purge selection.
- The remove-path damage now uses the shared player-owned DEFAULT damage helper
  instead of a local Tungsten-only branch.
- Golden Wing's attack option now uses upgraded master-deck attack damage,
  matching Java's card instance `baseDamage` gate.
- Direct calls to the disabled attack option now stay inert, matching Java's
  `if (!canAttack) break` guard instead of advancing to the leave screen.
- The remove-card selection is covered as Java
  `CardGroup.getGroupWithoutBottledCards(masterDeck.getPurgeableCards())`, and
  the shared resolver emits `Event(GoldenWing)` sourced removal events after
  the damage step.

Tests:

- `remove_path_damage_uses_event_source_before_purge_selection`
- `remove_path_damage_respects_tungsten_rod_like_java_player_damage`
- `remove_path_selection_excludes_bottled_and_unpurgeable_cards_like_java`
- `remove_path_removes_selected_card_with_event_source`
- `attack_option_uses_upgraded_master_deck_base_damage_like_java`
- `attack_option_does_not_count_non_attack_base_damage`
- `disabled_attack_option_does_not_advance_or_grant_gold`

### World of Goop constructor gold loss and gather order

Java `events/exordium/GoopPuddle.java` rolls the leave-branch gold loss in the
constructor, then immediately clamps it to the player's current gold. The
gather branch executes:

```text
AbstractDungeon.player.damage(new DamageInfo(AbstractDungeon.player, damage))
AbstractDungeon.player.gainGold(gold)
```

That means the displayed/recorded leave loss is the constructor-clamped amount,
and gather damage is normal `DamageInfo` with `owner = player`, so event-room
normal damage still reaches player relic hooks such as `Torii` and
`Tungsten Rod`.

Fixes:

- `init_goop_puddle_state` now stores the Java constructor-clamped gold loss.
- `[Gather Gold]` now applies DEFAULT damage before gold gain, matching Java's
  execution order.
- The gather damage now uses the shared DEFAULT damage event helper instead of
  directly mutating HP.

Tests:

- `init_clamps_leave_gold_loss_to_current_gold_like_java_constructor`
- `gather_gold_applies_java_damage_before_gold_gain`
- `gather_gold_default_damage_applies_tungsten_rod`

### Face Trader touch and relic trade

Java `events/shrines/FaceTrader.java` has two relevant resource boundaries:

- `Touch` calls `gainGold(goldReward)` and then
  `damage(new DamageInfo(null, damage))`.
- `Trade` calls `spawnRelicAndObtain(...)` with a face relic selected by
  shuffling the available face relic ids with `miscRng.randomLong()`, or
  `Circlet` when all face relics are already owned.

Fixes:

- Touch now emits gold before HP loss, matching Java's `gainGold` then
  `damage` execution order.
- Touch damage now emits `HpChanged` with `Event(FaceTrader)` source and
  preserves the Java `Tungsten Rod` one-point reduction path.
- Trade now routes the selected face relic or Circlet through
  `RunState::obtain_relic_with_source(..., Event(FaceTrader))` instead of
  directly pushing into `run_state.relics`.

Tests:

- `touch_uses_event_hp_and_gold_sources`
- `touch_damage_respects_tungsten_rod`
- `trade_obtains_face_relic_through_event_source_pipeline`
- `trade_grants_circlet_when_all_face_relics_are_owned`

### Forgotten Altar relic swap and blood choice

Java `events/city/ForgottenAltar.java` has two easy-to-misread mechanics:

- `gainChalice()` replaces `Golden Idol` at its original relic index with
  `Bloody Idol` only if the player does not already have `Bloody Idol`.
  If `Bloody Idol` is already owned, Java grants `Circlet` and does not remove
  `Golden Idol`.
- `Shed Blood` calls `increaseMaxHp(5, false)` and then
  `damage(new DamageInfo(null, hpLoss))`. Despite the `false` argument,
  Java `increaseMaxHp` still calls `heal(amount, true)`.

Fixes:

- Added `RunState::obtain_relic_at_with_source` for Java-style indexed relic
  insertion/replacement while preserving on-equip hooks and domain events.
- `Forgotten Altar` now replaces `Golden Idol` with `Bloody Idol` at the same
  slot, or grants `Circlet` when `Bloody Idol` is already owned.
- The disabled Offer branch now stays inert when `Golden Idol` is missing,
  rather than advancing to the result screen through a direct handler call.
- `Shed Blood` now gains max HP through `gain_max_hp_with_source` and then
  applies sourced normal damage with the Java `Tungsten Rod` reduction path.
- `Desecrate` continues through the event card-obtain helper; regression
  coverage now verifies that Omamori can block the `Decay` without bypassing
  the event pipeline.

Tests:

- `disabled_offer_without_golden_idol_does_not_advance_or_grant_relic`
- `offering_golden_idol_replaces_same_relic_slot_with_bloody_idol`
- `offering_golden_idol_with_existing_bloody_idol_grants_circlet_without_losing_idol`
- `shed_blood_increases_max_hp_then_heals_then_takes_java_damage`
- `shed_blood_damage_respects_tungsten_after_max_hp_heal`
- `desecrate_decay_uses_event_obtain_pipeline_and_omamori_can_block_it`

### Addict stolen relic / Shame timing

Java `events/city/Addict.java` handles the `[Rob]` branch by constructing
`ShowCardAndObtainEffect(new Shame(), ...)` before calling
`spawnRelicAndObtain(...)` for the stolen relic. The `ShowCardAndObtainEffect`
constructor immediately checks `Omamori`, while the card is actually added later
after the relic has already been obtained.

This means:

- an already-owned `Omamori` blocks the `Shame`;
- a newly stolen `Omamori` does not block that same `Shame`;
- newly stolen obtain-card relics such as `Darkstone Periapt` still see the
  later `Shame` obtain.

Rust previously obtained the relic and then ran the normal card-obtain pipeline,
so stealing `Omamori` could incorrectly block the `Shame`.

Fixes:

- Added a RunState card-obtain entrypoint that uses an explicit Omamori
  interception snapshot while still evaluating other obtain-card hooks from the
  current relic set.
- The Addict rob branch snapshots Omamori before obtaining the stolen relic, then
  obtains `Shame` with that snapshot.
- The disabled paid branch now stays inert when the player has less than 85
  gold.

Tests:

- `disabled_pay_does_not_advance_or_obtain_relic`
- `rob_new_omamori_does_not_block_shame_from_same_choice`
- `rob_existing_omamori_still_blocks_shame_before_stolen_relic_resolves`
- `rob_new_darkstone_still_triggers_on_shame_after_relic_obtain`

### Drug Dealer relic obtain source

Java `events/city/DrugDealer.java` has three first-screen choices:

- Obtain `J.A.X.` through `ShowCardAndObtainEffect`.
- Transform two purgeable cards through a grid select.
- Obtain `MutagenicStrength`, or `Circlet` if `MutagenicStrength` is already
  owned, through `spawnRelicAndObtain`.

Rust already routed `J.A.X.` through the event card-obtain helper. The
Mutagenic Strength branch still pushed the relic directly into `run_state`,
which skipped event source metadata and Circlet counter handling.

Fixes:

- The Inject Mutagens branch now uses `RunState::obtain_relic_with_source(...,
  Event(DrugDealer))` for both `MutagenicStrength` and fallback `Circlet`.
- Added regression coverage for `J.A.X.` event source, relic event source, and
  existing-Circlet counter increment.
- The disabled Test Subject branch now remains inert when fewer than two
  purgeable cards exist, matching Java's disabled grid-select option.
- Test Subject opens `masterDeck.getPurgeableCards()`, so bottled purgeable cards
  remain selectable unlike `LivingWall`/`Transmogrifier`.
- Transforming the two selected cards now has regression coverage for
  `Event(DrugDealer)` sourced `CardTransformed` domain events.

Tests:

- `ingest_mutagens_obtains_jax_with_event_source`
- `inject_mutagens_obtains_relic_with_event_source`
- `inject_mutagens_grants_circlet_through_obtain_pipeline_when_already_owned`
- `disabled_test_subject_does_not_open_transform_selection_with_too_few_purgeable_cards`
- `test_subject_transform_selection_uses_purgeable_cards_including_bottled_like_java`
- `test_subject_transforms_two_cards_with_event_source`

### Ghosts and Vampires max-HP trades

Java `events/city/Ghosts.java` and `events/city/Vampires.java` both reduce
max HP through `AbstractDungeon.player.decreaseMaxHealth(...)`, not by direct
field writes. They then obtain cards through `ShowCardAndObtainEffect`.

Additional Vampire-specific Java behavior:

- Accepting without `Blood Vial` removes max HP, removes all starter Strike
  cards from the master deck, and obtains five `Bite` cards.
- Giving `Blood Vial` removes the relic, does not reduce max HP, and performs
  the same starter-Strike replacement.

Fixes:

- `Ghosts` now calls `lose_max_hp_with_source(..., Event(Ghosts))` before
  obtaining the Apparitions.
- `Vampires` now calls `lose_max_hp_with_source(..., Event(Vampires))`.
- `Vampires` Blood Vial removal now calls `remove_relic_at_with_source`.
- `Vampires` starter Strike removal now uses
  `remove_card_from_deck_with_source(..., Event(Vampires))`.
- The disabled Blood Vial branch now remains inert when the relic is missing,
  instead of replacing starter Strikes through a direct handler call.

Tests:

- `accept_loses_max_hp_and_obtains_apparitions_with_event_source`
- `accept_on_ascension_fifteen_obtains_three_apparitions`
- `accept_loses_max_hp_replaces_starter_strikes_with_event_sources`
- `give_vial_removes_relic_without_max_hp_loss_and_replaces_strikes`
- `disabled_give_vial_does_not_replace_strikes_without_blood_vial`

### Moai Head max-HP heal and Golden Idol trade

Java `events/beyond/MoaiHead.java` implements the heal option manually:

```text
player.maxHealth -= hpAmt
clamp currentHealth to maxHealth
clamp maxHealth to at least 1
player.heal(player.maxHealth)
```

This means the full heal is still a real player heal and can be blocked by
`Mark of the Bloom`. The Golden Idol option calls `loseRelic("Golden Idol")`
and then `gainGold(333)`.

Fixes:

- Moai Head max-HP loss now uses `lose_max_hp_with_source`.
- The follow-up full heal now uses `heal_with_source`, preserving Java healing
  hooks.
- Golden Idol trade now uses `remove_relic_at_with_source` and sourced gold
  gain.
- The disabled Golden Idol trade now stays inert when the relic is missing.

Tests:

- `disabled_trade_without_golden_idol_does_not_advance_or_grant_gold`
- `enter_loses_max_hp_then_heals_to_new_max_with_event_source`
- `enter_max_hp_loss_survives_mark_but_full_heal_is_blocked`
- `trade_removes_golden_idol_and_grants_gold_with_event_sources`

### Gremlin Wheel result branches

Java `events/shrines/GremlinWheelGame.java` applies the spin result in
`applyResult()`:

```text
case 2:
  player.heal(player.maxHealth)
default:
  player.damage(new DamageInfo(null, damageAmount, HP_LOSS))
```

The full heal is therefore a normal Java heal and can be blocked by `Mark of the
Bloom`. The damage branch is HP-loss damage, so it bypasses block/attack
callbacks, but `AbstractPlayer.damage` still runs relic `onLoseHpLast`; this
means `Tungsten Rod` reduces the loss by 1 even though the damage type is
`HP_LOSS`.

The other result branches also matter for simulator traces: gold is scaled by
act, the relic branch opens a reward screen with one screenless relic, the curse
branch uses `ShowCardAndObtainEffect(new Decay())`, and the remove branch opens
`CardGroup.getGroupWithoutBottledCards(masterDeck.getPurgeableCards())` only
when there is at least one selectable card.

Fixes:

- Full-heal spin result now uses `heal_with_source(..., Event(GremlinWheelGame))`
  instead of direct `current_hp = max_hp`.
- HP-loss spin result now uses sourced HP change, allows HP to reach 0 like
  Java `damage`, and applies `Tungsten Rod`'s `onLoseHpLast`.
- Gold, relic, curse, and purge results now have event-level regression tests
  for source, reward-screen shape, Omamori interception, and non-bottled purge
  selection.
- The purge-result no-candidate path is locked to Java's `size() <= 0` guard,
  while the selection and submitted removal are covered for non-bottled
  purgeable filtering and `Event(GremlinWheelGame)` source.

Tests:

- `gold_result_uses_act_scaled_gold_and_event_source`
- `relic_result_opens_reward_screen_with_one_relic_reward`
- `curse_result_uses_obtain_pipeline_so_omamori_can_block_decay`
- `purge_result_opens_non_bottled_purge_selection_when_possible`
- `purge_result_without_purgeable_card_advances_without_pending_like_java`
- `purge_result_selection_excludes_bottled_and_unpurgeable_cards_like_java`
- `purge_result_removes_selected_card_with_event_source`
- `full_heal_uses_java_heal_source_and_respects_mark_of_the_bloom`
- `full_heal_emits_event_source_without_mark`
- `hp_loss_result_uses_source_and_can_reduce_hp_to_zero`
- `hp_loss_result_applies_tungsten_rod_on_lose_hp_last`

### Mind Bloom fight, Mark, and high-floor heal

Java `events/beyond/MindBloom.java` uses `miscRng.randomLong()` to shuffle the
three Act 1 bosses (`The Guardian`, `Hexaghost`, `Slime Boss`) for the fight
branch, adds gold plus a rare relic reward, uses `spawnRelicAndObtain` for the
`[I am Awake]` Mark of the Bloom branch, and the high-floor `[I am Rich]`
variant calls:

```text
player.heal(player.maxHealth)
ShowCardAndObtainEffect(new Doubt())
```

Fixes:

- The fight branch now uses the shuffled Act 1 boss key instead of a fixed
  fallback encounter, and the play adapter maps those event keys to the actual
  boss encounters.
- The fight branch now uses the ordinary rare relic reward path rather than the
  screenless event-relic path.
- The Mark of the Bloom branch now obtains the relic through
  `obtain_relic_with_source(..., Event(MindBloom))` instead of pushing directly
  into `run_state.relics`.
- The Mark of the Bloom branch now upgrades each Java `canUpgrade()` eligible
  master-deck card through `upgrade_card_with_source(..., Event(MindBloom))`
  instead of mutating `card.upgrades` directly. This preserves sourced upgrade
  events and keeps Searing Blow's repeat-upgrade exception centralized in the
  shared master-deck predicate.
- The high-floor heal branch now uses `heal_with_source`, preserving Java heal
  hooks such as `Mark of the Bloom`, before obtaining `Doubt` through the event
  card path.

Tests:

- `fight_uses_java_shuffled_act1_boss_key_and_rare_relic_reward`
- `remember_obtains_mark_of_the_bloom_with_event_source`
- `remember_upgrades_all_java_can_upgrade_cards_with_event_source`
- `high_floor_desire_heals_with_event_source_and_obtains_doubt`
- `high_floor_desire_heal_respects_mark_of_the_bloom`

### Winding Halls resource branches

Java `events/beyond/WindingHalls.java` computes the branch amounts at event
construction time with `MathUtils.round(...)`, then applies:

```text
Embrace Madness:
  player.damage(new DamageInfo(null, hpAmt))
  obtain 2 Madness
Retrace:
  player.heal(healAmt)
  obtain Writhe
Accept:
  player.decreaseMaxHealth(maxHPAmt)
```

Fixes:

- The Madness branch now keeps the existing `Tungsten Rod` adjustment for
  ownerless normal damage but emits the HP loss through
  `change_hp_with_source(..., Event(WindingHalls))`.
- The Writhe branch now uses `heal_with_source`, so Java heal hooks such as
  `Mark of the Bloom` are preserved.
- The max-HP branch now uses `lose_max_hp_with_source` instead of directly
  mutating `max_hp` and clamping `current_hp`.

Tests:

- `embrace_madness_damage_uses_event_source_and_obtains_two_madness`
- `embrace_madness_damage_applies_tungsten_rod`
- `retrace_heal_uses_event_source_and_obtains_writhe`
- `retrace_heal_respects_mark_of_the_bloom_but_still_obtains_writhe`
- `accept_loss_uses_max_hp_event_source_and_clamps_current_hp`

### Sensory Stone HP-loss rewards

Java `events/beyond/SensoryStone.java` opens one, two, or three colorless card
reward rows. The two higher-focus choices additionally call:

```text
player.damage(new DamageInfo(null, 5, HP_LOSS))
player.damage(new DamageInfo(null, 10, HP_LOSS))
```

`HP_LOSS` bypasses block and attack callbacks, but Java `AbstractPlayer.damage`
still applies `onLoseHpLast`, so `Tungsten Rod` reduces the loss by 1.

Fixes:

- Focus 2/3 HP loss now emits `HpChanged` with
  `Event(SensoryStone)` instead of directly mutating `current_hp`.
- The previous comment claiming `Tungsten Rod` does not reduce this HP loss was
  corrected, and the event now applies the Java `onLoseHpLast` reduction.

Tests:

- `focus_two_hp_loss_uses_event_source_and_opens_two_rewards`
- `focus_three_hp_loss_applies_tungsten_rod_on_lose_hp_last`

### Shining Light damage and random upgrades

Java `events/exordium/ShiningLight.java` applies:

```text
player.damage(new DamageInfo(player, damage))
upgradeCards()
```

The damage is normal player-owned damage. Out of combat there is no block to
consume, but Java still applies relic hooks such as `Torii` and `Tungsten Rod`
in the normal damage pipeline. The random upgrades are event-caused deck
mutations, not generic deck mutations.

Fixes:

- Entering the light now routes damage through sourced HP change after applying
  the relevant Java normal-damage relic reductions.
- Added `RunState::upgrade_random_cards_with_source`, leaving the old
  `upgrade_random_cards` default behavior unchanged.
- Shining Light random upgrades now emit `CardUpgraded` with
  `Event(ShiningLight)`.
- The disabled Enter option now stays inert if no master-deck cards can upgrade,
  matching Java's disabled dialog option instead of applying damage through a
  direct handler call.

Tests:

- `enter_light_damage_and_random_upgrades_use_event_source`
- `enter_light_normal_damage_applies_torii_then_tungsten`
- `leave_does_not_damage_or_upgrade`
- `disabled_enter_light_does_not_apply_damage_when_no_cards_can_upgrade`

### Scrap Ooze null-owner damage

Java `events/exordium/ScrapOoze.java` applies reach-in damage through:

```text
player.damage(new DamageInfo(null, dmg))
```

That is DEFAULT damage with no owner. It does not get the player-owned Torii
`onAttacked` reduction, but Tungsten Rod still applies at the HP-loss hook.

Fixes:

- Scrap Ooze now uses the shared event DEFAULT-damage helper with
  `EventDamageOwner::None` instead of a local Tungsten-only branch.

Tests:

- `reach_in_default_null_damage_ignores_torii_but_applies_tungsten`

### Nest gold and Ritual Dagger branch

Java `events/city/Nest.java` has two resource branches after the intro:

```text
Steal:
  player.gainGold(goldGain)
Join:
  player.damage(new DamageInfo(null, 6))
  ShowCardAndObtainEffect(new RitualDagger())
```

The Join damage is normal ownerless damage. That means `Tungsten Rod` still
applies through `onLoseHpLast`, but `Torii` does not because Java Torii requires
`info.owner != null`.

Fixes:

- Join damage now emits `HpChanged` with `Event(Nest)` instead of directly
  mutating `current_hp`.
- Tests now lock both the existing gold source path and the Ritual Dagger obtain
  source.

Tests:

- `steal_gold_uses_event_source`
- `join_cult_damage_and_ritual_dagger_use_event_source`
- `join_cult_damage_applies_tungsten_rod`

### The Joust roll and payout boundary

Java `events/city/TheJoust.java` separates the wager result into two clicks:

```text
PRE_JOUST:
  ownerWins = miscRng.randomBoolean(0.3f)
  screen = JOUST

JOUST:
  reveal result
  gainGold(250 or 100) only if the bet won
  screen = COMPLETE
```

Rust previously combined the roll and payout on the same `[Continue]` click.
That changed the event-state boundary and made gold appear one decision earlier
than Java.

Fixes:

- Screen 2 now only consumes the Java `miscRng.randomBoolean(0.3f)` result and
  records it in event state.
- Screen 3 now applies the actual wager payout and advances to the result
  screen.

Tests:

- `pre_joust_continue_rolls_result_without_payout_like_java`
- `result_continue_pays_murderer_bet_after_roll_screen`
- `result_continue_pays_owner_bet_after_roll_screen`

### Bonfire resource rewards

Java `events/shrines/Bonfire.java` applies the offered-card reward after grid
selection removes the card from the master deck:

- Curse: spawn and obtain `SpiritPoop`, or `Circlet` if `SpiritPoop` is already
  owned.
- Common / Special: `player.heal(5)`.
- Uncommon: `player.heal(player.maxHealth)`.
- Rare: `player.increaseMaxHp(10, false)`, then
  `player.heal(player.maxHealth)`.

Rust splits this Java event into `bonfire_elementals` and `bonfire_spirits`.
Both now use sourced run helpers instead of direct relic/HP/max-HP mutation.
The rare branch preserves Java's two-step health behavior: increasing max HP
heals by 10 through `increaseMaxHp`, then the event performs a separate full
heal. `Mark of the Bloom` therefore blocks the heals while leaving the max-HP
increase intact.

Java applies the selected-card reward inside `Bonfire.update()` after
`gridSelectScreen.selectedCards` becomes non-empty. Rust now mirrors that timing
by resuming only the whitelisted Bonfire screen-2 callback immediately after
`RunPendingChoice` returns to `EventRoom`; Bonfire no longer waits for a later
extra event click to apply the rarity reward.

Java's Bonfire choose screen does not disable the Offer button when there are no
non-bottled purgeable cards. Pressing it advances to the complete screen and
opens no grid. Rust now preserves that empty-offer path instead of creating an
empty pending selection. Because Rust keeps both `BonfireElementals` and the
live alias `BonfireSpirits`, both entries now share the same non-bottled
purgeable filter, record the offered card rarity through the shared selection
resolver, and emit sourced `CardRemoved` events.

Tests:

- `common_offer_heals_with_event_source`
- `rare_offer_matches_java_max_hp_then_full_heal_sequence`
- `heal_rewards_obey_mark_of_the_bloom`
- `curse_offer_obtains_spirit_poop_with_event_source`
- `offer_without_purgeable_card_advances_without_pending_like_java`
- `offer_selection_excludes_bottled_and_unpurgeable_cards_like_java`
- `offer_removes_selected_card_with_event_source_and_applies_post_selection_reward`
- `common_offer_selection_heals_during_post_selection_callback`
- `common_offer_heals_with_spirits_event_source`
- `curse_offer_obtains_spirit_poop_with_spirits_event_source`
- `offer_removes_selected_card_with_spirits_event_source_and_applies_post_selection_reward`

### Upgrade Shrine upgrade guard

Java `events/shrines/UpgradeShrine.java` disables `[Pray]` if
`masterDeck.hasUpgradableCards()` is false, then opens
`masterDeck.getUpgradableCards()` if the enabled option is pressed. Rust now uses
the shared Java-equivalent master-deck upgrade predicate instead of a local
copy, so Searing Blow and other custom upgrade rules stay centralized.

Tests:

- `disabled_pray_does_not_open_empty_upgrade_selection`
- `searing_blow_remains_upgradeable_after_prior_upgrades`

### Lab potion rewards

Java `events/shrines/Lab.java` does not put potions directly into player potion
slots. It clears room rewards, adds `RewardItem(PotionHelper.getRandomPotion())`
twice, adds a third potion below A15, marks the room complete, and opens the
combat reward screen.

Fixes:

- Lab now opens `EngineState::RewardScreen` containing potion reward items.
- Lab no longer calls `obtain_potion` directly, so potion slot capacity, Sozu,
  and claim/discard behavior remain in the reward handler instead of the event.

Tests:

- `lab_opens_three_potion_rewards_without_directly_filling_inventory`
- `lab_ascension_fifteen_opens_two_potion_rewards`

### Woman in Blue potion rewards and HP loss

Java `events/shrines/WomanInBlue.java` buys potion rewards, not direct potion
inventory entries. For each paid choice it loses gold, clears room rewards,
adds one to three `RewardItem(PotionHelper.getRandomPotion())`, marks the room
complete, and opens the combat reward screen. The purchase buttons are not
disabled by gold in the event handler; `loseGold(cost)` clamps the player's
gold to zero. Normal vanilla reachability is gated earlier in
`dungeons/AbstractDungeon.java`: `The Woman in Blue` is filtered out when
`AbstractDungeon.player.gold < 50`. Potion capacity and Sozu are handled later
by the reward screen.

The A15 leave branch applies
`DamageInfo(null, ceil(maxHealth * 0.05), HP_LOSS)`. HP_LOSS bypasses block and
Torii, but still reaches relic `onLoseHpLast`, so `Tungsten Rod` reduces it.

Fixes:

- Buying potions now opens `EngineState::RewardScreen` containing potion reward
  items instead of calling `obtain_potion` directly.
- Potion purchase semantics no longer require an empty potion slot.
- Event generation requires at least 50 gold, matching the Java event-pool
  gate.
- The handler remains Java-like under directly constructed/replay states:
  buying with insufficient gold clamps gold to zero and still opens potion
  rewards.
- A15 leave damage now emits `HpChanged` with `Event(WomanInBlue)` and applies
  Tungsten Rod's HP-loss reduction.

Tests:

- `three_potion_option_exposes_trade_semantics`
- `buying_potions_opens_reward_screen_without_filling_slots_directly`
- `buying_potions_with_insufficient_gold_clamps_gold_like_java`
- `ascension_leave_hp_loss_uses_event_source_and_tungsten_rod`

### Tomb of Lord Red Mask relic obtain

Java `events/beyond/TombRedMask.java` has asymmetric button indices:

- If the player already has `Red Mask`, button 0 wears the mask and gains 222
  gold.
- If the player does not have `Red Mask`, button 0 is a disabled relic-required
  affordance, button 1 loses all gold and obtains `Red Mask`, and button 2
  leaves.

The paid branch calls `player.loseGold(player.gold)` and
`spawnRelicAndObtain(..., new RedMask())`. Rust now uses
`obtain_relic_with_source` for this relic instead of pushing directly into the
relic list. The disabled button-0 affordance now stays inert when `Red Mask` is
missing, instead of being treated as the Leave branch.

Tests:

- `paying_without_mask_loses_all_gold_and_obtains_red_mask_with_event_source`
- `wearing_existing_mask_gains_222_gold_with_event_source`
- `choices_preserve_java_button_indices_when_mask_is_missing`
- `disabled_don_mask_without_red_mask_does_not_advance_or_grant_gold`

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

### Transmogrifier transform selection semantics

Java `events/shrines/Transmogrifier.java` uses event id string
`Transmorgrifier` and opens
`CardGroup.getGroupWithoutBottledCards(AbstractDungeon.player.masterDeck.getPurgeableCards())`.
The selected card is removed from the master deck, transformed with
`AbstractDungeon.miscRng`, and obtained through `ShowCardAndObtainEffect`.

Audit result:

- Rust keeps the Java id spelling through `EventId::Transmorgrifier`.
- The event opens `RunPendingChoiceReason::TransformNonBottled`, excluding
  bottled and unpurgeable cards.
- The shared transform resolver emits `CardTransformed` with the event source.

Tests:

- `transform_selection_excludes_bottled_and_unpurgeable_cards_like_java`
- `transformed_card_uses_event_source`

### Purifier purge selection semantics

Java `events/shrines/PurificationShrine.java` opens
`CardGroup.getGroupWithoutBottledCards(AbstractDungeon.player.masterDeck.getPurgeableCards())`.
The selected card is removed from the master deck through the event's grid
selection flow.

Audit result:

- Rust opens `RunPendingChoiceReason::PurgeNonBottled`, excluding bottled and
  unpurgeable cards.
- The shared purge resolver emits `CardRemoved` with `Event(Purifier)`.

Tests:

- `purge_selection_excludes_bottled_and_unpurgeable_cards_like_java`
- `purge_removes_selected_card_with_event_source`

### Upgrade Shrine selection semantics

Java `events/shrines/UpgradeShrine.java` disables the first dialog option when
`masterDeck.hasUpgradableCards()` is false, and otherwise opens
`AbstractDungeon.player.masterDeck.getUpgradableCards()`.

Audit result:

- Disabled direct calls remain inert instead of opening an empty upgrade
  selection.
- The pending upgrade selection uses Java `canUpgrade()` semantics, including
  the Searing Blow exception.
- The shared upgrade resolver emits `CardUpgraded` with `Event(UpgradeShrine)`.

Tests:

- `disabled_pray_does_not_open_empty_upgrade_selection`
- `searing_blow_remains_upgradeable_after_prior_upgrades`
- `upgrade_selection_uses_java_upgradable_cards`
- `selected_card_is_upgraded_with_event_source`

### Duplicator full-deck copy semantics

Java `events/shrines/Duplicator.java` opens `AbstractDungeon.player.masterDeck`
directly. It does not use `getPurgeableCards()` and does not filter bottled
cards. The selected card is copied through `makeStatEquivalentCopy()`, bottle
flags are cleared on the copied Java card, and the copy is obtained through
`ShowCardAndObtainEffect`.

Audit result:

- Rust already opens `RunPendingChoiceReason::Duplicate`, whose candidate
  filter includes the full master deck.
- The shared duplicate resolver uses `add_card_instance_copy_to_deck_from`,
  preserving stat-equivalent fields such as upgrades and `misc_value`, while
  allocating a fresh UUID so Rust bottle attachments remain on the original
  UUID.
- Added regression coverage that the Duplicator event source is used for the
  obtained copy.

Tests:

- `duplicate_selection_uses_full_master_deck_like_java`
- `duplicate_selection_obtains_stat_equivalent_copy_with_event_source`

### Note For Yourself profile card

Reachability:

Java `dungeons/AbstractDungeon.java::isNoteForYourselfAvailable()` adds
`NoteForYourself` to the special one-time event list only when the run is not a
Daily Run and either the current ascension is 0 or the current ascension is
below the profile's highest unlocked ascension. It is always disabled at A15+.
Rust now represents this explicitly in `EventContext` with `is_daily_run` and
`highest_unlocked_ascension_level`; it no longer treats the gate as plain
`ascension < 15`.

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
- The follow-up selection is covered after the note card is added, so the newly
  obtained card is included alongside existing non-bottled purgeable cards,
  matching Java's `masterDeck.addToTop(...)` before
  `CardGroup.getGroupWithoutBottledCards(masterDeck.getPurgeableCards())`.

Tests:

- `take_uses_profile_note_card_and_event_source`
- `take_manual_obtain_is_not_blocked_by_omamori`
- `take_selection_excludes_bottled_and_unpurgeable_cards_after_obtaining_note_card`
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
- The second-flip choice list no longer inserts a synthetic disabled
  `(First card: ...)` row. Java exposes board card hitboxes during play, not a
  disabled dialog option; keeping that row in Rust shifted action indices and
  let direct calls to the disabled row flip a real card.

Tests:

- `generated_board_stores_preview_obtain_upgrades_like_java`
- `matching_cards_obtain_previewed_copy_with_event_source`
- `matching_uses_card_id_not_board_type_index_like_java`
- `second_flip_choices_do_not_include_synthetic_disabled_info_row`

### Mushrooms and Masked Bandits event-combat boundaries

Java `events/exordium/Mushrooms.java` does not enter combat on the first
fight click. It first changes to the fight confirmation text, then generates
gold/relic rewards and calls `enterCombat()` on the next click. The eat path
uses `AbstractPlayer.heal()` and `ShowCardAndObtainEffect(new Parasite())`.

Java `events/city/MaskedBandits.java` uses event id `Masked Bandits` as the
monster encounter key. Paying gold advances through three dialogue screens and
the third continue opens the map immediately; it does not add an extra leave
screen. Its `stealGold()` animation samples monsters through LibGDX
`MathUtils.random`, not the game's seeded RNG streams, so the simulator should
not consume run RNG while paying.

Fixes:

- `Mushrooms` now preserves the Java two-step fight confirmation boundary.
- `Mushrooms` now heals through `RunState::heal_with_source`, so Mark of the
  Bloom blocks the heal, and obtains `Parasite` through the normal event card
  obtain pipeline.
- `Mushrooms` fight rewards are locked for the `Odd Mushroom` already-owned
  case, which must reward `Circlet`.
- `Mushrooms` and `Masked Bandits` event-combat keys now use the Java encounter
  names, while CLI/full-run adapters still accept the older local aliases.
- `Masked Bandits` paid dialogue now completes on the same click Java uses to
  open the map instead of requiring one extra `[Leave]`.
- `Masked Bandits` paid gold loss is locked to `Event(MaskedBandits)` source,
  and the already-owned `Red Mask` reward case is locked to `Circlet`.

Tests:

- `fight_path_requires_java_confirm_screen_before_combat`
- `eat_uses_player_heal_and_show_card_obtain_semantics`
- `eat_heal_is_blocked_by_mark_of_the_bloom_but_curse_obtain_still_runs`
- `eat_parasite_can_be_blocked_by_omamori_like_show_card_and_obtain_effect`
- `fight_reward_gives_circlet_when_odd_mushroom_is_already_owned`
- `pay_path_opens_map_after_java_dialog_sequence_without_extra_leave_click`
- `fight_uses_java_event_encounter_key_and_event_rewards`
- `fight_reward_gives_circlet_when_red_mask_is_already_owned`

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
  `EventCombat`, including the fixed rare `returnRandomScreenlessRelic(RARE)`
  source.
- `EventCombatState` now carries `elite_trigger` separately from reward
  generation. This lets Colosseum Nobs behave like Java for combat-start relics
  such as Preserved Insect, Sling, and Slaver's Collar without generating
  ordinary elite rewards.
- CLI/full-run event combat initialization now passes `elite_trigger` into
  `CombatMeta::is_elite_fight`.

Tests:

- `leave_path_preserves_java_end_screen_before_map`
- `fight_path_generates_java_event_rewards_before_event_combat`
- `fight_reward_uses_rare_screenless_relic_pool`
- `first_fight_returns_to_event_room_without_rewards_or_elite_trigger`
- `second_fight_preserves_java_elite_trigger_without_normal_elite_rewards`

### Accursed Blacksmith relic obtain source

Java `events/shrines/AccursedBlacksmith.java` has two mechanics-relevant paths:

- Forge opens `gridSelectScreen.open(masterDeck.getUpgradableCards(), 1, ...)`
  and upgrades the selected master-deck card.
- Rummage creates a `Pain` via `ShowCardAndObtainEffect` and obtains
  `WarpedTongs` through `spawnRelicAndObtain`.

Rust already routed `Pain` through the event card-obtain helper, so Omamori and
event source metadata were preserved. The relic side still pushed
`WarpedTongs` directly into `run_state.relics`, bypassing the unified relic
obtain pipeline and dropping the event source.

Fixes:

- `WarpedTongs` now uses `RunState::obtain_relic_with_source(...,
  Event(AccursedBlacksmith))`.
- Forge now uses the shared Java `canUpgrade()` helper and direct calls to the
  disabled Forge option stay inert when no upgradable card exists.
- Added regression coverage for Forge pending-upgrade state, Java
  `masterDeck.getUpgradableCards()` selection, submitted upgrade event source,
  Rummage event sources, and Omamori blocking `Pain` without blocking
  `WarpedTongs`.

Tests:

- `forge_opens_upgrade_pending_choice_like_grid_select`
- `disabled_forge_does_not_open_empty_upgrade_selection`
- `forge_selection_uses_upgradable_cards_like_java`
- `forge_upgrades_selected_card_with_event_source`
- `rummage_uses_event_sources_for_pain_and_warped_tongs`
- `rummage_pain_can_be_blocked_by_omamori_without_blocking_warped_tongs`

### The Mausoleum relic-before-curse timing

Java `events/city/TheMausoleum.java` always calls
`miscRng.randomBoolean()` when opening the coffin, then forces the curse result
to true on A15+. On a cursed result it adds `ShowCardAndObtainEffect(new
Writhe())` to the effect list, but then immediately obtains the random
screenless relic through `spawnRelicAndObtain`.

Mechanically, that means the relic is owned before the Writhe obtain effect
resolves. This matters for relics such as `DarkstonePeriapt`: if Mausoleum rolls
Darkstone and also gives Writhe, Darkstone should see that curse obtain and
grant max HP. Omamori is the exception point: the Writhe effect constructor
checks Omamori before the random relic is obtained, so a newly rolled Omamori
must not block that same Writhe.

Fixes:

- Mausoleum now obtains the random relic before routing Writhe through the event
  card obtain helper, but uses a pre-relic Omamori snapshot for interception.
- Added regression coverage for Darkstone timing, A15 RNG consumption, existing
  Omamori blocking Writhe, and newly obtained Omamori not blocking Writhe.

Tests:

- `cursed_open_obtains_relic_before_writhe_effect_resolves_like_java`
- `cursed_open_still_rolls_misc_rng_before_a15_forces_curse`
- `omamori_blocks_writhe_after_relic_obtain_so_darkstone_does_not_trigger`
- `newly_obtained_omamori_does_not_block_writhe_from_same_open`

### Cursed Tome HP_LOSS and book reward

Java `events/city/CursedTome.java` uses
`AbstractDungeon.player.damage(new DamageInfo(null, amount, HP_LOSS))` for each
page and for the final book/stop-reading damage. That damage bypasses block and
owner attack callbacks, but `AbstractPlayer.damage` still runs relic
`onLoseHpLast`, so `Tungsten Rod` reduces the HP loss by 1.

Java `randomBook()` also always rolls `AbstractDungeon.miscRng.random(size - 1)`
after constructing the possible-book list. If all three book relics are already
owned, the list contains only `Circlet`, but `miscRng.random(0)` still consumes
one gameplay RNG call.

Fixes:

- Added `content::events::apply_player_hp_loss_damage(...)` for event-owned
  Java `DamageInfo(null, amount, HP_LOSS)` semantics.
- `CursedTome`, `SensoryStone`, `WomanInBlue`, and `GremlinWheelGame` now share
  that helper instead of duplicating local Tungsten Rod handling.
- `CursedTome` book reward now consumes `misc_rng` even when only `Circlet` is
  possible.

Tests:

- `page_damage_uses_java_hp_loss_so_tungsten_rod_can_reduce_to_zero`
- `take_book_final_damage_uses_hp_loss_and_opens_book_reward`
- `random_book_consumes_misc_rng_even_when_only_circlet_is_possible`

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
- `Mushrooms` Parasite and `KnowingSkull` random colorless card now use the
  same event-owned obtain helper.
- Remaining production direct `add_card_to_deck_*` calls are intentional
  special cases: pre-relic Omamori snapshots (`Addict`, `BigFish`,
  `Mausoleum`), previewed upgrade copies (`MatchAndKeep`, `TheLibrary`), and
  `NoteForYourself`'s manual profile-card obtain path. The only plain
  `add_card_to_deck` occurrence in event modules is a `falling.rs` unit-test
  setup helper.

Validation:

- Static scan: `rg "add_card_to_deck" src/content/events -g "*.rs"`
- `cargo test --all-targets`

## Current High-Risk Event Areas

- Selection choice preconditions still need deeper event-by-event review.
  Some Java handlers check candidate availability only when clicked, not when
  drawing the button, and several Rust modules still simplify those UI states.
- Event HP/max-HP/gold direct mutations still need the same domain-source pass
  that card obtains just received. `BigFish`, `Cleric`, `GoldenIdol`,
  `GoldenWing`,
  `FaceTrader`, `ForgottenAltar`, `Ghosts`, `Vampires`, `MoaiHead`, and
  `GremlinWheelGame`, `MindBloom`, `WindingHalls`, and `SensoryStone` are now
  covered; `ShiningLight` is also covered for damage and random upgrade
  sources. `Nest` is covered for gold, damage, and Ritual Dagger obtain source.
  `BonfireElementals` and `BonfireSpirits` are covered for rarity reward relic,
  heal, full-heal, and max-HP paths. `Lab` now opens potion rewards instead of
  directly filling potion slots. `WomanInBlue` now opens potion rewards instead
  of directly filling potion slots and sources its A15 HP_LOSS leave damage.
  `TombRedMask` now routes paid `Red Mask` obtain through the relic obtain
  helper. `CursedTome` now uses shared Java HP_LOSS event semantics and preserves
  random-book RNG consumption. `AccursedBlacksmith` now routes `WarpedTongs`
  through the event-sourced relic obtain pipeline. `Mausoleum` now preserves the
  Java relic-before-Writhe effect timing. `DrugDealer` now routes Inject Mutagens
  relics through the event-sourced relic obtain pipeline. `KnowingSkull` now
  applies repeatable costs through the shared Java HP_LOSS event helper.
  `Addict` now preserves the Java Omamori timing around the stolen relic and
  Shame obtain. The remaining direct writes should be handled event-by-event
  against Java source.

## Validation

- `cargo test --all-targets`
- Current result after this pass: `993 passed`.
