# Relic Source Audit Notes

This file records cross-class relic fixes found while comparing Rust relic
behavior against the decompiled Java sources under `D:/rust/cardcrawl/relics`.
It is intentionally evidence-focused; UI-only actions are omitted from the Rust
simulator unless they affect mechanics, RNG, legality, visibility, or state.

## Warped Tongs

Java evidence:
- `D:/rust/cardcrawl/relics/WarpedTongs.java`
- `WarpedTongs.atTurnStartPostDraw()` flashes, queues UI-only
  `RelicAboveCreatureAction`, then queues `UpgradeRandomCardAction`.

Rust result:
- `WarpedTongs` is now subscribed to `at_turn_start_post_draw`, not
  `at_turn_start`.
- The Rust hook queues `Action::UpgradeRandomCard` at the bottom, leaving the
  random hand-card selection deferred to the engine action handler like Java.
- UI-only flash / above-creature action is intentionally not represented.

Coverage:
- `warped_tongs_triggers_after_turn_start_draw_like_java_sources`

## Black Star

Java evidence:
- `D:/rust/cardcrawl/relics/BlackStar.java`
- `D:/rust/cardcrawl/rooms/MonsterRoomElite.java`
- `MonsterRoomElite.dropReward()` first calls `addRelicToRewards(tier)`, then
  if Black Star is owned calls `addNoncampRelicToRewards(returnRandomRelicTier())`.
- `AbstractDungeon.returnRandomNonCampfireRelic(tier)` repeatedly draws from the
  same tier while the result is `Peace Pipe`, `Shovel`, or `Girya`; skipped
  candidates are consumed.

Rust result:
- Normal elite relic rewards continue to use `random_relic`.
- Black Star's second elite relic now uses `random_noncampfire_relic_reward`,
  matching Java's exclusion of the three campfire relics.

Coverage:
- `black_star_second_elite_relic_skips_campfire_relics_like_java`
