# Engine Parity Handoff

This file is the compressed handoff for starting a fresh conversation on engine parity work.

## Reality Check

- Do not assume the project is "almost done".
- The engine is still the weakest layer.
- `state_sync` / `carry` fixed many live parity bugs, but it is itself now a major source of complexity and risk.
- Large parts of the codebase were grown by patching an MVP, not by executing a stable architecture.
- A large test count does not imply architectural health.

## What Went Wrong

### 1. `state_sync` / `carry` became a second engine

- It started as a bridge from Java snapshots into Rust runtime.
- It now partially reconstructs hidden state, move history, monster identity, internal counters, and runtime-only semantics.
- This makes it easy to "fix parity" by teaching sync/carry to guess, instead of fixing the actual engine or protocol.
- Result: bugs move from obvious engine code into opaque sync heuristics.

### 2. Hidden Java state was underestimated

- The main lesson is:
  - if Java uses hidden state to determine future evolution, Rust should not guess it from visible state unless forced.
- `Hexaghost` was the clearest proof:
  - Java uses `activated / orbActiveCount / burnUpgraded`
  - Rust had tried to approximate from `move_history`
- `Darkling` was the next proof:
  - Java uses constructor-cached `nipDmg` and `firstMove`
  - Rust had been recomputing damage and inferring first move from history

### 3. Mechanical edits on giant files are expensive

- A seemingly small change to `MonsterEntity` exploded because of many struct literals.
- Regex or bulk replacement on large files is dangerous.
- The token cost of recovering from a bad bulk edit is often worse than the original change.

### 4. Test volume became misleading

- There are now many tests, but many were added on top of unstable structure.
- Some tests are valuable.
- Many broad tests increase fear of editing without guaranteeing the design is good.
- Prefer small, high-signal regression tests over huge piles of speculative unit coverage.

## Current Strategic Position

### Good progress that is real

- `Hexaghost` no longer relies primarily on `move_history`.
- `Time Eater / Draw Reduction` parity was fixed using Java move-history semantics and early-end-turn behavior.
- `Darkling` death / half-dead / revive semantics were repaired.
- `Darkling` now also has runtime state for:
  - `first_move`
  - `nip_dmg`
- `Darkling` move logic now uses:
  - cached `nip_dmg`
  - first-move semantics
  - board-position-sensitive `CHOMP` branch
- `live_regression.py` and `bugfix_workflow.py` now exist and are usable.

### But the architecture is still unhealthy

- Monster hidden state handling is ad hoc.
- Carry policies are scattered and bug-prone.
- `MonsterEntity` is accumulating one-off runtime fields.
- Protocol exports are still incomplete for some monsters and runtime systems.
- Very large files and many handwritten struct literals create maintenance drag.

## Files That Matter Most

### Engine / runtime core

- `src/combat.rs`
- `src/engine/action_handlers/spawning.rs`
- `src/content/monsters/mod.rs`
- `src/engine/action_handlers/mod.rs`

### Sync / carry debt zone

- `src/diff/state_sync/build.rs`
- `src/diff/state_sync/sync.rs`
- `src/diff/state_sync/internal_state.rs`
- `src/diff/state_sync/tests.rs`

### Recent monster case studies

- `src/content/monsters/exordium/hexaghost.rs`
- `src/content/monsters/exordium/the_guardian.rs`
- `src/content/monsters/beyond/darkling.rs`
- `src/content/monsters/city/gremlin_leader.rs`
- `src/content/monsters/beyond/time_eater.rs`

### Tooling

- `tools/analysis/live_regression.py`
- `tools/analysis/bugfix_workflow.py`
- `docs/BUGFIX_WORKFLOW.md`

## Recently Fixed, Worth Knowing

### Hexaghost

- Java hidden fields were exported and Rust now uses them.
- Do not regress back to `move_history`-driven approximation.

### Time Eater / Draw Reduction

- Root issue was not just power cleanup.
- Rust needed to seed monster `move_history` with the current `move_id`, matching Java `setMove(...)` semantics.
- Early-end-turn behavior after `Time Warp` also needed explicit handling.

### Darkling

- `Life Link` is represented by `PowerId::Regrow` in Rust.
- Death path now respects:
  - half-dead transition
  - count move
  - reincarnate move
  - all-darklings-dead final kill
- Runtime now tracks:
  - `darkling.first_move`
  - `darkling.nip_dmg`

## Things To Stop Doing

- Do not assume `move_history` is a universal substitute for hidden monster state.
- Do not add new carry heuristics before checking Java source.
- Do not use giant regex edits across the repo unless the change is trivial and reversible.
- Do not treat broad legacy tests as sacred.
- Do not debug large live logs directly if a single-field fixture can be extracted first.

## Better Rules Going Forward

### Rule 1: Combat truth prefers Java fidelity over Rust elegance

If the behavior affects future combat state:

- prefer explicit Java-aligned state
- prefer protocol export
- prefer runtime state over inference

Rust-idiomatic simplification is only acceptable when it can be shown not to affect parity.

### Rule 2: Sync should import truth, not invent it

`state_sync` should mostly:

- map snapshot fields
- preserve runtime state that Java omitted

It should not become a speculative recovery engine for missing design.

### Rule 3: Fix one field, one window, one cause chain

Use:

1. extract fixture
2. minimize fixture
3. read Java source
4. add regression
5. patch runtime

Avoid large "summary-level" bug fixing.

### Rule 4: Small trusted regressions beat giant test piles

Best tests are:

- live-derived regression fixtures
- snapshot-to-runtime seeding regressions
- narrow runtime regressions for known Java semantics

Worst tests are:

- large speculative coverage tests that lock bad structure in place

## Current Tool Reality

### `bugfix_workflow.py`

Useful:

- extracts live fixture
- minimizes fixture
- creates notes template

### `live_regression.py`

Improved, but still not perfect:

- field-directed minimization is better than before
- context guards help
- minimized fixtures are now more trustworthy
- but minimization still needs care and can still bloat

Interpretation:

- use it
- do not blindly trust it

## Recommendation For New Conversation

Yes, starting a fresh conversation is reasonable.

The next conversation should begin from this file plus the narrow bug target, not from a huge replay of the entire history.

Suggested opener:

1. name the exact target field or parity cluster
2. reference the live fixture or notes file
3. state whether the likely issue is:
   - engine execution
   - sync/carry
   - hidden Java state
   - identity/ordering

## If A Bigger Refactor Happens

The most plausible long-term cleanup path is not "keep patching forever".

Likely better direction:

- shrink `state_sync` responsibilities
- formalize hidden runtime state as explicit typed sub-structures
- reduce giant test surface in favor of fixture-driven parity regressions
- consider a more declarative or DSL-like execution model only if it reduces hidden ad hoc semantics instead of adding another abstraction layer

Important:

- a rewrite should not start from "delete everything"
- it should start from identifying which semantics are authoritative and which current layers are fabricating behavior

## Hard-Won Lessons

- Exporting a few more Java fields is often cheaper than inventing Rust inference.
- A parity fix that only works by carry heuristics is suspicious.
- Adding one runtime field can be correct; adding many ad hoc fields without a pattern is not.
- Big context windows hide cost rather than removing it.
- If a `Darkling` fix touches thousands of lines, the architecture is the problem, not the monster.
