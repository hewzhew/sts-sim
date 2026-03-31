# Relics: M

15 relics

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

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new MakeTempCardInDrawPileAction(new Wound(), 2, true, true))`

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

**Queue insertion:**
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

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

**Queue insertion:**
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

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

**Queue insertion:**
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

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

**Queue insertion:**
- [BOT] `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this))`
- [BOT] `this.addToBot(new ScryAction(3))`

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

**Queue insertion:**
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

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

**Queue insertion:**
- [TOP] `this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new StrengthPower(AbstractDungeon.player, 3), 3))`
- [TOP] `this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new LoseStrengthPower(AbstractDungeon.player, 3), 3))`
- [TOP] `this.addToTop(new RelicAboveCreatureAction(AbstractDungeon.player, this))`

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

