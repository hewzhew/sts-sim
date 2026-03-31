# Hook Query: `atDamageFinalGive`

## 1. Base Class Definition (2 signatures)

**Class**: `AbstractPower` — `powers\AbstractPower.java` L202

```java
public float atDamageFinalGive(float damage, DamageInfo.DamageType type) {
        return damage;
    }
```

**Class**: `AbstractPower` — `powers\AbstractPower.java` L218

```java
public float atDamageFinalGive(float damage, DamageInfo.DamageType type, AbstractCard card) {
        return this.atDamageFinalGive(damage, type);
    }
```

## 2. Engine Call Sites (2)

### `DamageInfo.applyPowers()`

File: `cards\DamageInfo.java`

```java
// --- Line 54 ---
                    this.isModified = true;
                }
                for (AbstractPower p : owner.powers) {
>>>                 tmp = p.atDamageFinalGive(tmp, this.type);
                    if (this.base == (int)tmp) continue;
                    this.isModified = true;
                }
// --- Line 82 ---
                    this.isModified = true;
                }
                for (AbstractPower p : owner.powers) {
>>>                 tmp = p.atDamageFinalGive(tmp, this.type);
                    if (this.base == (int)tmp) continue;
                    this.isModified = true;
                }
```

**Iterates**: `owner.powers` (ordered — sensitive to iteration order)

## 3. Subclass Overrides (0)

*No subclass overrides found.*

## 4. Rust Current Status (0 refs)

*No references to `at_damage_final_give` or `atDamageFinalGive` found in Rust source.*

