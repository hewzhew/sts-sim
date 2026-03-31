# Relics: A

7 relics

## Abacus
File: `relics\Abacus.java`

### onShuffle()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)AbstractDungeon.player, AbstractDungeon.player, 6)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new GainBlockAction((AbstractCreature)AbstractDungeon.player, AbstractDungeon.player, 6))`

<details><summary>Full body</summary>

```java
@Override
    public void onShuffle() {
        this.flash();
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new GainBlockAction((AbstractCreature)AbstractDungeon.player, AbstractDungeon.player, 6));
    }
```

</details>

### makeCopy()

**Creates:**
- `Abacus` — `new Abacus()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Abacus();
    }
```

</details>

## AbstractRelic
File: `relics\AbstractRelic.java`

### updateDescription(AbstractPlayer.PlayerClass c)

<details><summary>Full body</summary>

```java
public void updateDescription(AbstractPlayer.PlayerClass c) {
    }
```

</details>

### onEvokeOrb(AbstractOrb ammo)

<details><summary>Full body</summary>

```java
public void onEvokeOrb(AbstractOrb ammo) {
    }
```

</details>

### onPlayCard(AbstractCard c, AbstractMonster m)

<details><summary>Full body</summary>

```java
public void onPlayCard(AbstractCard c, AbstractMonster m) {
    }
```

</details>

### onObtainCard(AbstractCard c)

<details><summary>Full body</summary>

```java
public void onObtainCard(AbstractCard c) {
    }
```

</details>

### onEquip()

<details><summary>Full body</summary>

```java
public void onEquip() {
    }
```

</details>

### onUnequip()

<details><summary>Full body</summary>

```java
public void onUnequip() {
    }
```

</details>

### atPreBattle()

<details><summary>Full body</summary>

```java
public void atPreBattle() {
    }
```

</details>

### atBattleStart()

<details><summary>Full body</summary>

```java
public void atBattleStart() {
    }
```

</details>

### onSpawnMonster(AbstractMonster monster)

<details><summary>Full body</summary>

```java
public void onSpawnMonster(AbstractMonster monster) {
    }
```

</details>

### atBattleStartPreDraw()

<details><summary>Full body</summary>

```java
public void atBattleStartPreDraw() {
    }
```

</details>

### onPlayerEndTurn()

<details><summary>Full body</summary>

```java
public void onPlayerEndTurn() {
    }
```

</details>

### onManualDiscard()

<details><summary>Full body</summary>

```java
public void onManualDiscard() {
    }
```

</details>

### onUseCard(AbstractCard targetCard, UseCardAction useCardAction)

<details><summary>Full body</summary>

```java
public void onUseCard(AbstractCard targetCard, UseCardAction useCardAction) {
    }
```

</details>

### onVictory()

<details><summary>Full body</summary>

```java
public void onVictory() {
    }
```

</details>

### onMonsterDeath(AbstractMonster m)

<details><summary>Full body</summary>

```java
public void onMonsterDeath(AbstractMonster m) {
    }
```

</details>

### onBlockBroken(AbstractCreature m)

<details><summary>Full body</summary>

```java
public void onBlockBroken(AbstractCreature m) {
    }
```

</details>

### onPlayerGainBlock(int blockAmount)

<details><summary>Full body</summary>

```java
public int onPlayerGainBlock(int blockAmount) {
        return blockAmount;
    }
```

</details>

### onPlayerGainedBlock(float blockAmount)

<details><summary>Full body</summary>

```java
public int onPlayerGainedBlock(float blockAmount) {
        return MathUtils.floor(blockAmount);
    }
```

</details>

### onPlayerHeal(int healAmount)

<details><summary>Full body</summary>

```java
public int onPlayerHeal(int healAmount) {
        return healAmount;
    }
```

</details>

### onEnterRestRoom()

<details><summary>Full body</summary>

```java
public void onEnterRestRoom() {
    }
```

</details>

### onShuffle()

<details><summary>Full body</summary>

```java
public void onShuffle() {
    }
```

</details>

### onSmith()

<details><summary>Full body</summary>

```java
public void onSmith() {
    }
```

</details>

### onAttack(DamageInfo info, int damageAmount, AbstractCreature target)

<details><summary>Full body</summary>

```java
public void onAttack(DamageInfo info, int damageAmount, AbstractCreature target) {
    }
```

</details>

### onAttacked(DamageInfo info, int damageAmount)

<details><summary>Full body</summary>

```java
public int onAttacked(DamageInfo info, int damageAmount) {
        return damageAmount;
    }
```

</details>

### onAttackedToChangeDamage(DamageInfo info, int damageAmount)

<details><summary>Full body</summary>

```java
public int onAttackedToChangeDamage(DamageInfo info, int damageAmount) {
        return damageAmount;
    }
```

</details>

### onExhaust(AbstractCard card)

<details><summary>Full body</summary>

```java
public void onExhaust(AbstractCard card) {
    }
```

</details>

### onTrigger()

<details><summary>Full body</summary>

```java
public void onTrigger() {
    }
```

</details>

### onTrigger(AbstractCreature target)

<details><summary>Full body</summary>

```java
public void onTrigger(AbstractCreature target) {
    }
```

</details>

### onEnterRoom(AbstractRoom room)

<details><summary>Full body</summary>

```java
public void onEnterRoom(AbstractRoom room) {
    }
```

</details>

### justEnteredRoom(AbstractRoom room)

<details><summary>Full body</summary>

```java
public void justEnteredRoom(AbstractRoom room) {
    }
```

</details>

### onCardDraw(AbstractCard drawnCard)

<details><summary>Full body</summary>

```java
public void onCardDraw(AbstractCard drawnCard) {
    }
```

</details>

### onChestOpen(boolean bossChest)

<details><summary>Full body</summary>

```java
public void onChestOpen(boolean bossChest) {
    }
```

</details>

### onDrawOrDiscard()

<details><summary>Full body</summary>

```java
public void onDrawOrDiscard() {
    }
```

</details>

### onMasterDeckChange()

<details><summary>Full body</summary>

```java
public void onMasterDeckChange() {
    }
```

</details>

### makeCopy()

<details><summary>Full body</summary>

```java
public abstract AbstractRelic makeCopy();
```

</details>

### onChangeStance(AbstractStance prevStance, AbstractStance newStance)

<details><summary>Full body</summary>

```java
public void onChangeStance(AbstractStance prevStance, AbstractStance newStance) {
    }
```

</details>

### onLoseHp(int damageAmount)

<details><summary>Full body</summary>

```java
public void onLoseHp(int damageAmount) {
    }
```

</details>

### wasHPLost(int damageAmount)

<details><summary>Full body</summary>

```java
public void wasHPLost(int damageAmount) {
    }
```

</details>

## Akabeko
File: `relics\Akabeko.java`

### atBattleStart()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new VigorPower(AbstractDungeon.player, 8), 8)`
- `VigorPower` — `new VigorPower(AbstractDungeon.player, 8)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

**Queue insertion:**
- [TOP] `this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new VigorPower(AbstractDungeon.player, 8), 8))`
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new VigorPower(AbstractDungeon.player, 8), 8));
        this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
    }
```

</details>

### makeCopy()

**Creates:**
- `Akabeko` — `new Akabeko()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Akabeko();
    }
```

</details>

## Anchor
File: `relics\Anchor.java`

### atBattleStart()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)AbstractDungeon.player, AbstractDungeon.player, 10)`

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new GainBlockAction((AbstractCreature)AbstractDungeon.player, AbstractDungeon.player, 10))`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new GainBlockAction((AbstractCreature)AbstractDungeon.player, AbstractDungeon.player, 10));
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
- `Anchor` — `new Anchor()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Anchor();
    }
```

</details>

## AncientTeaSet
File: `relics\AncientTeaSet.java`

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

### onEnterRestRoom()

<details><summary>Full body</summary>

```java
@Override
    public void onEnterRestRoom() {
        this.flash();
        this.counter = -2;
        this.pulse = true;
    }
```

</details>

### makeCopy()

**Creates:**
- `AncientTeaSet` — `new AncientTeaSet()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new AncientTeaSet();
    }
```

</details>

## ArtOfWar
File: `relics\ArtOfWar.java`

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
        this.flash();
        this.firstTurn = true;
        this.gainEnergyNext = true;
        if (!this.pulse) {
            this.beginPulse();
            this.pulse = true;
        }
    }
```

</details>

### onUseCard(AbstractCard card, UseCardAction action)

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.ATTACK) {
            this.gainEnergyNext = false;
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
        this.pulse = false;
    }
```

</details>

### makeCopy()

**Creates:**
- `ArtOfWar` — `new ArtOfWar()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new ArtOfWar();
    }
```

</details>

## Astrolabe
File: `relics\Astrolabe.java`

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
        CardGroup tmp = new CardGroup(CardGroup.CardGroupType.UNSPECIFIED);
        for (AbstractCard card : AbstractDungeon.player.masterDeck.getPurgeableCards().group) {
            tmp.addToTop(card);
        }
        if (tmp.group.isEmpty()) {
            this.cardsSelected = true;
            return;
        }
        if (tmp.group.size() <= 3) {
            this.giveCards(tmp.group);
        } else if (!AbstractDungeon.isScreenUp) {
            AbstractDungeon.gridSelectScreen.open(tmp, 3, this.DESCRIPTIONS[1] + this.name + LocalizedStrings.PERIOD, false, false, false, false);
        } else {
            AbstractDungeon.dynamicBanner.hide();
            AbstractDungeon.previousScreen = AbstractDungeon.screen;
            AbstractDungeon.gridSelectScreen.open(tmp, 3, this.DESCRIPTIONS[1] + this.name + LocalizedStrings.PERIOD, false, false, false, false);
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `Astrolabe` — `new Astrolabe()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Astrolabe();
    }
```

</details>

