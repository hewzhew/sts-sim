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
- `AbstractDungeon.returnTotallyRandomPotion()` delegates directly to
  `PotionHelper.getRandomPotion()`.

Rust result:
- `potions_for_class` is the canonical Java-order pool for RNG selection.
- `PotionClass::Any` is the Java `getAll` / upload-style list, not a normal
  run class pool.
- `random_potion` models the Java rarity roll and rejection-sampling path.
- `random_potion_any` models the flat `PotionHelper.getRandomPotion()` path.

Coverage:
- `potion_helper_pools_match_java_order_for_all_classes`

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

Coverage:
- `potion_can_use_overrides_match_java_sources`
- `non_combat_potion_observation_uses_java_can_use_overrides`
- `we_meet_again_blocks_potion_use_and_discard_observation`
- `combat_potion_execution_respects_java_can_use_gate`

Open audit work:
- Continue per-potion effect comparison against each Java `use()` method.
- Recheck Toy Ornithopter / Sacred Bark ordering around run-level potion use.
- Recheck `ObtainPotionAction` and out-of-combat `ObtainPotionEffect` ordering
  where reward screens or event flows are involved.
