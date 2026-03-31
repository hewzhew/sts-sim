# Hook Query: `canPlayCard`

## 1. Base Class Definition (1 signatures)

**Class**: `AbstractPower` — `powers\AbstractPower.java` L388

```java
public boolean canPlayCard(AbstractCard card) {
        return true;
    }
```

## 2. Engine Call Sites (1)

### `AbstractCard.hasEnoughEnergy()`

File: `cards\AbstractCard.java`

```java
// --- Line 853 ---
                return false;
            }
            for (AbstractPower p : AbstractDungeon.player.powers) {
>>>             if (p.canPlayCard(this)) continue;
                this.cantUseMessage = TEXT[13];
                return false;
            }
```

**Iterates**: `AbstractDungeon.player.powers` (ordered — sensitive to iteration order)

**Hardcoded checks in this method:**

- L857: `hasPower("Entangled")`

## 3. Subclass Overrides (1)

| Class | Superclass | File | Lines | Status | Side Effects |
|-------|-----------|------|-------|--------|-------------|
| NoSkillsPower | AbstractPower | `powers\watcher\NoSkillsPower.java` | 39-42 | ⚠️ DEAD | pure |

### NoSkillsPower `((AbstractCard card))`

File: `powers\watcher\NoSkillsPower.java` L39-42

```java
@Override
    public boolean canPlayCard(AbstractCard card) {
        return card.type != AbstractCard.CardType.SKILL;
    }
```

## 4. Rust Current Status (8 refs)

- `D:\rust\sts_simulator\src\content\powers\mod.rs:551:pub fn resolve_power_can_play_card(`
- `D:\rust\sts_simulator\src\content\cards\mod.rs:1629:pub fn can_play_card(card: &CombatCard, state: &CombatState) -> Result<(), &'static str> {`
- `D:\rust\sts_simulator\src\content\cards\mod.rs:1655:            if !crate::content::powers::resolve_power_can_play_card(ps.power_type, card) {`
- `D:\rust\sts_simulator\src\content\relics\mod.rs:770:        RelicId::VelvetChoker => {}, // Passive — engine checks can_play_card`
- `D:\rust\sts_simulator\src\content\relics\velvet_choker.rs:7:pub fn can_play_card(cards_played_this_turn: u32) -> bool {`
- `D:\rust\sts_simulator\src\content\cards\mod.rs:1651:    // Java: hasEnoughEnergy() — Power.canPlayCard() hook`
- `D:\rust\sts_simulator\src\content\cards\mod.rs:1662:    // This is separate from the canPlayCard hook; Java checks it explicitly.`
- `D:\rust\sts_simulator\src\content\powers\mod.rs:548:/// Java: AbstractPower.canPlayCard(AbstractCard) — returns false to block card play.`

## 5. Parity Status

| Java Class | Rust PowerId | Status |
|------------|-------------|--------|
| NoSkillsPower | NoSkills | ⚠️ DEAD |

