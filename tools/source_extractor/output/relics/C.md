# Relics: C

17 relics

## Calipers
File: `relics\Calipers.java`

### makeCopy()

**Creates:**
- `Calipers` — `new Calipers()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Calipers();
    }
```

</details>

## CallingBell
File: `relics\CallingBell.java`

### onEquip()

**Creates:**
- `CardGroup` — `new CardGroup(CardGroup.CardGroupType.UNSPECIFIED)`
- `CurseOfTheBell` — `new CurseOfTheBell()`

**Queue insertion:**
- [BOT] `group.addToBottom(((AbstractCard)bellCurse).makeCopy())`

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        this.cardsReceived = false;
        CardGroup group = new CardGroup(CardGroup.CardGroupType.UNSPECIFIED);
        CurseOfTheBell bellCurse = new CurseOfTheBell();
        UnlockTracker.markCardAsSeen(bellCurse.cardID);
        group.addToBottom(((AbstractCard)bellCurse).makeCopy());
        AbstractDungeon.gridSelectScreen.openConfirmationGrid(group, this.DESCRIPTIONS[1]);
        CardCrawlGame.sound.playA("BELL", MathUtils.random(-0.2f, -0.3f));
    }
```

</details>

### makeCopy()

**Creates:**
- `CallingBell` — `new CallingBell()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new CallingBell();
    }
```

</details>

## CaptainsWheel
File: `relics\CaptainsWheel.java`

### atBattleStart()

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.counter = 0;
    }
```

</details>

### onVictory()

<details><summary>Full body</summary>

```java
@Override
    public void onVictory() {
        this.counter = -1;
        this.grayscale = false;
    }
```

</details>

### makeCopy()

**Creates:**
- `CaptainsWheel` — `new CaptainsWheel()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new CaptainsWheel();
    }
```

</details>

## Cauldron
File: `relics\Cauldron.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        for (int i = 0; i < 5; ++i) {
            AbstractDungeon.getCurrRoom().addPotionToRewards(PotionHelper.getRandomPotion());
        }
        AbstractDungeon.combatRewardScreen.open(this.DESCRIPTIONS[1]);
        AbstractDungeon.getCurrRoom().rewardPopOutTimer = 0.0f;
        int remove = -1;
        for (int i = 0; i < AbstractDungeon.combatRewardScreen.rewards.size(); ++i) {
            if (AbstractDungeon.combatRewardScreen.rewards.get((int)i).type != RewardItem.RewardType.CARD) continue;
            remove = i;
            break;
        }
        if (remove != -1) {
            AbstractDungeon.combatRewardScreen.rewards.remove(remove);
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `Cauldron` — `new Cauldron()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Cauldron();
    }
```

</details>

## CentennialPuzzle
File: `relics\CentennialPuzzle.java`

### atPreBattle()

<details><summary>Full body</summary>

```java
@Override
    public void atPreBattle() {
        usedThisCombat = false;
        this.pulse = true;
        this.beginPulse();
    }
```

</details>

### wasHPLost(int damageAmount)

**Creates:**
- `DrawCardAction` — `new DrawCardAction(AbstractDungeon.player, 3)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

**Queue insertion:**
- [TOP] `this.addToTop(new DrawCardAction(AbstractDungeon.player, 3))`
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

<details><summary>Full body</summary>

```java
@Override
    public void wasHPLost(int damageAmount) {
        if (damageAmount > 0 && AbstractDungeon.getCurrRoom().phase == AbstractRoom.RoomPhase.COMBAT && !usedThisCombat) {
            this.flash();
            this.pulse = false;
            this.addToTop(new DrawCardAction(AbstractDungeon.player, 3));
            this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
            usedThisCombat = true;
            this.grayscale = true;
        }
    }
```

</details>

### justEnteredRoom(AbstractRoom room)

<details><summary>Full body</summary>

```java
@Override
    public void justEnteredRoom(AbstractRoom room) {
        this.grayscale = false;
    }
```

</details>

### onVictory()

<details><summary>Full body</summary>

```java
@Override
    public void onVictory() {
        this.pulse = false;
    }
```

</details>

### makeCopy()

**Creates:**
- `CentennialPuzzle` — `new CentennialPuzzle()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new CentennialPuzzle();
    }
```

</details>

## CeramicFish
File: `relics\CeramicFish.java`

### use()

<details><summary>Full body</summary>

```java
public void use() {
        this.flash();
        --this.counter;
        if (this.counter == 0) {
            this.setCounter(0);
        } else {
            this.description = this.DESCRIPTIONS[1];
        }
    }
```

</details>

### onObtainCard(AbstractCard c)

<details><summary>Full body</summary>

```java
@Override
    public void onObtainCard(AbstractCard c) {
        AbstractDungeon.player.gainGold(9);
    }
```

</details>

### makeCopy()

**Creates:**
- `CeramicFish` — `new CeramicFish()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new CeramicFish();
    }
```

</details>

## ChampionsBelt
File: `relics\ChampionsBelt.java`

### onTrigger(AbstractCreature target)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `ApplyPowerAction` — `new ApplyPowerAction(target, AbstractDungeon.player, new WeakPower(target, 1, false), 1)`
- `WeakPower` — `new WeakPower(target, 1, false)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new ApplyPowerAction(target, AbstractDungeon.player, new WeakPower(target, 1, false), 1))`

<details><summary>Full body</summary>

```java
@Override
    public void onTrigger(AbstractCreature target) {
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new ApplyPowerAction(target, AbstractDungeon.player, new WeakPower(target, 1, false), 1));
    }
```

</details>

### makeCopy()

**Creates:**
- `ChampionsBelt` — `new ChampionsBelt()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new ChampionsBelt();
    }
```

</details>

## CharonsAshes
File: `relics\CharonsAshes.java`

### onExhaust(AbstractCard card)

**Creates:**
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(3, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.FIRE)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(mo, this)`

**Queue insertion:**
- [TOP] `this.addToTop(new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(3, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.F`
- [TOP] `this.addToTop(new RelicAboveCreatureAction(mo, this))`

<details><summary>Full body</summary>

```java
@Override
    public void onExhaust(AbstractCard card) {
        this.flash();
        this.addToTop(new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(3, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.FIRE));
        for (AbstractMonster mo : AbstractDungeon.getCurrRoom().monsters.monsters) {
            if (mo.isDead) continue;
            this.addToTop(new RelicAboveCreatureAction(mo, this));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `CharonsAshes` — `new CharonsAshes()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new CharonsAshes();
    }
```

</details>

## ChemicalX
File: `relics\ChemicalX.java`

### makeCopy()

**Creates:**
- `ChemicalX` — `new ChemicalX()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new ChemicalX();
    }
```

</details>

## Circlet
File: `relics\Circlet.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        this.flash();
    }
```

</details>

### onUnequip()

<details><summary>Full body</summary>

```java
@Override
    public void onUnequip() {
    }
```

</details>

### makeCopy()

**Creates:**
- `Circlet` — `new Circlet()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Circlet();
    }
```

</details>

## CloakClasp
File: `relics\CloakClasp.java`

### onPlayerEndTurn()

**Creates:**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)AbstractDungeon.player, null, AbstractDungeon.player.hand.group.size() * 1)`

**Queue insertion:**
- [BOT] `this.addToBot(new GainBlockAction((AbstractCreature)AbstractDungeon.player, null, AbstractDungeon.player.hand.group.size() * 1))`

<details><summary>Full body</summary>

```java
@Override
    public void onPlayerEndTurn() {
        if (!AbstractDungeon.player.hand.group.isEmpty()) {
            this.flash();
            this.addToBot(new GainBlockAction((AbstractCreature)AbstractDungeon.player, null, AbstractDungeon.player.hand.group.size() * 1));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `CloakClasp` — `new CloakClasp()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new CloakClasp();
    }
```

</details>

## ClockworkSouvenir
File: `relics\ClockworkSouvenir.java`

### atBattleStart()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new ArtifactPower(AbstractDungeon.player, 1), 1)`
- `ArtifactPower` — `new ArtifactPower(AbstractDungeon.player, 1)`

**Queue insertion:**
- [TOP] `this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new ArtifactPower(AbstractDungeon.player, 1), 1))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new ArtifactPower(AbstractDungeon.player, 1), 1));
    }
```

</details>

### makeCopy()

**Creates:**
- `ClockworkSouvenir` — `new ClockworkSouvenir()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new ClockworkSouvenir();
    }
```

</details>

## CoffeeDripper
File: `relics\CoffeeDripper.java`

### updateDescription(AbstractPlayer.PlayerClass c)

**Creates:**
- `PowerTip` — `new PowerTip(this.name, this.description)`

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription(AbstractPlayer.PlayerClass c) {
        this.description = this.setDescription(c);
        this.tips.clear();
        this.tips.add(new PowerTip(this.name, this.description));
        this.initializeTips();
    }
```

</details>

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        ++AbstractDungeon.player.energy.energyMaster;
    }
```

</details>

### onUnequip()

<details><summary>Full body</summary>

```java
@Override
    public void onUnequip() {
        --AbstractDungeon.player.energy.energyMaster;
    }
```

</details>

### makeCopy()

**Creates:**
- `CoffeeDripper` — `new CoffeeDripper()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new CoffeeDripper();
    }
```

</details>

## Courier
File: `relics\Courier.java`

### onEnterRoom(AbstractRoom room)

<details><summary>Full body</summary>

```java
@Override
    public void onEnterRoom(AbstractRoom room) {
        if (room instanceof ShopRoom) {
            this.flash();
            this.pulse = true;
        } else {
            this.pulse = false;
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `Courier` — `new Courier()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Courier();
    }
```

</details>

## CrackedCore
File: `relics\CrackedCore.java`

### atPreBattle()

**Creates:**
- `Lightning` — `new Lightning()`

<details><summary>Full body</summary>

```java
@Override
    public void atPreBattle() {
        AbstractDungeon.player.channelOrb(new Lightning());
    }
```

</details>

### makeCopy()

**Creates:**
- `CrackedCore` — `new CrackedCore()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new CrackedCore();
    }
```

</details>

## CultistMask
File: `relics\CultistMask.java`

### atBattleStart()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `SFXAction` — `new SFXAction("VO_CULTIST_1A")`
- `TalkAction` — `new TalkAction(true, this.DESCRIPTIONS[1], 1.0f, 2.0f)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new SFXAction("VO_CULTIST_1A"))`
- [BOT] `this.addToBot(new TalkAction(true, this.DESCRIPTIONS[1], 1.0f, 2.0f))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new SFXAction("VO_CULTIST_1A"));
        this.addToBot(new TalkAction(true, this.DESCRIPTIONS[1], 1.0f, 2.0f));
    }
```

</details>

### makeCopy()

**Creates:**
- `CultistMask` — `new CultistMask()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new CultistMask();
    }
```

</details>

## CursedKey
File: `relics\CursedKey.java`

### justEnteredRoom(AbstractRoom room)

<details><summary>Full body</summary>

```java
@Override
    public void justEnteredRoom(AbstractRoom room) {
        if (room instanceof TreasureRoom) {
            this.flash();
            this.pulse = true;
        } else {
            this.pulse = false;
        }
    }
```

</details>

### onChestOpen(boolean bossChest)

**Creates:**
- `ShowCardAndObtainEffect` — `new ShowCardAndObtainEffect(AbstractDungeon.returnRandomCurse(), Settings.WIDTH / 2, Settings.HEIGHT / 2)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

**Queue insertion:**
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

<details><summary>Full body</summary>

```java
@Override
    public void onChestOpen(boolean bossChest) {
        if (!bossChest) {
            AbstractDungeon.topLevelEffects.add(new ShowCardAndObtainEffect(AbstractDungeon.returnRandomCurse(), Settings.WIDTH / 2, Settings.HEIGHT / 2));
            this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        }
    }
```

</details>

### updateDescription(AbstractPlayer.PlayerClass c)

**Creates:**
- `PowerTip` — `new PowerTip(this.name, this.description)`

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription(AbstractPlayer.PlayerClass c) {
        this.description = this.setDescription(c);
        this.tips.clear();
        this.tips.add(new PowerTip(this.name, this.description));
        this.initializeTips();
    }
```

</details>

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        ++AbstractDungeon.player.energy.energyMaster;
    }
```

</details>

### onUnequip()

<details><summary>Full body</summary>

```java
@Override
    public void onUnequip() {
        --AbstractDungeon.player.energy.energyMaster;
    }
```

</details>

### makeCopy()

**Creates:**
- `CursedKey` — `new CursedKey()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new CursedKey();
    }
```

</details>

