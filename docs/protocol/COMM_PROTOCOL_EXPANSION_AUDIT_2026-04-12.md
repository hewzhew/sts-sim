# Communication Protocol Expansion Audit

## Status Note

This audit captures the reasoning that led to the protocol-truth refactor.

Since it was written:

- live `sync` no longer performs global previous-state carry
- replay rebuild no longer performs global previous-state carry
- reward-audit combat rebuild no longer performs global previous-state carry

So this document should now be read primarily as:

- a rationale for the direction
- a catalog of debt classes

not as a literal description of the current live main path.

## Why Now

`state_sync` is carrying too much protocol debt.

That is no longer a vague architectural complaint. Recent parity fixes showed three different failure modes:

- Rust inferred hidden state that Java already exported, but Rust did not consume.
- Rust carried or reconstructed runtime state that should be explicit protocol truth.
- `sync` and `carry` now act like a second engine instead of a thin importer.

This audit is meant to decide what to:

1. consume immediately from the current Java protocol
2. add next to the Java protocol
3. explicitly defer

The goal is not "add more fields everywhere". The goal is to stop letting `sync` guess.

## Current Protocol Reality

The current Java `CommunicationMod` already exports more than the Rust side is consistently using.

Important existing exports in [GameStateConverter.java](/d:/rust/CommunicationMod/src/main/java/communicationmod/GameStateConverter.java):

- `protocol_meta.capabilities`
- `protocol_meta.reward_session`
- `protocol_meta.combat_session`
- monster identity and ordering:
  - `monster_instance_id`
  - `spawn_order`
  - `monster_index`
  - `draw_x`
- monster move context:
  - `move_id`
  - `move_base_damage`
  - `move_adjusted_damage`
  - `move_hits`
  - `last_move_id`
  - `second_last_move_id`
- monster state flags:
  - `half_dead`
  - `is_gone`
  - `is_dying`
  - `is_escaping`
- monster-specific hidden/runtime exports already present:
  - `guardian_dmg_threshold`
  - `guardian_dmg_taken`
  - `guardian_is_open`
  - `guardian_close_up_triggered`
  - `hexaghost_activated`
  - `hexaghost_orb_active_count`
  - `hexaghost_burn_upgraded`
- power state:
  - `amount`
  - `damage`
  - `misc`
  - `just_applied`
- relic state:
  - `counter`
  - `used_up`
- combat meta:
  - `turn`
  - `cards_discarded_this_turn`
  - `times_damaged`
  - `card_queue`
  - `using_card`
  - `card_in_play`
  - `monster_turn_log`
- RNG:
  - `ai_rng`
  - `shuffle_rng`
  - `card_rng`
  - `misc_rng`
  - `monster_hp_rng`
  - `potion_rng`

This matters because some recent Rust fixes were not blocked on Java changes. They were blocked on Rust not trusting or consuming fields already present.

## Sync Debt Inventory

The main Rust debt zone is:

- [build.rs](/d:/rust/sts_simulator/src/diff/state_sync/build.rs)
- [sync.rs](/d:/rust/sts_simulator/src/diff/state_sync/sync.rs)
- [internal_state.rs](/d:/rust/sts_simulator/src/diff/state_sync/internal_state.rs)

Today those files still do all of the following:

- direct snapshot mapping
- runtime seeding
- continuity carry
- hidden power data carry
- monster identity repair
- relic runtime fallback

That is too much responsibility for one layer.

Concrete current debt signals:

- `seed_hexaghost_runtime_from_snapshot`
- `seed_darkling_runtime_from_snapshot`
- `seed_lagavulin_runtime_from_snapshot`
- `carry_hidden_monster_turn_state`
- `carry_internal_monster_power_state`
- `carry_internal_relic_state`
- `carry_internal_limbo_state`
- `carry_monster_logical_positions`

And in [internal_state.rs](/d:/rust/sts_simulator/src/diff/state_sync/internal_state.rs):

- power extra-data policies for:
  - `Combust`
  - `Malleable`
  - `Flight`
  - `Stasis`
  - `PanachePower`
  - `TheBombPower`
- relic `used_up` fallback policies for:
  - `CentennialPuzzle`
  - `HoveringKite`
  - `LizardTail`
  - `Necronomicon`
- seeded missing powers for:
  - `GuardianThreshold`
  - `Angry`

This is already enough to justify a protocol cleanup plan.

## Tier A: Consume Existing Java Exports First

These are not protocol additions. They are Rust cleanup work that should happen before expanding Java further.

### A1. Stop ignoring `power.misc`

This already produced a real bug.

`Combust.hpLoss` was already exported by Java as `misc`, but Rust initially ignored it and carried a guessed default.

That means all similar power state should be audited immediately:

- `Combust`
- `Malleable`
- `Flight`
- `PanachePower`
- `TheBombPower`
- any other power currently routed through `extra_data`

Required rule:

- if Java already exports enough truth via `damage`, `misc`, or `just_applied`, Rust should stop guessing.

### A2. Stop fallback-carrying relic `used_up` when Java already provides it

`CommunicationMod` now exports `relic.used_up`.

Rust still retains compatibility fallback logic in [internal_state.rs](/d:/rust/sts_simulator/src/diff/state_sync/internal_state.rs). That may still be useful for old fixtures, but live protocol paths should prefer snapshot truth and treat fallback as legacy-only compatibility.

### A3. Audit monster-specific exported fields already present

Rust should prefer explicit snapshot fields over inference for monsters that already have them:

- `Hexaghost`
- `Guardian`

This is partly done, but it should be treated as a pattern, not as one-off special handling.

### A4. Use protocol identity fields as authoritative identity inputs

The current protocol already exports:

- `monster_instance_id`
- `spawn_order`
- `draw_x`

Rust still sometimes falls back to rough matching or position carry. That should keep shrinking.

## Tier B: Next Java Protocol Additions

These are the best candidates for actual protocol expansion because Rust currently reconstructs them and the reconstruction is fragile.

### B1. Explicit per-monster hidden runtime fields for recurring offenders

`Lagavulin` proved that `move_history` is not a universal substitute for hidden Java state.

Good candidates for explicit export when they recur enough:

- `Lagavulin.idleCount`
- `Lagavulin.isOutTriggered`
- `Darkling.firstMove`
- `Darkling.nipDmg`

Rationale:

- Rust can carry these for now.
- But if these monster-specific runtime counters accumulate, `sync` becomes a hidden-state emulator.

### B2. Named power-internal fields instead of overloaded `misc`

`misc` is better than nothing, but it is not self-describing.

For powers that repeatedly matter to parity, Java should eventually emit explicit names, for example:

- `hp_loss`
- `base_power`
- `stored_amount`
- `cards_doubled_this_turn`

The current Java converter already probes several possible backing fields and shoves them into `misc`. That is good for compatibility, but weak for protocol clarity.

Recommendation:

- keep `misc` for backward compatibility
- add explicit named fields for high-value cases
- let Rust prefer named fields, then fall back to `misc`

### B3. Reward / interaction session protocol

This work is already partly designed in [COMM_PROTOCOL_REWARD_SESSION_DRAFT.md](COMM_PROTOCOL_REWARD_SESSION_DRAFT.md).

This is still worth doing because it removes inference around temporary reward-screen exits and human intervention.

The key point is that this is the same architectural move:

- push interaction truth into protocol
- stop forcing Rust to infer session continuity from screen transitions

### B4. Slot identity when draw order/position is semantically important

The current protocol has `draw_x`, which is already useful.

If repeated parity bugs show that `draw_x` plus `instance_id` is still not enough, the next escalation should be an explicit stable slot concept for relevant encounters, not more Rust-side guesswork.

This is not justified globally yet, but it should stay on the shortlist.

## Tier C: Do Not Expand Yet

These are not good protocol additions right now.

### C1. Everything hidden, all at once

Do not try to export every monster's private fields indiscriminately.

That will produce a bloated protocol with weak semantics and no prioritization.

### C2. Fields that do not currently cause repeated parity debt

If a field is neither:

- already guessed by Rust repeatedly
- nor responsible for live parity drift

then it should not be added just because it exists in Java.

### C3. Daily-mod / fringe-mode state

Unless livecomm is actively targeting those modes, they should not drive the next protocol batch.

## Recommended Batch Plan

### Batch 1: Shrink Rust debt without changing Java

This batch should happen first.

Tasks:

- audit all `extra_data` powers against current Java `damage/misc/just_applied`
- audit all relic `used_up` handling and isolate legacy fallback
- document every remaining `seed_*` and `carry_*` site by cause
- separate "live protocol debt" from "old fixture compatibility debt"

Success condition:

- fewer parity fixes are landing in `sync` when Java already emitted the truth

### Batch 2: Add explicit hidden-state fields for repeated monster offenders

Only after Batch 1.

Initial candidates:

- `Lagavulin.idleCount`
- `Lagavulin.isOutTriggered`
- `Darkling.firstMove`
- `Darkling.nipDmg`

Success condition:

- Rust removes the matching inference logic or downgrades it to legacy fallback

### Batch 3: Expand session-level protocol for noncombat / human-audit continuity

Use the existing reward-session draft as the first pattern.

Possible future extensions:

- card reward session
- discovery/toolbox session
- maybe other temporary offscreen interactions

Success condition:

- Rust no longer polls screens to guess whether an interaction is still active

## Immediate Recommendations

### 1. Treat "protocol expansion" as a debt-payoff program, not a feature spree

Every proposed Java field should answer:

- which current Rust inference does this delete?
- which current parity class does this stabilize?

If it deletes nothing, it is probably premature.

### 2. Start with a Rust-side consumption audit before adding more Java fields

Recent evidence says the cheapest wins are often here.

### 3. Maintain a live shortlist of protocol-worthy hidden state

Do not let those discoveries stay implicit inside bugfix commits.

### 4. Keep `sync` on a diet

If a parity fix wants to add another `carry_*` or monster-specific guess:

- first check whether Java already exports the truth
- second decide whether this is a candidate for the next protocol batch
- only then allow a temporary `sync` patch

## Proposed Next Step

Run a narrow Rust-side audit for all current `state_sync` debt with this output format:

- debt site
- current Rust behavior
- current Java export status
- action:
  - consume_now
  - expand_protocol
  - keep_local_for_now

That should produce the concrete backlog for the first Java protocol expansion batch.
