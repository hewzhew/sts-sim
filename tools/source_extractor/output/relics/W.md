# Relics: W

7 relics

## Waffle
File: `relics\Waffle.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        AbstractDungeon.player.increaseMaxHp(7, false);
        AbstractDungeon.player.heal(AbstractDungeon.player.maxHealth);
    }
```

</details>

### makeCopy()

**Creates:**
- `Waffle` — `new Waffle()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Waffle();
    }
```

</details>

## WarPaint
File: `relics\WarPaint.java`

### onEquip()

**Creates:**
- `None` — `new ArrayList<AbstractCard>()`
- `Random` — `new Random(AbstractDungeon.miscRng.randomLong())`
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
        Collections.shuffle(upgradableCards, new Random(AbstractDungeon.miscRng.randomLong()));
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
- `WarPaint` — `new WarPaint()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new WarPaint();
    }
```

</details>

## WarpedTongs
File: `relics\WarpedTongs.java`

### makeCopy()

**Creates:**
- `WarpedTongs` — `new WarpedTongs()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new WarpedTongs();
    }
```

</details>

## Whetstone
File: `relics\Whetstone.java`

### onEquip()

**Creates:**
- `None` — `new ArrayList<AbstractCard>()`
- `Random` — `new Random(AbstractDungeon.miscRng.randomLong())`
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
            if (!c.canUpgrade() || c.type != AbstractCard.CardType.ATTACK) continue;
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
- `Whetstone` — `new Whetstone()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Whetstone();
    }
```

</details>

## WhiteBeast
File: `relics\WhiteBeast.java`

### makeCopy()

**Creates:**
- `WhiteBeast` — `new WhiteBeast()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new WhiteBeast();
    }
```

</details>

## WingBoots
File: `relics\WingBoots.java`

### makeCopy()

**Creates:**
- `WingBoots` — `new WingBoots()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new WingBoots();
    }
```

</details>

## WristBlade
File: `relics\WristBlade.java`

### makeCopy()

**Creates:**
- `WristBlade` — `new WristBlade()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new WristBlade();
    }
```

</details>

