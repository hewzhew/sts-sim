# Relics: E

5 relics

## Ectoplasm
File: `relics\Ectoplasm.java`

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
- `Ectoplasm` — `new Ectoplasm()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Ectoplasm();
    }
```

</details>

## EmotionChip
File: `relics\EmotionChip.java`

### wasHPLost(int damageAmount)

<details><summary>Full body</summary>

```java
@Override
    public void wasHPLost(int damageAmount) {
        if (AbstractDungeon.getCurrRoom().phase == AbstractRoom.RoomPhase.COMBAT && damageAmount > 0) {
            this.flash();
            if (!this.pulse) {
                this.beginPulse();
                this.pulse = true;
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
        this.pulse = false;
    }
```

</details>

### makeCopy()

**Creates:**
- `EmotionChip` — `new EmotionChip()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new EmotionChip();
    }
```

</details>

## EmptyCage
File: `relics\EmptyCage.java`

### onEquip()

**Creates:**
- `CardGroup` — `new CardGroup(CardGroup.CardGroupType.UNSPECIFIED)`

**Queue insertion:**
- [TOP] `tmp.addToTop(card)`

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        this.cardsSelected = false;
        if (AbstractDungeon.isScreenUp) {
            AbstractDungeon.dynamicBanner.hide();
            AbstractDungeon.overlayMenu.cancelButton.hide();
            AbstractDungeon.previousScreen = AbstractDungeon.screen;
        }
        AbstractDungeon.getCurrRoom().phase = AbstractRoom.RoomPhase.INCOMPLETE;
        CardGroup tmp = new CardGroup(CardGroup.CardGroupType.UNSPECIFIED);
        for (AbstractCard card : AbstractDungeon.player.masterDeck.getPurgeableCards().group) {
            tmp.addToTop(card);
        }
        if (tmp.group.isEmpty()) {
            this.cardsSelected = true;
            return;
        }
        if (tmp.group.size() <= 2) {
            this.deleteCards(tmp.group);
        } else {
            AbstractDungeon.gridSelectScreen.open(AbstractDungeon.player.masterDeck.getPurgeableCards(), 2, this.DESCRIPTIONS[1], false, false, false, true);
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `EmptyCage` — `new EmptyCage()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new EmptyCage();
    }
```

</details>

## Enchiridion
File: `relics\Enchiridion.java`

### atPreBattle()

**Creates:**
- `MakeTempCardInHandAction` — `new MakeTempCardInHandAction(c)`

**Queue insertion:**
- [BOT] `this.addToBot(new MakeTempCardInHandAction(c))`

<details><summary>Full body</summary>

```java
@Override
    public void atPreBattle() {
        this.flash();
        AbstractCard c = AbstractDungeon.returnTrulyRandomCardInCombat(AbstractCard.CardType.POWER).makeCopy();
        if (c.cost != -1) {
            c.setCostForTurn(0);
        }
        UnlockTracker.markCardAsSeen(c.cardID);
        this.addToBot(new MakeTempCardInHandAction(c));
    }
```

</details>

### makeCopy()

**Creates:**
- `Enchiridion` — `new Enchiridion()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Enchiridion();
    }
```

</details>

## EternalFeather
File: `relics\EternalFeather.java`

### onEnterRoom(AbstractRoom room)

<details><summary>Full body</summary>

```java
@Override
    public void onEnterRoom(AbstractRoom room) {
        if (room instanceof RestRoom) {
            this.flash();
            int amountToGain = AbstractDungeon.player.masterDeck.size() / 5 * 3;
            AbstractDungeon.player.heal(amountToGain);
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `EternalFeather` — `new EternalFeather()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new EternalFeather();
    }
```

</details>

