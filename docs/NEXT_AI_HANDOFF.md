# Next AI Handoff

Date: 2026-05-10
Branch: `codex/evidence-path-cleanup-20260509`

## Current Rule

Continue Java-source-backed mechanics cleanup. Do not add strategy heuristics, bot compatibility, broad abstractions, UI/VFX simulation, or CleanRL/Gym-first constraints. Preserve gameplay semantics from `D:\rust\cardcrawl`; drop UI-only effects unless they carry gameplay state.

## Last Pushed Checkpoint

Latest pushed branch tip: `Match Pummel and Spot Weakness actions`

Recent pushed mechanics fixes:

- `Match Pummel and Spot Weakness actions`
  - Java `Pummel.use()` emits `magicNumber - 1` `PummelDamageAction` hits, then one ordinary `DamageAction`; Rust now has `Action::PummelDamage` and only the final hit is generic `Damage`.
  - Java `PummelDamageAction` re-checks `target.currentHealth > 0` at execution time and skips without damage/death cleanup if the target is already at 0 HP; Rust now preserves that guard.
  - Java `SpotWeaknessAction` checks `targetMonster.getIntentBaseDmg() >= 0`; Rust no longer relies solely on the UI-style visible-intent projection that hides dying/half-dead monsters.
- `2760997 Queue Limit Break strength application`
  - Java `LimitBreakAction` queues `ApplyPowerAction` with `addToTop`; Rust now queues `Action::ApplyPower` to the front instead of mutating Strength inline.
  - Regression checks the queued action before applying it.
- `329376e Match Armaments hand select order`
  - Java `ArmamentsAction` removes non-upgradeable cards before opening the hand select screen, then returns the selected card and non-upgradeables with `addToTop`.
  - Rust `HandSelectReason::Upgrade` now reproduces that multi-candidate hand order instead of upgrading in place.
  - Hand select now rejects duplicate UUID submissions before mutation.
- `0da451c Align vampire heal and Corruption timing`
  - Java `VampireDamageAction` / `VampireDamageAllEnemiesAction` queue `HealAction`; Rust no longer heals inline.
  - Single-target vampire heals queue to the front; Reaper/all-enemy vampire heals queue to the back, with post-combat cleanup re-run so retained `Heal` semantics match Java.
  - Java `ApplyPowerAction` applies Corruption to existing skills with `modifyCostForCombat(-9)`; Rust now mutates combat cost on apply while preserving `CorruptionPower.onCardDraw` as `setCostForTurn(-9)`.
- `0460ef3 Match Dual Wield hand select replacement`
  - Java `DualWieldAction` multi-candidate flow removes the selected original from hand via `HandCardSelectScreen` before adding replacement/copy cards.
  - Rust `HandSelectReason::Copy` now removes the selected original before making Dual Wield copies, avoiding one extra visible card.
  - Verified pending-choice, card, and action-handler tests after the change.
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

## Latest Mechanics Work

Pummel / Spot Weakness execution-time parity:

- Java `Pummel.use()` emits `magicNumber - 1` `PummelDamageAction` hits, then one ordinary `DamageAction`; Rust now has `Action::PummelDamage` and only the final hit is generic `Damage`.
- Java `PummelDamageAction` re-checks `target.currentHealth > 0` at execution time and skips without damage/death cleanup if the target is already at 0 HP; Rust now preserves that guard.
- Java `SpotWeaknessAction` checks `targetMonster.getIntentBaseDmg() >= 0`; Rust no longer relies solely on the UI-style visible-intent projection that hides dying/half-dead monsters.
- `PutOnDeckAction` and `FiendFireAction` were rechecked against Java. Existing Rust behavior/tests already cover the important edges: PutOnDeck's RNG fallback and shrinking-hand loop, Fiend Fire's queue order where random exhaust actions execute before the queued damage actions.

Verification passed for this work:

- `cargo fmt`
- `cargo check --lib`
- `cargo test pummel_damage_action --lib`
- `cargo test engine::action_handlers::damage::tests --lib`
- `cargo test spot_weakness_reads_raw_intent_base_damage_even_if_target_is_dying --lib`
- `cargo test engine::action_handlers::powers::tests --lib`
- `cargo test ironclad_multi_hit_and_rage_runtime_actions_match_java_use_methods --lib`
- `cargo test fire_breathing_flame_barrier_and_fiend_fire_hooks_match_java_sources --lib`
- `cargo test put_on_deck_action_matches_java_rng_and_selection_edges --lib`
- `cargo test content::cards::tests --lib`

Known verification limitation:

- `cargo test --lib` currently reports 432 passed / 13 failed because protocol fixture files under `tests/protocol_screen_action_space/...` are missing. Those failures are unrelated to this cost/instance cleanup and come from the deleted/dirty fixture tree.

## Next Work

Resume from Ironclad card-by-card Java audit, prioritizing non-trivial mechanics over simple damage/block cards.

Recommended next targets:

1. Continue Ironclad from the latter half, with emphasis on execution-time custom actions:
   - `CorruptionPower.onCardDraw` vs generated-card hand insertion paths
   - remaining custom queued actions where Java uses specialized action classes instead of plain `DamageAction`
   - selection actions whose Java screen order mutates hand or selected-card order
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
