# Tools Audit

## Summary

- `sts_tool` is the primary user-facing Java analysis entrypoint.
- `analysis_cache/*.json` is the canonical fact layer.
- `hook_query.py` is a thin cache consumer.
- `source_extractor` is legacy and should not own new workflow decisions.
- Markdown outputs are renderers, not truth.

## Canonical Files

- [java_entities.json](D:/rust/sts_simulator/tools/analysis_cache/java_entities.json)
- [java_methods.json](D:/rust/sts_simulator/tools/analysis_cache/java_methods.json)
- [java_hooks.json](D:/rust/sts_simulator/tools/analysis_cache/java_hooks.json)
- [java_callsites.json](D:/rust/sts_simulator/tools/analysis_cache/java_callsites.json)
- [rust_dispatch.json](D:/rust/sts_simulator/tools/analysis_cache/rust_dispatch.json)
- [schema_aliases.json](D:/rust/sts_simulator/tools/analysis_cache/schema_aliases.json)
- [manifest.json](D:/rust/sts_simulator/tools/analysis_cache/manifest.json)

## Active Roles

| Tool | Role | Status |
|------|------|--------|
| `python -m sts_tool cache` | Build unified cache | Primary |
| `python -m sts_tool query` | Cache-backed entity query | Primary |
| `python -m sts_tool family` | Cache-backed family audit | Primary |
| `python tools/hook_query.py` | Hook report renderer | Primary thin wrapper |
| `python -m analysis.quick_smoke` | Fast cache workflow validation | Primary |
| `python -m analysis.full_smoke` | Full integration validation | Primary |
| `source_extractor/sts_extractor.py` | Broad legacy extraction | Legacy |

## Drift Rules

- Do not add new primary parsing logic outside cache build.
- If a query/family report needs new facts, extend `cache_builder` instead of reparsing Java in the consumer.
- When workflow commands or cache files change, update:
  - `tools/README.md`
  - `tools/analysis_cache/README.md`
  - `tools/source_extractor/README.md`
  - `tools/source_extractor/AGENT_GUIDE.md`

## Remaining Consolidation

1. Continue reducing legacy dependencies in bridge helpers such as `query_relics.py`.
2. Extend family reports now that `java_methods.json` exists.
3. Keep `source_extractor` for compatibility until all needed structured facts have first-class cache equivalents.
