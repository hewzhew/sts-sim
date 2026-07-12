# Nonpositive Card Energy Payment Design

## Goal

Match the Java hand-play contract for non-X cards: being made playable does not
turn a negative card cost into energy gain. In particular, Blue Candle may make
an unplayable Curse playable and Medical Kit may make an unplayable Status
playable, but playing either must spend zero energy when its effective cost is
negative.

## Source Contract

Java separates legality from payment:

- `AbstractCard.canUse` permits otherwise unplayable Curse and Status cards when
  Blue Candle or Medical Kit applies.
- `AbstractPlayer.useCard` calls `energy.use(c.costForTurn)` only when
  `costForTurn > 0`.
- X-cost cards remain a separate path and consume their captured
  `energyOnUse` according to their existing implementation.

The Rust representation should preserve that observable behavior without
copying the Java class structure.

## Considered Approaches

1. **Normalize non-X payment at the hand-play boundary (recommended).** Compute
   payment as `max(effective_cost, 0)` before the affordability check and
   `spend_energy` call. This is the narrowest match for Java and leaves X-cost
   handling unchanged.
2. Clamp every negative argument inside `TurnState::spend_energy`. This provides
   broad protection, but it can hide unrelated invalid callers and changes a
   low-level primitive whose current contract is signed adjustment.
3. Special-case Blue Candle and Medical Kit. This fixes the two observed paths
   but duplicates relic knowledge in payment logic and can miss future legal
   nonpositive-cost cards.

Use approach 1. Do not modify `spend_energy`, card legality, relic hooks, or
X-cost card behavior in this repair.

## Tests

Add two mechanism-level tests through the real hand-play entry point:

1. Playing Writhe with Blue Candle starts and ends payment with the same energy;
   the existing Blue Candle HP-loss and exhaust behavior must still occur.
2. Playing an unplayable negative-cost Status with Medical Kit starts and ends
   payment with the same energy and still exhausts according to Medical Kit.

First run each test against the current implementation and confirm it fails
because energy increases. Then make the single payment normalization and rerun
the focused tests.

Existing positive-cost and X-cost tests are regression coverage; do not add a
large seed outcome as a permanent unit test.

## Scope Boundaries

- No combat-search scoring or action-priority changes.
- No run-control, owner-policy, dispatcher, or crate-boundary cleanup.
- No general refactor of card costs.
- No attempt to lock the A3F35 outcome in a unit test.

## Verification

After the red-green cycle:

1. Run the focused Blue Candle and Medical Kit tests.
2. Run the existing card/relic mechanism tests that cover positive and X costs.
3. Run the full library and `architecture_runtime_boundaries` suites.
4. Rerun the saved A3F35 capture and compare the selected line without treating
   any previous Writhe-first result as valid evidence.

## Success Criteria

- A legal negative-cost non-X hand play never increases energy.
- Blue Candle and Medical Kit retain their Java legality and exhaust semantics.
- Positive-cost and X-cost payment behavior is unchanged.
- The full required verification suites pass before the fix is committed.
