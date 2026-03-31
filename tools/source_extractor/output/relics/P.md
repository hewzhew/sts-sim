# Relics: P

14 relics

## PandorasBox
File: `relics\PandorasBox.java`

### onEquip()

**Creates:**
- `CardGroup` — `new CardGroup(CardGroup.CardGroupType.UNSPECIFIED)`

**Queue insertion:**
- [BOT] `group.addToBottom(card)`

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        this.calledTransform = false;
        Iterator<AbstractCard> i = AbstractDungeon.player.masterDeck.group.iterator();
        while (i.hasNext()) {
            AbstractCard e = i.next();
            if (!e.hasTag(AbstractCard.CardTags.STARTER_DEFEND) && !e.hasTag(AbstractCard.CardTags.STARTER_STRIKE)) continue;
            i.remove();
            ++this.count;
        }
        if (this.count > 0) {
            CardGroup group = new CardGroup(CardGroup.CardGroupType.UNSPECIFIED);
            for (int i2 = 0; i2 < this.count; ++i2) {
                AbstractCard card = AbstractDungeon.returnTrulyRandomCard().makeCopy();
                UnlockTracker.markCardAsSeen(card.cardID);
                card.isSeen = true;
                for (AbstractRelic r : AbstractDungeon.player.relics) {
                    r.onPreviewObtainCard(card);
                }
                group.addToBottom(card);
            }
            AbstractDungeon.gridSelectScreen.openConfirmationGrid(group, this.DESCRIPTIONS[1]);
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `PandorasBox` — `new PandorasBox()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new PandorasBox();
    }
```

</details>

## Pantograph
File: `relics\Pantograph.java`

### atBattleStart()

**Creates:**
- `HealAction` — `new HealAction(AbstractDungeon.player, AbstractDungeon.player, 25, 0.0f)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

**Queue insertion:**
- [TOP] `this.addToTop(new HealAction(AbstractDungeon.player, AbstractDungeon.player, 25, 0.0f))`
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        for (AbstractMonster m : AbstractDungeon.getMonsters().monsters) {
            if (m.type != AbstractMonster.EnemyType.BOSS) continue;
            this.flash();
            this.addToTop(new HealAction(AbstractDungeon.player, AbstractDungeon.player, 25, 0.0f));
            this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
            return;
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `Pantograph` — `new Pantograph()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Pantograph();
    }
```

</details>

## PaperCrane
File: `relics\PaperCrane.java`

### makeCopy()

**Creates:**
- `PaperCrane` — `new PaperCrane()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new PaperCrane();
    }
```

</details>

## PaperFrog
File: `relics\PaperFrog.java`

### makeCopy()

**Creates:**
- `PaperFrog` — `new PaperFrog()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new PaperFrog();
    }
```

</details>

## PeacePipe
File: `relics\PeacePipe.java`

### makeCopy()

**Creates:**
- `PeacePipe` — `new PeacePipe()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new PeacePipe();
    }
```

</details>

## Pear
File: `relics\Pear.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        AbstractDungeon.player.increaseMaxHp(10, true);
    }
```

</details>

### makeCopy()

**Creates:**
- `Pear` — `new Pear()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Pear();
    }
```

</details>

## PenNib
File: `relics\PenNib.java`

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `ApplyPowerAction` — `new ApplyPowerAction((AbstractCreature)AbstractDungeon.player, (AbstractCreature)AbstractDungeon.player, (AbstractPower)new PenNibPower(AbstractDungeon.player, 1), 1, true)`
- `PenNibPower` — `new PenNibPower(AbstractDungeon.player, 1)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new ApplyPowerAction((AbstractCreature)AbstractDungeon.player, (AbstractCreature)AbstractDungeon.player, (AbstractPower)new PenNibPower(`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.ATTACK) {
            ++this.counter;
            if (this.counter == 10) {
                this.counter = 0;
                this.flash();
                this.pulse = false;
            } else if (this.counter == 9) {
                this.beginPulse();
                this.pulse = true;
                AbstractDungeon.player.hand.refreshHandLayout();
                this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
                this.addToBot(new ApplyPowerAction((AbstractCreature)AbstractDungeon.player, (AbstractCreature)AbstractDungeon.player, (AbstractPower)new PenNibPower(AbstractDungeon.player, 1), 1, true));
            }
        }
    }
```

</details>

### atBattleStart()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction((AbstractCreature)AbstractDungeon.player, (AbstractCreature)AbstractDungeon.player, (AbstractPower)new PenNibPower(AbstractDungeon.player, 1), 1, true)`
- `PenNibPower` — `new PenNibPower(AbstractDungeon.player, 1)`

**Queue insertion:**
- [BOT] `this.addToBot(new ApplyPowerAction((AbstractCreature)AbstractDungeon.player, (AbstractCreature)AbstractDungeon.player, (AbstractPower)new PenNibPower(`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        if (this.counter == 9) {
            this.beginPulse();
            this.pulse = true;
            AbstractDungeon.player.hand.refreshHandLayout();
            this.addToBot(new ApplyPowerAction((AbstractCreature)AbstractDungeon.player, (AbstractCreature)AbstractDungeon.player, (AbstractPower)new PenNibPower(AbstractDungeon.player, 1), 1, true));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `PenNib` — `new PenNib()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new PenNib();
    }
```

</details>

## PhilosopherStone
File: `relics\PhilosopherStone.java`

### updateDescription(AbstractPlayer.PlayerClass c)

**Creates:**
- `PowerTip` — `new PowerTip(this.name, this.description)`

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription(AbstractPlayer.PlayerClass c) {
        this.description = this.getUpdatedDescription();
        this.tips.clear();
        this.tips.add(new PowerTip(this.name, this.description));
        this.initializeTips();
    }
```

</details>

### atBattleStart()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(m, this)`
- `StrengthPower` — `new StrengthPower(m, 1)`

**Queue insertion:**
- [TOP] `this.addToTop(new RelicAboveCreatureAction(m, this))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        for (AbstractMonster m : AbstractDungeon.getMonsters().monsters) {
            this.addToTop(new RelicAboveCreatureAction(m, this));
            m.addPower(new StrengthPower(m, 1));
        }
        AbstractDungeon.onModifyPower();
    }
```

</details>

### onSpawnMonster(AbstractMonster monster)

**Creates:**
- `StrengthPower` — `new StrengthPower(monster, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void onSpawnMonster(AbstractMonster monster) {
        monster.addPower(new StrengthPower(monster, 1));
        AbstractDungeon.onModifyPower();
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
- `PhilosopherStone` — `new PhilosopherStone()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new PhilosopherStone();
    }
```

</details>

## Pocketwatch
File: `relics\Pocketwatch.java`

### atBattleStart()

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.counter = 0;
        this.firstTurn = true;
    }
```

</details>

### onPlayCard(AbstractCard card, AbstractMonster m)

<details><summary>Full body</summary>

```java
@Override
    public void onPlayCard(AbstractCard card, AbstractMonster m) {
        ++this.counter;
        if (this.counter > 3) {
            this.stopPulse();
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
        this.stopPulse();
    }
```

</details>

### makeCopy()

**Creates:**
- `Pocketwatch` — `new Pocketwatch()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Pocketwatch();
    }
```

</details>

## PotionBelt
File: `relics\PotionBelt.java`

### onEquip()

**Creates:**
- `PotionSlot` — `new PotionSlot(AbstractDungeon.player.potionSlots - 2)`
- `PotionSlot` — `new PotionSlot(AbstractDungeon.player.potionSlots - 1)`

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        AbstractDungeon.player.potionSlots += 2;
        AbstractDungeon.player.potions.add(new PotionSlot(AbstractDungeon.player.potionSlots - 2));
        AbstractDungeon.player.potions.add(new PotionSlot(AbstractDungeon.player.potionSlots - 1));
    }
```

</details>

### makeCopy()

**Creates:**
- `PotionBelt` — `new PotionBelt()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new PotionBelt();
    }
```

</details>

## PrayerWheel
File: `relics\PrayerWheel.java`

### makeCopy()

**Creates:**
- `PrayerWheel` — `new PrayerWheel()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new PrayerWheel();
    }
```

</details>

## PreservedInsect
File: `relics\PreservedInsect.java`

### atBattleStart()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

**Queue insertion:**
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        if (AbstractDungeon.getCurrRoom().eliteTrigger) {
            this.flash();
            for (AbstractMonster m : AbstractDungeon.getCurrRoom().monsters.monsters) {
                if (m.currentHealth <= (int)((float)m.maxHealth * (1.0f - this.MODIFIER_AMT))) continue;
                m.currentHealth = (int)((float)m.maxHealth * (1.0f - this.MODIFIER_AMT));
                m.healthBarUpdatedEvent();
            }
            this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `PreservedInsect` — `new PreservedInsect()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new PreservedInsect();
    }
```

</details>

## PrismaticShard
File: `relics\PrismaticShard.java`

### makeCopy()

**Creates:**
- `PrismaticShard` — `new PrismaticShard()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new PrismaticShard();
    }
```

</details>

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        if (AbstractDungeon.player.chosenClass != AbstractPlayer.PlayerClass.DEFECT && AbstractDungeon.player.masterMaxOrbs == 0) {
            AbstractDungeon.player.masterMaxOrbs = 1;
        }
    }
```

</details>

## PureWater
File: `relics\PureWater.java`

### atBattleStartPreDraw()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `MakeTempCardInHandAction` — `new MakeTempCardInHandAction((AbstractCard)new Miracle(), 1, false)`
- `Miracle` — `new Miracle()`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new MakeTempCardInHandAction((AbstractCard)new Miracle(), 1, false))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStartPreDraw() {
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new MakeTempCardInHandAction((AbstractCard)new Miracle(), 1, false));
    }
```

</details>

### makeCopy()

**Creates:**
- `PureWater` — `new PureWater()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new PureWater();
    }
```

</details>

