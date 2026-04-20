# sts_simulator

Rust Slay the Spire simulator focused on three things:

- deterministic engine/runtime truth
- Rust versus Java parity through `live_comm`
- offline AI experiments on top of explicit contracts instead of guessed state

This is not a polished game client. It is a headless simulator and tooling repo.

## Current shape

The repo now has four active workstreams:

1. core engine work
   - `runtime`, `engine`, `content`, `state`, `map`
2. parity and protocol work
   - `live_comm`, replay, strict importer, Java protocol migration
3. bot and debug workbenches
   - search, noncombat policy, audit tools, scenario harnesses
4. offline learning sidecars
   - `CombatEnv`, `combat_env_driver`, structured combat PPO, local oracle and macro counterfactual experiments

The project is still incomplete. The important distinction is that it is no longer
"just a Rust port." The primary acceptance loop is the Java-connected `live_comm`
workflow, and the current learning work is real but still deliberately offline and
experimental.

## What is authoritative

- Java source in `../cardcrawl/`
  - actual game lifecycle and hidden runtime behavior
- this repo's `CommunicationMod` fork in `../CommunicationMod/`
  - protocol exporter used by `live_comm`
- Rust importer and parity tooling
  - must consume exported truth directly instead of reconstructing hidden state by guesswork

The current combat protocol is split:

- `game_state.combat_truth`
- `game_state.combat_observation`
- `protocol_meta.combat_action_space`
- session/runtime metadata such as `reward_session` and `combat_session`

The old merged `combat_state` model is historical. Do not treat it as the live contract.

## Current status

What is working well enough to matter:

- deterministic Rust runtime and replay surfaces
- run-profile based `live_comm` workflow
- strict protocol/importer rules for migrated `runtime_state` slices
- combat-focused offline environment and bridge:
  - `sts_simulator::bot::harness::combat_env::CombatEnv`
  - `combat_env_driver`
  - `tools/learning/structured_combat_env.py`

What is still weak:

- full engine parity is still incomplete
- noncombat protocol and handoff flow are still moving
- the bot is useful as a consumer and stress test, not as a trusted teacher
- learning experiments are reference probes, not production policy

## Read First

- [docs/README.md](docs/README.md)
  - active doc index and what is canonical versus historical
- [docs/REPOSITORY_MAP.md](docs/REPOSITORY_MAP.md)
  - ownership map and active repo surfaces
- [docs/LAYER_BOUNDARIES.md](docs/LAYER_BOUNDARIES.md)
  - hard dependency rules
- [docs/live_comm/README.md](docs/live_comm/README.md)
  - current parity/debugging workflow
- [docs/protocol/README.md](docs/protocol/README.md)
  - protocol truth and importer contract
- [tools/learning/README.md](tools/learning/README.md)
  - current offline learning sidecar path
- [../CommunicationMod/README.md](../CommunicationMod/README.md)
  - Java-side protocol fork used by this repo

## High-level layout

```text
src/
  runtime/ engine/ content/ state/ map/
    core simulator truth
  semantics/ projection/
    explicit truth-side semantic and preview layers
  diff/ protocol/ testing/ verification/
    importer, replay, fixtures, parity, and validation
  bot/ cli/ bin/
    consumers, workbenches, live_comm runtime, and tooling

tools/
  live_comm/
    launcher, run profiles, manual bridge helpers
  learning/
    offline dataset builders, env bridges, PPO/baseline experiments
  sts_tool/ source_extractor/
    Java source tracing and porting support
```

For the current ownership tags and boundaries, trust the docs above over older design notes.

## Common commands

```powershell
cargo build --release
cargo test
powershell -ExecutionPolicy Bypass -File .\tools\run_high_value_tests.ps1
cargo run --bin sts_dev_tool -- logs status
```

For Java-connected runs, prefer checked-in live profiles instead of hand-typed flags:

```powershell
powershell -ExecutionPolicy Bypass -File .\tools\live_comm\use_profile.ps1 Ironclad_Engine_Strict
```

## Learning status

There is active learning work in the repo again, but its scope is narrow:

- combat-only first
- offline-first
- contract-driven
- used for baselines, probes, and reference experiments

What exists today:

- structured combat observation/action contract
- Rust `combat_env_driver` bridge
- `Gymnasium` / PPO experiments
- local oracle and `Q_local` experiments
- macro counterfactual datasets for reward/shop/event analysis

What does not exist today:

- trusted runtime inference in the live bot
- full-run RL environment stability
- a claim that current learning artifacts are strong enough to guide engine truth

## Non-goals

- not a polished terminal game
- not a stable third-party protocol for general consumers
- not yet a trustworthy full-run training environment

## License

MIT
