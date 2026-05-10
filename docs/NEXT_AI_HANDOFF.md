# Next AI Handoff

Date: 2026-05-10
Branch: `codex/evidence-path-cleanup-20260509`

## Current Rule

Continue Java-source-backed mechanics cleanup. Do not add strategy heuristics, bot compatibility, broad abstractions, UI/VFX simulation, or CleanRL/Gym-first constraints. Preserve gameplay semantics from `D:\rust\cardcrawl`; drop UI-only effects unless they carry gameplay state.

## Last Pushed Checkpoint

`fd68b33 Ground Fiend Fire queue semantics`

What it fixed:

- Java `FiendFireAction` queues random `ExhaustAction` instances and `DamageAction` instances; it does not immediately drain hand and damage monsters.
- Rust `handle_fiend_fire` now queues `ExhaustRandomCard` and `Damage` actions instead of directly mutating hand/monster HP.

Verification already passed:

- `cargo test -q content::cards::tests::fire_breathing_flame_barrier_and_fiend_fire_hooks_match_java_sources`
- `cargo test -q content::cards::tests::second_wind_block_per_non_attack_matches_java_add_to_top_order`
- `cargo check -q`
- `git diff --check`

## Current Uncommitted Work

Files changed:

- `src/engine/action_handlers/cards.rs`
- `src/content/cards/tests.rs`

Java evidence:

- `D:\rust\cardcrawl\cards\red\Havoc.java`
- `D:\rust\cardcrawl\actions\common\PlayTopCardAction.java`

Finding:

- Java `Havoc.use()` calls `getRandomMonster(..., cardRandomRng)` immediately and stores the result in `PlayTopCardAction`.
- Rust `havoc_play` emitted `PlayTopCard { target: None }`, causing target selection to occur later in `handle_play_top_card`.
- That changes RNG timing and can change target semantics when queued effects intervene.

Implemented locally:

- `execute_played_card` now locks Havoc's random target into `Action::PlayTopCard` at real play time while keeping `resolve_card_play` pure.
- Added a test proving played Havoc consumes `cardRandomRng` immediately and queues `PlayTopCard` with `Some(target)`.

Verification already passed for this uncommitted work:

- `cargo test -q content::cards::tests::headbutt_and_havoc_execution_helpers_match_java_sources`
- `cargo test -q content::cards::tests::ironclad_topdeck_and_strength_scaling_runtime_actions_match_java_use_methods`
- `cargo test -q content::cards::tests::fire_breathing_flame_barrier_and_fiend_fire_hooks_match_java_sources`

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
