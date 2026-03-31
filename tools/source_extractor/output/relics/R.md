# Relics: R

9 relics

## RedCirclet
File: `relics\RedCirclet.java`

### makeCopy()

**Creates:**
- `RedCirclet` — `new RedCirclet()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new RedCirclet();
    }
```

</details>

## RedMask
File: `relics\RedMask.java`

### atBattleStart()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(mo, this)`
- `ApplyPowerAction` — `new ApplyPowerAction((AbstractCreature)mo, (AbstractCreature)AbstractDungeon.player, (AbstractPower)new WeakPower(mo, 1, false), 1, true)`
- `WeakPower` — `new WeakPower(mo, 1, false)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(mo, this))`
- [BOT] `this.addToBot(new ApplyPowerAction((AbstractCreature)mo, (AbstractCreature)AbstractDungeon.player, (AbstractPower)new WeakPower(mo, 1, false), 1, true`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        for (AbstractMonster mo : AbstractDungeon.getCurrRoom().monsters.monsters) {
            this.addToBot(new RelicAboveCreatureAction(mo, this));
            this.addToBot(new ApplyPowerAction((AbstractCreature)mo, (AbstractCreature)AbstractDungeon.player, (AbstractPower)new WeakPower(mo, 1, false), 1, true));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `RedMask` — `new RedMask()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new RedMask();
    }
```

</details>

## RedSkull
File: `relics\RedSkull.java`

### atBattleStart()

**Creates:**
- `AbstractGameAction` — `new AbstractGameAction(){

            @Override
            public void update() {
                if (!RedSkull.this.isActive && AbstractDungeon.player.isBloodied) {
                    RedSkull.thi...`
- `StrengthPower` — `new StrengthPower(AbstractDungeon.player, 3)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, RedSkull.this)`

**Queue insertion:**
- [BOT] `this.addToBot(new AbstractGameAction(){

            @Override
            public void update() {
                if (!RedSkull.this.isActive && Abstr`
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, RedSkull.this))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.isActive = false;
        this.addToBot(new AbstractGameAction(){

            @Override
            public void update() {
                if (!RedSkull.this.isActive && AbstractDungeon.player.isBloodied) {
                    RedSkull.this.flash();
                    RedSkull.this.pulse = true;
                    AbstractDungeon.player.addPower(new StrengthPower(AbstractDungeon.player, 3));
                    this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, RedSkull.this));
                    RedSkull.this.isActive = true;
                    AbstractDungeon.onModifyPower();
                }
                this.isDone = true;
            }
        });
    }
```

</details>

### onVictory()

<details><summary>Full body</summary>

```java
@Override
    public void onVictory() {
        this.pulse = false;
        this.isActive = false;
    }
```

</details>

### makeCopy()

**Creates:**
- `RedSkull` — `new RedSkull()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new RedSkull();
    }
```

</details>

## RegalPillow
File: `relics\RegalPillow.java`

### makeCopy()

**Creates:**
- `RegalPillow` — `new RegalPillow()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new RegalPillow();
    }
```

</details>

## RingOfTheSerpent
File: `relics\RingOfTheSerpent.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        ++AbstractDungeon.player.masterHandSize;
    }
```

</details>

### onUnequip()

<details><summary>Full body</summary>

```java
@Override
    public void onUnequip() {
        --AbstractDungeon.player.masterHandSize;
    }
```

</details>

### makeCopy()

**Creates:**
- `RingOfTheSerpent` — `new RingOfTheSerpent()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new RingOfTheSerpent();
    }
```

</details>

## RunicCapacitor
File: `relics\RunicCapacitor.java`

### atPreBattle()

<details><summary>Full body</summary>

```java
@Override
    public void atPreBattle() {
        this.firstTurn = true;
    }
```

</details>

### makeCopy()

**Creates:**
- `RunicCapacitor` — `new RunicCapacitor()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new RunicCapacitor();
    }
```

</details>

## RunicCube
File: `relics\RunicCube.java`

### wasHPLost(int damageAmount)

**Creates:**
- `DrawCardAction` — `new DrawCardAction(AbstractDungeon.player, 1)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

**Queue insertion:**
- [TOP] `this.addToTop(new DrawCardAction(AbstractDungeon.player, 1))`
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

<details><summary>Full body</summary>

```java
@Override
    public void wasHPLost(int damageAmount) {
        if (AbstractDungeon.getCurrRoom().phase == AbstractRoom.RoomPhase.COMBAT && damageAmount > 0) {
            this.flash();
            this.addToTop(new DrawCardAction(AbstractDungeon.player, 1));
            this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `RunicCube` — `new RunicCube()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new RunicCube();
    }
```

</details>

## RunicDome
File: `relics\RunicDome.java`

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
- `RunicDome` — `new RunicDome()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new RunicDome();
    }
```

</details>

## RunicPyramid
File: `relics\RunicPyramid.java`

### makeCopy()

**Creates:**
- `RunicPyramid` — `new RunicPyramid()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new RunicPyramid();
    }
```

</details>

