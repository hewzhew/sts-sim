# Hook Query: `atStartOfTurn`

## 1. Base Class Definition (1 signatures)

**Class**: `AbstractPower` — `powers\AbstractPower.java` L230

```java
public void atStartOfTurn() {
    }
```

## 2. Engine Call Sites (0)

*No call sites found outside base classes.*

## 3. Subclass Overrides (29)

| Class | Superclass | File | Lines | Status | Side Effects |
|-------|-----------|------|-------|--------|-------------|
| AbstractStance | None | `stances\AbstractStance.java` | 37-38 | ⚠️ DEAD | pure |
| BattleHymnPower | AbstractPower | `powers\watcher\BattleHymnPower.java` | 29-35 | ✅ | QUEUES_ACTIONS(addToBot) |
| BerserkPower | AbstractPower | `powers\BerserkPower.java` | 38-42 | ✅ | QUEUES_ACTIONS(addToBot) |
| BiasPower | AbstractPower | `powers\BiasPower.java` | 30-34 | ✅ | QUEUES_ACTIONS(addToBot) |
| ChokePower | AbstractPower | `powers\ChokePower.java` | 32-35 | ✅ | QUEUES_ACTIONS(addToBot) |
| CreativeAIPower | AbstractPower | `powers\CreativeAIPower.java` | 30-36 | ✅ | QUEUES_ACTIONS(addToBot) |
| DEPRECATEDDisciplinePower | AbstractPower | `powers\deprecated\DEPRECATEDDisciplinePower.java` | 36-44 | ✅ | QUEUES_ACTIONS(addToTop), MUTATES(this.amount), MUTATES(this.fontScale) |
| DivinityStance | AbstractStance | `stances\DivinityStance.java` | 50-53 | ✅ | QUEUES_ACTIONS(addToBot) |
| EchoPower | AbstractPower | `powers\EchoPower.java` | 39-42 | ✅ | MUTATES(this.cardsDoubledThisTurn) |
| EndTurnDeathPower | AbstractPower | `powers\watcher\EndTurnDeathPower.java` | 34-40 | ✅ | QUEUES_ACTIONS(addToBot) |
| EnergyDownPower | AbstractPower | `powers\watcher\EnergyDownPower.java` | 51-55 | ✅ | QUEUES_ACTIONS(addToBot) |
| FlameBarrierPower | AbstractPower | `powers\FlameBarrierPower.java` | 55-58 | ✅ | QUEUES_ACTIONS(addToBot) |
| FlightPower | AbstractPower | `powers\FlightPower.java` | 44-48 | ✅ | MUTATES(this.amount) |
| ForesightPower | AbstractPower | `powers\watcher\ForesightPower.java` | 33-40 | ✅ | QUEUES_ACTIONS(addToBot), QUEUES_ACTIONS(addToTop) |
| HelloPower | AbstractPower | `powers\HelloPower.java` | 30-38 | ✅ | QUEUES_ACTIONS(addToBot) |
| InfiniteBladesPower | AbstractPower | `powers\InfiniteBladesPower.java` | 29-35 | ✅ | QUEUES_ACTIONS(addToBot) |
| InvinciblePower | AbstractPower | `powers\InvinciblePower.java` | 44-48 | ✅ | MUTATES(this.amount) |
| LoopPower | AbstractPower | `powers\LoopPower.java` | 28-37 | ✅ | pure |
| MagnetismPower | AbstractPower | `powers\MagnetismPower.java` | 30-38 | ✅ | QUEUES_ACTIONS(addToBot) |
| MayhemPower | AbstractPower | `powers\MayhemPower.java` | 35-48 | ✅ | QUEUES_ACTIONS(addToBot), MUTATES(this.isDone) |
| NextTurnBlockPower | AbstractPower | `powers\NextTurnBlockPower.java` | 41-47 | ✅ | QUEUES_ACTIONS(addToBot) |
| NightmarePower | AbstractPower | `powers\NightmarePower.java` | 39-43 | ✅ | QUEUES_ACTIONS(addToBot) |
| PanachePower | AbstractPower | `powers\PanachePower.java` | 59-63 | ✅ | MUTATES(this.amount) |
| PhantasmalPower | AbstractPower | `powers\PhantasmalPower.java` | 35-40 | ✅ | QUEUES_ACTIONS(addToBot) |
| PoisonPower | AbstractPower | `powers\PoisonPower.java` | 58-64 | ✅ | QUEUES_ACTIONS(addToBot) |
| RechargingCorePower | AbstractPower | `powers\RechargingCorePower.java` | 42-53 | ⚠️ DEAD | QUEUES_ACTIONS(addToBot), MUTATES(this.turnTimer) |
| TimeMazePower | AbstractPower | `powers\TimeMazePower.java` | 56-59 | ⚠️ DEAD | MUTATES(this.amount) |
| WinterPower | AbstractPower | `powers\WinterPower.java` | 32-44 | ⚠️ DEAD | QUEUES_ACTIONS(addToBot) |
| WrathNextTurnPower | AbstractPower | `powers\watcher\WrathNextTurnPower.java` | 31-35 | ✅ | QUEUES_ACTIONS(addToBot) |

### AbstractStance `(())`

File: `stances\AbstractStance.java` L37-38

```java
public void atStartOfTurn() {
    }
```

### BattleHymnPower `(())` ⚠️ QUEUES_ACTIONS(addToBot)

File: `powers\watcher\BattleHymnPower.java` L29-35

```java
@Override
    public void atStartOfTurn() {
        if (!AbstractDungeon.getMonsters().areMonstersBasicallyDead()) {
            this.flash();
            this.addToBot(new MakeTempCardInHandAction((AbstractCard)new Smite(), this.amount, false));
        }
    }
```

### BerserkPower `(())` ⚠️ QUEUES_ACTIONS(addToBot)

File: `powers\BerserkPower.java` L38-42

```java
@Override
    public void atStartOfTurn() {
        this.addToBot(new GainEnergyAction(this.amount));
        this.flash();
    }
```

### BiasPower `(())` ⚠️ QUEUES_ACTIONS(addToBot)

File: `powers\BiasPower.java` L30-34

```java
@Override
    public void atStartOfTurn() {
        this.flash();
        this.addToBot(new ApplyPowerAction(this.owner, this.owner, new FocusPower(this.owner, -this.amount), -this.amount));
    }
```

### ChokePower `(())` ⚠️ QUEUES_ACTIONS(addToBot)

File: `powers\ChokePower.java` L32-35

```java
@Override
    public void atStartOfTurn() {
        this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
    }
```

### CreativeAIPower `(())` ⚠️ QUEUES_ACTIONS(addToBot)

File: `powers\CreativeAIPower.java` L30-36

```java
@Override
    public void atStartOfTurn() {
        for (int i = 0; i < this.amount; ++i) {
            AbstractCard card = AbstractDungeon.returnTrulyRandomCardInCombat(AbstractCard.CardType.POWER).makeCopy();
            this.addToBot(new MakeTempCardInHandAction(card));
        }
    }
```

### DEPRECATEDDisciplinePower `(())` ⚠️ QUEUES_ACTIONS(addToTop), MUTATES(this.amount), MUTATES(this.fontScale)

File: `powers\deprecated\DEPRECATEDDisciplinePower.java` L36-44

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

### DivinityStance `(())` ⚠️ QUEUES_ACTIONS(addToBot)

File: `stances\DivinityStance.java` L50-53

```java
@Override
    public void atStartOfTurn() {
        AbstractDungeon.actionManager.addToBottom(new ChangeStanceAction("Neutral"));
    }
```

### EchoPower `(())` ⚠️ MUTATES(this.cardsDoubledThisTurn)

File: `powers\EchoPower.java` L39-42

```java
@Override
    public void atStartOfTurn() {
        this.cardsDoubledThisTurn = 0;
    }
```

### EndTurnDeathPower `(())` ⚠️ QUEUES_ACTIONS(addToBot)

File: `powers\watcher\EndTurnDeathPower.java` L34-40

```java
@Override
    public void atStartOfTurn() {
        this.flash();
        this.addToBot(new VFXAction(new LightningEffect(this.owner.hb.cX, this.owner.hb.cY)));
        this.addToBot(new LoseHPAction(this.owner, this.owner, 99999));
        this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
    }
```

### EnergyDownPower `(())` ⚠️ QUEUES_ACTIONS(addToBot)

File: `powers\watcher\EnergyDownPower.java` L51-55

```java
@Override
    public void atStartOfTurn() {
        this.addToBot(new LoseEnergyAction(this.amount));
        this.flash();
    }
```

### FlameBarrierPower `(())` ⚠️ QUEUES_ACTIONS(addToBot)

File: `powers\FlameBarrierPower.java` L55-58

```java
@Override
    public void atStartOfTurn() {
        this.addToBot(new RemoveSpecificPowerAction((AbstractCreature)AbstractDungeon.player, (AbstractCreature)AbstractDungeon.player, POWER_ID));
    }
```

### FlightPower `(())` ⚠️ MUTATES(this.amount)

File: `powers\FlightPower.java` L44-48

```java
@Override
    public void atStartOfTurn() {
        this.amount = this.storedAmount;
        this.updateDescription();
    }
```

### ForesightPower `(())` ⚠️ QUEUES_ACTIONS(addToBot), QUEUES_ACTIONS(addToTop)

File: `powers\watcher\ForesightPower.java` L33-40

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

### HelloPower `(())` ⚠️ QUEUES_ACTIONS(addToBot)

File: `powers\HelloPower.java` L30-38

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

### InfiniteBladesPower `(())` ⚠️ QUEUES_ACTIONS(addToBot)

File: `powers\InfiniteBladesPower.java` L29-35

```java
@Override
    public void atStartOfTurn() {
        if (!AbstractDungeon.getMonsters().areMonstersBasicallyDead()) {
            this.flash();
            this.addToBot(new MakeTempCardInHandAction((AbstractCard)new Shiv(), this.amount, false));
        }
    }
```

### InvinciblePower `(())` ⚠️ MUTATES(this.amount)

File: `powers\InvinciblePower.java` L44-48

```java
@Override
    public void atStartOfTurn() {
        this.amount = this.maxAmt;
        this.updateDescription();
    }
```

### LoopPower `(())`

File: `powers\LoopPower.java` L28-37

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

### MagnetismPower `(())` ⚠️ QUEUES_ACTIONS(addToBot)

File: `powers\MagnetismPower.java` L30-38

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

### MayhemPower `(())` ⚠️ QUEUES_ACTIONS(addToBot), MUTATES(this.isDone)

File: `powers\MayhemPower.java` L35-48

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

### NextTurnBlockPower `(())` ⚠️ QUEUES_ACTIONS(addToBot)

File: `powers\NextTurnBlockPower.java` L41-47

```java
@Override
    public void atStartOfTurn() {
        this.flash();
        AbstractDungeon.effectList.add(new FlashAtkImgEffect(this.owner.hb.cX, this.owner.hb.cY, AbstractGameAction.AttackEffect.SHIELD));
        this.addToBot(new GainBlockAction(this.owner, this.owner, this.amount));
        this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
    }
```

### NightmarePower `(())` ⚠️ QUEUES_ACTIONS(addToBot)

File: `powers\NightmarePower.java` L39-43

```java
@Override
    public void atStartOfTurn() {
        this.addToBot(new MakeTempCardInHandAction(this.card, this.amount));
        this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, POWER_ID));
    }
```

### PanachePower `(())` ⚠️ MUTATES(this.amount)

File: `powers\PanachePower.java` L59-63

```java
@Override
    public void atStartOfTurn() {
        this.amount = 5;
        this.updateDescription();
    }
```

### PhantasmalPower `(())` ⚠️ QUEUES_ACTIONS(addToBot)

File: `powers\PhantasmalPower.java` L35-40

```java
@Override
    public void atStartOfTurn() {
        this.flash();
        this.addToBot(new ApplyPowerAction(this.owner, this.owner, new DoubleDamagePower(this.owner, 1, false), this.amount));
        this.addToBot(new ReducePowerAction(this.owner, this.owner, POWER_ID, 1));
    }
```

### PoisonPower `(())` ⚠️ QUEUES_ACTIONS(addToBot)

File: `powers\PoisonPower.java` L58-64

```java
@Override
    public void atStartOfTurn() {
        if (AbstractDungeon.getCurrRoom().phase == AbstractRoom.RoomPhase.COMBAT && !AbstractDungeon.getMonsters().areMonstersBasicallyDead()) {
            this.flashWithoutSound();
            this.addToBot(new PoisonLoseHpAction(this.owner, this.source, this.amount, AbstractGameAction.AttackEffect.POISON));
        }
    }
```

### RechargingCorePower `(())` ⚠️ QUEUES_ACTIONS(addToBot), MUTATES(this.turnTimer)

File: `powers\RechargingCorePower.java` L42-53

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

### TimeMazePower `(())` ⚠️ MUTATES(this.amount)

File: `powers\TimeMazePower.java` L56-59

```java
@Override
    public void atStartOfTurn() {
        this.amount = 15;
    }
```

### WinterPower `(())` ⚠️ QUEUES_ACTIONS(addToBot)

File: `powers\WinterPower.java` L32-44

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

### WrathNextTurnPower `(())` ⚠️ QUEUES_ACTIONS(addToBot)

File: `powers\watcher\WrathNextTurnPower.java` L31-35

```java
@Override
    public void atStartOfTurn() {
        this.addToBot(new ChangeStanceAction("Wrath"));
        this.addToBot(new RemoveSpecificPowerAction(this.owner, this.owner, this));
    }
```

## 4. Rust Current Status (0 refs)

*No references to `at_start_of_turn` or `atStartOfTurn` found in Rust source.*

