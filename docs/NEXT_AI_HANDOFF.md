# Next AI Handoff

Date: 2026-05-10
Branch: `codex/evidence-path-cleanup-20260509`

## Current Rule

Continue Java-source-backed mechanics cleanup. Do not add strategy heuristics, bot compatibility, broad abstractions, UI/VFX simulation, or CleanRL/Gym-first constraints. Preserve gameplay semantics from `D:\rust\cardcrawl`; drop UI-only effects unless they carry gameplay state.

## Last Pushed Checkpoint

`4cb0237 Ground X-cost action timing`

What it fixed:

- Java generic `AbstractPlayer.useCard` does not spend energy for X-cost cards; each X-cost action computes effect and spends current energy during its own `update()`.
- Rust now records raw current energy as `energy_on_use` for X cards and lets Whirlwind/Transmutation handlers own Chemical X and spend timing.

Verification already passed:

- `cargo test -q content::cards::tests::ironclad_topdeck_and_strength_scaling_runtime_actions_match_java_use_methods`
- `cargo test -q content::cards::tests::transmutation_x_cost_action_matches_java_energy_and_chemical_x_timing`
- `cargo check -q`
- `git diff --check`

## Current Uncommitted Work

Files changed:

- `src/engine/action_handlers/damage.rs`
- `src/content/cards/tests.rs`

Java evidence:

- `D:\rust\cardcrawl\actions\unique\FeedAction.java`
- `D:\rust\cardcrawl\actions\unique\GreedAction.java`
- `D:\rust\cardcrawl\actions\unique\RitualDaggerAction.java`

Finding:

- Java Feed / Greed / Ritual Dagger grant their kill reward only if the damaged target is dead and not `halfDead` and does not have `Minion`.
- Rust Feed had that guard, but Hand of Greed and Ritual Dagger rewarded any `outcome.died` target, including minions and half-dead targets.

Implemented locally:

- Added shared `target_qualifies_for_non_minion_kill_reward` in the damage handler.
- Feed, Hand of Greed, and Ritual Dagger now use the same non-minion/non-halfDead reward guard.
- Added a regression test proving Greed rewards normal kills, rejects Minion kills, and Ritual Dagger rejects half-dead kills.

Verification already passed for this uncommitted work:

- `cargo test -q content::cards::tests::on_kill_card_rewards_ignore_minions_and_half_dead_targets_like_java_actions`
- `cargo test -q content::cards::tests::evolve_exhume_feed_and_feel_no_pain_hooks_match_java_sources`
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
