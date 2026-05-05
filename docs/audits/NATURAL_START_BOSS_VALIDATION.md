# Natural Start Boss Validation

This document marks the shift from state-only combat fixtures to a minimal
`natural start -> prefix actions -> candidate line preference` workflow.

## What Changed

- `CombatStartSpec` is the new natural combat-start entrypoint.
- `CombatAuthorSpec` stays in the repo, but its role is state-level regression
  and motif audit, not natural-start training.
- `combat_env_driver` now accepts `start_spec` in addition to `author_spec`,
  `fixture`, and `replay_raw/replay_frame`.
- `combat_boss_validate` compares 2-3 named candidate lines and returns:
  - `prefer_a`
  - `prefer_b`
  - `close_enough`

## Current Validation Pack

Path:

- `data/boss_validation/hexaghost_v1/`

Contents:

- `start_spec.json`
- `state_case_h1_disarm_now.json`
- `state_case_h2_reduce_pressure_vs_race.json`
- `state_case_h3_close_enough.json`

Purpose:

- verify that the current method can prefer obvious pressure relief in a boss
  window
- verify that the current method can avoid forcing a fake unique answer in a
  gray window

## Explicit Non-Goals

- no PPO
- no learner
- no belief repair
- no full-boss benchmark sweep
- no expansion to `Evolve` / `Fire Breathing`

## Immediate CLI Examples

```powershell
cargo run --bin combat_env_driver -- --initial-start-spec data/boss_validation/hexaghost_v1/start_spec.json
```

```powershell
cargo run --bin combat_boss_validate -- --case data/boss_validation/hexaghost_v1/state_case_h1_disarm_now.json
```

```powershell
cargo run --bin combat_boss_validate -- `
  --case data/boss_validation/hexaghost_v1/state_case_h1_disarm_now.json `
  --jsonl-out tools/artifacts/boss_validation/ledger.jsonl
```
