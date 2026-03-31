# Relics: N

8 relics

## Necronomicon
File: `relics\Necronomicon.java`

### onEquip()

**Creates:**
- `ShowCardAndObtainEffect` — `new ShowCardAndObtainEffect(new Necronomicurse(), (float)Settings.WIDTH / 2.0f, (float)Settings.HEIGHT / 2.0f)`
- `Necronomicurse` — `new Necronomicurse()`

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        CardCrawlGame.sound.play("NECRONOMICON");
        this.description = this.DESCRIPTIONS[0] + 2 + this.DESCRIPTIONS[2];
        AbstractDungeon.effectList.add(new ShowCardAndObtainEffect(new Necronomicurse(), (float)Settings.WIDTH / 2.0f, (float)Settings.HEIGHT / 2.0f));
        UnlockTracker.markCardAsSeen("Necronomicurse");
    }
```

</details>

### onUnequip()

<details><summary>Full body</summary>

```java
@Override
    public void onUnequip() {
        AbstractCard cardToRemove = null;
        for (AbstractCard c : AbstractDungeon.player.masterDeck.group) {
            if (!(c instanceof Necronomicurse)) continue;
            cardToRemove = c;
            break;
        }
        if (cardToRemove != null) {
            AbstractDungeon.player.masterDeck.group.remove(cardToRemove);
        }
    }
```

</details>

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `CardQueueItem` — `new CardQueueItem(tmp, m, card.energyOnUse, true, true)`

**Queue insertion:**
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.ATTACK && (card.costForTurn >= 2 && !card.freeToPlayOnce || card.cost == -1 && card.energyOnUse >= 2) && this.activated) {
            this.activated = false;
            this.flash();
            AbstractMonster m = null;
            if (action.target != null) {
                m = (AbstractMonster)action.target;
            }
            this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
            AbstractCard tmp = card.makeSameInstanceOf();
            tmp.current_x = card.current_x;
            tmp.current_y = card.current_y;
            tmp.target_x = (float)Settings.WIDTH / 2.0f - 300.0f * Settings.scale;
            tmp.target_y = (float)Settings.HEIGHT / 2.0f;
            tmp.applyPowers();
            tmp.purgeOnUse = true;
            AbstractDungeon.actionManager.addCardQueueItem(new CardQueueItem(tmp, m, card.energyOnUse, true, true), true);
            this.pulse = false;
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `Necronomicon` — `new Necronomicon()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Necronomicon();
    }
```

</details>

## NeowsLament
File: `relics\NeowsLament.java`

### atBattleStart()

**Creates:**
- `PowerTip` — `new PowerTip(this.name, this.description)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

**Queue insertion:**
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        if (this.counter > 0) {
            --this.counter;
            if (this.counter == 0) {
                this.setCounter(-2);
                this.description = this.DESCRIPTIONS[1];
                this.tips.clear();
                this.tips.add(new PowerTip(this.name, this.description));
                this.initializeTips();
            }
            this.flash();
            for (AbstractMonster m : AbstractDungeon.getCurrRoom().monsters.monsters) {
                m.currentHealth = 1;
                m.healthBarUpdatedEvent();
            }
            this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `NeowsLament` — `new NeowsLament()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new NeowsLament();
    }
```

</details>

## NilrysCodex
File: `relics\NilrysCodex.java`

### onPlayerEndTurn()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `CodexAction` — `new CodexAction()`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new CodexAction())`

<details><summary>Full body</summary>

```java
@Override
    public void onPlayerEndTurn() {
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new CodexAction());
    }
```

</details>

### makeCopy()

**Creates:**
- `NilrysCodex` — `new NilrysCodex()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new NilrysCodex();
    }
```

</details>

## NinjaScroll
File: `relics\NinjaScroll.java`

### atBattleStartPreDraw()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `MakeTempCardInHandAction` — `new MakeTempCardInHandAction((AbstractCard)new Shiv(), 3, false)`
- `Shiv` — `new Shiv()`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new MakeTempCardInHandAction((AbstractCard)new Shiv(), 3, false))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStartPreDraw() {
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new MakeTempCardInHandAction((AbstractCard)new Shiv(), 3, false));
    }
```

</details>

### makeCopy()

**Creates:**
- `NinjaScroll` — `new NinjaScroll()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new NinjaScroll();
    }
```

</details>

## NlothsGift
File: `relics\NlothsGift.java`

### makeCopy()

**Creates:**
- `NlothsGift` — `new NlothsGift()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new NlothsGift();
    }
```

</details>

## NlothsMask
File: `relics\NlothsMask.java`

### makeCopy()

**Creates:**
- `NlothsMask` — `new NlothsMask()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new NlothsMask();
    }
```

</details>

## NuclearBattery
File: `relics\NuclearBattery.java`

### atPreBattle()

**Creates:**
- `Plasma` — `new Plasma()`

<details><summary>Full body</summary>

```java
@Override
    public void atPreBattle() {
        AbstractDungeon.player.channelOrb(new Plasma());
    }
```

</details>

### makeCopy()

**Creates:**
- `NuclearBattery` — `new NuclearBattery()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new NuclearBattery();
    }
```

</details>

## Nunchaku
File: `relics\Nunchaku.java`

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `GainEnergyAction` — `new GainEnergyAction(1)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new GainEnergyAction(1))`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.ATTACK) {
            ++this.counter;
            if (this.counter % 10 == 0) {
                this.counter = 0;
                this.flash();
                this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
                this.addToBot(new GainEnergyAction(1));
            }
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `Nunchaku` — `new Nunchaku()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Nunchaku();
    }
```

</details>

