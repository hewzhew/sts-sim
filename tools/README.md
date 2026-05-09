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
- `learning/`
  - DecisionRecord collection, contract audit, and replay verification only
- `live_comm/`
  - launch scripts, profiles, and operational helpers
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

## DecisionRecord Tools

```powershell
python tools\learning\collect_decision_records.py `
  --out tmp\decision_records\records.jsonl `
  --episodes 1 `
  --seed-start 1 `
  --max-steps 500

python tools\learning\audit_decision_record_contract.py `
  --input tmp\decision_records\records.jsonl

python tools\learning\verify_decision_records_replay.py `
  --inputs tmp\decision_records\records.jsonl `
  --max-steps 500 `
  --fail-on-mismatch
```

Replay verification must use the same env config as collection; `max_steps`,
class, ascension, and final-act status are part of the replayed state hash.

These scripts do not create teacher labels or policy preferences.

## Legacy

`source_extractor/` remains available for broad report rendering and
compatibility checks, but cache-backed `sts_tool` queries are the preferred first
stop when they cover the question.
