# Relics: T

19 relics

## TeardropLocket
File: `relics\TeardropLocket.java`

### atBattleStart()

**Creates:**
- `ChangeStanceAction` — `new ChangeStanceAction("Calm")`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

**Queue insertion:**
- [TOP] `this.addToTop(new ChangeStanceAction("Calm"))`
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToTop(new ChangeStanceAction("Calm"));
        this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
    }
```

</details>

### makeCopy()

**Creates:**
- `TeardropLocket` — `new TeardropLocket()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new TeardropLocket();
    }
```

</details>

## Test1
File: `relics\Test1.java`

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

### makeCopy()

**Creates:**
- `Test1` — `new Test1()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Test1();
    }
```

</details>

## Test3
File: `relics\Test3.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
    }
```

</details>

### makeCopy()

**Creates:**
- `Test3` — `new Test3()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Test3();
    }
```

</details>

## Test4
File: `relics\Test4.java`

### atBattleStart()

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
    }
```

</details>

### makeCopy()

**Creates:**
- `Test4` — `new Test4()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Test4();
    }
```

</details>

## Test5
File: `relics\Test5.java`

### onEquip()

**Creates:**
- `None` — `new ArrayList<AbstractCard>()`
- `ShowCardBrieflyEffect` — `new ShowCardBrieflyEffect(((AbstractCard)upgradableCards.get(0)).makeStatEquivalentCopy())`
- `UpgradeShineEffect` — `new UpgradeShineEffect((float)Settings.WIDTH / 2.0f, (float)Settings.HEIGHT / 2.0f)`
- `ShowCardBrieflyEffect` — `new ShowCardBrieflyEffect(((AbstractCard)upgradableCards.get(0)).makeStatEquivalentCopy(), (float)Settings.WIDTH / 2.0f - AbstractCard.IMG_WIDTH / 2.0f - 20.0f * Settings.scale, (float)Settings.HEIGHT...`
- `ShowCardBrieflyEffect` — `new ShowCardBrieflyEffect(((AbstractCard)upgradableCards.get(1)).makeStatEquivalentCopy(), (float)Settings.WIDTH / 2.0f + AbstractCard.IMG_WIDTH / 2.0f + 20.0f * Settings.scale, (float)Settings.HEIGHT...`
- `UpgradeShineEffect` — `new UpgradeShineEffect((float)Settings.WIDTH / 2.0f, (float)Settings.HEIGHT / 2.0f)`

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        ArrayList<AbstractCard> upgradableCards = new ArrayList<AbstractCard>();
        for (AbstractCard c : AbstractDungeon.player.masterDeck.group) {
            if (!c.canUpgrade() || c.type != AbstractCard.CardType.SKILL) continue;
            upgradableCards.add(c);
        }
        Collections.shuffle(upgradableCards);
        if (!upgradableCards.isEmpty()) {
            if (upgradableCards.size() == 1) {
                ((AbstractCard)upgradableCards.get(0)).upgrade();
                AbstractDungeon.player.bottledCardUpgradeCheck((AbstractCard)upgradableCards.get(0));
                AbstractDungeon.topLevelEffects.add(new ShowCardBrieflyEffect(((AbstractCard)upgradableCards.get(0)).makeStatEquivalentCopy()));
                AbstractDungeon.topLevelEffects.add(new UpgradeShineEffect((float)Settings.WIDTH / 2.0f, (float)Settings.HEIGHT / 2.0f));
            } else {
                ((AbstractCard)upgradableCards.get(0)).upgrade();
                ((AbstractCard)upgradableCards.get(1)).upgrade();
                AbstractDungeon.player.bottledCardUpgradeCheck((AbstractCard)upgradableCards.get(0));
                AbstractDungeon.player.bottledCardUpgradeCheck((AbstractCard)upgradableCards.get(1));
                AbstractDungeon.topLevelEffects.add(new ShowCardBrieflyEffect(((AbstractCard)upgradableCards.get(0)).makeStatEquivalentCopy(), (float)Settings.WIDTH / 2.0f - AbstractCard.IMG_WIDTH / 2.0f - 20.0f * Settings.scale, (float)Settings.HEIGHT / 2.0f));
                AbstractDungeon.topLevelEffects.add(new ShowCardBrieflyEffect(((AbstractCard)upgradableCards.get(1)).makeStatEquivalentCopy(), (float)Settings.WIDTH / 2.0f + AbstractCard.IMG_WIDTH / 2.0f + 20.0f * Settings.scale, (float)Settings.HEIGHT / 2.0f));
                AbstractDungeon.topLevelEffects.add(new UpgradeShineEffect((float)Settings.WIDTH / 2.0f, (float)Settings.HEIGHT / 2.0f));
            }
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `Test5` — `new Test5()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Test5();
    }
```

</details>

## Test6
File: `relics\Test6.java`

### onPlayerEndTurn()

**Creates:**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)AbstractDungeon.player, AbstractDungeon.player, 3 * (AbstractDungeon.player.gold / 100))`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

**Queue insertion:**
- [TOP] `this.addToTop(new GainBlockAction((AbstractCreature)AbstractDungeon.player, AbstractDungeon.player, 3 * (AbstractDungeon.player.gold / 100)))`
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

<details><summary>Full body</summary>

```java
@Override
    public void onPlayerEndTurn() {
        if (this.hasEnoughGold()) {
            this.flash();
            this.pulse = false;
            this.addToTop(new GainBlockAction((AbstractCreature)AbstractDungeon.player, AbstractDungeon.player, 3 * (AbstractDungeon.player.gold / 100)));
            this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        }
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
- `Test6` — `new Test6()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Test6();
    }
```

</details>

## TheSpecimen
File: `relics\TheSpecimen.java`

### onMonsterDeath(AbstractMonster m)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(m, this)`
- `ApplyPowerToRandomEnemyAction` — `new ApplyPowerToRandomEnemyAction(AbstractDungeon.player, new PoisonPower(null, AbstractDungeon.player, amount), amount, false, AbstractGameAction.AttackEffect.POISON)`
- `PoisonPower` — `new PoisonPower(null, AbstractDungeon.player, amount)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(m, this))`
- [BOT] `this.addToBot(new ApplyPowerToRandomEnemyAction(AbstractDungeon.player, new PoisonPower(null, AbstractDungeon.player, amount), amount, false, Abstract`

<details><summary>Full body</summary>

```java
@Override
    public void onMonsterDeath(AbstractMonster m) {
        if (m.hasPower("Poison")) {
            int amount = m.getPower((String)"Poison").amount;
            if (!AbstractDungeon.getMonsters().areMonstersBasicallyDead()) {
                this.flash();
                this.addToBot(new RelicAboveCreatureAction(m, this));
                this.addToBot(new ApplyPowerToRandomEnemyAction(AbstractDungeon.player, new PoisonPower(null, AbstractDungeon.player, amount), amount, false, AbstractGameAction.AttackEffect.POISON));
            } else {
                logger.info("no target for the specimen");
            }
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `TheSpecimen` — `new TheSpecimen()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new TheSpecimen();
    }
```

</details>

## ThreadAndNeedle
File: `relics\ThreadAndNeedle.java`

### atBattleStart()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new PlatedArmorPower(AbstractDungeon.player, 4), 4)`
- `PlatedArmorPower` — `new PlatedArmorPower(AbstractDungeon.player, 4)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

**Queue insertion:**
- [TOP] `this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new PlatedArmorPower(AbstractDungeon.player, 4), 4))`
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new PlatedArmorPower(AbstractDungeon.player, 4), 4));
        this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
    }
```

</details>

### makeCopy()

**Creates:**
- `ThreadAndNeedle` — `new ThreadAndNeedle()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new ThreadAndNeedle();
    }
```

</details>

## Tingsha
File: `relics\Tingsha.java`

### onManualDiscard()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `DamageRandomEnemyAction` — `new DamageRandomEnemyAction(new DamageInfo(AbstractDungeon.player, 3, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.FIRE)`
- `DamageInfo` — `new DamageInfo(AbstractDungeon.player, 3, DamageInfo.DamageType.THORNS)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new DamageRandomEnemyAction(new DamageInfo(AbstractDungeon.player, 3, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.FIR`

<details><summary>Full body</summary>

```java
@Override
    public void onManualDiscard() {
        this.flash();
        CardCrawlGame.sound.play("TINGSHA");
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new DamageRandomEnemyAction(new DamageInfo(AbstractDungeon.player, 3, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.FIRE));
    }
```

</details>

### makeCopy()

**Creates:**
- `Tingsha` — `new Tingsha()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Tingsha();
    }
```

</details>

## TinyChest
File: `relics\TinyChest.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        this.counter = 0;
    }
```

</details>

### makeCopy()

**Creates:**
- `TinyChest` — `new TinyChest()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new TinyChest();
    }
```

</details>

## TinyHouse
File: `relics\TinyHouse.java`

### onEquip()

**Creates:**
- `None` — `new ArrayList<AbstractCard>()`
- `Random` — `new Random(AbstractDungeon.miscRng.randomLong())`
- `ShowCardBrieflyEffect` — `new ShowCardBrieflyEffect(((AbstractCard)upgradableCards.get(0)).makeStatEquivalentCopy())`
- `UpgradeShineEffect` — `new UpgradeShineEffect((float)Settings.WIDTH / 2.0f, (float)Settings.HEIGHT / 2.0f)`
- `ShowCardBrieflyEffect` — `new ShowCardBrieflyEffect(((AbstractCard)upgradableCards.get(0)).makeStatEquivalentCopy(), (float)Settings.WIDTH / 2.0f, (float)Settings.HEIGHT / 2.0f)`
- `UpgradeShineEffect` — `new UpgradeShineEffect((float)Settings.WIDTH / 2.0f, (float)Settings.HEIGHT / 2.0f)`

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        ArrayList<AbstractCard> upgradableCards = new ArrayList<AbstractCard>();
        for (AbstractCard c : AbstractDungeon.player.masterDeck.group) {
            if (!c.canUpgrade()) continue;
            upgradableCards.add(c);
        }
        Collections.shuffle(upgradableCards, new Random(AbstractDungeon.miscRng.randomLong()));
        if (!upgradableCards.isEmpty()) {
            if (upgradableCards.size() == 1) {
                ((AbstractCard)upgradableCards.get(0)).upgrade();
                AbstractDungeon.player.bottledCardUpgradeCheck((AbstractCard)upgradableCards.get(0));
                AbstractDungeon.topLevelEffects.add(new ShowCardBrieflyEffect(((AbstractCard)upgradableCards.get(0)).makeStatEquivalentCopy()));
                AbstractDungeon.topLevelEffects.add(new UpgradeShineEffect((float)Settings.WIDTH / 2.0f, (float)Settings.HEIGHT / 2.0f));
            } else {
                ((AbstractCard)upgradableCards.get(0)).upgrade();
                AbstractDungeon.player.bottledCardUpgradeCheck((AbstractCard)upgradableCards.get(0));
                AbstractDungeon.player.bottledCardUpgradeCheck((AbstractCard)upgradableCards.get(1));
                AbstractDungeon.topLevelEffects.add(new ShowCardBrieflyEffect(((AbstractCard)upgradableCards.get(0)).makeStatEquivalentCopy(), (float)Settings.WIDTH / 2.0f, (float)Settings.HEIGHT / 2.0f));
                AbstractDungeon.topLevelEffects.add(new UpgradeShineEffect((float)Settings.WIDTH / 2.0f, (float)Settings.HEIGHT / 2.0f));
            }
        }
        AbstractDungeon.player.increaseMaxHp(5, true);
        AbstractDungeon.getCurrRoom().addGoldToRewards(50);
        AbstractDungeon.getCurrRoom().addPotionToRewards(PotionHelper.getRandomPotion(AbstractDungeon.miscRng));
        AbstractDungeon.combatRewardScreen.open(this.DESCRIPTIONS[3]);
        AbstractDungeon.getCurrRoom().rewardPopOutTimer = 0.0f;
    }
```

</details>

### makeCopy()

**Creates:**
- `TinyHouse` — `new TinyHouse()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new TinyHouse();
    }
```

</details>

## Toolbox
File: `relics\Toolbox.java`

### atBattleStartPreDraw()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `ChooseOneColorless` — `new ChooseOneColorless()`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new ChooseOneColorless())`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStartPreDraw() {
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new ChooseOneColorless());
    }
```

</details>

### makeCopy()

**Creates:**
- `Toolbox` — `new Toolbox()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Toolbox();
    }
```

</details>

## Torii
File: `relics\Torii.java`

### onAttacked(DamageInfo info, int damageAmount)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

<details><summary>Full body</summary>

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (info.owner != null && info.type != DamageInfo.DamageType.HP_LOSS && info.type != DamageInfo.DamageType.THORNS && damageAmount > 1 && damageAmount <= 5) {
            this.flash();
            this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
            return 1;
        }
        return damageAmount;
    }
```

</details>

### makeCopy()

**Creates:**
- `Torii` — `new Torii()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Torii();
    }
```

</details>

## ToughBandages
File: `relics\ToughBandages.java`

### onManualDiscard()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `GainBlockAction` — `new GainBlockAction(AbstractDungeon.player, AbstractDungeon.player, 3, true)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new GainBlockAction(AbstractDungeon.player, AbstractDungeon.player, 3, true))`

<details><summary>Full body</summary>

```java
@Override
    public void onManualDiscard() {
        this.flash();
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new GainBlockAction(AbstractDungeon.player, AbstractDungeon.player, 3, true));
    }
```

</details>

### makeCopy()

**Creates:**
- `ToughBandages` — `new ToughBandages()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new ToughBandages();
    }
```

</details>

## ToxicEgg2
File: `relics\ToxicEgg2.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        for (RewardItem reward : AbstractDungeon.combatRewardScreen.rewards) {
            if (reward.cards == null) continue;
            for (AbstractCard c : reward.cards) {
                this.onPreviewObtainCard(c);
            }
        }
    }
```

</details>

### onObtainCard(AbstractCard c)

<details><summary>Full body</summary>

```java
@Override
    public void onObtainCard(AbstractCard c) {
        if (c.type == AbstractCard.CardType.SKILL && c.canUpgrade() && !c.upgraded) {
            c.upgrade();
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `ToxicEgg2` — `new ToxicEgg2()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new ToxicEgg2();
    }
```

</details>

## ToyOrnithopter
File: `relics\ToyOrnithopter.java`

### makeCopy()

**Creates:**
- `ToyOrnithopter` — `new ToyOrnithopter()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new ToyOrnithopter();
    }
```

</details>

## TungstenRod
File: `relics\TungstenRod.java`

### makeCopy()

**Creates:**
- `TungstenRod` — `new TungstenRod()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new TungstenRod();
    }
```

</details>

## Turnip
File: `relics\Turnip.java`

### makeCopy()

**Creates:**
- `Turnip` — `new Turnip()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Turnip();
    }
```

</details>

## TwistedFunnel
File: `relics\TwistedFunnel.java`

### atBattleStart()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(m, AbstractDungeon.player, new PoisonPower(m, AbstractDungeon.player, 4), 4)`
- `PoisonPower` — `new PoisonPower(m, AbstractDungeon.player, 4)`

**Queue insertion:**
- [BOT] `this.addToBot(new ApplyPowerAction(m, AbstractDungeon.player, new PoisonPower(m, AbstractDungeon.player, 4), 4))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        for (AbstractMonster m : AbstractDungeon.getMonsters().monsters) {
            if (m.isDead || m.isDying) continue;
            this.addToBot(new ApplyPowerAction(m, AbstractDungeon.player, new PoisonPower(m, AbstractDungeon.player, 4), 4));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `TwistedFunnel` — `new TwistedFunnel()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new TwistedFunnel();
    }
```

</details>

