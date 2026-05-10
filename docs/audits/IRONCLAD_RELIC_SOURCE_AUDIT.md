# Ironclad Relic Source Audit

Purpose:
- Compare Rust relic mechanics against the decompiled Java source under
  `D:/rust/cardcrawl/relics`.
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

## Full Ironclad Relic Queue

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
