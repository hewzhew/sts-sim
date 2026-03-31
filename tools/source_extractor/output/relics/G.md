# Relics: G

8 relics

## GamblingChip
File: `relics\GamblingChip.java`

### atBattleStartPreDraw()

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStartPreDraw() {
        this.activated = false;
    }
```

</details>

### makeCopy()

**Creates:**
- `GamblingChip` — `new GamblingChip()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new GamblingChip();
    }
```

</details>

## Ginger
File: `relics\Ginger.java`

### makeCopy()

**Creates:**
- `Ginger` — `new Ginger()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Ginger();
    }
```

</details>

## Girya
File: `relics\Girya.java`

### atBattleStart()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new StrengthPower(AbstractDungeon.player, this.counter), this.counter)`
- `StrengthPower` — `new StrengthPower(AbstractDungeon.player, this.counter)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

**Queue insertion:**
- [TOP] `this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new StrengthPower(AbstractDungeon.player, this.counter), this.count`
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        if (this.counter != 0) {
            this.flash();
            this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new StrengthPower(AbstractDungeon.player, this.counter), this.counter));
            this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `Girya` — `new Girya()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Girya();
    }
```

</details>

## GoldPlatedCables
File: `relics\GoldPlatedCables.java`

### makeCopy()

**Creates:**
- `GoldPlatedCables` — `new GoldPlatedCables()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new GoldPlatedCables();
    }
```

</details>

## GoldenEye
File: `relics\GoldenEye.java`

### makeCopy()

**Creates:**
- `GoldenEye` — `new GoldenEye()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new GoldenEye();
    }
```

</details>

## GoldenIdol
File: `relics\GoldenIdol.java`

### makeCopy()

**Creates:**
- `GoldenIdol` — `new GoldenIdol()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new GoldenIdol();
    }
```

</details>

## GremlinHorn
File: `relics\GremlinHorn.java`

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

### onMonsterDeath(AbstractMonster m)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(m, this)`
- `GainEnergyAction` — `new GainEnergyAction(1)`
- `DrawCardAction` — `new DrawCardAction(AbstractDungeon.player, 1)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(m, this))`
- [BOT] `this.addToBot(new GainEnergyAction(1))`
- [BOT] `this.addToBot(new DrawCardAction(AbstractDungeon.player, 1))`

<details><summary>Full body</summary>

```java
@Override
    public void onMonsterDeath(AbstractMonster m) {
        if (m.currentHealth == 0 && !AbstractDungeon.getMonsters().areMonstersBasicallyDead()) {
            this.flash();
            this.addToBot(new RelicAboveCreatureAction(m, this));
            this.addToBot(new GainEnergyAction(1));
            this.addToBot(new DrawCardAction(AbstractDungeon.player, 1));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `GremlinHorn` — `new GremlinHorn()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new GremlinHorn();
    }
```

</details>

## GremlinMask
File: `relics\GremlinMask.java`

### atBattleStart()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new WeakPower(AbstractDungeon.player, 1, false), 1)`
- `WeakPower` — `new WeakPower(AbstractDungeon.player, 1, false)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new WeakPower(AbstractDungeon.player, 1, false), 1))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new WeakPower(AbstractDungeon.player, 1, false), 1));
    }
```

</details>

### makeCopy()

**Creates:**
- `GremlinMask` — `new GremlinMask()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new GremlinMask();
    }
```

</details>

