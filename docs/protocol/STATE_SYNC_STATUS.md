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

As of the current hard-reset batch, old traces are no longer treated as
supported by default. Missing `runtime_state` for migrated slices should fail
fast instead of triggering carry/shim repair.

Manual scenario protocol truth samples now exist for:

- `GuardianThreshold`
- `Angry`
- `Combust`
- `Stasis`

See:

- [MANUAL_SCENARIO_SAMPLE_INDEX.md](MANUAL_SCENARIO_SAMPLE_INDEX.md)

These samples prove protocol/importer truth only. They should not be read as
full mechanic regression coverage for the corresponding content.

Checked-in fixture copies now live under:

- `tests/protocol_truth_samples/`

and are exercised by:

- `tests/protocol_truth_samples.rs`

## What Rust Already Imports Directly

### Powers

Rust currently consumes protocol truth for:

- `power.just_applied`
- `power.runtime_state.hp_loss` for `Combust`
- `power.runtime_state.card_uuid` for `Stasis`
- `power.runtime_state.base_power` for `Malleable`
- `power.runtime_state.stored_amount` for `Flight`
- `power.runtime_state.damage` for `Panache` and `The Bomb`
- `power.damage` / `power.misc` for non-migrated extra-data slices

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

- `Guardian.guardian_threshold`
- `GremlinWarrior.angry_amount`
- `Hexaghost.activated`
- `Hexaghost.orb_active_count`
- `Hexaghost.burn_upgraded`
- `Darkling.first_move`
- `Darkling.nip_dmg`
- `Chosen.first_turn`
- `Chosen.used_hex`
- `Lagavulin.idle_count`
- `Lagavulin.is_out_triggered`

### Relics

Rust currently consumes explicit protocol runtime state for:

- `Centennial Puzzle.used_this_combat`
- `ArtOfWar.gain_energy_next`
- `ArtOfWar.first_turn`
- `Pocketwatch.first_turn`
- runtime-only `used_up` for `HoveringKite`, `LizardTail`, `Necronomicon`

## What Is Still Outstanding

### Rust representation or adapter debt still present

- `seed_move_history_from_snapshot`
- non-migrated power extra-data slices still rely on `damage` / `misc`

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

1. migrate any remaining non-slice power extra-data debt off `damage` / `misc`
2. delete any remaining top-level relic `used_up/counter` reliance after strict trace refresh
3. prune or refresh old historical traces that still encode retired top-level monster fields

## Related Docs

- [PROTOCOL_TRUTH_RULES.md](PROTOCOL_TRUTH_RULES.md)
- [COMM_PROTOCOL_DEBT_BACKLOG_2026-04-12.md](COMM_PROTOCOL_DEBT_BACKLOG_2026-04-12.md)
- [COMM_PROTOCOL_EXPANSION_AUDIT_2026-04-12.md](COMM_PROTOCOL_EXPANSION_AUDIT_2026-04-12.md)
