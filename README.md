# sts_simulator

Rust Slay the Spire simulator and parity tooling.

This repo is not a strong AI agent today. The current useful work is narrower:

- keep the simulator and replay surfaces deterministic
- compare Rust behavior against Java/CommunicationMod truth through `live_comm`
- capture legal observations, action candidates, transitions, and run outcomes
- keep combat search diagnostic code separate from macro-decision evidence

Old weak-evidence learning paths have been removed from the active tree. In
particular, BranchTrace/candidate rollout labels, verified teacher overrides,
DecisionRecord teacher labels, PPO/Gym bridges, single-seed policy patch
pipelines, and hand-written macro-policy modules are not active project
direction.

## What Is Authoritative

- Java source in `../cardcrawl/`
- this repo's `CommunicationMod` fork in `../CommunicationMod/`
- Rust importer, replay, state hash, and parity tests
- explicit full-run outcomes from declared seed suites

Anything else is diagnostic unless a current doc says otherwise.

## Current Active Surfaces

- `src/runtime/`, `src/engine/`, `src/content/`, `src/state/`
  - simulator/runtime truth
- `src/protocol/`, `src/diff/`, `src/testing/`, `src/verification/`
  - importer, replay, fixtures, and validation
- `src/cli/live_comm/`, `tools/live_comm/`
  - Java-connected parity and run capture
- `src/bin/full_run_env_driver/`
  - line-protocol driver for reset, legal observation, explicit step, and
    DecisionRecord capture
- `tools/learning/`
  - DecisionRecord collection, replay, and contract audit only

## Read First

- [docs/README.md](docs/README.md)
- [docs/AI_DIRECTION.md](docs/AI_DIRECTION.md)
- [docs/REPOSITORY_MAP.md](docs/REPOSITORY_MAP.md)
- [docs/LAYER_BOUNDARIES.md](docs/LAYER_BOUNDARIES.md)
- [docs/live_comm/README.md](docs/live_comm/README.md)
- [docs/protocol/README.md](docs/protocol/README.md)
- [tools/learning/README.md](tools/learning/README.md)

## Common Commands

```powershell
cargo build --release
cargo test
powershell -ExecutionPolicy Bypass -File .\tools\run_high_value_tests.ps1
cargo run --bin sts_dev_tool -- logs status
```

For Java-connected runs, prefer checked-in live profiles:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\live_comm\use_profile.ps1 Ironclad_Engine_Strict
```

## Non-Goals

- no A20H claim
- no trusted current learning policy
- no baseline-as-teacher pipeline
- no hand-written reward/shop/event/path/campfire/boss-relic policy core
- no policy conclusion from one seed death

## License

MIT
