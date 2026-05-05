# CombustPower

**File**: `powers\CombustPower.java`
**Category**: power
**ID**: `"Combust"`
**Extends**: `AbstractPower`

## Method: `atEndOfTurn(boolean isPlayer)`
Lines 34–41

### Structured Logic

- **IF** `(!AbstractDungeon.getMonsters().areMonstersBasicallyDead())`:
  - `this.flash();`
  - `this.addToBot(new LoseHPAction(this.owner, this.owner, this.hpLoss, AbstractGameAction.AttackEffect.FIRE));`
  - `this.addToBot(new DamageAllEnemiesAction(null, DamageInfo.createDamageMatrix(this.amount, true), DamageInfo.DamageType.THORNS, AbstractGameAction.AttackEffect.FIRE));`

### Call Chain

```
CombustPower.atEndOfTurn()
  ├─ creates: LoseHPAction [addToBot] L38
  ├─ creates: DamageAllEnemiesAction [addToBot] L39
  ├─ calls: this.flash() L37
```

## Method: `stackPower(int stackAmount)`
Lines 43–48

### Structured Logic

- `this.fontScale = 8.0f;`
- `this.amount += stackAmount;`
- `++this.hpLoss;`

## Rust Parity

✅ `CombustPower` → `Combust` (dispatched in: `resolve_power_at_end_of_turn`)
