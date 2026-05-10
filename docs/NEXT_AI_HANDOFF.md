# Next AI Handoff

Date: 2026-05-10
Branch: `codex/evidence-path-cleanup-20260509`

## Current Rule

Continue Java-source-backed mechanics cleanup. Do not add strategy heuristics, bot compatibility, broad abstractions, UI/VFX simulation, or CleanRL/Gym-first constraints. Preserve gameplay semantics from `D:\rust\cardcrawl`; drop UI-only effects unless they carry gameplay state.

## Last Pushed Checkpoint

`c98abfc Ground Clash forced-play semantics`

What it fixed:

- Java `Clash.canUse(...)` blocks only manual play when hand contains non-attacks.
- Java `Clash.use(...)` always queues damage.
- Rust now keeps the manual play gate in `can_play_card`, but `resolve_card_play(Clash)` emits damage for forced-play paths.

Verification already passed:

- `cargo test -q content::cards::tests::ironclad_attack_condition_and_dot_power_runtime_actions_match_java_use_methods`
- `cargo test -q engine::action_handlers::cards::tests`
- `cargo check -q`
- `git diff --check`

## Current Uncommitted Work

Files changed:

- `src/engine/pending_choices.rs`

Java evidence:

- `D:\rust\cardcrawl\actions\unique\DualWieldAction.java`
- `D:\rust\cardcrawl\actions\common\MakeTempCardInHandAction.java`

Finding:

- Java `DualWieldAction` creates copies via `MakeTempCardInHandAction`.
- Rust pending hand-select copy path was manually pushing copies with `uuid = 60000 + hand.len()`.
- That bypassed `card_uuid_counter` and generated-card hand overflow behavior.

Implemented locally:

- `HandSelectReason::Copy` now calls `handle_make_copy_in_hand`.
- Added a pending-choice test proving generated copies use `card_uuid_counter` and overflow to discard when hand is full.

Verification already passed for this uncommitted work:

- `cargo test -q engine::pending_choices::tests`
- `cargo test -q content::cards::tests::ironclad_copy_and_block_definitions_match_java_sources`
- `cargo test -q content::cards::tests::ironclad_block_exhaust_and_ethereal_definitions_match_java_sources`

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
