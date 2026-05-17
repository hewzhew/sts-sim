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
- Added `DomainEvent::PotionLost` and `RunState::remove_potion_at_with_source`
  so giving a potion is visible as an event-sourced resource cost.
- Giving a card removes it with `DomainEventSource::Event(WeMeetAgain)`.
- The relic obtained from potion / gold / card trades now uses
  `obtain_relic_with_source(..., Event(WeMeetAgain))` rather than the generic
  deck-mutation source.

Tests:

- `card_trade_option_exposes_specific_remove_effect`
- `card_trade_removes_card_and_obtains_relic_with_event_source`
- `potion_trade_removes_selected_potion_and_obtains_relic_with_event_source`

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
  and leave transition under Tungsten Rod.

Tests:

- `potion_reward_hp_loss_respects_tungsten_and_increments_only_potion_cost`
- `gold_reward_hp_loss_respects_tungsten_then_grants_gold`
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

Tests:

- `init_consumes_java_enemy_roll_and_stores_enemy_in_state`
- `combat_trigger_uses_stored_java_enemy_key`
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

Fixes:

- Cleric heal amount now uses Java's float-cast truncation.
- Cleric heal now calls `RunState::heal_with_source(..., Event(Cleric))`.
- Paying gold remains separate from healing, so `Mark of the Bloom` blocks only
  the heal and not the gold cost.

Tests:

- `heal_amount_uses_java_float_cast_not_rounding`
- `heal_cost_is_paid_even_when_mark_of_the_bloom_blocks_heal`

### Golden Wing remove damage and attack gate

Java `events/exordium/GoldenWing.java` handles the remove-card option by first
calling:

```text
AbstractDungeon.player.damage(new DamageInfo(AbstractDungeon.player, damage))
```

That is normal player damage, not a direct HP assignment. In practice, the
out-of-combat simulator currently needs at least the `onLoseHpLast` portion
that affects HP loss such as `Tungsten Rod`.

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
- Golden Wing's attack option now uses upgraded master-deck attack damage,
  matching Java's card instance `baseDamage` gate.

Tests:

- `remove_path_damage_uses_event_source_before_purge_selection`
- `remove_path_damage_respects_tungsten_rod_like_java_player_damage`
- `attack_option_uses_upgraded_master_deck_base_damage_like_java`
- `attack_option_does_not_count_non_attack_base_damage`

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
- `Shed Blood` now gains max HP through `gain_max_hp_with_source` and then
  applies sourced normal damage with the Java `Tungsten Rod` reduction path.
- `Desecrate` continues through the event card-obtain helper; regression
  coverage now verifies that Omamori can block the `Decay` without bypassing
  the event pipeline.

Tests:

- `offering_golden_idol_replaces_same_relic_slot_with_bloody_idol`
- `offering_golden_idol_with_existing_bloody_idol_grants_circlet_without_losing_idol`
- `shed_blood_increases_max_hp_then_heals_then_takes_java_damage`
- `shed_blood_damage_respects_tungsten_after_max_hp_heal`
- `desecrate_decay_uses_event_obtain_pipeline_and_omamori_can_block_it`

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

Tests:

- `ingest_mutagens_obtains_jax_with_event_source`
- `inject_mutagens_obtains_relic_with_event_source`
- `inject_mutagens_grants_circlet_through_obtain_pipeline_when_already_owned`

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

Tests:

- `accept_loses_max_hp_and_obtains_apparitions_with_event_source`
- `accept_on_ascension_fifteen_obtains_three_apparitions`
- `accept_loses_max_hp_replaces_starter_strikes_with_event_sources`
- `give_vial_removes_relic_without_max_hp_loss_and_replaces_strikes`

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

Tests:

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

Tests:

- `gold_result_uses_act_scaled_gold_and_event_source`
- `relic_result_opens_reward_screen_with_one_relic_reward`
- `curse_result_uses_obtain_pipeline_so_omamori_can_block_decay`
- `purge_result_opens_non_bottled_purge_selection_when_possible`
- `full_heal_uses_java_heal_source_and_respects_mark_of_the_bloom`
- `full_heal_emits_event_source_without_mark`
- `hp_loss_result_uses_source_and_can_reduce_hp_to_zero`
- `hp_loss_result_applies_tungsten_rod_on_lose_hp_last`

### Mind Bloom Mark and high-floor heal

Java `events/beyond/MindBloom.java` uses `spawnRelicAndObtain` for the
`[I am Awake]` Mark of the Bloom branch, and the high-floor `[I am Rich]`
variant calls:

```text
player.heal(player.maxHealth)
ShowCardAndObtainEffect(new Doubt())
```

Fixes:

- The Mark of the Bloom branch now obtains the relic through
  `obtain_relic_with_source(..., Event(MindBloom))` instead of pushing directly
  into `run_state.relics`.
- The high-floor heal branch now uses `heal_with_source`, preserving Java heal
  hooks such as `Mark of the Bloom`, before obtaining `Doubt` through the event
  card path.

Tests:

- `remember_obtains_mark_of_the_bloom_with_event_source`
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

Tests:

- `enter_light_damage_and_random_upgrades_use_event_source`
- `enter_light_normal_damage_applies_torii_then_tungsten`
- `leave_does_not_damage_or_upgrade`

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

Tests:

- `common_offer_heals_with_event_source`
- `rare_offer_matches_java_max_hp_then_full_heal_sequence`
- `heal_rewards_obey_mark_of_the_bloom`
- `curse_offer_obtains_spirit_poop_with_event_source`
- `common_offer_heals_with_spirits_event_source`
- `curse_offer_obtains_spirit_poop_with_spirits_event_source`

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
complete, and opens the combat reward screen. The purchase buttons are gated by
gold only; potion capacity and Sozu are handled later by the reward screen.

The A15 leave branch applies
`DamageInfo(null, ceil(maxHealth * 0.05), HP_LOSS)`. HP_LOSS bypasses block and
Torii, but still reaches relic `onLoseHpLast`, so `Tungsten Rod` reduces it.

Fixes:

- Buying potions now opens `EngineState::RewardScreen` containing potion reward
  items instead of calling `obtain_potion` directly.
- Potion purchase semantics no longer require an empty potion slot.
- A15 leave damage now emits `HpChanged` with `Event(WomanInBlue)` and applies
  Tungsten Rod's HP-loss reduction.

Tests:

- `three_potion_option_exposes_trade_semantics`
- `buying_potions_opens_reward_screen_without_filling_slots_directly`
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
relic list.

Tests:

- `paying_without_mask_loses_all_gold_and_obtains_red_mask_with_event_source`
- `wearing_existing_mask_gains_222_gold_with_event_source`
- `choices_preserve_java_button_indices_when_mask_is_missing`

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
- Added regression coverage for Forge pending-upgrade state, Rummage event
  sources, and Omamori blocking `Pain` without blocking `WarpedTongs`.

Tests:

- `forge_opens_upgrade_pending_choice_like_grid_select`
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
grant max HP.

Fixes:

- Mausoleum now obtains the random relic before routing Writhe through the event
  card obtain helper.
- Added regression coverage for Darkstone timing, A15 RNG consumption, and
  Omamori blocking Writhe after the relic has already been obtained.

Tests:

- `cursed_open_obtains_relic_before_writhe_effect_resolves_like_java`
- `cursed_open_still_rolls_misc_rng_before_a15_forces_curse`
- `omamori_blocks_writhe_after_relic_obtain_so_darkstone_does_not_trigger`

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
  that card obtains just received. `BigFish`, `Cleric`, `GoldenWing`,
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
  applies repeatable costs through the shared Java HP_LOSS event helper. The
  remaining direct writes should be handled event-by-event against Java source.

## Validation

- `cargo test --all-targets`
- Current result after this pass: `891 passed`.
