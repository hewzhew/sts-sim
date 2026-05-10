# Next AI Handoff

Date: 2026-05-10
Branch: `codex/evidence-path-cleanup-20260509`

## Current Rule

Continue Java-source-backed mechanics cleanup. Do not add strategy heuristics, bot compatibility, broad abstractions, UI/VFX simulation, or CleanRL/Gym-first constraints. Preserve gameplay semantics from `D:\rust\cardcrawl`; drop UI-only effects unless they carry gameplay state.

## Last Pushed Checkpoint

`7dc3875 Ground Second Wind action ordering`

What it fixed:

- Java `BlockPerNonAttackAction` queues `GainBlockAction` and `ExhaustSpecificCardAction` with two `addToTop` loops.
- Rust `handle_block_per_non_attack` now matches the Java queue order: all exhaust actions before all block gains, each group reversed relative to hand iteration.

Verification already passed:

- `cargo test -q engine::pending_choices::tests`
- `cargo test -q content::cards::tests::second_wind_block_per_non_attack_matches_java_add_to_top_order`
- `cargo test -q content::cards::tests::sever_soul_exhaust_all_non_attack_queues_exhausts_before_following_damage`
- `cargo check -q`
- `git diff --check`

## Current Uncommitted Work

Files changed:

- `src/engine/action_handlers/damage.rs`
- `src/content/cards/tests.rs`

Java evidence:

- `D:\rust\cardcrawl\cards\red\FiendFire.java`
- `D:\rust\cardcrawl\actions\unique\FiendFireAction.java`

Finding:

- Java `FiendFireAction` reads initial hand size, queues one `DamageAction` per hand card, then queues one random `ExhaustAction` per hand card.
- Actual execution is random exhaust actions first, then damage actions.
- Rust `handle_fiend_fire` directly drained the whole hand in order and applied damage immediately, skipping random exhaust actions and queue semantics.

Implemented locally:

- `handle_fiend_fire` now queues `Action::Damage` and `Action::ExhaustRandomCard { amount: 1 }` instead of immediately mutating hand/monster HP.
- Updated the Fiend Fire hook test to assert queued random exhausts appear before queued damage actions.

Verification already passed for this uncommitted work:

- `cargo test -q content::cards::tests::fire_breathing_flame_barrier_and_fiend_fire_hooks_match_java_sources`
- `cargo test -q content::cards::tests::second_wind_block_per_non_attack_matches_java_add_to_top_order`

Before continuing:

1. Run `cargo check -q`.
2. Run `git diff --check`.
3. Commit and push if clean.

## Next Work

Resume from Ironclad card-by-card Java audit, prioritizing non-trivial mechanics over simple damage/block cards.

Recommended next targets:

1. Forced-play/manual-gate separation:
   - cards with `canUse` overrides or conditional play rules;
   - verify `resolve_card_play` represents Java `use()`, not Java `canUse()`.
2. Hand-select and temporary hand removal semantics:
   - `ArmamentsAction`
   - `DualWieldAction`
   - `ExhaustAction`
   - confirm Rust pending choices preserve Java behavior without UI/VFX state.
3. Draw-pile order and generated-card placement:
   - keep using existing draw-pile helper APIs;
   - do not reintroduce direct `push` / `insert(0)` mutations.

Useful reminders:

- Rust draw pile top is index `0`.
- Java `CardGroup` top is the list end.
- Java `MakeTempCardInDrawPileAction`: `toBottom` -> bottom, `randomSpot` -> random spot, otherwise top.
- Java `CardGroup.addToRandomSpot` never creates a new top when the group is nonempty; Rust helper already maps this.
