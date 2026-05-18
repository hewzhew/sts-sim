# Mechanics Audit Ledger

This is the run-level mechanics ledger for the Rust simulator. It complements
`AI_COMBAT_SOURCE_COVERAGE_LEDGER.md`, which is combat-kernel focused.

Goal: every mechanism that can change a real run must eventually have a Java
source owner, Rust owner, status, and acceptance check.

Use `docs/MECHANICS_ACCEPTANCE_STANDARD.md` as the stopping rule. A locked row
must not be reopened unless the reopen reason matches that standard.

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

unsupported_recorded:
  source-backed Java behavior is intentionally outside the current simulator
  scope, with a non-trainable reason recorded
```

## Gates

- A row cannot be `locked` without at least one test or commit.
- A row cannot be `locked` if a named gameplay branch, RNG consumer,
  visibility rule, or executor/mask divergence remains unreviewed.
- "Looks right" is not a status.
- If Java behavior is UI/VFX-hosted, record the UI/VFX file and the extracted
  non-UI mechanic.
- If a mechanism is intentionally not implemented, record the exact unsupported
  Java behavior and why it is non-trainable or out of scope.
- Locked rows are skipped by default in future passes. Reopen only for a failed
  test, a new Java owner/call path, a touched Rust owner/shared helper, a live
  truth contradiction, or an explicit remaining-risk item.

## Current Audit Table

| Subsystem | Java source owner | Rust owner | Status | Evidence | Remaining risk / next action |
| --- | --- | --- | --- | --- | --- |
| Event delayed card obtains | `vfx/cardManip/ShowCardAndObtainEffect.java`, event-specific `events/**/*.java` | `src/content/events/*`, `src/state/run.rs` | locked | commits `81789e4`, `56bec7f`, `525fe0b`, `3426913`, `298b8b1`, `a3b9be9`, `1022eb3`, `b84fd78`, `a69430d` | Transform representation can still be revisited, but ordinary delayed obtain hook ordering is locked. |
| `NoteForYourself` manual obtain | `events/shrines/NoteForYourself.java`, `cards/CardGroup.java` | `src/content/events/note_for_yourself.rs`, `src/state/run.rs` | locked | commit `4d3d455` | Cross-run profile persistence is simplified to stored Rust run fields; keep this explicit. |
| Reward card selection obtain | `rewards/RewardItem.java`, `screens/CardRewardScreen.java`, `vfx/FastCardObtainEffect.java`, `cards/Soul.java` | `src/rewards/handler.rs` | locked | commit `dcec769` | Codex/Discovery/ChooseOne reward-screen modes belong to combat/generated-choice audit, not ordinary reward claim. |
| Shop card purchase obtain | `shop/ShopScreen.java`, `vfx/FastCardObtainEffect.java` | `src/engine/shop_handler.rs`, `src/shop/*` | locked | commit `b2cc6ce` | Courier restock and prices have tests, but full shop UI navigation is not a simulator concern. |
| Cursed Key chest curse | `relics/CursedKey.java`, `rewards/chests/AbstractChest.java`, `helpers/CardLibrary.java` | `src/engine/run_loop.rs` | locked | commit `4895ac6` | Cursed Key non-boss chest curse obtain path is locked; keep separate from generic chest reward ordering. |
| Ordinary reward generation | `rooms/AbstractRoom.java`, `screens/CombatRewardScreen.java`, `rewards/RewardItem.java` | `src/rewards/generator.rs`, `src/engine/run_loop.rs` | partial | existing reward generator tests | Continue checking boss/elite/normal/event combat reward ordering and RNG streams. |
| Treasure room chest rewards | `rooms/TreasureRoom.java`, `rewards/chests/AbstractChest.java`, chest subclasses, `relics/Matryoshka.java`, `relics/NlothsMask.java`, `relics/CursedKey.java` | `src/engine/run_loop.rs`, `src/rewards/state.rs` | locked | existing treasure tests plus `4895ac6`, `72d5620` | Non-boss chest size/reward roll, Cursed Key, Matryoshka before base relic, SapphireKey link, and N'loth's Mask after hook are locked. Boss chest handling is tracked separately. |
| Boss chest / boss relic choice | `rewards/chests/BossChest.java`, `screens/select/BossRelicSelectScreen.java`, `relics/AbstractRelic.java`, `rooms/TreasureRoomBoss.java`, `relics/CursedKey.java`, `relics/Matryoshka.java`, `relics/NlothsMask.java` | `src/rewards/handler.rs`, `src/rewards/boss_handler.rs`, `src/state/run.rs` | locked | existing boss reward tests plus `03779ed` | Boss relic choice generation, selection-before-act-transition, starter upgrade replacement, and boss chest exclusion from non-boss chest hooks are locked. Blight Chests custom-mod branch is intentionally unsupported until modded/custom mode is in scope. |
| Relic `onEquip` / `instantObtain` with direct run mutations | `relics/AbstractRelic.java`, concrete relics | `src/content/relics/*`, `src/state/run.rs`, reward handlers | partial | scattered relic tests; `CallingBell` locked in `72da496`; `Necronomicon` locked in `71c92b1`; `Astrolabe` fixed in `586fff0`; `PandorasBox` fixed in `0a795a8`; `TinyHouse` fixed in `72e808e`; `Cauldron` fixed in `00d2ecb`; `Orrery` fixed in `78aa564`; `DollysMirror` fixed in `c87f213`; bottled relic candidate filtering fixed in `ff3b846`; Empty Cage covered by `empty_cage_uses_java_purgeable_cards_and_auto_deletes_two_or_fewer` | Continue remaining selection-screen relics only if they are not already in the source audit; otherwise move to potion affordances or monster private state. |
| Master deck removal hooks | `cards/CardGroup.java`, curse/card `onRemoveFromMasterDeck` sources | `src/state/run.rs`, `src/deck/manager.rs` | partial | commits around `efbf00f`, `d8c5796`, `7529c30` | Recheck Necronomicurse, Parasite, event purge, shop purge, campfire toke, transform remove-all. |
| Master deck card copy / stat-equivalent copy | `cards/AbstractCard.java`, producers such as Nightmare/Duplicator/Anger/DollysMirror | `src/state/run.rs`, `src/runtime/combat.rs`, card modules | partial | commits around `ab78536`, `23d034d`, `d3c080e`, `b84fd78`; `DollysMirror` base block fixed in `c87f213` | Continue checking generated copies that preserve misc/cost/base-stat state, especially base magic representation gaps. |
| Card zone ordering and draw-pile API | `cards/CardGroup.java`, actions that add/remove/shuffle | `src/runtime/combat.rs`, action handlers | partial | runtime card zone tests | Keep revisiting whenever a Java call uses `addToTop`, `addToBottom`, `addToRandomSpot`, or `getTopCard`. |
| Potion run-level use/discard | `potions/*.java`, `ui/panels/PotionPopUp.java`, top panel/input code, `rewards/RewardItem.java` | `src/content/potions/*`, `src/engine/run_loop.rs`, observation/action code, `src/engine/action_handlers/mod.rs` | partial | run-level potion tests plus queued discard guard in `98208ad`; Fruit Juice immediate `increaseMaxHp` / on-use ordering covered by `combat_fruit_juice_increases_max_hp_immediately_before_toy_heal_queue` | Top-panel `canDiscard` is locked at both input/action-mask and queued action execution. Fruit Juice combat timing is locked against Java `increaseMaxHp`. Continue systematic audit of remaining target/use timing and any concrete potion not already source-checked. |
| Event pools and event gates | `dungeons/AbstractDungeon.java`, `events/**/*.java`, event helper classes | `src/events/generator.rs`, `src/engine/event_handler.rs` | partial | event generator tests exist | Continue source-backed gates for act, floor, gold, HP, relic/card ownership, one-time pools. |
| Map visibility and boss/key context | `map/*`, `dungeons/AbstractDungeon.java`, top panel/boss key fields | `src/map/*`, `src/state/run.rs`, observation code | partial | map visibility tests exist | Need keep boss visibility/public run state separate from hidden future nodes. |
| Monster pools and encounter selection | `dungeons/AbstractDungeon.java`, `monsters/*`, room classes | `src/content/monsters/*`, `src/engine/run_loop.rs` | partial | some monster/encounter tests | Needs systematic monster source sweep; avoid old move-history approximation where Java has private move fields. |
| Monster AI/intent internals | `monsters/AbstractMonster.java`, concrete monster classes, `EnemyMoveInfo.java`, `CommunicationMod GameStateConverter.java` runtime exports | `src/content/monsters/*`, `src/diff/state_sync/build/monster.rs`, `src/runtime/combat.rs` | partial | `docs/audits/MONSTER_RUNTIME_TRUTH_AUDIT_2026-04-18.md`, act monster audits, focused runtime truth tests; Bronze Automaton / Bronze Orb / Book of Stabbing index reconciled after `07645cb` | Migrated stateful monsters use explicit runtime truth instead of hidden-state history inference. Remaining risk is systematic coverage of non-migrated or newly touched monster modules; do not re-audit rows marked Good without a reopen reason. |
| Events that start combats | Event-specific Java files, room classes | `src/content/events/*`, `src/engine/run_loop.rs` | partial | event combat reward tests exist | Need verify rewardAllowed/noCardsInRewards, return-to-event state, elite triggers, boss encounter ids. |
| Shop generation and restock | `shop/ShopScreen.java`, `shop/StoreRelic.java`, `shop/StorePotion.java` | `src/shop/*`, `src/engine/shop_handler.rs` | partial | shop handler/shop screen tests | Continue checking initial price RNG, sale tags, Courier restock streams, Membership/Smiling Mask order. |
| Campfire options and effects | `campfire/*`, `vfx/campfire/*.java`, relic campfire hooks | `src/engine/campfire_handler.rs`, relic modules | partial | campfire tests exist | UI/VFX-hosted mechanics must stay extracted, not simulated as UI. |
| Neow rewards | `neow/NeowEvent.java`, `vfx/FastCardObtainEffect.java` | `src/content/events/neow.rs` | partial | many Neow tests exist | Revisit reward-card/potion/direct-obtain paths after relic obtain lane. |

## Next Suggested Lane

Continue with relic obtain/equip paths that open run-level selection screens or
interrupt existing reward screens:

```text
Potion top-panel use timing and remaining concrete potion affordances
Monster private intent state
```

For each packet:

1. Check whether the ledger row is already locked. If so, require a reopen
   reason before reading broad source trees.
2. Open the concrete Java owner file and any VFX/screen/helper file it calls.
3. Identify whether it uses ordinary obtain, manual mutation, reward screen,
   selection screen, RNG, visibility, or execution-time state.
3. Compare with the Rust owner.
4. Add one narrow regression per ordering or interception point.
5. Update this ledger, `docs/JAVA_SOURCE_MAP.md`, and
   `docs/NEXT_AI_HANDOFF.md`.
