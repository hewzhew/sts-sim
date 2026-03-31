# Hook Query: `onAfterCardPlayed`

## 1. Base Class Definition (1 signatures)

**Class**: `AbstractPower` — `powers\AbstractPower.java` L350

```java
public void onAfterCardPlayed(AbstractCard usedCard) {
    }
```

## 2. Engine Call Sites (1)

### `CardGroup.triggerOnOtherCardPlayed()`

File: `cards\CardGroup.java`

```java
// --- Line 983 ---
                c.triggerOnOtherCardPlayed(usedCard);
            }
            for (AbstractPower p : AbstractDungeon.player.powers) {
>>>             p.onAfterCardPlayed(usedCard);
            }
        }
    
```

**Iterates**: `AbstractDungeon.player.powers` (ordered — sensitive to iteration order)

## 3. Subclass Overrides (2)

| Class | Superclass | File | Lines | Status | Side Effects |
|-------|-----------|------|-------|--------|-------------|
| DEPRECATEDMasterRealityPower | AbstractPower | `powers\deprecated\DEPRECATEDMasterRealityPower.java` | 29-35 | ✅ | QUEUES_ACTIONS(addToBot) |
| ThousandCutsPower | AbstractPower | `powers\ThousandCutsPower.java` | 35-45 | ✅ | QUEUES_ACTIONS(addToBot) |

### DEPRECATEDMasterRealityPower `((AbstractCard card))` ⚠️ QUEUES_ACTIONS(addToBot)

File: `powers\deprecated\DEPRECATEDMasterRealityPower.java` L29-35

```java
@Override
    public void onAfterCardPlayed(AbstractCard card) {
        if (card.retain || card.selfRetain) {
            this.flash();
            this.addToBot(new DamageRandomEnemyAction(new DamageInfo(null, this.amount), AbstractGameAction.AttackEffect.FIRE));
        }
    }
```

### ThousandCutsPower `((AbstractCard card))` ⚠️ QUEUES_ACTIONS(addToBot)

File: `powers\ThousandCutsPower.java` L35-45

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

## 4. Rust Current Status (0 refs)

*No references to `on_after_card_played` or `onAfterCardPlayed` found in Rust source.*

## 5. Parity Status

| Java Class | Rust PowerId | Status |
|------------|-------------|--------|
| ThousandCutsPower | ThousandCuts | ❌ MISSING (no PowerId) |
| DEPRECATEDMasterRealityPower | DEPRECATEDMasterReality | ❌ MISSING (no PowerId) |

