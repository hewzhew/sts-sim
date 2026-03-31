# StS Hook Override Table

Shows which hook methods each power/relic/card overrides.

## POWER Hooks

### AbstractPower
File: `powers\AbstractPower.java`

- `updateDescription`
- `reducePower`
- `atDamageGive`
- `atDamageFinalGive`
- `atDamageFinalReceive`
- `atDamageReceive`
- `atDamageGive`
- `atDamageFinalGive`
- `atDamageFinalReceive`
- `atDamageReceive`
- `atStartOfTurn`
- `duringTurn`
- `atStartOfTurnPostDraw`
- `atEndOfTurn`
- `atEndOfTurnPreEndTurnCards`
- `atEndOfRound`
- `onHeal`
- `onAttacked`
- `onAttack`
- `onAttackedToChangeDamage`
- `onInflictDamage`
- `onEvokeOrb`
- `onCardDraw`
- `onPlayCard`
- `onUseCard`
- `onAfterUseCard`
- `wasHPLost`
- `onSpecificTrigger`
- `onDeath`
- `onChannel`
- `onExhaust`
- `onChangeStance`
- `onGainedBlock`
- `onPlayerGainedBlock`
- `onPlayerGainedBlock`
- `onRemove`
- `onDrawOrDiscard`
- `onAfterCardPlayed`
- `onInitialApplication`
- `onApplyPower`
- `onLoseHp`
- `onVictory`

### AccuracyPower
File: `powers\AccuracyPower.java`

- `updateDescription`
- `onDrawOrDiscard`

### AfterImagePower
File: `powers\AfterImagePower.java`

- `updateDescription`
- `onUseCard`

### AmplifyPower
File: `powers\AmplifyPower.java`

- `updateDescription`
- `onUseCard`
- `atEndOfTurn`

### AngerPower
File: `powers\AngerPower.java`

- `updateDescription`
- `onUseCard`

### AngryPower
File: `powers\AngryPower.java`

- `onAttacked`
- `updateDescription`

### ArtifactPower
File: `powers\ArtifactPower.java`

- `onSpecificTrigger`
- `updateDescription`

### AttackBurnPower
File: `powers\AttackBurnPower.java`

- `atEndOfRound`
- `updateDescription`
- `onUseCard`

### BackAttackPower
File: `powers\BackAttackPower.java`

- `updateDescription`

### BarricadePower
File: `powers\BarricadePower.java`

- `updateDescription`

### BattleHymnPower
File: `powers\watcher\BattleHymnPower.java`

- `atStartOfTurn`
- `updateDescription`

### BeatOfDeathPower
File: `powers\BeatOfDeathPower.java`

- `onAfterUseCard`
- `updateDescription`

### BerserkPower
File: `powers\BerserkPower.java`

- `updateDescription`
- `atStartOfTurn`

### BiasPower
File: `powers\BiasPower.java`

- `atStartOfTurn`
- `updateDescription`

### BlockReturnPower
File: `powers\watcher\BlockReturnPower.java`

- `onAttacked`
- `updateDescription`

### BlurPower
File: `powers\BlurPower.java`

- `updateDescription`
- `atEndOfRound`

### BrutalityPower
File: `powers\BrutalityPower.java`

- `updateDescription`
- `atStartOfTurnPostDraw`

### BufferPower
File: `powers\BufferPower.java`

- `updateDescription`
- `onAttackedToChangeDamage`

### BurstPower
File: `powers\BurstPower.java`

- `updateDescription`
- `onUseCard`
- `atEndOfTurn`

### CannotChangeStancePower
File: `powers\watcher\CannotChangeStancePower.java`

- `atEndOfTurn`
- `updateDescription`

### ChokePower
File: `powers\ChokePower.java`

- `atStartOfTurn`
- `onUseCard`

### CollectPower
File: `powers\CollectPower.java`

- `updateDescription`

### CombustPower
File: `powers\CombustPower.java`

- `atEndOfTurn`
- `updateDescription`

### ConfusionPower
File: `powers\ConfusionPower.java`

- `onCardDraw`
- `updateDescription`

### ConservePower
File: `powers\ConservePower.java`

- `updateDescription`
- `atEndOfRound`

### ConstrictedPower
File: `powers\ConstrictedPower.java`

- `updateDescription`
- `atEndOfTurn`

### CorpseExplosionPower
File: `powers\CorpseExplosionPower.java`

- `onDeath`
- `updateDescription`

### CorruptionPower
File: `powers\CorruptionPower.java`

- `updateDescription`
- `onCardDraw`
- `onUseCard`

### CreativeAIPower
File: `powers\CreativeAIPower.java`

- `atStartOfTurn`
- `updateDescription`

### CuriosityPower
File: `powers\CuriosityPower.java`

- `updateDescription`
- `onUseCard`

### CurlUpPower
File: `powers\CurlUpPower.java`

- `onAttacked`

### DEPRECATEDAlwaysMadPower
File: `powers\deprecated\DEPRECATEDAlwaysMadPower.java`

- `updateDescription`

### DEPRECATEDCondensePower
File: `powers\deprecated\DEPRECATEDCondensePower.java`

- `onLoseHp`
- `updateDescription`

### DEPRECATEDDisciplinePower
File: `powers\deprecated\DEPRECATEDDisciplinePower.java`

- `atEndOfTurn`
- `atStartOfTurn`
- `updateDescription`

### DEPRECATEDEmotionalTurmoilPower
File: `powers\deprecated\DEPRECATEDEmotionalTurmoilPower.java`

- `atStartOfTurnPostDraw`
- `updateDescription`

### DEPRECATEDFlickedPower
File: `powers\deprecated\DEPRECATEDFlickedPower.java`

- `updateDescription`

### DEPRECATEDFlowPower
File: `powers\deprecated\DEPRECATEDFlowPower.java`

- `updateDescription`
- `onUseCard`

### DEPRECATEDGroundedPower
File: `powers\deprecated\DEPRECATEDGroundedPower.java`

- `updateDescription`
- `onUseCard`

### DEPRECATEDHotHotPower
File: `powers\deprecated\DEPRECATEDHotHotPower.java`

- `updateDescription`
- `onAttacked`

### DEPRECATEDMasterRealityPower
File: `powers\deprecated\DEPRECATEDMasterRealityPower.java`

- `onAfterCardPlayed`
- `updateDescription`

### DEPRECATEDMasteryPower
File: `powers\deprecated\DEPRECATEDMasteryPower.java`

- `updateDescription`
- `onChangeStance`

### DEPRECATEDRetributionPower
File: `powers\deprecated\DEPRECATEDRetributionPower.java`

- `onAttacked`
- `updateDescription`

### DEPRECATEDSerenityPower
File: `powers\deprecated\DEPRECATEDSerenityPower.java`

- `updateDescription`
- `onAttacked`

### DarkEmbracePower
File: `powers\DarkEmbracePower.java`

- `updateDescription`
- `onExhaust`

### DemonFormPower
File: `powers\DemonFormPower.java`

- `updateDescription`
- `atStartOfTurnPostDraw`

### DevaPower
File: `powers\watcher\DevaPower.java`

- `updateDescription`

### DevotionPower
File: `powers\watcher\DevotionPower.java`

- `updateDescription`
- `atStartOfTurnPostDraw`

### DexterityPower
File: `powers\DexterityPower.java`

- `reducePower`
- `updateDescription`

### DoubleDamagePower
File: `powers\DoubleDamagePower.java`

- `atEndOfRound`
- `updateDescription`
- `atDamageGive`

### DoubleTapPower
File: `powers\DoubleTapPower.java`

- `updateDescription`
- `onUseCard`
- `atEndOfTurn`

### DrawCardNextTurnPower
File: `powers\DrawCardNextTurnPower.java`

- `updateDescription`
- `atStartOfTurnPostDraw`

### DrawPower
File: `powers\DrawPower.java`

- `onRemove`
- `reducePower`
- `updateDescription`

### DrawReductionPower
File: `powers\DrawReductionPower.java`

- `onInitialApplication`
- `atEndOfRound`
- `onRemove`
- `updateDescription`

### DuplicationPower
File: `powers\DuplicationPower.java`

- `updateDescription`
- `onUseCard`
- `atEndOfRound`

### EchoPower
File: `powers\EchoPower.java`

- `updateDescription`
- `atStartOfTurn`
- `onUseCard`

### ElectroPower
File: `powers\ElectroPower.java`

- `updateDescription`

### EndTurnDeathPower
File: `powers\watcher\EndTurnDeathPower.java`

- `updateDescription`
- `atStartOfTurn`

### EnergizedBluePower
File: `powers\EnergizedBluePower.java`

- `updateDescription`

### EnergizedPower
File: `powers\EnergizedPower.java`

- `updateDescription`

### EnergyDownPower
File: `powers\watcher\EnergyDownPower.java`

- `updateDescription`
- `atStartOfTurn`

### EntanglePower
File: `powers\EntanglePower.java`

- `updateDescription`
- `atEndOfTurn`

### EnvenomPower
File: `powers\EnvenomPower.java`

- `updateDescription`
- `onAttack`

### EquilibriumPower
File: `powers\EquilibriumPower.java`

- `updateDescription`
- `atEndOfTurn`
- `atEndOfRound`

### EstablishmentPower
File: `powers\watcher\EstablishmentPower.java`

- `updateDescription`
- `atEndOfTurn`

### EvolvePower
File: `powers\EvolvePower.java`

- `updateDescription`
- `onCardDraw`

### ExplosivePower
File: `powers\ExplosivePower.java`

- `updateDescription`
- `duringTurn`

### FadingPower
File: `powers\FadingPower.java`

- `updateDescription`
- `duringTurn`

### FeelNoPainPower
File: `powers\FeelNoPainPower.java`

- `updateDescription`
- `onExhaust`

### FireBreathingPower
File: `powers\FireBreathingPower.java`

- `updateDescription`
- `onCardDraw`

### FlameBarrierPower
File: `powers\FlameBarrierPower.java`

- `onAttacked`
- `atStartOfTurn`
- `updateDescription`

### FlightPower
File: `powers\FlightPower.java`

- `updateDescription`
- `atStartOfTurn`
- `atDamageFinalReceive`
- `onAttacked`
- `onRemove`

### FocusPower
File: `powers\FocusPower.java`

- `reducePower`
- `updateDescription`

### ForcefieldPower
File: `powers\ForcefieldPower.java`

- `updateDescription`
- `atDamageFinalReceive`

### ForesightPower
File: `powers\watcher\ForesightPower.java`

- `updateDescription`
- `atStartOfTurn`

### FrailPower
File: `powers\FrailPower.java`

- `atEndOfRound`
- `updateDescription`

### FreeAttackPower
File: `powers\watcher\FreeAttackPower.java`

- `updateDescription`
- `onUseCard`

### GainStrengthPower
File: `powers\GainStrengthPower.java`

- `reducePower`
- `updateDescription`
- `atEndOfTurn`

### GenericStrengthUpPower
File: `powers\GenericStrengthUpPower.java`

- `updateDescription`
- `atEndOfRound`

### GrowthPower
File: `powers\GrowthPower.java`

- `updateDescription`
- `atEndOfRound`

### HeatsinkPower
File: `powers\HeatsinkPower.java`

- `onUseCard`
- `updateDescription`

### HelloPower
File: `powers\HelloPower.java`

- `atStartOfTurn`
- `updateDescription`

### HexPower
File: `powers\HexPower.java`

- `onUseCard`

### InfiniteBladesPower
File: `powers\InfiniteBladesPower.java`

- `atStartOfTurn`
- `updateDescription`

### IntangiblePlayerPower
File: `powers\IntangiblePlayerPower.java`

- `atDamageFinalReceive`
- `updateDescription`
- `atEndOfRound`

### IntangiblePower
File: `powers\IntangiblePower.java`

- `atDamageFinalReceive`
- `updateDescription`
- `atEndOfTurn`

### InvinciblePower
File: `powers\InvinciblePower.java`

- `onAttackedToChangeDamage`
- `atStartOfTurn`
- `updateDescription`

### JuggernautPower
File: `powers\JuggernautPower.java`

- `onGainedBlock`
- `updateDescription`

### LightningMasteryPower
File: `powers\LightningMasteryPower.java`

- `updateDescription`

### LikeWaterPower
File: `powers\watcher\LikeWaterPower.java`

- `updateDescription`
- `atEndOfTurnPreEndTurnCards`

### LiveForeverPower
File: `powers\watcher\LiveForeverPower.java`

- `updateDescription`
- `atEndOfTurn`

### LockOnPower
File: `powers\LockOnPower.java`

- `atEndOfRound`
- `updateDescription`

### LoopPower
File: `powers\LoopPower.java`

- `atStartOfTurn`
- `updateDescription`

### LoseDexterityPower
File: `powers\LoseDexterityPower.java`

- `updateDescription`
- `atEndOfTurn`

### LoseStrengthPower
File: `powers\LoseStrengthPower.java`

- `updateDescription`
- `atEndOfTurn`

### MagnetismPower
File: `powers\MagnetismPower.java`

- `atStartOfTurn`
- `updateDescription`

### MalleablePower
File: `powers\MalleablePower.java`

- `updateDescription`
- `atEndOfTurn`
- `atEndOfRound`
- `onAttacked`

### MantraPower
File: `powers\watcher\MantraPower.java`

- `updateDescription`

### MarkPower
File: `powers\watcher\MarkPower.java`

- `updateDescription`

### MasterRealityPower
File: `powers\watcher\MasterRealityPower.java`

- `updateDescription`

### MayhemPower
File: `powers\MayhemPower.java`

- `updateDescription`
- `atStartOfTurn`

### MentalFortressPower
File: `powers\watcher\MentalFortressPower.java`

- `updateDescription`
- `onChangeStance`

### MetallicizePower
File: `powers\MetallicizePower.java`

- `updateDescription`
- `atEndOfTurnPreEndTurnCards`

### MinionPower
File: `powers\MinionPower.java`

- `updateDescription`

### ModeShiftPower
File: `powers\ModeShiftPower.java`

- `updateDescription`

### NextTurnBlockPower
File: `powers\NextTurnBlockPower.java`

- `updateDescription`
- `atStartOfTurn`

### NightmarePower
File: `powers\NightmarePower.java`

- `updateDescription`
- `atStartOfTurn`

### NirvanaPower
File: `powers\watcher\NirvanaPower.java`

- `updateDescription`

### NoBlockPower
File: `powers\NoBlockPower.java`

- `atEndOfRound`
- `updateDescription`

### NoDrawPower
File: `powers\NoDrawPower.java`

- `atEndOfTurn`

### NoSkillsPower
File: `powers\watcher\NoSkillsPower.java`

- `updateDescription`
- `atEndOfTurn`

### NoxiousFumesPower
File: `powers\NoxiousFumesPower.java`

- `atStartOfTurnPostDraw`
- `updateDescription`

### OmegaPower
File: `powers\watcher\OmegaPower.java`

- `updateDescription`
- `atEndOfTurn`

### OmnisciencePower
File: `powers\watcher\OmnisciencePower.java`

- `updateDescription`

### PainfulStabsPower
File: `powers\PainfulStabsPower.java`

- `updateDescription`
- `onInflictDamage`

### PanachePower
File: `powers\PanachePower.java`

- `updateDescription`
- `onUseCard`
- `atStartOfTurn`

### PenNibPower
File: `powers\PenNibPower.java`

- `onUseCard`
- `updateDescription`
- `atDamageGive`

### PhantasmalPower
File: `powers\PhantasmalPower.java`

- `updateDescription`
- `atStartOfTurn`

### PlatedArmorPower
File: `powers\PlatedArmorPower.java`

- `updateDescription`
- `wasHPLost`
- `onRemove`
- `atEndOfTurnPreEndTurnCards`

### PoisonPower
File: `powers\PoisonPower.java`

- `updateDescription`
- `atStartOfTurn`

### RagePower
File: `powers\RagePower.java`

- `updateDescription`
- `onUseCard`
- `atEndOfTurn`

### ReactivePower
File: `powers\ReactivePower.java`

- `updateDescription`
- `onAttacked`

### ReboundPower
File: `powers\ReboundPower.java`

- `updateDescription`
- `onAfterUseCard`
- `atEndOfTurn`

### RechargingCorePower
File: `powers\RechargingCorePower.java`

- `updateDescription`
- `atStartOfTurn`

### RegenPower
File: `powers\RegenPower.java`

- `updateDescription`
- `atEndOfTurn`

### RegenerateMonsterPower
File: `powers\RegenerateMonsterPower.java`

- `updateDescription`
- `atEndOfTurn`

### RegrowPower
File: `powers\RegrowPower.java`

- `updateDescription`

### RepairPower
File: `powers\RepairPower.java`

- `updateDescription`
- `onVictory`

### ResurrectPower
File: `powers\ResurrectPower.java`

- `updateDescription`

### RetainCardPower
File: `powers\RetainCardPower.java`

- `updateDescription`
- `atEndOfTurn`

### RitualPower
File: `powers\RitualPower.java`

- `updateDescription`
- `atEndOfTurn`
- `atEndOfRound`

### RupturePower
File: `powers\RupturePower.java`

- `wasHPLost`
- `updateDescription`

### RushdownPower
File: `powers\watcher\RushdownPower.java`

- `updateDescription`
- `onChangeStance`

### SadisticPower
File: `powers\SadisticPower.java`

- `updateDescription`
- `onApplyPower`

### SharpHidePower
File: `powers\SharpHidePower.java`

- `updateDescription`
- `onUseCard`

### ShiftingPower
File: `powers\ShiftingPower.java`

- `onAttacked`
- `updateDescription`

### SkillBurnPower
File: `powers\SkillBurnPower.java`

- `atEndOfRound`
- `updateDescription`
- `onUseCard`

### SlowPower
File: `powers\SlowPower.java`

- `atEndOfRound`
- `updateDescription`
- `onAfterUseCard`
- `atDamageReceive`

### SplitPower
File: `powers\SplitPower.java`

- `updateDescription`

### SporeCloudPower
File: `powers\SporeCloudPower.java`

- `updateDescription`
- `onDeath`

### StasisPower
File: `powers\StasisPower.java`

- `updateDescription`
- `onDeath`

### StaticDischargePower
File: `powers\StaticDischargePower.java`

- `onAttacked`
- `updateDescription`

### StormPower
File: `powers\StormPower.java`

- `onUseCard`
- `updateDescription`

### StrengthPower
File: `powers\StrengthPower.java`

- `reducePower`
- `updateDescription`
- `atDamageGive`

### StrikeUpPower
File: `powers\StrikeUpPower.java`

- `updateDescription`
- `onDrawOrDiscard`

### StudyPower
File: `powers\watcher\StudyPower.java`

- `atEndOfTurn`
- `updateDescription`

### SurroundedPower
File: `powers\SurroundedPower.java`

- `updateDescription`

### TheBombPower
File: `powers\TheBombPower.java`

- `atEndOfTurn`
- `updateDescription`

### ThieveryPower
File: `powers\ThieveryPower.java`

- `updateDescription`

### ThornsPower
File: `powers\ThornsPower.java`

- `onAttacked`
- `updateDescription`

### ThousandCutsPower
File: `powers\ThousandCutsPower.java`

- `onAfterCardPlayed`
- `updateDescription`

### TimeMazePower
File: `powers\TimeMazePower.java`

- `updateDescription`
- `onAfterUseCard`
- `atStartOfTurn`

### TimeWarpPower
File: `powers\TimeWarpPower.java`

- `updateDescription`
- `onAfterUseCard`

### ToolsOfTheTradePower
File: `powers\ToolsOfTheTradePower.java`

- `updateDescription`
- `atStartOfTurnPostDraw`

### UnawakenedPower
File: `powers\UnawakenedPower.java`

- `updateDescription`

### VaultPower
File: `powers\watcher\VaultPower.java`

- `updateDescription`
- `atEndOfRound`

### VigorPower
File: `powers\watcher\VigorPower.java`

- `updateDescription`
- `atDamageGive`
- `onUseCard`

### VulnerablePower
File: `powers\VulnerablePower.java`

- `atEndOfRound`
- `updateDescription`
- `atDamageReceive`

### WaveOfTheHandPower
File: `powers\watcher\WaveOfTheHandPower.java`

- `onGainedBlock`
- `atEndOfRound`
- `updateDescription`

### WeakPower
File: `powers\WeakPower.java`

- `atEndOfRound`
- `updateDescription`
- `atDamageGive`

### WinterPower
File: `powers\WinterPower.java`

- `atStartOfTurn`
- `updateDescription`

### WraithFormPower
File: `powers\WraithFormPower.java`

- `atEndOfTurn`
- `updateDescription`

### WrathNextTurnPower
File: `powers\watcher\WrathNextTurnPower.java`

- `updateDescription`
- `atStartOfTurn`

## RELIC Hooks

### Abacus
File: `relics\Abacus.java`

- `onShuffle`
- `makeCopy`

### AbstractRelic
File: `relics\AbstractRelic.java`

- `updateDescription`
- `onEvokeOrb`
- `onPlayCard`
- `onObtainCard`
- `onEquip`
- `onUnequip`
- `atPreBattle`
- `atBattleStart`
- `onSpawnMonster`
- `atBattleStartPreDraw`
- `onPlayerEndTurn`
- `onManualDiscard`
- `onUseCard`
- `onVictory`
- `onMonsterDeath`
- `onBlockBroken`
- `onPlayerGainBlock`
- `onPlayerGainedBlock`
- `onPlayerHeal`
- `onEnterRestRoom`
- `onShuffle`
- `onSmith`
- `onAttack`
- `onAttacked`
- `onAttackedToChangeDamage`
- `onExhaust`
- `onTrigger`
- `onTrigger`
- `onEnterRoom`
- `justEnteredRoom`
- `onCardDraw`
- `onChestOpen`
- `onDrawOrDiscard`
- `onMasterDeckChange`
- `makeCopy`
- `onChangeStance`
- `onLoseHp`
- `wasHPLost`

### Akabeko
File: `relics\Akabeko.java`

- `atBattleStart`
- `makeCopy`

### Anchor
File: `relics\Anchor.java`

- `atBattleStart`
- `justEnteredRoom`
- `makeCopy`

### AncientTeaSet
File: `relics\AncientTeaSet.java`

- `updateDescription`
- `atPreBattle`
- `onEnterRestRoom`
- `makeCopy`

### ArtOfWar
File: `relics\ArtOfWar.java`

- `updateDescription`
- `atPreBattle`
- `onUseCard`
- `onVictory`
- `makeCopy`

### Astrolabe
File: `relics\Astrolabe.java`

- `onEquip`
- `makeCopy`

### BagOfMarbles
File: `relics\BagOfMarbles.java`

- `atBattleStart`
- `makeCopy`

### BagOfPreparation
File: `relics\BagOfPreparation.java`

- `atBattleStart`
- `makeCopy`

### BirdFacedUrn
File: `relics\BirdFacedUrn.java`

- `onUseCard`
- `makeCopy`

### BlackBlood
File: `relics\BlackBlood.java`

- `onVictory`
- `makeCopy`

### BlackStar
File: `relics\BlackStar.java`

- `onEnterRoom`
- `onVictory`
- `makeCopy`

### BloodVial
File: `relics\BloodVial.java`

- `atBattleStart`
- `makeCopy`

### BloodyIdol
File: `relics\BloodyIdol.java`

- `makeCopy`

### BlueCandle
File: `relics\BlueCandle.java`

- `makeCopy`
- `onUseCard`

### Boot
File: `relics\Boot.java`

- `makeCopy`

### BottledFlame
File: `relics\BottledFlame.java`

- `onEquip`
- `onUnequip`
- `atBattleStart`
- `makeCopy`

### BottledLightning
File: `relics\BottledLightning.java`

- `onEquip`
- `onUnequip`
- `atBattleStart`
- `makeCopy`

### BottledTornado
File: `relics\BottledTornado.java`

- `onEquip`
- `onUnequip`
- `atBattleStart`
- `makeCopy`

### Brimstone
File: `relics\Brimstone.java`

- `makeCopy`

### BronzeScales
File: `relics\BronzeScales.java`

- `atBattleStart`
- `makeCopy`

### BurningBlood
File: `relics\BurningBlood.java`

- `onVictory`
- `makeCopy`

### BustedCrown
File: `relics\BustedCrown.java`

- `updateDescription`
- `onEquip`
- `onUnequip`
- `makeCopy`

### Calipers
File: `relics\Calipers.java`

- `makeCopy`

### CallingBell
File: `relics\CallingBell.java`

- `onEquip`
- `makeCopy`

### CaptainsWheel
File: `relics\CaptainsWheel.java`

- `atBattleStart`
- `onVictory`
- `makeCopy`

### Cauldron
File: `relics\Cauldron.java`

- `onEquip`
- `makeCopy`

### CentennialPuzzle
File: `relics\CentennialPuzzle.java`

- `atPreBattle`
- `wasHPLost`
- `justEnteredRoom`
- `onVictory`
- `makeCopy`

### CeramicFish
File: `relics\CeramicFish.java`

- `use`
- `onObtainCard`
- `makeCopy`

### ChampionsBelt
File: `relics\ChampionsBelt.java`

- `onTrigger`
- `makeCopy`

### CharonsAshes
File: `relics\CharonsAshes.java`

- `onExhaust`
- `makeCopy`

### ChemicalX
File: `relics\ChemicalX.java`

- `makeCopy`

### Circlet
File: `relics\Circlet.java`

- `onEquip`
- `onUnequip`
- `makeCopy`

### CloakClasp
File: `relics\CloakClasp.java`

- `onPlayerEndTurn`
- `makeCopy`

### ClockworkSouvenir
File: `relics\ClockworkSouvenir.java`

- `atBattleStart`
- `makeCopy`

### CoffeeDripper
File: `relics\CoffeeDripper.java`

- `updateDescription`
- `onEquip`
- `onUnequip`
- `makeCopy`

### Courier
File: `relics\Courier.java`

- `onEnterRoom`
- `makeCopy`

### CrackedCore
File: `relics\CrackedCore.java`

- `atPreBattle`
- `makeCopy`

### CultistMask
File: `relics\CultistMask.java`

- `atBattleStart`
- `makeCopy`

### CursedKey
File: `relics\CursedKey.java`

- `justEnteredRoom`
- `onChestOpen`
- `updateDescription`
- `onEquip`
- `onUnequip`
- `makeCopy`

### DEPRECATEDDodecahedron
File: `relics\deprecated\DEPRECATEDDodecahedron.java`

- `updateDescription`
- `atBattleStart`
- `onVictory`
- `onPlayerHeal`
- `onAttacked`
- `makeCopy`

### DEPRECATEDYin
File: `relics\deprecated\DEPRECATEDYin.java`

- `onUseCard`
- `makeCopy`

### DEPRECATED_DarkCore
File: `relics\deprecated\DEPRECATED_DarkCore.java`

- `makeCopy`

### Damaru
File: `relics\Damaru.java`

- `makeCopy`

### DarkstonePeriapt
File: `relics\DarkstonePeriapt.java`

- `onObtainCard`
- `makeCopy`

### DataDisk
File: `relics\DataDisk.java`

- `atBattleStart`
- `makeCopy`

### DeadBranch
File: `relics\DeadBranch.java`

- `onExhaust`
- `makeCopy`

### DerpRock
File: `relics\deprecated\DerpRock.java`

- `atPreBattle`
- `makeCopy`

### DiscerningMonocle
File: `relics\DiscerningMonocle.java`

- `onEnterRoom`
- `makeCopy`

### DollysMirror
File: `relics\DollysMirror.java`

- `onEquip`
- `makeCopy`

### DreamCatcher
File: `relics\DreamCatcher.java`

- `makeCopy`

### DuVuDoll
File: `relics\DuVuDoll.java`

- `onMasterDeckChange`
- `onEquip`
- `atBattleStart`
- `makeCopy`

### Duality
File: `relics\Duality.java`

- `onUseCard`
- `makeCopy`

### Ectoplasm
File: `relics\Ectoplasm.java`

- `updateDescription`
- `onEquip`
- `onUnequip`
- `makeCopy`

### EmotionChip
File: `relics\EmotionChip.java`

- `wasHPLost`
- `onVictory`
- `makeCopy`

### EmptyCage
File: `relics\EmptyCage.java`

- `onEquip`
- `makeCopy`

### Enchiridion
File: `relics\Enchiridion.java`

- `atPreBattle`
- `makeCopy`

### EternalFeather
File: `relics\EternalFeather.java`

- `onEnterRoom`
- `makeCopy`

### FaceOfCleric
File: `relics\FaceOfCleric.java`

- `onVictory`
- `makeCopy`

### FossilizedHelix
File: `relics\FossilizedHelix.java`

- `atBattleStart`
- `justEnteredRoom`
- `makeCopy`

### FrozenCore
File: `relics\FrozenCore.java`

- `onPlayerEndTurn`
- `makeCopy`

### FrozenEgg2
File: `relics\FrozenEgg2.java`

- `onEquip`
- `onObtainCard`
- `makeCopy`

### FrozenEye
File: `relics\FrozenEye.java`

- `makeCopy`

### FusionHammer
File: `relics\FusionHammer.java`

- `updateDescription`
- `onEquip`
- `onUnequip`
- `makeCopy`

### GamblingChip
File: `relics\GamblingChip.java`

- `atBattleStartPreDraw`
- `makeCopy`

### Ginger
File: `relics\Ginger.java`

- `makeCopy`

### Girya
File: `relics\Girya.java`

- `atBattleStart`
- `makeCopy`

### GoldPlatedCables
File: `relics\GoldPlatedCables.java`

- `makeCopy`

### GoldenEye
File: `relics\GoldenEye.java`

- `makeCopy`

### GoldenIdol
File: `relics\GoldenIdol.java`

- `makeCopy`

### GremlinHorn
File: `relics\GremlinHorn.java`

- `updateDescription`
- `onMonsterDeath`
- `makeCopy`

### GremlinMask
File: `relics\GremlinMask.java`

- `atBattleStart`
- `makeCopy`

### HandDrill
File: `relics\HandDrill.java`

- `onBlockBroken`
- `makeCopy`

### HappyFlower
File: `relics\HappyFlower.java`

- `updateDescription`
- `onEquip`
- `makeCopy`

### HolyWater
File: `relics\HolyWater.java`

- `atBattleStartPreDraw`
- `makeCopy`

### HornCleat
File: `relics\HornCleat.java`

- `atBattleStart`
- `onVictory`
- `makeCopy`

### HoveringKite
File: `relics\HoveringKite.java`

- `onManualDiscard`
- `makeCopy`

### IceCream
File: `relics\IceCream.java`

- `makeCopy`

### IncenseBurner
File: `relics\IncenseBurner.java`

- `onEquip`
- `makeCopy`

### InkBottle
File: `relics\InkBottle.java`

- `onUseCard`
- `atBattleStart`
- `makeCopy`

### Inserter
File: `relics\Inserter.java`

- `onEquip`
- `makeCopy`

### JuzuBracelet
File: `relics\JuzuBracelet.java`

- `makeCopy`

### Kunai
File: `relics\Kunai.java`

- `onUseCard`
- `onVictory`
- `makeCopy`

### Lantern
File: `relics\Lantern.java`

- `updateDescription`
- `atPreBattle`
- `makeCopy`

### LetterOpener
File: `relics\LetterOpener.java`

- `onUseCard`
- `onVictory`
- `makeCopy`

### LizardTail
File: `relics\LizardTail.java`

- `onTrigger`
- `makeCopy`

### MagicFlower
File: `relics\MagicFlower.java`

- `onPlayerHeal`
- `makeCopy`

### Mango
File: `relics\Mango.java`

- `onEquip`
- `makeCopy`

### MarkOfPain
File: `relics\MarkOfPain.java`

- `atBattleStart`
- `onEquip`
- `onUnequip`
- `makeCopy`

### MarkOfTheBloom
File: `relics\MarkOfTheBloom.java`

- `onPlayerHeal`
- `makeCopy`

### Matryoshka
File: `relics\Matryoshka.java`

- `onChestOpen`
- `makeCopy`

### MawBank
File: `relics\MawBank.java`

- `onEnterRoom`
- `makeCopy`

### MealTicket
File: `relics\MealTicket.java`

- `justEnteredRoom`
- `makeCopy`

### MeatOnTheBone
File: `relics\MeatOnTheBone.java`

- `onTrigger`
- `makeCopy`

### MedicalKit
File: `relics\MedicalKit.java`

- `makeCopy`
- `onUseCard`

### Melange
File: `relics\Melange.java`

- `onShuffle`
- `makeCopy`

### MembershipCard
File: `relics\MembershipCard.java`

- `onEnterRoom`
- `makeCopy`

### MercuryHourglass
File: `relics\MercuryHourglass.java`

- `makeCopy`

### MoltenEgg2
File: `relics\MoltenEgg2.java`

- `onEquip`
- `onObtainCard`
- `makeCopy`

### MummifiedHand
File: `relics\MummifiedHand.java`

- `onUseCard`
- `makeCopy`

### MutagenicStrength
File: `relics\MutagenicStrength.java`

- `atBattleStart`
- `makeCopy`

### Necronomicon
File: `relics\Necronomicon.java`

- `onEquip`
- `onUnequip`
- `onUseCard`
- `makeCopy`

### NeowsLament
File: `relics\NeowsLament.java`

- `atBattleStart`
- `makeCopy`

### NilrysCodex
File: `relics\NilrysCodex.java`

- `onPlayerEndTurn`
- `makeCopy`

### NinjaScroll
File: `relics\NinjaScroll.java`

- `atBattleStartPreDraw`
- `makeCopy`

### NlothsGift
File: `relics\NlothsGift.java`

- `makeCopy`

### NlothsMask
File: `relics\NlothsMask.java`

- `makeCopy`

### NuclearBattery
File: `relics\NuclearBattery.java`

- `atPreBattle`
- `makeCopy`

### Nunchaku
File: `relics\Nunchaku.java`

- `onUseCard`
- `makeCopy`

### OddMushroom
File: `relics\OddMushroom.java`

- `makeCopy`

### OddlySmoothStone
File: `relics\OddlySmoothStone.java`

- `atBattleStart`
- `makeCopy`

### OldCoin
File: `relics\OldCoin.java`

- `onEquip`
- `makeCopy`

### Omamori
File: `relics\Omamori.java`

- `use`
- `makeCopy`

### OrangePellets
File: `relics\OrangePellets.java`

- `onUseCard`
- `makeCopy`

### Orichalcum
File: `relics\Orichalcum.java`

- `onPlayerEndTurn`
- `onPlayerGainedBlock`
- `onVictory`
- `makeCopy`

### OrnamentalFan
File: `relics\OrnamentalFan.java`

- `onUseCard`
- `onVictory`
- `makeCopy`

### Orrery
File: `relics\Orrery.java`

- `onEquip`
- `makeCopy`

### PandorasBox
File: `relics\PandorasBox.java`

- `onEquip`
- `makeCopy`

### Pantograph
File: `relics\Pantograph.java`

- `atBattleStart`
- `makeCopy`

### PaperCrane
File: `relics\PaperCrane.java`

- `makeCopy`

### PaperFrog
File: `relics\PaperFrog.java`

- `makeCopy`

### PeacePipe
File: `relics\PeacePipe.java`

- `makeCopy`

### Pear
File: `relics\Pear.java`

- `onEquip`
- `makeCopy`

### PenNib
File: `relics\PenNib.java`

- `onUseCard`
- `atBattleStart`
- `makeCopy`

### PhilosopherStone
File: `relics\PhilosopherStone.java`

- `updateDescription`
- `atBattleStart`
- `onSpawnMonster`
- `onEquip`
- `onUnequip`
- `makeCopy`

### Pocketwatch
File: `relics\Pocketwatch.java`

- `atBattleStart`
- `onPlayCard`
- `onVictory`
- `makeCopy`

### PotionBelt
File: `relics\PotionBelt.java`

- `onEquip`
- `makeCopy`

### PrayerWheel
File: `relics\PrayerWheel.java`

- `makeCopy`

### PreservedInsect
File: `relics\PreservedInsect.java`

- `atBattleStart`
- `makeCopy`

### PrismaticShard
File: `relics\PrismaticShard.java`

- `makeCopy`
- `onEquip`

### PureWater
File: `relics\PureWater.java`

- `atBattleStartPreDraw`
- `makeCopy`

### QuestionCard
File: `relics\QuestionCard.java`

- `makeCopy`

### RedCirclet
File: `relics\RedCirclet.java`

- `makeCopy`

### RedMask
File: `relics\RedMask.java`

- `atBattleStart`
- `makeCopy`

### RedSkull
File: `relics\RedSkull.java`

- `atBattleStart`
- `onVictory`
- `makeCopy`

### RegalPillow
File: `relics\RegalPillow.java`

- `makeCopy`

### RingOfTheSerpent
File: `relics\RingOfTheSerpent.java`

- `onEquip`
- `onUnequip`
- `makeCopy`

### RunicCapacitor
File: `relics\RunicCapacitor.java`

- `atPreBattle`
- `makeCopy`

### RunicCube
File: `relics\RunicCube.java`

- `wasHPLost`
- `makeCopy`

### RunicDome
File: `relics\RunicDome.java`

- `updateDescription`
- `onEquip`
- `onUnequip`
- `makeCopy`

### RunicPyramid
File: `relics\RunicPyramid.java`

- `makeCopy`

### SacredBark
File: `relics\SacredBark.java`

- `onEquip`
- `makeCopy`

### SelfFormingClay
File: `relics\SelfFormingClay.java`

- `wasHPLost`
- `makeCopy`

### Shovel
File: `relics\Shovel.java`

- `makeCopy`

### Shuriken
File: `relics\Shuriken.java`

- `onUseCard`
- `onVictory`
- `makeCopy`

### SingingBowl
File: `relics\SingingBowl.java`

- `makeCopy`

### SlaversCollar
File: `relics\SlaversCollar.java`

- `updateDescription`
- `onVictory`
- `makeCopy`

### Sling
File: `relics\Sling.java`

- `atBattleStart`
- `makeCopy`

### SmilingMask
File: `relics\SmilingMask.java`

- `onEnterRoom`
- `makeCopy`

### SnakeRing
File: `relics\SnakeRing.java`

- `atBattleStart`
- `makeCopy`

### SneckoEye
File: `relics\SneckoEye.java`

- `onEquip`
- `onUnequip`
- `atPreBattle`
- `makeCopy`

### SneckoSkull
File: `relics\SneckoSkull.java`

- `makeCopy`

### Sozu
File: `relics\Sozu.java`

- `updateDescription`
- `onEquip`
- `onUnequip`
- `makeCopy`

### SpiritPoop
File: `relics\SpiritPoop.java`

- `makeCopy`

### SsserpentHead
File: `relics\SsserpentHead.java`

- `onEnterRoom`
- `makeCopy`

### StoneCalendar
File: `relics\StoneCalendar.java`

- `atBattleStart`
- `onPlayerEndTurn`
- `justEnteredRoom`
- `onVictory`
- `makeCopy`

### StrangeSpoon
File: `relics\StrangeSpoon.java`

- `makeCopy`

### Strawberry
File: `relics\Strawberry.java`

- `onEquip`
- `makeCopy`

### StrikeDummy
File: `relics\StrikeDummy.java`

- `makeCopy`

### Sundial
File: `relics\Sundial.java`

- `onEquip`
- `onShuffle`
- `makeCopy`

### SymbioticVirus
File: `relics\SymbioticVirus.java`

- `atPreBattle`
- `makeCopy`

### TeardropLocket
File: `relics\TeardropLocket.java`

- `atBattleStart`
- `makeCopy`

### Test1
File: `relics\Test1.java`

- `updateDescription`
- `makeCopy`

### Test3
File: `relics\Test3.java`

- `onEquip`
- `makeCopy`

### Test4
File: `relics\Test4.java`

- `atBattleStart`
- `makeCopy`

### Test5
File: `relics\Test5.java`

- `onEquip`
- `makeCopy`

### Test6
File: `relics\Test6.java`

- `onPlayerEndTurn`
- `onVictory`
- `makeCopy`

### TheSpecimen
File: `relics\TheSpecimen.java`

- `onMonsterDeath`
- `makeCopy`

### ThreadAndNeedle
File: `relics\ThreadAndNeedle.java`

- `atBattleStart`
- `makeCopy`

### Tingsha
File: `relics\Tingsha.java`

- `onManualDiscard`
- `makeCopy`

### TinyChest
File: `relics\TinyChest.java`

- `onEquip`
- `makeCopy`

### TinyHouse
File: `relics\TinyHouse.java`

- `onEquip`
- `makeCopy`

### Toolbox
File: `relics\Toolbox.java`

- `atBattleStartPreDraw`
- `makeCopy`

### Torii
File: `relics\Torii.java`

- `onAttacked`
- `makeCopy`

### ToughBandages
File: `relics\ToughBandages.java`

- `onManualDiscard`
- `makeCopy`

### ToxicEgg2
File: `relics\ToxicEgg2.java`

- `onEquip`
- `onObtainCard`
- `makeCopy`

### ToyOrnithopter
File: `relics\ToyOrnithopter.java`

- `makeCopy`

### TungstenRod
File: `relics\TungstenRod.java`

- `makeCopy`

### Turnip
File: `relics\Turnip.java`

- `makeCopy`

### TwistedFunnel
File: `relics\TwistedFunnel.java`

- `atBattleStart`
- `makeCopy`

### UnceasingTop
File: `relics\UnceasingTop.java`

- `atPreBattle`
- `makeCopy`

### Vajra
File: `relics\Vajra.java`

- `atBattleStart`
- `makeCopy`

### VelvetChoker
File: `relics\VelvetChoker.java`

- `updateDescription`
- `onEquip`
- `onUnequip`
- `atBattleStart`
- `onPlayCard`
- `onVictory`
- `makeCopy`

### VioletLotus
File: `relics\VioletLotus.java`

- `onChangeStance`
- `makeCopy`

### Waffle
File: `relics\Waffle.java`

- `onEquip`
- `makeCopy`

### WarPaint
File: `relics\WarPaint.java`

- `onEquip`
- `makeCopy`

### WarpedTongs
File: `relics\WarpedTongs.java`

- `makeCopy`

### Whetstone
File: `relics\Whetstone.java`

- `onEquip`
- `makeCopy`

### WhiteBeast
File: `relics\WhiteBeast.java`

- `makeCopy`

### WingBoots
File: `relics\WingBoots.java`

- `makeCopy`

### WristBlade
File: `relics\WristBlade.java`

- `makeCopy`

## CARD Hooks

### AThousandCuts
File: `cards\green\AThousandCuts.java`

- `use`
- `upgrade`
- `makeCopy`

### AbstractCard
File: `cards\AbstractCard.java`

- `upgrade`
- `makeStatEquivalentCopy`
- `canUse`
- `use`
- `triggerWhenDrawn`
- `triggerOnEndOfPlayerTurn`
- `triggerOnEndOfTurnForPlayingCard`
- `triggerOnOtherCardPlayed`
- `triggerOnManualDiscard`
- `triggerAtStartOfTurn`
- `onPlayCard`
- `triggerOnExhaust`
- `triggerOnGlowCheck`
- `makeCopy`

### Accuracy
File: `cards\green\Accuracy.java`

- `use`
- `upgrade`
- `makeCopy`

### Acrobatics
File: `cards\green\Acrobatics.java`

- `use`
- `upgrade`
- `makeCopy`

### Adrenaline
File: `cards\green\Adrenaline.java`

- `use`
- `upgrade`
- `makeCopy`

### AfterImage
File: `cards\green\AfterImage.java`

- `use`
- `upgrade`
- `makeCopy`

### Aggregate
File: `cards\blue\Aggregate.java`

- `use`
- `upgrade`
- `makeCopy`

### Alchemize
File: `cards\green\Alchemize.java`

- `use`
- `upgrade`
- `makeCopy`

### AllForOne
File: `cards\blue\AllForOne.java`

- `use`
- `upgrade`
- `makeCopy`

### AllOutAttack
File: `cards\green\AllOutAttack.java`

- `use`
- `upgrade`
- `makeCopy`

### Alpha
File: `cards\purple\Alpha.java`

- `use`
- `makeCopy`
- `upgrade`

### Amplify
File: `cards\blue\Amplify.java`

- `use`
- `upgrade`
- `makeCopy`

### Anger
File: `cards\red\Anger.java`

- `use`
- `upgrade`
- `makeCopy`

### Apotheosis
File: `cards\colorless\Apotheosis.java`

- `use`
- `upgrade`
- `makeCopy`

### Apparition
File: `cards\colorless\Apparition.java`

- `use`
- `upgrade`
- `makeCopy`

### Armaments
File: `cards\red\Armaments.java`

- `use`
- `upgrade`
- `makeCopy`

### AscendersBane
File: `cards\curses\AscendersBane.java`

- `use`
- `upgrade`
- `makeCopy`

### AutoShields
File: `cards\blue\AutoShields.java`

- `use`
- `upgrade`
- `makeCopy`

### Backflip
File: `cards\green\Backflip.java`

- `use`
- `upgrade`
- `makeCopy`

### Backstab
File: `cards\green\Backstab.java`

- `use`
- `upgrade`
- `makeCopy`

### BallLightning
File: `cards\blue\BallLightning.java`

- `use`
- `upgrade`
- `makeCopy`

### BandageUp
File: `cards\colorless\BandageUp.java`

- `use`
- `upgrade`
- `makeCopy`

### Bane
File: `cards\green\Bane.java`

- `use`
- `upgrade`
- `makeCopy`

### Barrage
File: `cards\blue\Barrage.java`

- `use`
- `upgrade`
- `makeCopy`

### Barricade
File: `cards\red\Barricade.java`

- `use`
- `upgrade`
- `makeCopy`

### Bash
File: `cards\red\Bash.java`

- `use`
- `upgrade`
- `makeCopy`

### BattleHymn
File: `cards\purple\BattleHymn.java`

- `use`
- `upgrade`
- `makeCopy`

### BattleTrance
File: `cards\red\BattleTrance.java`

- `use`
- `upgrade`
- `makeCopy`

### BeamCell
File: `cards\blue\BeamCell.java`

- `use`
- `makeCopy`
- `upgrade`

### BecomeAlmighty
File: `cards\optionCards\BecomeAlmighty.java`

- `use`
- `upgrade`
- `makeCopy`

### Berserk
File: `cards\red\Berserk.java`

- `use`
- `upgrade`
- `makeCopy`

### Beta
File: `cards\tempCards\Beta.java`

- `use`
- `upgrade`
- `makeCopy`

### BiasedCognition
File: `cards\blue\BiasedCognition.java`

- `use`
- `upgrade`
- `makeCopy`

### Bite
File: `cards\colorless\Bite.java`

- `use`
- `upgrade`
- `makeCopy`

### BladeDance
File: `cards\green\BladeDance.java`

- `use`
- `upgrade`
- `makeCopy`

### Blasphemy
File: `cards\purple\Blasphemy.java`

- `use`
- `upgrade`
- `makeCopy`

### Blind
File: `cards\colorless\Blind.java`

- `use`
- `upgrade`
- `makeCopy`

### Blizzard
File: `cards\blue\Blizzard.java`

- `use`
- `upgrade`
- `makeCopy`

### BloodForBlood
File: `cards\red\BloodForBlood.java`

- `use`
- `upgrade`
- `makeCopy`

### Bloodletting
File: `cards\red\Bloodletting.java`

- `use`
- `upgrade`
- `makeCopy`

### Bludgeon
File: `cards\red\Bludgeon.java`

- `use`
- `upgrade`
- `makeCopy`

### Blur
File: `cards\green\Blur.java`

- `use`
- `upgrade`
- `makeCopy`

### BodySlam
File: `cards\red\BodySlam.java`

- `use`
- `upgrade`
- `makeCopy`

### BootSequence
File: `cards\blue\BootSequence.java`

- `use`
- `upgrade`
- `makeCopy`

### BouncingFlask
File: `cards\green\BouncingFlask.java`

- `use`
- `upgrade`
- `makeCopy`

### BowlingBash
File: `cards\purple\BowlingBash.java`

- `use`
- `upgrade`
- `makeCopy`

### Brilliance
File: `cards\purple\Brilliance.java`

- `use`
- `upgrade`
- `makeCopy`

### Brutality
File: `cards\red\Brutality.java`

- `use`
- `upgrade`
- `makeCopy`

### Buffer
File: `cards\blue\Buffer.java`

- `use`
- `upgrade`
- `makeCopy`

### BulletTime
File: `cards\green\BulletTime.java`

- `use`
- `upgrade`
- `makeCopy`

### Burn
File: `cards\status\Burn.java`

- `use`
- `triggerOnEndOfTurnForPlayingCard`
- `makeCopy`
- `upgrade`

### BurningPact
File: `cards\red\BurningPact.java`

- `use`
- `upgrade`
- `makeCopy`

### Burst
File: `cards\green\Burst.java`

- `use`
- `upgrade`
- `makeCopy`

### CalculatedGamble
File: `cards\green\CalculatedGamble.java`

- `use`
- `upgrade`
- `makeCopy`

### Caltrops
File: `cards\green\Caltrops.java`

- `use`
- `upgrade`
- `makeCopy`

### Capacitor
File: `cards\blue\Capacitor.java`

- `use`
- `upgrade`
- `makeCopy`

### Carnage
File: `cards\red\Carnage.java`

- `use`
- `upgrade`
- `makeCopy`

### CarveReality
File: `cards\purple\CarveReality.java`

- `use`
- `upgrade`
- `makeCopy`

### Catalyst
File: `cards\green\Catalyst.java`

- `use`
- `upgrade`
- `makeCopy`

### Chaos
File: `cards\blue\Chaos.java`

- `use`
- `upgrade`
- `makeCopy`

### Chill
File: `cards\blue\Chill.java`

- `use`
- `upgrade`
- `makeCopy`

### Choke
File: `cards\green\Choke.java`

- `use`
- `upgrade`
- `makeCopy`

### ChooseCalm
File: `cards\optionCards\ChooseCalm.java`

- `use`
- `upgrade`
- `makeCopy`

### ChooseWrath
File: `cards\optionCards\ChooseWrath.java`

- `use`
- `upgrade`
- `makeCopy`

### Chrysalis
File: `cards\colorless\Chrysalis.java`

- `use`
- `upgrade`
- `makeCopy`

### Clash
File: `cards\red\Clash.java`

- `use`
- `canUse`
- `upgrade`
- `makeCopy`

### Claw
File: `cards\blue\Claw.java`

- `use`
- `upgrade`
- `makeCopy`

### Cleave
File: `cards\red\Cleave.java`

- `use`
- `upgrade`
- `makeCopy`

### CloakAndDagger
File: `cards\green\CloakAndDagger.java`

- `use`
- `upgrade`
- `makeCopy`

### Clothesline
File: `cards\red\Clothesline.java`

- `use`
- `upgrade`
- `makeCopy`

### Clumsy
File: `cards\curses\Clumsy.java`

- `use`
- `triggerOnEndOfPlayerTurn`
- `upgrade`
- `makeCopy`

### ColdSnap
File: `cards\blue\ColdSnap.java`

- `use`
- `upgrade`
- `makeCopy`

### Collect
File: `cards\purple\Collect.java`

- `use`
- `upgrade`
- `makeCopy`

### Combust
File: `cards\red\Combust.java`

- `use`
- `upgrade`
- `makeCopy`

### CompileDriver
File: `cards\blue\CompileDriver.java`

- `use`
- `upgrade`
- `makeCopy`

### Concentrate
File: `cards\green\Concentrate.java`

- `use`
- `upgrade`
- `makeCopy`

### Conclude
File: `cards\purple\Conclude.java`

- `use`
- `upgrade`
- `makeCopy`

### ConjureBlade
File: `cards\purple\ConjureBlade.java`

- `use`
- `upgrade`
- `makeCopy`

### Consecrate
File: `cards\purple\Consecrate.java`

- `use`
- `upgrade`
- `makeCopy`

### ConserveBattery
File: `cards\blue\ConserveBattery.java`

- `use`
- `upgrade`
- `makeCopy`

### Consume
File: `cards\blue\Consume.java`

- `use`
- `upgrade`
- `makeCopy`

### Coolheaded
File: `cards\blue\Coolheaded.java`

- `use`
- `upgrade`
- `makeCopy`

### CoreSurge
File: `cards\blue\CoreSurge.java`

- `use`
- `upgrade`
- `makeCopy`

### CorpseExplosion
File: `cards\green\CorpseExplosion.java`

- `use`
- `upgrade`
- `makeCopy`

### Corruption
File: `cards\red\Corruption.java`

- `use`
- `upgrade`
- `makeCopy`

### CreativeAI
File: `cards\blue\CreativeAI.java`

- `use`
- `makeCopy`
- `upgrade`

### Crescendo
File: `cards\purple\Crescendo.java`

- `use`
- `upgrade`
- `makeCopy`

### CripplingPoison
File: `cards\green\CripplingPoison.java`

- `use`
- `upgrade`
- `makeCopy`

### CrushJoints
File: `cards\purple\CrushJoints.java`

- `use`
- `triggerOnGlowCheck`
- `upgrade`
- `makeCopy`

### CurseOfTheBell
File: `cards\curses\CurseOfTheBell.java`

- `use`
- `upgrade`
- `makeCopy`

### CutThroughFate
File: `cards\purple\CutThroughFate.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDAlwaysMad
File: `cards\deprecated\DEPRECATEDAlwaysMad.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDAndCarryOn
File: `cards\deprecated\DEPRECATEDAndCarryOn.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDAwakenedStrike
File: `cards\deprecated\DEPRECATEDAwakenedStrike.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDBalancedViolence
File: `cards\deprecated\DEPRECATEDBalancedViolence.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDBigBrain
File: `cards\deprecated\DEPRECATEDBigBrain.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDBlessed
File: `cards\deprecated\DEPRECATEDBlessed.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDBliss
File: `cards\deprecated\DEPRECATEDBliss.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDBrillianceAura
File: `cards\deprecated\DEPRECATEDBrillianceAura.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDCalm
File: `cards\deprecated\DEPRECATEDCalm.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDCausality
File: `cards\deprecated\DEPRECATEDCausality.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDChallengeAccepted
File: `cards\deprecated\DEPRECATEDChallengeAccepted.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDChooseCalm
File: `cards\deprecated\DEPRECATEDChooseCalm.java`

- `use`
- `makeCopy`
- `upgrade`

### DEPRECATEDChooseCourage
File: `cards\deprecated\DEPRECATEDChooseCourage.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDClarity
File: `cards\deprecated\DEPRECATEDClarity.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDCleanseEvil
File: `cards\deprecated\DEPRECATEDCleanseEvil.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDCondense
File: `cards\deprecated\DEPRECATEDCondense.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDConfront
File: `cards\deprecated\DEPRECATEDConfront.java`

- `use`
- `makeCopy`
- `upgrade`

### DEPRECATEDContemplate
File: `cards\deprecated\DEPRECATEDContemplate.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDCrescentKick
File: `cards\deprecated\DEPRECATEDCrescentKick.java`

- `use`
- `triggerOnGlowCheck`
- `upgrade`
- `makeCopy`

### DEPRECATEDEruption
File: `cards\deprecated\DEPRECATEDEruption.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDExperienced
File: `cards\deprecated\DEPRECATEDExperienced.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDFlameMastery
File: `cards\deprecated\DEPRECATEDFlameMastery.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDFlare
File: `cards\deprecated\DEPRECATEDFlare.java`

- `use`
- `upgrade`
- `triggerOnGlowCheck`
- `makeCopy`

### DEPRECATEDFlick
File: `cards\deprecated\DEPRECATEDFlick.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDFlicker
File: `cards\deprecated\DEPRECATEDFlicker.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDFlow
File: `cards\deprecated\DEPRECATEDFlow.java`

- `use`
- `makeCopy`
- `upgrade`

### DEPRECATEDFlowState
File: `cards\deprecated\DEPRECATEDFlowState.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDFury
File: `cards\deprecated\DEPRECATEDFury.java`

- `use`
- `makeCopy`
- `upgrade`

### DEPRECATEDFuryAura
File: `cards\deprecated\DEPRECATEDFuryAura.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDGrounded
File: `cards\deprecated\DEPRECATEDGrounded.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDHotHot
File: `cards\deprecated\DEPRECATEDHotHot.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDIntrospection
File: `cards\deprecated\DEPRECATEDIntrospection.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDLetFateDecide
File: `cards\deprecated\DEPRECATEDLetFateDecide.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDMasterReality
File: `cards\deprecated\DEPRECATEDMasterReality.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDMastery
File: `cards\deprecated\DEPRECATEDMastery.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDMetaphysics
File: `cards\deprecated\DEPRECATEDMetaphysics.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDNothingness
File: `cards\deprecated\DEPRECATEDNothingness.java`

- `use`
- `makeCopy`
- `upgrade`

### DEPRECATEDPathToVictory
File: `cards\deprecated\DEPRECATEDPathToVictory.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDPeace
File: `cards\deprecated\DEPRECATEDPeace.java`

- `use`
- `makeCopy`
- `upgrade`

### DEPRECATEDPerfectedForm
File: `cards\deprecated\DEPRECATEDPerfectedForm.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDPolymath
File: `cards\deprecated\DEPRECATEDPolymath.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDPrediction
File: `cards\deprecated\DEPRECATEDPrediction.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDPunishment
File: `cards\deprecated\DEPRECATEDPunishment.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDRestrainingPalm
File: `cards\deprecated\DEPRECATEDRestrainingPalm.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDRetreatingHand
File: `cards\deprecated\DEPRECATEDRetreatingHand.java`

- `use`
- `triggerOnGlowCheck`
- `upgrade`
- `makeCopy`

### DEPRECATEDRetribution
File: `cards\deprecated\DEPRECATEDRetribution.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDSerenity
File: `cards\deprecated\DEPRECATEDSerenity.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDSimmeringRage
File: `cards\deprecated\DEPRECATEDSimmeringRage.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDSmile
File: `cards\deprecated\DEPRECATEDSmile.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDSoothingAura
File: `cards\deprecated\DEPRECATEDSoothingAura.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDStepAndStrike
File: `cards\deprecated\DEPRECATEDStepAndStrike.java`

- `triggerWhenDrawn`
- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDStomp
File: `cards\deprecated\DEPRECATEDStomp.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDSublimeSlice
File: `cards\deprecated\DEPRECATEDSublimeSlice.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDSurvey
File: `cards\deprecated\DEPRECATEDSurvey.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDSwipe
File: `cards\deprecated\DEPRECATEDSwipe.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDTemperTantrum
File: `cards\deprecated\DEPRECATEDTemperTantrum.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDTorrent
File: `cards\deprecated\DEPRECATEDTorrent.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDTranscendence
File: `cards\deprecated\DEPRECATEDTranscendence.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDTruth
File: `cards\deprecated\DEPRECATEDTruth.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDWardAura
File: `cards\deprecated\DEPRECATEDWardAura.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDWindup
File: `cards\deprecated\DEPRECATEDWindup.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDWisdom
File: `cards\deprecated\DEPRECATEDWisdom.java`

- `use`
- `upgrade`
- `makeCopy`

### DEPRECATEDWrath
File: `cards\deprecated\DEPRECATEDWrath.java`

- `use`
- `makeCopy`
- `upgrade`

### DaggerSpray
File: `cards\green\DaggerSpray.java`

- `use`
- `upgrade`
- `makeCopy`

### DaggerThrow
File: `cards\green\DaggerThrow.java`

- `use`
- `upgrade`
- `makeCopy`

### DarkEmbrace
File: `cards\red\DarkEmbrace.java`

- `use`
- `upgrade`
- `makeCopy`

### DarkShackles
File: `cards\colorless\DarkShackles.java`

- `use`
- `upgrade`
- `makeCopy`

### Darkness
File: `cards\blue\Darkness.java`

- `use`
- `upgrade`
- `makeCopy`

### Dash
File: `cards\green\Dash.java`

- `use`
- `upgrade`
- `makeCopy`

### Dazed
File: `cards\status\Dazed.java`

- `use`
- `upgrade`
- `makeCopy`

### DeadlyPoison
File: `cards\green\DeadlyPoison.java`

- `use`
- `upgrade`
- `makeCopy`

### Decay
File: `cards\curses\Decay.java`

- `use`
- `triggerOnEndOfTurnForPlayingCard`
- `upgrade`
- `makeCopy`

### DeceiveReality
File: `cards\purple\DeceiveReality.java`

- `use`
- `upgrade`
- `makeCopy`

### DeepBreath
File: `cards\colorless\DeepBreath.java`

- `use`
- `upgrade`
- `makeCopy`

### Defend_Blue
File: `cards\blue\Defend_Blue.java`

- `use`
- `upgrade`
- `makeCopy`

### Defend_Green
File: `cards\green\Defend_Green.java`

- `use`
- `upgrade`
- `makeCopy`

### Defend_Red
File: `cards\red\Defend_Red.java`

- `use`
- `upgrade`
- `makeCopy`

### Defend_Watcher
File: `cards\purple\Defend_Watcher.java`

- `use`
- `makeCopy`
- `upgrade`

### Deflect
File: `cards\green\Deflect.java`

- `use`
- `upgrade`
- `makeCopy`

### Defragment
File: `cards\blue\Defragment.java`

- `use`
- `upgrade`
- `makeCopy`

### DemonForm
File: `cards\red\DemonForm.java`

- `use`
- `upgrade`
- `makeCopy`

### DeusExMachina
File: `cards\purple\DeusExMachina.java`

- `triggerWhenDrawn`
- `use`
- `canUse`
- `upgrade`
- `makeCopy`

### DevaForm
File: `cards\purple\DevaForm.java`

- `use`
- `upgrade`
- `makeCopy`

### Devotion
File: `cards\purple\Devotion.java`

- `use`
- `makeCopy`
- `upgrade`

### DieDieDie
File: `cards\green\DieDieDie.java`

- `use`
- `upgrade`
- `makeCopy`

### Disarm
File: `cards\red\Disarm.java`

- `use`
- `upgrade`
- `makeCopy`

### Discipline
File: `cards\purple\Discipline.java`

- `use`
- `upgrade`
- `makeCopy`

### Discovery
File: `cards\colorless\Discovery.java`

- `use`
- `upgrade`
- `makeCopy`

### Distraction
File: `cards\green\Distraction.java`

- `use`
- `upgrade`
- `makeCopy`

### DodgeAndRoll
File: `cards\green\DodgeAndRoll.java`

- `use`
- `upgrade`
- `makeCopy`

### DoomAndGloom
File: `cards\blue\DoomAndGloom.java`

- `use`
- `upgrade`
- `makeCopy`

### Doppelganger
File: `cards\green\Doppelganger.java`

- `use`
- `upgrade`
- `makeCopy`

### DoubleEnergy
File: `cards\blue\DoubleEnergy.java`

- `use`
- `upgrade`
- `makeCopy`

### DoubleTap
File: `cards\red\DoubleTap.java`

- `use`
- `upgrade`
- `makeCopy`

### Doubt
File: `cards\curses\Doubt.java`

- `use`
- `triggerWhenDrawn`
- `triggerOnEndOfTurnForPlayingCard`
- `upgrade`
- `makeCopy`

### DramaticEntrance
File: `cards\colorless\DramaticEntrance.java`

- `use`
- `upgrade`
- `makeCopy`

### Dropkick
File: `cards\red\Dropkick.java`

- `use`
- `triggerOnGlowCheck`
- `upgrade`
- `makeCopy`

### DualWield
File: `cards\red\DualWield.java`

- `use`
- `upgrade`
- `makeCopy`

### Dualcast
File: `cards\blue\Dualcast.java`

- `use`
- `upgrade`
- `makeCopy`

### EchoForm
File: `cards\blue\EchoForm.java`

- `use`
- `upgrade`
- `makeCopy`

### Electrodynamics
File: `cards\blue\Electrodynamics.java`

- `use`
- `upgrade`
- `makeCopy`

### EmptyBody
File: `cards\purple\EmptyBody.java`

- `use`
- `makeCopy`
- `upgrade`

### EmptyFist
File: `cards\purple\EmptyFist.java`

- `use`
- `upgrade`
- `makeCopy`

### EmptyMind
File: `cards\purple\EmptyMind.java`

- `use`
- `upgrade`
- `makeCopy`

### EndlessAgony
File: `cards\green\EndlessAgony.java`

- `triggerWhenDrawn`
- `use`
- `upgrade`
- `makeCopy`

### Enlightenment
File: `cards\colorless\Enlightenment.java`

- `use`
- `upgrade`
- `makeCopy`

### Entrench
File: `cards\red\Entrench.java`

- `use`
- `triggerOnEndOfPlayerTurn`
- `upgrade`
- `makeCopy`

### Envenom
File: `cards\green\Envenom.java`

- `use`
- `upgrade`
- `makeCopy`

### Equilibrium
File: `cards\blue\Equilibrium.java`

- `use`
- `upgrade`
- `makeCopy`

### Eruption
File: `cards\purple\Eruption.java`

- `use`
- `upgrade`
- `makeCopy`

### EscapePlan
File: `cards\green\EscapePlan.java`

- `use`
- `upgrade`
- `makeCopy`

### Establishment
File: `cards\purple\Establishment.java`

- `use`
- `upgrade`
- `makeCopy`

### Evaluate
File: `cards\purple\Evaluate.java`

- `use`
- `makeCopy`
- `upgrade`

### Eviscerate
File: `cards\green\Eviscerate.java`

- `triggerWhenDrawn`
- `use`
- `upgrade`
- `makeCopy`

### Evolve
File: `cards\red\Evolve.java`

- `use`
- `upgrade`
- `makeCopy`

### Exhume
File: `cards\red\Exhume.java`

- `use`
- `upgrade`
- `makeCopy`

### Expertise
File: `cards\green\Expertise.java`

- `use`
- `upgrade`
- `makeCopy`

### Expunger
File: `cards\tempCards\Expunger.java`

- `use`
- `upgrade`
- `makeCopy`
- `makeStatEquivalentCopy`

### FTL
File: `cards\blue\FTL.java`

- `use`
- `triggerOnGlowCheck`
- `upgrade`
- `makeCopy`

### FameAndFortune
File: `cards\optionCards\FameAndFortune.java`

- `use`
- `upgrade`
- `makeCopy`

### Fasting
File: `cards\purple\Fasting.java`

- `use`
- `makeCopy`
- `upgrade`

### FearNoEvil
File: `cards\purple\FearNoEvil.java`

- `use`
- `upgrade`
- `makeCopy`

### Feed
File: `cards\red\Feed.java`

- `use`
- `upgrade`
- `makeCopy`

### FeelNoPain
File: `cards\red\FeelNoPain.java`

- `use`
- `upgrade`
- `makeCopy`

### FiendFire
File: `cards\red\FiendFire.java`

- `use`
- `upgrade`
- `makeCopy`

### Finesse
File: `cards\colorless\Finesse.java`

- `use`
- `upgrade`
- `makeCopy`

### Finisher
File: `cards\green\Finisher.java`

- `use`
- `upgrade`
- `makeCopy`

### FireBreathing
File: `cards\red\FireBreathing.java`

- `use`
- `upgrade`
- `makeCopy`

### Fission
File: `cards\blue\Fission.java`

- `use`
- `upgrade`
- `makeCopy`

### FlameBarrier
File: `cards\red\FlameBarrier.java`

- `use`
- `upgrade`
- `makeCopy`

### FlashOfSteel
File: `cards\colorless\FlashOfSteel.java`

- `use`
- `upgrade`
- `makeCopy`

### Flechettes
File: `cards\green\Flechettes.java`

- `use`
- `upgrade`
- `makeCopy`

### Flex
File: `cards\red\Flex.java`

- `use`
- `upgrade`
- `makeCopy`

### FlurryOfBlows
File: `cards\purple\FlurryOfBlows.java`

- `use`
- `upgrade`
- `makeCopy`

### FlyingKnee
File: `cards\green\FlyingKnee.java`

- `use`
- `upgrade`
- `makeCopy`

### FlyingSleeves
File: `cards\purple\FlyingSleeves.java`

- `use`
- `makeCopy`
- `upgrade`

### FollowUp
File: `cards\purple\FollowUp.java`

- `use`
- `triggerOnGlowCheck`
- `upgrade`
- `makeCopy`

### Footwork
File: `cards\green\Footwork.java`

- `use`
- `upgrade`
- `makeCopy`

### ForceField
File: `cards\blue\ForceField.java`

- `use`
- `upgrade`
- `makeCopy`

### ForeignInfluence
File: `cards\purple\ForeignInfluence.java`

- `use`
- `upgrade`
- `makeCopy`

### Foresight
File: `cards\purple\Foresight.java`

- `use`
- `upgrade`
- `makeCopy`

### Forethought
File: `cards\colorless\Forethought.java`

- `use`
- `upgrade`
- `makeCopy`

### Fusion
File: `cards\blue\Fusion.java`

- `use`
- `upgrade`
- `makeCopy`

### GeneticAlgorithm
File: `cards\blue\GeneticAlgorithm.java`

- `use`
- `upgrade`
- `makeCopy`

### GhostlyArmor
File: `cards\red\GhostlyArmor.java`

- `use`
- `triggerOnEndOfPlayerTurn`
- `upgrade`
- `makeCopy`

### Glacier
File: `cards\blue\Glacier.java`

- `use`
- `upgrade`
- `makeCopy`

### GlassKnife
File: `cards\green\GlassKnife.java`

- `use`
- `upgrade`
- `makeCopy`

### GoForTheEyes
File: `cards\blue\GoForTheEyes.java`

- `use`
- `triggerOnGlowCheck`
- `upgrade`
- `makeCopy`

### GoodInstincts
File: `cards\colorless\GoodInstincts.java`

- `use`
- `upgrade`
- `makeCopy`

### GrandFinale
File: `cards\green\GrandFinale.java`

- `use`
- `triggerOnGlowCheck`
- `canUse`
- `upgrade`
- `makeCopy`

### Halt
File: `cards\purple\Halt.java`

- `use`
- `upgrade`
- `makeCopy`

### HandOfGreed
File: `cards\colorless\HandOfGreed.java`

- `use`
- `upgrade`
- `makeCopy`

### Havoc
File: `cards\red\Havoc.java`

- `use`
- `upgrade`
- `makeCopy`

### Headbutt
File: `cards\red\Headbutt.java`

- `use`
- `upgrade`
- `makeCopy`

### Heatsinks
File: `cards\blue\Heatsinks.java`

- `use`
- `upgrade`
- `makeCopy`

### HeavyBlade
File: `cards\red\HeavyBlade.java`

- `use`
- `upgrade`
- `makeCopy`

### HeelHook
File: `cards\green\HeelHook.java`

- `use`
- `triggerOnGlowCheck`
- `upgrade`
- `makeCopy`

### HelloWorld
File: `cards\blue\HelloWorld.java`

- `use`
- `upgrade`
- `makeCopy`

### Hemokinesis
File: `cards\red\Hemokinesis.java`

- `use`
- `upgrade`
- `makeCopy`

### Hologram
File: `cards\blue\Hologram.java`

- `use`
- `upgrade`
- `makeCopy`

### Hyperbeam
File: `cards\blue\Hyperbeam.java`

- `use`
- `upgrade`
- `makeCopy`

### Immolate
File: `cards\red\Immolate.java`

- `use`
- `upgrade`
- `makeCopy`

### Impatience
File: `cards\colorless\Impatience.java`

- `use`
- `triggerOnGlowCheck`
- `upgrade`
- `makeCopy`

### Impervious
File: `cards\red\Impervious.java`

- `use`
- `upgrade`
- `makeCopy`

### Impulse
File: `cards\blue\Impulse.java`

- `use`
- `upgrade`
- `makeCopy`

### Indignation
File: `cards\purple\Indignation.java`

- `use`
- `upgrade`
- `makeCopy`

### InfernalBlade
File: `cards\red\InfernalBlade.java`

- `use`
- `upgrade`
- `makeCopy`

### InfiniteBlades
File: `cards\green\InfiniteBlades.java`

- `use`
- `upgrade`
- `makeCopy`

### Inflame
File: `cards\red\Inflame.java`

- `use`
- `upgrade`
- `makeCopy`

### Injury
File: `cards\curses\Injury.java`

- `use`
- `upgrade`
- `makeCopy`

### InnerPeace
File: `cards\purple\InnerPeace.java`

- `use`
- `upgrade`
- `makeCopy`

### Insight
File: `cards\tempCards\Insight.java`

- `use`
- `upgrade`
- `makeCopy`

### Intimidate
File: `cards\red\Intimidate.java`

- `use`
- `upgrade`
- `makeCopy`

### IronWave
File: `cards\red\IronWave.java`

- `use`
- `upgrade`
- `makeCopy`

### JAX
File: `cards\colorless\JAX.java`

- `use`
- `upgrade`
- `makeCopy`

### JackOfAllTrades
File: `cards\colorless\JackOfAllTrades.java`

- `use`
- `upgrade`
- `makeCopy`

### Judgement
File: `cards\purple\Judgement.java`

- `use`
- `upgrade`
- `makeCopy`

### Juggernaut
File: `cards\red\Juggernaut.java`

- `use`
- `upgrade`
- `makeCopy`

### JustLucky
File: `cards\purple\JustLucky.java`

- `use`
- `upgrade`
- `makeCopy`

### Leap
File: `cards\blue\Leap.java`

- `use`
- `upgrade`
- `makeCopy`

### LegSweep
File: `cards\green\LegSweep.java`

- `use`
- `upgrade`
- `makeCopy`

### LessonLearned
File: `cards\purple\LessonLearned.java`

- `use`
- `upgrade`
- `makeCopy`

### LikeWater
File: `cards\purple\LikeWater.java`

- `use`
- `upgrade`
- `makeCopy`

### LimitBreak
File: `cards\red\LimitBreak.java`

- `use`
- `makeCopy`
- `upgrade`

### LiveForever
File: `cards\optionCards\LiveForever.java`

- `use`
- `upgrade`
- `makeCopy`

### LockOn
File: `cards\blue\LockOn.java`

- `use`
- `makeCopy`
- `upgrade`

### Loop
File: `cards\blue\Loop.java`

- `use`
- `upgrade`
- `makeCopy`

### MachineLearning
File: `cards\blue\MachineLearning.java`

- `use`
- `upgrade`
- `makeCopy`

### Madness
File: `cards\colorless\Madness.java`

- `use`
- `upgrade`
- `makeCopy`

### Magnetism
File: `cards\colorless\Magnetism.java`

- `use`
- `upgrade`
- `makeCopy`

### Malaise
File: `cards\green\Malaise.java`

- `use`
- `upgrade`
- `makeCopy`

### MasterOfStrategy
File: `cards\colorless\MasterOfStrategy.java`

- `use`
- `makeCopy`
- `upgrade`

### MasterReality
File: `cards\purple\MasterReality.java`

- `use`
- `upgrade`
- `makeCopy`

### MasterfulStab
File: `cards\green\MasterfulStab.java`

- `use`
- `upgrade`
- `makeCopy`

### Mayhem
File: `cards\colorless\Mayhem.java`

- `use`
- `makeCopy`
- `upgrade`

### Meditate
File: `cards\purple\Meditate.java`

- `use`
- `upgrade`
- `makeCopy`

### Melter
File: `cards\blue\Melter.java`

- `use`
- `upgrade`
- `makeCopy`

### MentalFortress
File: `cards\purple\MentalFortress.java`

- `use`
- `upgrade`
- `makeCopy`

### Metallicize
File: `cards\red\Metallicize.java`

- `use`
- `upgrade`
- `makeCopy`

### Metamorphosis
File: `cards\colorless\Metamorphosis.java`

- `use`
- `upgrade`
- `makeCopy`

### MeteorStrike
File: `cards\blue\MeteorStrike.java`

- `use`
- `upgrade`
- `makeCopy`

### MindBlast
File: `cards\colorless\MindBlast.java`

- `use`
- `upgrade`
- `makeCopy`

### Miracle
File: `cards\tempCards\Miracle.java`

- `use`
- `upgrade`
- `makeCopy`

### MultiCast
File: `cards\blue\MultiCast.java`

- `use`
- `upgrade`
- `makeCopy`

### Necronomicurse
File: `cards\curses\Necronomicurse.java`

- `use`
- `triggerOnExhaust`
- `upgrade`
- `makeCopy`

### Neutralize
File: `cards\green\Neutralize.java`

- `use`
- `upgrade`
- `makeCopy`

### Nightmare
File: `cards\green\Nightmare.java`

- `use`
- `upgrade`
- `makeCopy`

### Nirvana
File: `cards\purple\Nirvana.java`

- `use`
- `makeCopy`
- `upgrade`

### Normality
File: `cards\curses\Normality.java`

- `use`
- `upgrade`
- `makeCopy`

### NoxiousFumes
File: `cards\green\NoxiousFumes.java`

- `use`
- `upgrade`
- `makeCopy`

### Offering
File: `cards\red\Offering.java`

- `use`
- `upgrade`
- `makeCopy`

### Omega
File: `cards\tempCards\Omega.java`

- `use`
- `upgrade`
- `makeCopy`

### Omniscience
File: `cards\purple\Omniscience.java`

- `use`
- `upgrade`
- `makeCopy`

### Outmaneuver
File: `cards\green\Outmaneuver.java`

- `use`
- `upgrade`
- `makeCopy`

### Overclock
File: `cards\blue\Overclock.java`

- `use`
- `upgrade`
- `makeCopy`

### Pain
File: `cards\curses\Pain.java`

- `use`
- `triggerOnOtherCardPlayed`
- `upgrade`
- `makeCopy`

### Panacea
File: `cards\colorless\Panacea.java`

- `use`
- `upgrade`
- `makeCopy`

### Panache
File: `cards\colorless\Panache.java`

- `use`
- `upgrade`
- `makeCopy`

### PanicButton
File: `cards\colorless\PanicButton.java`

- `use`
- `upgrade`
- `makeCopy`

### Parasite
File: `cards\curses\Parasite.java`

- `use`
- `upgrade`
- `makeCopy`

### PerfectedStrike
File: `cards\red\PerfectedStrike.java`

- `use`
- `makeCopy`
- `upgrade`

### Perseverance
File: `cards\purple\Perseverance.java`

- `use`
- `upgrade`
- `makeCopy`

### PhantasmalKiller
File: `cards\green\PhantasmalKiller.java`

- `use`
- `upgrade`
- `makeCopy`

### PiercingWail
File: `cards\green\PiercingWail.java`

- `use`
- `upgrade`
- `makeCopy`

### PoisonedStab
File: `cards\green\PoisonedStab.java`

- `use`
- `upgrade`
- `makeCopy`

### PommelStrike
File: `cards\red\PommelStrike.java`

- `use`
- `upgrade`
- `makeCopy`

### PowerThrough
File: `cards\red\PowerThrough.java`

- `use`
- `upgrade`
- `makeCopy`

### Pray
File: `cards\purple\Pray.java`

- `use`
- `upgrade`
- `makeCopy`

### Predator
File: `cards\green\Predator.java`

- `use`
- `upgrade`
- `makeCopy`

### Prepared
File: `cards\green\Prepared.java`

- `use`
- `upgrade`
- `makeCopy`

### PressurePoints
File: `cards\purple\PressurePoints.java`

- `use`
- `upgrade`
- `makeCopy`

### Pride
File: `cards\curses\Pride.java`

- `use`
- `triggerOnEndOfTurnForPlayingCard`
- `upgrade`
- `makeCopy`

### Prostrate
File: `cards\purple\Prostrate.java`

- `use`
- `upgrade`
- `makeCopy`

### Protect
File: `cards\purple\Protect.java`

- `use`
- `makeCopy`
- `upgrade`

### Pummel
File: `cards\red\Pummel.java`

- `use`
- `upgrade`
- `makeCopy`

### Purity
File: `cards\colorless\Purity.java`

- `use`
- `upgrade`
- `makeCopy`

### QuickSlash
File: `cards\green\QuickSlash.java`

- `use`
- `upgrade`
- `makeCopy`

### Rage
File: `cards\red\Rage.java`

- `use`
- `upgrade`
- `makeCopy`

### Ragnarok
File: `cards\purple\Ragnarok.java`

- `use`
- `upgrade`
- `makeCopy`

### Rainbow
File: `cards\blue\Rainbow.java`

- `use`
- `makeCopy`
- `upgrade`

### Rampage
File: `cards\red\Rampage.java`

- `use`
- `upgrade`
- `makeCopy`

### ReachHeaven
File: `cards\purple\ReachHeaven.java`

- `use`
- `makeCopy`
- `upgrade`

### Reaper
File: `cards\red\Reaper.java`

- `use`
- `upgrade`
- `makeCopy`

### Reboot
File: `cards\blue\Reboot.java`

- `use`
- `makeCopy`
- `upgrade`

### Rebound
File: `cards\blue\Rebound.java`

- `use`
- `upgrade`
- `makeCopy`

### RecklessCharge
File: `cards\red\RecklessCharge.java`

- `use`
- `upgrade`
- `makeCopy`

### Recursion
File: `cards\blue\Recursion.java`

- `use`
- `upgrade`
- `makeCopy`

### Recycle
File: `cards\blue\Recycle.java`

- `use`
- `upgrade`
- `makeCopy`

### Reflex
File: `cards\green\Reflex.java`

- `use`
- `canUse`
- `triggerOnManualDiscard`
- `upgrade`
- `makeCopy`

### Regret
File: `cards\curses\Regret.java`

- `use`
- `triggerOnEndOfTurnForPlayingCard`
- `upgrade`
- `makeCopy`

### ReinforcedBody
File: `cards\blue\ReinforcedBody.java`

- `use`
- `upgrade`
- `makeCopy`

### Reprogram
File: `cards\blue\Reprogram.java`

- `use`
- `makeCopy`
- `upgrade`

### RiddleWithHoles
File: `cards\green\RiddleWithHoles.java`

- `use`
- `upgrade`
- `makeCopy`

### RipAndTear
File: `cards\blue\RipAndTear.java`

- `use`
- `upgrade`
- `makeCopy`

### RitualDagger
File: `cards\colorless\RitualDagger.java`

- `use`
- `upgrade`
- `makeCopy`

### Rupture
File: `cards\red\Rupture.java`

- `use`
- `upgrade`
- `makeCopy`

### Rushdown
File: `cards\purple\Rushdown.java`

- `use`
- `upgrade`
- `makeCopy`

### SadisticNature
File: `cards\colorless\SadisticNature.java`

- `use`
- `upgrade`
- `makeCopy`

### Safety
File: `cards\tempCards\Safety.java`

- `use`
- `makeCopy`
- `upgrade`

### Sanctity
File: `cards\purple\Sanctity.java`

- `use`
- `triggerOnGlowCheck`
- `upgrade`
- `makeCopy`

### SandsOfTime
File: `cards\purple\SandsOfTime.java`

- `use`
- `upgrade`
- `makeCopy`

### SashWhip
File: `cards\purple\SashWhip.java`

- `use`
- `triggerOnGlowCheck`
- `upgrade`
- `makeCopy`

### Scrape
File: `cards\blue\Scrape.java`

- `use`
- `upgrade`
- `makeCopy`

### Scrawl
File: `cards\purple\Scrawl.java`

- `use`
- `upgrade`
- `makeCopy`

### SearingBlow
File: `cards\red\SearingBlow.java`

- `use`
- `upgrade`
- `makeCopy`

### SecondWind
File: `cards\red\SecondWind.java`

- `use`
- `upgrade`
- `makeCopy`

### SecretTechnique
File: `cards\colorless\SecretTechnique.java`

- `use`
- `canUse`
- `upgrade`
- `makeCopy`

### SecretWeapon
File: `cards\colorless\SecretWeapon.java`

- `use`
- `canUse`
- `upgrade`
- `makeCopy`

### SeeingRed
File: `cards\red\SeeingRed.java`

- `use`
- `upgrade`
- `makeCopy`

### Seek
File: `cards\blue\Seek.java`

- `use`
- `upgrade`
- `makeCopy`

### SelfRepair
File: `cards\blue\SelfRepair.java`

- `use`
- `upgrade`
- `makeCopy`

### Sentinel
File: `cards\red\Sentinel.java`

- `use`
- `triggerOnExhaust`
- `upgrade`
- `makeCopy`

### Setup
File: `cards\green\Setup.java`

- `use`
- `makeCopy`
- `upgrade`

### SeverSoul
File: `cards\red\SeverSoul.java`

- `use`
- `makeCopy`
- `upgrade`

### Shame
File: `cards\curses\Shame.java`

- `use`
- `triggerOnEndOfTurnForPlayingCard`
- `upgrade`
- `makeCopy`

### Shiv
File: `cards\tempCards\Shiv.java`

- `use`
- `makeCopy`
- `upgrade`

### Shockwave
File: `cards\red\Shockwave.java`

- `use`
- `upgrade`
- `makeCopy`

### ShrugItOff
File: `cards\red\ShrugItOff.java`

- `use`
- `makeCopy`
- `upgrade`

### SignatureMove
File: `cards\purple\SignatureMove.java`

- `use`
- `upgrade`
- `canUse`
- `triggerOnGlowCheck`
- `makeCopy`

### SimmeringFury
File: `cards\purple\SimmeringFury.java`

- `use`
- `upgrade`
- `makeCopy`

### Skewer
File: `cards\green\Skewer.java`

- `use`
- `makeCopy`
- `upgrade`

### Skim
File: `cards\blue\Skim.java`

- `use`
- `upgrade`
- `makeCopy`

### Slice
File: `cards\green\Slice.java`

- `use`
- `upgrade`
- `makeCopy`

### Slimed
File: `cards\status\Slimed.java`

- `use`
- `upgrade`
- `makeCopy`

### Smite
File: `cards\tempCards\Smite.java`

- `use`
- `makeCopy`
- `upgrade`

### SneakyStrike
File: `cards\green\SneakyStrike.java`

- `use`
- `triggerOnGlowCheck`
- `upgrade`
- `makeCopy`

### SpiritShield
File: `cards\purple\SpiritShield.java`

- `use`
- `upgrade`
- `makeCopy`

### SpotWeakness
File: `cards\red\SpotWeakness.java`

- `use`
- `upgrade`
- `makeCopy`

### Stack
File: `cards\blue\Stack.java`

- `use`
- `upgrade`
- `makeCopy`

### StaticDischarge
File: `cards\blue\StaticDischarge.java`

- `use`
- `makeCopy`
- `upgrade`

### SteamBarrier
File: `cards\blue\SteamBarrier.java`

- `use`
- `upgrade`
- `makeCopy`

### Storm
File: `cards\blue\Storm.java`

- `use`
- `upgrade`
- `makeCopy`

### StormOfSteel
File: `cards\green\StormOfSteel.java`

- `use`
- `makeCopy`
- `upgrade`

### Streamline
File: `cards\blue\Streamline.java`

- `use`
- `upgrade`
- `makeCopy`

### Strike_Blue
File: `cards\blue\Strike_Blue.java`

- `use`
- `upgrade`
- `makeCopy`

### Strike_Green
File: `cards\green\Strike_Green.java`

- `use`
- `upgrade`
- `makeCopy`

### Strike_Purple
File: `cards\purple\Strike_Purple.java`

- `use`
- `upgrade`
- `makeCopy`

### Strike_Red
File: `cards\red\Strike_Red.java`

- `use`
- `upgrade`
- `makeCopy`

### Study
File: `cards\purple\Study.java`

- `use`
- `makeCopy`
- `upgrade`

### SuckerPunch
File: `cards\green\SuckerPunch.java`

- `use`
- `upgrade`
- `makeCopy`

### Sunder
File: `cards\blue\Sunder.java`

- `use`
- `upgrade`
- `makeCopy`

### Survivor
File: `cards\green\Survivor.java`

- `use`
- `upgrade`
- `makeCopy`

### SweepingBeam
File: `cards\blue\SweepingBeam.java`

- `use`
- `upgrade`
- `makeCopy`

### SwiftStrike
File: `cards\colorless\SwiftStrike.java`

- `use`
- `upgrade`
- `makeCopy`

### Swivel
File: `cards\purple\Swivel.java`

- `use`
- `upgrade`
- `makeCopy`

### SwordBoomerang
File: `cards\red\SwordBoomerang.java`

- `use`
- `upgrade`
- `makeCopy`

### Tactician
File: `cards\green\Tactician.java`

- `use`
- `canUse`
- `triggerOnManualDiscard`
- `makeCopy`
- `upgrade`

### TalkToTheHand
File: `cards\purple\TalkToTheHand.java`

- `use`
- `upgrade`
- `makeCopy`

### Tantrum
File: `cards\purple\Tantrum.java`

- `use`
- `upgrade`
- `makeCopy`

### Tempest
File: `cards\blue\Tempest.java`

- `use`
- `upgrade`
- `makeCopy`

### Terror
File: `cards\green\Terror.java`

- `use`
- `upgrade`
- `makeCopy`

### TheBomb
File: `cards\colorless\TheBomb.java`

- `use`
- `makeCopy`
- `upgrade`

### ThinkingAhead
File: `cards\colorless\ThinkingAhead.java`

- `use`
- `makeCopy`
- `upgrade`

### ThirdEye
File: `cards\purple\ThirdEye.java`

- `use`
- `upgrade`
- `makeCopy`

### ThroughViolence
File: `cards\tempCards\ThroughViolence.java`

- `use`
- `upgrade`
- `makeCopy`

### ThunderClap
File: `cards\red\ThunderClap.java`

- `use`
- `upgrade`
- `makeCopy`

### ThunderStrike
File: `cards\blue\ThunderStrike.java`

- `use`
- `upgrade`
- `makeCopy`

### ToolsOfTheTrade
File: `cards\green\ToolsOfTheTrade.java`

- `use`
- `upgrade`
- `makeCopy`

### Tranquility
File: `cards\purple\Tranquility.java`

- `use`
- `upgrade`
- `makeCopy`

### Transmutation
File: `cards\colorless\Transmutation.java`

- `use`
- `makeCopy`
- `upgrade`

### Trip
File: `cards\colorless\Trip.java`

- `use`
- `makeCopy`
- `upgrade`

### TrueGrit
File: `cards\red\TrueGrit.java`

- `use`
- `upgrade`
- `makeCopy`

### Turbo
File: `cards\blue\Turbo.java`

- `use`
- `upgrade`
- `makeCopy`

### TwinStrike
File: `cards\red\TwinStrike.java`

- `use`
- `upgrade`
- `makeCopy`

### Unload
File: `cards\green\Unload.java`

- `use`
- `upgrade`
- `makeCopy`

### Unraveling
File: `cards\purple\Unraveling.java`

- `use`
- `upgrade`
- `makeCopy`

### Uppercut
File: `cards\red\Uppercut.java`

- `use`
- `upgrade`
- `makeCopy`

### Vault
File: `cards\purple\Vault.java`

- `use`
- `upgrade`
- `makeCopy`

### Vigilance
File: `cards\purple\Vigilance.java`

- `use`
- `upgrade`
- `makeCopy`

### Violence
File: `cards\colorless\Violence.java`

- `use`
- `upgrade`
- `makeCopy`

### VoidCard
File: `cards\status\VoidCard.java`

- `triggerWhenDrawn`
- `use`
- `upgrade`
- `makeCopy`

### Wallop
File: `cards\purple\Wallop.java`

- `use`
- `upgrade`
- `makeCopy`

### Warcry
File: `cards\red\Warcry.java`

- `use`
- `upgrade`
- `makeCopy`

### WaveOfTheHand
File: `cards\purple\WaveOfTheHand.java`

- `use`
- `upgrade`
- `makeCopy`

### Weave
File: `cards\purple\Weave.java`

- `use`
- `upgrade`
- `makeCopy`

### WellLaidPlans
File: `cards\green\WellLaidPlans.java`

- `use`
- `upgrade`
- `makeCopy`

### WheelKick
File: `cards\purple\WheelKick.java`

- `use`
- `upgrade`
- `makeCopy`

### Whirlwind
File: `cards\red\Whirlwind.java`

- `use`
- `upgrade`
- `makeCopy`

### WhiteNoise
File: `cards\blue\WhiteNoise.java`

- `use`
- `upgrade`
- `makeCopy`

### WildStrike
File: `cards\red\WildStrike.java`

- `use`
- `upgrade`
- `makeCopy`

### WindmillStrike
File: `cards\purple\WindmillStrike.java`

- `use`
- `upgrade`
- `makeCopy`

### Wish
File: `cards\purple\Wish.java`

- `use`
- `upgrade`
- `makeCopy`

### Worship
File: `cards\purple\Worship.java`

- `use`
- `upgrade`
- `makeCopy`

### Wound
File: `cards\status\Wound.java`

- `use`
- `upgrade`
- `makeCopy`

### WraithForm
File: `cards\green\WraithForm.java`

- `use`
- `upgrade`
- `makeCopy`

### WreathOfFlame
File: `cards\purple\WreathOfFlame.java`

- `use`
- `upgrade`
- `makeCopy`

### Writhe
File: `cards\curses\Writhe.java`

- `use`
- `upgrade`
- `makeCopy`

### Zap
File: `cards\blue\Zap.java`

- `use`
- `upgrade`
- `makeCopy`

