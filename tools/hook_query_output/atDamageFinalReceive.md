# Hook Query: `atDamageFinalReceive`

## 1. Base Class Definition (2 signatures)

**Class**: `AbstractPower` — `powers\AbstractPower.java` L206

```java
public float atDamageFinalReceive(float damage, DamageInfo.DamageType type) {
        return damage;
    }
```

**Class**: `AbstractPower` — `powers\AbstractPower.java` L222

```java
public float atDamageFinalReceive(float damage, DamageInfo.DamageType type, AbstractCard card) {
        return this.atDamageFinalReceive(damage, type);
    }
```

## 2. Engine Call Sites (3)

### `DamageInfo.applyEnemyPowersOnly()`

File: `cards\DamageInfo.java`

```java
// --- Line 108 ---
                this.isModified = true;
            }
            for (AbstractPower p : target.powers) {
>>>             tmp = p.atDamageFinalReceive(this.output, this.type);
                if (this.base == this.output) continue;
                this.isModified = true;
            }
```

**Iterates**: `target.powers` (ordered — sensitive to iteration order)

### `DamageInfo.applyPowers()`

File: `cards\DamageInfo.java`

```java
// --- Line 59 ---
                    this.isModified = true;
                }
                for (AbstractPower p : target.powers) {
>>>                 tmp = p.atDamageFinalReceive(tmp, this.type);
                    if (this.base == (int)tmp) continue;
                    this.isModified = true;
                }
// --- Line 87 ---
                    this.isModified = true;
                }
                for (AbstractPower p : target.powers) {
>>>                 tmp = p.atDamageFinalReceive(tmp, this.type);
                    if (this.base == (int)tmp) continue;
                    this.isModified = true;
                }
```

**Iterates**: `target.powers` (ordered — sensitive to iteration order)

## 3. Subclass Overrides (4)

| Class | Superclass | File | Lines | Status | Side Effects |
|-------|-----------|------|-------|--------|-------------|
| FlightPower | AbstractPower | `powers\FlightPower.java` | 50-53 | ✅ | pure |
| ForcefieldPower | AbstractPower | `powers\ForcefieldPower.java` | 33-39 | ⚠️ DEAD | pure |
| IntangiblePlayerPower | AbstractPower | `powers\IntangiblePlayerPower.java` | 36-42 | ✅ | pure |
| IntangiblePower | AbstractPower | `powers\IntangiblePower.java` | 38-44 | ✅ | pure |

### FlightPower `((float damage, DamageInfo.DamageType type))`

File: `powers\FlightPower.java` L50-53

```java
@Override
    public float atDamageFinalReceive(float damage, DamageInfo.DamageType type) {
        return this.calculateDamageTakenAmount(damage, type);
    }
```

### ForcefieldPower `((float damage, DamageInfo.DamageType type))`

File: `powers\ForcefieldPower.java` L33-39

```java
@Override
    public float atDamageFinalReceive(float damage, DamageInfo.DamageType type) {
        if (damage > 0.0f && type != DamageInfo.DamageType.HP_LOSS && type != DamageInfo.DamageType.THORNS) {
            return 0.0f;
        }
        return damage;
    }
```

### IntangiblePlayerPower `((float damage, DamageInfo.DamageType type))`

File: `powers\IntangiblePlayerPower.java` L36-42

```java
@Override
    public float atDamageFinalReceive(float damage, DamageInfo.DamageType type) {
        if (damage > 1.0f) {
            damage = 1.0f;
        }
        return damage;
    }
```

### IntangiblePower `((float damage, DamageInfo.DamageType type))`

File: `powers\IntangiblePower.java` L38-44

```java
@Override
    public float atDamageFinalReceive(float damage, DamageInfo.DamageType type) {
        if (damage > 1.0f) {
            damage = 1.0f;
        }
        return damage;
    }
```

## 4. Rust Current Status (0 refs)

*No references to `at_damage_final_receive` or `atDamageFinalReceive` found in Rust source.*

## 5. Parity Status

| Java Class | Rust PowerId | Status |
|------------|-------------|--------|
| FlightPower | Flight | ❌ MISSING (PowerId exists) |
| ForcefieldPower | Forcefield | ⚠️ DEAD |
| IntangiblePlayerPower | Intangible | ❌ MISSING (PowerId exists) |
| IntangiblePower | Intangible | ❌ MISSING (PowerId exists) |

