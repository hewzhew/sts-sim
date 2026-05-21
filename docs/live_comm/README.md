# Live Comm

`live_comm` is legacy Java bridge tooling. It is currently downgraded to
fixture-capture and historical parity context only.

Do not use it as the AI, search, workbench, or policy-development mainline.
The active direction is local simulator/search/eval first; Java-connected
execution can be revived later as an adapter around a working local system.

Current boundary and revival plan:

- [LEGACY_FIXTURE_ONLY.md](LEGACY_FIXTURE_ONLY.md)

## Start Here

- [LEGACY_FIXTURE_ONLY.md](LEGACY_FIXTURE_ONLY.md)
  - current boundary, old responsibilities, and future revival architecture
- [LIVE_COMM_RUNBOOK.md](LIVE_COMM_RUNBOOK.md)
  - historical launch and triage workflow
- [LIVE_COMM_MODES.md](LIVE_COMM_MODES.md)
  - historical mode split
- [LIVE_COMM_PARITY_WORKFLOW.md](LIVE_COMM_PARITY_WORKFLOW.md)
  - historical strict versus survey parity rules
- [LIVE_COMM_MANUAL_SCENARIO_RUNBOOK.md](LIVE_COMM_MANUAL_SCENARIO_RUNBOOK.md)
  - manual `scenario` capture workflow for protocol truth samples
- [../../tools/live_comm/README.md](../../tools/live_comm/README.md)
  - legacy launcher, profile, and config helper details

## Legacy Operating Model

The old workflow was:

1. pick a mode
   - `engine`
   - `survival`
   - `assisted_progression`
   - `handoff`
2. switch to a checked-in run profile
3. let `CommunicationMod` launch Rust through the PowerShell wrapper
4. archive the run under `logs/runs/<run_id>/`
5. triage from `focus.txt` and `findings.json`, not from raw logs first

Historical high-value profiles:

- `Ironclad_Engine_Strict`
- `Ironclad_Engine_Survey`
- `Ironclad_Progression`
- `Ironclad_Assisted_Progression`
- `Ironclad_Assisted_Progression_BossHandoff`
- `Ironclad_HumanPrimary_Capture`

## Historical Log Reading Defaults

Read artifacts in this order:

1. `focus.txt`
2. `findings.json`
3. `cargo run --bin sts_dev_tool -- logs inspect-findings ...`
4. `debug.txt`, `raw.jsonl`, `replay.json` only after narrowing to one finding family

If an older run predates `findings.json`, `inspect-findings` can synthesize a view
from `failure_snapshots.jsonl`.

## Historical Notes

The old workflow assumed:

- per-run archival under `logs/runs/<run_id>/`
- stale `play.exe` detection in `tools/live_comm/launch_live_comm.ps1`
- protocol-truth continuation in survey mode instead of hidden-state carry
- structured sidecar audit output for human-primary noncombat screens

Historical handoff notes such as
[BOSSHANDOFF_LIVE_OBSERVATION_2026-04-16.md](BOSSHANDOFF_LIVE_OBSERVATION_2026-04-16.md)
still matter as context, but they are not the canonical workflow.
