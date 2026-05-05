# RemoveDebuffsAction

**File**: `actions\unique\RemoveDebuffsAction.java`
**Category**: action
**Extends**: `AbstractGameAction`

## Method: `update()`
Lines 20–27

### Structured Logic

- **FOR EACH** `AbstractPower p : this.c.powers)`:
  - **IF** `(p.type != AbstractPower.PowerType.DEBUFF)`:
    - **CONTINUE**
  - `this.addToTop(new RemoveSpecificPowerAction(this.c, this.c, p.ID));`
- `this.isDone = true;`

### Call Chain

```
RemoveDebuffsAction.update()
  ├─ creates: RemoveSpecificPowerAction [addToTop] L24
```

## Rust Parity

❌ `RemoveDebuffsAction` → no Rust variant found
