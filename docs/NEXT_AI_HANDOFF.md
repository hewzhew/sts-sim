# Next AI Handoff

Date: 2026-05-10
Branch: `codex/evidence-path-cleanup-20260509`

## Current Rule

Continue Java-source-backed mechanics cleanup. Do not add strategy heuristics, bot compatibility, broad abstractions, UI/VFX simulation, or CleanRL/Gym-first constraints. Preserve gameplay semantics from `D:\rust\cardcrawl`; drop UI-only effects unless they carry gameplay state.

## Last Pushed Checkpoint

`6035c5e Ground half-dead random target guards`

What it fixed:

- Java `MonsterGroup.getRandomMonster(..., aliveOnly=true, rng)` excludes `halfDead`, `isDying`, and escaping monsters.
- Java `DamageAllEnemiesAction` skips `isDeadOrEscaped()` targets when applying damage, which includes half-dead monsters.
- Rust now has `MonsterEntity` helpers for Java-style dead/escaped and actionable/random-target checks.
- Random target, AOE, vampire-all, Dark orb lowest-HP target, Bouncing Flask, Time Warp monster self-buff targeting, and The Specimen no longer treat half-dead monsters as valid action targets.

Verification already passed:

- `cargo test -q content::cards::tests::random_enemy_attacks_ignore_half_dead_monsters_like_java_random_monster`
- `cargo test -q engine::action_handlers::powers::tests::bouncing_flask_random_target_ignores_half_dead_monsters_like_java_random_monster`
- `cargo test -q engine::action_handlers::powers::tests::apply_power_ignores_half_dead_monsters_like_java_is_dead_or_escaped`
- `cargo test -q content::cards::tests::ironclad_random_and_exhaust_attack_runtime_actions_match_java_use_methods`
- `cargo test -q content::cards::tests::ironclad_exhaust_debuff_and_intent_runtime_actions_match_java_use_methods`
- `cargo check -q`
- `git diff --check`

## Current Uncommitted Work

None.

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
