# Relics: V

3 relics

## Vajra
File: `relics\Vajra.java`

### atBattleStart()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new StrengthPower(AbstractDungeon.player, 1), 1)`
- `StrengthPower` — `new StrengthPower(AbstractDungeon.player, 1)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

**Queue insertion:**
- [TOP] `this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new StrengthPower(AbstractDungeon.player, 1), 1))`
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new StrengthPower(AbstractDungeon.player, 1), 1));
        this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
    }
```

</details>

### makeCopy()

**Creates:**
- `Vajra` — `new Vajra()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Vajra();
    }
```

</details>

## VelvetChoker
File: `relics\VelvetChoker.java`

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

### atBattleStart()

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.counter = 0;
    }
```

</details>

### onPlayCard(AbstractCard card, AbstractMonster m)

<details><summary>Full body</summary>

```java
@Override
    public void onPlayCard(AbstractCard card, AbstractMonster m) {
        if (this.counter < 6) {
            ++this.counter;
            if (this.counter >= 6) {
                this.flash();
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
- `VelvetChoker` — `new VelvetChoker()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new VelvetChoker();
    }
```

</details>

## VioletLotus
File: `relics\VioletLotus.java`

### onChangeStance(AbstractStance prevStance, AbstractStance newStance)

**Creates:**
- `GainEnergyAction` — `new GainEnergyAction(1)`

**Queue insertion:**
- [BOT] `this.addToBot(new GainEnergyAction(1))`

<details><summary>Full body</summary>

```java
@Override
    public void onChangeStance(AbstractStance prevStance, AbstractStance newStance) {
        if (!prevStance.ID.equals(newStance.ID) && prevStance.ID.equals("Calm")) {
            this.flash();
            this.addToBot(new GainEnergyAction(1));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `VioletLotus` — `new VioletLotus()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new VioletLotus();
    }
```

</details>

