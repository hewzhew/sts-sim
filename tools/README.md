# STS Simulator — Tools

## Active Tools

### `sts_tool/` ⭐ Unified Porting Assistant

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

# Build the structured cache
python -m sts_tool cache

# Build a bug-family audit
python -m sts_tool family exhaust
```

**Primary outputs**:
- `tools/sts_tool/output/<name>_report.md`
- `tools/analysis_cache/*.json`
- `tools/analysis_cache/family_audit/<family>.{json,md}`
- `tools/artifacts/*` for reusable generated reports

**Dependencies**: `tree-sitter`, `tree-sitter-java`, `ripgrep (rg)`

---

### `hook_query.py` ⭐ Focused Hook Analysis

Given a Java hook name, renders a focused report from the shared structured cache.

```powershell
python tools/hook_query.py onApplyPower
```

**Output**: `tools/artifacts/hook_query_output/<hook_name>.md`

### `family_audit.py` ⭐ Bug Family Audit

Targeted structured audit for parity-risk families.

```powershell
python tools/family_audit.py guardian
```

**Output**:
- `tools/analysis_cache/family_audit/<family>.json`
- `tools/analysis_cache/family_audit/<family>.md`

### `analysis.quick_smoke` / `analysis.full_smoke`

Validation is now split by cost:

```powershell
# Fast path for day-to-day development
python -m analysis.quick_smoke

# Full integration pass (minutes, not seconds)
python -m analysis.full_smoke
```

---

### `coverage/` ⭐ Coverage Dashboard

Automated Power/Relic hook coverage comparison (Java vs Rust), generates an HTML report.
Now prefers `analysis_cache/*.json` and falls back to `source_extractor/output/*.json` only when cache artifacts are missing.

```powershell
python -m tools.coverage.main
```

**Output**: `tools/artifacts/coverage_report.html`

Legacy Markdown parsers now live under:
- `tools/coverage/legacy_parsers/`
- for reference only, not normal execution

---

### `manual/combat_trace.py` — Manual Combat Trace Tool

Parses replay files into a human-readable combat flow for manual debugging.

### `legacy/`

Historical helper scripts that are no longer on the main tools path:
- `legacy/convert_logs.py`
- `legacy/validate_transitions.py`

### `ml_pipeline/` — Experimental

Prototype ML/data-prep area. Not part of the current primary `tools/` workflow.

---

## Data Files

| File | Purpose | Status |
|------|---------|--------|
| `protocol_schema_baseline.json` | Historical/manual baseline for Java↔Rust ID mappings | ✅ schema compiler input |
| `compiled_protocol_schema.json` | Runtime-consumed compiled mapping artifact | ✅ diff_driver/runtime dependency |
| `analysis_cache/java_hooks.json` | Canonical structured hook facts | ✅ new cache layer |
| `analysis_cache/java_callsites.json` | Canonical structured hook callsites | ✅ new cache layer |
| `analysis_cache/rust_dispatch.json` | Canonical Rust dispatch facts | ✅ new cache layer |
| `artifacts/observed_ids.json` | Generated observed-ID corpus | ✅ machine-readable artifact |
| `artifacts/interaction_coverage.json` | Generated interaction coverage dataset | ✅ machine-readable artifact |
| `artifacts/interaction_coverage_report.json` | Generated interaction coverage summary | ✅ human/machine report |
| `artifacts/coverage_report.html` | Generated coverage dashboard | ✅ rendered artifact |
| `artifacts/hook_query_output/*` | Generated hook query reports | ✅ rendered artifact |
| `artifacts/monster_audit_output/*` | Archived audit reports | ✅ rendered artifact |
| `source_extractor/output/hooks.md` | Legacy rendered hook summary | ✅ human reference only |
| `source_extractor/output/scattered_logic.md` | Legacy rendered engine-side checks | ✅ human reference only |
| `source_extractor/output/hooks.json` | Legacy extractor structured hook sidecar | ✅ machine-readable bridge artifact |
| `source_extractor/output/relics.json` | Legacy extractor structured relic sidecar | ✅ machine-readable bridge artifact |
| `source_extractor/output/scattered_logic.json` | Legacy extractor structured scattered-logic sidecar | ✅ machine-readable bridge artifact |
| `source_extractor/AGENT_GUIDE.md` | AI agent operating protocol | ✅ Workflow reference |
| `TOOLS_AUDIT.md` | Current tool roles, overlap, and drift notes | ✅ consolidation guide |

## Dependencies

```powershell
# Python packages
pip install tree-sitter tree-sitter-java

# CLI tools (via cargo)
cargo install ripgrep   # rg — fast text search
```
### `query_relics.py` — Schema / Relic Audit Helper

Lookup and audit helper for schema-mapped entities.
`query` and `audit` now prefer:
- `source_extractor/output/hooks.json`
- `source_extractor/output/relics.json`
- `source_extractor/output/scattered_logic.json`

`check-insertion` now also prefers `source_extractor/output/relics.json` for Java queue insertion facts.
