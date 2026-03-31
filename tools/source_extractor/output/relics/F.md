# Relics: F

6 relics

## FaceOfCleric
File: `relics\FaceOfCleric.java`

### onVictory()

<details><summary>Full body</summary>

```java
@Override
    public void onVictory() {
        this.flash();
        AbstractDungeon.player.increaseMaxHp(1, true);
    }
```

</details>

### makeCopy()

**Creates:**
- `FaceOfCleric` — `new FaceOfCleric()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new FaceOfCleric();
    }
```

</details>

## FossilizedHelix
File: `relics\FossilizedHelix.java`

### atBattleStart()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new BufferPower(AbstractDungeon.player, 1), 1)`
- `BufferPower` — `new BufferPower(AbstractDungeon.player, 1)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new BufferPower(AbstractDungeon.player, 1), 1))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new BufferPower(AbstractDungeon.player, 1), 1));
        this.grayscale = true;
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

### makeCopy()

**Creates:**
- `FossilizedHelix` — `new FossilizedHelix()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new FossilizedHelix();
    }
```

</details>

## FrozenCore
File: `relics\FrozenCore.java`

### onPlayerEndTurn()

**Creates:**
- `Frost` — `new Frost()`

<details><summary>Full body</summary>

```java
@Override
    public void onPlayerEndTurn() {
        if (AbstractDungeon.player.hasEmptyOrb()) {
            this.flash();
            AbstractDungeon.player.channelOrb(new Frost());
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `FrozenCore` — `new FrozenCore()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new FrozenCore();
    }
```

</details>

## FrozenEgg2
File: `relics\FrozenEgg2.java`

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
        if (c.type == AbstractCard.CardType.POWER && c.canUpgrade() && !c.upgraded) {
            c.upgrade();
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `FrozenEgg2` — `new FrozenEgg2()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new FrozenEgg2();
    }
```

</details>

## FrozenEye
File: `relics\FrozenEye.java`

### makeCopy()

**Creates:**
- `FrozenEye` — `new FrozenEye()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new FrozenEye();
    }
```

</details>

## FusionHammer
File: `relics\FusionHammer.java`

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
- `FusionHammer` — `new FusionHammer()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new FusionHammer();
    }
```

</details>

