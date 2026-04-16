# Slay the Spire Simulator: Architecture & Verification Guide

This document defines the architecture, performance context, and standard development workflow for the Rust-based Slay the Spire simulator.

## 1. Performance Context & Motivation

The simulator was rewritten in Rust to achieve extreme execution efficiency, primarily to act as a fast-forward environment for Monte Carlo Tree Search (MCTS) and Reinforcement Learning (RL) agents. 
- **Target**: Achieve execution speeds orders of magnitude faster than the Java JVM environment (e.g., thousands of states/ticks per second per core) to make deep search trees computationally viable.
- **Implementation Constraint**: Logic must be completely deterministic, garbage-collection-free, and allocation-light.

## 2. Quickstart & Onboarding

### Requirements
- Rust stable toolchain (`cargo`)

### Common Commands
- **Compile Project**:
  ```bash
  cargo build --release
  ```
- **Run Unit Tests** (e.g., map generation, isolated RNG verification):
  ```bash
  cargo test
  ```
- **Run Core Java-Parity Verification (diff_driver)**:
  ```bash
  cargo test test_diff_rng_replay -- --nocapture
  ```

## 3. Core Architecture

The system rigidly separates **State** from **Logic** to mirror the original game engine.

### 3.1 State Structures
- `EngineState`: Root state machine (e.g., `CombatPlayerTurn`, `EventRoom`, `PendingChoice`).
- `RunState`: Global structural data (HP, Gold, Deck, Relics).
- `CombatState`: Ephemeral battle data (Hand, Piles, Monster Intents, Active Orbs, RNG tracking).

### 3.2 Action Queue
Game events push discrete `Action` enums (e.g., `DamageAction`, `DrawCardAction`) into `CombatState::action_queue`.
- Evaluated sequentially via `tick_engine()`.
- Guarantees event-chain sequencing exactly matches the Java loop.

## 4. Verification Methodology

The absolute source of truth is the **Java Engine**. We rely exclusively on data-driven differential testing (`diff_driver.rs`) rather than manually assumed unit tests.

- **Trace Generation**: The Java `CommunicationMod` exports rigorous `.jsonl` trace files during actual gameplay. Each action outputs the resulting exact gamestate snapshot.
- **Verification Engine**: `diff_driver.rs` loads states sequentially from the trace, resolves the recorded player actions inside the Rust engine, and strictly asserts bit-for-bit parity across all state fields (HP, cards, buffs, RNG seed advancements).

Not every correctness-sensitive test should use the same oracle strength.
Protocol truth fixtures, behavior tests, differential replay checks, and simple
invariant tests all play different roles. The key rule is that exact behavior
tests must name an external oracle source rather than silently deriving expected
values from current Rust behavior.

See also:

- [TEST_ORACLE_STRATEGY.md](TEST_ORACLE_STRATEGY.md)

### 4.1 Protocol Truth Rule

Combat parity work must follow this chain:

1. Java engine runtime field
2. Java protocol export
3. Rust importer (`build` / `sync`)
4. Rust engine runtime

Do not insert a second semantic layer in between.

In particular:

- do not treat `state_sync` carry logic as a source of truth
- do not invent Rust-side generic fields such as `move_history` or logical slot abstractions as replacements for Java runtime fields when Java already has dedicated state
- if a hidden Java counter/flag matters for combat parity, export it through `CommunicationMod`

See also:

- [PROTOCOL_TRUTH_RULES.md](protocol/PROTOCOL_TRUTH_RULES.md)
- [COMM_PROTOCOL_DEBT_BACKLOG_2026-04-12.md](protocol/COMM_PROTOCOL_DEBT_BACKLOG_2026-04-12.md)
- [TEST_ORACLE_STRATEGY.md](TEST_ORACLE_STRATEGY.md)
- [RL_READINESS_CHECKLIST.md](RL_READINESS_CHECKLIST.md)

## 5. Standard Contribution Workflow

To add or modify a new Card, Relic, or mechanic, adhere to this strict feedback loop:

1. **Implement in Rust**: You MUST consult `tools/source_extractor/AGENT_GUIDE.md` and the extracted AST reports (especially `scattered_logic.md`) *before* writing Rust code to avoid missing hidden game engine dependencies. Then add the entity hooks to `src/content/` or `src/engine/action_handlers.rs`.
2. **Generate Java Ground Truth**: Boot the actual Java game with `CommunicationMod` enabled. Trigger your newly added mechanic in-game to generate a rigorous `.jsonl` trace. Depending on workflow, these traces may live under `tools/replays/` or under the `live_comm` raw-log path in `logs/raw/`.
3. **Run Differential Test**: Run the trace through the parity checker:
   ```bash
   cargo test test_diff_rng_replay -- --nocapture
   ```
4. **Iterate**: If assertions fail (e.g., RNG seed mismatch or buff counts differ), adjust the Rust logic and re-run the verification until 100% parity is achieved.

If the failure depends on hidden Java runtime state:

1. inspect Java source first
2. add or confirm protocol export in `CommunicationMod`
3. fix the Rust importer
4. only then touch Rust engine code

Do not default to sync/carry heuristics.
