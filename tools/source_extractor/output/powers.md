# StS Power Reference

Total Power subclasses: 162

## AbstractPower
File: `powers\AbstractPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
public void updateDescription() {
    }
```

</details>

### reducePower(int reduceAmount)

<details><summary>Full body</summary>

```java
public void reducePower(int reduceAmount) {
        if (this.amount - reduceAmount <= 0) {
            this.fontScale = 8.0f;
            this.amount = 0;
        } else {
            this.fontScale = 8.0f;
            this.amount -= reduceAmount;
        }
    }
```

</details>

### atDamageGive(float damage, DamageInfo.DamageType type)

<details><summary>Full body</summary>

```java
public float atDamageGive(float damage, DamageInfo.DamageType type) {
        return damage;
    }
```

</details>

### atDamageFinalGive(float damage, DamageInfo.DamageType type)

<details><summary>Full body</summary>

```java
public float atDamageFinalGive(float damage, DamageInfo.DamageType type) {
        return damage;
    }
```

</details>

### atDamageFinalReceive(float damage, DamageInfo.DamageType type)

<details><summary>Full body</summary>

```java
public float atDamageFinalReceive(float damage, DamageInfo.DamageType type) {
        return damage;
    }
```

</details>

### atDamageReceive(float damage, DamageInfo.DamageType damageType)

<details><summary>Full body</summary>

```java
public float atDamageReceive(float damage, DamageInfo.DamageType damageType) {
        return damage;
    }
```

</details>

### atDamageGive(float damage, DamageInfo.DamageType type, AbstractCard card)

<details><summary>Full body</summary>

```java
public float atDamageGive(float damage, DamageInfo.DamageType type, AbstractCard card) {
        return this.atDamageGive(damage, type);
    }
```

</details>

### atDamageFinalGive(float damage, DamageInfo.DamageType type, AbstractCard card)

<details><summary>Full body</summary>

```java
public float atDamageFinalGive(float damage, DamageInfo.DamageType type, AbstractCard card) {
        return this.atDamageFinalGive(damage, type);
    }
```

</details>

### atDamageFinalReceive(float damage, DamageInfo.DamageType type, AbstractCard card)

<details><summary>Full body</summary>

```java
public float atDamageFinalReceive(float damage, DamageInfo.DamageType type, AbstractCard card) {
        return this.atDamageFinalReceive(damage, type);
    }
```

</details>

### atDamageReceive(float damage, DamageInfo.DamageType damageType, AbstractCard card)

<details><summary>Full body</summary>

```java
public float atDamageReceive(float damage, DamageInfo.DamageType damageType, AbstractCard card) {
        return this.atDamageReceive(damage, damageType);
    }
```

</details>

### atStartOfTurn()

<details><summary>Full body</summary>

```java
public void atStartOfTurn() {
    }
```

</details>

### duringTurn()

<details><summary>Full body</summary>

```java
public void duringTurn() {
    }
```

</details>

### atStartOfTurnPostDraw()

<details><summary>Full body</summary>

```java
public void atStartOfTurnPostDraw() {
    }
```

</details>

### atEndOfTurn(boolean isPlayer)

<details><summary>Full body</summary>

```java
public void atEndOfTurn(boolean isPlayer) {
    }
```

</details>

### atEndOfTurnPreEndTurnCards(boolean isPlayer)

<details><summary>Full body</summary>

```java
public void atEndOfTurnPreEndTurnCards(boolean isPlayer) {
    }
```

</details>

### atEndOfRound()

<details><summary>Full body</summary>

```java
public void atEndOfRound() {
    }
```

</details>

### onHeal(int healAmount)

<details><summary>Full body</summary>

```java
public int onHeal(int healAmount) {
        return healAmount;
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

### onAttack(DamageInfo info, int damageAmount, AbstractCreature target)

<details><summary>Full body</summary>

```java
public void onAttack(DamageInfo info, int damageAmount, AbstractCreature target) {
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

### onInflictDamage(DamageInfo info, int damageAmount, AbstractCreature target)

<details><summary>Full body</summary>

```java
public void onInflictDamage(DamageInfo info, int damageAmount, AbstractCreature target) {
    }
```

</details>

### onEvokeOrb(AbstractOrb orb)

<details><summary>Full body</summary>

```java
public void onEvokeOrb(AbstractOrb orb) {
    }
```

</details>

### onCardDraw(AbstractCard card)

<details><summary>Full body</summary>

```java
public void onCardDraw(AbstractCard card) {
    }
```

</details>

### onPlayCard(AbstractCard card, AbstractMonster m)

<details><summary>Full body</summary>

```java
public void onPlayCard(AbstractCard card, AbstractMonster m) {
    }
```

</details>

### onUseCard(AbstractCard card, UseCardAction action)

<details><summary>Full body</summary>

```java
public void onUseCard(AbstractCard card, UseCardAction action) {
    }
```

</details>

### onAfterUseCard(AbstractCard card, UseCardAction action)

<details><summary>Full body</summary>

```java
public void onAfterUseCard(AbstractCard card, UseCardAction action) {
    }
```

</details>

### wasHPLost(DamageInfo info, int damageAmount)

<details><summary>Full body</summary>

```java
public void wasHPLost(DamageInfo info, int damageAmount) {
    }
```

</details>

### onSpecificTrigger()

<details><summary>Full body</summary>

```java
public void onSpecificTrigger() {
    }
```

</details>

### onDeath()

<details><summary>Full body</summary>

```java
public void onDeath() {
    }
```

</details>

### onChannel(AbstractOrb orb)

<details><summary>Full body</summary>

```java
public void onChannel(AbstractOrb orb) {
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

### onChangeStance(AbstractStance oldStance, AbstractStance newStance)

<details><summary>Full body</summary>

```java
public void onChangeStance(AbstractStance oldStance, AbstractStance newStance) {
    }
```

</details>

### onGainedBlock(float blockAmount)

<details><summary>Full body</summary>

```java
public void onGainedBlock(float blockAmount) {
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

### onPlayerGainedBlock(int blockAmount)

<details><summary>Full body</summary>

```java
public int onPlayerGainedBlock(int blockAmount) {
        return blockAmount;
    }
```

</details>

### onRemove()

<details><summary>Full body</summary>

```java
public void onRemove() {
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

### onAfterCardPlayed(AbstractCard usedCard)

<details><summary>Full body</summary>

```java
public void onAfterCardPlayed(AbstractCard usedCard) {
    }
```

</details>

### onInitialApplication()

<details><summary>Full body</summary>

```java
public void onInitialApplication() {
    }
```

</details>

### onApplyPower(AbstractPower power, AbstractCreature target, AbstractCreature source)

<details><summary>Full body</summary>

```java
public void onApplyPower(AbstractPower power, AbstractCreature target, AbstractCreature source) {
    }
```

</details>

### onLoseHp(int damageAmount)

<details><summary>Full body</summary>

```java
public int onLoseHp(int damageAmount) {
        return damageAmount;
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

## AccuracyPower
File: `powers\AccuracyPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

### onDrawOrDiscard()

<details><summary>Full body</summary>

```java
@Override
    public void onDrawOrDiscard() {
        for (AbstractCard c : AbstractDungeon.player.hand.group) {
            if (!(c instanceof Shiv)) continue;
            if (!c.upgraded) {
                c.baseDamage = 4 + this.amount;
                continue;
            }
            c.baseDamage = 6 + this.amount;
        }
    }
```

</details>

## AfterImagePower
File: `powers\AfterImagePower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = AfterImagePower.powerStrings.DESCRIPTIONS[0] + this.amount + AfterImagePower.powerStrings.DESCRIPTIONS[1];
    }
```

</details>

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `GainBlockAction` — `new GainBlockAction(AbstractDungeon.player, AbstractDungeon.player, this.amount, true)`
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)AbstractDungeon.player, AbstractDungeon.player, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (Settings.FAST_MODE) {
            this.addToBot(new GainBlockAction(AbstractDungeon.player, AbstractDungeon.player, this.amount, true));
        } else {
            this.addToBot(new GainBlockAction((AbstractCreature)AbstractDungeon.player, AbstractDungeon.player, this.amount));
        }
        this.flash();
    }
```

</details>

## AmplifyPower
File: `powers\AmplifyPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[0] : DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `CardQueueItem` — `new CardQueueItem(tmp, m, card.energyOnUse, true, true)`
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (!card.purgeOnUse && card.type == AbstractCard.CardType.POWER && this.amount > 0) {
            this.flash();
            AbstractMonster m = null;
            if (action.target != null) {
                m = (AbstractMonster)action.target;
            }
            AbstractCard tmp = card.makeSameInstanceOf();
            AbstractDungeon.player.limbo.addToBottom(tmp);
            tmp.current_x = card.current_x;
            tmp.current_y = card.current_y;
            tmp.target_x = (float)Settings.WIDTH / 2.0f - 300.0f * Settings.scale;
            tmp.target_y = (float)Settings.HEIGHT / 2.0f;
            if (m != null) {
                tmp.calculateCardDamage(m);
            }
            tmp.purgeOnUse = true;
            AbstractDungeon.actionManager.addCardQueueItem(new CardQueueItem(tmp, m, card.energyOnUse, true, true), true);
            --this.amount;
            if (this.amount == 0) {
                this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
            }
        }
    }
```

</details>

### atEndOfTurn(boolean isPlayer)

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        if (isPlayer) {
            this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        }
    }
```

</details>

## AngerPower
File: `powers\AngerPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, this.amount), this.amount)`
- `StrengthPower` — `new StrengthPower(this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.SKILL) {
            this.addToTop(new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, this.amount), this.amount));
            this.flash();
        }
    }
```

</details>

## AngryPower
File: `powers\AngryPower.java`

### onAttacked(DamageInfo info, int damageAmount)

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, this.amount), this.amount)`
- `StrengthPower` — `new StrengthPower(this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (info.owner != null && damageAmount > 0 && info.type != DamageInfo.DamageType.HP_LOSS && info.type != DamageInfo.DamageType.THORNS) {
            this.addToTop(new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, this.amount), this.amount));
            this.flash();
        }
        return damageAmount;
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

## ArtifactPower
File: `powers\ArtifactPower.java`

### onSpecificTrigger()

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`
- `ReducePowerAction` — `new ReducePowerAction(this.owner, this.owner, POWER_ID, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void onSpecificTrigger() {
        if (this.amount <= 0) {
            this.addToTop(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        } else {
            this.addToTop(new ReducePowerAction(this.owner, this.owner, POWER_ID, 1));
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1] : DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

## AttackBurnPower
File: `powers\AttackBurnPower.java`

### atEndOfRound()

**Creates:**
- `ReducePowerAction` — `new ReducePowerAction(this.owner, this.owner, POWER_ID, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfRound() {
        if (this.justApplied) {
            this.justApplied = false;
            return;
        }
        this.addToBot(new ReducePowerAction(this.owner, this.owner, POWER_ID, 1));
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1] : DESCRIPTIONS[2] + this.amount + DESCRIPTIONS[3];
    }
```

</details>

### onUseCard(AbstractCard card, UseCardAction action)

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.ATTACK) {
            action.exhaustCard = true;
            this.flash();
        }
    }
```

</details>

## BackAttackPower
File: `powers\BackAttackPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0];
    }
```

</details>

## BarricadePower
File: `powers\BarricadePower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.owner.isPlayer ? DESCRIPTIONS[0] : DESCRIPTIONS[1];
    }
```

</details>

## BattleHymnPower
File: `powers\watcher\BattleHymnPower.java`

### atStartOfTurn()

**Creates:**
- `MakeTempCardInHandAction` — `new MakeTempCardInHandAction((AbstractCard)new Smite(), this.amount, false)`
- `Smite` — `new Smite()`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        if (!AbstractDungeon.getMonsters().areMonstersBasicallyDead()) {
            this.flash();
            this.addToBot(new MakeTempCardInHandAction((AbstractCard)new Smite(), this.amount, false));
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount > 1 ? BattleHymnPower.powerStrings.DESCRIPTIONS[0] + this.amount + BattleHymnPower.powerStrings.DESCRIPTIONS[1] : BattleHymnPower.powerStrings.DESCRIPTIONS[0] + this.amount + BattleHymnPower.powerStrings.DESCRIPTIONS[2];
    }
```

</details>

## BeatOfDeathPower
File: `powers\BeatOfDeathPower.java`

### onAfterUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `DamageAction` — `new DamageAction((AbstractCreature)AbstractDungeon.player, new DamageInfo(this.owner, this.amount, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.BLUNT_LIGHT)`
- `DamageInfo` — `new DamageInfo(this.owner, this.amount, DamageInfo.DamageType.THORNS)`

<details><summary>Full body</summary>

```java
@Override
    public void onAfterUseCard(AbstractCard card, UseCardAction action) {
        this.flash();
        this.addToBot(new DamageAction((AbstractCreature)AbstractDungeon.player, new DamageInfo(this.owner, this.amount, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.BLUNT_LIGHT));
        this.updateDescription();
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

## BerserkPower
File: `powers\BerserkPower.java`

### updateDescription()

**Creates:**
- `StringBuilder` — `new StringBuilder()`

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        StringBuilder sb = new StringBuilder();
        sb.append(BerserkPower.powerStrings.DESCRIPTIONS[0]);
        for (int i = 0; i < this.amount; ++i) {
            sb.append("[R] ");
        }
        sb.append(LocalizedStrings.PERIOD);
        this.description = sb.toString();
    }
```

</details>

### atStartOfTurn()

**Creates:**
- `GainEnergyAction` — `new GainEnergyAction(this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        this.addToBot(new GainEnergyAction(this.amount));
        this.flash();
    }
```

</details>

## BiasPower
File: `powers\BiasPower.java`

### atStartOfTurn()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(this.owner, this.owner, new FocusPower(this.owner, -this.amount), -this.amount)`
- `FocusPower` — `new FocusPower(this.owner, -this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        this.flash();
        this.addToBot(new ApplyPowerAction(this.owner, this.owner, new FocusPower(this.owner, -this.amount), -this.amount));
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

## BlockReturnPower
File: `powers\watcher\BlockReturnPower.java`

### onAttacked(DamageInfo info, int damageAmount)

**Creates:**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)AbstractDungeon.player, this.amount, Settings.FAST_MODE)`

<details><summary>Full body</summary>

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (info.type != DamageInfo.DamageType.THORNS && info.type != DamageInfo.DamageType.HP_LOSS && info.owner != null && info.owner != this.owner) {
            this.flash();
            this.addToTop(new GainBlockAction((AbstractCreature)AbstractDungeon.player, this.amount, Settings.FAST_MODE));
        }
        return damageAmount;
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = BlockReturnPower.powerStrings.DESCRIPTIONS[0] + this.amount + BlockReturnPower.powerStrings.DESCRIPTIONS[1];
    }
```

</details>

## BlurPower
File: `powers\BlurPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[0] : DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

### atEndOfRound()

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`
- `ReducePowerAction` — `new ReducePowerAction(this.owner, this.owner, POWER_ID, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfRound() {
        if (this.amount == 0) {
            this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        } else {
            this.addToBot(new ReducePowerAction(this.owner, this.owner, POWER_ID, 1));
        }
    }
```

</details>

## BrutalityPower
File: `powers\BrutalityPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2] : DESCRIPTIONS[3] + this.amount + DESCRIPTIONS[4] + this.amount + DESCRIPTIONS[5];
    }
```

</details>

### atStartOfTurnPostDraw()

**Creates:**
- `DrawCardAction` — `new DrawCardAction(this.owner, this.amount)`
- `LoseHPAction` — `new LoseHPAction(this.owner, this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurnPostDraw() {
        this.flash();
        this.addToBot(new DrawCardAction(this.owner, this.amount));
        this.addToBot(new LoseHPAction(this.owner, this.owner, this.amount));
    }
```

</details>

## BufferPower
File: `powers\BufferPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount <= 1 ? DESCRIPTIONS[0] : DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

### onAttackedToChangeDamage(DamageInfo info, int damageAmount)

**Creates:**
- `ReducePowerAction` — `new ReducePowerAction(this.owner, this.owner, this.ID, 1)`

<details><summary>Full body</summary>

```java
@Override
    public int onAttackedToChangeDamage(DamageInfo info, int damageAmount) {
        if (damageAmount > 0) {
            this.addToTop(new ReducePowerAction(this.owner, this.owner, this.ID, 1));
        }
        return 0;
    }
```

</details>

## BurstPower
File: `powers\BurstPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[0] : DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `CardQueueItem` — `new CardQueueItem(tmp, m, card.energyOnUse, true, true)`
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (!card.purgeOnUse && card.type == AbstractCard.CardType.SKILL && this.amount > 0) {
            this.flash();
            AbstractMonster m = null;
            if (action.target != null) {
                m = (AbstractMonster)action.target;
            }
            AbstractCard tmp = card.makeSameInstanceOf();
            AbstractDungeon.player.limbo.addToBottom(tmp);
            tmp.current_x = card.current_x;
            tmp.current_y = card.current_y;
            tmp.target_x = (float)Settings.WIDTH / 2.0f - 300.0f * Settings.scale;
            tmp.target_y = (float)Settings.HEIGHT / 2.0f;
            if (m != null) {
                tmp.calculateCardDamage(m);
            }
            tmp.purgeOnUse = true;
            AbstractDungeon.actionManager.addCardQueueItem(new CardQueueItem(tmp, m, card.energyOnUse, true, true), true);
            --this.amount;
            if (this.amount == 0) {
                this.addToTop(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
            }
        }
    }
```

</details>

### atEndOfTurn(boolean isPlayer)

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        if (isPlayer) {
            this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        }
    }
```

</details>

## CannotChangeStancePower
File: `powers\watcher\CannotChangeStancePower.java`

### atEndOfTurn(boolean isPlayer)

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        if (isPlayer) {
            this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = CannotChangeStancePower.powerStrings.DESCRIPTIONS[0];
    }
```

</details>

## ChokePower
File: `powers\ChokePower.java`

### atStartOfTurn()

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
    }
```

</details>

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `LoseHPAction` — `new LoseHPAction(this.owner, null, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        this.flash();
        this.addToBot(new LoseHPAction(this.owner, null, this.amount));
    }
```

</details>

## CollectPower
File: `powers\CollectPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[0] : DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

## CombustPower
File: `powers\CombustPower.java`

### atEndOfTurn(boolean isPlayer)

**Creates:**
- `LoseHPAction` — `new LoseHPAction(this.owner, this.owner, this.hpLoss, AbstractGameAction.AttackEffect.FIRE)`
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(this.amount, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.FIRE)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        if (!AbstractDungeon.getMonsters().areMonstersBasicallyDead()) {
            this.flash();
            this.addToBot(new LoseHPAction(this.owner, this.owner, this.hpLoss, AbstractGameAction.AttackEffect.FIRE));
            this.addToBot(new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(this.amount, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.FIRE));
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.hpLoss + DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

## ConfusionPower
File: `powers\ConfusionPower.java`

### onCardDraw(AbstractCard card)

<details><summary>Full body</summary>

```java
@Override
    public void onCardDraw(AbstractCard card) {
        if (card.cost >= 0) {
            int newCost = AbstractDungeon.cardRandomRng.random(3);
            if (card.cost != newCost) {
                card.costForTurn = card.cost = newCost;
                card.isCostModified = true;
            }
            card.freeToPlayOnce = false;
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0];
    }
```

</details>

## ConservePower
File: `powers\ConservePower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[0] : DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

### atEndOfRound()

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`
- `ReducePowerAction` — `new ReducePowerAction(this.owner, this.owner, POWER_ID, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfRound() {
        if (this.amount == 0) {
            this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        } else {
            this.addToBot(new ReducePowerAction(this.owner, this.owner, POWER_ID, 1));
        }
    }
```

</details>

## ConstrictedPower
File: `powers\ConstrictedPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

### atEndOfTurn(boolean isPlayer)

**Creates:**
- `DamageAction` — `new DamageAction(this.owner, new DamageInfo(this.source, this.amount, DamageInfo.DamageType.THORNS))`
- `DamageInfo` — `new DamageInfo(this.source, this.amount, DamageInfo.DamageType.THORNS)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        this.flashWithoutSound();
        this.playApplyPowerSfx();
        this.addToBot(new DamageAction(this.owner, new DamageInfo(this.source, this.amount, DamageInfo.DamageType.THORNS)));
    }
```

</details>

## CorpseExplosionPower
File: `powers\CorpseExplosionPower.java`

### onDeath()

**Creates:**
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(this.owner.maxHealth * this.amount, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.FIRE)`

<details><summary>Full body</summary>

```java
@Override
    public void onDeath() {
        if (!AbstractDungeon.getMonsters().areMonstersBasicallyDead() && this.owner.currentHealth <= 0) {
            this.addToBot(new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(this.owner.maxHealth * this.amount, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.FIRE));
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[0] : DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

## CorruptionPower
File: `powers\CorruptionPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[1];
    }
```

</details>

### onCardDraw(AbstractCard card)

<details><summary>Full body</summary>

```java
@Override
    public void onCardDraw(AbstractCard card) {
        if (card.type == AbstractCard.CardType.SKILL) {
            card.setCostForTurn(-9);
        }
    }
```

</details>

### onUseCard(AbstractCard card, UseCardAction action)

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.SKILL) {
            this.flash();
            action.exhaustCard = true;
        }
    }
```

</details>

## CreativeAIPower
File: `powers\CreativeAIPower.java`

### atStartOfTurn()

**Creates:**
- `MakeTempCardInHandAction` — `new MakeTempCardInHandAction(card)`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        for (int i = 0; i < this.amount; ++i) {
            AbstractCard card = AbstractDungeon.returnTrulyRandomCardInCombat(AbstractCard.CardType.POWER).makeCopy();
            this.addToBot(new MakeTempCardInHandAction(card));
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount > 1 ? DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[2] : DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

## CuriosityPower
File: `powers\CuriosityPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, this.amount), this.amount)`
- `StrengthPower` — `new StrengthPower(this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.POWER) {
            this.flash();
            this.addToBot(new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, this.amount), this.amount));
        }
    }
```

</details>

## CurlUpPower
File: `powers\CurlUpPower.java`

### onAttacked(DamageInfo info, int damageAmount)

**Creates:**
- `ChangeStateAction` — `new ChangeStateAction((AbstractMonster)this.owner, "CLOSED")`
- `GainBlockAction` — `new GainBlockAction(this.owner, this.owner, this.amount)`
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (!this.triggered && damageAmount < this.owner.currentHealth && damageAmount > 0 && info.owner != null && info.type == DamageInfo.DamageType.NORMAL) {
            this.flash();
            this.triggered = true;
            this.addToBot(new ChangeStateAction((AbstractMonster)this.owner, "CLOSED"));
            this.addToBot(new GainBlockAction(this.owner, this.owner, this.amount));
            this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        }
        return damageAmount;
    }
```

</details>

## DEPRECATEDAlwaysMadPower
File: `powers\deprecated\DEPRECATEDAlwaysMadPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0];
    }
```

</details>

## DEPRECATEDCondensePower
File: `powers\deprecated\DEPRECATEDCondensePower.java`

### onLoseHp(int damageAmount)

<details><summary>Full body</summary>

```java
@Override
    public int onLoseHp(int damageAmount) {
        if (damageAmount > this.amount) {
            this.flash();
            return this.amount;
        }
        return damageAmount;
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

## DEPRECATEDDisciplinePower
File: `powers\deprecated\DEPRECATEDDisciplinePower.java`

### atEndOfTurn(boolean isPlayer)

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        if (EnergyPanel.totalCount > 0) {
            this.amount = EnergyPanel.totalCount;
            this.fontScale = 8.0f;
        }
    }
```

</details>

### atStartOfTurn()

**Creates:**
- `DrawCardAction` — `new DrawCardAction(this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        if (this.amount != -1) {
            this.addToTop(new DrawCardAction(this.amount));
            this.amount = -1;
            this.fontScale = 8.0f;
            this.flash();
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DEPRECATEDDisciplinePower.powerStrings.DESCRIPTIONS[0];
    }
```

</details>

## DEPRECATEDEmotionalTurmoilPower
File: `powers\deprecated\DEPRECATEDEmotionalTurmoilPower.java`

### atStartOfTurnPostDraw()

**Creates:**
- `DEPRECATEDRandomStanceAction` — `new DEPRECATEDRandomStanceAction()`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurnPostDraw() {
        this.addToBot(new DEPRECATEDRandomStanceAction());
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DEPRECATEDEmotionalTurmoilPower.powerStrings.DESCRIPTIONS[0];
    }
```

</details>

## DEPRECATEDFlickedPower
File: `powers\deprecated\DEPRECATEDFlickedPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DEPRECATEDFlickedPower.powerStrings.DESCRIPTIONS[0] + DEPRECATEDFlickedPower.powerStrings.DESCRIPTIONS[1] + 50 + DEPRECATEDFlickedPower.powerStrings.DESCRIPTIONS[3] : DEPRECATEDFlickedPower.powerStrings.DESCRIPTIONS[0] + DEPRECATEDFlickedPower.powerStrings.DESCRIPTIONS[2] + 50 + DEPRECATEDFlickedPower.powerStrings.DESCRIPTIONS[3];
    }
```

</details>

## DEPRECATEDFlowPower
File: `powers\deprecated\DEPRECATEDFlowPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DEPRECATEDFlowPower.powerStrings.DESCRIPTIONS[0] + this.amount + DEPRECATEDFlowPower.powerStrings.DESCRIPTIONS[1] + this.amount + DEPRECATEDFlowPower.powerStrings.DESCRIPTIONS[2];
    }
```

</details>

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, this.amount), this.amount)`
- `StrengthPower` — `new StrengthPower(this.owner, this.amount)`
- `ApplyPowerAction` — `new ApplyPowerAction(this.owner, this.owner, new LoseStrengthPower(this.owner, this.amount), this.amount)`
- `LoseStrengthPower` — `new LoseStrengthPower(this.owner, this.amount)`
- `ApplyPowerAction` — `new ApplyPowerAction(this.owner, this.owner, new DexterityPower(this.owner, this.amount), this.amount)`
- `DexterityPower` — `new DexterityPower(this.owner, this.amount)`
- `ApplyPowerAction` — `new ApplyPowerAction(this.owner, this.owner, new LoseDexterityPower(this.owner, this.amount), this.amount)`
- `LoseDexterityPower` — `new LoseDexterityPower(this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.SKILL) {
            this.flash();
            this.addToBot(new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, this.amount), this.amount));
            this.addToBot(new ApplyPowerAction(this.owner, this.owner, new LoseStrengthPower(this.owner, this.amount), this.amount));
        } else if (card.type == AbstractCard.CardType.ATTACK) {
            this.flash();
            this.addToBot(new ApplyPowerAction(this.owner, this.owner, new DexterityPower(this.owner, this.amount), this.amount));
            this.addToBot(new ApplyPowerAction(this.owner, this.owner, new LoseDexterityPower(this.owner, this.amount), this.amount));
        }
    }
```

</details>

## DEPRECATEDGroundedPower
File: `powers\deprecated\DEPRECATEDGroundedPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0];
    }
```

</details>

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `ChangeStanceAction` — `new ChangeStanceAction("Calm")`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.SKILL) {
            this.flash();
            this.addToBot(new ChangeStanceAction("Calm"));
        }
    }
```

</details>

## DEPRECATEDHotHotPower
File: `powers\deprecated\DEPRECATEDHotHotPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

### onAttacked(DamageInfo info, int damageAmount)

**Creates:**
- `DamageAction` — `new DamageAction(info.owner, new DamageInfo(this.owner, this.amount, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.FIRE, true)`
- `DamageInfo` — `new DamageInfo(this.owner, this.amount, DamageInfo.DamageType.THORNS)`

<details><summary>Full body</summary>

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (info.type != DamageInfo.DamageType.THORNS && info.type != DamageInfo.DamageType.HP_LOSS && info.owner != null && info.owner != this.owner && damageAmount > 0 && !this.owner.hasPower("Buffer")) {
            this.flash();
            AbstractDungeon.actionManager.addToTop(new DamageAction(info.owner, new DamageInfo(this.owner, this.amount, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.FIRE, true));
        }
        return damageAmount;
    }
```

</details>

## DEPRECATEDMasterRealityPower
File: `powers\deprecated\DEPRECATEDMasterRealityPower.java`

### onAfterCardPlayed(AbstractCard card)

**Creates:**
- `DamageRandomEnemyAction` — `new DamageRandomEnemyAction(new DamageInfo(null, this.amount), AbstractGameAction.AttackEffect.FIRE)`
- `DamageInfo` — `new DamageInfo(null, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void onAfterCardPlayed(AbstractCard card) {
        if (card.retain || card.selfRetain) {
            this.flash();
            this.addToBot(new DamageRandomEnemyAction(new DamageInfo(null, this.amount), AbstractGameAction.AttackEffect.FIRE));
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DEPRECATEDMasterRealityPower.powerStrings.DESCRIPTIONS[0] + this.amount + DEPRECATEDMasterRealityPower.powerStrings.DESCRIPTIONS[1];
    }
```

</details>

## DEPRECATEDMasteryPower
File: `powers\deprecated\DEPRECATEDMasteryPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

### onChangeStance(AbstractStance oldStance, AbstractStance newStance)

**Creates:**
- `GainEnergyAction` — `new GainEnergyAction(this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void onChangeStance(AbstractStance oldStance, AbstractStance newStance) {
        if (oldStance.ID.equals(newStance.ID) && !newStance.ID.equals("Neutral")) {
            this.flash();
            this.addToBot(new GainEnergyAction(this.amount));
        }
    }
```

</details>

## DEPRECATEDRetributionPower
File: `powers\deprecated\DEPRECATEDRetributionPower.java`

### onAttacked(DamageInfo info, int damageAmount)

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(this.owner, this.owner, new VigorPower(this.owner, this.amount), this.amount)`
- `VigorPower` — `new VigorPower(this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (damageAmount > 0) {
            this.flash();
            this.addToTop(new ApplyPowerAction(this.owner, this.owner, new VigorPower(this.owner, this.amount), this.amount));
        }
        return damageAmount;
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DEPRECATEDRetributionPower.powerStrings.DESCRIPTIONS[0] + this.amount + DEPRECATEDRetributionPower.powerStrings.DESCRIPTIONS[1];
    }
```

</details>

## DEPRECATEDSerenityPower
File: `powers\deprecated\DEPRECATEDSerenityPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + LocalizedStrings.PERIOD;
    }
```

</details>

### onAttacked(DamageInfo info, int damageAmount)

<details><summary>Full body</summary>

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (damageAmount > 0 && ((AbstractPlayer)this.owner).stance.ID.equals("Calm")) {
            this.flash();
            if ((damageAmount -= this.amount) < this.amount) {
                damageAmount = 0;
            }
        }
        return damageAmount;
    }
```

</details>

## DarkEmbracePower
File: `powers\DarkEmbracePower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[0] : DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

### onExhaust(AbstractCard card)

**Creates:**
- `DrawCardAction` — `new DrawCardAction(this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void onExhaust(AbstractCard card) {
        if (!AbstractDungeon.getMonsters().areMonstersBasicallyDead()) {
            this.flash();
            this.addToBot(new DrawCardAction(this.owner, this.amount));
        }
    }
```

</details>

## DemonFormPower
File: `powers\DemonFormPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DemonFormPower.powerStrings.DESCRIPTIONS[0] + this.amount + DemonFormPower.powerStrings.DESCRIPTIONS[1];
    }
```

</details>

### atStartOfTurnPostDraw()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, this.amount), this.amount)`
- `StrengthPower` — `new StrengthPower(this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurnPostDraw() {
        this.flash();
        this.addToBot(new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, this.amount), this.amount));
    }
```

</details>

## DevaPower
File: `powers\watcher\DevaPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.energyGainAmount == 1 ? DESCRIPTIONS[0] + DESCRIPTIONS[3] + this.amount + DESCRIPTIONS[4] : DESCRIPTIONS[1] + this.energyGainAmount + DESCRIPTIONS[2] + DESCRIPTIONS[3] + this.amount + DESCRIPTIONS[4];
    }
```

</details>

## DevotionPower
File: `powers\watcher\DevotionPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DevotionPower.powerStrings.DESCRIPTIONS[0] + this.amount + DevotionPower.powerStrings.DESCRIPTIONS[1];
    }
```

</details>

### atStartOfTurnPostDraw()

**Creates:**
- `ChangeStanceAction` — `new ChangeStanceAction("Divinity")`
- `ApplyPowerAction` — `new ApplyPowerAction(this.owner, this.owner, new MantraPower(this.owner, this.amount), this.amount)`
- `MantraPower` — `new MantraPower(this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurnPostDraw() {
        this.flash();
        if (!AbstractDungeon.player.hasPower("Mantra") && this.amount >= 10) {
            this.addToBot(new ChangeStanceAction("Divinity"));
        } else {
            this.addToBot(new ApplyPowerAction(this.owner, this.owner, new MantraPower(this.owner, this.amount), this.amount));
        }
    }
```

</details>

## DexterityPower
File: `powers\DexterityPower.java`

### reducePower(int reduceAmount)

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void reducePower(int reduceAmount) {
        this.fontScale = 8.0f;
        this.amount -= reduceAmount;
        if (this.amount == 0) {
            this.addToTop(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        }
        if (this.amount >= 999) {
            this.amount = 999;
        }
        if (this.amount <= -999) {
            this.amount = -999;
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        if (this.amount > 0) {
            this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[2];
            this.type = AbstractPower.PowerType.BUFF;
        } else {
            int tmp = -this.amount;
            this.description = DESCRIPTIONS[1] + tmp + DESCRIPTIONS[2];
            this.type = AbstractPower.PowerType.DEBUFF;
        }
    }
```

</details>

## DoubleDamagePower
File: `powers\DoubleDamagePower.java`

### atEndOfRound()

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`
- `ReducePowerAction` — `new ReducePowerAction(this.owner, this.owner, POWER_ID, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfRound() {
        if (this.justApplied) {
            this.justApplied = false;
            return;
        }
        if (this.amount == 0) {
            this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        } else {
            this.addToBot(new ReducePowerAction(this.owner, this.owner, POWER_ID, 1));
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1] : DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

### atDamageGive(float damage, DamageInfo.DamageType type)

<details><summary>Full body</summary>

```java
@Override
    public float atDamageGive(float damage, DamageInfo.DamageType type) {
        if (type == DamageInfo.DamageType.NORMAL) {
            return damage * 2.0f;
        }
        return damage;
    }
```

</details>

## DoubleTapPower
File: `powers\DoubleTapPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[0] : DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `CardQueueItem` — `new CardQueueItem(tmp, m, card.energyOnUse, true, true)`
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (!card.purgeOnUse && card.type == AbstractCard.CardType.ATTACK && this.amount > 0) {
            this.flash();
            AbstractMonster m = null;
            if (action.target != null) {
                m = (AbstractMonster)action.target;
            }
            AbstractCard tmp = card.makeSameInstanceOf();
            AbstractDungeon.player.limbo.addToBottom(tmp);
            tmp.current_x = card.current_x;
            tmp.current_y = card.current_y;
            tmp.target_x = (float)Settings.WIDTH / 2.0f - 300.0f * Settings.scale;
            tmp.target_y = (float)Settings.HEIGHT / 2.0f;
            if (m != null) {
                tmp.calculateCardDamage(m);
            }
            tmp.purgeOnUse = true;
            AbstractDungeon.actionManager.addCardQueueItem(new CardQueueItem(tmp, m, card.energyOnUse, true, true), true);
            --this.amount;
            if (this.amount == 0) {
                this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
            }
        }
    }
```

</details>

### atEndOfTurn(boolean isPlayer)

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        if (isPlayer) {
            this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        }
    }
```

</details>

## DrawCardNextTurnPower
File: `powers\DrawCardNextTurnPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount > 1 ? DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1] : DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

### atStartOfTurnPostDraw()

**Creates:**
- `DrawCardAction` — `new DrawCardAction(this.owner, this.amount)`
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurnPostDraw() {
        this.flash();
        this.addToBot(new DrawCardAction(this.owner, this.amount));
        this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
    }
```

</details>

## DrawPower
File: `powers\DrawPower.java`

### onRemove()

<details><summary>Full body</summary>

```java
@Override
    public void onRemove() {
        AbstractDungeon.player.gameHandSize -= this.amount;
    }
```

</details>

### reducePower(int reduceAmount)

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void reducePower(int reduceAmount) {
        this.fontScale = 8.0f;
        this.amount -= reduceAmount;
        if (this.amount == 0) {
            this.addToTop(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        if (this.amount > 0) {
            this.description = this.amount == 1 ? DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1] : DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[3];
            this.type = AbstractPower.PowerType.BUFF;
        } else {
            this.description = this.amount == -1 ? DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[2] : DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[4];
            this.type = AbstractPower.PowerType.DEBUFF;
        }
    }
```

</details>

## DrawReductionPower
File: `powers\DrawReductionPower.java`

### onInitialApplication()

<details><summary>Full body</summary>

```java
@Override
    public void onInitialApplication() {
        --AbstractDungeon.player.gameHandSize;
    }
```

</details>

### atEndOfRound()

**Creates:**
- `ReducePowerAction` — `new ReducePowerAction(this.owner, this.owner, POWER_ID, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfRound() {
        if (this.justApplied) {
            this.justApplied = false;
            return;
        }
        this.addToBot(new ReducePowerAction(this.owner, this.owner, POWER_ID, 1));
    }
```

</details>

### onRemove()

<details><summary>Full body</summary>

```java
@Override
    public void onRemove() {
        ++AbstractDungeon.player.gameHandSize;
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[0] : DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

## DuplicationPower
File: `powers\DuplicationPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DuplicationPower.powerStrings.DESCRIPTIONS[0] : DuplicationPower.powerStrings.DESCRIPTIONS[1] + this.amount + DuplicationPower.powerStrings.DESCRIPTIONS[2];
    }
```

</details>

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `CardQueueItem` — `new CardQueueItem(tmp, m, card.energyOnUse, true, true)`
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (!card.purgeOnUse && this.amount > 0) {
            this.flash();
            AbstractMonster m = null;
            if (action.target != null) {
                m = (AbstractMonster)action.target;
            }
            AbstractCard tmp = card.makeSameInstanceOf();
            AbstractDungeon.player.limbo.addToBottom(tmp);
            tmp.current_x = card.current_x;
            tmp.current_y = card.current_y;
            tmp.target_x = (float)Settings.WIDTH / 2.0f - 300.0f * Settings.scale;
            tmp.target_y = (float)Settings.HEIGHT / 2.0f;
            if (m != null) {
                tmp.calculateCardDamage(m);
            }
            tmp.purgeOnUse = true;
            AbstractDungeon.actionManager.addCardQueueItem(new CardQueueItem(tmp, m, card.energyOnUse, true, true), true);
            --this.amount;
            if (this.amount == 0) {
                this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
            }
        }
    }
```

</details>

### atEndOfRound()

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`
- `ReducePowerAction` — `new ReducePowerAction(this.owner, this.owner, POWER_ID, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfRound() {
        if (this.amount == 0) {
            this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        } else {
            this.addToBot(new ReducePowerAction(this.owner, this.owner, POWER_ID, 1));
        }
    }
```

</details>

## EchoPower
File: `powers\EchoPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[0] : DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

### atStartOfTurn()

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        this.cardsDoubledThisTurn = 0;
    }
```

</details>

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `CardQueueItem` — `new CardQueueItem(tmp, m, card.energyOnUse, true, true)`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (!card.purgeOnUse && this.amount > 0 && AbstractDungeon.actionManager.cardsPlayedThisTurn.size() - this.cardsDoubledThisTurn <= this.amount) {
            ++this.cardsDoubledThisTurn;
            this.flash();
            AbstractMonster m = null;
            if (action.target != null) {
                m = (AbstractMonster)action.target;
            }
            AbstractCard tmp = card.makeSameInstanceOf();
            AbstractDungeon.player.limbo.addToBottom(tmp);
            tmp.current_x = card.current_x;
            tmp.current_y = card.current_y;
            tmp.target_x = (float)Settings.WIDTH / 2.0f - 300.0f * Settings.scale;
            tmp.target_y = (float)Settings.HEIGHT / 2.0f;
            if (m != null) {
                tmp.calculateCardDamage(m);
            }
            tmp.purgeOnUse = true;
            AbstractDungeon.actionManager.addCardQueueItem(new CardQueueItem(tmp, m, card.energyOnUse, true, true), true);
        }
    }
```

</details>

## ElectroPower
File: `powers\ElectroPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0];
    }
```

</details>

## EndTurnDeathPower
File: `powers\watcher\EndTurnDeathPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = EndTurnDeathPower.powerStrings.DESCRIPTIONS[0];
    }
```

</details>

### atStartOfTurn()

**Creates:**
- `VFXAction` — `new VFXAction(new LightningEffect(this.owner.hb.cX, this.owner.hb.cY))`
- `LightningEffect` — `new LightningEffect(this.owner.hb.cX, this.owner.hb.cY)`
- `LoseHPAction` — `new LoseHPAction(this.owner, this.owner, 99999)`
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        this.flash();
        this.addToBot(new VFXAction(new LightningEffect(this.owner.hb.cX, this.owner.hb.cY)));
        this.addToBot(new LoseHPAction(this.owner, this.owner, 99999));
        this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
    }
```

</details>

## EnergizedBluePower
File: `powers\EnergizedBluePower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1] : DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

## EnergizedPower
File: `powers\EnergizedPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1] : DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

## EnergyDownPower
File: `powers\watcher\EnergyDownPower.java`

### updateDescription()

**Creates:**
- `StringBuilder` — `new StringBuilder()`

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        StringBuilder sb = new StringBuilder();
        sb.append(EnergyDownPower.powerStrings.DESCRIPTIONS[0]);
        for (int i = 0; i < this.amount; ++i) {
            sb.append("[E] ");
        }
        if (EnergyDownPower.powerStrings.DESCRIPTIONS[1].isEmpty()) {
            sb.append(LocalizedStrings.PERIOD);
        } else {
            sb.append(EnergyDownPower.powerStrings.DESCRIPTIONS[1]);
        }
        this.description = sb.toString();
    }
```

</details>

### atStartOfTurn()

**Creates:**
- `LoseEnergyAction` — `new LoseEnergyAction(this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        this.addToBot(new LoseEnergyAction(this.amount));
        this.flash();
    }
```

</details>

## EntanglePower
File: `powers\EntanglePower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = EntanglePower.powerStrings.DESCRIPTIONS[0];
    }
```

</details>

### atEndOfTurn(boolean isPlayer)

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        if (isPlayer) {
            this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        }
    }
```

</details>

## EnvenomPower
File: `powers\EnvenomPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

### onAttack(DamageInfo info, int damageAmount, AbstractCreature target)

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(target, this.owner, (AbstractPower)new PoisonPower(target, this.owner, this.amount), this.amount, true)`
- `PoisonPower` — `new PoisonPower(target, this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void onAttack(DamageInfo info, int damageAmount, AbstractCreature target) {
        if (damageAmount > 0 && target != this.owner && info.type == DamageInfo.DamageType.NORMAL) {
            this.flash();
            this.addToTop(new ApplyPowerAction(target, this.owner, (AbstractPower)new PoisonPower(target, this.owner, this.amount), this.amount, true));
        }
    }
```

</details>

## EquilibriumPower
File: `powers\EquilibriumPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[0] : DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

### atEndOfTurn(boolean isPlayer)

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        if (isPlayer) {
            for (AbstractCard c : AbstractDungeon.player.hand.group) {
                if (c.isEthereal) continue;
                c.retain = true;
            }
        }
    }
```

</details>

### atEndOfRound()

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`
- `ReducePowerAction` — `new ReducePowerAction(this.owner, this.owner, POWER_ID, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfRound() {
        if (this.amount == 0) {
            this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        } else {
            this.addToBot(new ReducePowerAction(this.owner, this.owner, POWER_ID, 1));
        }
    }
```

</details>

## EstablishmentPower
File: `powers\watcher\EstablishmentPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = EstablishmentPower.powerStrings.DESCRIPTIONS[0] + this.amount + EstablishmentPower.powerStrings.DESCRIPTIONS[1];
    }
```

</details>

### atEndOfTurn(boolean isPlayer)

**Creates:**
- `EstablishmentPowerAction` — `new EstablishmentPowerAction(this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        this.flash();
        this.addToBot(new EstablishmentPowerAction(this.amount));
    }
```

</details>

## EvolvePower
File: `powers\EvolvePower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? EvolvePower.powerStrings.DESCRIPTIONS[0] : EvolvePower.powerStrings.DESCRIPTIONS[1] + this.amount + EvolvePower.powerStrings.DESCRIPTIONS[2];
    }
```

</details>

### onCardDraw(AbstractCard card)

**Creates:**
- `DrawCardAction` — `new DrawCardAction(this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void onCardDraw(AbstractCard card) {
        if (card.type == AbstractCard.CardType.STATUS && !this.owner.hasPower("No Draw")) {
            this.flash();
            this.addToBot(new DrawCardAction(this.owner, this.amount));
        }
    }
```

</details>

## ExplosivePower
File: `powers\ExplosivePower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[3] + 30 + DESCRIPTIONS[2] : DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1] + 30 + DESCRIPTIONS[2];
    }
```

</details>

### duringTurn()

**Creates:**
- `VFXAction` — `new VFXAction(new ExplosionSmallEffect(this.owner.hb.cX, this.owner.hb.cY), 0.1f)`
- `ExplosionSmallEffect` — `new ExplosionSmallEffect(this.owner.hb.cX, this.owner.hb.cY)`
- `SuicideAction` — `new SuicideAction((AbstractMonster)this.owner)`
- `DamageInfo` — `new DamageInfo(this.owner, 30, DamageInfo.DamageType.THORNS)`
- `DamageAction` — `new DamageAction(AbstractDungeon.player, damageInfo, AbstractGameAction.AttackEffect.FIRE, true)`
- `ReducePowerAction` — `new ReducePowerAction(this.owner, this.owner, POWER_ID, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void duringTurn() {
        if (this.amount == 1 && !this.owner.isDying) {
            this.addToBot(new VFXAction(new ExplosionSmallEffect(this.owner.hb.cX, this.owner.hb.cY), 0.1f));
            this.addToBot(new SuicideAction((AbstractMonster)this.owner));
            DamageInfo damageInfo = new DamageInfo(this.owner, 30, DamageInfo.DamageType.THORNS);
            this.addToBot(new DamageAction(AbstractDungeon.player, damageInfo, AbstractGameAction.AttackEffect.FIRE, true));
        } else {
            this.addToBot(new ReducePowerAction(this.owner, this.owner, POWER_ID, 1));
            this.updateDescription();
        }
    }
```

</details>

## FadingPower
File: `powers\FadingPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[2] : DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

### duringTurn()

**Creates:**
- `VFXAction` — `new VFXAction(new ExplosionSmallEffect(this.owner.hb.cX, this.owner.hb.cY), 0.1f)`
- `ExplosionSmallEffect` — `new ExplosionSmallEffect(this.owner.hb.cX, this.owner.hb.cY)`
- `SuicideAction` — `new SuicideAction((AbstractMonster)this.owner)`
- `ReducePowerAction` — `new ReducePowerAction(this.owner, this.owner, POWER_ID, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void duringTurn() {
        if (this.amount == 1 && !this.owner.isDying) {
            this.addToBot(new VFXAction(new ExplosionSmallEffect(this.owner.hb.cX, this.owner.hb.cY), 0.1f));
            this.addToBot(new SuicideAction((AbstractMonster)this.owner));
        } else {
            this.addToBot(new ReducePowerAction(this.owner, this.owner, POWER_ID, 1));
            this.updateDescription();
        }
    }
```

</details>

## FeelNoPainPower
File: `powers\FeelNoPainPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

### onExhaust(AbstractCard card)

**Creates:**
- `GainBlockAction` — `new GainBlockAction(this.owner, this.amount, Settings.FAST_MODE)`

<details><summary>Full body</summary>

```java
@Override
    public void onExhaust(AbstractCard card) {
        this.flash();
        this.addToBot(new GainBlockAction(this.owner, this.amount, Settings.FAST_MODE));
    }
```

</details>

## FireBreathingPower
File: `powers\FireBreathingPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = FireBreathingPower.powerStrings.DESCRIPTIONS[0] + this.amount + FireBreathingPower.powerStrings.DESCRIPTIONS[1];
    }
```

</details>

### onCardDraw(AbstractCard card)

**Creates:**
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(this.amount, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.FIRE, true)`

<details><summary>Full body</summary>

```java
@Override
    public void onCardDraw(AbstractCard card) {
        if (card.type == AbstractCard.CardType.STATUS || card.type == AbstractCard.CardType.CURSE) {
            this.flash();
            this.addToBot(new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(this.amount, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.FIRE, true));
        }
    }
```

</details>

## FlameBarrierPower
File: `powers\FlameBarrierPower.java`

### onAttacked(DamageInfo info, int damageAmount)

**Creates:**
- `DamageAction` — `new DamageAction(info.owner, new DamageInfo(this.owner, this.amount, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.FIRE)`
- `DamageInfo` — `new DamageInfo(this.owner, this.amount, DamageInfo.DamageType.THORNS)`

<details><summary>Full body</summary>

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (info.owner != null && info.type != DamageInfo.DamageType.THORNS && info.type != DamageInfo.DamageType.HP_LOSS && info.owner != this.owner) {
            this.flash();
            this.addToTop(new DamageAction(info.owner, new DamageInfo(this.owner, this.amount, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.FIRE));
        }
        return damageAmount;
    }
```

</details>

### atStartOfTurn()

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction((AbstractCreature)AbstractDungeon.player, (AbstractCreature)AbstractDungeon.player, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        this.addToBot(new RemoveSpecificPowerAction((AbstractCreature)AbstractDungeon.player, (AbstractCreature)AbstractDungeon.player, POWER_ID));
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

## FlightPower
File: `powers\FlightPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

### atStartOfTurn()

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        this.amount = this.storedAmount;
        this.updateDescription();
    }
```

</details>

### atDamageFinalReceive(float damage, DamageInfo.DamageType type)

<details><summary>Full body</summary>

```java
@Override
    public float atDamageFinalReceive(float damage, DamageInfo.DamageType type) {
        return this.calculateDamageTakenAmount(damage, type);
    }
```

</details>

### onAttacked(DamageInfo info, int damageAmount)

**Creates:**
- `ReducePowerAction` — `new ReducePowerAction(this.owner, this.owner, POWER_ID, 1)`

<details><summary>Full body</summary>

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        Boolean willLive = this.calculateDamageTakenAmount(damageAmount, info.type) < (float)this.owner.currentHealth;
        if (info.owner != null && info.type != DamageInfo.DamageType.HP_LOSS && info.type != DamageInfo.DamageType.THORNS && damageAmount > 0 && willLive.booleanValue()) {
            this.flash();
            this.addToBot(new ReducePowerAction(this.owner, this.owner, POWER_ID, 1));
        }
        return damageAmount;
    }
```

</details>

### onRemove()

**Creates:**
- `ChangeStateAction` — `new ChangeStateAction((AbstractMonster)this.owner, "GROUNDED")`

<details><summary>Full body</summary>

```java
@Override
    public void onRemove() {
        this.addToBot(new ChangeStateAction((AbstractMonster)this.owner, "GROUNDED"));
    }
```

</details>

## FocusPower
File: `powers\FocusPower.java`

### reducePower(int reduceAmount)

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, NAME)`

<details><summary>Full body</summary>

```java
@Override
    public void reducePower(int reduceAmount) {
        this.fontScale = 8.0f;
        this.amount -= reduceAmount;
        if (this.amount == 0) {
            this.addToTop(new RemoveSpecificPowerAction(this.owner, this.owner, NAME));
        }
        if (this.amount >= 999) {
            this.amount = 999;
        }
        if (this.amount <= -999) {
            this.amount = -999;
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        if (this.amount > 0) {
            this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[2];
            this.type = AbstractPower.PowerType.BUFF;
        } else {
            int tmp = -this.amount;
            this.description = DESCRIPTIONS[1] + tmp + DESCRIPTIONS[2];
            this.type = AbstractPower.PowerType.DEBUFF;
        }
    }
```

</details>

## ForcefieldPower
File: `powers\ForcefieldPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0];
    }
```

</details>

### atDamageFinalReceive(float damage, DamageInfo.DamageType type)

<details><summary>Full body</summary>

```java
@Override
    public float atDamageFinalReceive(float damage, DamageInfo.DamageType type) {
        if (damage > 0.0f && type != DamageInfo.DamageType.HP_LOSS && type != DamageInfo.DamageType.THORNS) {
            return 0.0f;
        }
        return damage;
    }
```

</details>

## ForesightPower
File: `powers\watcher\ForesightPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = ForesightPower.powerStrings.DESCRIPTIONS[0] + this.amount + ForesightPower.powerStrings.DESCRIPTIONS[1];
    }
```

</details>

### atStartOfTurn()

**Creates:**
- `EmptyDeckShuffleAction` — `new EmptyDeckShuffleAction()`
- `ScryAction` — `new ScryAction(this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        if (AbstractDungeon.player.drawPile.size() <= 0) {
            this.addToTop(new EmptyDeckShuffleAction());
        }
        this.flash();
        this.addToBot(new ScryAction(this.amount));
    }
```

</details>

## FrailPower
File: `powers\FrailPower.java`

### atEndOfRound()

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`
- `ReducePowerAction` — `new ReducePowerAction(this.owner, this.owner, POWER_ID, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfRound() {
        if (this.justApplied) {
            this.justApplied = false;
            return;
        }
        if (this.amount == 0) {
            this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        } else {
            this.addToBot(new ReducePowerAction(this.owner, this.owner, POWER_ID, 1));
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1] : DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

## FreeAttackPower
File: `powers\watcher\FreeAttackPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? FreeAttackPower.powerStrings.DESCRIPTIONS[0] : FreeAttackPower.powerStrings.DESCRIPTIONS[1] + this.amount + FreeAttackPower.powerStrings.DESCRIPTIONS[2];
    }
```

</details>

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.ATTACK && !card.purgeOnUse && this.amount > 0) {
            this.flash();
            --this.amount;
            if (this.amount == 0) {
                this.addToTop(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
            }
        }
    }
```

</details>

## GainStrengthPower
File: `powers\GainStrengthPower.java`

### reducePower(int reduceAmount)

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, NAME)`

<details><summary>Full body</summary>

```java
@Override
    public void reducePower(int reduceAmount) {
        this.fontScale = 8.0f;
        this.amount -= reduceAmount;
        if (this.amount == 0) {
            this.addToTop(new RemoveSpecificPowerAction(this.owner, this.owner, NAME));
        }
        if (this.amount >= 999) {
            this.amount = 999;
        }
        if (this.amount <= -999) {
            this.amount = -999;
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

### atEndOfTurn(boolean isPlayer)

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, this.amount), this.amount)`
- `StrengthPower` — `new StrengthPower(this.owner, this.amount)`
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        this.flash();
        this.addToBot(new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, this.amount), this.amount));
        this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
    }
```

</details>

## GenericStrengthUpPower
File: `powers\GenericStrengthUpPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

### atEndOfRound()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, this.amount), this.amount)`
- `StrengthPower` — `new StrengthPower(this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfRound() {
        this.flash();
        this.addToBot(new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, this.amount), this.amount));
    }
```

</details>

## GrowthPower
File: `powers\GrowthPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

### atEndOfRound()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, this.amount), this.amount)`
- `StrengthPower` — `new StrengthPower(this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfRound() {
        if (!this.skipFirst) {
            this.flash();
            this.addToBot(new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, this.amount), this.amount));
        } else {
            this.skipFirst = false;
        }
    }
```

</details>

## HeatsinkPower
File: `powers\HeatsinkPower.java`

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `DrawCardAction` — `new DrawCardAction(this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.POWER) {
            this.flash();
            this.addToTop(new DrawCardAction(this.owner, this.amount));
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount <= 1 ? DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1] : DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

## HelloPower
File: `powers\HelloPower.java`

### atStartOfTurn()

**Creates:**
- `MakeTempCardInHandAction` — `new MakeTempCardInHandAction(AbstractDungeon.getCard(AbstractCard.CardRarity.COMMON, AbstractDungeon.cardRandomRng).makeCopy(), 1, false)`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        if (!AbstractDungeon.getMonsters().areMonstersBasicallyDead()) {
            this.flash();
            for (int i = 0; i < this.amount; ++i) {
                this.addToBot(new MakeTempCardInHandAction(AbstractDungeon.getCard(AbstractCard.CardRarity.COMMON, AbstractDungeon.cardRandomRng).makeCopy(), 1, false));
            }
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount > 1 ? DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1] : DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

## HexPower
File: `powers\HexPower.java`

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `MakeTempCardInDrawPileAction` — `new MakeTempCardInDrawPileAction(new Dazed(), this.amount, true, true)`
- `Dazed` — `new Dazed()`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type != AbstractCard.CardType.ATTACK) {
            this.flash();
            this.addToBot(new MakeTempCardInDrawPileAction(new Dazed(), this.amount, true, true));
        }
    }
```

</details>

## InfiniteBladesPower
File: `powers\InfiniteBladesPower.java`

### atStartOfTurn()

**Creates:**
- `MakeTempCardInHandAction` — `new MakeTempCardInHandAction((AbstractCard)new Shiv(), this.amount, false)`
- `Shiv` — `new Shiv()`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        if (!AbstractDungeon.getMonsters().areMonstersBasicallyDead()) {
            this.flash();
            this.addToBot(new MakeTempCardInHandAction((AbstractCard)new Shiv(), this.amount, false));
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount > 1 ? InfiniteBladesPower.powerStrings.DESCRIPTIONS[0] + this.amount + InfiniteBladesPower.powerStrings.DESCRIPTIONS[1] : InfiniteBladesPower.powerStrings.DESCRIPTIONS[0] + this.amount + InfiniteBladesPower.powerStrings.DESCRIPTIONS[2];
    }
```

</details>

## IntangiblePlayerPower
File: `powers\IntangiblePlayerPower.java`

### atDamageFinalReceive(float damage, DamageInfo.DamageType type)

<details><summary>Full body</summary>

```java
@Override
    public float atDamageFinalReceive(float damage, DamageInfo.DamageType type) {
        if (damage > 1.0f) {
            damage = 1.0f;
        }
        return damage;
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0];
    }
```

</details>

### atEndOfRound()

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`
- `ReducePowerAction` — `new ReducePowerAction(this.owner, this.owner, POWER_ID, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfRound() {
        this.flash();
        if (this.amount == 0) {
            this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        } else {
            this.addToBot(new ReducePowerAction(this.owner, this.owner, POWER_ID, 1));
        }
    }
```

</details>

## IntangiblePower
File: `powers\IntangiblePower.java`

### atDamageFinalReceive(float damage, DamageInfo.DamageType type)

<details><summary>Full body</summary>

```java
@Override
    public float atDamageFinalReceive(float damage, DamageInfo.DamageType type) {
        if (damage > 1.0f) {
            damage = 1.0f;
        }
        return damage;
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0];
    }
```

</details>

### atEndOfTurn(boolean isPlayer)

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`
- `ReducePowerAction` — `new ReducePowerAction(this.owner, this.owner, POWER_ID, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        if (this.justApplied) {
            this.justApplied = false;
            return;
        }
        this.flash();
        if (this.amount == 0) {
            this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        } else {
            this.addToBot(new ReducePowerAction(this.owner, this.owner, POWER_ID, 1));
        }
    }
```

</details>

## InvinciblePower
File: `powers\InvinciblePower.java`

### onAttackedToChangeDamage(DamageInfo info, int damageAmount)

<details><summary>Full body</summary>

```java
@Override
    public int onAttackedToChangeDamage(DamageInfo info, int damageAmount) {
        if (damageAmount > this.amount) {
            damageAmount = this.amount;
        }
        this.amount -= damageAmount;
        if (this.amount < 0) {
            this.amount = 0;
        }
        this.updateDescription();
        return damageAmount;
    }
```

</details>

### atStartOfTurn()

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        this.amount = this.maxAmt;
        this.updateDescription();
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount <= 0 ? DESCRIPTIONS[2] : DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

## JuggernautPower
File: `powers\JuggernautPower.java`

### onGainedBlock(float blockAmount)

**Creates:**
- `DamageRandomEnemyAction` — `new DamageRandomEnemyAction(new DamageInfo(this.owner, this.amount, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL)`
- `DamageInfo` — `new DamageInfo(this.owner, this.amount, DamageInfo.DamageType.THORNS)`

<details><summary>Full body</summary>

```java
@Override
    public void onGainedBlock(float blockAmount) {
        if (blockAmount > 0.0f) {
            this.flash();
            this.addToBot(new DamageRandomEnemyAction(new DamageInfo(this.owner, this.amount, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL));
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

## LightningMasteryPower
File: `powers\LightningMasteryPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

## LikeWaterPower
File: `powers\watcher\LikeWaterPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = LikeWaterPower.powerStrings.DESCRIPTIONS[0] + this.amount + LikeWaterPower.powerStrings.DESCRIPTIONS[1];
    }
```

</details>

### atEndOfTurnPreEndTurnCards(boolean isPlayer)

**Creates:**
- `GainBlockAction` — `new GainBlockAction(this.owner, this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurnPreEndTurnCards(boolean isPlayer) {
        if (isPlayer) {
            AbstractPlayer p = (AbstractPlayer)this.owner;
            if (p.stance.ID.equals("Calm")) {
                this.flash();
                this.addToBot(new GainBlockAction(this.owner, this.owner, this.amount));
            }
        }
    }
```

</details>

## LiveForeverPower
File: `powers\watcher\LiveForeverPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = LiveForeverPower.powerStrings.DESCRIPTIONS[0] + this.amount + LiveForeverPower.powerStrings.DESCRIPTIONS[1];
    }
```

</details>

### atEndOfTurn(boolean isPlayer)

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(this.owner, this.owner, new PlatedArmorPower(this.owner, this.amount), this.amount)`
- `PlatedArmorPower` — `new PlatedArmorPower(this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        this.flash();
        this.addToBot(new ApplyPowerAction(this.owner, this.owner, new PlatedArmorPower(this.owner, this.amount), this.amount));
    }
```

</details>

## LockOnPower
File: `powers\LockOnPower.java`

### atEndOfRound()

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`
- `ReducePowerAction` — `new ReducePowerAction(this.owner, this.owner, POWER_ID, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfRound() {
        if (this.amount == 0) {
            this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        } else {
            this.addToBot(new ReducePowerAction(this.owner, this.owner, POWER_ID, 1));
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        if (this.owner != null) {
            this.description = this.amount == 1 ? DESCRIPTIONS[0] + 50 + DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2] : DESCRIPTIONS[0] + 50 + DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[3];
        }
    }
```

</details>

## LoopPower
File: `powers\LoopPower.java`

### atStartOfTurn()

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        if (!AbstractDungeon.player.orbs.isEmpty()) {
            this.flash();
            for (int i = 0; i < this.amount; ++i) {
                AbstractDungeon.player.orbs.get(0).onStartOfTurn();
                AbstractDungeon.player.orbs.get(0).onEndOfTurn();
            }
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount <= 1 ? DESCRIPTIONS[0] : DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

## LoseDexterityPower
File: `powers\LoseDexterityPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = LoseDexterityPower.powerStrings.DESCRIPTIONS[0] + this.amount + LoseDexterityPower.powerStrings.DESCRIPTIONS[1];
    }
```

</details>

### atEndOfTurn(boolean isPlayer)

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(this.owner, this.owner, new DexterityPower(this.owner, -this.amount), -this.amount)`
- `DexterityPower` — `new DexterityPower(this.owner, -this.amount)`
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        this.flash();
        this.addToBot(new ApplyPowerAction(this.owner, this.owner, new DexterityPower(this.owner, -this.amount), -this.amount));
        this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
    }
```

</details>

## LoseStrengthPower
File: `powers\LoseStrengthPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

### atEndOfTurn(boolean isPlayer)

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, -this.amount), -this.amount)`
- `StrengthPower` — `new StrengthPower(this.owner, -this.amount)`
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        this.flash();
        this.addToBot(new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, -this.amount), -this.amount));
        this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
    }
```

</details>

## MagnetismPower
File: `powers\MagnetismPower.java`

### atStartOfTurn()

**Creates:**
- `MakeTempCardInHandAction` — `new MakeTempCardInHandAction(AbstractDungeon.returnTrulyRandomColorlessCardInCombat().makeCopy(), 1, false)`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        if (!AbstractDungeon.getMonsters().areMonstersBasicallyDead()) {
            this.flash();
            for (int i = 0; i < this.amount; ++i) {
                this.addToBot(new MakeTempCardInHandAction(AbstractDungeon.returnTrulyRandomColorlessCardInCombat().makeCopy(), 1, false));
            }
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount > 1 ? String.format(PLURAL_DESCRIPTION, this.amount) : String.format(SINGULAR_DESCRIPTION, this.amount);
    }
```

</details>

## MalleablePower
File: `powers\MalleablePower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1] + NAME + DESCRIPTIONS[2] + this.basePower + DESCRIPTIONS[3];
    }
```

</details>

### atEndOfTurn(boolean isPlayer)

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        if (this.owner.isPlayer) {
            return;
        }
        this.amount = this.basePower;
        this.updateDescription();
    }
```

</details>

### atEndOfRound()

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfRound() {
        if (!this.owner.isPlayer) {
            return;
        }
        this.amount = this.basePower;
        this.updateDescription();
    }
```

</details>

### onAttacked(DamageInfo info, int damageAmount)

**Creates:**
- `GainBlockAction` — `new GainBlockAction(this.owner, this.owner, this.amount)`
- `GainBlockAction` — `new GainBlockAction(this.owner, this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (damageAmount < this.owner.currentHealth && damageAmount > 0 && info.owner != null && info.type == DamageInfo.DamageType.NORMAL && info.type != DamageInfo.DamageType.HP_LOSS) {
            this.flash();
            if (this.owner.isPlayer) {
                this.addToTop(new GainBlockAction(this.owner, this.owner, this.amount));
            } else {
                this.addToBot(new GainBlockAction(this.owner, this.owner, this.amount));
            }
            ++this.amount;
            this.updateDescription();
        }
        return damageAmount;
    }
```

</details>

## MantraPower
File: `powers\watcher\MantraPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = MantraPower.powerStrings.DESCRIPTIONS[0] + 10 + MantraPower.powerStrings.DESCRIPTIONS[1];
    }
```

</details>

## MarkPower
File: `powers\watcher\MarkPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = MarkPower.powerStrings.DESCRIPTIONS[0] + this.amount + MarkPower.powerStrings.DESCRIPTIONS[1];
    }
```

</details>

## MasterRealityPower
File: `powers\watcher\MasterRealityPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = MasterRealityPower.powerStrings.DESCRIPTIONS[0];
    }
```

</details>

## MayhemPower
File: `powers\MayhemPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[0] : DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

### atStartOfTurn()

**Creates:**
- `AbstractGameAction` — `new AbstractGameAction(){

                @Override
                public void update() {
                    this.addToBot(new PlayTopCardAction(AbstractDungeon.getCurrRoom().monsters.getRandomMons...`
- `PlayTopCardAction` — `new PlayTopCardAction(AbstractDungeon.getCurrRoom().monsters.getRandomMonster(null, true, AbstractDungeon.cardRandomRng), false)`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        this.flash();
        for (int i = 0; i < this.amount; ++i) {
            this.addToBot(new AbstractGameAction(){

                @Override
                public void update() {
                    this.addToBot(new PlayTopCardAction(AbstractDungeon.getCurrRoom().monsters.getRandomMonster(null, true, AbstractDungeon.cardRandomRng), false));
                    this.isDone = true;
                }
            });
        }
    }
```

</details>

## MentalFortressPower
File: `powers\watcher\MentalFortressPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = MentalFortressPower.powerStrings.DESCRIPTIONS[0] + this.amount + MentalFortressPower.powerStrings.DESCRIPTIONS[1];
    }
```

</details>

### onChangeStance(AbstractStance oldStance, AbstractStance newStance)

**Creates:**
- `GainBlockAction` — `new GainBlockAction(this.owner, this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void onChangeStance(AbstractStance oldStance, AbstractStance newStance) {
        if (!oldStance.ID.equals(newStance.ID)) {
            this.flash();
            this.addToBot(new GainBlockAction(this.owner, this.owner, this.amount));
        }
    }
```

</details>

## MetallicizePower
File: `powers\MetallicizePower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.owner.isPlayer ? DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1] : DESCRIPTIONS[2] + this.amount + DESCRIPTIONS[3];
    }
```

</details>

### atEndOfTurnPreEndTurnCards(boolean isPlayer)

**Creates:**
- `GainBlockAction` — `new GainBlockAction(this.owner, this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurnPreEndTurnCards(boolean isPlayer) {
        this.flash();
        this.addToBot(new GainBlockAction(this.owner, this.owner, this.amount));
    }
```

</details>

## MinionPower
File: `powers\MinionPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0];
    }
```

</details>

## ModeShiftPower
File: `powers\ModeShiftPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

## NextTurnBlockPower
File: `powers\NextTurnBlockPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

### atStartOfTurn()

**Creates:**
- `FlashAtkImgEffect` — `new FlashAtkImgEffect(this.owner.hb.cX, this.owner.hb.cY, AbstractGameAction.AttackEffect.SHIELD)`
- `GainBlockAction` — `new GainBlockAction(this.owner, this.owner, this.amount)`
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        this.flash();
        AbstractDungeon.effectList.add(new FlashAtkImgEffect(this.owner.hb.cX, this.owner.hb.cY, AbstractGameAction.AttackEffect.SHIELD));
        this.addToBot(new GainBlockAction(this.owner, this.owner, this.amount));
        this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
    }
```

</details>

## NightmarePower
File: `powers\NightmarePower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + " " + FontHelper.colorString(this.card.name, "y") + DESCRIPTIONS[1];
    }
```

</details>

### atStartOfTurn()

**Creates:**
- `MakeTempCardInHandAction` — `new MakeTempCardInHandAction(this.card, this.amount)`
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        this.addToBot(new MakeTempCardInHandAction(this.card, this.amount));
        this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
    }
```

</details>

## NirvanaPower
File: `powers\watcher\NirvanaPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = NirvanaPower.powerStrings.DESCRIPTIONS[0] + this.amount + NirvanaPower.powerStrings.DESCRIPTIONS[1];
    }
```

</details>

## NoBlockPower
File: `powers\NoBlockPower.java`

### atEndOfRound()

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`
- `ReducePowerAction` — `new ReducePowerAction(this.owner, this.owner, POWER_ID, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfRound() {
        if (this.justApplied) {
            this.justApplied = false;
            return;
        }
        if (this.amount == 0) {
            this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        } else {
            this.addToBot(new ReducePowerAction(this.owner, this.owner, POWER_ID, 1));
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0];
    }
```

</details>

## NoDrawPower
File: `powers\NoDrawPower.java`

### atEndOfTurn(boolean isPlayer)

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        if (isPlayer) {
            this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        }
    }
```

</details>

## NoSkillsPower
File: `powers\watcher\NoSkillsPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = NoSkillsPower.powerStrings.DESCRIPTIONS[0];
    }
```

</details>

### atEndOfTurn(boolean isPlayer)

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        if (isPlayer) {
            this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        }
    }
```

</details>

## NoxiousFumesPower
File: `powers\NoxiousFumesPower.java`

### atStartOfTurnPostDraw()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(m, this.owner, new PoisonPower(m, this.owner, this.amount), this.amount)`
- `PoisonPower` — `new PoisonPower(m, this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurnPostDraw() {
        if (!AbstractDungeon.getMonsters().areMonstersBasicallyDead()) {
            this.flash();
            for (AbstractMonster m : AbstractDungeon.getMonsters().monsters) {
                if (m.isDead || m.isDying) continue;
                this.addToBot(new ApplyPowerAction(m, this.owner, new PoisonPower(m, this.owner, this.amount), this.amount));
            }
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

## OmegaPower
File: `powers\watcher\OmegaPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = OmegaPower.powerStrings.DESCRIPTIONS[0] + this.amount + OmegaPower.powerStrings.DESCRIPTIONS[1];
    }
```

</details>

### atEndOfTurn(boolean isPlayer)

**Creates:**
- `VFXAction` — `new VFXAction(new OmegaFlashEffect(m.hb.cX, m.hb.cY))`
- `OmegaFlashEffect` — `new OmegaFlashEffect(m.hb.cX, m.hb.cY)`
- `VFXAction` — `new VFXAction(new OmegaFlashEffect(m.hb.cX, m.hb.cY), 0.2f)`
- `OmegaFlashEffect` — `new OmegaFlashEffect(m.hb.cX, m.hb.cY)`
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(this.amount, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.FIRE, true)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        if (isPlayer) {
            this.flash();
            for (AbstractMonster m : AbstractDungeon.getMonsters().monsters) {
                if (m == null || m.isDeadOrEscaped()) continue;
                if (Settings.FAST_MODE) {
                    this.addToBot(new VFXAction(new OmegaFlashEffect(m.hb.cX, m.hb.cY)));
                    continue;
                }
                this.addToBot(new VFXAction(new OmegaFlashEffect(m.hb.cX, m.hb.cY), 0.2f));
            }
            this.addToBot(new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(this.amount, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.FIRE, true));
        }
    }
```

</details>

## OmnisciencePower
File: `powers\watcher\OmnisciencePower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = OmnisciencePower.powerStrings.DESCRIPTIONS[0] + this.amount + OmnisciencePower.powerStrings.DESCRIPTIONS[1];
    }
```

</details>

## PainfulStabsPower
File: `powers\PainfulStabsPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0];
    }
```

</details>

### onInflictDamage(DamageInfo info, int damageAmount, AbstractCreature target)

**Creates:**
- `MakeTempCardInDiscardAction` — `new MakeTempCardInDiscardAction((AbstractCard)new Wound(), 1)`
- `Wound` — `new Wound()`

<details><summary>Full body</summary>

```java
@Override
    public void onInflictDamage(DamageInfo info, int damageAmount, AbstractCreature target) {
        if (damageAmount > 0 && info.type != DamageInfo.DamageType.THORNS) {
            this.addToBot(new MakeTempCardInDiscardAction((AbstractCard)new Wound(), 1));
        }
    }
```

</details>

## PanachePower
File: `powers\PanachePower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1] + this.damage + DESCRIPTIONS[2] : DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[3] + this.damage + DESCRIPTIONS[2];
    }
```

</details>

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction((AbstractCreature)AbstractDungeon.player, DamageInfo.createDamageMatrix(this.damage, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.SLASH_DIAGONAL)`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        --this.amount;
        if (this.amount == 0) {
            this.flash();
            this.amount = 5;
            this.addToBot(new DamageAllEnemiesAction((AbstractCreature)AbstractDungeon.player, DamageInfo.createDamageMatrix(this.damage, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.SLASH_DIAGONAL));
        }
        this.updateDescription();
    }
```

</details>

### atStartOfTurn()

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        this.amount = 5;
        this.updateDescription();
    }
```

</details>

## PenNibPower
File: `powers\PenNibPower.java`

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.ATTACK) {
            this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0];
    }
```

</details>

### atDamageGive(float damage, DamageInfo.DamageType type)

<details><summary>Full body</summary>

```java
@Override
    public float atDamageGive(float damage, DamageInfo.DamageType type) {
        if (type == DamageInfo.DamageType.NORMAL) {
            return damage * 2.0f;
        }
        return damage;
    }
```

</details>

## PhantasmalPower
File: `powers\PhantasmalPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[0] : DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

### atStartOfTurn()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(this.owner, this.owner, new DoubleDamagePower(this.owner, 1, false), this.amount)`
- `DoubleDamagePower` — `new DoubleDamagePower(this.owner, 1, false)`
- `ReducePowerAction` — `new ReducePowerAction(this.owner, this.owner, POWER_ID, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        this.flash();
        this.addToBot(new ApplyPowerAction(this.owner, this.owner, new DoubleDamagePower(this.owner, 1, false), this.amount));
        this.addToBot(new ReducePowerAction(this.owner, this.owner, POWER_ID, 1));
    }
```

</details>

## PlatedArmorPower
File: `powers\PlatedArmorPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.owner.isPlayer ? DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1] : DESCRIPTIONS[2] + this.amount + DESCRIPTIONS[3];
    }
```

</details>

### wasHPLost(DamageInfo info, int damageAmount)

**Creates:**
- `ReducePowerAction` — `new ReducePowerAction(this.owner, this.owner, POWER_ID, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void wasHPLost(DamageInfo info, int damageAmount) {
        if (info.owner != null && info.owner != this.owner && info.type != DamageInfo.DamageType.HP_LOSS && info.type != DamageInfo.DamageType.THORNS && damageAmount > 0) {
            this.flash();
            this.addToBot(new ReducePowerAction(this.owner, this.owner, POWER_ID, 1));
        }
    }
```

</details>

### onRemove()

**Creates:**
- `ChangeStateAction` — `new ChangeStateAction((AbstractMonster)this.owner, "ARMOR_BREAK")`

<details><summary>Full body</summary>

```java
@Override
    public void onRemove() {
        if (!this.owner.isPlayer) {
            this.addToBot(new ChangeStateAction((AbstractMonster)this.owner, "ARMOR_BREAK"));
        }
    }
```

</details>

### atEndOfTurnPreEndTurnCards(boolean isPlayer)

**Creates:**
- `GainBlockAction` — `new GainBlockAction(this.owner, this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurnPreEndTurnCards(boolean isPlayer) {
        this.flash();
        this.addToBot(new GainBlockAction(this.owner, this.owner, this.amount));
    }
```

</details>

## PoisonPower
File: `powers\PoisonPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.owner == null || this.owner.isPlayer ? DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1] : DESCRIPTIONS[2] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

### atStartOfTurn()

**Creates:**
- `PoisonLoseHpAction` — `new PoisonLoseHpAction(this.owner, this.source, this.amount, AbstractGameAction.AttackEffect.POISON)`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        if (AbstractDungeon.getCurrRoom().phase == AbstractRoom.RoomPhase.COMBAT && !AbstractDungeon.getMonsters().areMonstersBasicallyDead()) {
            this.flashWithoutSound();
            this.addToBot(new PoisonLoseHpAction(this.owner, this.source, this.amount, AbstractGameAction.AttackEffect.POISON));
        }
    }
```

</details>

## RagePower
File: `powers\RagePower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)AbstractDungeon.player, AbstractDungeon.player, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.ATTACK) {
            this.addToBot(new GainBlockAction((AbstractCreature)AbstractDungeon.player, AbstractDungeon.player, this.amount));
            this.flash();
        }
    }
```

</details>

### atEndOfTurn(boolean isPlayer)

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
    }
```

</details>

## ReactivePower
File: `powers\ReactivePower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0];
    }
```

</details>

### onAttacked(DamageInfo info, int damageAmount)

**Creates:**
- `RollMoveAction` — `new RollMoveAction((AbstractMonster)this.owner)`

<details><summary>Full body</summary>

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (info.owner != null && info.type != DamageInfo.DamageType.HP_LOSS && info.type != DamageInfo.DamageType.THORNS && damageAmount > 0 && damageAmount < this.owner.currentHealth) {
            this.flash();
            this.addToBot(new RollMoveAction((AbstractMonster)this.owner));
        }
        return damageAmount;
    }
```

</details>

## ReboundPower
File: `powers\ReboundPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount > 1 ? DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2] : DESCRIPTIONS[0];
    }
```

</details>

### onAfterUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `ReducePowerAction` — `new ReducePowerAction(this.owner, this.owner, POWER_ID, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void onAfterUseCard(AbstractCard card, UseCardAction action) {
        if (this.justEvoked) {
            this.justEvoked = false;
            return;
        }
        if (card.type != AbstractCard.CardType.POWER) {
            this.flash();
            action.reboundCard = true;
        }
        this.addToBot(new ReducePowerAction(this.owner, this.owner, POWER_ID, 1));
    }
```

</details>

### atEndOfTurn(boolean isPlayer)

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        if (isPlayer) {
            this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        }
    }
```

</details>

## RechargingCorePower
File: `powers\RechargingCorePower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.turnTimer;
        this.description = this.turnTimer == 1 ? this.description + DESCRIPTIONS[1] : this.description + DESCRIPTIONS[2];
        for (int i = 0; i < this.amount; ++i) {
            this.description = this.description + DESCRIPTIONS[3];
        }
        this.description = this.description + " .";
    }
```

</details>

### atStartOfTurn()

**Creates:**
- `GainEnergyAction` — `new GainEnergyAction(this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        this.updateDescription();
        if (this.turnTimer == 1) {
            this.flash();
            this.turnTimer = 3;
            this.addToBot(new GainEnergyAction(this.amount));
        } else {
            --this.turnTimer;
        }
        this.updateDescription();
    }
```

</details>

## RegenPower
File: `powers\RegenPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

### atEndOfTurn(boolean isPlayer)

**Creates:**
- `RegenAction` — `new RegenAction(this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        this.flashWithoutSound();
        this.addToTop(new RegenAction(this.owner, this.amount));
    }
```

</details>

## RegenerateMonsterPower
File: `powers\RegenerateMonsterPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

### atEndOfTurn(boolean isPlayer)

**Creates:**
- `HealAction` — `new HealAction(this.owner, this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        this.flash();
        if (!(this.owner.halfDead || this.owner.isDying || this.owner.isDead)) {
            this.addToBot(new HealAction(this.owner, this.owner, this.amount));
        }
    }
```

</details>

## RegrowPower
File: `powers\RegrowPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0];
    }
```

</details>

## RepairPower
File: `powers\RepairPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

### onVictory()

<details><summary>Full body</summary>

```java
@Override
    public void onVictory() {
        AbstractPlayer p = AbstractDungeon.player;
        if (p.currentHealth > 0) {
            p.heal(this.amount);
        }
    }
```

</details>

## ResurrectPower
File: `powers\ResurrectPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0];
    }
```

</details>

## RetainCardPower
File: `powers\RetainCardPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1] : DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[2];
    }
```

</details>

### atEndOfTurn(boolean isPlayer)

**Creates:**
- `RetainCardsAction` — `new RetainCardsAction(this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        if (isPlayer && !AbstractDungeon.player.hand.isEmpty() && !AbstractDungeon.player.hasRelic("Runic Pyramid") && !AbstractDungeon.player.hasPower("Equilibrium")) {
            this.addToBot(new RetainCardsAction(this.owner, this.amount));
        }
    }
```

</details>

## RitualPower
File: `powers\RitualPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = !this.onPlayer ? DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1] : DESCRIPTIONS[2] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

### atEndOfTurn(boolean isPlayer)

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, this.amount), this.amount)`
- `StrengthPower` — `new StrengthPower(this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        if (isPlayer) {
            this.flash();
            this.addToBot(new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, this.amount), this.amount));
        }
    }
```

</details>

### atEndOfRound()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, this.amount), this.amount)`
- `StrengthPower` — `new StrengthPower(this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfRound() {
        if (!this.onPlayer) {
            if (!this.skipFirst) {
                this.flash();
                this.addToBot(new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, this.amount), this.amount));
            } else {
                this.skipFirst = false;
            }
        }
    }
```

</details>

## RupturePower
File: `powers\RupturePower.java`

### wasHPLost(DamageInfo info, int damageAmount)

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, this.amount), this.amount)`
- `StrengthPower` — `new StrengthPower(this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void wasHPLost(DamageInfo info, int damageAmount) {
        if (damageAmount > 0 && info.owner == this.owner) {
            this.flash();
            this.addToTop(new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, this.amount), this.amount));
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

## RushdownPower
File: `powers\watcher\RushdownPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount > 1 ? RushdownPower.powerStrings.DESCRIPTIONS[0] + this.amount + RushdownPower.powerStrings.DESCRIPTIONS[2] : RushdownPower.powerStrings.DESCRIPTIONS[0] + this.amount + RushdownPower.powerStrings.DESCRIPTIONS[1];
    }
```

</details>

### onChangeStance(AbstractStance oldStance, AbstractStance newStance)

**Creates:**
- `DrawCardAction` — `new DrawCardAction(this.owner, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void onChangeStance(AbstractStance oldStance, AbstractStance newStance) {
        if (!oldStance.ID.equals(newStance.ID) && newStance.ID.equals("Wrath")) {
            this.flash();
            this.addToBot(new DrawCardAction(this.owner, this.amount));
        }
    }
```

</details>

## SadisticPower
File: `powers\SadisticPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

### onApplyPower(AbstractPower power, AbstractCreature target, AbstractCreature source)

**Creates:**
- `DamageAction` — `new DamageAction(target, new DamageInfo(this.owner, this.amount, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.FIRE)`
- `DamageInfo` — `new DamageInfo(this.owner, this.amount, DamageInfo.DamageType.THORNS)`

<details><summary>Full body</summary>

```java
@Override
    public void onApplyPower(AbstractPower power, AbstractCreature target, AbstractCreature source) {
        if (power.type == AbstractPower.PowerType.DEBUFF && !power.ID.equals("Shackled") && source == this.owner && target != this.owner && !target.hasPower("Artifact")) {
            this.flash();
            this.addToBot(new DamageAction(target, new DamageInfo(this.owner, this.amount, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.FIRE));
        }
    }
```

</details>

## SharpHidePower
File: `powers\SharpHidePower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `DamageAction` — `new DamageAction((AbstractCreature)AbstractDungeon.player, new DamageInfo(this.owner, this.amount, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL)`
- `DamageInfo` — `new DamageInfo(this.owner, this.amount, DamageInfo.DamageType.THORNS)`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.ATTACK) {
            this.flash();
            this.addToBot(new DamageAction((AbstractCreature)AbstractDungeon.player, new DamageInfo(this.owner, this.amount, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL));
        }
    }
```

</details>

## ShiftingPower
File: `powers\ShiftingPower.java`

### onAttacked(DamageInfo info, int damageAmount)

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, -damageAmount), -damageAmount)`
- `StrengthPower` — `new StrengthPower(this.owner, -damageAmount)`
- `ApplyPowerAction` — `new ApplyPowerAction(this.owner, this.owner, new GainStrengthPower(this.owner, damageAmount), damageAmount)`
- `GainStrengthPower` — `new GainStrengthPower(this.owner, damageAmount)`

<details><summary>Full body</summary>

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (damageAmount > 0) {
            this.addToTop(new ApplyPowerAction(this.owner, this.owner, new StrengthPower(this.owner, -damageAmount), -damageAmount));
            if (!this.owner.hasPower("Artifact")) {
                this.addToTop(new ApplyPowerAction(this.owner, this.owner, new GainStrengthPower(this.owner, damageAmount), damageAmount));
            }
            this.flash();
        }
        return damageAmount;
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[1];
    }
```

</details>

## SkillBurnPower
File: `powers\SkillBurnPower.java`

### atEndOfRound()

**Creates:**
- `ReducePowerAction` — `new ReducePowerAction(this.owner, this.owner, POWER_ID, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfRound() {
        if (this.justApplied) {
            this.justApplied = false;
            return;
        }
        this.addToBot(new ReducePowerAction(this.owner, this.owner, POWER_ID, 1));
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1] : DESCRIPTIONS[2] + this.amount + DESCRIPTIONS[3];
    }
```

</details>

### onUseCard(AbstractCard card, UseCardAction action)

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.SKILL) {
            this.flash();
            action.exhaustCard = true;
        }
    }
```

</details>

## SlowPower
File: `powers\SlowPower.java`

### atEndOfRound()

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfRound() {
        this.amount = 0;
        this.updateDescription();
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + FontHelper.colorString(this.owner.name, "y") + DESCRIPTIONS[1];
        if (this.amount != 0) {
            this.description = this.description + DESCRIPTIONS[2] + this.amount * 10 + DESCRIPTIONS[3];
        }
    }
```

</details>

### onAfterUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(this.owner, this.owner, new SlowPower(this.owner, 1), 1)`
- `SlowPower` — `new SlowPower(this.owner, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void onAfterUseCard(AbstractCard card, UseCardAction action) {
        this.addToBot(new ApplyPowerAction(this.owner, this.owner, new SlowPower(this.owner, 1), 1));
    }
```

</details>

### atDamageReceive(float damage, DamageInfo.DamageType type)

<details><summary>Full body</summary>

```java
@Override
    public float atDamageReceive(float damage, DamageInfo.DamageType type) {
        if (type == DamageInfo.DamageType.NORMAL) {
            return damage * (1.0f + (float)this.amount * 0.1f);
        }
        return damage;
    }
```

</details>

## SplitPower
File: `powers\SplitPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + FontHelper.colorString(this.owner.name, "y") + DESCRIPTIONS[1];
    }
```

</details>

## SporeCloudPower
File: `powers\SporeCloudPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

### onDeath()

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, null, new VulnerablePower(AbstractDungeon.player, this.amount, true), this.amount)`
- `VulnerablePower` — `new VulnerablePower(AbstractDungeon.player, this.amount, true)`

<details><summary>Full body</summary>

```java
@Override
    public void onDeath() {
        if (AbstractDungeon.getCurrRoom().isBattleEnding()) {
            return;
        }
        CardCrawlGame.sound.play("SPORE_CLOUD_RELEASE");
        this.flashWithoutSound();
        this.addToTop(new ApplyPowerAction(AbstractDungeon.player, null, new VulnerablePower(AbstractDungeon.player, this.amount, true), this.amount));
    }
```

</details>

## StasisPower
File: `powers\StasisPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = StasisPower.powerStrings.DESCRIPTIONS[0] + FontHelper.colorString(this.card.name, "y") + StasisPower.powerStrings.DESCRIPTIONS[1];
    }
```

</details>

### onDeath()

**Creates:**
- `MakeTempCardInHandAction` — `new MakeTempCardInHandAction(this.card, false, true)`
- `MakeTempCardInDiscardAction` — `new MakeTempCardInDiscardAction(this.card, true)`

<details><summary>Full body</summary>

```java
@Override
    public void onDeath() {
        if (AbstractDungeon.player.hand.size() != 10) {
            this.addToBot(new MakeTempCardInHandAction(this.card, false, true));
        } else {
            this.addToBot(new MakeTempCardInDiscardAction(this.card, true));
        }
    }
```

</details>

## StaticDischargePower
File: `powers\StaticDischargePower.java`

### onAttacked(DamageInfo info, int damageAmount)

**Creates:**
- `ChannelAction` — `new ChannelAction(new Lightning())`
- `Lightning` — `new Lightning()`

<details><summary>Full body</summary>

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (info.type != DamageInfo.DamageType.THORNS && info.type != DamageInfo.DamageType.HP_LOSS && info.owner != null && info.owner != this.owner && damageAmount > 0) {
            this.flash();
            for (int i = 0; i < this.amount; ++i) {
                this.addToTop(new ChannelAction(new Lightning()));
            }
        }
        return damageAmount;
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = StaticDischargePower.powerStrings.DESCRIPTIONS[0] + this.amount + StaticDischargePower.powerStrings.DESCRIPTIONS[1];
    }
```

</details>

## StormPower
File: `powers\StormPower.java`

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `ChannelAction` — `new ChannelAction(new Lightning())`
- `Lightning` — `new Lightning()`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.POWER && this.amount > 0) {
            this.flash();
            for (int i = 0; i < this.amount; ++i) {
                this.addToBot(new ChannelAction(new Lightning()));
            }
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

## StrengthPower
File: `powers\StrengthPower.java`

### reducePower(int reduceAmount)

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, NAME)`

<details><summary>Full body</summary>

```java
@Override
    public void reducePower(int reduceAmount) {
        this.fontScale = 8.0f;
        this.amount -= reduceAmount;
        if (this.amount == 0) {
            this.addToTop(new RemoveSpecificPowerAction(this.owner, this.owner, NAME));
        }
        if (this.amount >= 999) {
            this.amount = 999;
        }
        if (this.amount <= -999) {
            this.amount = -999;
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        if (this.amount > 0) {
            this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[2];
            this.type = AbstractPower.PowerType.BUFF;
        } else {
            int tmp = -this.amount;
            this.description = DESCRIPTIONS[1] + tmp + DESCRIPTIONS[2];
            this.type = AbstractPower.PowerType.DEBUFF;
        }
    }
```

</details>

### atDamageGive(float damage, DamageInfo.DamageType type)

<details><summary>Full body</summary>

```java
@Override
    public float atDamageGive(float damage, DamageInfo.DamageType type) {
        if (type == DamageInfo.DamageType.NORMAL) {
            return damage + (float)this.amount;
        }
        return damage;
    }
```

</details>

## StrikeUpPower
File: `powers\StrikeUpPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = StrikeUpPower.powerStrings.DESCRIPTIONS[0] + this.amount + StrikeUpPower.powerStrings.DESCRIPTIONS[1];
    }
```

</details>

### onDrawOrDiscard()

<details><summary>Full body</summary>

```java
@Override
    public void onDrawOrDiscard() {
        for (AbstractCard c : AbstractDungeon.player.hand.group) {
            if (!c.hasTag(AbstractCard.CardTags.STRIKE)) continue;
            c.baseDamage = CardLibrary.getCard((String)c.cardID).baseDamage + this.amount;
        }
    }
```

</details>

## StudyPower
File: `powers\watcher\StudyPower.java`

### atEndOfTurn(boolean playerTurn)

**Creates:**
- `MakeTempCardInDrawPileAction` — `new MakeTempCardInDrawPileAction(new Insight(), this.amount, true, true)`
- `Insight` — `new Insight()`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean playerTurn) {
        this.addToBot(new MakeTempCardInDrawPileAction(new Insight(), this.amount, true, true));
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount > 1 ? StudyPower.powerStrings.DESCRIPTIONS[0] + this.amount + StudyPower.powerStrings.DESCRIPTIONS[1] : StudyPower.powerStrings.DESCRIPTIONS[0] + this.amount + StudyPower.powerStrings.DESCRIPTIONS[2];
    }
```

</details>

## SurroundedPower
File: `powers\SurroundedPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = SurroundedPower.powerStrings.DESCRIPTIONS[0];
    }
```

</details>

## TheBombPower
File: `powers\TheBombPower.java`

### atEndOfTurn(boolean isPlayer)

**Creates:**
- `ReducePowerAction` — `new ReducePowerAction(this.owner, this.owner, this, 1)`
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(this.damage, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.FIRE)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        if (!AbstractDungeon.getMonsters().areMonstersBasicallyDead()) {
            this.addToBot(new ReducePowerAction(this.owner, this.owner, this, 1));
            if (this.amount == 1) {
                this.addToBot(new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(this.damage, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.FIRE));
            }
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? String.format(DESCRIPTIONS[1], this.damage) : String.format(DESCRIPTIONS[0], this.amount, this.damage);
    }
```

</details>

## ThieveryPower
File: `powers\ThieveryPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.owner.name + DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

## ThornsPower
File: `powers\ThornsPower.java`

### onAttacked(DamageInfo info, int damageAmount)

**Creates:**
- `DamageAction` — `new DamageAction(info.owner, new DamageInfo(this.owner, this.amount, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL, true)`
- `DamageInfo` — `new DamageInfo(this.owner, this.amount, DamageInfo.DamageType.THORNS)`

<details><summary>Full body</summary>

```java
@Override
    public int onAttacked(DamageInfo info, int damageAmount) {
        if (info.type != DamageInfo.DamageType.THORNS && info.type != DamageInfo.DamageType.HP_LOSS && info.owner != null && info.owner != this.owner) {
            this.flash();
            this.addToTop(new DamageAction(info.owner, new DamageInfo(this.owner, this.amount, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL, true));
        }
        return damageAmount;
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

## ThousandCutsPower
File: `powers\ThousandCutsPower.java`

### onAfterCardPlayed(AbstractCard card)

**Creates:**
- `SFXAction` — `new SFXAction("ATTACK_HEAVY")`
- `VFXAction` — `new VFXAction(new CleaveEffect())`
- `CleaveEffect` — `new CleaveEffect()`
- `VFXAction` — `new VFXAction(this.owner, new CleaveEffect(), 0.2f)`
- `CleaveEffect` — `new CleaveEffect()`
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction(this.owner, DamageInfo.createDamageMatrix(this.amount, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.NONE, true)`

<details><summary>Full body</summary>

```java
@Override
    public void onAfterCardPlayed(AbstractCard card) {
        this.flash();
        this.addToBot(new SFXAction("ATTACK_HEAVY"));
        if (Settings.FAST_MODE) {
            this.addToBot(new VFXAction(new CleaveEffect()));
        } else {
            this.addToBot(new VFXAction(this.owner, new CleaveEffect(), 0.2f));
        }
        this.addToBot(new DamageAllEnemiesAction(this.owner, DamageInfo.createDamageMatrix(this.amount, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.NONE, true));
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

## TimeMazePower
File: `powers\TimeMazePower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESC[0] + this.maxAmount + DESC[1];
    }
```

</details>

### onAfterUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `ExhaustCardEffect` — `new ExhaustCardEffect(c)`

<details><summary>Full body</summary>

```java
@Override
    public void onAfterUseCard(AbstractCard card, UseCardAction action) {
        this.flashWithoutSound();
        --this.amount;
        if (this.amount == 0) {
            this.amount = this.maxAmount;
            AbstractDungeon.actionManager.cardQueue.clear();
            for (AbstractCard c : AbstractDungeon.player.limbo.group) {
                AbstractDungeon.effectList.add(new ExhaustCardEffect(c));
            }
            AbstractDungeon.player.limbo.group.clear();
            AbstractDungeon.player.releaseCard();
            AbstractDungeon.overlayMenu.endTurnButton.disable(true);
        }
        this.updateDescription();
    }
```

</details>

### atStartOfTurn()

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        this.amount = 15;
    }
```

</details>

## TimeWarpPower
File: `powers\TimeWarpPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESC[0] + 12 + DESC[1] + 2 + DESC[2];
    }
```

</details>

### onAfterUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `BorderFlashEffect` — `new BorderFlashEffect(Color.GOLD, true)`
- `TimeWarpTurnEndEffect` — `new TimeWarpTurnEndEffect()`
- `ApplyPowerAction` — `new ApplyPowerAction(m, m, new StrengthPower(m, 2), 2)`
- `StrengthPower` — `new StrengthPower(m, 2)`

<details><summary>Full body</summary>

```java
@Override
    public void onAfterUseCard(AbstractCard card, UseCardAction action) {
        this.flashWithoutSound();
        ++this.amount;
        if (this.amount == 12) {
            this.amount = 0;
            this.playApplyPowerSfx();
            AbstractDungeon.actionManager.callEndTurnEarlySequence();
            CardCrawlGame.sound.play("POWER_TIME_WARP", 0.05f);
            AbstractDungeon.effectsQueue.add(new BorderFlashEffect(Color.GOLD, true));
            AbstractDungeon.topLevelEffectsQueue.add(new TimeWarpTurnEndEffect());
            for (AbstractMonster m : AbstractDungeon.getMonsters().monsters) {
                this.addToBot(new ApplyPowerAction(m, m, new StrengthPower(m, 2), 2));
            }
        }
        this.updateDescription();
    }
```

</details>

## ToolsOfTheTradePower
File: `powers\ToolsOfTheTradePower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2] : DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[3] + this.amount + DESCRIPTIONS[4];
    }
```

</details>

### atStartOfTurnPostDraw()

**Creates:**
- `DrawCardAction` — `new DrawCardAction(this.owner, this.amount)`
- `DiscardAction` — `new DiscardAction(this.owner, this.owner, this.amount, false)`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurnPostDraw() {
        this.flash();
        this.addToBot(new DrawCardAction(this.owner, this.amount));
        this.addToBot(new DiscardAction(this.owner, this.owner, this.amount, false));
    }
```

</details>

## UnawakenedPower
File: `powers\UnawakenedPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0];
    }
```

</details>

## VaultPower
File: `powers\watcher\VaultPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = VaultPower.powerStrings.DESCRIPTIONS[0] + this.amount + VaultPower.powerStrings.DESCRIPTIONS[1];
    }
```

</details>

### atEndOfRound()

**Creates:**
- `DamageAction` — `new DamageAction(this.owner, new DamageInfo(this.source, this.amount, DamageInfo.DamageType.NORMAL), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(this.source, this.amount, DamageInfo.DamageType.NORMAL)`
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfRound() {
        this.flash();
        this.addToBot(new DamageAction(this.owner, new DamageInfo(this.source, this.amount, DamageInfo.DamageType.NORMAL), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
        this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
    }
```

</details>

## VigorPower
File: `powers\watcher\VigorPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = VigorPower.powerStrings.DESCRIPTIONS[0] + this.amount + VigorPower.powerStrings.DESCRIPTIONS[1];
    }
```

</details>

### atDamageGive(float damage, DamageInfo.DamageType type)

<details><summary>Full body</summary>

```java
@Override
    public float atDamageGive(float damage, DamageInfo.DamageType type) {
        if (type == DamageInfo.DamageType.NORMAL) {
            return damage += (float)this.amount;
        }
        return damage;
    }
```

</details>

### onUseCard(AbstractCard card, UseCardAction action)

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.ATTACK) {
            this.flash();
            this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        }
    }
```

</details>

## VulnerablePower
File: `powers\VulnerablePower.java`

### atEndOfRound()

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`
- `ReducePowerAction` — `new ReducePowerAction(this.owner, this.owner, POWER_ID, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfRound() {
        if (this.justApplied) {
            this.justApplied = false;
            return;
        }
        if (this.amount == 0) {
            this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        } else {
            this.addToBot(new ReducePowerAction(this.owner, this.owner, POWER_ID, 1));
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? (this.owner != null && this.owner.isPlayer && AbstractDungeon.player.hasRelic("Odd Mushroom") ? DESCRIPTIONS[0] + 25 + DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2] : (this.owner != null && !this.owner.isPlayer && AbstractDungeon.player.hasRelic("Paper Frog") ? DESCRIPTIONS[0] + 75 + DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2] : DESCRIPTIONS[0] + 50 + DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2])) : (this.owner != null && this.owner.isPlayer && AbstractDungeon.player.hasRelic("Odd Mushroom") ? DESCRIPTIONS[0] + 25 + DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[3] : (this.owner != null && !this.owner.isPlayer && AbstractDungeon.player.hasRelic("Paper Frog") ? DESCRIPTIONS[0] + 75 + DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[3] : DESCRIPTIONS[0] + 50 + DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[3]));
    }
```

</details>

### atDamageReceive(float damage, DamageInfo.DamageType type)

<details><summary>Full body</summary>

```java
@Override
    public float atDamageReceive(float damage, DamageInfo.DamageType type) {
        if (type == DamageInfo.DamageType.NORMAL) {
            if (this.owner.isPlayer && AbstractDungeon.player.hasRelic("Odd Mushroom")) {
                return damage * 1.25f;
            }
            if (this.owner != null && !this.owner.isPlayer && AbstractDungeon.player.hasRelic("Paper Frog")) {
                return damage * 1.75f;
            }
            return damage * 1.5f;
        }
        return damage;
    }
```

</details>

## WaveOfTheHandPower
File: `powers\watcher\WaveOfTheHandPower.java`

### onGainedBlock(float blockAmount)

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(mo, p, new WeakPower(mo, this.amount, false), this.amount, true, AbstractGameAction.AttackEffect.NONE)`
- `WeakPower` — `new WeakPower(mo, this.amount, false)`

<details><summary>Full body</summary>

```java
@Override
    public void onGainedBlock(float blockAmount) {
        if (blockAmount > 0.0f) {
            this.flash();
            AbstractPlayer p = AbstractDungeon.player;
            for (AbstractMonster mo : AbstractDungeon.getCurrRoom().monsters.monsters) {
                this.addToBot(new ApplyPowerAction(mo, p, new WeakPower(mo, this.amount, false), this.amount, true, AbstractGameAction.AttackEffect.NONE));
            }
        }
    }
```

</details>

### atEndOfRound()

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfRound() {
        this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = WaveOfTheHandPower.powerStrings.DESCRIPTIONS[0] + this.amount + WaveOfTheHandPower.powerStrings.DESCRIPTIONS[1];
    }
```

</details>

## WeakPower
File: `powers\WeakPower.java`

### atEndOfRound()

**Creates:**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID)`
- `ReducePowerAction` — `new ReducePowerAction(this.owner, this.owner, POWER_ID, 1)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfRound() {
        if (this.justApplied) {
            this.justApplied = false;
            return;
        }
        if (this.amount == 0) {
            this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
        } else {
            this.addToBot(new ReducePowerAction(this.owner, this.owner, POWER_ID, 1));
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = this.amount == 1 ? (this.owner != null && !this.owner.isPlayer && AbstractDungeon.player.hasRelic("Paper Crane") ? DESCRIPTIONS[0] + 40 + DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2] : DESCRIPTIONS[0] + 25 + DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[2]) : (this.owner != null && !this.owner.isPlayer && AbstractDungeon.player.hasRelic("Paper Crane") ? DESCRIPTIONS[0] + 40 + DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[3] : DESCRIPTIONS[0] + 25 + DESCRIPTIONS[1] + this.amount + DESCRIPTIONS[3]);
    }
```

</details>

### atDamageGive(float damage, DamageInfo.DamageType type)

<details><summary>Full body</summary>

```java
@Override
    public float atDamageGive(float damage, DamageInfo.DamageType type) {
        if (type == DamageInfo.DamageType.NORMAL) {
            if (!this.owner.isPlayer && AbstractDungeon.player.hasRelic("Paper Crane")) {
                return damage * 0.6f;
            }
            return damage * 0.75f;
        }
        return damage;
    }
```

</details>

## WinterPower
File: `powers\WinterPower.java`

### atStartOfTurn()

**Creates:**
- `ChannelAction` — `new ChannelAction(new Frost(), false)`
- `Frost` — `new Frost()`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        if (!AbstractDungeon.getMonsters().areMonstersBasicallyDead()) {
            for (AbstractOrb o : AbstractDungeon.player.orbs) {
                if (!(o instanceof EmptyOrbSlot)) continue;
                this.flash();
                break;
            }
            for (int i = 0; i < this.amount; ++i) {
                this.addToBot(new ChannelAction(new Frost(), false));
            }
        }
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + this.amount + DESCRIPTIONS[1];
    }
```

</details>

## WraithFormPower
File: `powers\WraithFormPower.java`

### atEndOfTurn(boolean isPlayer)

**Creates:**
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new DexterityPower(AbstractDungeon.player, this.amount), this.amount)`
- `DexterityPower` — `new DexterityPower(AbstractDungeon.player, this.amount)`

<details><summary>Full body</summary>

```java
@Override
    public void atEndOfTurn(boolean isPlayer) {
        this.addToBot(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new DexterityPower(AbstractDungeon.player, this.amount), this.amount));
    }
```

</details>

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = DESCRIPTIONS[0] + -this.amount + DESCRIPTIONS[1];
    }
```

</details>

## WrathNextTurnPower
File: `powers\watcher\WrathNextTurnPower.java`

### updateDescription()

<details><summary>Full body</summary>

```java
@Override
    public void updateDescription() {
        this.description = WrathNextTurnPower.powerStrings.DESCRIPTIONS[0];
    }
```

</details>

### atStartOfTurn()

**Creates:**
- `ChangeStanceAction` — `new ChangeStanceAction("Wrath")`
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction(this.owner, this.owner, this)`

<details><summary>Full body</summary>

```java
@Override
    public void atStartOfTurn() {
        this.addToBot(new ChangeStanceAction("Wrath"));
        this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, this));
    }
```

</details>

