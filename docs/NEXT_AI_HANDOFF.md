# Next AI Handoff

Date: 2026-05-10
Branch: `codex/evidence-path-cleanup-20260509`

## Current Rule

Continue Java-source-backed mechanics cleanup. Do not add strategy heuristics, bot compatibility, broad abstractions, UI/VFX simulation, or CleanRL/Gym-first constraints. Preserve gameplay semantics from `D:\rust\cardcrawl`; drop UI-only effects unless they carry gameplay state.

## Last Pushed Checkpoint

`8b1aef8 Ground Java generated card edge cases`

What it fixed:

- `8b1aef8 Ground Java generated card edge cases`
  - Added Java-backed `make_fresh_card_copy_for_combat` for `makeCopy()` hooks that depend on current combat state.
  - `Blood for Blood.makeCopy()` now applies `damagedThisCombat` cost reduction for random generated copies.
  - Master Reality generated-card handling now models Java upgrade call-site counts instead of treating UI/VFX classes as harmless:
    - hand paths such as `MakeTempCardInHandAction` plus `ShowCardAndAddToHandEffect` call upgrade twice;
    - draw-pile `MakeTempCardInDrawPileAction` with `amount < 6` plus `ShowCardAndAddToDrawPileEffect` calls upgrade twice;
    - discard-only `MakeTempCardInDiscardAction(card, amount)` calls upgrade once.
  - This matters mostly for `Searing Blow`, because normal cards ignore the second upgrade after their first upgrade but `SearingBlow.upgrade()` is repeatable.
  - Discovery / Toolbox / Codex pending-choice resolution now creates selected cards through the Java make-copy context and applies the appropriate Master Reality path.
- `35c4397 Ground generated card copy semantics`
  - Added Java-backed `CombatCard::make_stat_equivalent_copy_with_uuid`.
  - Generated copies now preserve permanent/stat-equivalent fields but drop transient evaluated damage/block/magic/multi-damage and one-shot play flags.
  - Added `PowerId::MasterRealityPower` so generated non-curse/non-status cards can upgrade when entering hand/discard/draw.
  - `MakeTempCardInDiscardAndDeck` now creates distinct draw/discard instances instead of cloning one UUID.
- `f637fed Ground Rupture HP loss strength amount`
  - Java `RupturePower.wasHPLost` grants `RupturePower.amount`, not HP lost.
  - Rust `resolve_power_on_hp_lost` now receives both `hp_lost` and `power_amount`.
  - Actual `handle_lose_hp` path now queues Strength equal to the Rupture stack amount.

Verification already passed:

- `cargo test -q engine::action_handlers::cards::tests`
- `cargo test -q engine::core::tests`
- `cargo test -q engine::core::tests::discovery_selection_uses_java_make_copy_and_master_reality_path`
- `cargo test -q engine::core::tests::card_reward_selection_preserves_codex_master_reality_single_draw_path`
- `cargo test -q content::cards::tests`
- `cargo test -q engine::action_handlers::cards::tests::searing_blow_preserves_java_master_reality_effect_call_counts`
- `cargo test -q engine::action_handlers::cards::tests::random_pool_blood_for_blood_copy_uses_java_make_copy_damage_discount`
- `cargo test -q engine::action_handlers::cards::tests::generated_cards_apply_master_reality_before_entering_zones`
- `cargo test -q content::cards::tests::rupture_and_reaper_execution_hooks_match_java_sources`
- `cargo test -q content::cards::tests::ironclad_random_and_exhaust_attack_runtime_actions_match_java_use_methods`
- `cargo test -q content::cards::tests::ironclad_common_utility_runtime_actions_match_java_use_methods`
- `cargo check -q`
- `git diff --check`

## Current Work To Commit

Ironclad Java audit continued and found two non-strategy mechanics differences:

- `Corruption` on-apply now matches Java `ApplyPowerAction` scan scope:
  - Java scans hand, draw pile, discard pile, and exhaust pile.
  - Java does not scan `limbo`.
  - Rust no longer zeroes Skill cards sitting in `limbo` when Corruption is applied.
- Lethal damage now applies Java `GameActionManager.clearPostCombatActions()` queue filtering:
  - Java damage actions call `clearPostCombatActions()` when all monsters are basically dead.
  - The filter keeps DAMAGE actions, `HealAction`, `GainBlockAction`, and `UseCardAction`.
  - Rust now keeps equivalent damage-domain actions, `Heal`, `GainBlock`, and `UseCardDone`.
  - Actions whose Java `actionType` is not DAMAGE, such as `WhirlwindAction`, `DropkickAction`, and `FiendFireAction`, are not retained merely because they may later emit damage.
  - This prevents lethal `Wild Strike` / `Reckless Charge` / `Immolate`-style queued card generation, lethal `Dropkick` draw/energy, and other non-retained post-combat actions from executing after the last monster dies.
  - `Hand of Greed` gold and `Ritual Dagger` misc growth now resolve immediately inside their damage handlers, matching Java unique action timing before post-combat queue filtering.

Verification passed for this work:

- `cargo test -q content::cards::tests::lethal_damage_filters_post_combat_actions_like_java_action_manager`
- `cargo test -q content::cards::tests::on_kill_card_rewards_ignore_minions_and_half_dead_targets_like_java_actions`
- `cargo test -q content::cards::tests::ironclad_power_and_debuff_runtime_actions_match_java_use_methods`
- `cargo test -q content::cards::tests`
- `cargo test -q content::cards::tests::ironclad`
- `cargo test -q engine::action_handlers::cards::tests`
- `cargo test -q engine::core::tests`

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
