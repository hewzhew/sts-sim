# Next AI Handoff

Date: 2026-05-10
Branch: `codex/evidence-path-cleanup-20260509`

## Current Rule

Continue Java-source-backed mechanics cleanup. Do not add strategy heuristics, bot compatibility, broad abstractions, UI/VFX simulation, or CleanRL/Gym-first constraints. Preserve gameplay semantics from `D:\rust\cardcrawl`; drop UI-only effects unless they carry gameplay state.

## Last Pushed Checkpoint

`f637fed Ground Rupture HP loss strength amount`

What it fixed:

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
- `cargo test -q content::cards::tests`
- `cargo test -q content::cards::tests::rupture_and_reaper_execution_hooks_match_java_sources`
- `cargo test -q content::cards::tests::ironclad_random_and_exhaust_attack_runtime_actions_match_java_use_methods`
- `cargo test -q content::cards::tests::ironclad_common_utility_runtime_actions_match_java_use_methods`
- `cargo check -q`
- `git diff --check`

## Current Uncommitted Work

None.

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
- Java `Shockwave.java` in this source applies Weak and Vulnerable only. Do not add Frail unless the Java source changes.
- Master Reality is a shared generated-card rule, not a reason to start the full Watcher card sweep yet.
