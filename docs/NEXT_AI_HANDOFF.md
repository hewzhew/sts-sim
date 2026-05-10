# Next AI Handoff

Date: 2026-05-10
Branch: `codex/evidence-path-cleanup-20260509`

## Current Rule

Continue Java-source-backed mechanics cleanup. Do not add strategy heuristics, bot compatibility, broad abstractions, UI/VFX simulation, or CleanRL/Gym-first constraints. Preserve gameplay semantics from `D:\rust\cardcrawl`; drop UI-only effects unless they carry gameplay state.

## Last Pushed Checkpoint

`f44003e Ground on-kill reward guards`

What it fixed:

- Java Feed / Greed / Ritual Dagger grant their kill reward only if the damaged target is dead and not `halfDead` and does not have `Minion`.
- Rust now shares that non-minion/non-halfDead reward guard for Feed, Hand of Greed, and Ritual Dagger.

Verification already passed:

- `cargo test -q content::cards::tests::on_kill_card_rewards_ignore_minions_and_half_dead_targets_like_java_actions`
- `cargo test -q content::cards::tests::evolve_exhume_feed_and_feel_no_pain_hooks_match_java_sources`
- `cargo check -q`
- `git diff --check`

## Current Uncommitted Work

Files changed:

- `src/engine/action_handlers/damage.rs`
- `src/engine/action_handlers/mod.rs`
- `src/runtime/action.rs`
- `src/content/cards/ironclad/entrench.rs`
- `src/content/cards/tests.rs`

Java evidence:

- `D:\rust\cardcrawl\actions\unique\VampireDamageAllEnemiesAction.java`
- `D:\rust\cardcrawl\actions\unique\VampireDamageAction.java`
- `D:\rust\cardcrawl\actions\common\HealAction.java`
- `D:\rust\cardcrawl\core\AbstractCreature.java`
- `D:\rust\cardcrawl\relics\MagicFlower.java`
- `D:\rust\cardcrawl\relics\MarkOfTheBloom.java`
- `D:\rust\cardcrawl\cards\red\Entrench.java`
- `D:\rust\cardcrawl\actions\unique\DoubleYourBlockAction.java`

Finding:

- Java Reaper/Shelled Parasite vampire damage queues `HealAction`; player healing therefore passes through `AbstractCreature.heal()`, including `MagicFlower.onPlayerHeal` and `MarkOfTheBloom.onPlayerHeal`.
- Rust vampire healing wrote player HP directly and bypassed those heal modifiers.
- Java Entrench queues `DoubleYourBlockAction`, which reads `target.currentBlock` when the action updates.
- Rust Entrench flattened current block into `GainBlock(amount)` during card play resolution.

Implemented locally:

- `heal_vampire_source` now applies the normal player heal modifier chain before changing HP.
- Added Reaper regression coverage for Magic Flower and Mark of the Bloom.
- Added `Action::DoubleBlock` and `handle_double_block`, and changed Entrench to emit it instead of precomputed `GainBlock`.
- Added regression coverage proving DoubleBlock reads block at execution time.

Verification already passed for this uncommitted work:

- `cargo test -q content::cards::tests::rupture_and_reaper_execution_hooks_match_java_sources`
- `cargo test -q content::cards::tests::ironclad_copy_and_block_runtime_actions_match_java_use_methods`
- `cargo check -q`
- `git diff --check`

Before continuing:

1. Commit and push if clean.

## Next Work

Resume from Ironclad card-by-card Java audit, prioritizing non-trivial mechanics over simple damage/block cards.

Recommended next targets:

1. Runtime condition actions:
   - `DropkickAction` already has deferred Vulnerable semantics; re-check handler against Java if touching nearby code.
   - Continue looking for cards where Java queues a custom action that checks state at execution time.
2. Multi-hit/random-target action boundaries:
   - `PummelDamageAction`
   - `AttackDamageRandomEnemyAction`
   - ensure Rust direct loops do not miss interleavable hooks or dead-target guards.
3. Remaining X-cost migration:
   - when implementing Defect/Watcher X cards, do not use generic hand-play energy spending;
   - each Java X action should own Chemical X and energy-spend timing.

Useful reminders:

- Rust draw pile top is index `0`.
- Java `CardGroup` top is the list end.
- Java `MakeTempCardInDrawPileAction`: `toBottom` -> bottom, `randomSpot` -> random spot, otherwise top.
- Java `CardGroup.addToRandomSpot` never creates a new top when the group is nonempty; Rust helper already maps this.
