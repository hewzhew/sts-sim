# Bugfix Workflow

The old live-window to `CombatCase` workflow is retired.

Current default:

1. inspect Java source when exact mechanics are unclear
2. reproduce locally with a focused Rust behavior test or a combat start-spec
3. run `cargo test --quiet`
4. for search behavior, run `combat_search_v2_driver` from a start-spec and
   compare whole-combat outcomes, not stepwise replay actions

Do not recreate the old case/scenario/protocol/state-sync artifact chain as the
default bugfix path. A future Java adapter may add a new oracle format, but it
should be separate from the simulator/search mainline.
