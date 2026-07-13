# Pending Persistent Burden Cutpoint Design

## Goal

Make the persistent-burden cutpoint probe identify the action that actually creates a lasting
curse obligation during combat, rather than the later action that merely finishes combat and
materializes an already-pending curse into the run deck.

The correction must remain generic. It may understand run-level curse additions and the engine's
pending persistent-change protocol, but it must not identify Writhing Mass, Implant, Parasite, or
monster move ids. Full-line adjudication continues to use the finalized run deck and UUID-aware
card deltas.

## Root Cause

`Action::AddCardToMasterDeck` appends `MetaChange::AddCardToMasterDeck(CardId)` to
`CombatState.meta.meta_changes`. `finish_active_combat` drains those changes and only then mutates
`RunState.master_deck`.

The existing cutpoint locator compares `RunState.master_deck` before and after each retained input.
For a curse acquired before the final blow, this reports the killing input as the trigger. An
alternative input that keeps combat unresolved then appears clean only because the pending change
has not yet been absorbed. The saved Writhing Mass review exhibits exactly this shape: the reported
triggers are late killing Whirlwinds, while unresolved End Turn or attack alternatives are labeled
as clean plan changes.

## Considered Approaches

1. Keep finalized run-deck comparison and weaken the conclusion. This preserves final-outcome truth
   but cannot locate the actionable transition and leaves one-action classifications misleading.
2. Read monster runtime flags such as `used_mega_debuff`. This locates the target case but duplicates
   combat mechanics and does not generalize to other persistent card effects.
3. Use a composite persistent-curse ledger. Count curse cards already present in the run deck plus
   pending curse additions in the active combat. Compare this total across each stable input. This
   follows the engine's actual persistence protocol and is the chosen approach.

## Design

Add a focused run-control helper that snapshots persistent curse burden by `CardId`:

- count every curse currently in `RunState.master_deck`;
- count every active-combat `MetaChange::AddCardToMasterDeck(card_id)` whose card definition is a
  curse;
- expose a deterministic positive delta between two snapshots as `(card, count)` entries sorted by
  card id.

Combining materialized and pending counts is essential. When combat finishes, a pending entry is
drained and a newly UUID-assigned deck card appears; the composite count remains unchanged, so the
finalization input is not mistaken for a new burden.

The cutpoint locator snapshots this ledger before and after every exactly replayed retained input.
The first positive delta is the trigger. It preserves the session and combat position immediately
before that input and stores the typed curse-count delta as trigger evidence.

The one-action probe uses the same ledger around each cloned input. An unresolved input that creates
a pending curse is classified as `new_curse`; an input that merely materializes an existing pending
curse is not. Clean victory and plan-change classification remain downstream of the corrected burden
test.

The public V1 diagnostic replaces the UUID-oriented one-action `gained_curses` field with
`gained_curse_counts` and adds the same typed counts to each cutpoint's trigger evidence. This
feature branch has not been integrated, so correcting the just-added V1 schema is preferable to
shipping knowingly false semantics or introducing a redundant V2. Final candidate adjudication and
its UUID-bearing `CardSnapshot` output are unchanged.

## Ownership and Boundaries

- The engine remains the owner of `MetaChange` creation and combat-finish absorption.
- Run-control owns the composite observation because it can see both the run and active combat.
- Combat Search V2 is not changed and no additional search is run by this fix.
- `combat_case_review` only serializes the typed result. It does not inspect cards, meta changes,
  monsters, or move ids.

## Verification

- A ledger unit test proves pending-to-materialized transfer does not create a second delta.
- A cutpoint regression fixture places a curse meta change before a stable pending choice and combat
  completion after the choice. The current implementation reports the completion input; the fixed
  implementation must report the earlier input.
- A one-action regression test proves an unresolved input that appends a pending curse is classified
  as `new_curse`.
- Existing Reactive plan-change, grouping, conclusion-precedence, CLI serialization, and architecture
  tests remain green.
- Re-run the saved case once. Its cutpoints must move earlier than the final killing actions, and a
  plan-change conclusion is accepted only when the alternative is clean against the composite
  ledger.
- Finish with the full library, `combat_case_review`, architecture-boundary, diff, and clean-status
  checks.

## Non-Goals

- No monster-specific burden inference or search scoring change.
- No suffix search, forced-root search, candidate-cap increase, or policy relaxation in this fix.
- No attempt to model non-curse persistent changes as harmful.
- No change to how the engine applies or materializes combat meta changes.
