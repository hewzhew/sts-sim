# StS Implementation Completeness Checklist

Each relic/power implementation must cover two parts: own-class logic + engine-side logic.
Use this checklist to track progress and avoid missing scattered engine hooks.

## Relics

| Relic | Own Class Hooks | Engine-side Checks | Self Done | Engine Done |
|---|---|---|---|---|
| Abacus | onShuffle, makeCopy | 0 | [ ] | [ ] |
| AbstractRelic | updateDescription, onEvokeOrb, onPlayCard, onObtainCard, onEquip, onUnequip, atPreBattle, atBattleStart, onSpawnMonster, atBattleStartPreDraw, onPlayerEndTurn, onManualDiscard, onUseCard, onVictory, onMonsterDeath, onBlockBroken, onPlayerGainBlock, onPlayerGainedBlock, onPlayerHeal, onEnterRestRoom, onShuffle, onSmith, onAttack, onAttacked, onAttackedToChangeDamage, onExhaust, onTrigger, onTrigger, onEnterRoom, justEnteredRoom, onCardDraw, onChestOpen, onDrawOrDiscard, onMasterDeckChange, makeCopy, onChangeStance, onLoseHp, wasHPLost | 0 | [ ] | [ ] |
| Akabeko | atBattleStart, makeCopy | 1 refs | [ ] | [ ] |
| Anchor | atBattleStart, justEnteredRoom, makeCopy | 0 | [ ] | [ ] |
| AncientTeaSet | updateDescription, atPreBattle, onEnterRestRoom, makeCopy | 0 | [ ] | [ ] |
| ArtOfWar | updateDescription, atPreBattle, onUseCard, onVictory, makeCopy | 1 refs | [ ] | [ ] |
| Astrolabe | onEquip, makeCopy | 0 | [ ] | [ ] |
| BagOfMarbles | atBattleStart, makeCopy | 0 | [ ] | [ ] |
| BagOfPreparation | atBattleStart, makeCopy | 0 | [ ] | [ ] |
| BirdFacedUrn | onUseCard, makeCopy | 0 | [ ] | [ ] |
| BlackBlood | onVictory, makeCopy | 0 | [ ] | [ ] |
| BlackStar | onEnterRoom, onVictory, makeCopy | 1 refs | [ ] | [ ] |
| BloodVial | atBattleStart, makeCopy | 1 refs | [ ] | [ ] |
| BloodyIdol | makeCopy | 2 refs | [ ] | [ ] |
| BlueCandle | makeCopy, onUseCard | 2 refs | [ ] | [ ] |
| Boot | makeCopy | 0 | [ ] | [ ] |
| BottledFlame | onEquip, onUnequip, atBattleStart, makeCopy | 11 refs | [ ] | [ ] |
| BottledLightning | onEquip, onUnequip, atBattleStart, makeCopy | 11 refs | [ ] | [ ] |
| BottledTornado | onEquip, onUnequip, atBattleStart, makeCopy | 11 refs | [ ] | [ ] |
| Brimstone | makeCopy | 0 | [ ] | [ ] |
| BronzeScales | atBattleStart, makeCopy | 0 | [ ] | [ ] |
| BurningBlood | onVictory, makeCopy | 1 refs | [ ] | [ ] |
| BustedCrown | updateDescription, onEquip, onUnequip, makeCopy | 2 refs | [ ] | [ ] |
| Calipers | makeCopy | 1 refs | [ ] | [ ] |
| CallingBell | onEquip, makeCopy | 0 | [ ] | [ ] |
| CaptainsWheel | atBattleStart, onVictory, makeCopy | 0 | [ ] | [ ] |
| Cauldron | onEquip, makeCopy | 0 | [ ] | [ ] |
| CentennialPuzzle | atPreBattle, wasHPLost, justEnteredRoom, onVictory, makeCopy | 0 | [ ] | [ ] |
| CeramicFish | use, onObtainCard, makeCopy | 1 refs | [ ] | [ ] |
| ChampionsBelt | onTrigger, makeCopy | 0 | [ ] | [ ] |
| CharonsAshes | onExhaust, makeCopy | 0 | [ ] | [ ] |
| ChemicalX | makeCopy | 24 refs | [ ] | [ ] |
| Circlet | onEquip, onUnequip, makeCopy | 11 refs | [ ] | [ ] |
| CloakClasp | onPlayerEndTurn, makeCopy | 1 refs | [ ] | [ ] |
| ClockworkSouvenir | atBattleStart, makeCopy | 0 | [ ] | [ ] |
| CoffeeDripper | updateDescription, onEquip, onUnequip, makeCopy | 0 | [ ] | [ ] |
| Courier | onEnterRoom, makeCopy | 0 | [ ] | [ ] |
| CrackedCore | atPreBattle, makeCopy | 1 refs | [ ] | [ ] |
| CultistMask | atBattleStart, makeCopy | 1 refs | [ ] | [ ] |
| CursedKey | justEnteredRoom, onChestOpen, updateDescription, onEquip, onUnequip, makeCopy | 0 | [ ] | [ ] |
| DEPRECATEDDodecahedron | updateDescription, atBattleStart, onVictory, onPlayerHeal, onAttacked, makeCopy | 0 | [ ] | [ ] |
| DEPRECATEDYin | onUseCard, makeCopy | 0 | [ ] | [ ] |
| DEPRECATED_DarkCore | makeCopy | 0 | [ ] | [ ] |
| Damaru | makeCopy | 0 | [ ] | [ ] |
| DarkstonePeriapt | onObtainCard, makeCopy | 0 | [ ] | [ ] |
| DataDisk | atBattleStart, makeCopy | 1 refs | [ ] | [ ] |
| DeadBranch | onExhaust, makeCopy | 1 refs | [ ] | [ ] |
| DerpRock | atPreBattle, makeCopy | 0 | [ ] | [ ] |
| DiscerningMonocle | onEnterRoom, makeCopy | 0 | [ ] | [ ] |
| DollysMirror | onEquip, makeCopy | 0 | [ ] | [ ] |
| DreamCatcher | makeCopy | 2 refs | [ ] | [ ] |
| DuVuDoll | onMasterDeckChange, onEquip, atBattleStart, makeCopy | 0 | [ ] | [ ] |
| Duality | onUseCard, makeCopy | 0 | [ ] | [ ] |
| Ectoplasm | updateDescription, onEquip, onUnequip, makeCopy | 2 refs | [ ] | [ ] |
| EmotionChip | wasHPLost, onVictory, makeCopy | 1 refs | [ ] | [ ] |
| EmptyCage | onEquip, makeCopy | 0 | [ ] | [ ] |
| Enchiridion | atPreBattle, makeCopy | 2 refs | [ ] | [ ] |
| EternalFeather | onEnterRoom, makeCopy | 0 | [ ] | [ ] |
| FaceOfCleric | onVictory, makeCopy | 1 refs | [ ] | [ ] |
| FossilizedHelix | atBattleStart, justEnteredRoom, makeCopy | 0 | [ ] | [ ] |
| FrozenCore | onPlayerEndTurn, makeCopy | 0 | [ ] | [ ] |
| FrozenEgg2 | onEquip, onObtainCard, makeCopy | 0 | [ ] | [ ] |
| FrozenEye | makeCopy | 5 refs | [ ] | [ ] |
| FusionHammer | updateDescription, onEquip, onUnequip, makeCopy | 0 | [ ] | [ ] |
| GamblingChip | atBattleStartPreDraw, makeCopy | 0 | [ ] | [ ] |
| Ginger | makeCopy | 2 refs | [ ] | [ ] |
| Girya | atBattleStart, makeCopy | 2 refs | [ ] | [ ] |
| GoldPlatedCables | makeCopy | 0 | [ ] | [ ] |
| GoldenEye | makeCopy | 2 refs | [ ] | [ ] |
| GoldenIdol | makeCopy | 8 refs | [ ] | [ ] |
| GremlinHorn | updateDescription, onMonsterDeath, makeCopy | 0 | [ ] | [ ] |
| GremlinMask | atBattleStart, makeCopy | 1 refs | [ ] | [ ] |
| HandDrill | onBlockBroken, makeCopy | 0 | [ ] | [ ] |
| HappyFlower | updateDescription, onEquip, makeCopy | 0 | [ ] | [ ] |
| HolyWater | atBattleStartPreDraw, makeCopy | 0 | [ ] | [ ] |
| HornCleat | atBattleStart, onVictory, makeCopy | 0 | [ ] | [ ] |
| HoveringKite | onManualDiscard, makeCopy | 0 | [ ] | [ ] |
| IceCream | makeCopy | 3 refs | [ ] | [ ] |
| IncenseBurner | onEquip, makeCopy | 0 | [ ] | [ ] |
| InkBottle | onUseCard, atBattleStart, makeCopy | 0 | [ ] | [ ] |
| Inserter | onEquip, makeCopy | 0 | [ ] | [ ] |
| JuzuBracelet | makeCopy | 4 refs | [ ] | [ ] |
| Kunai | onUseCard, onVictory, makeCopy | 0 | [ ] | [ ] |
| Lantern | updateDescription, atPreBattle, makeCopy | 0 | [ ] | [ ] |
| LetterOpener | onUseCard, onVictory, makeCopy | 0 | [ ] | [ ] |
| LizardTail | onTrigger, makeCopy | 2 refs | [ ] | [ ] |
| MagicFlower | onPlayerHeal, makeCopy | 0 | [ ] | [ ] |
| Mango | onEquip, makeCopy | 0 | [ ] | [ ] |
| MarkOfPain | atBattleStart, onEquip, onUnequip, makeCopy | 0 | [ ] | [ ] |
| MarkOfTheBloom | onPlayerHeal, makeCopy | 2 refs | [ ] | [ ] |
| Matryoshka | onChestOpen, makeCopy | 0 | [ ] | [ ] |
| MawBank | onEnterRoom, makeCopy | 0 | [ ] | [ ] |
| MealTicket | justEnteredRoom, makeCopy | 0 | [ ] | [ ] |
| MeatOnTheBone | onTrigger, makeCopy | 2 refs | [ ] | [ ] |
| MedicalKit | makeCopy, onUseCard | 1 refs | [ ] | [ ] |
| Melange | onShuffle, makeCopy | 0 | [ ] | [ ] |
| MembershipCard | onEnterRoom, makeCopy | 6 refs | [ ] | [ ] |
| MercuryHourglass | makeCopy | 0 | [ ] | [ ] |
| MoltenEgg2 | onEquip, onObtainCard, makeCopy | 0 | [ ] | [ ] |
| MummifiedHand | onUseCard, makeCopy | 0 | [ ] | [ ] |
| MutagenicStrength | atBattleStart, makeCopy | 1 refs | [ ] | [ ] |
| Necronomicon | onEquip, onUnequip, onUseCard, makeCopy | 12 refs | [ ] | [ ] |
| NeowsLament | atBattleStart, makeCopy | 0 | [ ] | [ ] |
| NilrysCodex | onPlayerEndTurn, makeCopy | 0 | [ ] | [ ] |
| NinjaScroll | atBattleStartPreDraw, makeCopy | 0 | [ ] | [ ] |
| NlothsGift | makeCopy | 0 | [ ] | [ ] |
| NlothsMask | makeCopy | 1 refs | [ ] | [ ] |
| NuclearBattery | atPreBattle, makeCopy | 0 | [ ] | [ ] |
| Nunchaku | onUseCard, makeCopy | 0 | [ ] | [ ] |
| OddMushroom | makeCopy | 4 refs | [ ] | [ ] |
| OddlySmoothStone | atBattleStart, makeCopy | 0 | [ ] | [ ] |
| OldCoin | onEquip, makeCopy | 0 | [ ] | [ ] |
| Omamori | use, makeCopy | 5 refs | [ ] | [ ] |
| OrangePellets | onUseCard, makeCopy | 0 | [ ] | [ ] |
| Orichalcum | onPlayerEndTurn, onPlayerGainedBlock, onVictory, makeCopy | 0 | [ ] | [ ] |
| OrnamentalFan | onUseCard, onVictory, makeCopy | 0 | [ ] | [ ] |
| Orrery | onEquip, makeCopy | 0 | [ ] | [ ] |
| PandorasBox | onEquip, makeCopy | 0 | [ ] | [ ] |
| Pantograph | atBattleStart, makeCopy | 0 | [ ] | [ ] |
| PaperCrane | makeCopy | 3 refs | [ ] | [ ] |
| PaperFrog | makeCopy | 3 refs | [ ] | [ ] |
| PeacePipe | makeCopy | 0 | [ ] | [ ] |
| Pear | onEquip, makeCopy | 0 | [ ] | [ ] |
| PenNib | onUseCard, atBattleStart, makeCopy | 0 | [ ] | [ ] |
| PhilosopherStone | updateDescription, atBattleStart, onSpawnMonster, onEquip, onUnequip, makeCopy | 0 | [ ] | [ ] |
| Pocketwatch | atBattleStart, onPlayCard, onVictory, makeCopy | 0 | [ ] | [ ] |
| PotionBelt | onEquip, makeCopy | 0 | [ ] | [ ] |
| PrayerWheel | makeCopy | 2 refs | [ ] | [ ] |
| PreservedInsect | atBattleStart, makeCopy | 1 refs | [ ] | [ ] |
| PrismaticShard | makeCopy, onEquip | 1 refs | [ ] | [ ] |
| PureWater | atBattleStartPreDraw, makeCopy | 1 refs | [ ] | [ ] |
| QuestionCard | makeCopy | 2 refs | [ ] | [ ] |
| RedCirclet | makeCopy | 0 | [ ] | [ ] |
| RedMask | atBattleStart, makeCopy | 3 refs | [ ] | [ ] |
| RedSkull | atBattleStart, onVictory, makeCopy | 0 | [ ] | [ ] |
| RegalPillow | makeCopy | 5 refs | [ ] | [ ] |
| RingOfTheSerpent | onEquip, onUnequip, makeCopy | 0 | [ ] | [ ] |
| RunicCapacitor | atPreBattle, makeCopy | 1 refs | [ ] | [ ] |
| RunicCube | wasHPLost, makeCopy | 0 | [ ] | [ ] |
| RunicDome | updateDescription, onEquip, onUnequip, makeCopy | 2 refs | [ ] | [ ] |
| RunicPyramid | makeCopy | 2 refs | [ ] | [ ] |
| SacredBark | onEquip, makeCopy | 6 refs | [ ] | [ ] |
| SelfFormingClay | wasHPLost, makeCopy | 0 | [ ] | [ ] |
| Shovel | makeCopy | 1 refs | [ ] | [ ] |
| Shuriken | onUseCard, onVictory, makeCopy | 0 | [ ] | [ ] |
| SingingBowl | makeCopy | 4 refs | [ ] | [ ] |
| SlaversCollar | updateDescription, onVictory, makeCopy | 2 refs | [ ] | [ ] |
| Sling | atBattleStart, makeCopy | 0 | [ ] | [ ] |
| SmilingMask | onEnterRoom, makeCopy | 5 refs | [ ] | [ ] |
| SnakeRing | atBattleStart, makeCopy | 0 | [ ] | [ ] |
| SneckoEye | onEquip, onUnequip, atPreBattle, makeCopy | 0 | [ ] | [ ] |
| SneckoSkull | makeCopy | 0 | [ ] | [ ] |
| Sozu | updateDescription, onEquip, onUnequip, makeCopy | 11 refs | [ ] | [ ] |
| SpiritPoop | makeCopy | 3 refs | [ ] | [ ] |
| SsserpentHead | onEnterRoom, makeCopy | 1 refs | [ ] | [ ] |
| StoneCalendar | atBattleStart, onPlayerEndTurn, justEnteredRoom, onVictory, makeCopy | 0 | [ ] | [ ] |
| StrangeSpoon | makeCopy | 2 refs | [ ] | [ ] |
| Strawberry | onEquip, makeCopy | 0 | [ ] | [ ] |
| StrikeDummy | makeCopy | 1 refs | [ ] | [ ] |
| Sundial | onEquip, onShuffle, makeCopy | 0 | [ ] | [ ] |
| SymbioticVirus | atPreBattle, makeCopy | 1 refs | [ ] | [ ] |
| TeardropLocket | atBattleStart, makeCopy | 1 refs | [ ] | [ ] |
| Test1 | updateDescription, makeCopy | 0 | [ ] | [ ] |
| Test3 | onEquip, makeCopy | 0 | [ ] | [ ] |
| Test4 | atBattleStart, makeCopy | 0 | [ ] | [ ] |
| Test5 | onEquip, makeCopy | 0 | [ ] | [ ] |
| Test6 | onPlayerEndTurn, onVictory, makeCopy | 0 | [ ] | [ ] |
| TheSpecimen | onMonsterDeath, makeCopy | 0 | [ ] | [ ] |
| ThreadAndNeedle | atBattleStart, makeCopy | 0 | [ ] | [ ] |
| Tingsha | onManualDiscard, makeCopy | 0 | [ ] | [ ] |
| TinyChest | onEquip, makeCopy | 3 refs | [ ] | [ ] |
| TinyHouse | onEquip, makeCopy | 0 | [ ] | [ ] |
| Toolbox | atBattleStartPreDraw, makeCopy | 0 | [ ] | [ ] |
| Torii | onAttacked, makeCopy | 0 | [ ] | [ ] |
| ToughBandages | onManualDiscard, makeCopy | 0 | [ ] | [ ] |
| ToxicEgg2 | onEquip, onObtainCard, makeCopy | 0 | [ ] | [ ] |
| ToyOrnithopter | makeCopy | 0 | [ ] | [ ] |
| TungstenRod | makeCopy | 0 | [ ] | [ ] |
| Turnip | makeCopy | 3 refs | [ ] | [ ] |
| TwistedFunnel | atBattleStart, makeCopy | 0 | [ ] | [ ] |
| UnceasingTop | atPreBattle, makeCopy | 1 refs | [ ] | [ ] |
| Vajra | atBattleStart, makeCopy | 0 | [ ] | [ ] |
| VelvetChoker | updateDescription, onEquip, onUnequip, atBattleStart, onPlayCard, onVictory, makeCopy | 0 | [ ] | [ ] |
| VioletLotus | onChangeStance, makeCopy | 0 | [ ] | [ ] |
| Waffle | onEquip, makeCopy | 0 | [ ] | [ ] |
| WarPaint | onEquip, makeCopy | 0 | [ ] | [ ] |
| WarpedTongs | makeCopy | 0 | [ ] | [ ] |
| Whetstone | onEquip, makeCopy | 0 | [ ] | [ ] |
| WhiteBeast | makeCopy | 0 | [ ] | [ ] |
| WingBoots | makeCopy | 0 | [ ] | [ ] |
| WristBlade | makeCopy | 0 | [ ] | [ ] |

## Powers

| Power | Own Class Hooks | Engine-side Checks | Self Done | Engine Done |
|---|---|---|---|---|
| AbstractPower | updateDescription, reducePower, atDamageGive, atDamageFinalGive, atDamageFinalReceive, atDamageReceive, atDamageGive, atDamageFinalGive, atDamageFinalReceive, atDamageReceive, atStartOfTurn, duringTurn, atStartOfTurnPostDraw, atEndOfTurn, atEndOfTurnPreEndTurnCards, atEndOfRound, onHeal, onAttacked, onAttack, onAttackedToChangeDamage, onInflictDamage, onEvokeOrb, onCardDraw, onPlayCard, onUseCard, onAfterUseCard, wasHPLost, onSpecificTrigger, onDeath, onChannel, onExhaust, onChangeStance, onGainedBlock, onPlayerGainedBlock, onPlayerGainedBlock, onRemove, onDrawOrDiscard, onAfterCardPlayed, onInitialApplication, onApplyPower, onLoseHp, onVictory | 0 | [ ] | [ ] |
| AccuracyPower | updateDescription, onDrawOrDiscard | 0 | [ ] | [ ] |
| AfterImagePower | updateDescription, onUseCard | 0 | [ ] | [ ] |
| AmplifyPower | updateDescription, onUseCard, atEndOfTurn | 0 | [ ] | [ ] |
| AngerPower | updateDescription, onUseCard | 0 | [ ] | [ ] |
| AngryPower | onAttacked, updateDescription | 0 | [ ] | [ ] |
| ArtifactPower | onSpecificTrigger, updateDescription | 0 | [ ] | [ ] |
| AttackBurnPower | atEndOfRound, updateDescription, onUseCard | 0 | [ ] | [ ] |
| BackAttackPower | updateDescription | 0 | [ ] | [ ] |
| BarricadePower | updateDescription | 0 | [ ] | [ ] |
| BattleHymnPower | atStartOfTurn, updateDescription | 0 | [ ] | [ ] |
| BeatOfDeathPower | onAfterUseCard, updateDescription | 0 | [ ] | [ ] |
| BerserkPower | updateDescription, atStartOfTurn | 0 | [ ] | [ ] |
| BiasPower | atStartOfTurn, updateDescription | 0 | [ ] | [ ] |
| BlockReturnPower | onAttacked, updateDescription | 0 | [ ] | [ ] |
| BlurPower | updateDescription, atEndOfRound | 0 | [ ] | [ ] |
| BrutalityPower | updateDescription, atStartOfTurnPostDraw | 0 | [ ] | [ ] |
| BufferPower | updateDescription, onAttackedToChangeDamage | 0 | [ ] | [ ] |
| BurstPower | updateDescription, onUseCard, atEndOfTurn | 0 | [ ] | [ ] |
| CannotChangeStancePower | atEndOfTurn, updateDescription | 1 refs | [ ] | [ ] |
| ChokePower | atStartOfTurn, onUseCard | 0 | [ ] | [ ] |
| CollectPower | updateDescription | 0 | [ ] | [ ] |
| CombustPower | atEndOfTurn, updateDescription | 0 | [ ] | [ ] |
| ConfusionPower | onCardDraw, updateDescription | 0 | [ ] | [ ] |
| ConservePower | updateDescription, atEndOfRound | 0 | [ ] | [ ] |
| ConstrictedPower | updateDescription, atEndOfTurn | 0 | [ ] | [ ] |
| CorpseExplosionPower | onDeath, updateDescription | 0 | [ ] | [ ] |
| CorruptionPower | updateDescription, onCardDraw, onUseCard | 0 | [ ] | [ ] |
| CreativeAIPower | atStartOfTurn, updateDescription | 0 | [ ] | [ ] |
| CuriosityPower | updateDescription, onUseCard | 0 | [ ] | [ ] |
| CurlUpPower | onAttacked | 0 | [ ] | [ ] |
| DEPRECATEDAlwaysMadPower | updateDescription | 0 | [ ] | [ ] |
| DEPRECATEDCondensePower | onLoseHp, updateDescription | 0 | [ ] | [ ] |
| DEPRECATEDDisciplinePower | atEndOfTurn, atStartOfTurn, updateDescription | 0 | [ ] | [ ] |
| DEPRECATEDEmotionalTurmoilPower | atStartOfTurnPostDraw, updateDescription | 0 | [ ] | [ ] |
| DEPRECATEDFlickedPower | updateDescription | 0 | [ ] | [ ] |
| DEPRECATEDFlowPower | updateDescription, onUseCard | 0 | [ ] | [ ] |
| DEPRECATEDGroundedPower | updateDescription, onUseCard | 0 | [ ] | [ ] |
| DEPRECATEDHotHotPower | updateDescription, onAttacked | 0 | [ ] | [ ] |
| DEPRECATEDMasterRealityPower | onAfterCardPlayed, updateDescription | 0 | [ ] | [ ] |
| DEPRECATEDMasteryPower | updateDescription, onChangeStance | 0 | [ ] | [ ] |
| DEPRECATEDRetributionPower | onAttacked, updateDescription | 0 | [ ] | [ ] |
| DEPRECATEDSerenityPower | updateDescription, onAttacked | 0 | [ ] | [ ] |
| DarkEmbracePower | updateDescription, onExhaust | 0 | [ ] | [ ] |
| DemonFormPower | updateDescription, atStartOfTurnPostDraw | 0 | [ ] | [ ] |
| DevaPower | updateDescription | 0 | [ ] | [ ] |
| DevotionPower | updateDescription, atStartOfTurnPostDraw | 0 | [ ] | [ ] |
| DexterityPower | reducePower, updateDescription | 0 | [ ] | [ ] |
| DoubleDamagePower | atEndOfRound, updateDescription, atDamageGive | 0 | [ ] | [ ] |
| DoubleTapPower | updateDescription, onUseCard, atEndOfTurn | 0 | [ ] | [ ] |
| DrawCardNextTurnPower | updateDescription, atStartOfTurnPostDraw | 0 | [ ] | [ ] |
| DrawPower | onRemove, reducePower, updateDescription | 0 | [ ] | [ ] |
| DrawReductionPower | onInitialApplication, atEndOfRound, onRemove, updateDescription | 0 | [ ] | [ ] |
| DuplicationPower | updateDescription, onUseCard, atEndOfRound | 0 | [ ] | [ ] |
| EchoPower | updateDescription, atStartOfTurn, onUseCard | 0 | [ ] | [ ] |
| ElectroPower | updateDescription | 0 | [ ] | [ ] |
| EndTurnDeathPower | updateDescription, atStartOfTurn | 0 | [ ] | [ ] |
| EnergizedBluePower | updateDescription | 0 | [ ] | [ ] |
| EnergizedPower | updateDescription | 0 | [ ] | [ ] |
| EnergyDownPower | updateDescription, atStartOfTurn | 0 | [ ] | [ ] |
| EntanglePower | updateDescription, atEndOfTurn | 0 | [ ] | [ ] |
| EnvenomPower | updateDescription, onAttack | 0 | [ ] | [ ] |
| EquilibriumPower | updateDescription, atEndOfTurn, atEndOfRound | 0 | [ ] | [ ] |
| EstablishmentPower | updateDescription, atEndOfTurn | 0 | [ ] | [ ] |
| EvolvePower | updateDescription, onCardDraw | 0 | [ ] | [ ] |
| ExplosivePower | updateDescription, duringTurn | 0 | [ ] | [ ] |
| FadingPower | updateDescription, duringTurn | 0 | [ ] | [ ] |
| FeelNoPainPower | updateDescription, onExhaust | 0 | [ ] | [ ] |
| FireBreathingPower | updateDescription, onCardDraw | 0 | [ ] | [ ] |
| FlameBarrierPower | onAttacked, atStartOfTurn, updateDescription | 0 | [ ] | [ ] |
| FlightPower | updateDescription, atStartOfTurn, atDamageFinalReceive, onAttacked, onRemove | 0 | [ ] | [ ] |
| FocusPower | reducePower, updateDescription | 0 | [ ] | [ ] |
| ForcefieldPower | updateDescription, atDamageFinalReceive | 0 | [ ] | [ ] |
| ForesightPower | updateDescription, atStartOfTurn | 0 | [ ] | [ ] |
| FrailPower | atEndOfRound, updateDescription | 0 | [ ] | [ ] |
| FreeAttackPower | updateDescription, onUseCard | 1 refs | [ ] | [ ] |
| GainStrengthPower | reducePower, updateDescription, atEndOfTurn | 0 | [ ] | [ ] |
| GenericStrengthUpPower | updateDescription, atEndOfRound | 0 | [ ] | [ ] |
| GrowthPower | updateDescription, atEndOfRound | 0 | [ ] | [ ] |
| HeatsinkPower | onUseCard, updateDescription | 0 | [ ] | [ ] |
| HelloPower | atStartOfTurn, updateDescription | 0 | [ ] | [ ] |
| HexPower | onUseCard | 0 | [ ] | [ ] |
| InfiniteBladesPower | atStartOfTurn, updateDescription | 0 | [ ] | [ ] |
| IntangiblePlayerPower | atDamageFinalReceive, updateDescription, atEndOfRound | 0 | [ ] | [ ] |
| IntangiblePower | atDamageFinalReceive, updateDescription, atEndOfTurn | 0 | [ ] | [ ] |
| InvinciblePower | onAttackedToChangeDamage, atStartOfTurn, updateDescription | 0 | [ ] | [ ] |
| JuggernautPower | onGainedBlock, updateDescription | 0 | [ ] | [ ] |
| LightningMasteryPower | updateDescription | 0 | [ ] | [ ] |
| LikeWaterPower | updateDescription, atEndOfTurnPreEndTurnCards | 0 | [ ] | [ ] |
| LiveForeverPower | updateDescription, atEndOfTurn | 0 | [ ] | [ ] |
| LockOnPower | atEndOfRound, updateDescription | 0 | [ ] | [ ] |
| LoopPower | atStartOfTurn, updateDescription | 0 | [ ] | [ ] |
| LoseDexterityPower | updateDescription, atEndOfTurn | 0 | [ ] | [ ] |
| LoseStrengthPower | updateDescription, atEndOfTurn | 0 | [ ] | [ ] |
| MagnetismPower | atStartOfTurn, updateDescription | 0 | [ ] | [ ] |
| MalleablePower | updateDescription, atEndOfTurn, atEndOfRound, onAttacked | 0 | [ ] | [ ] |
| MantraPower | updateDescription | 0 | [ ] | [ ] |
| MarkPower | updateDescription | 0 | [ ] | [ ] |
| MasterRealityPower | updateDescription | 13 refs | [ ] | [ ] |
| MayhemPower | updateDescription, atStartOfTurn | 0 | [ ] | [ ] |
| MentalFortressPower | updateDescription, onChangeStance | 0 | [ ] | [ ] |
| MetallicizePower | updateDescription, atEndOfTurnPreEndTurnCards | 0 | [ ] | [ ] |
| MinionPower | updateDescription | 0 | [ ] | [ ] |
| ModeShiftPower | updateDescription | 0 | [ ] | [ ] |
| NextTurnBlockPower | updateDescription, atStartOfTurn | 0 | [ ] | [ ] |
| NightmarePower | updateDescription, atStartOfTurn | 0 | [ ] | [ ] |
| NirvanaPower | updateDescription | 0 | [ ] | [ ] |
| NoBlockPower | atEndOfRound, updateDescription | 0 | [ ] | [ ] |
| NoDrawPower | atEndOfTurn | 0 | [ ] | [ ] |
| NoSkillsPower | updateDescription, atEndOfTurn | 0 | [ ] | [ ] |
| NoxiousFumesPower | atStartOfTurnPostDraw, updateDescription | 0 | [ ] | [ ] |
| OmegaPower | updateDescription, atEndOfTurn | 0 | [ ] | [ ] |
| OmnisciencePower | updateDescription | 0 | [ ] | [ ] |
| PainfulStabsPower | updateDescription, onInflictDamage | 0 | [ ] | [ ] |
| PanachePower | updateDescription, onUseCard, atStartOfTurn | 0 | [ ] | [ ] |
| PenNibPower | onUseCard, updateDescription, atDamageGive | 0 | [ ] | [ ] |
| PhantasmalPower | updateDescription, atStartOfTurn | 0 | [ ] | [ ] |
| PlatedArmorPower | updateDescription, wasHPLost, onRemove, atEndOfTurnPreEndTurnCards | 0 | [ ] | [ ] |
| PoisonPower | updateDescription, atStartOfTurn | 0 | [ ] | [ ] |
| RagePower | updateDescription, onUseCard, atEndOfTurn | 0 | [ ] | [ ] |
| ReactivePower | updateDescription, onAttacked | 0 | [ ] | [ ] |
| ReboundPower | updateDescription, onAfterUseCard, atEndOfTurn | 0 | [ ] | [ ] |
| RechargingCorePower | updateDescription, atStartOfTurn | 0 | [ ] | [ ] |
| RegenPower | updateDescription, atEndOfTurn | 0 | [ ] | [ ] |
| RegenerateMonsterPower | updateDescription, atEndOfTurn | 0 | [ ] | [ ] |
| RegrowPower | updateDescription | 0 | [ ] | [ ] |
| RepairPower | updateDescription, onVictory | 0 | [ ] | [ ] |
| ResurrectPower | updateDescription | 0 | [ ] | [ ] |
| RetainCardPower | updateDescription, atEndOfTurn | 0 | [ ] | [ ] |
| RitualPower | updateDescription, atEndOfTurn, atEndOfRound | 0 | [ ] | [ ] |
| RupturePower | wasHPLost, updateDescription | 0 | [ ] | [ ] |
| RushdownPower | updateDescription, onChangeStance | 0 | [ ] | [ ] |
| SadisticPower | updateDescription, onApplyPower | 0 | [ ] | [ ] |
| SharpHidePower | updateDescription, onUseCard | 0 | [ ] | [ ] |
| ShiftingPower | onAttacked, updateDescription | 0 | [ ] | [ ] |
| SkillBurnPower | atEndOfRound, updateDescription, onUseCard | 0 | [ ] | [ ] |
| SlowPower | atEndOfRound, updateDescription, onAfterUseCard, atDamageReceive | 0 | [ ] | [ ] |
| SplitPower | updateDescription | 0 | [ ] | [ ] |
| SporeCloudPower | updateDescription, onDeath | 0 | [ ] | [ ] |
| StasisPower | updateDescription, onDeath | 0 | [ ] | [ ] |
| StaticDischargePower | onAttacked, updateDescription | 0 | [ ] | [ ] |
| StormPower | onUseCard, updateDescription | 0 | [ ] | [ ] |
| StrengthPower | reducePower, updateDescription, atDamageGive | 0 | [ ] | [ ] |
| StrikeUpPower | updateDescription, onDrawOrDiscard | 0 | [ ] | [ ] |
| StudyPower | atEndOfTurn, updateDescription | 0 | [ ] | [ ] |
| SurroundedPower | updateDescription | 0 | [ ] | [ ] |
| TheBombPower | atEndOfTurn, updateDescription | 0 | [ ] | [ ] |
| ThieveryPower | updateDescription | 0 | [ ] | [ ] |
| ThornsPower | onAttacked, updateDescription | 0 | [ ] | [ ] |
| ThousandCutsPower | onAfterCardPlayed, updateDescription | 0 | [ ] | [ ] |
| TimeMazePower | updateDescription, onAfterUseCard, atStartOfTurn | 0 | [ ] | [ ] |
| TimeWarpPower | updateDescription, onAfterUseCard | 0 | [ ] | [ ] |
| ToolsOfTheTradePower | updateDescription, atStartOfTurnPostDraw | 0 | [ ] | [ ] |
| UnawakenedPower | updateDescription | 0 | [ ] | [ ] |
| VaultPower | updateDescription, atEndOfRound | 0 | [ ] | [ ] |
| VigorPower | updateDescription, atDamageGive, onUseCard | 0 | [ ] | [ ] |
| VulnerablePower | atEndOfRound, updateDescription, atDamageReceive | 0 | [ ] | [ ] |
| WaveOfTheHandPower | onGainedBlock, atEndOfRound, updateDescription | 0 | [ ] | [ ] |
| WeakPower | atEndOfRound, updateDescription, atDamageGive | 0 | [ ] | [ ] |
| WinterPower | atStartOfTurn, updateDescription | 0 | [ ] | [ ] |
| WraithFormPower | atEndOfTurn, updateDescription | 0 | [ ] | [ ] |
| WrathNextTurnPower | updateDescription, atStartOfTurn | 0 | [ ] | [ ] |
