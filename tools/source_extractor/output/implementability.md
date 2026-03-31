# Implementability Analysis

Cross-reference of Java entities against Rust implementation status.

## POWER (132 clean, 26 with warnings, 0 blocked)

### With Warnings (potentially incorrect Rust implementation)

**`Collect`** (`CollectPower`)
  - Custom stackPower modifies: 

**`Combust`** (`CombustPower`)
  - Has 1 private field(s): hpLoss

**`Confusion`** (`ConfusionPower`)
  - Uses RNG: cardRandomRng

**`Curl Up`** (`CurlUpPower`)
  - Has 1 private field(s): triggered

**`DevaForm`** (`DevaPower`)
  - Has 1 private field(s): energyGainAmount

**`Dexterity`** (`DexterityPower`)
  - Custom stackPower modifies: 

**`Echo Form`** (`EchoPower`)
  - Has 1 private field(s): cardsDoubledThisTurn

**`Energized`** (`EnergizedPower`)
  - Custom stackPower modifies: 

**`EnergizedBlue`** (`EnergizedBluePower`)
  - Custom stackPower modifies: 

**`Flight`** (`FlightPower`)
  - Has 2 private field(s): storedAmount, calculateDamageTakenAmount

**`Focus`** (`FocusPower`)
  - Custom stackPower modifies: 

**`GrowthPower`** (`GrowthPower`)
  - Has 1 private field(s): skipFirst

**`Hello`** (`HelloPower`)
  - Uses RNG: cardRandomRng

**`Invincible`** (`InvinciblePower`)
  - Has 1 private field(s): maxAmt

**`LikeWaterPower`** (`LikeWaterPower`)
  - Custom stackPower modifies: 

**`Malleable`** (`MalleablePower`)
  - Has 1 private field(s): basePower
  - Custom stackPower modifies: basePower

**`Mayhem`** (`MayhemPower`)
  - Uses RNG: cardRandomRng

**`Panache`** (`PanachePower`)
  - Has 1 private field(s): damage
  - Custom stackPower modifies: damage

**`Plated Armor`** (`PlatedArmorPower`)
  - Custom stackPower modifies: 

**`Rebound`** (`ReboundPower`)
  - Has 1 private field(s): justEvoked

**`RechargingCore`** (`RechargingCorePower`)
  - Has 1 private field(s): turnTimer

**`Ritual`** (`RitualPower`)
  - Has 2 private field(s): skipFirst, onPlayer

**`Shackled`** (`GainStrengthPower`)
  - Custom stackPower modifies: 

**`Strength`** (`StrengthPower`)
  - Custom stackPower modifies: 

**`TheBomb`** (`TheBombPower`)
  - Has 1 private field(s): damage

**`TimeMazePower`** (`TimeMazePower`)
  - Has 1 private field(s): maxAmount

## RELIC (174 clean, 40 with warnings, 0 blocked)

### With Warnings (potentially incorrect Rust implementation)

**`Ancient Tea Set`** (`AncientTeaSet`)
  - Has 2 private field(s): firstTurn, setDescription

**`Art of War`** (`ArtOfWar`)
  - Has 3 private field(s): gainEnergyNext, firstTurn, setDescription

**`Astrolabe`** (`Astrolabe`)
  - Has 1 private field(s): cardsSelected
  - Uses RNG: miscRng

**`Bottled Flame`** (`BottledFlame`)
  - Has 1 private field(s): cardSelected

**`Bottled Lightning`** (`BottledLightning`)
  - Has 1 private field(s): cardSelected

**`Bottled Tornado`** (`BottledTornado`)
  - Has 1 private field(s): cardSelected

**`Busted Crown`** (`BustedCrown`)
  - Has 1 private field(s): setDescription

**`Calling Bell`** (`CallingBell`)
  - Has 1 private field(s): cardsReceived

**`Coffee Dripper`** (`CoffeeDripper`)
  - Has 1 private field(s): setDescription

**`Cursed Key`** (`CursedKey`)
  - Has 1 private field(s): setDescription

**`Dodecahedron`** (`DEPRECATEDDodecahedron`)
  - Has 2 private field(s): setDescription, isActive

**`DollysMirror`** (`DollysMirror`)
  - Has 1 private field(s): cardSelected

**`Ectoplasm`** (`Ectoplasm`)
  - Has 1 private field(s): setDescription

**`Empty Cage`** (`EmptyCage`)
  - Has 1 private field(s): cardsSelected

**`Fusion Hammer`** (`FusionHammer`)
  - Has 1 private field(s): setDescription

**`Gambling Chip`** (`GamblingChip`)
  - Has 1 private field(s): activated

**`Gremlin Horn`** (`GremlinHorn`)
  - Has 1 private field(s): setDescription

**`Happy Flower`** (`HappyFlower`)
  - Has 1 private field(s): setDescription

**`HoveringKite`** (`HoveringKite`)
  - Has 1 private field(s): triggeredThisTurn

**`Lantern`** (`Lantern`)
  - Has 2 private field(s): firstTurn, setDescription

**`Matryoshka`** (`Matryoshka`)
  - Uses RNG: relicRng

**`Mummified Hand`** (`MummifiedHand`)
  - Uses RNG: cardRandomRng

**`Necronomicon`** (`Necronomicon`)
  - Has 1 private field(s): activated

**`Nunchaku`** (`Nunchaku`)
  - Has 1 private field(s): setDescription

**`Pandora's Box`** (`PandorasBox`)
  - Has 2 private field(s): count, calledTransform

**`Pocketwatch`** (`Pocketwatch`)
  - Has 1 private field(s): firstTurn

**`PreservedInsect`** (`PreservedInsect`)
  - Has 1 private field(s): MODIFIER_AMT

**`Red Skull`** (`RedSkull`)
  - Has 1 private field(s): isActive

**`Runic Capacitor`** (`RunicCapacitor`)
  - Has 1 private field(s): firstTurn

**`Runic Dome`** (`RunicDome`)
  - Has 1 private field(s): setDescription

**`SlaversCollar`** (`SlaversCollar`)
  - Has 1 private field(s): setDescription

**`Sozu`** (`Sozu`)
  - Has 1 private field(s): setDescription

**`Sundial`** (`Sundial`)
  - Has 1 private field(s): setDescription

**`Test 1`** (`Test1`)
  - Has 1 private field(s): setDescription

**`Test 6`** (`Test6`)
  - Has 1 private field(s): hasEnoughGold

**`Tiny House`** (`TinyHouse`)
  - Uses RNG: miscRng

**`Unceasing Top`** (`UnceasingTop`)
  - Has 2 private field(s): canDraw, disabledUntilEndOfTurn

**`Velvet Choker`** (`VelvetChoker`)
  - Has 1 private field(s): setDescription

**`War Paint`** (`WarPaint`)
  - Uses RNG: miscRng

**`Whetstone`** (`Whetstone`)
  - Uses RNG: miscRng

## CARD (410 clean, 7 with warnings, 0 blocked)

### With Warnings (potentially incorrect Rust implementation)

**`Bouncing Flask`** (`BouncingFlask`)
  - Uses RNG: cardRandomRng

**`Havoc`** (`Havoc`)
  - Uses RNG: cardRandomRng

**`Impatience`** (`Impatience`)
  - Has 1 private field(s): shouldGlow

**`Jack Of All Trades`** (`JackOfAllTrades`)
  - Uses RNG: cardRandomRng

**`LetFateDecide`** (`DEPRECATEDLetFateDecide`)
  - Uses RNG: cardRandomRng

**`SoothingAura`** (`DEPRECATEDSoothingAura`)
  - Uses RNG: cardRandomRng

**`TemperTantrum`** (`DEPRECATEDTemperTantrum`)
  - Uses RNG: cardRandomRng

## POTION (42 clean, 1 with warnings, 0 blocked)

### With Warnings (potentially incorrect Rust implementation)

**`DistilledChaos`** (`DistilledChaosPotion`)
  - Uses RNG: cardRandomRng

