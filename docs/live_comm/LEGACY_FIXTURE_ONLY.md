# Live Comm Legacy Boundary

`live_comm` is no longer an active AI, workbench, or strategy-development path.
Treat the old stack as historical context and fixture-capture support only.

The current AI mainline is:

1. simulator truth
2. explicit state representation
3. combat search / rollout
4. value estimates with reliability labels
5. policy improvement from whole-run outcomes

`live_comm` can help collect external Java observations for that stack, but it
must not own the stack.

## What The Old Stack Did

The old `live_comm` work connected the Rust project to the Java game through
CommunicationMod. Its useful responsibilities were:

- launch the Rust side from the CommunicationMod `config.properties` command
- read Java frames containing `game_state`, `protocol_meta`, and visible action
  affordances
- archive raw/debug/watch logs for later investigation
- capture manual scenario samples for exact Java behavior
- import Java snapshots into Rust combat fixtures
- replay captured windows to compare Rust transitions against Java-observed
  truth
- expose handoff/watch flows for human-primary experiments

That work was useful for discovering mechanics bugs and protocol gaps. It was
not a strong AI architecture.

## Why It Was Downgraded

The old implementation grew around operational convenience instead of a clean
runtime boundary. The main problems were:

- live transport, protocol parsing, replay diffing, UI/watch behavior, and
  strategy experiments became coupled
- Java-connected runs were too slow and stateful for the inner search loop
- stepwise agreement with captured actions encouraged imitation of a run rather
  than whole-combat outcome improvement
- noncombat decision helpers drifted toward hard-coded policy experiments and
  prompt artifacts
- public and hidden state boundaries were not explicit enough for search/eval
- top-level modules such as `verification`, `diff`, and `protocol` made old
  bridge code look like simulator architecture

For now, live Java observation is an oracle source and fixture source. It is not
a controller, not a workbench surface, and not a teacher label.

## Current Boundary

Allowed:

- use checked-in protocol truth samples as fixtures
- use manual Java captures to create focused `CombatCase` or scenario fixtures
- keep importer helpers private under `src/testing`
- use Java observations to debug simulator mechanics when source inspection is
  insufficient

Not allowed:

- reintroduce `src/verification`, `src/diff`, or `src/protocol` as active
  top-level architecture
- build policy search around live CommunicationMod frames
- expand watch UI, recording UI, DecisionBrief, DecisionFrame, or LLM reviewer
  paths
- use captured human actions as stepwise teacher labels
- hide live protocol assumptions inside the simulator runtime

Current code boundary:

- `src/testing/protocol` parses the remaining Java/protocol fixture metadata
- `src/testing/state_sync` builds Rust combat states from fixture snapshots
- `src/testing/replay_support` holds compatibility helpers needed by old
  fixture imports
- `tools/live_comm` is legacy operational scaffolding, not a default workflow

## If This Is Revived Later

Revival should be a new adapter architecture, not restoration of the old module
shape.

The clean shape should be:

1. Protocol contract
   - versioned Rust structs for CommunicationMod frames and commands
   - explicit capability flags
   - explicit public-state versus hidden-truth fields
   - no simulator mutation logic

2. Capture transport
   - one small executable responsible for talking to CommunicationMod
   - append-only raw frame archive
   - no strategy policy, no UI decisions, no search

3. Fixture importer
   - converts a captured frame/window into `CombatCase`, scenario, or start-spec
   - lives under testing/import code
   - rejects missing required truth instead of silently repairing runtime state

4. Offline replay oracle
   - runs captured fixture windows against the simulator
   - reports field-level divergence
   - never acts as the combat planner

5. Action adapter
   - maps simulator legal actions to CommunicationMod commands only after the
     simulator/search layer has chosen an action
   - validates every command against public legal actions
   - refuses unknown or stale frame IDs

6. Optional diagnostics UI
   - reads logs and reports
   - cannot own protocol, search, or policy behavior

## Revival Acceptance Bar

Do not revive this path until the local Rust simulator and search path can
already produce useful whole-combat outcomes from local fixtures.

A future live adapter should require:

- zero `cargo check` warnings
- a versioned protocol schema test suite
- fixture-import tests independent of CommunicationMod
- one minimal end-to-end smoke run against Java
- no dependency from core runtime/search into live transport code
- no UI/workbench dependency for capture, replay, or action execution
- outcome comparison at whole-combat or whole-run level, not stepwise imitation

The intended future use is external execution and oracle capture for an already
working simulator/search system. It should not become the place where strategy is
invented.
