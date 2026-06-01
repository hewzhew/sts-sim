# Defect Card Source Audit

Status: source-grounding ledger before Rust Defect card implementation.

Java sources:

- `D:/rust/cardcrawl/cards/blue/*.java`
- `D:/rust/cardcrawl/helpers/CardLibrary.java::addBlueCards()`

Rust state at audit time:

- no `src/content/cards/defect/` directory exists yet;
- `CardId` has no Defect card variants yet;
- `defect_pool_for_rarity` is still an empty stub;
- core orb primitives exist: `OrbId`, `OrbEntity`, `Action::ChannelOrb`, `Action::EvokeOrb`, `Action::IncreaseMaxOrb`, start/end orb hooks, and Focus-aware orb values.

## Counts

- Java files in `cards/blue`: `76`
- Cards registered by `CardLibrary.addBlueCards()`: `75`
- Source-present but not registered: `1`
- Not registered: `Impulse`

Do not add source-present/unregistered cards to reward pools unless Java `CardLibrary` does.
The registered card table follows Java `CardLibrary.addBlueCards()` order, not filesystem order.

## Immediate Implementation Order

1. Create `src/content/cards/defect/` and add only registered Blue cards.
2. Add `CardId` variants and `get_card_definition` dispatch in small batches.
3. Start with starter/basic cards: `Strike_Blue`, `Defend_Blue`, `Zap`, `Dualcast`.
4. Before broad card implementation, harden orb action support for multiple evoke, evoke-without-removing, random orb channeling, and max-orb decrease.
5. Keep each card file in the established `definition() + <card>_play(...)` shape.

## Existing Rust Orb Support

| Area | Status | Notes |
| --- | --- | --- |
| Orb IDs | partial | `Empty`, `Lightning`, `Dark`, `Frost`, `Plasma` exist. |
| Orb values | partial | Passive/evoke values and Focus refresh exist. |
| Channel | partial | `Action::ChannelOrb` channels or front-queues evoke+channel when full. Needs Java parity tests for multi-channel ordering. |
| Evoke | partial | `Action::EvokeOrb` evokes one front orb and removes it. Defect cards need multiple evoke and evoke-without-removing semantics. |
| Start/end hooks | partial | Start/end orb triggers exist, including Gold Plated Cables handling. |
| Random orb channeling | missing | Needed for `Chaos`. |
| Max orb decrease | missing | Needed for `Consume`; must evoke/drop slots exactly like Java. |
| Orb history | likely missing | `Blizzard` and `Thunder Strike` depend on combat orb-channel history. |

## Registered Card Table

| Java class | ID | Cost | Type | Rarity | Target | D/B/M | Flags | Tags | Evoke UI | Upgrade ops | Use dependencies | Orbs | Powers | Hooks | Notes |
| --- | --- | ---: | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |
| Aggregate | Aggregate | 1 | SKILL | UNCOMMON | SELF | 0/0/4 | - | - | - | upgradeName(), upgradeMagicNumber(-1), initializeDescription | AggregateEnergyAction | - | - | - | - |
| AllForOne | All For One | 2 | ATTACK | RARE | ENEMY | 10/0/0 | - | - | - | upgradeDamage(4), upgradeName() | AllCostToHandAction, DamageAction | - | - | - | - |
| Amplify | Amplify | 1 | SKILL | RARE | SELF | 0/0/1 | - | - | - | upgradeName(), upgradeMagicNumber(1), rawDescription, initializeDescription | ApplyPowerAction | - | AmplifyPower | - | - |
| AutoShields | Auto Shields | 1 | SKILL | UNCOMMON | SELF | 0/11/0 | - | - | - | upgradeName(), upgradeBlock(4) | GainBlockAction | - | - | - | - |
| BallLightning | Ball Lightning | 1 | ATTACK | COMMON | ENEMY | 7/0/1 | - | - | value, count=1 | upgradeName(), upgradeDamage(3) | ChannelAction, DamageAction | Lightning | - | - | - |
| Barrage | Barrage | 1 | ATTACK | COMMON | ENEMY | 4/0/0 | - | - | - | upgradeName(), upgradeDamage(2) | BarrageAction | - | - | - | - |
| BeamCell | Beam Cell | 0 | ATTACK | COMMON | ENEMY | 3/0/1 | - | - | - | upgradeDamage(1), upgradeMagicNumber(1), upgradeName() | ApplyPowerAction, DamageAction | - | VulnerablePower | - | - |
| BiasedCognition | Biased Cognition | 1 | POWER | RARE | SELF | 0/0/4 | - | - | - | upgradeName(), upgradeMagicNumber(1) | ApplyPowerAction | - | BiasPower, FocusPower | - | - |
| Blizzard | Blizzard | 1 | ATTACK | UNCOMMON | ALL_ENEMY | 0/0/2 | isMultiDamage | - | - | upgradeName(), upgradeMagicNumber(1) | DamageAllEnemiesAction, VFXAction | - | - | applyPowers, calculateCardDamage, onMoveToDiscard | - |
| BootSequence | BootSequence | 0 | SKILL | UNCOMMON | SELF | 0/10/0 | exhaust, isInnate | - | - | upgradeName(), upgradeBlock(3) | GainBlockAction | - | - | - | - |
| Buffer | Buffer | 2 | POWER | RARE | SELF | 0/0/1 | - | - | - | upgradeName(), upgradeMagicNumber(1), rawDescription, initializeDescription | ApplyPowerAction | - | BufferPower | - | - |
| Capacitor | Capacitor | 1 | POWER | UNCOMMON | SELF | 0/0/2 | - | - | - | upgradeName(), upgradeMagicNumber(1), rawDescription, initializeDescription | IncreaseMaxOrbAction | - | - | - | - |
| Chaos | Chaos | 1 | SKILL | UNCOMMON | SELF | 0/0/1 | - | - | value, count=1 | upgradeName(), upgradeMagicNumber(1), rawDescription, initializeDescription | ChannelAction | - | - | - | - |
| Chill | Chill | 0 | SKILL | UNCOMMON | SELF | 0/0/1 | exhaust | - | value, count=3 | upgradeName(), rawDescription, initializeDescription, isInnate=true | ChannelAction | Frost | - | - | - |
| Claw | Gash | 0 | ATTACK | COMMON | ENEMY | 3/0/2 | - | - | - | upgradeName(), upgradeDamage(2) | DamageAction, GashAction, VFXAction | - | - | - | - |
| ColdSnap | Cold Snap | 1 | ATTACK | COMMON | ENEMY | 6/0/1 | - | - | value, count=1 | upgradeName(), upgradeDamage(3) | ChannelAction, DamageAction | Frost | - | - | - |
| CompileDriver | Compile Driver | 1 | ATTACK | COMMON | ENEMY | 7/0/1 | - | - | - | upgradeName(), upgradeDamage(3) | CompileDriverAction, DamageAction | - | - | - | - |
| ConserveBattery | Conserve Battery | 1 | SKILL | COMMON | SELF | 0/7/0 | - | - | - | upgradeName(), upgradeBlock(3) | ApplyPowerAction, GainBlockAction | - | EnergizedBluePower | - | - |
| Consume | Consume | 2 | SKILL | UNCOMMON | SELF | 0/0/2 | - | - | - | upgradeName(), upgradeMagicNumber(1) | ApplyPowerAction, DecreaseMaxOrbAction | - | FocusPower | - | - |
| Coolheaded | Coolheaded | 1 | SKILL | COMMON | SELF | 0/0/1 | - | - | value, count=1 | upgradeName(), upgradeMagicNumber(1), rawDescription, initializeDescription | ChannelAction, DrawCardAction | Frost | - | - | - |
| CoreSurge | Core Surge | 1 | ATTACK | RARE | ENEMY | 11/0/1 | exhaust | - | - | upgradeName(), upgradeDamage(4) | ApplyPowerAction, DamageAction | - | ArtifactPower | - | - |
| CreativeAI | Creative AI | 3 | POWER | RARE | SELF | 0/0/1 | - | - | - | upgradeName(), upgradeBaseCost(2) | ApplyPowerAction | - | CreativeAIPower | - | - |
| Darkness | Darkness | 1 | SKILL | UNCOMMON | SELF | 0/0/1 | - | - | value, count=1 | upgradeName(), rawDescription, initializeDescription | ChannelAction, DarkImpulseAction | Dark | - | - | - |
| Defend_Blue | Defend_B | 1 | SKILL | BASIC | SELF | 0/5/0 | - | STARTER_DEFEND | - | upgradeName(), upgradeBlock(3) | GainBlockAction | - | - | - | use has Settings.isDebug branch; normal gameplay branch is authoritative |
| Defragment | Defragment | 1 | POWER | UNCOMMON | SELF | 0/0/1 | - | - | - | upgradeName(), upgradeMagicNumber(1) | ApplyPowerAction | - | FocusPower | - | - |
| DoomAndGloom | Doom and Gloom | 2 | ATTACK | UNCOMMON | ALL_ENEMY | 10/0/1 | isMultiDamage | - | value, count=1 | upgradeName(), upgradeDamage(4) | ChannelAction, DamageAllEnemiesAction, SFXAction, VFXAction | Dark | - | - | - |
| DoubleEnergy | Double Energy | 1 | SKILL | UNCOMMON | SELF | 0/0/0 | exhaust | - | - | upgradeName(), upgradeBaseCost(0) | DoubleEnergyAction | - | - | - | - |
| Dualcast | Dualcast | 1 | SKILL | BASIC | NONE | 0/0/0 | - | - | value | upgradeName(), upgradeBaseCost(0) | AnimateOrbAction, EvokeOrbAction, EvokeWithoutRemovingOrbAction | - | - | - | - |
| EchoForm | Echo Form | 3 | POWER | RARE | SELF | 0/0/0 | isEthereal | - | - | upgradeName(), rawDescription, initializeDescription, isEthereal=false | ApplyPowerAction | - | EchoPower | - | - |
| Electrodynamics | Electrodynamics | 2 | POWER | RARE | SELF | 0/0/2 | - | - | - | upgradeName(), upgradeMagicNumber(1) | ApplyPowerAction, ChannelAction | Lightning | ElectroPower | - | - |
| Fission | Fission | 0 | SKILL | RARE | NONE | 0/0/1 | exhaust | - | - | upgradeName(), rawDescription, initializeDescription | FissionAction | - | - | - | - |
| ForceField | Force Field | 4 | SKILL | UNCOMMON | SELF | 0/12/0 | - | - | - | upgradeName(), upgradeBlock(4) | GainBlockAction | - | - | - | - |
| FTL | FTL | 0 | ATTACK | UNCOMMON | ENEMY | 5/0/3 | - | - | - | upgradeName(), upgradeDamage(1), upgradeMagicNumber(1) | FTLAction | - | - | applyPowers, onMoveToDiscard | - |
| Fusion | Fusion | 2 | SKILL | UNCOMMON | SELF | 0/0/1 | - | - | - | upgradeName(), upgradeBaseCost(1) | ChannelAction | Plasma | - | - | - |
| GeneticAlgorithm | Genetic Algorithm | 1 | SKILL | UNCOMMON | SELF | 0/0/2 | exhaust | - | - | upgradeMagicNumber(1), upgradeName() | GainBlockAction, IncreaseMiscAction | - | - | applyPowers | misc starts 1, base block is initialized from misc |
| Glacier | Glacier | 2 | SKILL | UNCOMMON | SELF | 0/7/2 | - | - | value, count=2 | upgradeName(), upgradeBlock(3), rawDescription, initializeDescription | ChannelAction, GainBlockAction | Frost | - | - | - |
| GoForTheEyes | Go for the Eyes | 0 | ATTACK | COMMON | ENEMY | 3/0/1 | - | - | - | upgradeDamage(1), upgradeMagicNumber(1), upgradeName() | DamageAction, ForTheEyesAction | - | - | - | - |
| Heatsinks | Heatsinks | 1 | POWER | UNCOMMON | SELF | 0/0/1 | - | - | - | upgradeName(), upgradeMagicNumber(1), rawDescription, initializeDescription | ApplyPowerAction | - | HeatsinkPower | - | - |
| HelloWorld | Hello World | 1 | POWER | UNCOMMON | SELF | 0/0/0 | - | - | - | upgradeName(), rawDescription, initializeDescription, isInnate=true | ApplyPowerAction | - | HelloPower | - | - |
| Hologram | Hologram | 1 | SKILL | COMMON | SELF | 0/3/0 | exhaust | - | - | upgradeName(), upgradeBlock(2), rawDescription, initializeDescription, exhaust=false | BetterDiscardPileToHandAction, GainBlockAction | - | - | - | - |
| Hyperbeam | Hyperbeam | 2 | ATTACK | RARE | ALL_ENEMY | 26/0/3 | isMultiDamage | - | - | upgradeName(), upgradeDamage(8) | ApplyPowerAction, DamageAllEnemiesAction, SFXAction, VFXAction | - | FocusPower | - | - |
| Leap | Leap | 1 | SKILL | COMMON | SELF | 0/9/0 | - | - | - | upgradeName(), upgradeBlock(3) | GainBlockAction | - | - | - | - |
| LockOn | Lockon | 1 | ATTACK | UNCOMMON | ENEMY | 8/0/2 | - | - | - | upgradeName(), upgradeDamage(3), upgradeMagicNumber(1) | ApplyPowerAction, DamageAction | - | LockOnPower | - | - |
| Loop | Loop | 1 | POWER | UNCOMMON | SELF | 0/0/1 | - | - | - | upgradeName(), upgradeMagicNumber(1), rawDescription, initializeDescription | ApplyPowerAction | - | LoopPower | - | - |
| MachineLearning | Machine Learning | 1 | POWER | RARE | SELF | 0/0/1 | - | - | - | upgradeName(), rawDescription, initializeDescription, isInnate=true | ApplyPowerAction | - | DrawPower | - | - |
| Melter | Melter | 1 | ATTACK | UNCOMMON | ENEMY | 10/0/0 | - | - | - | upgradeName(), upgradeDamage(4) | DamageAction, RemoveAllBlockAction | - | - | - | - |
| MeteorStrike | Meteor Strike | 5 | ATTACK | RARE | ENEMY | 24/0/3 | - | STRIKE | - | upgradeName(), upgradeDamage(6) | ChannelAction, DamageAction, VFXAction, WaitAction | Plasma | - | - | - |
| MultiCast | Multi-Cast | -1 | SKILL | RARE | NONE | 0/0/0 | - | - | value | upgradeName(), rawDescription, initializeDescription | MulticastAction | - | - | - | - |
| Overclock | Steam Power | 0 | SKILL | UNCOMMON | SELF | 0/0/2 | - | - | - | upgradeName(), upgradeMagicNumber(1) | DrawCardAction, MakeTempCardInDiscardAction | - | - | - | - |
| Rainbow | Rainbow | 2 | SKILL | RARE | SELF | 0/0/0 | exhaust | - | value, count=3 | upgradeName(), rawDescription, initializeDescription, exhaust=false | ChannelAction, VFXAction | Dark, Frost, Lightning | - | - | - |
| Reboot | Reboot | 0 | SKILL | RARE | SELF | 0/0/4 | exhaust | - | - | upgradeName(), upgradeMagicNumber(2) | DrawCardAction, ShuffleAction, ShuffleAllAction | - | - | - | - |
| Rebound | Rebound | 1 | ATTACK | COMMON | ENEMY | 9/0/0 | - | - | - | upgradeName(), upgradeDamage(3) | ApplyPowerAction, DamageAction | - | ReboundPower | - | - |
| Recursion | Redo | 1 | SKILL | COMMON | SELF | 0/0/0 | - | - | - | upgradeName(), upgradeBaseCost(0) | RedoAction | - | - | - | - |
| Recycle | Recycle | 1 | SKILL | UNCOMMON | SELF | 0/0/0 | - | - | - | upgradeName(), upgradeBaseCost(0) | RecycleAction | - | - | - | - |
| ReinforcedBody | Reinforced Body | -1 | SKILL | UNCOMMON | SELF | 0/7/0 | - | - | - | upgradeName(), upgradeBlock(2) | ReinforcedBodyAction | - | - | - | - |
| Reprogram | Reprogram | 1 | SKILL | UNCOMMON | NONE | 0/0/1 | - | - | - | upgradeName(), upgradeMagicNumber(1) | ApplyPowerAction | - | DexterityPower, FocusPower, StrengthPower | - | - |
| RipAndTear | Rip and Tear | 1 | ATTACK | UNCOMMON | ALL_ENEMY | 7/0/2 | - | - | - | upgradeName(), upgradeDamage(2) | NewRipAndTearAction | - | - | - | - |
| Scrape | Scrape | 1 | ATTACK | UNCOMMON | ENEMY | 7/0/4 | - | - | - | upgradeDamage(3), upgradeName(), upgradeMagicNumber(1) | DamageAction, DrawCardAction, ScrapeFollowUpAction, VFXAction | - | - | - | - |
| Seek | Seek | 0 | SKILL | RARE | NONE | 0/0/1 | exhaust | - | - | upgradeName(), upgradeMagicNumber(1), rawDescription, initializeDescription | BetterDrawPileToHandAction | - | - | - | - |
| SelfRepair | Self Repair | 1 | POWER | UNCOMMON | SELF | 0/0/7 | - | HEALING | - | upgradeName(), upgradeMagicNumber(3) | ApplyPowerAction | - | RepairPower | - | - |
| Skim | Skim | 1 | SKILL | UNCOMMON | NONE | 0/0/3 | - | - | - | upgradeName(), upgradeMagicNumber(1) | DrawCardAction | - | - | - | - |
| Stack | Stack | 1 | SKILL | COMMON | SELF | 0/0/0 | - | - | - | upgradeName(), upgradeBlock(3), rawDescription, initializeDescription | GainBlockAction | - | - | applyPowers | - |
| StaticDischarge | Static Discharge | 1 | POWER | UNCOMMON | SELF | 0/0/1 | - | - | - | upgradeName(), upgradeMagicNumber(1), rawDescription, initializeDescription | ApplyPowerAction | - | StaticDischargePower | - | - |
| SteamBarrier | Steam | 0 | SKILL | COMMON | SELF | 0/6/0 | - | - | - | upgradeName(), upgradeBlock(2) | GainBlockAction, ModifyBlockAction | - | - | - | - |
| Storm | Storm | 1 | POWER | UNCOMMON | SELF | 0/0/1 | - | - | - | upgradeName(), rawDescription, initializeDescription, isInnate=true | ApplyPowerAction | - | StormPower | - | - |
| Streamline | Streamline | 2 | ATTACK | COMMON | ENEMY | 15/0/1 | - | - | - | upgradeName(), upgradeDamage(5), initializeDescription | DamageAction, ReduceCostAction | - | - | - | - |
| Strike_Blue | Strike_B | 1 | ATTACK | BASIC | ENEMY | 6/0/0 | - | STRIKE, STARTER_STRIKE | - | upgradeName(), upgradeDamage(3) | DamageAction, DamageAllEnemiesAction | - | - | - | use has Settings.isDebug branch; normal gameplay branch is authoritative |
| Sunder | Sunder | 3 | ATTACK | UNCOMMON | ENEMY | 24/0/0 | - | - | - | upgradeName(), upgradeDamage(8) | SunderAction, VFXAction, WaitAction | - | - | - | - |
| SweepingBeam | Sweeping Beam | 1 | ATTACK | COMMON | ALL_ENEMY | 6/0/1 | isMultiDamage | - | - | upgradeName(), upgradeDamage(3) | DamageAllEnemiesAction, DrawCardAction, SFXAction, VFXAction | - | - | - | - |
| Tempest | Tempest | -1 | SKILL | UNCOMMON | SELF | 0/0/0 | exhaust | - | value, count=3 | upgradeName(), rawDescription, initializeDescription | TempestAction | - | - | - | - |
| ThunderStrike | Thunder Strike | 3 | ATTACK | RARE | ALL_ENEMY | 7/0/0 | - | STRIKE | - | upgradeName(), upgradeDamage(2) | NewThunderStrikeAction | - | - | applyPowers, calculateCardDamage, onMoveToDiscard | - |
| Turbo | Turbo | 0 | SKILL | COMMON | SELF | 0/0/2 | - | - | - | upgradeName(), upgradeMagicNumber(1), rawDescription, initializeDescription | GainEnergyAction, MakeTempCardInDiscardAction | - | - | - | - |
| Equilibrium | Undo | 2 | SKILL | UNCOMMON | SELF | 0/13/1 | - | - | - | upgradeName(), upgradeBlock(3) | ApplyPowerAction, GainBlockAction | - | EquilibriumPower | - | - |
| WhiteNoise | White Noise | 1 | SKILL | UNCOMMON | NONE | 0/0/0 | exhaust | - | - | upgradeName(), upgradeBaseCost(0) | MakeTempCardInHandAction | - | - | - | - |
| Zap | Zap | 1 | SKILL | BASIC | SELF | 0/0/1 | - | - | value, count=1 | upgradeName(), upgradeBaseCost(0) | ChannelAction | Lightning | - | - | - |

## Source-Present But Not Registered

| Java class | ID | Reason |
| --- | --- | --- |
| Impulse | Impulse | Present in `cards/blue`, absent from `CardLibrary.addBlueCards()`. Treat as non-pool/deprecated until proven otherwise. |

## Action Dependency Inventory

| Java action | Rust status |
| --- | --- |
| `AggregateEnergyAction` | needs Defect action implementation/audit |
| `AllCostToHandAction` | needs Defect action implementation/audit |
| `AnimateOrbAction` | presentation no-op; real behavior is later evoke action |
| `ApplyPowerAction` | generic |
| `BarrageAction` | needs Defect action implementation/audit |
| `BetterDiscardPileToHandAction` | needs Defect action implementation/audit |
| `BetterDrawPileToHandAction` | needs Defect action implementation/audit |
| `ChannelAction` | partial: Action::ChannelOrb exists |
| `CompileDriverAction` | needs Defect action implementation/audit |
| `DamageAction` | generic |
| `DamageAllEnemiesAction` | generic |
| `DarkImpulseAction` | needs Defect action implementation/audit |
| `DecreaseMaxOrbAction` | needs Defect action implementation/audit |
| `DoubleEnergyAction` | needs Defect action implementation/audit |
| `DrawCardAction` | generic |
| `EvokeOrbAction` | partial: Action::EvokeOrb exists; Java class semantics need parity tests |
| `EvokeWithoutRemovingOrbAction` | needs Defect action implementation/audit |
| `FTLAction` | needs Defect action implementation/audit |
| `FissionAction` | needs Defect action implementation/audit |
| `ForTheEyesAction` | needs Defect action implementation/audit |
| `GainBlockAction` | generic |
| `GainEnergyAction` | generic |
| `GashAction` | needs Defect action implementation/audit |
| `IncreaseMaxOrbAction` | generic: Action::IncreaseMaxOrb exists |
| `IncreaseMiscAction` | needs Defect action implementation/audit |
| `MakeTempCardInDiscardAction` | generic card-copy support exists |
| `MakeTempCardInHandAction` | generic card-copy support exists |
| `ModifyBlockAction` | needs Defect action implementation/audit |
| `MulticastAction` | needs Defect action implementation/audit |
| `NewRipAndTearAction` | needs Defect action implementation/audit |
| `NewThunderStrikeAction` | needs Defect action implementation/audit |
| `RecycleAction` | needs Defect action implementation/audit |
| `RedoAction` | needs Defect action implementation/audit |
| `ReduceCostAction` | needs Defect action implementation/audit |
| `ReinforcedBodyAction` | needs Defect action implementation/audit |
| `RemoveAllBlockAction` | generic |
| `SFXAction` | presentation no-op for Rust simulation |
| `ScrapeFollowUpAction` | needs Defect action implementation/audit |
| `ShuffleAction` | generic shuffle support exists |
| `ShuffleAllAction` | generic shuffle support exists |
| `SunderAction` | needs Defect action implementation/audit |
| `TempestAction` | needs Defect action implementation/audit |
| `VFXAction` | presentation no-op for Rust simulation |
| `WaitAction` | presentation/timing no-op unless action ordering proves otherwise |

## Orb Dependency Inventory

| Orb | Rust status |
| --- | --- |
| `Dark` | `OrbId::Dark` exists; card-specific channel/evoke ordering still needs tests. |
| `Frost` | `OrbId::Frost` exists; card-specific channel/evoke ordering still needs tests. |
| `Lightning` | `OrbId::Lightning` exists; card-specific channel/evoke ordering still needs tests. |
| `Plasma` | `OrbId::Plasma` exists; card-specific channel/evoke ordering still needs tests. |

## Power Dependency Inventory

| Java power | Rust status |
| --- | --- |
| `AmplifyPower` | missing or not audited for Defect card behavior |
| `ArtifactPower` | PowerId::Artifact exists; shared Artifact interception exists |
| `BiasPower` | missing or not audited for Defect card behavior |
| `BufferPower` | PowerId::Buffer exists; runtime hooks need card-specific audit |
| `CreativeAIPower` | missing or not audited for Defect card behavior |
| `DexterityPower` | PowerId::Dexterity exists; shared block modifier exists |
| `DrawPower` | missing or not audited for Defect card behavior |
| `EchoPower` | missing or not audited for Defect card behavior |
| `ElectroPower` | PowerId::Electro exists; lightning all-enemy hook exists |
| `EnergizedBluePower` | PowerId::Energized exists as shared next-turn energy |
| `EquilibriumPower` | PowerId::Equilibrium exists for retain behavior |
| `FocusPower` | PowerId::Focus exists; orb focus refresh exists |
| `HeatsinkPower` | missing or not audited for Defect card behavior |
| `HelloPower` | missing or not audited for Defect card behavior |
| `LockOnPower` | missing or not audited for Defect card behavior |
| `LoopPower` | missing or not audited for Defect card behavior |
| `ReboundPower` | missing or not audited for Defect card behavior |
| `RepairPower` | missing or not audited for Defect card behavior |
| `StaticDischargePower` | missing or not audited for Defect card behavior |
| `StormPower` | missing or not audited for Defect card behavior |
| `StrengthPower` | PowerId::Strength exists; shared damage modifier exists |
| `VulnerablePower` | PowerId::Vulnerable exists; shared debuff behavior exists |

## Early Risk Notes

- `Dualcast`, `Multi-Cast`, `Fission`, `Recursion`, and `Consume` should not be implemented as card-local hacks; they require reusable orb actions.
- `Blizzard` and `Thunder Strike` require combat orb-channel history, not only current orb slots.
- `Genetic Algorithm`, `Claw`, `Streamline`, and `Steam Barrier` mutate card instance fields and need UUID/all-instances/cost/block mutation parity.
- `Hologram`, `Seek`, `Recycle`, and `Scrape` depend on choice/draw/discard mechanics from the high-risk backlog.
- `Echo Form`, `Amplify`, `Heatsinks`, `Hello World`, `Creative AI`, `Loop`, `Static Discharge`, and `Storm` require real power hooks before their cards are meaningful.

## Verification

Regenerate this file with:

```powershell
python tools/audit_defect_card_source.py
```

