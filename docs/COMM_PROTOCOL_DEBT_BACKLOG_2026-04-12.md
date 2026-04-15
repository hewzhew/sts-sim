# Communication Protocol Debt Backlog

This is the concrete follow-up to [COMM_PROTOCOL_EXPANSION_AUDIT_2026-04-12.md](/d:/rust/sts_simulator/docs/COMM_PROTOCOL_EXPANSION_AUDIT_2026-04-12.md).

## Status Update

Since this backlog was first written, several items have already moved forward:

- Rust importer now consumes:
  - `power.just_applied`
  - `power.misc` / `power.damage` for current supported power cases
  - `power.card.uuid` for `Stasis`
  - `relic.runtime_state.used_this_combat` for `Centennial Puzzle`
- live `sync` no longer performs global previous-state carry
- replay rebuild no longer performs global previous-state carry
- reward-audit combat rebuild no longer performs global previous-state carry

This file should now be read as:

- a classification ledger
- a remaining-work backlog

not as a description of the current live main path.

The point of this file is not theory. It is to classify each live `state_sync` debt site as one of:

- `consume_now`
- `expand_protocol`
- `keep_local_for_now`

## Classification Rules

### `consume_now`

Java already exports enough truth. Rust should stop guessing and import it directly.

### `expand_protocol`

Java does not export enough truth for a recurring parity-sensitive runtime field. This should go into the next protocol batch.

### `keep_local_for_now`

This is still a Rust representation or compatibility issue, not a good protocol expansion target yet.

## Priority Summary

Highest-value immediate work is still `consume_now`, not adding more fields.

The most obvious current examples are:

- `power.just_applied`
- `power.misc` / `power.damage`
- `power.card.uuid` for `Stasis`
- `relic.used_up`

## Debt Table

### Powers

| Debt site | Current Rust behavior | Current Java export | Action | Notes |
|---|---|---|---|---|
| `build_powers_from_snapshot` `just_applied` handling | Rust always seeds `just_applied = false` | Java exports `power.just_applied` | `consume_now` | This is a real importer gap, not protocol debt. |
| `Combust.extra_data` | Rust used to guess/carry `hpLoss`; now partially fixed via `misc` | Java exports `misc` from `hpLoss` | `consume_now` | Long-term nicer field name is optional, not urgent. |
| `Malleable.extra_data` | Rust imports `misc` into `extra_data`; old carry assumptions should continue shrinking | Java exports `misc` from `basePower` | `consume_now` | Live path should rely on importer truth, not carry. |
| `Flight.extra_data` | Rust imports `misc` into `extra_data`; old carry assumptions should continue shrinking | Java exports `misc` from `storedAmount` | `consume_now` | Same class as `Combust/Malleable`. |
| `PanachePower.extra_data` | Rust imports `damage` into `extra_data` | Java exports `damage` when present | `consume_now` | Continue deleting old carry assumptions around this field. |
| `TheBombPower.extra_data` | Rust imports `damage` into `extra_data` | Java exports `damage` when present | `consume_now` | Same pattern as Panache. |
| `Stasis.extra_data` | Rust now reads captured card uuid from snapshot; limbo/runtime follow-up still needs cleanup | Java exports `power.card` and `card.uuid` | `consume_now` | Main importer gap is closed; remove remaining compatibility debt over time. |
| `Ritual skipFirst` | Rust mirrors `just_applied` into runtime representation | Java exports `just_applied` from `skipFirst` | `consume_now` | Representation can still improve, but importer gap is largely closed. |

### Monster runtime state

| Debt site | Current Rust behavior | Current Java export | Action | Notes |
|---|---|---|---|---|
| `seed_hexaghost_runtime_from_snapshot` | Rust seeds runtime from explicit fields but still has inference fallback | Java exports `hexaghost_activated`, `hexaghost_orb_active_count`, `hexaghost_burn_upgraded` | `consume_now` | Remove fallback once old-fixture compatibility is no longer needed. |
| `seed_darkling_runtime_from_snapshot` `first_move` | Rust infers from move ids/history | No explicit Java export | `expand_protocol` | Recurring hidden-state debt. |
| `seed_darkling_runtime_from_snapshot` `nip_dmg` | Rust infers from current move damage when possible | No explicit Java export | `expand_protocol` | Good protocol candidate. |
| `seed_lagavulin_runtime_from_snapshot` `idle_count` | Rust reconstructs from visible move + turn count | No explicit Java export | `expand_protocol` | Proven bad fit for `move_history`. |
| `seed_lagavulin_runtime_from_snapshot` `is_out_triggered` | Rust reconstructs from visible state | No explicit Java export | `expand_protocol` | Same as above. |
| `seed_move_history_from_snapshot` | Rust rebuilds move history from `move_id/last/second_last` | Java exports `move_id`, `last_move_id`, `second_last_move_id` | `keep_local_for_now` | This is a representation adapter, not a protocol problem by itself. |
| hidden-intent handling under Runic Dome | Rust may still need local representation support for intentionally hidden move data | Java intentionally hides intent under Runic Dome | `keep_local_for_now` | This is a gameplay-semantics case, not a normal truth-import case. |

### Monster identity / positioning

| Debt site | Current Rust behavior | Current Java export | Action | Notes |
|---|---|---|---|---|
| `seed_monster_protocol_identity_from_snapshot` | Rust imports `instance_id/spawn_order/draw_x` | Java exports all three | `consume_now` | Good path already exists; continue shrinking fallbacks. |
| generic monster logical position | Rust should prefer imported `draw_x` / protocol identity | Java exports `draw_x` | `consume_now` | Previous-state carry has been removed from the live main path. |
| `carry_gremlin_leader_logical_positions` | Rust has leader-specific slot reconstruction | Java exports `draw_x`, but slot semantics still need stable mapping | `keep_local_for_now` | Revisit only if repeated parity debt remains. |
| explicit stable slot identity beyond `draw_x` | Not available | Not exported | `expand_protocol` only if repeated bugs continue | Do not add yet without evidence. |

### Monster powers synthesized by Rust

| Debt site | Current Rust behavior | Current Java export | Action | Notes |
|---|---|---|---|---|
| `GuardianThreshold` seeded as synthetic power | Rust injects power from monster-specific fields | Java exports `guardian_dmg_threshold` and friends | `keep_local_for_now` | This is mostly a Rust internal representation mismatch. |
| `GremlinWarrior Angry` missing-power policy | Rust may preserve synthetic/internal state | Java should already expose `Angry` in powers when present | `keep_local_for_now` | Audit if this still fires in modern live logs before changing protocol. |

### Relics

| Debt site | Current Rust behavior | Current Java export | Action | Notes |
|---|---|---|---|---|
| relic `used_up` fallback policies | Rust live path should trust protocol fields first; compatibility fallback still exists in code for legacy scenarios | Java exports `used_up` | `consume_now` | Continue deleting fallback usage as explicit runtime fields land. |
| `ArtOfWar.counter` special handling | Rust preserves previous counter when snapshot uses `-1` | Java exports `counter` only | `keep_local_for_now` | Could become protocol work later if this keeps causing drift. |

### Session / interaction protocol

| Debt site | Current Rust behavior | Current Java export | Action | Notes |
|---|---|---|---|---|
| reward interaction continuity | Rust has audit/state machine fallback | Java already has `protocol_meta.reward_session` | `consume_now` | Existing draft is partly implemented; Rust should trust session truth first. |
| future noncombat session continuity beyond rewards | Rust still infers from screens in some places | Partial support only | `expand_protocol` | Good next protocol family after combat hidden-state debt. |

## Immediate Implementation Order

### 1. Rust importer cleanup

Do these before changing Java:

- continue auditing all `PowerExtraDataPolicy` cases against Java `damage/misc`
- remove remaining fallback-only assumptions that are no longer needed in live paths
- keep reward audit preferring `protocol_meta.reward_session` whenever available
- consume `Pocketwatch.first_turn` now that Java exports it

### 2. Java protocol batch for hidden monster runtime

First candidates:

- `darkling_first_move`
- `darkling_nip_dmg`
- `lagavulin_idle_count`
- `lagavulin_is_out_triggered`

### 3. Revisit identity/slot protocol only if bugs keep recurring

Do not add slot fields speculatively.

## What This Means Operationally

When a new parity bug lands in `sync`, ask these in order:

1. Is Java already exporting the truth?
2. If yes, fix Rust importer instead of growing `carry`.
3. If no, is this a recurring hidden-state class?
4. If yes, add it to the next Java protocol batch.
5. Otherwise keep the Rust workaround local and temporary.
