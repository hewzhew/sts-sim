---
description: How to synchronize Parser mappings with protocol schema when encountering missing monsters or powers in the Parity Report.
---

# Parity Mapping Sync Workflow

When running `batch_parity_checker.py` via `/monster_ai_parity` or manually against a new set of Java source files, you might encounter falsely reported missing mappings like "Unknown Mapping for AbstractGameAction", or missing rust files because the Java class name does not map naturally to the Rust module name (e.g. `SpikeSlime_S` not mapping to `spike_slime_s.rs`).

Follow these steps to synchronize the single-source-of-truth schema:

1. **Identify the missing mapping**:
   - Run `batch_parity_checker.py --src d:\rust\cardcrawl\monsters\<layer> --out report.md`.
   - Open `report.md` and look for `:x: Expected Rust file ... not found!` or `Unknown Mapping`.

2. **Update Schema**:
   Open `d:\rust\sts_simulator\tools\protocol_schema.json`.
   
   **For Monsters:**
   - Locate the `enums -> monster_id -> entries` block.
   - Find the Rust Enum Variant name that corresponds to the intended Rust logic.
   - Insert the raw Java class name into the `"java": [...]` array.
   - Example: If Java name is `SpikeSlime_S` and Rust variant is `SpikeSlimeS`, add `"SpikeSlime_S"` to `SpikeSlimeS`'s `java` array.
   
   **For Powers:**
   - Locate the `enums -> power_id -> entries` block.
   - Locate the missing Java Power class name (prefix only or full `XXXPower`).
   - Add it to the corresponding Rust variant's `"java": [...]` alias list.

3. **Re-run the Checker**:
// turbo
python d:\rust\sts_simulator\tools\source_extractor\batch_parity_checker.py --src d:\rust\cardcrawl\monsters\city --out city_batch_report.md
   - And the checker will automatically resolve the name from `protocol_schema.json`.

**Rule**: NEVER hardcode aliases into `batch_parity_checker.py` directly. Always update `protocol_schema.json`.
