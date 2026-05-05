# CorruptionPower

**File**: `powers\CorruptionPower.java`
**Category**: power
**ID**: `"Corruption"`
**Extends**: `AbstractPower`

## Method: `onCardDraw(AbstractCard card)`
Lines 34–39

### Structured Logic

- **IF** `(card.type == AbstractCard.CardType.SKILL)`:
  - `card.setCostForTurn(-9);`

## Method: `onUseCard(AbstractCard card, UseCardAction action)`
Lines 41–47

### Structured Logic

- **IF** `(card.type == AbstractCard.CardType.SKILL)`:
  - `this.flash();`
  - `action.exhaustCard = true;`

## Rust Parity

✅ `CorruptionPower` → `Corruption` (dispatched in: `resolve_power_on_card_draw`)
