# DeadBranch

**File**: `relics\DeadBranch.java`
**Category**: relic
**ID**: `"Dead Branch"`
**Extends**: `AbstractRelic`

## Method: `onExhaust(AbstractCard card)`
Lines 20–27

### Structured Logic

- **IF** `(!AbstractDungeon.getMonsters().areMonstersBasicallyDead())`:
  - `this.flash();`
  - `this.addToBot(new RelicAboveCreatureAction(AbstractDungeon.player, this));`
  - `this.addToBot(new MakeTempCardInHandAction(AbstractDungeon.returnTrulyRandomCardInCombat().makeCopy(), false));`

### Call Chain

```
DeadBranch.onExhaust()
  ├─ creates: RelicAboveCreatureAction [addToBot] L24
  ├─ creates: MakeTempCardInHandAction [addToBot] L25
  ├─ calls: this.flash() L23
```

## Rust Parity

✅ `DeadBranch` → `DeadBranch`
