# Monster Runtime Truth Audit 2026-04-18

This audit is the phase-2 follow-up to [MONSTER_RUNTIME_STATE_FRAMEWORK.md](../design/MONSTER_RUNTIME_STATE_FRAMEWORK.md).

Scope:

- already-migrated stateful semantic monsters
- protocol truth completeness for hidden runtime state
- explicit execution-time runtime patches
- retirement of `move_history` fallback for hidden runtime truth

## Rules

- Hidden runtime truth must come from protocol truth or factory/spawn initialization.
- Execution-time runtime changes must flow through `Action::UpdateMonsterRuntime`.
- `move_history` is allowed only for Java rules that explicitly depend on `lastMove` / `lastTwoMoves`.
- `move_history` must not recover hidden runtime truth such as `first_turn`, `used_hex`, or `is_flying`.

## Audit Table

| Monster | Hidden runtime truth | Protocol truth | Explicit runtime patch | Hidden-truth history fallback retired | Allowed history use | Status |
| --- | --- | --- | --- | --- | --- | --- |
| Hexaghost | `activated`, `orb_active_count`, `burn_upgraded`, divider cache | Yes | Yes | Yes | Sequence history only | Good |
| Lagavulin | `idle_count`, `debuff_turn_count`, `is_out`, `is_out_triggered` | Yes | Yes | Yes | Sequence history only | Good |
| The Guardian | `damage_threshold`, `damage_taken`, `is_open`, `close_up_triggered` | Yes | Yes | Yes | Sequence history only | Good |
| Byrd | `first_move`, `is_flying` | Yes | Yes | Yes | `PECK`/`SWOOP`/`CAW` sequencing | Good |
| Chosen | `first_turn`, `used_hex` | Yes | Yes | Yes | `DRAIN`/`DEBILITATE` sequencing | Good |
| Looter | `slash_count`, `stolen_gold` | Yes | Yes | Yes | None | Good |
| Mugger | `slash_count`, `stolen_gold` | Yes | Yes | Yes | None | Good |
| Shelled Parasite | `first_move` | Yes | Yes | Yes | `lastMove`/`lastTwoMoves` sequencing only | Good |
| Healer | None | N/A | N/A | N/A | Heal/attack/buff sequencing only | Good |
| Snake Plant | None | N/A | N/A | N/A | `lastMove`/`lastMoveBefore`/`lastTwoMoves` sequencing only | Good |
| Louse | `bite_damage` | Yes | N/A | Yes | None | Good |
| Snecko | `first_turn` | Yes | Yes | Yes | `lastTwoMoves(BITE)` sequencing | Good |
| Champ | `first_turn`, `num_turns`, `forge_times`, `threshold_reached` | Yes | Yes | Yes | `lastMove`/`lastMoveBefore` sequencing only | Good |
| Darkling | `first_move`, `nip_dmg` | Yes | Partial | N/A | Legacy-heavy | Debt remains |

## Notes

### Byrd

- `runtime_state.first_move` and `runtime_state.is_flying` are exported by `CommunicationMod`.
- Rust now requires these fields to be protocol-seeded or factory-seeded before semantic roll logic runs.
- Remaining `move_history` usage is intentional Java sequence logic, not hidden-state recovery.

### Chosen

- `runtime_state.first_turn` and `runtime_state.used_hex` are exported by `CommunicationMod`.
- Rust now requires these fields to be protocol-seeded or factory-seeded before semantic roll logic runs.
- Remaining `move_history` usage is limited to the Java branch that avoids repeating `DRAIN` and `DEBILITATE`.

### Louse

- `runtime_state.bite_damage` is now treated as strict protocol truth.
- Rust no longer falls back to `move_base_damage` during split truth import.
- This removes the last hidden-state recovery path for Louse parity.

### Looter and Mugger

- `runtime_state.slash_count` and `runtime_state.stolen_gold` are now exported by `CommunicationMod`.
- Rust semantic execution updates thief runtime explicitly when gold is stolen.
- Death rewards now rely on seeded thief runtime truth instead of reconstructing it from sequence history.

### Shelled Parasite

- `runtime_state.first_move` is now exported by `CommunicationMod`.
- Rust semantic roll logic requires `first_move` to be protocol-seeded or factory-seeded.
- `move_history` is still used for Java's explicit `lastMove` / `lastTwoMoves` branching only.

### Healer

- `Healer` does not require hidden runtime truth from protocol.
- The semantic migration exposed a small framework gap, so group-heal intent is now represented explicitly as `MonsterMoveSpec::Heal(HealSpec { target: AllMonsters, ... })`.
- Execution still expands group heal and group strength buff into per-monster `Action::Heal` / `Action::ApplyPower` calls in `take_turn_plan`, which keeps the target set explicit and avoids pretending helpers already support generic group targeting.
- Action-family review aligned two generic handlers with Java:
  - `handle_heal` now ignores `is_dying` monster targets, matching `AbstractCreature.heal(...)`.
  - `handle_apply_power` now ignores escaped monster targets, matching `ApplyPowerAction.update()`.
- `RollMonsterMove` was reviewed but left unchanged in this pass; Java itself does not add an extra dead/escaped guard there, and changing it now risks interfering with half-dead/revival style monsters.

### Snake Plant

- `Snake Plant` does not require hidden runtime truth from protocol.
- The semantic migration is purely a Java-sequence port:
  - `lastTwoMoves(CHOMPY_CHOMPS)`
  - `lastMove(SPORES)`
  - and at A17 specifically `lastMoveBefore(SPORES)`
- `use_pre_battle_action` is now wired through semantic dispatch so `Malleable` no longer depends on legacy paths.
- `on_death` is also routed through semantic dispatch to the default no-op implementation, which removes one more unsupported hallway-monster death edge.

### Snecko

- `runtime_state.first_turn` is now exported by `CommunicationMod`.
- Rust semantic roll logic requires `first_turn` to be protocol-seeded or factory-seeded.
- Remaining history usage is limited to Java's explicit `lastTwoMoves(BITE)` rule.

### Champ

- `runtime_state.first_turn`, `num_turns`, `forge_times`, and `threshold_reached` are now exported by `CommunicationMod`.
- Rust semantic roll logic requires this runtime truth to be protocol-seeded or factory-seeded before branch selection runs.
- `Champ` also uses the new semantic `on_roll_move` hook to model Java's `getMove()` side effects explicitly, instead of recovering them later from `move_history`.
- Remaining history usage is limited to Java's explicit `lastMove` / `lastMoveBefore` sequencing rules around `EXECUTE` and `GLOAT`.

### Darkling

- Runtime truth is protocol-exported, but the monster still relies on older legacy-style execution paths.
- It still needs a full semantic/runtime audit before it can be marked clean.

### The Guardian

- `runtime_state.guardian_threshold`, `damage_taken`, `is_open`, and `close_up_triggered` are now all treated as strict protocol truth.
- Rust split import seeds `entity.guardian` directly from those fields instead of silently falling back to the runtime default.
- This closes the live parity bug where imported Guardian state kept `damage_threshold = 0`, causing any positive hit to mis-trigger defensive mode.

## Outcome

Phase 2 is complete for the migrated hallway/stateful monsters that currently matter for live act1/act2 frontier work:

- hidden runtime truth no longer falls back to `move_history` for Byrd and Chosen
- thief runtime truth is explicit and protocol-seeded for Looter and Mugger
- Louse truth import is now strict
- remaining stateful debt is concentrated in monsters that have not completed semantic migration
