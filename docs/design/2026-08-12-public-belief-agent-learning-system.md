# Public-Information Agent: Facts And Working Hypotheses

Status: **working design, expected to change with evidence**

This page is not a prescribed training recipe. It separates constraints that
follow from the game and implemented code from hypotheses that still need
experiments. When evidence rejects a hypothesis, change or delete it here; do
not preserve it for compatibility.

## Confirmed Constraints

- The deployed agent may use only player-visible run/combat history and the
  current public legal choices. Exact draw order when hidden, RNG state,
  unrevealed intent, card/potion UUIDs, and monster entity ids are private.
- Public candidate identity must remain executable. Public ordinals and typed
  semantics are resolved through a parallel private table owned by the
  environment, never by the model.
- Potion acquisition, use, discard, and retention affect complete-run value.
  A no-potion combat objective or static potion tier cannot be the final task.
- A combat win, local HP score, exact-future witness, or named encounter is not
  by itself a complete-run policy target.
- A sampler must declare what it conditions on. A mechanics-feasible ensemble
  must not be called a posterior over run histories.

## Implemented Boundary

`crates/sts_agent` is the Cargo owner for the physical `src/agent` tree.
`cargo test-agent` is the routine edit loop.

- `information/state.rs`: `PublicCombatStateV1`, without simulator handles.
- `information/action.rs`: public atomic/indexed/symbolic choices and
  `CombatActionResolutionTableV1`.
- `information/run.rs`: public run-continuation context carried into combat.
- `belief/combat.rs`: typed particles and sampler conditioning. The current
  `IndependentStreams` implementation only conditions on the present public
  boundary; it independently resamples hidden combat streams and rejects
  hidden current intent.
- `belief/environment.rs`: typed public action-observation history and exact
  particle stepping. One public action is resolved separately in every
  particle. Different next visible observations become conditional chance
  branches with normalized particle mass.

These types provide information and transition mechanics. They do not produce
policy targets, search values, teacher labels, or a trained agent.

## Open Questions

The next implementation should answer one question at a time rather than
assuming the full stack in advance:

1. Can a small potion-aware information-set search consumer use the belief
   environment with bounded, comparable work per public root action?
2. Which hidden sources materially require run-seed/history conditioning, and
   can that sampler be validated on replayable natural histories?
3. What bootstrap behavior makes complete runs informative without turning an
   existing heuristic/search owner into an unquestioned teacher?
4. Which replay target and model memory improve held-out complete-run outcomes?
5. Does A0 help as a curriculum for the same deployed task, and which parts
   transfer toward A20? This is an empirical question, not an architecture
   promise.

Information-set MCTS/POMCP, visit-policy distillation, recurrent models, and
the ordering of the questions above are candidates, not established facts.
They should be retained only when a small downstream experiment distinguishes
them from simpler alternatives.

## Acceptance And Deletion

A new layer is accepted only when a real caller exercises its typed output and
the result answers a stated distributional question. Source-shape checks,
report prose, named-fight anecdotes, and long milestone lists are not evidence.

Breaking migration is the default. Once a replacement has a consumer, delete
the old entry point, schema, helper, and duplicate documentation. Git history
is sufficient archaeology.
