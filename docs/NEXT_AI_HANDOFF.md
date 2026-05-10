# Next AI Handoff

Date: 2026-05-10
Branch: `codex/evidence-path-cleanup-20260509`

## Current Rule

Continue Java-source-backed mechanics cleanup. Do not add strategy heuristics, bot compatibility, broad abstractions, UI/VFX simulation, or CleanRL/Gym-first constraints. Preserve gameplay semantics from `D:\rust\cardcrawl`; drop UI-only effects unless they carry gameplay state.

## Last Pushed Checkpoint

`8f36fed Ground Havoc target timing`

What it fixed:

- Java `Havoc.use()` calls `getRandomMonster(..., cardRandomRng)` immediately and stores the result in `PlayTopCardAction`.
- Rust now locks Havoc's random target into `Action::PlayTopCard` at real play time while keeping `resolve_card_play` pure.

Verification already passed:

- `cargo test -q content::cards::tests::headbutt_and_havoc_execution_helpers_match_java_sources`
- `cargo test -q content::cards::tests::ironclad_topdeck_and_strength_scaling_runtime_actions_match_java_use_methods`
- `cargo test -q content::cards::tests::fire_breathing_flame_barrier_and_fiend_fire_hooks_match_java_sources`
- `cargo check -q`
- `git diff --check`

## Current Uncommitted Work

Files changed:

- `src/runtime/action.rs`
- `src/engine/action_handlers/mod.rs`
- `src/engine/action_handlers/powers.rs`
- `src/content/cards/ironclad/spot_weakness.rs`
- `src/content/cards/tests.rs`

Java evidence:

- `D:\rust\cardcrawl\cards\red\SpotWeakness.java`
- `D:\rust\cardcrawl\actions\unique\SpotWeaknessAction.java`

Finding:

- Java `SpotWeakness.use()` queues `SpotWeaknessAction`; `SpotWeaknessAction.update()` checks `targetMonster.getIntentBaseDmg() >= 0` when the action executes.
- Rust checked the monster's visible attack intent during `resolve_card_play`, so it could make the Strength decision too early and emit no action for non-attacking targets.

Implemented locally:

- Added `Action::SpotWeakness { target, amount }`.
- `spot_weakness_play` now always emits that action.
- `powers::handle_spot_weakness` checks the current monster intent at action execution and queues `ApplyPower(Strength)` to the back only if the target is currently attacking, matching Java `addToBot`.
- Updated the Ironclad intent test to prove an intent change after action creation changes the result.

Verification already passed for this uncommitted work:

- `cargo test -q content::cards::tests::ironclad_exhaust_debuff_and_intent_runtime_actions_match_java_use_methods`
- `cargo test -q content::cards::tests::ironclad_copy_and_block_runtime_actions_match_java_use_methods`
- `cargo test -q content::cards::tests::ironclad_topdeck_and_strength_scaling_runtime_actions_match_java_use_methods`
- `cargo check -q`
- `git diff --check`

Before continuing:

1. Commit and push if clean.

## Next Work

Resume from Ironclad card-by-card Java audit, prioritizing non-trivial mechanics over simple damage/block cards.

Recommended next targets:

1. X-cost and energy spend:
   - `Whirlwind.java`
   - `WhirlwindAction.java`
   - confirm energy use, Chemical X-style extra effect hooks, and repeated all-enemy damage semantics.
2. Runtime condition actions:
   - `DropkickAction` already has deferred Vulnerable semantics; re-check handler against Java if touching nearby code.
   - Continue looking for cards where Java queues a custom action that checks state at execution time.
3. Multi-hit/random-target action boundaries:
   - `PummelDamageAction`
   - `AttackDamageRandomEnemyAction`
   - ensure Rust direct loops do not miss interleavable hooks or dead-target guards.

Useful reminders:

- Rust draw pile top is index `0`.
- Java `CardGroup` top is the list end.
- Java `MakeTempCardInDrawPileAction`: `toBottom` -> bottom, `randomSpot` -> random spot, otherwise top.
- Java `CardGroup.addToRandomSpot` never creates a new top when the group is nonempty; Rust helper already maps this.
