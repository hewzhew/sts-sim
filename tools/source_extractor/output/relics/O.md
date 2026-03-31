# Relics: O

8 relics

## OddMushroom
File: `relics\OddMushroom.java`

### makeCopy()

**Creates:**
- `OddMushroom` — `new OddMushroom()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new OddMushroom();
    }
```

</details>

## OddlySmoothStone
File: `relics\OddlySmoothStone.java`

### atBattleStart()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new DexterityPower(AbstractDungeon.player, 1), 1)`
- `DexterityPower` — `new DexterityPower(AbstractDungeon.player, 1)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

**Queue insertion:**
- [TOP] `this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new DexterityPower(AbstractDungeon.player, 1), 1))`
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new DexterityPower(AbstractDungeon.player, 1), 1));
        this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
    }
```

</details>

### makeCopy()

**Creates:**
- `OddlySmoothStone` — `new OddlySmoothStone()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new OddlySmoothStone();
    }
```

</details>

## OldCoin
File: `relics\OldCoin.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        CardCrawlGame.sound.play("GOLD_GAIN");
        AbstractDungeon.player.gainGold(300);
    }
```

</details>

### makeCopy()

**Creates:**
- `OldCoin` — `new OldCoin()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new OldCoin();
    }
```

</details>

## Omamori
File: `relics\Omamori.java`

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

### makeCopy()

**Creates:**
- `Omamori` — `new Omamori()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Omamori();
    }
```

</details>

## OrangePellets
File: `relics\OrangePellets.java`

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `RemoveDebuffsAction` — `new RemoveDebuffsAction(AbstractDungeon.player)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new RemoveDebuffsAction(AbstractDungeon.player))`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.ATTACK) {
            ATTACK = true;
        } else if (card.type == AbstractCard.CardType.SKILL) {
            SKILL = true;
        } else if (card.type == AbstractCard.CardType.POWER) {
            POWER = true;
        }
        if (ATTACK && SKILL && POWER) {
            this.flash();
            this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
            this.addToBot(new RemoveDebuffsAction(AbstractDungeon.player));
            SKILL = false;
            POWER = false;
            ATTACK = false;
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `OrangePellets` — `new OrangePellets()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new OrangePellets();
    }
```

</details>

## Orichalcum
File: `relics\Orichalcum.java`

### onPlayerEndTurn()

**Creates:**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)AbstractDungeon.player, AbstractDungeon.player, 6)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

**Queue insertion:**
- [TOP] `this.addToTop(new GainBlockAction((AbstractCreature)AbstractDungeon.player, AbstractDungeon.player, 6))`
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

<details><summary>Full body</summary>

```java
@Override
    public void onPlayerEndTurn() {
        if (AbstractDungeon.player.currentBlock == 0 || this.trigger) {
            this.trigger = false;
            this.flash();
            this.stopPulse();
            this.addToTop(new GainBlockAction((AbstractCreature)AbstractDungeon.player, AbstractDungeon.player, 6));
            this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        }
    }
```

</details>

### onPlayerGainedBlock(float blockAmount)

<details><summary>Full body</summary>

```java
@Override
    public int onPlayerGainedBlock(float blockAmount) {
        if (blockAmount > 0.0f) {
            this.stopPulse();
        }
        return MathUtils.floor(blockAmount);
    }
```

</details>

### onVictory()

<details><summary>Full body</summary>

```java
@Override
    public void onVictory() {
        this.stopPulse();
    }
```

</details>

### makeCopy()

**Creates:**
- `Orichalcum` — `new Orichalcum()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Orichalcum();
    }
```

</details>

## OrnamentalFan
File: `relics\OrnamentalFan.java`

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)AbstractDungeon.player, AbstractDungeon.player, 4)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new GainBlockAction((AbstractCreature)AbstractDungeon.player, AbstractDungeon.player, 4))`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.ATTACK) {
            ++this.counter;
            if (this.counter % 3 == 0) {
                this.flash();
                this.counter = 0;
                this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
                this.addToBot(new GainBlockAction((AbstractCreature)AbstractDungeon.player, AbstractDungeon.player, 4));
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
- `OrnamentalFan` — `new OrnamentalFan()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new OrnamentalFan();
    }
```

</details>

## Orrery
File: `relics\Orrery.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        for (int i = 0; i < 4; ++i) {
            AbstractDungeon.getCurrRoom().addCardToRewards();
        }
        AbstractDungeon.combatRewardScreen.open(this.DESCRIPTIONS[1]);
        AbstractDungeon.getCurrRoom().rewardPopOutTimer = 0.0f;
    }
```

</details>

### makeCopy()

**Creates:**
- `Orrery` — `new Orrery()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Orrery();
    }
```

</details>

