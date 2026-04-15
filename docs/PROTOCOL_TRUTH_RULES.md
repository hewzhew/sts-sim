# Protocol Truth Rules

This document is the hard rulebook for combat parity work.

It exists to prevent the project from sliding back into a shadow sync system.

## Core Rule

For hidden combat runtime state, the source of truth must be:

1. Java engine field
2. Java protocol export
3. Rust importer
4. Rust engine runtime

Not the other way around.

Do not invent a Rust-side replacement state model first and only later discover
that Java already had a different, more exact field.

## What Is Allowed

### Allowed

- Export a Java private field through `CommunicationMod` if it affects combat parity.
- Mirror Java runtime state explicitly in Rust if the Rust engine genuinely needs it.
- Keep Rust-only derived fields for search, UI, or diagnostics as long as they are
  clearly downstream of protocol truth.
- Rebuild Rust combat state from the current Java snapshot after a parity failure.

### Not Allowed

- Using `carry` as the default way to recover hidden runtime state during live sync.
- Letting replay rebuild or live comparator silently inherit previous Rust runtime
  state as if it were truth.
- Replacing Java semantics with a "cleaner" Rust abstraction before protocol truth
  is available.
- Keeping old fallback logic indefinitely after protocol truth has been added and
  consumed.

## Definitions

### Protocol Truth

Data explicitly exported by Java and directly consumed by Rust importer logic.

Examples:

- `power.just_applied`
- `power.misc`
- `power.damage`
- `power.card.uuid`
- `monster.hexaghost_activated`
- `relic.runtime_state.used_this_combat`

### Importer

Code that maps Java snapshot fields into Rust state.

Relevant files today:

- [build.rs](d:\rust\sts_simulator\src\diff\state_sync\build.rs)
- [sync.rs](d:\rust\sts_simulator\src\diff\state_sync\sync.rs)

Importer code may translate representation shape, but it should not guess missing
runtime semantics if protocol truth can be added instead.

### Carry

Previous-Rust-state inheritance used to repair missing or ambiguous runtime state.

Carry is not a source of truth.

Carry may exist temporarily for old fixtures or one-off migration periods, but it
must not remain in the main live path.

## Current Boundary

As of the current refactor:

- live `sync_state(...)` is importer-first and no longer performs global
  previous-state carry at the end
- live comparator truth rebuild no longer performs global carry
- replay rebuild no longer performs global carry
- reward audit combat rebuild no longer performs global carry

This is intentional.

If parity now fails because a field is missing, that is a protocol/importer issue
to fix, not a reason to restore shadow-state carry.

## Decision Rule For New Bugs

When a new parity bug appears, handle it in this order:

1. Check Java source and identify the real runtime field or lifecycle.
2. Check whether `CommunicationMod` already exports that truth.
3. If Java already exports it, fix the Rust importer.
4. If Java does not export it and the field is parity-relevant, extend protocol.
5. Only use a Rust-side temporary workaround if the above cannot be done in the
   current batch.

Do not jump directly to `carry`.

## Design Rule For Runtime Fields

If a monster, relic, or power has a hidden counter/flag/latch in Java, then the
default assumption should be:

- Rust will probably need the same field or a direct semantic equivalent.

Examples of the correct pattern:

- `Lagavulin.idleCount`
- `Lagavulin.isOutTriggered`
- `Darkling.firstMove`
- `Darkling.nipDmg`
- `CentennialPuzzle.usedThisCombat`
- `Pocketwatch.firstTurn`

Do not try to compress these into generic placeholders like:

- `move_history`
- `logical_slot`
- `used_up` carry

unless Java already uses those as the actual semantic source.

## Fallback Retirement Rule

If protocol truth has been added and Rust consumes it successfully, the matching
fallback must move toward deletion.

Acceptable temporary state:

- protocol truth exists
- fallback remains only for old fixtures or old logs
- fallback is clearly marked as compatibility-only

Unacceptable long-term state:

- protocol truth exists
- live path still depends on fallback
- nobody knows which path actually won

## Review Checklist

Before merging parity-sensitive changes, answer these questions:

1. What is the actual Java runtime field or lifecycle?
2. Is that truth exported directly?
3. Does Rust importer consume that exact truth?
4. Does live sync rely on previous Rust runtime state for this field?
5. If a fallback remains, is it compatibility-only and clearly marked?

If any of `1-4` is unclear, the implementation is not done.

## Long-Term Goal

The long-term goal is not "smart sync".

The long-term goal is:

- protocol-complete enough Java snapshots
- dumb importer
- honest rebuilds
- no shadow combat semantics outside the engine itself
