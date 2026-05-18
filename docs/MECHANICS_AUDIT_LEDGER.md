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
| Ordinary reward generation | `rooms/AbstractRoom.java`, `rooms/MonsterRoom.java`, `rooms/MonsterRoomElite.java`, `rooms/MonsterRoomBoss.java`, `screens/CombatRewardScreen.java`, `rewards/RewardItem.java` | `src/rewards/generator.rs`, `src/engine/run_loop.rs` | partial | reward generator tests, `daily_normal_combat_gold_is_fixed_and_does_not_consume_treasure_rng`, `daily_elite_combat_gold_is_fixed_and_does_not_consume_treasure_rng`, `daily_boss_combat_gold_is_fixed_and_does_not_consume_misc_rng` | Boss/elite/normal gold, elite relic/key order, EventRoom combat split, potion-before-card ordering, smoked/mugged handling, existing reward merge, and Daily Run fixed combat gold are source-checked. Remaining risk is uncommon/shared reward-screen modifiers or mod/custom branches not covered by current tests. |
| Treasure room chest rewards | `rooms/TreasureRoom.java`, `rewards/chests/AbstractChest.java`, chest subclasses, `relics/Matryoshka.java`, `relics/NlothsMask.java`, `relics/CursedKey.java` | `src/engine/run_loop.rs`, `src/rewards/state.rs` | locked | existing treasure tests plus `4895ac6`, `72d5620` | Non-boss chest size/reward roll, Cursed Key, Matryoshka before base relic, SapphireKey link, and N'loth's Mask after hook are locked. Boss chest handling is tracked separately. |
| Boss chest / boss relic choice | `rewards/chests/BossChest.java`, `screens/select/BossRelicSelectScreen.java`, `relics/AbstractRelic.java`, `rooms/TreasureRoomBoss.java`, `relics/CursedKey.java`, `relics/Matryoshka.java`, `relics/NlothsMask.java` | `src/rewards/handler.rs`, `src/rewards/boss_handler.rs`, `src/state/run.rs` | locked | existing boss reward tests plus `03779ed` | Boss relic choice generation, selection-before-act-transition, starter upgrade replacement, and boss chest exclusion from non-boss chest hooks are locked. Blight Chests custom-mod branch is intentionally unsupported until modded/custom mode is in scope. |
| Relic `onEquip` / `instantObtain` with direct run mutations | `relics/AbstractRelic.java`, concrete relics | `src/content/relics/*`, `src/state/run.rs`, reward handlers | partial | scattered relic tests; `CallingBell` locked in `72da496`; `Necronomicon` locked in `71c92b1`; `Astrolabe` fixed in `586fff0`; `PandorasBox` fixed in `0a795a8`; `TinyHouse` fixed in `72e808e`; `Cauldron` fixed in `00d2ecb`; `Orrery` fixed in `78aa564`; `DollysMirror` fixed in `c87f213`; bottled relic candidate filtering fixed in `ff3b846`; Empty Cage covered by `empty_cage_uses_java_purgeable_cards_and_auto_deletes_two_or_fewer` | Continue remaining selection-screen relics only if they are not already in the source audit; otherwise move to potion affordances or monster private state. |
| Master deck removal hooks | `cards/CardGroup.java`, curse/card `onRemoveFromMasterDeck` sources | `src/state/run.rs`, `src/deck/manager.rs` | partial | commits around `efbf00f`, `d8c5796`, `7529c30` | Recheck Necronomicurse, Parasite, event purge, shop purge, campfire toke, transform remove-all. |
| Master deck card copy / stat-equivalent copy | `cards/AbstractCard.java`, producers such as Nightmare/Duplicator/Anger/DollysMirror | `src/state/run.rs`, `src/runtime/combat.rs`, card modules | partial | commits around `ab78536`, `23d034d`, `d3c080e`, `b84fd78`; `DollysMirror` base block fixed in `c87f213` | Continue checking generated copies that preserve misc/cost/base-stat state, especially base magic representation gaps. |
| Card zone ordering and draw-pile API | `cards/CardGroup.java`, actions that add/remove/shuffle | `src/runtime/combat.rs`, action handlers | partial | runtime card zone tests | Keep revisiting whenever a Java call uses `addToTop`, `addToBottom`, `addToRandomSpot`, or `getTopCard`. |
| Potion run-level use/discard | `potions/*.java`, `ui/panels/PotionPopUp.java`, top panel/input code, `rewards/RewardItem.java` | `src/content/potions/*`, `src/engine/run_loop.rs`, observation/action code, `src/engine/action_handlers/mod.rs` | partial | run-level potion tests plus queued discard guard in `98208ad`; Fruit Juice immediate `increaseMaxHp` / on-use ordering covered by `combat_fruit_juice_increases_max_hp_immediately_before_toy_heal_queue` | Top-panel `canDiscard` is locked at both input/action-mask and queued action execution. Fruit Juice combat timing is locked against Java `increaseMaxHp`. Continue systematic audit of remaining target/use timing and any concrete potion not already source-checked. |
| Event pools and event gates | `dungeons/AbstractDungeon.java`, dungeon subclasses, `helpers/EventHelper.java`, `events/**/*.java` | `src/events/generator.rs`, `src/engine/run_loop.rs`, `src/engine/event_handler.rs` | partial | `events::generator` tests, `event_room_specific_event_selection_uses_duplicate_event_rng_like_java`, `question_mark_tiny_chest_forces_actual_treasure_after_event_room_enter_hooks` | Event/shrine pool ordering, one-time pool initialization, Java gate predicates for ordinary and one-time events, Act 4 empty pools, no fallback fabrication, EventHelper room-roll probabilities, Juzu/Tiny Chest/previous-shop handling, and duplicate event RNG selection are source-checked. Remaining risk is per-event handler coverage and events that start or resume combats/reward screens. |
| Map visibility and boss/key context | `map/MapRoomNode.java`, `map/DungeonMap.java`, `dungeons/AbstractDungeon.java`, `ui/panels/TopPanel.java`, `ui/buttons/ProceedButton.java` | `src/map/*`, `src/state/run.rs`, full-run observation code, map action code | partial | `wing_boots_matches_java_next_row_only_semantics`, `legal_map_actions_expose_wing_boots_only_on_next_row`, `map_observation_separates_owned_emerald_key_from_emerald_elite_marker`, `boss_node_availability_is_derived_from_java_map_position`, `map_observation_derives_boss_node_availability_from_position`, `combat_observation_keeps_public_map_context_like_java_top_panel`, `run_observation_exposes_all_top_panel_keys` | Wing Boots next-row movement, boss-node availability, public map context during combat/choices, top-panel key visibility, and Emerald elite marker versus owned Emerald key are source-checked. Remaining risk is full map generation/room assignment parity and any later observation schema change that could leak hidden future room/event contents. |
| Monster pools and encounter selection | `dungeons/AbstractDungeon.java`, `dungeons/Exordium.java`, `dungeons/TheCity.java`, `dungeons/TheBeyond.java`, `dungeons/TheEnding.java`, `monsters/MonsterInfo.java`, `helpers/MonsterHelper.java`, room classes | `src/content/monsters/encounter_pool.rs`, `src/content/monsters/factory.rs`, `src/state/run.rs`, CLI/full-run combat entrypoints | partial | `first_strong_exclusion_tables_match_java_sources`, `encounter_lists_preserve_java_generation_invariants`, `unknown_act_does_not_fall_back_to_exordium_encounter_lists`, boss-list tests, `cargo test factory --all-targets` | Encounter list generation, first-strong exclusions, Act 4 fixed Shield/Spear lists, boss-list generation, no fake JawWorm/Hexaghost fallbacks, and high-risk `MonsterHelper` factory RNG/constructor side effects are source-checked. Remaining risk is room-lifecycle parity: Java room creation reads the queue front and removes/regenerates when leaving the room, while Rust currently consumes at combat creation. Per-monster private AI/runtime fields are tracked in the next ledger row. |
| Monster AI/intent internals | `monsters/AbstractMonster.java`, concrete monster classes, `EnemyMoveInfo.java`, `CommunicationMod GameStateConverter.java` runtime exports | `src/content/monsters/*`, `src/diff/state_sync/build/monster.rs`, `src/runtime/combat.rs` | partial | `docs/audits/MONSTER_RUNTIME_TRUTH_AUDIT_2026-04-18.md`, act monster audits, focused runtime truth tests; Bronze Automaton / Bronze Orb / Book of Stabbing index reconciled after `07645cb`; Blue/Red Slaver, Fungi Beast/Spore Cloud, Small/Medium Slimes, Fat/Angry/Sneaky/Shield Gremlin, Centurion, Orb Walker, Spire Growth focused tests; Bandit trio SetMoveAction-chain tests; Repulsor and Taskmaster focused tests | Migrated stateful monsters use explicit runtime truth instead of hidden-state history inference. Blue Slaver, Fungi Beast, Small/Medium Slimes, Fat/Angry/Sneaky/Shield Gremlin, Centurion, Orb Walker, Spire Growth, Bandit Bear/Leader/Pointy, Repulsor, and Taskmaster are now recorded as no-hidden-runtime monsters whose history use is Java sequence logic, Java `takeTurn()` SetMoveAction chaining, or fixed-move behavior; Red Slaver remains explicit runtime truth for `first_turn`/`used_entangle`; Large Slimes remain explicit runtime truth for `split_triggered`. Spore Cloud now preserves Java's battle-ending guard. Remaining risk is systematic coverage of non-migrated or newly touched monster modules; do not re-audit rows marked Good without a reopen reason. |
| Events that start combats | `events/AbstractEvent.java`, `events/AbstractImageEvent.java`, `rooms/AbstractRoom.java`, event-specific Java files | `src/content/events/*`, `src/state/core.rs`, `src/engine/run_loop.rs` | partial | `fight_uses_java_shuffled_act1_boss_key_and_rare_relic_reward`, Dead Adventurer/Mushrooms/Masked Bandits/Mysterious Sphere/Colosseum event-combat tests, `event_combat_rewards_do_not_call_standard_combat_loot_generator` | Dead Adventurer, Mushrooms, Masked Bandits, Mysterious Sphere, Mind Bloom boss, and Colosseum event-combat boundaries are source-checked for Java encounter keys, preloaded rewards, Daily gold rolls, `rewardAllowed`, return-to-event, and `eliteTrigger`. Remaining risk is ordinary reward generation as a shared row plus unreviewed per-event handlers that start/resume combat or reward screens. |
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
