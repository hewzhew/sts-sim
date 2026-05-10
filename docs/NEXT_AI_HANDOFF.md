# Next AI Handoff

Date: 2026-05-10
Branch: `codex/evidence-path-cleanup-20260509`

## Current Rule

Continue Java-source-backed mechanics cleanup. Do not add strategy heuristics, bot compatibility, broad abstractions, UI/VFX simulation, or CleanRL/Gym-first constraints. Preserve gameplay semantics from `D:\rust\cardcrawl`; drop UI-only effects unless they carry gameplay state.

## Last Pushed Checkpoint

`511e3f7 Ground Spot Weakness action timing`

What it fixed:

- Java `SpotWeakness.use()` queues `SpotWeaknessAction`; `SpotWeaknessAction.update()` checks `targetMonster.getIntentBaseDmg() >= 0` when the action executes.
- Rust now emits `Action::SpotWeakness`; execution checks current monster intent and queues `ApplyPower(Strength)` to the back, matching Java `addToBot`.

Verification already passed:

- `cargo test -q content::cards::tests::ironclad_exhaust_debuff_and_intent_runtime_actions_match_java_use_methods`
- `cargo test -q content::cards::tests::ironclad_copy_and_block_runtime_actions_match_java_use_methods`
- `cargo test -q content::cards::tests::ironclad_topdeck_and_strength_scaling_runtime_actions_match_java_use_methods`
- `cargo check -q`
- `git diff --check`

## Current Uncommitted Work

Files changed:

- `src/runtime/action.rs`
- `src/engine/action_handlers/mod.rs`
- `src/engine/action_handlers/cards.rs`
- `src/engine/action_handlers/damage.rs`
- `src/content/cards/ironclad/whirlwind.rs`
- `src/content/cards/colorless/mod.rs`
- `src/content/cards/tests.rs`

Java evidence:

- `D:\rust\cardcrawl\cards\red\Whirlwind.java`
- `D:\rust\cardcrawl\actions\unique\WhirlwindAction.java`
- `D:\rust\cardcrawl\cards\colorless\Transmutation.java`
- `D:\rust\cardcrawl\actions\unique\TransmutationAction.java`
- `D:\rust\cardcrawl\characters\AbstractPlayer.java`
- `D:\rust\cardcrawl\cards\CardQueueItem.java`

Finding:

- Java generic `AbstractPlayer.useCard` does not spend energy for X-cost cards; each X-cost action computes effect and spends current energy during its own `update()`.
- Java X-cost actions add Chemical X inside the action, not in the generic card-play path.
- Rust spent all X energy in `handle_play_card_from_hand` before card actions executed and stored Chemical X-adjusted `energy_on_use` on the card.

Implemented locally:

- Added `Action::Whirlwind` and `Action::Transmutation`.
- `handle_play_card_from_hand` now records raw current energy as `energy_on_use` for X cards and does not spend X energy in the generic path.
- Whirlwind and Transmutation handlers apply Chemical X at action execution, queue their repeated effects, and spend current energy only if not `free_to_play_once`.
- Updated Whirlwind tests to prove Chemical X adds two repeats, free X does not spend energy, and manual hand play leaves X energy untouched until the X action executes.
- Added a Transmutation test for Java's stale `energyOnUse` correction to current energy plus Chemical X timing.

Verification already passed for this uncommitted work:

- `cargo test -q content::cards::tests::ironclad_topdeck_and_strength_scaling_runtime_actions_match_java_use_methods`
- `cargo test -q content::cards::tests::transmutation_x_cost_action_matches_java_energy_and_chemical_x_timing`
- `cargo check -q`
- `git diff --check`

Before continuing:

1. Commit and push if clean.

## Next Work

Resume from Ironclad card-by-card Java audit, prioritizing non-trivial mechanics over simple damage/block cards.

Recommended next targets:

1. Runtime condition actions:
   - `DropkickAction` already has deferred Vulnerable semantics; re-check handler against Java if touching nearby code.
   - Continue looking for cards where Java queues a custom action that checks state at execution time.
2. Multi-hit/random-target action boundaries:
   - `PummelDamageAction`
   - `AttackDamageRandomEnemyAction`
   - ensure Rust direct loops do not miss interleavable hooks or dead-target guards.
3. Remaining X-cost migration:
   - when implementing Defect/Watcher X cards, do not use generic hand-play energy spending;
   - each Java X action should own Chemical X and energy-spend timing.

Useful reminders:

- Rust draw pile top is index `0`.
- Java `CardGroup` top is the list end.
- Java `MakeTempCardInDrawPileAction`: `toBottom` -> bottom, `randomSpot` -> random spot, otherwise top.
- Java `CardGroup.addToRandomSpot` never creates a new top when the group is nonempty; Rust helper already maps this.
