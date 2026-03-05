---
description: Look up Java Slay the Spire source code for powers, cards, actions, and game mechanics
---

# Java STS Source Reference

The decompiled Java source for Slay the Spire is located at `C:\Dev\rust\cardcrawl`.

## Directory Structure

| Directory | Contents |
|-----------|----------|
| `powers/` | 124 power files (AbstractPower subclasses) |
| `powers/watcher/` | 26 Watcher-specific powers |
| `cards/` | AbstractCard.java + card subdirectories |
| `cards/red/` | Ironclad cards |
| `cards/green/` | Silent cards |
| `cards/blue/` | Defect cards |
| `cards/purple/` | Watcher cards |
| `cards/colorless/` | Colorless cards |
| `cards/curses/` | Curse cards |
| `cards/status/` | Status cards |
| `cards/tempCards/` | Generated/temp cards (Shiv, Smite, etc.) |
| `actions/` | Game actions (DamageAction, ApplyPowerAction, etc.) |
| `monsters/` | Monster AI and definitions |
| `relics/` | Relic implementations |
| `stances/` | Watcher stance implementations |
| `orbs/` | Defect orb implementations |

## Common Lookup Patterns

### Look up a specific power's hooks
```
// View the full power implementation
view_file C:\Dev\rust\cardcrawl\powers\{PowerName}Power.java

// Example: SharpHidePower
view_file C:\Dev\rust\cardcrawl\powers\SharpHidePower.java

// Watcher powers are in a subdirectory
view_file C:\Dev\rust\cardcrawl\powers\watcher\{PowerName}Power.java
```

### Find which powers implement a specific hook
```
// Find all powers that override onUseCard
grep_search "onUseCard" in C:\Dev\rust\cardcrawl\powers

// Find all powers that override atDamageGive
grep_search "atDamageGive" in C:\Dev\rust\cardcrawl\powers

// Common hook method names:
// - atDamageGive / atDamageReceive / atDamageFinalReceive
// - modifyBlock / modifyBlockLast
// - onAttacked / onAttackedToChangeDamage
// - onUseCard / onAfterUseCard
// - onCardDraw
// - onExhaust
// - atStartOfTurn / atStartOfTurnPostDraw
// - atEndOfTurn / atEndOfTurnPreEndTurnCards
// - onGainedBlock
// - wasHPLost
// - onAttack (after dealing damage)
// - onDeath
// - onApplyPower
// - stackPower
// - onRemove
```

### Look up a specific card
```
// Cards are organized by color:
// Red = Ironclad, Green = Silent, Blue = Defect, Purple = Watcher
view_file C:\Dev\rust\cardcrawl\cards\red\{CardName}.java
```

### Find what type a card is
```
// In card constructors, look for: this.type = CardType.SKILL / ATTACK / POWER
grep_search "CardType.SKILL" in C:\Dev\rust\cardcrawl\cards\red
```

## Key Java Patterns

### Card Type Check (what we're replicating in Rust)
Java powers check card type directly via the card instance:
```java
// In CorruptionPower.java:
public void onCardDraw(AbstractCard card) {
    if (card.type == AbstractCard.CardType.SKILL) {
        card.setCostForTurn(-9);
    }
}
```

### Power Hook Pattern
Every power extends `AbstractPower` and overrides specific hooks:
```java
public class SharpHidePower extends AbstractPower {
    @Override
    public void onUseCard(AbstractCard card, UseCardAction action) {
        if (card.type == AbstractCard.CardType.ATTACK) {
            // Deal this.amount damage to player (THORNS type)
            this.addToBot(new DamageAction(player, new DamageInfo(this.owner, this.amount, DamageType.THORNS), ...));
        }
    }
}
```
