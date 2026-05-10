# Ironclad Relic Source Audit

Purpose:
- Compare Rust relic mechanics available to Ironclad runs against the
  decompiled Java source under `D:/rust/cardcrawl/relics`.
- Preserve gameplay semantics even when the Java behavior is odd.
- Exclude UI/VFX-only behavior unless it changes state, RNG, ordering, or
  observable combat decisions.

Cards are already tracked in `docs/audits/IRONCLAD_CARD_SOURCE_AUDIT.md`.
This file starts the same evidence-driven pass for Ironclad relics.

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
- `D:/rust/cardcrawl/relics/ChampionBelt.java`
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
| 6 | `ChampionBelt.java` | `champion_belt.rs` | `wrong-fixed` |
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
