# Relics: D

13 relics

## DEPRECATEDDodecahedron
File: `relics\deprecated\DEPRECATEDDodecahedron.java`

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

### atBattleStart()

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.controlPulse();
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

### onPlayerHeal(int healAmount)

<details><summary>Full body</summary>

```java
@Override
    public int onPlayerHeal(int healAmount) {
        this.controlPulse();
        return super.onPlayerHeal(healAmount);
    }
```

</details>

### onAttacked(DamageInfo info, int damageAmount)

<details><summary>Full body</summary>

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (damageAmount > 0) {
            this.stopPulse();
        }
        return super.onAttacked(info, damageAmount);
    }
```

</details>

### makeCopy()

**Creates:**
- `DEPRECATEDDodecahedron` — `new DEPRECATEDDodecahedron()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new DEPRECATEDDodecahedron();
    }
```

</details>

## DEPRECATEDYin
File: `relics\deprecated\DEPRECATEDYin.java`

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(p, this)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new StrengthPower(p, 1), 1)`
- `StrengthPower` — `new StrengthPower(p, 1)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new LoseStrengthPower(p, 1), 1)`
- `LoseStrengthPower` — `new LoseStrengthPower(p, 1)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(p, this))`
- [BOT] `this.addToBot(new ApplyPowerAction(p, p, new StrengthPower(p, 1), 1))`
- [BOT] `this.addToBot(new ApplyPowerAction(p, p, new LoseStrengthPower(p, 1), 1))`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.SKILL) {
            this.flash();
            AbstractPlayer p = AbstractDungeon.player;
            this.addToBot(new RelicAboveCreatureAction(p, this));
            this.addToBot(new ApplyPowerAction(p, p, new StrengthPower(p, 1), 1));
            this.addToBot(new ApplyPowerAction(p, p, new LoseStrengthPower(p, 1), 1));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `DEPRECATEDYin` — `new DEPRECATEDYin()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new DEPRECATEDYin();
    }
```

</details>

## DEPRECATED_DarkCore
File: `relics\deprecated\DEPRECATED_DarkCore.java`

### makeCopy()

**Creates:**
- `DEPRECATED_DarkCore` — `new DEPRECATED_DarkCore()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new DEPRECATED_DarkCore();
    }
```

</details>

## Damaru
File: `relics\Damaru.java`

### makeCopy()

**Creates:**
- `Damaru` — `new Damaru()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Damaru();
    }
```

</details>

## DarkstonePeriapt
File: `relics\DarkstonePeriapt.java`

### onObtainCard(AbstractCard card)

<details><summary>Full body</summary>

```java
@Override
    public void onObtainCard(AbstractCard card) {
        if (card.color == AbstractCard.CardColor.CURSE) {
            if (ModHelper.isModEnabled("Hoarder")) {
                AbstractDungeon.player.increaseMaxHp(6, true);
                AbstractDungeon.player.increaseMaxHp(6, true);
            }
            AbstractDungeon.player.increaseMaxHp(6, true);
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `DarkstonePeriapt` — `new DarkstonePeriapt()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new DarkstonePeriapt();
    }
```

</details>

## DataDisk
File: `relics\DataDisk.java`

### atBattleStart()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new FocusPower(AbstractDungeon.player, 1), 1)`
- `FocusPower` — `new FocusPower(AbstractDungeon.player, 1)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

**Queue insertion:**
- [TOP] `this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new FocusPower(AbstractDungeon.player, 1), 1))`
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new FocusPower(AbstractDungeon.player, 1), 1));
        this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
    }
```

</details>

### makeCopy()

**Creates:**
- `DataDisk` — `new DataDisk()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new DataDisk();
    }
```

</details>

## DeadBranch
File: `relics\DeadBranch.java`

### onExhaust(AbstractCard card)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `MakeTempCardInHandAction` — `new MakeTempCardInHandAction(AbstractDungeon.returnTrulyRandomCardInCombat().makeCopy(), false)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new MakeTempCardInHandAction(AbstractDungeon.returnTrulyRandomCardInCombat().makeCopy(), false))`

<details><summary>Full body</summary>

```java
@Override
    public void onExhaust(AbstractCard card) {
        if (!AbstractDungeon.getMonsters().areMonstersBasicallyDead()) {
            this.flash();
            this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
            this.addToBot(new MakeTempCardInHandAction(AbstractDungeon.returnTrulyRandomCardInCombat().makeCopy(), false));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `DeadBranch` — `new DeadBranch()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new DeadBranch();
    }
```

</details>

## DerpRock
File: `relics\deprecated\DerpRock.java`

### atPreBattle()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

**Queue insertion:**
- [TOP] `AbstractDungeon.actionManager.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

<details><summary>Full body</summary>

```java
@Override
    public void atPreBattle() {
        AbstractDungeon.actionManager.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
    }
```

</details>

### makeCopy()

**Creates:**
- `DerpRock` — `new DerpRock()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new DerpRock();
    }
```

</details>

## DiscerningMonocle
File: `relics\DiscerningMonocle.java`

### onEnterRoom(AbstractRoom room)

<details><summary>Full body</summary>

```java
@Override
    public void onEnterRoom(AbstractRoom room) {
        if (room instanceof ShopRoom) {
            this.flash();
            this.pulse = true;
        } else {
            this.pulse = false;
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `DiscerningMonocle` — `new DiscerningMonocle()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new DiscerningMonocle();
    }
```

</details>

## DollysMirror
File: `relics\DollysMirror.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        this.cardSelected = false;
        if (AbstractDungeon.isScreenUp) {
            AbstractDungeon.dynamicBanner.hide();
            AbstractDungeon.overlayMenu.cancelButton.hide();
            AbstractDungeon.previousScreen = AbstractDungeon.screen;
        }
        AbstractDungeon.getCurrRoom().phase = AbstractRoom.RoomPhase.INCOMPLETE;
        AbstractDungeon.gridSelectScreen.open(AbstractDungeon.player.masterDeck, 1, this.DESCRIPTIONS[1], false, false, false, false);
    }
```

</details>

### makeCopy()

**Creates:**
- `DollysMirror` — `new DollysMirror()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new DollysMirror();
    }
```

</details>

## DreamCatcher
File: `relics\DreamCatcher.java`

### makeCopy()

**Creates:**
- `DreamCatcher` — `new DreamCatcher()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new DreamCatcher();
    }
```

</details>

## DuVuDoll
File: `relics\DuVuDoll.java`

### onMasterDeckChange()

**Creates:**
- `PowerTip` — `new PowerTip(this.name, this.description)`

<details><summary>Full body</summary>

```java
@Override
    public void onMasterDeckChange() {
        this.counter = 0;
        for (AbstractCard c : AbstractDungeon.player.masterDeck.group) {
            if (c.type != AbstractCard.CardType.CURSE) continue;
            ++this.counter;
        }
        this.description = this.counter == 0 ? this.DESCRIPTIONS[0] + 1 + this.DESCRIPTIONS[1] + this.DESCRIPTIONS[2] : this.DESCRIPTIONS[0] + 1 + this.DESCRIPTIONS[1] + this.DESCRIPTIONS[3] + this.counter + this.DESCRIPTIONS[4];
        this.tips.clear();
        this.tips.add(new PowerTip(this.name, this.description));
        this.initializeTips();
    }
```

</details>

### onEquip()

**Creates:**
- `PowerTip` — `new PowerTip(this.name, this.description)`

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        this.counter = 0;
        for (AbstractCard c : AbstractDungeon.player.masterDeck.group) {
            if (c.type != AbstractCard.CardType.CURSE) continue;
            ++this.counter;
        }
        this.description = this.counter == 0 ? this.DESCRIPTIONS[0] + 1 + this.DESCRIPTIONS[1] + this.DESCRIPTIONS[2] : this.DESCRIPTIONS[0] + 1 + this.DESCRIPTIONS[1] + this.DESCRIPTIONS[3] + this.counter + this.DESCRIPTIONS[4];
        this.tips.clear();
        this.tips.add(new PowerTip(this.name, this.description));
        this.initializeTips();
    }
```

</details>

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
        if (this.counter > 0) {
            this.flash();
            this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new StrengthPower(AbstractDungeon.player, this.counter), this.counter));
            this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `DuVuDoll` — `new DuVuDoll()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new DuVuDoll();
    }
```

</details>

## Duality
File: `relics\Duality.java`

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(p, this)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new DexterityPower(p, 1), 1)`
- `DexterityPower` — `new DexterityPower(p, 1)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new LoseDexterityPower(p, 1), 1)`
- `LoseDexterityPower` — `new LoseDexterityPower(p, 1)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(p, this))`
- [BOT] `this.addToBot(new ApplyPowerAction(p, p, new DexterityPower(p, 1), 1))`
- [BOT] `this.addToBot(new ApplyPowerAction(p, p, new LoseDexterityPower(p, 1), 1))`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.ATTACK) {
            this.flash();
            AbstractPlayer p = AbstractDungeon.player;
            this.addToBot(new RelicAboveCreatureAction(p, this));
            this.addToBot(new ApplyPowerAction(p, p, new DexterityPower(p, 1), 1));
            this.addToBot(new ApplyPowerAction(p, p, new LoseDexterityPower(p, 1), 1));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `Duality` — `new Duality()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Duality();
    }
```

</details>

