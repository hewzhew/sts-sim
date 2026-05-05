# SneckoEye

**File**: `relics\SneckoEye.java`
**Category**: relic
**ID**: `"Snecko Eye"`
**Extends**: `AbstractRelic`

## Method: `onEquip()`
Lines 25–28

### Structured Logic

- `AbstractDungeon.player.masterHandSize += 2;`

## Method: `onUnequip()`
Lines 30–33

### Structured Logic

- `AbstractDungeon.player.masterHandSize -= 2;`

## Method: `atPreBattle()`
Lines 35–39

### Structured Logic

- `this.flash();`
- `this.addToBot(new ApplyPowerAction(AbstractDungeon.player, AbstractDungeon.player, new ConfusionPower(AbstractDungeon.player)));`

### Call Chain

```
SneckoEye.atPreBattle()
  ├─ creates: ApplyPowerAction [addToBot] L38
  ├─ creates: ConfusionPower [addToBot] L38
  ├─ calls: this.flash() L37
```

## Rust Parity

✅ `SneckoEye` → `SneckoEye`
