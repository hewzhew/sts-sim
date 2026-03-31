# Relics: I

4 relics

## IceCream
File: `relics\IceCream.java`

### makeCopy()

**Creates:**
- `IceCream` — `new IceCream()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new IceCream();
    }
```

</details>

## IncenseBurner
File: `relics\IncenseBurner.java`

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
- `IncenseBurner` — `new IncenseBurner()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new IncenseBurner();
    }
```

</details>

## InkBottle
File: `relics\InkBottle.java`

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `DrawCardAction` — `new DrawCardAction(1)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new DrawCardAction(1))`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        ++this.counter;
        if (this.counter == 10) {
            this.counter = 0;
            this.flash();
            this.pulse = false;
            this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
            this.addToBot(new DrawCardAction(1));
        } else if (this.counter == 9) {
            this.beginPulse();
            this.pulse = true;
        }
    }
```

</details>

### atBattleStart()

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        if (this.counter == 9) {
            this.beginPulse();
            this.pulse = true;
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `InkBottle` — `new InkBottle()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new InkBottle();
    }
```

</details>

## Inserter
File: `relics\Inserter.java`

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
- `Inserter` — `new Inserter()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Inserter();
    }
```

</details>

