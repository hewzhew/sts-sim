# Next AI Handoff

Date: 2026-05-10
Branch: `codex/evidence-path-cleanup-20260509`

## Current Rule

Continue Java-source-backed mechanics cleanup. Do not add strategy heuristics, bot compatibility, broad abstractions, UI/VFX simulation, or CleanRL/Gym-first constraints. Preserve gameplay semantics from `D:\rust\cardcrawl`; drop UI-only effects unless they carry gameplay state.

## Last Pushed Checkpoint

`889a0f0 Update all matching misc card instances`

Recent pushed mechanics fixes:

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

Ironclad / shared Java action audit continued and found one non-strategy mechanics difference:

- Combat misc growth now persists back to the run master deck:
  - Java `RitualDaggerAction` first mutates the matching `player.masterDeck` card, then mutates `GetAllInBattleInstances`.
  - Rust now emits `MetaChange::ModifyCardMisc { card_uuid, amount }` from combat.
  - `tick_run` consumes that meta change and updates the matching `RunState.master_deck` card.
  - The post-combat stable turn-state key now includes this meta-change variant.

Verification passed for this work:

- `cargo test -q content::cards::tests::on_kill_card_rewards_ignore_minions_and_half_dead_targets_like_java_actions`
- `cargo test -q engine::run_loop::tests::combat_misc_meta_change_updates_matching_master_deck_card`
- `cargo test -q engine::run_loop::tests`
- `cargo test -q content::cards::tests`
- `cargo test -q engine::action_handlers::cards::tests`
- `cargo check -q`

## Next Work

Resume from Ironclad card-by-card Java audit, prioritizing non-trivial mechanics over simple damage/block cards.

Recommended next targets:

1. Continue Ironclad from the latter half, with emphasis on execution-time custom actions:
   - `Sentinel.triggerOnExhaust`
   - `RagePower.onUseCard`
   - `BerserkPower.atStartOfTurn`
   - `ExhumeAction`
   - `DualWieldAction`
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
