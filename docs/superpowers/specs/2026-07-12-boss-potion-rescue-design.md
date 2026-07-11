# Bounded Boss Potion Rescue Design

## Goal

Recover executable boss victories that the authoritative primary search misses when immediate child rollouts consume the search budget in potion- and generated-card-heavy branches.

The motivating frozen Collector case has a complete four-turn win with 35 HP remaining under `LazyOnPop`, while the otherwise equivalent `Immediate` search finds no win. The repair must preserve the current primary search and add no cost when that search succeeds.

## Design

Keep the existing boss primary lane unchanged. When it returns `CombatGap`, schedule exactly one existing `BossPotionRescue` lane after it.

Configure that rescue lane as follows:

- boss search node and wall-time budget;
- `LazyOnPop` child rollout in every act;
- adaptive no-potion rollout evaluation;
- all legal potion actions, with the existing boss potion budget;
- `AcceptedLineOnly` acceptance.

The existing orchestrator remains responsible for skipping post-primary lanes after a primary success, an operation-budget stop, or an outer wall-budget cap. No new replay, prefix, checkpoint, or run-control mechanism is introduced.

## Scope Boundaries

Do not change the boss primary profile, elite or hallway behavior, potion semantics, acceptance semantics, or combat search internals. Do not re-enable `BossNoPotion`, `BossTimeEaterClock`, or `QualityRealHp` lanes. Do not add adaptive rollout selection in this repair.

## Tests

Add only two narrow architecture checks:

1. A boss post-primary plan contains exactly `BossPotionRescue`.
2. The Act 2 boss potion rescue profile uses `LazyOnPop` and retains the existing boss budget, all-potion policy, and complete-line-only acceptance.

Do not add the full frozen Collector outcome as a permanent unit test. Use it as an explicit verification command after the focused tests pass.

## Success Criteria

- Existing successful primary boss searches perform no rescue work.
- A primary boss `CombatGap` gets one bounded lazy potion rescue unless existing budget guards stop the portfolio.
- The motivating Collector frozen case produces an executable complete win in the lazy diagnostic profile.
- Focused tests, architecture tests, and the library test suite pass.
