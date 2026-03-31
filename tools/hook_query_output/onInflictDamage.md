# Hook Query: `onInflictDamage`

## 1. Base Class Definition (1 signatures)

**Class**: `AbstractPower` — `powers\AbstractPower.java` L273

```java
public void onInflictDamage(DamageInfo info, int damageAmount, AbstractCreature target) {
    }
```

## 2. Engine Call Sites (1)

### `AbstractPlayer.damage()`

File: `characters\AbstractPlayer.java`

```java
// --- Line 1435 ---
                }
                if (info.owner != null) {
                    for (AbstractPower abstractPower : info.owner.powers) {
>>>                     abstractPower.onInflictDamage(info, damageAmount, this);
                    }
                }
                if (info.owner != this) {
```

**Iterates**: `info.owner.powers` (ordered — sensitive to iteration order)

**Hardcoded checks in this method:**

- L1378: `hasPower("IntangiblePlayer")`
- L1466: `hasRelic("Mark of the Bloom")`
- L1476: `hasRelic("Lizard Tail")`

## 3. Subclass Overrides (1)

| Class | Superclass | File | Lines | Status | Side Effects |
|-------|-----------|------|-------|--------|-------------|
| PainfulStabsPower | AbstractPower | `powers\PainfulStabsPower.java` | 36-41 | ✅ | QUEUES_ACTIONS(addToBot) |

### PainfulStabsPower `((DamageInfo info, int damageAmount, AbstractCreature target))` ⚠️ QUEUES_ACTIONS(addToBot)

File: `powers\PainfulStabsPower.java` L36-41

```java
@Override
    public void onInflictDamage(DamageInfo info, int damageAmount, AbstractCreature target) {
        if (damageAmount > 0 && info.type != DamageInfo.DamageType.THORNS) {
            this.addToBot(new MakeTempCardInDiscardAction((AbstractCard)new Wound(), 1));
        }
    }
```

## 4. Rust Current Status (0 refs)

*No references to `on_inflict_damage` or `onInflictDamage` found in Rust source.*

## 5. Parity Status

| Java Class | Rust PowerId | Status |
|------------|-------------|--------|
| PainfulStabsPower | PainfulStabs | ❌ MISSING (PowerId exists) |

