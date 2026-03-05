---
description: Run card/power validation checks after modifying card JSONs, powers, or hooks
---

# Card & Power Validation

Run after modifying card JSONs, powers.json, hooks.rs, or any power-related code.

## Quick validation
// turbo
1. Run the validation checks:
```
python data/scripts/card_db.py validate
```

## Full cross-reference queries

2. Check all Power cards and their ApplyPower status:
```
python data/scripts/card_db.py power-cards
```

3. Check TurnTrigger cards specifically:
```
python data/scripts/card_db.py turn-trigger
```

4. Cross-reference power names between card JSON and hooks.rs:
```
python data/scripts/card_db.py power-names
```

## Re-extract powers spec from Java source

5. Only needed when adding new powers or fixing spec issues:
```
python data/scripts/extract_powers.py --cards
```

## Naming conventions

- **Power names in card JSON ApplyPower**: Use the Java `POWER_ID` string (e.g. `"Demon Form"` not `"DemonForm"`)
- **Check hooks.rs `from_str`**: The name must resolve there. Both camelCase and spaced forms are usually accepted
- **Stack values**: Must match what the Java card passes to `ApplyPowerAction` (4th arg), NOT the power description
  - Example: Combust stacks = `this.magicNumber` (5/7), NOT hpLoss (1)
