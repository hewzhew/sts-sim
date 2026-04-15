# STS Simulator Tools

`tools/` is now treated as the offline tooling layer, not as an extension of the runtime code.

## Directory Map

- `analysis/`
  - cache-first Java and parity analysis scripts
- `analysis_cache/`
  - machine-readable cached truth used by renderers and audits
- `artifacts/`
  - generated reports, datasets, coverage outputs, and other derived files
- `combat_lab/`
  - local batch helpers for combat-lab style experiments
- `coverage/`
  - coverage dashboard and parsers
- `learning/`
  - RL / dataset build scripts and learning-side utilities
- `legacy/`
  - retained old scripts and retired implementation snapshots
- `live_comm/`
  - launch scripts, profiles, and operational helpers
- `manual/`
  - hand-run helper scripts
- `replays/`
  - stored replay inputs
- `rust_ast_extractor/`
  - Rust AST extraction helper crate
- `schema_builder/`
  - schema generation and comparison helpers
- `source_extractor/`
  - broader Java source report generation
- `sts_tool/`
  - primary structured analysis CLI

## Output Rules

- generated reports and datasets belong under `tools/artifacts/`
- cache files belong under `tools/analysis_cache/`
- replay inputs belong under `tools/replays/`
- loose live-comm captures do not belong in the repo root; they now live under `logs/live_comm/`
- root-level one-off snapshots such as `coverage.json` or `ledger.jsonl` belong under `tools/artifacts/root_snapshots/`

## Primary Workflow

The Java analysis toolchain is cache-first:

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

`analysis_cache/*.json` is the canonical machine-readable truth layer. Markdown reports are renderers over that cache.

## Active Tools

### `sts_tool/`

Primary entrypoint for Java→Rust analysis.

Commands:
- `cache`
- `query`
- `find`
- `overrides`
- `family`
- `inspect`

### `hook_query.py`

Thin wrapper that renders a focused hook report from the shared cache.

### `analysis.quick_smoke` / `analysis.full_smoke`

Validation tiers for the cache-first workflow.

```powershell
python -m analysis.quick_smoke
python -m analysis.full_smoke
```

### `analysis.live_regression`

Live log extraction and minimization for `live_comm` fixtures.

### `analysis.bugfix_workflow`

Opinionated wrapper over `analysis.live_regression` for parity bug work.

### `learning/`

Dataset-build and learning-side workflow for future RL work.

## Canonical Artifacts

Machine-readable:
- `analysis_cache/java_entities.json`
- `analysis_cache/java_methods.json`
- `analysis_cache/java_hooks.json`
- `analysis_cache/java_callsites.json`
- `analysis_cache/rust_dispatch.json`
- `analysis_cache/schema_aliases.json`
- `analysis_cache/manifest.json`
- `compiled_protocol_schema.json`

Rendered:
- `analysis_cache/family_audit/<family>.json`
- `analysis_cache/family_audit/<family>.md`
- `artifacts/hook_query_output/<hook>.md`
- `artifacts/coverage_report.html`

## Legacy

`source_extractor/` remains available for broad report rendering and compatibility checks, but it is not the preferred first stop when cache-backed analysis exists.
