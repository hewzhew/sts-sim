# Campfire Owner Strategy Boundary Design

## Goal

Stop the owner-audit campfire path from independently ranking smith targets
without run-level strategy context. Preserve the existing ownership of relic
campfire actions while making Rest and Smith consume the existing strategic
campfire policy.

## Decision

Make two narrow corrections:

1. Record `Limit Break` as an upgrade whose mechanical delta removes Exhaust,
   matching combat runtime behavior.
2. Ask `campfire_policy_v1` for the preferred Rest/Smith action before the
   owner falls back to its existing Toke, Dig, Lift, Recall, and final Rest
   handling.

The owner maps only executable `Rest` and `Smith` policy actions to visible
inputs. A policy `Stop` is not an owner gap by itself; it returns control to
the existing non-strategic campfire fallbacks. The old deck-only smith ranking
is removed from this owner so there is one strategic source of smith truth.

## Stable Tests

Add only boundary and semantic contracts:

- card analysis reports that upgrading `Limit Break` removes Exhaust;
- when a Smith action is available, the owner uses the target selected by the
  existing campfire strategy rather than its former deck-only scorer;
- a policy stop still permits the owner's existing non-Smith fallback path.

Do not assert that the Act 2 Collector deck, or any other temporary deck
snapshot, must upgrade a particular card. Candidate scoring and exact card
choice remain free to evolve behind the strategic policy boundary.

## Non-Goals

- Do not add a boss search lane or increase combat budgets.
- Do not redesign Toke, Dig, Lift, or Recall strategy.
- Do not add seed replay, checkpoint, or panel tests.
- Do not preserve the old deck-local ranking as a second Smith policy.
