# Learned Agent Module Map

`src/agent` is the simulator-level owner for the learned agent's information
boundary. It is intentionally below evaluation, run control, Python bindings,
and training code.

| Module | Owns | Does not own |
| --- | --- | --- |
| `information/combat.rs` | compact public observation and visibility semantics | model architecture or exact handles |
| `information/state.rs` | rich `PublicCombatStateV1` used by model-facing adapters | potion UUIDs, monster entity IDs, RNG state |
| `information/action.rs` | canonical public atomic/indexed/symbolic candidates plus a parallel private resolution table | search values or policy choices |
| `belief/combat.rs` | typed hidden-future particles and sampling provenance | search verdicts, teacher labels, run-control checkpoints |

Current important limitation: `IndependentStreams` is a public-boundary-
preserving feasibility sampler, not a posterior over complete run histories.
The next structural migration is the belief environment and conditioned
sampler interface described in
[`docs/design/2026-08-12-public-belief-agent-learning-system.md`](../../docs/design/2026-08-12-public-belief-agent-learning-system.md).
