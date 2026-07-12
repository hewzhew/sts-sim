# Card Fact Semantics Repair Design

**Date:** 2026-07-12

## Goal

Correct three factual card descriptions used by non-combat strategy so later Collector, shop, and mainline-policy work starts from truthful mechanics.

This slice does not tune scores to make seed `20260712002` win. It only makes the strategy layer describe Reaper, Dark Shackles, and Shockwave consistently with their implemented game behavior.

## Scope

### Reaper

Keep its existing Strength payoff facts and add the two missing direct effects:

- `AreaDamage`, because it damages all enemies;
- `RecoverCurrentHp`, because it heals from unblocked damage dealt.

Reaper remains conditional on Strength for large healing. The factual model must nevertheless preserve its baseline multi-target damage and sustain instead of reducing it to an unsupported Strength payoff.

### Dark Shackles

Add a strategy `CardDefinition` with:

- `EnemyStrengthDown`;
- `ExhaustsSelf`.

The current semantic vocabulary does not distinguish temporary Strength reduction from permanent Strength reduction. This slice reuses the existing coarse mechanic already used by the legacy card-facts layer and does not introduce a new temporal-debuff type.

### Shockwave

Keep:

- `Weak`;
- `Vulnerable`;
- `ExhaustsSelf`.

Remove `EnemyStrengthDown` from both the strategy `CardDefinition` and the legacy `card_facts` mapping. Shockwave applies Weak and Vulnerable; it does not directly reduce enemy Strength.

## Architecture Boundary

The authoritative edits stay inside factual semantic sources:

- `src/ai/analysis/card_semantics.rs` for the newer non-combat strategy pipeline;
- `src/ai/card_reward_policy_v1/facts.rs` only where its Shockwave fact is also incorrect.

No score constants, candidate lanes, Collector-specific rules, shop bundle rules, combat search behavior, or card runtime implementations change in this slice.

## Test Design

Add small semantic contract tests that assert mechanics rather than seed outcomes:

1. Reaper contains `DamageUses(Strength)`, `AreaDamage`, and `RecoverCurrentHp`.
2. Dark Shackles contains `Provide(EnemyStrengthDown)` and `ExhaustsSelf`.
3. Shockwave contains `Provide(Weak)`, `Provide(Vulnerable)`, and `ExhaustsSelf`, but not `Provide(EnemyStrengthDown)`.
4. Legacy Shockwave facts report Weak and Vulnerable but zero direct enemy Strength reduction.

Follow red-green TDD: introduce each failing contract before changing production facts. These tests intentionally avoid asserting card scores, candidate ordering, or a frozen seed decision.

## Validation

Run the focused semantic tests during red-green work. At completion run:

- `cargo fmt --all -- --check`;
- the full library test suite;
- `architecture_runtime_boundaries`;
- `git diff --check`.

## Non-Goals

- Do not add Collector-specific acquisition evidence yet.
- Do not change Rupture support or Strength reliability yet.
- Do not change `ProbeOnly` expansion behavior yet.
- Do not change shop liquidity or card-removal policy yet.
- Do not assert that any of these cards must always be selected.
