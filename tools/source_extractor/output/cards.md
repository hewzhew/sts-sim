# StS Card Reference

Total Card subclasses: 439

## AThousandCuts
File: `cards\green\AThousandCuts.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new ThousandCutsPower(p, this.magicNumber), this.magicNumber)`
- `ThousandCutsPower` — `new ThousandCutsPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new ThousandCutsPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new ThousandCutsPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## AbstractCard
File: `cards\AbstractCard.java`

<details><summary>Full use() body</summary>

```java
public abstract void use(AbstractPlayer var1, AbstractMonster var2);
```

</details>

## Accuracy
File: `cards\green\Accuracy.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new AccuracyPower(p, this.magicNumber), this.magicNumber)`
- `AccuracyPower` — `new AccuracyPower(p, this.magicNumber)`

**Queue order:**
- L28: `this.addToBot(new ApplyPowerAction(p, p, new AccuracyPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new AccuracyPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Acrobatics
File: `cards\green\Acrobatics.java`

**Action sequence (in order):**
- `DrawCardAction` — `new DrawCardAction(p, this.magicNumber)`
- `DiscardAction` — `new DiscardAction(p, p, 1, false)`

**Queue order:**
- L26: `this.addToBot(new DrawCardAction(p, this.magicNumber))`
- L27: `this.addToBot(new DiscardAction(p, p, 1, false))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DrawCardAction(p, this.magicNumber));
        this.addToBot(new DiscardAction(p, p, 1, false));
    }
```

</details>

## Adrenaline
File: `cards\green\Adrenaline.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new AdrenalineEffect(), 0.15f)`
- `AdrenalineEffect` — `new AdrenalineEffect()`
- `GainEnergyAction` — `new GainEnergyAction(2)`
- `GainEnergyAction` — `new GainEnergyAction(1)`
- `DrawCardAction` — `new DrawCardAction(p, 2)`

**Queue order:**
- L28: `this.addToBot(new VFXAction(new AdrenalineEffect(), 0.15f))`
- L30: `this.addToBot(new GainEnergyAction(2))`
- L32: `this.addToBot(new GainEnergyAction(1))`
- L34: `this.addToBot(new DrawCardAction(p, 2))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new VFXAction(new AdrenalineEffect(), 0.15f));
        if (this.upgraded) {
            this.addToBot(new GainEnergyAction(2));
        } else {
            this.addToBot(new GainEnergyAction(1));
        }
        this.addToBot(new DrawCardAction(p, 2));
    }
```

</details>

## AfterImage
File: `cards\green\AfterImage.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new AfterImagePower(p, 1), 1)`
- `AfterImagePower` — `new AfterImagePower(p, 1)`

**Queue order:**
- L25: `this.addToBot(new ApplyPowerAction(p, p, new AfterImagePower(p, 1), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new AfterImagePower(p, 1), 1));
    }
```

</details>

## Aggregate
File: `cards\blue\Aggregate.java`

**Action sequence (in order):**
- `AggregateEnergyAction` — `new AggregateEnergyAction(this.magicNumber)`

**Queue order:**
- L25: `this.addToBot(new AggregateEnergyAction(this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new AggregateEnergyAction(this.magicNumber));
    }
```

</details>

## Alchemize
File: `cards\green\Alchemize.java`

**Action sequence (in order):**
- `ObtainPotionAction` — `new ObtainPotionAction(AbstractDungeon.returnRandomPotion(true))`

**Queue order:**
- L27: `this.addToBot(new ObtainPotionAction(AbstractDungeon.returnRandomPotion(true)))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ObtainPotionAction(AbstractDungeon.returnRandomPotion(true)));
    }
```

</details>

## AllForOne
File: `cards\blue\AllForOne.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `AllCostToHandAction` — `new AllCostToHandAction(0)`

**Queue order:**
- L29: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L30: `this.addToBot(new AllCostToHandAction(0))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
        this.addToBot(new AllCostToHandAction(0));
    }
```

</details>

## AllOutAttack
File: `cards\green\AllOutAttack.java`

**Action sequence (in order):**
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.SLASH_HEAVY)`
- `DiscardAction` — `new DiscardAction(p, p, 1, true)`

**Queue order:**
- L29: `this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.SLASH_HEAVY))`
- L30: `this.addToBot(new DiscardAction(p, p, 1, true))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.SLASH_HEAVY));
        this.addToBot(new DiscardAction(p, p, 1, true));
    }
```

</details>

## Alpha
File: `cards\purple\Alpha.java`

**Action sequence (in order):**
- `MakeTempCardInDrawPileAction` — `new MakeTempCardInDrawPileAction(this.cardsToPreview.makeStatEquivalentCopy(), 1, true, true)`

**Queue order:**
- L27: `this.addToBot(new MakeTempCardInDrawPileAction(this.cardsToPreview.makeStatEquivalentCopy(), 1, true, true))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new MakeTempCardInDrawPileAction(this.cardsToPreview.makeStatEquivalentCopy(), 1, true, true));
    }
```

</details>

## Amplify
File: `cards\blue\Amplify.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new AmplifyPower(p, this.magicNumber), this.magicNumber)`
- `AmplifyPower` — `new AmplifyPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new AmplifyPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new AmplifyPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Anger
File: `cards\red\Anger.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `VFXAction` — `new VFXAction(p, new VerticalAuraEffect(Color.FIREBRICK, p.hb.cX, p.hb.cY), 0.0f)`
- `VerticalAuraEffect` — `new VerticalAuraEffect(Color.FIREBRICK, p.hb.cX, p.hb.cY)`
- `MakeTempCardInDiscardAction` — `new MakeTempCardInDiscardAction(this.makeStatEquivalentCopy(), 1)`

**Queue order:**
- L32: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L33: `this.addToBot(new VFXAction(p, new VerticalAuraEffect(Color.FIREBRICK, p.hb.cX, p.hb.cY), 0.0f))`
- L34: `this.addToBot(new MakeTempCardInDiscardAction(this.makeStatEquivalentCopy(), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
        this.addToBot(new VFXAction(p, new VerticalAuraEffect(Color.FIREBRICK, p.hb.cX, p.hb.cY), 0.0f));
        this.addToBot(new MakeTempCardInDiscardAction(this.makeStatEquivalentCopy(), 1));
    }
```

</details>

## Apotheosis
File: `cards\colorless\Apotheosis.java`

**Action sequence (in order):**
- `ApotheosisAction` — `new ApotheosisAction()`

**Queue order:**
- L25: `this.addToBot(new ApotheosisAction())`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApotheosisAction());
    }
```

</details>

## Apparition
File: `cards\colorless\Apparition.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new IntangiblePlayerPower(p, 1), 1)`
- `IntangiblePlayerPower` — `new IntangiblePlayerPower(p, 1)`

**Queue order:**
- L27: `this.addToBot(new ApplyPowerAction(p, p, new IntangiblePlayerPower(p, 1), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new IntangiblePlayerPower(p, 1), 1));
    }
```

</details>

## Armaments
File: `cards\red\Armaments.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `ArmamentsAction` — `new ArmamentsAction(this.upgraded)`

**Queue order:**
- L27: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L28: `this.addToBot(new ArmamentsAction(this.upgraded))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.addToBot(new ArmamentsAction(this.upgraded));
    }
```

</details>

## AscendersBane
File: `cards\curses\AscendersBane.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
    }
```

</details>

## AutoShields
File: `cards\blue\AutoShields.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`

**Queue order:**
- L28: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (AbstractDungeon.player.currentBlock == 0) {
            this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        }
    }
```

</details>

## Backflip
File: `cards\green\Backflip.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `DrawCardAction` — `new DrawCardAction(p, 2)`

**Queue order:**
- L27: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L28: `this.addToBot(new DrawCardAction(p, 2))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.addToBot(new DrawCardAction(p, 2));
    }
```

</details>

## Backstab
File: `cards\green\Backstab.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L30: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY));
    }
```

</details>

## BallLightning
File: `cards\blue\BallLightning.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ChannelAction` — `new ChannelAction(new Lightning())`
- `Lightning` — `new Lightning()`

**Queue order:**
- L33: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY))`
- L35: `this.addToBot(new ChannelAction(new Lightning()))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY));
        for (int i = 0; i < this.magicNumber; ++i) {
            this.addToBot(new ChannelAction(new Lightning()));
        }
    }
```

</details>

## BandageUp
File: `cards\colorless\BandageUp.java`

**Action sequence (in order):**
- `HealAction` — `new HealAction(p, p, this.magicNumber)`

**Queue order:**
- L27: `this.addToBot(new HealAction(p, p, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new HealAction(p, p, this.magicNumber));
    }
```

</details>

## Bane
File: `cards\green\Bane.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `BaneAction` — `new BaneAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn))`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L29: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL))`
- L30: `this.addToBot(new BaneAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn)))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL));
        this.addToBot(new BaneAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn)));
    }
```

</details>

## Barrage
File: `cards\blue\Barrage.java`

**Action sequence (in order):**
- `BarrageAction` — `new BarrageAction(m, new DamageInfo(p, this.damage, DamageInfo.DamageType.NORMAL))`
- `DamageInfo` — `new DamageInfo(p, this.damage, DamageInfo.DamageType.NORMAL)`

**Queue order:**
- L26: `this.addToBot(new BarrageAction(m, new DamageInfo(p, this.damage, DamageInfo.DamageType.NORMAL)))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new BarrageAction(m, new DamageInfo(p, this.damage, DamageInfo.DamageType.NORMAL)));
    }
```

</details>

## Barricade
File: `cards\red\Barricade.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new BarricadePower(p))`
- `BarricadePower` — `new BarricadePower(p)`

**Queue order:**
- L33: `this.addToBot(new ApplyPowerAction(p, p, new BarricadePower(p)))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        boolean powerExists = false;
        for (AbstractPower pow : p.powers) {
            if (!pow.ID.equals(ID)) continue;
            powerExists = true;
            break;
        }
        if (!powerExists) {
            this.addToBot(new ApplyPowerAction(p, p, new BarricadePower(p)));
        }
    }
```

</details>

## Bash
File: `cards\red\Bash.java`

**Action sequence (in order):**
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ApplyPowerAction` — `new ApplyPowerAction(m, p, new VulnerablePower(m, this.magicNumber, false), this.magicNumber)`
- `VulnerablePower` — `new VulnerablePower(m, this.magicNumber, false)`

**Queue order:**
- L39: `this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L41: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L43: `this.addToBot(new ApplyPowerAction(m, p, new VulnerablePower(m, this.magicNumber, false), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (Settings.isDebug) {
            this.multiDamage = new int[AbstractDungeon.getCurrRoom().monsters.monsters.size()];
            for (int i = 0; i < AbstractDungeon.getCurrRoom().monsters.monsters.size(); ++i) {
                this.multiDamage[i] = 100;
            }
            this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.BLUNT_HEAVY));
        } else {
            this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
        }
        this.addToBot(new ApplyPowerAction(m, p, new VulnerablePower(m, this.magicNumber, false), this.magicNumber));
    }
```

</details>

## BattleHymn
File: `cards\purple\BattleHymn.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new BattleHymnPower(p, this.magicNumber), this.magicNumber)`
- `BattleHymnPower` — `new BattleHymnPower(p, this.magicNumber)`

**Queue order:**
- L28: `this.addToBot(new ApplyPowerAction(p, p, new BattleHymnPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new BattleHymnPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## BattleTrance
File: `cards\red\BattleTrance.java`

**Action sequence (in order):**
- `DrawCardAction` — `new DrawCardAction(p, this.magicNumber)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new NoDrawPower(p))`
- `NoDrawPower` — `new NoDrawPower(p)`

**Queue order:**
- L27: `this.addToBot(new DrawCardAction(p, this.magicNumber))`
- L28: `this.addToBot(new ApplyPowerAction(p, p, new NoDrawPower(p)))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DrawCardAction(p, this.magicNumber));
        this.addToBot(new ApplyPowerAction(p, p, new NoDrawPower(p)));
    }
```

</details>

## BeamCell
File: `cards\blue\BeamCell.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ApplyPowerAction` — `new ApplyPowerAction(m, p, new VulnerablePower(m, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE)`
- `VulnerablePower` — `new VulnerablePower(m, this.magicNumber, false)`

**Queue order:**
- L31: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L32: `this.addToBot(new ApplyPowerAction(m, p, new VulnerablePower(m, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
        this.addToBot(new ApplyPowerAction(m, p, new VulnerablePower(m, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE));
    }
```

</details>

## BecomeAlmighty
File: `cards\optionCards\BecomeAlmighty.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.onChoseThisOption();
    }
```

</details>

## Berserk
File: `cards\red\Berserk.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new VulnerablePower(p, this.magicNumber, false), this.magicNumber)`
- `VulnerablePower` — `new VulnerablePower(p, this.magicNumber, false)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new BerserkPower(p, 1), 1)`
- `BerserkPower` — `new BerserkPower(p, 1)`

**Queue order:**
- L27: `this.addToBot(new ApplyPowerAction(p, p, new VulnerablePower(p, this.magicNumber, false), this.magicNumber))`
- L28: `this.addToBot(new ApplyPowerAction(p, p, new BerserkPower(p, 1), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new VulnerablePower(p, this.magicNumber, false), this.magicNumber));
        this.addToBot(new ApplyPowerAction(p, p, new BerserkPower(p, 1), 1));
    }
```

</details>

## Beta
File: `cards\tempCards\Beta.java`

**Action sequence (in order):**
- `MakeTempCardInDrawPileAction` — `new MakeTempCardInDrawPileAction(this.cardsToPreview.makeStatEquivalentCopy(), 1, true, true)`

**Queue order:**
- L27: `this.addToBot(new MakeTempCardInDrawPileAction(this.cardsToPreview.makeStatEquivalentCopy(), 1, true, true))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new MakeTempCardInDrawPileAction(this.cardsToPreview.makeStatEquivalentCopy(), 1, true, true));
    }
```

</details>

## BiasedCognition
File: `cards\blue\BiasedCognition.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new FocusPower(p, this.magicNumber), this.magicNumber)`
- `FocusPower` — `new FocusPower(p, this.magicNumber)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new BiasPower(p, 1), 1)`
- `BiasPower` — `new BiasPower(p, 1)`

**Queue order:**
- L27: `this.addToBot(new ApplyPowerAction(p, p, new FocusPower(p, this.magicNumber), this.magicNumber))`
- L28: `this.addToBot(new ApplyPowerAction(p, p, new BiasPower(p, 1), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new FocusPower(p, this.magicNumber), this.magicNumber));
        this.addToBot(new ApplyPowerAction(p, p, new BiasPower(p, 1), 1));
    }
```

</details>

## Bite
File: `cards\colorless\Bite.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new BiteEffect(m.hb.cX, m.hb.cY - 40.0f * Settings.scale, Settings.GOLD_COLOR.cpy()), 0.1f)`
- `BiteEffect` — `new BiteEffect(m.hb.cX, m.hb.cY - 40.0f * Settings.scale, Settings.GOLD_COLOR.cpy())`
- `VFXAction` — `new VFXAction(new BiteEffect(m.hb.cX, m.hb.cY - 40.0f * Settings.scale, Settings.GOLD_COLOR.cpy()), 0.3f)`
- `BiteEffect` — `new BiteEffect(m.hb.cX, m.hb.cY - 40.0f * Settings.scale, Settings.GOLD_COLOR.cpy())`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `HealAction` — `new HealAction(p, p, this.magicNumber)`

**Queue order:**
- L36: `this.addToBot(new VFXAction(new BiteEffect(m.hb.cX, m.hb.cY - 40.0f * Settings.scale, Settings.GOLD_COLOR.cpy()), 0.1f))`
- L38: `this.addToBot(new VFXAction(new BiteEffect(m.hb.cX, m.hb.cY - 40.0f * Settings.scale, Settings.GOLD_COLOR.cpy()), 0.3f))`
- L41: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE))`
- L42: `this.addToBot(new HealAction(p, p, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (m != null) {
            if (Settings.FAST_MODE) {
                this.addToBot(new VFXAction(new BiteEffect(m.hb.cX, m.hb.cY - 40.0f * Settings.scale, Settings.GOLD_COLOR.cpy()), 0.1f));
            } else {
                this.addToBot(new VFXAction(new BiteEffect(m.hb.cX, m.hb.cY - 40.0f * Settings.scale, Settings.GOLD_COLOR.cpy()), 0.3f));
            }
        }
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE));
        this.addToBot(new HealAction(p, p, this.magicNumber));
    }
```

</details>

## BladeDance
File: `cards\green\BladeDance.java`

**Action sequence (in order):**
- `MakeTempCardInHandAction` — `new MakeTempCardInHandAction((AbstractCard)new Shiv(), this.magicNumber)`
- `Shiv` — `new Shiv()`

**Queue order:**
- L27: `this.addToBot(new MakeTempCardInHandAction((AbstractCard)new Shiv(), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new MakeTempCardInHandAction((AbstractCard)new Shiv(), this.magicNumber));
    }
```

</details>

## Blasphemy
File: `cards\purple\Blasphemy.java`

**Action sequence (in order):**
- `ChangeStanceAction` — `new ChangeStanceAction("Divinity")`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new EndTurnDeathPower(p))`
- `EndTurnDeathPower` — `new EndTurnDeathPower(p)`

**Queue order:**
- L27: `this.addToBot(new ChangeStanceAction("Divinity"))`
- L28: `this.addToBot(new ApplyPowerAction(p, p, new EndTurnDeathPower(p)))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ChangeStanceAction("Divinity"));
        this.addToBot(new ApplyPowerAction(p, p, new EndTurnDeathPower(p)));
    }
```

</details>

## Blind
File: `cards\colorless\Blind.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber)`
- `WeakPower` — `new WeakPower(m, this.magicNumber, false)`
- `ApplyPowerAction` — `new ApplyPowerAction(mo, p, new WeakPower(mo, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE)`
- `WeakPower` — `new WeakPower(mo, this.magicNumber, false)`

**Queue order:**
- L29: `this.addToBot(new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber))`
- L32: `this.addToBot(new ApplyPowerAction(mo, p, new WeakPower(mo, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (!this.upgraded) {
            this.addToBot(new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber));
        } else {
            for (AbstractMonster mo : AbstractDungeon.getCurrRoom().monsters.monsters) {
                this.addToBot(new ApplyPowerAction(mo, p, new WeakPower(mo, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE));
            }
        }
    }
```

</details>

## Blizzard
File: `cards\blue\Blizzard.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new BlizzardEffect(frostCount, AbstractDungeon.getMonsters().shouldFlipVfx()), 0.25f)`
- `BlizzardEffect` — `new BlizzardEffect(frostCount, AbstractDungeon.getMonsters().shouldFlipVfx())`
- `VFXAction` — `new VFXAction(new BlizzardEffect(frostCount, AbstractDungeon.getMonsters().shouldFlipVfx()), 1.0f)`
- `BlizzardEffect` — `new BlizzardEffect(frostCount, AbstractDungeon.getMonsters().shouldFlipVfx())`
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction(p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.BLUNT_HEAVY, false)`

**Queue order:**
- L42: `this.addToBot(new VFXAction(new BlizzardEffect(frostCount, AbstractDungeon.getMonsters().shouldFlipVfx()), 0.25f))`
- L44: `this.addToBot(new VFXAction(new BlizzardEffect(frostCount, AbstractDungeon.getMonsters().shouldFlipVfx()), 1.0f))`
- L46: `this.addToBot(new DamageAllEnemiesAction(p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.BLUNT_HEAVY, false))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        int frostCount = 0;
        for (AbstractOrb o : AbstractDungeon.actionManager.orbsChanneledThisCombat) {
            if (!(o instanceof Frost)) continue;
            ++frostCount;
        }
        this.baseDamage = frostCount * this.magicNumber;
        this.calculateCardDamage(null);
        if (Settings.FAST_MODE) {
            this.addToBot(new VFXAction(new BlizzardEffect(frostCount, AbstractDungeon.getMonsters().shouldFlipVfx()), 0.25f));
        } else {
            this.addToBot(new VFXAction(new BlizzardEffect(frostCount, AbstractDungeon.getMonsters().shouldFlipVfx()), 1.0f));
        }
        this.addToBot(new DamageAllEnemiesAction(p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.BLUNT_HEAVY, false));
    }
```

</details>

## BloodForBlood
File: `cards\red\BloodForBlood.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L34: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY));
    }
```

</details>

## Bloodletting
File: `cards\red\Bloodletting.java`

**Action sequence (in order):**
- `LoseHPAction` — `new LoseHPAction(p, p, 3)`
- `GainEnergyAction` — `new GainEnergyAction(this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new LoseHPAction(p, p, 3))`
- L27: `this.addToBot(new GainEnergyAction(this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new LoseHPAction(p, p, 3));
        this.addToBot(new GainEnergyAction(this.magicNumber));
    }
```

</details>

## Bludgeon
File: `cards\red\Bludgeon.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new WeightyImpactEffect(m.hb.cX, m.hb.cY))`
- `WeightyImpactEffect` — `new WeightyImpactEffect(m.hb.cX, m.hb.cY)`
- `WaitAction` — `new WaitAction(0.8f)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L32: `this.addToBot(new VFXAction(new WeightyImpactEffect(m.hb.cX, m.hb.cY)))`
- L34: `this.addToBot(new WaitAction(0.8f))`
- L35: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (m != null) {
            this.addToBot(new VFXAction(new WeightyImpactEffect(m.hb.cX, m.hb.cY)));
        }
        this.addToBot(new WaitAction(0.8f));
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE));
    }
```

</details>

## Blur
File: `cards\green\Blur.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new BlurPower(p, 1), 1)`
- `BlurPower` — `new BlurPower(p, 1)`

**Queue order:**
- L28: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L29: `this.addToBot(new ApplyPowerAction(p, p, new BlurPower(p, 1), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.addToBot(new ApplyPowerAction(p, p, new BlurPower(p, 1), 1));
    }
```

</details>

## BodySlam
File: `cards\red\BodySlam.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, DamageInfo.DamageType.NORMAL), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, DamageInfo.DamageType.NORMAL)`

**Queue order:**
- L31: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, DamageInfo.DamageType.NORMAL), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.baseDamage = p.currentBlock;
        this.calculateCardDamage(m);
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, DamageInfo.DamageType.NORMAL), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
        this.rawDescription = BodySlam.cardStrings.DESCRIPTION;
        this.initializeDescription();
    }
```

</details>

## BootSequence
File: `cards\blue\BootSequence.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`

**Queue order:**
- L28: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
    }
```

</details>

## BouncingFlask
File: `cards\green\BouncingFlask.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new PotionBounceEffect(p.hb.cX, p.hb.cY, randomMonster.hb.cX, this.hb.cY), 0.4f)`
- `PotionBounceEffect` — `new PotionBounceEffect(p.hb.cX, p.hb.cY, randomMonster.hb.cX, this.hb.cY)`
- `BouncingFlaskAction` — `new BouncingFlaskAction(randomMonster, 3, this.magicNumber)`

**Queue order:**
- L30: `this.addToBot(new VFXAction(new PotionBounceEffect(p.hb.cX, p.hb.cY, randomMonster.hb.cX, this.hb.cY), 0.4f))`
- L32: `this.addToBot(new BouncingFlaskAction(randomMonster, 3, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        AbstractMonster randomMonster = AbstractDungeon.getMonsters().getRandomMonster(null, true, AbstractDungeon.cardRandomRng);
        if (randomMonster != null) {
            this.addToBot(new VFXAction(new PotionBounceEffect(p.hb.cX, p.hb.cY, randomMonster.hb.cX, this.hb.cY), 0.4f));
        }
        this.addToBot(new BouncingFlaskAction(randomMonster, 3, this.magicNumber));
    }
```

</details>

## BowlingBash
File: `cards\purple\BowlingBash.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `SFXAction` — `new SFXAction("ATTACK_BOWLING")`

**Queue order:**
- L34: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L37: `this.addToBot(new SFXAction("ATTACK_BOWLING"))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        int count = 0;
        for (AbstractMonster m2 : AbstractDungeon.getCurrRoom().monsters.monsters) {
            if (m2.isDeadOrEscaped()) continue;
            ++count;
            this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
        }
        if (count >= 3) {
            this.addToBot(new SFXAction("ATTACK_BOWLING"));
        }
    }
```

</details>

## Brilliance
File: `cards\purple\Brilliance.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.FIRE, true)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L51: `this.addToBot(new DamageAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.FIRE, true))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.damage += this.magicNumber;
        this.calculateCardDamage(m);
        this.addToBot(new DamageAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.FIRE, true));
    }
```

</details>

## Brutality
File: `cards\red\Brutality.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new BrutalityPower(p, 1), 1)`
- `BrutalityPower` — `new BrutalityPower(p, 1)`

**Queue order:**
- L25: `this.addToBot(new ApplyPowerAction(p, p, new BrutalityPower(p, 1), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new BrutalityPower(p, 1), 1));
    }
```

</details>

## Buffer
File: `cards\blue\Buffer.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new BufferPower(p, this.magicNumber), this.magicNumber)`
- `BufferPower` — `new BufferPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new BufferPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new BufferPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## BulletTime
File: `cards\green\BulletTime.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new NoDrawPower(p), 1)`
- `NoDrawPower` — `new NoDrawPower(p)`
- `ApplyBulletTimeAction` — `new ApplyBulletTimeAction()`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new NoDrawPower(p), 1))`
- L27: `this.addToBot(new ApplyBulletTimeAction())`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new NoDrawPower(p), 1));
        this.addToBot(new ApplyBulletTimeAction());
    }
```

</details>

## Burn
File: `cards\status\Burn.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)AbstractDungeon.player, new DamageInfo(AbstractDungeon.player, this.magicNumber, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.FIRE)`
- `DamageInfo` — `new DamageInfo(AbstractDungeon.player, this.magicNumber, DamageInfo.DamageType.THORNS)`

**Queue order:**
- L32: `this.addToBot(new DamageAction((AbstractCreature)AbstractDungeon.player, new DamageInfo(AbstractDungeon.player, this.magicNumber, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.FIRE))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (this.dontTriggerOnUseCard) {
            this.addToBot(new DamageAction((AbstractCreature)AbstractDungeon.player, new DamageInfo(AbstractDungeon.player, this.magicNumber, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.FIRE));
        }
    }
```

</details>

## BurningPact
File: `cards\red\BurningPact.java`

**Action sequence (in order):**
- `ExhaustAction` — `new ExhaustAction(1, false)`
- `DrawCardAction` — `new DrawCardAction(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ExhaustAction(1, false))`
- L27: `this.addToBot(new DrawCardAction(p, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ExhaustAction(1, false));
        this.addToBot(new DrawCardAction(p, this.magicNumber));
    }
```

</details>

## Burst
File: `cards\green\Burst.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new BurstPower(p, this.magicNumber), this.magicNumber)`
- `BurstPower` — `new BurstPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new BurstPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new BurstPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## CalculatedGamble
File: `cards\green\CalculatedGamble.java`

**Action sequence (in order):**
- `CalculatedGambleAction` — `new CalculatedGambleAction(false)`

**Queue order:**
- L25: `this.addToBot(new CalculatedGambleAction(false))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new CalculatedGambleAction(false));
    }
```

</details>

## Caltrops
File: `cards\green\Caltrops.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new ThornsPower(p, this.magicNumber), this.magicNumber)`
- `ThornsPower` — `new ThornsPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new ThornsPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new ThornsPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Capacitor
File: `cards\blue\Capacitor.java`

**Action sequence (in order):**
- `IncreaseMaxOrbAction` — `new IncreaseMaxOrbAction(this.magicNumber)`

**Queue order:**
- L25: `this.addToBot(new IncreaseMaxOrbAction(this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new IncreaseMaxOrbAction(this.magicNumber));
    }
```

</details>

## Carnage
File: `cards\red\Carnage.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new ViolentAttackEffect(m.hb.cX, m.hb.cY, Color.RED))`
- `ViolentAttackEffect` — `new ViolentAttackEffect(m.hb.cX, m.hb.cY, Color.RED)`
- `VFXAction` — `new VFXAction(new StarBounceEffect(m.hb.cX, m.hb.cY))`
- `StarBounceEffect` — `new StarBounceEffect(m.hb.cX, m.hb.cY)`
- `VFXAction` — `new VFXAction(new ViolentAttackEffect(m.hb.cX, m.hb.cY, Color.RED), 0.4f)`
- `ViolentAttackEffect` — `new ViolentAttackEffect(m.hb.cX, m.hb.cY, Color.RED)`
- `VFXAction` — `new VFXAction(new StarBounceEffect(m.hb.cX, m.hb.cY))`
- `StarBounceEffect` — `new StarBounceEffect(m.hb.cX, m.hb.cY)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L35: `this.addToBot(new VFXAction(new ViolentAttackEffect(m.hb.cX, m.hb.cY, Color.RED)))`
- L37: `this.addToBot(new VFXAction(new StarBounceEffect(m.hb.cX, m.hb.cY)))`
- L40: `this.addToBot(new VFXAction(new ViolentAttackEffect(m.hb.cX, m.hb.cY, Color.RED), 0.4f))`
- L42: `this.addToBot(new VFXAction(new StarBounceEffect(m.hb.cX, m.hb.cY)))`
- L45: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (Settings.FAST_MODE) {
            this.addToBot(new VFXAction(new ViolentAttackEffect(m.hb.cX, m.hb.cY, Color.RED)));
            for (int i = 0; i < 5; ++i) {
                this.addToBot(new VFXAction(new StarBounceEffect(m.hb.cX, m.hb.cY)));
            }
        } else {
            this.addToBot(new VFXAction(new ViolentAttackEffect(m.hb.cX, m.hb.cY, Color.RED), 0.4f));
            for (int i = 0; i < 5; ++i) {
                this.addToBot(new VFXAction(new StarBounceEffect(m.hb.cX, m.hb.cY)));
            }
        }
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
    }
```

</details>

## CarveReality
File: `cards\purple\CarveReality.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `MakeTempCardInHandAction` — `new MakeTempCardInHandAction(this.cardsToPreview.makeStatEquivalentCopy(), 1)`

**Queue order:**
- L31: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY))`
- L32: `this.addToBot(new MakeTempCardInHandAction(this.cardsToPreview.makeStatEquivalentCopy(), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY));
        this.addToBot(new MakeTempCardInHandAction(this.cardsToPreview.makeStatEquivalentCopy(), 1));
    }
```

</details>

## Catalyst
File: `cards\green\Catalyst.java`

**Action sequence (in order):**
- `DoublePoisonAction` — `new DoublePoisonAction(m, p)`
- `TriplePoisonAction` — `new TriplePoisonAction(m, p)`

**Queue order:**
- L27: `this.addToBot(new DoublePoisonAction(m, p))`
- L29: `this.addToBot(new TriplePoisonAction(m, p))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (!this.upgraded) {
            this.addToBot(new DoublePoisonAction(m, p));
        } else {
            this.addToBot(new TriplePoisonAction(m, p));
        }
    }
```

</details>

## Chaos
File: `cards\blue\Chaos.java`

**Action sequence (in order):**
- `ChannelAction` — `new ChannelAction(AbstractOrb.getRandomOrb(true))`
- `ChannelAction` — `new ChannelAction(AbstractOrb.getRandomOrb(true))`

**Queue order:**
- L29: `this.addToBot(new ChannelAction(AbstractOrb.getRandomOrb(true)))`
- L31: `this.addToBot(new ChannelAction(AbstractOrb.getRandomOrb(true)))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (this.upgraded) {
            this.addToBot(new ChannelAction(AbstractOrb.getRandomOrb(true)));
        }
        this.addToBot(new ChannelAction(AbstractOrb.getRandomOrb(true)));
    }
```

</details>

## Chill
File: `cards\blue\Chill.java`

**Action sequence (in order):**
- `ChannelAction` — `new ChannelAction(new Frost())`
- `Frost` — `new Frost()`

**Queue order:**
- L36: `this.addToBot(new ChannelAction(new Frost()))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        int count = 0;
        for (AbstractMonster mon : AbstractDungeon.getMonsters().monsters) {
            if (mon.isDeadOrEscaped()) continue;
            ++count;
        }
        for (int i = 0; i < count * this.magicNumber; ++i) {
            this.addToBot(new ChannelAction(new Frost()));
        }
    }
```

</details>

## Choke
File: `cards\green\Choke.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ApplyPowerAction` — `new ApplyPowerAction(m, p, new ChokePower(m, this.magicNumber), this.magicNumber)`
- `ChokePower` — `new ChokePower(m, this.magicNumber)`

**Queue order:**
- L31: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY))`
- L32: `this.addToBot(new ApplyPowerAction(m, p, new ChokePower(m, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY));
        this.addToBot(new ApplyPowerAction(m, p, new ChokePower(m, this.magicNumber), this.magicNumber));
    }
```

</details>

## ChooseCalm
File: `cards\optionCards\ChooseCalm.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
    }
```

</details>

## ChooseWrath
File: `cards\optionCards\ChooseWrath.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
    }
```

</details>

## Chrysalis
File: `cards\colorless\Chrysalis.java`

**Action sequence (in order):**
- `MakeTempCardInDrawPileAction` — `new MakeTempCardInDrawPileAction(card, 1, true, true)`

**Queue order:**
- L34: `this.addToBot(new MakeTempCardInDrawPileAction(card, 1, true, true))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        for (int i = 0; i < this.magicNumber; ++i) {
            AbstractCard card = AbstractDungeon.returnTrulyRandomCardInCombat(AbstractCard.CardType.SKILL).makeCopy();
            if (card.cost > 0) {
                card.cost = 0;
                card.costForTurn = 0;
                card.isCostModified = true;
            }
            this.addToBot(new MakeTempCardInDrawPileAction(card, 1, true, true));
        }
    }
```

</details>

## Clash
File: `cards\red\Clash.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new ClashEffect(m.hb.cX, m.hb.cY), 0.1f)`
- `ClashEffect` — `new ClashEffect(m.hb.cX, m.hb.cY)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L31: `this.addToBot(new VFXAction(new ClashEffect(m.hb.cX, m.hb.cY), 0.1f))`
- L33: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (m != null) {
            this.addToBot(new VFXAction(new ClashEffect(m.hb.cX, m.hb.cY), 0.1f));
        }
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE));
    }
```

</details>

## Claw
File: `cards\blue\Claw.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new ClawEffect(m.hb.cX, m.hb.cY, Color.CYAN, Color.WHITE), 0.1f)`
- `ClawEffect` — `new ClawEffect(m.hb.cX, m.hb.cY, Color.CYAN, Color.WHITE)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, DamageInfo.DamageType.NORMAL), AbstractGameAction.AttackEffect.NONE)`
- `DamageInfo` — `new DamageInfo(p, this.damage, DamageInfo.DamageType.NORMAL)`
- `GashAction` — `new GashAction(this, this.magicNumber)`

**Queue order:**
- L34: `this.addToBot(new VFXAction(new ClawEffect(m.hb.cX, m.hb.cY, Color.CYAN, Color.WHITE), 0.1f))`
- L36: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, DamageInfo.DamageType.NORMAL), AbstractGameAction.AttackEffect.NONE))`
- L37: `this.addToBot(new GashAction(this, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (m != null) {
            this.addToBot(new VFXAction(new ClawEffect(m.hb.cX, m.hb.cY, Color.CYAN, Color.WHITE), 0.1f));
        }
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, DamageInfo.DamageType.NORMAL), AbstractGameAction.AttackEffect.NONE));
        this.addToBot(new GashAction(this, this.magicNumber));
    }
```

</details>

## Cleave
File: `cards\red\Cleave.java`

**Action sequence (in order):**
- `SFXAction` — `new SFXAction("ATTACK_HEAVY")`
- `VFXAction` — `new VFXAction(p, new CleaveEffect(), 0.1f)`
- `CleaveEffect` — `new CleaveEffect()`
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE)`

**Queue order:**
- L31: `this.addToBot(new SFXAction("ATTACK_HEAVY"))`
- L32: `this.addToBot(new VFXAction(p, new CleaveEffect(), 0.1f))`
- L33: `this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new SFXAction("ATTACK_HEAVY"));
        this.addToBot(new VFXAction(p, new CleaveEffect(), 0.1f));
        this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE));
    }
```

</details>

## CloakAndDagger
File: `cards\green\CloakAndDagger.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `MakeTempCardInHandAction` — `new MakeTempCardInHandAction((AbstractCard)new Shiv(), this.magicNumber)`
- `Shiv` — `new Shiv()`

**Queue order:**
- L30: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L31: `this.addToBot(new MakeTempCardInHandAction((AbstractCard)new Shiv(), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.addToBot(new MakeTempCardInHandAction((AbstractCard)new Shiv(), this.magicNumber));
    }
```

</details>

## Clothesline
File: `cards\red\Clothesline.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ApplyPowerAction` — `new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber)`
- `WeakPower` — `new WeakPower(m, this.magicNumber, false)`

**Queue order:**
- L31: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L32: `this.addToBot(new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
        this.addToBot(new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber));
    }
```

</details>

## Clumsy
File: `cards\curses\Clumsy.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
    }
```

</details>

## ColdSnap
File: `cards\blue\ColdSnap.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ChannelAction` — `new ChannelAction(new Frost())`
- `Frost` — `new Frost()`

**Queue order:**
- L33: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY))`
- L35: `this.addToBot(new ChannelAction(new Frost()))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY));
        for (int i = 0; i < this.magicNumber; ++i) {
            this.addToBot(new ChannelAction(new Frost()));
        }
    }
```

</details>

## Collect
File: `cards\purple\Collect.java`

**Action sequence (in order):**
- `CollectAction` — `new CollectAction(p, this.freeToPlayOnce, this.energyOnUse, this.upgraded)`

**Queue order:**
- L28: `this.addToBot(new CollectAction(p, this.freeToPlayOnce, this.energyOnUse, this.upgraded))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new CollectAction(p, this.freeToPlayOnce, this.energyOnUse, this.upgraded));
    }
```

</details>

## Combust
File: `cards\red\Combust.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new CombustPower(p, 1, this.magicNumber), this.magicNumber)`
- `CombustPower` — `new CombustPower(p, 1, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new CombustPower(p, 1, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new CombustPower(p, 1, this.magicNumber), this.magicNumber));
    }
```

</details>

## CompileDriver
File: `cards\blue\CompileDriver.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `CompileDriverAction` — `new CompileDriverAction(p, this.magicNumber)`

**Queue order:**
- L30: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT))`
- L31: `this.addToBot(new CompileDriverAction(p, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT));
        this.addToBot(new CompileDriverAction(p, this.magicNumber));
    }
```

</details>

## Concentrate
File: `cards\green\Concentrate.java`

**Action sequence (in order):**
- `DiscardAction` — `new DiscardAction(p, p, this.magicNumber, false)`
- `GainEnergyAction` — `new GainEnergyAction(2)`

**Queue order:**
- L26: `this.addToBot(new DiscardAction(p, p, this.magicNumber, false))`
- L27: `this.addToBot(new GainEnergyAction(2))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DiscardAction(p, p, this.magicNumber, false));
        this.addToBot(new GainEnergyAction(2));
    }
```

</details>

## Conclude
File: `cards\purple\Conclude.java`

**Action sequence (in order):**
- `SFXAction` — `new SFXAction("ATTACK_HEAVY")`
- `VFXAction` — `new VFXAction(p, new CleaveEffect(), 0.1f)`
- `CleaveEffect` — `new CleaveEffect()`
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE)`
- `PressEndTurnButtonAction` — `new PressEndTurnButtonAction()`

**Queue order:**
- L32: `this.addToBot(new SFXAction("ATTACK_HEAVY"))`
- L33: `this.addToBot(new VFXAction(p, new CleaveEffect(), 0.1f))`
- L34: `this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE))`
- L35: `this.addToBot(new PressEndTurnButtonAction())`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new SFXAction("ATTACK_HEAVY"));
        this.addToBot(new VFXAction(p, new CleaveEffect(), 0.1f));
        this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE));
        this.addToBot(new PressEndTurnButtonAction());
    }
```

</details>

## ConjureBlade
File: `cards\purple\ConjureBlade.java`

**Action sequence (in order):**
- `ConjureBladeAction` — `new ConjureBladeAction(p, this.freeToPlayOnce, this.energyOnUse + 1)`
- `ConjureBladeAction` — `new ConjureBladeAction(p, this.freeToPlayOnce, this.energyOnUse)`

**Queue order:**
- L28: `this.addToBot(new ConjureBladeAction(p, this.freeToPlayOnce, this.energyOnUse + 1))`
- L30: `this.addToBot(new ConjureBladeAction(p, this.freeToPlayOnce, this.energyOnUse))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (this.upgraded) {
            this.addToBot(new ConjureBladeAction(p, this.freeToPlayOnce, this.energyOnUse + 1));
        } else {
            this.addToBot(new ConjureBladeAction(p, this.freeToPlayOnce, this.energyOnUse));
        }
    }
```

</details>

## Consecrate
File: `cards\purple\Consecrate.java`

**Action sequence (in order):**
- `SFXAction` — `new SFXAction("ATTACK_HEAVY")`
- `VFXAction` — `new VFXAction(p, new CleaveEffect(), 0.1f)`
- `CleaveEffect` — `new CleaveEffect()`
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE)`

**Queue order:**
- L31: `this.addToBot(new SFXAction("ATTACK_HEAVY"))`
- L32: `this.addToBot(new VFXAction(p, new CleaveEffect(), 0.1f))`
- L33: `this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new SFXAction("ATTACK_HEAVY"));
        this.addToBot(new VFXAction(p, new CleaveEffect(), 0.1f));
        this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE));
    }
```

</details>

## ConserveBattery
File: `cards\blue\ConserveBattery.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new EnergizedBluePower(p, 1), 1)`
- `EnergizedBluePower` — `new EnergizedBluePower(p, 1)`

**Queue order:**
- L28: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L29: `this.addToBot(new ApplyPowerAction(p, p, new EnergizedBluePower(p, 1), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.addToBot(new ApplyPowerAction(p, p, new EnergizedBluePower(p, 1), 1));
    }
```

</details>

## Consume
File: `cards\blue\Consume.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new FocusPower(p, this.magicNumber), this.magicNumber)`
- `FocusPower` — `new FocusPower(p, this.magicNumber)`
- `DecreaseMaxOrbAction` — `new DecreaseMaxOrbAction(1)`

**Queue order:**
- L27: `this.addToBot(new ApplyPowerAction(p, p, new FocusPower(p, this.magicNumber), this.magicNumber))`
- L28: `this.addToBot(new DecreaseMaxOrbAction(1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new FocusPower(p, this.magicNumber), this.magicNumber));
        this.addToBot(new DecreaseMaxOrbAction(1));
    }
```

</details>

## Coolheaded
File: `cards\blue\Coolheaded.java`

**Action sequence (in order):**
- `ChannelAction` — `new ChannelAction(new Frost())`
- `Frost` — `new Frost()`
- `DrawCardAction` — `new DrawCardAction(p, this.magicNumber)`

**Queue order:**
- L29: `this.addToBot(new ChannelAction(new Frost()))`
- L30: `this.addToBot(new DrawCardAction(p, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ChannelAction(new Frost()));
        this.addToBot(new DrawCardAction(p, this.magicNumber));
    }
```

</details>

## CoreSurge
File: `cards\blue\CoreSurge.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new ArtifactPower(p, this.magicNumber), this.magicNumber)`
- `ArtifactPower` — `new ArtifactPower(p, this.magicNumber)`

**Queue order:**
- L32: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L33: `this.addToBot(new ApplyPowerAction(p, p, new ArtifactPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
        this.addToBot(new ApplyPowerAction(p, p, new ArtifactPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## CorpseExplosion
File: `cards\green\CorpseExplosion.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction((AbstractCreature)m, (AbstractCreature)p, (AbstractPower)new PoisonPower(m, p, this.magicNumber), this.magicNumber, AbstractGameAction.AttackEffect.POISON)`
- `PoisonPower` — `new PoisonPower(m, p, this.magicNumber)`
- `ApplyPowerAction` — `new ApplyPowerAction((AbstractCreature)m, (AbstractCreature)p, (AbstractPower)new CorpseExplosionPower(m), 1, AbstractGameAction.AttackEffect.POISON)`
- `CorpseExplosionPower` — `new CorpseExplosionPower(m)`

**Queue order:**
- L30: `this.addToBot(new ApplyPowerAction((AbstractCreature)m, (AbstractCreature)p, (AbstractPower)new PoisonPower(m, p, this.magicNumber), this.magicNumber, AbstractGameAction.AttackEffect.POISON))`
- L31: `this.addToBot(new ApplyPowerAction((AbstractCreature)m, (AbstractCreature)p, (AbstractPower)new CorpseExplosionPower(m), 1, AbstractGameAction.AttackEffect.POISON))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction((AbstractCreature)m, (AbstractCreature)p, (AbstractPower)new PoisonPower(m, p, this.magicNumber), this.magicNumber, AbstractGameAction.AttackEffect.POISON));
        this.addToBot(new ApplyPowerAction((AbstractCreature)m, (AbstractCreature)p, (AbstractPower)new CorpseExplosionPower(m), 1, AbstractGameAction.AttackEffect.POISON));
    }
```

</details>

## Corruption
File: `cards\red\Corruption.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(p, new VerticalAuraEffect(Color.BLACK, p.hb.cX, p.hb.cY), 0.33f)`
- `VerticalAuraEffect` — `new VerticalAuraEffect(Color.BLACK, p.hb.cX, p.hb.cY)`
- `SFXAction` — `new SFXAction("ATTACK_FIRE")`
- `VFXAction` — `new VFXAction(p, new VerticalAuraEffect(Color.PURPLE, p.hb.cX, p.hb.cY), 0.33f)`
- `VerticalAuraEffect` — `new VerticalAuraEffect(Color.PURPLE, p.hb.cX, p.hb.cY)`
- `VFXAction` — `new VFXAction(p, new VerticalAuraEffect(Color.CYAN, p.hb.cX, p.hb.cY), 0.0f)`
- `VerticalAuraEffect` — `new VerticalAuraEffect(Color.CYAN, p.hb.cX, p.hb.cY)`
- `VFXAction` — `new VFXAction(p, new BorderLongFlashEffect(Color.MAGENTA), 0.0f, true)`
- `BorderLongFlashEffect` — `new BorderLongFlashEffect(Color.MAGENTA)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new CorruptionPower(p))`
- `CorruptionPower` — `new CorruptionPower(p)`

**Queue order:**
- L32: `this.addToBot(new VFXAction(p, new VerticalAuraEffect(Color.BLACK, p.hb.cX, p.hb.cY), 0.33f))`
- L33: `this.addToBot(new SFXAction("ATTACK_FIRE"))`
- L34: `this.addToBot(new VFXAction(p, new VerticalAuraEffect(Color.PURPLE, p.hb.cX, p.hb.cY), 0.33f))`
- L35: `this.addToBot(new VFXAction(p, new VerticalAuraEffect(Color.CYAN, p.hb.cX, p.hb.cY), 0.0f))`
- L36: `this.addToBot(new VFXAction(p, new BorderLongFlashEffect(Color.MAGENTA), 0.0f, true))`
- L44: `this.addToBot(new ApplyPowerAction(p, p, new CorruptionPower(p)))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new VFXAction(p, new VerticalAuraEffect(Color.BLACK, p.hb.cX, p.hb.cY), 0.33f));
        this.addToBot(new SFXAction("ATTACK_FIRE"));
        this.addToBot(new VFXAction(p, new VerticalAuraEffect(Color.PURPLE, p.hb.cX, p.hb.cY), 0.33f));
        this.addToBot(new VFXAction(p, new VerticalAuraEffect(Color.CYAN, p.hb.cX, p.hb.cY), 0.0f));
        this.addToBot(new VFXAction(p, new BorderLongFlashEffect(Color.MAGENTA), 0.0f, true));
        boolean powerExists = false;
        for (AbstractPower pow : p.powers) {
            if (!pow.ID.equals(ID)) continue;
            powerExists = true;
            break;
        }
        if (!powerExists) {
            this.addToBot(new ApplyPowerAction(p, p, new CorruptionPower(p)));
        }
    }
```

</details>

## CreativeAI
File: `cards\blue\CreativeAI.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new CreativeAIPower(p, this.magicNumber), this.magicNumber)`
- `CreativeAIPower` — `new CreativeAIPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new CreativeAIPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new CreativeAIPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Crescendo
File: `cards\purple\Crescendo.java`

**Action sequence (in order):**
- `ChangeStanceAction` — `new ChangeStanceAction("Wrath")`

**Queue order:**
- L26: `this.addToBot(new ChangeStanceAction("Wrath"))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ChangeStanceAction("Wrath"));
    }
```

</details>

## CripplingPoison
File: `cards\green\CripplingPoison.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(monster, p, new PoisonPower(monster, p, this.magicNumber), this.magicNumber)`
- `PoisonPower` — `new PoisonPower(monster, p, this.magicNumber)`
- `ApplyPowerAction` — `new ApplyPowerAction(monster, p, new WeakPower(monster, 2, false), 2)`
- `WeakPower` — `new WeakPower(monster, 2, false)`

**Queue order:**
- L33: `this.addToBot(new ApplyPowerAction(monster, p, new PoisonPower(monster, p, this.magicNumber), this.magicNumber))`
- L34: `this.addToBot(new ApplyPowerAction(monster, p, new WeakPower(monster, 2, false), 2))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (!AbstractDungeon.getMonsters().areMonstersBasicallyDead()) {
            this.flash();
            for (AbstractMonster monster : AbstractDungeon.getMonsters().monsters) {
                if (monster.isDead || monster.isDying) continue;
                this.addToBot(new ApplyPowerAction(monster, p, new PoisonPower(monster, p, this.magicNumber), this.magicNumber));
                this.addToBot(new ApplyPowerAction(monster, p, new WeakPower(monster, 2, false), 2));
            }
        }
    }
```

</details>

## CrushJoints
File: `cards\purple\CrushJoints.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `CrushJointsAction` — `new CrushJointsAction(m, this.magicNumber)`

**Queue order:**
- L31: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L32: `this.addToBot(new CrushJointsAction(m, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
        this.addToBot(new CrushJointsAction(m, this.magicNumber));
    }
```

</details>

## CurseOfTheBell
File: `cards\curses\CurseOfTheBell.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
    }
```

</details>

## CutThroughFate
File: `cards\purple\CutThroughFate.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ScryAction` — `new ScryAction(this.magicNumber)`
- `DrawCardAction` — `new DrawCardAction(p, 1)`

**Queue order:**
- L31: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL))`
- L32: `this.addToBot(new ScryAction(this.magicNumber))`
- L33: `this.addToBot(new DrawCardAction(p, 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL));
        this.addToBot(new ScryAction(this.magicNumber));
        this.addToBot(new DrawCardAction(p, 1));
    }
```

</details>

## DEPRECATEDAlwaysMad
File: `cards\deprecated\DEPRECATEDAlwaysMad.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new DEPRECATEDAlwaysMadPower(p))`
- `DEPRECATEDAlwaysMadPower` — `new DEPRECATEDAlwaysMadPower(p)`

**Queue order:**
- L25: `this.addToBot(new ApplyPowerAction(p, p, new DEPRECATEDAlwaysMadPower(p)))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new DEPRECATEDAlwaysMadPower(p)));
    }
```

</details>

## DEPRECATEDAndCarryOn
File: `cards\deprecated\DEPRECATEDAndCarryOn.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction(p, p, this.block, false)`

**Queue order:**
- L27: `this.addToBot(new GainBlockAction(p, p, this.block, false))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction(p, p, this.block, false));
    }
```

</details>

## DEPRECATEDAwakenedStrike
File: `cards\deprecated\DEPRECATEDAwakenedStrike.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L30: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L32: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (this.upgraded) {
            this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
        } else {
            this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT));
        }
    }
```

</details>

## DEPRECATEDBalancedViolence
File: `cards\deprecated\DEPRECATEDBalancedViolence.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(m, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL)`
- `DamageInfo` — `new DamageInfo(m, this.damage, this.damageTypeForTurn)`
- `StanceCheckAction` — `new StanceCheckAction("Wrath", new DamageAction((AbstractCreature)m, new DamageInfo(m, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL))`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(m, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL)`
- `DamageInfo` — `new DamageInfo(m, this.damage, this.damageTypeForTurn)`
- `StanceCheckAction` — `new StanceCheckAction("Calm", new DrawCardAction(this.magicNumber))`
- `DrawCardAction` — `new DrawCardAction(this.magicNumber)`

**Queue order:**
- L33: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(m, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL))`
- L34: `this.addToBot(new StanceCheckAction("Wrath", new DamageAction((AbstractCreature)m, new DamageInfo(m, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL)))`
- L35: `this.addToBot(new StanceCheckAction("Calm", new DrawCardAction(this.magicNumber)))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(m, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL));
        this.addToBot(new StanceCheckAction("Wrath", new DamageAction((AbstractCreature)m, new DamageInfo(m, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL)));
        this.addToBot(new StanceCheckAction("Calm", new DrawCardAction(this.magicNumber)));
    }
```

</details>

## DEPRECATEDBigBrain
File: `cards\deprecated\DEPRECATEDBigBrain.java`

**Action sequence (in order):**
- `DrawCardAction` — `new DrawCardAction(1)`

**Queue order:**
- L26: `this.addToBot(new DrawCardAction(1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DrawCardAction(1));
    }
```

</details>

## DEPRECATEDBlessed
File: `cards\deprecated\DEPRECATEDBlessed.java`

**Action sequence (in order):**
- `MakeTempCardInDrawPileAction` — `new MakeTempCardInDrawPileAction(miracle, this.magicNumber, true, true, false)`

**Queue order:**
- L33: `this.addToBot(new MakeTempCardInDrawPileAction(miracle, this.magicNumber, true, true, false))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        AbstractCard miracle = CardLibrary.getCard("Miracle").makeCopy();
        if (this.upgraded) {
            miracle.upgrade();
        }
        this.addToBot(new MakeTempCardInDrawPileAction(miracle, this.magicNumber, true, true, false));
    }
```

</details>

## DEPRECATEDBliss
File: `cards\deprecated\DEPRECATEDBliss.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `ChangeStanceAction` — `new ChangeStanceAction("Calm")`

**Queue order:**
- L28: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L29: `this.addToBot(new ChangeStanceAction("Calm"))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.addToBot(new ChangeStanceAction("Calm"));
    }
```

</details>

## DEPRECATEDBrillianceAura
File: `cards\deprecated\DEPRECATEDBrillianceAura.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
    }
```

</details>

## DEPRECATEDCalm
File: `cards\deprecated\DEPRECATEDCalm.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
    }
```

</details>

## DEPRECATEDCausality
File: `cards\deprecated\DEPRECATEDCausality.java`

**Action sequence (in order):**
- `ExpertiseAction` — `new ExpertiseAction(p, 10)`

**Queue order:**
- L25: `this.addToBot(new ExpertiseAction(p, 10))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ExpertiseAction(p, 10));
    }
```

</details>

## DEPRECATEDChallengeAccepted
File: `cards\deprecated\DEPRECATEDChallengeAccepted.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new VulnerablePower(p, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE)`
- `VulnerablePower` — `new VulnerablePower(p, this.magicNumber, false)`
- `ApplyPowerAction` — `new ApplyPowerAction(mo, p, new VulnerablePower(mo, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE)`
- `VulnerablePower` — `new VulnerablePower(mo, this.magicNumber, false)`
- `ChangeStanceAction` — `new ChangeStanceAction("Calm")`

**Queue order:**
- L29: `this.addToBot(new ApplyPowerAction(p, p, new VulnerablePower(p, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE))`
- L31: `this.addToBot(new ApplyPowerAction(mo, p, new VulnerablePower(mo, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE))`
- L33: `this.addToBot(new ChangeStanceAction("Calm"))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new VulnerablePower(p, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE));
        for (AbstractMonster mo : AbstractDungeon.getCurrRoom().monsters.monsters) {
            this.addToBot(new ApplyPowerAction(mo, p, new VulnerablePower(mo, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE));
        }
        this.addToBot(new ChangeStanceAction("Calm"));
    }
```

</details>

## DEPRECATEDChooseCalm
File: `cards\deprecated\DEPRECATEDChooseCalm.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
    }
```

</details>

## DEPRECATEDChooseCourage
File: `cards\deprecated\DEPRECATEDChooseCourage.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
    }
```

</details>

## DEPRECATEDClarity
File: `cards\deprecated\DEPRECATEDClarity.java`

**Action sequence (in order):**
- `ClarityAction` — `new ClarityAction(this.magicNumber)`

**Queue order:**
- L25: `this.addToBot(new ClarityAction(this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ClarityAction(this.magicNumber));
    }
```

</details>

## DEPRECATEDCleanseEvil
File: `cards\deprecated\DEPRECATEDCleanseEvil.java`

**Action sequence (in order):**
- `Smite` — `new Smite()`
- `DivinePunishmentAction` — `new DivinePunishmentAction(c, this.freeToPlayOnce, this.energyOnUse)`

**Queue order:**
- L30: `this.addToBot(new DivinePunishmentAction(c, this.freeToPlayOnce, this.energyOnUse))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        Smite c = new Smite();
        if (this.upgraded) {
            ((AbstractCard)c).upgrade();
        }
        this.addToBot(new DivinePunishmentAction(c, this.freeToPlayOnce, this.energyOnUse));
    }
```

</details>

## DEPRECATEDCondense
File: `cards\deprecated\DEPRECATEDCondense.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new DEPRECATEDCondensePower(p, this.magicNumber))`
- `DEPRECATEDCondensePower` — `new DEPRECATEDCondensePower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new DEPRECATEDCondensePower(p, this.magicNumber)))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new DEPRECATEDCondensePower(p, this.magicNumber)));
    }
```

</details>

## DEPRECATEDConfront
File: `cards\deprecated\DEPRECATEDConfront.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ChangeStanceAction` — `new ChangeStanceAction("Calm")`

**Queue order:**
- L30: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY))`
- L31: `this.addToBot(new ChangeStanceAction("Calm"))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY));
        this.addToBot(new ChangeStanceAction("Calm"));
    }
```

</details>

## DEPRECATEDContemplate
File: `cards\deprecated\DEPRECATEDContemplate.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
    }
```

</details>

## DEPRECATEDCrescentKick
File: `cards\deprecated\DEPRECATEDCrescentKick.java`

**Action sequence (in order):**
- `CrescentKickAction` — `new CrescentKickAction(p, this)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L31: `this.addToBot(new CrescentKickAction(p, this))`
- L32: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new CrescentKickAction(p, this));
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
    }
```

</details>

## DEPRECATEDEruption
File: `cards\deprecated\DEPRECATEDEruption.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.FIRE)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ChangeStanceAction` — `new ChangeStanceAction("Wrath")`

**Queue order:**
- L29: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.FIRE))`
- L30: `this.addToBot(new ChangeStanceAction("Wrath"))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.FIRE));
        this.addToBot(new ChangeStanceAction("Wrath"));
    }
```

</details>

## DEPRECATEDExperienced
File: `cards\deprecated\DEPRECATEDExperienced.java`

**Action sequence (in order):**
- `DEPRECATEDExperiencedAction` — `new DEPRECATEDExperiencedAction(this.block, this)`

**Queue order:**
- L26: `this.addToBot(new DEPRECATEDExperiencedAction(this.block, this))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DEPRECATEDExperiencedAction(this.block, this));
    }
```

</details>

## DEPRECATEDFlameMastery
File: `cards\deprecated\DEPRECATEDFlameMastery.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(p, new InflameEffect(p), 1.0f)`
- `InflameEffect` — `new InflameEffect(p)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new StrengthPower(p, this.magicNumber), this.magicNumber)`
- `StrengthPower` — `new StrengthPower(p, this.magicNumber)`

**Queue order:**
- L28: `this.addToBot(new VFXAction(p, new InflameEffect(p), 1.0f))`
- L29: `this.addToBot(new ApplyPowerAction(p, p, new StrengthPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new VFXAction(p, new InflameEffect(p), 1.0f));
        this.addToBot(new ApplyPowerAction(p, p, new StrengthPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## DEPRECATEDFlare
File: `cards\deprecated\DEPRECATEDFlare.java`

**Action sequence (in order):**
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.FIRE)`
- `StanceCheckAction` — `new StanceCheckAction("Wrath", new GainEnergyAction(2))`
- `GainEnergyAction` — `new GainEnergyAction(2)`

**Queue order:**
- L31: `this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.FIRE))`
- L32: `this.addToBot(new StanceCheckAction("Wrath", new GainEnergyAction(2)))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.FIRE));
        this.addToBot(new StanceCheckAction("Wrath", new GainEnergyAction(2)));
    }
```

</details>

## DEPRECATEDFlick
File: `cards\deprecated\DEPRECATEDFlick.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ApplyPowerAction` — `new ApplyPowerAction(m, p, new DEPRECATEDFlickedPower(m, 1), 1)`
- `DEPRECATEDFlickedPower` — `new DEPRECATEDFlickedPower(m, 1)`

**Queue order:**
- L31: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT))`
- L32: `this.addToBot(new ApplyPowerAction(m, p, new DEPRECATEDFlickedPower(m, 1), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT));
        this.addToBot(new ApplyPowerAction(m, p, new DEPRECATEDFlickedPower(m, 1), 1));
    }
```

</details>

## DEPRECATEDFlicker
File: `cards\deprecated\DEPRECATEDFlicker.java`

**Action sequence (in order):**
- `FlickerAction` — `new FlickerAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn), this)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L26: `this.addToBot(new FlickerAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn), this))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new FlickerAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn), this));
    }
```

</details>

## DEPRECATEDFlow
File: `cards\deprecated\DEPRECATEDFlow.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new DEPRECATEDFlowPower(p, 1), 1)`
- `DEPRECATEDFlowPower` — `new DEPRECATEDFlowPower(p, 1)`

**Queue order:**
- L25: `this.addToBot(new ApplyPowerAction(p, p, new DEPRECATEDFlowPower(p, 1), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new DEPRECATEDFlowPower(p, 1), 1));
    }
```

</details>

## DEPRECATEDFlowState
File: `cards\deprecated\DEPRECATEDFlowState.java`

**Action sequence (in order):**
- `EmotionalTurmoilAction` — `new EmotionalTurmoilAction()`

**Queue order:**
- L25: `this.addToBot(new EmotionalTurmoilAction())`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new EmotionalTurmoilAction());
    }
```

</details>

## DEPRECATEDFury
File: `cards\deprecated\DEPRECATEDFury.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ChangeStanceAction` — `new ChangeStanceAction("Wrath")`

**Queue order:**
- L29: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT))`
- L30: `this.addToBot(new ChangeStanceAction("Wrath"))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT));
        this.addToBot(new ChangeStanceAction("Wrath"));
    }
```

</details>

## DEPRECATEDFuryAura
File: `cards\deprecated\DEPRECATEDFuryAura.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new StrengthPower(p, this.magicNumber), this.magicNumber)`
- `StrengthPower` — `new StrengthPower(p, this.magicNumber)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new LoseStrengthPower(p, this.magicNumber), this.magicNumber)`
- `LoseStrengthPower` — `new LoseStrengthPower(p, this.magicNumber)`

**Queue order:**
- L30: `this.addToBot(new ApplyPowerAction(p, p, new StrengthPower(p, this.magicNumber), this.magicNumber))`
- L31: `this.addToBot(new ApplyPowerAction(p, p, new LoseStrengthPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new StrengthPower(p, this.magicNumber), this.magicNumber));
        this.addToBot(new ApplyPowerAction(p, p, new LoseStrengthPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## DEPRECATEDGrounded
File: `cards\deprecated\DEPRECATEDGrounded.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new DEPRECATEDGroundedPower(p))`
- `DEPRECATEDGroundedPower` — `new DEPRECATEDGroundedPower(p)`

**Queue order:**
- L25: `this.addToBot(new ApplyPowerAction(p, p, new DEPRECATEDGroundedPower(p)))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new DEPRECATEDGroundedPower(p)));
    }
```

</details>

## DEPRECATEDHotHot
File: `cards\deprecated\DEPRECATEDHotHot.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new DEPRECATEDHotHotPower(p, this.magicNumber), this.magicNumber)`
- `DEPRECATEDHotHotPower` — `new DEPRECATEDHotHotPower(p, this.magicNumber)`

**Queue order:**
- L29: `this.addToBot(new ApplyPowerAction(p, p, new DEPRECATEDHotHotPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new DEPRECATEDHotHotPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## DEPRECATEDIntrospection
File: `cards\deprecated\DEPRECATEDIntrospection.java`

**Action sequence (in order):**
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction((AbstractCreature)p, (AbstractCreature)p, "Frail")`
- `RemoveSpecificPowerAction` — `new RemoveSpecificPowerAction((AbstractCreature)p, (AbstractCreature)p, "Vulnerable")`

**Queue order:**
- L26: `this.addToBot(new RemoveSpecificPowerAction((AbstractCreature)p, (AbstractCreature)p, "Frail"))`
- L27: `this.addToBot(new RemoveSpecificPowerAction((AbstractCreature)p, (AbstractCreature)p, "Vulnerable"))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new RemoveSpecificPowerAction((AbstractCreature)p, (AbstractCreature)p, "Frail"));
        this.addToBot(new RemoveSpecificPowerAction((AbstractCreature)p, (AbstractCreature)p, "Vulnerable"));
    }
```

</details>

## DEPRECATEDLetFateDecide
File: `cards\deprecated\DEPRECATEDLetFateDecide.java`

**Action sequence (in order):**
- `PlayTopCardAction` — `new PlayTopCardAction(AbstractDungeon.getCurrRoom().monsters.getRandomMonster(null, true, AbstractDungeon.cardRandomRng), false)`

**Queue order:**
- L26: `this.addToBot(new PlayTopCardAction(AbstractDungeon.getCurrRoom().monsters.getRandomMonster(null, true, AbstractDungeon.cardRandomRng), false))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        for (int i = 0; i < this.energyOnUse; ++i) {
            this.addToBot(new PlayTopCardAction(AbstractDungeon.getCurrRoom().monsters.getRandomMonster(null, true, AbstractDungeon.cardRandomRng), false));
        }
        if (this.energyOnUse >= 3) {
            // empty if block
        }
    }
```

</details>

## DEPRECATEDMasterReality
File: `cards\deprecated\DEPRECATEDMasterReality.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new DEPRECATEDMasterRealityPower(p, this.magicNumber), this.magicNumber)`
- `DEPRECATEDMasterRealityPower` — `new DEPRECATEDMasterRealityPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new DEPRECATEDMasterRealityPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new DEPRECATEDMasterRealityPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## DEPRECATEDMastery
File: `cards\deprecated\DEPRECATEDMastery.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new DEPRECATEDMasteryPower(p, this.magicNumber), this.magicNumber)`
- `DEPRECATEDMasteryPower` — `new DEPRECATEDMasteryPower(p, this.magicNumber)`

**Queue order:**
- L27: `this.addToBot(new ApplyPowerAction(p, p, new DEPRECATEDMasteryPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new DEPRECATEDMasteryPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## DEPRECATEDMetaphysics
File: `cards\deprecated\DEPRECATEDMetaphysics.java`

**Action sequence (in order):**
- `MakeTempCardInDrawPileAction` — `new MakeTempCardInDrawPileAction(this.cardsToPreview.makeStatEquivalentCopy(), 1, true, true)`

**Queue order:**
- L27: `this.addToBot(new MakeTempCardInDrawPileAction(this.cardsToPreview.makeStatEquivalentCopy(), 1, true, true))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new MakeTempCardInDrawPileAction(this.cardsToPreview.makeStatEquivalentCopy(), 1, true, true));
    }
```

</details>

## DEPRECATEDNothingness
File: `cards\deprecated\DEPRECATEDNothingness.java`

**Action sequence (in order):**
- `ScryAction` — `new ScryAction(DEPRECATEDNothingness.countCards())`
- `DrawCardAction` — `new DrawCardAction(p, DEPRECATEDNothingness.countCards())`

**Queue order:**
- L50: `this.addToBot(new ScryAction(DEPRECATEDNothingness.countCards()))`
- L52: `this.addToBot(new DrawCardAction(p, DEPRECATEDNothingness.countCards()))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (this.upgraded) {
            this.addToBot(new ScryAction(DEPRECATEDNothingness.countCards()));
        }
        this.addToBot(new DrawCardAction(p, DEPRECATEDNothingness.countCards()));
    }
```

</details>

## DEPRECATEDPathToVictory
File: `cards\deprecated\DEPRECATEDPathToVictory.java`

**Action sequence (in order):**
- `PlayTopCardAction` — `new PlayTopCardAction(AbstractDungeon.getCurrRoom().monsters.getRandomMonster(null, true, AbstractDungeon.cardRandomRng), false)`

**Queue order:**
- L26: `this.addToBot(new PlayTopCardAction(AbstractDungeon.getCurrRoom().monsters.getRandomMonster(null, true, AbstractDungeon.cardRandomRng), false))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new PlayTopCardAction(AbstractDungeon.getCurrRoom().monsters.getRandomMonster(null, true, AbstractDungeon.cardRandomRng), false));
    }
```

</details>

## DEPRECATEDPeace
File: `cards\deprecated\DEPRECATEDPeace.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(mo, p, new StrengthPower(mo, -this.magicNumber), -this.magicNumber, true, AbstractGameAction.AttackEffect.NONE)`
- `StrengthPower` — `new StrengthPower(mo, -this.magicNumber)`
- `ApplyPowerAction` — `new ApplyPowerAction(mo, p, new GainStrengthPower(mo, this.magicNumber), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE)`
- `GainStrengthPower` — `new GainStrengthPower(mo, this.magicNumber)`

**Queue order:**
- L32: `this.addToBot(new ApplyPowerAction(mo, p, new StrengthPower(mo, -this.magicNumber), -this.magicNumber, true, AbstractGameAction.AttackEffect.NONE))`
- L36: `this.addToBot(new ApplyPowerAction(mo, p, new GainStrengthPower(mo, this.magicNumber), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        for (AbstractMonster mo : AbstractDungeon.getCurrRoom().monsters.monsters) {
            this.addToBot(new ApplyPowerAction(mo, p, new StrengthPower(mo, -this.magicNumber), -this.magicNumber, true, AbstractGameAction.AttackEffect.NONE));
        }
        for (AbstractMonster mo : AbstractDungeon.getCurrRoom().monsters.monsters) {
            if (mo.hasPower("Artifact")) continue;
            this.addToBot(new ApplyPowerAction(mo, p, new GainStrengthPower(mo, this.magicNumber), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE));
        }
    }
```

</details>

## DEPRECATEDPerfectedForm
File: `cards\deprecated\DEPRECATEDPerfectedForm.java`

**Action sequence (in order):**
- `PerfectedFormAction` — `new PerfectedFormAction()`

**Queue order:**
- L25: `this.addToBot(new PerfectedFormAction())`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new PerfectedFormAction());
    }
```

</details>

## DEPRECATEDPolymath
File: `cards\deprecated\DEPRECATEDPolymath.java`

**Action sequence (in order):**
- `None` — `new ArrayList<AbstractCard>()`
- `ChooseWrath` — `new ChooseWrath()`
- `ChooseCalm` — `new ChooseCalm()`
- `ChooseOneAction` — `new ChooseOneAction(stanceChoices)`

**Queue order:**
- L31: `this.addToBot(new ChooseOneAction(stanceChoices))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        ArrayList<AbstractCard> stanceChoices = new ArrayList<AbstractCard>();
        stanceChoices.add(new ChooseWrath());
        stanceChoices.add(new ChooseCalm());
        this.addToBot(new ChooseOneAction(stanceChoices));
    }
```

</details>

## DEPRECATEDPrediction
File: `cards\deprecated\DEPRECATEDPrediction.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new NextTurnBlockPower(p, this.block), this.block)`
- `NextTurnBlockPower` — `new NextTurnBlockPower(p, this.block)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new NextTurnBlockPower(p, this.block), this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new NextTurnBlockPower(p, this.block), this.block));
    }
```

</details>

## DEPRECATEDPunishment
File: `cards\deprecated\DEPRECATEDPunishment.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
    }
```

</details>

## DEPRECATEDRestrainingPalm
File: `cards\deprecated\DEPRECATEDRestrainingPalm.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ApplyPowerAction` — `new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber)`
- `WeakPower` — `new WeakPower(m, this.magicNumber, false)`

**Queue order:**
- L31: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L32: `this.addToBot(new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
        this.addToBot(new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber));
    }
```

</details>

## DEPRECATEDRetreatingHand
File: `cards\deprecated\DEPRECATEDRetreatingHand.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `RetreatingHandAction` — `new RetreatingHandAction(this)`

**Queue order:**
- L29: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L30: `this.addToBot(new RetreatingHandAction(this))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.addToBot(new RetreatingHandAction(this));
    }
```

</details>

## DEPRECATEDRetribution
File: `cards\deprecated\DEPRECATEDRetribution.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new DEPRECATEDRetributionPower(p, this.magicNumber), this.magicNumber)`
- `DEPRECATEDRetributionPower` — `new DEPRECATEDRetributionPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new DEPRECATEDRetributionPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new DEPRECATEDRetributionPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## DEPRECATEDSerenity
File: `cards\deprecated\DEPRECATEDSerenity.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new DEPRECATEDSerenityPower(p, this.magicNumber), this.magicNumber)`
- `DEPRECATEDSerenityPower` — `new DEPRECATEDSerenityPower(p, this.magicNumber)`

**Queue order:**
- L27: `this.addToBot(new ApplyPowerAction(p, p, new DEPRECATEDSerenityPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new DEPRECATEDSerenityPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## DEPRECATEDSimmeringRage
File: `cards\deprecated\DEPRECATEDSimmeringRage.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new StrengthPower(p, this.magicNumber), this.magicNumber)`
- `StrengthPower` — `new StrengthPower(p, this.magicNumber)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new DexterityPower(p, -1), -1)`
- `DexterityPower` — `new DexterityPower(p, -1)`
- `ChangeStanceAction` — `new ChangeStanceAction("Wrath")`

**Queue order:**
- L28: `this.addToBot(new ApplyPowerAction(p, p, new StrengthPower(p, this.magicNumber), this.magicNumber))`
- L29: `this.addToBot(new ApplyPowerAction(p, p, new DexterityPower(p, -1), -1))`
- L30: `this.addToBot(new ChangeStanceAction("Wrath"))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new StrengthPower(p, this.magicNumber), this.magicNumber));
        this.addToBot(new ApplyPowerAction(p, p, new DexterityPower(p, -1), -1));
        this.addToBot(new ChangeStanceAction("Wrath"));
    }
```

</details>

## DEPRECATEDSmile
File: `cards\deprecated\DEPRECATEDSmile.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
    }
```

</details>

## DEPRECATEDSoothingAura
File: `cards\deprecated\DEPRECATEDSoothingAura.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber)`
- `WeakPower` — `new WeakPower(m, this.magicNumber, false)`

**Queue order:**
- L28: `this.addToBot(new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber));
    }
```

</details>

## DEPRECATEDStepAndStrike
File: `cards\deprecated\DEPRECATEDStepAndStrike.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L41: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L42: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY));
    }
```

</details>

## DEPRECATEDStomp
File: `cards\deprecated\DEPRECATEDStomp.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ApplyPowerAction` — `new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE)`
- `WeakPower` — `new WeakPower(m, this.magicNumber, false)`

**Queue order:**
- L31: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT))`
- L32: `this.addToBot(new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT));
        this.addToBot(new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE));
    }
```

</details>

## DEPRECATEDSublimeSlice
File: `cards\deprecated\DEPRECATEDSublimeSlice.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(m, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL)`
- `DamageInfo` — `new DamageInfo(m, this.damage, this.damageTypeForTurn)`
- `DEPRECATEDRandomStanceAction` — `new DEPRECATEDRandomStanceAction()`

**Queue order:**
- L29: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(m, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL))`
- L30: `this.addToBot(new DEPRECATEDRandomStanceAction())`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(m, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL));
        this.addToBot(new DEPRECATEDRandomStanceAction());
    }
```

</details>

## DEPRECATEDSurvey
File: `cards\deprecated\DEPRECATEDSurvey.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new VigorPower(p, this.magicNumber), this.magicNumber)`
- `VigorPower` — `new VigorPower(p, this.magicNumber)`

**Queue order:**
- L29: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L30: `this.addToBot(new ApplyPowerAction(p, p, new VigorPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.addToBot(new ApplyPowerAction(p, p, new VigorPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## DEPRECATEDSwipe
File: `cards\deprecated\DEPRECATEDSwipe.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn))`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `DamageAction` — `new DamageAction(mo, new DamageInfo(p, this.damage / 2, this.damageTypeForTurn))`
- `DamageInfo` — `new DamageInfo(p, this.damage / 2, this.damageTypeForTurn)`

**Queue order:**
- L27: `this.addToBot(new DamageAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn)))`
- L30: `this.addToBot(new DamageAction(mo, new DamageInfo(p, this.damage / 2, this.damageTypeForTurn)))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn)));
        for (AbstractMonster mo : AbstractDungeon.getMonsters().monsters) {
            if (mo == m) continue;
            this.addToBot(new DamageAction(mo, new DamageInfo(p, this.damage / 2, this.damageTypeForTurn)));
        }
    }
```

</details>

## DEPRECATEDTemperTantrum
File: `cards\deprecated\DEPRECATEDTemperTantrum.java`

**Action sequence (in order):**
- `SwordBoomerangAction` — `new SwordBoomerangAction(AbstractDungeon.getMonsters().getRandomMonster(null, true, AbstractDungeon.cardRandomRng), new DamageInfo(p, this.baseDamage), 1)`
- `DamageInfo` — `new DamageInfo(p, this.baseDamage)`
- `StanceCheckAction` — `new StanceCheckAction("Wrath", new SwordBoomerangAction(new DamageInfo(p, this.baseDamage), 1))`
- `SwordBoomerangAction` — `new SwordBoomerangAction(new DamageInfo(p, this.baseDamage), 1)`
- `DamageInfo` — `new DamageInfo(p, this.baseDamage)`

**Queue order:**
- L29: `this.addToBot(new SwordBoomerangAction(AbstractDungeon.getMonsters().getRandomMonster(null, true, AbstractDungeon.cardRandomRng), new DamageInfo(p, this.baseDamage), 1))`
- L30: `this.addToBot(new StanceCheckAction("Wrath", new SwordBoomerangAction(new DamageInfo(p, this.baseDamage), 1)))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new SwordBoomerangAction(AbstractDungeon.getMonsters().getRandomMonster(null, true, AbstractDungeon.cardRandomRng), new DamageInfo(p, this.baseDamage), 1));
        this.addToBot(new StanceCheckAction("Wrath", new SwordBoomerangAction(new DamageInfo(p, this.baseDamage), 1)));
    }
```

</details>

## DEPRECATEDTorrent
File: `cards\deprecated\DEPRECATEDTorrent.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new BorderLongFlashEffect(Color.CYAN))`
- `BorderLongFlashEffect` — `new BorderLongFlashEffect(Color.CYAN)`
- `ShakeScreenAction` — `new ShakeScreenAction(0.0f, ScreenShake.ShakeDur.MED, ScreenShake.ShakeIntensity.HIGH)`
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction(p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.SLASH_HORIZONTAL, true)`

**Queue order:**
- L35: `this.addToBot(new VFXAction(new BorderLongFlashEffect(Color.CYAN)))`
- L36: `this.addToBot(new ShakeScreenAction(0.0f, ScreenShake.ShakeDur.MED, ScreenShake.ShakeIntensity.HIGH))`
- L38: `this.addToBot(new DamageAllEnemiesAction(p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.SLASH_HORIZONTAL, true))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new VFXAction(new BorderLongFlashEffect(Color.CYAN)));
        this.addToBot(new ShakeScreenAction(0.0f, ScreenShake.ShakeDur.MED, ScreenShake.ShakeIntensity.HIGH));
        for (int i = 0; i < this.magicNumber; ++i) {
            this.addToBot(new DamageAllEnemiesAction(p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.SLASH_HORIZONTAL, true));
        }
    }
```

</details>

## DEPRECATEDTranscendence
File: `cards\deprecated\DEPRECATEDTranscendence.java`

**Action sequence (in order):**
- `TranscendenceAction` — `new TranscendenceAction()`

**Queue order:**
- L24: `this.addToBot(new TranscendenceAction())`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new TranscendenceAction());
    }
```

</details>

## DEPRECATEDTruth
File: `cards\deprecated\DEPRECATEDTruth.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `DrawCardAction` — `new DrawCardAction(this.magicNumber)`

**Queue order:**
- L32: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L33: `this.addToBot(new DrawCardAction(this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
        this.addToBot(new DrawCardAction(this.magicNumber));
    }
```

</details>

## DEPRECATEDWardAura
File: `cards\deprecated\DEPRECATEDWardAura.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`

**Queue order:**
- L30: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
    }
```

</details>

## DEPRECATEDWindup
File: `cards\deprecated\DEPRECATEDWindup.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new VigorPower(p, this.magicNumber), this.magicNumber)`
- `VigorPower` — `new VigorPower(p, this.magicNumber)`

**Queue order:**
- L31: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L32: `this.addToBot(new ApplyPowerAction(p, p, new VigorPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
        this.addToBot(new ApplyPowerAction(p, p, new VigorPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## DEPRECATEDWisdom
File: `cards\deprecated\DEPRECATEDWisdom.java`

**Action sequence (in order):**
- `DrawCardAction` — `new DrawCardAction(this.magicNumber)`

**Queue order:**
- L27: `this.addToBot(new DrawCardAction(this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DrawCardAction(this.magicNumber));
    }
```

</details>

## DEPRECATEDWrath
File: `cards\deprecated\DEPRECATEDWrath.java`

**Action sequence (in order):**
- `ChangeStanceAction` — `new ChangeStanceAction(ID)`

**Queue order:**
- L24: `this.addToBot(new ChangeStanceAction(ID))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ChangeStanceAction(ID));
    }
```

</details>

## DaggerSpray
File: `cards\green\DaggerSpray.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new DaggerSprayEffect(AbstractDungeon.getMonsters().shouldFlipVfx()), 0.0f)`
- `DaggerSprayEffect` — `new DaggerSprayEffect(AbstractDungeon.getMonsters().shouldFlipVfx())`
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE)`
- `VFXAction` — `new VFXAction(new DaggerSprayEffect(AbstractDungeon.getMonsters().shouldFlipVfx()), 0.0f)`
- `DaggerSprayEffect` — `new DaggerSprayEffect(AbstractDungeon.getMonsters().shouldFlipVfx())`
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE)`

**Queue order:**
- L31: `this.addToBot(new VFXAction(new DaggerSprayEffect(AbstractDungeon.getMonsters().shouldFlipVfx()), 0.0f))`
- L32: `this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE))`
- L33: `this.addToBot(new VFXAction(new DaggerSprayEffect(AbstractDungeon.getMonsters().shouldFlipVfx()), 0.0f))`
- L34: `this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new VFXAction(new DaggerSprayEffect(AbstractDungeon.getMonsters().shouldFlipVfx()), 0.0f));
        this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE));
        this.addToBot(new VFXAction(new DaggerSprayEffect(AbstractDungeon.getMonsters().shouldFlipVfx()), 0.0f));
        this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE));
    }
```

</details>

## DaggerThrow
File: `cards\green\DaggerThrow.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new ThrowDaggerEffect(m.hb.cX, m.hb.cY))`
- `ThrowDaggerEffect` — `new ThrowDaggerEffect(m.hb.cX, m.hb.cY)`
- `DamageAction` — `new DamageAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn))`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `DrawCardAction` — `new DrawCardAction(p, 1)`
- `DiscardAction` — `new DiscardAction(p, p, 1, false)`

**Queue order:**
- L31: `this.addToBot(new VFXAction(new ThrowDaggerEffect(m.hb.cX, m.hb.cY)))`
- L33: `this.addToBot(new DamageAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn)))`
- L34: `this.addToBot(new DrawCardAction(p, 1))`
- L35: `this.addToBot(new DiscardAction(p, p, 1, false))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (m != null) {
            this.addToBot(new VFXAction(new ThrowDaggerEffect(m.hb.cX, m.hb.cY)));
        }
        this.addToBot(new DamageAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn)));
        this.addToBot(new DrawCardAction(p, 1));
        this.addToBot(new DiscardAction(p, p, 1, false));
    }
```

</details>

## DarkEmbrace
File: `cards\red\DarkEmbrace.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new DarkEmbracePower(p, 1), 1)`
- `DarkEmbracePower` — `new DarkEmbracePower(p, 1)`

**Queue order:**
- L25: `this.addToBot(new ApplyPowerAction(p, p, new DarkEmbracePower(p, 1), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new DarkEmbracePower(p, 1), 1));
    }
```

</details>

## DarkShackles
File: `cards\colorless\DarkShackles.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(m, p, new StrengthPower(m, -this.magicNumber), -this.magicNumber)`
- `StrengthPower` — `new StrengthPower(m, -this.magicNumber)`
- `ApplyPowerAction` — `new ApplyPowerAction(m, p, new GainStrengthPower(m, this.magicNumber), this.magicNumber)`
- `GainStrengthPower` — `new GainStrengthPower(m, this.magicNumber)`

**Queue order:**
- L28: `this.addToBot(new ApplyPowerAction(m, p, new StrengthPower(m, -this.magicNumber), -this.magicNumber))`
- L30: `this.addToBot(new ApplyPowerAction(m, p, new GainStrengthPower(m, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(m, p, new StrengthPower(m, -this.magicNumber), -this.magicNumber));
        if (m != null && !m.hasPower("Artifact")) {
            this.addToBot(new ApplyPowerAction(m, p, new GainStrengthPower(m, this.magicNumber), this.magicNumber));
        }
    }
```

</details>

## Darkness
File: `cards\blue\Darkness.java`

**Action sequence (in order):**
- `ChannelAction` — `new ChannelAction(new Dark())`
- `Dark` — `new Dark()`
- `DarkImpulseAction` — `new DarkImpulseAction()`

**Queue order:**
- L30: `this.addToBot(new ChannelAction(new Dark()))`
- L32: `this.addToBot(new DarkImpulseAction())`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ChannelAction(new Dark()));
        if (this.upgraded) {
            this.addToBot(new DarkImpulseAction());
        }
    }
```

</details>

## Dash
File: `cards\green\Dash.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L30: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L31: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL));
    }
```

</details>

## Dazed
File: `cards\status\Dazed.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
    }
```

</details>

## DeadlyPoison
File: `cards\green\DeadlyPoison.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction((AbstractCreature)m, (AbstractCreature)p, (AbstractPower)new PoisonPower(m, p, this.magicNumber), this.magicNumber, AbstractGameAction.AttackEffect.POISON)`
- `PoisonPower` — `new PoisonPower(m, p, this.magicNumber)`

**Queue order:**
- L29: `this.addToBot(new ApplyPowerAction((AbstractCreature)m, (AbstractCreature)p, (AbstractPower)new PoisonPower(m, p, this.magicNumber), this.magicNumber, AbstractGameAction.AttackEffect.POISON))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction((AbstractCreature)m, (AbstractCreature)p, (AbstractPower)new PoisonPower(m, p, this.magicNumber), this.magicNumber, AbstractGameAction.AttackEffect.POISON));
    }
```

</details>

## Decay
File: `cards\curses\Decay.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)AbstractDungeon.player, new DamageInfo(AbstractDungeon.player, 2, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.FIRE)`
- `DamageInfo` — `new DamageInfo(AbstractDungeon.player, 2, DamageInfo.DamageType.THORNS)`

**Queue order:**
- L30: `this.addToTop(new DamageAction((AbstractCreature)AbstractDungeon.player, new DamageInfo(AbstractDungeon.player, 2, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.FIRE))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (this.dontTriggerOnUseCard) {
            this.addToTop(new DamageAction((AbstractCreature)AbstractDungeon.player, new DamageInfo(AbstractDungeon.player, 2, DamageInfo.DamageType.THORNS), AbstractGameAction.AttackEffect.FIRE));
        }
    }
```

</details>

## DeceiveReality
File: `cards\purple\DeceiveReality.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `MakeTempCardInHandAction` — `new MakeTempCardInHandAction(this.cardsToPreview.makeStatEquivalentCopy(), 1)`

**Queue order:**
- L29: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L30: `this.addToBot(new MakeTempCardInHandAction(this.cardsToPreview.makeStatEquivalentCopy(), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.addToBot(new MakeTempCardInHandAction(this.cardsToPreview.makeStatEquivalentCopy(), 1));
    }
```

</details>

## DeepBreath
File: `cards\colorless\DeepBreath.java`

**Action sequence (in order):**
- `EmptyDeckShuffleAction` — `new EmptyDeckShuffleAction()`
- `ShuffleAction` — `new ShuffleAction(AbstractDungeon.player.drawPile, false)`
- `DrawCardAction` — `new DrawCardAction(p, this.magicNumber)`

**Queue order:**
- L29: `this.addToBot(new EmptyDeckShuffleAction())`
- L30: `this.addToBot(new ShuffleAction(AbstractDungeon.player.drawPile, false))`
- L32: `this.addToBot(new DrawCardAction(p, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (AbstractDungeon.player.discardPile.size() > 0) {
            this.addToBot(new EmptyDeckShuffleAction());
            this.addToBot(new ShuffleAction(AbstractDungeon.player.drawPile, false));
        }
        this.addToBot(new DrawCardAction(p, this.magicNumber));
    }
```

</details>

## Defend_Blue
File: `cards\blue\Defend_Blue.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, 50)`
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`

**Queue order:**
- L29: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, 50))`
- L31: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (Settings.isDebug) {
            this.addToBot(new GainBlockAction((AbstractCreature)p, p, 50));
        } else {
            this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        }
    }
```

</details>

## Defend_Green
File: `cards\green\Defend_Green.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, 50)`
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`

**Queue order:**
- L29: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, 50))`
- L31: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (Settings.isDebug) {
            this.addToBot(new GainBlockAction((AbstractCreature)p, p, 50));
        } else {
            this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        }
    }
```

</details>

## Defend_Red
File: `cards\red\Defend_Red.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, 50)`
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`

**Queue order:**
- L29: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, 50))`
- L31: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (Settings.isDebug) {
            this.addToBot(new GainBlockAction((AbstractCreature)p, p, 50));
        } else {
            this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        }
    }
```

</details>

## Defend_Watcher
File: `cards\purple\Defend_Watcher.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, 50)`
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`

**Queue order:**
- L29: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, 50))`
- L31: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (Settings.isDebug) {
            this.addToBot(new GainBlockAction((AbstractCreature)p, p, 50));
        } else {
            this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        }
    }
```

</details>

## Deflect
File: `cards\green\Deflect.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`

**Queue order:**
- L26: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
    }
```

</details>

## Defragment
File: `cards\blue\Defragment.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new FocusPower(p, this.magicNumber), this.magicNumber)`
- `FocusPower` — `new FocusPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new FocusPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new FocusPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## DemonForm
File: `cards\red\DemonForm.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new DemonFormPower(p, this.magicNumber), this.magicNumber)`
- `DemonFormPower` — `new DemonFormPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new DemonFormPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new DemonFormPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## DeusExMachina
File: `cards\purple\DeusExMachina.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
    }
```

</details>

## DevaForm
File: `cards\purple\DevaForm.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new DevaPower(p), 1)`
- `DevaPower` — `new DevaPower(p)`

**Queue order:**
- L27: `this.addToBot(new ApplyPowerAction(p, p, new DevaPower(p), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new DevaPower(p), 1));
    }
```

</details>

## Devotion
File: `cards\purple\Devotion.java`

**Action sequence (in order):**
- `SFXAction` — `new SFXAction("HEAL_2", -0.4f, true)`
- `VFXAction` — `new VFXAction(new DevotionEffect(), doop)`
- `DevotionEffect` — `new DevotionEffect()`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new DevotionPower(p, this.magicNumber), this.magicNumber)`
- `DevotionPower` — `new DevotionPower(p, this.magicNumber)`

**Queue order:**
- L31: `this.addToBot(new SFXAction("HEAL_2", -0.4f, true))`
- L36: `this.addToBot(new VFXAction(new DevotionEffect(), doop))`
- L37: `this.addToBot(new ApplyPowerAction(p, p, new DevotionPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new SFXAction("HEAL_2", -0.4f, true));
        float doop = 0.8f;
        if (Settings.FAST_MODE) {
            doop = 0.0f;
        }
        this.addToBot(new VFXAction(new DevotionEffect(), doop));
        this.addToBot(new ApplyPowerAction(p, p, new DevotionPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## DieDieDie
File: `cards\green\DieDieDie.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new BorderLongFlashEffect(Color.LIGHT_GRAY))`
- `BorderLongFlashEffect` — `new BorderLongFlashEffect(Color.LIGHT_GRAY)`
- `VFXAction` — `new VFXAction(new DieDieDieEffect(), 0.7f)`
- `DieDieDieEffect` — `new DieDieDieEffect()`
- `ShakeScreenAction` — `new ShakeScreenAction(0.0f, ScreenShake.ShakeDur.MED, ScreenShake.ShakeIntensity.HIGH)`
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.SLASH_HORIZONTAL)`

**Queue order:**
- L35: `this.addToBot(new VFXAction(new BorderLongFlashEffect(Color.LIGHT_GRAY)))`
- L36: `this.addToBot(new VFXAction(new DieDieDieEffect(), 0.7f))`
- L37: `this.addToBot(new ShakeScreenAction(0.0f, ScreenShake.ShakeDur.MED, ScreenShake.ShakeIntensity.HIGH))`
- L38: `this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.SLASH_HORIZONTAL))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new VFXAction(new BorderLongFlashEffect(Color.LIGHT_GRAY)));
        this.addToBot(new VFXAction(new DieDieDieEffect(), 0.7f));
        this.addToBot(new ShakeScreenAction(0.0f, ScreenShake.ShakeDur.MED, ScreenShake.ShakeIntensity.HIGH));
        this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.SLASH_HORIZONTAL));
    }
```

</details>

## Disarm
File: `cards\red\Disarm.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(m, p, new StrengthPower(m, -this.magicNumber), -this.magicNumber)`
- `StrengthPower` — `new StrengthPower(m, -this.magicNumber)`

**Queue order:**
- L27: `this.addToBot(new ApplyPowerAction(m, p, new StrengthPower(m, -this.magicNumber), -this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(m, p, new StrengthPower(m, -this.magicNumber), -this.magicNumber));
    }
```

</details>

## Discipline
File: `cards\purple\Discipline.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new DEPRECATEDDisciplinePower(p))`
- `DEPRECATEDDisciplinePower` — `new DEPRECATEDDisciplinePower(p)`

**Queue order:**
- L25: `this.addToBot(new ApplyPowerAction(p, p, new DEPRECATEDDisciplinePower(p)))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new DEPRECATEDDisciplinePower(p)));
    }
```

</details>

## Discovery
File: `cards\colorless\Discovery.java`

**Action sequence (in order):**
- `DiscoveryAction` — `new DiscoveryAction()`

**Queue order:**
- L25: `this.addToBot(new DiscoveryAction())`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DiscoveryAction());
    }
```

</details>

## Distraction
File: `cards\green\Distraction.java`

**Action sequence (in order):**
- `MakeTempCardInHandAction` — `new MakeTempCardInHandAction(c, true)`

**Queue order:**
- L28: `this.addToBot(new MakeTempCardInHandAction(c, true))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        AbstractCard c = AbstractDungeon.returnTrulyRandomCardInCombat(AbstractCard.CardType.SKILL).makeCopy();
        c.setCostForTurn(-99);
        this.addToBot(new MakeTempCardInHandAction(c, true));
    }
```

</details>

## DodgeAndRoll
File: `cards\green\DodgeAndRoll.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new NextTurnBlockPower(p, this.block), this.block)`
- `NextTurnBlockPower` — `new NextTurnBlockPower(p, this.block)`

**Queue order:**
- L28: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L29: `this.addToBot(new ApplyPowerAction(p, p, new NextTurnBlockPower(p, this.block), this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.addToBot(new ApplyPowerAction(p, p, new NextTurnBlockPower(p, this.block), this.block));
    }
```

</details>

## DoomAndGloom
File: `cards\blue\DoomAndGloom.java`

**Action sequence (in order):**
- `SFXAction` — `new SFXAction("ATTACK_HEAVY")`
- `VFXAction` — `new VFXAction(p, new CleaveEffect(), 0.1f)`
- `CleaveEffect` — `new CleaveEffect()`
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE)`
- `ChannelAction` — `new ChannelAction(new Dark())`
- `Dark` — `new Dark()`

**Queue order:**
- L37: `this.addToBot(new SFXAction("ATTACK_HEAVY"))`
- L38: `this.addToBot(new VFXAction(p, new CleaveEffect(), 0.1f))`
- L39: `this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE))`
- L40: `this.addToBot(new ChannelAction(new Dark()))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new SFXAction("ATTACK_HEAVY"));
        this.addToBot(new VFXAction(p, new CleaveEffect(), 0.1f));
        this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE));
        this.addToBot(new ChannelAction(new Dark()));
    }
```

</details>

## Doppelganger
File: `cards\green\Doppelganger.java`

**Action sequence (in order):**
- `DoppelgangerAction` — `new DoppelgangerAction(p, this.upgraded, this.freeToPlayOnce, this.energyOnUse)`

**Queue order:**
- L25: `this.addToBot(new DoppelgangerAction(p, this.upgraded, this.freeToPlayOnce, this.energyOnUse))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DoppelgangerAction(p, this.upgraded, this.freeToPlayOnce, this.energyOnUse));
    }
```

</details>

## DoubleEnergy
File: `cards\blue\DoubleEnergy.java`

**Action sequence (in order):**
- `DoubleEnergyAction` — `new DoubleEnergyAction()`

**Queue order:**
- L25: `this.addToBot(new DoubleEnergyAction())`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DoubleEnergyAction());
    }
```

</details>

## DoubleTap
File: `cards\red\DoubleTap.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new DoubleTapPower(p, this.magicNumber), this.magicNumber)`
- `DoubleTapPower` — `new DoubleTapPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new DoubleTapPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new DoubleTapPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Doubt
File: `cards\curses\Doubt.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new WeakPower(AbstractDungeon.player, 1, true), 1)`
- `WeakPower` — `new WeakPower(AbstractDungeon.player, 1, true)`

**Queue order:**
- L29: `this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new WeakPower(AbstractDungeon.player, 1, true), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (this.dontTriggerOnUseCard) {
            this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new WeakPower(AbstractDungeon.player, 1, true), 1));
        }
    }
```

</details>

## DramaticEntrance
File: `cards\colorless\DramaticEntrance.java`

**Action sequence (in order):**
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.SLASH_DIAGONAL)`

**Queue order:**
- L30: `this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.SLASH_DIAGONAL))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.SLASH_DIAGONAL));
    }
```

</details>

## Dropkick
File: `cards\red\Dropkick.java`

**Action sequence (in order):**
- `DropkickAction` — `new DropkickAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn))`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L27: `this.addToBot(new DropkickAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn)))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DropkickAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn)));
    }
```

</details>

## DualWield
File: `cards\red\DualWield.java`

**Action sequence (in order):**
- `DualWieldAction` — `new DualWieldAction(p, this.magicNumber)`

**Queue order:**
- L25: `this.addToBot(new DualWieldAction(p, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DualWieldAction(p, this.magicNumber));
    }
```

</details>

## Dualcast
File: `cards\blue\Dualcast.java`

**Action sequence (in order):**
- `AnimateOrbAction` — `new AnimateOrbAction(1)`
- `EvokeWithoutRemovingOrbAction` — `new EvokeWithoutRemovingOrbAction(1)`
- `AnimateOrbAction` — `new AnimateOrbAction(1)`
- `EvokeOrbAction` — `new EvokeOrbAction(1)`

**Queue order:**
- L27: `this.addToBot(new AnimateOrbAction(1))`
- L28: `this.addToBot(new EvokeWithoutRemovingOrbAction(1))`
- L29: `this.addToBot(new AnimateOrbAction(1))`
- L30: `this.addToBot(new EvokeOrbAction(1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new AnimateOrbAction(1));
        this.addToBot(new EvokeWithoutRemovingOrbAction(1));
        this.addToBot(new AnimateOrbAction(1));
        this.addToBot(new EvokeOrbAction(1));
    }
```

</details>

## EchoForm
File: `cards\blue\EchoForm.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new EchoPower(p, 1), 1)`
- `EchoPower` — `new EchoPower(p, 1)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new EchoPower(p, 1), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new EchoPower(p, 1), 1));
    }
```

</details>

## Electrodynamics
File: `cards\blue\Electrodynamics.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new ElectroPower(p))`
- `ElectroPower` — `new ElectroPower(p)`
- `Lightning` — `new Lightning()`
- `ChannelAction` — `new ChannelAction(orb)`

**Queue order:**
- L29: `this.addToBot(new ApplyPowerAction(p, p, new ElectroPower(p)))`
- L33: `this.addToBot(new ChannelAction(orb))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (!p.hasPower(ID)) {
            this.addToBot(new ApplyPowerAction(p, p, new ElectroPower(p)));
        }
        for (int i = 0; i < this.magicNumber; ++i) {
            Lightning orb = new Lightning();
            this.addToBot(new ChannelAction(orb));
        }
    }
```

</details>

## EmptyBody
File: `cards\purple\EmptyBody.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `NotStanceCheckAction` — `new NotStanceCheckAction("Neutral", new VFXAction(new EmptyStanceEffect(p.hb.cX, p.hb.cY), 0.1f))`
- `VFXAction` — `new VFXAction(new EmptyStanceEffect(p.hb.cX, p.hb.cY), 0.1f)`
- `EmptyStanceEffect` — `new EmptyStanceEffect(p.hb.cX, p.hb.cY)`
- `ChangeStanceAction` — `new ChangeStanceAction("Neutral")`

**Queue order:**
- L31: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L32: `this.addToBot(new NotStanceCheckAction("Neutral", new VFXAction(new EmptyStanceEffect(p.hb.cX, p.hb.cY), 0.1f)))`
- L33: `this.addToBot(new ChangeStanceAction("Neutral"))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.addToBot(new NotStanceCheckAction("Neutral", new VFXAction(new EmptyStanceEffect(p.hb.cX, p.hb.cY), 0.1f)));
        this.addToBot(new ChangeStanceAction("Neutral"));
    }
```

</details>

## EmptyFist
File: `cards\purple\EmptyFist.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `NotStanceCheckAction` — `new NotStanceCheckAction("Neutral", new VFXAction(new EmptyStanceEffect(p.hb.cX, p.hb.cY), 0.1f))`
- `VFXAction` — `new VFXAction(new EmptyStanceEffect(p.hb.cX, p.hb.cY), 0.1f)`
- `EmptyStanceEffect` — `new EmptyStanceEffect(p.hb.cX, p.hb.cY)`
- `ChangeStanceAction` — `new ChangeStanceAction("Neutral")`

**Queue order:**
- L33: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT))`
- L34: `this.addToBot(new NotStanceCheckAction("Neutral", new VFXAction(new EmptyStanceEffect(p.hb.cX, p.hb.cY), 0.1f)))`
- L35: `this.addToBot(new ChangeStanceAction("Neutral"))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT));
        this.addToBot(new NotStanceCheckAction("Neutral", new VFXAction(new EmptyStanceEffect(p.hb.cX, p.hb.cY), 0.1f)));
        this.addToBot(new ChangeStanceAction("Neutral"));
    }
```

</details>

## EmptyMind
File: `cards\purple\EmptyMind.java`

**Action sequence (in order):**
- `DrawCardAction` — `new DrawCardAction(this.magicNumber)`
- `NotStanceCheckAction` — `new NotStanceCheckAction("Neutral", new VFXAction(new EmptyStanceEffect(p.hb.cX, p.hb.cY), 0.1f))`
- `VFXAction` — `new VFXAction(new EmptyStanceEffect(p.hb.cX, p.hb.cY), 0.1f)`
- `EmptyStanceEffect` — `new EmptyStanceEffect(p.hb.cX, p.hb.cY)`
- `ChangeStanceAction` — `new ChangeStanceAction("Neutral")`

**Queue order:**
- L31: `this.addToBot(new DrawCardAction(this.magicNumber))`
- L32: `this.addToBot(new NotStanceCheckAction("Neutral", new VFXAction(new EmptyStanceEffect(p.hb.cX, p.hb.cY), 0.1f)))`
- L33: `this.addToBot(new ChangeStanceAction("Neutral"))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DrawCardAction(this.magicNumber));
        this.addToBot(new NotStanceCheckAction("Neutral", new VFXAction(new EmptyStanceEffect(p.hb.cX, p.hb.cY), 0.1f)));
        this.addToBot(new ChangeStanceAction("Neutral"));
    }
```

</details>

## EndlessAgony
File: `cards\green\EndlessAgony.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SMASH)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L35: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SMASH))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SMASH));
    }
```

</details>

## Enlightenment
File: `cards\colorless\Enlightenment.java`

**Action sequence (in order):**
- `EnlightenmentAction` — `new EnlightenmentAction(this.upgraded)`

**Queue order:**
- L24: `this.addToBot(new EnlightenmentAction(this.upgraded))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new EnlightenmentAction(this.upgraded));
    }
```

</details>

## Entrench
File: `cards\red\Entrench.java`

**Action sequence (in order):**
- `DoubleYourBlockAction` — `new DoubleYourBlockAction(p)`

**Queue order:**
- L25: `this.addToBot(new DoubleYourBlockAction(p))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DoubleYourBlockAction(p));
    }
```

</details>

## Envenom
File: `cards\green\Envenom.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new EnvenomPower(p, 1), 1)`
- `EnvenomPower` — `new EnvenomPower(p, 1)`

**Queue order:**
- L25: `this.addToBot(new ApplyPowerAction(p, p, new EnvenomPower(p, 1), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new EnvenomPower(p, 1), 1));
    }
```

</details>

## Equilibrium
File: `cards\blue\Equilibrium.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new EquilibriumPower(p, this.magicNumber), this.magicNumber)`
- `EquilibriumPower` — `new EquilibriumPower(p, this.magicNumber)`

**Queue order:**
- L29: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L30: `this.addToBot(new ApplyPowerAction(p, p, new EquilibriumPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.addToBot(new ApplyPowerAction(p, p, new EquilibriumPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Eruption
File: `cards\purple\Eruption.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.FIRE)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ChangeStanceAction` — `new ChangeStanceAction("Wrath")`

**Queue order:**
- L29: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.FIRE))`
- L30: `this.addToBot(new ChangeStanceAction("Wrath"))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.FIRE));
        this.addToBot(new ChangeStanceAction("Wrath"));
    }
```

</details>

## EscapePlan
File: `cards\green\EscapePlan.java`

**Action sequence (in order):**
- `DrawCardAction` — `new DrawCardAction(1, new EscapePlanAction(this.block))`
- `EscapePlanAction` — `new EscapePlanAction(this.block)`

**Queue order:**
- L26: `this.addToBot(new DrawCardAction(1, new EscapePlanAction(this.block)))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DrawCardAction(1, new EscapePlanAction(this.block)));
    }
```

</details>

## Establishment
File: `cards\purple\Establishment.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new EstablishmentPower(p, this.magicNumber), this.magicNumber)`
- `EstablishmentPower` — `new EstablishmentPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new EstablishmentPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new EstablishmentPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Evaluate
File: `cards\purple\Evaluate.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `Insight` — `new Insight()`
- `MakeTempCardInDrawPileAction` — `new MakeTempCardInDrawPileAction(card, 1, true, true, false)`

**Queue order:**
- L29: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L31: `this.addToBot(new MakeTempCardInDrawPileAction(card, 1, true, true, false))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        Insight card = new Insight();
        this.addToBot(new MakeTempCardInDrawPileAction(card, 1, true, true, false));
    }
```

</details>

## Eviscerate
File: `cards\green\Eviscerate.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L48: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY))`
- L49: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY))`
- L50: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY));
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY));
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY));
    }
```

</details>

## Evolve
File: `cards\red\Evolve.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new EvolvePower(p, this.magicNumber), this.magicNumber)`
- `EvolvePower` — `new EvolvePower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new EvolvePower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new EvolvePower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Exhume
File: `cards\red\Exhume.java`

**Action sequence (in order):**
- `ExhumeAction` — `new ExhumeAction(false)`

**Queue order:**
- L25: `this.addToBot(new ExhumeAction(false))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ExhumeAction(false));
    }
```

</details>

## Expertise
File: `cards\green\Expertise.java`

**Action sequence (in order):**
- `ExpertiseAction` — `new ExpertiseAction(p, this.magicNumber)`

**Queue order:**
- L25: `this.addToBot(new ExpertiseAction(p, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ExpertiseAction(p, this.magicNumber));
    }
```

</details>

## Expunger
File: `cards\tempCards\Expunger.java`

**Action sequence (in order):**
- `ExpungeVFXAction` — `new ExpungeVFXAction(m)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L41: `this.addToBot(new ExpungeVFXAction(m))`
- L42: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        for (int i = 0; i < this.magicNumber; ++i) {
            this.addToBot(new ExpungeVFXAction(m));
            this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE));
        }
    }
```

</details>

## FTL
File: `cards\blue\FTL.java`

**Action sequence (in order):**
- `FTLAction` — `new FTLAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn), this.magicNumber)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L28: `this.addToBot(new FTLAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new FTLAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn), this.magicNumber));
        this.rawDescription = FTL.cardStrings.DESCRIPTION;
        this.initializeDescription();
    }
```

</details>

## FameAndFortune
File: `cards\optionCards\FameAndFortune.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.onChoseThisOption();
    }
```

</details>

## Fasting
File: `cards\purple\Fasting.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new FastingEffect(p.hb.cX, p.hb.cY, Color.CHARTREUSE))`
- `FastingEffect` — `new FastingEffect(p.hb.cX, p.hb.cY, Color.CHARTREUSE)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new StrengthPower(p, this.magicNumber), this.magicNumber)`
- `StrengthPower` — `new StrengthPower(p, this.magicNumber)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new DexterityPower(p, this.magicNumber), this.magicNumber)`
- `DexterityPower` — `new DexterityPower(p, this.magicNumber)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new EnergyDownPower(p, 1, true), 1)`
- `EnergyDownPower` — `new EnergyDownPower(p, 1, true)`

**Queue order:**
- L32: `this.addToBot(new VFXAction(new FastingEffect(p.hb.cX, p.hb.cY, Color.CHARTREUSE)))`
- L34: `this.addToBot(new ApplyPowerAction(p, p, new StrengthPower(p, this.magicNumber), this.magicNumber))`
- L35: `this.addToBot(new ApplyPowerAction(p, p, new DexterityPower(p, this.magicNumber), this.magicNumber))`
- L36: `this.addToBot(new ApplyPowerAction(p, p, new EnergyDownPower(p, 1, true), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (p != null) {
            this.addToBot(new VFXAction(new FastingEffect(p.hb.cX, p.hb.cY, Color.CHARTREUSE)));
        }
        this.addToBot(new ApplyPowerAction(p, p, new StrengthPower(p, this.magicNumber), this.magicNumber));
        this.addToBot(new ApplyPowerAction(p, p, new DexterityPower(p, this.magicNumber), this.magicNumber));
        this.addToBot(new ApplyPowerAction(p, p, new EnergyDownPower(p, 1, true), 1));
    }
```

</details>

## FearNoEvil
File: `cards\purple\FearNoEvil.java`

**Action sequence (in order):**
- `FearNoEvilAction` — `new FearNoEvilAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn))`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L26: `this.addToBot(new FearNoEvilAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn)))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new FearNoEvilAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn)));
    }
```

</details>

## Feed
File: `cards\red\Feed.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new BiteEffect(m.hb.cX, m.hb.cY - 40.0f * Settings.scale, Color.SCARLET.cpy()), 0.3f)`
- `BiteEffect` — `new BiteEffect(m.hb.cX, m.hb.cY - 40.0f * Settings.scale, Color.SCARLET.cpy())`
- `FeedAction` — `new FeedAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn), this.magicNumber)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L34: `this.addToBot(new VFXAction(new BiteEffect(m.hb.cX, m.hb.cY - 40.0f * Settings.scale, Color.SCARLET.cpy()), 0.3f))`
- L36: `this.addToBot(new FeedAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (m != null) {
            this.addToBot(new VFXAction(new BiteEffect(m.hb.cX, m.hb.cY - 40.0f * Settings.scale, Color.SCARLET.cpy()), 0.3f));
        }
        this.addToBot(new FeedAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn), this.magicNumber));
    }
```

</details>

## FeelNoPain
File: `cards\red\FeelNoPain.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new FeelNoPainPower(p, this.magicNumber), this.magicNumber)`
- `FeelNoPainPower` — `new FeelNoPainPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new FeelNoPainPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new FeelNoPainPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## FiendFire
File: `cards\red\FiendFire.java`

**Action sequence (in order):**
- `FiendFireAction` — `new FiendFireAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn))`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L27: `this.addToBot(new FiendFireAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn)))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new FiendFireAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn)));
    }
```

</details>

## Finesse
File: `cards\colorless\Finesse.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `DrawCardAction` — `new DrawCardAction(p, 1)`

**Queue order:**
- L27: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L28: `this.addToBot(new DrawCardAction(p, 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.addToBot(new DrawCardAction(p, 1));
    }
```

</details>

## Finisher
File: `cards\green\Finisher.java`

**Action sequence (in order):**
- `DamagePerAttackPlayedAction` — `new DamagePerAttackPlayedAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L28: `this.addToBot(new DamagePerAttackPlayedAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamagePerAttackPlayedAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL));
        this.rawDescription = Finisher.cardStrings.DESCRIPTION;
        this.initializeDescription();
    }
```

</details>

## FireBreathing
File: `cards\red\FireBreathing.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new FireBreathingPower(p, this.magicNumber), this.magicNumber)`
- `FireBreathingPower` — `new FireBreathingPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new FireBreathingPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new FireBreathingPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Fission
File: `cards\blue\Fission.java`

**Action sequence (in order):**
- `FissionAction` — `new FissionAction(this.upgraded)`

**Queue order:**
- L27: `this.addToBot(new FissionAction(this.upgraded))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new FissionAction(this.upgraded));
    }
```

</details>

## FlameBarrier
File: `cards\red\FlameBarrier.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(p, new FlameBarrierEffect(p.hb.cX, p.hb.cY), 0.1f)`
- `FlameBarrierEffect` — `new FlameBarrierEffect(p.hb.cX, p.hb.cY)`
- `VFXAction` — `new VFXAction(p, new FlameBarrierEffect(p.hb.cX, p.hb.cY), 0.5f)`
- `FlameBarrierEffect` — `new FlameBarrierEffect(p.hb.cX, p.hb.cY)`
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new FlameBarrierPower(p, this.magicNumber), this.magicNumber)`
- `FlameBarrierPower` — `new FlameBarrierPower(p, this.magicNumber)`

**Queue order:**
- L33: `this.addToBot(new VFXAction(p, new FlameBarrierEffect(p.hb.cX, p.hb.cY), 0.1f))`
- L35: `this.addToBot(new VFXAction(p, new FlameBarrierEffect(p.hb.cX, p.hb.cY), 0.5f))`
- L37: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L38: `this.addToBot(new ApplyPowerAction(p, p, new FlameBarrierPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (Settings.FAST_MODE) {
            this.addToBot(new VFXAction(p, new FlameBarrierEffect(p.hb.cX, p.hb.cY), 0.1f));
        } else {
            this.addToBot(new VFXAction(p, new FlameBarrierEffect(p.hb.cX, p.hb.cY), 0.5f));
        }
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.addToBot(new ApplyPowerAction(p, p, new FlameBarrierPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## FlashOfSteel
File: `cards\colorless\FlashOfSteel.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `DrawCardAction` — `new DrawCardAction(p, 1)`

**Queue order:**
- L29: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL))`
- L30: `this.addToBot(new DrawCardAction(p, 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL));
        this.addToBot(new DrawCardAction(p, 1));
    }
```

</details>

## Flechettes
File: `cards\green\Flechettes.java`

**Action sequence (in order):**
- `FlechetteAction` — `new FlechetteAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn))`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L27: `this.addToBot(new FlechetteAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn)))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new FlechetteAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn)));
        this.rawDescription = Flechettes.cardStrings.DESCRIPTION;
        this.initializeDescription();
    }
```

</details>

## Flex
File: `cards\red\Flex.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new StrengthPower(p, this.magicNumber), this.magicNumber)`
- `StrengthPower` — `new StrengthPower(p, this.magicNumber)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new LoseStrengthPower(p, this.magicNumber), this.magicNumber)`
- `LoseStrengthPower` — `new LoseStrengthPower(p, this.magicNumber)`

**Queue order:**
- L27: `this.addToBot(new ApplyPowerAction(p, p, new StrengthPower(p, this.magicNumber), this.magicNumber))`
- L28: `this.addToBot(new ApplyPowerAction(p, p, new LoseStrengthPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new StrengthPower(p, this.magicNumber), this.magicNumber));
        this.addToBot(new ApplyPowerAction(p, p, new LoseStrengthPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## FlurryOfBlows
File: `cards\purple\FlurryOfBlows.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L30: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
    }
```

</details>

## FlyingKnee
File: `cards\green\FlyingKnee.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new EnergizedPower(p, 1), 1)`
- `EnergizedPower` — `new EnergizedPower(p, 1)`

**Queue order:**
- L30: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L31: `this.addToBot(new ApplyPowerAction(p, p, new EnergizedPower(p, 1), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
        this.addToBot(new ApplyPowerAction(p, p, new EnergizedPower(p, 1), 1));
    }
```

</details>

## FlyingSleeves
File: `cards\purple\FlyingSleeves.java`

**Action sequence (in order):**
- `SFXAction` — `new SFXAction("ATTACK_WHIFF_2", 0.3f)`
- `SFXAction` — `new SFXAction("ATTACK_FAST", 0.2f)`
- `VFXAction` — `new VFXAction(new AnimatedSlashEffect(m.hb.cX, m.hb.cY - 30.0f * Settings.scale, 500.0f, 200.0f, 290.0f, 3.0f, Color.VIOLET, Color.PINK))`
- `AnimatedSlashEffect` — `new AnimatedSlashEffect(m.hb.cX, m.hb.cY - 30.0f * Settings.scale, 500.0f, 200.0f, 290.0f, 3.0f, Color.VIOLET, Color.PINK)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `SFXAction` — `new SFXAction("ATTACK_WHIFF_1", 0.2f)`
- `SFXAction` — `new SFXAction("ATTACK_FAST", 0.2f)`
- `VFXAction` — `new VFXAction(new AnimatedSlashEffect(m.hb.cX, m.hb.cY - 30.0f * Settings.scale, 500.0f, -200.0f, 250.0f, 3.0f, Color.VIOLET, Color.PINK))`
- `AnimatedSlashEffect` — `new AnimatedSlashEffect(m.hb.cX, m.hb.cY - 30.0f * Settings.scale, 500.0f, -200.0f, 250.0f, 3.0f, Color.VIOLET, Color.PINK)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L35: `this.addToBot(new SFXAction("ATTACK_WHIFF_2", 0.3f))`
- L36: `this.addToBot(new SFXAction("ATTACK_FAST", 0.2f))`
- L37: `this.addToBot(new VFXAction(new AnimatedSlashEffect(m.hb.cX, m.hb.cY - 30.0f * Settings.scale, 500.0f, 200.0f, 290.0f, 3.0f, Color.VIOLET, Color.PINK)))`
- L39: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE))`
- L41: `this.addToBot(new SFXAction("ATTACK_WHIFF_1", 0.2f))`
- L42: `this.addToBot(new SFXAction("ATTACK_FAST", 0.2f))`
- L43: `this.addToBot(new VFXAction(new AnimatedSlashEffect(m.hb.cX, m.hb.cY - 30.0f * Settings.scale, 500.0f, -200.0f, 250.0f, 3.0f, Color.VIOLET, Color.PINK)))`
- L45: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (m != null) {
            this.addToBot(new SFXAction("ATTACK_WHIFF_2", 0.3f));
            this.addToBot(new SFXAction("ATTACK_FAST", 0.2f));
            this.addToBot(new VFXAction(new AnimatedSlashEffect(m.hb.cX, m.hb.cY - 30.0f * Settings.scale, 500.0f, 200.0f, 290.0f, 3.0f, Color.VIOLET, Color.PINK)));
        }
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE));
        if (m != null) {
            this.addToBot(new SFXAction("ATTACK_WHIFF_1", 0.2f));
            this.addToBot(new SFXAction("ATTACK_FAST", 0.2f));
            this.addToBot(new VFXAction(new AnimatedSlashEffect(m.hb.cX, m.hb.cY - 30.0f * Settings.scale, 500.0f, -200.0f, 250.0f, 3.0f, Color.VIOLET, Color.PINK)));
        }
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE));
    }
```

</details>

## FollowUp
File: `cards\purple\FollowUp.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `FollowUpAction` — `new FollowUpAction()`

**Queue order:**
- L30: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L31: `this.addToBot(new FollowUpAction())`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
        this.addToBot(new FollowUpAction());
    }
```

</details>

## Footwork
File: `cards\green\Footwork.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new DexterityPower(p, this.magicNumber), this.magicNumber)`
- `DexterityPower` — `new DexterityPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new DexterityPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new DexterityPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## ForceField
File: `cards\blue\ForceField.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`

**Queue order:**
- L44: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
    }
```

</details>

## ForeignInfluence
File: `cards\purple\ForeignInfluence.java`

**Action sequence (in order):**
- `ForeignInfluenceAction` — `new ForeignInfluenceAction(this.upgraded)`

**Queue order:**
- L25: `this.addToBot(new ForeignInfluenceAction(this.upgraded))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ForeignInfluenceAction(this.upgraded));
    }
```

</details>

## Foresight
File: `cards\purple\Foresight.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new BorderFlashEffect(Color.VIOLET, true))`
- `BorderFlashEffect` — `new BorderFlashEffect(Color.VIOLET, true)`
- `VFXAction` — `new VFXAction(new GiantEyeEffect(p.hb.cX, p.hb.cY + 300.0f * Settings.scale, new Color(1.0f, 0.8f, 1.0f, 0.0f)))`
- `GiantEyeEffect` — `new GiantEyeEffect(p.hb.cX, p.hb.cY + 300.0f * Settings.scale, new Color(1.0f, 0.8f, 1.0f, 0.0f))`
- `Color` — `new Color(1.0f, 0.8f, 1.0f, 0.0f)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new ForesightPower(p, this.magicNumber), this.magicNumber)`
- `ForesightPower` — `new ForesightPower(p, this.magicNumber)`

**Queue order:**
- L32: `this.addToBot(new VFXAction(new BorderFlashEffect(Color.VIOLET, true)))`
- L33: `this.addToBot(new VFXAction(new GiantEyeEffect(p.hb.cX, p.hb.cY + 300.0f * Settings.scale, new Color(1.0f, 0.8f, 1.0f, 0.0f))))`
- L35: `this.addToBot(new ApplyPowerAction(p, p, new ForesightPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (p != null) {
            this.addToBot(new VFXAction(new BorderFlashEffect(Color.VIOLET, true)));
            this.addToBot(new VFXAction(new GiantEyeEffect(p.hb.cX, p.hb.cY + 300.0f * Settings.scale, new Color(1.0f, 0.8f, 1.0f, 0.0f))));
        }
        this.addToBot(new ApplyPowerAction(p, p, new ForesightPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Forethought
File: `cards\colorless\Forethought.java`

**Action sequence (in order):**
- `ForethoughtAction` — `new ForethoughtAction(this.upgraded)`

**Queue order:**
- L24: `this.addToBot(new ForethoughtAction(this.upgraded))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ForethoughtAction(this.upgraded));
    }
```

</details>

## Fusion
File: `cards\blue\Fusion.java`

**Action sequence (in order):**
- `Plasma` — `new Plasma()`
- `ChannelAction` — `new ChannelAction(orb)`

**Queue order:**
- L28: `this.addToBot(new ChannelAction(orb))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        for (int i = 0; i < this.magicNumber; ++i) {
            Plasma orb = new Plasma();
            this.addToBot(new ChannelAction(orb));
        }
    }
```

</details>

## GeneticAlgorithm
File: `cards\blue\GeneticAlgorithm.java`

**Action sequence (in order):**
- `IncreaseMiscAction` — `new IncreaseMiscAction(this.uuid, this.misc, this.magicNumber)`
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`

**Queue order:**
- L30: `this.addToBot(new IncreaseMiscAction(this.uuid, this.misc, this.magicNumber))`
- L31: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new IncreaseMiscAction(this.uuid, this.misc, this.magicNumber));
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
    }
```

</details>

## GhostlyArmor
File: `cards\red\GhostlyArmor.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`

**Queue order:**
- L28: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
    }
```

</details>

## Glacier
File: `cards\blue\Glacier.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `ChannelAction` — `new ChannelAction(new Frost())`
- `Frost` — `new Frost()`

**Queue order:**
- L31: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L33: `this.addToBot(new ChannelAction(new Frost()))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        for (int i = 0; i < this.magicNumber; ++i) {
            this.addToBot(new ChannelAction(new Frost()));
        }
    }
```

</details>

## GlassKnife
File: `cards\green\GlassKnife.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ModifyDamageAction` — `new ModifyDamageAction(this.uuid, -2)`

**Queue order:**
- L29: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL))`
- L30: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY))`
- L31: `this.addToBot(new ModifyDamageAction(this.uuid, -2))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL));
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY));
        this.addToBot(new ModifyDamageAction(this.uuid, -2));
    }
```

</details>

## GoForTheEyes
File: `cards\blue\GoForTheEyes.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ForTheEyesAction` — `new ForTheEyesAction(this.magicNumber, m)`

**Queue order:**
- L31: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL))`
- L32: `this.addToBot(new ForTheEyesAction(this.magicNumber, m))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL));
        this.addToBot(new ForTheEyesAction(this.magicNumber, m));
    }
```

</details>

## GoodInstincts
File: `cards\colorless\GoodInstincts.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`

**Queue order:**
- L26: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
    }
```

</details>

## GrandFinale
File: `cards\green\GrandFinale.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new GrandFinalEffect(), 0.7f)`
- `GrandFinalEffect` — `new GrandFinalEffect()`
- `VFXAction` — `new VFXAction(new GrandFinalEffect(), 1.0f)`
- `GrandFinalEffect` — `new GrandFinalEffect()`
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.SLASH_HEAVY)`

**Queue order:**
- L33: `this.addToBot(new VFXAction(new GrandFinalEffect(), 0.7f))`
- L35: `this.addToBot(new VFXAction(new GrandFinalEffect(), 1.0f))`
- L37: `this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.SLASH_HEAVY))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (Settings.FAST_MODE) {
            this.addToBot(new VFXAction(new GrandFinalEffect(), 0.7f));
        } else {
            this.addToBot(new VFXAction(new GrandFinalEffect(), 1.0f));
        }
        this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.SLASH_HEAVY));
    }
```

</details>

## Halt
File: `cards\purple\Halt.java`

**Action sequence (in order):**
- `HaltAction` — `new HaltAction(p, this.block, this.magicNumber)`

**Queue order:**
- L33: `this.addToBot(new HaltAction(p, this.block, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.applyPowers();
        this.addToBot(new HaltAction(p, this.block, this.magicNumber));
    }
```

</details>

## HandOfGreed
File: `cards\colorless\HandOfGreed.java`

**Action sequence (in order):**
- `GreedAction` — `new GreedAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn), this.magicNumber)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L27: `this.addToBot(new GreedAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GreedAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn), this.magicNumber));
    }
```

</details>

## Havoc
File: `cards\red\Havoc.java`

**Action sequence (in order):**
- `PlayTopCardAction` — `new PlayTopCardAction(AbstractDungeon.getCurrRoom().monsters.getRandomMonster(null, true, AbstractDungeon.cardRandomRng), true)`

**Queue order:**
- L25: `this.addToBot(new PlayTopCardAction(AbstractDungeon.getCurrRoom().monsters.getRandomMonster(null, true, AbstractDungeon.cardRandomRng), true))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new PlayTopCardAction(AbstractDungeon.getCurrRoom().monsters.getRandomMonster(null, true, AbstractDungeon.cardRandomRng), true));
    }
```

</details>

## Headbutt
File: `cards\red\Headbutt.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `DiscardPileToTopOfDeckAction` — `new DiscardPileToTopOfDeckAction(p)`

**Queue order:**
- L29: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L30: `this.addToBot(new DiscardPileToTopOfDeckAction(p))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
        this.addToBot(new DiscardPileToTopOfDeckAction(p));
    }
```

</details>

## Heatsinks
File: `cards\blue\Heatsinks.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new HeatsinkPower(p, this.magicNumber), this.magicNumber)`
- `HeatsinkPower` — `new HeatsinkPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new HeatsinkPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new HeatsinkPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## HeavyBlade
File: `cards\red\HeavyBlade.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new VerticalImpactEffect(m.hb.cX + m.hb.width / 4.0f, m.hb.cY - m.hb.height / 4.0f))`
- `VerticalImpactEffect` — `new VerticalImpactEffect(m.hb.cX + m.hb.width / 4.0f, m.hb.cY - m.hb.height / 4.0f)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L34: `this.addToBot(new VFXAction(new VerticalImpactEffect(m.hb.cX + m.hb.width / 4.0f, m.hb.cY - m.hb.height / 4.0f)))`
- L36: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (m != null) {
            this.addToBot(new VFXAction(new VerticalImpactEffect(m.hb.cX + m.hb.width / 4.0f, m.hb.cY - m.hb.height / 4.0f)));
        }
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
    }
```

</details>

## HeelHook
File: `cards\green\HeelHook.java`

**Action sequence (in order):**
- `HeelHookAction` — `new HeelHookAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn))`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L27: `this.addToBot(new HeelHookAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn)))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new HeelHookAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn)));
    }
```

</details>

## HelloWorld
File: `cards\blue\HelloWorld.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new HelloPower(p, 1), 1)`
- `HelloPower` — `new HelloPower(p, 1)`

**Queue order:**
- L25: `this.addToBot(new ApplyPowerAction(p, p, new HelloPower(p, 1), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new HelloPower(p, 1), 1));
    }
```

</details>

## Hemokinesis
File: `cards\red\Hemokinesis.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new HemokinesisEffect(p.hb.cX, p.hb.cY, m.hb.cX, m.hb.cY), 0.5f)`
- `HemokinesisEffect` — `new HemokinesisEffect(p.hb.cX, p.hb.cY, m.hb.cX, m.hb.cY)`
- `LoseHPAction` — `new LoseHPAction(p, p, this.magicNumber)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L33: `this.addToBot(new VFXAction(new HemokinesisEffect(p.hb.cX, p.hb.cY, m.hb.cX, m.hb.cY), 0.5f))`
- L35: `this.addToBot(new LoseHPAction(p, p, this.magicNumber))`
- L36: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (m != null) {
            this.addToBot(new VFXAction(new HemokinesisEffect(p.hb.cX, p.hb.cY, m.hb.cX, m.hb.cY), 0.5f));
        }
        this.addToBot(new LoseHPAction(p, p, this.magicNumber));
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
    }
```

</details>

## Hologram
File: `cards\blue\Hologram.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `BetterDiscardPileToHandAction` — `new BetterDiscardPileToHandAction(1)`

**Queue order:**
- L28: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L29: `this.addToBot(new BetterDiscardPileToHandAction(1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.addToBot(new BetterDiscardPileToHandAction(1));
    }
```

</details>

## Hyperbeam
File: `cards\blue\Hyperbeam.java`

**Action sequence (in order):**
- `SFXAction` — `new SFXAction("ATTACK_HEAVY")`
- `VFXAction` — `new VFXAction(p, new MindblastEffect(p.dialogX, p.dialogY, p.flipHorizontal), 0.1f)`
- `MindblastEffect` — `new MindblastEffect(p.dialogX, p.dialogY, p.flipHorizontal)`
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new FocusPower(p, -this.magicNumber), -this.magicNumber)`
- `FocusPower` — `new FocusPower(p, -this.magicNumber)`

**Queue order:**
- L34: `this.addToBot(new SFXAction("ATTACK_HEAVY"))`
- L35: `this.addToBot(new VFXAction(p, new MindblastEffect(p.dialogX, p.dialogY, p.flipHorizontal), 0.1f))`
- L36: `this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE))`
- L37: `this.addToBot(new ApplyPowerAction(p, p, new FocusPower(p, -this.magicNumber), -this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new SFXAction("ATTACK_HEAVY"));
        this.addToBot(new VFXAction(p, new MindblastEffect(p.dialogX, p.dialogY, p.flipHorizontal), 0.1f));
        this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE));
        this.addToBot(new ApplyPowerAction(p, p, new FocusPower(p, -this.magicNumber), -this.magicNumber));
    }
```

</details>

## Immolate
File: `cards\red\Immolate.java`

**Action sequence (in order):**
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.FIRE)`
- `MakeTempCardInDiscardAction` — `new MakeTempCardInDiscardAction((AbstractCard)new Burn(), 1)`
- `Burn` — `new Burn()`

**Queue order:**
- L31: `this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.FIRE))`
- L32: `this.addToBot(new MakeTempCardInDiscardAction((AbstractCard)new Burn(), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.FIRE));
        this.addToBot(new MakeTempCardInDiscardAction((AbstractCard)new Burn(), 1));
    }
```

</details>

## Impatience
File: `cards\colorless\Impatience.java`

**Action sequence (in order):**
- `ConditionalDrawAction` — `new ConditionalDrawAction(this.magicNumber, AbstractCard.CardType.ATTACK)`

**Queue order:**
- L26: `this.addToBot(new ConditionalDrawAction(this.magicNumber, AbstractCard.CardType.ATTACK))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ConditionalDrawAction(this.magicNumber, AbstractCard.CardType.ATTACK));
    }
```

</details>

## Impervious
File: `cards\red\Impervious.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`

**Queue order:**
- L27: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
    }
```

</details>

## Impulse
File: `cards\blue\Impulse.java`

**Action sequence (in order):**
- `ImpulseAction` — `new ImpulseAction()`

**Queue order:**
- L25: `this.addToBot(new ImpulseAction())`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ImpulseAction());
    }
```

</details>

## Indignation
File: `cards\purple\Indignation.java`

**Action sequence (in order):**
- `IndignationAction` — `new IndignationAction(this.magicNumber)`

**Queue order:**
- L25: `this.addToBot(new IndignationAction(this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new IndignationAction(this.magicNumber));
    }
```

</details>

## InfernalBlade
File: `cards\red\InfernalBlade.java`

**Action sequence (in order):**
- `MakeTempCardInHandAction` — `new MakeTempCardInHandAction(c, true)`

**Queue order:**
- L28: `this.addToBot(new MakeTempCardInHandAction(c, true))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        AbstractCard c = AbstractDungeon.returnTrulyRandomCardInCombat(AbstractCard.CardType.ATTACK).makeCopy();
        c.setCostForTurn(0);
        this.addToBot(new MakeTempCardInHandAction(c, true));
    }
```

</details>

## InfiniteBlades
File: `cards\green\InfiniteBlades.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new InfiniteBladesPower(p, 1), 1)`
- `InfiniteBladesPower` — `new InfiniteBladesPower(p, 1)`

**Queue order:**
- L27: `this.addToBot(new ApplyPowerAction(p, p, new InfiniteBladesPower(p, 1), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new InfiniteBladesPower(p, 1), 1));
    }
```

</details>

## Inflame
File: `cards\red\Inflame.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(p, new InflameEffect(p), 1.0f)`
- `InflameEffect` — `new InflameEffect(p)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new StrengthPower(p, this.magicNumber), this.magicNumber)`
- `StrengthPower` — `new StrengthPower(p, this.magicNumber)`

**Queue order:**
- L28: `this.addToBot(new VFXAction(p, new InflameEffect(p), 1.0f))`
- L29: `this.addToBot(new ApplyPowerAction(p, p, new StrengthPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new VFXAction(p, new InflameEffect(p), 1.0f));
        this.addToBot(new ApplyPowerAction(p, p, new StrengthPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Injury
File: `cards\curses\Injury.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
    }
```

</details>

## InnerPeace
File: `cards\purple\InnerPeace.java`

**Action sequence (in order):**
- `InnerPeaceAction` — `new InnerPeaceAction(this.magicNumber)`

**Queue order:**
- L25: `this.addToBot(new InnerPeaceAction(this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new InnerPeaceAction(this.magicNumber));
    }
```

</details>

## Insight
File: `cards\tempCards\Insight.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new LightBulbEffect(p.hb))`
- `LightBulbEffect` — `new LightBulbEffect(p.hb)`
- `VFXAction` — `new VFXAction(new LightBulbEffect(p.hb), 0.2f)`
- `LightBulbEffect` — `new LightBulbEffect(p.hb)`
- `DrawCardAction` — `new DrawCardAction(p, this.magicNumber)`

**Queue order:**
- L31: `this.addToBot(new VFXAction(new LightBulbEffect(p.hb)))`
- L33: `this.addToBot(new VFXAction(new LightBulbEffect(p.hb), 0.2f))`
- L35: `this.addToBot(new DrawCardAction(p, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (Settings.FAST_MODE) {
            this.addToBot(new VFXAction(new LightBulbEffect(p.hb)));
        } else {
            this.addToBot(new VFXAction(new LightBulbEffect(p.hb), 0.2f));
        }
        this.addToBot(new DrawCardAction(p, this.magicNumber));
    }
```

</details>

## Intimidate
File: `cards\red\Intimidate.java`

**Action sequence (in order):**
- `SFXAction` — `new SFXAction("INTIMIDATE")`
- `VFXAction` — `new VFXAction(p, new IntimidateEffect(AbstractDungeon.player.hb.cX, AbstractDungeon.player.hb.cY), 1.0f)`
- `IntimidateEffect` — `new IntimidateEffect(AbstractDungeon.player.hb.cX, AbstractDungeon.player.hb.cY)`
- `ApplyPowerAction` — `new ApplyPowerAction(mo, p, new WeakPower(mo, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE)`
- `WeakPower` — `new WeakPower(mo, this.magicNumber, false)`

**Queue order:**
- L32: `this.addToBot(new SFXAction("INTIMIDATE"))`
- L33: `this.addToBot(new VFXAction(p, new IntimidateEffect(AbstractDungeon.player.hb.cX, AbstractDungeon.player.hb.cY), 1.0f))`
- L35: `this.addToBot(new ApplyPowerAction(mo, p, new WeakPower(mo, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new SFXAction("INTIMIDATE"));
        this.addToBot(new VFXAction(p, new IntimidateEffect(AbstractDungeon.player.hb.cX, AbstractDungeon.player.hb.cY), 1.0f));
        for (AbstractMonster mo : AbstractDungeon.getCurrRoom().monsters.monsters) {
            this.addToBot(new ApplyPowerAction(mo, p, new WeakPower(mo, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE));
        }
    }
```

</details>

## IronWave
File: `cards\red\IronWave.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `WaitAction` — `new WaitAction(0.1f)`
- `VFXAction` — `new VFXAction(new IronWaveEffect(p.hb.cX, p.hb.cY, m.hb.cX), 0.5f)`
- `IronWaveEffect` — `new IronWaveEffect(p.hb.cX, p.hb.cY, m.hb.cX)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_VERTICAL)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L33: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L34: `this.addToBot(new WaitAction(0.1f))`
- L36: `this.addToBot(new VFXAction(new IronWaveEffect(p.hb.cX, p.hb.cY, m.hb.cX), 0.5f))`
- L38: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_VERTICAL))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.addToBot(new WaitAction(0.1f));
        if (p != null && m != null) {
            this.addToBot(new VFXAction(new IronWaveEffect(p.hb.cX, p.hb.cY, m.hb.cX), 0.5f));
        }
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_VERTICAL));
    }
```

</details>

## JAX
File: `cards\colorless\JAX.java`

**Action sequence (in order):**
- `LoseHPAction` — `new LoseHPAction(p, p, 3)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new StrengthPower(p, this.magicNumber), this.magicNumber)`
- `StrengthPower` — `new StrengthPower(p, this.magicNumber)`

**Queue order:**
- L27: `this.addToBot(new LoseHPAction(p, p, 3))`
- L28: `this.addToBot(new ApplyPowerAction(p, p, new StrengthPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new LoseHPAction(p, p, 3));
        this.addToBot(new ApplyPowerAction(p, p, new StrengthPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## JackOfAllTrades
File: `cards\colorless\JackOfAllTrades.java`

**Action sequence (in order):**
- `MakeTempCardInHandAction` — `new MakeTempCardInHandAction(c, 1)`
- `MakeTempCardInHandAction` — `new MakeTempCardInHandAction(c, 1)`

**Queue order:**
- L28: `this.addToBot(new MakeTempCardInHandAction(c, 1))`
- L31: `this.addToBot(new MakeTempCardInHandAction(c, 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        AbstractCard c = AbstractDungeon.returnTrulyRandomColorlessCardInCombat(AbstractDungeon.cardRandomRng).makeCopy();
        this.addToBot(new MakeTempCardInHandAction(c, 1));
        if (this.upgraded) {
            c = AbstractDungeon.returnTrulyRandomColorlessCardInCombat(AbstractDungeon.cardRandomRng).makeCopy();
            this.addToBot(new MakeTempCardInHandAction(c, 1));
        }
    }
```

</details>

## Judgement
File: `cards\purple\Judgement.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new WeightyImpactEffect(m.hb.cX, m.hb.cY, Color.GOLD.cpy()))`
- `WeightyImpactEffect` — `new WeightyImpactEffect(m.hb.cX, m.hb.cY, Color.GOLD.cpy())`
- `WaitAction` — `new WaitAction(0.8f)`
- `VFXAction` — `new VFXAction(new GiantTextEffect(m.hb.cX, m.hb.cY))`
- `GiantTextEffect` — `new GiantTextEffect(m.hb.cX, m.hb.cY)`
- `JudgementAction` — `new JudgementAction(m, this.magicNumber)`

**Queue order:**
- L32: `this.addToBot(new VFXAction(new WeightyImpactEffect(m.hb.cX, m.hb.cY, Color.GOLD.cpy())))`
- L33: `this.addToBot(new WaitAction(0.8f))`
- L34: `this.addToBot(new VFXAction(new GiantTextEffect(m.hb.cX, m.hb.cY)))`
- L36: `this.addToBot(new JudgementAction(m, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (m != null) {
            this.addToBot(new VFXAction(new WeightyImpactEffect(m.hb.cX, m.hb.cY, Color.GOLD.cpy())));
            this.addToBot(new WaitAction(0.8f));
            this.addToBot(new VFXAction(new GiantTextEffect(m.hb.cX, m.hb.cY)));
        }
        this.addToBot(new JudgementAction(m, this.magicNumber));
    }
```

</details>

## Juggernaut
File: `cards\red\Juggernaut.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new JuggernautPower(p, this.magicNumber), this.magicNumber)`
- `JuggernautPower` — `new JuggernautPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new JuggernautPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new JuggernautPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## JustLucky
File: `cards\purple\JustLucky.java`

**Action sequence (in order):**
- `ScryAction` — `new ScryAction(this.magicNumber)`
- `VFXAction` — `new VFXAction(new FlickCoinEffect(p.hb.cX, p.hb.cY, m.hb.cX, m.hb.cY), 0.3f)`
- `FlickCoinEffect` — `new FlickCoinEffect(p.hb.cX, p.hb.cY, m.hb.cX, m.hb.cY)`
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L34: `this.addToBot(new ScryAction(this.magicNumber))`
- L35: `this.addToBot(new VFXAction(new FlickCoinEffect(p.hb.cX, p.hb.cY, m.hb.cX, m.hb.cY), 0.3f))`
- L36: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L37: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ScryAction(this.magicNumber));
        this.addToBot(new VFXAction(new FlickCoinEffect(p.hb.cX, p.hb.cY, m.hb.cX, m.hb.cY), 0.3f));
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE));
    }
```

</details>

## Leap
File: `cards\blue\Leap.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`

**Queue order:**
- L26: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
    }
```

</details>

## LegSweep
File: `cards\green\LegSweep.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber)`
- `WeakPower` — `new WeakPower(m, this.magicNumber, false)`
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`

**Queue order:**
- L29: `this.addToBot(new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber))`
- L30: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber));
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
    }
```

</details>

## LessonLearned
File: `cards\purple\LessonLearned.java`

**Action sequence (in order):**
- `LessonLearnedAction` — `new LessonLearnedAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn))`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L28: `this.addToBot(new LessonLearnedAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn)))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new LessonLearnedAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn)));
    }
```

</details>

## LikeWater
File: `cards\purple\LikeWater.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new LikeWaterPower(p, this.magicNumber), this.magicNumber)`
- `LikeWaterPower` — `new LikeWaterPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new LikeWaterPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new LikeWaterPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## LimitBreak
File: `cards\red\LimitBreak.java`

**Action sequence (in order):**
- `LimitBreakAction` — `new LimitBreakAction()`

**Queue order:**
- L25: `this.addToBot(new LimitBreakAction())`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new LimitBreakAction());
    }
```

</details>

## LiveForever
File: `cards\optionCards\LiveForever.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.onChoseThisOption();
    }
```

</details>

## LockOn
File: `cards\blue\LockOn.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ApplyPowerAction` — `new ApplyPowerAction(m, p, new LockOnPower(m, this.magicNumber), this.magicNumber)`
- `LockOnPower` — `new LockOnPower(m, this.magicNumber)`

**Queue order:**
- L31: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT))`
- L32: `this.addToBot(new ApplyPowerAction(m, p, new LockOnPower(m, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT));
        this.addToBot(new ApplyPowerAction(m, p, new LockOnPower(m, this.magicNumber), this.magicNumber));
    }
```

</details>

## Loop
File: `cards\blue\Loop.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new LoopPower(p, this.magicNumber), this.magicNumber)`
- `LoopPower` — `new LoopPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new LoopPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new LoopPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## MachineLearning
File: `cards\blue\MachineLearning.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new DrawPower(p, this.magicNumber), this.magicNumber)`
- `DrawPower` — `new DrawPower(p, this.magicNumber)`

**Queue order:**
- L27: `this.addToBot(new ApplyPowerAction(p, p, new DrawPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new DrawPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Madness
File: `cards\colorless\Madness.java`

**Action sequence (in order):**
- `MadnessAction` — `new MadnessAction()`

**Queue order:**
- L25: `this.addToBot(new MadnessAction())`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new MadnessAction());
    }
```

</details>

## Magnetism
File: `cards\colorless\Magnetism.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new MagnetismPower(p, this.magicNumber), this.magicNumber)`
- `MagnetismPower` — `new MagnetismPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new MagnetismPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new MagnetismPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Malaise
File: `cards\green\Malaise.java`

**Action sequence (in order):**
- `MalaiseAction` — `new MalaiseAction(p, m, this.upgraded, this.freeToPlayOnce, this.energyOnUse)`

**Queue order:**
- L25: `this.addToBot(new MalaiseAction(p, m, this.upgraded, this.freeToPlayOnce, this.energyOnUse))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new MalaiseAction(p, m, this.upgraded, this.freeToPlayOnce, this.energyOnUse));
    }
```

</details>

## MasterOfStrategy
File: `cards\colorless\MasterOfStrategy.java`

**Action sequence (in order):**
- `DrawCardAction` — `new DrawCardAction(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new DrawCardAction(p, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DrawCardAction(p, this.magicNumber));
    }
```

</details>

## MasterReality
File: `cards\purple\MasterReality.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new MasterRealityPower(p))`
- `MasterRealityPower` — `new MasterRealityPower(p)`

**Queue order:**
- L25: `this.addToBot(new ApplyPowerAction(p, p, new MasterRealityPower(p)))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new MasterRealityPower(p)));
    }
```

</details>

## MasterfulStab
File: `cards\green\MasterfulStab.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L33: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY));
    }
```

</details>

## Mayhem
File: `cards\colorless\Mayhem.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new MayhemPower(p, this.magicNumber), this.magicNumber)`
- `MayhemPower` — `new MayhemPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new MayhemPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new MayhemPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Meditate
File: `cards\purple\Meditate.java`

**Action sequence (in order):**
- `MeditateAction` — `new MeditateAction(this.magicNumber)`
- `ChangeStanceAction` — `new ChangeStanceAction("Calm")`
- `PressEndTurnButtonAction` — `new PressEndTurnButtonAction()`

**Queue order:**
- L27: `this.addToBot(new MeditateAction(this.magicNumber))`
- L28: `this.addToBot(new ChangeStanceAction("Calm"))`
- L29: `this.addToBot(new PressEndTurnButtonAction())`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new MeditateAction(this.magicNumber));
        this.addToBot(new ChangeStanceAction("Calm"));
        this.addToBot(new PressEndTurnButtonAction());
    }
```

</details>

## Melter
File: `cards\blue\Melter.java`

**Action sequence (in order):**
- `RemoveAllBlockAction` — `new RemoveAllBlockAction(m, p)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.FIRE)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L29: `this.addToBot(new RemoveAllBlockAction(m, p))`
- L30: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.FIRE))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new RemoveAllBlockAction(m, p));
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.FIRE));
    }
```

</details>

## MentalFortress
File: `cards\purple\MentalFortress.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new MentalFortressPower(p, this.magicNumber), this.magicNumber)`
- `MentalFortressPower` — `new MentalFortressPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new MentalFortressPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new MentalFortressPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Metallicize
File: `cards\red\Metallicize.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new MetallicizePower(p, this.magicNumber), this.magicNumber)`
- `MetallicizePower` — `new MetallicizePower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new MetallicizePower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new MetallicizePower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Metamorphosis
File: `cards\colorless\Metamorphosis.java`

**Action sequence (in order):**
- `MakeTempCardInDrawPileAction` — `new MakeTempCardInDrawPileAction(card, 1, true, true)`

**Queue order:**
- L34: `this.addToBot(new MakeTempCardInDrawPileAction(card, 1, true, true))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        for (int i = 0; i < this.magicNumber; ++i) {
            AbstractCard card = AbstractDungeon.returnTrulyRandomCardInCombat(AbstractCard.CardType.ATTACK).makeCopy();
            if (card.cost > 0) {
                card.cost = 0;
                card.costForTurn = 0;
                card.isCostModified = true;
            }
            this.addToBot(new MakeTempCardInDrawPileAction(card, 1, true, true));
        }
    }
```

</details>

## MeteorStrike
File: `cards\blue\MeteorStrike.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new WeightyImpactEffect(m.hb.cX, m.hb.cY))`
- `WeightyImpactEffect` — `new WeightyImpactEffect(m.hb.cX, m.hb.cY)`
- `WaitAction` — `new WaitAction(0.8f)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ChannelAction` — `new ChannelAction(new Plasma())`
- `Plasma` — `new Plasma()`

**Queue order:**
- L36: `this.addToBot(new VFXAction(new WeightyImpactEffect(m.hb.cX, m.hb.cY)))`
- L38: `this.addToBot(new WaitAction(0.8f))`
- L39: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE))`
- L41: `this.addToBot(new ChannelAction(new Plasma()))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (m != null) {
            this.addToBot(new VFXAction(new WeightyImpactEffect(m.hb.cX, m.hb.cY)));
        }
        this.addToBot(new WaitAction(0.8f));
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE));
        for (int i = 0; i < this.magicNumber; ++i) {
            this.addToBot(new ChannelAction(new Plasma()));
        }
    }
```

</details>

## MindBlast
File: `cards\colorless\MindBlast.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new MindblastEffect(p.dialogX, p.dialogY, p.flipHorizontal))`
- `MindblastEffect` — `new MindblastEffect(p.dialogX, p.dialogY, p.flipHorizontal)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, DamageInfo.DamageType.NORMAL), AbstractGameAction.AttackEffect.NONE)`
- `DamageInfo` — `new DamageInfo(p, this.damage, DamageInfo.DamageType.NORMAL)`

**Queue order:**
- L32: `this.addToBot(new VFXAction(new MindblastEffect(p.dialogX, p.dialogY, p.flipHorizontal)))`
- L33: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, DamageInfo.DamageType.NORMAL), AbstractGameAction.AttackEffect.NONE))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new VFXAction(new MindblastEffect(p.dialogX, p.dialogY, p.flipHorizontal)));
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, DamageInfo.DamageType.NORMAL), AbstractGameAction.AttackEffect.NONE));
        this.rawDescription = MindBlast.cardStrings.DESCRIPTION;
        this.initializeDescription();
    }
```

</details>

## Miracle
File: `cards\tempCards\Miracle.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new BorderFlashEffect(Color.GOLDENROD, true))`
- `BorderFlashEffect` — `new BorderFlashEffect(Color.GOLDENROD, true)`
- `VFXAction` — `new VFXAction(new MiracleEffect())`
- `MiracleEffect` — `new MiracleEffect()`
- `GainEnergyAction` — `new GainEnergyAction(2)`
- `GainEnergyAction` — `new GainEnergyAction(1)`

**Queue order:**
- L32: `this.addToBot(new VFXAction(new BorderFlashEffect(Color.GOLDENROD, true)))`
- L34: `this.addToBot(new VFXAction(new MiracleEffect()))`
- L36: `this.addToBot(new GainEnergyAction(2))`
- L38: `this.addToBot(new GainEnergyAction(1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (!Settings.DISABLE_EFFECTS) {
            this.addToBot(new VFXAction(new BorderFlashEffect(Color.GOLDENROD, true)));
        }
        this.addToBot(new VFXAction(new MiracleEffect()));
        if (this.upgraded) {
            this.addToBot(new GainEnergyAction(2));
        } else {
            this.addToBot(new GainEnergyAction(1));
        }
    }
```

</details>

## MultiCast
File: `cards\blue\MultiCast.java`

**Action sequence (in order):**
- `MulticastAction` — `new MulticastAction(p, this.energyOnUse, this.upgraded, this.freeToPlayOnce)`

**Queue order:**
- L25: `this.addToBot(new MulticastAction(p, this.energyOnUse, this.upgraded, this.freeToPlayOnce))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new MulticastAction(p, this.energyOnUse, this.upgraded, this.freeToPlayOnce));
    }
```

</details>

## Necronomicurse
File: `cards\curses\Necronomicurse.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
    }
```

</details>

## Neutralize
File: `cards\green\Neutralize.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ApplyPowerAction` — `new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber)`
- `WeakPower` — `new WeakPower(m, this.magicNumber, false)`

**Queue order:**
- L31: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L32: `this.addToBot(new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
        this.addToBot(new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber));
    }
```

</details>

## Nightmare
File: `cards\green\Nightmare.java`

**Action sequence (in order):**
- `NightmareAction` — `new NightmareAction(p, p, this.magicNumber)`

**Queue order:**
- L27: `this.addToBot(new NightmareAction(p, p, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new NightmareAction(p, p, this.magicNumber));
    }
```

</details>

## Nirvana
File: `cards\purple\Nirvana.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new NirvanaPower(p, this.magicNumber), this.magicNumber)`
- `NirvanaPower` — `new NirvanaPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new NirvanaPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new NirvanaPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Normality
File: `cards\curses\Normality.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
    }
```

</details>

## NoxiousFumes
File: `cards\green\NoxiousFumes.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new NoxiousFumesPower(p, this.magicNumber), this.magicNumber)`
- `NoxiousFumesPower` — `new NoxiousFumesPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new NoxiousFumesPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new NoxiousFumesPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Offering
File: `cards\red\Offering.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new OfferingEffect(), 0.1f)`
- `OfferingEffect` — `new OfferingEffect()`
- `VFXAction` — `new VFXAction(new OfferingEffect(), 0.5f)`
- `OfferingEffect` — `new OfferingEffect()`
- `LoseHPAction` — `new LoseHPAction(p, p, 6)`
- `GainEnergyAction` — `new GainEnergyAction(2)`
- `DrawCardAction` — `new DrawCardAction(p, this.magicNumber)`

**Queue order:**
- L32: `this.addToBot(new VFXAction(new OfferingEffect(), 0.1f))`
- L34: `this.addToBot(new VFXAction(new OfferingEffect(), 0.5f))`
- L36: `this.addToBot(new LoseHPAction(p, p, 6))`
- L37: `this.addToBot(new GainEnergyAction(2))`
- L38: `this.addToBot(new DrawCardAction(p, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (Settings.FAST_MODE) {
            this.addToBot(new VFXAction(new OfferingEffect(), 0.1f));
        } else {
            this.addToBot(new VFXAction(new OfferingEffect(), 0.5f));
        }
        this.addToBot(new LoseHPAction(p, p, 6));
        this.addToBot(new GainEnergyAction(2));
        this.addToBot(new DrawCardAction(p, this.magicNumber));
    }
```

</details>

## Omega
File: `cards\tempCards\Omega.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new OmegaPower(p, this.magicNumber), this.magicNumber)`
- `OmegaPower` — `new OmegaPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new OmegaPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new OmegaPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Omniscience
File: `cards\purple\Omniscience.java`

**Action sequence (in order):**
- `OmniscienceAction` — `new OmniscienceAction(this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new OmniscienceAction(this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new OmniscienceAction(this.magicNumber));
    }
```

</details>

## Outmaneuver
File: `cards\green\Outmaneuver.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new EnergizedPower(p, 2), 2)`
- `EnergizedPower` — `new EnergizedPower(p, 2)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new EnergizedPower(p, 3), 3)`
- `EnergizedPower` — `new EnergizedPower(p, 3)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new EnergizedPower(p, 2), 2))`
- L28: `this.addToBot(new ApplyPowerAction(p, p, new EnergizedPower(p, 3), 3))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (!this.upgraded) {
            this.addToBot(new ApplyPowerAction(p, p, new EnergizedPower(p, 2), 2));
        } else {
            this.addToBot(new ApplyPowerAction(p, p, new EnergizedPower(p, 3), 3));
        }
    }
```

</details>

## Overclock
File: `cards\blue\Overclock.java`

**Action sequence (in order):**
- `DrawCardAction` — `new DrawCardAction(p, this.magicNumber, false)`
- `MakeTempCardInDiscardAction` — `new MakeTempCardInDiscardAction((AbstractCard)new Burn(), 1)`
- `Burn` — `new Burn()`

**Queue order:**
- L28: `this.addToBot(new DrawCardAction(p, this.magicNumber, false))`
- L29: `this.addToBot(new MakeTempCardInDiscardAction((AbstractCard)new Burn(), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DrawCardAction(p, this.magicNumber, false));
        this.addToBot(new MakeTempCardInDiscardAction((AbstractCard)new Burn(), 1));
    }
```

</details>

## Pain
File: `cards\curses\Pain.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
    }
```

</details>

## Panacea
File: `cards\colorless\Panacea.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new ArtifactPower(p, this.magicNumber), this.magicNumber)`
- `ArtifactPower` — `new ArtifactPower(p, this.magicNumber)`

**Queue order:**
- L27: `this.addToBot(new ApplyPowerAction(p, p, new ArtifactPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new ArtifactPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Panache
File: `cards\colorless\Panache.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new PanachePower(p, this.magicNumber), this.magicNumber)`
- `PanachePower` — `new PanachePower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new PanachePower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new PanachePower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## PanicButton
File: `cards\colorless\PanicButton.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new NoBlockPower(p, this.magicNumber, false), this.magicNumber)`
- `NoBlockPower` — `new NoBlockPower(p, this.magicNumber, false)`

**Queue order:**
- L30: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L31: `this.addToBot(new ApplyPowerAction(p, p, new NoBlockPower(p, this.magicNumber, false), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.addToBot(new ApplyPowerAction(p, p, new NoBlockPower(p, this.magicNumber, false), this.magicNumber));
    }
```

</details>

## Parasite
File: `cards\curses\Parasite.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
    }
```

</details>

## PerfectedStrike
File: `cards\red\PerfectedStrike.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L52: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL));
    }
```

</details>

## Perseverance
File: `cards\purple\Perseverance.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`

**Queue order:**
- L33: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
    }
```

</details>

## PhantasmalKiller
File: `cards\green\PhantasmalKiller.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new PhantasmalPower(p, 1), 1)`
- `PhantasmalPower` — `new PhantasmalPower(p, 1)`

**Queue order:**
- L25: `this.addToBot(new ApplyPowerAction(p, p, new PhantasmalPower(p, 1), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new PhantasmalPower(p, 1), 1));
    }
```

</details>

## PiercingWail
File: `cards\green\PiercingWail.java`

**Action sequence (in order):**
- `SFXAction` — `new SFXAction("ATTACK_PIERCING_WAIL")`
- `VFXAction` — `new VFXAction(p, new ShockWaveEffect(p.hb.cX, p.hb.cY, Settings.GREEN_TEXT_COLOR, ShockWaveEffect.ShockWaveType.CHAOTIC), 0.3f)`
- `ShockWaveEffect` — `new ShockWaveEffect(p.hb.cX, p.hb.cY, Settings.GREEN_TEXT_COLOR, ShockWaveEffect.ShockWaveType.CHAOTIC)`
- `VFXAction` — `new VFXAction(p, new ShockWaveEffect(p.hb.cX, p.hb.cY, Settings.GREEN_TEXT_COLOR, ShockWaveEffect.ShockWaveType.CHAOTIC), 1.5f)`
- `ShockWaveEffect` — `new ShockWaveEffect(p.hb.cX, p.hb.cY, Settings.GREEN_TEXT_COLOR, ShockWaveEffect.ShockWaveType.CHAOTIC)`
- `ApplyPowerAction` — `new ApplyPowerAction(mo, p, new StrengthPower(mo, -this.magicNumber), -this.magicNumber, true, AbstractGameAction.AttackEffect.NONE)`
- `StrengthPower` — `new StrengthPower(mo, -this.magicNumber)`
- `ApplyPowerAction` — `new ApplyPowerAction(mo, p, new GainStrengthPower(mo, this.magicNumber), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE)`
- `GainStrengthPower` — `new GainStrengthPower(mo, this.magicNumber)`

**Queue order:**
- L34: `this.addToBot(new SFXAction("ATTACK_PIERCING_WAIL"))`
- L36: `this.addToBot(new VFXAction(p, new ShockWaveEffect(p.hb.cX, p.hb.cY, Settings.GREEN_TEXT_COLOR, ShockWaveEffect.ShockWaveType.CHAOTIC), 0.3f))`
- L38: `this.addToBot(new VFXAction(p, new ShockWaveEffect(p.hb.cX, p.hb.cY, Settings.GREEN_TEXT_COLOR, ShockWaveEffect.ShockWaveType.CHAOTIC), 1.5f))`
- L41: `this.addToBot(new ApplyPowerAction(mo, p, new StrengthPower(mo, -this.magicNumber), -this.magicNumber, true, AbstractGameAction.AttackEffect.NONE))`
- L45: `this.addToBot(new ApplyPowerAction(mo, p, new GainStrengthPower(mo, this.magicNumber), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new SFXAction("ATTACK_PIERCING_WAIL"));
        if (Settings.FAST_MODE) {
            this.addToBot(new VFXAction(p, new ShockWaveEffect(p.hb.cX, p.hb.cY, Settings.GREEN_TEXT_COLOR, ShockWaveEffect.ShockWaveType.CHAOTIC), 0.3f));
        } else {
            this.addToBot(new VFXAction(p, new ShockWaveEffect(p.hb.cX, p.hb.cY, Settings.GREEN_TEXT_COLOR, ShockWaveEffect.ShockWaveType.CHAOTIC), 1.5f));
        }
        for (AbstractMonster mo : AbstractDungeon.getCurrRoom().monsters.monsters) {
            this.addToBot(new ApplyPowerAction(mo, p, new StrengthPower(mo, -this.magicNumber), -this.magicNumber, true, AbstractGameAction.AttackEffect.NONE));
        }
        for (AbstractMonster mo : AbstractDungeon.getCurrRoom().monsters.monsters) {
            if (mo.hasPower("Artifact")) continue;
            this.addToBot(new ApplyPowerAction(mo, p, new GainStrengthPower(mo, this.magicNumber), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE));
        }
    }
```

</details>

## PoisonedStab
File: `cards\green\PoisonedStab.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_VERTICAL)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ApplyPowerAction` — `new ApplyPowerAction(m, p, new PoisonPower(m, p, this.magicNumber), this.magicNumber)`
- `PoisonPower` — `new PoisonPower(m, p, this.magicNumber)`

**Queue order:**
- L31: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_VERTICAL))`
- L32: `this.addToBot(new ApplyPowerAction(m, p, new PoisonPower(m, p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_VERTICAL));
        this.addToBot(new ApplyPowerAction(m, p, new PoisonPower(m, p, this.magicNumber), this.magicNumber));
    }
```

</details>

## PommelStrike
File: `cards\red\PommelStrike.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `DrawCardAction` — `new DrawCardAction(p, this.magicNumber)`

**Queue order:**
- L31: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL))`
- L32: `this.addToBot(new DrawCardAction(p, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL));
        this.addToBot(new DrawCardAction(p, this.magicNumber));
    }
```

</details>

## PowerThrough
File: `cards\red\PowerThrough.java`

**Action sequence (in order):**
- `MakeTempCardInHandAction` — `new MakeTempCardInHandAction((AbstractCard)new Wound(), 2)`
- `Wound` — `new Wound()`
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`

**Queue order:**
- L29: `this.addToBot(new MakeTempCardInHandAction((AbstractCard)new Wound(), 2))`
- L30: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new MakeTempCardInHandAction((AbstractCard)new Wound(), 2));
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
    }
```

</details>

## Pray
File: `cards\purple\Pray.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new MantraPower(p, this.magicNumber), this.magicNumber)`
- `MantraPower` — `new MantraPower(p, this.magicNumber)`
- `MakeTempCardInDrawPileAction` — `new MakeTempCardInDrawPileAction(this.cardsToPreview.makeStatEquivalentCopy(), 1, true, true)`

**Queue order:**
- L30: `this.addToBot(new ApplyPowerAction(p, p, new MantraPower(p, this.magicNumber), this.magicNumber))`
- L31: `this.addToBot(new MakeTempCardInDrawPileAction(this.cardsToPreview.makeStatEquivalentCopy(), 1, true, true))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new MantraPower(p, this.magicNumber), this.magicNumber));
        this.addToBot(new MakeTempCardInDrawPileAction(this.cardsToPreview.makeStatEquivalentCopy(), 1, true, true));
    }
```

</details>

## Predator
File: `cards\green\Predator.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new DrawCardNextTurnPower(p, 2), 2)`
- `DrawCardNextTurnPower` — `new DrawCardNextTurnPower(p, 2)`

**Queue order:**
- L30: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY))`
- L31: `this.addToBot(new ApplyPowerAction(p, p, new DrawCardNextTurnPower(p, 2), 2))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY));
        this.addToBot(new ApplyPowerAction(p, p, new DrawCardNextTurnPower(p, 2), 2));
    }
```

</details>

## Prepared
File: `cards\green\Prepared.java`

**Action sequence (in order):**
- `DrawCardAction` — `new DrawCardAction(p, this.magicNumber)`
- `DiscardAction` — `new DiscardAction(p, p, this.magicNumber, false)`

**Queue order:**
- L26: `this.addToBot(new DrawCardAction(p, this.magicNumber))`
- L27: `this.addToBot(new DiscardAction(p, p, this.magicNumber, false))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DrawCardAction(p, this.magicNumber));
        this.addToBot(new DiscardAction(p, p, this.magicNumber, false));
    }
```

</details>

## PressurePoints
File: `cards\purple\PressurePoints.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new PressurePointEffect(m.hb.cX, m.hb.cY))`
- `PressurePointEffect` — `new PressurePointEffect(m.hb.cX, m.hb.cY)`
- `ApplyPowerAction` — `new ApplyPowerAction(m, p, new MarkPower(m, this.magicNumber), this.magicNumber)`
- `MarkPower` — `new MarkPower(m, this.magicNumber)`
- `TriggerMarksAction` — `new TriggerMarksAction(this)`

**Queue order:**
- L30: `this.addToBot(new VFXAction(new PressurePointEffect(m.hb.cX, m.hb.cY)))`
- L32: `this.addToBot(new ApplyPowerAction(m, p, new MarkPower(m, this.magicNumber), this.magicNumber))`
- L33: `this.addToBot(new TriggerMarksAction(this))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (m != null) {
            this.addToBot(new VFXAction(new PressurePointEffect(m.hb.cX, m.hb.cY)));
        }
        this.addToBot(new ApplyPowerAction(m, p, new MarkPower(m, this.magicNumber), this.magicNumber));
        this.addToBot(new TriggerMarksAction(this));
    }
```

</details>

## Pride
File: `cards\curses\Pride.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
    }
```

</details>

## Prostrate
File: `cards\purple\Prostrate.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new MantraPower(p, this.magicNumber), this.magicNumber)`
- `MantraPower` — `new MantraPower(p, this.magicNumber)`
- `GainBlockAction` — `new GainBlockAction(p, this.block)`

**Queue order:**
- L30: `this.addToBot(new ApplyPowerAction(p, p, new MantraPower(p, this.magicNumber), this.magicNumber))`
- L31: `this.addToBot(new GainBlockAction(p, this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new MantraPower(p, this.magicNumber), this.magicNumber));
        this.addToBot(new GainBlockAction(p, this.block));
    }
```

</details>

## Protect
File: `cards\purple\Protect.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`

**Queue order:**
- L27: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
    }
```

</details>

## Pummel
File: `cards\red\Pummel.java`

**Action sequence (in order):**
- `PummelDamageAction` — `new PummelDamageAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn))`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L32: `this.addToBot(new PummelDamageAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn)))`
- L34: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        for (int i = 1; i < this.magicNumber; ++i) {
            this.addToBot(new PummelDamageAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn)));
        }
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
    }
```

</details>

## Purity
File: `cards\colorless\Purity.java`

**Action sequence (in order):**
- `ExhaustAction` — `new ExhaustAction(this.magicNumber, false, true, true)`

**Queue order:**
- L26: `this.addToBot(new ExhaustAction(this.magicNumber, false, true, true))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ExhaustAction(this.magicNumber, false, true, true));
    }
```

</details>

## QuickSlash
File: `cards\green\QuickSlash.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `DrawCardAction` — `new DrawCardAction(p, 1)`

**Queue order:**
- L29: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL))`
- L30: `this.addToBot(new DrawCardAction(p, 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL));
        this.addToBot(new DrawCardAction(p, 1));
    }
```

</details>

## Rage
File: `cards\red\Rage.java`

**Action sequence (in order):**
- `SFXAction` — `new SFXAction("RAGE")`
- `VFXAction` — `new VFXAction(p, new ShockWaveEffect(p.hb.cX, p.hb.cY, Color.ORANGE, ShockWaveEffect.ShockWaveType.CHAOTIC), 1.0f)`
- `ShockWaveEffect` — `new ShockWaveEffect(p.hb.cX, p.hb.cY, Color.ORANGE, ShockWaveEffect.ShockWaveType.CHAOTIC)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new RagePower(p, this.magicNumber), this.magicNumber)`
- `RagePower` — `new RagePower(p, this.magicNumber)`

**Queue order:**
- L30: `this.addToBot(new SFXAction("RAGE"))`
- L31: `this.addToBot(new VFXAction(p, new ShockWaveEffect(p.hb.cX, p.hb.cY, Color.ORANGE, ShockWaveEffect.ShockWaveType.CHAOTIC), 1.0f))`
- L32: `this.addToBot(new ApplyPowerAction(p, p, new RagePower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new SFXAction("RAGE"));
        this.addToBot(new VFXAction(p, new ShockWaveEffect(p.hb.cX, p.hb.cY, Color.ORANGE, ShockWaveEffect.ShockWaveType.CHAOTIC), 1.0f));
        this.addToBot(new ApplyPowerAction(p, p, new RagePower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Ragnarok
File: `cards\purple\Ragnarok.java`

**Action sequence (in order):**
- `AttackDamageRandomEnemyAction` — `new AttackDamageRandomEnemyAction(this, AbstractGameAction.AttackEffect.LIGHTNING)`

**Queue order:**
- L28: `this.addToBot(new AttackDamageRandomEnemyAction(this, AbstractGameAction.AttackEffect.LIGHTNING))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        for (int i = 0; i < this.magicNumber; ++i) {
            this.addToBot(new AttackDamageRandomEnemyAction(this, AbstractGameAction.AttackEffect.LIGHTNING));
        }
    }
```

</details>

## Rainbow
File: `cards\blue\Rainbow.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new RainbowCardEffect())`
- `RainbowCardEffect` — `new RainbowCardEffect()`
- `ChannelAction` — `new ChannelAction(new Lightning())`
- `Lightning` — `new Lightning()`
- `ChannelAction` — `new ChannelAction(new Frost())`
- `Frost` — `new Frost()`
- `ChannelAction` — `new ChannelAction(new Dark())`
- `Dark` — `new Dark()`

**Queue order:**
- L32: `this.addToBot(new VFXAction(new RainbowCardEffect()))`
- L33: `this.addToBot(new ChannelAction(new Lightning()))`
- L34: `this.addToBot(new ChannelAction(new Frost()))`
- L35: `this.addToBot(new ChannelAction(new Dark()))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new VFXAction(new RainbowCardEffect()));
        this.addToBot(new ChannelAction(new Lightning()));
        this.addToBot(new ChannelAction(new Frost()));
        this.addToBot(new ChannelAction(new Dark()));
    }
```

</details>

## Rampage
File: `cards\red\Rampage.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ModifyDamageAction` — `new ModifyDamageAction(this.uuid, this.magicNumber)`

**Queue order:**
- L30: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL))`
- L31: `this.addToBot(new ModifyDamageAction(this.uuid, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL));
        this.addToBot(new ModifyDamageAction(this.uuid, this.magicNumber));
    }
```

</details>

## ReachHeaven
File: `cards\purple\ReachHeaven.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `MakeTempCardInDrawPileAction` — `new MakeTempCardInDrawPileAction(this.cardsToPreview, 1, true, true)`

**Queue order:**
- L31: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY))`
- L32: `this.addToBot(new MakeTempCardInDrawPileAction(this.cardsToPreview, 1, true, true))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY));
        this.addToBot(new MakeTempCardInDrawPileAction(this.cardsToPreview, 1, true, true));
    }
```

</details>

## Reaper
File: `cards\red\Reaper.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new ReaperEffect())`
- `ReaperEffect` — `new ReaperEffect()`
- `VampireDamageAllEnemiesAction` — `new VampireDamageAllEnemiesAction(p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE)`

**Queue order:**
- L31: `this.addToBot(new VFXAction(new ReaperEffect()))`
- L32: `this.addToBot(new VampireDamageAllEnemiesAction(p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new VFXAction(new ReaperEffect()));
        this.addToBot(new VampireDamageAllEnemiesAction(p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE));
    }
```

</details>

## Reboot
File: `cards\blue\Reboot.java`

**Action sequence (in order):**
- `ShuffleAllAction` — `new ShuffleAllAction()`
- `ShuffleAction` — `new ShuffleAction(AbstractDungeon.player.drawPile, false)`
- `DrawCardAction` — `new DrawCardAction(p, this.magicNumber)`

**Queue order:**
- L29: `this.addToBot(new ShuffleAllAction())`
- L30: `this.addToBot(new ShuffleAction(AbstractDungeon.player.drawPile, false))`
- L31: `this.addToBot(new DrawCardAction(p, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ShuffleAllAction());
        this.addToBot(new ShuffleAction(AbstractDungeon.player.drawPile, false));
        this.addToBot(new DrawCardAction(p, this.magicNumber));
    }
```

</details>

## Rebound
File: `cards\blue\Rebound.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new ReboundPower(p), 1)`
- `ReboundPower` — `new ReboundPower(p)`

**Queue order:**
- L30: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L31: `this.addToBot(new ApplyPowerAction(p, p, new ReboundPower(p), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
        this.addToBot(new ApplyPowerAction(p, p, new ReboundPower(p), 1));
    }
```

</details>

## RecklessCharge
File: `cards\red\RecklessCharge.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `MakeTempCardInDrawPileAction` — `new MakeTempCardInDrawPileAction(new Dazed(), 1, true, true)`
- `Dazed` — `new Dazed()`

**Queue order:**
- L31: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY))`
- L32: `this.addToBot(new MakeTempCardInDrawPileAction(new Dazed(), 1, true, true))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY));
        this.addToBot(new MakeTempCardInDrawPileAction(new Dazed(), 1, true, true));
    }
```

</details>

## Recursion
File: `cards\blue\Recursion.java`

**Action sequence (in order):**
- `RedoAction` — `new RedoAction()`

**Queue order:**
- L24: `this.addToBot(new RedoAction())`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new RedoAction());
    }
```

</details>

## Recycle
File: `cards\blue\Recycle.java`

**Action sequence (in order):**
- `RecycleAction` — `new RecycleAction()`

**Queue order:**
- L24: `this.addToBot(new RecycleAction())`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new RecycleAction());
    }
```

</details>

## Reflex
File: `cards\green\Reflex.java`

**Action sequence (in order):**
- `DrawCardAction` — `new DrawCardAction(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new DrawCardAction(p, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DrawCardAction(p, this.magicNumber));
    }
```

</details>

## Regret
File: `cards\curses\Regret.java`

**Action sequence (in order):**
- `LoseHPAction` — `new LoseHPAction(AbstractDungeon.player, AbstractDungeon.player, this.magicNumber, AbstractGameAction.AttackEffect.FIRE)`

**Queue order:**
- L28: `this.addToTop(new LoseHPAction(AbstractDungeon.player, AbstractDungeon.player, this.magicNumber, AbstractGameAction.AttackEffect.FIRE))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (this.dontTriggerOnUseCard) {
            this.addToTop(new LoseHPAction(AbstractDungeon.player, AbstractDungeon.player, this.magicNumber, AbstractGameAction.AttackEffect.FIRE));
        }
    }
```

</details>

## ReinforcedBody
File: `cards\blue\ReinforcedBody.java`

**Action sequence (in order):**
- `ReinforcedBodyAction` — `new ReinforcedBodyAction(p, this.block, this.freeToPlayOnce, this.energyOnUse)`

**Queue order:**
- L25: `this.addToBot(new ReinforcedBodyAction(p, this.block, this.freeToPlayOnce, this.energyOnUse))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ReinforcedBodyAction(p, this.block, this.freeToPlayOnce, this.energyOnUse));
    }
```

</details>

## Reprogram
File: `cards\blue\Reprogram.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new FocusPower(p, -this.magicNumber), -this.magicNumber)`
- `FocusPower` — `new FocusPower(p, -this.magicNumber)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new StrengthPower(p, this.magicNumber), this.magicNumber)`
- `StrengthPower` — `new StrengthPower(p, this.magicNumber)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new DexterityPower(p, this.magicNumber), this.magicNumber)`
- `DexterityPower` — `new DexterityPower(p, this.magicNumber)`

**Queue order:**
- L28: `this.addToBot(new ApplyPowerAction(p, p, new FocusPower(p, -this.magicNumber), -this.magicNumber))`
- L29: `this.addToBot(new ApplyPowerAction(p, p, new StrengthPower(p, this.magicNumber), this.magicNumber))`
- L30: `this.addToBot(new ApplyPowerAction(p, p, new DexterityPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new FocusPower(p, -this.magicNumber), -this.magicNumber));
        this.addToBot(new ApplyPowerAction(p, p, new StrengthPower(p, this.magicNumber), this.magicNumber));
        this.addToBot(new ApplyPowerAction(p, p, new DexterityPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## RiddleWithHoles
File: `cards\green\RiddleWithHoles.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L29: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        for (int i = 0; i < 5; ++i) {
            this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL));
        }
    }
```

</details>

## RipAndTear
File: `cards\blue\RipAndTear.java`

**Action sequence (in order):**
- `NewRipAndTearAction` — `new NewRipAndTearAction(this)`

**Queue order:**
- L27: `this.addToBot(new NewRipAndTearAction(this))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        for (int i = 0; i < this.magicNumber; ++i) {
            this.addToBot(new NewRipAndTearAction(this));
        }
    }
```

</details>

## RitualDagger
File: `cards\colorless\RitualDagger.java`

**Action sequence (in order):**
- `RitualDaggerAction` — `new RitualDaggerAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn), this.magicNumber, this.uuid)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L29: `this.addToBot(new RitualDaggerAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn), this.magicNumber, this.uuid))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new RitualDaggerAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn), this.magicNumber, this.uuid));
    }
```

</details>

## Rupture
File: `cards\red\Rupture.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new RupturePower(p, this.magicNumber), this.magicNumber)`
- `RupturePower` — `new RupturePower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new RupturePower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new RupturePower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Rushdown
File: `cards\purple\Rushdown.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new RushdownPower(p, this.magicNumber), this.magicNumber)`
- `RushdownPower` — `new RushdownPower(p, this.magicNumber)`

**Queue order:**
- L27: `this.addToBot(new ApplyPowerAction(p, p, new RushdownPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new RushdownPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## SadisticNature
File: `cards\colorless\SadisticNature.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new SadisticPower(p, this.magicNumber), this.magicNumber)`
- `SadisticPower` — `new SadisticPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new SadisticPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new SadisticPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Safety
File: `cards\tempCards\Safety.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`

**Queue order:**
- L28: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
    }
```

</details>

## Sanctity
File: `cards\purple\Sanctity.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction(p, this.block)`
- `SanctityAction` — `new SanctityAction(this.magicNumber)`

**Queue order:**
- L28: `this.addToBot(new GainBlockAction(p, this.block))`
- L29: `this.addToBot(new SanctityAction(this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction(p, this.block));
        this.addToBot(new SanctityAction(this.magicNumber));
    }
```

</details>

## SandsOfTime
File: `cards\purple\SandsOfTime.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SMASH)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L35: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SMASH))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SMASH));
    }
```

</details>

## SashWhip
File: `cards\purple\SashWhip.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `HeadStompAction` — `new HeadStompAction(m, this.magicNumber)`

**Queue order:**
- L31: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L32: `this.addToBot(new HeadStompAction(m, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
        this.addToBot(new HeadStompAction(m, this.magicNumber));
    }
```

</details>

## Scrape
File: `cards\blue\Scrape.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new ScrapeEffect(m.hb.cX, m.hb.cY), 0.1f)`
- `ScrapeEffect` — `new ScrapeEffect(m.hb.cX, m.hb.cY)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `DrawCardAction` — `new DrawCardAction(this.magicNumber, new ScrapeFollowUpAction())`
- `ScrapeFollowUpAction` — `new ScrapeFollowUpAction()`

**Queue order:**
- L34: `this.addToBot(new VFXAction(new ScrapeEffect(m.hb.cX, m.hb.cY), 0.1f))`
- L36: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY))`
- L37: `this.addToBot(new DrawCardAction(this.magicNumber, new ScrapeFollowUpAction()))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (m != null) {
            this.addToBot(new VFXAction(new ScrapeEffect(m.hb.cX, m.hb.cY), 0.1f));
        }
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY));
        this.addToBot(new DrawCardAction(this.magicNumber, new ScrapeFollowUpAction()));
    }
```

</details>

## Scrawl
File: `cards\purple\Scrawl.java`

**Action sequence (in order):**
- `ExpertiseAction` — `new ExpertiseAction(p, 10)`

**Queue order:**
- L25: `this.addToBot(new ExpertiseAction(p, 10))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ExpertiseAction(p, 10));
    }
```

</details>

## SearingBlow
File: `cards\red\SearingBlow.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new SearingBlowEffect(m.hb.cX, m.hb.cY, this.timesUpgraded), 0.2f)`
- `SearingBlowEffect` — `new SearingBlowEffect(m.hb.cX, m.hb.cY, this.timesUpgraded)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L36: `this.addToBot(new VFXAction(new SearingBlowEffect(m.hb.cX, m.hb.cY, this.timesUpgraded), 0.2f))`
- L38: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (m != null) {
            this.addToBot(new VFXAction(new SearingBlowEffect(m.hb.cX, m.hb.cY, this.timesUpgraded), 0.2f));
        }
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
    }
```

</details>

## SecondWind
File: `cards\red\SecondWind.java`

**Action sequence (in order):**
- `BlockPerNonAttackAction` — `new BlockPerNonAttackAction(this.block)`

**Queue order:**
- L25: `this.addToBot(new BlockPerNonAttackAction(this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new BlockPerNonAttackAction(this.block));
    }
```

</details>

## SecretTechnique
File: `cards\colorless\SecretTechnique.java`

**Action sequence (in order):**
- `SkillFromDeckToHandAction` — `new SkillFromDeckToHandAction(1)`

**Queue order:**
- L25: `this.addToBot(new SkillFromDeckToHandAction(1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new SkillFromDeckToHandAction(1));
    }
```

</details>

## SecretWeapon
File: `cards\colorless\SecretWeapon.java`

**Action sequence (in order):**
- `AttackFromDeckToHandAction` — `new AttackFromDeckToHandAction(1)`

**Queue order:**
- L25: `this.addToBot(new AttackFromDeckToHandAction(1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new AttackFromDeckToHandAction(1));
    }
```

</details>

## SeeingRed
File: `cards\red\SeeingRed.java`

**Action sequence (in order):**
- `GainEnergyAction` — `new GainEnergyAction(2)`

**Queue order:**
- L25: `this.addToBot(new GainEnergyAction(2))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainEnergyAction(2));
    }
```

</details>

## Seek
File: `cards\blue\Seek.java`

**Action sequence (in order):**
- `BetterDrawPileToHandAction` — `new BetterDrawPileToHandAction(this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new BetterDrawPileToHandAction(this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new BetterDrawPileToHandAction(this.magicNumber));
    }
```

</details>

## SelfRepair
File: `cards\blue\SelfRepair.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new RepairPower(p, this.magicNumber), this.magicNumber)`
- `RepairPower` — `new RepairPower(p, this.magicNumber)`

**Queue order:**
- L27: `this.addToBot(new ApplyPowerAction(p, p, new RepairPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new RepairPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Sentinel
File: `cards\red\Sentinel.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`

**Queue order:**
- L27: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
    }
```

</details>

## Setup
File: `cards\green\Setup.java`

**Action sequence (in order):**
- `SetupAction` — `new SetupAction()`

**Queue order:**
- L24: `this.addToBot(new SetupAction())`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new SetupAction());
    }
```

</details>

## SeverSoul
File: `cards\red\SeverSoul.java`

**Action sequence (in order):**
- `ExhaustAllNonAttackAction` — `new ExhaustAllNonAttackAction()`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.FIRE)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L29: `this.addToBot(new ExhaustAllNonAttackAction())`
- L30: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.FIRE))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ExhaustAllNonAttackAction());
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.FIRE));
    }
```

</details>

## Shame
File: `cards\curses\Shame.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new FrailPower(AbstractDungeon.player, 1, true), 1)`
- `FrailPower` — `new FrailPower(AbstractDungeon.player, 1, true)`

**Queue order:**
- L28: `this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new FrailPower(AbstractDungeon.player, 1, true), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (this.dontTriggerOnUseCard) {
            this.addToTop(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new FrailPower(AbstractDungeon.player, 1, true), 1));
        }
    }
```

</details>

## Shiv
File: `cards\tempCards\Shiv.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L32: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL));
    }
```

</details>

## Shockwave
File: `cards\red\Shockwave.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(mo, p, new WeakPower(mo, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE)`
- `WeakPower` — `new WeakPower(mo, this.magicNumber, false)`
- `ApplyPowerAction` — `new ApplyPowerAction(mo, p, new VulnerablePower(mo, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE)`
- `VulnerablePower` — `new VulnerablePower(mo, this.magicNumber, false)`

**Queue order:**
- L31: `this.addToBot(new ApplyPowerAction(mo, p, new WeakPower(mo, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE))`
- L32: `this.addToBot(new ApplyPowerAction(mo, p, new VulnerablePower(mo, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        for (AbstractMonster mo : AbstractDungeon.getCurrRoom().monsters.monsters) {
            this.addToBot(new ApplyPowerAction(mo, p, new WeakPower(mo, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE));
            this.addToBot(new ApplyPowerAction(mo, p, new VulnerablePower(mo, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE));
        }
    }
```

</details>

## ShrugItOff
File: `cards\red\ShrugItOff.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `DrawCardAction` — `new DrawCardAction(p, 1)`

**Queue order:**
- L27: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L28: `this.addToBot(new DrawCardAction(p, 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.addToBot(new DrawCardAction(p, 1));
    }
```

</details>

## SignatureMove
File: `cards\purple\SignatureMove.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new ClashEffect(m.hb.cX, m.hb.cY), 0.1f)`
- `ClashEffect` — `new ClashEffect(m.hb.cX, m.hb.cY)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L32: `this.addToBot(new VFXAction(new ClashEffect(m.hb.cX, m.hb.cY), 0.1f))`
- L34: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (m != null) {
            this.addToBot(new VFXAction(new ClashEffect(m.hb.cX, m.hb.cY), 0.1f));
        }
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.NONE));
    }
```

</details>

## SimmeringFury
File: `cards\purple\SimmeringFury.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new WrathNextTurnPower(p))`
- `WrathNextTurnPower` — `new WrathNextTurnPower(p)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new DrawCardNextTurnPower(p, this.magicNumber), this.magicNumber)`
- `DrawCardNextTurnPower` — `new DrawCardNextTurnPower(p, this.magicNumber)`

**Queue order:**
- L27: `this.addToBot(new ApplyPowerAction(p, p, new WrathNextTurnPower(p)))`
- L28: `this.addToBot(new ApplyPowerAction(p, p, new DrawCardNextTurnPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new WrathNextTurnPower(p)));
        this.addToBot(new ApplyPowerAction(p, p, new DrawCardNextTurnPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Skewer
File: `cards\green\Skewer.java`

**Action sequence (in order):**
- `SkewerAction` — `new SkewerAction(p, m, this.damage, this.damageTypeForTurn, this.freeToPlayOnce, this.energyOnUse)`

**Queue order:**
- L25: `this.addToBot(new SkewerAction(p, m, this.damage, this.damageTypeForTurn, this.freeToPlayOnce, this.energyOnUse))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new SkewerAction(p, m, this.damage, this.damageTypeForTurn, this.freeToPlayOnce, this.energyOnUse));
    }
```

</details>

## Skim
File: `cards\blue\Skim.java`

**Action sequence (in order):**
- `DrawCardAction` — `new DrawCardAction(p, this.magicNumber)`

**Queue order:**
- L25: `this.addToBot(new DrawCardAction(p, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DrawCardAction(p, this.magicNumber));
    }
```

</details>

## Slice
File: `cards\green\Slice.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L28: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL));
    }
```

</details>

## Slimed
File: `cards\status\Slimed.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
    }
```

</details>

## Smite
File: `cards\tempCards\Smite.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L30: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
    }
```

</details>

## SneakyStrike
File: `cards\green\SneakyStrike.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `GainEnergyIfDiscardAction` — `new GainEnergyIfDiscardAction(2)`

**Queue order:**
- L31: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL))`
- L32: `this.addToBot(new GainEnergyIfDiscardAction(2))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL));
        this.addToBot(new GainEnergyIfDiscardAction(2));
    }
```

</details>

## SpiritShield
File: `cards\purple\SpiritShield.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`

**Queue order:**
- L28: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.applyPowers();
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
    }
```

</details>

## SpotWeakness
File: `cards\red\SpotWeakness.java`

**Action sequence (in order):**
- `SpotWeaknessAction` — `new SpotWeaknessAction(this.magicNumber, m)`

**Queue order:**
- L25: `this.addToBot(new SpotWeaknessAction(this.magicNumber, m))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new SpotWeaknessAction(this.magicNumber, m));
    }
```

</details>

## Stack
File: `cards\blue\Stack.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`

**Queue order:**
- L27: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.rawDescription = !this.upgraded ? Stack.cardStrings.DESCRIPTION : Stack.cardStrings.UPGRADE_DESCRIPTION;
        this.initializeDescription();
    }
```

</details>

## StaticDischarge
File: `cards\blue\StaticDischarge.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new StaticDischargePower(p, this.magicNumber), this.magicNumber)`
- `StaticDischargePower` — `new StaticDischargePower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new StaticDischargePower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new StaticDischargePower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## SteamBarrier
File: `cards\blue\SteamBarrier.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `ModifyBlockAction` — `new ModifyBlockAction(this.uuid, -1)`

**Queue order:**
- L27: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L28: `this.addToBot(new ModifyBlockAction(this.uuid, -1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.addToBot(new ModifyBlockAction(this.uuid, -1));
    }
```

</details>

## Storm
File: `cards\blue\Storm.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new StormPower(p, this.magicNumber), this.magicNumber)`
- `StormPower` — `new StormPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new StormPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new StormPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## StormOfSteel
File: `cards\green\StormOfSteel.java`

**Action sequence (in order):**
- `BladeFuryAction` — `new BladeFuryAction(this.upgraded)`

**Queue order:**
- L26: `this.addToBot(new BladeFuryAction(this.upgraded))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new BladeFuryAction(this.upgraded));
    }
```

</details>

## Streamline
File: `cards\blue\Streamline.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ReduceCostAction` — `new ReduceCostAction(this.uuid, this.magicNumber)`

**Queue order:**
- L30: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL))`
- L31: `this.addToBot(new ReduceCostAction(this.uuid, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL));
        this.addToBot(new ReduceCostAction(this.uuid, this.magicNumber));
    }
```

</details>

## Strike_Blue
File: `cards\blue\Strike_Blue.java`

**Action sequence (in order):**
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.BLUNT_LIGHT)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, 150, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, 150, this.damageTypeForTurn)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L39: `this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.BLUNT_LIGHT))`
- L41: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, 150, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L44: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (Settings.isDebug) {
            if (Settings.isInfo) {
                this.multiDamage = new int[AbstractDungeon.getCurrRoom().monsters.monsters.size()];
                for (int i = 0; i < AbstractDungeon.getCurrRoom().monsters.monsters.size(); ++i) {
                    this.multiDamage[i] = 150;
                }
                this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.BLUNT_LIGHT));
            } else {
                this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, 150, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
            }
        } else {
            this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT));
        }
    }
```

</details>

## Strike_Green
File: `cards\green\Strike_Green.java`

**Action sequence (in order):**
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.SLASH_DIAGONAL)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, 150, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, 150, this.damageTypeForTurn)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L39: `this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.SLASH_DIAGONAL))`
- L41: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, 150, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L44: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (Settings.isDebug) {
            if (Settings.isInfo) {
                this.multiDamage = new int[AbstractDungeon.getCurrRoom().monsters.monsters.size()];
                for (int i = 0; i < AbstractDungeon.getCurrRoom().monsters.monsters.size(); ++i) {
                    this.multiDamage[i] = 150;
                }
                this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.SLASH_DIAGONAL));
            } else {
                this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, 150, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
            }
        } else {
            this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL));
        }
    }
```

</details>

## Strike_Purple
File: `cards\purple\Strike_Purple.java`

**Action sequence (in order):**
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.BLUNT_LIGHT)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, 150, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, 150, this.damageTypeForTurn)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L39: `this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.BLUNT_LIGHT))`
- L41: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, 150, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L44: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (Settings.isDebug) {
            if (Settings.isInfo) {
                this.multiDamage = new int[AbstractDungeon.getCurrRoom().monsters.monsters.size()];
                for (int i = 0; i < AbstractDungeon.getCurrRoom().monsters.monsters.size(); ++i) {
                    this.multiDamage[i] = 150;
                }
                this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.BLUNT_LIGHT));
            } else {
                this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, 150, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
            }
        } else {
            this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT));
        }
    }
```

</details>

## Strike_Red
File: `cards\red\Strike_Red.java`

**Action sequence (in order):**
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.SLASH_DIAGONAL)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, 150, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, 150, this.damageTypeForTurn)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L39: `this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.SLASH_DIAGONAL))`
- L41: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, 150, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L44: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (Settings.isDebug) {
            if (Settings.isInfo) {
                this.multiDamage = new int[AbstractDungeon.getCurrRoom().monsters.monsters.size()];
                for (int i = 0; i < AbstractDungeon.getCurrRoom().monsters.monsters.size(); ++i) {
                    this.multiDamage[i] = 150;
                }
                this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.SLASH_DIAGONAL));
            } else {
                this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, 150, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
            }
        } else {
            this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL));
        }
    }
```

</details>

## Study
File: `cards\purple\Study.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new StudyPower(p, this.magicNumber), this.magicNumber)`
- `StudyPower` — `new StudyPower(p, this.magicNumber)`

**Queue order:**
- L28: `this.addToBot(new ApplyPowerAction(p, p, new StudyPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new StudyPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## SuckerPunch
File: `cards\green\SuckerPunch.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ApplyPowerAction` — `new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber)`
- `WeakPower` — `new WeakPower(m, this.magicNumber, false)`

**Queue order:**
- L31: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L32: `this.addToBot(new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
        this.addToBot(new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber));
    }
```

</details>

## Sunder
File: `cards\blue\Sunder.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new WeightyImpactEffect(m.hb.cX, m.hb.cY))`
- `WeightyImpactEffect` — `new WeightyImpactEffect(m.hb.cX, m.hb.cY)`
- `WaitAction` — `new WaitAction(0.8f)`
- `SunderAction` — `new SunderAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn), 3)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L30: `this.addToBot(new VFXAction(new WeightyImpactEffect(m.hb.cX, m.hb.cY)))`
- L31: `this.addToBot(new WaitAction(0.8f))`
- L33: `this.addToBot(new SunderAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn), 3))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (m != null) {
            this.addToBot(new VFXAction(new WeightyImpactEffect(m.hb.cX, m.hb.cY)));
            this.addToBot(new WaitAction(0.8f));
        }
        this.addToBot(new SunderAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn), 3));
    }
```

</details>

## Survivor
File: `cards\green\Survivor.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `DiscardAction` — `new DiscardAction(p, p, 1, false)`

**Queue order:**
- L27: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L28: `this.addToBot(new DiscardAction(p, p, 1, false))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.addToBot(new DiscardAction(p, p, 1, false));
    }
```

</details>

## SweepingBeam
File: `cards\blue\SweepingBeam.java`

**Action sequence (in order):**
- `SFXAction` — `new SFXAction("ATTACK_DEFECT_BEAM")`
- `VFXAction` — `new VFXAction(p, new SweepingBeamEffect(AbstractDungeon.player.hb.cX, AbstractDungeon.player.hb.cY, AbstractDungeon.player.flipHorizontal), 0.4f)`
- `SweepingBeamEffect` — `new SweepingBeamEffect(AbstractDungeon.player.hb.cX, AbstractDungeon.player.hb.cY, AbstractDungeon.player.flipHorizontal)`
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.FIRE)`
- `DrawCardAction` — `new DrawCardAction(p, this.magicNumber)`

**Queue order:**
- L34: `this.addToBot(new SFXAction("ATTACK_DEFECT_BEAM"))`
- L35: `this.addToBot(new VFXAction(p, new SweepingBeamEffect(AbstractDungeon.player.hb.cX, AbstractDungeon.player.hb.cY, AbstractDungeon.player.flipHorizontal), 0.4f))`
- L36: `this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.FIRE))`
- L37: `this.addToBot(new DrawCardAction(p, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new SFXAction("ATTACK_DEFECT_BEAM"));
        this.addToBot(new VFXAction(p, new SweepingBeamEffect(AbstractDungeon.player.hb.cX, AbstractDungeon.player.hb.cY, AbstractDungeon.player.flipHorizontal), 0.4f));
        this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.FIRE));
        this.addToBot(new DrawCardAction(p, this.magicNumber));
    }
```

</details>

## SwiftStrike
File: `cards\colorless\SwiftStrike.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L29: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT));
    }
```

</details>

## Swivel
File: `cards\purple\Swivel.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new FreeAttackPower(p, 1), 1)`
- `FreeAttackPower` — `new FreeAttackPower(p, 1)`

**Queue order:**
- L29: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L30: `this.addToBot(new ApplyPowerAction(p, p, new FreeAttackPower(p, 1), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.addToBot(new ApplyPowerAction(p, p, new FreeAttackPower(p, 1), 1));
    }
```

</details>

## SwordBoomerang
File: `cards\red\SwordBoomerang.java`

**Action sequence (in order):**
- `AttackDamageRandomEnemyAction` — `new AttackDamageRandomEnemyAction(this, AbstractGameAction.AttackEffect.SLASH_HORIZONTAL)`

**Queue order:**
- L28: `this.addToBot(new AttackDamageRandomEnemyAction(this, AbstractGameAction.AttackEffect.SLASH_HORIZONTAL))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        for (int i = 0; i < this.magicNumber; ++i) {
            this.addToBot(new AttackDamageRandomEnemyAction(this, AbstractGameAction.AttackEffect.SLASH_HORIZONTAL));
        }
    }
```

</details>

## Tactician
File: `cards\green\Tactician.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
    }
```

</details>

## TalkToTheHand
File: `cards\purple\TalkToTheHand.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ApplyPowerAction` — `new ApplyPowerAction(m, p, new BlockReturnPower(m, this.magicNumber), this.magicNumber)`
- `BlockReturnPower` — `new BlockReturnPower(m, this.magicNumber)`

**Queue order:**
- L32: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L33: `this.addToBot(new ApplyPowerAction(m, p, new BlockReturnPower(m, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
        this.addToBot(new ApplyPowerAction(m, p, new BlockReturnPower(m, this.magicNumber), this.magicNumber));
    }
```

</details>

## Tantrum
File: `cards\purple\Tantrum.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ChangeStanceAction` — `new ChangeStanceAction("Wrath")`

**Queue order:**
- L33: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT))`
- L35: `this.addToBot(new ChangeStanceAction("Wrath"))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        for (int i = 0; i < this.magicNumber; ++i) {
            this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT));
        }
        this.addToBot(new ChangeStanceAction("Wrath"));
    }
```

</details>

## Tempest
File: `cards\blue\Tempest.java`

**Action sequence (in order):**
- `TempestAction` — `new TempestAction(p, this.energyOnUse, this.upgraded, this.freeToPlayOnce)`

**Queue order:**
- L27: `this.addToBot(new TempestAction(p, this.energyOnUse, this.upgraded, this.freeToPlayOnce))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new TempestAction(p, this.energyOnUse, this.upgraded, this.freeToPlayOnce));
    }
```

</details>

## Terror
File: `cards\green\Terror.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(m, p, new VulnerablePower(m, 99, false), 99)`
- `VulnerablePower` — `new VulnerablePower(m, 99, false)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(m, p, new VulnerablePower(m, 99, false), 99))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(m, p, new VulnerablePower(m, 99, false), 99));
    }
```

</details>

## TheBomb
File: `cards\colorless\TheBomb.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new TheBombPower(p, 3, this.magicNumber), 3)`
- `TheBombPower` — `new TheBombPower(p, 3, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new TheBombPower(p, 3, this.magicNumber), 3))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new TheBombPower(p, 3, this.magicNumber), 3));
    }
```

</details>

## ThinkingAhead
File: `cards\colorless\ThinkingAhead.java`

**Action sequence (in order):**
- `DrawCardAction` — `new DrawCardAction(p, 2)`
- `PutOnDeckAction` — `new PutOnDeckAction(p, p, 1, false)`

**Queue order:**
- L27: `this.addToBot(new DrawCardAction(p, 2))`
- L29: `this.addToBot(new PutOnDeckAction(p, p, 1, false))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DrawCardAction(p, 2));
        if (AbstractDungeon.player.hand.size() > 0) {
            this.addToBot(new PutOnDeckAction(p, p, 1, false));
        }
    }
```

</details>

## ThirdEye
File: `cards\purple\ThirdEye.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new ThirdEyeEffect(p.hb.cX, p.hb.cY))`
- `ThirdEyeEffect` — `new ThirdEyeEffect(p.hb.cX, p.hb.cY)`
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `ScryAction` — `new ScryAction(this.magicNumber)`

**Queue order:**
- L31: `this.addToBot(new VFXAction(new ThirdEyeEffect(p.hb.cX, p.hb.cY)))`
- L33: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L34: `this.addToBot(new ScryAction(this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (p != null) {
            this.addToBot(new VFXAction(new ThirdEyeEffect(p.hb.cX, p.hb.cY)));
        }
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        this.addToBot(new ScryAction(this.magicNumber));
    }
```

</details>

## ThroughViolence
File: `cards\tempCards\ThroughViolence.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new ViolentAttackEffect(m.hb.cX, m.hb.cY, Color.VIOLET))`
- `ViolentAttackEffect` — `new ViolentAttackEffect(m.hb.cX, m.hb.cY, Color.VIOLET)`
- `VFXAction` — `new VFXAction(new ViolentAttackEffect(m.hb.cX, m.hb.cY, Color.VIOLET), 0.4f)`
- `ViolentAttackEffect` — `new ViolentAttackEffect(m.hb.cX, m.hb.cY, Color.VIOLET)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L36: `this.addToBot(new VFXAction(new ViolentAttackEffect(m.hb.cX, m.hb.cY, Color.VIOLET)))`
- L38: `this.addToBot(new VFXAction(new ViolentAttackEffect(m.hb.cX, m.hb.cY, Color.VIOLET), 0.4f))`
- L41: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (m != null) {
            if (Settings.FAST_MODE) {
                this.addToBot(new VFXAction(new ViolentAttackEffect(m.hb.cX, m.hb.cY, Color.VIOLET)));
            } else {
                this.addToBot(new VFXAction(new ViolentAttackEffect(m.hb.cX, m.hb.cY, Color.VIOLET), 0.4f));
            }
        }
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY));
    }
```

</details>

## ThunderClap
File: `cards\red\ThunderClap.java`

**Action sequence (in order):**
- `SFXAction` — `new SFXAction("THUNDERCLAP", 0.05f)`
- `VFXAction` — `new VFXAction(new LightningEffect(mo.drawX, mo.drawY), 0.05f)`
- `LightningEffect` — `new LightningEffect(mo.drawX, mo.drawY)`
- `DamageAllEnemiesAction` — `new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE)`
- `ApplyPowerAction` — `new ApplyPowerAction(mo, p, new VulnerablePower(mo, 1, false), 1, true, AbstractGameAction.AttackEffect.NONE)`
- `VulnerablePower` — `new VulnerablePower(mo, 1, false)`

**Queue order:**
- L34: `this.addToBot(new SFXAction("THUNDERCLAP", 0.05f))`
- L37: `this.addToBot(new VFXAction(new LightningEffect(mo.drawX, mo.drawY), 0.05f))`
- L39: `this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE))`
- L41: `this.addToBot(new ApplyPowerAction(mo, p, new VulnerablePower(mo, 1, false), 1, true, AbstractGameAction.AttackEffect.NONE))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new SFXAction("THUNDERCLAP", 0.05f));
        for (AbstractMonster mo : AbstractDungeon.getCurrRoom().monsters.monsters) {
            if (mo.isDeadOrEscaped()) continue;
            this.addToBot(new VFXAction(new LightningEffect(mo.drawX, mo.drawY), 0.05f));
        }
        this.addToBot(new DamageAllEnemiesAction((AbstractCreature)p, this.multiDamage, this.damageTypeForTurn, AbstractGameAction.AttackEffect.NONE));
        for (AbstractMonster mo : AbstractDungeon.getCurrRoom().monsters.monsters) {
            this.addToBot(new ApplyPowerAction(mo, p, new VulnerablePower(mo, 1, false), 1, true, AbstractGameAction.AttackEffect.NONE));
        }
    }
```

</details>

## ThunderStrike
File: `cards\blue\ThunderStrike.java`

**Action sequence (in order):**
- `NewThunderStrikeAction` — `new NewThunderStrikeAction(this)`

**Queue order:**
- L38: `this.addToBot(new NewThunderStrikeAction(this))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.baseMagicNumber = 0;
        for (AbstractOrb o : AbstractDungeon.actionManager.orbsChanneledThisCombat) {
            if (!(o instanceof Lightning)) continue;
            ++this.baseMagicNumber;
        }
        this.magicNumber = this.baseMagicNumber;
        for (int i = 0; i < this.magicNumber; ++i) {
            this.addToBot(new NewThunderStrikeAction(this));
        }
    }
```

</details>

## ToolsOfTheTrade
File: `cards\green\ToolsOfTheTrade.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new ToolsOfTheTradePower(p, 1), 1)`
- `ToolsOfTheTradePower` — `new ToolsOfTheTradePower(p, 1)`

**Queue order:**
- L25: `this.addToBot(new ApplyPowerAction(p, p, new ToolsOfTheTradePower(p, 1), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new ToolsOfTheTradePower(p, 1), 1));
    }
```

</details>

## Tranquility
File: `cards\purple\Tranquility.java`

**Action sequence (in order):**
- `ChangeStanceAction` — `new ChangeStanceAction("Calm")`

**Queue order:**
- L26: `this.addToBot(new ChangeStanceAction("Calm"))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ChangeStanceAction("Calm"));
    }
```

</details>

## Transmutation
File: `cards\colorless\Transmutation.java`

**Action sequence (in order):**
- `TransmutationAction` — `new TransmutationAction(p, this.upgraded, this.freeToPlayOnce, this.energyOnUse)`

**Queue order:**
- L29: `this.addToBot(new TransmutationAction(p, this.upgraded, this.freeToPlayOnce, this.energyOnUse))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (this.energyOnUse < EnergyPanel.totalCount) {
            this.energyOnUse = EnergyPanel.totalCount;
        }
        this.addToBot(new TransmutationAction(p, this.upgraded, this.freeToPlayOnce, this.energyOnUse));
    }
```

</details>

## Trip
File: `cards\colorless\Trip.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(m, p, new VulnerablePower(m, this.magicNumber, false), this.magicNumber)`
- `VulnerablePower` — `new VulnerablePower(m, this.magicNumber, false)`
- `ApplyPowerAction` — `new ApplyPowerAction(mo, p, new VulnerablePower(mo, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE)`
- `VulnerablePower` — `new VulnerablePower(mo, this.magicNumber, false)`

**Queue order:**
- L29: `this.addToBot(new ApplyPowerAction(m, p, new VulnerablePower(m, this.magicNumber, false), this.magicNumber))`
- L32: `this.addToBot(new ApplyPowerAction(mo, p, new VulnerablePower(mo, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (!this.upgraded) {
            this.addToBot(new ApplyPowerAction(m, p, new VulnerablePower(m, this.magicNumber, false), this.magicNumber));
        } else {
            for (AbstractMonster mo : AbstractDungeon.getCurrRoom().monsters.monsters) {
                this.addToBot(new ApplyPowerAction(mo, p, new VulnerablePower(mo, this.magicNumber, false), this.magicNumber, true, AbstractGameAction.AttackEffect.NONE));
            }
        }
    }
```

</details>

## TrueGrit
File: `cards\red\TrueGrit.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction((AbstractCreature)p, p, this.block)`
- `ExhaustAction` — `new ExhaustAction(1, false)`
- `ExhaustAction` — `new ExhaustAction(1, true, false, false)`

**Queue order:**
- L27: `this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block))`
- L29: `this.addToBot(new ExhaustAction(1, false))`
- L31: `this.addToBot(new ExhaustAction(1, true, false, false))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction((AbstractCreature)p, p, this.block));
        if (this.upgraded) {
            this.addToBot(new ExhaustAction(1, false));
        } else {
            this.addToBot(new ExhaustAction(1, true, false, false));
        }
    }
```

</details>

## Turbo
File: `cards\blue\Turbo.java`

**Action sequence (in order):**
- `GainEnergyAction` — `new GainEnergyAction(this.magicNumber)`
- `MakeTempCardInDiscardAction` — `new MakeTempCardInDiscardAction((AbstractCard)new VoidCard(), 1)`
- `VoidCard` — `new VoidCard()`

**Queue order:**
- L28: `this.addToBot(new GainEnergyAction(this.magicNumber))`
- L29: `this.addToBot(new MakeTempCardInDiscardAction((AbstractCard)new VoidCard(), 1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainEnergyAction(this.magicNumber));
        this.addToBot(new MakeTempCardInDiscardAction((AbstractCard)new VoidCard(), 1));
    }
```

</details>

## TwinStrike
File: `cards\red\TwinStrike.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_VERTICAL)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L29: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL))`
- L30: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_VERTICAL))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HORIZONTAL));
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_VERTICAL));
    }
```

</details>

## Unload
File: `cards\green\Unload.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `UnloadAction` — `new UnloadAction(p)`

**Queue order:**
- L29: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL))`
- L30: `this.addToBot(new UnloadAction(p))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_DIAGONAL));
        this.addToBot(new UnloadAction(p));
    }
```

</details>

## Unraveling
File: `cards\purple\Unraveling.java`

**Action sequence (in order):**
- `UnravelingAction` — `new UnravelingAction()`

**Queue order:**
- L25: `this.addToBot(new UnravelingAction())`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new UnravelingAction());
    }
```

</details>

## Uppercut
File: `cards\red\Uppercut.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `ApplyPowerAction` — `new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber)`
- `WeakPower` — `new WeakPower(m, this.magicNumber, false)`
- `ApplyPowerAction` — `new ApplyPowerAction(m, p, new VulnerablePower(m, this.magicNumber, false), this.magicNumber)`
- `VulnerablePower` — `new VulnerablePower(m, this.magicNumber, false)`

**Queue order:**
- L32: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L33: `this.addToBot(new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber))`
- L34: `this.addToBot(new ApplyPowerAction(m, p, new VulnerablePower(m, this.magicNumber, false), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
        this.addToBot(new ApplyPowerAction(m, p, new WeakPower(m, this.magicNumber, false), this.magicNumber));
        this.addToBot(new ApplyPowerAction(m, p, new VulnerablePower(m, this.magicNumber, false), this.magicNumber));
    }
```

</details>

## Vault
File: `cards\purple\Vault.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(new WhirlwindEffect(new Color(1.0f, 0.9f, 0.4f, 1.0f), true))`
- `WhirlwindEffect` — `new WhirlwindEffect(new Color(1.0f, 0.9f, 0.4f, 1.0f), true)`
- `Color` — `new Color(1.0f, 0.9f, 0.4f, 1.0f)`
- `SkipEnemiesTurnAction` — `new SkipEnemiesTurnAction()`
- `PressEndTurnButtonAction` — `new PressEndTurnButtonAction()`

**Queue order:**
- L29: `this.addToBot(new VFXAction(new WhirlwindEffect(new Color(1.0f, 0.9f, 0.4f, 1.0f), true)))`
- L30: `this.addToBot(new SkipEnemiesTurnAction())`
- L31: `this.addToBot(new PressEndTurnButtonAction())`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new VFXAction(new WhirlwindEffect(new Color(1.0f, 0.9f, 0.4f, 1.0f), true)));
        this.addToBot(new SkipEnemiesTurnAction());
        this.addToBot(new PressEndTurnButtonAction());
    }
```

</details>

## Vigilance
File: `cards\purple\Vigilance.java`

**Action sequence (in order):**
- `GainBlockAction` — `new GainBlockAction(p, this.block)`
- `ChangeStanceAction` — `new ChangeStanceAction("Calm")`

**Queue order:**
- L26: `this.addToBot(new GainBlockAction(p, this.block))`
- L27: `this.addToBot(new ChangeStanceAction("Calm"))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new GainBlockAction(p, this.block));
        this.addToBot(new ChangeStanceAction("Calm"));
    }
```

</details>

## Violence
File: `cards\colorless\Violence.java`

**Action sequence (in order):**
- `DrawPileToHandAction` — `new DrawPileToHandAction(this.magicNumber, AbstractCard.CardType.ATTACK)`

**Queue order:**
- L26: `this.addToBot(new DrawPileToHandAction(this.magicNumber, AbstractCard.CardType.ATTACK))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DrawPileToHandAction(this.magicNumber, AbstractCard.CardType.ATTACK));
    }
```

</details>

## VoidCard
File: `cards\status\VoidCard.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
    }
```

</details>

## Wallop
File: `cards\purple\Wallop.java`

**Action sequence (in order):**
- `WallopAction` — `new WallopAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn))`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L26: `this.addToBot(new WallopAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn)))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new WallopAction(m, new DamageInfo(p, this.damage, this.damageTypeForTurn)));
    }
```

</details>

## Warcry
File: `cards\red\Warcry.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(p, new ShockWaveEffect(p.hb.cX, p.hb.cY, Settings.RED_TEXT_COLOR, ShockWaveEffect.ShockWaveType.ADDITIVE), 0.5f)`
- `ShockWaveEffect` — `new ShockWaveEffect(p.hb.cX, p.hb.cY, Settings.RED_TEXT_COLOR, ShockWaveEffect.ShockWaveType.ADDITIVE)`
- `DrawCardAction` — `new DrawCardAction(p, this.magicNumber)`
- `PutOnDeckAction` — `new PutOnDeckAction(p, p, 1, false)`

**Queue order:**
- L30: `this.addToBot(new VFXAction(p, new ShockWaveEffect(p.hb.cX, p.hb.cY, Settings.RED_TEXT_COLOR, ShockWaveEffect.ShockWaveType.ADDITIVE), 0.5f))`
- L31: `this.addToBot(new DrawCardAction(p, this.magicNumber))`
- L32: `this.addToBot(new PutOnDeckAction(p, p, 1, false))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new VFXAction(p, new ShockWaveEffect(p.hb.cX, p.hb.cY, Settings.RED_TEXT_COLOR, ShockWaveEffect.ShockWaveType.ADDITIVE), 0.5f));
        this.addToBot(new DrawCardAction(p, this.magicNumber));
        this.addToBot(new PutOnDeckAction(p, p, 1, false));
    }
```

</details>

## WaveOfTheHand
File: `cards\purple\WaveOfTheHand.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new WaveOfTheHandPower(p, this.magicNumber), this.magicNumber)`
- `WaveOfTheHandPower` — `new WaveOfTheHandPower(p, this.magicNumber)`

**Queue order:**
- L26: `this.addToBot(new ApplyPowerAction(p, p, new WaveOfTheHandPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new WaveOfTheHandPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Weave
File: `cards\purple\Weave.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L29: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT));
    }
```

</details>

## WellLaidPlans
File: `cards\green\WellLaidPlans.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new RetainCardPower(p, this.magicNumber), this.magicNumber)`
- `RetainCardPower` — `new RetainCardPower(p, this.magicNumber)`

**Queue order:**
- L27: `this.addToBot(new ApplyPowerAction(p, p, new RetainCardPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new RetainCardPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## WheelKick
File: `cards\purple\WheelKick.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `DrawCardAction` — `new DrawCardAction(p, this.magicNumber)`

**Queue order:**
- L30: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY))`
- L31: `this.addToBot(new DrawCardAction(p, this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_HEAVY));
        this.addToBot(new DrawCardAction(p, this.magicNumber));
    }
```

</details>

## Whirlwind
File: `cards\red\Whirlwind.java`

**Action sequence (in order):**
- `WhirlwindAction` — `new WhirlwindAction(p, this.multiDamage, this.damageTypeForTurn, this.freeToPlayOnce, this.energyOnUse)`

**Queue order:**
- L26: `this.addToBot(new WhirlwindAction(p, this.multiDamage, this.damageTypeForTurn, this.freeToPlayOnce, this.energyOnUse))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new WhirlwindAction(p, this.multiDamage, this.damageTypeForTurn, this.freeToPlayOnce, this.energyOnUse));
    }
```

</details>

## WhiteNoise
File: `cards\blue\WhiteNoise.java`

**Action sequence (in order):**
- `MakeTempCardInHandAction` — `new MakeTempCardInHandAction(c, true)`

**Queue order:**
- L28: `this.addToBot(new MakeTempCardInHandAction(c, true))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        AbstractCard c = AbstractDungeon.returnTrulyRandomCardInCombat(AbstractCard.CardType.POWER).makeCopy();
        c.setCostForTurn(0);
        this.addToBot(new MakeTempCardInHandAction(c, true));
    }
```

</details>

## WildStrike
File: `cards\red\WildStrike.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`
- `MakeTempCardInDrawPileAction` — `new MakeTempCardInDrawPileAction(new Wound(), 1, true, true)`
- `Wound` — `new Wound()`

**Queue order:**
- L32: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY))`
- L33: `this.addToBot(new MakeTempCardInDrawPileAction(new Wound(), 1, true, true))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.SLASH_HEAVY));
        this.addToBot(new MakeTempCardInDrawPileAction(new Wound(), 1, true, true));
    }
```

</details>

## WindmillStrike
File: `cards\purple\WindmillStrike.java`

**Action sequence (in order):**
- `DamageAction` — `new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT)`
- `DamageInfo` — `new DamageInfo(p, this.damage, this.damageTypeForTurn)`

**Queue order:**
- L36: `this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new DamageAction((AbstractCreature)m, new DamageInfo(p, this.damage, this.damageTypeForTurn), AbstractGameAction.AttackEffect.BLUNT_LIGHT));
    }
```

</details>

## Wish
File: `cards\purple\Wish.java`

**Action sequence (in order):**
- `None` — `new ArrayList<AbstractCard>()`
- `BecomeAlmighty` — `new BecomeAlmighty()`
- `FameAndFortune` — `new FameAndFortune()`
- `LiveForever` — `new LiveForever()`
- `ChooseOneAction` — `new ChooseOneAction(stanceChoices)`

**Queue order:**
- L43: `this.addToBot(new ChooseOneAction(stanceChoices))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        ArrayList<AbstractCard> stanceChoices = new ArrayList<AbstractCard>();
        stanceChoices.add(new BecomeAlmighty());
        stanceChoices.add(new FameAndFortune());
        stanceChoices.add(new LiveForever());
        if (this.upgraded) {
            for (AbstractCard c : stanceChoices) {
                c.upgrade();
            }
        }
        this.addToBot(new ChooseOneAction(stanceChoices));
    }
```

</details>

## Worship
File: `cards\purple\Worship.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new MantraPower(p, this.magicNumber), this.magicNumber)`
- `MantraPower` — `new MantraPower(p, this.magicNumber)`

**Queue order:**
- L28: `this.addToBot(new ApplyPowerAction(p, p, new MantraPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new MantraPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Wound
File: `cards\status\Wound.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
    }
```

</details>

## WraithForm
File: `cards\green\WraithForm.java`

**Action sequence (in order):**
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new IntangiblePlayerPower(p, this.magicNumber), this.magicNumber)`
- `IntangiblePlayerPower` — `new IntangiblePlayerPower(p, this.magicNumber)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new WraithFormPower(p, -1), -1)`
- `WraithFormPower` — `new WraithFormPower(p, -1)`

**Queue order:**
- L27: `this.addToBot(new ApplyPowerAction(p, p, new IntangiblePlayerPower(p, this.magicNumber), this.magicNumber))`
- L28: `this.addToBot(new ApplyPowerAction(p, p, new WraithFormPower(p, -1), -1))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        this.addToBot(new ApplyPowerAction(p, p, new IntangiblePlayerPower(p, this.magicNumber), this.magicNumber));
        this.addToBot(new ApplyPowerAction(p, p, new WraithFormPower(p, -1), -1));
    }
```

</details>

## WreathOfFlame
File: `cards\purple\WreathOfFlame.java`

**Action sequence (in order):**
- `VFXAction` — `new VFXAction(p, new FlameBarrierEffect(p.hb.cX, p.hb.cY), 0.1f)`
- `FlameBarrierEffect` — `new FlameBarrierEffect(p.hb.cX, p.hb.cY)`
- `VFXAction` — `new VFXAction(p, new FlameBarrierEffect(p.hb.cX, p.hb.cY), 0.5f)`
- `FlameBarrierEffect` — `new FlameBarrierEffect(p.hb.cX, p.hb.cY)`
- `ApplyPowerAction` — `new ApplyPowerAction(p, p, new VigorPower(p, this.magicNumber), this.magicNumber)`
- `VigorPower` — `new VigorPower(p, this.magicNumber)`

**Queue order:**
- L30: `this.addToBot(new VFXAction(p, new FlameBarrierEffect(p.hb.cX, p.hb.cY), 0.1f))`
- L32: `this.addToBot(new VFXAction(p, new FlameBarrierEffect(p.hb.cX, p.hb.cY), 0.5f))`
- L34: `this.addToBot(new ApplyPowerAction(p, p, new VigorPower(p, this.magicNumber), this.magicNumber))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        if (Settings.FAST_MODE) {
            this.addToBot(new VFXAction(p, new FlameBarrierEffect(p.hb.cX, p.hb.cY), 0.1f));
        } else {
            this.addToBot(new VFXAction(p, new FlameBarrierEffect(p.hb.cX, p.hb.cY), 0.5f));
        }
        this.addToBot(new ApplyPowerAction(p, p, new VigorPower(p, this.magicNumber), this.magicNumber));
    }
```

</details>

## Writhe
File: `cards\curses\Writhe.java`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
    }
```

</details>

## Zap
File: `cards\blue\Zap.java`

**Action sequence (in order):**
- `ChannelAction` — `new ChannelAction(new Lightning())`
- `Lightning` — `new Lightning()`

**Queue order:**
- L29: `this.addToBot(new ChannelAction(new Lightning()))`

<details><summary>Full use() body</summary>

```java
@Override
    public void use(AbstractPlayer p, AbstractMonster m) {
        for (int i = 0; i < this.magicNumber; ++i) {
            this.addToBot(new ChannelAction(new Lightning()));
        }
    }
```

</details>

