# Mechanics Audit Ledger

This is the run-level mechanics ledger for the Rust simulator. It complements
`AI_COMBAT_SOURCE_COVERAGE_LEDGER.md`, which is combat-kernel focused.

Goal: every mechanism that can change a real run must eventually have a Java
source owner, Rust owner, status, and acceptance check.

## Status

```text
locked:
  Java source checked and behavior protected by focused Rust tests

partial:
  some behavior locked, but important branches or ordering remain unreviewed

source_checked:
  Java source read and summarized, but Rust tests or implementation still absent

suspect:
  likely mismatch or fragile abstraction; do not build AI/training assumptions on it

unreviewed:
  no current source-backed claim
```

## Gates

- A row cannot be `locked` without at least one test or commit.
- "Looks right" is not a status.
- If Java behavior is UI/VFX-hosted, record the UI/VFX file and the extracted
  non-UI mechanic.
- If a mechanism is intentionally not implemented, record the exact unsupported
  Java behavior and why it is non-trainable or out of scope.

## Current Audit Table

| Subsystem | Java source owner | Rust owner | Status | Evidence | Remaining risk / next action |
| --- | --- | --- | --- | --- | --- |
| Event delayed card obtains | `vfx/cardManip/ShowCardAndObtainEffect.java`, event-specific `events/**/*.java` | `src/content/events/*`, `src/state/run.rs` | locked | commits `81789e4`, `56bec7f`, `525fe0b`, `3426913`, `298b8b1`, `a3b9be9`, `1022eb3`, `b84fd78`, `a69430d` | Transform representation can still be revisited, but ordinary delayed obtain hook ordering is locked. |
| `NoteForYourself` manual obtain | `events/shrines/NoteForYourself.java`, `cards/CardGroup.java` | `src/content/events/note_for_yourself.rs`, `src/state/run.rs` | locked | commit `4d3d455` | Cross-run profile persistence is simplified to stored Rust run fields; keep this explicit. |
| Reward card selection obtain | `rewards/RewardItem.java`, `screens/CardRewardScreen.java`, `vfx/FastCardObtainEffect.java`, `cards/Soul.java` | `src/rewards/handler.rs` | locked | commit `dcec769` | Codex/Discovery/ChooseOne reward-screen modes belong to combat/generated-choice audit, not ordinary reward claim. |
| Shop card purchase obtain | `shop/ShopScreen.java`, `vfx/FastCardObtainEffect.java` | `src/engine/shop_handler.rs`, `src/shop/*` | locked | commit `b2cc6ce` | Courier restock and prices have tests, but full shop UI navigation is not a simulator concern. |
| Cursed Key chest curse | `relics/CursedKey.java`, `rewards/chests/AbstractChest.java`, `helpers/CardLibrary.java` | `src/engine/run_loop.rs` | locked | commit `4895ac6` | Chest `onChestOpenAfter` ordering should continue to be audited with Matryoshka/N'loth/Sapphire interactions. |
| Ordinary reward generation | `rooms/AbstractRoom.java`, `screens/CombatRewardScreen.java`, `rewards/RewardItem.java` | `src/rewards/generator.rs`, `src/engine/run_loop.rs` | partial | existing reward generator tests | Continue checking boss/elite/normal/event combat reward ordering and RNG streams. |
| Treasure room chest rewards | `rooms/TreasureRoom.java`, `rewards/chests/AbstractChest.java`, chest subclasses | `src/engine/run_loop.rs`, `src/rewards/state.rs` | partial | existing treasure tests plus `4895ac6` | Audit all chest hook phases: `onChestOpen`, gold, relic/key link, `onChestOpenAfter`, reward screen. |
| Relic `onEquip` / `instantObtain` with direct run mutations | `relics/AbstractRelic.java`, concrete relics | `src/content/relics/*`, `src/state/run.rs`, reward handlers | partial | scattered relic tests | Next high-value lane: `CallingBell`, `Necronomicon`, `Astrolabe`, `PandorasBox`, `TinyHouse`, `Cauldron`, `Orrery`. |
| Master deck removal hooks | `cards/CardGroup.java`, curse/card `onRemoveFromMasterDeck` sources | `src/state/run.rs`, `src/deck/manager.rs` | partial | commits around `efbf00f`, `d8c5796`, `7529c30` | Recheck Necronomicurse, Parasite, event purge, shop purge, campfire toke, transform remove-all. |
| Master deck card copy / stat-equivalent copy | `cards/AbstractCard.java`, producers such as Nightmare/Duplicator/Anger | `src/state/run.rs`, `src/runtime/combat.rs`, card modules | partial | commits around `ab78536`, `23d034d`, `d3c080e`, `b84fd78` | Continue checking generated copies that preserve misc/cost/base-stat state. |
| Card zone ordering and draw-pile API | `cards/CardGroup.java`, actions that add/remove/shuffle | `src/runtime/combat.rs`, action handlers | partial | runtime card zone tests | Keep revisiting whenever a Java call uses `addToTop`, `addToBottom`, `addToRandomSpot`, or `getTopCard`. |
| Potion run-level use/discard | `potions/*.java`, top panel/input code, `rewards/RewardItem.java` | `src/content/potions/*`, `src/engine/run_loop.rs`, observation/action code | partial | run-level potion tests exist | Need systematic audit of outside-combat usable potions, discard affordance, full slots, Sozu, Sacred Bark, Toy Ornithopter. |
| Event pools and event gates | `dungeons/AbstractDungeon.java`, `events/**/*.java`, event helper classes | `src/events/generator.rs`, `src/engine/event_handler.rs` | partial | event generator tests exist | Continue source-backed gates for act, floor, gold, HP, relic/card ownership, one-time pools. |
| Map visibility and boss/key context | `map/*`, `dungeons/AbstractDungeon.java`, top panel/boss key fields | `src/map/*`, `src/state/run.rs`, observation code | partial | map visibility tests exist | Need keep boss visibility/public run state separate from hidden future nodes. |
| Monster pools and encounter selection | `dungeons/AbstractDungeon.java`, `monsters/*`, room classes | `src/content/monsters/*`, `src/engine/run_loop.rs` | partial | some monster/encounter tests | Needs systematic monster source sweep; avoid old move-history approximation where Java has private move fields. |
| Monster AI/intent internals | `monsters/AbstractMonster.java`, concrete monster classes, `EnemyMoveInfo.java` | `src/content/monsters/*` | suspect | prior fixes and tests | Continue adding source fields instead of inferring from history. High priority before serious full-run training. |
| Events that start combats | Event-specific Java files, room classes | `src/content/events/*`, `src/engine/run_loop.rs` | partial | event combat reward tests exist | Need verify rewardAllowed/noCardsInRewards, return-to-event state, elite triggers, boss encounter ids. |
| Shop generation and restock | `shop/ShopScreen.java`, `shop/StoreRelic.java`, `shop/StorePotion.java` | `src/shop/*`, `src/engine/shop_handler.rs` | partial | shop handler/shop screen tests | Continue checking initial price RNG, sale tags, Courier restock streams, Membership/Smiling Mask order. |
| Campfire options and effects | `campfire/*`, `vfx/campfire/*.java`, relic campfire hooks | `src/engine/campfire_handler.rs`, relic modules | partial | campfire tests exist | UI/VFX-hosted mechanics must stay extracted, not simulated as UI. |
| Neow rewards | `neow/NeowEvent.java`, `vfx/FastCardObtainEffect.java` | `src/content/events/neow.rs` | partial | many Neow tests exist | Revisit reward-card/potion/direct-obtain paths after relic obtain lane. |

## Next Suggested Lane

Continue with relic obtain/equip paths that create cards or reward screens:

```text
CallingBell
Necronomicon
Astrolabe
PandorasBox
TinyHouse
Cauldron
Orrery
```

For each relic:

1. Open the concrete Java relic file and any VFX/screen helper it calls.
2. Identify whether it uses ordinary obtain, manual deck mutation, reward screen,
   or selection screen.
3. Compare with the Rust owner.
4. Add one narrow regression per ordering or interception point.
5. Update this ledger and `docs/NEXT_AI_HANDOFF.md`.
