# Relics: L

3 relics

## Lantern
File: `relics\Lantern.java`

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
- `Lantern` — `new Lantern()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Lantern();
    }
```

</details>

## LetterOpener
File: `relics\LetterOpener.java`

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(5, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.SLASH_HEAVY)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(5, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.S`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.SKILL) {
            ++this.counter;
            if (this.counter % 3 == 0) {
                this.flash();
                this.counter = 0;
                this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
                this.addToBot(new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(5, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.SLASH_HEAVY));
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
- `LetterOpener` — `new LetterOpener()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new LetterOpener();
    }
```

</details>

## LizardTail
File: `relics\LizardTail.java`

### onTrigger()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

**Queue insertion:**
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

<details><summary>Full body</summary>

```java
@Override
    public void onTrigger() {
        this.flash();
        this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        int healAmt = AbstractDungeon.player.maxHealth / 2;
        if (healAmt < 1) {
            healAmt = 1;
        }
        AbstractDungeon.player.heal(healAmt, true);
        this.setCounter(-2);
    }
```

</details>

### makeCopy()

**Creates:**
- `LizardTail` — `new LizardTail()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new LizardTail();
    }
```

</details>

