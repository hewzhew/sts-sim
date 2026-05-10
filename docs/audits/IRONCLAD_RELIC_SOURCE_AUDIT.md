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

Status: `known-gap`

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
- UI-only above-creature action is intentionally not represented.
- Known gap: the current Rust run layer has no canonical out-of-combat potion
  use path, so the Java non-combat `player.heal(5)` branch is not implemented.

Coverage:
- `shared_common_obtain_potion_upgrade_relic_metadata_matches_java_sources`
- `toy_ornithopter_queues_bottom_heal_when_potion_is_used`

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
| 30 | `ToyOrnithopter.java` | `toy_ornithopter.rs` | `known-gap` |
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
