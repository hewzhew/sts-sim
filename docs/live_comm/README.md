# Live Comm

Operational live communication docs live here.

Use this directory for:

- launch/runbook instructions
- mode selection and watch behavior
- parity workflow notes
- manual scenario capture workflow
- live comm schema/watch notes

Recent checkpoints:

- [BOSSHANDOFF_LIVE_OBSERVATION_2026-04-16.md](BOSSHANDOFF_LIVE_OBSERVATION_2026-04-16.md)
  - first explicit note that `BossHandoff` is operational as a human-assisted
    full-run mode, while still carrying tainted parity debt

Current log-reading defaults:

- start with archived `focus.txt` for grouped triage
- use `findings.json` for machine-readable families and counts
- use `cargo run --bin sts_dev_tool -- logs inspect-findings ...` to collapse from grouped findings to one bug family
- drop to `debug.txt` / `raw.jsonl` / `replay.json` only when following a specific finding
- if an older run has no archived `findings.json`, `inspect-findings` will synthesize a report from `failure_snapshots.jsonl`

Current launch defaults:

- `tools/live_comm/launch_live_comm.ps1` now checks whether the selected
  `play.exe` is stale relative to Rust build inputs
- stale repo-local binaries are rebuilt automatically before the game is allowed
  to attach
- run provenance now records:
  - compiled binary git sha
  - launcher repo-head sha
  - `binary_matches_head`
  - `binary_is_fresh`

These files are active workflow docs, but they are not repo-wide architecture entrypoints.
