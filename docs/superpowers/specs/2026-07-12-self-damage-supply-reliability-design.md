# Self-Damage Supply Reliability Design

**Date:** 2026-07-12

## Goal

Stop a single exhaust-on-use self-damage card such as Offering from making Rupture look like a fully supported package and a stable boss-scaling source.

The model should preserve the real synergy: Offering can trigger Rupture once and therefore makes the pairing more credible than an unsupported Rupture. It must not treat that one trigger as equivalent to a self-damage source that can recur during a long fight.

## Root Cause

The semantic layer currently records only whether `CardSelfDamage` is available. It loses the source lifetime before the strategy layer consumes the fact.

That flattened boolean causes three independent overclaims:

- package state promotes Offering plus Rupture directly to `Supported`;
- reward admission and construction pressure award full package credit;
- strength and deck-role profiles count Rupture as stable boss scaling.

The resulting score is not one bad constant. It is the same incomplete fact being rewarded at several downstream boundaries.

## Supply Model

Extend `DeckMechanicContext` with repeatable-event information while retaining the existing `event_streams` collection.

For `CardSelfDamage`:

- a direct self-damage effect on a card that exhausts itself is **limited** supply;
- a direct self-damage effect on a card that does not exhaust itself is **repeatable** supply because it can recur on later deck cycles;
- a turn-start or turn-end installed handler that loses HP from a card is **repeatable** supply;
- no source is **absent** supply.

The implementation should infer these categories from `CardDefinition` effects and handlers. It must not use a card-ID allowlist and must not introduce a fixed source-count threshold.

The existing generic event-presence behavior remains unchanged. In particular, this slice does not redefine Exhaust package semantics merely because Exhaust also uses event streams.

## Package Maturity

Self-damage package maturity receives one targeted refinement:

| Supply | Rupture payoff | Maturity |
| --- | --- | --- |
| absent | absent | `None` |
| limited or repeatable | absent | `SourceOnly` |
| absent | present | `PayoffOnly` |
| limited | present | `Seeded` |
| repeatable | present | `Supported` |

This makes Offering plus Rupture a real but unfinished engine seed. Bloodletting, Hemokinesis, Combust, or Brutality can provide repeatable support through their semantic shape.

No score constant changes are required. Existing reward-admission behavior already gives `Seeded` less authority than `Supported`; construction pressure will therefore stop receiving full self-damage package support automatically.

## Stable Strength and Boss Scaling

Rupture's `CardSelfDamage -> Strength` handler is conditional on the deck's supply quality:

- with repeatable supply, it may count as a stable Strength source;
- with only limited supply, it is conditional/fragile rather than stable;
- with absent supply, it is unsupported.

`DeckRoleInventory`, `strength_profile_v1`, and boss-scaling evidence must agree on this distinction. A candidate Rupture backed only by limited supply must not receive `boss-scaling-source` or repair-reliability authority.

The startup profile may continue reporting the total number of self-damage cards for diagnostics. Its `persistent_strength_source_count`, which is derived from the Strength profile, must no longer become positive from Rupture plus limited supply alone.

## Architecture Boundary

The source-lifetime fact belongs in the semantic/context layer. Package state and strategic profiles consume it; they do not rediscover it from card IDs.

Expected implementation area:

- `src/ai/analysis/card_semantics.rs` for repeatable event supply derivation;
- `src/ai/strategy/package_state.rs` for the self-damage maturity refinement;
- `src/ai/strategy/deck_role_inventory.rs` and `src/ai/strategy/boss_scaling_evidence.rs` for stable-source authority;
- `src/ai/strength_profile_v1.rs` for the legacy startup/diagnostic consumer.

Small adjacent changes are allowed only where required to pass the same supply fact across an existing boundary. Do not add Collector-specific policy, seed-specific bonuses, combat-search ordering rules, or general score retuning.

## Test Design

Use behavior-focused contract tests rather than freezing a seed outcome:

1. Offering produces `CardSelfDamage` availability but not repeatable supply.
2. Bloodletting or Hemokinesis produces repeatable direct supply.
3. Combust or Brutality produces repeatable triggered supply.
4. Offering plus Rupture is `Seeded`, not `Supported`.
5. A repeatable source plus Rupture is `Supported`.
6. Offering alone does not make Rupture a stable deck-role or Strength-profile source.
7. A repeatable source does make Rupture stable.
8. A Rupture candidate backed only by Offering receives no relevant boss-scaling-source evidence.

Follow red-green TDD. Prefer focused semantic and strategy tests during implementation. Avoid asserting a precise total card score or requiring seed `20260712002` to select a particular reward.

## Validation

At completion run:

- focused semantic, package-state, role-inventory, Strength-profile, and boss-scaling tests;
- `cargo fmt --all -- --check`;
- `cargo test --lib`;
- `cargo test --test architecture_runtime_boundaries`;
- `git diff --check`.

## Non-Goals

- Do not encode a rule such as "Rupture needs three self-damage cards."
- Do not estimate exact trigger frequency, draw order, or fight length yet.
- Do not change Offering's independent draw and energy value.
- Do not globally require repeatability for Exhaust, Block, or other packages.
- Do not tune Collector, route, shop, or combat-search policy in this slice.
- Do not require this repair alone to make the investigated seed win.
