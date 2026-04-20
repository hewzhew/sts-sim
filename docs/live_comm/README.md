# Live Comm

`live_comm` is the primary Rust versus Java acceptance loop for this repo.

Use it for:

- engine parity debugging
- protocol/importer validation
- human-assist handoff and noncombat audit experiments
- collecting real archived runs for later analysis

## Start Here

- [LIVE_COMM_RUNBOOK.md](LIVE_COMM_RUNBOOK.md)
  - day-to-day launch and triage workflow
- [LIVE_COMM_MODES.md](LIVE_COMM_MODES.md)
  - choose the run goal before picking watch settings
- [LIVE_COMM_PARITY_WORKFLOW.md](LIVE_COMM_PARITY_WORKFLOW.md)
  - strict versus survey parity rules
- [LIVE_COMM_MANUAL_SCENARIO_RUNBOOK.md](LIVE_COMM_MANUAL_SCENARIO_RUNBOOK.md)
  - manual `scenario` capture workflow for protocol truth samples
- [../../tools/live_comm/README.md](../../tools/live_comm/README.md)
  - launcher, profile, and config helper details

## Current Operating Model

1. pick a mode
   - `engine`
   - `survival`
   - `assisted_progression`
   - `handoff`
2. switch to a checked-in run profile
3. let `CommunicationMod` launch Rust through the PowerShell wrapper
4. archive the run under `logs/runs/<run_id>/`
5. triage from `focus.txt` and `findings.json`, not from raw logs first

Current high-value profiles:

- `Ironclad_Engine_Strict`
- `Ironclad_Engine_Survey`
- `Ironclad_Progression`
- `Ironclad_Assisted_Progression`
- `Ironclad_Assisted_Progression_BossHandoff`
- `Ironclad_HumanPrimary_Capture`

## Log Reading Defaults

Read artifacts in this order:

1. `focus.txt`
2. `findings.json`
3. `cargo run --bin sts_dev_tool -- logs inspect-findings ...`
4. `debug.txt`, `raw.jsonl`, `replay.json` only after narrowing to one finding family

If an older run predates `findings.json`, `inspect-findings` can synthesize a view
from `failure_snapshots.jsonl`.

## What Changed Recently

The active workflow assumes:

- per-run archival under `logs/runs/<run_id>/`
- stale `play.exe` detection in `tools/live_comm/launch_live_comm.ps1`
- protocol-truth continuation in survey mode instead of hidden-state carry
- structured sidecar audit output for human-primary noncombat screens

Historical handoff notes such as
[BOSSHANDOFF_LIVE_OBSERVATION_2026-04-16.md](BOSSHANDOFF_LIVE_OBSERVATION_2026-04-16.md)
still matter as context, but they are not the canonical workflow.
