# Tools Audit

## Summary
- `sts_tool` is the primary interactive entrypoint and should remain the user-facing tool.
- `hook_query.py` is now a thin wrapper over the shared structured cache instead of owning a third parsing implementation.
- `source_extractor` remains valuable for broad report rendering, but it is no longer the recommended primary extraction workflow.
- The new canonical machine-readable layer lives under [analysis_cache](D:/rust/sts_simulator/tools/analysis_cache).
- Generated reports and reusable datasets should live under [artifacts](D:/rust/sts_simulator/tools/artifacts), not at the top level of `tools/`.

## Current Tools
| Tool | Role | Inputs | Outputs | Status |
|------|------|--------|---------|--------|
| `tools/sts_tool` | Primary Java→Rust query tool | Java source, Rust source | Markdown reports, structured cache, family audits | Keep as main entrypoint |
| `tools/hook_query.py` | Focused hook report | Shared cache | Markdown report | Keep as wrapper |
| `tools/source_extractor/sts_extractor.py` | Broad legacy extractor/report generator | Java source | Markdown reports, JSON artifacts | Keep, but legacy extraction path |
| `tools/coverage` | Coverage dashboard | `analysis_cache/*.json` (fallback: `source_extractor/output/*.json`) | HTML report | Keep, now cache-first |
| `tools/query_relics.py` | Schema/relic audit helper | `compiled_protocol_schema.json`, `hooks.json`, `relics.json`, `scattered_logic.json` | Console output | Keep, now JSON-only for query/audit/insertion |
| `tools/manual/combat_trace.py` | Manual replay trace renderer | replay JSONL | Console trace + assertions | Keep, but not part of main pipeline |
| `tools/legacy/*` | Historical helper scripts | old logs / replay JSONL | ad hoc converted outputs | Legacy only |
| `tools/ml_pipeline` | Experimental ML/data parsing | ad hoc JSON inputs | exploratory scripts | Experimental, not on main path |

## Findings
### `sts_tool`
- Strengths:
  - Already uses tree-sitter via [java_parser.py](D:/rust/sts_simulator/tools/sts_tool/java_parser.py).
  - Already has cross-file call-chain logic via [call_chain.py](D:/rust/sts_simulator/tools/sts_tool/call_chain.py).
  - Already closest to a unified user entrypoint.
- Gaps:
  - Previously emitted Markdown only.
  - Had no structured cache or family-audit subcommand.

### `hook_query.py`
- Previous state:
  - Duplicated AST traversal, override scanning, hardcoded-check extraction, liveness checks, and Rust parity mapping.
  - Mixed tree-sitter parsing with shell/regex heuristics.
- Current state:
  - Reduced to a wrapper that consumes [java_hooks.json](D:/rust/sts_simulator/tools/analysis_cache/java_hooks.json),
    [java_callsites.json](D:/rust/sts_simulator/tools/analysis_cache/java_callsites.json), and
    [rust_dispatch.json](D:/rust/sts_simulator/tools/analysis_cache/rust_dispatch.json).
- Recommendation:
  - Keep for compatibility, but do not add new parsing logic here.

### `source_extractor`
- Strengths:
  - Produces wide, high-value reports.
  - Still useful for manual reading and large-surface audits.
  - Now also emits `hooks.json`, `relics.json`, and `scattered_logic.json` sidecars for bridge compatibility.
- Gaps:
  - Markdown-first architecture.
  - Docs mentioned legacy pieces such as `dep_graph.py` that are not current.
  - Extraction logic is monolithic and not the best base for new targeted tooling.
- Recommendation:
  - Preserve high-value reports.
  - Treat it as a renderer/legacy extractor until its structured-fact phase is further decomposed.

## Drift Notes
- The old `source_extractor` README referred to `dep_graph.py`, but that script is not part of the current active workflow.
- `hook_query.py` was previously documented as a primary parser. That is no longer true; it now rides on the shared cache.
- `coverage/legacy_parsers/` now exists only as an isolation bucket for retired Markdown parsers.

## Structured IR
The first-stage canonical structured artifacts are:
- [java_entities.json](D:/rust/sts_simulator/tools/analysis_cache/java_entities.json)
- [java_hooks.json](D:/rust/sts_simulator/tools/analysis_cache/java_hooks.json)
- [java_callsites.json](D:/rust/sts_simulator/tools/analysis_cache/java_callsites.json)
- [rust_dispatch.json](D:/rust/sts_simulator/tools/analysis_cache/rust_dispatch.json)

Markdown reports are now renderers over these facts rather than the primary truth source.

## Next Consolidation Targets
1. Split `source_extractor/sts_extractor.py` into:
   - structured fact extraction
   - Markdown rendering
2. Add more family audits after `exhaust / guardian / vulnerable`.
3. Remove or isolate the remaining legacy Markdown parser modules once JSON-first has proven stable.

## Validation Tiers
- `python -m analysis.quick_smoke`
  - cache must already exist
  - validates `sts_tool`, `hook_query`, and one family audit
  - intended for frequent local use
- `python -m analysis.full_smoke`
  - rebuilds and re-renders the full toolchain path
  - includes `coverage.main` and `source_extractor`
  - intended for integration checks, not per-edit use
