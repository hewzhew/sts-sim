# Relic Implementation Audit

**Total**: 33 | **Implemented**: 0 | **Missing**: 33


## Starter (0/1)

| Relic | Java ID | Hooks | Status | Location |
|-------|---------|-------|--------|----------|
| CrackedCore | Cracked Core | atPreBattle | ⚠️ ref-only | relics.rs:L1826 |

## Common (0/5)

| Relic | Java ID | Hooks | Status | Location |
|-------|---------|-------|--------|----------|
| PotionBelt | Potion Belt | onEquip, canSpawn | ⚠️ ref-only | - |
| Test5 | Test 5 | onEquip | ❌ missing | - |
| TinyChest | Tiny Chest | onEquip, canSpawn | ❌ missing | - |
| ToyOrnithopter | Toy Ornithopter | onUsePotion | ⚠️ ref-only | - |
| WarPaint | War Paint | onEquip | ❌ missing | - |

## Uncommon (0/6)

| Relic | Java ID | Hooks | Status | Location |
|-------|---------|-------|--------|----------|
| BottledFlame | Bottled Flame | onEquip, onUnequip, update +2 | ❌ missing | - |
| BottledLightning | Bottled Lightning | onEquip, onUnequip, update +2 | ❌ missing | - |
| DarkstonePeriapt | Darkstone Periapt | onObtainCard, canSpawn | ❌ missing | - |
| SymbioticVirus | Symbiotic Virus | atPreBattle | ❌ missing | - |
| Test1 | Test 1 | onUsePotion | ❌ missing | - |
| Test6 | Test 6 | onPlayerEndTurn, atTurnStart, onVictory | ❌ missing | - |

## Rare (0/9)

| Relic | Java ID | Hooks | Status | Location |
|-------|---------|-------|--------|----------|
| ChampionsBelt | Champion Belt | onTrigger | ❌ missing | - |
| EmotionChip | Emotion Chip | atTurnStart, wasHPLost, onVictory | ❌ missing | - |
| LizardTail | Lizard Tail | setCounter, onTrigger | ⚠️ ref-only | - |
| OldCoin | Old Coin | onEquip, canSpawn | ❌ missing | - |
| Test3 | Test 3 | onEquip | ❌ missing | - |
| Test4 | Test 4 | atBattleStart | ❌ missing | - |
| TheSpecimen | The Specimen | onMonsterDeath | ❌ missing | - |
| ToughBandages | Tough Bandages | onManualDiscard | ❌ missing | - |
| UnceasingTop | Unceasing Top | atPreBattle, atTurnStart, onRefreshHand | ❌ missing | - |

## Boss (0/7)

| Relic | Java ID | Hooks | Status | Location |
|-------|---------|-------|--------|----------|
| BlackStar | Black Star | onEnterRoom, onVictory | ❌ missing | - |
| CallingBell | Calling Bell | onEquip, update | ❌ missing | - |
| CursedKey | Cursed Key | justEnteredRoom, onChestOpen, onEquip +1 | ❌ missing | - |
| EmptyCage | Empty Cage | onEquip, update | ❌ missing | - |
| NuclearBattery | Nuclear Battery | atPreBattle | ❌ missing | - |
| PandorasBox | Pandora's Box | onEquip, update | ❌ missing | - |
| TinyHouse | Tiny House | onEquip | ❌ missing | - |

## Shop (0/3)

| Relic | Java ID | Hooks | Status | Location |
|-------|---------|-------|--------|----------|
| DollysMirror | DollysMirror | onEquip, update | ❌ missing | - |
| RunicCapacitor | Runic Capacitor | atPreBattle, atTurnStart | ❌ missing | - |
| Waffle | Lee's Waffle | onEquip | ❌ missing | - |

## Special (0/2)

| Relic | Java ID | Hooks | Status | Location |
|-------|---------|-------|--------|----------|
| Circlet | Circlet | onEquip, onUnequip | ❌ missing | - |
| CultistMask | CultistMask | atBattleStart | ❌ missing | - |

## Missing Combat Relics (33)

| Relic | Tier | Java Hooks | Difficulty |
|-------|------|------------|------------|
| BlackStar | Boss | onEnterRoom, onVictory | 🟢 Easy |
| CallingBell | Boss | onEquip, update | 🔴 Hard |
| CursedKey | Boss | justEnteredRoom, onChestOpen, onEquip, onUnequip | 🔴 Hard |
| EmptyCage | Boss | onEquip, update | 🔴 Hard |
| NuclearBattery | Boss | atPreBattle | 🔴 Hard |
| PandorasBox | Boss | onEquip, update | 🔴 Hard |
| TinyHouse | Boss | onEquip | 🟢 Easy |
| PotionBelt | Common | onEquip, canSpawn | 🟢 Easy |
| Test5 | Common | onEquip | 🟢 Easy |
| TinyChest | Common | onEquip, canSpawn | 🟢 Easy |
| ToyOrnithopter | Common | onUsePotion | 🔴 Hard |
| WarPaint | Common | onEquip | 🟢 Easy |
| ChampionsBelt | Rare | onTrigger | 🔴 Hard |
| EmotionChip | Rare | atTurnStart, wasHPLost, onVictory | 🟡 Medium |
| LizardTail | Rare | setCounter, onTrigger | 🔴 Hard |
| OldCoin | Rare | onEquip, canSpawn | 🟢 Easy |
| Test3 | Rare | onEquip | 🟢 Easy |
| Test4 | Rare | atBattleStart | 🟢 Easy |
| TheSpecimen | Rare | onMonsterDeath | 🔴 Hard |
| ToughBandages | Rare | onManualDiscard | 🔴 Hard |
| UnceasingTop | Rare | atPreBattle, atTurnStart, onRefreshHand | 🔴 Hard |
| DollysMirror | Shop | onEquip, update | 🔴 Hard |
| RunicCapacitor | Shop | atPreBattle, atTurnStart | 🔴 Hard |
| Waffle | Shop | onEquip | 🟢 Easy |
| Circlet | Special | onEquip, onUnequip | 🔴 Hard |
| CultistMask | Special | atBattleStart | 🟢 Easy |
| CrackedCore | Starter | atPreBattle | 🔴 Hard |
| BottledFlame | Uncommon | onEquip, onUnequip, update, atBattleStart | 🔴 Hard |
| BottledLightning | Uncommon | onEquip, onUnequip, update, atBattleStart | 🔴 Hard |
| DarkstonePeriapt | Uncommon | onObtainCard, canSpawn | 🔴 Hard |
| SymbioticVirus | Uncommon | atPreBattle | 🔴 Hard |
| Test1 | Uncommon | onUsePotion | 🔴 Hard |
| Test6 | Uncommon | onPlayerEndTurn, atTurnStart, onVictory | 🟡 Medium |
