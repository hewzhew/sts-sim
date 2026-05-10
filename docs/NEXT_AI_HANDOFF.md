# Next AI Handoff

Date: 2026-05-10
Branch: `codex/evidence-path-cleanup-20260509`

## Current Rule

Continue Java-source-backed mechanics cleanup. Do not add strategy heuristics, bot compatibility, broad abstractions, UI/VFX simulation, or CleanRL/Gym-first constraints. Preserve gameplay semantics from `D:\rust\cardcrawl`; drop UI-only effects unless they carry gameplay state.

## Last Pushed Checkpoint

`59187db Ground hand-select copy generation`

What it fixed:

- Java `DualWieldAction` creates copies via `MakeTempCardInHandAction`.
- Rust pending hand-select copy path no longer manually pushes copies with synthetic UUIDs.
- `HandSelectReason::Copy` now uses `handle_make_copy_in_hand`, preserving `card_uuid_counter` and hand-overflow-to-discard behavior.

Verification already passed:

- `cargo test -q engine::pending_choices::tests`
- `cargo test -q content::cards::tests::ironclad_copy_and_block_definitions_match_java_sources`
- `cargo test -q content::cards::tests::ironclad_block_exhaust_and_ethereal_definitions_match_java_sources`
- `cargo check -q`
- `git diff --check`

## Current Uncommitted Work

Files changed:

- `src/engine/action_handlers/damage.rs`
- `src/content/cards/tests.rs`

Java evidence:

- `D:\rust\cardcrawl\actions\unique\BlockPerNonAttackAction.java`
- `D:\rust\cardcrawl\cards\red\SecondWind.java`

Finding:

- Java `BlockPerNonAttackAction` uses `addToTop` in two loops:
  - first one `GainBlockAction` per non-attack;
  - then one `ExhaustSpecificCardAction` per non-attack.
- Actual execution order is all exhausts first, then all block gains, with each group reversed relative to hand iteration.
- Rust `handle_block_per_non_attack` used an intermediate action list and reversed it, producing a different order.

Implemented locally:

- `handle_block_per_non_attack` now queues actions in the same two-loop `addToTop` shape as Java.
- Added a test proving two non-attacks execute as: later non-attack exhaust, earlier non-attack exhaust, block, block.

Verification already passed for this uncommitted work:

- `cargo test -q engine::pending_choices::tests`
- `cargo test -q content::cards::tests::second_wind_block_per_non_attack_matches_java_add_to_top_order`
- `cargo test -q content::cards::tests::sever_soul_exhaust_all_non_attack_queues_exhausts_before_following_damage`

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
