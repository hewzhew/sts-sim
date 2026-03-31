# Relics: B

16 relics

## BagOfMarbles
File: `relics\BagOfMarbles.java`

### atBattleStart()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(mo, this)`
- `ApplyPowerAction` — `new ApplyPowerAction((AbstractCreature)mo, (AbstractCreature)AbstractDungeon.player, (AbstractPower)new VulnerablePower(mo, 1, false), 1, true)`
- `VulnerablePower` — `new VulnerablePower(mo, 1, false)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(mo, this))`
- [BOT] `this.addToBot(new ApplyPowerAction((AbstractCreature)mo, (AbstractCreature)AbstractDungeon.player, (AbstractPower)new VulnerablePower(mo, 1, false), 1`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        for (AbstractMonster mo : AbstractDungeon.getCurrRoom().monsters.monsters) {
            this.addToBot(new RelicAboveCreatureAction(mo, this));
            this.addToBot(new ApplyPowerAction((AbstractCreature)mo, (AbstractCreature)AbstractDungeon.player, (AbstractPower)new VulnerablePower(mo, 1, false), 1, true));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `BagOfMarbles` — `new BagOfMarbles()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new BagOfMarbles();
    }
```

</details>

## BagOfPreparation
File: `relics\BagOfPreparation.java`

### atBattleStart()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `DrawCardAction` — `new DrawCardAction(AbstractDungeon.player, 2)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new DrawCardAction(AbstractDungeon.player, 2))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new DrawCardAction(AbstractDungeon.player, 2));
    }
```

</details>

### makeCopy()

**Creates:**
- `BagOfPreparation` — `new BagOfPreparation()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new BagOfPreparation();
    }
```

</details>

## BirdFacedUrn
File: `relics\BirdFacedUrn.java`

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `HealAction` — `new HealAction(AbstractDungeon.player, AbstractDungeon.player, 2)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

**Queue insertion:**
- [TOP] `this.addToTop(new HealAction(AbstractDungeon.player, AbstractDungeon.player, 2))`
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.POWER) {
            this.flash();
            this.addToTop(new HealAction(AbstractDungeon.player, AbstractDungeon.player, 2));
            this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `BirdFacedUrn` — `new BirdFacedUrn()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new BirdFacedUrn();
    }
```

</details>

## BlackBlood
File: `relics\BlackBlood.java`

### onVictory()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(p, this)`

**Queue insertion:**
- [TOP] `this.addToTop(new RelicAboveCreatureAction(p, this))`

<details><summary>Full body</summary>

```java
@Override
    public void onVictory() {
        this.flash();
        AbstractPlayer p = AbstractDungeon.player;
        this.addToTop(new RelicAboveCreatureAction(p, this));
        if (p.currentHealth > 0) {
            p.heal(12);
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `BlackBlood` — `new BlackBlood()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new BlackBlood();
    }
```

</details>

## BlackStar
File: `relics\BlackStar.java`

### onEnterRoom(AbstractRoom room)

<details><summary>Full body</summary>

```java
@Override
    public void onEnterRoom(AbstractRoom room) {
        if (room instanceof MonsterRoomElite) {
            this.pulse = true;
            this.beginPulse();
        } else {
            this.pulse = false;
        }
    }
```

</details>

### onVictory()

<details><summary>Full body</summary>

```java
@Override
    public void onVictory() {
        if (AbstractDungeon.getCurrRoom() instanceof MonsterRoomElite) {
            this.flash();
            this.pulse = false;
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `BlackStar` — `new BlackStar()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new BlackStar();
    }
```

</details>

## BloodVial
File: `relics\BloodVial.java`

### atBattleStart()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `HealAction` — `new HealAction(AbstractDungeon.player, AbstractDungeon.player, 2, 0.0f)`

**Queue insertion:**
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [TOP] `this.addToTop(new HealAction(AbstractDungeon.player, AbstractDungeon.player, 2, 0.0f))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToTop(new HealAction(AbstractDungeon.player, AbstractDungeon.player, 2, 0.0f));
    }
```

</details>

### makeCopy()

**Creates:**
- `BloodVial` — `new BloodVial()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new BloodVial();
    }
```

</details>

## BloodyIdol
File: `relics\BloodyIdol.java`

### makeCopy()

**Creates:**
- `BloodyIdol` — `new BloodyIdol()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new BloodyIdol();
    }
```

</details>

## BlueCandle
File: `relics\BlueCandle.java`

### makeCopy()

**Creates:**
- `BlueCandle` — `new BlueCandle()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new BlueCandle();
    }
```

</details>

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `LoseHPAction` — `new LoseHPAction(AbstractDungeon.player, AbstractDungeon.player, 1, AbstractGameAction.AttackEffect.FIRE)`

**Queue insertion:**
- [BOT] `this.addToBot(new LoseHPAction(AbstractDungeon.player, AbstractDungeon.player, 1, AbstractGameAction.AttackEffect.FIRE))`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.CURSE) {
            AbstractDungeon.player.getRelic(ID).flash();
            this.addToBot(new LoseHPAction(AbstractDungeon.player, AbstractDungeon.player, 1, AbstractGameAction.AttackEffect.FIRE));
            card.exhaust = true;
            action.exhaustCard = true;
        }
    }
```

</details>

## Boot
File: `relics\Boot.java`

### makeCopy()

**Creates:**
- `Boot` — `new Boot()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Boot();
    }
```

</details>

## BottledFlame
File: `relics\BottledFlame.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        if (AbstractDungeon.player.masterDeck.getPurgeableCards().getAttacks().size() > 0) {
            this.cardSelected = false;
            if (AbstractDungeon.isScreenUp) {
                AbstractDungeon.dynamicBanner.hide();
                AbstractDungeon.overlayMenu.cancelButton.hide();
                AbstractDungeon.previousScreen = AbstractDungeon.screen;
            }
            AbstractDungeon.getCurrRoom().phase = AbstractRoom.RoomPhase.INCOMPLETE;
            AbstractDungeon.gridSelectScreen.open(AbstractDungeon.player.masterDeck.getPurgeableCards().getAttacks(), 1, this.DESCRIPTIONS[1] + this.name + LocalizedStrings.PERIOD, false, false, false, false);
        }
    }
```

</details>

### onUnequip()

<details><summary>Full body</summary>

```java
@Override
    public void onUnequip() {
        AbstractCard cardInDeck;
        if (this.card != null && (cardInDeck = AbstractDungeon.player.masterDeck.getSpecificCard(this.card)) != null) {
            cardInDeck.inBottleFlame = false;
        }
    }
```

</details>

### atBattleStart()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

**Queue insertion:**
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
    }
```

</details>

### makeCopy()

**Creates:**
- `BottledFlame` — `new BottledFlame()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new BottledFlame();
    }
```

</details>

## BottledLightning
File: `relics\BottledLightning.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        if (AbstractDungeon.player.masterDeck.getPurgeableCards().getSkills().size() > 0) {
            this.cardSelected = false;
            if (AbstractDungeon.isScreenUp) {
                AbstractDungeon.dynamicBanner.hide();
                AbstractDungeon.overlayMenu.cancelButton.hide();
                AbstractDungeon.previousScreen = AbstractDungeon.screen;
            }
            AbstractDungeon.getCurrRoom().phase = AbstractRoom.RoomPhase.INCOMPLETE;
            AbstractDungeon.gridSelectScreen.open(AbstractDungeon.player.masterDeck.getPurgeableCards().getSkills(), 1, this.DESCRIPTIONS[1] + this.name + LocalizedStrings.PERIOD, false, false, false, false);
        }
    }
```

</details>

### onUnequip()

<details><summary>Full body</summary>

```java
@Override
    public void onUnequip() {
        AbstractCard cardInDeck;
        if (this.card != null && (cardInDeck = AbstractDungeon.player.masterDeck.getSpecificCard(this.card)) != null) {
            cardInDeck.inBottleLightning = false;
        }
    }
```

</details>

### atBattleStart()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

**Queue insertion:**
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
    }
```

</details>

### makeCopy()

**Creates:**
- `BottledLightning` — `new BottledLightning()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new BottledLightning();
    }
```

</details>

## BottledTornado
File: `relics\BottledTornado.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        if (AbstractDungeon.player.masterDeck.getPurgeableCards().getPowers().size() > 0) {
            this.cardSelected = false;
            if (AbstractDungeon.isScreenUp) {
                AbstractDungeon.dynamicBanner.hide();
                AbstractDungeon.overlayMenu.cancelButton.hide();
                AbstractDungeon.previousScreen = AbstractDungeon.screen;
            }
            AbstractDungeon.getCurrRoom().phase = AbstractRoom.RoomPhase.INCOMPLETE;
            AbstractDungeon.gridSelectScreen.open(AbstractDungeon.player.masterDeck.getPurgeableCards().getPowers(), 1, this.DESCRIPTIONS[1] + this.name + LocalizedStrings.PERIOD, false, false, false, false);
        }
    }
```

</details>

### onUnequip()

<details><summary>Full body</summary>

```java
@Override
    public void onUnequip() {
        AbstractCard cardInDeck;
        if (this.card != null && (cardInDeck = AbstractDungeon.player.masterDeck.getSpecificCard(this.card)) != null) {
            cardInDeck.inBottleTornado = false;
        }
    }
```

</details>

### atBattleStart()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

**Queue insertion:**
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
    }
```

</details>

### makeCopy()

**Creates:**
- `BottledTornado` — `new BottledTornado()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new BottledTornado();
    }
```

</details>

## Brimstone
File: `relics\Brimstone.java`

### makeCopy()

**Creates:**
- `Brimstone` — `new Brimstone()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Brimstone();
    }
```

</details>

## BronzeScales
File: `relics\BronzeScales.java`

### atBattleStart()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new ThornsPower(AbstractDungeon.player, 3), 3)`
- `ThornsPower` — `new ThornsPower(AbstractDungeon.player, 3)`

**Queue insertion:**
- [TOP] `this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new ThornsPower(AbstractDungeon.player, 3), 3))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new ThornsPower(AbstractDungeon.player, 3), 3));
    }
```

</details>

### makeCopy()

**Creates:**
- `BronzeScales` — `new BronzeScales()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new BronzeScales();
    }
```

</details>

## BurningBlood
File: `relics\BurningBlood.java`

### onVictory()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

**Queue insertion:**
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

<details><summary>Full body</summary>

```java
@Override
    public void onVictory() {
        this.flash();
        this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        AbstractPlayer p = AbstractDungeon.player;
        if (p.currentHealth > 0) {
            p.heal(6);
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `BurningBlood` — `new BurningBlood()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new BurningBlood();
    }
```

</details>

## BustedCrown
File: `relics\BustedCrown.java`

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
- `BustedCrown` — `new BustedCrown()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new BustedCrown();
    }
```

</details>

