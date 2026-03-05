---
description: Run relic implementation audit - cross-references Java sources with Rust code to show what's implemented vs missing
---

# Relic Implementation Audit

// turbo-all

## Full Audit Report

Generate the complete status report for all relics:

```
python .agent/scripts/audit_relics.py
```

## Missing Combat Relics Only

Show only unimplemented combat-relevant relics:

```
python .agent/scripts/audit_relics.py --missing-only --combat-only
```

## Filter by Tier

Show relics of a specific tier (Starter, Common, Uncommon, Rare, Boss, Shop, Event, Special):

```
python .agent/scripts/audit_relics.py --tier Common
```

## JSON Output

Get machine-readable output for further processing:

```
python .agent/scripts/audit_relics.py --json --missing-only --combat-only
```

## Notes

- The script scans `c:\Dev\rust\cardcrawl\relics\*.java` for Java sources
- Cross-references with all `*.rs` files under `c:\Dev\rust\sts_sim\src\`
- Also checks `data/relics_patched.json` for data-driven relic hooks
- Status categories:
  - ✅ **hardcoded**: Logic in `trigger_hardcoded_relic_standalone()` match arms
  - ✅ **pre-loop**: Logic in `trigger_relics()` pre-loop section (needs GameState access)
  - ✅ **inline**: Logic inline in state.rs, commands.rs, combat.rs, etc.
  - ✅ **json-data**: Relic has hooks defined in relics_patched.json
  - ⚠️ **ref-only**: Referenced in code but no complete logic
  - ❌ **missing**: Not referenced anywhere in Rust code
