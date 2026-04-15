# State Sync Status

This document answers one narrow question:

What does `state_sync` currently do in the live main path, and what still
remains as protocol/importer debt?

It should stay short and current.

## Current Live Boundary

In the current live main path:

- `build_combat_state(...)` builds Rust combat state from Java snapshot truth
- `sync_state(...)` updates Rust combat state from Java snapshot truth
- live comparator truth rebuild does **not** perform global previous-state carry
- replay rebuild does **not** perform global previous-state carry
- reward-audit combat rebuild does **not** perform global previous-state carry

This means live parity work should now treat missing runtime state as:

- importer debt
- protocol debt
- or an actual engine bug

not as something to silently repair from the previous Rust frame.

## What Rust Already Imports Directly

### Powers

Rust currently consumes protocol truth for:

- `power.just_applied`
- `power.misc` for supported runtime-backed powers
- `power.damage` for supported runtime-backed powers
- `power.card.uuid` for `Stasis`

Examples already wired:

- `Combust`
- `Malleable`
- `Flight`
- `PanachePower`
- `TheBombPower`
- `Ritual`
- `Stasis`

### Monsters

Rust currently consumes explicit Java runtime fields for:

- `Hexaghost.activated`
- `Hexaghost.orbActiveCount`
- `Hexaghost.burnUpgraded`

### Relics

Rust currently consumes explicit protocol runtime state for:

- `Centennial Puzzle.used_this_combat`

## What Is Still Outstanding

### Java already exports it, Rust still needs to consume it

- `Pocketwatch.first_turn`

### Java does not yet export enough truth

These should move to protocol fields instead of remaining Rust guesses:

- `Darkling.first_move`
- `Darkling.nip_dmg`
- `Lagavulin.idle_count`
- `Lagavulin.is_out_triggered`

### Rust representation or adapter debt still present

- `seed_move_history_from_snapshot`
- some monster-specific runtime reconstruction paths
- legacy `used_up` fallback policies still present in code

These are no longer part of the preferred live truth path, but they still exist
as debt and should keep shrinking.

## What Should Not Come Back

Do not restore:

- global `carry_internal_runtime_state(...)` in live sync
- global `carry_internal_runtime_state(...)` in replay rebuild
- global `carry_internal_runtime_state(...)` in reward-audit combat truth rebuild

If parity becomes worse after carry removal, that is a signal that protocol truth
or importer coverage is still incomplete.

## Next Recommended Order

1. consume `Pocketwatch.first_turn`
2. add protocol fields for `Darkling`
3. add protocol fields for `Lagavulin`
4. delete any fallback path that becomes redundant after those land

## Related Docs

- [PROTOCOL_TRUTH_RULES.md](d:\rust\sts_simulator\docs\PROTOCOL_TRUTH_RULES.md)
- [COMM_PROTOCOL_DEBT_BACKLOG_2026-04-12.md](d:\rust\sts_simulator\docs\COMM_PROTOCOL_DEBT_BACKLOG_2026-04-12.md)
- [COMM_PROTOCOL_EXPANSION_AUDIT_2026-04-12.md](d:\rust\sts_simulator\docs\COMM_PROTOCOL_EXPANSION_AUDIT_2026-04-12.md)
