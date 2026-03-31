# Relics: H

5 relics

## HandDrill
File: `relics\HandDrill.java`

### onBlockBroken(AbstractCreature m)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(m, this)`
- `ApplyPowerAction` — `new ApplyPowerAction(m, AbstractDungeon.player, new VulnerablePower(m, 2, false), 2)`
- `VulnerablePower` — `new VulnerablePower(m, 2, false)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(m, this))`
- [BOT] `this.addToBot(new ApplyPowerAction(m, AbstractDungeon.player, new VulnerablePower(m, 2, false), 2))`

<details><summary>Full body</summary>

```java
@Override
    public void onBlockBroken(AbstractCreature m) {
        this.flash();
        this.addToBot(new RelicAboveCreatureAction(m, this));
        this.addToBot(new ApplyPowerAction(m, AbstractDungeon.player, new VulnerablePower(m, 2, false), 2));
    }
```

</details>

### makeCopy()

**Creates:**
- `HandDrill` — `new HandDrill()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new HandDrill();
    }
```

</details>

## HappyFlower
File: `relics\HappyFlower.java`

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
        this.counter = 0;
    }
```

</details>

### makeCopy()

**Creates:**
- `HappyFlower` — `new HappyFlower()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new HappyFlower();
    }
```

</details>

## HolyWater
File: `relics\HolyWater.java`

### atBattleStartPreDraw()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `MakeTempCardInHandAction` — `new MakeTempCardInHandAction((AbstractCard)new Miracle(), 3, false)`
- `Miracle` — `new Miracle()`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new MakeTempCardInHandAction((AbstractCard)new Miracle(), 3, false))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStartPreDraw() {
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new MakeTempCardInHandAction((AbstractCard)new Miracle(), 3, false));
    }
```

</details>

### makeCopy()

**Creates:**
- `HolyWater` — `new HolyWater()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new HolyWater();
    }
```

</details>

## HornCleat
File: `relics\HornCleat.java`

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
- `HornCleat` — `new HornCleat()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new HornCleat();
    }
```

</details>

## HoveringKite
File: `relics\HoveringKite.java`

### onManualDiscard()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `GainEnergyAction` — `new GainEnergyAction(1)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new GainEnergyAction(1))`

<details><summary>Full body</summary>

```java
@Override
    public void onManualDiscard() {
        if (!this.triggeredThisTurn) {
            this.triggeredThisTurn = true;
            this.flash();
            this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
            this.addToBot(new GainEnergyAction(1));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `HoveringKite` — `new HoveringKite()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new HoveringKite();
    }
```

</details>

