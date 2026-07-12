# Whirlwind Draw-Position Counterfactual Design

## Goal

Test whether taking the unupgraded Whirlwind offered at A2F29 could plausibly turn the frozen A2F32 Collector combat into a win. This is diagnostic evidence for card-acquisition policy, not a production policy change.

## Existing Evidence

Seed `20260712002` reaches The Collector at 25/40 HP with a 21-card deck. The ordinary all-potion search, two Collector tactic lanes, and a 40/40 HP counterfactual all find zero wins. Static review reports thin AOE/minion control and thin boss scaling. The run skipped an unupgraded Whirlwind at A2F29 despite assigning it a positive probe score.

The saved combat begins after the combat-start shuffle. Adding Whirlwind to one arbitrary draw position would therefore overstate certainty: the exact draw position that would have resulted from taking the card upstream is not recoverable from this snapshot.

## Considered Approaches

### 1. Draw-position portfolio (selected)

Insert one unupgraded Whirlwind at several representative positions in the frozen draw pile while leaving every other combat field unchanged. Use short searches to screen the positions, then rerun the strongest position with the full comparison budget.

This isolates card availability while making draw-order uncertainty explicit. It does not claim to reproduce the exact upstream shuffle.

### 2. Put Whirlwind in the opening hand

This is a useful optimistic upper bound but is too favorable to answer the acquisition question by itself. It is not the primary experiment.

### 3. Replay from the A2F29 reward boundary

This would be the most faithful experiment, but the run capsule does not contain a resumable exact state at that boundary. Reconstructing a prefix would reintroduce the expensive whole-run replay problem and is outside this slice.

## Experiment

Use a temporary internal test so the diagnostic can load the exact `CombatCase` without adding a permanent CLI or report schema.

1. Reconfirm the frozen baseline has no win under the selected search configuration.
2. Add a fresh unupgraded Whirlwind with a collision-free UUID to representative draw-pile positions: next draw, middle, and last draw.
3. Run each position with the same all-legal-potion policy and a short equal budget.
4. Rank results by exact win first, then living enemies, remaining enemy HP, and player HP.
5. Rerun the strongest position with the full 8-second/800,000-node budget.
6. If a winning witness is found, replay it exactly against the same mutated position and require an exact terminal win before calling the counterfactual successful.
7. Write the comparison as ignored JSON evidence under the seed capsule's `diagnostics` directory.
8. Remove the temporary test and verify the tracked worktree returns to its pre-experiment state.

The original 21-card combat remains the control. The counterfactual has 22 cards because the actual A2F30 purge remains part of the observed route and Whirlwind is the only added acquisition.

## Interpretation

- A robust win from more than one representative position is strong evidence that skipping Whirlwind was a meaningful acquisition error.
- A win only from the next-draw position is an optimistic existence result, not enough to justify a general card-reward change.
- No win under the full rerun weakens the Whirlwind hypothesis but does not prove the card could never help under a different upstream shuffle.
- The experiment cannot distinguish all other route alternatives and must not directly encode a Collector-specific Whirlwind rule.

## Non-Goals

- no permanent test for this seed;
- no new production CLI, schema, or card-injection API;
- no mutation of the original combat case or run capsule;
- no card-reward policy change in this slice;
- no claim that a representative draw position is the exact upstream shuffle result.
