# Monster Runtime-State Framework

This document defines the first-stage framework for stateful monster semantics.

It exists to stop semantic monster ports from drifting back into:

- hidden runtime truth guessed from `move_history`
- per-monster ad hoc update side effects
- protocol truth that lands in Java but is never maintained by Rust execution

## Current Goal

Do **not** unify monster storage first.

First unify:

1. protocol/runtime truth rules
2. importer seeding rules
3. execution-time runtime patch rules
4. audit rules for `move_history`

Only after these are stable should storage shape be reconsidered.

## Phase 1 Architecture

The current first-stage framework is:

1. Java source identifies real runtime fields.
2. `CommunicationMod` exports those fields in `monster.runtime_state`.
3. Rust state-sync seeds typed runtime fields on `MonsterEntity`.
4. Semantic execution updates those fields only through:
   - `Action::UpdateMonsterRuntime { monster_id, patch }`
   - `MonsterRuntimePatch::*`

This is intentional.

The patch channel is now the canonical execution boundary for monster runtime-state updates.

## Hard Rules

### Rule 1: Runtime truth must come from Java if Java owns it

If Java has a private field or lifecycle latch that affects `getMove`, `takeTurn`, or `changeState`,
the default assumption is:

- it is protocol truth
- it should be exported
- Rust should import it explicitly

Do not replace it with a "cleaner" Rust guess first.

### Rule 2: Semantic execution must update runtime state explicitly

If a semantic monster changes runtime state during execution, it must emit
`UpdateMonsterRuntime`.

Not allowed:

- mutating monster runtime state as a silent side effect in unrelated paths
- relying on a later importer pass to repair state drift
- relying on `move_history` to retroactively infer hidden state

### Rule 3: `move_history` is only for Java rule-sequence logic

Allowed:

- reproducing Java `lastMove(...)`
- reproducing Java `lastTwoMoves(...)`
- other move-sequence rules that are explicitly present in Java AI

Not allowed:

- inferring `first_turn`
- inferring `is_flying`
- inferring `used_hex`
- inferring `activated`
- inferring similar hidden runtime flags/counters/latches

If the field is not literally a move-sequence rule in Java, `move_history` is not truth.

### Rule 4: Importer truth must be maintainable by execution

If `state_sync` seeds a runtime field, semantic execution must be able to maintain it.

Unacceptable state:

- importer seeds field `X`
- execution never updates field `X`
- later move selection still depends on `X`

That is a broken contract even if live parity looks good for a few frames.

## Canonical Update Channel

The canonical runtime update action is:

- [action.rs](D:\rust\sts_simulator\src\runtime\action.rs)

Specifically:

- `Action::UpdateMonsterRuntime`
- `MonsterRuntimePatch`

Current typed patches:

- `Hexaghost`
- `Lagavulin`
- `Guardian`
- `Byrd`
- `Chosen`

This is the repo-wide pattern for new stateful semantic monsters.

Do not add new `Action::UpdateXState` variants.

## Audit Checklist Per Monster

Every new stateful semantic monster must answer these questions:

1. Which Java private fields or lifecycle latches affect move choice or execution?
2. Which of those are already exported by protocol?
3. Which Rust typed runtime fields mirror them?
4. Which execution steps update them explicitly through `UpdateMonsterRuntime`?
5. Which uses of `move_history` remain, and are they strictly Java rule-sequence logic?

If question 4 or 5 is fuzzy, the monster is not done.

## Current Audit Status

### Hexaghost

- Runtime truth:
  - `activated`
  - `orb_active_count`
  - `burn_upgraded`
- Patch channel: yes
- History used as truth: no
- Notes:
  - `divider_damage` is execution-locked semantic truth, not a Java private field export

### Lagavulin

- Runtime truth:
  - `idle_count`
  - `debuff_turn_count`
  - `is_out`
  - `is_out_triggered`
- Patch channel: yes
- History used as truth:
  - only for Java attack/debuff sequence checks

### The Guardian

- Runtime truth:
  - `damage_threshold`
  - `damage_taken`
  - `is_open`
  - `close_up_triggered`
- Patch channel: yes
- History used as truth: no
- Notes:
  - review whether every parity-relevant field is exported or intentionally execution-owned

### Chosen

- Runtime truth:
  - `first_turn`
  - `used_hex`
- Patch channel: yes
- History used as truth:
  - allowed only for Java `lastMove` attack/debuff branching
- Notes:
  - prior fallback from history for `first_turn`/`used_hex` is migration-only and should keep shrinking

### Byrd

- Runtime truth:
  - `first_move`
  - `is_flying`
- Patch channel: yes
- History used as truth:
  - allowed only for Java `lastMove` / `lastTwoMoves` sequence checks among `PECK/SWOOP/CAW`
- Notes:
  - `CommunicationMod` must export `runtime_state.first_move` and `runtime_state.is_flying`

### Darkling

- Runtime truth:
  - `first_move`
  - `nip_dmg`
- Patch channel: not yet unified under semantic main-path review
- History used as truth: review required
- Status: audit pending

### Louse

- Runtime truth:
  - `bite_damage`
- Patch channel: no execution patch needed after lock, but protocol truth is required
- History used as truth: no
- Status: acceptable with strict protocol/runtime fallback retirement

## Review Target List

The next runtime-state audit pass should explicitly review:

1. `Darkling`
2. `Hexaghost`
3. `Lagavulin`
4. `The Guardian`
5. `Byrd`
6. `Chosen`
7. `Louse`

The point of this pass is not to change behavior immediately.

The point is to verify that each migrated monster now satisfies:

- protocol truth if Java owns it
- explicit patch updates if execution mutates it
- no hidden-state recovery through `move_history`

## Decision Rule For New Act 2 Monsters

When porting a new act 2 monster:

1. read Java first
2. list hidden runtime fields
3. extend protocol if needed
4. seed typed runtime state
5. implement semantic execution
6. add explicit `UpdateMonsterRuntime` patch if execution mutates runtime state
7. justify every remaining `move_history` use as a Java rule-sequence requirement

If step 7 cannot be justified, it is a design bug.

## Non-Goal For Phase 1

Phase 1 does **not** require converting `MonsterEntity` into a single runtime enum/container.

That may happen later.

For now, the winning condition is:

- one protocol truth rulebook
- one importer seeding path
- one execution patch channel
- one audit checklist

That is enough to keep act 2 ports honest.
