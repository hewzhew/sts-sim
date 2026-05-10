# Next AI Handoff

Date: 2026-05-10
Branch: `codex/evidence-path-cleanup-20260509`

## Current Rule

Continue Java-source-backed mechanics cleanup. Do not add strategy heuristics, bot compatibility, broad abstractions, UI/VFX simulation, or CleanRL/Gym-first constraints. Preserve gameplay semantics from `D:\rust\cardcrawl`; drop UI-only effects unless they carry gameplay state.

## Last Pushed Checkpoint

`dda7e4e Centralize Java card cost semantics`

Recent pushed mechanics fixes:

- `dda7e4e Centralize Java card cost semantics`
  - Added centralized `CombatCard` helpers for Java `AbstractCard` cost semantics.
  - Migrated known direct cost mutations through those helpers, including Blood for Blood, Corruption, Madness, Enlightenment, Confusion, Mummified Hand, generated-card overrides, and Liquid Memories.
  - Added `CardZones` UUID helpers for Java battle instances and queued-card artifacts.
  - Removed the old Blood for Blood cost rewrite from `evaluate_card`; cost changes now happen at Java lifecycle points.
- `ce0fb76 Persist combat misc card growth`
  - Java `RitualDaggerAction` first mutates the matching master-deck card and then the matching combat instances.
  - Rust now emits `MetaChange::ModifyCardMisc { card_uuid, amount }` from combat and applies it to `RunState.master_deck`.
  - Post-combat state keys include that meta change so retained action cleanup does not hide permanent growth.
- `889a0f0 Update all matching misc card instances`
  - `ModifyCardMisc` now matches Java `GetAllInBattleInstances` combat-zone semantics.
  - Java `RitualDaggerAction` mutates every battle instance with the matching UUID.
  - Rust no longer stops after the first matching card in hand/draw/discard/exhaust/limbo.
- `e2ef01d Match Corruption cost-for-turn semantics`
  - `Corruption` on-apply now matches Java `AbstractCard.setCostForTurn(-9)` semantics.
  - Rust no longer subtracts from `CombatCard.cost_modifier` when Corruption is applied.
  - Java `setCostForTurn` changes only the current turn cost field, not base cost or combat cost modifiers.
- `f7733d6 Tighten post-combat retained actions`
  - Refined Rust post-combat queue filtering to mirror Java `GameActionManager.clearPostCombatActions()`.
  - Only Java DAMAGE-equivalent actions, `Heal`, `GainBlock`, and `UseCardDone` are retained after lethal cleanup.
  - Java WAIT wrappers such as `WhirlwindAction`, `DropkickAction`, and `FiendFireAction` are not retained merely because they may later emit damage.
- `27e0fdf Match Java post-combat damage cleanup`
  - Added Java post-combat cleanup after lethal damage.
  - `Hand of Greed` gold and `Ritual Dagger` misc growth now resolve immediately inside their damage handlers, matching Java unique-action timing.
  - `Corruption` on-apply scan scope matches Java `ApplyPowerAction`: hand, draw pile, discard pile, and exhaust pile only; not `limbo`.

Verification already passed for those checkpoints:

- `cargo test -q content::cards::tests::lethal_damage_filters_post_combat_actions_like_java_action_manager`
- `cargo test -q content::cards::tests::on_kill_card_rewards_ignore_minions_and_half_dead_targets_like_java_actions`
- `cargo test -q content::cards::tests::ironclad_power_and_debuff_runtime_actions_match_java_use_methods`
- `cargo test -q content::cards::tests`
- `cargo test -q content::cards::tests::ironclad`
- `cargo check -q`
- `cargo test -q engine::action_handlers::cards::tests`
- `cargo test -q engine::core::tests`

## Current Work To Commit

Dual Wield pending-choice execution parity:

- Java `HandCardSelectScreen` removes the selected original from hand before `DualWieldAction` reads `selectedCards`.
- Java `DualWieldAction` then queues one extra `MakeTempCardInHandAction` plus `dupeAmount` more copies, replacing the removed original and creating the visible net copies.
- Rust `HandSelectReason::Copy` previously kept the selected original and also made the extra replacement copy, giving one extra card in multi-candidate Dual Wield selections.
- Rust now removes the selected hand card before calling `handle_make_copy_in_hand`.

Verification passed for this work:

- `cargo fmt`
- `cargo check --lib`
- `cargo test hand_select_copy_matches_dual_wield_selected_card_replacement --lib`
- `cargo test ironclad_copy_and_block_runtime_actions_match_java_use_methods --lib`
- `cargo test content::cards::tests --lib`
- `cargo test engine::pending_choices::tests --lib`
- `cargo test engine::action_handlers::cards::tests --lib`

Known verification limitation:

- `cargo test --lib` currently reports 432 passed / 13 failed because protocol fixture files under `tests/protocol_screen_action_space/...` are missing. Those failures are unrelated to this cost/instance cleanup and come from the deleted/dirty fixture tree.

## Next Work

Resume from Ironclad card-by-card Java audit, prioritizing non-trivial mechanics over simple damage/block cards.

Recommended next targets:

1. Continue Ironclad from the latter half, with emphasis on execution-time custom actions:
   - `Sentinel.triggerOnExhaust`
   - `RagePower.onUseCard`
   - `BerserkPower.atStartOfTurn`
   - `ExhumeAction`
   - `VampireDamageAllEnemiesAction`
2. Keep checking Java action timing, not just card `use()` methods:
   - queued `addToTop` vs `addToBot`
   - state inspected at action execution time
   - card instance mutation through `GetAllInBattleInstances`
   - generated-card `makeStatEquivalentCopy`
3. After Ironclad card audit stabilizes, move to shared mechanics exposed by those cards before starting full Silent/Defect/Watcher sweeps.

Useful reminders:

- Rust draw pile top is index `0`.
- Java `CardGroup` top is the list end.
- Java `MakeTempCardInDrawPileAction`: `toBottom` -> bottom, `randomSpot` -> random spot, otherwise top.
- Java `CardGroup.addToRandomSpot` never creates a new top when the group is nonempty; Rust helper already maps this.
- Java UI/VFX classes are ignored only when they are UI-only. `ShowCardAndAddToHandEffect`,
  `ShowCardAndAddToDiscardEffect`, and `ShowCardAndAddToDrawPileEffect` can call Master Reality
  and therefore carry gameplay state for repeat-upgrade cards like `Searing Blow`.
- Java `Shockwave.java` in this source applies Weak and Vulnerable only. Do not add Frail unless the Java source changes.
- Master Reality is a shared generated-card rule, not a reason to start the full Watcher card sweep yet.
