# STS Simulator Tools

`tools/` is the offline tooling layer, not runtime code.

## Directory Map

- root scripts
  - `campaign.ps1`: minimal launcher for `branch_campaign_driver`
  - `gap_panel.py`: small `branch_tiny` seed-panel runner
  - audit helpers for generated card/source data
- `data/`
  - curated support data such as card facts
- `ml/`
  - offline combat/search dataset and baseline utilities

## Output Rules

- generated reports and datasets belong under `tools/artifacts/`
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

## High-Value Tests

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\run_high_value_tests.ps1
```
