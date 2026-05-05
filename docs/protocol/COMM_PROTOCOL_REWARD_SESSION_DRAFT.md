# Communication Protocol Draft: Reward Session and Future Debug Hooks

## Purpose

This draft defines a protocol upgrade for `CommunicationMod` so Rust no longer has to infer
whether a human card reward interaction is still active by polling screen transitions such as
`MASTER_DECK_VIEW` and `MAP`.

The immediate goal is:

- make human card reward audit explicit and robust
- reduce blind `STATE`/`WAIT` polling during temporary inspection screens
- give later debug/scenario-control work a clean place to extend the protocol

This document does **not** require immediate implementation of debug commands. It only reserves
protocol structure so that future debug work does not need another ad hoc redesign.

## Current Protocol Reality

Current state responses already expose:

- `available_commands`
- `ready_for_command`
- `in_game`
- `protocol_meta.response_id`
- `protocol_meta.state_frame_id`
- `protocol_meta.last_command*`
- `protocol_meta.emitted_for_command*`
- `protocol_meta.recent_human_card_reward_choice`

Current reward audit limitations:

- Java only emits a reward-related payload **after** a human reward choice has completed.
- Rust must infer whether reward context is still alive by watching `screen_type`,
  `screen_name`, and `room_phase`.
- Temporary inspection screens such as `MASTER_DECK_VIEW` and `MAP` force Rust into
  polling logic because no explicit reward-session state exists.

This is why Rust still needs a hold/disposition state machine even after log throttling and
`WAIT 30` support.

## Design Goals

The upgraded protocol should:

- explicitly identify each reward interaction with a stable session id
- distinguish `active`, `temporarily_offscreen`, and `closed`
- keep reward context alive while the player inspects map/deck/boss information
- expose enough metadata to audit source, offered cards, and final resolution
- remain additive so Rust can support old and new protocol versions during migration
- reserve a capability mechanism for future debug/scenario commands

## Proposed Additions

### 1. `protocol_meta.reward_session`

Add an optional object under `protocol_meta`:

```json
{
  "protocol_meta": {
    "response_id": 1234,
    "state_frame_id": 567,
    "reward_session": {
      "session_id": "reward-1234-1",
      "kind": "card_reward",
      "state": "active",
      "source_kind": "combat_reward",
      "source_hint": "none",
      "offered_card_ids": ["Shrug It Off", "Flame Barrier", "Warcry"],
      "choice_count": 3,
      "source_response_id": 1229,
      "source_state_frame_id": 562,
      "opened_screen_type": "CARD_REWARD",
      "opened_room_phase": "COMPLETE",
      "temporarily_offscreen": false
    }
  }
}
```

This object is present whenever a reward interaction still exists as live protocol context,
even if the player has temporarily left the reward screen to inspect something else.

### 2. `protocol_meta.recent_human_card_reward_choice`

Keep the existing object, but define it as the **resolution event** for a previously active
reward session.

Add one required field:

- `session_id`

Example:

```json
{
  "protocol_meta": {
    "recent_human_card_reward_choice": {
      "session_id": "reward-1234-1",
      "screen_type": "CARD_REWARD",
      "choice_kind": "card",
      "choice_index": 1,
      "choice_label": "Flame Barrier",
      "card_id": "Flame Barrier",
      "choice_source": "combat_reward",
      "choice_destination": "deck",
      "offered_card_ids": ["Shrug It Off", "Flame Barrier", "Warcry"],
      "source_response_id": 1229,
      "source_state_frame_id": 562
    }
  }
}
```

This lets Rust reconcile:

- the live reward session it has been tracking
- the final human resolution event

without guessing from screens.

## Reward Session Schema

### Required fields

- `session_id: string`
- `kind: string`
  - initial supported value: `card_reward`
- `state: string`
  - one of:
    - `active`
    - `temporarily_offscreen`
    - `resolved`
    - `closed_without_choice`
- `source_kind: string`
  - examples:
    - `combat_reward`
    - `card_reward_potion`
    - `discovery`
    - `toolbox`
- `offered_card_ids: string[]`
- `source_response_id: integer|null`
- `source_state_frame_id: integer|null`

### Optional fields

- `source_hint: string|null`
- `choice_count: integer`
- `opened_screen_type: string`
- `opened_room_phase: string`
- `temporarily_offscreen: boolean`
- `offscreen_screen_name: string|null`
- `offscreen_screen_type: string|null`

## Session State Semantics

### `active`

Meaning:

- the reward UI is logically still open
- the player may choose now
- Rust should keep audit state alive

Typical examples:

- still on `CARD_REWARD`
- card reward spawned by potion/discovery-like effect and still open

### `temporarily_offscreen`

Meaning:

- reward UI context is still alive
- player is inspecting another screen
- Rust must **not** abandon the audit

Typical examples:

- `MASTER_DECK_VIEW`
- `MAP`
- any future inspect screen explicitly marked as reward-preserving

This state removes the need for Rust to guess from `screen_name`.

### `resolved`

Meaning:

- a final card reward choice was made
- `recent_human_card_reward_choice` should appear with the same `session_id`
- session may either disappear immediately after one state response or remain as
  `resolved` for a single frame

### `closed_without_choice`

Meaning:

- reward context is over
- no final choice event exists
- examples:
  - skip
  - cancel
  - reward replaced by some exceptional transition

Rust should close audit cleanly without blind inference.

## Java Ownership Rules

Java should own reward session lifecycle.

### Session creation

Create a new `reward_session` when:

- card reward becomes logically open for human interaction
- potion/discovery-generated card reward opens

### Session persistence

Keep the same `session_id` while:

- player stays on reward screen
- player temporarily leaves for permitted inspection screens

### Session resolution

When player picks, skips, or cancels:

- emit `recent_human_card_reward_choice` with `session_id`
- or emit `reward_session.state = closed_without_choice`

### Session disposal

After a stable post-resolution state has been emitted at least once:

- session may be removed from later frames

## Rust Consumption Rules

When `protocol_meta.reward_session` exists:

- prefer it over current screen heuristics
- keep pending audit alive while `state` is `active` or `temporarily_offscreen`
- only abandon when `state == closed_without_choice`
- finalize when `recent_human_card_reward_choice.session_id` matches pending session

When `protocol_meta.reward_session` is absent:

- continue using the current heuristic fallback
- this preserves backward compatibility during migration

## Migration Plan

### Phase 1: Additive Java emission

Java emits:

- `protocol_meta.reward_session`
- existing `recent_human_card_reward_choice`

Rust still keeps fallback heuristics.

### Phase 2: Rust preference switch

Rust reward audit logic prefers:

1. explicit `reward_session`
2. legacy screen heuristics only if explicit session missing

### Phase 3: Heuristic cleanup

Once the new protocol is stable:

- shrink the heuristic hold/abandon rules
- keep only a compatibility fallback for old builds

## Future Debug Command Hooks

Debug commands are intentionally **not** specified in full here, but the protocol should reserve
two additive extension points now.

### 1. `protocol_meta.capabilities`

Example:

```json
{
  "protocol_meta": {
    "capabilities": {
      "reward_session": true,
      "debug_commands": false,
      "scenario_control": false
    }
  }
}
```

This lets Rust know whether:

- explicit reward sessions are supported
- debug command namespace is supported
- scenario-control commands are available

### 2. Reserved debug command namespace

Future commands should use a stable prefix:

- `debug ...`

Examples for later:

- `debug hp set 20`
- `debug gold set 150`
- `debug hand add Bash`
- `debug relic add Slavers Collar`
- `debug monster spawn JawWorm`
- `debug reward open card Shrug It Off,Flame Barrier,Warcry`

This keeps debug/scenario-control separate from normal gameplay commands like:

- `play`
- `choose`
- `state`
- `wait`

## Why This Is Better Than Smarter Polling Alone

Smarter polling helps, but it cannot answer:

- whether reward context is still logically open
- whether deck/map inspection is temporary or final
- whether a later choice belongs to the same reward interaction

Only Java has authoritative access to that context.

Therefore:

- polling improvements are useful short-term
- explicit reward sessions are the correct long-term fix

## Open Questions

These do not block the draft.

1. Should skip/cancel create `recent_human_card_reward_choice` with `choice_kind=skip`,
   or only use `reward_session.state=closed_without_choice`?
2. Should potion/discovery-created card rewards share the same `kind=card_reward` with
   different `source_kind`, or split into multiple kinds?
3. Should `reward_session` remain in `protocol_meta`, or move under `game_state`?
   Current recommendation: keep it in `protocol_meta`, because it is protocol interaction
   context, not world state.

## Recommendation

Implement this in the following order:

1. Add `protocol_meta.reward_session`
2. Add `session_id` to `recent_human_card_reward_choice`
3. Add `protocol_meta.capabilities.reward_session`
4. Switch Rust reward audit to prefer explicit session semantics
5. Later, design the separate `debug` command family on top of the same capability model
