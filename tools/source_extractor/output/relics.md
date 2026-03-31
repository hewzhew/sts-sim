# StS Relic Reference

Total Relic subclasses: 191

## Abacus
File: `relics\Abacus.java`

### onShuffle()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)AbstractDungeon.player, AbstractDungeon.player, 6)`

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

## BagOfMarbles
File: `relics\BagOfMarbles.java`

### atBattleStart()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(mo, this)`
- `ApplyPowerAction` — `new ApplyPowerAction((AbstractCreature)mo, (AbstractCreature)AbstractDungeon.player, (AbstractPower)new VulnerablePower(mo, 1, false), 1, true)`
- `VulnerablePower` — `new VulnerablePower(mo, 1, false)`

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

## Calipers
File: `relics\Calipers.java`

### makeCopy()

**Creates:**
- `Calipers` — `new Calipers()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Calipers();
    }
```

</details>

## CallingBell
File: `relics\CallingBell.java`

### onEquip()

**Creates:**
- `CardGroup` — `new CardGroup(CardGroup.CardGroupType.UNSPECIFIED)`
- `CurseOfTheBell` — `new CurseOfTheBell()`

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        this.cardsReceived = false;
        CardGroup group = new CardGroup(CardGroup.CardGroupType.UNSPECIFIED);
        CurseOfTheBell bellCurse = new CurseOfTheBell();
        UnlockTracker.markCardAsSeen(bellCurse.cardID);
        group.addToBottom(((AbstractCard)bellCurse).makeCopy());
        AbstractDungeon.gridSelectScreen.openConfirmationGrid(group, this.DESCRIPTIONS[1]);
        CardCrawlGame.sound.playA("BELL", MathUtils.random(-0.2f, -0.3f));
    }
```

</details>

### makeCopy()

**Creates:**
- `CallingBell` — `new CallingBell()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new CallingBell();
    }
```

</details>

## CaptainsWheel
File: `relics\CaptainsWheel.java`

### atBattleStart()

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.counter = 0;
    }
```

</details>

### onVictory()

<details><summary>Full body</summary>

```java
@Override
    public void onVictory() {
        this.counter = -1;
        this.grayscale = false;
    }
```

</details>

### makeCopy()

**Creates:**
- `CaptainsWheel` — `new CaptainsWheel()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new CaptainsWheel();
    }
```

</details>

## Cauldron
File: `relics\Cauldron.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        for (int i = 0; i < 5; ++i) {
            AbstractDungeon.getCurrRoom().addPotionToRewards(PotionHelper.getRandomPotion());
        }
        AbstractDungeon.combatRewardScreen.open(this.DESCRIPTIONS[1]);
        AbstractDungeon.getCurrRoom().rewardPopOutTimer = 0.0f;
        int remove = -1;
        for (int i = 0; i < AbstractDungeon.combatRewardScreen.rewards.size(); ++i) {
            if (AbstractDungeon.combatRewardScreen.rewards.get((int)i).type != RewardItem.RewardType.CARD) continue;
            remove = i;
            break;
        }
        if (remove != -1) {
            AbstractDungeon.combatRewardScreen.rewards.remove(remove);
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `Cauldron` — `new Cauldron()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Cauldron();
    }
```

</details>

## CentennialPuzzle
File: `relics\CentennialPuzzle.java`

### atPreBattle()

<details><summary>Full body</summary>

```java
@Override
    public void atPreBattle() {
        usedThisCombat = false;
        this.pulse = true;
        this.beginPulse();
    }
```

</details>

### wasHPLost(int damageAmount)

**Creates:**
- `DrawCardAction` — `new DrawCardAction(AbstractDungeon.player, 3)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

<details><summary>Full body</summary>

```java
@Override
    public void wasHPLost(int damageAmount) {
        if (damageAmount > 0 && AbstractDungeon.getCurrRoom().phase == AbstractRoom.RoomPhase.COMBAT && !usedThisCombat) {
            this.flash();
            this.pulse = false;
            this.addToTop(new DrawCardAction(AbstractDungeon.player, 3));
            this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
            usedThisCombat = true;
            this.grayscale = true;
        }
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
- `CentennialPuzzle` — `new CentennialPuzzle()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new CentennialPuzzle();
    }
```

</details>

## CeramicFish
File: `relics\CeramicFish.java`

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

### onObtainCard(AbstractCard c)

<details><summary>Full body</summary>

```java
@Override
    public void onObtainCard(AbstractCard c) {
        AbstractDungeon.player.gainGold(9);
    }
```

</details>

### makeCopy()

**Creates:**
- `CeramicFish` — `new CeramicFish()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new CeramicFish();
    }
```

</details>

## ChampionsBelt
File: `relics\ChampionsBelt.java`

### onTrigger(AbstractCreature target)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `ApplyPowerAction` — `new ApplyPowerAction(target, AbstractDungeon.player, new WeakPower(target, 1, false), 1)`
- `WeakPower` — `new WeakPower(target, 1, false)`

<details><summary>Full body</summary>

```java
@Override
    public void onTrigger(AbstractCreature target) {
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new ApplyPowerAction(target, AbstractDungeon.player, new WeakPower(target, 1, false), 1));
    }
```

</details>

### makeCopy()

**Creates:**
- `ChampionsBelt` — `new ChampionsBelt()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new ChampionsBelt();
    }
```

</details>

## CharonsAshes
File: `relics\CharonsAshes.java`

### onExhaust(AbstractCard card)

**Creates:**
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(3, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.FIRE)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(mo, this)`

<details><summary>Full body</summary>

```java
@Override
    public void onExhaust(AbstractCard card) {
        this.flash();
        this.addToTop(new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(3, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.FIRE));
        for (AbstractMonster mo : AbstractDungeon.getCurrRoom().monsters.monsters) {
            if (mo.isDead) continue;
            this.addToTop(new RelicAboveCreatureAction(mo, this));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `CharonsAshes` — `new CharonsAshes()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new CharonsAshes();
    }
```

</details>

## ChemicalX
File: `relics\ChemicalX.java`

### makeCopy()

**Creates:**
- `ChemicalX` — `new ChemicalX()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new ChemicalX();
    }
```

</details>

## Circlet
File: `relics\Circlet.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        this.flash();
    }
```

</details>

### onUnequip()

<details><summary>Full body</summary>

```java
@Override
    public void onUnequip() {
    }
```

</details>

### makeCopy()

**Creates:**
- `Circlet` — `new Circlet()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Circlet();
    }
```

</details>

## CloakClasp
File: `relics\CloakClasp.java`

### onPlayerEndTurn()

**Creates:**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)AbstractDungeon.player, null, AbstractDungeon.player.hand.group.size() * 1)`

<details><summary>Full body</summary>

```java
@Override
    public void onPlayerEndTurn() {
        if (!AbstractDungeon.player.hand.group.isEmpty()) {
            this.flash();
            this.addToBot(new GainBlockAction((AbstractCreature)AbstractDungeon.player, null, AbstractDungeon.player.hand.group.size() * 1));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `CloakClasp` — `new CloakClasp()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new CloakClasp();
    }
```

</details>

## ClockworkSouvenir
File: `relics\ClockworkSouvenir.java`

### atBattleStart()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new ArtifactPower(AbstractDungeon.player, 1), 1)`
- `ArtifactPower` — `new ArtifactPower(AbstractDungeon.player, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new ArtifactPower(AbstractDungeon.player, 1), 1));
    }
```

</details>

### makeCopy()

**Creates:**
- `ClockworkSouvenir` — `new ClockworkSouvenir()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new ClockworkSouvenir();
    }
```

</details>

## CoffeeDripper
File: `relics\CoffeeDripper.java`

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
- `CoffeeDripper` — `new CoffeeDripper()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new CoffeeDripper();
    }
```

</details>

## Courier
File: `relics\Courier.java`

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
- `Courier` — `new Courier()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Courier();
    }
```

</details>

## CrackedCore
File: `relics\CrackedCore.java`

### atPreBattle()

**Creates:**
- `Lightning` — `new Lightning()`

<details><summary>Full body</summary>

```java
@Override
    public void atPreBattle() {
        AbstractDungeon.player.channelOrb(new Lightning());
    }
```

</details>

### makeCopy()

**Creates:**
- `CrackedCore` — `new CrackedCore()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new CrackedCore();
    }
```

</details>

## CultistMask
File: `relics\CultistMask.java`

### atBattleStart()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `SFXAction` — `new SFXAction("VO_CULTIST_1A")`
- `TalkAction` — `new TalkAction(true, this.DESCRIPTIONS[1], 1.0f, 2.0f)`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new SFXAction("VO_CULTIST_1A"));
        this.addToBot(new TalkAction(true, this.DESCRIPTIONS[1], 1.0f, 2.0f));
    }
```

</details>

### makeCopy()

**Creates:**
- `CultistMask` — `new CultistMask()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new CultistMask();
    }
```

</details>

## CursedKey
File: `relics\CursedKey.java`

### justEnteredRoom(AbstractRoom room)

<details><summary>Full body</summary>

```java
@Override
    public void justEnteredRoom(AbstractRoom room) {
        if (room instanceof TreasureRoom) {
            this.flash();
            this.pulse = true;
        } else {
            this.pulse = false;
        }
    }
```

</details>

### onChestOpen(boolean bossChest)

**Creates:**
- `ShowCardAndObtainEffect` — `new ShowCardAndObtainEffect(AbstractDungeon.returnRandomCurse(), Settings.WIDTH / 2, Settings.HEIGHT / 2)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

<details><summary>Full body</summary>

```java
@Override
    public void onChestOpen(boolean bossChest) {
        if (!bossChest) {
            AbstractDungeon.topLevelEffects.add(new ShowCardAndObtainEffect(AbstractDungeon.returnRandomCurse(), Settings.WIDTH / 2, Settings.HEIGHT / 2));
            this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        }
    }
```

</details>

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
- `CursedKey` — `new CursedKey()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new CursedKey();
    }
```

</details>

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

## FaceOfCleric
File: `relics\FaceOfCleric.java`

### onVictory()

<details><summary>Full body</summary>

```java
@Override
    public void onVictory() {
        this.flash();
        AbstractDungeon.player.increaseMaxHp(1, true);
    }
```

</details>

### makeCopy()

**Creates:**
- `FaceOfCleric` — `new FaceOfCleric()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new FaceOfCleric();
    }
```

</details>

## FossilizedHelix
File: `relics\FossilizedHelix.java`

### atBattleStart()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new BufferPower(AbstractDungeon.player, 1), 1)`
- `BufferPower` — `new BufferPower(AbstractDungeon.player, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new BufferPower(AbstractDungeon.player, 1), 1));
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
- `FossilizedHelix` — `new FossilizedHelix()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new FossilizedHelix();
    }
```

</details>

## FrozenCore
File: `relics\FrozenCore.java`

### onPlayerEndTurn()

**Creates:**
- `Frost` — `new Frost()`

<details><summary>Full body</summary>

```java
@Override
    public void onPlayerEndTurn() {
        if (AbstractDungeon.player.hasEmptyOrb()) {
            this.flash();
            AbstractDungeon.player.channelOrb(new Frost());
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `FrozenCore` — `new FrozenCore()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new FrozenCore();
    }
```

</details>

## FrozenEgg2
File: `relics\FrozenEgg2.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        for (RewardItem reward : AbstractDungeon.combatRewardScreen.rewards) {
            if (reward.cards == null) continue;
            for (AbstractCard c : reward.cards) {
                this.onPreviewObtainCard(c);
            }
        }
    }
```

</details>

### onObtainCard(AbstractCard c)

<details><summary>Full body</summary>

```java
@Override
    public void onObtainCard(AbstractCard c) {
        if (c.type == AbstractCard.CardType.POWER && c.canUpgrade() && !c.upgraded) {
            c.upgrade();
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `FrozenEgg2` — `new FrozenEgg2()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new FrozenEgg2();
    }
```

</details>

## FrozenEye
File: `relics\FrozenEye.java`

### makeCopy()

**Creates:**
- `FrozenEye` — `new FrozenEye()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new FrozenEye();
    }
```

</details>

## FusionHammer
File: `relics\FusionHammer.java`

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
- `FusionHammer` — `new FusionHammer()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new FusionHammer();
    }
```

</details>

## GamblingChip
File: `relics\GamblingChip.java`

### atBattleStartPreDraw()

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStartPreDraw() {
        this.activated = false;
    }
```

</details>

### makeCopy()

**Creates:**
- `GamblingChip` — `new GamblingChip()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new GamblingChip();
    }
```

</details>

## Ginger
File: `relics\Ginger.java`

### makeCopy()

**Creates:**
- `Ginger` — `new Ginger()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Ginger();
    }
```

</details>

## Girya
File: `relics\Girya.java`

### atBattleStart()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new StrengthPower(AbstractDungeon.player, this.counter), this.counter)`
- `StrengthPower` — `new StrengthPower(AbstractDungeon.player, this.counter)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        if (this.counter != 0) {
            this.flash();
            this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new StrengthPower(AbstractDungeon.player, this.counter), this.counter));
            this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `Girya` — `new Girya()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Girya();
    }
```

</details>

## GoldPlatedCables
File: `relics\GoldPlatedCables.java`

### makeCopy()

**Creates:**
- `GoldPlatedCables` — `new GoldPlatedCables()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new GoldPlatedCables();
    }
```

</details>

## GoldenEye
File: `relics\GoldenEye.java`

### makeCopy()

**Creates:**
- `GoldenEye` — `new GoldenEye()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new GoldenEye();
    }
```

</details>

## GoldenIdol
File: `relics\GoldenIdol.java`

### makeCopy()

**Creates:**
- `GoldenIdol` — `new GoldenIdol()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new GoldenIdol();
    }
```

</details>

## GremlinHorn
File: `relics\GremlinHorn.java`

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

### onMonsterDeath(AbstractMonster m)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(m, this)`
- `GainEnergyAction` — `new GainEnergyAction(1)`
- `DrawCardAction` — `new DrawCardAction(AbstractDungeon.player, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void onMonsterDeath(AbstractMonster m) {
        if (m.currentHealth == 0 && !AbstractDungeon.getMonsters().areMonstersBasicallyDead()) {
            this.flash();
            this.addToBot(new RelicAboveCreatureAction(m, this));
            this.addToBot(new GainEnergyAction(1));
            this.addToBot(new DrawCardAction(AbstractDungeon.player, 1));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `GremlinHorn` — `new GremlinHorn()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new GremlinHorn();
    }
```

</details>

## GremlinMask
File: `relics\GremlinMask.java`

### atBattleStart()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new WeakPower(AbstractDungeon.player, 1, false), 1)`
- `WeakPower` — `new WeakPower(AbstractDungeon.player, 1, false)`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new WeakPower(AbstractDungeon.player, 1, false), 1));
    }
```

</details>

### makeCopy()

**Creates:**
- `GremlinMask` — `new GremlinMask()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new GremlinMask();
    }
```

</details>

## HandDrill
File: `relics\HandDrill.java`

### onBlockBroken(AbstractCreature m)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(m, this)`
- `ApplyPowerAction` — `new ApplyPowerAction(m, AbstractDungeon.player, new VulnerablePower(m, 2, false), 2)`
- `VulnerablePower` — `new VulnerablePower(m, 2, false)`

<details><summary>Full body</summary>

```java
@Override
    public void onBlockBroken(AbstractCreature m) {
        this.flash();
        this.addToBot(new RelicAboveCreatureAction(m, this));
        this.addToBot(new ApplyPowerAction(m, AbstractDungeon.player, new VulnerablePower(m, 2, false), 2));
    }
```

</details>

### makeCopy()

**Creates:**
- `HandDrill` — `new HandDrill()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new HandDrill();
    }
```

</details>

## HappyFlower
File: `relics\HappyFlower.java`

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
        this.counter = 0;
    }
```

</details>

### makeCopy()

**Creates:**
- `HappyFlower` — `new HappyFlower()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new HappyFlower();
    }
```

</details>

## HolyWater
File: `relics\HolyWater.java`

### atBattleStartPreDraw()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `MakeTempCardInHandAction` — `new MakeTempCardInHandAction((AbstractCard)new Miracle(), 3, false)`
- `Miracle` — `new Miracle()`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStartPreDraw() {
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new MakeTempCardInHandAction((AbstractCard)new Miracle(), 3, false));
    }
```

</details>

### makeCopy()

**Creates:**
- `HolyWater` — `new HolyWater()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new HolyWater();
    }
```

</details>

## HornCleat
File: `relics\HornCleat.java`

### atBattleStart()

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.counter = 0;
    }
```

</details>

### onVictory()

<details><summary>Full body</summary>

```java
@Override
    public void onVictory() {
        this.counter = -1;
        this.grayscale = false;
    }
```

</details>

### makeCopy()

**Creates:**
- `HornCleat` — `new HornCleat()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new HornCleat();
    }
```

</details>

## HoveringKite
File: `relics\HoveringKite.java`

### onManualDiscard()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `GainEnergyAction` — `new GainEnergyAction(1)`

<details><summary>Full body</summary>

```java
@Override
    public void onManualDiscard() {
        if (!this.triggeredThisTurn) {
            this.triggeredThisTurn = true;
            this.flash();
            this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
            this.addToBot(new GainEnergyAction(1));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `HoveringKite` — `new HoveringKite()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new HoveringKite();
    }
```

</details>

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

## JuzuBracelet
File: `relics\JuzuBracelet.java`

### makeCopy()

**Creates:**
- `JuzuBracelet` — `new JuzuBracelet()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new JuzuBracelet();
    }
```

</details>

## Kunai
File: `relics\Kunai.java`

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new DexterityPower(AbstractDungeon.player, 1), 1)`
- `DexterityPower` — `new DexterityPower(AbstractDungeon.player, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.ATTACK) {
            ++this.counter;
            if (this.counter % 3 == 0) {
                this.counter = 0;
                this.flash();
                this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
                this.addToBot(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new DexterityPower(AbstractDungeon.player, 1), 1));
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
- `Kunai` — `new Kunai()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Kunai();
    }
```

</details>

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

## MagicFlower
File: `relics\MagicFlower.java`

### onPlayerHeal(int healAmount)

<details><summary>Full body</summary>

```java
@Override
    public int onPlayerHeal(int healAmount) {
        if (AbstractDungeon.currMapNode != null && AbstractDungeon.getCurrRoom().phase == AbstractRoom.RoomPhase.COMBAT) {
            this.flash();
            return MathUtils.round((float)healAmount * 1.5f);
        }
        return healAmount;
    }
```

</details>

### makeCopy()

**Creates:**
- `MagicFlower` — `new MagicFlower()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new MagicFlower();
    }
```

</details>

## Mango
File: `relics\Mango.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        AbstractDungeon.player.increaseMaxHp(14, true);
    }
```

</details>

### makeCopy()

**Creates:**
- `Mango` — `new Mango()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Mango();
    }
```

</details>

## MarkOfPain
File: `relics\MarkOfPain.java`

### atBattleStart()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `MakeTempCardInDrawPileAction` — `new MakeTempCardInDrawPileAction(new Wound(), 2, true, true)`
- `Wound` — `new Wound()`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new MakeTempCardInDrawPileAction(new Wound(), 2, true, true));
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
- `MarkOfPain` — `new MarkOfPain()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new MarkOfPain();
    }
```

</details>

## MarkOfTheBloom
File: `relics\MarkOfTheBloom.java`

### onPlayerHeal(int healAmount)

<details><summary>Full body</summary>

```java
@Override
    public int onPlayerHeal(int healAmount) {
        this.flash();
        return 0;
    }
```

</details>

### makeCopy()

**Creates:**
- `MarkOfTheBloom` — `new MarkOfTheBloom()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new MarkOfTheBloom();
    }
```

</details>

## Matryoshka
File: `relics\Matryoshka.java`

### onChestOpen(boolean bossChest)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

<details><summary>Full body</summary>

```java
@Override
    public void onChestOpen(boolean bossChest) {
        if (!bossChest && this.counter > 0) {
            --this.counter;
            this.flash();
            this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
            if (AbstractDungeon.relicRng.randomBoolean(0.75f)) {
                AbstractDungeon.getCurrRoom().addRelicToRewards(AbstractRelic.RelicTier.COMMON);
            } else {
                AbstractDungeon.getCurrRoom().addRelicToRewards(AbstractRelic.RelicTier.UNCOMMON);
            }
            if (this.counter == 0) {
                this.setCounter(-2);
                this.description = this.DESCRIPTIONS[2];
            } else {
                this.description = this.DESCRIPTIONS[1];
            }
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `Matryoshka` — `new Matryoshka()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Matryoshka();
    }
```

</details>

## MawBank
File: `relics\MawBank.java`

### onEnterRoom(AbstractRoom room)

<details><summary>Full body</summary>

```java
@Override
    public void onEnterRoom(AbstractRoom room) {
        if (!this.usedUp) {
            this.flash();
            AbstractDungeon.player.gainGold(12);
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `MawBank` — `new MawBank()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new MawBank();
    }
```

</details>

## MealTicket
File: `relics\MealTicket.java`

### justEnteredRoom(AbstractRoom room)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

<details><summary>Full body</summary>

```java
@Override
    public void justEnteredRoom(AbstractRoom room) {
        if (room instanceof ShopRoom) {
            this.flash();
            this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
            AbstractDungeon.player.heal(15);
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `MealTicket` — `new MealTicket()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new MealTicket();
    }
```

</details>

## MeatOnTheBone
File: `relics\MeatOnTheBone.java`

### onTrigger()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

<details><summary>Full body</summary>

```java
@Override
    public void onTrigger() {
        AbstractPlayer p = AbstractDungeon.player;
        if ((float)p.currentHealth <= (float)p.maxHealth / 2.0f && p.currentHealth > 0) {
            this.flash();
            this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
            p.heal(12);
            this.stopPulse();
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `MeatOnTheBone` — `new MeatOnTheBone()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new MeatOnTheBone();
    }
```

</details>

## MedicalKit
File: `relics\MedicalKit.java`

### makeCopy()

**Creates:**
- `MedicalKit` — `new MedicalKit()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new MedicalKit();
    }
```

</details>

### onUseCard(AbstractCard card, UseCardAction action)

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.STATUS) {
            AbstractDungeon.player.getRelic(ID).flash();
            card.exhaust = true;
            action.exhaustCard = true;
        }
    }
```

</details>

## Melange
File: `relics\Melange.java`

### onShuffle()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `ScryAction` — `new ScryAction(3)`

<details><summary>Full body</summary>

```java
@Override
    public void onShuffle() {
        this.flash();
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new ScryAction(3));
    }
```

</details>

### makeCopy()

**Creates:**
- `Melange` — `new Melange()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Melange();
    }
```

</details>

## MembershipCard
File: `relics\MembershipCard.java`

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
- `MembershipCard` — `new MembershipCard()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new MembershipCard();
    }
```

</details>

## MercuryHourglass
File: `relics\MercuryHourglass.java`

### makeCopy()

**Creates:**
- `MercuryHourglass` — `new MercuryHourglass()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new MercuryHourglass();
    }
```

</details>

## MoltenEgg2
File: `relics\MoltenEgg2.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        for (RewardItem reward : AbstractDungeon.combatRewardScreen.rewards) {
            if (reward.cards == null) continue;
            for (AbstractCard c : reward.cards) {
                this.onPreviewObtainCard(c);
            }
        }
    }
```

</details>

### onObtainCard(AbstractCard c)

<details><summary>Full body</summary>

```java
@Override
    public void onObtainCard(AbstractCard c) {
        if (c.type == AbstractCard.CardType.ATTACK && c.canUpgrade() && !c.upgraded) {
            c.upgrade();
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `MoltenEgg2` — `new MoltenEgg2()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new MoltenEgg2();
    }
```

</details>

## MummifiedHand
File: `relics\MummifiedHand.java`

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `None` — `new ArrayList<AbstractCard>()`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.POWER) {
            this.flash();
            this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
            ArrayList<AbstractCard> groupCopy = new ArrayList<AbstractCard>();
            for (AbstractCard c : AbstractDungeon.player.hand.group) {
                if (c.cost > 0 && c.costForTurn > 0 && !c.freeToPlayOnce) {
                    groupCopy.add(c);
                    continue;
                }
                logger.info("COST IS 0: " + c.name);
            }
            for (CardQueueItem i : AbstractDungeon.actionManager.cardQueue) {
                if (i.card == null) continue;
                logger.info("INVALID: " + i.card.name);
                groupCopy.remove(i.card);
            }
            AbstractCard c = null;
            if (!groupCopy.isEmpty()) {
                logger.info("VALID CARDS: ");
                for (AbstractCard cc : groupCopy) {
                    logger.info(cc.name);
                }
                c = (AbstractCard)groupCopy.get(AbstractDungeon.cardRandomRng.random(0, groupCopy.size() - 1));
            } else {
                logger.info("NO VALID CARDS");
            }
            if (c != null) {
                logger.info("Mummified hand: " + c.name);
                c.setCostForTurn(0);
            } else {
                logger.info("ERROR: MUMMIFIED HAND NOT WORKING");
            }
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `MummifiedHand` — `new MummifiedHand()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new MummifiedHand();
    }
```

</details>

## MutagenicStrength
File: `relics\MutagenicStrength.java`

### atBattleStart()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new StrengthPower(AbstractDungeon.player, 3), 3)`
- `StrengthPower` — `new StrengthPower(AbstractDungeon.player, 3)`
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new LoseStrengthPower(AbstractDungeon.player, 3), 3)`
- `LoseStrengthPower` — `new LoseStrengthPower(AbstractDungeon.player, 3)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new StrengthPower(AbstractDungeon.player, 3), 3));
        this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new LoseStrengthPower(AbstractDungeon.player, 3), 3));
        this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
    }
```

</details>

### makeCopy()

**Creates:**
- `MutagenicStrength` — `new MutagenicStrength()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new MutagenicStrength();
    }
```

</details>

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

## PandorasBox
File: `relics\PandorasBox.java`

### onEquip()

**Creates:**
- `CardGroup` — `new CardGroup(CardGroup.CardGroupType.UNSPECIFIED)`

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

## QuestionCard
File: `relics\QuestionCard.java`

### makeCopy()

**Creates:**
- `QuestionCard` — `new QuestionCard()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new QuestionCard();
    }
```

</details>

## RedCirclet
File: `relics\RedCirclet.java`

### makeCopy()

**Creates:**
- `RedCirclet` — `new RedCirclet()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new RedCirclet();
    }
```

</details>

## RedMask
File: `relics\RedMask.java`

### atBattleStart()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(mo, this)`
- `ApplyPowerAction` — `new ApplyPowerAction((AbstractCreature)mo, (AbstractCreature)AbstractDungeon.player, (AbstractPower)new WeakPower(mo, 1, false), 1, true)`
- `WeakPower` — `new WeakPower(mo, 1, false)`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        for (AbstractMonster mo : AbstractDungeon.getCurrRoom().monsters.monsters) {
            this.addToBot(new RelicAboveCreatureAction(mo, this));
            this.addToBot(new ApplyPowerAction((AbstractCreature)mo, (AbstractCreature)AbstractDungeon.player, (AbstractPower)new WeakPower(mo, 1, false), 1, true));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `RedMask` — `new RedMask()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new RedMask();
    }
```

</details>

## RedSkull
File: `relics\RedSkull.java`

### atBattleStart()

**Creates:**
- `AbstractGameAction` — `new AbstractGameAction(){

            @Override
            public void update() {
                if (!RedSkull.this.isActive && AbstractDungeon.player.isBloodied) {
                    RedSkull.thi...`
- `StrengthPower` — `new StrengthPower(AbstractDungeon.player, 3)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, RedSkull.this)`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.isActive = false;
        this.addToBot(new AbstractGameAction(){

            @Override
            public void update() {
                if (!RedSkull.this.isActive && AbstractDungeon.player.isBloodied) {
                    RedSkull.this.flash();
                    RedSkull.this.pulse = true;
                    AbstractDungeon.player.addPower(new StrengthPower(AbstractDungeon.player, 3));
                    this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, RedSkull.this));
                    RedSkull.this.isActive = true;
                    AbstractDungeon.onModifyPower();
                }
                this.isDone = true;
            }
        });
    }
```

</details>

### onVictory()

<details><summary>Full body</summary>

```java
@Override
    public void onVictory() {
        this.pulse = false;
        this.isActive = false;
    }
```

</details>

### makeCopy()

**Creates:**
- `RedSkull` — `new RedSkull()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new RedSkull();
    }
```

</details>

## RegalPillow
File: `relics\RegalPillow.java`

### makeCopy()

**Creates:**
- `RegalPillow` — `new RegalPillow()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new RegalPillow();
    }
```

</details>

## RingOfTheSerpent
File: `relics\RingOfTheSerpent.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        ++AbstractDungeon.player.masterHandSize;
    }
```

</details>

### onUnequip()

<details><summary>Full body</summary>

```java
@Override
    public void onUnequip() {
        --AbstractDungeon.player.masterHandSize;
    }
```

</details>

### makeCopy()

**Creates:**
- `RingOfTheSerpent` — `new RingOfTheSerpent()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new RingOfTheSerpent();
    }
```

</details>

## RunicCapacitor
File: `relics\RunicCapacitor.java`

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
- `RunicCapacitor` — `new RunicCapacitor()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new RunicCapacitor();
    }
```

</details>

## RunicCube
File: `relics\RunicCube.java`

### wasHPLost(int damageAmount)

**Creates:**
- `DrawCardAction` — `new DrawCardAction(AbstractDungeon.player, 1)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

<details><summary>Full body</summary>

```java
@Override
    public void wasHPLost(int damageAmount) {
        if (AbstractDungeon.getCurrRoom().phase == AbstractRoom.RoomPhase.COMBAT && damageAmount > 0) {
            this.flash();
            this.addToTop(new DrawCardAction(AbstractDungeon.player, 1));
            this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `RunicCube` — `new RunicCube()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new RunicCube();
    }
```

</details>

## RunicDome
File: `relics\RunicDome.java`

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
- `RunicDome` — `new RunicDome()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new RunicDome();
    }
```

</details>

## RunicPyramid
File: `relics\RunicPyramid.java`

### makeCopy()

**Creates:**
- `RunicPyramid` — `new RunicPyramid()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new RunicPyramid();
    }
```

</details>

## SacredBark
File: `relics\SacredBark.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        for (AbstractPotion p : AbstractDungeon.player.potions) {
            p.initializeData();
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `SacredBark` — `new SacredBark()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new SacredBark();
    }
```

</details>

## SelfFormingClay
File: `relics\SelfFormingClay.java`

### wasHPLost(int damageAmount)

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new NextTurnBlockPower(AbstractDungeon.player, 3, this.name), 3)`
- `NextTurnBlockPower` — `new NextTurnBlockPower(AbstractDungeon.player, 3, this.name)`

<details><summary>Full body</summary>

```java
@Override
    public void wasHPLost(int damageAmount) {
        if (AbstractDungeon.getCurrRoom().phase == AbstractRoom.RoomPhase.COMBAT && damageAmount > 0) {
            this.flash();
            this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new NextTurnBlockPower(AbstractDungeon.player, 3, this.name), 3));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `SelfFormingClay` — `new SelfFormingClay()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new SelfFormingClay();
    }
```

</details>

## Shovel
File: `relics\Shovel.java`

### makeCopy()

**Creates:**
- `Shovel` — `new Shovel()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Shovel();
    }
```

</details>

## Shuriken
File: `relics\Shuriken.java`

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new StrengthPower(AbstractDungeon.player, 1), 1)`
- `StrengthPower` — `new StrengthPower(AbstractDungeon.player, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.ATTACK) {
            ++this.counter;
            if (this.counter % 3 == 0) {
                this.counter = 0;
                this.flash();
                this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
                this.addToBot(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new StrengthPower(AbstractDungeon.player, 1), 1));
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
- `Shuriken` — `new Shuriken()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Shuriken();
    }
```

</details>

## SingingBowl
File: `relics\SingingBowl.java`

### makeCopy()

**Creates:**
- `SingingBowl` — `new SingingBowl()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new SingingBowl();
    }
```

</details>

## SlaversCollar
File: `relics\SlaversCollar.java`

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

### onVictory()

<details><summary>Full body</summary>

```java
@Override
    public void onVictory() {
        if (this.pulse) {
            --AbstractDungeon.player.energy.energyMaster;
            this.stopPulse();
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `SlaversCollar` — `new SlaversCollar()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new SlaversCollar();
    }
```

</details>

## Sling
File: `relics\Sling.java`

### atBattleStart()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new StrengthPower(AbstractDungeon.player, 2), 2)`
- `StrengthPower` — `new StrengthPower(AbstractDungeon.player, 2)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        if (AbstractDungeon.getCurrRoom().eliteTrigger) {
            this.flash();
            this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new StrengthPower(AbstractDungeon.player, 2), 2));
            this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `Sling` — `new Sling()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Sling();
    }
```

</details>

## SmilingMask
File: `relics\SmilingMask.java`

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
- `SmilingMask` — `new SmilingMask()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new SmilingMask();
    }
```

</details>

## SnakeRing
File: `relics\SnakeRing.java`

### atBattleStart()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `DrawCardAction` — `new DrawCardAction(AbstractDungeon.player, 2)`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new DrawCardAction(AbstractDungeon.player, 2));
    }
```

</details>

### makeCopy()

**Creates:**
- `SnakeRing` — `new SnakeRing()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new SnakeRing();
    }
```

</details>

## SneckoEye
File: `relics\SneckoEye.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        AbstractDungeon.player.masterHandSize += 2;
    }
```

</details>

### onUnequip()

<details><summary>Full body</summary>

```java
@Override
    public void onUnequip() {
        AbstractDungeon.player.masterHandSize -= 2;
    }
```

</details>

### atPreBattle()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new ConfusionPower(AbstractDungeon.player))`
- `ConfusionPower` — `new ConfusionPower(AbstractDungeon.player)`

<details><summary>Full body</summary>

```java
@Override
    public void atPreBattle() {
        this.flash();
        this.addToBot(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new ConfusionPower(AbstractDungeon.player)));
    }
```

</details>

### makeCopy()

**Creates:**
- `SneckoEye` — `new SneckoEye()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new SneckoEye();
    }
```

</details>

## SneckoSkull
File: `relics\SneckoSkull.java`

### makeCopy()

**Creates:**
- `SneckoSkull` — `new SneckoSkull()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new SneckoSkull();
    }
```

</details>

## Sozu
File: `relics\Sozu.java`

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
- `Sozu` — `new Sozu()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Sozu();
    }
```

</details>

## SpiritPoop
File: `relics\SpiritPoop.java`

### makeCopy()

**Creates:**
- `SpiritPoop` — `new SpiritPoop()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new SpiritPoop();
    }
```

</details>

## SsserpentHead
File: `relics\SsserpentHead.java`

### onEnterRoom(AbstractRoom room)

<details><summary>Full body</summary>

```java
@Override
    public void onEnterRoom(AbstractRoom room) {
        if (room instanceof EventRoom) {
            this.flash();
            AbstractDungeon.player.gainGold(50);
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `SsserpentHead` — `new SsserpentHead()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new SsserpentHead();
    }
```

</details>

## StoneCalendar
File: `relics\StoneCalendar.java`

### atBattleStart()

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.counter = 0;
    }
```

</details>

### onPlayerEndTurn()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(52, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.BLUNT_HEAVY)`

<details><summary>Full body</summary>

```java
@Override
    public void onPlayerEndTurn() {
        if (this.counter == 7) {
            this.flash();
            this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
            this.addToBot(new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(52, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.BLUNT_HEAVY));
            this.stopPulse();
            this.grayscale = true;
        }
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
- `StoneCalendar` — `new StoneCalendar()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new StoneCalendar();
    }
```

</details>

## StrangeSpoon
File: `relics\StrangeSpoon.java`

### makeCopy()

**Creates:**
- `StrangeSpoon` — `new StrangeSpoon()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new StrangeSpoon();
    }
```

</details>

## Strawberry
File: `relics\Strawberry.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        AbstractDungeon.player.increaseMaxHp(7, true);
    }
```

</details>

### makeCopy()

**Creates:**
- `Strawberry` — `new Strawberry()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Strawberry();
    }
```

</details>

## StrikeDummy
File: `relics\StrikeDummy.java`

### makeCopy()

**Creates:**
- `StrikeDummy` — `new StrikeDummy()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new StrikeDummy();
    }
```

</details>

## Sundial
File: `relics\Sundial.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        this.counter = 0;
    }
```

</details>

### onShuffle()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `GainEnergyAction` — `new GainEnergyAction(2)`

<details><summary>Full body</summary>

```java
@Override
    public void onShuffle() {
        ++this.counter;
        if (this.counter == 3) {
            this.counter = 0;
            this.flash();
            this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
            this.addToBot(new GainEnergyAction(2));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `Sundial` — `new Sundial()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Sundial();
    }
```

</details>

## SymbioticVirus
File: `relics\SymbioticVirus.java`

### atPreBattle()

**Creates:**
- `Dark` — `new Dark()`

<details><summary>Full body</summary>

```java
@Override
    public void atPreBattle() {
        AbstractDungeon.player.channelOrb(new Dark());
    }
```

</details>

### makeCopy()

**Creates:**
- `SymbioticVirus` — `new SymbioticVirus()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new SymbioticVirus();
    }
```

</details>

## TeardropLocket
File: `relics\TeardropLocket.java`

### atBattleStart()

**Creates:**
- `ChangeStanceAction` — `new ChangeStanceAction("Calm")`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToTop(new ChangeStanceAction("Calm"));
        this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
    }
```

</details>

### makeCopy()

**Creates:**
- `TeardropLocket` — `new TeardropLocket()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new TeardropLocket();
    }
```

</details>

## Test1
File: `relics\Test1.java`

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

### makeCopy()

**Creates:**
- `Test1` — `new Test1()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Test1();
    }
```

</details>

## Test3
File: `relics\Test3.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
    }
```

</details>

### makeCopy()

**Creates:**
- `Test3` — `new Test3()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Test3();
    }
```

</details>

## Test4
File: `relics\Test4.java`

### atBattleStart()

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
    }
```

</details>

### makeCopy()

**Creates:**
- `Test4` — `new Test4()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Test4();
    }
```

</details>

## Test5
File: `relics\Test5.java`

### onEquip()

**Creates:**
- `None` — `new ArrayList<AbstractCard>()`
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
        Collections.shuffle(upgradableCards);
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
- `Test5` — `new Test5()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Test5();
    }
```

</details>

## Test6
File: `relics\Test6.java`

### onPlayerEndTurn()

**Creates:**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)AbstractDungeon.player, AbstractDungeon.player, 3 * (AbstractDungeon.player.gold / 100))`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

<details><summary>Full body</summary>

```java
@Override
    public void onPlayerEndTurn() {
        if (this.hasEnoughGold()) {
            this.flash();
            this.pulse = false;
            this.addToTop(new GainBlockAction((AbstractCreature)AbstractDungeon.player, AbstractDungeon.player, 3 * (AbstractDungeon.player.gold / 100)));
            this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
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
- `Test6` — `new Test6()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Test6();
    }
```

</details>

## TheSpecimen
File: `relics\TheSpecimen.java`

### onMonsterDeath(AbstractMonster m)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(m, this)`
- `ApplyPowerToRandomEnemyAction` — `new ApplyPowerToRandomEnemyAction(AbstractDungeon.player, new PoisonPower(null, AbstractDungeon.player, amount), amount, false, AbstractGameAction.AttackEffect.POISON)`
- `PoisonPower` — `new PoisonPower(null, AbstractDungeon.player, amount)`

<details><summary>Full body</summary>

```java
@Override
    public void onMonsterDeath(AbstractMonster m) {
        if (m.hasPower("Poison")) {
            int amount = m.getPower((String)"Poison").amount;
            if (!AbstractDungeon.getMonsters().areMonstersBasicallyDead()) {
                this.flash();
                this.addToBot(new RelicAboveCreatureAction(m, this));
                this.addToBot(new ApplyPowerToRandomEnemyAction(AbstractDungeon.player, new PoisonPower(null, AbstractDungeon.player, amount), amount, false, AbstractGameAction.AttackEffect.POISON));
            } else {
                logger.info("no target for the specimen");
            }
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `TheSpecimen` — `new TheSpecimen()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new TheSpecimen();
    }
```

</details>

## ThreadAndNeedle
File: `relics\ThreadAndNeedle.java`

### atBattleStart()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new PlatedArmorPower(AbstractDungeon.player, 4), 4)`
- `PlatedArmorPower` — `new PlatedArmorPower(AbstractDungeon.player, 4)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new PlatedArmorPower(AbstractDungeon.player, 4), 4));
        this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
    }
```

</details>

### makeCopy()

**Creates:**
- `ThreadAndNeedle` — `new ThreadAndNeedle()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new ThreadAndNeedle();
    }
```

</details>

## Tingsha
File: `relics\Tingsha.java`

### onManualDiscard()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `DamageRandomEnemyAction` — `new DamageRandomEnemyAction(new DamageInfo(AbstractDungeon.player, 3, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.FIRE)`
- `DamageInfo` — `new DamageInfo(AbstractDungeon.player, 3, DamageInfo.DamageType.THORNS)`

<details><summary>Full body</summary>

```java
@Override
    public void onManualDiscard() {
        this.flash();
        CardCrawlGame.sound.play("TINGSHA");
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new DamageRandomEnemyAction(new DamageInfo(AbstractDungeon.player, 3, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.FIRE));
    }
```

</details>

### makeCopy()

**Creates:**
- `Tingsha` — `new Tingsha()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Tingsha();
    }
```

</details>

## TinyChest
File: `relics\TinyChest.java`

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
- `TinyChest` — `new TinyChest()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new TinyChest();
    }
```

</details>

## TinyHouse
File: `relics\TinyHouse.java`

### onEquip()

**Creates:**
- `None` — `new ArrayList<AbstractCard>()`
- `Random` — `new Random(AbstractDungeon.miscRng.randomLong())`
- `ShowCardBrieflyEffect` — `new ShowCardBrieflyEffect(((AbstractCard)upgradableCards.get(0)).makeStatEquivalentCopy())`
- `UpgradeShineEffect` — `new UpgradeShineEffect((float)Settings.WIDTH / 2.0f, (float)Settings.HEIGHT / 2.0f)`
- `ShowCardBrieflyEffect` — `new ShowCardBrieflyEffect(((AbstractCard)upgradableCards.get(0)).makeStatEquivalentCopy(), (float)Settings.WIDTH / 2.0f, (float)Settings.HEIGHT / 2.0f)`
- `UpgradeShineEffect` — `new UpgradeShineEffect((float)Settings.WIDTH / 2.0f, (float)Settings.HEIGHT / 2.0f)`

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        ArrayList<AbstractCard> upgradableCards = new ArrayList<AbstractCard>();
        for (AbstractCard c : AbstractDungeon.player.masterDeck.group) {
            if (!c.canUpgrade()) continue;
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
                AbstractDungeon.player.bottledCardUpgradeCheck((AbstractCard)upgradableCards.get(0));
                AbstractDungeon.player.bottledCardUpgradeCheck((AbstractCard)upgradableCards.get(1));
                AbstractDungeon.topLevelEffects.add(new ShowCardBrieflyEffect(((AbstractCard)upgradableCards.get(0)).makeStatEquivalentCopy(), (float)Settings.WIDTH / 2.0f, (float)Settings.HEIGHT / 2.0f));
                AbstractDungeon.topLevelEffects.add(new UpgradeShineEffect((float)Settings.WIDTH / 2.0f, (float)Settings.HEIGHT / 2.0f));
            }
        }
        AbstractDungeon.player.increaseMaxHp(5, true);
        AbstractDungeon.getCurrRoom().addGoldToRewards(50);
        AbstractDungeon.getCurrRoom().addPotionToRewards(PotionHelper.getRandomPotion(AbstractDungeon.miscRng));
        AbstractDungeon.combatRewardScreen.open(this.DESCRIPTIONS[3]);
        AbstractDungeon.getCurrRoom().rewardPopOutTimer = 0.0f;
    }
```

</details>

### makeCopy()

**Creates:**
- `TinyHouse` — `new TinyHouse()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new TinyHouse();
    }
```

</details>

## Toolbox
File: `relics\Toolbox.java`

### atBattleStartPreDraw()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `ChooseOneColorless` — `new ChooseOneColorless()`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStartPreDraw() {
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new ChooseOneColorless());
    }
```

</details>

### makeCopy()

**Creates:**
- `Toolbox` — `new Toolbox()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Toolbox();
    }
```

</details>

## Torii
File: `relics\Torii.java`

### onAttacked(DamageInfo info, int damageAmount)

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

<details><summary>Full body</summary>

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (info.owner != null && info.type != DamageInfo.DamageType.HP_LOSS && info.type != DamageInfo.DamageType.THORNS && damageAmount > 1 && damageAmount <= 5) {
            this.flash();
            this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
            return 1;
        }
        return damageAmount;
    }
```

</details>

### makeCopy()

**Creates:**
- `Torii` — `new Torii()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Torii();
    }
```

</details>

## ToughBandages
File: `relics\ToughBandages.java`

### onManualDiscard()

**Creates:**
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`
- `GainBlockAction` — `new GainBlockAction(AbstractDungeon.player, AbstractDungeon.player, 3, true)`

<details><summary>Full body</summary>

```java
@Override
    public void onManualDiscard() {
        this.flash();
        this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));
        this.addToBot(new GainBlockAction(AbstractDungeon.player, AbstractDungeon.player, 3, true));
    }
```

</details>

### makeCopy()

**Creates:**
- `ToughBandages` — `new ToughBandages()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new ToughBandages();
    }
```

</details>

## ToxicEgg2
File: `relics\ToxicEgg2.java`

### onEquip()

<details><summary>Full body</summary>

```java
@Override
    public void onEquip() {
        for (RewardItem reward : AbstractDungeon.combatRewardScreen.rewards) {
            if (reward.cards == null) continue;
            for (AbstractCard c : reward.cards) {
                this.onPreviewObtainCard(c);
            }
        }
    }
```

</details>

### onObtainCard(AbstractCard c)

<details><summary>Full body</summary>

```java
@Override
    public void onObtainCard(AbstractCard c) {
        if (c.type == AbstractCard.CardType.SKILL && c.canUpgrade() && !c.upgraded) {
            c.upgrade();
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `ToxicEgg2` — `new ToxicEgg2()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new ToxicEgg2();
    }
```

</details>

## ToyOrnithopter
File: `relics\ToyOrnithopter.java`

### makeCopy()

**Creates:**
- `ToyOrnithopter` — `new ToyOrnithopter()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new ToyOrnithopter();
    }
```

</details>

## TungstenRod
File: `relics\TungstenRod.java`

### makeCopy()

**Creates:**
- `TungstenRod` — `new TungstenRod()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new TungstenRod();
    }
```

</details>

## Turnip
File: `relics\Turnip.java`

### makeCopy()

**Creates:**
- `Turnip` — `new Turnip()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Turnip();
    }
```

</details>

## TwistedFunnel
File: `relics\TwistedFunnel.java`

### atBattleStart()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(m, AbstractDungeon.player, new PoisonPower(m, AbstractDungeon.player, 4), 4)`
- `PoisonPower` — `new PoisonPower(m, AbstractDungeon.player, 4)`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        for (AbstractMonster m : AbstractDungeon.getMonsters().monsters) {
            if (m.isDead || m.isDying) continue;
            this.addToBot(new ApplyPowerAction(m, AbstractDungeon.player, new PoisonPower(m, AbstractDungeon.player, 4), 4));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `TwistedFunnel` — `new TwistedFunnel()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new TwistedFunnel();
    }
```

</details>

## UnceasingTop
File: `relics\UnceasingTop.java`

### atPreBattle()

<details><summary>Full body</summary>

```java
@Override
    public void atPreBattle() {
        this.canDraw = false;
    }
```

</details>

### makeCopy()

**Creates:**
- `UnceasingTop` — `new UnceasingTop()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new UnceasingTop();
    }
```

</details>

## Vajra
File: `relics\Vajra.java`

### atBattleStart()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new StrengthPower(AbstractDungeon.player, 1), 1)`
- `StrengthPower` — `new StrengthPower(AbstractDungeon.player, 1)`
- `RelicAboveCreatureAction` — `new RelicAboveCreatureAction(AbstractDungeon.player, this)`

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.flash();
        this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new StrengthPower(AbstractDungeon.player, 1), 1));
        this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this));
    }
```

</details>

### makeCopy()

**Creates:**
- `Vajra` — `new Vajra()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new Vajra();
    }
```

</details>

## VelvetChoker
File: `relics\VelvetChoker.java`

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

### atBattleStart()

<details><summary>Full body</summary>

```java
@Override
    public void atBattleStart() {
        this.counter = 0;
    }
```

</details>

### onPlayCard(AbstractCard card, AbstractMonster m)

<details><summary>Full body</summary>

```java
@Override
    public void onPlayCard(AbstractCard card, AbstractMonster m) {
        if (this.counter < 6) {
            ++this.counter;
            if (this.counter >= 6) {
                this.flash();
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
- `VelvetChoker` — `new VelvetChoker()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new VelvetChoker();
    }
```

</details>

## VioletLotus
File: `relics\VioletLotus.java`

### onChangeStance(AbstractStance prevStance, AbstractStance newStance)

**Creates:**
- `GainEnergyAction` — `new GainEnergyAction(1)`

<details><summary>Full body</summary>

```java
@Override
    public void onChangeStance(AbstractStance prevStance, AbstractStance newStance) {
        if (!prevStance.ID.equals(newStance.ID) && prevStance.ID.equals("Calm")) {
            this.flash();
            this.addToBot(new GainEnergyAction(1));
        }
    }
```

</details>

### makeCopy()

**Creates:**
- `VioletLotus` — `new VioletLotus()`

<details><summary>Full body</summary>

```java
@Override
    public AbstractRelic makeCopy() {
        return new VioletLotus();
    }
```

</details>

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

