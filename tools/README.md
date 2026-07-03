# STS Simulator Tools

`tools/` is the offline tooling layer, not runtime code.

## Directory Map

- `analysis/`
  - cache-first Java and parity analysis scripts
- `analysis_cache/`
  - machine-readable cached Java/protocol truth used by audits
- `artifacts/`
  - generated reports, datasets, coverage outputs, and other derived files
- `coverage/`
  - coverage dashboard and parsers
- `llm/`
  - experimental LLM controller adapters over public simulator observations and legal actions
- `live_comm/`
  - legacy Java bridge scripts and fixture-capture helpers
- `manual/`
  - hand-run helper scripts
- `rust_ast_extractor/`
  - Rust AST extraction helper crate
- `schema_builder/`
  - schema generation and comparison helpers
- `source_extractor/`
  - broad Java source report generation
- `sts_tool/`
  - primary structured analysis CLI

## Output Rules

- generated reports and datasets belong under `tools/artifacts/`
- cache files belong under `tools/analysis_cache/`
- live replay captures belong under `logs/replays/` or `logs/runs/`
- loose live-comm captures do not belong in the repo root
- root-level one-off snapshots belong under `tools/artifacts/root_snapshots/`

## Primary Campaign Workflow

The campaign architecture belongs to the Rust `branch_campaign_driver`
campaign application. `tools/campaign.ps1` is now a minimal launcher. It owns
only source selection, output allocation, and the smallest continuation path.
It must not own manifest, milestone, coverage-gap, report-shaping, or artifact
schema semantics.

```powershell
.\tools\campaign.ps1 -Mode quick
.\tools\campaign.ps1 -From latest -Continue -Mode quick -Rounds 2
.\tools\campaign.ps1 -From latest -Inspect
```

Normal runs write artifacts under `tools/artifacts/campaigns/runs/<run-id>/`
and update `tools/artifacts/campaigns/latest.json` through Rust-owned artifact
store logic. Scratch/latest shortcut semantics are retired from this wrapper.

The old `-More` shortcut is retired because it mixed source, output, and
round-budget semantics. Coverage-gap and milestone orchestration are not part
of this launcher.

See `docs/CURRENT_DIRECTION.md` for the current launcher boundary.

## Primary Java Analysis Workflow

```powershell
cd tools
python -m sts_tool cache
python -m sts_tool query ApplyPower
python -m sts_tool query ApplyPower --json
python -m sts_tool find Corruption
python -m sts_tool overrides onApplyPower
python -m sts_tool family power_lifecycle
python -m sts_tool inspect ApplyPower --method update
python hook_query.py onApplyPower
```

`analysis_cache/*.json` is the canonical machine-readable analysis layer.
Markdown reports are renderers over that cache.

## High-Value Tests

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\run_high_value_tests.ps1
powershell -ExecutionPolicy Bypass -File .\tools\run_high_value_tests.ps1 -IncludeParity
```

## Legacy

`source_extractor/` remains available for broad report rendering and
compatibility checks, but cache-backed `sts_tool` queries are the preferred first
stop when they cover the question.

`live_comm/` is also legacy. Use it only for fixture capture or historical
investigation unless the adapter is rebuilt under the boundary in
`docs_legacy/2026-06-03_pre_rewrite/docs/live_comm/LEGACY_FIXTURE_ONLY.md`.
