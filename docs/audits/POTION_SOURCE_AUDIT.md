# Potion Source Audit

Purpose:
- Compare Rust potion pool, use/discard legality, and potion effects against the
  decompiled Java source under `D:/rust/cardcrawl/potions` and
  `D:/rust/cardcrawl/helpers/PotionHelper.java`.
- Preserve mechanical semantics and RNG consumption. UI-only sound, flash,
  particles, hitboxes, and render state are intentionally excluded unless they
  change state, RNG, ordering, or visible legal decisions.

## Pool Order And Availability

Java evidence:
- `PotionHelper.getPotions(chosenClass, false)` prepends the three
  class-specific potions for the current class.
- Shared potions are appended in a fixed order.
- `PotionHelper.getPotions(null, true)` prepends all twelve class-specific
  potions, then appends the same shared potion list.
- `AbstractDungeon.returnRandomPotion()` first rolls rarity with `potionRng`
  and then rejection-samples from `PotionHelper.getRandomPotion()`.
- `AbstractDungeon.returnRandomPotion(rarity, true)` rejects `Fruit Juice`.
- In the Java source, `returnRandomPotion(rarity, true)` also initializes
  `spamCheck` to true after the first flat `PotionHelper.getRandomPotion()`
  call, which means that first flat roll is always consumed and discarded before
  the limited rejection-sampling loop can return.
- `AbstractDungeon.returnTotallyRandomPotion()` delegates directly to
  `PotionHelper.getRandomPotion()`.

Rust result:
- `potions_for_class` is the canonical Java-order pool for RNG selection.
- `PotionClass::Any` is the Java `getAll` / upload-style list, not a normal
  run class pool.
- `random_potion` models the Java rarity roll and rejection-sampling path.
- The limited path consumes Java's discarded initial flat potion roll before
  accepting a non-Fruit-Juice result.
- `random_potion_any` models the flat `PotionHelper.getRandomPotion()` path.

Coverage:
- `potion_helper_pools_match_java_order_for_all_classes`
- `limited_random_potion_discards_initial_flat_roll_like_java`

## Use And Discard Legality

Java evidence:
- `AbstractPotion.canDiscard()` is false only during `WeMeetAgain`.
- Base `AbstractPotion.canUse()` requires a combat room, living monsters,
  `turnHasEnded == false`, and not `WeMeetAgain`.
- Only `BloodPotion`, `FruitJuice`, and `EntropicBrew` override `canUse()` to
  allow non-combat use outside `WeMeetAgain`, while still rejecting combat use
  after the turn has ended.
- `FairyPotion.canUse()` always returns false.
- `SmokeBomb.canUse()` delegates to base `canUse()` and then rejects boss
  monsters and monsters with `BackAttack`.

Rust result:
- Run-level potion actions expose only Blood Potion, Fruit Juice, and Entropic
  Brew outside combat.
- Combat potion legality rejects Fairy Potion, dead/ended combat states, and
  Java's Smoke Bomb boss/back-attack cases.
- We Meet Again blocks both use and discard.
- Combat action enumeration now exposes `DiscardPotion` for owned potions with
  `can_discard == true`, even when `can_use == false`, matching the Java
  potion pop-up's separate discard button.
- Combat discard input validates `can_discard` before queuing the low-level
  discard action, so direct client input cannot bypass Java affordance truth.

Coverage:
- `potion_can_use_overrides_match_java_sources`
- `non_combat_potion_observation_uses_java_can_use_overrides`
- `we_meet_again_blocks_potion_use_and_discard_observation`
- `combat_potion_execution_respects_java_can_use_gate`
- `engine_local_moves_skip_unusable_potions`
- `combat_discard_potion_input_respects_java_can_discard_affordance`
- `combat_action_mask_exposes_discardable_unusable_potions`

Open audit work:
- Continue per-potion effect comparison against each Java `use()` method.
- Recheck Toy Ornithopter / Sacred Bark ordering around run-level potion use.
- Recheck `ObtainPotionAction` and out-of-combat `ObtainPotionEffect` ordering
  where reward screens or event flows are involved.

## High-Risk `use()` Effects - First Pass

### Liquid Memories

Status: `wrong-fixed`

Java evidence:
- `LiquidMemories.use()` queues `BetterDiscardPileToHandAction(this.potency, 0)`.
- `BetterDiscardPileToHandAction` auto-moves all discard cards only when
  `discardPile.size() <= numberOfCards` and the action is not optional.
- Otherwise it opens `GridCardSelectScreen` with `anyNumber == false`, so the
  screen closes only after exactly `numberOfCards` cards are selected.
- The selected cards receive `setCostForTurn(newCost)`.

Rust result:
- Existing auto-move handling already preserved the full-hand case: selected
  discard cards are left in discard if the hand is full.
- Fixed the pending grid-select path to require exact potency instead of
  allowing any count from `1..=potency`. This matters with Sacred Bark, where
  Liquid Memories has potency `2`.

Coverage:
- `liquid_memories_auto_move_does_not_drop_cards_when_hand_fills`
- `liquid_memories_sacred_bark_grid_select_requires_exact_potency`

### Fire Potion

Status: `wrong-fixed`

Java evidence:
- `FirePotion.use()` creates `DamageInfo(AbstractDungeon.player, potency,
  DamageType.THORNS)`.
- It immediately calls `info.applyEnemyPowersOnly(target)` before queuing
  `DamageAction`.
- That means target final-receive powers such as Nemesis-style
  `IntangiblePower` are baked into the queued damage value.

Rust result:
- Fixed combat potion use so Fire Potion pre-applies target final-receive powers
  to the THORNS damage before queuing the damage action.

Coverage:
- `fire_potion_applies_enemy_final_receive_before_damage_action_like_java`

### Blood Potion

Status: `wrong-fixed`

Java evidence:
- `BloodPotion.use()` computes `(int)(player.maxHealth * potency / 100.0f)`
  immediately when the potion is used.
- In combat it queues a `HealAction` with that fixed amount.
- Unlike `FairyPotion.use()`, Blood Potion does not apply a minimum-one heal
  rule before calling heal.

Rust result:
- Fixed combat Blood Potion use so it queues a fixed heal amount computed at
  use time instead of using the generic negative percentage sentinel at heal
  execution time.

Coverage:
- `blood_potion_queues_fixed_use_time_heal_amount_without_minimum_one`
- `blood_potion_heal_amount_is_computed_when_used_not_when_heal_executes`

## `use()` Effects Reviewed Without Code Change

Status: `reviewed-clean`

Java evidence and Rust result:
- Basic immediate/queued effects match their Java action type and potency:
  `BlockPotion`, `EnergyPotion`, `SwiftPotion`, `BlessingOfTheForge`,
  `BottledMiracle`, `CunningPotion`, and `PotionOfCapacity`.
- Apply-power potions match target/source/power/amount ordering through the
  shared `ApplyPowerAction` path: `PoisonPotion`, `WeakenPotion`, `FearPotion`,
  `StrengthPotion`, `DexterityPotion`, `SpeedPotion`, `SteroidPotion`,
  `FocusPotion`, `AncientPotion`, `RegenPotion`, `EssenceOfSteel`,
  `LiquidBronze`, `DuplicationPotion`, `GhostInAJar`, `HeartOfIron`, and
  `CultistPotion`.
- Discovery potions match Java `DiscoveryAction`: Attack/Skill/Power are typed
  discoveries with skip enabled; Colorless is colorless discovery with skip
  disabled; Sacred Bark changes amount/copies, not the three-option offer.
- Stance/Ambrosia match Java `ChooseOneAction(ChooseWrath, ChooseCalm)` and
  `ChangeStanceAction("Divinity")` mechanics.
- `GamblersBrew`, `Elixir`, `SneckoOil`, `DistilledChaosPotion`, and
  `EssenceOfDarkness` had already been checked against their Java actions; the
  important RNG/queue timing paths are covered by existing tests.

Open edge note:
- Java potion targeting UI excludes `isDying` and controller mode also rejects
  `halfDead`; Rust target validation currently uses the stricter live-monster
  target set. This stays unchanged until a concrete Java-visible targetability
  case proves a mechanical mismatch.
