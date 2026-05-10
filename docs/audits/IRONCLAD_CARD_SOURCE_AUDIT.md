# Ironclad Card Source Audit

This ledger is the required path for Ironclad card work. It exists to stop
mechanic edits from becoming seed patches or memory-based rewrites.

The source of truth is the decompiled Java game source under
`D:/rust/cardcrawl/cards/red`. Rust may change structure for performance and AI
use, but it must preserve gameplay semantics unless this ledger marks a Java
behavior as UI/VFX-only or intentionally unsupported.

## Rules

- Audit one Java card file at a time.
- For each card, compare constructor fields, `use`, `upgrade`, `canUse`, hooks,
  side effects, queued action order, random sources, card tags, and instance
  fields such as `misc`, cost mutation, exhaust, innate, and ethereal.
- A Rust implementation is not accepted by card definition alone. Runtime
  actions and supporting engine behavior must also match.
- Every reviewed card must cite the Java file and Rust files inspected.
- When Java behavior is gameplay-visible, Rust must preserve it even if the
  Java implementation is awkward. UI/VFX-only behavior must be named and
  excluded explicitly.
- `unreviewed` is only a queue state. Final accepted states are `exact`,
  `wrong-fixed`, `missing`, or `intentionally-unsupported`.

## Batch 1 - Starter / Basic Ironclad Cards

### Strike_Red

Status: `exact`

Java source:
- `D:/rust/cardcrawl/cards/red/Strike_Red.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/strike.rs`
- `src/content/cards/runtime_impl.rs`

Java evidence:
- Constructor: cost `1`, type `ATTACK`, color `RED`, rarity `BASIC`, target
  `ENEMY`, `baseDamage = 6`.
- Tags: `STRIKE`, `STARTER_STRIKE`.
- `use`: normal mode queues one `DamageAction` targeting `m` with
  `new DamageInfo(p, this.damage, this.damageTypeForTurn)`.
- `upgrade`: `upgradeDamage(3)`.
- Debug-only damage branches are not simulator gameplay semantics.

Rust result:
- Definition matches the Java constructor and tags.
- Runtime emits one `Action::Damage` with player source, requested target,
  normal damage, and evaluated damage.

Coverage:
- `ironclad_starter_basic_definitions_match_java_sources`
- `ironclad_starter_basic_runtime_actions_match_java_use_methods`

### Defend_Red

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Defend_Red.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/defend.rs`
- `src/content/cards/runtime_impl.rs`

Java evidence:
- Constructor: cost `1`, type `SKILL`, color `RED`, rarity `BASIC`, target
  `SELF`, `baseBlock = 5`.
- Tags: `STARTER_DEFEND`.
- `use`: normal mode queues one `GainBlockAction(p, p, this.block)`.
- `upgrade`: `upgradeBlock(3)`.
- Debug-only `50` block branch is not simulator gameplay semantics.

Rust result:
- Fixed Rust definition to include `CardTag::StarterDefend`.
- Fixed Rust runtime to evaluate the card at play time before emitting
  `Action::GainBlock`, so block powers and upgrades do not depend on stale card
  mutation fields.

Why this mattered:
- Missing `STARTER_DEFEND` affects starter-card recognition paths such as
  Pandora's Box and events that operate on starter Strikes/Defends.

Coverage:
- `ironclad_starter_basic_definitions_match_java_sources`
- `ironclad_starter_basic_runtime_actions_match_java_use_methods`

### Bash

Status: `exact`

Java source:
- `D:/rust/cardcrawl/cards/red/Bash.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/bash.rs`
- `src/content/cards/runtime_impl.rs`

Java evidence:
- Constructor: cost `2`, type `ATTACK`, color `RED`, rarity `BASIC`, target
  `ENEMY`, `baseDamage = 8`, `baseMagicNumber = magicNumber = 2`.
- `use`: queues `DamageAction` first, then `ApplyPowerAction` applying
  `VulnerablePower(m, this.magicNumber, false)` to the target.
- `upgrade`: `upgradeDamage(2)` and `upgradeMagicNumber(1)`.
- Debug-only all-enemy damage branch is not simulator gameplay semantics.

Rust result:
- Definition matches Java constructor and upgrade values.
- Runtime emits damage first, then vulnerable application to the same target.

Coverage:
- `ironclad_starter_basic_definitions_match_java_sources`
- `ironclad_starter_basic_runtime_actions_match_java_use_methods`

## Batch 2 - Early Ironclad Utility / Power Coverage

### Anger

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Anger.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/anger.rs`
- `src/content/cards/runtime_impl.rs`

Java evidence:
- Constructor: cost `0`, type `ATTACK`, color `RED`, rarity `COMMON`,
  target `ENEMY`, `baseDamage = 6`.
- `use`: queues `DamageAction`, then a VFX action, then
  `MakeTempCardInDiscardAction(this.makeStatEquivalentCopy(), 1)`.
- `upgrade`: `upgradeDamage(2)`.
- `VFXAction` / `VerticalAuraEffect` is UI/VFX-only and is not part of Rust
  simulator mechanics.

Rust result:
- Runtime now evaluates the card at play time before producing damage.
- Runtime emits damage followed by `MakeCopyInDiscard`, preserving upgraded
  stat-equivalent copy behavior.

Coverage:
- `ironclad_common_utility_definitions_match_java_sources`
- `ironclad_common_utility_runtime_actions_match_java_use_methods`

### Armaments

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Armaments.java`
- `D:/rust/cardcrawl/actions/unique/ArmamentsAction.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/armaments.rs`
- `src/content/cards/runtime_impl.rs`

Java evidence:
- Constructor: cost `1`, type `SKILL`, color `RED`, rarity `COMMON`, target
  `SELF`, `baseBlock = 5`.
- `use`: queues `GainBlockAction(p, p, this.block)`, then
  `ArmamentsAction(this.upgraded)`.
- Unupgraded `ArmamentsAction`: if exactly one card in hand can upgrade, upgrade
  it automatically; if more than one can upgrade, open a 1-card hand select.
- Upgraded `ArmamentsAction`: upgrades every card in hand where `c.canUpgrade()`
  is true.
- `superFlash`, `refreshHandLayout`, and select-screen UI presentation are
  UI-only. The gameplay-visible result is which cards are upgraded.

Rust result:
- Fixed definition target from `None` to `SelfTarget`.
- Runtime now evaluates block at play time.
- Runtime now applies the Java `canUpgrade()` equivalent: exclude status/curse
  cards, skip already-upgraded ordinary cards, but still allow Searing Blow's
  repeat upgrades.
- Unupgraded Armaments still auto-upgrades one candidate and opens hand-select
  for multiple candidates.

Coverage:
- `ironclad_common_utility_definitions_match_java_sources`
- `ironclad_common_utility_runtime_actions_match_java_use_methods`

### Barricade

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Barricade.java`
- `D:/rust/cardcrawl/powers/BarricadePower.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/barricade.rs`
- `src/content/cards/runtime_impl.rs`

Java evidence:
- Constructor: cost `3`, type `POWER`, color `RED`, rarity `RARE`, target
  `SELF`.
- `use`: checks player powers; only queues `ApplyPowerAction` if Barricade is
  not already present.
- `BarricadePower` has sentinel `amount = -1`.
- `upgrade`: `upgradeBaseCost(2)`.

Rust result:
- Fixed upgraded base cost override for `Barricade+` to `2`.
- Runtime now emits no action if the player already has Barricade.
- Runtime applies Barricade with sentinel amount `-1`.

Coverage:
- `ironclad_common_utility_definitions_match_java_sources`
- `ironclad_common_utility_runtime_actions_match_java_use_methods`

### Battle Trance

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/BattleTrance.java`
- `D:/rust/cardcrawl/powers/NoDrawPower.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/battle_trance.rs`
- `src/content/cards/runtime_impl.rs`

Java evidence:
- Constructor: cost `0`, type `SKILL`, color `RED`, rarity `UNCOMMON`, target
  `NONE`, `baseMagicNumber = magicNumber = 3`.
- `use`: queues `DrawCardAction(p, this.magicNumber)`, then
  `ApplyPowerAction(p, p, new NoDrawPower(p))`.
- `NoDrawPower` has sentinel `amount = -1` and removes itself at end of the
  player's turn.
- `upgrade`: `upgradeMagicNumber(1)`.

Rust result:
- Runtime now evaluates the card at play time before emitting draw count.
- Runtime emits `DrawCards(3/4)` followed by player `NoDraw` with sentinel
  amount `-1`.

Coverage:
- `ironclad_common_utility_definitions_match_java_sources`
- `ironclad_common_utility_runtime_actions_match_java_use_methods`

## Batch 3 - Cost / HP-Loss Ironclad Coverage

### Berserk

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Berserk.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/berserk.rs`
- `src/content/cards/runtime_impl.rs`

Java evidence:
- Constructor: cost `0`, type `POWER`, color `RED`, rarity `RARE`, target
  `SELF`, `baseMagicNumber = magicNumber = 2`.
- `use`: queues `ApplyPowerAction` for player Vulnerable with `magicNumber`,
  then `ApplyPowerAction` for `BerserkPower(p, 1)`.
- `upgrade`: `upgradeMagicNumber(-1)`.

Rust result:
- Fixed definition target from `None` to `SelfTarget`.
- Runtime now evaluates magic at play time, so `Berserk+` applies 1
  Vulnerable instead of relying on stale mutation fields.

Coverage:
- `ironclad_cost_and_hp_cards_definitions_match_java_sources`
- `ironclad_cost_and_hp_cards_runtime_actions_match_java_use_methods`

### Blood for Blood

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/BloodForBlood.java`
- `D:/rust/cardcrawl/characters/AbstractPlayer.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/blood_for_blood.rs`
- `src/content/cards/runtime_impl.rs`
- `src/engine/action_handlers/damage.rs`

Java evidence:
- Constructor: cost `4`, type `ATTACK`, color `RED`, rarity `UNCOMMON`,
  target `ENEMY`, `baseDamage = 18`.
- `tookDamage`: `updateCost(-1)`.
- `AbstractPlayer.updateCardsOnDamage`: when the player loses HP in combat,
  calls `tookDamage()` on cards in hand, discard pile, and draw pile.
- `makeCopy`: if player exists, applies
  `updateCost(-AbstractDungeon.player.damagedThisCombat)`.
- `use`: queues one `DamageAction`.
- `upgrade`: if current cost is already below `4`, upgrades base cost to
  current cost minus one, clamped at zero; otherwise upgrades base cost to `3`;
  then `upgradeDamage(4)`.

Rust result:
- Fixed upgraded base cost override for `Blood for Blood+` to `3`.
- Damage / HP-loss handling now decrements Blood for Blood cost modifiers in
  hand, discard pile, and draw pile when the player actually loses HP.
- Runtime damage already evaluates card damage at play time.

Coverage:
- `ironclad_cost_and_hp_cards_definitions_match_java_sources`
- `ironclad_cost_and_hp_cards_runtime_actions_match_java_use_methods`
- `blood_for_blood_cost_updates_when_player_takes_hp_loss`

### Bloodletting

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Bloodletting.java`
- `D:/rust/cardcrawl/actions/common/LoseHPAction.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/bloodletting.rs`
- `src/engine/action_handlers/damage.rs`

Java evidence:
- Constructor: cost `0`, type `SKILL`, color `RED`, rarity `UNCOMMON`, target
  `SELF`, `baseMagicNumber = magicNumber = 2`.
- `use`: queues `LoseHPAction(p, p, 3)`, then
  `GainEnergyAction(this.magicNumber)`.
- `LoseHPAction` calls `target.damage(... DamageType.HP_LOSS)`, so ordinary
  player HP-loss hooks and card `tookDamage()` behavior apply.
- `upgrade`: `upgradeMagicNumber(1)`.

Rust result:
- Runtime now evaluates magic at play time before emitting energy gain.
- Existing `LoseHp` action keeps `triggers_rupture = true` for self-inflicted
  card HP loss and now also updates Blood for Blood costs through the damage
  pipeline.

Coverage:
- `ironclad_cost_and_hp_cards_definitions_match_java_sources`
- `ironclad_cost_and_hp_cards_runtime_actions_match_java_use_methods`

### Bludgeon

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Bludgeon.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/bludgeon.rs`
- `src/content/cards/runtime_impl.rs`

Java evidence:
- Constructor: cost `3`, type `ATTACK`, color `RED`, rarity `RARE`, target
  `ENEMY`, `baseDamage = 32`.
- `use`: queues VFX if target exists, then `WaitAction(0.8f)`, then one
  `DamageAction`.
- `upgrade`: `upgradeDamage(10)`.
- VFX and wait timing are presentation/timing-only for the Rust simulator and
  do not change combat mechanics.

Rust result:
- Runtime now evaluates the card at play time before emitting damage.
- Runtime emits the gameplay-visible damage action with base/upgraded values.

Coverage:
- `ironclad_cost_and_hp_cards_definitions_match_java_sources`
- `ironclad_cost_and_hp_cards_runtime_actions_match_java_use_methods`

### Shared Cost-Spend Fix

Status: `wrong-fixed`

Java evidence:
- Cards with `upgradeBaseCost` spend the upgraded base cost when played.

Rust result:
- `handle_play_card_from_hand` now uses `CombatCard::get_cost()`, including
  upgraded base cost overrides and cost modifiers, when spending non-X-cost
  energy. This fixes `Barricade+` and protects future upgraded-cost cards.

Coverage:
- `upgraded_base_cost_is_used_when_spending_energy`

## Batch 4 - Block / Exhaust / Ethereal Ironclad Coverage

### Body Slam

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/BodySlam.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/body_slam.rs`
- `src/content/cards/runtime_impl.rs`

Java evidence:
- Constructor: cost `1`, type `ATTACK`, color `RED`, rarity `COMMON`, target
  `ENEMY`, `baseDamage = 0`.
- `applyPowers` and `use`: set `baseDamage` to player current block, then
  calculate damage.
- `upgrade`: `upgradeBaseCost(0)`.
- Description changes in `applyPowers`, `calculateCardDamage`, and
  `onMoveToDiscard` are UI text behavior and are not part of Rust simulator
  mechanics.

Rust result:
- Fixed upgraded base cost override for `Body Slam+` to `0`.
- Runtime now evaluates the card at play time, so damage is based on current
  player block and target damage modifiers without relying on stale mutation
  fields.

Coverage:
- `ironclad_block_exhaust_and_ethereal_definitions_match_java_sources`
- `ironclad_block_exhaust_and_ethereal_runtime_actions_match_java_use_methods`

### Brutality

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Brutality.java`
- `D:/rust/cardcrawl/powers/BrutalityPower.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/brutality.rs`
- `src/content/powers/ironclad/brutality.rs`

Java evidence:
- Constructor: cost `0`, type `POWER`, color `RED`, rarity `RARE`, target
  `SELF`.
- `use`: applies `BrutalityPower(p, 1)`.
- `upgrade`: sets `isInnate = true`.
- `BrutalityPower.atStartOfTurnPostDraw`: draws `amount`, then queues
  `LoseHPAction(owner, owner, amount)`.

Rust result:
- Fixed `is_innate_card` so `Brutality+` is innate.
- Runtime card use applies `PowerId::Brutality` amount `1`.
- Existing post-draw power hook draws and then loses HP through the normal
  HP-loss pipeline.

Coverage:
- `ironclad_block_exhaust_and_ethereal_definitions_match_java_sources`
- `ironclad_block_exhaust_and_ethereal_runtime_actions_match_java_use_methods`

### Burning Pact

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/BurningPact.java`
- `D:/rust/cardcrawl/actions/common/ExhaustAction.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/burning_pact.rs`

Java evidence:
- Constructor: cost `1`, type `SKILL`, color `RED`, rarity `UNCOMMON`, target
  `NONE`, `baseMagicNumber = magicNumber = 2`.
- `use`: queues `ExhaustAction(1, false)`, then
  `DrawCardAction(p, this.magicNumber)`.
- `ExhaustAction(1, false)`: if hand size is 0, does nothing; if hand size is
  `<= amount`, exhausts all; otherwise opens a non-random 1-card hand select.
- `upgrade`: `upgradeMagicNumber(1)`.

Rust result:
- Runtime now evaluates magic at play time before emitting draw count.
- Runtime preserves the Java action order: exhaust one eligible hand card first,
  then draw 2/3 cards.

Coverage:
- `ironclad_block_exhaust_and_ethereal_definitions_match_java_sources`
- `ironclad_block_exhaust_and_ethereal_runtime_actions_match_java_use_methods`

### Carnage

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Carnage.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/carnage.rs`

Java evidence:
- Constructor: cost `2`, type `ATTACK`, color `RED`, rarity `UNCOMMON`, target
  `ENEMY`, `baseDamage = 20`, `isEthereal = true`.
- `use`: queues VFX actions, then one `DamageAction`.
- `upgrade`: `upgradeDamage(8)`.
- VFX and wait/timing behavior are presentation-only and are not part of Rust
  simulator mechanics.

Rust result:
- Runtime now evaluates the card at play time before emitting damage.
- Definition already preserves ethereal behavior and upgrade damage.

Coverage:
- `ironclad_block_exhaust_and_ethereal_definitions_match_java_sources`
- `ironclad_block_exhaust_and_ethereal_runtime_actions_match_java_use_methods`

## Batch 5 - Conditional / AoE / Weak / End-Turn Damage Coverage

### Clash

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Clash.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/clash.rs`
- `src/content/cards/runtime_impl.rs`

Java evidence:
- Constructor: cost `0`, type `ATTACK`, color `RED`, rarity `COMMON`, target
  `ENEMY`, `baseDamage = 14`.
- `canUse`: after `super.canUse`, scans every card in player hand and rejects
  play if any card is not an `ATTACK`.
- `use`: queues optional VFX, then one `DamageAction` with
  `new DamageInfo(p, this.damage, this.damageTypeForTurn)`.
- `upgrade`: `upgradeDamage(4)`.
- `ClashEffect` is UI/VFX-only and is not simulator mechanics.

Rust result:
- Definition matches Java constructor and upgrade damage.
- `can_play_card` preserves the Java all-hand attack requirement.
- Runtime now evaluates the card at play time before emitting damage, so
  upgrades and damage modifiers do not rely on stale mutation fields.

Coverage:
- `ironclad_attack_condition_and_dot_power_definitions_match_java_sources`
- `ironclad_attack_condition_and_dot_power_runtime_actions_match_java_use_methods`

### Cleave

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Cleave.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/cleave.rs`
- `src/content/cards/runtime_impl.rs`

Java evidence:
- Constructor: cost `1`, type `ATTACK`, color `RED`, rarity `COMMON`, target
  `ALL_ENEMY`, `baseDamage = 8`, `isMultiDamage = true`.
- `use`: queues SFX/VFX, then `DamageAllEnemiesAction(p, this.multiDamage,
  this.damageTypeForTurn, NONE)`.
- `upgrade`: `upgradeDamage(3)`.
- SFX/VFX actions are presentation-only and are not simulator mechanics.

Rust result:
- Definition matches Java constructor, multi-damage flag, target, and upgrade.
- Runtime now evaluates the card at play time before emitting
  `Action::DamageAllEnemies`, so upgraded and per-target damage are generated
  from current combat state.

Coverage:
- `ironclad_attack_condition_and_dot_power_definitions_match_java_sources`
- `ironclad_attack_condition_and_dot_power_runtime_actions_match_java_use_methods`

### Clothesline

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Clothesline.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/clothesline.rs`
- `src/content/cards/runtime_impl.rs`

Java evidence:
- Constructor: cost `2`, type `ATTACK`, color `RED`, rarity `COMMON`, target
  `ENEMY`, `baseDamage = 12`, `baseMagicNumber = magicNumber = 2`.
- `use`: queues `DamageAction`, then `ApplyPowerAction` applying
  `WeakPower(m, this.magicNumber, false)` to the same target.
- `upgrade`: `upgradeDamage(2)` and `upgradeMagicNumber(1)`.

Rust result:
- Definition matches Java constructor and upgrade values.
- Runtime now evaluates damage and magic at play time, preserving the Java
  damage-then-weak action order with upgraded Weak duration.

Coverage:
- `ironclad_attack_condition_and_dot_power_definitions_match_java_sources`
- `ironclad_attack_condition_and_dot_power_runtime_actions_match_java_use_methods`

### Combust

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Combust.java`
- `D:/rust/cardcrawl/powers/CombustPower.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/combust.rs`
- `src/content/powers/ironclad/combust.rs`
- `src/engine/action_handlers/powers.rs`

Java evidence:
- Card constructor: cost `1`, type `POWER`, color `RED`, rarity `UNCOMMON`,
  target `SELF`, `baseMagicNumber = magicNumber = 5`.
- Card `use`: applies `new CombustPower(p, 1, this.magicNumber)`.
- Card `upgrade`: `upgradeMagicNumber(2)`.
- `CombustPower` stores two gameplay values: `amount` is all-enemy damage, and
  `hpLoss` is the player's end-turn HP loss.
- `CombustPower.stackPower(stackAmount)`: adds `stackAmount` to damage and
  increments `hpLoss` by exactly `1`.
- `CombustPower.atEndOfTurn`: if monsters are not basically dead, queues
  `LoseHPAction(owner, owner, hpLoss, FIRE)` and then
  `DamageAllEnemiesAction(null, createDamageMatrix(amount, true), THORNS, FIRE)`.

Rust result:
- Definition matches Java constructor and upgrade magic.
- Runtime now evaluates magic at play time before applying Combust.
- Existing power storage matches Java by using `Power.amount` for damage and
  `Power.extra_data` for `hpLoss`; stacking adds damage by applied amount and
  increments `extra_data` by `1`.
- End-turn all-enemy THORNS damage uses Rust `NO_SOURCE`, preserving Java's
  `DamageAllEnemiesAction(null, ...)` source semantics.
- Fixed the end-turn hook to skip when all monsters are basically dead, matching
  the Java guard before HP loss and all-enemy damage.

Coverage:
- `ironclad_attack_condition_and_dot_power_definitions_match_java_sources`
- `ironclad_attack_condition_and_dot_power_runtime_actions_match_java_use_methods`
- `combust_power_stacks_damage_and_hp_loss_like_java_source`

## Batch 6 - Power Hooks / Cost Override / Debuff Coverage

### Corruption

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Corruption.java`
- `D:/rust/cardcrawl/powers/CorruptionPower.java`
- `D:/rust/cardcrawl/actions/common/ApplyPowerAction.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/corruption.rs`
- `src/content/cards/runtime_impl.rs`
- `src/content/powers/ironclad/corruption.rs`
- `src/engine/action_handlers/powers.rs`

Java evidence:
- Card constructor: cost `3`, type `POWER`, color `RED`, rarity `RARE`,
  target `SELF`, `baseMagicNumber = magicNumber = 3`.
- Card `use`: queues VFX/SFX, scans player powers, and applies
  `CorruptionPower(p)` only if the player does not already have Corruption.
- Card `upgrade`: `upgradeBaseCost(2)`.
- `ApplyPowerAction` constructor special-cases Corruption by calling
  `modifyCostForCombat(-9)` on every Skill in hand, draw, discard, and exhaust.
- `CorruptionPower.onCardDraw`: Skills drawn after Corruption get
  `setCostForTurn(-9)`, effectively `0`.
- `CorruptionPower.onUseCard`: played Skills set `UseCardAction.exhaustCard =
  true`.
- VFX/SFX and flash behavior are presentation-only and are not simulator
  mechanics.

Rust result:
- Fixed definition to preserve Java `baseMagicNumber = 3`.
- Fixed upgraded base cost override for `Corruption+` to `2`.
- Runtime preserves the Java no-duplicate-power check before applying
  Corruption.
- Existing Corruption power hooks preserve skill cost reduction on apply/draw
  and force played Skills to exhaust.

Coverage:
- `ironclad_power_and_debuff_definitions_match_java_sources`
- `ironclad_power_and_debuff_runtime_actions_match_java_use_methods`
- `corruption_power_on_apply_modifies_skill_costs_in_java_piles`

### Dark Embrace

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/DarkEmbrace.java`
- `D:/rust/cardcrawl/powers/DarkEmbracePower.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/dark_embrace.rs`
- `src/content/powers/ironclad/dark_embrace.rs`
- `src/content/powers/mod.rs`

Java evidence:
- Card constructor: cost `2`, type `POWER`, color `RED`, rarity `UNCOMMON`,
  target `SELF`.
- Card `use`: applies `DarkEmbracePower(p, 1)` with stack amount `1`.
- Card `upgrade`: `upgradeBaseCost(1)`.
- `DarkEmbracePower.onExhaust`: if monsters are not basically dead, draws
  `amount` cards.

Rust result:
- Fixed upgraded base cost override for `Dark Embrace+` to `1`.
- Runtime now evaluates the effect amount at play time before applying the
  power.
- Fixed Dark Embrace exhaust hook to skip draw when monsters are basically
  dead, matching the Java guard.

Coverage:
- `ironclad_power_and_debuff_definitions_match_java_sources`
- `ironclad_power_and_debuff_runtime_actions_match_java_use_methods`
- `dark_embrace_and_demon_form_power_hooks_match_java_sources`

### Demon Form

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/DemonForm.java`
- `D:/rust/cardcrawl/powers/DemonFormPower.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/demon_form.rs`
- `src/content/powers/ironclad/demon_form.rs`

Java evidence:
- Card constructor: cost `3`, type `POWER`, color `RED`, rarity `RARE`, target
  `NONE`, `baseMagicNumber = magicNumber = 2`.
- Card `use`: applies `DemonFormPower(p, this.magicNumber)` with the same
  stack amount.
- Card `upgrade`: `upgradeMagicNumber(1)`.
- `DemonFormPower.atStartOfTurnPostDraw`: applies `StrengthPower(owner,
  amount)` to the player.

Rust result:
- Fixed definition target from `SelfTarget` to `None`.
- Runtime now evaluates magic at play time, so `Demon Form+` applies amount
  `3` instead of stale base amount `2`.
- Existing post-draw power hook applies Strength equal to stored power amount.

Coverage:
- `ironclad_power_and_debuff_definitions_match_java_sources`
- `ironclad_power_and_debuff_runtime_actions_match_java_use_methods`
- `dark_embrace_and_demon_form_power_hooks_match_java_sources`

### Disarm

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Disarm.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/disarm.rs`

Java evidence:
- Constructor: cost `1`, type `SKILL`, color `RED`, rarity `UNCOMMON`, target
  `ENEMY`, `baseMagicNumber = magicNumber = 2`, `exhaust = true`.
- `use`: applies `StrengthPower(m, -this.magicNumber)` to the target with
  stack amount `-this.magicNumber`.
- `upgrade`: `upgradeMagicNumber(1)`.

Rust result:
- Definition already preserves target, exhaust, and base/upgrade magic.
- Runtime now evaluates magic at play time, so `Disarm+` applies `-3`
  Strength instead of stale `-2`.

Coverage:
- `ironclad_power_and_debuff_definitions_match_java_sources`
- `ironclad_power_and_debuff_runtime_actions_match_java_use_methods`

## Batch 7 - Copy / Conditional Attack / Block Doubling Coverage

### Double Tap

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/DoubleTap.java`
- `D:/rust/cardcrawl/powers/DoubleTapPower.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/double_tap.rs`
- `src/content/powers/ironclad/double_tap.rs`

Java evidence:
- Card constructor: cost `1`, type `SKILL`, color `RED`, rarity `RARE`,
  target `SELF`, `baseMagicNumber = magicNumber = 1`.
- Card `use`: applies `DoubleTapPower(p, this.magicNumber)` with the same
  stack amount.
- Card `upgrade`: `upgradeMagicNumber(1)`.
- `DoubleTapPower.onUseCard`: if the played card is a non-purge Attack and
  amount is positive, creates a same-instance copy in limbo, queues it with
  `purgeOnUse = true`, decrements amount, and removes the power at zero.
- `DoubleTapPower.atEndOfTurn(true)`: removes itself.

Rust result:
- Definition matches Java constructor and upgrade magic.
- Runtime now evaluates magic at play time, so `Double Tap+` applies amount
  `2` instead of stale amount `1`.
- Existing power hook queues a purge-on-use `QueuedCardPlay` copy for non-purge
  Attacks, decrements the power, and removes it at zero.

Coverage:
- `ironclad_copy_and_block_definitions_match_java_sources`
- `ironclad_copy_and_block_runtime_actions_match_java_use_methods`
- `dropkick_and_double_tap_action_hooks_match_java_sources`

### Dropkick

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Dropkick.java`
- `D:/rust/cardcrawl/actions/unique/DropkickAction.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/dropkick.rs`
- `src/engine/action_handlers/damage.rs`

Java evidence:
- Constructor: cost `1`, type `ATTACK`, color `RED`, rarity `UNCOMMON`, target
  `ENEMY`, `baseDamage = 5`.
- `use`: queues `DropkickAction(m, new DamageInfo(p, this.damage,
  this.damageTypeForTurn))`.
- `DropkickAction.update`: if the target has Vulnerable at execution time,
  queues draw 1 and gain 1 energy, then queues damage on top. Because all three
  are added to top, damage executes before energy gain and draw.
- `triggerOnGlowCheck` changes border color only; it is UI feedback, not
  simulator mechanics.
- `upgrade`: `upgradeDamage(3)`.

Rust result:
- Definition matches Java constructor and upgrade damage.
- Runtime now evaluates damage at play time before emitting the deferred
  Dropkick action.
- Engine Dropkick action checks Vulnerable at execution time and preserves Java
  queue order: damage, then gain energy, then draw.

Coverage:
- `ironclad_copy_and_block_definitions_match_java_sources`
- `ironclad_copy_and_block_runtime_actions_match_java_use_methods`
- `dropkick_and_double_tap_action_hooks_match_java_sources`

### Dual Wield

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/DualWield.java`
- `D:/rust/cardcrawl/actions/unique/DualWieldAction.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/dual_wield.rs`
- `src/engine/pending_choices.rs`

Java evidence:
- Constructor: cost `1`, type `SKILL`, color `RED`, rarity `UNCOMMON`, target
  `NONE`, `baseMagicNumber = magicNumber = 1`.
- `use`: queues `DualWieldAction(p, this.magicNumber)`.
- `DualWieldAction` only allows Attack or Power cards.
- If no valid cards exist, it does nothing.
- If exactly one valid card exists, it creates `dupeAmount` stat-equivalent
  copies.
- If multiple valid cards exist, it opens a one-card hand select. The decompiled
  selected-card branch queues one copy before its `dupeAmount` loop, making the
  selected branch create `dupeAmount + 1` copies. This is source-visible
  gameplay behavior, not UI.
- `upgrade`: `upgradeMagicNumber(1)`.

Rust result:
- Definition matches Java constructor and upgrade magic.
- Runtime now evaluates magic at play time.
- Runtime preserves the Java valid-card filter, no-op/auto/select split, and
  the selected-branch `dupeAmount + 1` copy behavior.

Coverage:
- `ironclad_copy_and_block_definitions_match_java_sources`
- `ironclad_copy_and_block_runtime_actions_match_java_use_methods`

### Entrench

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Entrench.java`
- `D:/rust/cardcrawl/actions/unique/DoubleYourBlockAction.java`
- `D:/rust/cardcrawl/actions/utility/ExhaustAllEtherealAction.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/entrench.rs`
- `src/content/cards/runtime_impl.rs`
- `src/engine/action_handlers/cards.rs`

Java evidence:
- Constructor: cost `2`, type `SKILL`, color `RED`, rarity `UNCOMMON`, target
  `SELF`.
- `use`: queues `DoubleYourBlockAction(p)`.
- `DoubleYourBlockAction.update`: if target exists and current block is
  positive, adds block equal to current block.
- `triggerOnEndOfPlayerTurn`: queues `ExhaustAllEtherealAction`, which exhausts
  all ethereal cards in hand. Rust already exhausts ethereal cards from the
  end-turn card pipeline, so no Entrench-specific UI action is needed.
- `upgrade`: `upgradeBaseCost(1)`.

Rust result:
- Fixed upgraded base cost override for `Entrench+` to `1`.
- Runtime already preserves the block doubling behavior by gaining block equal
  to the player's current block and doing nothing at zero block.

Coverage:
- `ironclad_copy_and_block_definitions_match_java_sources`
- `ironclad_copy_and_block_runtime_actions_match_java_use_methods`

## Batch 8 - Exhaust Retrieval / Growth Hooks Coverage

### Evolve

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Evolve.java`
- `D:/rust/cardcrawl/powers/EvolvePower.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/evolve.rs`
- `src/content/powers/ironclad/evolve.rs`
- `src/content/powers/mod.rs`

Java evidence:
- Constructor: cost `1`, type `POWER`, color `RED`, rarity `UNCOMMON`, target
  `SELF`, `baseMagicNumber = magicNumber = 1`.
- `use`: queues `ApplyPowerAction(p, p, new EvolvePower(p, this.magicNumber),
  this.magicNumber)`.
- `upgrade`: `upgradeMagicNumber(1)`.
- `EvolvePower.onCardDraw`: if the drawn card is a `STATUS` and the owner does
  not have `No Draw`, queues `DrawCardAction(owner, amount)`.

Rust result:
- Definition matches Java constructor and upgrade magic.
- Runtime now evaluates magic at play time before applying Evolve.
- Power hook now checks `NoDraw` before queuing the bonus draw, matching Java
  source behavior rather than relying only on the draw handler's later guard.

Coverage:
- `ironclad_exhaust_and_growth_definitions_match_java_sources`
- `ironclad_exhaust_and_growth_runtime_actions_match_java_use_methods`
- `evolve_exhume_feed_and_feel_no_pain_hooks_match_java_sources`

### Exhume

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Exhume.java`
- `D:/rust/cardcrawl/actions/unique/ExhumeAction.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/runtime_impl.rs`
- `src/content/cards/ironclad/exhume.rs`
- `src/runtime/action.rs`
- `src/engine/action_handlers/cards.rs`
- `src/engine/pending_choices.rs`

Java evidence:
- Constructor: cost `1`, type `SKILL`, color `RED`, rarity `RARE`, target
  `NONE`, `exhaust = true`.
- `use`: queues `ExhumeAction(false)`. Upgraded Exhume only changes base cost;
  it does not pass `true` to the action.
- `upgrade`: `upgradeBaseCost(0)`.
- `ExhumeAction`: if hand is full, does nothing. If exhaust pile is empty, does
  nothing. If the only exhaust card is `Exhume`, does nothing. If the exhaust
  pile has exactly one non-Exhume card, moves it to hand immediately. If there
  are multiple exhaust cards, temporarily removes Exhume cards from the grid,
  opens a non-cancellable one-card exhaust selection, then restores the removed
  Exhumes.
- When a selected Skill is returned while the player has Corruption, Java calls
  `setCostForTurn(-9)`, which makes the returned Skill cost `0` this turn.
- Hover/fade/target position calls are UI presentation only and are not
  simulator mechanics.

Rust result:
- Fixed upgraded base cost override for `Exhume+` to `0`.
- Runtime now preserves Java's hand-full no-op and sole-card auto-return split.
- Runtime emits a dedicated `ExhumeCard` action instead of generic `MoveCard`,
  because Exhume has source-specific behavior: exclude Exhume itself, do not
  drop the card when hand is full, and apply Corruption's temporary Skill cost.
- `Exhume+` now still passes `upgrade = false`, matching the actual card source.

Coverage:
- `ironclad_exhaust_and_growth_definitions_match_java_sources`
- `ironclad_exhaust_and_growth_runtime_actions_match_java_use_methods`
- `evolve_exhume_feed_and_feel_no_pain_hooks_match_java_sources`

### Feed

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Feed.java`
- `D:/rust/cardcrawl/actions/unique/FeedAction.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/feed.rs`
- `src/engine/action_handlers/damage.rs`

Java evidence:
- Constructor: cost `1`, type `ATTACK`, color `RED`, rarity `RARE`, target
  `ENEMY`, `baseDamage = 10`, `baseMagicNumber = magicNumber = 3`,
  `exhaust = true`, tag `HEALING`.
- `use`: if target exists, queues `FeedAction(m, new DamageInfo(p, this.damage,
  this.damageTypeForTurn), this.magicNumber)`.
- `upgrade`: `upgradeDamage(2)` and `upgradeMagicNumber(1)`.
- `FeedAction.update`: after damaging the target, increases player max HP only
  if the target died and is not `halfDead` and does not have `Minion`.

Rust result:
- Definition matches Java constructor, upgrade damage, upgrade magic, exhaust,
  target, and Healing tag.
- Runtime now evaluates damage and magic at play time before emitting the
  deferred Feed action.
- Engine Feed handler now excludes Minion and half-dead targets from max HP gain.

Coverage:
- `ironclad_exhaust_and_growth_definitions_match_java_sources`
- `ironclad_exhaust_and_growth_runtime_actions_match_java_use_methods`
- `evolve_exhume_feed_and_feel_no_pain_hooks_match_java_sources`

### Feel No Pain

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/FeelNoPain.java`
- `D:/rust/cardcrawl/powers/FeelNoPainPower.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/feel_no_pain.rs`
- `src/content/powers/ironclad/feel_no_pain.rs`

Java evidence:
- Constructor: cost `1`, type `POWER`, color `RED`, rarity `UNCOMMON`, target
  `SELF`, `baseMagicNumber = magicNumber = 3`.
- `use`: queues `ApplyPowerAction(p, p, new FeelNoPainPower(p,
  this.magicNumber), this.magicNumber)`.
- `upgrade`: `upgradeMagicNumber(1)`.
- `FeelNoPainPower.onExhaust`: flashes and queues `GainBlockAction(owner,
  amount)`. There is no alive-monster guard.

Rust result:
- Definition matches Java constructor and upgrade magic.
- Runtime now evaluates magic at play time before applying the power.
- Existing power hook already queues block on exhaust without an alive-monster
  guard, matching Java.

Coverage:
- `ironclad_exhaust_and_growth_definitions_match_java_sources`
- `ironclad_exhaust_and_growth_runtime_actions_match_java_use_methods`
- `evolve_exhume_feed_and_feel_no_pain_hooks_match_java_sources`

## Batch 9 - Fire / Temporary Strength Coverage

### Fiend Fire

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/FiendFire.java`
- `D:/rust/cardcrawl/actions/unique/FiendFireAction.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/fiend_fire.rs`
- `src/engine/action_handlers/damage.rs`

Java evidence:
- Constructor: cost `2`, type `ATTACK`, color `RED`, rarity `RARE`, target
  `ENEMY`, `baseDamage = 7`, `exhaust = true`.
- `use`: queues `FiendFireAction(m, new DamageInfo(p, this.damage,
  this.damageTypeForTurn))`.
- `upgrade`: `upgradeDamage(3)`.
- `FiendFireAction.update`: captures current hand size, queues that many damage
  actions and that many random exhaust actions. Since the exhaust count equals
  the hand size at execution, the whole hand is exhausted.

Rust result:
- Definition matches Java constructor, exhaust, target, and upgrade damage.
- Runtime now evaluates damage at play time before emitting Fiend Fire's
  deferred action.
- Existing engine action exhausts the current hand and deals one hit per card
  in that hand, matching the source-visible effect.

Coverage:
- `ironclad_fire_and_strength_definitions_match_java_sources`
- `ironclad_fire_and_strength_runtime_actions_match_java_use_methods`
- `fire_breathing_flame_barrier_and_fiend_fire_hooks_match_java_sources`

### Fire Breathing

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/FireBreathing.java`
- `D:/rust/cardcrawl/powers/FireBreathingPower.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/fire_breathing.rs`
- `src/content/powers/ironclad/fire_breathing.rs`

Java evidence:
- Constructor: cost `1`, type `POWER`, color `RED`, rarity `UNCOMMON`, target
  `SELF`, `baseMagicNumber = magicNumber = 6`.
- `use`: queues `ApplyPowerAction(p, p, new FireBreathingPower(p,
  this.magicNumber), this.magicNumber)`.
- `upgrade`: `upgradeMagicNumber(4)`.
- `FireBreathingPower.onCardDraw`: if the drawn card is `STATUS` or `CURSE`,
  queues `DamageAllEnemiesAction(null, createDamageMatrix(amount, true),
  THORNS, FIRE, true)`.

Rust result:
- Definition matches Java constructor and upgrade magic.
- Runtime now evaluates magic at play time before applying Fire Breathing.
- Power hook now emits no-source `THORNS` all-enemy damage for Status/Curse
  draws instead of player-source Normal damage.

Coverage:
- `ironclad_fire_and_strength_definitions_match_java_sources`
- `ironclad_fire_and_strength_runtime_actions_match_java_use_methods`
- `fire_breathing_flame_barrier_and_fiend_fire_hooks_match_java_sources`

### Flame Barrier

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/FlameBarrier.java`
- `D:/rust/cardcrawl/powers/FlameBarrierPower.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/flame_barrier.rs`
- `src/content/powers/ironclad/flame_barrier.rs`
- `src/content/powers/mod.rs`

Java evidence:
- Constructor: cost `2`, type `SKILL`, color `RED`, rarity `UNCOMMON`, target
  `SELF`, `baseBlock = 12`, `baseMagicNumber = magicNumber = 4`.
- `use`: VFX only, then queues `GainBlockAction(p, p, this.block)` and
  `ApplyPowerAction(p, p, new FlameBarrierPower(p, this.magicNumber),
  this.magicNumber)`.
- `upgrade`: `upgradeBlock(4)` and `upgradeMagicNumber(2)`.
- `FlameBarrierPower.onAttacked`: retaliates only when `info.owner != null`,
  damage type is neither `THORNS` nor `HP_LOSS`, and attacker is not the owner.
  Retaliation damage is `THORNS`.

Rust result:
- Definition matches Java constructor, block/magic values, and upgrades.
- Runtime now evaluates block and magic at play time.
- Power hook now checks no-source/self-source/Thorns/HP-loss exclusions before
  retaliating and emits Thorns damage from the owner to the attacker.
- VFX is intentionally ignored as presentation, not simulator mechanics.

Coverage:
- `ironclad_fire_and_strength_definitions_match_java_sources`
- `ironclad_fire_and_strength_runtime_actions_match_java_use_methods`
- `fire_breathing_flame_barrier_and_fiend_fire_hooks_match_java_sources`

### Flex

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Flex.java`
- `D:/rust/cardcrawl/powers/LoseStrengthPower.java`
- `D:/rust/cardcrawl/powers/StrengthPower.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/flex.rs`
- `src/content/powers/core/lose_strength.rs`
- `src/content/powers/core/strength.rs`

Java evidence:
- Constructor: cost `0`, type `SKILL`, color `RED`, rarity `COMMON`, target
  `SELF`, `baseMagicNumber = magicNumber = 2`.
- `use`: queues `ApplyPowerAction` for `StrengthPower(amount)`, then
  `LoseStrengthPower(amount)`.
- `upgrade`: `upgradeMagicNumber(2)`.
- `LoseStrengthPower.atEndOfTurn`: applies Strength `-amount`, then removes
  the `Flex` power.
- `StrengthPower` can go negative and clamps to `[-999, 999]`.

Rust result:
- Definition matches Java constructor and upgrade magic.
- Runtime now evaluates magic at play time before applying Strength and
  LoseStrength.
- Existing LoseStrength hook applies negative Strength and removes itself at end
  of turn, matching Java.

Coverage:
- `ironclad_fire_and_strength_definitions_match_java_sources`
- `ironclad_fire_and_strength_runtime_actions_match_java_use_methods`
- `fire_breathing_flame_barrier_and_fiend_fire_hooks_match_java_sources`

## Batch 10 - Topdeck / Ethereal / Strength-Scaling Coverage

### Ghostly Armor

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/GhostlyArmor.java`
- `D:/rust/cardcrawl/actions/utility/ExhaustAllEtherealAction.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/ghostly_armor.rs`
- `src/content/cards/runtime_impl.rs`

Java evidence:
- Constructor: cost `1`, type `SKILL`, color `RED`, rarity `UNCOMMON`, target
  `SELF`, `isEthereal = true`, `baseBlock = 10`.
- `use`: queues `GainBlockAction(p, p, this.block)`.
- `triggerOnEndOfPlayerTurn`: queues `ExhaustAllEtherealAction`.
- `upgrade`: `upgradeBlock(3)`.

Rust result:
- Definition matches Java constructor, ethereal flag, block, target, and upgrade
  block.
- Runtime now evaluates block at play time before emitting GainBlock.
- Existing end-turn ethereal exhaust pipeline covers the Java trigger without a
  card-specific UI action.

Coverage:
- `ironclad_topdeck_and_strength_scaling_definitions_match_java_sources`
- `ironclad_topdeck_and_strength_scaling_runtime_actions_match_java_use_methods`

### Havoc

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Havoc.java`
- `D:/rust/cardcrawl/actions/common/PlayTopCardAction.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/runtime_impl.rs`
- `src/content/cards/ironclad/havoc.rs`
- `src/engine/action_handlers/cards.rs`

Java evidence:
- Constructor: cost `1`, type `SKILL`, color `RED`, rarity `COMMON`, target
  `NONE`.
- `use`: queues `PlayTopCardAction(random alive monster, true)`.
- `upgrade`: `upgradeBaseCost(0)`.
- `PlayTopCardAction`: if draw and discard are both empty, no-op. If draw is
  empty, queues itself and `EmptyDeckShuffleAction` on top. Otherwise removes
  the top draw-pile card, sets `exhaustOnUseOnce`, places it in limbo, applies
  powers, then queues it for free autoplay.
- Target selection happens before the draw-pile action resolves; UI positioning
  and waits are presentation only.

Rust result:
- Definition matches Java constructor.
- Fixed upgraded base cost override for `Havoc+` to `0`.
- Runtime emits a top-card autoplay with `exhaust = true`.
- `PlayTopCard` now locks a random target before empty-deck shuffle handling
  instead of waiting until after the top card is drawn.

Coverage:
- `ironclad_topdeck_and_strength_scaling_definitions_match_java_sources`
- `ironclad_topdeck_and_strength_scaling_runtime_actions_match_java_use_methods`
- `headbutt_and_havoc_execution_helpers_match_java_sources`

### Headbutt

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Headbutt.java`
- `D:/rust/cardcrawl/actions/unique/DiscardPileToTopOfDeckAction.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/headbutt.rs`
- `src/runtime/action.rs`
- `src/engine/action_handlers/cards.rs`

Java evidence:
- Constructor: cost `1`, type `ATTACK`, color `RED`, rarity `COMMON`, target
  `ENEMY`, `baseDamage = 9`.
- `use`: queues damage, then `DiscardPileToTopOfDeckAction`.
- `upgrade`: `upgradeDamage(3)`.
- `DiscardPileToTopOfDeckAction.update`: if battle is ending, no-op. If discard
  pile is empty, no-op. If discard has one card, move that card to draw-pile top.
  If discard has more than one card, open a non-cancellable one-card grid select
  from discard.

Rust result:
- Definition matches Java constructor and upgrade damage.
- Runtime already evaluated damage at play time.
- Replaced static play-time discard-pile branching with a dedicated
  `DiscardPileToTopOfDeck` execution action so battle-ending and discard-pile
  state are checked after the damage action, matching Java ordering.

Coverage:
- `ironclad_topdeck_and_strength_scaling_definitions_match_java_sources`
- `ironclad_topdeck_and_strength_scaling_runtime_actions_match_java_use_methods`
- `headbutt_and_havoc_execution_helpers_match_java_sources`

### Heavy Blade

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/HeavyBlade.java`
- `D:/rust/cardcrawl/powers/StrengthPower.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/runtime_impl.rs`
- `src/content/cards/ironclad/heavy_blade.rs`
- `src/content/powers/core/strength.rs`

Java evidence:
- Constructor: cost `2`, type `ATTACK`, color `RED`, rarity `COMMON`, target
  `ENEMY`, `baseDamage = 14`, `baseMagicNumber = magicNumber = 3`.
- `use`: queues `DamageAction(m, new DamageInfo(p, this.damage,
  this.damageTypeForTurn))`; VFX is presentation only.
- `applyPowers` and `calculateCardDamage`: temporarily multiply Strength power
  amount by `magicNumber`, call the base calculation, then divide it back.
- `upgrade`: `upgradeMagicNumber(2)` only.

Rust result:
- Definition matches Java constructor and upgrade magic.
- Runtime now evaluates damage at play time.
- Fixed card evaluation ordering so upgraded/base magic is written before
  card-specific and power damage calculations. This makes Heavy Blade's Strength
  multiplier use the current magic value instead of stale cached state.

Coverage:
- `ironclad_topdeck_and_strength_scaling_definitions_match_java_sources`
- `ironclad_topdeck_and_strength_scaling_runtime_actions_match_java_use_methods`

## Batch 11 - HP Loss / Status Generation / Random Attack Coverage

### Hemokinesis

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Hemokinesis.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/hemokinesis.rs`

Java evidence:
- Constructor: cost `1`, type `ATTACK`, color `RED`, rarity `UNCOMMON`, target
  `ENEMY`, `baseDamage = 15`, `baseMagicNumber = magicNumber = 2`.
- `use`: VFX only, then queues `LoseHPAction(p, p, this.magicNumber)`, then
  damage with `this.damage`.
- `upgrade`: `upgradeDamage(5)` only.

Rust result:
- Definition matches Java constructor and upgrade damage.
- Runtime now evaluates damage and magic at play time.
- Existing HP-loss action is player-authored and keeps `triggers_rupture = true`,
  matching the Java `LoseHPAction(p, p, ...)` path.

Coverage:
- `ironclad_hp_loss_and_generated_attack_definitions_match_java_sources`
- `ironclad_hp_loss_and_generated_attack_runtime_actions_match_java_use_methods`

### Immolate

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Immolate.java`
- `D:/rust/cardcrawl/actions/common/MakeTempCardInDiscardAction.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/immolate.rs`

Java evidence:
- Constructor: cost `2`, type `ATTACK`, color `RED`, rarity `RARE`, target
  `ALL_ENEMY`, `baseDamage = 21`, `isMultiDamage = true`, preview card `Burn`.
- `use`: queues `DamageAllEnemiesAction(p, this.multiDamage, this.damageTypeForTurn,
  FIRE)`, then `MakeTempCardInDiscardAction(new Burn(), 1)`.
- `upgrade`: `upgradeDamage(7)`.
- The generated Burn is a Status, so Master Reality does not upgrade it.

Rust result:
- Definition matches Java constructor, multi-damage flag, target, and upgrade
  damage.
- Runtime now evaluates the card at play time before reading `multi_damage`;
  this prevents stale/empty multi-damage from being emitted.
- Runtime already adds one unupgraded Burn to discard.

Coverage:
- `ironclad_hp_loss_and_generated_attack_definitions_match_java_sources`
- `ironclad_hp_loss_and_generated_attack_runtime_actions_match_java_use_methods`

### Impervious

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Impervious.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/impervious.rs`

Java evidence:
- Constructor: cost `2`, type `SKILL`, color `RED`, rarity `RARE`, target
  `SELF`, `baseBlock = 30`, `exhaust = true`.
- `use`: queues `GainBlockAction(p, p, this.block)`.
- `upgrade`: `upgradeBlock(10)`.

Rust result:
- Definition matches Java constructor, exhaust flag, and upgrade block.
- Runtime now evaluates block at play time before emitting GainBlock.

Coverage:
- `ironclad_hp_loss_and_generated_attack_definitions_match_java_sources`
- `ironclad_hp_loss_and_generated_attack_runtime_actions_match_java_use_methods`

### Infernal Blade

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/InfernalBlade.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/runtime_impl.rs`
- `src/content/cards/ironclad/infernal_blade.rs`

Java evidence:
- Constructor: cost `1`, type `SKILL`, color `RED`, rarity `UNCOMMON`, target
  `NONE`, `exhaust = true`.
- `use`: creates a truly random combat Attack, copies it, sets cost for turn to
  `0`, then queues `MakeTempCardInHandAction(c, true)`.
- `upgrade`: `upgradeBaseCost(0)`.

Rust result:
- Definition matches Java constructor and exhaust flag.
- Fixed upgraded base cost override for `InfernalBlade+` to `0`.
- Runtime already creates a random Attack in hand with `cost_for_turn = Some(0)`.

Coverage:
- `ironclad_hp_loss_and_generated_attack_definitions_match_java_sources`
- `ironclad_hp_loss_and_generated_attack_runtime_actions_match_java_use_methods`

## Batch 12 - Strength / Weak / Hybrid Attack / Block-Damage Power Coverage

### Inflame

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Inflame.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/inflame.rs`

Java evidence:
- Constructor: cost `1`, type `POWER`, color `RED`, rarity `UNCOMMON`, target
  `SELF`, `baseMagicNumber = magicNumber = 2`.
- `use`: VFX only, then applies Strength equal to `this.magicNumber`.
- `upgrade`: `upgradeMagicNumber(1)`.

Rust result:
- Definition matches Java constructor and upgrade magic.
- Runtime now evaluates magic at play time before applying Strength.

Coverage:
- `ironclad_power_and_hybrid_attack_definitions_match_java_sources`
- `ironclad_power_and_hybrid_attack_runtime_actions_match_java_use_methods`

### Intimidate

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Intimidate.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/intimidate.rs`

Java evidence:
- Constructor: cost `0`, type `SKILL`, color `RED`, rarity `UNCOMMON`, target
  `ALL_ENEMY`, `exhaust = true`, `baseMagicNumber = magicNumber = 1`.
- `use`: SFX/VFX only, then applies Weak equal to `this.magicNumber` to each
  room monster.
- `upgrade`: `upgradeMagicNumber(1)`.

Rust result:
- Definition matches Java constructor, exhaust flag, target, and upgrade magic.
- Runtime now evaluates magic at play time before applying Weak.

Coverage:
- `ironclad_power_and_hybrid_attack_definitions_match_java_sources`
- `ironclad_power_and_hybrid_attack_runtime_actions_match_java_use_methods`

### Iron Wave

Status: `exact`

Java source:
- `D:/rust/cardcrawl/cards/red/IronWave.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/iron_wave.rs`

Java evidence:
- Constructor: cost `1`, type `ATTACK`, color `RED`, rarity `COMMON`, target
  `ENEMY`, `baseDamage = 5`, `baseBlock = 5`.
- `use`: queues GainBlock, waits/VFX, then queues damage.
- `upgrade`: `upgradeDamage(2)` and `upgradeBlock(2)`.

Rust result:
- Definition matches Java constructor and upgrades.
- Runtime already evaluates block and damage at play time and emits GainBlock
  before damage. Wait/VFX actions are presentation only.

Coverage:
- `ironclad_power_and_hybrid_attack_definitions_match_java_sources`
- `ironclad_power_and_hybrid_attack_runtime_actions_match_java_use_methods`

### Juggernaut

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Juggernaut.java`
- `D:/rust/cardcrawl/powers/JuggernautPower.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/juggernaut.rs`
- `src/content/powers/ironclad/juggernaut.rs`

Java evidence:
- Constructor: cost `2`, type `POWER`, color `RED`, rarity `RARE`, target
  `SELF`, `baseMagicNumber = magicNumber = 5`.
- `use`: applies `JuggernautPower(p, this.magicNumber)`.
- `upgrade`: `upgradeMagicNumber(2)`.
- `JuggernautPower.onGainedBlock`: when `blockAmount > 0`, queues random enemy
  damage with `DamageType.THORNS` and amount equal to the power amount.

Rust result:
- Definition matches Java constructor and upgrade magic.
- Runtime now evaluates magic at play time before applying Juggernaut.
- Existing block-gained hook already only runs for positive gained block and
  emits random enemy Thorns damage with target modifiers disabled.

Coverage:
- `ironclad_power_and_hybrid_attack_definitions_match_java_sources`
- `ironclad_power_and_hybrid_attack_runtime_actions_match_java_use_methods`
- `juggernaut_block_hook_matches_java_source`

## Batch 13 - Limit / Metallicize / Offering / Strike Count Coverage

### Limit Break

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/LimitBreak.java`
- `D:/rust/cardcrawl/actions/unique/LimitBreakAction.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/runtime_impl.rs`
- `src/content/cards/ironclad/limit_break.rs`
- `src/engine/action_handlers/damage.rs`

Java evidence:
- Constructor: cost `1`, type `SKILL`, color `RED`, rarity `RARE`, target
  `SELF`, `exhaust = true`.
- `use`: queues `LimitBreakAction()`.
- `upgrade`: sets `exhaust = false`.
- `LimitBreakAction.update`: if player has Strength, gets current Strength
  amount and queues `ApplyPowerAction(player, player, new StrengthPower(player,
  strAmt), strAmt)`.

Rust result:
- Definition matches Java constructor.
- Existing `exhausts_when_played` correctly makes `LimitBreak+` non-exhausting.
- Runtime already emits `Action::LimitBreak`.
- Engine handler now routes the doubling through the normal ApplyPower handler
  instead of directly mutating Strength amount.

Coverage:
- `ironclad_limit_and_strike_scaling_definitions_match_java_sources`
- `ironclad_limit_and_strike_scaling_runtime_actions_match_java_use_methods`
- `limit_break_and_metallicize_hooks_match_java_sources`

### Metallicize

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Metallicize.java`
- `D:/rust/cardcrawl/powers/MetallicizePower.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/metallicize.rs`
- `src/content/powers/ironclad/metallicize.rs`

Java evidence:
- Constructor: cost `1`, type `POWER`, color `RED`, rarity `UNCOMMON`, target
  `SELF`, `baseMagicNumber = magicNumber = 3`.
- `use`: applies `MetallicizePower(p, this.magicNumber)`.
- `upgrade`: `upgradeMagicNumber(1)`.
- `MetallicizePower.atEndOfTurnPreEndTurnCards`: queues `GainBlockAction(owner,
  owner, amount)`.

Rust result:
- Definition matches Java constructor and upgrade magic.
- Runtime now evaluates magic at play time before applying Metallicize.
- Existing end-of-turn hook queues GainBlock for the power amount.

Coverage:
- `ironclad_limit_and_strike_scaling_definitions_match_java_sources`
- `ironclad_limit_and_strike_scaling_runtime_actions_match_java_use_methods`
- `limit_break_and_metallicize_hooks_match_java_sources`

### Offering

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Offering.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/offering.rs`

Java evidence:
- Constructor: cost `0`, type `SKILL`, color `RED`, rarity `RARE`, target
  `SELF`, `exhaust = true`, `baseMagicNumber = magicNumber = 3`.
- `use`: VFX only, then queues LoseHP `6`, gain energy `2`, and draw
  `this.magicNumber`.
- `upgrade`: `upgradeMagicNumber(2)`.

Rust result:
- Definition matches Java constructor, exhaust flag, and upgrade magic.
- Runtime now evaluates magic at play time before emitting DrawCards.
- Existing HP-loss action keeps `triggers_rupture = true`, matching the Java
  `LoseHPAction(p, p, 6)` path.

Coverage:
- `ironclad_limit_and_strike_scaling_definitions_match_java_sources`
- `ironclad_limit_and_strike_scaling_runtime_actions_match_java_use_methods`

### Perfected Strike

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/PerfectedStrike.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/runtime_impl.rs`
- `src/content/cards/ironclad/perfected_strike.rs`

Java evidence:
- Constructor: cost `2`, type `ATTACK`, color `RED`, rarity `COMMON`, target
  `ENEMY`, `baseDamage = 6`, `baseMagicNumber = magicNumber = 2`, tag `STRIKE`.
- `countCards`: counts Strike-tagged cards in hand, draw pile, and discard pile.
  It does not count limbo.
- `applyPowers` / `calculateCardDamage`: temporarily adds `magicNumber *
  countCards()` to base damage, runs normal calculation, then restores base
  damage.
- `upgrade`: `upgradeMagicNumber(1)`.

Rust result:
- Definition matches Java constructor, Strike tag, and upgrade magic.
- Runtime already evaluates damage at play time.
- Fixed card evaluation so Perfected Strike counts only hand/draw/discard
  Strike cards plus the card itself, not other limbo Strike cards.

Coverage:
- `ironclad_limit_and_strike_scaling_definitions_match_java_sources`
- `ironclad_limit_and_strike_scaling_runtime_actions_match_java_use_methods`

## Batch 14 - Multi-Hit / Wound Generation / Rage Timing Coverage

### Pommel Strike

Status: `exact`

Java source:
- `D:/rust/cardcrawl/cards/red/PommelStrike.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/pommel_strike.rs`

Java evidence:
- Constructor: cost `1`, type `ATTACK`, color `RED`, rarity `COMMON`, target
  `ENEMY`, `baseDamage = 9`, `baseMagicNumber = magicNumber = 1`, tag `STRIKE`.
- `use`: queues damage, then draw `this.magicNumber`.
- `upgrade`: `upgradeDamage(1)` and `upgradeMagicNumber(1)`.

Rust result:
- Definition matches Java constructor, Strike tag, and upgrades.
- Runtime already evaluates damage/magic at play time and emits damage before
  draw.

Coverage:
- `ironclad_multi_hit_and_rage_definitions_match_java_sources`
- `ironclad_multi_hit_and_rage_runtime_actions_match_java_use_methods`

### Power Through

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/PowerThrough.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/power_through.rs`

Java evidence:
- Constructor: cost `1`, type `SKILL`, color `RED`, rarity `UNCOMMON`, target
  `SELF`, `baseBlock = 15`, preview card `Wound`.
- `use`: queues `MakeTempCardInHandAction(new Wound(), 2)`, then GainBlock.
- `upgrade`: `upgradeBlock(5)`.

Rust result:
- Definition matches Java constructor and upgrade block.
- Runtime now evaluates block at play time before emitting GainBlock.
- Existing generated-card action adds two unupgraded Wounds to hand before block.

Coverage:
- `ironclad_multi_hit_and_rage_definitions_match_java_sources`
- `ironclad_multi_hit_and_rage_runtime_actions_match_java_use_methods`

### Pummel

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Pummel.java`
- `D:/rust/cardcrawl/actions/common/PummelDamageAction.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/pummel.rs`

Java evidence:
- Constructor: cost `1`, type `ATTACK`, color `RED`, rarity `UNCOMMON`, target
  `ENEMY`, `baseDamage = 2`, `exhaust = true`,
  `baseMagicNumber = magicNumber = 4`.
- `use`: queues `magicNumber - 1` light Pummel damage actions, then one ordinary
  DamageAction. Gameplay damage is one hit per magic number.
- `upgrade`: `upgradeMagicNumber(1)`.

Rust result:
- Definition matches Java constructor, exhaust flag, and upgrade magic.
- Runtime now evaluates damage and magic at play time before emitting one damage
  action per hit.

Coverage:
- `ironclad_multi_hit_and_rage_definitions_match_java_sources`
- `ironclad_multi_hit_and_rage_runtime_actions_match_java_use_methods`

### Rage

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Rage.java`
- `D:/rust/cardcrawl/powers/RagePower.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/rage.rs`
- `src/content/powers/core/rage.rs`
- `src/content/powers/mod.rs`

Java evidence:
- Constructor: cost `0`, type `SKILL`, color `RED`, rarity `UNCOMMON`, target
  `SELF`, `baseMagicNumber = magicNumber = 3`.
- `use`: SFX/VFX only, then applies `RagePower(p, this.magicNumber)`.
- `upgrade`: `upgradeMagicNumber(2)`.
- `RagePower.onUseCard`: if the played card is an Attack, queues GainBlock for
  the Rage amount.
- `RagePower.atEndOfTurn`: removes Rage. It is not a start-of-turn removal.

Rust result:
- Fixed definition target to `SELF`.
- Runtime now evaluates magic at play time before applying Rage.
- Existing attack-only block hook matches Java.
- Fixed power lifecycle so Rage removes at end of turn instead of start of turn.

Coverage:
- `ironclad_multi_hit_and_rage_definitions_match_java_sources`
- `ironclad_multi_hit_and_rage_runtime_actions_match_java_use_methods`
- `rage_power_hooks_match_java_source`

## Batch 15 - Rampage / Reaper / Reckless Charge / Rupture Coverage

### Rampage

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Rampage.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/rampage.rs`
- `src/engine/action_handlers/cards.rs`

Java evidence:
- Constructor: cost `1`, type `ATTACK`, color `RED`, rarity `UNCOMMON`, target
  `ENEMY`, `baseDamage = 8`, `baseMagicNumber = magicNumber = 5`.
- `use`: queues DamageAction using `this.damage`, then
  `ModifyDamageAction(this.uuid, this.magicNumber)`.
- `upgrade`: `upgradeMagicNumber(3)`.

Rust result:
- Definition matches Java constructor and upgrade magic.
- Runtime now evaluates damage and magic at play time before emitting damage and
  `ModifyCardDamage`.
- Existing `ModifyCardDamage` targets the concrete card UUID across combat piles
  and queued card state, matching Java's UUID-based `ModifyDamageAction`.

Coverage:
- `ironclad_rampage_and_rupture_definitions_match_java_sources`
- `ironclad_rampage_and_rupture_runtime_actions_match_java_use_methods`

### Reaper

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Reaper.java`
- `D:/rust/cardcrawl/actions/unique/VampireDamageAllEnemiesAction.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/reaper.rs`
- `src/engine/action_handlers/damage.rs`

Java evidence:
- Constructor: cost `2`, type `ATTACK`, color `RED`, rarity `RARE`, target
  `ALL_ENEMY`, `baseDamage = 4`, `isMultiDamage = true`, `exhaust = true`, tag
  `HEALING`.
- `use`: queues VFX only, then `VampireDamageAllEnemiesAction(p,
  this.multiDamage, this.damageTypeForTurn, NONE)`.
- `VampireDamageAllEnemiesAction`: iterates the monster list by index, skips
  dying/dead/escaping monsters, applies `damage[i]`, and heals the source for
  total HP lost.
- `upgrade`: `upgradeDamage(1)`.

Rust result:
- Definition matches Java constructor, exhaust flag, Healing tag, and upgrade
  damage.
- Runtime now evaluates multi-damage at play time before emitting
  `VampireDamageAllEnemies`.
- Fixed `VampireDamageAllEnemies` execution to target the monster list entries
  that correspond to the damage array, instead of assuming monster IDs are
  `index + 1`; escaping monsters are also skipped.

Coverage:
- `ironclad_rampage_and_rupture_definitions_match_java_sources`
- `ironclad_rampage_and_rupture_runtime_actions_match_java_use_methods`
- `rupture_and_reaper_execution_hooks_match_java_sources`

### Reckless Charge

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/RecklessCharge.java`
- `D:/rust/cardcrawl/actions/common/MakeTempCardInDrawPileAction.java`
- `D:/rust/cardcrawl/vfx/cardManip/ShowCardAndAddToDrawPileEffect.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/reckless_charge.rs`

Java evidence:
- Constructor: cost `0`, type `ATTACK`, color `RED`, rarity `UNCOMMON`, target
  `ENEMY`, `baseDamage = 7`, preview card `Dazed`.
- `use`: queues DamageAction using `this.damage`, then
  `MakeTempCardInDrawPileAction(new Dazed(), 1, true, true)`.
- The four-argument draw-pile constructor maps to `randomSpot = true`,
  `autoPosition = true`, `toBottom = false`.
- `ShowCardAndAddToDrawPileEffect` applies draw-pile placement immediately:
  `toBottom` -> bottom, `randomSpot` -> random spot, otherwise top. The
  `autoPosition` argument is visual-only.
- Generated `Dazed` is a Status, so Master Reality does not upgrade it.
- `upgrade`: `upgradeDamage(3)`.

Rust result:
- Definition matches Java constructor and upgrade damage.
- Runtime now evaluates damage at play time before emitting DamageAction.
- Generated `Dazed` action uses `random_spot = true`, `to_bottom = false`, and
  `upgraded = false`. No UI positioning state is migrated.

Coverage:
- `ironclad_rampage_and_rupture_definitions_match_java_sources`
- `ironclad_rampage_and_rupture_runtime_actions_match_java_use_methods`

### Rupture

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Rupture.java`
- `D:/rust/cardcrawl/powers/RupturePower.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/rupture.rs`
- `src/content/powers/ironclad/rupture.rs`
- `src/content/powers/mod.rs`

Java evidence:
- Constructor: cost `1`, type `POWER`, color `RED`, rarity `UNCOMMON`, target
  `SELF`, `baseMagicNumber = magicNumber = 1`.
- `use`: applies `RupturePower(p, this.magicNumber)`.
- `RupturePower.wasHPLost`: if `damageAmount > 0` and `info.owner == owner`,
  queues Strength gain equal to Rupture amount.
- `upgrade`: `upgradeMagicNumber(1)`.

Rust result:
- Definition matches Java constructor and upgrade magic.
- Runtime now evaluates magic at play time before applying Rupture.
- Existing HP-loss provenance flag remains the Rust migration boundary for
  Java's `info.owner == owner` condition; tests lock that Rupture fires only for
  player-authored self HP-loss paths marked with `triggers_rupture`.

Coverage:
- `ironclad_rampage_and_rupture_definitions_match_java_sources`
- `ironclad_rampage_and_rupture_runtime_actions_match_java_use_methods`
- `rupture_and_reaper_execution_hooks_match_java_sources`

## Batch 16 - Upgrade Scaling / Exhaust Utility / Energy Coverage

### Searing Blow

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/SearingBlow.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/runtime_impl.rs`
- `src/content/cards/ironclad/searing_blow.rs`

Java evidence:
- Constructor: cost `2`, type `ATTACK`, color `RED`, rarity `UNCOMMON`, target
  `ENEMY`, `baseDamage = 12`, `timesUpgraded = upgrades`.
- `use`: VFX only, then DamageAction using `this.damage`.
- `upgrade`: `upgradeDamage(4 + this.timesUpgraded)`, increments
  `timesUpgraded`, sets upgraded/name/title, and `canUpgrade()` always returns
  true.
- Closed-form damage for `n` upgrades is `12 + n * (n + 7) / 2`.

Rust result:
- Definition matches Java constructor.
- Existing upgradeability paths already special-case Searing Blow as always
  upgradeable.
- Runtime already uses the closed-form Searing Blow damage formula in card
  evaluation; play now evaluates at use time before emitting DamageAction.

Coverage:
- `ironclad_upgrade_and_exhaust_utility_definitions_match_java_sources`
- `ironclad_upgrade_and_exhaust_utility_runtime_actions_match_java_use_methods`

### Second Wind

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/SecondWind.java`
- `D:/rust/cardcrawl/actions/unique/BlockPerNonAttackAction.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/second_wind.rs`
- `src/engine/action_handlers/damage.rs`

Java evidence:
- Constructor: cost `1`, type `SKILL`, color `RED`, rarity `UNCOMMON`, target
  `SELF`, `baseBlock = 5`.
- `use`: queues `BlockPerNonAttackAction(this.block)`.
- `BlockPerNonAttackAction`: snapshots non-Attack cards in hand, queues one
  GainBlock per card, then queues one ExhaustSpecificCardAction per card with
  `addToTop`, so exhaust actions resolve before block actions.
- `upgrade`: `upgradeBlock(2)`.

Rust result:
- Definition matches Java constructor and upgrade block.
- Runtime now evaluates block at play time before emitting `BlockPerNonAttack`.
- Existing handler snapshots non-Attack hand cards, queues exhaust actions before
  one GainBlock per exhausted card, preserving the important exhaust-before-block
  behavior for Feel No Pain, Juggernaut, Sentinel, and related hooks.

Coverage:
- `ironclad_upgrade_and_exhaust_utility_definitions_match_java_sources`
- `ironclad_upgrade_and_exhaust_utility_runtime_actions_match_java_use_methods`

### Seeing Red

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/SeeingRed.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/runtime_impl.rs`
- `src/content/cards/ironclad/seeing_red.rs`

Java evidence:
- Constructor: cost `1`, type `SKILL`, color `RED`, rarity `UNCOMMON`, target
  `NONE`, `exhaust = true`.
- `use`: queues `GainEnergyAction(2)`.
- `upgrade`: `upgradeBaseCost(0)`.

Rust result:
- Definition matches Java constructor and exhaust flag.
- Runtime action already emits GainEnergy 2.
- Added upgraded base-cost override so Seeing Red+ costs 0 instead of relying on
  nonexistent damage/block/magic upgrade fields.

Coverage:
- `ironclad_upgrade_and_exhaust_utility_definitions_match_java_sources`
- `ironclad_upgrade_and_exhaust_utility_runtime_actions_match_java_use_methods`

### Sentinel

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Sentinel.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/sentinel.rs`
- `src/engine/action_handlers/cards.rs`

Java evidence:
- Constructor: cost `1`, type `SKILL`, color `RED`, rarity `UNCOMMON`, target
  `SELF`, `baseBlock = 5`; it does not define base magic.
- `use`: queues GainBlock using `this.block`.
- `triggerOnExhaust`: `addToTop(new GainEnergyAction(2))`, or 3 if upgraded.
- `upgrade`: `upgradeBlock(3)` and description change only.

Rust result:
- Fixed definition base magic from fake `2` to `0`; energy-on-exhaust is card
  hook behavior, not intrinsic magic.
- Runtime now evaluates block at play time before emitting GainBlock.
- Fixed Sentinel's exhaust hook insertion mode to `AddTo::Top`, matching Java
  `addToTop`.
- Reordered generic exhaust trigger collection to call relic hooks, then power
  hooks, then card-specific `triggerOnExhaust`, matching Java
  `CardGroup.moveToExhaustPile`.
- Added a Sentinel + Feel No Pain exhaustion check to lock the Java-visible
  order: Sentinel energy resolves before the bottom-queued Feel No Pain block.

Coverage:
- `ironclad_upgrade_and_exhaust_utility_definitions_match_java_sources`
- `ironclad_upgrade_and_exhaust_utility_runtime_actions_match_java_use_methods`
- `sentinel_exhaust_trigger_matches_java_add_to_top_energy`

## Batch 17 - Sever Soul / Shockwave / Shrug It Off / Spot Weakness Coverage

### Sever Soul

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/SeverSoul.java`
- `D:/rust/cardcrawl/actions/unique/ExhaustAllNonAttackAction.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/sever_soul.rs`
- `src/engine/action_handlers/damage.rs`

Java evidence:
- Constructor: cost `2`, type `ATTACK`, color `RED`, rarity `UNCOMMON`, target
  `ENEMY`, `baseDamage = 16`.
- `use`: queues `ExhaustAllNonAttackAction()`, then DamageAction using
  `this.damage`.
- `ExhaustAllNonAttackAction`: iterates the current hand and `addToTop`s one
  ExhaustSpecificCardAction for each non-Attack card. Those exhausts therefore
  resolve before the following queued DamageAction.
- `upgrade`: `upgradeDamage(6)`.

Rust result:
- Definition matches Java constructor and upgrade damage.
- Runtime now evaluates damage at play time before emitting DamageAction.
- Fixed `ExhaustAllNonAttack` to queue exhausted cards to the front, preserving
  Java's exhaust-before-following-damage order.

Coverage:
- `ironclad_exhaust_debuff_and_intent_definitions_match_java_sources`
- `ironclad_exhaust_debuff_and_intent_runtime_actions_match_java_use_methods`
- `sever_soul_exhaust_all_non_attack_queues_exhausts_before_following_damage`

### Shockwave

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/Shockwave.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/shockwave.rs`

Java evidence:
- Constructor: cost `2`, type `SKILL`, color `RED`, rarity `UNCOMMON`, target
  `ALL_ENEMY`, `exhaust = true`,
  `baseMagicNumber = magicNumber = 3`.
- `use`: for every monster in the room, queues Weak then Vulnerable for
  `this.magicNumber`.
- `upgrade`: `upgradeMagicNumber(2)`.

Rust result:
- Definition matches Java constructor, exhaust flag, and upgrade magic.
- Runtime now evaluates magic at play time before applying Weak/Vulnerable.
- Existing implementation applies Weak then Vulnerable per monster, matching the
  Java loop order. It does not migrate UI-only attack effects.

Coverage:
- `ironclad_exhaust_debuff_and_intent_definitions_match_java_sources`
- `ironclad_exhaust_debuff_and_intent_runtime_actions_match_java_use_methods`

### Shrug It Off

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/ShrugItOff.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/shrug_it_off.rs`

Java evidence:
- Constructor: cost `1`, type `SKILL`, color `RED`, rarity `COMMON`, target
  `SELF`, `baseBlock = 8`.
- `use`: queues GainBlock using `this.block`, then DrawCardAction 1.
- `upgrade`: `upgradeBlock(3)`.

Rust result:
- Fixed definition base magic from fake draw count `1` to `0`; draw 1 is an
  action constant, not Java card magic.
- Runtime now evaluates block at play time and then emits DrawCards(1).

Coverage:
- `ironclad_exhaust_debuff_and_intent_definitions_match_java_sources`
- `ironclad_exhaust_debuff_and_intent_runtime_actions_match_java_use_methods`

### Spot Weakness

Status: `wrong-fixed`

Java source:
- `D:/rust/cardcrawl/cards/red/SpotWeakness.java`
- `D:/rust/cardcrawl/actions/unique/SpotWeaknessAction.java`

Rust source:
- `src/content/cards/mod.rs`
- `src/content/cards/ironclad/spot_weakness.rs`
- `src/engine/targeting.rs`

Java evidence:
- Constructor: cost `1`, type `SKILL`, color `RED`, rarity `UNCOMMON`, target
  `SELF_AND_ENEMY`, `baseMagicNumber = magicNumber = 3`.
- `use`: queues `SpotWeaknessAction(this.magicNumber, m)`.
- `SpotWeaknessAction`: if the target monster exists and
  `getIntentBaseDmg() >= 0`, applies Strength to the player; otherwise it only
  shows a ThoughtBubble.
- `upgrade`: `upgradeMagicNumber(1)`.

Rust result:
- Added `CardTarget::SelfAndEnemy` and mapped it to enemy target validation.
- Fixed Spot Weakness definition target from `Enemy` to `SelfAndEnemy`.
- Runtime now evaluates magic at play time and applies Strength only when the
  selected monster's resolved visible turn plan contains an attack.

Coverage:
- `ironclad_exhaust_debuff_and_intent_definitions_match_java_sources`
- `ironclad_exhaust_debuff_and_intent_runtime_actions_match_java_use_methods`

## Full Ironclad Queue

Cards remain `unreviewed` until their Java file, Rust definition, Rust runtime,
and supporting engine behavior have all been checked.

| # | Java card file | Rust card module | Status |
|---:|---|---|---|
| 1 | `Strike_Red.java` | `strike.rs` | `exact` |
| 2 | `Defend_Red.java` | `defend.rs` | `wrong-fixed` |
| 3 | `Bash.java` | `bash.rs` | `exact` |
| 4 | `Anger.java` | `anger.rs` | `wrong-fixed` |
| 5 | `Armaments.java` | `armaments.rs` | `wrong-fixed` |
| 6 | `Barricade.java` | `barricade.rs` | `wrong-fixed` |
| 7 | `BattleTrance.java` | `battle_trance.rs` | `wrong-fixed` |
| 8 | `Berserk.java` | `berserk.rs` | `wrong-fixed` |
| 9 | `BloodForBlood.java` | `blood_for_blood.rs` | `wrong-fixed` |
| 10 | `Bloodletting.java` | `bloodletting.rs` | `wrong-fixed` |
| 11 | `Bludgeon.java` | `bludgeon.rs` | `wrong-fixed` |
| 12 | `BodySlam.java` | `body_slam.rs` | `wrong-fixed` |
| 13 | `Brutality.java` | `brutality.rs` | `wrong-fixed` |
| 14 | `BurningPact.java` | `burning_pact.rs` | `wrong-fixed` |
| 15 | `Carnage.java` | `carnage.rs` | `wrong-fixed` |
| 16 | `Clash.java` | `clash.rs` | `wrong-fixed` |
| 17 | `Cleave.java` | `cleave.rs` | `wrong-fixed` |
| 18 | `Clothesline.java` | `clothesline.rs` | `wrong-fixed` |
| 19 | `Combust.java` | `combust.rs` | `wrong-fixed` |
| 20 | `Corruption.java` | `corruption.rs` | `wrong-fixed` |
| 21 | `DarkEmbrace.java` | `dark_embrace.rs` | `wrong-fixed` |
| 22 | `DemonForm.java` | `demon_form.rs` | `wrong-fixed` |
| 23 | `Disarm.java` | `disarm.rs` | `wrong-fixed` |
| 24 | `DoubleTap.java` | `double_tap.rs` | `wrong-fixed` |
| 25 | `Dropkick.java` | `dropkick.rs` | `wrong-fixed` |
| 26 | `DualWield.java` | `dual_wield.rs` | `wrong-fixed` |
| 27 | `Entrench.java` | `entrench.rs` | `wrong-fixed` |
| 28 | `Evolve.java` | `evolve.rs` | `wrong-fixed` |
| 29 | `Exhume.java` | `exhume.rs` | `wrong-fixed` |
| 30 | `Feed.java` | `feed.rs` | `wrong-fixed` |
| 31 | `FeelNoPain.java` | `feel_no_pain.rs` | `wrong-fixed` |
| 32 | `FiendFire.java` | `fiend_fire.rs` | `wrong-fixed` |
| 33 | `FireBreathing.java` | `fire_breathing.rs` | `wrong-fixed` |
| 34 | `FlameBarrier.java` | `flame_barrier.rs` | `wrong-fixed` |
| 35 | `Flex.java` | `flex.rs` | `wrong-fixed` |
| 36 | `GhostlyArmor.java` | `ghostly_armor.rs` | `wrong-fixed` |
| 37 | `Havoc.java` | `havoc.rs` | `wrong-fixed` |
| 38 | `Headbutt.java` | `headbutt.rs` | `wrong-fixed` |
| 39 | `HeavyBlade.java` | `heavy_blade.rs` | `wrong-fixed` |
| 40 | `Hemokinesis.java` | `hemokinesis.rs` | `wrong-fixed` |
| 41 | `Immolate.java` | `immolate.rs` | `wrong-fixed` |
| 42 | `Impervious.java` | `impervious.rs` | `wrong-fixed` |
| 43 | `InfernalBlade.java` | `infernal_blade.rs` | `wrong-fixed` |
| 44 | `Inflame.java` | `inflame.rs` | `wrong-fixed` |
| 45 | `Intimidate.java` | `intimidate.rs` | `wrong-fixed` |
| 46 | `IronWave.java` | `iron_wave.rs` | `exact` |
| 47 | `Juggernaut.java` | `juggernaut.rs` | `wrong-fixed` |
| 48 | `LimitBreak.java` | `limit_break.rs` | `wrong-fixed` |
| 49 | `Metallicize.java` | `metallicize.rs` | `wrong-fixed` |
| 50 | `Offering.java` | `offering.rs` | `wrong-fixed` |
| 51 | `PerfectedStrike.java` | `perfected_strike.rs` | `wrong-fixed` |
| 52 | `PommelStrike.java` | `pommel_strike.rs` | `exact` |
| 53 | `PowerThrough.java` | `power_through.rs` | `wrong-fixed` |
| 54 | `Pummel.java` | `pummel.rs` | `wrong-fixed` |
| 55 | `Rage.java` | `rage.rs` | `wrong-fixed` |
| 56 | `Rampage.java` | `rampage.rs` | `wrong-fixed` |
| 57 | `Reaper.java` | `reaper.rs` | `wrong-fixed` |
| 58 | `RecklessCharge.java` | `reckless_charge.rs` | `wrong-fixed` |
| 59 | `Rupture.java` | `rupture.rs` | `wrong-fixed` |
| 60 | `SearingBlow.java` | `searing_blow.rs` | `wrong-fixed` |
| 61 | `SecondWind.java` | `second_wind.rs` | `wrong-fixed` |
| 62 | `SeeingRed.java` | `seeing_red.rs` | `wrong-fixed` |
| 63 | `Sentinel.java` | `sentinel.rs` | `wrong-fixed` |
| 64 | `SeverSoul.java` | `sever_soul.rs` | `wrong-fixed` |
| 65 | `Shockwave.java` | `shockwave.rs` | `wrong-fixed` |
| 66 | `ShrugItOff.java` | `shrug_it_off.rs` | `wrong-fixed` |
| 67 | `SpotWeakness.java` | `spot_weakness.rs` | `wrong-fixed` |
| 68 | `SwordBoomerang.java` | `sword_boomerang.rs` | `unreviewed` |
| 69 | `ThunderClap.java` | `thunderclap.rs` | `unreviewed` |
| 70 | `TrueGrit.java` | `true_grit.rs` | `unreviewed` |
| 71 | `TwinStrike.java` | `twin_strike.rs` | `unreviewed` |
| 72 | `Uppercut.java` | `uppercut.rs` | `unreviewed` |
| 73 | `Warcry.java` | `warcry.rs` | `unreviewed` |
| 74 | `Whirlwind.java` | `whirlwind.rs` | `unreviewed` |
| 75 | `WildStrike.java` | `wild_strike.rs` | `unreviewed` |
