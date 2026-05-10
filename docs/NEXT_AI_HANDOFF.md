# Next AI Handoff

Date: 2026-05-10
Branch: `codex/evidence-path-cleanup-20260509`

## Current Rule

Continue Java-source-backed mechanics cleanup. Do not add strategy heuristics, bot compatibility, broad abstractions, UI/VFX simulation, or CleanRL/Gym-first constraints. Preserve gameplay semantics from `D:\rust\cardcrawl`; drop UI-only effects unless they carry gameplay state.

## Last Pushed Checkpoint

`99b6234 Ground random generated card class pools`

What it fixed:

- `MakeRandomCardInHand` and `MakeRandomCardInDrawPile` now use `state.meta.player_class` instead of hardcoded Ironclad pools.
- Added Silent regression tests proving random generated combat cards do not leak Ironclad cards.

Verification already passed:

- `cargo test -q engine::action_handlers::cards::tests`
- `cargo check -q`
- `git diff --check`

## Current Uncommitted Work

Files changed:

- `src/content/cards/ironclad/clash.rs`
- `src/content/cards/tests.rs`

Java evidence:

- `D:\rust\cardcrawl\cards\red\Clash.java`
- `D:\rust\cardcrawl\actions\common\PlayTopCardAction.java`

Finding:

- Java `Clash.canUse(...)` blocks only manual play when hand contains non-attacks.
- Java `Clash.use(...)` always queues damage.
- Forced play paths such as `PlayTopCardAction` / Havoc-style play should execute `Clash.use(...)`; they should not be blocked by manual `canUse` rules.

Implemented locally:

- Removed the hand non-attack check from `clash_play`.
- Kept the manual play gate in `can_play_card`.
- Updated the Ironclad runtime test to assert:
  - manual `can_play_card(Clash)` still fails with a Defend in hand;
  - direct `resolve_card_play(Clash)` still emits DamageAction, matching Java `use()`.

Verification already passed for this uncommitted work:

- `cargo test -q content::cards::tests::ironclad_attack_condition_and_dot_power_runtime_actions_match_java_use_methods`
- `cargo test -q engine::action_handlers::cards::tests`

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

