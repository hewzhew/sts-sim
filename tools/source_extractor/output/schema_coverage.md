# Schema Coverage Report

Three-way diff: Java Source -> Protocol Schema -> Rust Enums

## POWER (67/161 in schema, 67/161 fully covered, 74 Rust variants)

| Java ID | Class | Schema -> Rust | In Rust Enum | Status |
|---------|-------|----------------|--------------|--------|
| `Accuracy` | `AccuracyPower` | **MISSING** | - | MISSING |
| `Adaptation` | `RushdownPower` | **MISSING** | - | MISSING |
| `After Image` | `AfterImagePower` | **MISSING** | - | MISSING |
| `AlwaysMad` | `DEPRECATEDAlwaysMadPower` | **MISSING** | - | MISSING |
| `Amplify` | `AmplifyPower` | **MISSING** | - | MISSING |
| `AngelForm` | `LiveForeverPower` | **MISSING** | - | MISSING |
| `Anger` | `AngerPower` | `Angry` | YES | FULL |
| `Angry` | `AngryPower` | `Angry` | YES | FULL |
| `Artifact` | `ArtifactPower` | `Artifact` | YES | FULL |
| `Attack Burn` | `AttackBurnPower` | **MISSING** | - | MISSING |
| `BackAttack` | `BackAttackPower` | **MISSING** | - | MISSING |
| `Barricade` | `BarricadePower` | `Barricade` | YES | FULL |
| `BattleHymn` | `BattleHymnPower` | **MISSING** | - | MISSING |
| `BeatOfDeath` | `BeatOfDeathPower` | **MISSING** | - | MISSING |
| `Berserk` | `BerserkPower` | `Berserk` | YES | FULL |
| `Bias` | `BiasPower` | **MISSING** | - | MISSING |
| `BlockReturnPower` | `BlockReturnPower` | **MISSING** | - | MISSING |
| `Blur` | `BlurPower` | **MISSING** | - | MISSING |
| `Brutality` | `BrutalityPower` | `Brutality` | YES | FULL |
| `Buffer` | `BufferPower` | `Buffer` | YES | FULL |
| `Burst` | `BurstPower` | **MISSING** | - | MISSING |
| `CannotChangeStancePower` | `CannotChangeStancePower` | **MISSING** | - | MISSING |
| `Choked` | `ChokePower` | **MISSING** | - | MISSING |
| `Collect` | `CollectPower` | **MISSING** | - | MISSING |
| `Combust` | `CombustPower` | `Combust` | YES | FULL |
| `Compulsive` | `ReactivePower` | **MISSING** | - | MISSING |
| `Confusion` | `ConfusionPower` | `Confusion` | YES | FULL |
| `Conserve` | `ConservePower` | **MISSING** | - | MISSING |
| `Constricted` | `ConstrictedPower` | `Constricted` | YES | FULL |
| `Controlled` | `MentalFortressPower` | **MISSING** | - | MISSING |
| `CorpseExplosionPower` | `CorpseExplosionPower` | **MISSING** | - | MISSING |
| `Corruption` | `CorruptionPower` | `Corruption` | YES | FULL |
| `Creative AI` | `CreativeAIPower` | **MISSING** | - | MISSING |
| `Curiosity` | `CuriosityPower` | `Curiosity` | YES | FULL |
| `Curl Up` | `CurlUpPower` | `CurlUp` | YES | FULL |
| `DEPRECATEDCondense` | `DEPRECATEDCondensePower` | **MISSING** | - | MISSING |
| `Dark Embrace` | `DarkEmbracePower` | `DarkEmbrace` | YES | FULL |
| `Demon Form` | `DemonFormPower` | `DemonForm` | YES | FULL |
| `DevaForm` | `DevaPower` | **MISSING** | - | MISSING |
| `DevotionPower` | `DevotionPower` | **MISSING** | - | MISSING |
| `DexLoss` | `LoseDexterityPower` | `DexterityDown` | YES | FULL |
| `Dexterity` | `DexterityPower` | `Dexterity` | YES | FULL |
| `DisciplinePower` | `DEPRECATEDDisciplinePower` | **MISSING** | - | MISSING |
| `Double Damage` | `DoubleDamagePower` | **MISSING** | - | MISSING |
| `Double Tap` | `DoubleTapPower` | `DoubleTap` | YES | FULL |
| `Draw` | `DrawPower` | **MISSING** | - | MISSING |
| `Draw Card` | `DrawCardNextTurnPower` | **MISSING** | - | MISSING |
| `Draw Reduction` | `DrawReductionPower` | **MISSING** | - | MISSING |
| `DuplicationPower` | `DuplicationPower` | **MISSING** | - | MISSING |
| `Echo Form` | `EchoPower` | **MISSING** | - | MISSING |
| `Electro` | `ElectroPower` | **MISSING** | - | MISSING |
| `EmotionalTurmoilPower` | `DEPRECATEDEmotionalTurmoilPower` | **MISSING** | - | MISSING |
| `EndTurnDeath` | `EndTurnDeathPower` | **MISSING** | - | MISSING |
| `Energized` | `EnergizedPower` | `Energized` | YES | FULL |
| `EnergizedBlue` | `EnergizedBluePower` | **MISSING** | - | MISSING |
| `EnergyDownPower` | `EnergyDownPower` | **MISSING** | - | MISSING |
| `Entangled` | `EntanglePower` | `Entangle` | YES | FULL |
| `Envenom` | `EnvenomPower` | **MISSING** | - | MISSING |
| `Equilibrium` | `EquilibriumPower` | **MISSING** | - | MISSING |
| `EstablishmentPower` | `EstablishmentPower` | **MISSING** | - | MISSING |
| `Evolve` | `EvolvePower` | `Evolve` | YES | FULL |
| `Explosive` | `ExplosivePower` | `Explosive` | YES | FULL |
| `Fading` | `FadingPower` | `Fading` | YES | FULL |
| `Feel No Pain` | `FeelNoPainPower` | `FeelNoPain` | YES | FULL |
| `Fire Breathing` | `FireBreathingPower` | `FireBreathing` | YES | FULL |
| `Flame Barrier` | `FlameBarrierPower` | `FlameBarrier` | YES | FULL |
| `Flex` | `LoseStrengthPower` | **MISSING** | - | MISSING |
| `FlickPower` | `DEPRECATEDFlickedPower` | **MISSING** | - | MISSING |
| `Flight` | `FlightPower` | `Flight` | YES | FULL |
| `FlowPower` | `DEPRECATEDFlowPower` | **MISSING** | - | MISSING |
| `Focus` | `FocusPower` | `Focus` | YES | FULL |
| `Frail` | `FrailPower` | `Frail` | YES | FULL |
| `FreeAttackPower` | `FreeAttackPower` | **MISSING** | - | MISSING |
| `Generic Strength Up Power` | `GenericStrengthUpPower` | **MISSING** | - | MISSING |
| `Grounded` | `DEPRECATEDGroundedPower` | **MISSING** | - | MISSING |
| `GrowthPower` | `GrowthPower` | **MISSING** | - | MISSING |
| `Heatsink` | `HeatsinkPower` | **MISSING** | - | MISSING |
| `Hello` | `HelloPower` | **MISSING** | - | MISSING |
| `Hex` | `HexPower` | `Hex` | YES | FULL |
| `HotHot` | `DEPRECATEDHotHotPower` | **MISSING** | - | MISSING |
| `Infinite Blades` | `InfiniteBladesPower` | **MISSING** | - | MISSING |
| `Intangible` | `IntangiblePower` | `Intangible` | YES | FULL |
| `IntangiblePlayer` | `IntangiblePlayerPower` | **MISSING** | - | MISSING |
| `Invincible` | `InvinciblePower` | `Invincible` | YES | FULL |
| `Juggernaut` | `JuggernautPower` | `Juggernaut` | YES | FULL |
| `Life Link` | `RegrowPower` | **MISSING** | - | MISSING |
| `Life Link` | `ResurrectPower` | **MISSING** | - | MISSING |
| `Lightning Mastery` | `LightningMasteryPower` | **MISSING** | - | MISSING |
| `LikeWaterPower` | `LikeWaterPower` | **MISSING** | - | MISSING |
| `Lockon` | `LockOnPower` | **MISSING** | - | MISSING |
| `Loop` | `LoopPower` | **MISSING** | - | MISSING |
| `Magnetism` | `MagnetismPower` | `MagnetismPower` | YES | FULL |
| `Malleable` | `MalleablePower` | `Malleable` | YES | FULL |
| `Mantra` | `MantraPower` | `Mantra` | YES | FULL |
| `MasterRealityPower` | `DEPRECATEDMasterRealityPower` | **MISSING** | - | MISSING |
| `MasterRealityPower` | `MasterRealityPower` | **MISSING** | - | MISSING |
| `Mastery` | `DEPRECATEDMasteryPower` | **MISSING** | - | MISSING |
| `Mayhem` | `MayhemPower` | `MayhemPower` | YES | FULL |
| `Metallicize` | `MetallicizePower` | `Metallicize` | YES | FULL |
| `Minion` | `MinionPower` | `Minion` | YES | FULL |
| `Mode Shift` | `ModeShiftPower` | `ModeShift` | YES | FULL |
| `Next Turn Block` | `NextTurnBlockPower` | `NextTurnBlock` | YES | FULL |
| `Night Terror` | `NightmarePower` | **MISSING** | - | MISSING |
| `Nirvana` | `NirvanaPower` | **MISSING** | - | MISSING |
| `No Draw` | `NoDrawPower` | `NoDraw` | YES | FULL |
| `NoBlockPower` | `NoBlockPower` | **MISSING** | - | MISSING |
| `NoSkills` | `NoSkillsPower` | **MISSING** | - | MISSING |
| `Noxious Fumes` | `NoxiousFumesPower` | **MISSING** | - | MISSING |
| `Nullify Attack` | `ForcefieldPower` | **MISSING** | - | MISSING |
| `OmegaPower` | `OmegaPower` | **MISSING** | - | MISSING |
| `OmnisciencePower` | `OmnisciencePower` | **MISSING** | - | MISSING |
| `Painful Stabs` | `PainfulStabsPower` | `PainfulStabs` | YES | FULL |
| `Panache` | `PanachePower` | `PanachePower` | YES | FULL |
| `PathToVictoryPower` | `MarkPower` | **MISSING** | - | MISSING |
| `Pen Nib` | `PenNibPower` | `PenNibPower` | YES | FULL |
| `Phantasmal` | `PhantasmalPower` | **MISSING** | - | MISSING |
| `Plated Armor` | `PlatedArmorPower` | `PlatedArmor` | YES | FULL |
| `Poison` | `PoisonPower` | `Poison` | YES | FULL |
| `Rage` | `RagePower` | `Rage` | YES | FULL |
| `Rebound` | `ReboundPower` | **MISSING** | - | MISSING |
| `RechargingCore` | `RechargingCorePower` | **MISSING** | - | MISSING |
| `Regenerate` | `RegenerateMonsterPower` | `Regen` | YES | FULL |
| `Regeneration` | `RegenPower` | `Regen` | YES | FULL |
| `Repair` | `RepairPower` | **MISSING** | - | MISSING |
| `Retain Cards` | `RetainCardPower` | **MISSING** | - | MISSING |
| `Retribution` | `DEPRECATEDRetributionPower` | **MISSING** | - | MISSING |
| `Ritual` | `RitualPower` | `Ritual` | YES | FULL |
| `Rupture` | `RupturePower` | `Rupture` | YES | FULL |
| `Sadistic` | `SadisticPower` | **MISSING** | - | MISSING |
| `Serenity` | `DEPRECATEDSerenityPower` | **MISSING** | - | MISSING |
| `Shackled` | `GainStrengthPower` | **MISSING** | - | MISSING |
| `Sharp Hide` | `SharpHidePower` | `SharpHide` | YES | FULL |
| `Shifting` | `ShiftingPower` | `Shifting` | YES | FULL |
| `Skill Burn` | `SkillBurnPower` | **MISSING** | - | MISSING |
| `Slow` | `SlowPower` | `Slow` | YES | FULL |
| `Split` | `SplitPower` | `Split` | YES | FULL |
| `Spore Cloud` | `SporeCloudPower` | `SporeCloud` | YES | FULL |
| `Stasis` | `StasisPower` | `Stasis` | YES | FULL |
| `StaticDischarge` | `StaticDischargePower` | **MISSING** | - | MISSING |
| `Storm` | `StormPower` | **MISSING** | - | MISSING |
| `Strength` | `StrengthPower` | `Strength` | YES | FULL |
| `StrikeUp` | `StrikeUpPower` | **MISSING** | - | MISSING |
| `Study` | `StudyPower` | **MISSING** | - | MISSING |
| `Surrounded` | `SurroundedPower` | **MISSING** | - | MISSING |
| `TheBomb` | `TheBombPower` | `TheBombPower` | YES | FULL |
| `Thievery` | `ThieveryPower` | `Thievery` | YES | FULL |
| `Thorns` | `ThornsPower` | `Thorns` | YES | FULL |
| `Thousand Cuts` | `ThousandCutsPower` | **MISSING** | - | MISSING |
| `Time Warp` | `TimeWarpPower` | `TimeWarp` | YES | FULL |
| `TimeMazePower` | `TimeMazePower` | **MISSING** | - | MISSING |
| `Tools Of The Trade` | `ToolsOfTheTradePower` | **MISSING** | - | MISSING |
| `Unawakened` | `UnawakenedPower` | `Unawakened` | YES | FULL |
| `Vault` | `VaultPower` | **MISSING** | - | MISSING |
| `Vigor` | `VigorPower` | `Vigor` | YES | FULL |
| `Vulnerable` | `VulnerablePower` | `Vulnerable` | YES | FULL |
| `WaveOfTheHandPower` | `WaveOfTheHandPower` | **MISSING** | - | MISSING |
| `Weakened` | `WeakPower` | `Weak` | YES | FULL |
| `Winter` | `WinterPower` | **MISSING** | - | MISSING |
| `WireheadingPower` | `ForesightPower` | **MISSING** | - | MISSING |
| `Wraith Form v2` | `WraithFormPower` | **MISSING** | - | MISSING |
| `WrathNextTurnPower` | `WrathNextTurnPower` | **MISSING** | - | MISSING |

## RELIC (35/190 in schema, 0/190 fully covered, 9 Rust variants)

| Java ID | Class | Schema -> Rust | In Rust Enum | Status |
|---------|-------|----------------|--------------|--------|
| `Akabeko` | `Akabeko` | `Akabeko` | NO (variant missing) | SCHEMA_ONLY |
| `Anchor` | `Anchor` | `Anchor` | NO (variant missing) | SCHEMA_ONLY |
| `Ancient Tea Set` | `AncientTeaSet` | **MISSING** | - | MISSING |
| `Art of War` | `ArtOfWar` | **MISSING** | - | MISSING |
| `Astrolabe` | `Astrolabe` | **MISSING** | - | MISSING |
| `Bag of Marbles` | `BagOfMarbles` | `BagOfMarbles` | NO (variant missing) | SCHEMA_ONLY |
| `Bag of Preparation` | `BagOfPreparation` | **MISSING** | - | MISSING |
| `Bird Faced Urn` | `BirdFacedUrn` | **MISSING** | - | MISSING |
| `Black Blood` | `BlackBlood` | **MISSING** | - | MISSING |
| `Black Star` | `BlackStar` | **MISSING** | - | MISSING |
| `Blood Vial` | `BloodVial` | **MISSING** | - | MISSING |
| `Bloody Idol` | `BloodyIdol` | **MISSING** | - | MISSING |
| `Blue Candle` | `BlueCandle` | **MISSING** | - | MISSING |
| `Boot` | `Boot` | **MISSING** | - | MISSING |
| `Bottled Flame` | `BottledFlame` | **MISSING** | - | MISSING |
| `Bottled Lightning` | `BottledLightning` | **MISSING** | - | MISSING |
| `Bottled Tornado` | `BottledTornado` | **MISSING** | - | MISSING |
| `Brimstone` | `Brimstone` | **MISSING** | - | MISSING |
| `Bronze Scales` | `BronzeScales` | `BronzeScales` | NO (variant missing) | SCHEMA_ONLY |
| `Burning Blood` | `BurningBlood` | `BurningBlood` | NO (variant missing) | SCHEMA_ONLY |
| `Busted Crown` | `BustedCrown` | **MISSING** | - | MISSING |
| `Cables` | `GoldPlatedCables` | **MISSING** | - | MISSING |
| `Calipers` | `Calipers` | **MISSING** | - | MISSING |
| `Calling Bell` | `CallingBell` | **MISSING** | - | MISSING |
| `CaptainsWheel` | `CaptainsWheel` | **MISSING** | - | MISSING |
| `Cauldron` | `Cauldron` | **MISSING** | - | MISSING |
| `Centennial Puzzle` | `CentennialPuzzle` | `CentennialPuzzle` | NO (variant missing) | SCHEMA_ONLY |
| `CeramicFish` | `CeramicFish` | **MISSING** | - | MISSING |
| `Champion Belt` | `ChampionsBelt` | **MISSING** | - | MISSING |
| `Charon's Ashes` | `CharonsAshes` | **MISSING** | - | MISSING |
| `Chemical X` | `ChemicalX` | **MISSING** | - | MISSING |
| `Circlet` | `Circlet` | **MISSING** | - | MISSING |
| `CloakClasp` | `CloakClasp` | **MISSING** | - | MISSING |
| `ClockworkSouvenir` | `ClockworkSouvenir` | **MISSING** | - | MISSING |
| `Coffee Dripper` | `CoffeeDripper` | **MISSING** | - | MISSING |
| `Cracked Core` | `CrackedCore` | **MISSING** | - | MISSING |
| `CultistMask` | `CultistMask` | **MISSING** | - | MISSING |
| `Cursed Key` | `CursedKey` | **MISSING** | - | MISSING |
| `Damaru` | `Damaru` | **MISSING** | - | MISSING |
| `Dark Core` | `DEPRECATED_DarkCore` | **MISSING** | - | MISSING |
| `Darkstone Periapt` | `DarkstonePeriapt` | **MISSING** | - | MISSING |
| `DataDisk` | `DataDisk` | `DataDisk` | NO (variant missing) | SCHEMA_ONLY |
| `Dead Branch` | `DeadBranch` | **MISSING** | - | MISSING |
| `Derp Rock` | `DerpRock` | **MISSING** | - | MISSING |
| `Discerning Monocle` | `DiscerningMonocle` | **MISSING** | - | MISSING |
| `Dodecahedron` | `DEPRECATEDDodecahedron` | **MISSING** | - | MISSING |
| `DollysMirror` | `DollysMirror` | **MISSING** | - | MISSING |
| `Dream Catcher` | `DreamCatcher` | **MISSING** | - | MISSING |
| `Du-Vu Doll` | `DuVuDoll` | **MISSING** | - | MISSING |
| `Ectoplasm` | `Ectoplasm` | **MISSING** | - | MISSING |
| `Emotion Chip` | `EmotionChip` | **MISSING** | - | MISSING |
| `Empty Cage` | `EmptyCage` | **MISSING** | - | MISSING |
| `Enchiridion` | `Enchiridion` | **MISSING** | - | MISSING |
| `Eternal Feather` | `EternalFeather` | `_EternalFeather` | NO (variant missing) | SCHEMA_ONLY |
| `FaceOfCleric` | `FaceOfCleric` | **MISSING** | - | MISSING |
| `FossilizedHelix` | `FossilizedHelix` | **MISSING** | - | MISSING |
| `Frozen Egg 2` | `FrozenEgg2` | `_FrozenEgg2` | NO (variant missing) | SCHEMA_ONLY |
| `Frozen Eye` | `FrozenEye` | **MISSING** | - | MISSING |
| `FrozenCore` | `FrozenCore` | **MISSING** | - | MISSING |
| `Fusion Hammer` | `FusionHammer` | **MISSING** | - | MISSING |
| `Gambling Chip` | `GamblingChip` | **MISSING** | - | MISSING |
| `Ginger` | `Ginger` | **MISSING** | - | MISSING |
| `Girya` | `Girya` | **MISSING** | - | MISSING |
| `Golden Idol` | `GoldenIdol` | **MISSING** | - | MISSING |
| `GoldenEye` | `GoldenEye` | **MISSING** | - | MISSING |
| `Gremlin Horn` | `GremlinHorn` | `GremlinHorn` | NO (variant missing) | SCHEMA_ONLY |
| `GremlinMask` | `GremlinMask` | **MISSING** | - | MISSING |
| `HandDrill` | `HandDrill` | **MISSING** | - | MISSING |
| `Happy Flower` | `HappyFlower` | `HappyFlower` | NO (variant missing) | SCHEMA_ONLY |
| `HolyWater` | `HolyWater` | **MISSING** | - | MISSING |
| `HornCleat` | `HornCleat` | **MISSING** | - | MISSING |
| `HoveringKite` | `HoveringKite` | **MISSING** | - | MISSING |
| `Ice Cream` | `IceCream` | **MISSING** | - | MISSING |
| `Incense Burner` | `IncenseBurner` | **MISSING** | - | MISSING |
| `InkBottle` | `InkBottle` | **MISSING** | - | MISSING |
| `Inserter` | `Inserter` | **MISSING** | - | MISSING |
| `Juzu Bracelet` | `JuzuBracelet` | **MISSING** | - | MISSING |
| `Kunai` | `Kunai` | `Kunai` | NO (variant missing) | SCHEMA_ONLY |
| `Lantern` | `Lantern` | `Lantern` | NO (variant missing) | SCHEMA_ONLY |
| `Lee's Waffle` | `Waffle` | **MISSING** | - | MISSING |
| `Letter Opener` | `LetterOpener` | `LetterOpener` | NO (variant missing) | SCHEMA_ONLY |
| `Lizard Tail` | `LizardTail` | **MISSING** | - | MISSING |
| `Magic Flower` | `MagicFlower` | **MISSING** | - | MISSING |
| `Mango` | `Mango` | **MISSING** | - | MISSING |
| `Mark of Pain` | `MarkOfPain` | **MISSING** | - | MISSING |
| `Mark of the Bloom` | `MarkOfTheBloom` | **MISSING** | - | MISSING |
| `Matryoshka` | `Matryoshka` | **MISSING** | - | MISSING |
| `MawBank` | `MawBank` | **MISSING** | - | MISSING |
| `MealTicket` | `MealTicket` | **MISSING** | - | MISSING |
| `Meat on the Bone` | `MeatOnTheBone` | `MeatOnTheBone` | NO (variant missing) | SCHEMA_ONLY |
| `Medical Kit` | `MedicalKit` | **MISSING** | - | MISSING |
| `Melange` | `Melange` | **MISSING** | - | MISSING |
| `Membership Card` | `MembershipCard` | **MISSING** | - | MISSING |
| `Mercury Hourglass` | `MercuryHourglass` | `MercuryHourglass` | NO (variant missing) | SCHEMA_ONLY |
| `Molten Egg 2` | `MoltenEgg2` | `_MoltenEgg2` | NO (variant missing) | SCHEMA_ONLY |
| `Mummified Hand` | `MummifiedHand` | **MISSING** | - | MISSING |
| `MutagenicStrength` | `MutagenicStrength` | **MISSING** | - | MISSING |
| `Necronomicon` | `Necronomicon` | **MISSING** | - | MISSING |
| `NeowsBlessing` | `NeowsLament` | `NeowsLament` | NO (variant missing) | SCHEMA_ONLY |
| `Nilry's Codex` | `NilrysCodex` | **MISSING** | - | MISSING |
| `Ninja Scroll` | `NinjaScroll` | **MISSING** | - | MISSING |
| `Nloth's Gift` | `NlothsGift` | **MISSING** | - | MISSING |
| `NlothsMask` | `NlothsMask` | **MISSING** | - | MISSING |
| `Nuclear Battery` | `NuclearBattery` | **MISSING** | - | MISSING |
| `Nunchaku` | `Nunchaku` | `Nunchaku` | NO (variant missing) | SCHEMA_ONLY |
| `Odd Mushroom` | `OddMushroom` | **MISSING** | - | MISSING |
| `Oddly Smooth Stone` | `OddlySmoothStone` | `OddlySmoothStone` | NO (variant missing) | SCHEMA_ONLY |
| `Old Coin` | `OldCoin` | **MISSING** | - | MISSING |
| `Omamori` | `Omamori` | **MISSING** | - | MISSING |
| `OrangePellets` | `OrangePellets` | **MISSING** | - | MISSING |
| `Orichalcum` | `Orichalcum` | `Orichalcum` | NO (variant missing) | SCHEMA_ONLY |
| `Ornamental Fan` | `OrnamentalFan` | `OrnamentalFan` | NO (variant missing) | SCHEMA_ONLY |
| `Orrery` | `Orrery` | **MISSING** | - | MISSING |
| `Pandora's Box` | `PandorasBox` | **MISSING** | - | MISSING |
| `Pantograph` | `Pantograph` | **MISSING** | - | MISSING |
| `Paper Crane` | `PaperCrane` | `PaperCrane` | NO (variant missing) | SCHEMA_ONLY |
| `Paper Frog` | `PaperFrog` | **MISSING** | - | MISSING |
| `Peace Pipe` | `PeacePipe` | **MISSING** | - | MISSING |
| `Pear` | `Pear` | **MISSING** | - | MISSING |
| `Pen Nib` | `PenNib` | `PenNib` | NO (variant missing) | SCHEMA_ONLY |
| `Philosopher's Stone` | `PhilosopherStone` | **MISSING** | - | MISSING |
| `Pocketwatch` | `Pocketwatch` | **MISSING** | - | MISSING |
| `Potion Belt` | `PotionBelt` | **MISSING** | - | MISSING |
| `Prayer Wheel` | `PrayerWheel` | **MISSING** | - | MISSING |
| `PreservedInsect` | `PreservedInsect` | **MISSING** | - | MISSING |
| `PrismaticShard` | `PrismaticShard` | **MISSING** | - | MISSING |
| `PureWater` | `PureWater` | **MISSING** | - | MISSING |
| `Question Card` | `QuestionCard` | **MISSING** | - | MISSING |
| `Red Circlet` | `RedCirclet` | **MISSING** | - | MISSING |
| `Red Mask` | `RedMask` | **MISSING** | - | MISSING |
| `Red Skull` | `RedSkull` | `RedSkull` | NO (variant missing) | SCHEMA_ONLY |
| `Regal Pillow` | `RegalPillow` | **MISSING** | - | MISSING |
| `Ring of the Serpent` | `RingOfTheSerpent` | **MISSING** | - | MISSING |
| `Ring of the Snake` | `SnakeRing` | **MISSING** | - | MISSING |
| `Runic Capacitor` | `RunicCapacitor` | **MISSING** | - | MISSING |
| `Runic Cube` | `RunicCube` | `RunicCube` | NO (variant missing) | SCHEMA_ONLY |
| `Runic Dome` | `RunicDome` | **MISSING** | - | MISSING |
| `Runic Pyramid` | `RunicPyramid` | **MISSING** | - | MISSING |
| `SacredBark` | `SacredBark` | **MISSING** | - | MISSING |
| `Self Forming Clay` | `SelfFormingClay` | `SelfFormingClay` | NO (variant missing) | SCHEMA_ONLY |
| `Shovel` | `Shovel` | **MISSING** | - | MISSING |
| `Shuriken` | `Shuriken` | `Shuriken` | NO (variant missing) | SCHEMA_ONLY |
| `Singing Bowl` | `SingingBowl` | **MISSING** | - | MISSING |
| `SlaversCollar` | `SlaversCollar` | **MISSING** | - | MISSING |
| `Sling` | `Sling` | **MISSING** | - | MISSING |
| `Smiling Mask` | `SmilingMask` | **MISSING** | - | MISSING |
| `Snake Skull` | `SneckoSkull` | **MISSING** | - | MISSING |
| `Snecko Eye` | `SneckoEye` | **MISSING** | - | MISSING |
| `Sozu` | `Sozu` | **MISSING** | - | MISSING |
| `Spirit Poop` | `SpiritPoop` | **MISSING** | - | MISSING |
| `SsserpentHead` | `SsserpentHead` | **MISSING** | - | MISSING |
| `StoneCalendar` | `StoneCalendar` | **MISSING** | - | MISSING |
| `Strange Spoon` | `StrangeSpoon` | **MISSING** | - | MISSING |
| `Strawberry` | `Strawberry` | `Strawberry` | NO (variant missing) | SCHEMA_ONLY |
| `StrikeDummy` | `StrikeDummy` | **MISSING** | - | MISSING |
| `Sundial` | `Sundial` | **MISSING** | - | MISSING |
| `Symbiotic Virus` | `SymbioticVirus` | **MISSING** | - | MISSING |
| `TeardropLocket` | `TeardropLocket` | **MISSING** | - | MISSING |
| `Test 1` | `Test1` | **MISSING** | - | MISSING |
| `Test 3` | `Test3` | **MISSING** | - | MISSING |
| `Test 4` | `Test4` | **MISSING** | - | MISSING |
| `Test 5` | `Test5` | **MISSING** | - | MISSING |
| `Test 6` | `Test6` | **MISSING** | - | MISSING |
| `The Courier` | `Courier` | **MISSING** | - | MISSING |
| `The Specimen` | `TheSpecimen` | **MISSING** | - | MISSING |
| `TheAbacus` | `Abacus` | **MISSING** | - | MISSING |
| `Thread and Needle` | `ThreadAndNeedle` | **MISSING** | - | MISSING |
| `Tingsha` | `Tingsha` | **MISSING** | - | MISSING |
| `Tiny Chest` | `TinyChest` | `_TinyChest` | NO (variant missing) | SCHEMA_ONLY |
| `Tiny House` | `TinyHouse` | **MISSING** | - | MISSING |
| `Toolbox` | `Toolbox` | **MISSING** | - | MISSING |
| `Torii` | `Torii` | `Torii` | NO (variant missing) | SCHEMA_ONLY |
| `Tough Bandages` | `ToughBandages` | **MISSING** | - | MISSING |
| `Toxic Egg 2` | `ToxicEgg2` | `_ToxicEgg2` | NO (variant missing) | SCHEMA_ONLY |
| `Toy Ornithopter` | `ToyOrnithopter` | `ToyOrnithopter` | NO (variant missing) | SCHEMA_ONLY |
| `TungstenRod` | `TungstenRod` | **MISSING** | - | MISSING |
| `Turnip` | `Turnip` | **MISSING** | - | MISSING |
| `TwistedFunnel` | `TwistedFunnel` | **MISSING** | - | MISSING |
| `Unceasing Top` | `UnceasingTop` | **MISSING** | - | MISSING |
| `Vajra` | `Vajra` | `Vajra` | NO (variant missing) | SCHEMA_ONLY |
| `Velvet Choker` | `VelvetChoker` | **MISSING** | - | MISSING |
| `VioletLotus` | `VioletLotus` | **MISSING** | - | MISSING |
| `War Paint` | `WarPaint` | **MISSING** | - | MISSING |
| `WarpedTongs` | `WarpedTongs` | **MISSING** | - | MISSING |
| `Whetstone` | `Whetstone` | `_Whetstone` | NO (variant missing) | SCHEMA_ONLY |
| `White Beast Statue` | `WhiteBeast` | **MISSING** | - | MISSING |
| `WingedGreaves` | `WingBoots` | **MISSING** | - | MISSING |
| `WristBlade` | `WristBlade` | **MISSING** | - | MISSING |
| `Yang` | `Duality` | **MISSING** | - | MISSING |
| `Yin` | `DEPRECATEDYin` | **MISSING** | - | MISSING |

## POTION (42/43 in schema, 35/43 fully covered, 42 Rust variants)

| Java ID | Class | Schema -> Rust | In Rust Enum | Status |
|---------|-------|----------------|--------------|--------|
| `Ambrosia` | `Ambrosia` | `Ambrosia` | NO (variant missing) | SCHEMA_ONLY |
| `Ancient Potion` | `AncientPotion` | `AncientPotion` | YES | FULL |
| `AttackPotion` | `AttackPotion` | `AttackPotion` | YES | FULL |
| `BlessingOfTheForge` | `BlessingOfTheForge` | `BlessingOfTheForge` | YES | FULL |
| `Block Potion` | `BlockPotion` | `BlockPotion` | YES | FULL |
| `BloodPotion` | `BloodPotion` | `BloodPotion` | YES | FULL |
| `BottledMiracle` | `BottledMiracle` | `BottledMiracle` | NO (variant missing) | SCHEMA_ONLY |
| `ColorlessPotion` | `ColorlessPotion` | `ColorlessPotion` | YES | FULL |
| `CultistPotion` | `CultistPotion` | `CultistPotion` | YES | FULL |
| `CunningPotion` | `CunningPotion` | `CunningPotion` | NO (variant missing) | SCHEMA_ONLY |
| `Dexterity Potion` | `DexterityPotion` | `DexterityPotion` | YES | FULL |
| `DistilledChaos` | `DistilledChaosPotion` | `DistilledChaosPotion` | YES | FULL |
| `DuplicationPotion` | `DuplicationPotion` | `DuplicationPotion` | YES | FULL |
| `ElixirPotion` | `Elixir` | `Elixir` | YES | FULL |
| `Energy Potion` | `EnergyPotion` | `EnergyPotion` | YES | FULL |
| `EntropicBrew` | `EntropicBrew` | `EntropicBrew` | YES | FULL |
| `EssenceOfDarkness` | `EssenceOfDarkness` | `EssenceOfDarkness` | NO (variant missing) | SCHEMA_ONLY |
| `EssenceOfSteel` | `EssenceOfSteel` | `EssenceOfSteel` | YES | FULL |
| `Explosive Potion` | `ExplosivePotion` | `ExplosivePotion` | YES | FULL |
| `FairyPotion` | `FairyPotion` | `FairyPotion` | YES | FULL |
| `FearPotion` | `FearPotion` | `FearPotion` | YES | FULL |
| `Fire Potion` | `FirePotion` | `FirePotion` | YES | FULL |
| `FocusPotion` | `FocusPotion` | `FocusPotion` | NO (variant missing) | SCHEMA_ONLY |
| `Fruit Juice` | `FruitJuice` | `FruitJuice` | YES | FULL |
| `GamblersBrew` | `GamblersBrew` | `GamblersBrew` | YES | FULL |
| `GhostInAJar` | `GhostInAJar` | `GhostInAJar` | YES | FULL |
| `HeartOfIron` | `HeartOfIron` | `HeartOfIron` | YES | FULL |
| `LiquidBronze` | `LiquidBronze` | `LiquidBronze` | YES | FULL |
| `LiquidMemories` | `LiquidMemories` | `LiquidMemories` | YES | FULL |
| `Poison Potion` | `PoisonPotion` | `PoisonPotion` | YES | FULL |
| `Potion Slot` | `PotionSlot` | **MISSING** | - | MISSING |
| `PotionOfCapacity` | `PotionOfCapacity` | `PotionOfCapacity` | NO (variant missing) | SCHEMA_ONLY |
| `PowerPotion` | `PowerPotion` | `PowerPotion` | YES | FULL |
| `Regen Potion` | `RegenPotion` | `RegenPotion` | YES | FULL |
| `SkillPotion` | `SkillPotion` | `SkillPotion` | YES | FULL |
| `SmokeBomb` | `SmokeBomb` | `SmokeBomb` | YES | FULL |
| `SneckoOil` | `SneckoOil` | `SneckoOil` | YES | FULL |
| `SpeedPotion` | `SpeedPotion` | `SpeedPotion` | YES | FULL |
| `StancePotion` | `StancePotion` | `StancePotion` | NO (variant missing) | SCHEMA_ONLY |
| `SteroidPotion` | `SteroidPotion` | `SteroidPotion` | YES | FULL |
| `Strength Potion` | `StrengthPotion` | `StrengthPotion` | YES | FULL |
| `Swift Potion` | `SwiftPotion` | `SwiftPotion` | YES | FULL |
| `Weak Potion` | `WeakenPotion` | `WeakenPotion` | YES | FULL |

## CARD (133/438 in schema, 82/438 fully covered, 135 Rust variants)

| Java ID | Class | Schema -> Rust | In Rust Enum | Status |
|---------|-------|----------------|--------------|--------|
| `A Thousand Cuts` | `AThousandCuts` | **MISSING** | - | MISSING |
| `Accuracy` | `Accuracy` | **MISSING** | - | MISSING |
| `Acrobatics` | `Acrobatics` | **MISSING** | - | MISSING |
| `Adaptation` | `Rushdown` | **MISSING** | - | MISSING |
| `Adrenaline` | `Adrenaline` | **MISSING** | - | MISSING |
| `After Image` | `AfterImage` | **MISSING** | - | MISSING |
| `Aggregate` | `Aggregate` | **MISSING** | - | MISSING |
| `All For One` | `AllForOne` | **MISSING** | - | MISSING |
| `All Out Attack` | `AllOutAttack` | **MISSING** | - | MISSING |
| `Alpha` | `Alpha` | **MISSING** | - | MISSING |
| `AlwaysMad` | `DEPRECATEDAlwaysMad` | **MISSING** | - | MISSING |
| `Amplify` | `Amplify` | **MISSING** | - | MISSING |
| `AndCarryOn` | `DEPRECATEDAndCarryOn` | **MISSING** | - | MISSING |
| `Anger` | `Anger` | `Anger` | YES | FULL |
| `Apotheosis` | `Apotheosis` | `Apotheosis` | YES | FULL |
| `Armaments` | `Armaments` | `Armaments` | YES | FULL |
| `AscendersBane` | `AscendersBane` | `AscendersBane` | YES | FULL |
| `Auto Shields` | `AutoShields` | **MISSING** | - | MISSING |
| `AwakenedStrike` | `DEPRECATEDAwakenedStrike` | **MISSING** | - | MISSING |
| `Backflip` | `Backflip` | **MISSING** | - | MISSING |
| `Backstab` | `Backstab` | **MISSING** | - | MISSING |
| `Ball Lightning` | `BallLightning` | **MISSING** | - | MISSING |
| `Bandage Up` | `BandageUp` | `Bandage Up` | NO (variant missing) | SCHEMA_ONLY |
| `Bane` | `Bane` | **MISSING** | - | MISSING |
| `Barrage` | `Barrage` | **MISSING** | - | MISSING |
| `Barricade` | `Barricade` | `Barricade` | YES | FULL |
| `Bash` | `Bash` | `Bash` | YES | FULL |
| `Battle Trance` | `BattleTrance` | `Battle Trance` | NO (variant missing) | SCHEMA_ONLY |
| `BattleHymn` | `BattleHymn` | **MISSING** | - | MISSING |
| `Beam Cell` | `BeamCell` | **MISSING** | - | MISSING |
| `BecomeAlmighty` | `BecomeAlmighty` | **MISSING** | - | MISSING |
| `Berserk` | `Berserk` | `Berserk` | YES | FULL |
| `Beta` | `Beta` | **MISSING** | - | MISSING |
| `Biased Cognition` | `BiasedCognition` | **MISSING** | - | MISSING |
| `BigBrain` | `DEPRECATEDBigBrain` | **MISSING** | - | MISSING |
| `Bite` | `Bite` | `Bite` | YES | FULL |
| `Blade Dance` | `BladeDance` | **MISSING** | - | MISSING |
| `Blasphemy` | `Blasphemy` | **MISSING** | - | MISSING |
| `Blessed` | `DEPRECATEDBlessed` | **MISSING** | - | MISSING |
| `Blind` | `Blind` | `Blind` | YES | FULL |
| `Bliss` | `DEPRECATEDBliss` | **MISSING** | - | MISSING |
| `Blizzard` | `Blizzard` | **MISSING** | - | MISSING |
| `Blood for Blood` | `BloodForBlood` | `Blood for Blood` | NO (variant missing) | SCHEMA_ONLY |
| `Bloodletting` | `Bloodletting` | `Bloodletting` | YES | FULL |
| `Bludgeon` | `Bludgeon` | `Bludgeon` | YES | FULL |
| `Blur` | `Blur` | **MISSING** | - | MISSING |
| `Body Slam` | `BodySlam` | `Body Slam` | NO (variant missing) | SCHEMA_ONLY |
| `BootSequence` | `BootSequence` | **MISSING** | - | MISSING |
| `Bouncing Flask` | `BouncingFlask` | **MISSING** | - | MISSING |
| `BowlingBash` | `BowlingBash` | **MISSING** | - | MISSING |
| `Brilliance` | `Brilliance` | **MISSING** | - | MISSING |
| `BrillianceAura` | `DEPRECATEDBrillianceAura` | **MISSING** | - | MISSING |
| `Brutality` | `Brutality` | `Brutality` | YES | FULL |
| `Buffer` | `Buffer` | **MISSING** | - | MISSING |
| `Bullet Time` | `BulletTime` | **MISSING** | - | MISSING |
| `Burn` | `Burn` | `Burn` | YES | FULL |
| `Burning Pact` | `BurningPact` | `Burning Pact` | NO (variant missing) | SCHEMA_ONLY |
| `Burst` | `Burst` | **MISSING** | - | MISSING |
| `Calculated Gamble` | `CalculatedGamble` | **MISSING** | - | MISSING |
| `Calm` | `DEPRECATEDCalm` | **MISSING** | - | MISSING |
| `Calm` | `DEPRECATEDChooseCalm` | **MISSING** | - | MISSING |
| `Calm` | `ChooseCalm` | **MISSING** | - | MISSING |
| `Caltrops` | `Caltrops` | **MISSING** | - | MISSING |
| `Capacitor` | `Capacitor` | **MISSING** | - | MISSING |
| `Carnage` | `Carnage` | `Carnage` | YES | FULL |
| `CarveReality` | `CarveReality` | **MISSING** | - | MISSING |
| `Catalyst` | `Catalyst` | **MISSING** | - | MISSING |
| `Causality` | `DEPRECATEDCausality` | **MISSING** | - | MISSING |
| `ChallengeAccepted` | `DEPRECATEDChallengeAccepted` | **MISSING** | - | MISSING |
| `Chaos` | `Chaos` | **MISSING** | - | MISSING |
| `Chill` | `Chill` | **MISSING** | - | MISSING |
| `Choke` | `Choke` | **MISSING** | - | MISSING |
| `Chrysalis` | `Chrysalis` | `Chrysalis` | YES | FULL |
| `Clarity` | `DEPRECATEDClarity` | **MISSING** | - | MISSING |
| `Clash` | `Clash` | `Clash` | YES | FULL |
| `CleanseEvil` | `DEPRECATEDCleanseEvil` | **MISSING** | - | MISSING |
| `ClearTheMind` | `Tranquility` | **MISSING** | - | MISSING |
| `Cleave` | `Cleave` | `Cleave` | YES | FULL |
| `Cloak And Dagger` | `CloakAndDagger` | **MISSING** | - | MISSING |
| `Clothesline` | `Clothesline` | `Clothesline` | YES | FULL |
| `Clumsy` | `Clumsy` | `Clumsy` | YES | FULL |
| `Cold Snap` | `ColdSnap` | **MISSING** | - | MISSING |
| `Collect` | `Collect` | **MISSING** | - | MISSING |
| `Combust` | `Combust` | `Combust` | YES | FULL |
| `Compile Driver` | `CompileDriver` | **MISSING** | - | MISSING |
| `Concentrate` | `Concentrate` | **MISSING** | - | MISSING |
| `Conclude` | `Conclude` | **MISSING** | - | MISSING |
| `Condense` | `DEPRECATEDCondense` | **MISSING** | - | MISSING |
| `Confront` | `DEPRECATEDConfront` | **MISSING** | - | MISSING |
| `ConjureBlade` | `ConjureBlade` | **MISSING** | - | MISSING |
| `Consecrate` | `Consecrate` | **MISSING** | - | MISSING |
| `Conserve Battery` | `ConserveBattery` | **MISSING** | - | MISSING |
| `Consume` | `Consume` | **MISSING** | - | MISSING |
| `Contemplate` | `DEPRECATEDContemplate` | **MISSING** | - | MISSING |
| `Coolheaded` | `Coolheaded` | **MISSING** | - | MISSING |
| `Core Surge` | `CoreSurge` | **MISSING** | - | MISSING |
| `Corpse Explosion` | `CorpseExplosion` | **MISSING** | - | MISSING |
| `Corruption` | `Corruption` | `Corruption` | YES | FULL |
| `Creative AI` | `CreativeAI` | **MISSING** | - | MISSING |
| `Crescendo` | `Crescendo` | **MISSING** | - | MISSING |
| `CrescentKick` | `DEPRECATEDCrescentKick` | **MISSING** | - | MISSING |
| `Crippling Poison` | `CripplingPoison` | **MISSING** | - | MISSING |
| `CrushJoints` | `CrushJoints` | **MISSING** | - | MISSING |
| `CurseOfTheBell` | `CurseOfTheBell` | `CurseOfTheBell` | YES | FULL |
| `CutThroughFate` | `CutThroughFate` | **MISSING** | - | MISSING |
| `DEPRECATEDBalancedViolence` | `DEPRECATEDBalancedViolence` | **MISSING** | - | MISSING |
| `DEPRECATEDFlicker` | `DEPRECATEDFlicker` | **MISSING** | - | MISSING |
| `Dagger Spray` | `DaggerSpray` | **MISSING** | - | MISSING |
| `Dagger Throw` | `DaggerThrow` | **MISSING** | - | MISSING |
| `Dark Embrace` | `DarkEmbrace` | `Dark Embrace` | NO (variant missing) | SCHEMA_ONLY |
| `Dark Shackles` | `DarkShackles` | `Dark Shackles` | NO (variant missing) | SCHEMA_ONLY |
| `Darkness` | `Darkness` | **MISSING** | - | MISSING |
| `Dash` | `Dash` | **MISSING** | - | MISSING |
| `Dazed` | `Dazed` | `Dazed` | YES | FULL |
| `Deadly Poison` | `DeadlyPoison` | **MISSING** | - | MISSING |
| `Decay` | `Decay` | `Decay` | YES | FULL |
| `DeceiveReality` | `DeceiveReality` | **MISSING** | - | MISSING |
| `Deep Breath` | `DeepBreath` | `Deep Breath` | NO (variant missing) | SCHEMA_ONLY |
| `Defend_B` | `Defend_Blue` | **MISSING** | - | MISSING |
| `Defend_G` | `Defend_Green` | **MISSING** | - | MISSING |
| `Defend_P` | `Defend_Watcher` | **MISSING** | - | MISSING |
| `Defend_R` | `Defend_Red` | `Defend_R` | NO (variant missing) | SCHEMA_ONLY |
| `Deflect` | `Deflect` | **MISSING** | - | MISSING |
| `Defragment` | `Defragment` | **MISSING** | - | MISSING |
| `Demon Form` | `DemonForm` | `Demon Form` | NO (variant missing) | SCHEMA_ONLY |
| `DeusExMachina` | `DeusExMachina` | **MISSING** | - | MISSING |
| `DevaForm` | `DevaForm` | **MISSING** | - | MISSING |
| `Devotion` | `Devotion` | **MISSING** | - | MISSING |
| `Die Die Die` | `DieDieDie` | **MISSING** | - | MISSING |
| `Disarm` | `Disarm` | `Disarm` | YES | FULL |
| `Discipline` | `Discipline` | **MISSING** | - | MISSING |
| `Discovery` | `Discovery` | `Discovery` | YES | FULL |
| `Distraction` | `Distraction` | **MISSING** | - | MISSING |
| `Dodge and Roll` | `DodgeAndRoll` | **MISSING** | - | MISSING |
| `Doom and Gloom` | `DoomAndGloom` | **MISSING** | - | MISSING |
| `Doppelganger` | `Doppelganger` | **MISSING** | - | MISSING |
| `Double Energy` | `DoubleEnergy` | **MISSING** | - | MISSING |
| `Double Tap` | `DoubleTap` | `Double Tap` | NO (variant missing) | SCHEMA_ONLY |
| `Doubt` | `Doubt` | `Doubt` | YES | FULL |
| `Dramatic Entrance` | `DramaticEntrance` | `Dramatic Entrance` | NO (variant missing) | SCHEMA_ONLY |
| `Dropkick` | `Dropkick` | `Dropkick` | YES | FULL |
| `Dual Wield` | `DualWield` | `Dual Wield` | NO (variant missing) | SCHEMA_ONLY |
| `Dualcast` | `Dualcast` | **MISSING** | - | MISSING |
| `Echo Form` | `EchoForm` | **MISSING** | - | MISSING |
| `Electrodynamics` | `Electrodynamics` | **MISSING** | - | MISSING |
| `EmptyBody` | `EmptyBody` | **MISSING** | - | MISSING |
| `EmptyFist` | `EmptyFist` | **MISSING** | - | MISSING |
| `EmptyMind` | `EmptyMind` | **MISSING** | - | MISSING |
| `Endless Agony` | `EndlessAgony` | **MISSING** | - | MISSING |
| `Enlightenment` | `Enlightenment` | `Enlightenment` | YES | FULL |
| `Entrench` | `Entrench` | `Entrench` | YES | FULL |
| `Envenom` | `Envenom` | **MISSING** | - | MISSING |
| `Eruption` | `DEPRECATEDEruption` | **MISSING** | - | MISSING |
| `Eruption` | `Eruption` | **MISSING** | - | MISSING |
| `Escape Plan` | `EscapePlan` | **MISSING** | - | MISSING |
| `Establishment` | `Establishment` | **MISSING** | - | MISSING |
| `Evaluate` | `Evaluate` | **MISSING** | - | MISSING |
| `Eviscerate` | `Eviscerate` | **MISSING** | - | MISSING |
| `Evolve` | `Evolve` | `Evolve` | YES | FULL |
| `Exhume` | `Exhume` | `Exhume` | YES | FULL |
| `Experienced` | `DEPRECATEDExperienced` | **MISSING** | - | MISSING |
| `Expertise` | `Expertise` | **MISSING** | - | MISSING |
| `Expunger` | `Expunger` | **MISSING** | - | MISSING |
| `FTL` | `FTL` | **MISSING** | - | MISSING |
| `FameAndFortune` | `FameAndFortune` | **MISSING** | - | MISSING |
| `Fasting2` | `Fasting` | **MISSING** | - | MISSING |
| `FearNoEvil` | `FearNoEvil` | **MISSING** | - | MISSING |
| `Feed` | `Feed` | `Feed` | YES | FULL |
| `Feel No Pain` | `FeelNoPain` | `Feel No Pain` | NO (variant missing) | SCHEMA_ONLY |
| `Fiend Fire` | `FiendFire` | `Fiend Fire` | NO (variant missing) | SCHEMA_ONLY |
| `Finesse` | `Finesse` | `Finesse` | YES | FULL |
| `Finisher` | `Finisher` | **MISSING** | - | MISSING |
| `Fire Breathing` | `FireBreathing` | `Fire Breathing` | NO (variant missing) | SCHEMA_ONLY |
| `Fission` | `Fission` | **MISSING** | - | MISSING |
| `Flame Barrier` | `FlameBarrier` | `Flame Barrier` | NO (variant missing) | SCHEMA_ONLY |
| `FlameMastery` | `DEPRECATEDFlameMastery` | **MISSING** | - | MISSING |
| `Flare` | `DEPRECATEDFlare` | **MISSING** | - | MISSING |
| `Flash of Steel` | `FlashOfSteel` | `Flash of Steel` | NO (variant missing) | SCHEMA_ONLY |
| `Flechettes` | `Flechettes` | **MISSING** | - | MISSING |
| `Flex` | `Flex` | `Flex` | YES | FULL |
| `Flick` | `DEPRECATEDFlick` | **MISSING** | - | MISSING |
| `Flow` | `DEPRECATEDFlow` | **MISSING** | - | MISSING |
| `FlowState` | `DEPRECATEDFlowState` | **MISSING** | - | MISSING |
| `FlurryOfBlows` | `FlurryOfBlows` | **MISSING** | - | MISSING |
| `Flying Knee` | `FlyingKnee` | **MISSING** | - | MISSING |
| `FlyingSleeves` | `FlyingSleeves` | **MISSING** | - | MISSING |
| `FollowUp` | `FollowUp` | **MISSING** | - | MISSING |
| `Footwork` | `Footwork` | **MISSING** | - | MISSING |
| `Force Field` | `ForceField` | **MISSING** | - | MISSING |
| `ForeignInfluence` | `ForeignInfluence` | **MISSING** | - | MISSING |
| `Forethought` | `Forethought` | `Forethought` | YES | FULL |
| `Fury` | `DEPRECATEDFury` | **MISSING** | - | MISSING |
| `FuryAura` | `DEPRECATEDFuryAura` | **MISSING** | - | MISSING |
| `Fusion` | `Fusion` | **MISSING** | - | MISSING |
| `Gash` | `Claw` | **MISSING** | - | MISSING |
| `Genetic Algorithm` | `GeneticAlgorithm` | **MISSING** | - | MISSING |
| `Ghostly` | `Apparition` | `Ghostly` | NO (variant missing) | SCHEMA_ONLY |
| `Ghostly Armor` | `GhostlyArmor` | `Ghostly Armor` | NO (variant missing) | SCHEMA_ONLY |
| `Glacier` | `Glacier` | **MISSING** | - | MISSING |
| `Glass Knife` | `GlassKnife` | **MISSING** | - | MISSING |
| `Go for the Eyes` | `GoForTheEyes` | **MISSING** | - | MISSING |
| `Good Instincts` | `GoodInstincts` | `Good Instincts` | NO (variant missing) | SCHEMA_ONLY |
| `Grand Finale` | `GrandFinale` | **MISSING** | - | MISSING |
| `Grounded` | `DEPRECATEDGrounded` | **MISSING** | - | MISSING |
| `Halt` | `Halt` | **MISSING** | - | MISSING |
| `HandOfGreed` | `HandOfGreed` | `HandOfGreed` | YES | FULL |
| `Havoc` | `Havoc` | `Havoc` | YES | FULL |
| `Headbutt` | `Headbutt` | `Headbutt` | YES | FULL |
| `Heatsinks` | `Heatsinks` | **MISSING** | - | MISSING |
| `Heavy Blade` | `HeavyBlade` | `Heavy Blade` | NO (variant missing) | SCHEMA_ONLY |
| `Heel Hook` | `HeelHook` | **MISSING** | - | MISSING |
| `Hello World` | `HelloWorld` | **MISSING** | - | MISSING |
| `Hemokinesis` | `Hemokinesis` | `Hemokinesis` | YES | FULL |
| `Hologram` | `Hologram` | **MISSING** | - | MISSING |
| `HotHot` | `DEPRECATEDHotHot` | **MISSING** | - | MISSING |
| `Hyperbeam` | `Hyperbeam` | **MISSING** | - | MISSING |
| `Immolate` | `Immolate` | `Immolate` | YES | FULL |
| `Impatience` | `Impatience` | `Impatience` | YES | FULL |
| `Impervious` | `Impervious` | `Impervious` | YES | FULL |
| `Impulse` | `Impulse` | **MISSING** | - | MISSING |
| `Indignation` | `Indignation` | **MISSING** | - | MISSING |
| `Infernal Blade` | `InfernalBlade` | `Infernal Blade` | NO (variant missing) | SCHEMA_ONLY |
| `Infinite Blades` | `InfiniteBlades` | **MISSING** | - | MISSING |
| `Inflame` | `Inflame` | `Inflame` | YES | FULL |
| `Injury` | `Injury` | `Injury` | YES | FULL |
| `InnerPeace` | `InnerPeace` | **MISSING** | - | MISSING |
| `Insight` | `Insight` | **MISSING** | - | MISSING |
| `Intimidate` | `Intimidate` | `Intimidate` | YES | FULL |
| `Introspection` | `DEPRECATEDIntrospection` | **MISSING** | - | MISSING |
| `Iron Wave` | `IronWave` | `Iron Wave` | NO (variant missing) | SCHEMA_ONLY |
| `J.A.X.` | `JAX` | `J.A.X.` | NO (variant missing) | SCHEMA_ONLY |
| `Jack Of All Trades` | `JackOfAllTrades` | `Jack Of All Trades` | NO (variant missing) | SCHEMA_ONLY |
| `Joy` | `DEPRECATEDChooseCourage` | **MISSING** | - | MISSING |
| `Judgement` | `Judgement` | **MISSING** | - | MISSING |
| `Juggernaut` | `Juggernaut` | `Juggernaut` | YES | FULL |
| `JustLucky` | `JustLucky` | **MISSING** | - | MISSING |
| `Leap` | `Leap` | **MISSING** | - | MISSING |
| `Leg Sweep` | `LegSweep` | **MISSING** | - | MISSING |
| `LessonLearned` | `LessonLearned` | **MISSING** | - | MISSING |
| `LetFateDecide` | `DEPRECATEDLetFateDecide` | **MISSING** | - | MISSING |
| `LikeWater` | `LikeWater` | **MISSING** | - | MISSING |
| `Limit Break` | `LimitBreak` | `Limit Break` | NO (variant missing) | SCHEMA_ONLY |
| `LiveForever` | `LiveForever` | **MISSING** | - | MISSING |
| `Lockon` | `LockOn` | **MISSING** | - | MISSING |
| `Loop` | `Loop` | **MISSING** | - | MISSING |
| `Machine Learning` | `MachineLearning` | **MISSING** | - | MISSING |
| `Madness` | `Madness` | `Madness` | YES | FULL |
| `Magnetism` | `Magnetism` | `Magnetism` | YES | FULL |
| `Malaise` | `Malaise` | **MISSING** | - | MISSING |
| `Master of Strategy` | `MasterOfStrategy` | `Master of Strategy` | NO (variant missing) | SCHEMA_ONLY |
| `MasterReality` | `DEPRECATEDMasterReality` | **MISSING** | - | MISSING |
| `MasterReality` | `MasterReality` | **MISSING** | - | MISSING |
| `Masterful Stab` | `MasterfulStab` | **MISSING** | - | MISSING |
| `Mastery` | `DEPRECATEDMastery` | **MISSING** | - | MISSING |
| `Mayhem` | `Mayhem` | `Mayhem` | YES | FULL |
| `Meditate` | `Meditate` | **MISSING** | - | MISSING |
| `Melter` | `Melter` | **MISSING** | - | MISSING |
| `MentalFortress` | `MentalFortress` | **MISSING** | - | MISSING |
| `Metallicize` | `Metallicize` | `Metallicize` | YES | FULL |
| `Metamorphosis` | `Metamorphosis` | `Metamorphosis` | YES | FULL |
| `Metaphysics` | `DEPRECATEDMetaphysics` | **MISSING** | - | MISSING |
| `Meteor Strike` | `MeteorStrike` | **MISSING** | - | MISSING |
| `Mind Blast` | `MindBlast` | `Mind Blast` | NO (variant missing) | SCHEMA_ONLY |
| `Miracle` | `Miracle` | **MISSING** | - | MISSING |
| `Multi-Cast` | `MultiCast` | **MISSING** | - | MISSING |
| `Necronomicurse` | `Necronomicurse` | `Necronomicurse` | YES | FULL |
| `Neutralize` | `Neutralize` | **MISSING** | - | MISSING |
| `Night Terror` | `Nightmare` | **MISSING** | - | MISSING |
| `Nirvana` | `Nirvana` | **MISSING** | - | MISSING |
| `Normality` | `Normality` | `Normality` | YES | FULL |
| `Nothingness` | `DEPRECATEDNothingness` | **MISSING** | - | MISSING |
| `Noxious Fumes` | `NoxiousFumes` | **MISSING** | - | MISSING |
| `Offering` | `Offering` | `Offering` | YES | FULL |
| `Omega` | `Omega` | **MISSING** | - | MISSING |
| `Omniscience` | `Omniscience` | **MISSING** | - | MISSING |
| `Outmaneuver` | `Outmaneuver` | **MISSING** | - | MISSING |
| `Pain` | `Pain` | `Pain` | YES | FULL |
| `PalmThatRestrains` | `DEPRECATEDRestrainingPalm` | **MISSING** | - | MISSING |
| `Panacea` | `Panacea` | `Panacea` | YES | FULL |
| `Panache` | `Panache` | `Panache` | YES | FULL |
| `PanicButton` | `PanicButton` | `PanicButton` | YES | FULL |
| `Parasite` | `Parasite` | `Parasite` | YES | FULL |
| `PathToVictory` | `DEPRECATEDPathToVictory` | **MISSING** | - | MISSING |
| `PathToVictory` | `PressurePoints` | **MISSING** | - | MISSING |
| `Peace` | `DEPRECATEDPeace` | **MISSING** | - | MISSING |
| `Perfected Strike` | `PerfectedStrike` | `Perfected Strike` | NO (variant missing) | SCHEMA_ONLY |
| `PerfectedForm` | `DEPRECATEDPerfectedForm` | **MISSING** | - | MISSING |
| `Perseverance` | `Perseverance` | **MISSING** | - | MISSING |
| `Phantasmal Killer` | `PhantasmalKiller` | **MISSING** | - | MISSING |
| `PiercingWail` | `PiercingWail` | **MISSING** | - | MISSING |
| `Poisoned Stab` | `PoisonedStab` | **MISSING** | - | MISSING |
| `Polymath` | `DEPRECATEDPolymath` | **MISSING** | - | MISSING |
| `Pommel Strike` | `PommelStrike` | `Pommel Strike` | NO (variant missing) | SCHEMA_ONLY |
| `Power Through` | `PowerThrough` | `Power Through` | NO (variant missing) | SCHEMA_ONLY |
| `Pray` | `Pray` | **MISSING** | - | MISSING |
| `Predator` | `Predator` | **MISSING** | - | MISSING |
| `Prediction` | `DEPRECATEDPrediction` | **MISSING** | - | MISSING |
| `Prepared` | `Prepared` | **MISSING** | - | MISSING |
| `Pride` | `Pride` | `Pride` | YES | FULL |
| `Prostrate` | `Prostrate` | **MISSING** | - | MISSING |
| `Protect` | `Protect` | **MISSING** | - | MISSING |
| `Pummel` | `Pummel` | `Pummel` | YES | FULL |
| `Punishment` | `DEPRECATEDPunishment` | **MISSING** | - | MISSING |
| `Purity` | `Purity` | `Purity` | YES | FULL |
| `Quick Slash` | `QuickSlash` | **MISSING** | - | MISSING |
| `Rage` | `Rage` | `Rage` | YES | FULL |
| `Ragnarok` | `Ragnarok` | **MISSING** | - | MISSING |
| `Rainbow` | `Rainbow` | **MISSING** | - | MISSING |
| `Rampage` | `Rampage` | `Rampage` | YES | FULL |
| `ReachHeaven` | `ReachHeaven` | **MISSING** | - | MISSING |
| `Reaper` | `Reaper` | `Reaper` | YES | FULL |
| `Reboot` | `Reboot` | **MISSING** | - | MISSING |
| `Rebound` | `Rebound` | **MISSING** | - | MISSING |
| `Reckless Charge` | `RecklessCharge` | `Reckless Charge` | NO (variant missing) | SCHEMA_ONLY |
| `Recycle` | `Recycle` | **MISSING** | - | MISSING |
| `Redo` | `Recursion` | **MISSING** | - | MISSING |
| `Reflex` | `Reflex` | **MISSING** | - | MISSING |
| `Regret` | `Regret` | `Regret` | YES | FULL |
| `Reinforced Body` | `ReinforcedBody` | **MISSING** | - | MISSING |
| `Reprogram` | `Reprogram` | **MISSING** | - | MISSING |
| `RetreatingHand` | `DEPRECATEDRetreatingHand` | **MISSING** | - | MISSING |
| `Retribution` | `DEPRECATEDRetribution` | **MISSING** | - | MISSING |
| `Riddle With Holes` | `RiddleWithHoles` | **MISSING** | - | MISSING |
| `Rip and Tear` | `RipAndTear` | **MISSING** | - | MISSING |
| `RitualDagger` | `RitualDagger` | `RitualDagger` | YES | FULL |
| `Rupture` | `Rupture` | `Rupture` | YES | FULL |
| `Sadistic Nature` | `SadisticNature` | `Sadistic Nature` | NO (variant missing) | SCHEMA_ONLY |
| `Safety` | `Safety` | **MISSING** | - | MISSING |
| `Sanctity` | `Sanctity` | **MISSING** | - | MISSING |
| `SandsOfTime` | `SandsOfTime` | **MISSING** | - | MISSING |
| `SashWhip` | `SashWhip` | **MISSING** | - | MISSING |
| `Scrape` | `Scrape` | **MISSING** | - | MISSING |
| `Scrawl` | `Scrawl` | **MISSING** | - | MISSING |
| `Searing Blow` | `SearingBlow` | `Searing Blow` | NO (variant missing) | SCHEMA_ONLY |
| `Second Wind` | `SecondWind` | `Second Wind` | NO (variant missing) | SCHEMA_ONLY |
| `Secret Technique` | `SecretTechnique` | `Secret Technique` | NO (variant missing) | SCHEMA_ONLY |
| `Secret Weapon` | `SecretWeapon` | `Secret Weapon` | NO (variant missing) | SCHEMA_ONLY |
| `Seeing Red` | `SeeingRed` | `Seeing Red` | NO (variant missing) | SCHEMA_ONLY |
| `Seek` | `Seek` | **MISSING** | - | MISSING |
| `Self Repair` | `SelfRepair` | **MISSING** | - | MISSING |
| `Sentinel` | `Sentinel` | `Sentinel` | YES | FULL |
| `Serenity` | `DEPRECATEDSerenity` | **MISSING** | - | MISSING |
| `Setup` | `Setup` | **MISSING** | - | MISSING |
| `Sever Soul` | `SeverSoul` | `Sever Soul` | NO (variant missing) | SCHEMA_ONLY |
| `Shame` | `Shame` | `Shame` | YES | FULL |
| `Shiv` | `Shiv` | **MISSING** | - | MISSING |
| `Shockwave` | `Shockwave` | `Shockwave` | YES | FULL |
| `Shrug It Off` | `ShrugItOff` | `Shrug It Off` | NO (variant missing) | SCHEMA_ONLY |
| `SignatureMove` | `SignatureMove` | **MISSING** | - | MISSING |
| `SimmeringRage` | `DEPRECATEDSimmeringRage` | **MISSING** | - | MISSING |
| `Skewer` | `Skewer` | **MISSING** | - | MISSING |
| `Skim` | `Skim` | **MISSING** | - | MISSING |
| `Slice` | `Slice` | **MISSING** | - | MISSING |
| `Slimed` | `Slimed` | `Slimed` | YES | FULL |
| `Smile` | `DEPRECATEDSmile` | **MISSING** | - | MISSING |
| `Smite` | `Smite` | **MISSING** | - | MISSING |
| `SoothingAura` | `DEPRECATEDSoothingAura` | **MISSING** | - | MISSING |
| `SpiritShield` | `SpiritShield` | **MISSING** | - | MISSING |
| `Spot Weakness` | `SpotWeakness` | `Spot Weakness` | NO (variant missing) | SCHEMA_ONLY |
| `Stack` | `Stack` | **MISSING** | - | MISSING |
| `Static Discharge` | `StaticDischarge` | **MISSING** | - | MISSING |
| `Steam` | `SteamBarrier` | **MISSING** | - | MISSING |
| `Steam Power` | `Overclock` | **MISSING** | - | MISSING |
| `StepAndStrike` | `DEPRECATEDStepAndStrike` | **MISSING** | - | MISSING |
| `Stomp` | `DEPRECATEDStomp` | **MISSING** | - | MISSING |
| `Storm` | `Storm` | **MISSING** | - | MISSING |
| `Storm of Steel` | `StormOfSteel` | **MISSING** | - | MISSING |
| `Streamline` | `Streamline` | **MISSING** | - | MISSING |
| `Strike_B` | `Strike_Blue` | **MISSING** | - | MISSING |
| `Strike_G` | `Strike_Green` | **MISSING** | - | MISSING |
| `Strike_P` | `Strike_Purple` | **MISSING** | - | MISSING |
| `Strike_R` | `Strike_Red` | `Strike_R` | NO (variant missing) | SCHEMA_ONLY |
| `Study` | `Study` | **MISSING** | - | MISSING |
| `SublimeSlice` | `DEPRECATEDSublimeSlice` | **MISSING** | - | MISSING |
| `Sucker Punch` | `SuckerPunch` | **MISSING** | - | MISSING |
| `Sunder` | `Sunder` | **MISSING** | - | MISSING |
| `Survey` | `DEPRECATEDSurvey` | **MISSING** | - | MISSING |
| `Survivor` | `Survivor` | **MISSING** | - | MISSING |
| `Sweeping Beam` | `SweepingBeam` | **MISSING** | - | MISSING |
| `Swift Strike` | `SwiftStrike` | `Swift Strike` | NO (variant missing) | SCHEMA_ONLY |
| `Swipe` | `DEPRECATEDSwipe` | **MISSING** | - | MISSING |
| `Swivel` | `Swivel` | **MISSING** | - | MISSING |
| `Sword Boomerang` | `SwordBoomerang` | `Sword Boomerang` | NO (variant missing) | SCHEMA_ONLY |
| `Tactician` | `Tactician` | **MISSING** | - | MISSING |
| `TalkToTheHand` | `TalkToTheHand` | **MISSING** | - | MISSING |
| `Tantrum` | `Tantrum` | **MISSING** | - | MISSING |
| `TemperTantrum` | `DEPRECATEDTemperTantrum` | **MISSING** | - | MISSING |
| `Tempest` | `Tempest` | **MISSING** | - | MISSING |
| `Terror` | `Terror` | **MISSING** | - | MISSING |
| `The Bomb` | `TheBomb` | `The Bomb` | NO (variant missing) | SCHEMA_ONLY |
| `Thinking Ahead` | `ThinkingAhead` | `Thinking Ahead` | NO (variant missing) | SCHEMA_ONLY |
| `ThirdEye` | `ThirdEye` | **MISSING** | - | MISSING |
| `ThroughViolence` | `ThroughViolence` | **MISSING** | - | MISSING |
| `Thunder Strike` | `ThunderStrike` | **MISSING** | - | MISSING |
| `Thunderclap` | `ThunderClap` | `Thunderclap` | NO (variant missing) | SCHEMA_ONLY |
| `Tools of the Trade` | `ToolsOfTheTrade` | **MISSING** | - | MISSING |
| `Torrent` | `DEPRECATEDTorrent` | **MISSING** | - | MISSING |
| `Transcendence` | `DEPRECATEDTranscendence` | **MISSING** | - | MISSING |
| `Transmutation` | `Transmutation` | `Transmutation` | YES | FULL |
| `Trip` | `Trip` | `Trip` | YES | FULL |
| `True Grit` | `TrueGrit` | `True Grit` | NO (variant missing) | SCHEMA_ONLY |
| `Truth` | `DEPRECATEDTruth` | **MISSING** | - | MISSING |
| `Turbo` | `Turbo` | **MISSING** | - | MISSING |
| `Twin Strike` | `TwinStrike` | `Twin Strike` | NO (variant missing) | SCHEMA_ONLY |
| `Underhanded Strike` | `SneakyStrike` | **MISSING** | - | MISSING |
| `Undo` | `Equilibrium` | **MISSING** | - | MISSING |
| `Unload` | `Unload` | **MISSING** | - | MISSING |
| `Unraveling` | `Unraveling` | **MISSING** | - | MISSING |
| `Uppercut` | `Uppercut` | `Uppercut` | YES | FULL |
| `Vault` | `Vault` | **MISSING** | - | MISSING |
| `Vengeance` | `SimmeringFury` | **MISSING** | - | MISSING |
| `Venomology` | `Alchemize` | **MISSING** | - | MISSING |
| `Vigilance` | `Vigilance` | **MISSING** | - | MISSING |
| `Violence` | `Violence` | `Violence` | YES | FULL |
| `Void` | `VoidCard` | `Void` | YES | FULL |
| `Wallop` | `Wallop` | **MISSING** | - | MISSING |
| `Warcry` | `Warcry` | `Warcry` | YES | FULL |
| `WardAura` | `DEPRECATEDWardAura` | **MISSING** | - | MISSING |
| `WaveOfTheHand` | `WaveOfTheHand` | **MISSING** | - | MISSING |
| `Weave` | `Weave` | **MISSING** | - | MISSING |
| `Well Laid Plans` | `WellLaidPlans` | **MISSING** | - | MISSING |
| `WheelKick` | `WheelKick` | **MISSING** | - | MISSING |
| `Whirlwind` | `Whirlwind` | `Whirlwind` | YES | FULL |
| `White Noise` | `WhiteNoise` | **MISSING** | - | MISSING |
| `Wild Strike` | `WildStrike` | `Wild Strike` | NO (variant missing) | SCHEMA_ONLY |
| `WindmillStrike` | `WindmillStrike` | **MISSING** | - | MISSING |
| `Windup` | `DEPRECATEDWindup` | **MISSING** | - | MISSING |
| `Wireheading` | `Foresight` | **MISSING** | - | MISSING |
| `Wisdom` | `DEPRECATEDWisdom` | **MISSING** | - | MISSING |
| `Wish` | `Wish` | **MISSING** | - | MISSING |
| `Worship` | `Worship` | **MISSING** | - | MISSING |
| `Wound` | `Wound` | `Wound` | YES | FULL |
| `Wraith Form v2` | `WraithForm` | **MISSING** | - | MISSING |
| `Wrath` | `DEPRECATEDWrath` | **MISSING** | - | MISSING |
| `Wrath` | `ChooseWrath` | **MISSING** | - | MISSING |
| `WreathOfFlame` | `WreathOfFlame` | **MISSING** | - | MISSING |
| `Writhe` | `Writhe` | `Writhe` | YES | FULL |
| `Zap` | `Zap` | **MISSING** | - | MISSING |

## MONSTER (0/66 in schema, 0/66 fully covered, 65 Rust variants)

| Java ID | Class | Schema -> Rust | In Rust Enum | Status |
|---------|-------|----------------|--------------|--------|
| `AcidSlime_L` | `AcidSlime_L` | **MISSING** | - | MISSING |
| `AcidSlime_M` | `AcidSlime_M` | **MISSING** | - | MISSING |
| `AcidSlime_S` | `AcidSlime_S` | **MISSING** | - | MISSING |
| `Apology Slime` | `ApologySlime` | **MISSING** | - | MISSING |
| `AwakenedOne` | `AwakenedOne` | **MISSING** | - | MISSING |
| `BanditBear` | `BanditBear` | **MISSING** | - | MISSING |
| `BanditChild` | `BanditPointy` | **MISSING** | - | MISSING |
| `BanditLeader` | `BanditLeader` | **MISSING** | - | MISSING |
| `BookOfStabbing` | `BookOfStabbing` | **MISSING** | - | MISSING |
| `BronzeAutomaton` | `BronzeAutomaton` | **MISSING** | - | MISSING |
| `BronzeOrb` | `BronzeOrb` | **MISSING** | - | MISSING |
| `Byrd` | `Byrd` | **MISSING** | - | MISSING |
| `Centurion` | `Centurion` | **MISSING** | - | MISSING |
| `Champ` | `Champ` | **MISSING** | - | MISSING |
| `Chosen` | `Chosen` | **MISSING** | - | MISSING |
| `CorruptHeart` | `CorruptHeart` | **MISSING** | - | MISSING |
| `Cultist` | `Cultist` | **MISSING** | - | MISSING |
| `Dagger` | `SnakeDagger` | **MISSING** | - | MISSING |
| `Darkling` | `Darkling` | **MISSING** | - | MISSING |
| `Deca` | `Deca` | **MISSING** | - | MISSING |
| `Donu` | `Donu` | **MISSING** | - | MISSING |
| `Exploder` | `Exploder` | **MISSING** | - | MISSING |
| `FungiBeast` | `FungiBeast` | **MISSING** | - | MISSING |
| `FuzzyLouseDefensive` | `LouseDefensive` | **MISSING** | - | MISSING |
| `FuzzyLouseNormal` | `LouseNormal` | **MISSING** | - | MISSING |
| `GiantHead` | `GiantHead` | **MISSING** | - | MISSING |
| `GremlinFat` | `GremlinFat` | **MISSING** | - | MISSING |
| `GremlinLeader` | `GremlinLeader` | **MISSING** | - | MISSING |
| `GremlinNob` | `GremlinNob` | **MISSING** | - | MISSING |
| `GremlinThief` | `GremlinThief` | **MISSING** | - | MISSING |
| `GremlinTsundere` | `GremlinTsundere` | **MISSING** | - | MISSING |
| `GremlinWarrior` | `GremlinWarrior` | **MISSING** | - | MISSING |
| `GremlinWizard` | `GremlinWizard` | **MISSING** | - | MISSING |
| `Healer` | `Healer` | **MISSING** | - | MISSING |
| `Hexaghost` | `Hexaghost` | **MISSING** | - | MISSING |
| `JawWorm` | `JawWorm` | **MISSING** | - | MISSING |
| `Lagavulin` | `Lagavulin` | **MISSING** | - | MISSING |
| `Looter` | `Looter` | **MISSING** | - | MISSING |
| `Maw` | `Maw` | **MISSING** | - | MISSING |
| `Mugger` | `Mugger` | **MISSING** | - | MISSING |
| `Nemesis` | `Nemesis` | **MISSING** | - | MISSING |
| `Orb Walker` | `OrbWalker` | **MISSING** | - | MISSING |
| `Reptomancer` | `Reptomancer` | **MISSING** | - | MISSING |
| `Repulsor` | `Repulsor` | **MISSING** | - | MISSING |
| `Sentry` | `Sentry` | **MISSING** | - | MISSING |
| `Serpent` | `SpireGrowth` | **MISSING** | - | MISSING |
| `Shelled Parasite` | `ShelledParasite` | **MISSING** | - | MISSING |
| `SlaverBlue` | `SlaverBlue` | **MISSING** | - | MISSING |
| `SlaverBoss` | `Taskmaster` | **MISSING** | - | MISSING |
| `SlaverRed` | `SlaverRed` | **MISSING** | - | MISSING |
| `SlimeBoss` | `SlimeBoss` | **MISSING** | - | MISSING |
| `SnakePlant` | `SnakePlant` | **MISSING** | - | MISSING |
| `Snecko` | `Snecko` | **MISSING** | - | MISSING |
| `SphericGuardian` | `SphericGuardian` | **MISSING** | - | MISSING |
| `SpikeSlime_L` | `SpikeSlime_L` | **MISSING** | - | MISSING |
| `SpikeSlime_M` | `SpikeSlime_M` | **MISSING** | - | MISSING |
| `SpikeSlime_S` | `SpikeSlime_S` | **MISSING** | - | MISSING |
| `Spiker` | `Spiker` | **MISSING** | - | MISSING |
| `SpireShield` | `SpireShield` | **MISSING** | - | MISSING |
| `SpireSpear` | `SpireSpear` | **MISSING** | - | MISSING |
| `TheCollector` | `TheCollector` | **MISSING** | - | MISSING |
| `TheGuardian` | `TheGuardian` | **MISSING** | - | MISSING |
| `TimeEater` | `TimeEater` | **MISSING** | - | MISSING |
| `TorchHead` | `TorchHead` | **MISSING** | - | MISSING |
| `Transient` | `Transient` | **MISSING** | - | MISSING |
| `WrithingMass` | `WrithingMass` | **MISSING** | - | MISSING |

## Summary

- **power**: 67/161 (42%) in schema, 67/161 (42%) fully covered, **94 missing from schema**
- **relic**: 35/190 (18%) in schema, 0/190 (0%) fully covered, **155 missing from schema**
- **potion**: 42/43 (98%) in schema, 35/43 (81%) fully covered, **1 missing from schema**
- **card**: 133/438 (30%) in schema, 82/438 (19%) fully covered, **305 missing from schema**
- **monster**: 0/66 (0%) in schema, 0/66 (0%) fully covered, **66 missing from schema**

## Missing Items (Not in Schema)

### POWER (94 missing)

- `Accuracy` (class: `AccuracyPower`, file: `powers\AccuracyPower.java`)
- `Adaptation` (class: `RushdownPower`, file: `powers\watcher\RushdownPower.java`)
- `After Image` (class: `AfterImagePower`, file: `powers\AfterImagePower.java`)
- `AlwaysMad` (class: `DEPRECATEDAlwaysMadPower`, file: `powers\deprecated\DEPRECATEDAlwaysMadPower.java`)
- `Amplify` (class: `AmplifyPower`, file: `powers\AmplifyPower.java`)
- `AngelForm` (class: `LiveForeverPower`, file: `powers\watcher\LiveForeverPower.java`)
- `Attack Burn` (class: `AttackBurnPower`, file: `powers\AttackBurnPower.java`)
- `BackAttack` (class: `BackAttackPower`, file: `powers\BackAttackPower.java`)
- `BattleHymn` (class: `BattleHymnPower`, file: `powers\watcher\BattleHymnPower.java`)
- `BeatOfDeath` (class: `BeatOfDeathPower`, file: `powers\BeatOfDeathPower.java`)
- `Bias` (class: `BiasPower`, file: `powers\BiasPower.java`)
- `BlockReturnPower` (class: `BlockReturnPower`, file: `powers\watcher\BlockReturnPower.java`)
- `Blur` (class: `BlurPower`, file: `powers\BlurPower.java`)
- `Burst` (class: `BurstPower`, file: `powers\BurstPower.java`)
- `CannotChangeStancePower` (class: `CannotChangeStancePower`, file: `powers\watcher\CannotChangeStancePower.java`)
- `Choked` (class: `ChokePower`, file: `powers\ChokePower.java`)
- `Collect` (class: `CollectPower`, file: `powers\CollectPower.java`)
- `Compulsive` (class: `ReactivePower`, file: `powers\ReactivePower.java`)
- `Conserve` (class: `ConservePower`, file: `powers\ConservePower.java`)
- `Controlled` (class: `MentalFortressPower`, file: `powers\watcher\MentalFortressPower.java`)
- `CorpseExplosionPower` (class: `CorpseExplosionPower`, file: `powers\CorpseExplosionPower.java`)
- `Creative AI` (class: `CreativeAIPower`, file: `powers\CreativeAIPower.java`)
- `DEPRECATEDCondense` (class: `DEPRECATEDCondensePower`, file: `powers\deprecated\DEPRECATEDCondensePower.java`)
- `DevaForm` (class: `DevaPower`, file: `powers\watcher\DevaPower.java`)
- `DevotionPower` (class: `DevotionPower`, file: `powers\watcher\DevotionPower.java`)
- `DisciplinePower` (class: `DEPRECATEDDisciplinePower`, file: `powers\deprecated\DEPRECATEDDisciplinePower.java`)
- `Double Damage` (class: `DoubleDamagePower`, file: `powers\DoubleDamagePower.java`)
- `Draw` (class: `DrawPower`, file: `powers\DrawPower.java`)
- `Draw Card` (class: `DrawCardNextTurnPower`, file: `powers\DrawCardNextTurnPower.java`)
- `Draw Reduction` (class: `DrawReductionPower`, file: `powers\DrawReductionPower.java`)
- `DuplicationPower` (class: `DuplicationPower`, file: `powers\DuplicationPower.java`)
- `Echo Form` (class: `EchoPower`, file: `powers\EchoPower.java`)
- `Electro` (class: `ElectroPower`, file: `powers\ElectroPower.java`)
- `EmotionalTurmoilPower` (class: `DEPRECATEDEmotionalTurmoilPower`, file: `powers\deprecated\DEPRECATEDEmotionalTurmoilPower.java`)
- `EndTurnDeath` (class: `EndTurnDeathPower`, file: `powers\watcher\EndTurnDeathPower.java`)
- `EnergizedBlue` (class: `EnergizedBluePower`, file: `powers\EnergizedBluePower.java`)
- `EnergyDownPower` (class: `EnergyDownPower`, file: `powers\watcher\EnergyDownPower.java`)
- `Envenom` (class: `EnvenomPower`, file: `powers\EnvenomPower.java`)
- `Equilibrium` (class: `EquilibriumPower`, file: `powers\EquilibriumPower.java`)
- `EstablishmentPower` (class: `EstablishmentPower`, file: `powers\watcher\EstablishmentPower.java`)
- `Flex` (class: `LoseStrengthPower`, file: `powers\LoseStrengthPower.java`)
- `FlickPower` (class: `DEPRECATEDFlickedPower`, file: `powers\deprecated\DEPRECATEDFlickedPower.java`)
- `FlowPower` (class: `DEPRECATEDFlowPower`, file: `powers\deprecated\DEPRECATEDFlowPower.java`)
- `FreeAttackPower` (class: `FreeAttackPower`, file: `powers\watcher\FreeAttackPower.java`)
- `Generic Strength Up Power` (class: `GenericStrengthUpPower`, file: `powers\GenericStrengthUpPower.java`)
- `Grounded` (class: `DEPRECATEDGroundedPower`, file: `powers\deprecated\DEPRECATEDGroundedPower.java`)
- `GrowthPower` (class: `GrowthPower`, file: `powers\GrowthPower.java`)
- `Heatsink` (class: `HeatsinkPower`, file: `powers\HeatsinkPower.java`)
- `Hello` (class: `HelloPower`, file: `powers\HelloPower.java`)
- `HotHot` (class: `DEPRECATEDHotHotPower`, file: `powers\deprecated\DEPRECATEDHotHotPower.java`)
- `Infinite Blades` (class: `InfiniteBladesPower`, file: `powers\InfiniteBladesPower.java`)
- `IntangiblePlayer` (class: `IntangiblePlayerPower`, file: `powers\IntangiblePlayerPower.java`)
- `Life Link` (class: `RegrowPower`, file: `powers\RegrowPower.java`)
- `Life Link` (class: `ResurrectPower`, file: `powers\ResurrectPower.java`)
- `Lightning Mastery` (class: `LightningMasteryPower`, file: `powers\LightningMasteryPower.java`)
- `LikeWaterPower` (class: `LikeWaterPower`, file: `powers\watcher\LikeWaterPower.java`)
- `Lockon` (class: `LockOnPower`, file: `powers\LockOnPower.java`)
- `Loop` (class: `LoopPower`, file: `powers\LoopPower.java`)
- `MasterRealityPower` (class: `DEPRECATEDMasterRealityPower`, file: `powers\deprecated\DEPRECATEDMasterRealityPower.java`)
- `MasterRealityPower` (class: `MasterRealityPower`, file: `powers\watcher\MasterRealityPower.java`)
- `Mastery` (class: `DEPRECATEDMasteryPower`, file: `powers\deprecated\DEPRECATEDMasteryPower.java`)
- `Night Terror` (class: `NightmarePower`, file: `powers\NightmarePower.java`)
- `Nirvana` (class: `NirvanaPower`, file: `powers\watcher\NirvanaPower.java`)
- `NoBlockPower` (class: `NoBlockPower`, file: `powers\NoBlockPower.java`)
- `NoSkills` (class: `NoSkillsPower`, file: `powers\watcher\NoSkillsPower.java`)
- `Noxious Fumes` (class: `NoxiousFumesPower`, file: `powers\NoxiousFumesPower.java`)
- `Nullify Attack` (class: `ForcefieldPower`, file: `powers\ForcefieldPower.java`)
- `OmegaPower` (class: `OmegaPower`, file: `powers\watcher\OmegaPower.java`)
- `OmnisciencePower` (class: `OmnisciencePower`, file: `powers\watcher\OmnisciencePower.java`)
- `PathToVictoryPower` (class: `MarkPower`, file: `powers\watcher\MarkPower.java`)
- `Phantasmal` (class: `PhantasmalPower`, file: `powers\PhantasmalPower.java`)
- `Rebound` (class: `ReboundPower`, file: `powers\ReboundPower.java`)
- `RechargingCore` (class: `RechargingCorePower`, file: `powers\RechargingCorePower.java`)
- `Repair` (class: `RepairPower`, file: `powers\RepairPower.java`)
- `Retain Cards` (class: `RetainCardPower`, file: `powers\RetainCardPower.java`)
- `Retribution` (class: `DEPRECATEDRetributionPower`, file: `powers\deprecated\DEPRECATEDRetributionPower.java`)
- `Sadistic` (class: `SadisticPower`, file: `powers\SadisticPower.java`)
- `Serenity` (class: `DEPRECATEDSerenityPower`, file: `powers\deprecated\DEPRECATEDSerenityPower.java`)
- `Shackled` (class: `GainStrengthPower`, file: `powers\GainStrengthPower.java`)
- `Skill Burn` (class: `SkillBurnPower`, file: `powers\SkillBurnPower.java`)
- `StaticDischarge` (class: `StaticDischargePower`, file: `powers\StaticDischargePower.java`)
- `Storm` (class: `StormPower`, file: `powers\StormPower.java`)
- `StrikeUp` (class: `StrikeUpPower`, file: `powers\StrikeUpPower.java`)
- `Study` (class: `StudyPower`, file: `powers\watcher\StudyPower.java`)
- `Surrounded` (class: `SurroundedPower`, file: `powers\SurroundedPower.java`)
- `Thousand Cuts` (class: `ThousandCutsPower`, file: `powers\ThousandCutsPower.java`)
- `TimeMazePower` (class: `TimeMazePower`, file: `powers\TimeMazePower.java`)
- `Tools Of The Trade` (class: `ToolsOfTheTradePower`, file: `powers\ToolsOfTheTradePower.java`)
- `Vault` (class: `VaultPower`, file: `powers\watcher\VaultPower.java`)
- `WaveOfTheHandPower` (class: `WaveOfTheHandPower`, file: `powers\watcher\WaveOfTheHandPower.java`)
- `Winter` (class: `WinterPower`, file: `powers\WinterPower.java`)
- `WireheadingPower` (class: `ForesightPower`, file: `powers\watcher\ForesightPower.java`)
- `Wraith Form v2` (class: `WraithFormPower`, file: `powers\WraithFormPower.java`)
- `WrathNextTurnPower` (class: `WrathNextTurnPower`, file: `powers\watcher\WrathNextTurnPower.java`)

### RELIC (155 missing)

- `Ancient Tea Set` (class: `AncientTeaSet`, file: `relics\AncientTeaSet.java`)
- `Art of War` (class: `ArtOfWar`, file: `relics\ArtOfWar.java`)
- `Astrolabe` (class: `Astrolabe`, file: `relics\Astrolabe.java`)
- `Bag of Preparation` (class: `BagOfPreparation`, file: `relics\BagOfPreparation.java`)
- `Bird Faced Urn` (class: `BirdFacedUrn`, file: `relics\BirdFacedUrn.java`)
- `Black Blood` (class: `BlackBlood`, file: `relics\BlackBlood.java`)
- `Black Star` (class: `BlackStar`, file: `relics\BlackStar.java`)
- `Blood Vial` (class: `BloodVial`, file: `relics\BloodVial.java`)
- `Bloody Idol` (class: `BloodyIdol`, file: `relics\BloodyIdol.java`)
- `Blue Candle` (class: `BlueCandle`, file: `relics\BlueCandle.java`)
- `Boot` (class: `Boot`, file: `relics\Boot.java`)
- `Bottled Flame` (class: `BottledFlame`, file: `relics\BottledFlame.java`)
- `Bottled Lightning` (class: `BottledLightning`, file: `relics\BottledLightning.java`)
- `Bottled Tornado` (class: `BottledTornado`, file: `relics\BottledTornado.java`)
- `Brimstone` (class: `Brimstone`, file: `relics\Brimstone.java`)
- `Busted Crown` (class: `BustedCrown`, file: `relics\BustedCrown.java`)
- `Cables` (class: `GoldPlatedCables`, file: `relics\GoldPlatedCables.java`)
- `Calipers` (class: `Calipers`, file: `relics\Calipers.java`)
- `Calling Bell` (class: `CallingBell`, file: `relics\CallingBell.java`)
- `CaptainsWheel` (class: `CaptainsWheel`, file: `relics\CaptainsWheel.java`)
- `Cauldron` (class: `Cauldron`, file: `relics\Cauldron.java`)
- `CeramicFish` (class: `CeramicFish`, file: `relics\CeramicFish.java`)
- `Champion Belt` (class: `ChampionsBelt`, file: `relics\ChampionsBelt.java`)
- `Charon's Ashes` (class: `CharonsAshes`, file: `relics\CharonsAshes.java`)
- `Chemical X` (class: `ChemicalX`, file: `relics\ChemicalX.java`)
- `Circlet` (class: `Circlet`, file: `relics\Circlet.java`)
- `CloakClasp` (class: `CloakClasp`, file: `relics\CloakClasp.java`)
- `ClockworkSouvenir` (class: `ClockworkSouvenir`, file: `relics\ClockworkSouvenir.java`)
- `Coffee Dripper` (class: `CoffeeDripper`, file: `relics\CoffeeDripper.java`)
- `Cracked Core` (class: `CrackedCore`, file: `relics\CrackedCore.java`)
- `CultistMask` (class: `CultistMask`, file: `relics\CultistMask.java`)
- `Cursed Key` (class: `CursedKey`, file: `relics\CursedKey.java`)
- `Damaru` (class: `Damaru`, file: `relics\Damaru.java`)
- `Dark Core` (class: `DEPRECATED_DarkCore`, file: `relics\deprecated\DEPRECATED_DarkCore.java`)
- `Darkstone Periapt` (class: `DarkstonePeriapt`, file: `relics\DarkstonePeriapt.java`)
- `Dead Branch` (class: `DeadBranch`, file: `relics\DeadBranch.java`)
- `Derp Rock` (class: `DerpRock`, file: `relics\deprecated\DerpRock.java`)
- `Discerning Monocle` (class: `DiscerningMonocle`, file: `relics\DiscerningMonocle.java`)
- `Dodecahedron` (class: `DEPRECATEDDodecahedron`, file: `relics\deprecated\DEPRECATEDDodecahedron.java`)
- `DollysMirror` (class: `DollysMirror`, file: `relics\DollysMirror.java`)
- `Dream Catcher` (class: `DreamCatcher`, file: `relics\DreamCatcher.java`)
- `Du-Vu Doll` (class: `DuVuDoll`, file: `relics\DuVuDoll.java`)
- `Ectoplasm` (class: `Ectoplasm`, file: `relics\Ectoplasm.java`)
- `Emotion Chip` (class: `EmotionChip`, file: `relics\EmotionChip.java`)
- `Empty Cage` (class: `EmptyCage`, file: `relics\EmptyCage.java`)
- `Enchiridion` (class: `Enchiridion`, file: `relics\Enchiridion.java`)
- `FaceOfCleric` (class: `FaceOfCleric`, file: `relics\FaceOfCleric.java`)
- `FossilizedHelix` (class: `FossilizedHelix`, file: `relics\FossilizedHelix.java`)
- `Frozen Eye` (class: `FrozenEye`, file: `relics\FrozenEye.java`)
- `FrozenCore` (class: `FrozenCore`, file: `relics\FrozenCore.java`)
- `Fusion Hammer` (class: `FusionHammer`, file: `relics\FusionHammer.java`)
- `Gambling Chip` (class: `GamblingChip`, file: `relics\GamblingChip.java`)
- `Ginger` (class: `Ginger`, file: `relics\Ginger.java`)
- `Girya` (class: `Girya`, file: `relics\Girya.java`)
- `Golden Idol` (class: `GoldenIdol`, file: `relics\GoldenIdol.java`)
- `GoldenEye` (class: `GoldenEye`, file: `relics\GoldenEye.java`)
- `GremlinMask` (class: `GremlinMask`, file: `relics\GremlinMask.java`)
- `HandDrill` (class: `HandDrill`, file: `relics\HandDrill.java`)
- `HolyWater` (class: `HolyWater`, file: `relics\HolyWater.java`)
- `HornCleat` (class: `HornCleat`, file: `relics\HornCleat.java`)
- `HoveringKite` (class: `HoveringKite`, file: `relics\HoveringKite.java`)
- `Ice Cream` (class: `IceCream`, file: `relics\IceCream.java`)
- `Incense Burner` (class: `IncenseBurner`, file: `relics\IncenseBurner.java`)
- `InkBottle` (class: `InkBottle`, file: `relics\InkBottle.java`)
- `Inserter` (class: `Inserter`, file: `relics\Inserter.java`)
- `Juzu Bracelet` (class: `JuzuBracelet`, file: `relics\JuzuBracelet.java`)
- `Lee's Waffle` (class: `Waffle`, file: `relics\Waffle.java`)
- `Lizard Tail` (class: `LizardTail`, file: `relics\LizardTail.java`)
- `Magic Flower` (class: `MagicFlower`, file: `relics\MagicFlower.java`)
- `Mango` (class: `Mango`, file: `relics\Mango.java`)
- `Mark of Pain` (class: `MarkOfPain`, file: `relics\MarkOfPain.java`)
- `Mark of the Bloom` (class: `MarkOfTheBloom`, file: `relics\MarkOfTheBloom.java`)
- `Matryoshka` (class: `Matryoshka`, file: `relics\Matryoshka.java`)
- `MawBank` (class: `MawBank`, file: `relics\MawBank.java`)
- `MealTicket` (class: `MealTicket`, file: `relics\MealTicket.java`)
- `Medical Kit` (class: `MedicalKit`, file: `relics\MedicalKit.java`)
- `Melange` (class: `Melange`, file: `relics\Melange.java`)
- `Membership Card` (class: `MembershipCard`, file: `relics\MembershipCard.java`)
- `Mummified Hand` (class: `MummifiedHand`, file: `relics\MummifiedHand.java`)
- `MutagenicStrength` (class: `MutagenicStrength`, file: `relics\MutagenicStrength.java`)
- `Necronomicon` (class: `Necronomicon`, file: `relics\Necronomicon.java`)
- `Nilry's Codex` (class: `NilrysCodex`, file: `relics\NilrysCodex.java`)
- `Ninja Scroll` (class: `NinjaScroll`, file: `relics\NinjaScroll.java`)
- `Nloth's Gift` (class: `NlothsGift`, file: `relics\NlothsGift.java`)
- `NlothsMask` (class: `NlothsMask`, file: `relics\NlothsMask.java`)
- `Nuclear Battery` (class: `NuclearBattery`, file: `relics\NuclearBattery.java`)
- `Odd Mushroom` (class: `OddMushroom`, file: `relics\OddMushroom.java`)
- `Old Coin` (class: `OldCoin`, file: `relics\OldCoin.java`)
- `Omamori` (class: `Omamori`, file: `relics\Omamori.java`)
- `OrangePellets` (class: `OrangePellets`, file: `relics\OrangePellets.java`)
- `Orrery` (class: `Orrery`, file: `relics\Orrery.java`)
- `Pandora's Box` (class: `PandorasBox`, file: `relics\PandorasBox.java`)
- `Pantograph` (class: `Pantograph`, file: `relics\Pantograph.java`)
- `Paper Frog` (class: `PaperFrog`, file: `relics\PaperFrog.java`)
- `Peace Pipe` (class: `PeacePipe`, file: `relics\PeacePipe.java`)
- `Pear` (class: `Pear`, file: `relics\Pear.java`)
- `Philosopher's Stone` (class: `PhilosopherStone`, file: `relics\PhilosopherStone.java`)
- `Pocketwatch` (class: `Pocketwatch`, file: `relics\Pocketwatch.java`)
- `Potion Belt` (class: `PotionBelt`, file: `relics\PotionBelt.java`)
- `Prayer Wheel` (class: `PrayerWheel`, file: `relics\PrayerWheel.java`)
- `PreservedInsect` (class: `PreservedInsect`, file: `relics\PreservedInsect.java`)
- `PrismaticShard` (class: `PrismaticShard`, file: `relics\PrismaticShard.java`)
- `PureWater` (class: `PureWater`, file: `relics\PureWater.java`)
- `Question Card` (class: `QuestionCard`, file: `relics\QuestionCard.java`)
- `Red Circlet` (class: `RedCirclet`, file: `relics\RedCirclet.java`)
- `Red Mask` (class: `RedMask`, file: `relics\RedMask.java`)
- `Regal Pillow` (class: `RegalPillow`, file: `relics\RegalPillow.java`)
- `Ring of the Serpent` (class: `RingOfTheSerpent`, file: `relics\RingOfTheSerpent.java`)
- `Ring of the Snake` (class: `SnakeRing`, file: `relics\SnakeRing.java`)
- `Runic Capacitor` (class: `RunicCapacitor`, file: `relics\RunicCapacitor.java`)
- `Runic Dome` (class: `RunicDome`, file: `relics\RunicDome.java`)
- `Runic Pyramid` (class: `RunicPyramid`, file: `relics\RunicPyramid.java`)
- `SacredBark` (class: `SacredBark`, file: `relics\SacredBark.java`)
- `Shovel` (class: `Shovel`, file: `relics\Shovel.java`)
- `Singing Bowl` (class: `SingingBowl`, file: `relics\SingingBowl.java`)
- `SlaversCollar` (class: `SlaversCollar`, file: `relics\SlaversCollar.java`)
- `Sling` (class: `Sling`, file: `relics\Sling.java`)
- `Smiling Mask` (class: `SmilingMask`, file: `relics\SmilingMask.java`)
- `Snake Skull` (class: `SneckoSkull`, file: `relics\SneckoSkull.java`)
- `Snecko Eye` (class: `SneckoEye`, file: `relics\SneckoEye.java`)
- `Sozu` (class: `Sozu`, file: `relics\Sozu.java`)
- `Spirit Poop` (class: `SpiritPoop`, file: `relics\SpiritPoop.java`)
- `SsserpentHead` (class: `SsserpentHead`, file: `relics\SsserpentHead.java`)
- `StoneCalendar` (class: `StoneCalendar`, file: `relics\StoneCalendar.java`)
- `Strange Spoon` (class: `StrangeSpoon`, file: `relics\StrangeSpoon.java`)
- `StrikeDummy` (class: `StrikeDummy`, file: `relics\StrikeDummy.java`)
- `Sundial` (class: `Sundial`, file: `relics\Sundial.java`)
- `Symbiotic Virus` (class: `SymbioticVirus`, file: `relics\SymbioticVirus.java`)
- `TeardropLocket` (class: `TeardropLocket`, file: `relics\TeardropLocket.java`)
- `Test 1` (class: `Test1`, file: `relics\Test1.java`)
- `Test 3` (class: `Test3`, file: `relics\Test3.java`)
- `Test 4` (class: `Test4`, file: `relics\Test4.java`)
- `Test 5` (class: `Test5`, file: `relics\Test5.java`)
- `Test 6` (class: `Test6`, file: `relics\Test6.java`)
- `The Courier` (class: `Courier`, file: `relics\Courier.java`)
- `The Specimen` (class: `TheSpecimen`, file: `relics\TheSpecimen.java`)
- `TheAbacus` (class: `Abacus`, file: `relics\Abacus.java`)
- `Thread and Needle` (class: `ThreadAndNeedle`, file: `relics\ThreadAndNeedle.java`)
- `Tingsha` (class: `Tingsha`, file: `relics\Tingsha.java`)
- `Tiny House` (class: `TinyHouse`, file: `relics\TinyHouse.java`)
- `Toolbox` (class: `Toolbox`, file: `relics\Toolbox.java`)
- `Tough Bandages` (class: `ToughBandages`, file: `relics\ToughBandages.java`)
- `TungstenRod` (class: `TungstenRod`, file: `relics\TungstenRod.java`)
- `Turnip` (class: `Turnip`, file: `relics\Turnip.java`)
- `TwistedFunnel` (class: `TwistedFunnel`, file: `relics\TwistedFunnel.java`)
- `Unceasing Top` (class: `UnceasingTop`, file: `relics\UnceasingTop.java`)
- `Velvet Choker` (class: `VelvetChoker`, file: `relics\VelvetChoker.java`)
- `VioletLotus` (class: `VioletLotus`, file: `relics\VioletLotus.java`)
- `War Paint` (class: `WarPaint`, file: `relics\WarPaint.java`)
- `WarpedTongs` (class: `WarpedTongs`, file: `relics\WarpedTongs.java`)
- `White Beast Statue` (class: `WhiteBeast`, file: `relics\WhiteBeast.java`)
- `WingedGreaves` (class: `WingBoots`, file: `relics\WingBoots.java`)
- `WristBlade` (class: `WristBlade`, file: `relics\WristBlade.java`)
- `Yang` (class: `Duality`, file: `relics\Duality.java`)
- `Yin` (class: `DEPRECATEDYin`, file: `relics\deprecated\DEPRECATEDYin.java`)

### POTION (1 missing)

- `Potion Slot` (class: `PotionSlot`, file: `potions\PotionSlot.java`)

### CARD (305 missing)

- `A Thousand Cuts` (class: `AThousandCuts`, file: `cards\green\AThousandCuts.java`)
- `Accuracy` (class: `Accuracy`, file: `cards\green\Accuracy.java`)
- `Acrobatics` (class: `Acrobatics`, file: `cards\green\Acrobatics.java`)
- `Adaptation` (class: `Rushdown`, file: `cards\purple\Rushdown.java`)
- `Adrenaline` (class: `Adrenaline`, file: `cards\green\Adrenaline.java`)
- `After Image` (class: `AfterImage`, file: `cards\green\AfterImage.java`)
- `Aggregate` (class: `Aggregate`, file: `cards\blue\Aggregate.java`)
- `All For One` (class: `AllForOne`, file: `cards\blue\AllForOne.java`)
- `All Out Attack` (class: `AllOutAttack`, file: `cards\green\AllOutAttack.java`)
- `Alpha` (class: `Alpha`, file: `cards\purple\Alpha.java`)
- `AlwaysMad` (class: `DEPRECATEDAlwaysMad`, file: `cards\deprecated\DEPRECATEDAlwaysMad.java`)
- `Amplify` (class: `Amplify`, file: `cards\blue\Amplify.java`)
- `AndCarryOn` (class: `DEPRECATEDAndCarryOn`, file: `cards\deprecated\DEPRECATEDAndCarryOn.java`)
- `Auto Shields` (class: `AutoShields`, file: `cards\blue\AutoShields.java`)
- `AwakenedStrike` (class: `DEPRECATEDAwakenedStrike`, file: `cards\deprecated\DEPRECATEDAwakenedStrike.java`)
- `Backflip` (class: `Backflip`, file: `cards\green\Backflip.java`)
- `Backstab` (class: `Backstab`, file: `cards\green\Backstab.java`)
- `Ball Lightning` (class: `BallLightning`, file: `cards\blue\BallLightning.java`)
- `Bane` (class: `Bane`, file: `cards\green\Bane.java`)
- `Barrage` (class: `Barrage`, file: `cards\blue\Barrage.java`)
- `BattleHymn` (class: `BattleHymn`, file: `cards\purple\BattleHymn.java`)
- `Beam Cell` (class: `BeamCell`, file: `cards\blue\BeamCell.java`)
- `BecomeAlmighty` (class: `BecomeAlmighty`, file: `cards\optionCards\BecomeAlmighty.java`)
- `Beta` (class: `Beta`, file: `cards\tempCards\Beta.java`)
- `Biased Cognition` (class: `BiasedCognition`, file: `cards\blue\BiasedCognition.java`)
- `BigBrain` (class: `DEPRECATEDBigBrain`, file: `cards\deprecated\DEPRECATEDBigBrain.java`)
- `Blade Dance` (class: `BladeDance`, file: `cards\green\BladeDance.java`)
- `Blasphemy` (class: `Blasphemy`, file: `cards\purple\Blasphemy.java`)
- `Blessed` (class: `DEPRECATEDBlessed`, file: `cards\deprecated\DEPRECATEDBlessed.java`)
- `Bliss` (class: `DEPRECATEDBliss`, file: `cards\deprecated\DEPRECATEDBliss.java`)
- `Blizzard` (class: `Blizzard`, file: `cards\blue\Blizzard.java`)
- `Blur` (class: `Blur`, file: `cards\green\Blur.java`)
- `BootSequence` (class: `BootSequence`, file: `cards\blue\BootSequence.java`)
- `Bouncing Flask` (class: `BouncingFlask`, file: `cards\green\BouncingFlask.java`)
- `BowlingBash` (class: `BowlingBash`, file: `cards\purple\BowlingBash.java`)
- `Brilliance` (class: `Brilliance`, file: `cards\purple\Brilliance.java`)
- `BrillianceAura` (class: `DEPRECATEDBrillianceAura`, file: `cards\deprecated\DEPRECATEDBrillianceAura.java`)
- `Buffer` (class: `Buffer`, file: `cards\blue\Buffer.java`)
- `Bullet Time` (class: `BulletTime`, file: `cards\green\BulletTime.java`)
- `Burst` (class: `Burst`, file: `cards\green\Burst.java`)
- `Calculated Gamble` (class: `CalculatedGamble`, file: `cards\green\CalculatedGamble.java`)
- `Calm` (class: `DEPRECATEDCalm`, file: `cards\deprecated\DEPRECATEDCalm.java`)
- `Calm` (class: `DEPRECATEDChooseCalm`, file: `cards\deprecated\DEPRECATEDChooseCalm.java`)
- `Calm` (class: `ChooseCalm`, file: `cards\optionCards\ChooseCalm.java`)
- `Caltrops` (class: `Caltrops`, file: `cards\green\Caltrops.java`)
- `Capacitor` (class: `Capacitor`, file: `cards\blue\Capacitor.java`)
- `CarveReality` (class: `CarveReality`, file: `cards\purple\CarveReality.java`)
- `Catalyst` (class: `Catalyst`, file: `cards\green\Catalyst.java`)
- `Causality` (class: `DEPRECATEDCausality`, file: `cards\deprecated\DEPRECATEDCausality.java`)
- `ChallengeAccepted` (class: `DEPRECATEDChallengeAccepted`, file: `cards\deprecated\DEPRECATEDChallengeAccepted.java`)
- `Chaos` (class: `Chaos`, file: `cards\blue\Chaos.java`)
- `Chill` (class: `Chill`, file: `cards\blue\Chill.java`)
- `Choke` (class: `Choke`, file: `cards\green\Choke.java`)
- `Clarity` (class: `DEPRECATEDClarity`, file: `cards\deprecated\DEPRECATEDClarity.java`)
- `CleanseEvil` (class: `DEPRECATEDCleanseEvil`, file: `cards\deprecated\DEPRECATEDCleanseEvil.java`)
- `ClearTheMind` (class: `Tranquility`, file: `cards\purple\Tranquility.java`)
- `Cloak And Dagger` (class: `CloakAndDagger`, file: `cards\green\CloakAndDagger.java`)
- `Cold Snap` (class: `ColdSnap`, file: `cards\blue\ColdSnap.java`)
- `Collect` (class: `Collect`, file: `cards\purple\Collect.java`)
- `Compile Driver` (class: `CompileDriver`, file: `cards\blue\CompileDriver.java`)
- `Concentrate` (class: `Concentrate`, file: `cards\green\Concentrate.java`)
- `Conclude` (class: `Conclude`, file: `cards\purple\Conclude.java`)
- `Condense` (class: `DEPRECATEDCondense`, file: `cards\deprecated\DEPRECATEDCondense.java`)
- `Confront` (class: `DEPRECATEDConfront`, file: `cards\deprecated\DEPRECATEDConfront.java`)
- `ConjureBlade` (class: `ConjureBlade`, file: `cards\purple\ConjureBlade.java`)
- `Consecrate` (class: `Consecrate`, file: `cards\purple\Consecrate.java`)
- `Conserve Battery` (class: `ConserveBattery`, file: `cards\blue\ConserveBattery.java`)
- `Consume` (class: `Consume`, file: `cards\blue\Consume.java`)
- `Contemplate` (class: `DEPRECATEDContemplate`, file: `cards\deprecated\DEPRECATEDContemplate.java`)
- `Coolheaded` (class: `Coolheaded`, file: `cards\blue\Coolheaded.java`)
- `Core Surge` (class: `CoreSurge`, file: `cards\blue\CoreSurge.java`)
- `Corpse Explosion` (class: `CorpseExplosion`, file: `cards\green\CorpseExplosion.java`)
- `Creative AI` (class: `CreativeAI`, file: `cards\blue\CreativeAI.java`)
- `Crescendo` (class: `Crescendo`, file: `cards\purple\Crescendo.java`)
- `CrescentKick` (class: `DEPRECATEDCrescentKick`, file: `cards\deprecated\DEPRECATEDCrescentKick.java`)
- `Crippling Poison` (class: `CripplingPoison`, file: `cards\green\CripplingPoison.java`)
- `CrushJoints` (class: `CrushJoints`, file: `cards\purple\CrushJoints.java`)
- `CutThroughFate` (class: `CutThroughFate`, file: `cards\purple\CutThroughFate.java`)
- `DEPRECATEDBalancedViolence` (class: `DEPRECATEDBalancedViolence`, file: `cards\deprecated\DEPRECATEDBalancedViolence.java`)
- `DEPRECATEDFlicker` (class: `DEPRECATEDFlicker`, file: `cards\deprecated\DEPRECATEDFlicker.java`)
- `Dagger Spray` (class: `DaggerSpray`, file: `cards\green\DaggerSpray.java`)
- `Dagger Throw` (class: `DaggerThrow`, file: `cards\green\DaggerThrow.java`)
- `Darkness` (class: `Darkness`, file: `cards\blue\Darkness.java`)
- `Dash` (class: `Dash`, file: `cards\green\Dash.java`)
- `Deadly Poison` (class: `DeadlyPoison`, file: `cards\green\DeadlyPoison.java`)
- `DeceiveReality` (class: `DeceiveReality`, file: `cards\purple\DeceiveReality.java`)
- `Defend_B` (class: `Defend_Blue`, file: `cards\blue\Defend_Blue.java`)
- `Defend_G` (class: `Defend_Green`, file: `cards\green\Defend_Green.java`)
- `Defend_P` (class: `Defend_Watcher`, file: `cards\purple\Defend_Watcher.java`)
- `Deflect` (class: `Deflect`, file: `cards\green\Deflect.java`)
- `Defragment` (class: `Defragment`, file: `cards\blue\Defragment.java`)
- `DeusExMachina` (class: `DeusExMachina`, file: `cards\purple\DeusExMachina.java`)
- `DevaForm` (class: `DevaForm`, file: `cards\purple\DevaForm.java`)
- `Devotion` (class: `Devotion`, file: `cards\purple\Devotion.java`)
- `Die Die Die` (class: `DieDieDie`, file: `cards\green\DieDieDie.java`)
- `Discipline` (class: `Discipline`, file: `cards\purple\Discipline.java`)
- `Distraction` (class: `Distraction`, file: `cards\green\Distraction.java`)
- `Dodge and Roll` (class: `DodgeAndRoll`, file: `cards\green\DodgeAndRoll.java`)
- `Doom and Gloom` (class: `DoomAndGloom`, file: `cards\blue\DoomAndGloom.java`)
- `Doppelganger` (class: `Doppelganger`, file: `cards\green\Doppelganger.java`)
- `Double Energy` (class: `DoubleEnergy`, file: `cards\blue\DoubleEnergy.java`)
- `Dualcast` (class: `Dualcast`, file: `cards\blue\Dualcast.java`)
- `Echo Form` (class: `EchoForm`, file: `cards\blue\EchoForm.java`)
- `Electrodynamics` (class: `Electrodynamics`, file: `cards\blue\Electrodynamics.java`)
- `EmptyBody` (class: `EmptyBody`, file: `cards\purple\EmptyBody.java`)
- `EmptyFist` (class: `EmptyFist`, file: `cards\purple\EmptyFist.java`)
- `EmptyMind` (class: `EmptyMind`, file: `cards\purple\EmptyMind.java`)
- `Endless Agony` (class: `EndlessAgony`, file: `cards\green\EndlessAgony.java`)
- `Envenom` (class: `Envenom`, file: `cards\green\Envenom.java`)
- `Eruption` (class: `DEPRECATEDEruption`, file: `cards\deprecated\DEPRECATEDEruption.java`)
- `Eruption` (class: `Eruption`, file: `cards\purple\Eruption.java`)
- `Escape Plan` (class: `EscapePlan`, file: `cards\green\EscapePlan.java`)
- `Establishment` (class: `Establishment`, file: `cards\purple\Establishment.java`)
- `Evaluate` (class: `Evaluate`, file: `cards\purple\Evaluate.java`)
- `Eviscerate` (class: `Eviscerate`, file: `cards\green\Eviscerate.java`)
- `Experienced` (class: `DEPRECATEDExperienced`, file: `cards\deprecated\DEPRECATEDExperienced.java`)
- `Expertise` (class: `Expertise`, file: `cards\green\Expertise.java`)
- `Expunger` (class: `Expunger`, file: `cards\tempCards\Expunger.java`)
- `FTL` (class: `FTL`, file: `cards\blue\FTL.java`)
- `FameAndFortune` (class: `FameAndFortune`, file: `cards\optionCards\FameAndFortune.java`)
- `Fasting2` (class: `Fasting`, file: `cards\purple\Fasting.java`)
- `FearNoEvil` (class: `FearNoEvil`, file: `cards\purple\FearNoEvil.java`)
- `Finisher` (class: `Finisher`, file: `cards\green\Finisher.java`)
- `Fission` (class: `Fission`, file: `cards\blue\Fission.java`)
- `FlameMastery` (class: `DEPRECATEDFlameMastery`, file: `cards\deprecated\DEPRECATEDFlameMastery.java`)
- `Flare` (class: `DEPRECATEDFlare`, file: `cards\deprecated\DEPRECATEDFlare.java`)
- `Flechettes` (class: `Flechettes`, file: `cards\green\Flechettes.java`)
- `Flick` (class: `DEPRECATEDFlick`, file: `cards\deprecated\DEPRECATEDFlick.java`)
- `Flow` (class: `DEPRECATEDFlow`, file: `cards\deprecated\DEPRECATEDFlow.java`)
- `FlowState` (class: `DEPRECATEDFlowState`, file: `cards\deprecated\DEPRECATEDFlowState.java`)
- `FlurryOfBlows` (class: `FlurryOfBlows`, file: `cards\purple\FlurryOfBlows.java`)
- `Flying Knee` (class: `FlyingKnee`, file: `cards\green\FlyingKnee.java`)
- `FlyingSleeves` (class: `FlyingSleeves`, file: `cards\purple\FlyingSleeves.java`)
- `FollowUp` (class: `FollowUp`, file: `cards\purple\FollowUp.java`)
- `Footwork` (class: `Footwork`, file: `cards\green\Footwork.java`)
- `Force Field` (class: `ForceField`, file: `cards\blue\ForceField.java`)
- `ForeignInfluence` (class: `ForeignInfluence`, file: `cards\purple\ForeignInfluence.java`)
- `Fury` (class: `DEPRECATEDFury`, file: `cards\deprecated\DEPRECATEDFury.java`)
- `FuryAura` (class: `DEPRECATEDFuryAura`, file: `cards\deprecated\DEPRECATEDFuryAura.java`)
- `Fusion` (class: `Fusion`, file: `cards\blue\Fusion.java`)
- `Gash` (class: `Claw`, file: `cards\blue\Claw.java`)
- `Genetic Algorithm` (class: `GeneticAlgorithm`, file: `cards\blue\GeneticAlgorithm.java`)
- `Glacier` (class: `Glacier`, file: `cards\blue\Glacier.java`)
- `Glass Knife` (class: `GlassKnife`, file: `cards\green\GlassKnife.java`)
- `Go for the Eyes` (class: `GoForTheEyes`, file: `cards\blue\GoForTheEyes.java`)
- `Grand Finale` (class: `GrandFinale`, file: `cards\green\GrandFinale.java`)
- `Grounded` (class: `DEPRECATEDGrounded`, file: `cards\deprecated\DEPRECATEDGrounded.java`)
- `Halt` (class: `Halt`, file: `cards\purple\Halt.java`)
- `Heatsinks` (class: `Heatsinks`, file: `cards\blue\Heatsinks.java`)
- `Heel Hook` (class: `HeelHook`, file: `cards\green\HeelHook.java`)
- `Hello World` (class: `HelloWorld`, file: `cards\blue\HelloWorld.java`)
- `Hologram` (class: `Hologram`, file: `cards\blue\Hologram.java`)
- `HotHot` (class: `DEPRECATEDHotHot`, file: `cards\deprecated\DEPRECATEDHotHot.java`)
- `Hyperbeam` (class: `Hyperbeam`, file: `cards\blue\Hyperbeam.java`)
- `Impulse` (class: `Impulse`, file: `cards\blue\Impulse.java`)
- `Indignation` (class: `Indignation`, file: `cards\purple\Indignation.java`)
- `Infinite Blades` (class: `InfiniteBlades`, file: `cards\green\InfiniteBlades.java`)
- `InnerPeace` (class: `InnerPeace`, file: `cards\purple\InnerPeace.java`)
- `Insight` (class: `Insight`, file: `cards\tempCards\Insight.java`)
- `Introspection` (class: `DEPRECATEDIntrospection`, file: `cards\deprecated\DEPRECATEDIntrospection.java`)
- `Joy` (class: `DEPRECATEDChooseCourage`, file: `cards\deprecated\DEPRECATEDChooseCourage.java`)
- `Judgement` (class: `Judgement`, file: `cards\purple\Judgement.java`)
- `JustLucky` (class: `JustLucky`, file: `cards\purple\JustLucky.java`)
- `Leap` (class: `Leap`, file: `cards\blue\Leap.java`)
- `Leg Sweep` (class: `LegSweep`, file: `cards\green\LegSweep.java`)
- `LessonLearned` (class: `LessonLearned`, file: `cards\purple\LessonLearned.java`)
- `LetFateDecide` (class: `DEPRECATEDLetFateDecide`, file: `cards\deprecated\DEPRECATEDLetFateDecide.java`)
- `LikeWater` (class: `LikeWater`, file: `cards\purple\LikeWater.java`)
- `LiveForever` (class: `LiveForever`, file: `cards\optionCards\LiveForever.java`)
- `Lockon` (class: `LockOn`, file: `cards\blue\LockOn.java`)
- `Loop` (class: `Loop`, file: `cards\blue\Loop.java`)
- `Machine Learning` (class: `MachineLearning`, file: `cards\blue\MachineLearning.java`)
- `Malaise` (class: `Malaise`, file: `cards\green\Malaise.java`)
- `MasterReality` (class: `DEPRECATEDMasterReality`, file: `cards\deprecated\DEPRECATEDMasterReality.java`)
- `MasterReality` (class: `MasterReality`, file: `cards\purple\MasterReality.java`)
- `Masterful Stab` (class: `MasterfulStab`, file: `cards\green\MasterfulStab.java`)
- `Mastery` (class: `DEPRECATEDMastery`, file: `cards\deprecated\DEPRECATEDMastery.java`)
- `Meditate` (class: `Meditate`, file: `cards\purple\Meditate.java`)
- `Melter` (class: `Melter`, file: `cards\blue\Melter.java`)
- `MentalFortress` (class: `MentalFortress`, file: `cards\purple\MentalFortress.java`)
- `Metaphysics` (class: `DEPRECATEDMetaphysics`, file: `cards\deprecated\DEPRECATEDMetaphysics.java`)
- `Meteor Strike` (class: `MeteorStrike`, file: `cards\blue\MeteorStrike.java`)
- `Miracle` (class: `Miracle`, file: `cards\tempCards\Miracle.java`)
- `Multi-Cast` (class: `MultiCast`, file: `cards\blue\MultiCast.java`)
- `Neutralize` (class: `Neutralize`, file: `cards\green\Neutralize.java`)
- `Night Terror` (class: `Nightmare`, file: `cards\green\Nightmare.java`)
- `Nirvana` (class: `Nirvana`, file: `cards\purple\Nirvana.java`)
- `Nothingness` (class: `DEPRECATEDNothingness`, file: `cards\deprecated\DEPRECATEDNothingness.java`)
- `Noxious Fumes` (class: `NoxiousFumes`, file: `cards\green\NoxiousFumes.java`)
- `Omega` (class: `Omega`, file: `cards\tempCards\Omega.java`)
- `Omniscience` (class: `Omniscience`, file: `cards\purple\Omniscience.java`)
- `Outmaneuver` (class: `Outmaneuver`, file: `cards\green\Outmaneuver.java`)
- `PalmThatRestrains` (class: `DEPRECATEDRestrainingPalm`, file: `cards\deprecated\DEPRECATEDRestrainingPalm.java`)
- `PathToVictory` (class: `DEPRECATEDPathToVictory`, file: `cards\deprecated\DEPRECATEDPathToVictory.java`)
- `PathToVictory` (class: `PressurePoints`, file: `cards\purple\PressurePoints.java`)
- `Peace` (class: `DEPRECATEDPeace`, file: `cards\deprecated\DEPRECATEDPeace.java`)
- `PerfectedForm` (class: `DEPRECATEDPerfectedForm`, file: `cards\deprecated\DEPRECATEDPerfectedForm.java`)
- `Perseverance` (class: `Perseverance`, file: `cards\purple\Perseverance.java`)
- `Phantasmal Killer` (class: `PhantasmalKiller`, file: `cards\green\PhantasmalKiller.java`)
- `PiercingWail` (class: `PiercingWail`, file: `cards\green\PiercingWail.java`)
- `Poisoned Stab` (class: `PoisonedStab`, file: `cards\green\PoisonedStab.java`)
- `Polymath` (class: `DEPRECATEDPolymath`, file: `cards\deprecated\DEPRECATEDPolymath.java`)
- `Pray` (class: `Pray`, file: `cards\purple\Pray.java`)
- `Predator` (class: `Predator`, file: `cards\green\Predator.java`)
- `Prediction` (class: `DEPRECATEDPrediction`, file: `cards\deprecated\DEPRECATEDPrediction.java`)
- `Prepared` (class: `Prepared`, file: `cards\green\Prepared.java`)
- `Prostrate` (class: `Prostrate`, file: `cards\purple\Prostrate.java`)
- `Protect` (class: `Protect`, file: `cards\purple\Protect.java`)
- `Punishment` (class: `DEPRECATEDPunishment`, file: `cards\deprecated\DEPRECATEDPunishment.java`)
- `Quick Slash` (class: `QuickSlash`, file: `cards\green\QuickSlash.java`)
- `Ragnarok` (class: `Ragnarok`, file: `cards\purple\Ragnarok.java`)
- `Rainbow` (class: `Rainbow`, file: `cards\blue\Rainbow.java`)
- `ReachHeaven` (class: `ReachHeaven`, file: `cards\purple\ReachHeaven.java`)
- `Reboot` (class: `Reboot`, file: `cards\blue\Reboot.java`)
- `Rebound` (class: `Rebound`, file: `cards\blue\Rebound.java`)
- `Recycle` (class: `Recycle`, file: `cards\blue\Recycle.java`)
- `Redo` (class: `Recursion`, file: `cards\blue\Recursion.java`)
- `Reflex` (class: `Reflex`, file: `cards\green\Reflex.java`)
- `Reinforced Body` (class: `ReinforcedBody`, file: `cards\blue\ReinforcedBody.java`)
- `Reprogram` (class: `Reprogram`, file: `cards\blue\Reprogram.java`)
- `RetreatingHand` (class: `DEPRECATEDRetreatingHand`, file: `cards\deprecated\DEPRECATEDRetreatingHand.java`)
- `Retribution` (class: `DEPRECATEDRetribution`, file: `cards\deprecated\DEPRECATEDRetribution.java`)
- `Riddle With Holes` (class: `RiddleWithHoles`, file: `cards\green\RiddleWithHoles.java`)
- `Rip and Tear` (class: `RipAndTear`, file: `cards\blue\RipAndTear.java`)
- `Safety` (class: `Safety`, file: `cards\tempCards\Safety.java`)
- `Sanctity` (class: `Sanctity`, file: `cards\purple\Sanctity.java`)
- `SandsOfTime` (class: `SandsOfTime`, file: `cards\purple\SandsOfTime.java`)
- `SashWhip` (class: `SashWhip`, file: `cards\purple\SashWhip.java`)
- `Scrape` (class: `Scrape`, file: `cards\blue\Scrape.java`)
- `Scrawl` (class: `Scrawl`, file: `cards\purple\Scrawl.java`)
- `Seek` (class: `Seek`, file: `cards\blue\Seek.java`)
- `Self Repair` (class: `SelfRepair`, file: `cards\blue\SelfRepair.java`)
- `Serenity` (class: `DEPRECATEDSerenity`, file: `cards\deprecated\DEPRECATEDSerenity.java`)
- `Setup` (class: `Setup`, file: `cards\green\Setup.java`)
- `Shiv` (class: `Shiv`, file: `cards\tempCards\Shiv.java`)
- `SignatureMove` (class: `SignatureMove`, file: `cards\purple\SignatureMove.java`)
- `SimmeringRage` (class: `DEPRECATEDSimmeringRage`, file: `cards\deprecated\DEPRECATEDSimmeringRage.java`)
- `Skewer` (class: `Skewer`, file: `cards\green\Skewer.java`)
- `Skim` (class: `Skim`, file: `cards\blue\Skim.java`)
- `Slice` (class: `Slice`, file: `cards\green\Slice.java`)
- `Smile` (class: `DEPRECATEDSmile`, file: `cards\deprecated\DEPRECATEDSmile.java`)
- `Smite` (class: `Smite`, file: `cards\tempCards\Smite.java`)
- `SoothingAura` (class: `DEPRECATEDSoothingAura`, file: `cards\deprecated\DEPRECATEDSoothingAura.java`)
- `SpiritShield` (class: `SpiritShield`, file: `cards\purple\SpiritShield.java`)
- `Stack` (class: `Stack`, file: `cards\blue\Stack.java`)
- `Static Discharge` (class: `StaticDischarge`, file: `cards\blue\StaticDischarge.java`)
- `Steam` (class: `SteamBarrier`, file: `cards\blue\SteamBarrier.java`)
- `Steam Power` (class: `Overclock`, file: `cards\blue\Overclock.java`)
- `StepAndStrike` (class: `DEPRECATEDStepAndStrike`, file: `cards\deprecated\DEPRECATEDStepAndStrike.java`)
- `Stomp` (class: `DEPRECATEDStomp`, file: `cards\deprecated\DEPRECATEDStomp.java`)
- `Storm` (class: `Storm`, file: `cards\blue\Storm.java`)
- `Storm of Steel` (class: `StormOfSteel`, file: `cards\green\StormOfSteel.java`)
- `Streamline` (class: `Streamline`, file: `cards\blue\Streamline.java`)
- `Strike_B` (class: `Strike_Blue`, file: `cards\blue\Strike_Blue.java`)
- `Strike_G` (class: `Strike_Green`, file: `cards\green\Strike_Green.java`)
- `Strike_P` (class: `Strike_Purple`, file: `cards\purple\Strike_Purple.java`)
- `Study` (class: `Study`, file: `cards\purple\Study.java`)
- `SublimeSlice` (class: `DEPRECATEDSublimeSlice`, file: `cards\deprecated\DEPRECATEDSublimeSlice.java`)
- `Sucker Punch` (class: `SuckerPunch`, file: `cards\green\SuckerPunch.java`)
- `Sunder` (class: `Sunder`, file: `cards\blue\Sunder.java`)
- `Survey` (class: `DEPRECATEDSurvey`, file: `cards\deprecated\DEPRECATEDSurvey.java`)
- `Survivor` (class: `Survivor`, file: `cards\green\Survivor.java`)
- `Sweeping Beam` (class: `SweepingBeam`, file: `cards\blue\SweepingBeam.java`)
- `Swipe` (class: `DEPRECATEDSwipe`, file: `cards\deprecated\DEPRECATEDSwipe.java`)
- `Swivel` (class: `Swivel`, file: `cards\purple\Swivel.java`)
- `Tactician` (class: `Tactician`, file: `cards\green\Tactician.java`)
- `TalkToTheHand` (class: `TalkToTheHand`, file: `cards\purple\TalkToTheHand.java`)
- `Tantrum` (class: `Tantrum`, file: `cards\purple\Tantrum.java`)
- `TemperTantrum` (class: `DEPRECATEDTemperTantrum`, file: `cards\deprecated\DEPRECATEDTemperTantrum.java`)
- `Tempest` (class: `Tempest`, file: `cards\blue\Tempest.java`)
- `Terror` (class: `Terror`, file: `cards\green\Terror.java`)
- `ThirdEye` (class: `ThirdEye`, file: `cards\purple\ThirdEye.java`)
- `ThroughViolence` (class: `ThroughViolence`, file: `cards\tempCards\ThroughViolence.java`)
- `Thunder Strike` (class: `ThunderStrike`, file: `cards\blue\ThunderStrike.java`)
- `Tools of the Trade` (class: `ToolsOfTheTrade`, file: `cards\green\ToolsOfTheTrade.java`)
- `Torrent` (class: `DEPRECATEDTorrent`, file: `cards\deprecated\DEPRECATEDTorrent.java`)
- `Transcendence` (class: `DEPRECATEDTranscendence`, file: `cards\deprecated\DEPRECATEDTranscendence.java`)
- `Truth` (class: `DEPRECATEDTruth`, file: `cards\deprecated\DEPRECATEDTruth.java`)
- `Turbo` (class: `Turbo`, file: `cards\blue\Turbo.java`)
- `Underhanded Strike` (class: `SneakyStrike`, file: `cards\green\SneakyStrike.java`)
- `Undo` (class: `Equilibrium`, file: `cards\blue\Equilibrium.java`)
- `Unload` (class: `Unload`, file: `cards\green\Unload.java`)
- `Unraveling` (class: `Unraveling`, file: `cards\purple\Unraveling.java`)
- `Vault` (class: `Vault`, file: `cards\purple\Vault.java`)
- `Vengeance` (class: `SimmeringFury`, file: `cards\purple\SimmeringFury.java`)
- `Venomology` (class: `Alchemize`, file: `cards\green\Alchemize.java`)
- `Vigilance` (class: `Vigilance`, file: `cards\purple\Vigilance.java`)
- `Wallop` (class: `Wallop`, file: `cards\purple\Wallop.java`)
- `WardAura` (class: `DEPRECATEDWardAura`, file: `cards\deprecated\DEPRECATEDWardAura.java`)
- `WaveOfTheHand` (class: `WaveOfTheHand`, file: `cards\purple\WaveOfTheHand.java`)
- `Weave` (class: `Weave`, file: `cards\purple\Weave.java`)
- `Well Laid Plans` (class: `WellLaidPlans`, file: `cards\green\WellLaidPlans.java`)
- `WheelKick` (class: `WheelKick`, file: `cards\purple\WheelKick.java`)
- `White Noise` (class: `WhiteNoise`, file: `cards\blue\WhiteNoise.java`)
- `WindmillStrike` (class: `WindmillStrike`, file: `cards\purple\WindmillStrike.java`)
- `Windup` (class: `DEPRECATEDWindup`, file: `cards\deprecated\DEPRECATEDWindup.java`)
- `Wireheading` (class: `Foresight`, file: `cards\purple\Foresight.java`)
- `Wisdom` (class: `DEPRECATEDWisdom`, file: `cards\deprecated\DEPRECATEDWisdom.java`)
- `Wish` (class: `Wish`, file: `cards\purple\Wish.java`)
- `Worship` (class: `Worship`, file: `cards\purple\Worship.java`)
- `Wraith Form v2` (class: `WraithForm`, file: `cards\green\WraithForm.java`)
- `Wrath` (class: `DEPRECATEDWrath`, file: `cards\deprecated\DEPRECATEDWrath.java`)
- `Wrath` (class: `ChooseWrath`, file: `cards\optionCards\ChooseWrath.java`)
- `WreathOfFlame` (class: `WreathOfFlame`, file: `cards\purple\WreathOfFlame.java`)
- `Zap` (class: `Zap`, file: `cards\blue\Zap.java`)

### MONSTER (66 missing)

- `AcidSlime_L` (class: `AcidSlime_L`, file: `monsters\exordium\AcidSlime_L.java`)
- `AcidSlime_M` (class: `AcidSlime_M`, file: `monsters\exordium\AcidSlime_M.java`)
- `AcidSlime_S` (class: `AcidSlime_S`, file: `monsters\exordium\AcidSlime_S.java`)
- `Apology Slime` (class: `ApologySlime`, file: `monsters\exordium\ApologySlime.java`)
- `AwakenedOne` (class: `AwakenedOne`, file: `monsters\beyond\AwakenedOne.java`)
- `BanditBear` (class: `BanditBear`, file: `monsters\city\BanditBear.java`)
- `BanditChild` (class: `BanditPointy`, file: `monsters\city\BanditPointy.java`)
- `BanditLeader` (class: `BanditLeader`, file: `monsters\city\BanditLeader.java`)
- `BookOfStabbing` (class: `BookOfStabbing`, file: `monsters\city\BookOfStabbing.java`)
- `BronzeAutomaton` (class: `BronzeAutomaton`, file: `monsters\city\BronzeAutomaton.java`)
- `BronzeOrb` (class: `BronzeOrb`, file: `monsters\city\BronzeOrb.java`)
- `Byrd` (class: `Byrd`, file: `monsters\city\Byrd.java`)
- `Centurion` (class: `Centurion`, file: `monsters\city\Centurion.java`)
- `Champ` (class: `Champ`, file: `monsters\city\Champ.java`)
- `Chosen` (class: `Chosen`, file: `monsters\city\Chosen.java`)
- `CorruptHeart` (class: `CorruptHeart`, file: `monsters\ending\CorruptHeart.java`)
- `Cultist` (class: `Cultist`, file: `monsters\exordium\Cultist.java`)
- `Dagger` (class: `SnakeDagger`, file: `monsters\beyond\SnakeDagger.java`)
- `Darkling` (class: `Darkling`, file: `monsters\beyond\Darkling.java`)
- `Deca` (class: `Deca`, file: `monsters\beyond\Deca.java`)
- `Donu` (class: `Donu`, file: `monsters\beyond\Donu.java`)
- `Exploder` (class: `Exploder`, file: `monsters\beyond\Exploder.java`)
- `FungiBeast` (class: `FungiBeast`, file: `monsters\exordium\FungiBeast.java`)
- `FuzzyLouseDefensive` (class: `LouseDefensive`, file: `monsters\exordium\LouseDefensive.java`)
- `FuzzyLouseNormal` (class: `LouseNormal`, file: `monsters\exordium\LouseNormal.java`)
- `GiantHead` (class: `GiantHead`, file: `monsters\beyond\GiantHead.java`)
- `GremlinFat` (class: `GremlinFat`, file: `monsters\exordium\GremlinFat.java`)
- `GremlinLeader` (class: `GremlinLeader`, file: `monsters\city\GremlinLeader.java`)
- `GremlinNob` (class: `GremlinNob`, file: `monsters\exordium\GremlinNob.java`)
- `GremlinThief` (class: `GremlinThief`, file: `monsters\exordium\GremlinThief.java`)
- `GremlinTsundere` (class: `GremlinTsundere`, file: `monsters\exordium\GremlinTsundere.java`)
- `GremlinWarrior` (class: `GremlinWarrior`, file: `monsters\exordium\GremlinWarrior.java`)
- `GremlinWizard` (class: `GremlinWizard`, file: `monsters\exordium\GremlinWizard.java`)
- `Healer` (class: `Healer`, file: `monsters\city\Healer.java`)
- `Hexaghost` (class: `Hexaghost`, file: `monsters\exordium\Hexaghost.java`)
- `JawWorm` (class: `JawWorm`, file: `monsters\exordium\JawWorm.java`)
- `Lagavulin` (class: `Lagavulin`, file: `monsters\exordium\Lagavulin.java`)
- `Looter` (class: `Looter`, file: `monsters\exordium\Looter.java`)
- `Maw` (class: `Maw`, file: `monsters\beyond\Maw.java`)
- `Mugger` (class: `Mugger`, file: `monsters\city\Mugger.java`)
- `Nemesis` (class: `Nemesis`, file: `monsters\beyond\Nemesis.java`)
- `Orb Walker` (class: `OrbWalker`, file: `monsters\beyond\OrbWalker.java`)
- `Reptomancer` (class: `Reptomancer`, file: `monsters\beyond\Reptomancer.java`)
- `Repulsor` (class: `Repulsor`, file: `monsters\beyond\Repulsor.java`)
- `Sentry` (class: `Sentry`, file: `monsters\exordium\Sentry.java`)
- `Serpent` (class: `SpireGrowth`, file: `monsters\beyond\SpireGrowth.java`)
- `Shelled Parasite` (class: `ShelledParasite`, file: `monsters\city\ShelledParasite.java`)
- `SlaverBlue` (class: `SlaverBlue`, file: `monsters\exordium\SlaverBlue.java`)
- `SlaverBoss` (class: `Taskmaster`, file: `monsters\city\Taskmaster.java`)
- `SlaverRed` (class: `SlaverRed`, file: `monsters\exordium\SlaverRed.java`)
- `SlimeBoss` (class: `SlimeBoss`, file: `monsters\exordium\SlimeBoss.java`)
- `SnakePlant` (class: `SnakePlant`, file: `monsters\city\SnakePlant.java`)
- `Snecko` (class: `Snecko`, file: `monsters\city\Snecko.java`)
- `SphericGuardian` (class: `SphericGuardian`, file: `monsters\city\SphericGuardian.java`)
- `SpikeSlime_L` (class: `SpikeSlime_L`, file: `monsters\exordium\SpikeSlime_L.java`)
- `SpikeSlime_M` (class: `SpikeSlime_M`, file: `monsters\exordium\SpikeSlime_M.java`)
- `SpikeSlime_S` (class: `SpikeSlime_S`, file: `monsters\exordium\SpikeSlime_S.java`)
- `Spiker` (class: `Spiker`, file: `monsters\beyond\Spiker.java`)
- `SpireShield` (class: `SpireShield`, file: `monsters\ending\SpireShield.java`)
- `SpireSpear` (class: `SpireSpear`, file: `monsters\ending\SpireSpear.java`)
- `TheCollector` (class: `TheCollector`, file: `monsters\city\TheCollector.java`)
- `TheGuardian` (class: `TheGuardian`, file: `monsters\exordium\TheGuardian.java`)
- `TimeEater` (class: `TimeEater`, file: `monsters\beyond\TimeEater.java`)
- `TorchHead` (class: `TorchHead`, file: `monsters\city\TorchHead.java`)
- `Transient` (class: `Transient`, file: `monsters\beyond\Transient.java`)
- `WrithingMass` (class: `WrithingMass`, file: `monsters\beyond\WrithingMass.java`)

