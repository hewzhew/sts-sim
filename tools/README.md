# STS Simulator — Tools

## Active Tools

### `sts_tool/` ⭐ Unified Porting Assistant (NEW)

All-in-one Java→Rust porting tool. Tree-sitter based, zero hardcoded mappings.

```powershell
# Full report: structured AST + call chain + Rust parity
python -m sts_tool query ApplyPower

# Just AST extraction for a single method
python -m sts_tool ast RemoveDebuffs

# Find all overrides of a hook
python -m sts_tool overrides onApplyPower

# Find Java files by name
python -m sts_tool find Corruption
```

**Output**: `tools/sts_tool/output/<name>_report.md`

**Dependencies**: `tree-sitter`, `tree-sitter-java`, `ripgrep (rg)`

---

### `hook_query.py` ⭐ Focused Hook Analysis

Given a Java hook name, extracts all implementation context.

```powershell
python tools/hook_query.py onApplyPower
```

**Output**: `tools/hook_query_output/<hook_name>.md`

---

### `coverage/` ⭐ Coverage Dashboard

Automated Power/Relic hook coverage comparison (Java vs Rust), generates an HTML report.

```powershell
python -m tools.coverage.main
```

**Output**: `tools/coverage_report.html`

---

### `convert_logs.py` — Trace Converter

Converts Java CommunicationMod logs to `.jsonl` replay files for `diff_driver`.

### `validate_transitions.py` — State Validator

Validates state transition consistency in replay files.

### `combat_trace.py` — Combat Flow Renderer

Parses replay files into human-readable combat flow.

---

## Data Files

| File | Purpose | Status |
|------|---------|--------|
| `protocol_schema.json` | Java Mod communication protocol | ✅ diff_driver dependency |
| `upgrade_deltas.json` | Card upgrade stat changes | ✅ Card definition dependency |
| `source_extractor/output/hooks.md` | All Power/Relic/Card hook overrides | ✅ coverage/ data source |
| `source_extractor/output/scattered_logic.md` | Engine-side hasRelic/hasPower checks | ✅ Critical reference |
| `source_extractor/AGENT_GUIDE.md` | AI agent operating protocol | ✅ Workflow reference |

## Dependencies

```powershell
# Python packages
pip install tree-sitter tree-sitter-java

# CLI tools (via cargo)
cargo install ripgrep   # rg — fast text search
```
