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

## Full Ironclad Relic Queue

Relics remain `unreviewed` until their Java file, Rust definition/subscription,
hook implementation, and supporting engine behavior have all been checked.

| # | Java relic file | Rust relic module | Status |
|---:|---|---|---|
| 1 | `BurningBlood.java` | `burning_blood.rs` | `wrong-fixed` |
| 2 | `BlackBlood.java` | `black_blood.rs` | `wrong-fixed` |
| 3 | `RedSkull.java` | `red_skull.rs` | `exact` |
| 4 | `PaperFrog.java` | `paper_frog.rs` | `exact` |
| 5 | `Brimstone.java` | `brimstone.rs` | `unreviewed` |
| 6 | `ChampionBelt.java` | `champion_belt.rs` | `unreviewed` |
| 7 | `CharonsAshes.java` | `charons_ashes.rs` | `unreviewed` |
| 8 | `MagicFlower.java` | `magic_flower.rs` | `unreviewed` |
| 9 | `MarkOfPain.java` | `mark_of_pain.rs` | `unreviewed` |
| 10 | `RunicCube.java` | `runic_cube.rs` | `unreviewed` |
| 11 | `SelfFormingClay.java` | `self_forming_clay.rs` | `unreviewed` |
