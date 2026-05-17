# Ironclad Relic Source Audit

Purpose:
- Compare Rust relic mechanics available to Ironclad runs against the
  decompiled Java source under `D:/rust/cardcrawl/relics`.
- Preserve gameplay semantics even when the Java behavior is odd.
- Exclude UI/VFX-only behavior unless it changes state, RNG, ordering, or
  observable combat decisions.

Cards are already tracked in `docs/audits/IRONCLAD_CARD_SOURCE_AUDIT.md`.
This file starts the same evidence-driven pass for Ironclad relics.

## Pool-Level Correction - Normal vs End Relic Draws

Java evidence:
- `AbstractDungeon.returnRandomRelicKey(tier)` removes from index `0`.
- `AbstractDungeon.returnEndRandomRelicKey(tier)` removes from the end for
  common/uncommon/rare/shop pools.
- Shop relic generation and Courier replacement use `returnRandomRelicEnd`;
  normal combat/chest reward paths use `returnRandomRelic`.
- If a normal front candidate fails `canSpawn`, Java falls back to the end
  path for that tier.
- `AbstractDungeon.initializeRelicList()` populates each tier pool, shuffles
  every full pool with `Collections.shuffle(pool, new Random(relicRng.randomLong()))`,
  and only then removes `relicsToRemoveOnStart`.
- `RelicLibrary.populateRelicPool()` iterates `sharedRelics.entrySet()` and
  then the matching class-specific `HashMap.entrySet()`, so pre-shuffle relic
  pool order is Java HashMap traversal order rather than source `add(...)`
  order.

Rust result:
- `RunState::random_relic_by_tier` now models the normal front draw path.
- `RunState::random_relic_end_by_tier` now models the shop/end draw path.
- Both paths share the same `canSpawn` context, so shop/end paths no longer
  bypass bottled/floor/class/boss relic gates.
- Shop generation and Courier relic-slot replacement now use the end path.
- `RunState::init_relic_pools` now shuffles before removing already-owned
  relics, preserving Java's remaining-pool order.
- `build_relic_pool` now uses Java HashMap traversal order for shared and
  class-specific registered relic maps.

Coverage:
- `normal_and_end_relic_paths_consume_opposite_pool_ends_like_java`
- `normal_relic_rewards_can_return_bottled_relics_but_screenless_rewards_skip_them`
- `ectoplasm_can_spawn_only_in_act_one_and_blocks_gold_gain`
- `rare_run_relic_can_spawn_gates_match_java_sources`
- `init_relic_pools_shuffles_before_removing_owned_relics_like_java`
- `relic_pool_build_order_matches_java_hashmap_traversal`

## Pool-Level Correction - Relic `canSpawn` Gates

Java evidence:
- `AncientTeaSet`, `CeramicFish`, `DarkstonePeriapt`, `DreamCatcher`,
  `FrozenEgg2`, `JuzuBracelet`, `MealTicket`, `MeatOnTheBone`, `Omamori`,
  `MoltenEgg2`, `PotionBelt`, `PrayerWheel`, `QuestionCard`, `RegalPillow`,
  `SingingBowl`, and `ToxicEgg2` return
  `Settings.isEndless || AbstractDungeon.floorNum <= 48`.
- `PreservedInsect` returns `Settings.isEndless || AbstractDungeon.floorNum <= 52`.
- `TinyChest` returns `Settings.isEndless || AbstractDungeon.floorNum <= 35`.
- `Courier`, `MawBank`, `OldCoin`, and `SmilingMask` add a current-room guard:
  they reject spawning when `AbstractDungeon.getCurrRoom() instanceof ShopRoom`.
- `Girya`, `PeacePipe`, and `Shovel` reject `floorNum >= 48` and reject if two
  of the three campfire relics are already owned.

Rust result:
- The Rust simulator currently models standard non-Endless runs, so the
  `Settings.isEndless` escape hatch is intentionally not represented.
- `RelicSpawnContext` now carries the current map room type, allowing relic
  pool selection to reject the Java ShopRoom-blocked relics when appropriate.
- The floor gates above are enforced in both front-draw and end-draw relic
  paths.

Coverage:
- `rare_run_relic_can_spawn_gates_match_java_sources`

## Batch 1 - Blood / Bloodied / Vulnerable Relics

### Burning Blood

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/BurningBlood.java`

Rust source:
- `src/content/relics/burning_blood.rs`
- `src/content/relics/hooks.rs`
- `src/content/relics/mod.rs`

Java evidence:
- Constructor: ID `"Burning Blood"`, tier `STARTER`, landing sound `MAGICAL`.
- `onVictory`: flashes, queues a UI-only `RelicAboveCreatureAction`, then heals
  the player for `6` only if `p.currentHealth > 0`.

Rust result:
- Tier and victory subscription match Java.
- Fixed `on_victory` to inspect `CombatState` and emit no heal when player HP
  is `0` or lower.
- UI-only relic flash / above-creature visual action is intentionally not
  represented.

Coverage:
- `ironclad_blood_skull_and_frog_relic_metadata_matches_java_sources`
- `burning_and_black_blood_victory_heal_matches_java_current_hp_guard`

### Black Blood

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/BlackBlood.java`

Rust source:
- `src/content/relics/black_blood.rs`
- `src/content/relics/hooks.rs`
- `src/content/relics/mod.rs`

Java evidence:
- Constructor: ID `"Black Blood"`, tier `BOSS`, landing sound `FLAT`.
- `onVictory`: flashes, queues a UI-only `RelicAboveCreatureAction`, then heals
  the player for `12` only if `p.currentHealth > 0`.
- `canSpawn`: returns true only if the player has `"Burning Blood"`.

Rust result:
- Tier and victory subscription match Java.
- Fixed `on_victory` to inspect `CombatState` and emit no heal when player HP
  is `0` or lower.
- Spawn gating is a relic-pool/reward-generation concern and is not handled by
  the combat victory hook.
- UI-only relic flash / above-creature visual action is intentionally not
  represented.

Coverage:
- `ironclad_blood_skull_and_frog_relic_metadata_matches_java_sources`
- `burning_and_black_blood_victory_heal_matches_java_current_hp_guard`

### Red Skull

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/RedSkull.java`

Rust source:
- `src/content/relics/red_skull.rs`
- `src/content/relics/hooks.rs`
- `src/engine/action_handlers/damage.rs`

Java evidence:
- Constructor: ID `"Red Skull"`, tier `COMMON`, landing sound `FLAT`.
- `atBattleStart`: resets internal active state, then queues an action that
  applies Strength `3` if the player is bloodied.
- `onBloodied`: during combat, if not already active, applies Strength `3`.
- `onNotBloodied`: during combat, if active, applies Strength `-3` and clears
  pulse/active state.
- `onVictory`: clears pulse and active state.

Rust result:
- Tier and battle-start subscription match Java.
- Existing battle-start and HP-threshold hooks apply Strength `3` when crossing
  into bloodied state and Strength `-3` when crossing out.
- Rust does not model pulse/flash UI state. Active-state behavior is represented
  through threshold crossing plus normal combat power state.

Coverage:
- `ironclad_blood_skull_and_frog_relic_metadata_matches_java_sources`
- `red_skull_threshold_hooks_match_java_bloodied_edges`

### Paper Frog

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/PaperFrog.java`

Rust source:
- `src/content/relics/paper_frog.rs`
- `src/content/relics/hooks.rs`
- `src/content/powers/mod.rs`

Java evidence:
- Constructor: ID `"Paper Frog"`, tier `UNCOMMON`, landing sound `FLAT`.
- Constant `VULN_EFFECTIVENESS = 1.75f`.
- No explicit hook method exists on the relic class; the vulnerable damage
  multiplier is consumed by the damage calculation path.

Rust result:
- Tier and vulnerable-multiplier subscription match Java.
- Rust returns `1.75` only when the vulnerable owner is an enemy target. Player
  vulnerable remains the normal `1.5` unless another relic, such as Odd
  Mushroom, modifies it.

Coverage:
- `ironclad_blood_skull_and_frog_relic_metadata_matches_java_sources`
- `paper_frog_vulnerable_multiplier_applies_only_to_enemy_targets`

## Batch 2 - Strength / Debuff / Exhaust / Heal Relics

### Brimstone

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/Brimstone.java`

Rust source:
- `src/content/relics/brimstone.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Brimstone"`, tier `SHOP`, landing sound `CLINK`.
- `atTurnStart`: flashes, queues a UI-only `RelicAboveCreatureAction`, then
  calls `addToTop` for Strength `2` on the player.
- The same method loops all current monsters and calls `addToTop` for
  Strength `1` on each monster, with each monster as its own source.

Rust result:
- Tier and turn-start subscription match Java.
- Fixed the player Strength action to use top insertion rather than bottom
  insertion.
- Fixed enemy Strength source from player to the target monster itself.
- Emitted top actions in Java's effective execution order, because repeated
  `addToTop` calls execute later calls first.
- UI-only relic flash / above-creature visual action is intentionally not
  represented.

Coverage:
- `ironclad_brimstone_belt_ashes_flower_relic_metadata_matches_java_sources`
- `brimstone_turn_start_matches_java_strength_sources_and_top_order`

### Champion Belt

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/ChampionsBelt.java`
- `D:/rust/cardcrawl/actions/common/ApplyPowerAction.java`

Rust source:
- `src/content/relics/champion_belt.rs`
- `src/content/relics/hooks.rs`
- `src/engine/action_handlers/powers.rs`

Java evidence:
- Constructor: ID `"Champion Belt"`, tier `RARE`, landing sound `HEAVY`.
- `ChampionBelt.onTrigger(target)`: queues a UI-only relic action, then queues
  Weak `1` on `target` from the player.
- `ApplyPowerAction` triggers the relic only when the source is the player, the
  target is not the source, the applied power is Vulnerable, and the target
  does not have Artifact.

Rust result:
- Tier and apply-power subscription match Java.
- Fixed the relic hook to receive the power source and inspect current target
  powers.
- Fixed false triggers from non-player sources and fixed the missing Artifact
  guard.
- UI-only relic flash / above-creature visual action is intentionally not
  represented.

Coverage:
- `ironclad_brimstone_belt_ashes_flower_relic_metadata_matches_java_sources`
- `champion_belt_respects_java_player_source_and_artifact_guard`

### Charon's Ashes

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/CharonsAshes.java`

Rust source:
- `src/content/relics/charons_ashes.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Charon's Ashes"`, tier `RARE`, landing sound `MAGICAL`.
- `onExhaust`: flashes, then calls `addToTop` with
  `DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(3, true),
  DamageInfo.DamageType.THORNS, FIRE)`.
- The per-enemy relic-above-creature actions are UI/VFX only.

Rust result:
- Tier and exhaust subscription match Java.
- Fixed `DamageAllEnemies` source from player to `NO_SOURCE`.
- Fixed damage type from `Normal` to `Thorns`; this preserves Java's pure relic
  damage path and avoids player attack modifiers.
- UI-only relic flash / above-creature visual actions are intentionally not
  represented.

Coverage:
- `ironclad_brimstone_belt_ashes_flower_relic_metadata_matches_java_sources`
- `charons_ashes_exhaust_damage_matches_java_thorns_null_source`

### Magic Flower

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/MagicFlower.java`

Rust source:
- `src/content/relics/magic_flower.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Magic Flower"`, tier `RARE`, landing sound `SOLID`.
- `onPlayerHeal`: during combat only, returns
  `MathUtils.round(healAmount * 1.5f)`; outside combat, returns the original
  heal amount.

Rust result:
- Tier and heal-calculation subscription match Java.
- Existing combat hook applies `round(amount * 1.5)` for combat healing.
- The hook is combat-state scoped; out-of-combat healing must not route through
  this combat hook.

Coverage:
- `ironclad_brimstone_belt_ashes_flower_relic_metadata_matches_java_sources`
- `magic_flower_combat_heal_rounding_matches_java_mathutils_round`

## Batch 3 - Wound / HP-Loss Relics

### Mark of Pain

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/MarkOfPain.java`

Rust source:
- `src/content/relics/mark_of_pain.rs`
- `src/content/relics/hooks.rs`
- `src/content/relics/mod.rs`

Java evidence:
- Constructor: ID `"Mark of Pain"`, tier `BOSS`, landing sound `MAGICAL`.
- `atBattleStart`: flashes, queues a UI-only `RelicAboveCreatureAction`, then
  queues `MakeTempCardInDrawPileAction(new Wound(), 2, true, true)`.
- The four-argument constructor maps to `randomSpot=true`,
  `autoPosition=true`, and `toBottom=false`.
- `onEquip` increments `player.energy.energyMaster`; `onUnequip`
  decrements it.

Rust result:
- Tier, battle-start subscription, and permanent energy delta match Java.
- Battle start emits two Wounds into random draw-pile positions with
  `to_bottom=false`.
- UI-only relic flash / above-creature visual action and card spawn animation
  auto-position are intentionally not represented.

Coverage:
- `ironclad_pain_cube_clay_relic_metadata_matches_java_sources`
- `mark_of_pain_battle_start_matches_java_wound_generation_and_energy`

### Runic Cube

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/RunicCube.java`

Rust source:
- `src/content/relics/runic_cube.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Runic Cube"`, tier `BOSS`, landing sound `FLAT`.
- `wasHPLost(damageAmount)`: during combat, if `damageAmount > 0`, flashes,
  then calls `addToTop(new DrawCardAction(player, 1))` and `addToTop` for a
  UI-only relic action.

Rust result:
- Tier and HP-loss subscription match Java.
- Hook emits `DrawCards(1)` with top insertion only for positive HP loss.
- UI-only relic flash / above-creature visual action is intentionally not
  represented.

Coverage:
- `ironclad_pain_cube_clay_relic_metadata_matches_java_sources`
- `runic_cube_hp_loss_hook_matches_java_positive_damage_guard`

### Self-Forming Clay

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/SelfFormingClay.java`

Rust source:
- `src/content/relics/self_forming_clay.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Self Forming Clay"`, tier `UNCOMMON`, landing sound `FLAT`.
- `wasHPLost(damageAmount)`: during combat, if `damageAmount > 0`, flashes and
  calls `addToTop(new ApplyPowerAction(player, player,
  new NextTurnBlockPower(player, 3, this.name), 3))`.

Rust result:
- Tier and HP-loss subscription match Java.
- Fixed the relic hook itself to guard on positive HP loss, rather than relying
  only on the current caller to filter zero/negative values.
- Emits `NextTurnBlock` amount `3` on the player with top insertion.
- UI-only relic flash is intentionally not represented.

Coverage:
- `ironclad_pain_cube_clay_relic_metadata_matches_java_sources`
- `self_forming_clay_hp_loss_hook_matches_java_positive_damage_guard`

## Shared Relic Batch 1 - Common Battle-Start Relics

### Akabeko

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/Akabeko.java`

Rust source:
- `src/content/relics/akabeko.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Akabeko"`, tier `COMMON`, landing sound `CLINK`.
- `atBattleStart`: flashes, calls `addToTop` for Vigor `8` on the player,
  then calls `addToTop` for a UI-only relic action.

Rust result:
- Tier and battle-start subscription match Java.
- Emits player Vigor `8` with top insertion.
- UI-only relic flash / above-creature visual action is intentionally not
  represented.

Coverage:
- `shared_common_battle_start_relic_metadata_matches_java_sources`
- `akabeko_anchor_and_bag_of_preparation_battle_start_actions_match_java_sources`

### Anchor

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/Anchor.java`

Rust source:
- `src/content/relics/anchor.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Anchor"`, tier `COMMON`, landing sound `HEAVY`.
- `atBattleStart`: flashes, queues a UI-only relic action, then queues
  `GainBlockAction(player, player, 10)` with `addToBot`.
- `justEnteredRoom` only clears grayscale UI state.

Rust result:
- Tier and battle-start subscription match Java.
- Emits player block `10` with bottom insertion.
- UI-only relic flash / above-creature / grayscale state is intentionally not
  represented.

Coverage:
- `shared_common_battle_start_relic_metadata_matches_java_sources`
- `akabeko_anchor_and_bag_of_preparation_battle_start_actions_match_java_sources`

### Bag of Marbles

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/BagOfMarbles.java`

Rust source:
- `src/content/relics/bag_of_marbles.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Bag of Marbles"`, tier `COMMON`, landing sound `FLAT`.
- `atBattleStart`: loops every monster in the room, queues a UI-only relic
  action, then queues Vulnerable `1` from the player to that monster with
  `addToBot`.
- The loop itself does not skip dying or escaped monsters; invalid targets are
  handled by `ApplyPowerAction` when the action executes.

Rust result:
- Tier and battle-start subscription match Java.
- Fixed Rust to emit an ApplyPower action for every current monster rather than
  filtering dying/escaped monsters before the action queue.
- UI-only relic flash / above-creature visual actions are intentionally not
  represented.

Coverage:
- `shared_common_battle_start_relic_metadata_matches_java_sources`
- `bag_of_marbles_queues_vulnerable_for_every_current_monster`

### Bag of Preparation

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/BagOfPreparation.java`

Rust source:
- `src/content/relics/bag_of_preparation.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Bag of Preparation"`, tier `COMMON`, landing sound `FLAT`.
- `atBattleStart`: flashes, queues a UI-only relic action, then queues
  `DrawCardAction(player, 2)` with `addToBot`.

Rust result:
- Tier and battle-start subscription match Java.
- Emits draw `2` with bottom insertion.
- UI-only relic flash / above-creature visual action is intentionally not
  represented.

Coverage:
- `shared_common_battle_start_relic_metadata_matches_java_sources`
- `akabeko_anchor_and_bag_of_preparation_battle_start_actions_match_java_sources`

## Shared Relic Batch 2 - Common Heal / Thorns / First HP Loss Relics

### Blood Vial

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/BloodVial.java`

Rust source:
- `src/content/relics/blood_vial.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Blood Vial"`, tier `COMMON`, landing sound `CLINK`.
- `atBattleStart`: flashes, calls `addToTop` for a UI-only relic action, then
  calls `addToTop(new HealAction(player, player, 2, 0.0f))`.

Rust result:
- Tier and battle-start subscription match Java.
- Emits player heal `2` with top insertion. The normal heal path still applies
  combat healing modifiers such as Magic Flower.
- UI-only relic flash / above-creature visual action is intentionally not
  represented.

Coverage:
- `shared_common_hp_and_thorns_relic_metadata_matches_java_sources`
- `blood_vial_and_bronze_scales_battle_start_actions_match_java_sources`

### Bronze Scales

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/BronzeScales.java`

Rust source:
- `src/content/relics/bronze_scales.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Bronze Scales"`, tier `COMMON`, landing sound `CLINK`.
- `atBattleStart`: flashes and calls `addToTop` with Thorns `3` on the player.

Rust result:
- Tier and battle-start subscription match Java.
- Emits player Thorns `3` with top insertion.
- UI-only relic flash is intentionally not represented.

Coverage:
- `shared_common_hp_and_thorns_relic_metadata_matches_java_sources`
- `blood_vial_and_bronze_scales_battle_start_actions_match_java_sources`

### Centennial Puzzle

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/CentennialPuzzle.java`

Rust source:
- `src/content/relics/centennial_puzzle.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Centennial Puzzle"`, tier `COMMON`, landing sound `HEAVY`.
- Static `usedThisCombat` is reset to false in `atPreBattle`.
- `wasHPLost(damageAmount)`: during combat, if `damageAmount > 0` and the
  relic has not triggered this combat, calls `addToTop(new DrawCardAction(player,
  3))`, then sets `usedThisCombat = true` immediately inside the hook method.
- `justEnteredRoom`, pulse, grayscale, and relic-above-creature behavior are
  UI-only.

Rust result:
- Tier, pre-battle subscription, and HP-loss subscription match Java.
- Fixed the hook to mutate `RelicState.used_up` immediately when the first
  positive HP loss triggers it, rather than queuing a later state-update action
  after the draw action.
- Pre-battle reset clears `used_up`.
- UI-only relic flash / pulse / grayscale / above-creature visual action is
  intentionally not represented.

Coverage:
- `shared_common_hp_and_thorns_relic_metadata_matches_java_sources`
- `centennial_puzzle_marks_used_immediately_and_resets_pre_battle`
- `centennial_puzzle_hook_updates_relic_state_before_draw_action_executes`

## Shared Relic Batch 3 - Common Counter / Energy Relics

### Happy Flower

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/HappyFlower.java`

Rust source:
- `src/content/relics/happy_flower.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Happy Flower"`, tier `COMMON`, landing sound `SOLID`.
- `onEquip` initializes `counter = 0`.
- `atTurnStart`: mutates `counter` immediately; `-1` becomes `1`, otherwise it
  increments by one. When the counter reaches `3`, it resets to `0` immediately
  and queues energy `1` with `addToBot`.

Rust result:
- Tier and turn-start subscription match Java.
- Fixed the hook to mutate `RelicState.counter` immediately instead of queuing
  a later counter-update action.
- Emits energy `1` with bottom insertion when the counter reaches `3`.
- UI-only relic flash / above-creature visual action is intentionally not
  represented.

Coverage:
- `shared_common_counter_relic_metadata_matches_java_sources`
- `happy_flower_counter_updates_immediately_like_java`

### Lantern

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/Lantern.java`

Rust source:
- `src/content/relics/lantern.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Lantern"`, tier `COMMON`, landing sound `SOLID`.
- `atPreBattle`: sets `firstTurn = true` immediately.
- `atTurnStart`: if `firstTurn`, calls `addToTop(new GainEnergyAction(1))` and
  then sets `firstTurn = false` immediately.

Rust result:
- Tier, pre-battle subscription, and turn-start subscription match Java.
- Fixed pre-battle and first-turn state to mutate `RelicState.used_up`
  immediately rather than queuing update actions.
- Fixed first-turn energy insertion to top insertion.
- UI-only relic flash / above-creature visual action is intentionally not
  represented.

Coverage:
- `shared_common_counter_relic_metadata_matches_java_sources`
- `lantern_first_turn_state_updates_immediately_like_java`

### Nunchaku

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/Nunchaku.java`

Rust source:
- `src/content/relics/nunchaku.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Nunchaku"`, tier `COMMON`, landing sound `FLAT`,
  initializes `counter = 0`.
- `onUseCard`: only for Attack cards, increments `counter` immediately. When
  `counter % 10 == 0`, resets `counter` to `0` immediately and queues energy
  `1` with `addToBot`.

Rust result:
- Tier and use-card subscription match Java.
- Fixed the hook to mutate `RelicState.counter` immediately instead of queuing
  a later counter-update action.
- The dispatcher still gates the hook to Attack cards.
- UI-only relic flash / above-creature visual action is intentionally not
  represented.

Coverage:
- `shared_common_counter_relic_metadata_matches_java_sources`
- `nunchaku_counter_updates_immediately_like_java`

### Pen Nib

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/PenNib.java`

Rust source:
- `src/content/relics/pen_nib.rs`
- `src/content/relics/hooks.rs`
- `src/content/powers/core/pen_nib.rs`

Java evidence:
- Constructor: ID `"Pen Nib"`, tier `COMMON`, landing sound `CLINK`,
  initializes `counter = 0`.
- `onUseCard`: only for Attack cards, increments `counter` immediately.
- When `counter == 9`, queues `PenNibPower(1)` with `addToBot`; this prepares
  the next attack. When `counter == 10`, resets `counter` to `0` immediately.
- `atBattleStart`: if `counter == 9`, queues `PenNibPower(1)` with `addToBot`.

Rust result:
- Tier, battle-start subscription, and use-card subscription match Java.
- Fixed the hook to mutate `RelicState.counter` immediately instead of queuing
  later counter-update actions.
- Existing `PenNibPower` doubles attack damage and removes itself when an Attack
  card is used.
- UI-only pulse / hand layout / relic-above-creature behavior is intentionally
  not represented.

Coverage:
- `shared_common_counter_relic_metadata_matches_java_sources`
- `pen_nib_counter_and_power_timing_match_java`

## Shared Relic Batch 4 - Common Turn-State Relics

### Ancient Tea Set

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/AncientTeaSet.java`

Rust source:
- `src/content/relics/ancient_tea_set.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Ancient Tea Set"`, tier `COMMON`, landing sound `SOLID`.
- `onEnterRestRoom`: sets `counter = -2` and starts pulse UI.
- `atPreBattle`: sets `firstTurn = true`.
- `atTurnStart`: only on first turn, if `counter == -2`, sets `counter = -1`
  immediately and queues energy `2` with `addToTop`.
- `canSpawn` is false after floor 48 unless Endless mode is active.

Rust result:
- Tier, pre-battle, turn-start, and rest-room subscriptions match Java.
- Fixed first-turn state to mutate `RelicState.used_up` immediately.
- Fixed `counter` mutation from delayed queued action to immediate mutation and
  fixed energy insertion from bottom to top.
- Spawn gating is a relic-pool/reward-generation concern and is not handled by
  the combat hook.
- UI-only pulse / relic-above-creature visual action is intentionally not
  represented.

Coverage:
- `shared_common_turn_state_relic_metadata_matches_java_sources`
- `ancient_tea_set_first_turn_state_matches_java`

### Art of War

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/ArtOfWar.java`

Rust source:
- `src/content/relics/art_of_war.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Art of War"`, tier `COMMON`, landing sound `FLAT`.
- `atPreBattle`: sets `firstTurn = true` and `gainEnergyNext = true`.
- `atTurnStart`: if `gainEnergyNext` and not first turn, queues energy `1` with
  `addToBot`; then sets `firstTurn = false` and `gainEnergyNext = true`.
- `onUseCard`: if the card is an Attack, sets `gainEnergyNext = false`
  immediately.

Rust result:
- Tier, pre-battle, turn-start, and use-card subscriptions match Java.
- Fixed the hook to mutate `RelicState.counter` immediately instead of queuing
  later counter-update actions.
- Counter encoding: `-1` means the first turn skip, `1` means gain energy next
  turn, and `0` means an Attack was played this turn.
- UI-only pulse / relic-above-creature visual action is intentionally not
  represented.

Coverage:
- `shared_common_turn_state_relic_metadata_matches_java_sources`
- `art_of_war_turn_and_attack_state_matches_java`

### Orichalcum

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/Orichalcum.java`

Rust source:
- `src/content/relics/orichalcum.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Orichalcum"`, tier `COMMON`, landing sound `HEAVY`.
- `onPlayerEndTurn`: if player block is `0` or public field `trigger` is true,
  clears `trigger` and queues block `6` with `addToTop`.
- No decompiled game source in `D:/rust/cardcrawl` writes `trigger = true`.
- `onPlayerGainedBlock`, pulse, and victory behavior are UI/presentation state
  except for the final floored block amount that the normal block pipeline
  already uses.

Rust result:
- Tier and end-of-turn subscription match Java's effective game path.
- Emits player block `6` with top insertion when current block is `0`.
- Public `trigger` side path is not modeled because the Java source tree has no
  gameplay writer for it.
- UI-only pulse / relic-above-creature visual action is intentionally not
  represented.

Coverage:
- `shared_common_turn_state_relic_metadata_matches_java_sources`
- `orichalcum_and_smooth_stone_actions_match_java_sources`

### Oddly Smooth Stone

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/OddlySmoothStone.java`

Rust source:
- `src/content/relics/oddly_smooth_stone.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Oddly Smooth Stone"`, tier `COMMON`, landing sound `SOLID`.
- `atBattleStart`: queues Dexterity `1` on the player with `addToTop`.

Rust result:
- Tier and battle-start subscription match Java.
- Emits player Dexterity `1` with top insertion.
- UI-only relic flash / above-creature visual action is intentionally not
  represented.

Coverage:
- `shared_common_turn_state_relic_metadata_matches_java_sources`
- `orichalcum_and_smooth_stone_actions_match_java_sources`

## Shared Relic Batch 5 - Common Damage / HP Relics

### Boot

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/Boot.java`

Rust source:
- `src/content/relics/boot.rs`
- `src/engine/action_handlers/damage.rs`

Java evidence:
- Constructor: ID `"Boot"`, tier `COMMON`, landing sound `HEAVY`.
- `onAttackToChangeDamage`: if `info.owner != null`, damage type is not
  `HP_LOSS` or `THORNS`, and final damage is between `1` and `4`, returns `5`.
- The relic-above-creature action is UI-only.

Rust result:
- Tier matches Java. Boot is handled natively in the damage pipeline rather than
  through a subscription bus.
- Player-origin normal damage after block is raised to `5` when it would deal
  `1..4`.
- THORNS / no-source damage is not raised.
- UI-only relic flash / above-creature visual action is intentionally not
  represented.

Coverage:
- `shared_common_damage_hp_relic_metadata_matches_java_sources`
- `boot_damage_floor_applies_only_to_positive_normal_player_damage`

### Preserved Insect

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/PreservedInsect.java`

Rust source:
- `src/content/relics/preserved_insect.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"PreservedInsect"`, tier `COMMON`, landing sound `FLAT`.
- `atBattleStart`: only if the current room's `eliteTrigger` is true, loops all
  current monsters and sets `currentHealth` to `floor(maxHealth * 0.75)` only
  when the monster is above that threshold.
- The relic does not reduce max HP.
- `canSpawn` is false after floor 52 unless Endless mode is active.

Rust result:
- Tier and battle-start subscription match Java.
- Fixed Rust to use `CombatMeta.is_elite_fight` rather than guessing from
  monster IDs.
- Fixed Rust to mutate monster current HP immediately and leave max HP
  unchanged, instead of queuing max-HP loss actions.
- Spawn gating is a relic-pool/reward-generation concern and is not handled by
  the combat hook.
- UI-only relic flash / above-creature visual action is intentionally not
  represented.

Coverage:
- `shared_common_damage_hp_relic_metadata_matches_java_sources`
- `preserved_insect_uses_elite_room_flag_and_reduces_current_hp_only`

### Vajra

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/Vajra.java`

Rust source:
- `src/content/relics/vajra.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Vajra"`, tier `COMMON`, landing sound `CLINK`.
- `atBattleStart`: queues Strength `1` on the player with `addToTop`.

Rust result:
- Tier and battle-start subscription match Java.
- Fixed insertion from bottom to top.
- UI-only relic flash / above-creature visual action is intentionally not
  represented.

Coverage:
- `shared_common_damage_hp_relic_metadata_matches_java_sources`
- `vajra_and_strawberry_match_java_sources`

### Strawberry

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/Strawberry.java`

Rust source:
- `src/content/relics/strawberry.rs`
- `src/engine/relic_manager.rs`

Java evidence:
- Constructor: ID `"Strawberry"`, tier `COMMON`, landing sound `FLAT`.
- `onEquip`: calls `increaseMaxHp(7, true)`.

Rust result:
- Tier matches Java.
- Run-level on-equip increases max HP by `7` and heals current HP by `7`, capped
  at max HP.

Coverage:
- `shared_common_damage_hp_relic_metadata_matches_java_sources`
- `vajra_and_strawberry_match_java_sources`

## Shared Common Run Gold / Event Batch

### Cross-Cutting Gold Entry Normalization

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/characters/AbstractPlayer.java`
- `D:/rust/cardcrawl/relics/Ectoplasm.java`

Rust source:
- `src/state/run.rs`
- `src/engine/run_loop.rs`
- `src/engine/action_handlers/damage.rs`
- `src/content/events/*.rs`

Java evidence:
- `AbstractPlayer.gainGold(int)` returns without changing gold when the player
  has `Ectoplasm`.
- `AbstractPlayer.loseGold(int)` calls `AbstractRelic.onSpendGold()` only when
  the current room is a `ShopRoom`; event gold loss does not use up `MawBank`.

Rust result:
- Fixed `RunState::change_gold_with_source` to block positive gold gains while
  `Ectoplasm` is present.
- Fixed combat `GainGold` action handling to respect `Ectoplasm`.
- Routed event gold gains/losses through `change_gold_with_source` instead of
  direct `run_state.gold +=/-=`, preserving Ectoplasm and domain-event
  semantics.
- Kept shop-only `MawBank` spend exhaustion, matching Java `loseGold`.
- Relic `on_equip` paths that are already wrapped by
  `obtain_relic_with_source` resource-diff emission were guarded for Ectoplasm
  without switching to `change_gold_with_source`, avoiding duplicate resource
  events.

Coverage:
- `content::events`
- `content::relics::tests`
- `engine::shop_handler::tests`
- `ectoplasm_blocks_run_combat_and_on_equip_gold_gain_paths`

### Ceramic Fish

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/CeramicFish.java`

Rust source:
- `src/deck/manager.rs`
- `src/state/run.rs`
- `src/content/relics/ceramic_fish.rs`

Java evidence:
- Constructor: ID `"CeramicFish"`, tier `COMMON`, landing sound `FLAT`.
- `onObtainCard(AbstractCard c)` calls `AbstractDungeon.player.gainGold(9)`.
- `canSpawn` is false after floor 48 unless Endless mode is active.
- Because this routes through `gainGold`, `Ectoplasm` blocks the gold.

Rust result:
- Tier matches Java.
- Existing deck obtain pipeline grants `+9` gold only after Omamori has failed
  to prevent the obtain, so blocked curses do not trigger Ceramic Fish.
- Fixed the shared gold entry path so Ceramic Fish gold is blocked by
  `Ectoplasm`, matching Java.
- Spawn gating is a relic-pool/reward-generation concern and is not handled by
  the deck hook.

Coverage:
- `shared_common_run_gold_relic_metadata_matches_java_sources`
- `ceramic_fish_obtain_card_gold_uses_java_gain_gold_semantics`

### Dream Catcher

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/DreamCatcher.java`
- `D:/rust/cardcrawl/vfx/campfire/CampfireSleepEffect.java`

Rust source:
- `src/engine/campfire_handler.rs`
- `src/content/relics/dream_catcher.rs`

Java evidence:
- Constructor: ID `"Dream Catcher"`, tier `COMMON`, landing sound `MAGICAL`.
- The relic class itself has no hook method; after sleep resolves,
  `CampfireSleepEffect` checks for Dream Catcher and opens a normal card reward
  if reward cards are available.
- `canSpawn` is false after floor 48 unless Endless mode is active.

Rust result:
- Tier matches Java.
- Campfire rest path generates a card reward after resting and preserves
  `QuestionCard` reward-size adjustment.
- Spawn gating is a relic-pool/reward-generation concern and is not handled by
  the campfire hook.

Coverage:
- `shared_common_run_gold_relic_metadata_matches_java_sources`
- `dream_catcher_reward_respects_question_card`

### Juzu Bracelet

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/JuzuBracelet.java`
- `D:/rust/cardcrawl/helpers/EventHelper.java`

Rust source:
- `src/state/run.rs`
- `src/events/generator.rs`
- `src/content/relics/mod.rs`

Java evidence:
- Constructor: ID `"Juzu Bracelet"`, tier `COMMON`, landing sound `MAGICAL`.
- The relic class itself has no hook method.
- `EventHelper.roll` converts a rolled `MONSTER` result into `EVENT` when the
  player has Juzu Bracelet, but still resets `MONSTER_CHANCE` to `0.1`.
- `canSpawn` is false after floor 48 unless Endless mode is active.

Rust result:
- Tier matches Java.
- `RunState::generate_event` passes `has_juzu_bracelet` into the event roll
  context.
- `EventGenerator::roll_room_type` converts monster rolls to event and still
  resets monster chance to `0.10`.
- Spawn gating is a relic-pool/reward-generation concern and is not handled by
  the event hook.

Coverage:
- `shared_common_run_gold_relic_metadata_matches_java_sources`
- `juzu_bracelet_converts_monster_event_roll_without_preserving_monster_chance`

### Maw Bank

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/MawBank.java`
- `D:/rust/cardcrawl/characters/AbstractPlayer.java`

Rust source:
- `src/engine/run_loop.rs`
- `src/state/run.rs`
- `src/engine/shop_handler.rs`

Java evidence:
- Constructor: ID `"MawBank"`, tier `COMMON`, landing sound `CLINK`.
- `onEnterRoom`: if not used up, calls `AbstractDungeon.player.gainGold(12)`.
- `onSpendGold`: if not used up, calls `setCounter(-2)`, which marks the relic
  used up and leaves counter at `-2`.
- `AbstractPlayer.loseGold` only calls `onSpendGold` in `ShopRoom`.
- `canSpawn` is false after floor 48 unless Endless mode is active, and false
  in shop rooms.

Rust result:
- Tier matches Java.
- Existing shop purchase/removal path uses up Maw Bank through
  `change_gold_with_source(..., Shop)`, matching Java.
- Fixed room-entry Maw Bank gold to use `change_gold_with_source` instead of
  direct `gold += 12`, preserving `Ectoplasm` and gold-domain events.
- Confirmed event gold loss does not use up Maw Bank, matching Java's
  shop-room guard.
- Spawn gating is a relic-pool/reward-generation concern and is not handled by
  the room-entry hook.

Coverage:
- `shared_common_run_gold_relic_metadata_matches_java_sources`
- `maw_bank_only_spending_in_shop_uses_it_up_like_java_lose_gold`
- `engine::shop_handler::tests`

## Shared Common Shop / Rest / Event Batch

### Cross-Cutting Room-Entry Heal Source Normalization

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/core/AbstractCreature.java`
- `D:/rust/cardcrawl/relics/MarkOfTheBloom.java`
- `D:/rust/cardcrawl/relics/MagicFlower.java`

Rust source:
- `src/engine/run_loop.rs`

Java evidence:
- Player healing routes through `AbstractCreature.heal`.
- `MarkOfTheBloom.onPlayerHeal` always returns `0`.
- `MagicFlower.onPlayerHeal` only modifies healing while the current room phase
  is `COMBAT`, so shop/rest-room entry healing is not multiplied.

Rust result:
- Kept the existing Mark of the Bloom guard for out-of-combat room-entry heals.
- Routed room-entry `MealTicket` and `EternalFeather` heals through
  `change_hp_with_source` after the Mark guard, preserving resource-domain
  events without incorrectly applying combat-only Magic Flower.

Coverage:
- `engine::run_loop::tests`

### Meal Ticket

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/MealTicket.java`

Rust source:
- `src/engine/run_loop.rs`

Java evidence:
- Constructor: ID `"MealTicket"`, tier `COMMON`, landing sound `CLINK`.
- `justEnteredRoom(AbstractRoom room)`: when the room is a `ShopRoom`, queues
  visual relic action and calls `AbstractDungeon.player.heal(15)`.
- `canSpawn` is false after floor 48 unless Endless mode is active.

Rust result:
- Tier matches Java.
- Shop-room entry heals `15` HP and is blocked by `MarkOfTheBloom`.
- Fixed the heal to route through `change_hp_with_source` with relic source
  instead of directly mutating HP.
- UI-only flash / above-creature action is intentionally not represented.
- Spawn gating is a relic-pool/reward-generation concern and is not handled by
  the room-entry hook.

Coverage:
- `shared_common_shop_rest_event_relic_metadata_matches_java_sources`
- `meal_ticket_shop_entry_heal_uses_relic_source_and_mark_of_bloom_guard`

### Regal Pillow

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/RegalPillow.java`
- `D:/rust/cardcrawl/vfx/campfire/CampfireSleepEffect.java`
- `D:/rust/cardcrawl/ui/campfire/RestOption.java`

Rust source:
- `src/engine/campfire_handler.rs`

Java evidence:
- Constructor: ID `"Regal Pillow"`, tier `COMMON`, landing sound `MAGICAL`.
- The relic class has no hook method; sleep/rest code adds flat `15` to the
  normal campfire rest heal.
- The resulting heal still routes through `player.heal`, so Mark of the Bloom
  blocks it.
- `canSpawn` is false after floor 48 unless Endless mode is active.

Rust result:
- Tier matches Java.
- Campfire rest adds flat `15` before applying the existing Mark of the Bloom
  block.
- Spawn gating is a relic-pool/reward-generation concern and is not handled by
  the campfire hook.

Coverage:
- `shared_common_shop_rest_event_relic_metadata_matches_java_sources`
- `regal_pillow_adds_to_rest_heal_but_mark_of_bloom_blocks_it`

### Smiling Mask

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/SmilingMask.java`
- `D:/rust/cardcrawl/shop/ShopScreen.java`
- `D:/rust/cardcrawl/shop/StoreRelic.java`

Rust source:
- `src/shop/shop_screen.rs`
- `src/engine/shop_handler.rs`

Java evidence:
- Constructor: ID `"Smiling Mask"`, tier `COMMON`, landing sound `FLAT`.
- `onEnterRoom`: pulses only in shop rooms; this is UI-only.
- Shop init and purge updates force `actualPurgeCost = 50` when owned.
- Buying Smiling Mask in a shop immediately sets current shop
  `actualPurgeCost` to `50`.
- `canSpawn` is false after floor 48 unless Endless mode is active, and false
  in shop rooms.

Rust result:
- Tier matches Java.
- Shop generation forces purge cost to `50` when owned, after other discounts.
- Buying Smiling Mask in the current shop immediately sets purge cost to `50`.
- UI-only pulse/stopPulse is intentionally not represented.
- Spawn gating is a relic-pool/reward-generation concern and is not handled by
  the shop hook.

Coverage:
- `shared_common_shop_rest_event_relic_metadata_matches_java_sources`
- `smiling_mask_overrides_discounted_initial_purge_cost`
- `smiling_mask_purchase_sets_purge_cost_to_50`

### Tiny Chest

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/TinyChest.java`
- `D:/rust/cardcrawl/helpers/EventHelper.java`

Rust source:
- `src/state/run.rs`
- `src/events/generator.rs`

Java evidence:
- Constructor: ID `"Tiny Chest"`, tier `COMMON`, landing sound `SOLID`;
  constructor sets counter `-1`, and `onEquip` sets counter `0`.
- On every unknown-room roll, EventHelper increments the counter. When it
  reaches `4`, the counter resets to `0`, the relic flashes, and the roll is
  forced to `TREASURE`.
- The random roll is still consumed before the forced treasure result.
- `canSpawn` is false after floor 35 unless Endless mode is active.

Rust result:
- Tier and initial equipped counter match Java's `onEquip` state.
- `RunState::generate_event` increments and resets Tiny Chest before room-type
  rolling.
- `EventGenerator::roll_room_type` consumes the event RNG and then forces a
  treasure result when `tiny_chest_counter == 3`.
- UI-only flash is intentionally not represented.
- Spawn gating is a relic-pool/reward-generation concern and is not handled by
  the event hook.

Coverage:
- `shared_common_shop_rest_event_relic_metadata_matches_java_sources`
- `tiny_chest_counter_forces_treasure_roll_every_fourth_unknown_room`

## Shared Common Obtain / Potion / Upgrade Batch

### Omamori

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/Omamori.java`
- `D:/rust/cardcrawl/vfx/FastCardObtainEffect.java`
- `D:/rust/cardcrawl/vfx/cardManip/ShowCardAndObtainEffect.java`

Rust source:
- `src/deck/manager.rs`
- `src/state/run.rs`
- `src/content/relics/omamori.rs`

Java evidence:
- Constructor: ID `"Omamori"`, tier `COMMON`, landing sound `FLAT`, counter
  starts at `2`.
- Curse obtain effects check for Omamori with nonzero counter, call `use()`,
  decrement counter, and prevent the curse from being obtained.
- `setCounter(0)` marks the relic used up.
- `canSpawn` is false after floor 48 unless Endless mode is active.

Rust result:
- Tier and initial counter match Java.
- Deck obtain pipeline blocks curses while Omamori counter is positive,
  decrements the counter, and marks the relic used up at `0`.
- Removed an unused placeholder function from `omamori.rs`; the real behavior is
  in the deck obtain pipeline.
- Spawn gating is a relic-pool/reward-generation concern and is not handled by
  the deck hook.

Coverage:
- `shared_common_obtain_potion_upgrade_relic_metadata_matches_java_sources`
- `omamori_blocks_exactly_two_curse_obtains_then_marks_used_up`

### Potion Belt

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/PotionBelt.java`

Rust source:
- `src/content/relics/potion_belt.rs`
- `src/engine/relic_manager.rs`

Java evidence:
- Constructor: ID `"Potion Belt"`, tier `COMMON`, landing sound `FLAT`.
- `onEquip`: increments potion slots by `2` and appends two empty `PotionSlot`
  objects at the end.
- `canSpawn` is false after floor 48 unless Endless mode is active.

Rust result:
- Tier matches Java.
- On-equip appends two empty potion slots without reordering existing potions.
- Spawn gating is a relic-pool/reward-generation concern and is not handled by
  the on-equip hook.

Coverage:
- `shared_common_obtain_potion_upgrade_relic_metadata_matches_java_sources`
- `potion_belt_appends_two_empty_slots_without_reordering_existing_potions`

### Toy Ornithopter

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/ToyOrnithopter.java`
- `D:/rust/cardcrawl/ui/panels/PotionPopUp.java`

Rust source:
- `src/content/relics/toy_ornithopter.rs`
- `src/content/relics/hooks.rs`
- `src/engine/action_handlers/cards.rs`

Java evidence:
- Constructor: ID `"Toy Ornithopter"`, tier `COMMON`, landing sound `FLAT`.
- `PotionPopUp` calls `r.onUsePotion()` after a potion is used.
- In combat, Toy Ornithopter queues `RelicAboveCreatureAction` and
  `HealAction(player, player, 5)` at the bottom.
- Outside combat, it directly calls `AbstractDungeon.player.heal(5)`.

Rust result:
- Tier and combat `on_use_potion` subscription match Java.
- Combat potion use queues a bottom `Heal { amount: 5 }`, so combat healing
  still flows through the normal heal handler, including Magic Flower and Mark
  of the Bloom.
- Run-level potion use now covers the Java `PotionPopUp` non-combat path for
  Blood Potion, Fruit Juice, and Entropic Brew. Toy Ornithopter heals directly
  through the run-state HP path after the potion effect resolves, matching the
  Java non-combat `player.heal(5)` branch.
- Entropic Brew keeps the Java split: in combat it uses
  `returnRandomPotion(true)`, while outside combat it uses
  `returnRandomPotion()` unless Sozu blocks generation. The used potion is then
  removed and the queued/generated obtains fill open slots.
- UI-only above-creature action is intentionally not represented.

Coverage:
- `shared_common_obtain_potion_upgrade_relic_metadata_matches_java_sources`
- `toy_ornithopter_queues_bottom_heal_when_potion_is_used`
- `run_level_blood_potion_uses_sacred_bark_toy_ornithopter_and_consumes_slot`
- `run_level_entropic_brew_consumes_slot_and_refills_without_limited_filter`
- `run_level_entropic_brew_with_sozu_consumes_without_generating_potions`

### War Paint

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/WarPaint.java`

Rust source:
- `src/content/relics/war_paint.rs`
- `src/engine/relic_manager.rs`

Java evidence:
- Constructor: ID `"War Paint"`, tier `COMMON`, landing sound `SOLID`.
- `onEquip`: collects upgradable SKILL cards, shuffles with
  `new Random(AbstractDungeon.miscRng.randomLong())`, and upgrades up to two.
- Calls `bottledCardUpgradeCheck` for upgraded cards; this is only relevant once
  bottled-card display/state metadata is fully modeled.

Rust result:
- Tier matches Java.
- Random selection uses `misc_rng.randomLong()` shuffle and upgrades up to two
  matching skill cards.
- Fixed direct `upgrades += 1` mutation to call `upgrade_card_with_source` with
  `DomainEventSource::Relic(WarPaint)`, preserving card-upgrade trace data.
- UI-only card show / shine effects are intentionally not represented.

Coverage:
- `shared_common_obtain_potion_upgrade_relic_metadata_matches_java_sources`
- `war_paint_and_whetstone_upgrade_only_matching_card_types_with_relic_source`

### Whetstone

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/Whetstone.java`

Rust source:
- `src/content/relics/whetstone.rs`
- `src/engine/relic_manager.rs`

Java evidence:
- Constructor: ID `"Whetstone"`, tier `COMMON`, landing sound `SOLID`.
- `onEquip`: collects upgradable ATTACK cards, shuffles with
  `new Random(AbstractDungeon.miscRng.randomLong())`, and upgrades up to two.
- Calls `bottledCardUpgradeCheck` for upgraded cards; this is only relevant once
  bottled-card display/state metadata is fully modeled.

Rust result:
- Tier matches Java.
- Random selection uses `misc_rng.randomLong()` shuffle and upgrades up to two
  matching attack cards, including repeat-upgradable `SearingBlow`.
- Fixed direct `upgrades += 1` mutation to call `upgrade_card_with_source` with
  `DomainEventSource::Relic(Whetstone)`, preserving card-upgrade trace data.
- UI-only card show / shine effects are intentionally not represented.

Coverage:
- `shared_common_obtain_potion_upgrade_relic_metadata_matches_java_sources`
- `war_paint_and_whetstone_upgrade_only_matching_card_types_with_relic_source`

### Darkstone Periapt

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/DarkstonePeriapt.java`

Rust source:
- `src/deck/manager.rs`
- `src/state/run.rs`

Java evidence:
- Constructor: ID `"Darkstone Periapt"`, tier `UNCOMMON`, landing sound
  `MAGICAL`.
- `onObtainCard(AbstractCard card)` grants `+6` max HP only when the obtained
  card color is `CURSE`.
- If Omamori prevents the curse from being obtained, this hook does not fire.
- `canSpawn` is false after floor 48 unless Endless mode is active.

Rust result:
- Tier matches Java.
- The deck obtain pipeline now preserves the original obtain source when
  resolving Darkstone's max-HP side effect, instead of collapsing it to a
  generic deck mutation.
- Omamori interception runs before Darkstone, so blocked curses do not grant
  max HP.
- Spawn gating is a relic-pool/reward-generation concern and is not handled by
  the deck hook.

Coverage:
- `shared_uncommon_card_reward_relic_metadata_matches_java_sources`
- `darkstone_periapt_triggers_only_after_curse_is_obtained`

### Molten Egg

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/MoltenEgg2.java`
- `D:/rust/cardcrawl/dungeons/AbstractDungeon.java`
- `D:/rust/cardcrawl/shop/ShopScreen.java`
- `D:/rust/cardcrawl/shop/StoreRelic.java`

Rust source:
- `src/deck/manager.rs`
- `src/rewards/generator.rs`
- `src/shop/shop_screen.rs`
- `src/engine/shop_handler.rs`

Java evidence:
- Constructor: ID `"Molten Egg 2"`, tier `UNCOMMON`, landing sound `SOLID`.
- `onObtainCard(AbstractCard c)` upgrades ATTACK cards only when
  `c.canUpgrade() && !c.upgraded`.
- `onPreviewObtainCard(c)` calls the same upgrade path for visible reward/shop
  cards.
- `AbstractDungeon.getRewardCards()` skips relic preview for cards already
  upgraded by `cardUpgradedChance`.
- Buying an Egg relic in a shop immediately calls `onPreviewObtainCard` on
  existing shop cards.

Rust result:
- Tier matches Java.
- Added one shared obtain-preview helper used by deck obtains, card rewards,
  generated reward rows, initial shop cards, Courier shop-card replacement, and
  visible shop cards after buying an Egg.
- Fixed the previous Searing Blow over-upgrade: a pre-upgraded card remains at
  its current upgrade count because Java checks `!c.upgraded`.
- `ShopCard` now carries `upgrades`, and buying a shop card preserves the
  visible preview upgrade into `master_deck`.

Coverage:
- `shared_uncommon_card_reward_relic_metadata_matches_java_sources`
- `egg_relics_preview_obtain_upgrades_without_double_upgrading_existing_plus_cards`
- `initial_shop_card_previews_apply_existing_egg_relics`
- `shop_card_purchase_preserves_preview_upgrades`
- `buying_egg_relic_previews_existing_shop_cards_like_java_store_relic`
- `relic_id_from_java_maps_java_egg2_ids_to_rust_egg_ids`

### Toxic Egg

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/ToxicEgg2.java`

Rust source:
- `src/deck/manager.rs`
- `src/rewards/generator.rs`
- `src/shop/shop_screen.rs`
- `src/engine/shop_handler.rs`

Java evidence:
- Constructor: ID `"Toxic Egg 2"`, tier `UNCOMMON`, landing sound `SOLID`.
- Uses the same obtain/preview structure as Molten Egg, but targets SKILL cards.
- `canSpawn` is false after floor 48 unless Endless mode is active.

Rust result:
- Shares the corrected Egg preview pipeline with Molten Egg.
- Applies to unupgraded SKILL cards across real obtain, reward preview, shop
  preview, and shop purchase preservation.

Coverage:
- `shared_uncommon_card_reward_relic_metadata_matches_java_sources`
- `egg_relics_preview_obtain_upgrades_without_double_upgrading_existing_plus_cards`
- `initial_shop_card_previews_apply_existing_egg_relics`
- `relic_id_from_java_maps_java_egg2_ids_to_rust_egg_ids`

### Frozen Egg

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/FrozenEgg2.java`

Rust source:
- `src/deck/manager.rs`
- `src/rewards/generator.rs`
- `src/shop/shop_screen.rs`
- `src/engine/shop_handler.rs`

Java evidence:
- Constructor: ID `"Frozen Egg 2"`, tier `UNCOMMON`, landing sound `CLINK`.
- Uses the same obtain/preview structure as Molten Egg, but targets POWER cards.
- `canSpawn` is false after floor 48 unless Endless mode is active.

Rust result:
- Shares the corrected Egg preview pipeline with Molten Egg.
- Applies to unupgraded POWER cards across real obtain, reward preview, shop
  preview, and shop purchase preservation.

Coverage:
- `shared_uncommon_card_reward_relic_metadata_matches_java_sources`
- `egg_relics_preview_obtain_upgrades_without_double_upgrading_existing_plus_cards`
- `initial_shop_card_previews_apply_existing_egg_relics`
- `relic_id_from_java_maps_java_egg2_ids_to_rust_egg_ids`

### Question Card

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/QuestionCard.java`
- `D:/rust/cardcrawl/dungeons/AbstractDungeon.java`

Rust source:
- `src/rewards/generator.rs`

Java evidence:
- Constructor: ID `"Question Card"`, tier `UNCOMMON`, landing sound `FLAT`.
- `changeNumberOfCardsInReward(int n)` returns `n + 1`.
- `AbstractDungeon.getRewardCards()` starts at 3 cards and lets relics adjust
  the count before rolling card rewards.
- `canSpawn` is false after floor 48 unless Endless mode is active.

Rust result:
- Tier matches Java.
- Card reward count adjustment adds one choice and composes with Busted Crown.
- Existing Dream Catcher and Orrery reward generation use the same adjusted
  reward count path.
- Spawn gating is a relic-pool/reward-generation concern and is not handled by
  the reward-count helper.

Coverage:
- `question_card_adds_one_choice`
- `question_card_and_busted_crown_still_net_one_choice`
- `dream_catcher_reward_respects_question_card`
- `orrery_card_rewards_respect_question_card`

### Gremlin Horn

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/GremlinHorn.java`

Rust source:
- `src/content/relics/gremlin_horn.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Gremlin Horn"`, tier `UNCOMMON`, landing sound `HEAVY`.
- `energyBased = true`.
- `onMonsterDeath(AbstractMonster m)` triggers only when
  `m.currentHealth == 0` and `!AbstractDungeon.getMonsters().areMonstersBasicallyDead()`.
- Queues, in order, `GainEnergyAction(1)` then `DrawCardAction(player, 1)`.

Rust result:
- Tier and `on_monster_death` subscription match Java.
- Fixed the hook to check that the target is actually dead and that another
  monster remains alive; the final kill no longer grants energy/draw.
- UI-only relic-above-creature action is intentionally not represented.

Coverage:
- `shared_uncommon_combat_trigger_relic_metadata_matches_java_sources`
- `gremlin_horn_triggers_only_when_another_monster_remains_alive`

### Letter Opener

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/LetterOpener.java`

Rust source:
- `src/content/relics/letter_opener.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Letter Opener"`, tier `UNCOMMON`, landing sound `CLINK`.
- `atTurnStart()` sets `counter = 0`.
- `onUseCard`: if the card is a SKILL, increments `counter`; on every third
  skill it resets counter to 0 and queues 5 THORNS damage to all enemies.
- `onVictory()` sets `counter = -1`.

Rust result:
- Tier and subscriptions now match Java, including the previously missing
  `at_turn_start` reset.
- The counter hook treats negative pre-turn values as zero, then updates the
  visible relic counter through the shared counter action.
- Damage uses all-enemy THORNS damage with 5 per monster slot.

Coverage:
- `shared_uncommon_combat_trigger_relic_metadata_matches_java_sources`
- `letter_opener_resets_each_turn_and_fires_on_third_skill`

### Kunai

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/Kunai.java`

Rust source:
- `src/content/relics/kunai.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Kunai"`, tier `UNCOMMON`, landing sound `CLINK`.
- `atTurnStart()` sets `counter = 0`.
- `onUseCard`: every third ATTACK resets counter to 0 and queues
  `DexterityPower(player, 1)`.
- `onVictory()` sets `counter = -1`.

Rust result:
- Tier and subscriptions match Java.
- Counter logic now treats negative values as zero before incrementing, avoiding
  an off-by-one if a turn-start reset has not yet materialized.
- Third attack queues `+1 Dexterity` and counter reset.

Coverage:
- `shared_uncommon_combat_trigger_relic_metadata_matches_java_sources`
- `attack_counter_relics_fire_on_third_attack_and_reset_on_victory`

### Shuriken

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/Shuriken.java`

Rust source:
- `src/content/relics/shuriken.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Shuriken"`, tier `UNCOMMON`, landing sound `CLINK`.
- `atTurnStart()` sets `counter = 0`.
- `onUseCard`: every third ATTACK resets counter to 0 and queues
  `StrengthPower(player, 1)`.
- `onVictory()` sets `counter = -1`.

Rust result:
- Tier and subscriptions match Java.
- Counter logic now treats negative values as zero before incrementing.
- Third attack queues `+1 Strength` and counter reset.

Coverage:
- `shared_uncommon_combat_trigger_relic_metadata_matches_java_sources`
- `attack_counter_relics_fire_on_third_attack_and_reset_on_victory`

### Ornamental Fan

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/OrnamentalFan.java`

Rust source:
- `src/content/relics/ornamental_fan.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Ornamental Fan"`, tier `UNCOMMON`, landing sound `FLAT`.
- `atTurnStart()` sets `counter = 0`.
- `onUseCard`: every third ATTACK resets counter to 0 and queues
  `GainBlockAction(player, player, 4)`.
- `onVictory()` sets `counter = -1`.

Rust result:
- Tier and subscriptions match Java.
- Existing implementation already normalizes negative counter values and queues
  counter update plus 4 block on the third attack.
- UI-only relic-above-creature action is intentionally not represented.

Coverage:
- `shared_uncommon_combat_trigger_relic_metadata_matches_java_sources`
- `attack_counter_relics_fire_on_third_attack_and_reset_on_victory`

### Mercury Hourglass

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/MercuryHourglass.java`

Rust source:
- `src/content/relics/mercury_hourglass.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Mercury Hourglass"`, tier `UNCOMMON`, landing sound
  `CLINK`.
- `atTurnStart()` queues 3 THORNS damage to all enemies.

Rust result:
- Tier and `at_turn_start` subscription match Java.
- Queues one all-enemy THORNS damage action with 3 damage per monster slot.
- UI-only relic-above-creature action is intentionally not represented.

Coverage:
- `shared_uncommon_combat_trigger_relic_metadata_matches_java_sources`
- `mercury_hourglass_queues_thorns_damage_to_all_monster_slots`

## Shared Relic Batch 5 - Uncommon Start / Victory / Reward Relics

### Horn Cleat

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/HornCleat.java`

Rust source:
- `src/content/relics/horn_cleat.rs`
- `src/content/relics/hooks.rs`
- `src/content/relics/mod.rs`

Java evidence:
- Constructor: ID `"HornCleat"`, tier `UNCOMMON`, landing sound `HEAVY`.
- `atBattleStart()` sets `counter = 0` immediately.
- `atTurnStart()`: if not grayscale, increments `counter`; when `counter == 2`,
  queues block `14` with `addToBot`, then sets `counter = -1` and
  `grayscale = true`.
- `onVictory()` sets `counter = -1` and clears grayscale.

Rust result:
- Tier and subscriptions now match Java, including the previously missing
  victory reset.
- Fixed the relic to mutate its combat counter immediately rather than queuing
  a later counter-update action.
- Fixed the repeated-trigger bug: after the second-turn block, the counter is
  set to `-1` and later turn starts in the same combat do nothing.
- Rust does not model grayscale; the gameplay gate is represented by
  `counter = -1`.

Coverage:
- `shared_uncommon_start_victory_reward_relic_metadata_matches_java_sources`
- `horn_cleat_triggers_only_on_second_turn_then_disables_until_next_combat`

### Pantograph

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/Pantograph.java`

Rust source:
- `src/content/relics/pantograph.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Pantograph"`, tier `UNCOMMON`, landing sound `CLINK`.
- `atBattleStart()` scans the current monsters and triggers if any monster has
  `EnemyType.BOSS`.
- Queues `HealAction(player, player, 25, 0.0f)` with `addToTop`; the relic
  visual action is UI-only.

Rust result:
- Tier and battle-start subscription match Java.
- Boss combat detection is currently derived from known boss enemy IDs.
- Fixed heal insertion from bottom to top insertion to preserve Java action
  ordering.
- UI-only relic-above-creature action is intentionally not represented.

Coverage:
- `shared_uncommon_start_victory_reward_relic_metadata_matches_java_sources`
- `pantograph_heals_only_in_boss_combat_with_java_top_insertion`

### Meat on the Bone

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/MeatOnTheBone.java`
- `D:/rust/cardcrawl/rooms/AbstractRoom.java`

Rust source:
- `src/content/relics/meat_on_the_bone.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Meat on the Bone"`, tier `UNCOMMON`, landing sound
  `HEAVY`.
- `AbstractRoom.endBattle()` calls `onTrigger()` if the player has the relic.
- `onTrigger()`: if `currentHealth <= maxHealth / 2.0f` and
  `currentHealth > 0`, directly heals `12`.
- `onBloodied`, `onNotBloodied`, pulse, flash, and relic visual actions are
  UI-only.
- `canSpawn` is false after floor 48 unless Endless mode is active.

Rust result:
- Tier and victory subscription match Java.
- Fixed the hook to ignore `RelicState.used_up`; Meat on the Bone is not a
  one-time relic.
- Emits top insertion heal `12` when the player is alive and at or below half
  HP at combat end.
- Spawn gating is a relic-pool/reward-generation concern and is not handled by
  the combat victory hook.

Coverage:
- `shared_uncommon_start_victory_reward_relic_metadata_matches_java_sources`
- `meat_on_the_bone_heals_at_or_below_half_hp_without_used_up_gate`

### Pear

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/Pear.java`

Rust source:
- `src/content/relics/pear.rs`
- `src/engine/relic_manager.rs`

Java evidence:
- Constructor: ID `"Pear"`, tier `UNCOMMON`, landing sound `FLAT`.
- `onEquip()` calls `AbstractDungeon.player.increaseMaxHp(10, true)`.

Rust result:
- Tier matches Java.
- On-equip increases max HP by `10` and heals the same amount, capped by the
  new max HP.

Coverage:
- `shared_uncommon_start_victory_reward_relic_metadata_matches_java_sources`
- `pear_on_equip_grants_ten_max_hp_and_heals_same_amount`

### Singing Bowl

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/SingingBowl.java`
- `D:/rust/cardcrawl/screens/CardRewardScreen.java`
- `D:/rust/cardcrawl/ui/buttons/SingingBowlButton.java`

Rust source:
- `src/rewards/handler.rs`

Java evidence:
- Constructor: ID `"Singing Bowl"`, tier `UNCOMMON`, landing sound `FLAT`.
- The relic class itself only handles hover/click sound and flash.
- Card reward screen adds an extra bowl option; choosing it records the
  Singing Bowl choice and grants `+2` max HP instead of taking a card.
- `canSpawn` is false after floor 48 unless Endless mode is active.

Rust result:
- Tier matches Java.
- Reward card-choice handling exposes the bowl option at `idx == cards.len()`
  only when the relic is present and grants `+2` max HP through
  `DomainEventSource::RewardScreen`.
- UI-only hover sound/flash behavior is intentionally not represented.
- Spawn gating is a relic-pool/reward-generation concern and is not handled by
  the card-choice handler.

Coverage:
- `shared_uncommon_start_victory_reward_relic_metadata_matches_java_sources`
- `singing_bowl_card_reward_option_grants_two_max_hp_with_reward_source`

### White Beast Statue

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/WhiteBeast.java`
- `D:/rust/cardcrawl/rooms/AbstractRoom.java`

Rust source:
- `src/rewards/generator.rs`

Java evidence:
- Constructor: ID `"White Beast Statue"`, tier `UNCOMMON`, landing sound
  `SOLID`.
- The relic class itself has no gameplay hook.
- `AbstractRoom.addPotionToRewards()` sets potion chance to `100` when the
  player has the relic.
- The same Java method sets chance to `0` if the room already has at least four
  reward items.

Rust result:
- Tier matches Java.
- Combat reward generation sets potion drop chance to `100` when the relic is
  present and no Sozu potion block is active.
- The current Rust combat reward generator builds rewards from an empty list and
  checks potion generation before card/relic rewards, so the Java
  `rewards.size() >= 4` cap has no reachable equivalent in this path.

Coverage:
- `shared_uncommon_start_victory_reward_relic_metadata_matches_java_sources`
- `white_beast_statue_forces_potion_reward_unless_sozu_blocks_potions`

## Shared Relic Batch 6 - Uncommon Action / Counter Relics

### Blue Candle

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/BlueCandle.java`

Rust source:
- `src/content/relics/blue_candle.rs`
- `src/content/relics/hooks.rs`
- `src/engine/action_handlers/cards.rs`

Java evidence:
- Constructor: ID `"Blue Candle"`, tier `UNCOMMON`, landing sound `MAGICAL`.
- `onUseCard(card, action)`: if the card type is `CURSE`, queues
  `LoseHPAction(player, player, 1, FIRE)`, sets `card.exhaust = true`, and sets
  `action.exhaustCard = true`.

Rust result:
- Tier and `on_use_card` subscription match Java.
- Curse use queues player self HP loss `1` with the Rupture-triggering
  provenance used by Java's `LoseHPAction(player, player, ...)` path.
- The play handler exhausts Curse cards when Blue Candle is present, matching
  Java's `action.exhaustCard = true`.
- UI-only relic flash is intentionally not represented.

Coverage:
- `shared_uncommon_action_counter_relic_metadata_matches_java_sources`
- `blue_candle_only_loses_hp_for_curse_cards_and_marks_rupture_path`

### Ink Bottle

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/InkBottle.java`

Rust source:
- `src/content/relics/ink_bottle.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"InkBottle"`, tier `UNCOMMON`, landing sound `CLINK`, and
  initializes `counter = 0`.
- `onUseCard`: increments `counter` immediately. When `counter == 10`, it sets
  `counter = 0` immediately and queues `DrawCardAction(1)` with `addToBot`.
- `atBattleStart` only starts UI pulse when `counter == 9`.

Rust result:
- Tier, initial counter, and `on_use_card` subscription match Java.
- Fixed counter updates to mutate `RelicState.counter` immediately rather than
  queueing a later `UpdateRelicCounter` action.
- Fixed the negative-counter edge: Java `++counter` from `-1` reaches `0`
  without drawing; the old Rust `next_counter == 0` check would draw
  incorrectly.
- UI-only pulse/flash/relic-above-creature behavior is intentionally not
  represented.

Coverage:
- `shared_uncommon_action_counter_relic_metadata_matches_java_sources`
- `ink_bottle_counter_mutates_immediately_and_draws_on_tenth_card`

### Mummified Hand

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/MummifiedHand.java`

Rust source:
- `src/content/relics/mummified_hand.rs`
- `src/content/relics/hooks.rs`
- `src/engine/action_handlers/cards.rs`

Java evidence:
- Constructor: ID `"Mummified Hand"`, tier `UNCOMMON`, landing sound `FLAT`.
- `onUseCard`: if the played card is a POWER, it builds a candidate list from
  hand cards with `cost > 0`, `costForTurn > 0`, and not `freeToPlayOnce`.
- It removes cards already present in `AbstractDungeon.actionManager.cardQueue`.
- It selects one candidate with `AbstractDungeon.cardRandomRng` and immediately
  calls `setCostForTurn(0)`.

Rust result:
- Tier and `on_use_card` subscription match Java.
- Fixed the effect to mutate hand-card cost immediately during the relic hook,
  rather than queueing a later synthetic `MummifiedHandEffect` action.
- The candidate filter preserves Java's base-cost, current-cost,
  free-to-play-once, and queued-card exclusion rules.
- UI-only flash/relic-above-creature behavior and Java logger messages are
  intentionally not represented.

Coverage:
- `shared_uncommon_action_counter_relic_metadata_matches_java_sources`
- `mummified_hand_sets_one_eligible_hand_card_cost_to_zero_immediately`

### Sundial

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/Sundial.java`

Rust source:
- `src/content/relics/sundial.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Sundial"`, tier `UNCOMMON`, landing sound `SOLID`.
- `onEquip()` sets `counter = 0`.
- `onShuffle()` increments `counter` immediately. When `counter == 3`, it sets
  `counter = 0` immediately and queues `GainEnergyAction(2)` with `addToBot`.

Rust result:
- Tier, initial counter, and shuffle subscription match Java.
- Fixed counter updates to mutate `RelicState.counter` immediately rather than
  queueing a later `UpdateRelicCounter` action.
- Fixed the negative-counter edge: Java `++counter` from `-1` reaches `0`
  without granting energy; the old Rust `next_counter == 0` check would grant
  energy incorrectly.
- UI-only relic-above-creature behavior is intentionally not represented.

Coverage:
- `shared_uncommon_action_counter_relic_metadata_matches_java_sources`
- `sundial_counter_mutates_immediately_and_grants_energy_on_third_shuffle`

### Strike Dummy

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/StrikeDummy.java`

Rust source:
- `src/content/relics/strike_dummy.rs`
- `src/content/relics/hooks.rs`
- `src/content/cards/runtime_impl.rs`

Java evidence:
- Constructor: ID `"StrikeDummy"`, tier `UNCOMMON`, landing sound `HEAVY`.
- `atDamageModify(damage, card)` returns `damage + 3.0f` when the card has the
  `STRIKE` tag; otherwise it returns the input damage.

Rust result:
- Tier matches Java.
- Damage calculation routes relic damage modifiers before player power damage
  modifiers, preserving Java ordering for cases like Strike Dummy under Weak.
- The modifier checks the `Strike` tag, not the literal card name, so cards such
  as Pommel Strike and Twin Strike are covered.

Coverage:
- `shared_uncommon_action_counter_relic_metadata_matches_java_sources`
- `strike_dummy_adds_three_damage_to_strike_tag_attacks_before_power_modifiers`

### Matryoshka

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/Matryoshka.java`

Rust source:
- `src/content/relics/matryoshka.rs`
- `src/engine/run_loop.rs`
- `src/content/relics/mod.rs`

Java evidence:
- Constructor: ID `"Matryoshka"`, tier `UNCOMMON`, landing sound `SOLID`, and
  initializes `counter = 2`.
- `onChestOpen(bossChest)`: only non-boss chests trigger. While `counter > 0`,
  decrements the counter, adds one extra relic reward, and rolls the extra relic
  tier as `75% COMMON / 25% UNCOMMON`.
- When counter reaches `0`, `setCounter(-2)` marks the relic used up.
- `canSpawn` is false after floor 40 unless Endless mode is active.

Rust result:
- Tier and initial counter match Java.
- Treasure-room handling applies the non-boss chest behavior, decrements the
  counter, marks the relic used up at `-2`, and rolls the extra reward tier with
  the same 75/25 split.
- Spawn gating is a relic-pool/reward-generation concern and is not handled by
  the chest helper.

Coverage:
- `shared_uncommon_action_counter_relic_metadata_matches_java_sources`
- `matryoshka_counter_starts_at_two_and_only_positive_counter_grants_extra_relic`

## Shared Relic Batch 7 - Bottles / Shop / Rest / Special Reward Relics

### Bottled Flame

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/BottledFlame.java`
- `D:/rust/cardcrawl/dungeons/AbstractDungeon.java`

Rust source:
- `src/engine/relic_manager.rs`
- `src/engine/run_loop.rs`
- `src/rewards/handler.rs`
- `src/runtime/combat.rs`
- `src/state/core.rs`
- `src/state/run.rs`

Java evidence:
- Constructor: ID `"Bottled Flame"`, tier `UNCOMMON`, landing sound `CLINK`.
- `canSpawn()` is true only when the master deck has a non-BASIC ATTACK.
- `onEquip()` opens a grid select over purgeable ATTACK cards; the selected
  card is marked `inBottleFlame = true`.
- `onUnequip()` clears that selected card flag.
- `atBattleStart()` only flashes and queues a relic visual action.
- `returnRandomRelic` can return bottled relics, while
  `returnRandomScreenlessRelic` skips Bottled Flame, Bottled Lightning,
  Bottled Tornado, and Whetstone.

Rust result:
- Normal relic reward rolls can now return bottled relics; screenless event
  reward paths skip them.
- Added a run pending selection reason for Bottled Flame and filter targets to
  ATTACK cards for `onEquip`.
- The selected card uuid is stored in the relic state, and combat deck
  initialization treats that uuid as innate.
- Claiming a bottled relic from a reward screen preserves the remaining reward
  items as the return state after the deck selection.
- Spawn gating now rejects Bottled Flame unless the deck has a non-BASIC attack.
- Java's UI/VFX-only battle-start relic action is intentionally not represented.

Coverage:
- `shared_uncommon_bottle_shop_and_rest_relic_metadata_matches_java_sources`
- `normal_relic_rewards_can_return_bottled_relics_but_screenless_rewards_skip_them`
- `bottled_relic_on_equip_filters_selection_by_card_type_and_marks_uuid`
- `bottled_relic_uuid_counts_as_innate_during_combat_deck_initialization`
- `interrupting_relic_claim_preserves_remaining_reward_screen_items`

### Bottled Lightning

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/BottledLightning.java`

Rust source:
- `src/engine/relic_manager.rs`
- `src/engine/run_loop.rs`
- `src/rewards/handler.rs`
- `src/runtime/combat.rs`
- `src/state/core.rs`
- `src/state/run.rs`

Java evidence:
- Constructor: ID `"Bottled Lightning"`, tier `UNCOMMON`, landing sound
  `CLINK`.
- `canSpawn()` is true only when the master deck has a non-BASIC SKILL.
- `onEquip()` opens a grid select over purgeable SKILL cards; the selected card
  is marked `inBottleLightning = true`.
- `atBattleStart()` is UI-only.

Rust result:
- Added selection reason, target filtering, selected uuid persistence, and
  combat start innate handling for Bottled Lightning.
- Spawn gating now rejects it unless the deck has a non-BASIC skill.
- UI-only battle-start behavior is intentionally not represented.

Coverage:
- `shared_uncommon_bottle_shop_and_rest_relic_metadata_matches_java_sources`
- `normal_relic_rewards_can_return_bottled_relics_but_screenless_rewards_skip_them`

### Bottled Tornado

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/BottledTornado.java`

Rust source:
- `src/engine/relic_manager.rs`
- `src/engine/run_loop.rs`
- `src/rewards/handler.rs`
- `src/runtime/combat.rs`
- `src/state/core.rs`
- `src/state/run.rs`

Java evidence:
- Constructor: ID `"Bottled Tornado"`, tier `UNCOMMON`, landing sound `CLINK`.
- `canSpawn()` checks for any POWER card in the master deck.
- `onEquip()` opens a grid select over purgeable POWER cards; the selected card
  is marked `inBottleTornado = true`.
- `atBattleStart()` is UI-only.

Rust result:
- Added selection reason, target filtering, selected uuid persistence, and
  combat start innate handling for Bottled Tornado.
- Spawn gating now rejects it unless the deck has a power.
- UI-only battle-start behavior is intentionally not represented.

Coverage:
- `shared_uncommon_bottle_shop_and_rest_relic_metadata_matches_java_sources`
- `bottled_relic_uuid_counts_as_innate_during_combat_deck_initialization`

### Courier

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/Courier.java`
- `D:/rust/cardcrawl/shop/ShopScreen.java`

Rust source:
- `src/shop/shop_handler.rs`
- `src/shop/shop_screen.rs`
- `src/state/run.rs`

Java evidence:
- Constructor: ID `"The Courier"`, tier `UNCOMMON`, landing sound `FLAT`.
- Shop prices are multiplied by `0.8`.
- Purchased shop cards, relics, and potions are replenished rather than leaving
  the slot empty.
- `onEnterRoom(ShopRoom)` only sets UI pulse/flash state.
- `canSpawn()` is false after floor 48 unless Endless mode is active and is
  false while currently in a shop.

Rust result:
- Existing shop purchase paths preserve the 20% discount and replenish card,
  relic, and potion slots when Courier is present.
- Spawn gating now blocks Courier after floor 48 in non-Endless runs and rejects
  it while the current room is a ShopRoom, matching the Java `canSpawn` clause.
  Rust does not currently model Endless mode.
- UI-only room-entry pulse behavior is intentionally not represented.

Coverage:
- `shared_uncommon_bottle_shop_and_rest_relic_metadata_matches_java_sources`
- `rare_run_relic_can_spawn_gates_match_java_sources`
- `courier_keeps_relic_slot_filled_after_purchase`
- `courier_keeps_potion_slot_filled_after_purchase`
- `courier_does_not_refill_sozu_blocked_shop_potion_purchase`
- `courier_keeps_card_slot_filled_after_purchase`

### Eternal Feather

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/EternalFeather.java`

Rust source:
- `src/engine/run_loop.rs`

Java evidence:
- Constructor: ID `"Eternal Feather"`, tier `UNCOMMON`, landing sound
  `MAGICAL`.
- `onEnterRoom(RestRoom)` heals `masterDeck.size() / 5 * 3`.

Rust result:
- Rest-room entry applies the same integer deck-size formula and emits the
  healing through the Eternal Feather relic source.
- Mark of the Bloom blocks the heal through the out-of-combat healing guard.
- UI-only flash behavior is intentionally not represented.

Coverage:
- `shared_uncommon_bottle_shop_and_rest_relic_metadata_matches_java_sources`
- `eternal_feather_rest_room_heal_uses_relic_source_and_mark_of_bloom_guard`

### Nloth's Gift

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/NlothsGift.java`

Rust source:
- `src/rewards/generator.rs`
- `src/content/relics/mod.rs`

Java evidence:
- Constructor: ID `"Nloth's Gift"`, tier `SPECIAL`, landing sound `FLAT`.
- `changeRareCardRewardChance(rareCardChance)` returns `rareCardChance * 3`.

Rust result:
- Card reward generation triples the rare-card threshold while Nloth's Gift is
  present.
- The relic has no combat hook subscriptions.

Coverage:
- `shared_uncommon_bottle_shop_and_rest_relic_metadata_matches_java_sources`
- `rewards::generator::tests` coverage for rare-card reward generation

## Shared Relic Batch 8 - Rare Start / Turn Counter Relics

### Captain's Wheel

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/CaptainsWheel.java`

Rust source:
- `src/content/relics/captains_wheel.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"CaptainsWheel"`, tier `RARE`, landing sound `FLAT`.
- `atBattleStart`: sets `counter = 0`.
- `atTurnStart`: increments `counter` only while the relic is not grayscale.
- When `counter == 3`, queues player block `18` with `addToBot`, then sets
  `counter = -1` and `grayscale = true`.
- `onVictory`: sets `counter = -1` and clears grayscale.

Rust result:
- Tier and hook subscriptions match Java.
- Fixed the turn-start hook to mutate the relic counter immediately instead of
  emitting `UpdateRelicCounter` actions.
- Fixed the firing state to set `counter = -1` after the block trigger, matching
  Java's disabled/grayscale combat state.
- UI-only grayscale and relic-above-creature actions are intentionally not
  represented.

Coverage:
- `shared_rare_start_and_turn_relic_metadata_matches_java_sources`
- `captains_wheel_mutates_counter_immediately_and_fires_once_on_third_turn`
- `hook_persists_mutated_turn_start_relic_counters_before_actions_execute`

### Clockwork Souvenir

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/ClockworkSouvenir.java`

Rust source:
- `src/content/relics/clockwork_souvenir.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"ClockworkSouvenir"`, tier `SHOP`, landing sound `HEAVY`.
- `atBattleStart`: queues player Artifact `1` with `addToTop`.

Rust result:
- Tier and battle-start subscription match Java.
- Emits player Artifact `1` with top insertion.
- UI-only flash behavior is intentionally not represented.

Coverage:
- `shared_rare_start_and_turn_relic_metadata_matches_java_sources`
- `start_relic_action_order_matches_java_non_ui_actions`

### Fossilized Helix

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/FossilizedHelix.java`

Rust source:
- `src/content/relics/fossilized_helix.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"FossilizedHelix"`, tier `RARE`, landing sound `HEAVY`.
- `atBattleStart`: queues relic-above-creature UI action, then queues player
  Buffer `1` with `addToBot`, and sets grayscale.
- `justEnteredRoom`: clears grayscale.

Rust result:
- Tier and battle-start subscription match Java.
- Emits player Buffer `1` with bottom insertion.
- UI-only relic-above-creature and grayscale room-state behavior is
  intentionally not represented.

Coverage:
- `shared_rare_start_and_turn_relic_metadata_matches_java_sources`
- `start_relic_action_order_matches_java_non_ui_actions`

### Incense Burner

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/IncenseBurner.java`

Rust source:
- `src/content/relics/incense_burner.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Incense Burner"`, tier `RARE`, landing sound `CLINK`.
- `onEquip`: sets `counter = 0`.
- `atTurnStart`: updates `counter = counter == -1 ? counter + 2 : counter + 1`.
- If the resulting counter is `6`, sets `counter = 0` and queues player
  Intangible `1` with `addToBot`.

Rust result:
- Tier and turn-start subscription match Java.
- Fixed the turn-start hook to mutate `RelicState.counter` immediately instead
  of queuing a delayed counter update.
- Fixed the `-1 -> 1` uninitialized transition and the `6 -> 0` firing reset.
- UI-only flash / relic-above-creature behavior is intentionally not
  represented.

Coverage:
- `shared_rare_start_and_turn_relic_metadata_matches_java_sources`
- `incense_burner_counter_mutates_immediately_and_grants_intangible_on_six`
- `hook_persists_mutated_turn_start_relic_counters_before_actions_execute`

### Pocketwatch

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/Pocketwatch.java`

Rust source:
- `src/content/relics/pocketwatch.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Pocketwatch"`, tier `RARE`, landing sound `FLAT`.
- `atBattleStart`: sets `counter = 0` and private `firstTurn = true`.
- `onPlayCard`: increments `counter`.
- `atTurnStartPostDraw`: if `counter <= 3` and not first turn, queues draw
  `3`; otherwise clears `firstTurn`; then sets `counter = 0`.
- `onVictory`: sets `counter = -1`.

Rust result:
- Tier and hook subscriptions match Java.
- Uses `RelicState.amount` to store Java's private `firstTurn` flag.
- Draws 3 on non-first turns after 0-3 cards were played and resets counter
  every post-draw turn.
- UI-only pulse behavior is intentionally not represented.

Coverage:
- `shared_rare_start_and_turn_relic_metadata_matches_java_sources`
- `pocketwatch_first_turn_and_three_card_limit_match_java`

### Stone Calendar

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/StoneCalendar.java`

Rust source:
- `src/content/relics/stone_calendar.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"StoneCalendar"`, tier `RARE`, landing sound `HEAVY`.
- `atBattleStart`: sets `counter = 0`.
- `atTurnStart`: increments `counter`; starts a UI pulse when `counter == 7`.
- `onPlayerEndTurn`: if `counter == 7`, queues all-enemy damage `52` using
  `DamageAllEnemiesAction(null, ..., THORNS, ...)`, stops pulse, and sets
  grayscale.
- `onVictory`: sets `counter = -1`.

Rust result:
- Tier and hook subscriptions match Java.
- Fixed turn-start to mutate the relic counter immediately instead of queuing a
  delayed counter update.
- Fixed the end-turn damage source to `NO_SOURCE`, matching Java's `null`
  `DamageInfo.owner`.
- UI-only pulse/grayscale and relic-above-creature actions are intentionally not
  represented.

Coverage:
- `shared_rare_start_and_turn_relic_metadata_matches_java_sources`
- `stone_calendar_counter_and_null_source_damage_match_java`
- `hook_persists_mutated_turn_start_relic_counters_before_actions_execute`

### Thread and Needle

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/ThreadAndNeedle.java`

Rust source:
- `src/content/relics/thread_and_needle.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Thread and Needle"`, tier `RARE`, landing sound `CLINK`.
- `atBattleStart`: queues player Plated Armor `4` with `addToTop`, then queues a
  relic-above-creature UI action with `addToTop`.

Rust result:
- Tier and battle-start subscription match Java.
- Fixed Plated Armor action insertion from bottom to top.
- UI-only relic-above-creature behavior is intentionally not represented.

Coverage:
- `shared_rare_start_and_turn_relic_metadata_matches_java_sources`
- `start_relic_action_order_matches_java_non_ui_actions`

## Shared Relic Batch 9 - Rare Damage / Retention Modifiers

### Calipers

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/Calipers.java`

Rust source:
- `src/content/relics/calipers.rs`
- `src/engine/core.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Calipers"`, tier `RARE`, landing sound `CLINK`.
- The relic has no explicit hook method in its own Java class; block-retention
  behavior is implemented by the player's end/start turn block clearing logic.
- The gameplay effect is losing `15` block instead of all block when turn block
  cleanup runs, unless another mechanic such as Barricade preserves all block.

Rust result:
- Tier and block-retention subscription match the engine-level Java behavior.
- Retains `max(block - 15, 0)` at the player-turn boundary when Barricade is not
  present.

Coverage:
- `shared_rare_damage_retention_relic_metadata_matches_java_sources`
- `calipers_retains_only_block_above_fifteen_without_barricade_logic`

### Torii

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/Torii.java`

Rust source:
- `src/content/relics/torii.rs`
- `src/content/relics/hooks.rs`
- `src/engine/action_handlers/damage.rs`

Java evidence:
- Constructor: ID `"Torii"`, tier `RARE`, landing sound `HEAVY`.
- `onAttacked(info, damageAmount)` returns `1` only when:
  - `info.owner != null`;
  - damage type is not `HP_LOSS`;
  - damage type is not `THORNS`;
  - final incoming damage is `2..=5`.
- UI-only flash / relic-above-creature behavior has no simulator state effect.

Rust result:
- Tier and attacked-damage modifier subscription match Java.
- Fixed Rust to exclude both player source `0` and `NO_SOURCE`, matching Java's
  `info.owner != null` guard.
- HP-loss and thorns damage remain unmodified.

Coverage:
- `shared_rare_damage_retention_relic_metadata_matches_java_sources`
- `torii_requires_real_non_player_owner_and_normal_non_thorns_damage`

### Tungsten Rod

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/TungstenRod.java`

Rust source:
- `src/content/relics/tungsten_rod.rs`
- `src/content/relics/hooks.rs`
- `src/engine/action_handlers/damage.rs`

Java evidence:
- Constructor: ID `"TungstenRod"`, tier `RARE`, landing sound `HEAVY`.
- `onLoseHpLast(damageAmount)` returns `damageAmount - 1` only when
  `damageAmount > 0`.
- The Java relic has no `onLoseHp` action hook.

Rust result:
- Tier and final HP-loss modifier subscription match Java.
- Removed the spurious `on_lose_hp` subscription and empty action hook.
- Keeps the final HP-loss reduction at `max(amount - 1, 0)`.

Coverage:
- `shared_rare_damage_retention_relic_metadata_matches_java_sources`
- `tungsten_rod_is_only_final_hp_loss_modifier`

## Shared Relic Batch 10 - Rare Passive / Revive / Energy Relics

### Ginger

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/Ginger.java`
- `D:/rust/cardcrawl/actions/common/ApplyPowerAction.java`

Rust source:
- `src/content/relics/ginger.rs`
- `src/content/relics/hooks.rs`
- `src/engine/action_handlers/powers.rs`
- `src/content/powers/core/weak.rs`

Java evidence:
- Constructor: ID `"Ginger"`, tier `RARE`, landing sound `FLAT`.
- Ginger has no hook method in its relic class.
- `ApplyPowerAction.update()` checks Ginger before Artifact: if the target is
  the player and the power to apply is `"Weakened"`, it returns without applying
  the power or consuming Artifact.
- Weak end-of-round cleanup uses `ReducePowerAction`, not `ApplyPowerAction`.

Rust result:
- Tier and receive-power modifier subscription match the Java ApplyPowerAction
  path.
- Removed unused empty hook helpers from the Ginger module.
- Fixed Weak cleanup to use `ReducePower` instead of `ApplyPower(-1)`, so Ginger
  blocks new Weak applications without blocking normal turn countdown.
- Artifact remains unconsumed when Ginger blocks Weak.

Coverage:
- `shared_rare_passive_resource_relic_metadata_matches_java_sources`
- `ginger_and_turnip_block_apply_power_before_artifact_without_blocking_cleanup`

### Turnip

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/Turnip.java`
- `D:/rust/cardcrawl/actions/common/ApplyPowerAction.java`

Rust source:
- `src/content/relics/turnip.rs`
- `src/content/relics/hooks.rs`
- `src/engine/action_handlers/powers.rs`
- `src/content/powers/core/frail.rs`

Java evidence:
- Constructor: ID `"Turnip"`, tier `RARE`, landing sound `FLAT`.
- Turnip has no hook method in its relic class.
- `ApplyPowerAction.update()` checks Turnip before Artifact: if the target is
  the player and the power to apply is `"Frail"`, it returns without applying
  the power or consuming Artifact.
- Frail end-of-round cleanup uses `ReducePowerAction`, not `ApplyPowerAction`.

Rust result:
- Tier and receive-power modifier subscription match the Java ApplyPowerAction
  path.
- Removed unused empty helper functions from the Turnip module.
- Fixed Frail cleanup to use `ReducePower` instead of `ApplyPower(-1)`, so
  Turnip blocks new Frail applications without blocking normal turn countdown.

Coverage:
- `shared_rare_passive_resource_relic_metadata_matches_java_sources`
- `ginger_and_turnip_block_apply_power_before_artifact_without_blocking_cleanup`

### Ice Cream

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/IceCream.java`
- `D:/rust/cardcrawl/core/EnergyManager.java`

Rust source:
- `src/content/relics/ice_cream.rs`
- `src/content/relics/hooks.rs`
- `src/runtime/combat.rs`

Java evidence:
- Constructor: ID `"Ice Cream"`, tier `RARE`, landing sound `FLAT`.
- Ice Cream has no hook method in its relic class.
- `EnergyManager.recharge()`: when Ice Cream is present, it calls
  `EnergyPanel.addEnergy(this.energy)` instead of `EnergyPanel.setEnergy(...)`,
  preserving unspent energy and adding the base recharge.
- UI-only flash / relic-above-creature behavior is intentionally ignored.

Rust result:
- Tier and energy-retention subscription match Java.
- Fixed player turn begin to preserve current energy and add `energy_master`
  when the Ice Cream energy-retention hook is present.

Coverage:
- `shared_rare_passive_resource_relic_metadata_matches_java_sources`
- `ice_cream_recharge_preserves_unspent_energy_before_adding_base_energy`

### Lizard Tail

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/LizardTail.java`
- `D:/rust/cardcrawl/characters/AbstractPlayer.java`

Rust source:
- `src/content/relics/lizard_tail.rs`
- `src/engine/action_handlers/mod.rs`

Java evidence:
- Constructor: ID `"Lizard Tail"`, tier `RARE`, landing sound `MAGICAL`.
- `AbstractPlayer.damage(...)` revive priority:
  - if Mark of the Bloom is present, no Fairy/Lizard revive;
  - Fairy Potion triggers before Lizard Tail;
  - Lizard Tail triggers only when the relic exists and `counter == -1`.
- `LizardTail.onTrigger()` heals `max(maxHealth / 2, 1)` and calls
  `setCounter(-2)`, marking the relic used up.

Rust result:
- Tier matches Java. Lizard Tail remains handled inline by death/revive logic,
  not by a generic on-lose-hp bus.
- Fixed revive eligibility to require `counter == -1` as well as `!used_up`.
- Fairy Potion priority and Mark of the Bloom blocking behavior are covered.
- Fairy Potion passive revive is deliberately not treated as a potion-use relic
  hook. Java reaches it from `AbstractPlayer.damage`, not `PotionPopUp`, so
  `ToyOrnithopter.onUsePotion()` does not fire on Fairy revive.
- UI-only relic-above-creature behavior is intentionally not represented.

Coverage:
- `shared_rare_passive_resource_relic_metadata_matches_java_sources`
- `lizard_tail_uses_java_counter_gate_and_fairy_priority`
- `fairy_passive_revive_does_not_trigger_toy_ornithopter_on_use_potion`

## Shared Relic Batch 11 - Rare Run / Campfire Relics

### Mango

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/Mango.java`

Rust source:
- `src/content/relics/mango.rs`
- `src/engine/relic_manager.rs`

Java evidence:
- Constructor: ID `"Mango"`, tier `RARE`, landing sound `FLAT`.
- `onEquip`: calls `increaseMaxHp(14, true)`.

Rust result:
- Tier matches Java.
- Run-level on-equip increases max HP by `14` and heals current HP by `14`,
  capped at max HP.

Coverage:
- `shared_rare_run_campfire_relic_metadata_matches_java_sources`
- `mango_and_old_coin_on_equip_match_java_resource_changes`

### Old Coin

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/OldCoin.java`

Rust source:
- `src/content/relics/old_coin.rs`
- `src/engine/relic_manager.rs`
- `src/state/run.rs`
- `src/engine/shop_handler.rs`

Java evidence:
- Constructor: ID `"Old Coin"`, tier `RARE`, landing sound `CLINK`.
- `onEquip`: calls `AbstractDungeon.player.gainGold(300)`.
- `canSpawn`: requires floor `<= 48` unless Endless, and rejects shop rooms.

Rust result:
- Tier and on-equip gold amount match Java.
- Ectoplasm blocks the gold gain through the same gain-gold semantics.
- Added normal relic-pool spawn gating for floor `<= 48`.
- Shop-room exclusion is handled by shop generation excluding Old Coin from
  shop relic sale slots.

Coverage:
- `shared_rare_run_campfire_relic_metadata_matches_java_sources`
- `mango_and_old_coin_on_equip_match_java_resource_changes`
- `rare_run_relic_can_spawn_gates_match_java_sources`

### Girya

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/Girya.java`

Rust source:
- `src/content/relics/girya.rs`
- `src/content/relics/mod.rs`
- `src/content/relics/hooks.rs`
- `src/engine/campfire_handler.rs`
- `src/state/run.rs`

Java evidence:
- Constructor: ID `"Girya"`, tier `RARE`, landing sound `HEAVY`, and
  `counter = 0`.
- `atBattleStart`: if `counter != 0`, queues player Strength equal to counter
  with `addToTop`.
- `addCampfireOption`: adds Lift, enabled while `counter < 3`.
- `canSpawn`: rejects floor `>= 48` unless Endless and rejects when two of
  Girya / Peace Pipe / Shovel are already owned.

Rust result:
- Tier and battle-start subscription match Java.
- Fixed `RelicState::new(Girya)` to initialize `counter = 0`.
- Campfire Lift increments counter up to `3`; battle start applies matching
  Strength with top insertion.
- Added rare relic-pool spawn gating for floor and two-campfire-relic limit.
- UI-only relic-above-creature behavior is intentionally not represented.

Coverage:
- `shared_rare_run_campfire_relic_metadata_matches_java_sources`
- `girya_lift_counter_and_battle_start_strength_match_java`
- `rare_run_relic_can_spawn_gates_match_java_sources`

### Peace Pipe

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/PeacePipe.java`

Rust source:
- `src/engine/campfire_handler.rs`
- `src/state/run.rs`

Java evidence:
- Constructor: ID `"Peace Pipe"`, tier `RARE`, landing sound `FLAT`.
- `addCampfireOption`: adds Toke when the deck has a non-bottled purgeable card.
- `canSpawn`: rejects floor `>= 48` unless Endless and rejects when two of
  Girya / Peace Pipe / Shovel are already owned.

Rust result:
- Tier matches Java. The effect is modeled in campfire option generation and
  campfire execution, not in a combat hook.
- Existing Toke path filters bottled cards by bottle relic stored UUID.
- Added rare relic-pool spawn gating for floor and two-campfire-relic limit.

Coverage:
- `shared_rare_run_campfire_relic_metadata_matches_java_sources`
- `rare_run_relic_can_spawn_gates_match_java_sources`

### Prayer Wheel

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/PrayerWheel.java`

Rust source:
- `src/rewards/generator.rs`
- `src/state/run.rs`

Java evidence:
- Constructor: ID `"Prayer Wheel"`, tier `RARE`, landing sound `CLINK`.
- `canSpawn`: requires floor `<= 48` unless Endless.
- Reward generation adds an extra card reward for non-boss monster rewards.

Rust result:
- Tier matches Java.
- Existing reward generation adds a second card reward for non-boss fights while
  Prayer Wheel is present.
- Added normal relic-pool spawn gating for floor `<= 48`.

Coverage:
- `shared_rare_run_campfire_relic_metadata_matches_java_sources`
- `prayer_wheel_adds_second_non_boss_card_reward`
- `rare_run_relic_can_spawn_gates_match_java_sources`

### Shovel

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/Shovel.java`

Rust source:
- `src/engine/campfire_handler.rs`
- `src/state/run.rs`

Java evidence:
- Constructor: ID `"Shovel"`, tier `RARE`, landing sound `FLAT`.
- `addCampfireOption`: adds Dig.
- `canSpawn`: rejects floor `>= 48` unless Endless and rejects when two of
  Girya / Peace Pipe / Shovel are already owned.

Rust result:
- Tier matches Java. The effect is modeled in campfire option generation and Dig
  execution, not in a combat hook.
- Dig grants a relic through a reward screen.
- Added rare relic-pool spawn gating for floor and two-campfire-relic limit.

Coverage:
- `shared_rare_run_campfire_relic_metadata_matches_java_sources`
- `rare_run_relic_can_spawn_gates_match_java_sources`

### Wing Boots

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/WingBoots.java`

Rust source:
- `src/content/relics/mod.rs`
- `src/engine/run_loop.rs`
- `src/map/state.rs`
- `src/cli/full_run_smoke/actions.rs`
- `src/state/run.rs`

Java evidence:
- Constructor: ID `"WingedGreaves"`, tier `RARE`, landing sound `FLAT`, and
  `counter = 3`.
- `setCounter(-2)` marks the relic used up.
- `canSpawn`: requires floor `<= 40` unless Endless.
- Map movement is hosted in `MapRoomNode.wingedIsConnectedTo`: while the relic
  has charges, it ignores target X but still requires the target node to be on
  an outgoing edge row (`node.y == edge.dstY`). It does not skip arbitrary
  future rows.

Rust result:
- Tier and initial counter `3` match Java.
- Flight consumes charges in map navigation and marks the relic used up at zero.
- Added normal relic-pool spawn gating for floor `<= 40`.
- Corrected map flight semantics to expose/use `FlyToNode` only for otherwise
  unconnected nodes on the next reachable row, not multi-row jumps.

Coverage:
- `shared_rare_run_campfire_relic_metadata_matches_java_sources`
- `rare_run_relic_can_spawn_gates_match_java_sources`
- `wing_boots_matches_java_next_row_only_semantics`
- `legal_map_actions_expose_wing_boots_only_on_next_row`

## Shared Relic Batch 12 - Rare Card Flow / Refresh Relics

### Bird-Faced Urn

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/BirdFacedUrn.java`

Rust source:
- `src/content/relics/bird_faced_urn.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Bird-Faced Urn"`, tier `RARE`, landing sound `CLINK`.
- `onUseCard`: if the used card type is `POWER`, queues `HealAction` for `2`
  with `addToTop`; UI relic action is also top but is not gameplay.

Rust result:
- Tier and `on_use_card` subscription match Java.
- Power-card use queues player heal `2` with top insertion.
- Non-power cards do not trigger.
- UI-only relic-above-creature behavior is intentionally not represented.

Coverage:
- `shared_rare_card_flow_relic_metadata_matches_java_sources`
- `bird_faced_urn_heals_only_when_power_card_is_used`

### Dead Branch

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/DeadBranch.java`

Rust source:
- `src/content/relics/dead_branch.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Dead Branch"`, tier `RARE`, landing sound `FLAT`.
- `onExhaust`: only fires when `!AbstractDungeon.getMonsters().areMonstersBasicallyDead()`.
- Gameplay action is `MakeTempCardInHandAction(returnTrulyRandomCardInCombat().makeCopy(), false)`
  with `addToBot`; UI relic action is ignored.

Rust result:
- Tier and `on_exhaust` subscription match Java.
- Fixed missing `areMonstersBasicallyDead` gate so final-kill exhausts do not
  generate a random card.
- Random combat card generation remains unfiltered and bottom-queued.

Coverage:
- `shared_rare_card_flow_relic_metadata_matches_java_sources`
- `dead_branch_skips_when_monsters_are_basically_dead`

### Du-Vu Doll

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/DuVuDoll.java`

Rust source:
- `src/content/relics/du_vu_doll.rs`
- `src/engine/relic_manager.rs`
- `src/state/run.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Du-Vu Doll"`, tier `RARE`, landing sound `MAGICAL`.
- `onEquip` and `onMasterDeckChange`: recalculate `counter` from curses in
  `AbstractDungeon.player.masterDeck.group`.
- `atBattleStart`: if `counter > 0`, queues player Strength equal to counter
  with `addToTop`; UI relic action is ignored.

Rust result:
- Tier and battle-start subscription match Java.
- Fixed battle-start logic to use the relic counter, not the combat draw pile.
  The old implementation missed curses already drawn into hand before
  `atBattleStart`.
- Added run-level `on_equip` and master-deck-change counter refresh for Du-Vu
  Doll.

Coverage:
- `shared_rare_card_flow_relic_metadata_matches_java_sources`
- `du_vu_doll_counter_tracks_master_deck_and_battle_start_uses_counter`

### Gambling Chip

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/GamblingChip.java`
- `D:/rust/cardcrawl/actions/unique/GamblingChipAction.java`

Rust source:
- `src/content/relics/gambling_chip.rs`
- `src/content/relics/mod.rs`
- `src/content/relics/hooks.rs`
- `src/engine/pending_choices.rs`

Java evidence:
- Constructor: ID `"Gambling Chip"`, tier `RARE`, landing sound `FLAT`.
- `atBattleStartPreDraw`: sets private `activated = false`.
- `atTurnStartPostDraw`: if not activated, sets activated true and queues
  `GamblingChipAction` with `addToBot`.
- `GamblingChipAction`: choose any number of hand cards, discard them, then
  draw the same count.

Rust result:
- Tier matches Java.
- Fixed subscriptions: Gambling Chip now registers both
  `at_battle_start_pre_draw` and `at_turn_start_post_draw`, and no longer uses
  the wrong `at_turn_start` phase.
- Reuses `RelicState.used_up` for Java's private activated flag.
- Existing pending-choice resolution discards selected cards and draws the same
  count.

Coverage:
- `shared_rare_card_flow_relic_metadata_matches_java_sources`
- `gambling_chip_resets_pre_draw_and_fires_once_post_draw`

### Unceasing Top

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/UnceasingTop.java`
- `D:/rust/cardcrawl/actions/GameActionManager.java`

Rust source:
- `src/content/relics/unceasing_top.rs`
- `src/content/relics/mod.rs`
- `src/content/relics/hooks.rs`
- `src/engine/core.rs`
- `src/engine/action_handlers/cards.rs`

Java evidence:
- Constructor: ID `"Unceasing Top"`, tier `RARE`, landing sound `CLINK`.
- `atPreBattle`: `canDraw = false`.
- `atTurnStart`: `canDraw = true`, `disabledUntilEndOfTurn = false`.
- `onRefreshHand`: when the action queue is empty, the player hand is empty,
  turn has not ended, canDraw is true, the player lacks `No Draw`, the room is
  in combat, the relic is not disabled, and draw/discard piles are not both
  empty, queues draw 1.
- `GameActionManager.getNextAction`: disables Unceasing Top until turn end when
  resolving the final end-turn autoplay queued card.

Rust result:
- Tier and start-turn subscriptions match Java.
- Implemented a headless engine refresh check from mechanical state only:
  no UI/screen state is modeled.
- `RelicState.amount` stores Java `canDraw`; `RelicState.used_up` stores
  Java `disabledUntilEndOfTurn`.
- Engine checks Unceasing Top before returning player control when the action
  queue is empty.
- Final end-turn autoplay queued card disables the relic until the turn ends.

Coverage:
- `shared_rare_card_flow_relic_metadata_matches_java_sources`
- `unceasing_top_uses_mechanical_refresh_conditions_without_ui_state`

### Pre-Battle Relic Bus Registration

Status: `wrong-fixed`

Rust source:
- `src/runtime/combat.rs`

Finding:
- `PlayerEntity::add_relic` registered most relic buses but omitted
  `at_pre_battle` and `at_battle_start_pre_draw`. That meant runtime-added
  combat relics could silently miss Java lifecycle hooks.

Rust result:
- Added both missing bus registrations.

Coverage:
- `player_add_relic_registers_pre_battle_and_pre_draw_buses`

## Shared Relic Batch 13 - Boss Relics Part 1

### Astrolabe

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/Astrolabe.java`
- `D:/rust/cardcrawl/cards/CardGroup.java`

Rust source:
- `src/content/relics/astrolabe.rs`
- `src/engine/relic_manager.rs`
- `src/state/run.rs`

Java evidence:
- Constructor: ID `"Astrolabe"`, tier `BOSS`, landing sound `CLINK`.
- `onEquip`: builds candidates from `masterDeck.getPurgeableCards()`.
- `CardGroup.getPurgeableCards()` excludes only `Necronomicurse`,
  `CurseOfTheBell`, and `AscendersBane`.
- If candidate count is `0`, no effect.
- If candidate count is `<= 3`, transforms all candidates immediately.
- If candidate count is `> 3`, opens a grid select for exactly `3`.
- `giveCards`: removes selected cards, calls `transformCard(card, true, miscRng)`.

Rust result:
- Tier and relic-manager on-equip route match Java.
- Fixed candidate filtering: normal curses such as Injury/Pain are purgeable;
  only Java's three unpurgeable special curses are excluded.
- Fixed small-deck behavior: `<= 3` candidates now auto-transform without
  creating a pending choice.
- Pending choice for larger decks is exactly three `TransformUpgraded` picks.
- Run pending-choice candidate generation and execution reject Java's
  unpurgeable special curses for Astrolabe selections.

Coverage:
- `shared_boss_relic_first_batch_metadata_matches_java_sources`
- `astrolabe_uses_java_purgeable_cards_and_auto_transforms_three_or_fewer`

### Black Star

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/BlackStar.java`

Rust source:
- `src/rewards/generator.rs`

Java evidence:
- Constructor: ID `"Black Star"`, tier `BOSS`, landing sound `HEAVY`.
- UI pulse/flash occurs on elite rooms and victory.
- Gameplay effect is reward-side: elite fights grant an additional relic reward.

Rust result:
- Tier matches Java.
- Elite combat reward generation adds a second relic while Black Star is owned.
- UI-only pulse/flash behavior is intentionally not represented.

Coverage:
- `shared_boss_relic_first_batch_metadata_matches_java_sources`
- `boss_relic_passives_affect_rewards_campfires_and_curse_pool`

### Busted Crown

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/BustedCrown.java`

Rust source:
- `src/content/relics/mod.rs`
- `src/rewards/generator.rs`

Java evidence:
- Constructor: ID `"Busted Crown"`, tier `BOSS`, landing sound `CLINK`.
- `onEquip`: increments `energyMaster`.
- `onUnequip`: decrements `energyMaster`.
- `changeNumberOfCardsInReward(numberOfCards)`: returns `numberOfCards - 2`.

Rust result:
- Tier and energy delta match Java.
- Combat player construction derives +1 energy from boss relic energy deltas.
- Card reward choice count applies the `-2` modifier in reward generation.

Coverage:
- `shared_boss_relic_first_batch_metadata_matches_java_sources`
- `rewards::generator::tests::busted_crown_reduces_choices_with_floor_of_one`

### Calling Bell

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/CallingBell.java`

Rust source:
- `src/content/relics/calling_bell.rs`
- `src/engine/relic_manager.rs`
- `src/state/run.rs`

Java evidence:
- Constructor: ID `"Calling Bell"`, tier `BOSS`, landing sound `SOLID`.
- `onEquip`: presents Curse of the Bell through a confirmation grid.
- After confirmation, opens a reward screen containing exactly one common,
  one uncommon, and one rare `returnRandomScreenlessRelic` reward.

Rust result:
- Tier and on-equip route match Java.
- Curse of the Bell is obtained through the deck manager, preserving Omamori
  interception semantics from obtain effects.
- Fixed relic rewards to use `random_screenless_relic` for each tier instead of
  normal tier rolls that could return screen-interrupting relics.

Coverage:
- `shared_boss_relic_first_batch_metadata_matches_java_sources`
- `calling_bell_uses_screenless_relic_rewards_after_curse_obtain`

### Coffee Dripper

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/CoffeeDripper.java`

Rust source:
- `src/content/relics/mod.rs`
- `src/engine/campfire_handler.rs`

Java evidence:
- Constructor: ID `"Coffee Dripper"`, tier `BOSS`, landing sound `CLINK`.
- `onEquip`: increments `energyMaster`.
- `onUnequip`: decrements `energyMaster`.
- `canUseCampfireOption`: disables the normal Rest option.

Rust result:
- Tier and energy delta match Java.
- Campfire option generation omits Rest while Coffee Dripper is owned.

Coverage:
- `shared_boss_relic_first_batch_metadata_matches_java_sources`
- `boss_relic_passives_affect_rewards_campfires_and_curse_pool`

### Cursed Key

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/CursedKey.java`
- `D:/rust/cardcrawl/helpers/CardLibrary.java`

Rust source:
- `src/content/relics/mod.rs`
- `src/content/cards/mod.rs`
- `src/engine/run_loop.rs`

Java evidence:
- Constructor: ID `"Cursed Key"`, tier `BOSS`, landing sound `CLINK`.
- `onEquip`: increments `energyMaster`.
- `onUnequip`: decrements `energyMaster`.
- `onChestOpen(false)`: obtains `AbstractDungeon.returnRandomCurse()`.
- `CardLibrary.getCurse()` excludes `AscendersBane`, `Necronomicurse`,
  `CurseOfTheBell`, and `Pride`, and rolls with `cardRng`.
- Boss chests do not trigger the curse.

Rust result:
- Tier and energy delta match Java.
- Fixed random curse pool to exclude `Necronomicurse` and `Pride` in addition
  to Ascender's Bane and Curse of the Bell.
- Fixed non-boss chest curse roll to use `card_rng` and record the event source
  as Cursed Key.
- Omamori/Darkstone style obtain hooks still pass through the deck manager.

Coverage:
- `shared_boss_relic_first_batch_metadata_matches_java_sources`
- `boss_relic_passives_affect_rewards_campfires_and_curse_pool`

## Shared Relic Batch 14 - Boss Relics Part 2

### Ectoplasm

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/Ectoplasm.java`

Rust source:
- `src/content/relics/mod.rs`
- `src/state/run.rs`

Java evidence:
- Constructor: ID `"Ectoplasm"`, tier `BOSS`, landing sound `FLAT`.
- `onEquip`: increments `energyMaster`.
- `onUnequip`: decrements `energyMaster`.
- `canSpawn`: returns `AbstractDungeon.actNum <= 1`.
- Gold gain prevention is represented through the gold-gain path, not a
  combat hook.

Rust result:
- Tier and energy delta match Java.
- Fixed boss relic pool gating so Ectoplasm cannot spawn after Act 1.
- Gold gain path already blocks positive gold changes while owned.
- Removed the misleading empty combat hook module; Ectoplasm has no combat
  action hook.

Coverage:
- `shared_boss_relic_second_batch_metadata_matches_java_sources`
- `ectoplasm_can_spawn_only_in_act_one_and_blocks_gold_gain`

### Empty Cage

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/EmptyCage.java`
- `D:/rust/cardcrawl/cards/CardGroup.java`

Rust source:
- `src/content/relics/empty_cage.rs`
- `src/engine/run_loop.rs`

Java evidence:
- Constructor: ID `"Empty Cage"`, tier `BOSS`, landing sound `SOLID`.
- `onEquip`: candidates come from `masterDeck.getPurgeableCards()`.
- `CardGroup.getPurgeableCards()` excludes only `Necronomicurse`,
  `CurseOfTheBell`, and `AscendersBane`.
- If candidate count is `0`, no effect.
- If candidate count is `<= 2`, deletes all candidates immediately.
- If candidate count is `> 2`, opens a grid select for exactly `2`.

Rust result:
- Fixed candidate filtering to use Java purgeable semantics instead of full
  deck length.
- Fixed `<= 2` candidates to auto-delete without a pending choice.
- Pending choice for larger decks is exactly two purge picks.
- Run pending-choice candidate generation and execution reject Java's
  unpurgeable special curses for purge selections.
- Run pending-choice event source now records the owning relic for Empty Cage
  rather than generic `Selection(Purge)` when not inside an event.

Coverage:
- `shared_boss_relic_second_batch_metadata_matches_java_sources`
- `empty_cage_uses_java_purgeable_cards_and_auto_deletes_two_or_fewer`

### Fusion Hammer

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/FusionHammer.java`

Rust source:
- `src/content/relics/mod.rs`
- `src/engine/campfire_handler.rs`

Java evidence:
- Constructor: ID `"Fusion Hammer"`, tier `BOSS`, landing sound `HEAVY`.
- `onEquip`: increments `energyMaster`.
- `onUnequip`: decrements `energyMaster`.
- `canUseCampfireOption`: disables the normal `SmithOption`.

Rust result:
- Tier and energy delta match Java.
- Campfire option generation omits normal Smith while Fusion Hammer is owned.
- The Rust relic module now documents that it has no combat hook instead of
  carrying unused placeholder code.

Coverage:
- `shared_boss_relic_second_batch_metadata_matches_java_sources`
- `fusion_hammer_blocks_only_normal_smith_option`

### Pandora's Box

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/PandorasBox.java`

Rust source:
- `src/content/relics/pandoras_box.rs`
- `src/state/run.rs`

Java evidence:
- Constructor: ID `"Pandora's Box"`, tier `BOSS`, landing sound `MAGICAL`.
- `onEquip`: removes master-deck cards tagged `STARTER_STRIKE` or
  `STARTER_DEFEND`.
- For each removed card, rolls `AbstractDungeon.returnTrulyRandomCard()`,
  which uses `cardRandomRng` and does not exclude the removed card by transform
  identity.
- Calls each relic's `onPreviewObtainCard` before showing a confirmation grid.

Rust result:
- Starter Strike/Defend removal and replacement count match Java.
- Random replacement uses the card RNG and class card pools rather than the
  transform path.
- Fixed card removal and obtain events to record `Relic(PandorasBox)` as the
  source.
- Egg/Ceramic Fish/Omamori style obtain hooks continue through the deck manager.

Coverage:
- `shared_boss_relic_second_batch_metadata_matches_java_sources`
- `pandoras_box_replaces_only_starter_strike_defend_with_relic_source`

### Philosopher's Stone

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/PhilosopherStone.java`

Rust source:
- `src/content/relics/philosopher_stone.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Philosopher's Stone"`, tier `BOSS`, landing sound `CLINK`.
- `onEquip`: increments `energyMaster`.
- `onUnequip`: decrements `energyMaster`.
- `atBattleStart`: iterates every monster in
  `AbstractDungeon.getMonsters().monsters` and adds `StrengthPower(1)`.
- `onSpawnMonster`: adds `StrengthPower(1)` to the spawned monster.

Rust result:
- Tier and energy delta match Java.
- Fixed battle-start hook to iterate every monster in the group instead of
  adding Rust-only dying/escaped filters.
- Spawn hook already applies Strength to the spawned monster.

Coverage:
- `shared_boss_relic_second_batch_metadata_matches_java_sources`
- `philosopher_stone_strength_matches_java_battle_and_spawn_hooks`

### Runic Dome

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/RunicDome.java`

Rust source:
- `src/content/relics/mod.rs`
- `src/bot/combat/monster_belief.rs`

Java evidence:
- Constructor: ID `"Runic Dome"`, tier `BOSS`, landing sound `HEAVY`.
- `onEquip`: increments `energyMaster`.
- `onUnequip`: decrements `energyMaster`.
- The relic itself has no combat action hook; intent hiding is a public
  observation rule.

Rust result:
- Tier and energy delta match Java.
- No combat hook is registered.
- The current combat belief/observation path treats Runic Dome as hidden intent
  even when a protocol visible-intent cache exists.
- No Java UI/VFX model is copied into Rust.

Coverage:
- `shared_boss_relic_second_batch_metadata_matches_java_sources`
- `runic_dome_hides_public_intent_without_a_ui_model`

## Shared Relic Batch 15 - Boss Relics Part 3

### Sacred Bark

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/SacredBark.java`

Rust source:
- `src/content/relics/sacred_bark.rs`
- `src/engine/action_handlers/cards.rs`
- `src/engine/action_handlers/mod.rs`

Java evidence:
- Constructor: ID `"SacredBark"`, tier `BOSS`, landing sound `MAGICAL`.
- `onEquip`: reinitializes existing potion data so UI text/potency reflects
  the doubled potion effect.
- Gameplay effect is potion potency doubling.

Rust result:
- Tier matches Java.
- Potion effect resolution doubles potency while Sacred Bark is owned.
- No UI reinitialization state is carried; potency is derived at use time.

Coverage:
- `shared_boss_relic_third_batch_metadata_matches_java_sources`

### Slaver's Collar

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/SlaversCollar.java`

Rust source:
- `src/content/relics/slavers_collar.rs`
- `src/content/relics/mod.rs`
- `src/content/monsters/mod.rs`

Java evidence:
- Constructor: ID `"SlaversCollar"`, tier `BOSS`, landing sound `FLAT`.
- `beforeEnergyPrep`: starts from `eliteTrigger`, then scans every monster and
  activates if any monster has `EnemyType.BOSS`.
- If active, increments `player.energy.energyMaster` until victory.
- `onVictory`: decrements energy master only if it had pulsed/activated.

Rust result:
- Tier matches Java.
- Fixed activation to scan monster identity for boss enemies in addition to
  combat metadata flags.
- Fixed activation to add energy to the current first turn as well as
  `energy_master`, matching Java's pre-energy-prep timing.
- Restore/rebuild energy now uses the same elite/boss predicate.

Coverage:
- `shared_boss_relic_third_batch_metadata_matches_java_sources`
- `slavers_collar_uses_java_elite_or_boss_detection_and_affects_current_turn_energy`

### Snecko Eye

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/SneckoEye.java`

Rust source:
- `src/content/relics/snecko_eye.rs`
- `src/engine/action_handlers/cards.rs`
- `src/engine/core.rs`

Java evidence:
- Constructor: ID `"Snecko Eye"`, tier `BOSS`, landing sound `FLAT`.
- `onEquip`: increases `masterHandSize` by `2`.
- `onUnequip`: decreases `masterHandSize` by `2`.
- `atPreBattle`: applies `ConfusionPower` to the player.

Rust result:
- Tier and pre-battle subscription match Java.
- Initial and per-turn draw counts add two cards while Snecko Eye is owned.
- Pre-battle hook applies Confusion to the player.

Coverage:
- `shared_boss_relic_third_batch_metadata_matches_java_sources`

### Sozu

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/Sozu.java`

Rust source:
- `src/content/relics/mod.rs`
- `src/rewards/generator.rs`
- `src/rewards/handler.rs`
- `src/engine/action_handlers/cards.rs`
- `src/engine/shop_handler.rs`

Java evidence:
- Constructor: ID `"Sozu"`, tier `BOSS`, landing sound `FLAT`.
- `onEquip`: increments `energyMaster`.
- `onUnequip`: decrements `energyMaster`.
- Potion prevention is handled by potion-obtain paths, not by a combat action.

Rust result:
- Tier and energy delta match Java.
- Combat and reward potion obtain paths respect Sozu as a prevention/absorb
  rule.
- Shop potion purchase is stricter: Java `StorePotion.purchasePotion()` returns
  immediately under Sozu before spending gold, attempting obtain, or triggering
  Courier restock. Rust now treats shop potion buys under Sozu as blocked
  no-ops rather than absorbed purchases.

Coverage:
- `shared_boss_relic_third_batch_metadata_matches_java_sources`
- `white_beast_statue_forces_potion_reward_unless_sozu_blocks_potions`
- `sozu_shop_potion_purchase_is_blocked_without_spending_or_removing_offer`
- `courier_does_not_refill_sozu_blocked_shop_potion_purchase`

### Tiny House

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/TinyHouse.java`
- `D:/rust/cardcrawl/screens/CombatRewardScreen.java`

Rust source:
- `src/content/relics/tiny_house.rs`
- `src/rewards/generator.rs`

Java evidence:
- Constructor: ID `"Tiny House"`, tier `BOSS`, landing sound `FLAT`.
- `onEquip`: shuffles all upgradable master-deck cards with
  `new Random(AbstractDungeon.miscRng.randomLong())` and upgrades one if any.
- Increases max HP by `5`.
- Adds `50` gold and one random potion to the current room rewards. The potion
  uses `PotionHelper.getRandomPotion(AbstractDungeon.miscRng)`, which samples the
  initialized potion list directly rather than rolling drop rarity first.
- Opens the combat reward screen, whose setup also provides the normal card
  reward unless the current room suppresses card rewards.

Rust result:
- Fixed random upgrade to use the normal upgrade event/source path.
- Fixed max HP gain to emit `Relic(TinyHouse)` source.
- Fixed gold, potion, and card reward handling to return a `RewardScreen`
  instead of directly adding gold and omitting potion/card rewards.
- Fixed the potion reward to use the uniform PotionHelper-style pool with
  `misc_rng`, matching the Java overload used by Tiny House.
- Gold is now gained only when the reward is claimed, preserving Ectoplasm and
  other reward-claim modifiers.

Coverage:
- `shared_boss_relic_third_batch_metadata_matches_java_sources`
- `tiny_house_uses_reward_screen_for_gold_potion_and_card_reward`

### Velvet Choker

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/VelvetChoker.java`

Rust source:
- `src/content/relics/mod.rs`
- `src/engine/action_handlers/cards.rs`
- `src/bot/combat/legal_moves.rs`

Java evidence:
- Constructor: ID `"Velvet Choker"`, tier `BOSS`, landing sound `FLAT`.
- `onEquip`: increments `energyMaster`.
- `onUnequip`: decrements `energyMaster`.
- `atBattleStart`/`atTurnStart`: counter resets to `0`.
- `onPlayCard`: increments counter up to the limit.
- `canPlay`: rejects cards once counter reaches `6`.

Rust result:
- Tier and energy delta match Java.
- The engine uses the canonical per-turn cards-played counter for the same
  six-card limit.
- Legal move generation mirrors the same limit.

Coverage:
- `shared_boss_relic_third_batch_metadata_matches_java_sources`

### Wrist Blade

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/WristBlade.java`

Rust source:
- `src/content/relics/wrist_blade.rs`
- `src/content/relics/hooks.rs`
- `src/content/cards/runtime_impl.rs`

Java evidence:
- Constructor: ID `"WristBlade"`, tier `BOSS`, landing sound `FLAT`.
- `atDamageModify`: if `card.costForTurn == 0`, or if
  `card.freeToPlayOnce && card.cost != -1`, returns `damage + 4.0f`.

Rust result:
- Fixed damage bonus from the old `+3` placeholder to Java's `+4`.
- Fixed hook dispatch so Wrist Blade actually participates in card damage
  calculation.
- Preserved Java's X-cost exclusion for free-to-play-once cards.

Coverage:
- `shared_boss_relic_third_batch_metadata_matches_java_sources`
- `wrist_blade_adds_four_damage_to_java_zero_cost_attacks_only`

## Shared Relic Batch 16 - Shop / Special Gaps Part 1

### Abacus

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/Abacus.java`

Rust source:
- `src/content/relics/abacus.rs`
- `src/content/relics/hooks.rs`
- `src/engine/action_handlers/cards.rs`

Java evidence:
- Constructor: ID `"TheAbacus"`, tier `SHOP`, landing sound `SOLID`.
- `onShuffle`: flashes, queues relic-above-player VFX, then queues
  `GainBlockAction(player, player, 6)`.

Rust result:
- Tier and shuffle subscription match Java.
- Shuffle hook queues 6 block. UI-only relic-above-player VFX is not modeled.

Coverage:
- `shared_shop_special_relic_gap_batch_metadata_matches_java_sources`
- `abacus_grants_six_block_on_shuffle`

### Bloody Idol

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/BloodyIdol.java`
- `D:/rust/cardcrawl/characters/AbstractPlayer.java`

Rust source:
- `src/content/relics/bloody_idol.rs`
- `src/engine/action_handlers/damage.rs`
- `src/state/run.rs`

Java evidence:
- Constructor: ID `"Bloody Idol"`, tier `SPECIAL`, landing sound `HEAVY`.
- `AbstractPlayer.gainGold`: Ectoplasm blocks positive gold before it changes
  gold; otherwise positive gold increments gold and then calls every relic's
  `onGainGold`.
- `BloodyIdol.onGainGold`: heals 5.

Rust result:
- Tier matches Java.
- Combat `GainGold` already triggered Bloody Idol through the combat action
  path.
- Fixed run/reward/event-level positive gold changes to heal 5 through
  `Relic(BloodyIdol)` after gold is actually gained.
- Ectoplasm-blocked gold gain does not trigger the heal.

Coverage:
- `shared_shop_special_relic_gap_batch_metadata_matches_java_sources`
- `bloody_idol_heals_from_run_level_gold_gain_unless_ectoplasm_blocks_gold`

### Cauldron

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/Cauldron.java`
- `D:/rust/cardcrawl/helpers/PotionHelper.java`
- `D:/rust/cardcrawl/rooms/AbstractRoom.java`

Rust source:
- `src/content/relics/cauldron.rs`
- `src/engine/relic_manager.rs`

Java evidence:
- Constructor: ID `"Cauldron"`, tier `SHOP`, landing sound `HEAVY`.
- `onEquip`: adds five potion rewards with `PotionHelper.getRandomPotion()`,
  opens the combat reward screen, and removes the first card reward if one is
  present.
- `PotionHelper.getRandomPotion()` samples uniformly from the initialized
  potion list through `AbstractDungeon.potionRng`; it is not the weighted
  reward-drop potion roll.

Rust result:
- Tier matches Java.
- Fixed Cauldron to route through the relic manager on acquire.
- Fixed Cauldron to return a reward screen containing five potion rewards,
  preserving existing non-card rewards and removing the first card reward.
- Fixed potion generation to use the uniform PotionHelper-style pool with
  `potion_rng`.

Coverage:
- `shared_shop_special_relic_gap_batch_metadata_matches_java_sources`
- `cauldron_opens_potion_reward_screen_and_removes_first_card_reward`

### Chemical X

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/ChemicalX.java`

Rust source:
- `src/content/relics/chemical_x.rs`
- `src/content/relics/hooks.rs`
- `src/engine/action_handlers/cards.rs`

Java evidence:
- Constructor: ID `"Chemical X"`, tier `SHOP`, landing sound `CLINK`.
- Relic class stores `BOOST = 2`; X-card action code consumes this as an
  additional X amount.

Rust result:
- Tier and X-cost subscription match Java.
- X-cost calculation adds 2 when Chemical X is owned.

Coverage:
- `shared_shop_special_relic_gap_batch_metadata_matches_java_sources`
- `chemical_x_adds_two_to_x_cost_amount`

### Circlet

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/Circlet.java`
- `D:/rust/cardcrawl/relics/AbstractRelic.java`

Rust source:
- `src/content/relics/mod.rs`
- `src/state/run.rs`

Java evidence:
- Constructor: ID `"Circlet"`, tier `SPECIAL`, landing sound `CLINK`, and
  `counter = 1`.
- `AbstractRelic.instantObtain` / `obtain`: if the player already has Circlet,
  increments the existing Circlet counter instead of adding another relic copy.

Rust result:
- Fixed new Circlets to start with counter 1.
- Fixed run relic acquisition so duplicate Circlets increment the existing
  Circlet counter and do not add a second relic instance.

Coverage:
- `shared_shop_special_relic_gap_batch_metadata_matches_java_sources`
- `circlet_duplicate_obtain_increments_existing_counter_instead_of_adding_copy`

### Red Circlet

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/RedCirclet.java`
- `D:/rust/cardcrawl/dungeons/AbstractDungeon.java`

Rust source:
- `src/content/relics/mod.rs`
- `src/content/relics/red_circlet.rs`
- `src/state/relic_pool.rs`
- `tools/protocol_schema_baseline.json`
- `tools/compiled_protocol_schema.json`

Java evidence:
- Constructor: ID `"Red Circlet"`, tier `SPECIAL`, landing sound `CLINK`.
- Unlike `Circlet`, the constructor does not set `counter = 1`.
- `AbstractDungeon.returnRandomRelicKey(BOSS)` and
  `returnEndRandomRelicKey(BOSS)` return `"Red Circlet"` when
  `bossRelicPool` is empty. Empty rare pools return `"Circlet"` instead.

Rust result:
- Added `RelicId::RedCirclet` as a distinct special relic with no hooks and
  default counter `-1`.
- Fixed front/end boss relic pool fallback to return `RedCirclet` instead of
  `Circlet`.
- Mapped Java protocol ID `"Red Circlet"` to the Rust relic ID.

Coverage:
- `shared_shop_special_relic_gap_batch_metadata_matches_java_sources`
- `empty_boss_relic_pool_returns_red_circlet_like_java_sources`
- `relic_id_from_java_maps_boss_pool_fallback_red_circlet`

### DarkBlood Rust Stub

Status: `removed`

Java source:
- `D:/rust/cardcrawl/relics/BlackBlood.java`
- `D:/rust/cardcrawl/relics/BurningBlood.java`

Rust source:
- `src/content/relics/mod.rs`
- `src/content/relics/hooks.rs`
- deleted `src/content/relics/dark_blood.rs`

Java evidence:
- The Java relic directory contains `BlackBlood.java` and `BurningBlood.java`;
  there is no `DarkBlood.java`.
- `Black Blood` is the Ironclad boss upgrade relic and heals 12 on victory.

Rust result:
- Removed the non-Java `RelicId::DarkBlood` content stub and victory hook.
- Kept the Java-backed `RelicId::BlackBlood` and `RelicId::BurningBlood`
  implementations.

### Dodecahedron

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/deprecated/DEPRECATEDDodecahedron.java`

Rust source:
- `src/content/relics/dodecahedron.rs`
- `src/content/relics/hooks.rs`
- `src/content/relics/mod.rs`

Java evidence:
- The only Java class for ID `"Dodecahedron"` is
  `DEPRECATEDDodecahedron`, under `relics/deprecated`.
- `atBattleStart`, `onVictory`, `onPlayerHeal`, and `onAttacked` only control
  relic pulse UI.
- `atTurnStart` queues a deferred action; when it updates, if
  `currentHealth >= maxHealth`, it queues `GainEnergyAction(1)`.

Rust result:
- Marked Dodecahedron as `Deprecated`, keeping it out of normal relic pools
  while preserving the Java-backed ID.
- Removed the incorrect battle-start energy hook.
- Registered the turn-start hook and emits one bottom-queued
  `GainEnergy { amount: 1 }` only when player HP is at least max HP.
- UI-only pulse/flash/above-creature actions are intentionally not modeled.

Coverage:
- `deprecated_dodecahedron_triggers_energy_at_turn_start_only_like_java_source`

### Discerning Monocle

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/DiscerningMonocle.java`

Rust source:
- `src/content/relics/discerning_monocle.rs`
- `src/content/relics/mod.rs`

Java evidence:
- Constructor: ID `"Discerning Monocle"`, tier `UNCOMMON`, landing sound
  `CLINK`.
- `RelicLibrary.initialize()` never calls `RelicLibrary.add(new
  DiscerningMonocle())`, so the class exists in source but is not registered in
  normal vanilla relic pools.
- `onEnterRoom`: pulses only in `ShopRoom`; otherwise stops pulsing.
- Although the class defines `MULTIPLIER = 0.8f`, this vanilla Java source tree
  does not reference `DiscerningMonocle` in `ShopScreen` price calculation.

Rust result:
- Fixed tier from the old shop-tier assumption to Java's `UNCOMMON`.
- Split constructor tier from pool registration: Rust no longer includes
  `DiscerningMonocle` in the RelicLibrary-registered pool list.
- Removed the misleading headless comment that claimed a shop discount.
- No UI-only shop pulse is modeled.

Coverage:
- `shared_shop_special_relic_gap_batch_metadata_matches_java_sources`

## Shared Relic Batch 17 - Event / Special Gaps Part 1

### Dolly's Mirror

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/DollysMirror.java`
- `D:/rust/cardcrawl/cards/AbstractCard.java`
- `D:/rust/cardcrawl/vfx/cardManip/ShowCardAndObtainEffect.java`

Rust source:
- `src/content/relics/dollys_mirror.rs`
- `src/engine/run_loop.rs`
- `src/state/run.rs`

Java evidence:
- Constructor: ID `"DollysMirror"`, tier `SHOP`, landing sound `SOLID`.
- `onEquip`: opens a one-card master deck grid selection.
- `update`: when one card is selected, calls `makeStatEquivalentCopy()`, clears
  bottle flags on the copy, and obtains the copied card through
  `ShowCardAndObtainEffect`.
- `makeStatEquivalentCopy()` preserves upgrades and persistent card state such
  as `misc`.

Rust result:
- Tier and deck-selection interrupt match Java.
- Fixed duplicate resolution to add a stat-equivalent copy of the selected
  master-deck instance instead of only copying `(card_id, upgrades)`.
- Persistent `misc` and card stat overrides are preserved on the duplicate.
- Bottle attachment stays on the original card UUID; the copied card receives a
  new UUID.

Coverage:
- `shared_event_special_relic_gap_batch_metadata_matches_java_sources`
- `dollys_mirror_opens_duplicate_selection_when_deck_has_cards`
- `duplicate_selection_preserves_stat_equivalent_card_state_without_copying_bottle_attachment`

### Enchiridion

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/Enchiridion.java`
- `D:/rust/cardcrawl/dungeons/AbstractDungeon.java`

Rust source:
- `src/content/relics/enchiridion.rs`
- `src/content/relics/hooks.rs`
- `src/engine/action_handlers/cards.rs`

Java evidence:
- Constructor: ID `"Enchiridion"`, tier `SPECIAL`, landing sound `FLAT`.
- `atPreBattle`: chooses `returnTrulyRandomCardInCombat(CardType.POWER)`,
  copies it, sets cost for turn to zero if the base cost is not X-cost, and
  adds it to hand.

Rust result:
- Tier and pre-battle subscription match Java.
- Pre-battle hook queues a random generated Power card with zero cost for turn.
- UI unlock/mark-seen effects are not modeled.

Coverage:
- `shared_event_special_relic_gap_batch_metadata_matches_java_sources`
- `enchiridion_adds_random_zero_cost_power_at_pre_battle`

### Face of Cleric

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/FaceOfCleric.java`

Rust source:
- `src/content/relics/face_of_cleric.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"FaceOfCleric"`, tier `SPECIAL`, landing sound `CLINK`.
- `onVictory`: increases max HP by 1.

Rust result:
- Tier and victory subscription match Java.
- Victory hook queues `GainMaxHp { amount: 1 }`.

Coverage:
- `shared_event_special_relic_gap_batch_metadata_matches_java_sources`
- `face_of_cleric_gains_one_max_hp_on_victory`

### Gremlin Mask

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/GremlinMask.java`

Rust source:
- `src/content/relics/gremlin_mask.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"GremlinMask"`, tier `SPECIAL`, landing sound `CLINK`.
- `atBattleStart`: queues UI relic VFX and applies 1 Weak to the player.

Rust result:
- Tier and battle-start subscription match Java.
- Battle-start hook applies 1 Weak to the player.
- UI-only relic-above-player VFX is not modeled.

Coverage:
- `shared_event_special_relic_gap_batch_metadata_matches_java_sources`
- `gremlin_mask_applies_one_weak_to_player_at_battle_start`

## Shared Relic Batch 18 - Shop Relics Part 1

### Membership Card

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/MembershipCard.java`
- `D:/rust/cardcrawl/shop/ShopScreen.java`

Rust source:
- `src/engine/shop_handler.rs`

Java evidence:
- Constructor: ID `"Membership Card"`, tier `SHOP`, landing sound `MAGICAL`.
- `onEnterRoom`: shop pulse only.
- `ShopScreen` applies the 0.5 price multiplier to shop inventory and purge
  cost. Buying Membership Card discounts remaining shop inventory.

Rust result:
- Tier matches Java.
- Shop pricing and post-purchase repricing apply the 0.5 multiplier.
- UI-only shop pulse is not modeled.

Coverage:
- `shared_shop_relic_gap_batch_two_metadata_matches_java_sources`
- `membership_card_purchase_discounts_remaining_shop_inventory`

### Orrery

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/Orrery.java`
- `D:/rust/cardcrawl/screens/CombatRewardScreen.java`

Rust source:
- `src/content/relics/orrery.rs`
- `src/engine/relic_manager.rs`

Java evidence:
- Constructor: ID `"Orrery"`, tier `SHOP`, landing sound `CLINK`.
- `onEquip`: adds four card rewards to the current room, then opens the combat
  reward screen.
- `CombatRewardScreen.open(String)` calls `setupItemReward()`, which adds the
  normal card reward unless the room suppresses card rewards. This yields five
  card rewards in the normal case.

Rust result:
- Tier and relic-manager route match Java.
- On equip opens a reward screen with five card rewards, and those card rewards
  pass through existing card reward modifiers such as Question Card.

Coverage:
- `shared_shop_relic_gap_batch_two_metadata_matches_java_sources`
- `orrery_card_rewards_respect_question_card`

### Medical Kit

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/MedicalKit.java`

Rust source:
- `src/content/relics/medical_kit.rs`
- `src/content/cards/runtime_impl.rs`
- `src/engine/action_handlers/cards.rs`

Java evidence:
- Constructor: ID `"Medical Kit"`, tier `SHOP`, landing sound `MAGICAL`.
- `onUseCard`: when the played card is `STATUS`, sets both card and use action
  exhaust flags.
- The playable-status behavior is represented by card playability checks.

Rust result:
- Tier and on-use subscription match Java.
- Status cards become playable when Medical Kit is owned.
- Status cards played with Medical Kit are exhausted by the card play cleanup
  path.

Coverage:
- `shared_shop_relic_gap_batch_two_metadata_matches_java_sources`
- `medical_kit_allows_status_cards_to_be_played`

### Orange Pellets

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/OrangePellets.java`
- `D:/rust/cardcrawl/actions/unique/RemoveDebuffsAction.java`

Rust source:
- `src/content/relics/orange_pellets.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"OrangePellets"`, tier `SHOP`, landing sound `CLINK`.
- Tracks whether an Attack, Skill, and Power have been played this turn.
- Once all three are seen, queues `RemoveDebuffsAction(player)` and resets its
  three flags.

Rust result:
- Tier and on-use subscription match Java.
- Fixed the cleanup action to use `RemoveAllDebuffs` instead of only removing
  Weak, Vulnerable, and Frail.
- Existing turn-start reset clears the combo counter each turn.

Coverage:
- `shared_shop_relic_gap_batch_two_metadata_matches_java_sources`
- `orange_pellets_uses_remove_all_debuffs_action_and_resets_combo_counter`

### Sling

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/Sling.java`

Rust source:
- `src/content/relics/sling.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Sling"`, tier `SHOP`, landing sound `CLINK`.
- `atBattleStart`: if current room `eliteTrigger` is true, adds 2 Strength to
  the player. It does nothing in hallway or boss combats.

Rust result:
- Tier and battle-start subscription match Java.
- Fixed Sling to grant Strength only when `CombatMeta.is_elite_fight` is true.

Coverage:
- `shared_shop_relic_gap_batch_two_metadata_matches_java_sources`
- `sling_only_grants_strength_in_elite_combats`

## Shared Relic Batch 19 - Defect Orb Relic Grounding

### Cloak Clasp

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/CloakClasp.java`

Rust source:
- `src/content/relics/cloak_clasp.rs`
- `src/engine/action_handlers/cards.rs`

Java evidence:
- Constructor: ID `"CloakClasp"`, tier `RARE`, landing sound `CLINK`.
- `onPlayerEndTurn`: if the hand is not empty, adds `GainBlockAction` for
  `player.hand.group.size()`.

Rust result:
- Tier and end-turn subscription match Java.
- The combat end-turn marker now owns the end-turn relic expansion, preventing a
  duplicate Cloak Clasp trigger from the previous split path.

Coverage:
- `defect_orb_relic_gap_batch_metadata_matches_java_sources`
- `end_turn_marker_triggers_cloak_clasp_once`

### Cracked Core

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/CrackedCore.java`

Rust source:
- `src/content/relics/cracked_core.rs`
- `src/content/orbs/hooks.rs`

Java evidence:
- Constructor: ID `"Cracked Core"`, tier `STARTER`, landing sound `CLINK`.
- `atPreBattle`: directly channels one `Lightning` orb.

Rust result:
- Tier and pre-battle subscription match Java.
- Fixed the pre-battle hook from an empty placeholder to `ChannelOrb(Lightning)`.
- Basic orb passive execution now has a real marker/action-handler path.
- Full-slot channeling now evokes the oldest orb before channeling the new orb,
  matching `AbstractPlayer.channelOrb`.

Coverage:
- `defect_orb_relic_gap_batch_metadata_matches_java_sources`
- `cracked_core_channels_lightning_from_pre_battle_hook`
- `channeling_into_full_orb_slots_evokes_oldest_before_channeling_new_orb`

### Damaru

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/Damaru.java`

Rust source:
- `src/content/relics/damaru.rs`

Java evidence:
- Constructor: ID `"Damaru"`, tier `COMMON`, landing sound `SOLID`.
- `atTurnStart`: queues `ApplyPowerAction(... MantraPower(player, 1), 1)`.
- `update` only plays a sound and flashes when clicked; this is UI-only.

Rust result:
- Tier and turn-start subscription match Java.
- Turn-start hook grants one Mantra and omits click/sound UI behavior.

Coverage:
- `defect_orb_relic_gap_batch_metadata_matches_java_sources`

### Data Disk

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/DataDisk.java`

Rust source:
- `src/content/relics/data_disk.rs`
- `src/content/orbs/hooks.rs`
- `src/engine/action_handlers/mod.rs`

Java evidence:
- Constructor: ID `"DataDisk"`, tier `COMMON`, landing sound `FLAT`.
- `atBattleStart`: adds `FocusPower(player, 1)` to the top of the action queue.

Rust result:
- Tier and battle-start subscription match Java.
- Focus is applied as a power.
- Orb passive/evoke amount refresh now reads current Focus so Data Disk affects
  Lightning/Frost/Dark orb behavior instead of being disconnected from orbs.

Coverage:
- `defect_orb_relic_gap_batch_metadata_matches_java_sources`
- `data_disk_focus_changes_orb_passive_amounts`

### Gold-Plated Cables

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/GoldPlatedCables.java`
- `D:/rust/cardcrawl/actions/defect/TriggerEndOfTurnOrbsAction.java`
- `D:/rust/cardcrawl/characters/AbstractPlayer.java`
- `D:/rust/cardcrawl/actions/defect/ImpulseAction.java`

Rust source:
- `src/content/orbs/hooks.rs`
- `src/content/relics/mod.rs`

Java evidence:
- Constructor: ID `"Cables"`, tier `UNCOMMON`, landing sound `FLAT`.
- The relic class has no callback method. Orb trigger code checks whether the
  player has `"Cables"` and then triggers the first orb one additional time.
- The extra trigger applies to end-of-turn orbs, start-of-turn orbs, and
  `ImpulseAction`.

Rust result:
- Tier matches Java.
- Removed the false relic end-turn subscription and stale standalone relic hook.
- Implemented the extra first-orb trigger inside the orb trigger path, matching
  the Java ownership boundary.

Coverage:
- `defect_orb_relic_gap_batch_metadata_matches_java_sources`
- `orb_passives_fire_from_marker_actions_and_gold_plated_cables_doubles_first_orb`

### Emotion Chip

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/EmotionChip.java`
- `D:/rust/cardcrawl/actions/defect/ImpulseAction.java`

Rust source:
- `src/content/relics/emotion_chip.rs`
- `src/content/orbs/hooks.rs`

Java evidence:
- `wasHPLost`: during combat, positive HP loss sets the relic pulse flag.
- `atTurnStart`: if pulsing, resets the pulse and queues `ImpulseAction`.
- `ImpulseAction`: for each orb, triggers start-of-turn and end-of-turn orb
  behavior, then applies Gold-Plated Cables to the first orb if present.
- `onVictory`: clears the pulse flag.

Rust result:
- Tier and subscriptions match Java.
- Fixed the queued action from an ambiguous/unhandled passive marker to
  `TriggerImpulseOrbs`.
- The marker now executes Plasma start passives, Frost/Lightning/Dark end
  passives, and the Gold-Plated Cables extra first-orb behavior.

Coverage:
- `defect_orb_relic_gap_batch_metadata_matches_java_sources`
- `emotion_chip_impulse_triggers_start_and_end_orb_passives_then_resets_counter`

### Frozen Core

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/FrozenCore.java`

Rust source:
- `src/content/relics/frozen_core.rs`

Java evidence:
- Constructor: ID `"FrozenCore"`, tier `BOSS`, landing sound `CLINK`.
- `onPlayerEndTurn`: if the player has any empty orb slot, channels one `Frost`.
- `canSpawn`: requires `Cracked Core`.

Rust result:
- Tier and end-turn subscription match Java.
- End-turn hook checks for any empty orb slot and channels Frost only then.
- Spawn-condition audit is still tracked at reward/relic-pool level; combat hook
  behavior is exact.

Coverage:
- `defect_orb_relic_gap_batch_metadata_matches_java_sources`
- `frozen_core_channels_frost_only_when_an_orb_slot_is_empty`

### Hand Drill

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/HandDrill.java`

Rust source:
- `src/content/relics/hand_drill.rs`
- `src/engine/action_handlers/damage.rs`

Java evidence:
- Constructor: ID `"HandDrill"`, tier `SHOP`, landing sound `FLAT`.
- `onBlockBroken(AbstractCreature m)`: queues Vulnerable 2 on the block-broken
  creature.

Rust result:
- Tier matches Java.
- Damage pipeline detects monster block breaking and queues Vulnerable 2 through
  the Hand Drill hook.

Coverage:
- `defect_orb_relic_gap_batch_metadata_matches_java_sources`
- `hand_drill_applies_vulnerable_when_damage_exactly_breaks_block`

## Shared Relic Batch 20 - Watcher Relic Grounding

### Pure Water

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/PureWater.java`

Rust source:
- `src/content/relics/pure_water.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"PureWater"`, tier `STARTER`, landing sound `MAGICAL`.
- `atBattleStartPreDraw`: queues one `Miracle` into hand.

Rust result:
- Tier and pre-draw battle-start subscription match Java.
- Hook queues one unupgraded Miracle.
- Relic-above-creature VFX is UI-only and is not modeled.

Coverage:
- `watcher_relic_gap_batch_metadata_matches_java_sources`
- `pure_and_holy_water_add_correct_miracle_counts_pre_draw`

### Holy Water

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/HolyWater.java`

Rust source:
- `src/content/relics/holy_water.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"HolyWater"`, tier `BOSS`, landing sound `MAGICAL`.
- `atBattleStartPreDraw`: queues three `Miracle` cards into hand.
- `canSpawn`: requires `PureWater`.

Rust result:
- Tier and pre-draw battle-start subscription match Java.
- Hook queues three unupgraded Miracles.
- Spawn-condition audit is still tracked at reward/relic-pool level; combat hook
  behavior is exact.

Coverage:
- `watcher_relic_gap_batch_metadata_matches_java_sources`
- `pure_and_holy_water_add_correct_miracle_counts_pre_draw`

### Duality

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/Duality.java`

Rust source:
- `src/content/relics/duality.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Yang"`, tier `UNCOMMON`, landing sound `MAGICAL`.
- `onUseCard`: if the used card is an Attack, queues Dexterity +1 and
  `LoseDexterityPower` 1.

Rust result:
- Tier and on-use subscription match Java.
- Attack cards queue Dexterity and DexterityDown; non-attacks do nothing.

Coverage:
- `watcher_relic_gap_batch_metadata_matches_java_sources`
- `duality_grants_temporary_dexterity_only_for_attack_cards`

### Golden Eye

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/GoldenEye.java`

Rust source:
- `src/content/relics/golden_eye.rs`
- `src/content/relics/hooks.rs`
- `src/engine/core.rs`

Java evidence:
- Constructor: ID `"GoldenEye"`, tier `RARE`, landing sound `HEAVY`.
- Relic class has no explicit callback; scry amount is affected by the scry
  pipeline.

Rust result:
- Tier and scry subscription match Java intent.
- The scry hook adds two to the requested scry amount before the pending-choice
  frame is created.

Coverage:
- `watcher_relic_gap_batch_metadata_matches_java_sources`
- `golden_eye_adds_two_to_scry_amount_and_melange_queues_scry_three_on_shuffle`

### Melange

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/Melange.java`

Rust source:
- `src/content/relics/melange.rs`
- `src/content/relics/hooks.rs`
- `src/engine/action_handlers/cards.rs`

Java evidence:
- Constructor: ID `"Melange"`, tier `SHOP`, landing sound `MAGICAL`.
- `onShuffle`: queues `ScryAction(3)`.

Rust result:
- Tier and shuffle subscription match Java.
- Shuffle hook queues `Scry(3)`.
- Relic-above-creature VFX is UI-only and is not modeled.

Coverage:
- `watcher_relic_gap_batch_metadata_matches_java_sources`
- `golden_eye_adds_two_to_scry_amount_and_melange_queues_scry_three_on_shuffle`

## Shared Relic Batch 21 - Silent Relic Grounding

### Ring of the Snake

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/SnakeRing.java`

Rust source:
- `src/content/relics/snake_ring.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Ring of the Snake"`, tier `STARTER`, landing sound `FLAT`.
- `atBattleStart`: queues `DrawCardAction(..., 2)`.

Rust result:
- Tier and battle-start subscription match Java.
- Battle-start hook queues `DrawCards(2)`.
- Relic-above-creature VFX is UI-only and is not modeled.

Coverage:
- `silent_relic_gap_batch_metadata_matches_java_sources`
- `snake_ring_and_ninja_scroll_start_actions_match_java_counts`

### Ring of the Serpent

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/RingOfTheSerpent.java`

Rust source:
- `src/runtime/combat.rs`
- `src/engine/core.rs`
- `src/engine/action_handlers/cards.rs`

Java evidence:
- Constructor: ID `"Ring of the Serpent"`, tier `BOSS`, landing sound `CLINK`.
- `onEquip`: increments `AbstractDungeon.player.masterHandSize`.
- `onUnequip`: decrements `masterHandSize`.
- `atTurnStart`: only flashes.
- `canSpawn`: requires `"Ring of the Snake"`.

Rust result:
- Tier matches Java.
- Fixed the passive hand-size effect by including Ring of the Serpent in the
  derived turn-start draw modifier.
- Opening combat draw now uses the same draw-count helper as ordinary turn-start
  draw, so the +1 applies to the opening hand as well.
- Java flash-only `atTurnStart` is UI-only and is not modeled.

Coverage:
- `silent_relic_gap_batch_metadata_matches_java_sources`
- `ring_of_the_serpent_increases_opening_and_turn_start_draw_count`

### Ninja Scroll

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/NinjaScroll.java`

Rust source:
- `src/content/relics/ninja_scroll.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Ninja Scroll"`, tier `UNCOMMON`, landing sound `FLAT`.
- `atBattleStartPreDraw`: queues three `Shiv` cards into hand.

Rust result:
- Tier and pre-draw battle-start subscription match Java.
- Hook queues three unupgraded Shivs.
- Relic-above-creature VFX is UI-only and is not modeled.

Coverage:
- `silent_relic_gap_batch_metadata_matches_java_sources`
- `snake_ring_and_ninja_scroll_start_actions_match_java_counts`

### Hovering Kite

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/HoveringKite.java`

Rust source:
- `src/content/relics/hovering_kite.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"HoveringKite"`, tier `BOSS`, landing sound `MAGICAL`.
- `atTurnStart`: clears `triggeredThisTurn`.
- `onManualDiscard`: the first manual discard each turn queues one energy.

Rust result:
- Tier and subscriptions match Java.
- Relic state uses `used_up` as the per-turn fired flag.
- First discard queues one energy, later discards that turn do nothing, and
  turn-start resets the flag.

Coverage:
- `silent_relic_gap_batch_metadata_matches_java_sources`
- `hovering_kite_gains_energy_only_on_first_manual_discard_each_turn`

### Snecko Skull

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/SneckoSkull.java`

Rust source:
- `src/engine/action_handlers/powers.rs`

Java evidence:
- Constructor: ID `"Snake Skull"`, tier `COMMON`, landing sound `FLAT`.
- Relic class has no callback method; poison amount mutation is handled by power
  application logic.

Rust result:
- Tier matches Java.
- Player-authored Poison applied to a monster is increased by one.
- Poison applied to the player is not increased.

Coverage:
- `silent_relic_gap_batch_metadata_matches_java_sources`
- `snecko_skull_adds_one_poison_only_when_player_applies_poison_to_monster`

### Paper Crane

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/PaperCrane.java`
- `D:/rust/cardcrawl/powers/WeakPower.java`

Rust source:
- `src/content/relics/paper_crane.rs`
- `src/content/powers/mod.rs`

Java evidence:
- Constructor: ID `"Paper Crane"`, tier `UNCOMMON`, landing sound `FLAT`.
- `WeakPower.atDamageGive`: for non-player Weak owners, if the player has
  Paper Crane, normal attack damage is multiplied by `0.6` instead of `0.75`.

Rust result:
- Tier matches Java.
- Fixed the monster-damage pipeline to use the Paper Crane multiplier when a
  Weak monster attacks the player.

Coverage:
- `silent_relic_gap_batch_metadata_matches_java_sources`
- `paper_crane_changes_weak_monster_damage_from_75_to_60_percent`

## Shared Relic Batch 22 - Defect Orb Slot Relic Grounding

### Nuclear Battery

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/NuclearBattery.java`

Rust source:
- `src/content/relics/nuclear_battery.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Nuclear Battery"`, tier `BOSS`, landing sound `HEAVY`.
- `atPreBattle`: directly channels one `Plasma`.

Rust result:
- Tier and pre-battle subscription match Java.
- Hook queues `ChannelOrb(Plasma)`.

Coverage:
- `defect_orb_slot_relic_gap_batch_metadata_matches_java_sources`
- `nuclear_battery_and_symbiotic_virus_channel_expected_orbs_pre_battle`

### Symbiotic Virus

Status: `exact`

Java source:
- `D:/rust/cardcrawl/relics/SymbioticVirus.java`

Rust source:
- `src/content/relics/symbiotic_virus.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Symbiotic Virus"`, tier `UNCOMMON`, landing sound
  `MAGICAL`.
- `atPreBattle`: directly channels one `Dark`.

Rust result:
- Tier and pre-battle subscription match Java.
- Hook queues `ChannelOrb(Dark)`.

Coverage:
- `defect_orb_slot_relic_gap_batch_metadata_matches_java_sources`
- `nuclear_battery_and_symbiotic_virus_channel_expected_orbs_pre_battle`

### Runic Capacitor

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/RunicCapacitor.java`

Rust source:
- `src/content/relics/runic_capacitor.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Runic Capacitor"`, tier `SHOP`, landing sound `SOLID`.
- `atPreBattle`: sets `firstTurn = true`.
- `atTurnStart`: on the first turn only, queues `IncreaseMaxOrbAction(3)` and
  clears `firstTurn`.

Rust result:
- Tier matches Java.
- Fixed the previous pre-battle immediate slot increase. The relic now marks the
  first turn at pre-battle and queues `IncreaseMaxOrb(3)` from turn-start once.

Coverage:
- `defect_orb_slot_relic_gap_batch_metadata_matches_java_sources`
- `runic_capacitor_increases_orb_slots_on_first_turn_after_pre_battle_only`

### Inserter

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/relics/Inserter.java`

Rust source:
- `src/content/relics/inserter.rs`
- `src/content/relics/mod.rs`
- `src/content/relics/hooks.rs`

Java evidence:
- Constructor: ID `"Inserter"`, tier `BOSS`, landing sound `SOLID`.
- `onEquip`: sets counter to `0`.
- `atTurnStart`: increments the counter; every second turn, resets it to `0` and
  queues `IncreaseMaxOrbAction(1)`.

Rust result:
- Tier and turn-start subscription match Java.
- Fixed default relic state so Inserter starts with counter `0`.
- Turn-start hook already matches Java's every-second-turn orb slot increase.

Coverage:
- `defect_orb_slot_relic_gap_batch_metadata_matches_java_sources`
- `inserter_counter_starts_at_zero_and_adds_orb_slot_every_second_turn`

## Full Ironclad Class-Specific Relic Queue

Relics remain `unreviewed` until their Java file, Rust definition/subscription,
hook implementation, and supporting engine behavior have all been checked.

| # | Java relic file | Rust relic module | Status |
|---:|---|---|---|
| 1 | `BurningBlood.java` | `burning_blood.rs` | `wrong-fixed` |
| 2 | `BlackBlood.java` | `black_blood.rs` | `wrong-fixed` |
| 3 | `RedSkull.java` | `red_skull.rs` | `exact` |
| 4 | `PaperFrog.java` | `paper_frog.rs` | `exact` |
| 5 | `Brimstone.java` | `brimstone.rs` | `wrong-fixed` |
| 6 | `ChampionsBelt.java` | `champion_belt.rs` | `wrong-fixed` |
| 7 | `CharonsAshes.java` | `charons_ashes.rs` | `wrong-fixed` |
| 8 | `MagicFlower.java` | `magic_flower.rs` | `exact` |
| 9 | `MarkOfPain.java` | `mark_of_pain.rs` | `exact` |
| 10 | `RunicCube.java` | `runic_cube.rs` | `exact` |
| 11 | `SelfFormingClay.java` | `self_forming_clay.rs` | `wrong-fixed` |

## Shared Relic Queue Started

Shared relics are available to Ironclad runs and are audited after the
class-specific queue.

| # | Java relic file | Rust relic module | Status |
|---:|---|---|---|
| 1 | `Akabeko.java` | `akabeko.rs` | `exact` |
| 2 | `Anchor.java` | `anchor.rs` | `exact` |
| 3 | `BagOfMarbles.java` | `bag_of_marbles.rs` | `wrong-fixed` |
| 4 | `BagOfPreparation.java` | `bag_of_preparation.rs` | `exact` |
| 5 | `BloodVial.java` | `blood_vial.rs` | `exact` |
| 6 | `BronzeScales.java` | `bronze_scales.rs` | `exact` |
| 7 | `CentennialPuzzle.java` | `centennial_puzzle.rs` | `wrong-fixed` |
| 8 | `HappyFlower.java` | `happy_flower.rs` | `wrong-fixed` |
| 9 | `Lantern.java` | `lantern.rs` | `wrong-fixed` |
| 10 | `Nunchaku.java` | `nunchaku.rs` | `wrong-fixed` |
| 11 | `PenNib.java` | `pen_nib.rs` | `wrong-fixed` |
| 12 | `AncientTeaSet.java` | `ancient_tea_set.rs` | `wrong-fixed` |
| 13 | `ArtOfWar.java` | `art_of_war.rs` | `wrong-fixed` |
| 14 | `Orichalcum.java` | `orichalcum.rs` | `exact` |
| 15 | `OddlySmoothStone.java` | `oddly_smooth_stone.rs` | `exact` |
| 16 | `Boot.java` | `boot.rs` / damage handler | `exact` |
| 17 | `PreservedInsect.java` | `preserved_insect.rs` | `wrong-fixed` |
| 18 | `Vajra.java` | `vajra.rs` | `wrong-fixed` |
| 19 | `Strawberry.java` | `strawberry.rs` | `exact` |
| 20 | `CeramicFish.java` | `ceramic_fish.rs` / deck manager | `wrong-fixed` |
| 21 | `DreamCatcher.java` | `dream_catcher.rs` / campfire handler | `exact` |
| 22 | `JuzuBracelet.java` | event generator | `exact` |
| 23 | `MawBank.java` | run loop / shop handler | `wrong-fixed` |
| 24 | `MealTicket.java` | run loop | `wrong-fixed` |
| 25 | `RegalPillow.java` | campfire handler | `exact` |
| 26 | `SmilingMask.java` | shop generation / shop handler | `exact` |
| 27 | `TinyChest.java` | event generator | `exact` |
| 28 | `Omamori.java` | deck manager | `exact` |
| 29 | `PotionBelt.java` | `potion_belt.rs` | `exact` |
| 30 | `ToyOrnithopter.java` | `toy_ornithopter.rs` / run-level potion path | `wrong-fixed` |
| 31 | `WarPaint.java` | `war_paint.rs` | `wrong-fixed` |
| 32 | `Whetstone.java` | `whetstone.rs` | `wrong-fixed` |
| 33 | `DarkstonePeriapt.java` | deck manager | `wrong-fixed` |
| 34 | `MoltenEgg2.java` | reward/shop/deck preview pipeline | `wrong-fixed` |
| 35 | `ToxicEgg2.java` | reward/shop/deck preview pipeline | `wrong-fixed` |
| 36 | `FrozenEgg2.java` | reward/shop/deck preview pipeline | `wrong-fixed` |
| 37 | `QuestionCard.java` | reward generator | `exact` |
| 38 | `GremlinHorn.java` | `gremlin_horn.rs` | `wrong-fixed` |
| 39 | `LetterOpener.java` | `letter_opener.rs` | `wrong-fixed` |
| 40 | `Kunai.java` | `kunai.rs` | `wrong-fixed` |
| 41 | `Shuriken.java` | `shuriken.rs` | `wrong-fixed` |
| 42 | `OrnamentalFan.java` | `ornamental_fan.rs` | `exact` |
| 43 | `MercuryHourglass.java` | `mercury_hourglass.rs` | `exact` |
| 44 | `HornCleat.java` | `horn_cleat.rs` | `wrong-fixed` |
| 45 | `Pantograph.java` | `pantograph.rs` | `wrong-fixed` |
| 46 | `MeatOnTheBone.java` | `meat_on_the_bone.rs` | `wrong-fixed` |
| 47 | `Pear.java` | `pear.rs` | `exact` |
| 48 | `SingingBowl.java` | reward handler | `exact` |
| 49 | `WhiteBeast.java` | reward generator | `exact` |
| 50 | `BlueCandle.java` | `blue_candle.rs` / play handler | `exact` |
| 51 | `InkBottle.java` | `ink_bottle.rs` | `wrong-fixed` |
| 52 | `MummifiedHand.java` | `mummified_hand.rs` | `wrong-fixed` |
| 53 | `Sundial.java` | `sundial.rs` | `wrong-fixed` |
| 54 | `StrikeDummy.java` | `strike_dummy.rs` | `exact` |
| 55 | `Matryoshka.java` | `matryoshka.rs` / run loop | `exact` |
| 56 | `BottledFlame.java` | `relic_manager.rs` / run loop / combat init | `wrong-fixed` |
| 57 | `BottledLightning.java` | `relic_manager.rs` / run loop / combat init | `wrong-fixed` |
| 58 | `BottledTornado.java` | `relic_manager.rs` / run loop / combat init | `wrong-fixed` |
| 59 | `Courier.java` | shop generation / shop handler | `wrong-fixed` |
| 60 | `EternalFeather.java` | run loop | `exact` |
| 61 | `NlothsGift.java` | reward generator | `exact` |
| 62 | `CaptainsWheel.java` | `captains_wheel.rs` | `wrong-fixed` |
| 63 | `ClockworkSouvenir.java` | `clockwork_souvenir.rs` | `exact` |
| 64 | `FossilizedHelix.java` | `fossilized_helix.rs` | `exact` |
| 65 | `IncenseBurner.java` | `incense_burner.rs` | `wrong-fixed` |
| 66 | `Pocketwatch.java` | `pocketwatch.rs` | `exact` |
| 67 | `StoneCalendar.java` | `stone_calendar.rs` | `wrong-fixed` |
| 68 | `ThreadAndNeedle.java` | `thread_and_needle.rs` | `wrong-fixed` |
| 69 | `Calipers.java` | `calipers.rs` / turn block cleanup | `exact` |
| 70 | `Torii.java` | `torii.rs` / damage handler | `wrong-fixed` |
| 71 | `TungstenRod.java` | `tungsten_rod.rs` / damage handler | `wrong-fixed` |
| 72 | `Ginger.java` | `ginger.rs` / apply-power handler | `wrong-fixed` |
| 73 | `Turnip.java` | `turnip.rs` / apply-power handler | `wrong-fixed` |
| 74 | `IceCream.java` | `ice_cream.rs` / turn energy recharge | `wrong-fixed` |
| 75 | `LizardTail.java` | `lizard_tail.rs` / death revive check | `wrong-fixed` |
| 76 | `Mango.java` | `mango.rs` / relic manager | `exact` |
| 77 | `OldCoin.java` | `old_coin.rs` / relic manager / pool gate | `wrong-fixed` |
| 78 | `Girya.java` | `girya.rs` / campfire handler / pool gate | `wrong-fixed` |
| 79 | `PeacePipe.java` | campfire handler / pool gate | `wrong-fixed` |
| 80 | `PrayerWheel.java` | reward generator / pool gate | `wrong-fixed` |
| 81 | `Shovel.java` | campfire handler / pool gate | `wrong-fixed` |
| 82 | `WingBoots.java` | run loop / pool gate | `wrong-fixed` |
| 83 | `BirdFacedUrn.java` | `bird_faced_urn.rs` | `exact` |
| 84 | `DeadBranch.java` | `dead_branch.rs` | `wrong-fixed` |
| 85 | `DuVuDoll.java` | `du_vu_doll.rs` / relic manager / run deck change | `wrong-fixed` |
| 86 | `GamblingChip.java` | `gambling_chip.rs` / pending choice | `wrong-fixed` |
| 87 | `UnceasingTop.java` | `unceasing_top.rs` / engine refresh loop | `wrong-fixed` |
| 88 | `Astrolabe.java` | `astrolabe.rs` / run pending choice | `wrong-fixed` |
| 89 | `BlackStar.java` | reward generator | `exact` |
| 90 | `BustedCrown.java` | reward generator / energy delta | `exact` |
| 91 | `CallingBell.java` | `calling_bell.rs` / reward screen | `wrong-fixed` |
| 92 | `CoffeeDripper.java` | campfire handler / energy delta | `exact` |
| 93 | `CursedKey.java` | run loop chest / curse pool | `wrong-fixed` |
| 94 | `Ectoplasm.java` | `mod.rs` / run relic pool gate | `wrong-fixed` |
| 95 | `EmptyCage.java` | `empty_cage.rs` / run pending choice | `wrong-fixed` |
| 96 | `FusionHammer.java` | campfire handler / energy delta | `exact` |
| 97 | `PandorasBox.java` | `pandoras_box.rs` / deck manager | `wrong-fixed` |
| 98 | `PhilosopherStone.java` | `philosopher_stone.rs` / spawn hooks | `wrong-fixed` |
| 99 | `RunicDome.java` | intent observation / energy delta | `exact` |
| 100 | `SacredBark.java` | potion potency path | `exact` |
| 101 | `SlaversCollar.java` | `slavers_collar.rs` / energy restore | `wrong-fixed` |
| 102 | `SneckoEye.java` | `snecko_eye.rs` / draw count | `exact` |
| 103 | `Sozu.java` | potion obtain paths / energy delta | `exact` |
| 104 | `TinyHouse.java` | `tiny_house.rs` / reward screen | `wrong-fixed` |
| 105 | `VelvetChoker.java` | card play limit / energy delta | `exact` |
| 106 | `WristBlade.java` | `wrist_blade.rs` / damage hook | `wrong-fixed` |
| 107 | `Abacus.java` | `abacus.rs` / shuffle hook | `exact` |
| 108 | `BloodyIdol.java` | `bloody_idol.rs` / run gold gain | `wrong-fixed` |
| 109 | `Cauldron.java` | `cauldron.rs` / reward screen | `wrong-fixed` |
| 110 | `ChemicalX.java` | `chemical_x.rs` / X-cost hook | `exact` |
| 111 | `Circlet.java` | `mod.rs` / run relic obtain | `wrong-fixed` |
| 112 | `DiscerningMonocle.java` | `discerning_monocle.rs` / unregistered tier | `wrong-fixed` |
| 113 | `DollysMirror.java` | `dollys_mirror.rs` / run duplicate choice | `wrong-fixed` |
| 114 | `Enchiridion.java` | `enchiridion.rs` / pre-battle card generation | `exact` |
| 115 | `FaceOfCleric.java` | `face_of_cleric.rs` / victory max HP | `exact` |
| 116 | `GremlinMask.java` | `gremlin_mask.rs` / battle-start weak | `exact` |
| 117 | `MembershipCard.java` | shop handler / price multiplier | `exact` |
| 118 | `Orrery.java` | `orrery.rs` / reward screen | `exact` |
| 119 | `MedicalKit.java` | playability / exhaust path | `exact` |
| 120 | `OrangePellets.java` | `orange_pellets.rs` / debuff cleanup | `wrong-fixed` |
| 121 | `Sling.java` | `sling.rs` / elite check | `wrong-fixed` |
| 122 | `CloakClasp.java` | `cloak_clasp.rs` / end-turn block | `exact` |
| 123 | `CrackedCore.java` | `cracked_core.rs` / channel Lightning | `wrong-fixed` |
| 124 | `Damaru.java` | `damaru.rs` / Mantra turn start | `exact` |
| 125 | `DataDisk.java` | `data_disk.rs` / Focus and orb passives | `exact` |
| 126 | `GoldPlatedCables.java` | orb trigger path / first orb extra passive | `wrong-fixed` |
| 127 | `EmotionChip.java` | `emotion_chip.rs` / Impulse orb trigger | `wrong-fixed` |
| 128 | `FrozenCore.java` | `frozen_core.rs` / empty slot Frost channel | `exact` |
| 129 | `HandDrill.java` | damage pipeline / block break Vulnerable | `exact` |
| 130 | `PureWater.java` | `pure_water.rs` / pre-draw Miracle | `exact` |
| 131 | `HolyWater.java` | `holy_water.rs` / pre-draw Miracles | `exact` |
| 132 | `Duality.java` | `duality.rs` / temporary Dexterity | `exact` |
| 133 | `GoldenEye.java` | `golden_eye.rs` / scry amount modifier | `exact` |
| 134 | `Melange.java` | `melange.rs` / shuffle Scry | `exact` |
| 135 | `SnakeRing.java` | `snake_ring.rs` / battle-start draw | `exact` |
| 136 | `RingOfTheSerpent.java` | draw-count runtime / hand-size passive | `wrong-fixed` |
| 137 | `NinjaScroll.java` | `ninja_scroll.rs` / pre-draw Shivs | `exact` |
| 138 | `HoveringKite.java` | `hovering_kite.rs` / manual discard energy | `exact` |
| 139 | `SneckoSkull.java` | power application / Poison amount mutation | `exact` |
| 140 | `PaperCrane.java` | monster damage pipeline / Weak multiplier | `wrong-fixed` |
| 141 | `NuclearBattery.java` | `nuclear_battery.rs` / pre-battle Plasma | `exact` |
| 142 | `SymbioticVirus.java` | `symbiotic_virus.rs` / pre-battle Dark | `exact` |
| 143 | `RunicCapacitor.java` | `runic_capacitor.rs` / first-turn orb slots | `wrong-fixed` |
| 144 | `Inserter.java` | `inserter.rs` / every-second-turn orb slot | `wrong-fixed` |
| 145 | `deprecated/DEPRECATEDDodecahedron.java` | `dodecahedron.rs` / deprecated turn-start energy | `wrong-fixed` |
