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
| 12 | `BodySlam.java` | `body_slam.rs` | `unreviewed` |
| 13 | `Brutality.java` | `brutality.rs` | `unreviewed` |
| 14 | `BurningPact.java` | `burning_pact.rs` | `unreviewed` |
| 15 | `Carnage.java` | `carnage.rs` | `unreviewed` |
| 16 | `Clash.java` | `clash.rs` | `unreviewed` |
| 17 | `Cleave.java` | `cleave.rs` | `unreviewed` |
| 18 | `Clothesline.java` | `clothesline.rs` | `unreviewed` |
| 19 | `Combust.java` | `combust.rs` | `unreviewed` |
| 20 | `Corruption.java` | `corruption.rs` | `unreviewed` |
| 21 | `DarkEmbrace.java` | `dark_embrace.rs` | `unreviewed` |
| 22 | `DemonForm.java` | `demon_form.rs` | `unreviewed` |
| 23 | `Disarm.java` | `disarm.rs` | `unreviewed` |
| 24 | `DoubleTap.java` | `double_tap.rs` | `unreviewed` |
| 25 | `Dropkick.java` | `dropkick.rs` | `unreviewed` |
| 26 | `DualWield.java` | `dual_wield.rs` | `unreviewed` |
| 27 | `Entrench.java` | `entrench.rs` | `unreviewed` |
| 28 | `Evolve.java` | `evolve.rs` | `unreviewed` |
| 29 | `Exhume.java` | `exhume.rs` | `unreviewed` |
| 30 | `Feed.java` | `feed.rs` | `unreviewed` |
| 31 | `FeelNoPain.java` | `feel_no_pain.rs` | `unreviewed` |
| 32 | `FiendFire.java` | `fiend_fire.rs` | `unreviewed` |
| 33 | `FireBreathing.java` | `fire_breathing.rs` | `unreviewed` |
| 34 | `FlameBarrier.java` | `flame_barrier.rs` | `unreviewed` |
| 35 | `Flex.java` | `flex.rs` | `unreviewed` |
| 36 | `GhostlyArmor.java` | `ghostly_armor.rs` | `unreviewed` |
| 37 | `Havoc.java` | `havoc.rs` | `unreviewed` |
| 38 | `Headbutt.java` | `headbutt.rs` | `unreviewed` |
| 39 | `HeavyBlade.java` | `heavy_blade.rs` | `unreviewed` |
| 40 | `Hemokinesis.java` | `hemokinesis.rs` | `unreviewed` |
| 41 | `Immolate.java` | `immolate.rs` | `unreviewed` |
| 42 | `Impervious.java` | `impervious.rs` | `unreviewed` |
| 43 | `InfernalBlade.java` | `infernal_blade.rs` | `unreviewed` |
| 44 | `Inflame.java` | `inflame.rs` | `unreviewed` |
| 45 | `Intimidate.java` | `intimidate.rs` | `unreviewed` |
| 46 | `IronWave.java` | `iron_wave.rs` | `unreviewed` |
| 47 | `Juggernaut.java` | `juggernaut.rs` | `unreviewed` |
| 48 | `LimitBreak.java` | `limit_break.rs` | `unreviewed` |
| 49 | `Metallicize.java` | `metallicize.rs` | `unreviewed` |
| 50 | `Offering.java` | `offering.rs` | `unreviewed` |
| 51 | `PerfectedStrike.java` | `perfected_strike.rs` | `unreviewed` |
| 52 | `PommelStrike.java` | `pommel_strike.rs` | `unreviewed` |
| 53 | `PowerThrough.java` | `power_through.rs` | `unreviewed` |
| 54 | `Pummel.java` | `pummel.rs` | `unreviewed` |
| 55 | `Rage.java` | `rage.rs` | `unreviewed` |
| 56 | `Rampage.java` | `rampage.rs` | `unreviewed` |
| 57 | `Reaper.java` | `reaper.rs` | `unreviewed` |
| 58 | `RecklessCharge.java` | `reckless_charge.rs` | `unreviewed` |
| 59 | `Rupture.java` | `rupture.rs` | `unreviewed` |
| 60 | `SearingBlow.java` | `searing_blow.rs` | `unreviewed` |
| 61 | `SecondWind.java` | `second_wind.rs` | `unreviewed` |
| 62 | `SeeingRed.java` | `seeing_red.rs` | `unreviewed` |
| 63 | `Sentinel.java` | `sentinel.rs` | `unreviewed` |
| 64 | `SeverSoul.java` | `sever_soul.rs` | `unreviewed` |
| 65 | `Shockwave.java` | `shockwave.rs` | `unreviewed` |
| 66 | `ShrugItOff.java` | `shrug_it_off.rs` | `unreviewed` |
| 67 | `SpotWeakness.java` | `spot_weakness.rs` | `unreviewed` |
| 68 | `SwordBoomerang.java` | `sword_boomerang.rs` | `unreviewed` |
| 69 | `ThunderClap.java` | `thunderclap.rs` | `unreviewed` |
| 70 | `TrueGrit.java` | `true_grit.rs` | `unreviewed` |
| 71 | `TwinStrike.java` | `twin_strike.rs` | `unreviewed` |
| 72 | `Uppercut.java` | `uppercut.rs` | `unreviewed` |
| 73 | `Warcry.java` | `warcry.rs` | `unreviewed` |
| 74 | `Whirlwind.java` | `whirlwind.rs` | `unreviewed` |
| 75 | `WildStrike.java` | `wild_strike.rs` | `unreviewed` |
