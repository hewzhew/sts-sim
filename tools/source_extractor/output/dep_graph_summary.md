# StS Dependency Graph Summary (v2)

## Overview

- **card**: 417
- **power**: 158
- **relic**: 214
- **potion**: 43
- **monster**: 66
- **action**: 278

**Total**: 1181 nodes, 2949 edges

- **checks_for**: 166
- **creates**: 2783

## Entities with Hidden State (Private Fields)

These entities have private fields beyond `amount` — each is a potential extra_data bug.

| Entity | Category | Fields | Custom Stack? |
|--------|----------|--------|---------------|
| `AcidSlime_L` | monster | `saveX`(float), `saveY`(float), `splitTriggered`(boolean) |  |
| `AggregateEnergyAction` | action | `divideAmount`(int) |  |
| `AllCostToHandAction` | action | `costTarget`(int) |  |
| `Ancient Tea Set` | relic | `firstTurn`(boolean), `setDescription`(String) |  |
| `AnimateFastAttackAction` | action | `called`(boolean) |  |
| `AnimateHopAction` | action | `called`(boolean) |  |
| `AnimateJumpAction` | action | `called`(boolean) |  |
| `AnimateOrbAction` | action | `orbCount`(int) |  |
| `AnimateShakeAction` | action | `called`(boolean), `shakeDur`(float) |  |
| `AnimateSlowAttackAction` | action | `called`(boolean) |  |
| `ApplyPoisonOnRandomMonsterAction` | action | `startingDuration`(float) |  |
| `ApplyPowerAction` | action | `startingDuration`(float) |  |
| `ApplyPowerToRandomEnemyAction` | action | `isFast`(boolean) |  |
| `ApplyStasisAction` | action | `startingDuration`(float) |  |
| `ArmamentsAction` | action | `upgraded`(boolean) |  |
| `Art of War` | relic | `gainEnergyNext`(boolean), `firstTurn`(boolean), `setDescription`(String) |  |
| `Astrolabe` | relic | `cardsSelected`(boolean) |  |
| `Attack Burn` | power | `justApplied`(boolean) |  |
| `AwakenedOne` | monster | `form1`(boolean), `firstTurn`(boolean), `saidPower`(boolean), `fireTimer`(float), `animateParticles`(boolean) |  |
| `BanditBear` | monster | `maulDmg`(int), `lungeDmg`(int), `con_reduction`(int) |  |
| `BanditChild` | monster | `attackDmg`(int) |  |
| `BanditLeader` | monster | `slashDmg`(int), `agonizeDmg`(int), `weakAmount`(int) |  |
| `BetterDiscardPileToHandAction` | action | `numberOfCards`(int), `optional`(boolean), `newCost`(int), `setCost`(boolean) |  |
| `BetterDrawPileToHandAction` | action | `numberOfCards`(int), `optional`(boolean) |  |
| `BladeFuryAction` | action | `upgrade`(boolean) |  |
| `BlockPerNonAttackAction` | action | `blockPerCard`(int) |  |
| `BookOfStabbing` | monster | `stabDmg`(int), `bigStabDmg`(int), `stabCount`(int) |  |
| `Bottled Flame` | relic | `cardSelected`(boolean) |  |
| `Bottled Lightning` | relic | `cardSelected`(boolean) |  |
| `Bottled Tornado` | relic | `cardSelected`(boolean) |  |
| `BouncingFlaskAction` | action | `numTimes`(int), `amount`(int) |  |
| `BrillianceAction` | action | `freeToPlayOnce`(boolean), `energyOnUse`(int) |  |
| `BronzeAutomaton` | monster | `flailDmg`(int), `beamDmg`(int), `strAmt`(int), `blockAmt`(int), `numTurns`(int), `firstTurn`(boolean) |  |
| `BronzeOrb` | monster | `usedStasis`(boolean), `count`(int) |  |
| `BurnIncreaseAction` | action | `gotBurned`(boolean) |  |
| `Busted Crown` | relic | `setDescription`(String) |  |
| `Byrd` | monster | `peckDmg`(int), `peckCount`(int), `swoopDmg`(int), `flightAmt`(int), `firstMove`(boolean), `isFlying`(boolean) |  |
| `CalculatedGambleAction` | action | `startingDuration`(float), `isUpgraded`(boolean) |  |
| `Calling Bell` | relic | `cardsReceived`(boolean) |  |
| `Centurion` | monster | `slashDmg`(int), `furyDmg`(int), `furyHits`(int), `blockAmount`(int), `BLOCK_AMOUNT`(int), `A_17_BLOCK_AMOUNT`(int) |  |
| `Champ` | monster | `slashDmg`(int), `executeDmg`(int), `slapDmg`(int), `blockAmt`(int), `strAmt`(int), `forgeAmt`(int), `numTurns`(int), `forgeTimes`(int), `forgeThreshold`(int), `thresholdReached`(boolean), `firstTurn`(boolean), `getTaunt`(String), `getLimitBreak`(String), `getDeathQuote`(String) |  |
| `ChangeStanceAction` | action | `id`(String) |  |
| `ChangeStateAction` | action | `called`(boolean), `stateName`(String) |  |
| `ChannelAction` | action | `autoEvoke`(boolean) |  |
| `ChooseOneColorless` | action | `retrieveCard`(boolean) |  |
| `Chosen` | monster | `zapDmg`(int), `debilitateDmg`(int), `pokeDmg`(int), `firstTurn`(boolean), `usedHex`(boolean) |  |
| `ClarityAction` | action | `startingDuration`(float) |  |
| `CodexAction` | action | `retrieveCard`(boolean) |  |
| `Coffee Dripper` | relic | `setDescription`(String) |  |
| `CollectAction` | action | `freeToPlayOnce`(boolean), `upgraded`(boolean), `energyOnUse`(int) |  |
| `Combust` | power | `hpLoss`(int) |  |
| `ConditionalDrawAction` | action | `checkCondition`(boolean) |  |
| `ConjureBladeAction` | action | `freeToPlayOnce`(boolean), `energyOnUse`(int) |  |
| `ContemplateAction` | action | `upgraded`(boolean) |  |
| `CorruptHeart` | monster | `bloodHitCount`(int), `isFirstMove`(boolean), `moveCount`(int), `buffCount`(int) |  |
| `CrushJointsAction` | action | `magicNumber`(int) |  |
| `Cultist` | monster | `firstMove`(boolean), `saidPower`(boolean), `ritualAmount`(int), `talky`(boolean) |  |
| `Curl Up` | power | `triggered`(boolean) |  |
| `Cursed Key` | relic | `setDescription`(String) |  |
| `DEPRECATEDDamagePerCardAction` | action | `cardName`(String) |  |
| `DEPRECATEDEruptionAction` | action | `baseDamage`(int) |  |
| `DEPRECATEDExperiencedAction` | action | `blockPerCard`(int) |  |
| `DamageAction` | action | `goldAmount`(int), `skipWait`(boolean), `muteSfx`(boolean) |  |
| `DamageAllButOneEnemyAction` | action | `firstFrame`(boolean) |  |
| `DamageAllEnemiesAction` | action | `baseDamage`(int), `firstFrame`(boolean), `utilizeBaseDamage`(boolean) |  |
| `Dark` | orb | `vfxTimer`(float) |  |
| `DarkOrbEvokeAction` | action | `muteSfx`(boolean) |  |
| `Darkling` | monster | `chompDmg`(int), `nipDmg`(int), `firstMove`(boolean) |  |
| `Deca` | monster | `beamDmg`(int), `isAttacking`(boolean) |  |
| `DevaForm` | power | `energyGainAmount`(int) |  |
| `DiscardAction` | action | `isRandom`(boolean), `endTurn`(boolean) |  |
| `DiscoveryAction` | action | `retrieveCard`(boolean), `returnColorless`(boolean) |  |
| `DivinePunishmentAction` | action | `freeToPlayOnce`(boolean), `energyOnUse`(int) |  |
| `Dodecahedron` | relic | `setDescription`(String), `isActive`(boolean) |  |
| `DollysMirror` | relic | `cardSelected`(boolean) |  |
| `Donu` | monster | `beamDmg`(int), `isAttacking`(boolean) |  |
| `DoppelgangerAction` | action | `freeToPlayOnce`(boolean), `upgraded`(boolean), `energyOnUse`(int) |  |
| `Double Damage` | power | `justApplied`(boolean) |  |
| `Draw Reduction` | power | `justApplied`(boolean) |  |
| `DrawCardAction` | action | `shuffleCheck`(boolean), `clearDrawHistory`(boolean) |  |
| `DualWieldAction` | action | `dupeAmount`(int), `isDualWieldable`(boolean) |  |
| `Echo Form` | power | `cardsDoubledThisTurn`(int) |  |
| `Ectoplasm` | relic | `setDescription`(String) |  |
| `Empty Cage` | relic | `cardsSelected`(boolean) |  |
| `EmptyBodyAction` | action | `additionalDraw`(int) |  |
| `EmptyDeckShuffleAction` | action | `shuffled`(boolean), `vfxDone`(boolean), `count`(int) |  |
| `EnergyBlockAction` | action | `upg`(boolean) |  |
| `EnlightenmentAction` | action | `forCombat`(boolean) |  |
| `EscapePlanAction` | action | `blockGain`(int) |  |
| `EstablishmentPowerAction` | action | `discountAmount`(int) |  |
| `EvokeOrbAction` | action | `orbCount`(int) |  |
| `EvokeWithoutRemovingOrbAction` | action | `orbCount`(int) |  |
| `ExhaustAction` | action | `isRandom`(boolean), `anyNumber`(boolean), `canPickZero`(boolean) |  |
| `ExhaustAllNonAttackAction` | action | `startingDuration`(float) |  |
| `ExhaustSpecificCardAction` | action | `startingDuration`(float) |  |
| `Exploder` | monster | `turnCount`(int), `attackDmg`(int) |  |
| `FTLAction` | action | `cardPlayCount`(int) |  |
| `FastDrawCardAction` | action | `shuffleCheck`(boolean) |  |
| `FastShakeAction` | action | `called`(boolean), `shakeDur`(float) |  |
| `FeedAction` | action | `increaseHpAmount`(int) |  |
| `FiendFireAction` | action | `startingDuration`(float) |  |
| `FissionAction` | action | `upgraded`(boolean) |  |
| `Flight` | power | `storedAmount`(int), `calculateDamageTakenAmount`(float) |  |
| `ForeignInfluenceAction` | action | `retrieveCard`(boolean), `upgraded`(boolean) |  |
| `ForethoughtAction` | action | `chooseAny`(boolean) |  |
| `Frail` | power | `justApplied`(boolean) |  |
| `Frost` | orb | `hFlip1`(boolean), `hFlip2`(boolean), `vfxTimer`(float), `vfxIntervalMin`(float), `vfxIntervalMax`(float) |  |
| `FungiBeast` | monster | `biteDamage`(int), `strAmt`(int) |  |
| `Fusion Hammer` | relic | `setDescription`(String) |  |
| `FuzzyLouseDefensive` | monster | `isOpen`(boolean) |  |
| `FuzzyLouseNormal` | monster | `isOpen`(boolean), `biteDamage`(int) |  |
| `GainEnergyAction` | action | `energyGain`(int) |  |
| `GainEnergyAndEnableControlsAction` | action | `energyGain`(int) |  |
| `GainEnergyIfDiscardAction` | action | `energyGain`(int) |  |
| `Gambling Chip` | relic | `activated`(boolean) |  |
| `GamblingChipAction` | action | `notchip`(boolean) |  |
| `GiantHead` | monster | `startingDeathDmg`(int), `count`(int), `getTimeQuote`(String) |  |
| `GreedAction` | action | `increaseGold`(int) |  |
| `Gremlin Horn` | relic | `setDescription`(String) |  |
| `GremlinLeader` | monster | `strAmt`(int), `blockAmt`(int), `STAB_DMG`(int), `STAB_AMT`(int), `getEncourageQuote`(String), `numAliveGremlins`(int) |  |
| `GremlinNob` | monster | `bashDmg`(int), `rushDmg`(int), `usedBellow`(boolean), `canVuln`(boolean) |  |
| `GremlinThief` | monster | `thiefDamage`(int) |  |
| `GremlinTsundere` | monster | `blockAmt`(int), `bashDmg`(int) |  |
| `GremlinWizard` | monster | `currentCharge`(int) |  |
| `GrowthPower` | power | `skipFirst`(boolean) |  |
| `Happy Flower` | relic | `setDescription`(String) |  |
| `HeadStompAction` | action | `magicNumber`(int) |  |
| `Healer` | monster | `magicDmg`(int), `strAmt`(int), `healAmt`(int) |  |
| `Hexaghost` | monster | `searDmg`(int), `strengthenBlockAmt`(int), `strAmount`(int), `searBurnCount`(int), `fireTackleDmg`(int), `fireTackleCount`(int), `infernoDmg`(int), `infernoHits`(int), `activated`(boolean), `burnUpgraded`(boolean), `orbActiveCount`(int) |  |
| `HoveringKite` | relic | `triggeredThisTurn`(boolean) |  |
| `IceWallAction` | action | `perOrbAmt`(int) |  |
| `Impatience` | card | `shouldGlow`(boolean) |  |
| `IncreaseMaxHpAction` | action | `showEffect`(boolean), `increasePercent`(float) |  |
| `IncreaseMiscAction` | action | `miscIncrease`(int) |  |
| `Intangible` | power | `justApplied`(boolean) |  |
| `Invincible` | power | `maxAmt`(int) |  |
| `JawWorm` | monster | `bellowBlock`(int), `chompDmg`(int), `thrashDmg`(int), `thrashBlock`(int), `bellowStr`(int), `firstMove`(boolean), `hardMode`(boolean) |  |
| `JudgementAction` | action | `cutoff`(int) |  |
| `Lagavulin` | monster | `attackDmg`(int), `debuff`(int), `isOut`(boolean), `asleep`(boolean), `isOutTriggered`(boolean), `idleCount`(int), `debuffTurnCount`(int) |  |
| `Lantern` | relic | `firstTurn`(boolean), `setDescription`(String) |  |
| `Lightning` | orb | `vfxTimer`(float) |  |
| `LightningOrbEvokeAction` | action | `hitAll`(boolean) |  |
| `LightningOrbPassiveAction` | action | `hitAll`(boolean) |  |
| `Looter` | monster | `swipeDmg`(int), `lungeDmg`(int), `escapeDef`(int), `goldAmt`(int), `slashCount`(int), `stolenGold`(int) |  |
| `LoseEnergyAction` | action | `energyLoss`(int) |  |
| `MakeTempCardInDiscardAction` | action | `numCards`(int), `sameUUID`(boolean) |  |
| `MakeTempCardInDrawPileAction` | action | `randomSpot`(boolean), `autoPosition`(boolean), `toBottom`(boolean), `x`(float), `y`(float) |  |
| `MakeTempCardInHandAction` | action | `isOtherCardInCenter`(boolean), `sameUUID`(boolean) |  |
| `MalaiseAction` | action | `freeToPlayOnce`(boolean), `upgraded`(boolean), `energyOnUse`(int) |  |
| `Malleable` | power | `basePower`(int) | YES |
| `Maw` | monster | `slamDmg`(int), `nomDmg`(int), `roared`(boolean), `turnCount`(int), `strUp`(int), `terrifyDur`(int) |  |
| `MeditateAction` | action | `numberOfCards`(int), `optional`(boolean) |  |
| `Mugger` | monster | `swipeDmg`(int), `bigSwipeDmg`(int), `goldAmt`(int), `escapeDef`(int), `slashCount`(int), `stolenGold`(int) |  |
| `MulticastAction` | action | `freeToPlayOnce`(boolean), `energyOnUse`(int), `upgraded`(boolean) |  |
| `Necronomicon` | relic | `activated`(boolean) |  |
| `Nemesis` | monster | `fireDmg`(int), `scytheCooldown`(int), `fireTimer`(float), `firstMove`(boolean) |  |
| `NewQueueCardAction` | action | `randomTarget`(boolean), `immediateCard`(boolean), `autoplayCard`(boolean), `queueContains`(boolean), `queueContainsEndTurnCard`(boolean) |  |
| `NoBlockPower` | power | `justApplied`(boolean) |  |
| `NotStanceCheckAction` | action | `stanceToCheck`(String) |  |
| `Nunchaku` | relic | `setDescription`(String) |  |
| `OmniscienceAction` | action | `playAmt`(int) |  |
| `Orb Walker` | monster | `clawDmg`(int), `laserDmg`(int) |  |
| `Panache` | power | `damage`(int) | YES |
| `Pandora's Box` | relic | `count`(int), `calledTransform`(boolean) |  |
| `Plasma` | orb | `vfxTimer`(float), `vfxIntervalMin`(float), `vfxIntervalMax`(float) |  |
| `PlayTopCardAction` | action | `exhaustCards`(boolean) |  |
| `Pocketwatch` | relic | `firstTurn`(boolean) |  |
| `PreservedInsect` | relic | `MODIFIER_AMT`(float) |  |
| `PutOnBottomOfDeckAction` | action | `isRandom`(boolean) |  |
| `PutOnDeckAction` | action | `isRandom`(boolean) |  |
| `QueueCardAction` | action | `queueContains`(boolean) |  |
| `Rebound` | power | `justEvoked`(boolean) |  |
| `RechargingCore` | power | `turnTimer`(int) |  |
| `Red Skull` | relic | `isActive`(boolean) |  |
| `ReducePowerAction` | action | `powerID`(String) |  |
| `ReinforcedBodyAction` | action | `freeToPlayOnce`(boolean), `energyOnUse`(int) |  |
| `RelicAboveCreatureAction` | action | `used`(boolean) |  |
| `RemoveAllPowersAction` | action | `debuffsOnly`(boolean) |  |
| `RemoveSpecificPowerAction` | action | `powerToRemove`(String) |  |
| `ReprieveAction` | action | `focusIncrease`(int) |  |
| `Reptomancer` | monster | `daggersPerSpawn`(int), `firstMove`(boolean), `canSpawn`(boolean) |  |
| `Repulsor` | monster | `attackDmg`(int), `dazeAmt`(int) |  |
| `ReviveMonsterAction` | action | `healingEffect`(boolean) |  |
| `RipAndTearAction` | action | `numTimes`(int) |  |
| `Ritual` | power | `skipFirst`(boolean), `onPlayer`(boolean) |  |
| `RitualDaggerAction` | action | `increaseAmount`(int) |  |
| `Runic Capacitor` | relic | `firstTurn`(boolean) |  |
| `Runic Dome` | relic | `setDescription`(String) |  |
| `SFXAction` | action | `key`(String), `pitchVar`(float), `adjust`(boolean) |  |
| `SanctityAction` | action | `amtToDraw`(int) |  |
| `ScrapeAction` | action | `shuffleCheck`(boolean) |  |
| `ScryAction` | action | `startingDuration`(float) |  |
| `Sentry` | monster | `beamDmg`(int), `dazedAmt`(int), `firstMove`(boolean) |  |
| `Serpent` | monster | `tackleDmg`(int), `smashDmg`(int), `constrictDmg`(int), `A_2_tackleDmg`(int), `A_2_smashDmg`(int), `tackleDmgActual`(int), `smashDmgActual`(int) |  |
| `SetAnimationAction` | action | `called`(boolean), `animation`(String) |  |
| `SetDontTriggerAction` | action | `trigger`(boolean) |  |
| `SetMoveAction` | action | `theNextDamage`(int), `theNextName`(String), `theMultiplier`(int), `isMultiplier`(boolean) |  |
| `ShakeScreenAction` | action | `startDur`(float) |  |
| `Shelled Parasite` | monster | `fellDmg`(int), `doubleStrikeDmg`(int), `suckDmg`(int), `firstMove`(boolean) |  |
| `ShoutAction` | action | `msg`(String), `used`(boolean), `bubbleDuration`(float) |  |
| `ShowMoveNameAction` | action | `msg`(String) |  |
| `ShuffleAction` | action | `triggerRelics`(boolean) |  |
| `ShuffleAllAction` | action | `shuffled`(boolean), `vfxDone`(boolean), `count`(int) |  |
| `SkewerAction` | action | `freeToPlayOnce`(boolean), `damage`(int), `energyOnUse`(int) |  |
| `Skill Burn` | power | `justApplied`(boolean) |  |
| `SlaverBlue` | monster | `stabDmg`(int), `rakeDmg`(int), `weakAmt`(int) |  |
| `SlaverBoss` | monster | `woundCount`(int) |  |
| `SlaverRed` | monster | `stabDmg`(int), `scrapeDmg`(int), `VULN_AMT`(int), `usedEntangle`(boolean), `firstTurn`(boolean) |  |
| `SlaversCollar` | relic | `setDescription`(String) |  |
| `SlimeBoss` | monster | `tackleDmg`(int), `slamDmg`(int), `firstTurn`(boolean) |  |
| `SnakePlant` | monster | `rainBlowsDmg`(int) |  |
| `Snecko` | monster | `biteDmg`(int), `tailDmg`(int), `firstTurn`(boolean) |  |
| `Sozu` | relic | `setDescription`(String) |  |
| `SpawnMonsterAction` | action | `used`(boolean), `minion`(boolean), `targetSlot`(int), `useSmartPositioning`(boolean) |  |
| `SphericGuardian` | monster | `dmg`(int), `firstMove`(boolean), `secondMove`(boolean) |  |
| `SpikeSlime_L` | monster | `saveX`(float), `saveY`(float), `splitTriggered`(boolean) |  |
| `Spiker` | monster | `startingThorns`(int), `attackDmg`(int), `thornsCount`(int) |  |
| `SpireShield` | monster | `moveCount`(int) |  |
| `SpireSpear` | monster | `moveCount`(int), `skewerCount`(int) |  |
| `SpiritShieldAction` | action | `blockPerCard`(int) |  |
| `SpotWeaknessAction` | action | `damageIncrease`(int) |  |
| `StanceCheckAction` | action | `stanceToCheck`(String) |  |
| `SuicideAction` | action | `relicTrigger`(boolean) |  |
| `SummonGremlinAction` | action | `identifySlot`(int), `getSmartPosition`(int) |  |
| `SunderAction` | action | `energyGainAmt`(int) |  |
| `Sundial` | relic | `setDescription`(String) |  |
| `SwipeAction` | action | `skipWait`(boolean) |  |
| `SwordBoomerangAction` | action | `numTimes`(int) |  |
| `TalkAction` | action | `msg`(String), `used`(boolean), `bubbleDuration`(float), `player`(boolean) |  |
| `TempestAction` | action | `freeToPlayOnce`(boolean), `energyOnUse`(int), `upgraded`(boolean) |  |
| `Test 1` | relic | `setDescription`(String) |  |
| `Test 6` | relic | `hasEnoughGold`(boolean) |  |
| `TextAboveCreatureAction` | action | `used`(boolean), `msg`(String) |  |
| `TextCenteredAction` | action | `used`(boolean), `msg`(String) |  |
| `TheBomb` | power | `damage`(int) |  |
| `TheCollector` | monster | `rakeDmg`(int), `strAmt`(int), `blockAmt`(int), `megaDebuffAmt`(int), `turnsTaken`(int), `spawnX`(float), `fireTimer`(float), `ultUsed`(boolean), `initialSpawn`(boolean), `isMinionDead`(boolean) |  |
| `TheGuardian` | monster | `dmgThreshold`(int), `dmgThresholdIncrease`(int), `dmgTaken`(int), `fierceBashDamage`(int), `whirlwindDamage`(int), `twinSlamDamage`(int), `rollDamage`(int), `whirlwindCount`(int), `DEFENSIVE_BLOCK`(int), `blockAmount`(int), `thornsDamage`(int), `VENT_DEBUFF`(int), `isOpen`(boolean), `closeUpTriggered`(boolean) |  |
| `ThunderStrikeAction` | action | `numTimes`(int) |  |
| `TimeEater` | monster | `reverbDmg`(int), `headSlamDmg`(int), `usedHaste`(boolean), `firstTurn`(boolean) |  |
| `TimeMazePower` | power | `maxAmount`(int) |  |
| `TorchHead` | monster | `fireTimer`(float) |  |
| `TransformCardInHandAction` | action | `handIndex`(int) |  |
| `Transient` | monster | `count`(int), `startingDeathDmg`(int) |  |
| `TransmutationAction` | action | `freeToPlayOnce`(boolean), `upgraded`(boolean), `energyOnUse`(int) |  |
| `Unceasing Top` | relic | `canDraw`(boolean), `disabledUntilEndOfTurn`(boolean) |  |
| `UnlimboAction` | action | `exhaust`(boolean) |  |
| `VFXAction` | action | `startingDuration`(float), `isTopLevelEffect`(boolean) |  |
| `Velvet Choker` | relic | `setDescription`(String) |  |
| `Vulnerable` | power | `justApplied`(boolean) |  |
| `Weakened` | power | `justApplied`(boolean) |  |
| `WhirlwindAction` | action | `freeToPlayOnce`(boolean), `energyOnUse`(int) |  |
| `WrithingMass` | monster | `firstMove`(boolean), `usedMegaDebuff`(boolean), `normalDebuffAmt`(int) |  |

## RNG Dependencies

Entities that use specific RNG streams (critical for bit-for-bit replay).

- `AcidSlime_L` (monster): aiRng
- `AcidSlime_M` (monster): aiRng
- `AcidSlime_S` (monster): aiRng
- `Apology Slime` (monster): monsterHpRng, aiRng
- `ApplyPowerToRandomEnemyAction` (action): cardRandomRng
- `ApplyStasisAction` (action): cardRandomRng
- `Astrolabe` (relic): miscRng
- `AttackDamageRandomEnemyAction` (action): cardRandomRng
- `Bouncing Flask` (card): cardRandomRng
- `BouncingFlaskAction` (action): cardRandomRng
- `BronzeOrb` (monster): monsterHpRng
- `Byrd` (monster): aiRng
- `Confusion` (power): cardRandomRng
- `ContemplateAction` (action): cardRandomRng
- `CorruptHeart` (monster): aiRng
- `DEPRECATEDRandomStanceAction` (action): cardRandomRng
- `Dagger` (monster): monsterHpRng
- `DamageRandomEnemyAction` (action): cardRandomRng
- `Darkling` (monster): monsterHpRng, aiRng
- `DiscardAction` (action): cardRandomRng
- `DistilledChaos` (potion): cardRandomRng
- `EmptyDeckShuffleAction` (action): shuffleRng
- `ExhaustAction` (action): cardRandomRng
- `ForeignInfluenceAction` (action): cardRandomRng
- `FuzzyLouseDefensive` (monster): monsterHpRng
- `FuzzyLouseNormal` (monster): monsterHpRng
- `GainBlockRandomMonsterAction` (action): aiRng
- `GremlinLeader` (monster): aiRng
- `Havoc` (card): cardRandomRng
- `Hello` (power): cardRandomRng
- `Jack Of All Trades` (card): cardRandomRng
- `JawWorm` (monster): aiRng
- `LessonLearnedAction` (action): miscRng
- `LetFateDecide` (card): cardRandomRng
- `Looter` (monster): aiRng
- `MadnessAction` (action): cardRandomRng
- `Matryoshka` (relic): relicRng
- `Mayhem` (power): cardRandomRng
- `Mugger` (monster): aiRng
- `Mummified Hand` (relic): cardRandomRng
- `Nemesis` (monster): aiRng
- `Orb Walker` (monster): monsterHpRng
- `PutOnBottomOfDeckAction` (action): cardRandomRng
- `PutOnDeckAction` (action): cardRandomRng
- `RandomCardFromDiscardPileToHandAction` (action): cardRandomRng
- `RandomizeHandCostAction` (action): cardRandomRng
- `Reptomancer` (monster): monsterHpRng, aiRng
- `RipAndTearAction` (action): cardRandomRng
- `Shelled Parasite` (monster): aiRng
- `ShuffleAllAction` (action): shuffleRng
- `SlaverBoss` (monster): monsterHpRng
- `SoothingAura` (card): cardRandomRng
- `SpireShield` (monster): aiRng
- `SpireSpear` (monster): aiRng
- `SummonGremlinAction` (action): aiRng
- `SwordBoomerangAction` (action): cardRandomRng
- `TemperTantrum` (card): cardRandomRng
- `ThunderStrikeAction` (action): cardRandomRng
- `TimeEater` (monster): aiRng
- `Tiny House` (relic): miscRng
- `TorchHead` (monster): monsterHpRng
- `UseCardAction` (action): cardRandomRng
- `War Paint` (relic): miscRng
- `Whetstone` (relic): miscRng
- `WrithingMass` (monster): aiRng

## Ascension-Conditional Entities

- `AcidSlime_L` (monster): A2, A7, A17
- `AcidSlime_M` (monster): A2, A7, A17
- `AcidSlime_S` (monster): A2, A7, A17
- `AwakenedOne` (monster): A4, A9, A19
- `BanditBear` (monster): A2, A7, A17
- `BanditChild` (monster): A2, A7
- `BanditLeader` (monster): A2, A7, A17
- `BookOfStabbing` (monster): A3, A8, A18
- `BronzeAutomaton` (monster): A4, A9, A19
- `BronzeOrb` (monster): A9
- `Byrd` (monster): A2, A7, A17
- `Centurion` (monster): A2, A7, A17
- `Champ` (monster): A4, A9, A19
- `Chosen` (monster): A2, A7, A17
- `CorruptHeart` (monster): A4, A9, A19
- `Cultist` (monster): A2, A7, A17
- `Darkling` (monster): A2, A7, A17
- `Deca` (monster): A4, A9, A19
- `Donu` (monster): A4, A9, A19
- `Exploder` (monster): A2, A7
- `FungiBeast` (monster): A2, A7, A17
- `FuzzyLouseDefensive` (monster): A2, A7, A17
- `FuzzyLouseNormal` (monster): A2, A7, A17
- `GiantHead` (monster): A3, A8, A18
- `GremlinFat` (monster): A2, A7, A17
- `GremlinLeader` (monster): A3, A8, A18
- `GremlinNob` (monster): A3, A8, A18
- `GremlinThief` (monster): A2, A7
- `GremlinTsundere` (monster): A2, A7, A17
- `GremlinWarrior` (monster): A2, A7, A17
- `GremlinWizard` (monster): A2, A7, A17
- `Healer` (monster): A2, A7, A17
- `Hexaghost` (monster): A4, A9, A19
- `JawWorm` (monster): A2, A7, A17
- `Lagavulin` (monster): A3, A8, A18
- `Looter` (monster): A2, A7, A17
- `Maw` (monster): A2, A17
- `Mugger` (monster): A2, A7, A17
- `Nemesis` (monster): A3, A8, A18
- `Orb Walker` (monster): A2, A7, A17
- `Reptomancer` (monster): A3, A8, A18
- `Repulsor` (monster): A2, A7
- `Sentry` (monster): A3, A8, A18
- `Serpent` (monster): A2, A7, A17
- `Shelled Parasite` (monster): A2, A7, A17
- `SlaverBlue` (monster): A2, A7, A17
- `SlaverBoss` (monster): A3, A8, A18
- `SlaverRed` (monster): A2, A7, A17
- `SlimeBoss` (monster): A4, A9, A19
- `SnakePlant` (monster): A2, A7, A17
- `Snecko` (monster): A2, A7, A17
- `SphericGuardian` (monster): A2, A17
- `SpikeSlime_L` (monster): A2, A7, A17
- `SpikeSlime_M` (monster): A2, A7, A17
- `SpikeSlime_S` (monster): A2, A7
- `Spiker` (monster): A2, A7, A17
- `SpireShield` (monster): A3, A8, A18
- `SpireSpear` (monster): A3, A8, A18
- `TheCollector` (monster): A4, A9, A19
- `TheGuardian` (monster): A4, A9, A19
- `TimeEater` (monster): A4, A9
- `TorchHead` (monster): A9
- `Transient` (monster): A2, A17
- `WrithingMass` (monster): A2, A7

## Powers with Custom stackPower

These powers do MORE than just `amount += stackAmount` when stacked.

- `Collect`: modifies [], cap=True, removes_at_zero=False
- `Dexterity`: modifies [], cap=True, removes_at_zero=True
- `Energized`: modifies [], cap=True, removes_at_zero=False
- `EnergizedBlue`: modifies [], cap=True, removes_at_zero=False
- `Focus`: modifies [], cap=True, removes_at_zero=True
- `LikeWaterPower`: modifies [], cap=True, removes_at_zero=False
- `Malleable`: modifies ['basePower'], cap=False, removes_at_zero=False
- `Panache`: modifies ['damage'], cap=False, removes_at_zero=False
- `Plated Armor`: modifies [], cap=True, removes_at_zero=False
- `Shackled`: modifies [], cap=True, removes_at_zero=True
- `Strength`: modifies [], cap=True, removes_at_zero=True

## Relics with NO Own Hooks (Engine-Side Only)

- `AkabekoUnlock` (AkabekoUnlock) -- **0 refs (orphan!)**
- `ArtOfWarUnlock` (ArtOfWarUnlock) -- **0 refs (orphan!)**
- `Bloody Idol` (BloodyIdol) -- **0 refs (orphan!)**
- `BlueCandleUnlock` (BlueCandleUnlock) -- **0 refs (orphan!)**
- `Boot` (Boot) -- **0 refs (orphan!)**
- `Brimstone` (Brimstone) -- **0 refs (orphan!)**
- `Cables` (GoldPlatedCables) -- 4 engine refs
- `CablesUnlock` (CablesUnlock) -- **0 refs (orphan!)**
- `Calipers` (Calipers) -- **0 refs (orphan!)**
- `CeramicFishUnlock` (CeramicFishUnlock) -- **0 refs (orphan!)**
- `Chemical X` (ChemicalX) -- 24 engine refs
- `CloakClaspUnlock` (CloakClaspUnlock) -- **0 refs (orphan!)**
- `CourierUnlock` (CourierUnlock) -- **0 refs (orphan!)**
- `Damaru` (Damaru) -- **0 refs (orphan!)**
- `Dark Core` (DEPRECATED_DarkCore) -- **0 refs (orphan!)**
- `DataDiskUnlock` (DataDiskUnlock) -- **0 refs (orphan!)**
- `DeadBranchUnlock` (DeadBranchUnlock) -- **0 refs (orphan!)**
- `Discerning Monocle` (DiscerningMonocle) -- **0 refs (orphan!)**
- `Dream Catcher` (DreamCatcher) -- **0 refs (orphan!)**
- `DuvuDollUnlock` (DuvuDollUnlock) -- **0 refs (orphan!)**
- `EmotionChipUnlock` (EmotionChipUnlock) -- **0 refs (orphan!)**
- `Eternal Feather` (EternalFeather) -- **0 refs (orphan!)**
- `Frozen Eye` (FrozenEye) -- **0 refs (orphan!)**
- `Ginger` (Ginger) -- 2 engine refs
- `Golden Idol` (GoldenIdol) -- **0 refs (orphan!)**
- `GoldenEye` (GoldenEye) -- 2 engine refs
- `Ice Cream` (IceCream) -- **0 refs (orphan!)**
- `Juzu Bracelet` (JuzuBracelet) -- **0 refs (orphan!)**
- `Matryoshka` (Matryoshka) -- **0 refs (orphan!)**
- `MawBank` (MawBank) -- **0 refs (orphan!)**
- `MealTicket` (MealTicket) -- **0 refs (orphan!)**
- `Membership Card` (MembershipCard) -- **0 refs (orphan!)**
- `Mercury Hourglass` (MercuryHourglass) -- **0 refs (orphan!)**
- `Nloth's Gift` (NlothsGift) -- **0 refs (orphan!)**
- `NlothsMask` (NlothsMask) -- **0 refs (orphan!)**
- `Odd Mushroom` (OddMushroom) -- 3 engine refs
- `OmamoriUnlock` (OmamoriUnlock) -- **0 refs (orphan!)**
- `PandorasBoxUnlock` (PandorasBoxUnlock) -- **0 refs (orphan!)**
- `Paper Crane` (PaperCrane) -- 3 engine refs
- `Paper Frog` (PaperFrog) -- 3 engine refs
- `Peace Pipe` (PeacePipe) -- **0 refs (orphan!)**
- `Prayer Wheel` (PrayerWheel) -- 1 engine refs
- `PrayerWheelUnlock` (PrayerWheelUnlock) -- **0 refs (orphan!)**
- `Question Card` (QuestionCard) -- **0 refs (orphan!)**
- `Red Circlet` (RedCirclet) -- **0 refs (orphan!)**
- `Regal Pillow` (RegalPillow) -- **0 refs (orphan!)**
- `Runic Pyramid` (RunicPyramid) -- 2 engine refs
- `RunicCapacitorUnlock` (RunicCapacitorUnlock) -- **0 refs (orphan!)**
- `Shovel` (Shovel) -- 1 engine refs
- `ShovelUnlock` (ShovelUnlock) -- **0 refs (orphan!)**
- `Singing Bowl` (SingingBowl) -- 1 engine refs
- `SingingBowlUnlock` (SingingBowlUnlock) -- **0 refs (orphan!)**
- `Smiling Mask` (SmilingMask) -- 1 engine refs
- `SmilingMaskUnlock` (SmilingMaskUnlock) -- **0 refs (orphan!)**
- `Snake Skull` (SneckoSkull) -- 2 engine refs
- `Spirit Poop` (SpiritPoop) -- **0 refs (orphan!)**
- `SsserpentHead` (SsserpentHead) -- **0 refs (orphan!)**
- `Strange Spoon` (StrangeSpoon) -- 2 engine refs
- `StrikeDummy` (StrikeDummy) -- 1 engine refs
- `StrikeDummyUnlock` (StrikeDummyUnlock) -- **0 refs (orphan!)**
- `TeardropUnlock` (TeardropUnlock) -- **0 refs (orphan!)**
- `Test 1` (Test1) -- **0 refs (orphan!)**
- `The Courier` (Courier) -- 1 engine refs
- `TinyChestUnlock` (TinyChestUnlock) -- **0 refs (orphan!)**
- `Toy Ornithopter` (ToyOrnithopter) -- **0 refs (orphan!)**
- `TungstenRod` (TungstenRod) -- **0 refs (orphan!)**
- `Turnip` (Turnip) -- 3 engine refs
- `TurnipUnlock` (TurnipUnlock) -- **0 refs (orphan!)**
- `VirusUnlock` (VirusUnlock) -- **0 refs (orphan!)**
- `WarpedTongs` (WarpedTongs) -- **0 refs (orphan!)**
- `White Beast Statue` (WhiteBeast) -- **0 refs (orphan!)**
- `WingedGreaves` (WingBoots) -- **0 refs (orphan!)**
- `WristBlade` (WristBlade) -- **0 refs (orphan!)**
- `YangUnlock` (YangUnlock) -- **0 refs (orphan!)**

## Most Referenced Entities

- `Chemical X` (relic): 24 refs
- `Artifact` (power): 12 refs
- `No Draw` (power): 10 refs
- `MasterRealityPower` (power): 8 refs
- `Poison` (power): 6 refs
- `SacredBark` (relic): 5 refs
- `Sozu` (relic): 4 refs
- `Cables` (relic): 4 refs
- `Minion` (power): 4 refs
- `Strength` (power): 4 refs
- `Necronomicon` (relic): 4 refs
- `Champion Belt` (relic): 3 refs
- `Turnip` (relic): 3 refs
- `BackAttack` (power): 3 refs
- `Odd Mushroom` (relic): 3 refs
- `Paper Frog` (relic): 3 refs
- `Paper Crane` (relic): 3 refs
- `Snake Skull` (relic): 2 refs
- `Ginger` (relic): 2 refs
- `Runic Pyramid` (relic): 2 refs

## Most Complex Entities (by actions created)

- `AwakenedOne` (monster): 41 actions
- `Champ` (monster): 40 actions
- `CorruptHeart` (monster): 36 actions
- `TheGuardian` (monster): 36 actions
- `Hexaghost` (monster): 29 actions
- `Lagavulin` (monster): 27 actions
- `TimeEater` (monster): 25 actions
- `SphericGuardian` (monster): 24 actions
- `AcidSlime_L` (monster): 23 actions
- `SlimeBoss` (monster): 23 actions
- `Shelled Parasite` (monster): 22 actions
- `SpireShield` (monster): 22 actions
- `WrithingMass` (monster): 21 actions
- `BronzeAutomaton` (monster): 21 actions
- `TheCollector` (monster): 21 actions
- `Looter` (monster): 21 actions
- `FuzzyLouseDefensive` (monster): 21 actions
- `Darkling` (monster): 20 actions
- `GremlinLeader` (monster): 20 actions
- `Mugger` (monster): 20 actions
