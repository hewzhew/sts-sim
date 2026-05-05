# Source Extractor (Legacy)

`source_extractor/` is no longer the primary Java analysis workflow.

Use the cache-first path instead:

```powershell
cd ..\tools
python -m sts_tool cache
python -m sts_tool query ApplyPower
python -m sts_tool family power_lifecycle
python hook_query.py onApplyPower
```

Canonical truth now lives under:
- `tools/analysis_cache/*.json`

`source_extractor/output/*.json` and Markdown reports remain useful for:
- migration gaps
- broad human browsing
- bridge compatibility with older scripts

They should not be treated as the primary truth layer for new tooling.
