# Learned Agent Module Map

`src/agent` is the physical source tree for the learned agent's information
boundary. `crates/sts_agent` is its only Cargo owner and sits between simulator
mechanics and evaluation, run control, Python bindings, and training code. Use
`cargo test-agent` for the routine loop.

| Module | Owns | Does not own |
| --- | --- | --- |
| `information/combat.rs` | compact public observation and visibility semantics | model architecture or exact handles |
| `information/state.rs` | rich `PublicCombatStateV1` used by model-facing adapters | potion UUIDs, monster entity IDs, RNG state |
| `information/action.rs` | canonical public atomic/indexed/symbolic candidates plus a parallel private resolution table | search values or policy choices |
| `information/run.rs` | public run-continuation context carried into combat | exact run checkpoints or hidden map state |
| `belief/combat.rs` | typed hidden-future particles and declared sampler conditioning | search verdicts, teacher labels, run-control checkpoints |
| `belief/environment.rs` | public history, particle stepping, exact action resolution, and visible chance branches | search policy, rollout value, or training targets |

Current limits: `IndependentStreams` is not a run-history posterior, hidden
current intent is unsupported, and no search or learning target consumes the
belief environment yet. Candidate approaches and their falsification questions
live in the
[working design](../../docs/design/2026-08-12-public-belief-agent-learning-system.md).
