# Relics: S

20 relics

## SacredBark
File: `relics\SacredBark.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        for (AbstractPotion p : AbstractDungeon.player.potions) {
            p.initializeData();
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `SacredBark` — `new SacredBark()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new SacredBark();
    }
```

</details>

## SelfFormingClay
File: `relics\SelfFormingClay.java`

### wasHPLost(int damageAmount)

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new NextTurnBlockPower(AbstractDungeon.player, 3, this.name), 3)`
- `NextTurnBlockPower` — `new NextTurnBlockPower(AbstractDungeon.player, 3, this.name)`

**Queue insertion:**
- [TOP] `this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new NextTurnBlockPower(AbstractDungeon.player, 3, this.name), 3))`

<details><summary>Full body</summary>

```java
@Override
    public void wasHPLost(int damageAmount) {
        if (AbstractDungeon.getCurrRoom().phase == AbstractRoom.RoomPhase.COMBAT && damageAmount > 0) {
            this.flash();
            this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new NextTurnBlockPower(AbstractDungeon.player, 3, this.name), 3));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `SelfFormingClay` — `new SelfFormingClay()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new SelfFormingClay();
    }
```

</details>

## Shovel
File: `relics\Shovel.java`

### makeCopy()

**Creates:**
- `Shovel` — `new Shovel()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Shovel();
    }
```

</details>

## Shuriken
File: `relics\Shuriken.java`

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new StrengthPower(AbstractDungeon.player, 1), 1)`
- `StrengthPower` — `new StrengthPower(AbstractDungeon.player, 1)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new StrengthPower(AbstractDungeon.player, 1), 1))`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.ATTACK) {
            ++this.counter;
            if (this.counter % 3 == 0) {
                this.counter = 0;
                this.flash();
                this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
                this.addToBot(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new StrengthPower(AbstractDungeon.player, 1), 1));
            }
        }
    }
```

</details>

### onVictory()

<details><summary>Full body</summary>

```java
@Override
    public void onVictory() {
        this.counter = -1;
    }
```

</details>

### makeCopy()

**Creates:**
- `Shuriken` — `new Shuriken()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Shuriken();
    }
```

</details>

## SingingBowl
File: `relics\SingingBowl.java`

### makeCopy()

**Creates:**
- `SingingBowl` — `new SingingBowl()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new SingingBowl();
    }
```

</details>

## SlaversCollar
File: `relics\SlaversCollar.java`

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

### onVictory()

<details><summary>Full body</summary>

```java
@Override
    public void onVictory() {
        if (this.pulse) {
            --AbstractDungeon.player.energy.energyMaster;
            this.stopPulse();
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `SlaversCollar` — `new SlaversCollar()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new SlaversCollar();
    }
```

</details>

## Sling
File: `relics\Sling.java`

### atBattleStart()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new StrengthPower(AbstractDungeon.player, 2), 2)`
- `StrengthPower` — `new StrengthPower(AbstractDungeon.player, 2)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

**Queue insertion:**
- [TOP] `this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new StrengthPower(AbstractDungeon.player, 2), 2))`
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        if (AbstractDungeon.getCurrRoom().eliteTrigger) {
            this.flash();
            this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new StrengthPower(AbstractDungeon.player, 2), 2));
            this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `Sling` — `new Sling()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Sling();
    }
```

</details>

## SmilingMask
File: `relics\SmilingMask.java`

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
- `SmilingMask` — `new SmilingMask()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new SmilingMask();
    }
```

</details>

## SnakeRing
File: `relics\SnakeRing.java`

### atBattleStart()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `DrawCardAction` — `new DrawCardAction(AbstractDungeon.player, 2)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new DrawCardAction(AbstractDungeon.player, 2))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new DrawCardAction(AbstractDungeon.player, 2));
    }
```

</details>

### makeCopy()

**Creates:**
- `SnakeRing` — `new SnakeRing()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new SnakeRing();
    }
```

</details>

## SneckoEye
File: `relics\SneckoEye.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        AbstractDungeon.player.masterHandSize += 2;
    }
```

</details>

### onUnequip()

<details><summary>Full body</summary>

```java
@Override
    public void onUnequip() {
        AbstractDungeon.player.masterHandSize -= 2;
    }
```

</details>

### atPreBattle()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new ConfusionPower(AbstractDungeon.player))`
- `ConfusionPower` — `new ConfusionPower(AbstractDungeon.player)`

**Queue insertion:**
- [BOT] `this.addToBot(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new ConfusionPower(AbstractDungeon.player)))`

<details><summary>Full body</summary>

```java
@Override
    public void atPreBattle() {
        this.flash();
        this.addToBot(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new ConfusionPower(AbstractDungeon.player)));
    }
```

</details>

### makeCopy()

**Creates:**
- `SneckoEye` — `new SneckoEye()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new SneckoEye();
    }
```

</details>

## SneckoSkull
File: `relics\SneckoSkull.java`

### makeCopy()

**Creates:**
- `SneckoSkull` — `new SneckoSkull()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new SneckoSkull();
    }
```

</details>

## Sozu
File: `relics\Sozu.java`

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
- `Sozu` — `new Sozu()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Sozu();
    }
```

</details>

## SpiritPoop
File: `relics\SpiritPoop.java`

### makeCopy()

**Creates:**
- `SpiritPoop` — `new SpiritPoop()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new SpiritPoop();
    }
```

</details>

## SsserpentHead
File: `relics\SsserpentHead.java`

### onEnterRoom(AbstractRoom room)

<details><summary>Full body</summary>

```java
@Override
    public void onEnterRoom(AbstractRoom room) {
        if (room instanceof EventRoom) {
            this.flash();
            AbstractDungeon.player.gainGold(50);
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `SsserpentHead` — `new SsserpentHead()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new SsserpentHead();
    }
```

</details>

## StoneCalendar
File: `relics\StoneCalendar.java`

### atBattleStart()

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.counter = 0;
    }
```

</details>

### onPlayerEndTurn()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(52, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.BLUNT_HEAVY)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(52, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.`

<details><summary>Full body</summary>

```java
@Override
    public void onPlayerEndTurn() {
        if (this.counter == 7) {
            this.flash();
            this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
            this.addToBot(new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(52, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.BLUNT_HEAVY));
            this.stopPulse();
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
        this.counter = -1;
        this.stopPulse();
    }
```

</details>

### makeCopy()

**Creates:**
- `StoneCalendar` — `new StoneCalendar()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new StoneCalendar();
    }
```

</details>

## StrangeSpoon
File: `relics\StrangeSpoon.java`

### makeCopy()

**Creates:**
- `StrangeSpoon` — `new StrangeSpoon()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new StrangeSpoon();
    }
```

</details>

## Strawberry
File: `relics\Strawberry.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        AbstractDungeon.player.increaseMaxHp(7, true);
    }
```

</details>

### makeCopy()

**Creates:**
- `Strawberry` — `new Strawberry()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Strawberry();
    }
```

</details>

## StrikeDummy
File: `relics\StrikeDummy.java`

### makeCopy()

**Creates:**
- `StrikeDummy` — `new StrikeDummy()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new StrikeDummy();
    }
```

</details>

## Sundial
File: `relics\Sundial.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        this.counter = 0;
    }
```

</details>

### onShuffle()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `GainEnergyAction` — `new GainEnergyAction(2)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new GainEnergyAction(2))`

<details><summary>Full body</summary>

```java
@Override
    public void onShuffle() {
        ++this.counter;
        if (this.counter == 3) {
            this.counter = 0;
            this.flash();
            this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
            this.addToBot(new GainEnergyAction(2));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `Sundial` — `new Sundial()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Sundial();
    }
```

</details>

## SymbioticVirus
File: `relics\SymbioticVirus.java`

### atPreBattle()

**Creates:**
- `Dark` — `new Dark()`

<details><summary>Full body</summary>

```java
@Override
    public void atPreBattle() {
        AbstractDungeon.player.channelOrb(new Dark());
    }
```

</details>

### makeCopy()

**Creates:**
- `SymbioticVirus` — `new SymbioticVirus()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new SymbioticVirus();
    }
```

</details>

